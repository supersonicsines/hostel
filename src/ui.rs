use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    Frame,
};

use crate::app::{App, Phase, SelectorSide, StatusKind};
use crate::config::KeybindMode;
use crate::service::{LocalService, MEMO_LIMIT, TITLE_LIMIT};
use crate::PRODUCT_VERSION;

const BLACK: Color = Color::Black;
const WHITE: Color = Color::White;
const MID: Color = Color::Rgb(170, 170, 170);
const DIM: Color = Color::Rgb(120, 120, 120);
const FAINT: Color = Color::Rgb(85, 85, 85);

const SEL_SECONDARY: Color = Color::Rgb(45, 45, 45);
const SEL_TERTIARY: Color = Color::Rgb(90, 90, 90);
const SEL_FAINT: Color = Color::Rgb(130, 130, 130);

const PID_COLUMN: u16 = 8;
const SERVICE_COLUMN: u16 = 18;
const NARROW_SERVICE_COLUMN: u16 = 8;
const WIDE_ROW_MIN_WIDTH: u16 = 56;
const COMMAND_MIN_WIDTH: u16 = 78;

pub const HOSTEL_LOGO: &str = r"      ___           ___           ___                         ___
     /\  \         /\  \         /\__\                       /\__\
     \:\  \       /::\  \       /:/ _/_         ___         /:/ _/_
      \:\  \     /:/\:\  \     /:/ /\  \       /\__\       /:/ /\__\
  ___ /::\  \   /:/  \:\  \   /:/ /::\  \     /:/  /      /:/ /:/ _/_   ___     ___
 /\  /:/\:\__\ /:/__/ \:\__\ /:/_/:/\:\__\   /:/__/      /:/_/:/ /\__\ /\  \   /\__\
 \:\/:/  \/__/ \:\  \ /:/  / \:\/:/ /:/  /  /::\  \      \:\/:/ /:/  / \:\  \ /:/  /
  \::/__/       \:\  /:/  /   \::/ /:/  /  /:/\:\  \      \::/_/:/  /   \:\  /:/  /
   \:\  \        \:\/:/  /     \/_/:/  /   \/__\:\  \      \:\/:/  /     \:\/:/  /
    \:\__\        \::/  /        /:/  /         \:\__\      \::/  /       \::/  /
     \/__/         \/__/         \/__/           \/__/       \/__/         \/__/             ";

const COMPACT_LOGO: &str = "HOSTEL";

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.size();
    let buf = frame.buffer_mut();
    clear(buf, area);

    match &app.phase {
        Phase::Boot => render_boot(buf, area, app.boot_tick),
        Phase::KeybindSelect => render_selector(buf, area, app.selector_side),
        Phase::Main => render_main(buf, area, app),
        Phase::MemoEditor { text, .. } => {
            render_main(buf, area, app);
            render_memo_editor(buf, area, text, app.selected_service());
        }
        Phase::TitleEditor { text, .. } => {
            render_main(buf, area, app);
            render_title_editor(buf, area, text, app.selected_service());
        }
        Phase::TagsEditor { text, .. } => {
            render_main(buf, area, app);
            render_tags_editor(buf, area, text, app.selected_service());
        }
        Phase::FilterEditor { text, .. } => {
            render_main(buf, area, app);
            render_filter_editor(buf, area, text);
        }
        Phase::UrlEditor { text, .. } => {
            render_main(buf, area, app);
            render_url_editor(buf, area, text, app.selected_service());
        }
        Phase::ConfirmKill { service } => {
            render_main(buf, area, app);
            render_confirm_kill(buf, area, service);
        }
        Phase::Help => {
            render_main(buf, area, app);
            render_help(buf, area, app.config.keybind_mode);
        }
    }
}

fn render_boot(buf: &mut Buffer, area: Rect, tick: u64) {
    render_centered_logo(buf, area, logo_top(area), WHITE);

    let dots = match tick % 4 {
        0 => ".",
        1 => "..",
        2 => "...",
        _ => "",
    };
    let progress = ((tick.min(18) as usize) * 18) / 18;
    let bar = format!("[{}{}]", "█".repeat(progress), " ".repeat(18 - progress));
    let y = area.y + area.height.saturating_sub(5);
    centered_text(
        buf,
        area,
        y,
        "localhost services 1024-9999",
        Style::default().fg(MID).bg(BLACK),
    );
    centered_text(
        buf,
        area,
        y.saturating_add(1),
        &format!("scanning{dots}"),
        Style::default().fg(DIM).bg(BLACK),
    );
    centered_text(
        buf,
        area,
        y.saturating_add(2),
        &bar,
        Style::default().fg(WHITE).bg(BLACK),
    );
}

