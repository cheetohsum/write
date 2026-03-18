use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{AppState, LlmStatus, Screen};
use crate::theme;

// Dither noise characters — blocks + dots for a textured dissolve
const DITHER_CHARS: &[char] = &['░', '▒', '▓', '·', ':', '∷', '─', '┄'];
const DITHER_COLORS: &[Color] = &[
    theme::SANDSTONE,
    theme::CLAY,
    theme::WHEAT,
    theme::GOLD,
    theme::MAROON,
];

pub fn render(f: &mut Frame, state: &mut AppState) {
    let area = f.area();
    let bg = Block::default().style(theme::base());
    f.render_widget(bg, area);

    match state.screen {
        Screen::Startup => render_startup(f, state, area),
        Screen::Editor => render_editor(f, state, area),
        Screen::QuitConfirm => {
            render_editor(f, state, area);
            render_quit_modal(f, area);
        }
    }

    // Overlay: startup reveal animation (noise → content)
    if state.screen == Screen::Startup {
        if let Some(ref anim) = state.startup_anim {
            let elapsed = anim.start.elapsed().as_millis() as u64;
            if elapsed < 900 {
                apply_reveal_overlay(f.buffer_mut(), area, elapsed, 900, true);
            }
        }
    }

    // Overlay: screen transition (content → noise → content)
    if let Some(ref trans) = state.transition {
        let elapsed = trans.start.elapsed().as_millis() as u64;
        if elapsed < 350 {
            apply_reveal_overlay(f.buffer_mut(), area, elapsed, 350, false);
        }
    }
}

