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
const DIM: Color = Color::Rgb(120, 120, 120);
const MID: Color = Color::Rgb(170, 170, 170);

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
            render_memo_editor(buf, area, text);
        }
        Phase::TitleEditor { text, .. } => {
            render_main(buf, area, app);
            render_title_editor(buf, area, text);
        }
        Phase::TagsEditor { text, .. } => {
            render_main(buf, area, app);
            render_tags_editor(buf, area, text);
        }
        Phase::FilterEditor { text, .. } => {
            render_main(buf, area, app);
            render_filter_editor(buf, area, text);
        }
        Phase::UrlEditor { text, .. } => {
            render_main(buf, area, app);
            render_url_editor(buf, area, text);
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
    draw_box(buf, area, Style::default().fg(DIM).bg(BLACK));
    let inner = inner(area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let header = if inner.width >= 56 {
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

    if app.services.is_empty() {
        centered_text(
            buf,
            inner,
            inner.y + inner.height / 2,
            "No localhost services on ports 1024-9999",
            Style::default().fg(DIM).bg(BLACK),
        );
        return;
    }

    let mut y = inner.y.saturating_add(2);
    for (index, service) in app.services.iter().enumerate().skip(app.scroll) {
        if y >= inner.bottom() {
            break;
        }

        let selected = index == app.selected;
        render_service_row(buf, inner, y, service, selected);
        y += 1;

        if let Some(memo) = &service.metadata.memo {
            if y >= inner.bottom() {
                break;
            }
            render_memo_subtitle(buf, inner, y, memo, selected);
            y += 1;
        }
    }
}

fn render_service_row(
    buf: &mut Buffer,
    inner: Rect,
    y: u16,
    service: &LocalService,
    selected: bool,
) {
    let style = selected_style(selected);
    fill_line(buf, inner.x, y, inner.width, style);

    if inner.width >= 56 {
        let port = format!("{:<6}", service.port);
        let pid = format!("{:<8}", service.pid);
        write_clipped(buf, inner.x + 1, y, 6, &port, style);
        write_clipped(buf, inner.x + 8, y, 8, &pid, style);
        let mut service_text = service.display_title().to_string();
        if service.metadata.title.is_some() {
            service_text.push_str("  ");
            service_text.push_str(service.display_name());
        }
        if let Some(label) = service.kind.label() {
            service_text.push_str("  ");
            service_text.push_str(label);
        }
        if !service.metadata.tags.is_empty() {
            service_text.push_str("  [");
            service_text.push_str(&service.metadata.tags.join(","));
            service_text.push(']');
        }
        if let Some(path) = &service.metadata.url_path {
            service_text.push_str("  -> ");
            service_text.push_str(path);
        }
        if let Some(source) = &service.metadata.source {
            service_text.push_str("  @");
            service_text.push_str(source);
        }
        if inner.width >= 78 && !service.command.trim().is_empty() {
            service_text.push_str("  ");
            service_text.push_str(&service.command);
        }
        write_clipped(
            buf,
            inner.x + 18,
            y,
            inner.width.saturating_sub(19),
            &service_text,
            style,
        );
    } else {
        let row = format!("{:<6} {}", service.port, service.display_title());
        write_clipped(
            buf,
            inner.x + 1,
            y,
            inner.width.saturating_sub(2),
            &row,
            style,
        );
    }
}

fn render_memo_subtitle(buf: &mut Buffer, inner: Rect, y: u16, memo: &str, selected: bool) {
    let style = if selected {
        Style::default().fg(BLACK).bg(WHITE)
    } else {
        Style::default().fg(DIM).bg(BLACK)
    };
    fill_line(buf, inner.x, y, inner.width, style);
    write_clipped(
        buf,
        inner.x + 8,
        y,
        inner.width.saturating_sub(9),
        memo,
        style,
    );
}

fn render_footer(buf: &mut Buffer, area: Rect, y: u16, app: &App) {
    let keys = match app.config.keybind_mode {
        KeybindMode::Regular => {
            "↑↓ select   Enter open   k kill   t title   m memo   g tags   u url   f filter   r refresh   q quit"
        }
        KeybindMode::Vim => {
            "j/k select   Enter open   K kill   t title   m memo   g tags   u url   f filter   r refresh   q quit"
        }
    };

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
        centered_text(buf, area, y, keys, Style::default().fg(DIM).bg(BLACK));
    }

    centered_text(
        buf,
        area,
        y.saturating_add(1),
        &format!("v{PRODUCT_VERSION}"),
        Style::default().fg(Color::Rgb(70, 70, 70)).bg(BLACK),
    );
}

fn render_memo_editor(buf: &mut Buffer, area: Rect, text: &str) {
    let width = area.width.saturating_sub(4).clamp(32, 72);
    let height = 7.min(area.height.saturating_sub(2)).max(5);
    let rect = centered_rect(area, width, height);
    fill(buf, rect, Style::default().fg(WHITE).bg(BLACK));
    draw_box(buf, rect, Style::default().fg(WHITE).bg(BLACK));
    write_clipped(
        buf,
        rect.x + 2,
        rect.y,
        rect.width.saturating_sub(4),
        " Memo ",
        Style::default()
            .fg(WHITE)
            .bg(BLACK)
            .add_modifier(Modifier::BOLD),
    );

    let counter = format!("{}/{}", text.chars().count(), MEMO_LIMIT);
    write_clipped(
        buf,
        rect.x
            + rect
                .width
                .saturating_sub(counter.chars().count() as u16 + 2),
        rect.y,
        counter.chars().count() as u16,
        &counter,
        Style::default().fg(DIM).bg(BLACK),
    );

    write_clipped(
        buf,
        rect.x + 2,
        rect.y + 2,
        rect.width.saturating_sub(4),
        text,
        Style::default().fg(WHITE).bg(BLACK),
    );
    write_clipped(
        buf,
        rect.x + 2,
        rect.bottom().saturating_sub(2),
        rect.width.saturating_sub(4),
        "Enter save   Esc cancel",
        Style::default().fg(DIM).bg(BLACK),
    );
}

fn render_title_editor(buf: &mut Buffer, area: Rect, text: &str) {
    let width = area.width.saturating_sub(4).clamp(32, 72);
    let height = 7.min(area.height.saturating_sub(2)).max(5);
    let rect = centered_rect(area, width, height);
    fill(buf, rect, Style::default().fg(WHITE).bg(BLACK));
    draw_box(buf, rect, Style::default().fg(WHITE).bg(BLACK));
    write_clipped(
        buf,
        rect.x + 2,
        rect.y,
        rect.width.saturating_sub(4),
        " Title ",
        Style::default()
            .fg(WHITE)
            .bg(BLACK)
            .add_modifier(Modifier::BOLD),
    );

    let counter = format!("{}/{}", text.chars().count(), TITLE_LIMIT);
    write_clipped(
        buf,
        rect.x
            + rect
                .width
                .saturating_sub(counter.chars().count() as u16 + 2),
        rect.y,
        counter.chars().count() as u16,
        &counter,
        Style::default().fg(DIM).bg(BLACK),
    );

    write_clipped(
        buf,
        rect.x + 2,
        rect.y + 2,
        rect.width.saturating_sub(4),
        text,
        Style::default().fg(WHITE).bg(BLACK),
    );
    write_clipped(
        buf,
        rect.x + 2,
        rect.bottom().saturating_sub(2),
        rect.width.saturating_sub(4),
        "Enter save   Esc cancel   empty clears",
        Style::default().fg(DIM).bg(BLACK),
    );
}

fn render_tags_editor(buf: &mut Buffer, area: Rect, text: &str) {
    let width = area.width.saturating_sub(4).clamp(38, 76);
    let height = 8.min(area.height.saturating_sub(2)).max(6);
    let rect = centered_rect(area, width, height);
    fill(buf, rect, Style::default().fg(WHITE).bg(BLACK));
    draw_box(buf, rect, Style::default().fg(WHITE).bg(BLACK));
    write_clipped(
        buf,
        rect.x + 2,
        rect.y,
        rect.width.saturating_sub(4),
        " Tags ",
        Style::default()
            .fg(WHITE)
            .bg(BLACK)
            .add_modifier(Modifier::BOLD),
    );
    write_clipped(
        buf,
        rect.x + 2,
        rect.y + 2,
        rect.width.saturating_sub(4),
        "Comma-separated tags",
        Style::default().fg(DIM).bg(BLACK),
    );
    write_clipped(
        buf,
        rect.x + 2,
        rect.y + 3,
        rect.width.saturating_sub(4),
        text,
        Style::default().fg(WHITE).bg(BLACK),
    );
    write_clipped(
        buf,
        rect.x + 2,
        rect.bottom().saturating_sub(2),
        rect.width.saturating_sub(4),
        "Enter save   Esc cancel   empty clears",
        Style::default().fg(DIM).bg(BLACK),
    );
}

fn render_filter_editor(buf: &mut Buffer, area: Rect, text: &str) {
    let width = area.width.saturating_sub(4).clamp(38, 76);
    let height = 8.min(area.height.saturating_sub(2)).max(6);
    let rect = centered_rect(area, width, height);
    fill(buf, rect, Style::default().fg(WHITE).bg(BLACK));
    draw_box(buf, rect, Style::default().fg(WHITE).bg(BLACK));
    write_clipped(
        buf,
        rect.x + 2,
        rect.y,
        rect.width.saturating_sub(4),
        " Hide filters ",
        Style::default()
            .fg(WHITE)
            .bg(BLACK)
            .add_modifier(Modifier::BOLD),
    );
    write_clipped(
        buf,
        rect.x + 2,
        rect.y + 2,
        rect.width.saturating_sub(4),
        "Comma-separated keywords",
        Style::default().fg(DIM).bg(BLACK),
    );
    write_clipped(
        buf,
        rect.x + 2,
        rect.y + 3,
        rect.width.saturating_sub(4),
        text,
        Style::default().fg(WHITE).bg(BLACK),
    );
    write_clipped(
        buf,
        rect.x + 2,
        rect.bottom().saturating_sub(2),
        rect.width.saturating_sub(4),
        "Enter save   Esc cancel   empty clears",
        Style::default().fg(DIM).bg(BLACK),
    );
}

fn render_url_editor(buf: &mut Buffer, area: Rect, text: &str) {
    let width = area.width.saturating_sub(4).clamp(38, 76);
    let height = 8.min(area.height.saturating_sub(2)).max(6);
    let rect = centered_rect(area, width, height);
    fill(buf, rect, Style::default().fg(WHITE).bg(BLACK));
    draw_box(buf, rect, Style::default().fg(WHITE).bg(BLACK));
    write_clipped(
        buf,
        rect.x + 2,
        rect.y,
        rect.width.saturating_sub(4),
        " Open path ",
        Style::default()
            .fg(WHITE)
            .bg(BLACK)
            .add_modifier(Modifier::BOLD),
    );

    let counter = format!(
        "{}/{}",
        text.chars().count(),
        crate::service::URL_PATH_LIMIT
    );
    write_clipped(
        buf,
        rect.x
            + rect
                .width
                .saturating_sub(counter.chars().count() as u16 + 2),
        rect.y,
        counter.chars().count() as u16,
        &counter,
        Style::default().fg(DIM).bg(BLACK),
    );

    write_clipped(
        buf,
        rect.x + 2,
        rect.y + 2,
        rect.width.saturating_sub(4),
        "Path appended to localhost, like /docs",
        Style::default().fg(DIM).bg(BLACK),
    );
    write_clipped(
        buf,
        rect.x + 2,
        rect.y + 3,
        rect.width.saturating_sub(4),
        text,
        Style::default().fg(WHITE).bg(BLACK),
    );
    write_clipped(
        buf,
        rect.x + 2,
        rect.bottom().saturating_sub(2),
        rect.width.saturating_sub(4),
        "Enter save   Esc cancel   empty clears",
        Style::default().fg(DIM).bg(BLACK),
    );
}

fn render_confirm_kill(buf: &mut Buffer, area: Rect, service: &LocalService) {
    let width = area.width.saturating_sub(4).clamp(34, 58);
    let height = 8.min(area.height.saturating_sub(2)).max(6);
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
        rect.bottom().saturating_sub(2),
        "Enter confirm   Esc cancel",
        Style::default().fg(DIM).bg(BLACK),
    );
}