fn render_selector(buf: &mut Buffer, area: Rect, selected: SelectorSide) {
    if area.width < 46 || area.height < 12 {
        centered_text(
            buf,
            area,
            area.y + area.height / 2,
            "Choose: Left = Regular, Right = Vim, Enter = confirm",
            Style::default().fg(WHITE).bg(BLACK),
        );
        return;
    }

    let gap = 2;
    let half_width = (area.width.saturating_sub(gap + 4)) / 2;
    let height = area.height.saturating_sub(4);
    let y = area.y + 2;
    let left = Rect::new(area.x + 2, y, half_width, height);
    let right = Rect::new(left.right() + gap, y, half_width, height);

    render_selector_panel(
        buf,
        left,
        "REGULAR",
        &[
            "Arrow keys to move",
            "Enter opens service",
            "k kills safely",
        ],
        selected == SelectorSide::Regular,
    );
    render_selector_panel(
        buf,
        right,
        "VIM",
        &["j / k to move", "Enter opens service", "K kills safely"],
        selected == SelectorSide::Vim,
    );

    centered_text(
        buf,
        area,
        area.y,
        COMPACT_LOGO,
        Style::default()
            .fg(WHITE)
            .bg(BLACK)
            .add_modifier(Modifier::BOLD),
    );
    centered_text(
        buf,
        area,
        area.bottom().saturating_sub(1),
        "← → choose     Enter confirm",
        Style::default().fg(DIM).bg(BLACK),
    );
}

fn render_selector_panel(
    buf: &mut Buffer,
    area: Rect,
    title: &str,
    lines: &[&str],
    selected: bool,
) {
    let fg = if selected { BLACK } else { WHITE };
    let bg = if selected { WHITE } else { BLACK };
    let border = if selected { WHITE } else { DIM };
    fill(buf, area, Style::default().fg(fg).bg(bg));
    draw_box(buf, area, Style::default().fg(border).bg(bg));

    centered_text(
        buf,
        area,
        area.y + area.height / 2 - 3,
        title,
        Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
    );
    for (idx, line) in lines.iter().enumerate() {
        centered_text(
            buf,
            area,
            area.y + area.height / 2 + idx as u16,
            line,
            Style::default().fg(fg).bg(bg),
        );
    }
}

fn render_main(buf: &mut Buffer, area: Rect, app: &App) {
    if area.width < 40 || area.height < 13 {
        centered_text(
            buf,
            area,
            area.y + area.height / 2,
            "HOSTEL needs a little more room",
            Style::default().fg(WHITE).bg(BLACK),
        );
        return;
    }

    let logo_height = render_centered_logo(buf, area, logo_top(area), WHITE);
    let title_y = logo_top(area) + logo_height + 1;
    let title = if app.config.hidden_keywords.is_empty() {
        "localhost services 1024-9999".to_string()
    } else {
        let count = app.config.hidden_keywords.len();
        let noun = if count == 1 { "filter" } else { "filters" };
        format!("localhost services 1024-9999 · {count} {noun} active")
    };
    centered_text(
        buf,
        area,
        title_y,
        &title,
        Style::default().fg(MID).bg(BLACK),
    );

    let footer_y = area.bottom().saturating_sub(2);
    let table_top = title_y.saturating_add(2);
    let table_bottom = footer_y.saturating_sub(1);
    if table_bottom <= table_top + 4 {
        centered_text(
            buf,
            area,
            area.y + area.height / 2,
            "HOSTEL needs a little more room",
            Style::default().fg(WHITE).bg(BLACK),
        );
        return;
    }

    let table_width = area.width.saturating_sub(4).clamp(36, 88);
    let table_x = area.x + (area.width.saturating_sub(table_width)) / 2;
    let table = Rect::new(
        table_x,
        table_top,
        table_width,
        table_bottom.saturating_sub(table_top),
    );
    render_services_table(buf, table, app);
    render_footer(buf, area, footer_y, app);
}