fn render_startup(f: &mut Frame, state: &mut AppState, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Min(1),      // top padding
        Constraint::Length(20),  // form area
        Constraint::Min(1),      // bottom padding
    ])
    .split(area);

    let form_area = centered_rect(55, chunks[1]);

    let form_chunks = Layout::vertical([
        Constraint::Length(1), // decorative top line
        Constraint::Length(1), // blank
        Constraint::Length(1), // title "write"
        Constraint::Length(1), // subtitle
        Constraint::Length(1), // blank
        Constraint::Length(1), // decorative mid line
        Constraint::Length(1), // blank
        Constraint::Length(1), // dir label
        Constraint::Length(3), // dir input
        Constraint::Length(1), // title label
        Constraint::Length(3), // title input
        Constraint::Length(1), // blank
        Constraint::Length(1), // decorative bottom line
        Constraint::Length(1), // blank
        Constraint::Length(1), // provider + keybindings
    ])
    .split(form_area);

    // Decorative top line ═══
    let deco_width = form_area.width as usize;
    let top_line = "━".repeat(deco_width);
    let deco_top = Paragraph::new(Span::styled(&top_line, theme::decorative_line()));
    f.render_widget(deco_top, form_chunks[0]);

    // App title
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "write",
            Style::default()
                .fg(theme::MAROON)
                .bg(theme::PARCHMENT)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(title, form_chunks[2]);

    // Subtitle
    let subtitle = Paragraph::new(Line::from(vec![
        Span::styled("── ", theme::decorative_line_subtle()),
        Span::styled("a writing tool", theme::secondary()),
        Span::styled(" ──", theme::decorative_line_subtle()),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(subtitle, form_chunks[3]);

    // Decorative mid line
    let mid_line = "─".repeat(deco_width);
    let deco_mid = Paragraph::new(Span::styled(&mid_line, theme::decorative_line_subtle()));
    f.render_widget(deco_mid, form_chunks[5]);

    // Directory label
    let dir_label = Paragraph::new(Span::styled("  Output Directory", theme::label()));
    f.render_widget(dir_label, form_chunks[7]);

    // Directory input
    let dir_border_style = if state.startup_field == 0 {
        theme::border_active()
    } else {
        theme::border()
    };
    let dir_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(dir_border_style)
        .style(theme::base());

    if state.startup_field == 0 {
        state.dir_input.set_block(dir_block);
        state.dir_input.set_cursor_style(theme::cursor());
        state.dir_input.set_style(theme::input_active());
    } else {
        state.dir_input.set_block(dir_block);
        state
            .dir_input
            .set_cursor_style(ratatui::style::Style::default());
        state.dir_input.set_style(theme::input_inactive());
    }
    f.render_widget(&state.dir_input, form_chunks[8]);

    // Title label
    let title_label = Paragraph::new(Span::styled("  Document Title", theme::label()));
    f.render_widget(title_label, form_chunks[9]);

    // Title input
    let title_border_style = if state.startup_field == 1 {
        theme::border_active()
    } else {
        theme::border()
    };
    let title_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(title_border_style)
        .style(theme::base());

    if state.startup_field == 1 {
        state.title_input.set_block(title_block);
        state.title_input.set_cursor_style(theme::cursor());
        state.title_input.set_style(theme::input_active());
    } else {
        state.title_input.set_block(title_block);
        state
            .title_input
            .set_cursor_style(ratatui::style::Style::default());
        state.title_input.set_style(theme::input_inactive());
    }
    f.render_widget(&state.title_input, form_chunks[10]);

    // Decorative bottom line
    let deco_bot = Paragraph::new(Span::styled(&mid_line, theme::decorative_line_subtle()));
    f.render_widget(deco_bot, form_chunks[12]);

    // Provider info + keybindings on same line
    let provider_text = match &state.llm_config {
        Some(cfg) => cfg.display(),
        None => "no API key".to_string(),
    };

    let bottom_line = Line::from(vec![
        Span::styled("  ", theme::base()),
        Span::styled(&provider_text, theme::hint()),
        Span::styled("    ", theme::base()),
        Span::styled("tab", Style::default().fg(theme::MAROON).bg(theme::PARCHMENT)),
        Span::styled(" · ", theme::hint()),
        Span::styled(
            "enter",
            Style::default().fg(theme::MAROON).bg(theme::PARCHMENT),
        ),
        Span::styled(" · ", theme::hint()),
        Span::styled("esc", Style::default().fg(theme::MAROON).bg(theme::PARCHMENT)),
    ]);
    f.render_widget(Paragraph::new(bottom_line), form_chunks[14]);
}

fn render_editor(f: &mut Frame, state: &mut AppState, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(1), // title bar
        Constraint::Min(1),   // editor
        Constraint::Length(1), // status bar
    ])
    .split(area);

    // Title bar — umber background with maroon dot for modified
    let modified_indicator = if state.editor.modified {
        Span::styled(
            "  ● ",
            Style::default()
                .fg(theme::MAROON)
                .bg(theme::UMBER)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled("    ", theme::title_bar())
    };

    let title_line = Line::from(vec![
        Span::styled(
            "  write ",
            Style::default()
                .fg(theme::GOLD)
                .bg(theme::UMBER)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ ", Style::default().fg(theme::CLAY).bg(theme::UMBER)),
        Span::styled(format!("{}.md", state.doc_title), theme::title_bar()),
        modified_indicator,
    ]);
    let title_bar = Paragraph::new(title_line).style(theme::title_bar());
    f.render_widget(title_bar, chunks[0]);

    // Editor with horizontal padding
    let editor_padded = Layout::horizontal([
        Constraint::Length(5),
        Constraint::Min(1),
        Constraint::Length(5),
    ])
    .split(chunks[1]);
    let pad_bg = Block::default().style(theme::base());
    f.render_widget(pad_bg.clone(), editor_padded[0]);
    f.render_widget(pad_bg, editor_padded[2]);
    f.render_widget(&state.editor.textarea, editor_padded[1]);

    // Status bar
    let llm_status_text = match state.llm_status {
        LlmStatus::Disabled => "LLM off",
        LlmStatus::Idle => "LLM idle",
        LlmStatus::Waiting => "LLM waiting",
        LlmStatus::Cleaning => "LLM cleaning...",
        LlmStatus::Applied => "LLM applied",
        LlmStatus::Error => "LLM error",
        LlmStatus::Off => "LLM paused",
    };

    let llm_dot_color = match state.llm_status {
        LlmStatus::Applied => theme::SAGE,
        LlmStatus::Cleaning => theme::GOLD,
        LlmStatus::Error => theme::MAROON,
        LlmStatus::Disabled | LlmStatus::Off => theme::CLAY,
        _ => theme::SANDSTONE,
    };

    let save_span = if state.just_saved {
        Span::styled(
            " saved ",
            Style::default()
                .fg(theme::SAGE)
                .bg(theme::UMBER)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled("  ", theme::status_bar())
    };

    let status_line = Line::from(vec![
        save_span,
        Span::styled(
            format!(" {} words ", state.editor.word_count()),
            theme::status_bar(),
        ),
        Span::styled("│ ", Style::default().fg(theme::CLAY).bg(theme::UMBER)),
        Span::styled(
            "● ",
            Style::default().fg(llm_dot_color).bg(theme::UMBER),
        ),
        Span::styled(llm_status_text, theme::status_bar()),
        Span::styled(
            " │ ^S save  ^Q quit  ^L llm ",
            Style::default().fg(theme::CLAY).bg(theme::UMBER),
        ),
    ]);

    let status_bar = Paragraph::new(status_line).style(theme::status_bar());
    f.render_widget(status_bar, chunks[2]);
}

fn render_quit_modal(f: &mut Frame, area: Rect) {
    let modal_area = centered_rect_fixed(42, 7, area);
    f.render_widget(Clear, modal_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::MAROON).bg(theme::WHEAT))
        .title(" unsaved changes ")
        .title_style(
            Style::default()
                .fg(theme::MAROON)
                .bg(theme::WHEAT)
                .add_modifier(Modifier::BOLD),
        )
        .style(theme::modal_bg());
    f.render_widget(block, modal_area);

    let inner = modal_area.inner(Margin::new(2, 2));
    let msg_chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(inner);

    let msg = Paragraph::new("You have unsaved changes.")
        .style(theme::modal_bg())
        .alignment(Alignment::Center);
    f.render_widget(msg, msg_chunks[0]);

    let prompt = Paragraph::new(Line::from(vec![
        Span::styled("quit anyway? ", theme::modal_bg()),
        Span::styled(
            "[y]",
            Style::default()
                .fg(theme::MAROON)
                .bg(theme::WHEAT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("es  ", theme::modal_bg()),
        Span::styled(
            "[n]",
            Style::default()
                .fg(theme::MAROON)
                .bg(theme::WHEAT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("o", theme::modal_bg()),
    ]))
    .alignment(Alignment::Center)
    .wrap(Wrap { trim: false });
    f.render_widget(prompt, msg_chunks[2]);
}

// --- Dither / reveal overlay ---

fn apply_reveal_overlay(
    buf: &mut Buffer,
    area: Rect,
    elapsed_ms: u64,
    total_ms: u64,
    top_down: bool,
) {
    let progress = elapsed_ms as f64 / total_ms as f64;

    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let y_ratio = if area.height > 0 {
                (y - area.top()) as f64 / area.height as f64
            } else {
                0.0
            };

            // Hash for this cell — deterministic per position
            let cell_hash = hash_position(x, y);
            let random_part = (cell_hash >> 33) as f64 / (u32::MAX as f64);

            // Threshold: mix of directional sweep and randomness
            let threshold = if top_down {
                y_ratio * 0.55 + random_part * 0.45
            } else {
                // Scatter: fully random for transitions
                random_part
            };

            if progress < threshold {
                // This cell is still noise — overwrite with dither
                let ch = dither_char(cell_hash, elapsed_ms);
                let fg = dither_color(cell_hash, elapsed_ms);
                let cell = &mut buf[(x, y)];
                cell.set_char(ch);
                cell.set_fg(fg);
                cell.set_bg(theme::PARCHMENT);
            }
        }
    }
}

fn hash_position(x: u16, y: u16) -> u64 {
    let v = (x as u64)
        .wrapping_mul(2654435761)
        .wrapping_add((y as u64).wrapping_mul(40503));
    v.wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
}

fn dither_char(hash: u64, elapsed_ms: u64) -> char {
    let idx = ((hash.wrapping_add(elapsed_ms / 45)) as usize) % DITHER_CHARS.len();
    DITHER_CHARS[idx]
}

fn dither_color(hash: u64, elapsed_ms: u64) -> Color {
    let idx = ((hash >> 16).wrapping_add(elapsed_ms / 80) as usize) % DITHER_COLORS.len();
    DITHER_COLORS[idx]
}

// --- Layout helpers ---

fn centered_rect(percent_x: u16, area: Rect) -> Rect {
    let layout = Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(area);
    layout[1]
}

fn centered_rect_fixed(width: u16, height: u16, area: Rect) -> Rect {
    let vert = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(height),
        Constraint::Min(0),
    ])
    .split(area);
    let horiz = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(width),
        Constraint::Min(0),
    ])
    .split(vert[1]);
    horiz[1]
}
