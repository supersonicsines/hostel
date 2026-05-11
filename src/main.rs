use std::io::{self, stdout};
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    cursor,
    event::{Event, EventStream, KeyEventKind},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::time::{interval, MissedTickBehavior};
use tokio_stream::StreamExt;

mod app;
mod cli;
mod config;
mod mcp;
mod registry;
mod scanner;
mod service;
mod ui;

use app::App;

pub const PRODUCT_VERSION: &str = "I.0245";

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!("hostel {PRODUCT_VERSION}");
        return Ok(());
    }

    let runtime = tokio::runtime::Runtime::new()?;
    if cli::is_cli_command(&args) {
        runtime.block_on(cli::run(&args))
    } else {
        runtime.block_on(async_main())
    }
}

async fn async_main() -> Result<()> {
    let first_run = !config::config_exists();
    let config = config::load_config()?;
    let data = config::load_data()?;
    let mut app = App::new(config, data, first_run);

    terminal::enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, cursor::Hide)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = run_terminal_app(&mut terminal, &mut app).await;

    restore_terminal(&mut terminal)?;
    app.persist()?;
    result
}

async fn run_terminal_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    let mut key_events = EventStream::new();
    let mut tick = interval(Duration::from_millis(100));
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let mut refresh = interval(Duration::from_secs(2));
    refresh.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        tokio::select! {
            _ = tick.tick() => {
                if app.tick() {
                    app.refresh_services().await?;
                }
            }
            _ = refresh.tick() => {
                if app.should_auto_refresh() {
                    app.refresh_services().await?;
                }
            }
            maybe_event = key_events.next() => {
                match maybe_event {
                    Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat => {
                        app.handle_key(key).await?;
                    }
                    Some(Ok(_)) => {}
                    Some(Err(err)) => return Err(err.into()),
                    None => break,
                }
            }
        }

        if !app.running {
            break;
        }
    }

    Ok(())
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, cursor::Show)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::PRODUCT_VERSION;

    #[test]
    fn product_version_uses_display_format() {
        let (major, suffix) = PRODUCT_VERSION
            .split_once('.')
            .expect("product version has a dot");

        assert!(
            major == "0"
                || major
                    .chars()
                    .all(|ch| matches!(ch, 'I' | 'V' | 'X' | 'L' | 'C' | 'D' | 'M'))
        );
        assert_eq!(suffix.len(), 4);
        assert!(suffix.chars().all(|ch| ch.is_ascii_digit()));
    }
}