fn render_services_table(buf: &mut Buffer, area: Rect, app: &App) {
    draw_box(buf, area, Style::default().fg(FAINT).bg(BLACK));
    let inner = inner(area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let wide = inner.width >= WIDE_ROW_MIN_WIDTH;
    let header = if wide {
        " PORT   PID       SERVICE"
    } else {
        " PORT   SERVICE"
    };
    write_clipped(
        buf,
        inner.x,
        inner.y,
        inner.width,
        header,
        Style::default().fg(MID).bg(BLACK),
    );
    if inner.height >= 2 {
        let rule = "─".repeat(inner.width as usize);
        let rule_style = Style::default().fg(FAINT).bg(BLACK);
        write_clipped(buf, inner.x, inner.y + 1, inner.width, &rule, rule_style);
        buf.get_mut(area.left(), inner.y + 1)
            .set_char('├')
            .set_style(rule_style);
        buf.get_mut(area.right().saturating_sub(1), inner.y + 1)
            .set_char('┤')
            .set_style(rule_style);
    }

    if app.services.is_empty() {
        let middle = inner.y + inner.height / 2;
        centered_text(
            buf,
            inner,
            middle,
            "No localhost services on ports 1024-9999",
            Style::default().fg(DIM).bg(BLACK),
        );
        centered_text(
            buf,
            inner,
            middle.saturating_add(1),
            "start a dev server, then press r to refresh",
            Style::default().fg(FAINT).bg(BLACK),
        );
        return;
    }

    let mut y = inner.y.saturating_add(2);
    for (index, service) in app.services.iter().enumerate().skip(app.scroll) {
        if y >= inner.bottom() {
            break;
        }

        let selected = index == app.selected;
        render_service_row(buf, inner, y, service, selected, wide);
        y += 1;

        if let Some(memo) = &service.metadata.memo {
            if y >= inner.bottom() {
                break;
            }
            render_memo_subtitle(buf, inner, y, memo, selected, wide);
            y += 1;
        }
    }
}

struct RowInk {
    base: Style,
    primary: Style,
    secondary: Style,
    tertiary: Style,
    faint: Style,
}

fn row_ink(selected: bool) -> RowInk {
    if selected {
        RowInk {
            base: Style::default().fg(BLACK).bg(WHITE),
            primary: Style::default().fg(BLACK).bg(WHITE),
            secondary: Style::default().fg(SEL_SECONDARY).bg(WHITE),
            tertiary: Style::default().fg(SEL_TERTIARY).bg(WHITE),
            faint: Style::default().fg(SEL_FAINT).bg(WHITE),
        }
    } else {
        RowInk {
            base: Style::default().fg(WHITE).bg(BLACK),
            primary: Style::default().fg(WHITE).bg(BLACK),
            secondary: Style::default().fg(MID).bg(BLACK),
            tertiary: Style::default().fg(DIM).bg(BLACK),
            faint: Style::default().fg(FAINT).bg(BLACK),
        }
    }
}

fn render_service_row(
    buf: &mut Buffer,
    inner: Rect,
    y: u16,
    service: &LocalService,
    selected: bool,
    wide: bool,
) {
    let ink = row_ink(selected);
    fill_line(buf, inner.x, y, inner.width, ink.base);

    if !wide {
        let port = format!("{:<6}", service.port);
        write_clipped(
            buf,
            inner.x + 1,
            y,
            6,
            &port,
            ink.primary.add_modifier(Modifier::BOLD),
        );
        write_clipped(
            buf,
            inner.x + NARROW_SERVICE_COLUMN,
            y,
            inner.width.saturating_sub(NARROW_SERVICE_COLUMN + 1),
            service.display_title(),
            ink.primary,
        );
        return;
    }

    let port = format!("{:<6}", service.port);
    let pid = format!("{:<8}", service.pid);
    write_clipped(
        buf,
        inner.x + 1,
        y,
        6,
        &port,
        ink.primary.add_modifier(Modifier::BOLD),
    );
    write_clipped(buf, inner.x + PID_COLUMN, y, 8, &pid, ink.tertiary);

    let titled = service.metadata.title.is_some();
    let mut segments: Vec<(String, Style)> = Vec::new();
    let title_style = if titled {
        ink.primary.add_modifier(Modifier::BOLD)
    } else {
        ink.primary
    };
    segments.push((service.display_title().to_string(), title_style));
    if titled {
        segments.push((format!("  {}", service.display_name()), ink.tertiary));
    }
    if let Some(label) = service.kind.label() {
        if !label.eq_ignore_ascii_case(service.display_title()) {
            segments.push((format!("  {label}"), ink.secondary));
        }
    }
    if !service.metadata.tags.is_empty() {
        segments.push((
            format!("  [{}]", service.metadata.tags.join(",")),
            ink.tertiary,
        ));
    }
    if let Some(path) = &service.metadata.url_path {
        segments.push((format!("  → {path}"), ink.secondary));
    }
    if let Some(source) = &service.metadata.source {
        segments.push((format!("  @{source}"), ink.tertiary));
    }
    if inner.width >= COMMAND_MIN_WIDTH && !service.command.trim().is_empty() {
        segments.push((format!("  {}", service.command), ink.faint));
    }

    write_segments(
        buf,
        inner.x + SERVICE_COLUMN,
        y,
        inner.width.saturating_sub(SERVICE_COLUMN + 1),
        &segments,
    );
}

fn render_memo_subtitle(
    buf: &mut Buffer,
    inner: Rect,
    y: u16,
    memo: &str,
    selected: bool,
    wide: bool,
) {
    let ink = row_ink(selected);
    let indent = if wide {
        SERVICE_COLUMN
    } else {
        NARROW_SERVICE_COLUMN
    };
    fill_line(buf, inner.x, y, inner.width, ink.base);
    write_clipped(
        buf,
        inner.x + indent,
        y,
        inner.width.saturating_sub(indent + 1),
        memo,
        ink.tertiary,
    );
}

fn render_footer(buf: &mut Buffer, area: Rect, y: u16, app: &App) {
    if let Some(status) = &app.status {
        let color = match status.kind {
            StatusKind::Info => WHITE,
            StatusKind::Error => Color::Rgb(255, 110, 110),
        };
        centered_text(
            buf,
            area,
            y,
            &status.text,
            Style::default().fg(color).bg(BLACK),
        );
    } else {
        render_key_hints(buf, area, y, app.config.keybind_mode);
    }

    centered_text(
        buf,
        area,
        y.saturating_add(1),
        &format!("HOSTEL {PRODUCT_VERSION}"),
        Style::default().fg(FAINT).bg(BLACK),
    );
}

fn render_key_hints(buf: &mut Buffer, area: Rect, y: u16, mode: KeybindMode) {
    let full = key_hints(mode);
    let minimal = vec![full[0], full[1], ("?", "help"), ("q", "quit")];

    let hint_width = |hints: &[(&str, &str)], sep: usize| {
        hints
            .iter()
            .map(|(key, verb)| key.chars().count() + 1 + verb.chars().count())
            .sum::<usize>()
            + sep * hints.len().saturating_sub(1)
    };

    let fitted = [(&full, 3usize), (&full, 2), (&minimal, 3), (&minimal, 2)]
        .into_iter()
        .find(|(hints, sep)| hint_width(hints, *sep) <= area.width as usize);

    let Some((hints, sep_width)) = fitted else {
        let joined = minimal
            .iter()
            .map(|(key, verb)| format!("{key} {verb}"))
            .collect::<Vec<_>>()
            .join("  ");
        centered_text(buf, area, y, &joined, Style::default().fg(DIM).bg(BLACK));
        return;
    };

    let total = hint_width(hints, sep_width) as u16;
    let mut segments: Vec<(String, Style)> = Vec::new();
    for (idx, (key, verb)) in hints.iter().enumerate() {
        if idx > 0 {
            segments.push((" ".repeat(sep_width), Style::default().fg(DIM).bg(BLACK)));
        }
        segments.push((key.to_string(), Style::default().fg(MID).bg(BLACK)));
        segments.push((format!(" {verb}"), Style::default().fg(DIM).bg(BLACK)));
    }

    let x = area.x + (area.width.saturating_sub(total)) / 2;
    write_segments(buf, x, y, total, &segments);
}

fn key_hints(mode: KeybindMode) -> Vec<(&'static str, &'static str)> {
    let (select, kill) = match mode {
        KeybindMode::Regular => ("↑↓", "k"),
        KeybindMode::Vim => ("j/k", "K"),
    };
    vec![
        (select, "select"),
        ("Enter", "open"),
        (kill, "kill"),
        ("t", "title"),
        ("m", "memo"),
        ("g", "tags"),
        ("u", "url"),
        ("f", "filter"),
        ("r", "refresh"),
        ("?", "help"),
        ("q", "quit"),
    ]
}

struct EditorChrome<'a> {
    title: &'a str,
    context: Option<String>,
    hint: Option<&'a str>,
    text: &'a str,
    limit: Option<usize>,
    min_width: u16,
    max_width: u16,
}