fn render_help(buf: &mut Buffer, area: Rect, mode: KeybindMode) {
    let width = area.width.saturating_sub(4).clamp(34, 58);
    let height = 13.min(area.height.saturating_sub(2)).max(8);
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
    let lines = match mode {
        KeybindMode::Regular => [
            "↑↓ move",
            "Enter open",
            "k kill",
            "t title",
            "m memo",
            "g tags",
            "u url",
            "f filter",
            "r refresh",
            "q quit",
        ],
        KeybindMode::Vim => [
            "j/k move",
            "Enter open",
            "K kill",
            "t title",
            "m memo",
            "g tags",
            "u url",
            "f filter",
            "r refresh",
            "q quit",
        ],
    };
    for (idx, line) in lines.iter().enumerate() {
        centered_text(
            buf,
            rect,
            rect.y + 3 + idx as u16,
            line,
            Style::default().fg(MID).bg(BLACK),
        );
    }
}

fn render_centered_logo(buf: &mut Buffer, area: Rect, top: u16, color: Color) -> u16 {
    let lines = logo_lines(area);
    let height = lines.len() as u16;
    for (idx, line) in lines.iter().enumerate() {
        centered_text(
            buf,
            area,
            top + idx as u16,
            line,
            Style::default()
                .fg(color)
                .bg(BLACK)
                .add_modifier(Modifier::BOLD),
        );
    }
    height
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

fn selected_style(selected: bool) -> Style {
    if selected {
        Style::default().fg(BLACK).bg(WHITE)
    } else {
        Style::default().fg(WHITE).bg(BLACK)
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

fn write_clipped(buf: &mut Buffer, x: u16, y: u16, width: u16, text: &str, style: Style) {
    if width == 0 {
        return;
    }

    let text = truncate_to_width(text, width as usize);
    for (idx, ch) in text.chars().enumerate() {
        let col = x + idx as u16;
        if col >= x.saturating_add(width) {
            break;
        }
        buf.get_mut(col, y).set_char(ch).set_style(style);
    }
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
