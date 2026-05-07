use std::collections::HashSet;
use std::process::Command;
use std::time::Instant;

use anyhow::{anyhow, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config::{self, AppData, Config, KeybindMode};
use crate::scanner;
use crate::service::{is_safe_pid, normalize_memo, LocalService, MEMO_LIMIT};

const BOOT_TICKS: u64 = 19;
const SCROLL_ESTIMATE: usize = 8;

#[derive(Debug, Clone)]
pub enum Phase {
    Boot,
    KeybindSelect,
    Main,
    MemoEditor {
        service_key: String,
        original: String,
        text: String,
    },
    ConfirmKill {
        service: LocalService,
    },
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectorSide {
    Regular,
    Vim,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Info,
    Error,
}

#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub text: String,
    pub kind: StatusKind,
    pub created_at: Instant,
}

#[derive(Debug)]
pub struct App {
    pub config: Config,
    pub data: AppData,
    pub phase: Phase,
    pub first_run: bool,
    pub selector_side: SelectorSide,
    pub services: Vec<LocalService>,
    pub selected: usize,
    pub scroll: usize,
    pub boot_tick: u64,
    pub status: Option<StatusMessage>,
    pub running: bool,
}

impl App {
    pub fn new(config: Config, data: AppData, first_run: bool) -> Self {
        Self {
            config,
            data,
            phase: Phase::Boot,
            first_run,
            selector_side: SelectorSide::Regular,
            services: Vec::new(),
            selected: 0,
            scroll: 0,
            boot_tick: 0,
            status: None,
            running: true,
        }
    }

    pub fn tick(&mut self) -> bool {
        if matches!(self.phase, Phase::Boot) {
            self.boot_tick += 1;
            if self.boot_tick >= BOOT_TICKS {
                return self.finish_boot();
            }
        }

        if self
            .status
            .as_ref()
            .is_some_and(|status| status.created_at.elapsed().as_secs_f32() > 2.0)
        {
            self.status = None;
        }

        false
    }

    pub fn should_auto_refresh(&self) -> bool {
        matches!(self.phase, Phase::Main)
    }

    pub async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match &self.phase {
            Phase::Boot => self.handle_boot_key(key).await,
            Phase::KeybindSelect => self.handle_selector_key(key).await,
            Phase::Main => self.handle_main_key(key).await,
            Phase::MemoEditor { .. } => self.handle_memo_key(key),
            Phase::ConfirmKill { .. } => self.handle_confirm_key(key).await,
            Phase::Help => {
                self.phase = Phase::Main;
                Ok(())
            }
        }
    }

    pub async fn refresh_services(&mut self) -> Result<()> {
        let old_key = self.selected_service().map(LocalService::memo_key);
        let mut services = scanner::scan_services().await?;

        let live_keys = services
            .iter()
            .map(LocalService::memo_key)
            .collect::<HashSet<_>>();
        let before = self.data.memos.len();
        self.data.memos.retain(|key, _| live_keys.contains(key));
        if self.data.memos.len() != before {
            config::save_data(&self.data)?;
        }

        for service in &mut services {
            service.memo = self.data.memos.get(&service.memo_key()).cloned();
        }

        self.services = services;
        self.selected = old_key
            .and_then(|key| {
                self.services
                    .iter()
                    .position(|service| service.memo_key() == key)
            })
            .unwrap_or_else(|| self.selected.min(self.services.len().saturating_sub(1)));

        if self.services.is_empty() {
            self.selected = 0;
            self.scroll = 0;
        } else {
            self.keep_selection_visible();
        }

        Ok(())
    }

    pub fn persist(&self) -> Result<()> {
        config::save_config(&self.config)?;
        config::save_data(&self.data)
    }

    pub fn selected_service(&self) -> Option<&LocalService> {
        self.services.get(self.selected)
    }

    fn finish_boot(&mut self) -> bool {
        self.boot_tick = BOOT_TICKS;
        self.phase = if self.first_run {
            Phase::KeybindSelect
        } else {
            Phase::Main
        };
        !self.first_run
    }

    async fn handle_boot_key(&mut self, key: KeyEvent) -> Result<()> {
        if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) && self.finish_boot() {
            self.refresh_services().await?;
        }
        Ok(())
    }

    async fn handle_selector_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Left => self.selector_side = SelectorSide::Regular,
            KeyCode::Right => self.selector_side = SelectorSide::Vim,
            KeyCode::Enter => {
                self.config.keybind_mode = match self.selector_side {
                    SelectorSide::Regular => KeybindMode::Regular,
                    SelectorSide::Vim => KeybindMode::Vim,
                };
                config::save_config(&self.config)?;
                self.first_run = false;
                self.phase = Phase::Main;
                self.refresh_services().await?;
            }
            KeyCode::Char('q') => self.running = false,
            _ => {}
        }
        Ok(())
    }

    async fn handle_main_key(&mut self, key: KeyEvent) -> Result<()> {
        if self.matches_up(key) {
            self.select_previous();
            return Ok(());
        }
        if self.matches_down(key) {
            self.select_next();
            return Ok(());
        }

        match key.code {
            KeyCode::Enter => self.open_selected(),
            KeyCode::Char('m') => {
                self.open_memo_editor();
                Ok(())
            }
            KeyCode::Char('r') => {
                self.refresh_services().await?;
                self.set_status("refreshed localhost services", StatusKind::Info);
                Ok(())
            }
            KeyCode::Char('?') => {
                self.phase = Phase::Help;
                Ok(())
            }
            KeyCode::Char('q') => {
                self.running = false;
                Ok(())
            }
            KeyCode::Char('k') if self.config.keybind_mode == KeybindMode::Regular => {
                self.open_kill_confirmation();
                Ok(())
            }
            KeyCode::Char('K') if self.config.keybind_mode == KeybindMode::Vim => {
                self.open_kill_confirmation();
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn handle_memo_key(&mut self, key: KeyEvent) -> Result<()> {
        let Phase::MemoEditor {
            service_key,
            original,
            text,
        } = &mut self.phase
        else {
            return Ok(());
        };

        match key.code {
            KeyCode::Esc => {
                let _ = original;
                self.phase = Phase::Main;
            }
            KeyCode::Enter => {
                let service_key = service_key.clone();
                let memo = normalize_memo(text);
                match memo {
                    Some(memo) => {
                        self.data.memos.insert(service_key.clone(), memo.clone());
                        if let Some(service) = self
                            .services
                            .iter_mut()
                            .find(|service| service.memo_key() == service_key)
                        {
                            service.memo = Some(memo);
                        }
                    }
                    None => {
                        self.data.memos.remove(&service_key);
                        if let Some(service) = self
                            .services
                            .iter_mut()
                            .find(|service| service.memo_key() == service_key)
                        {
                            service.memo = None;
                        }
                    }
                }
                config::save_data(&self.data)?;
                self.phase = Phase::Main;
                self.set_status("memo saved", StatusKind::Info);
            }
            KeyCode::Backspace => {
                text.pop();
            }
            KeyCode::Char(ch)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT)
                    && text.chars().count() < MEMO_LIMIT =>
            {
                if ch != '\n' && ch != '\r' {
                    text.push(ch);
                }
            }
            _ => {}
        }

        Ok(())
    }

    async fn handle_confirm_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.phase = Phase::Main;
                Ok(())
            }
            KeyCode::Enter => self.kill_confirmed().await,
            _ => Ok(()),
        }
    }

    fn matches_up(&self, key: KeyEvent) -> bool {
        matches!(key.code, KeyCode::Up)
            || (self.config.keybind_mode == KeybindMode::Vim
                && matches!(key.code, KeyCode::Char('k')))
    }

    fn matches_down(&self, key: KeyEvent) -> bool {
        matches!(key.code, KeyCode::Down)
            || (self.config.keybind_mode == KeybindMode::Vim
                && matches!(key.code, KeyCode::Char('j')))
    }

    fn select_previous(&mut self) {
        if self.services.is_empty() {
            return;
        }
        self.selected = self.selected.saturating_sub(1);
        self.keep_selection_visible();
    }

    fn select_next(&mut self) {
        if self.services.is_empty() {
            return;
        }
        self.selected = (self.selected + 1).min(self.services.len() - 1);
        self.keep_selection_visible();
    }

    fn keep_selection_visible(&mut self) {
        if self.selected < self.scroll {
            self.scroll = self.selected;
        }

        if self.selected >= self.scroll + SCROLL_ESTIMATE {
            self.scroll = self.selected.saturating_sub(SCROLL_ESTIMATE - 1);
        }

        if self.scroll >= self.services.len() {
            self.scroll = self.services.len().saturating_sub(1);
        }
    }

    fn open_selected(&mut self) -> Result<()> {
        let Some(service) = self.selected_service() else {
            self.set_status("no service selected", StatusKind::Error);
            return Ok(());
        };

        let url = format!("http://localhost:{}", service.port);
        open_url(&url)?;
        self.set_status(&format!("opened {url}"), StatusKind::Info);
        Ok(())
    }

    fn open_memo_editor(&mut self) {
        let Some(service) = self.selected_service() else {
            self.set_status("no service selected", StatusKind::Error);
            return;
        };

        let service_key = service.memo_key();
        let original = service.memo.clone().unwrap_or_default();
        self.phase = Phase::MemoEditor {
            service_key,
            original: original.clone(),
            text: original,
        };
    }

    fn open_kill_confirmation(&mut self) {
        let Some(service) = self.selected_service() else {
            self.set_status("no service selected", StatusKind::Error);
            return;
        };

        if !is_safe_pid(service.pid) {
            self.set_status("refusing to kill unsafe pid", StatusKind::Error);
            return;
        }

        self.phase = Phase::ConfirmKill {
            service: service.clone(),
        };
    }

    async fn kill_confirmed(&mut self) -> Result<()> {
        let Phase::ConfirmKill { service } = self.phase.clone() else {
            return Ok(());
        };

        if !is_safe_pid(service.pid) {
            self.phase = Phase::Main;
            self.set_status("refusing to kill unsafe pid", StatusKind::Error);
            return Ok(());
        }

        let fresh = scanner::scan_services().await?;
        let still_listening = fresh
            .iter()
            .any(|candidate| candidate.pid == service.pid && candidate.port == service.port);

        if !still_listening {
            self.phase = Phase::Main;
            self.set_status("service changed; kill cancelled", StatusKind::Error);
            self.refresh_services().await?;
            return Ok(());
        }

        send_sigterm(service.pid)?;
        self.data.memos.remove(&service.memo_key());
        config::save_data(&self.data)?;
        self.phase = Phase::Main;
        self.set_status(
            &format!(
                "sent SIGTERM to {}:{}",
                service.display_name(),
                service.port
            ),
            StatusKind::Info,
        );
        self.refresh_services().await?;
        Ok(())
    }

    fn set_status(&mut self, text: &str, kind: StatusKind) {
        self.status = Some(StatusMessage {
            text: text.to_string(),
            kind,
            created_at: Instant::now(),
        });
    }
}

fn open_url(url: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    let opener = "open";
    #[cfg(target_os = "linux")]
    let opener = "xdg-open";
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    let opener = "";

    if opener.is_empty() {
        return Err(anyhow!("opening URLs is supported only on macOS and Linux"));
    }

    Command::new(opener)
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|err| anyhow!("failed to open {url}: {err}"))
}

fn send_sigterm(pid: u32) -> Result<()> {
    if !is_safe_pid(pid) {
        return Err(anyhow!("refusing to kill unsafe pid {pid}"));
    }

    let result = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
    if result == 0 {
        Ok(())
    } else {
        Err(anyhow!(
            "failed to send SIGTERM to pid {pid}: {}",
            std::io::Error::last_os_error()
        ))
    }
}