fn service_context(service: Option<&LocalService>) -> Option<String> {
    service.map(|service| format!("{} · localhost:{}", service.display_name(), service.port))
}

fn render_editor(buf: &mut Buffer, area: Rect, chrome: EditorChrome) {
    let extra = u16::from(chrome.context.is_some()) + u16::from(chrome.hint.is_some());
    let width = area
        .width
        .saturating_sub(4)
        .clamp(chrome.min_width, chrome.max_width);
    let height = (7 + extra).min(area.height.saturating_sub(2)).max(5);
    let rect = centered_rect(area, width, height);
    fill(buf, rect, Style::default().fg(WHITE).bg(BLACK));
    draw_box(buf, rect, Style::default().fg(WHITE).bg(BLACK));

    write_clipped(
        buf,
        rect.x + 2,
        rect.y,
        rect.width.saturating_sub(4),
        &format!(" {} ", chrome.title),
        Style::default()
            .fg(WHITE)
            .bg(BLACK)
            .add_modifier(Modifier::BOLD),
    );

    if let Some(limit) = chrome.limit {
        let count = chrome.text.chars().count();
        let counter = format!("{count}/{limit}");
        let counter_color = if count >= limit { WHITE } else { DIM };
        write_clipped(
            buf,
            rect.x
                + rect
                    .width
                    .saturating_sub(counter.chars().count() as u16 + 2),
            rect.y,
            counter.chars().count() as u16,
            &counter,
            Style::default().fg(counter_color).bg(BLACK),
        );
    }

    let inner_width = rect.width.saturating_sub(4);
    let last_inner_y = rect.bottom().saturating_sub(2);
    let mut row = rect.y + 1;

    if let Some(context) = &chrome.context {
        if row < last_inner_y {
            write_clipped(
                buf,
                rect.x + 2,
                row,
                inner_width,
                context,
                Style::default().fg(MID).bg(BLACK),
            );
        }
        row += 1;
    }
    if let Some(hint) = chrome.hint {
        if row < last_inner_y {
            write_clipped(
                buf,
                rect.x + 2,
                row,
                inner_width,
                hint,
                Style::default().fg(DIM).bg(BLACK),
            );
        }
        row += 1;
    }

    let input_y = row + 1;
    if input_y < last_inner_y {
        write_clipped(
            buf,
            rect.x + 2,
            input_y,
            inner_width,
            chrome.text,
            Style::default().fg(WHITE).bg(BLACK),
        );
        let cursor_offset = (chrome.text.chars().count() as u16).min(inner_width.saturating_sub(1));
        fill_line(
            buf,
            rect.x + 2 + cursor_offset,
            input_y,
            1,
            Style::default().fg(BLACK).bg(WHITE),
        );
    }

    write_clipped(
        buf,
        rect.x + 2,
        last_inner_y,
        inner_width,
        "Enter save   Esc cancel   empty clears",
        Style::default().fg(DIM).bg(BLACK),
    );
}

fn render_memo_editor(buf: &mut Buffer, area: Rect, text: &str, service: Option<&LocalService>) {
    render_editor(
        buf,
        area,
        EditorChrome {
            title: "Memo",
            context: service_context(service),
            hint: None,
            text,
            limit: Some(MEMO_LIMIT),
            min_width: 32,
            max_width: 72,
        },
    );
}

fn render_title_editor(buf: &mut Buffer, area: Rect, text: &str, service: Option<&LocalService>) {
    render_editor(
        buf,
        area,
        EditorChrome {
            title: "Title",
            context: service_context(service),
            hint: None,
            text,
            limit: Some(TITLE_LIMIT),
            min_width: 32,
            max_width: 72,
        },
    );
}

fn render_tags_editor(buf: &mut Buffer, area: Rect, text: &str, service: Option<&LocalService>) {
    render_editor(
        buf,
        area,
        EditorChrome {
            title: "Tags",
            context: service_context(service),
            hint: Some("Comma-separated tags"),
            text,
            limit: None,
            min_width: 38,
            max_width: 76,
        },
    );
}

fn render_filter_editor(buf: &mut Buffer, area: Rect, text: &str) {
    render_editor(
        buf,
        area,
        EditorChrome {
            title: "Hide filters",
            context: None,
            hint: Some("Comma-separated keywords"),
            text,
            limit: None,
            min_width: 38,
            max_width: 76,
        },
    );
}

fn render_url_editor(buf: &mut Buffer, area: Rect, text: &str, service: Option<&LocalService>) {
    render_editor(
        buf,
        area,
        EditorChrome {
            title: "Open path",
            context: service_context(service),
            hint: Some("Path appended to localhost, like /docs"),
            text,
            limit: Some(crate::service::URL_PATH_LIMIT),
            min_width: 38,
            max_width: 76,
        },
    );
}

fn render_confirm_kill(buf: &mut Buffer, area: Rect, service: &LocalService) {
    let width = area.width.saturating_sub(4).clamp(38, 58);
    let height = 9.min(area.height.saturating_sub(2)).max(6);
    let rect = centered_rect(area, width, height);
    fill(buf, rect, Style::default().fg(WHITE).bg(BLACK));
    draw_box(buf, rect, Style::default().fg(WHITE).bg(BLACK));

    centered_text(
        buf,
        rect,
        rect.y + 1,
        &format!(
            "Kill {} on localhost:{}?",
            service.display_name(),
            service.port
        ),
        Style::default()
            .fg(WHITE)
            .bg(BLACK)
            .add_modifier(Modifier::BOLD),
    );
    centered_text(
        buf,
        rect,
        rect.y + 3,
        &format!("PID {}", service.pid),
        Style::default().fg(MID).bg(BLACK),
    );
    centered_text(
        buf,
        rect,
        rect.y + 4,
        "rescans and verifies pid, then sends SIGTERM",
        Style::default().fg(DIM).bg(BLACK),
    );
    centered_text(
        buf,
        rect,
        rect.bottom().saturating_sub(2),
        "Enter confirm   Esc cancel",
        Style::default().fg(DIM).bg(BLACK),
    );
}

fn render_help(buf: &mut Buffer, area: Rect, mode: KeybindMode) {
    let hints = key_hints(mode);
    let key_width = hints
        .iter()
        .map(|(key, _)| key.chars().count())
        .max()
        .unwrap_or(0);
    let verb_width = hints
        .iter()
        .map(|(_, verb)| verb.chars().count())
        .max()
        .unwrap_or(0);
    let line_width = (key_width + 2 + verb_width) as u16;

    let width = area.width.saturating_sub(4).clamp(34, 58);
    let height = (hints.len() as u16 + 6)
        .min(area.height.saturating_sub(2))
        .max(8);
    let rect = centered_rect(area, width, height);
    fill(buf, rect, Style::default().fg(WHITE).bg(BLACK));
    draw_box(buf, rect, Style::default().fg(DIM).bg(BLACK));
    centered_text(
        buf,
        rect,
        rect.y + 1,
        "HOSTEL keys",
        Style::default()
            .fg(WHITE)
            .bg(BLACK)
            .add_modifier(Modifier::BOLD),
    );

    let x = rect.x + rect.width.saturating_sub(line_width) / 2;
    let last_inner_y = rect.bottom().saturating_sub(2);
    for (idx, (key, verb)) in hints.iter().enumerate() {
        let y = rect.y + 3 + idx as u16;
        if y >= last_inner_y {
            break;
        }
        let segments = [
            (
                format!("{key:>key_width$}"),
                Style::default().fg(WHITE).bg(BLACK),
            ),
            (format!("  {verb}"), Style::default().fg(DIM).bg(BLACK)),
        ];
        write_segments(buf, x, y, line_width, &segments);
    }

    centered_text(
        buf,
        rect,
        last_inner_y,
        "any key closes",
        Style::default().fg(FAINT).bg(BLACK),
    );
}

fn render_centered_logo(buf: &mut Buffer, area: Rect, top: u16, color: Color) -> u16 {
    let lines = logo_lines(area);
    let width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0) as u16;
    let x = area.x + area.width.saturating_sub(width) / 2;
    for (idx, line) in lines.iter().enumerate() {
        let y = top + idx as u16;
        if y >= area.bottom() {
            break;
        }
        write_clipped(
            buf,
            x,
            y,
            area.width.saturating_sub(x - area.x),
            line,
            Style::default()
                .fg(color)
                .bg(BLACK)
                .add_modifier(Modifier::BOLD),
        );
    }
    lines.len() as u16
}

fn logo_lines(area: Rect) -> Vec<&'static str> {
    let full = HOSTEL_LOGO.lines().collect::<Vec<_>>();
    let width = full
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0) as u16;
    if area.width >= width.saturating_add(2) && area.height >= 25 {
        full
    } else {
        vec![COMPACT_LOGO]
    }
}

fn logo_top(area: Rect) -> u16 {
    if area.height >= 30 {
        area.y + 1
    } else {
        area.y
    }
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width.min(area.width),
        height.min(area.height),
    )
}

fn inner(area: Rect) -> Rect {
    Rect::new(
        area.x.saturating_add(1),
        area.y.saturating_add(1),
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    )
}

fn clear(buf: &mut Buffer, area: Rect) {
    fill(buf, area, Style::default().fg(WHITE).bg(BLACK));
}

fn fill(buf: &mut Buffer, area: Rect, style: Style) {
    for y in area.top()..area.bottom() {
        fill_line(buf, area.x, y, area.width, style);
    }
}

fn fill_line(buf: &mut Buffer, x: u16, y: u16, width: u16, style: Style) {
    for col in x..x.saturating_add(width) {
        buf.get_mut(col, y).set_char(' ').set_style(style);
    }
}

fn draw_box(buf: &mut Buffer, area: Rect, style: Style) {
    if area.width < 2 || area.height < 2 {
        return;
    }

    let left = area.left();
    let right = area.right().saturating_sub(1);
    let top = area.top();
    let bottom = area.bottom().saturating_sub(1);

    buf.get_mut(left, top).set_char('╭').set_style(style);
    buf.get_mut(right, top).set_char('╮').set_style(style);
    buf.get_mut(left, bottom).set_char('╰').set_style(style);
    buf.get_mut(right, bottom).set_char('╯').set_style(style);

    for x in left.saturating_add(1)..right {
        buf.get_mut(x, top).set_char('─').set_style(style);
        buf.get_mut(x, bottom).set_char('─').set_style(style);
    }
    for y in top.saturating_add(1)..bottom {
        buf.get_mut(left, y).set_char('│').set_style(style);
        buf.get_mut(right, y).set_char('│').set_style(style);
    }
}

fn centered_text(buf: &mut Buffer, area: Rect, y: u16, text: &str, style: Style) {
    if y >= area.bottom() || area.width == 0 {
        return;
    }

    let text = truncate_to_width(text, area.width as usize);
    let width = text.chars().count() as u16;
    let x = area.x + area.width.saturating_sub(width) / 2;
    write_clipped(buf, x, y, width.min(area.width), &text, style);
}

fn write_segments(buf: &mut Buffer, x: u16, y: u16, width: u16, segments: &[(String, Style)]) {
    let end = x.saturating_add(width);
    let mut cursor = x;
    for (text, style) in segments {
        if cursor >= end {
            break;
        }
        let written = write_clipped(buf, cursor, y, end - cursor, text, *style);
        cursor = cursor.saturating_add(written);
    }
}

fn write_clipped(buf: &mut Buffer, x: u16, y: u16, width: u16, text: &str, style: Style) -> u16 {
    if width == 0 {
        return 0;
    }

    let text = truncate_to_width(text, width as usize);
    let mut written = 0;
    for (idx, ch) in text.chars().enumerate() {
        let col = x + idx as u16;
        if col >= x.saturating_add(width) {
            break;
        }
        buf.get_mut(col, y).set_char(ch).set_style(style);
        written += 1;
    }
    written
}

fn truncate_to_width(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }

    let len = text.chars().count();
    if len <= width {
        return text.to_string();
    }

    if width == 1 {
        return "…".to_string();
    }

    text.chars().take(width - 1).collect::<String>() + "…"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::config::{AppData, Config};
    use crate::service::{ServiceKind, ServiceMetadata};
    use ratatui::{backend::TestBackend, Terminal};

    fn sample_services() -> Vec<LocalService> {
        vec![
            LocalService {
                pid: 48213,
                port: 3000,
                address: "127.0.0.1".to_string(),
                process_name: "node".to_string(),
                command: "node server.js".to_string(),
                kind: ServiceKind::Node,
                metadata: ServiceMetadata::default(),
            },
            LocalService {
                pid: 47102,
                port: 5173,
                address: "127.0.0.1".to_string(),
                process_name: "node".to_string(),
                command: "npm run dev".to_string(),
                kind: ServiceKind::Vite,
                metadata: ServiceMetadata {
                    title: Some("Checkout frontend".to_string()),
                    memo: Some("Vite app for checkout edits".to_string()),
                    tags: vec!["vite".to_string()],
                    source: Some("codex".to_string()),
                    ..ServiceMetadata::default()
                },
            },
            LocalService {
                pid: 46711,
                port: 8000,
                address: "127.0.0.1".to_string(),
                process_name: "python3".to_string(),
                command: "uvicorn app:main".to_string(),
                kind: ServiceKind::Api,
                metadata: ServiceMetadata {
                    title: Some("API server".to_string()),
                    url_path: Some("/docs".to_string()),
                    ..ServiceMetadata::default()
                },
            },
        ]
    }

    fn sample_app() -> App {
        let mut app = App::new(Config::default(), AppData::default(), false);
        app.phase = Phase::Main;
        app.services = sample_services();
        app.selected = 1;
        app
    }

    fn render_to_text(app: &App, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("terminal");
        terminal.draw(|frame| render(frame, app)).expect("draw");
        let buffer = terminal.backend().buffer();
        let mut out = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                out.push_str(buffer.get(x, y).symbol());
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn main_screen_renders_columned_register() {
        let text = render_to_text(&sample_app(), 100, 30);
        println!("{text}");
        assert!(text.contains("PORT   PID       SERVICE"));
        assert!(text.contains("5173"));
        assert!(text.contains("Checkout frontend"));
        assert!(text.contains("@codex"));
        assert!(text.contains("→ /docs"));
        assert!(text.contains("? help"));
        assert!(text.contains(&format!("HOSTEL {PRODUCT_VERSION}")));
    }

    #[test]
    fn kind_label_is_skipped_when_it_repeats_the_service_name() {
        let text = render_to_text(&sample_app(), 100, 30);
        assert!(!text.contains("node  Node"));
        assert!(text.contains("node server.js"));
    }

    #[test]
    fn classic_80x24_terminal_gets_columns_without_command() {
        let text = render_to_text(&sample_app(), 80, 24);
        println!("{text}");
        assert!(text.contains("PORT   PID       SERVICE"));
        assert!(text.contains("Checkout frontend"));
        assert!(!text.contains("npm run dev"));
    }

    #[test]
    fn selected_row_is_reverse_video_with_visible_hierarchy() {
        let app = sample_app();
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).expect("terminal");
        terminal.draw(|frame| render(frame, &app)).expect("draw");
        let buffer = terminal.backend().buffer();

        let mut selected_row_y = None;
        for y in 0..buffer.area.height {
            let line: String = (0..buffer.area.width)
                .map(|x| buffer.get(x, y).symbol())
                .collect::<Vec<_>>()
                .join("");
            if line.contains("Checkout frontend") {
                selected_row_y = Some(y);
            }
        }
        let y = selected_row_y.expect("selected row rendered");
        let cell = buffer
            .content
            .iter()
            .enumerate()
            .find(|(idx, cell)| (*idx as u16) / buffer.area.width == y && cell.symbol() == "C")
            .map(|(_, cell)| cell)
            .expect("title cell");
        assert_eq!(cell.bg, WHITE);
        assert_eq!(cell.fg, BLACK);
    }

    #[test]
    fn empty_register_invites_action() {
        let mut app = sample_app();
        app.services.clear();
        let text = render_to_text(&app, 100, 30);
        assert!(text.contains("No localhost services on ports 1024-9999"));
        assert!(text.contains("start a dev server, then press r to refresh"));
    }

    #[test]
    fn memo_editor_shows_context_and_saves_hint() {
        let mut app = sample_app();
        app.phase = Phase::MemoEditor {
            service_key: app.services[1].metadata_key(),
            original: String::new(),
            text: "Vite app".to_string(),
        };
        let text = render_to_text(&app, 100, 30);
        println!("{text}");
        assert!(text.contains(" Memo "));
        assert!(text.contains("node · localhost:5173"));
        assert!(text.contains("8/100"));
        assert!(text.contains("Enter save   Esc cancel   empty clears"));
    }

    #[test]
    fn confirm_kill_states_the_safety_pipeline() {
        let mut app = sample_app();
        app.phase = Phase::ConfirmKill {
            service: app.services[1].clone(),
        };
        let text = render_to_text(&app, 100, 30);
        println!("{text}");
        assert!(text.contains("Kill node on localhost:5173?"));
        assert!(text.contains("rescans and verifies pid, then sends SIGTERM"));
        assert!(text.contains("Enter confirm   Esc cancel"));
    }

    #[test]
    fn help_overlay_lists_all_keys_for_vim_mode() {
        let mut app = sample_app();
        app.config.keybind_mode = KeybindMode::Vim;
        app.phase = Phase::Help;
        let text = render_to_text(&app, 100, 34);
        println!("{text}");
        assert!(text.contains("HOSTEL keys"));
        assert!(text.contains("j/k  select"));
        assert!(text.contains("K  kill"));
        assert!(text.contains("any key closes"));
    }
}
