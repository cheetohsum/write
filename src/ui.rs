use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{AppState, LlmStatus, RevealStyle, Screen};
use crate::theme;

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
        Screen::Settings => render_settings(f, state, area),
        Screen::Editor => render_editor(f, state, area),
        Screen::QuitConfirm => {
            render_editor(f, state, area);
            render_quit_modal(f, area);
        }
    }

    // Startup reveal animation
    if matches!(state.screen, Screen::Startup | Screen::Settings) {
        if let Some(ref anim) = state.startup_anim {
            let elapsed = anim.start.elapsed().as_millis() as u64;
            if elapsed < 900 {
                apply_reveal_overlay(f.buffer_mut(), area, elapsed, 900, &RevealStyle::TopDown);
            }
        }
    }

    // Screen transition overlay
    if let Some(ref trans) = state.transition {
        let elapsed = trans.start.elapsed().as_millis() as u64;
        if elapsed < 350 {
            apply_reveal_overlay(f.buffer_mut(), area, elapsed, 350, &trans.style);
        }
    }

    // CRT post-processing — always last
    apply_crt_effect(f.buffer_mut(), area);
}

fn render_startup(f: &mut Frame, state: &mut AppState, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(20),
        Constraint::Min(1),
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

    let deco_width = form_area.width as usize;
    let top_line = "━".repeat(deco_width);
    f.render_widget(
        Paragraph::new(Span::styled(&top_line, theme::decorative_line())),
        form_chunks[0],
    );

    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            "write",
            Style::default()
                .fg(theme::MAROON)
                .bg(theme::PARCHMENT)
                .add_modifier(Modifier::BOLD),
        )]))
        .alignment(Alignment::Center),
        form_chunks[2],
    );

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("── ", theme::decorative_line_subtle()),
            Span::styled("a writing tool", theme::secondary()),
            Span::styled(" ──", theme::decorative_line_subtle()),
        ]))
        .alignment(Alignment::Center),
        form_chunks[3],
    );

    let mid_line = "─".repeat(deco_width);
    f.render_widget(
        Paragraph::new(Span::styled(&mid_line, theme::decorative_line_subtle())),
        form_chunks[5],
    );

    f.render_widget(
        Paragraph::new(Span::styled("  Output Directory", theme::label())),
        form_chunks[7],
    );

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

    f.render_widget(
        Paragraph::new(Span::styled("  Document Title", theme::label())),
        form_chunks[9],
    );

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

    f.render_widget(
        Paragraph::new(Span::styled(&mid_line, theme::decorative_line_subtle())),
        form_chunks[12],
    );

    let provider_text = match &state.llm_config {
        Some(cfg) => cfg.display(),
        None => "no API key".to_string(),
    };

    let key_style = Style::default().fg(theme::MAROON).bg(theme::PARCHMENT);
    let settings_style = if state.startup_field == 2 {
        Style::default()
            .fg(theme::CREAM)
            .bg(theme::TERRACOTTA)
            .add_modifier(Modifier::BOLD)
    } else {
        theme::hint()
    };

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  ", theme::base()),
            Span::styled(&provider_text, theme::hint()),
            Span::styled("  ", theme::base()),
            Span::styled("tab", key_style),
            Span::styled(" · ", theme::hint()),
            Span::styled("enter", key_style),
            Span::styled(" · ", theme::hint()),
            Span::styled(" ⚙ settings ", settings_style),
            Span::styled(" · ", theme::hint()),
            Span::styled("esc", key_style),
        ])),
        form_chunks[14],
    );
}

fn render_settings(f: &mut Frame, state: &mut AppState, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(22),
        Constraint::Min(1),
    ])
    .split(area);

    let form_area = centered_rect(55, chunks[1]);
    let deco_width = form_area.width as usize;

    let form_chunks = Layout::vertical([
        Constraint::Length(1), // [0] top decorative line
        Constraint::Length(1), // [1] blank
        Constraint::Length(1), // [2] "settings" title
        Constraint::Length(1), // [3] subtitle
        Constraint::Length(1), // [4] blank
        Constraint::Length(1), // [5] mid decorative line
        Constraint::Length(1), // [6] Anthropic label + link
        Constraint::Length(3), // [7] Anthropic input
        Constraint::Length(1), // [8] OpenAI label + link
        Constraint::Length(3), // [9] OpenAI input
        Constraint::Length(1), // [10] OpenRouter label + link
        Constraint::Length(3), // [11] OpenRouter input
        Constraint::Length(1), // [12] blank
        Constraint::Length(1), // [13] bottom decorative line
        Constraint::Length(1), // [14] blank
        Constraint::Length(1), // [15] hints bar
    ])
    .split(form_area);

    // Top decorative line
    let top_line = "━".repeat(deco_width);
    f.render_widget(
        Paragraph::new(Span::styled(&top_line, theme::decorative_line())),
        form_chunks[0],
    );

    // Title
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            "settings",
            Style::default()
                .fg(theme::MAROON)
                .bg(theme::PARCHMENT)
                .add_modifier(Modifier::BOLD),
        )]))
        .alignment(Alignment::Center),
        form_chunks[2],
    );

    // Subtitle
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("── ", theme::decorative_line_subtle()),
            Span::styled("API keys", theme::secondary()),
            Span::styled(" ──", theme::decorative_line_subtle()),
        ]))
        .alignment(Alignment::Center),
        form_chunks[3],
    );

    // Mid decorative line
    let mid_line = "─".repeat(deco_width);
    f.render_widget(
        Paragraph::new(Span::styled(&mid_line, theme::decorative_line_subtle())),
        form_chunks[5],
    );

    // Provider fields
    let label_indices = [6, 8, 10];
    let input_indices = [7, 9, 11];
    let link_style = Style::default()
        .fg(theme::TERRACOTTA)
        .bg(theme::PARCHMENT)
        .add_modifier(Modifier::UNDERLINED);

    for i in 0..3 {
        let label_chunk = form_chunks[label_indices[i]];
        let input_chunk = form_chunks[input_indices[i]];

        // Label row: split into label (left) and link (right)
        let link_text = crate::config::PROVIDER_URLS[i];
        let link_display = format!("{} ↗", link_text);
        let link_width = link_display.chars().count() as u16 + 1; // +1 for padding

        let label_link_chunks = Layout::horizontal([
            Constraint::Min(1),
            Constraint::Length(link_width),
        ])
        .split(label_chunk);

        // Label
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("  {}", crate::config::PROVIDER_NAMES[i]),
                theme::label(),
            )),
            label_link_chunks[0],
        );

        // Link button
        f.render_widget(
            Paragraph::new(Span::styled(&link_display, link_style)),
            label_link_chunks[1],
        );

        // Store link rect for mouse click handling
        state.settings_link_rects[i] = label_link_chunks[1];

        // Input field
        let is_active = state.settings_field == i as u8;
        let border_style = if is_active {
            theme::border_active()
        } else {
            theme::border()
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style)
            .style(theme::base());

        if is_active {
            state.settings_inputs[i].set_block(block);
            state.settings_inputs[i].set_cursor_style(theme::cursor());
            state.settings_inputs[i].set_style(theme::input_active());
        } else {
            state.settings_inputs[i].set_block(block);
            state.settings_inputs[i]
                .set_cursor_style(ratatui::style::Style::default());
            state.settings_inputs[i].set_style(theme::input_inactive());
        }
        f.render_widget(&state.settings_inputs[i], input_chunk);
    }

    // Bottom decorative line
    f.render_widget(
        Paragraph::new(Span::styled(&mid_line, theme::decorative_line_subtle())),
        form_chunks[13],
    );

    // Hints bar
    let key_style = Style::default().fg(theme::MAROON).bg(theme::PARCHMENT);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  ", theme::base()),
            Span::styled("tab", key_style),
            Span::styled(" switch  ", theme::hint()),
            Span::styled("^O", key_style),
            Span::styled(" open link  ", theme::hint()),
            Span::styled("esc", key_style),
            Span::styled(" back", theme::hint()),
        ])),
        form_chunks[15],
    );
}

fn render_editor(f: &mut Frame, state: &mut AppState, area: Rect) {
    // Dark border color for frame edges
    let edge = Style::default().fg(theme::WALNUT).bg(theme::WALNUT);

    let chunks = Layout::vertical([
        Constraint::Length(1), // top dark edge
        Constraint::Length(1), // title bar padding above
        Constraint::Length(1), // title bar
        Constraint::Length(1), // title bar padding below / editor gap
        Constraint::Min(1),   // editor
        Constraint::Length(1), // info bar padding above
        Constraint::Length(1), // info bar
        Constraint::Length(1), // command bar
        Constraint::Length(1), // bottom dark edge
    ])
    .split(area);

    // --- Dark top edge ---
    f.render_widget(Block::default().style(edge), chunks[0]);
    // --- Dark bottom edge ---
    f.render_widget(Block::default().style(edge), chunks[8]);

    // --- Padding rows ---
    f.render_widget(Block::default().style(theme::title_bar()), chunks[1]); // above title
    f.render_widget(Block::default().style(theme::base()), chunks[3]);      // below title
    f.render_widget(Block::default().style(theme::base()), chunks[5]);      // above info bar

    // --- Centered title bar with icon ---
    let breadcrumb = state.breadcrumb();
    let icon_color = if state.llm_enabled { theme::GOLD } else { theme::MAROON };
    let title_line = Line::from(vec![
        Span::styled(
            "✧ ",
            Style::default().fg(icon_color).bg(theme::UMBER),
        ),
        Span::styled(
            "write",
            Style::default()
                .fg(theme::GOLD)
                .bg(theme::UMBER)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(theme::CLAY).bg(theme::UMBER)),
        Span::styled(&breadcrumb, theme::title_bar()),
        if state.editor.modified {
            Span::styled(
                "  ●",
                Style::default()
                    .fg(theme::MAROON)
                    .bg(theme::UMBER)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled("", theme::title_bar())
        },
    ]);
    f.render_widget(
        Paragraph::new(title_line)
            .style(theme::title_bar())
            .alignment(Alignment::Center),
        chunks[2],
    );

    // --- Editor with dark side borders and padding ---
    let editor_outer = Layout::horizontal([
        Constraint::Length(1), // left dark edge
        Constraint::Length(4), // left padding
        Constraint::Min(1),   // editor
        Constraint::Length(4), // right padding
        Constraint::Length(1), // right dark edge
    ])
    .split(chunks[4]);

    // Store editor area for mouse click mapping
    state.editor_area = editor_outer[2];

    // Dark side edges
    for y in editor_outer[0].top()..editor_outer[0].bottom() {
        let cell = &mut f.buffer_mut()[(editor_outer[0].left(), y)];
        cell.set_char('▏');
        cell.set_fg(theme::WALNUT);
        cell.set_bg(theme::PARCHMENT);
    }
    for y in editor_outer[4].top()..editor_outer[4].bottom() {
        let cell = &mut f.buffer_mut()[(editor_outer[4].left(), y)];
        cell.set_char('▕');
        cell.set_fg(theme::WALNUT);
        cell.set_bg(theme::PARCHMENT);
    }

    // Padding fill
    let pad_bg = Block::default().style(theme::base());
    f.render_widget(pad_bg.clone(), editor_outer[1]);
    f.render_widget(pad_bg, editor_outer[3]);

    // Editor textarea
    f.render_widget(&state.editor.textarea, editor_outer[2]);

    // Markdown rich text styling
    style_markdown(f.buffer_mut(), editor_outer[2]);

    // Per-character dissolve on LLM-changed positions
    if let Some(ref dissolve) = state.text_dissolve {
        let elapsed = dissolve.start.elapsed().as_millis() as u64;
        if elapsed < 450 {
            let area = editor_outer[2];
            // Estimate scroll offset (same as mouse click logic)
            let (cur_row, _) = state.editor.textarea.cursor();
            let viewport_h = area.height as usize;
            let scroll_top = cur_row.saturating_sub(viewport_h.saturating_sub(1));

            for (i, &(doc_row, doc_col)) in dissolve.changed_positions.iter().enumerate() {
                // Map document position to screen position
                if doc_row < scroll_top {
                    continue;
                }
                let screen_row = doc_row - scroll_top;
                if screen_row >= viewport_h {
                    continue;
                }
                let sx = area.left() + doc_col as u16;
                let sy = area.top() + screen_row as u16;
                if sx >= area.right() || sy >= area.bottom() {
                    continue;
                }

                // Only overlay if this position hasn't resolved yet
                if elapsed < dissolve.resolve_times[i] {
                    let cell = &mut f.buffer_mut()[(sx, sy)];
                    let ch_hash = hash_position(sx, sy)
                        .wrapping_add(elapsed / 40);
                    let dither_ch = DITHER_CHARS[(ch_hash as usize) % DITHER_CHARS.len()];
                    let dither_fg = DITHER_COLORS[((ch_hash >> 16) as usize) % DITHER_COLORS.len()];
                    cell.set_char(dither_ch);
                    cell.set_fg(dither_fg);
                }
            }
        }
    }

    // --- Info bar: word count + LLM status with flash dissolve ---
    let llm_status_text = match state.llm_status {
        LlmStatus::Disabled => "off",
        LlmStatus::Idle => "",
        LlmStatus::Waiting => "waiting",
        LlmStatus::Cleaning => "cleaning...",
        LlmStatus::Applied => "applied ✓",
        LlmStatus::Error => "error",
        LlmStatus::Off => "paused",
    };

    let llm_icon_color = match state.llm_status {
        LlmStatus::Applied => theme::SAGE,
        LlmStatus::Cleaning => theme::GOLD,
        LlmStatus::Error => theme::MAROON,
        LlmStatus::Disabled | LlmStatus::Off => theme::CLAY,
        _ => theme::SANDSTONE,
    };

    // Flash: show status for 1.5s then fade to just the icon
    let show_status_text = if let Some(flash_start) = state.llm_status_flash {
        flash_start.elapsed().as_millis() < 1500
    } else {
        false
    };

    let save_span = if state.just_saved {
        Span::styled(
            " ✓ saved ",
            Style::default()
                .fg(theme::SAGE)
                .bg(theme::UMBER)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled("", theme::status_bar())
    };

    let mut info_spans = vec![
        save_span,
        Span::styled(
            "  ✦ ",
            Style::default().fg(theme::GOLD).bg(theme::UMBER),
        ),
        Span::styled(
            format!("{}", state.editor.word_count()),
            theme::status_bar(),
        ),
        Span::styled("   ", theme::status_bar()),
        Span::styled(
            "⟡ ",
            Style::default().fg(llm_icon_color).bg(theme::UMBER),
        ),
    ];
    if show_status_text && !llm_status_text.is_empty() {
        info_spans.push(Span::styled(llm_status_text, theme::status_bar()));
    }

    f.render_widget(
        Paragraph::new(Line::from(info_spans))
            .style(theme::status_bar())
            .alignment(Alignment::Center),
        chunks[6],
    );

    // --- Command bar ---
    let cmd_style = Style::default().fg(theme::SANDSTONE).bg(theme::UMBER);
    let key_style = Style::default()
        .fg(theme::CREAM)
        .bg(theme::UMBER)
        .add_modifier(Modifier::BOLD);

    let mut cmd_spans = vec![
        Span::styled("^S", key_style),
        Span::styled(" save  ", cmd_style),
        Span::styled("^G", key_style),
        Span::styled(" link  ", cmd_style),
        Span::styled("^O", key_style),
        Span::styled(" open  ", cmd_style),
        Span::styled("^A", key_style),
        Span::styled(" all  ", cmd_style),
        Span::styled("^L", key_style),
        Span::styled(" llm  ", cmd_style),
        Span::styled("Esc", key_style),
    ];
    if state.is_nested() {
        cmd_spans.push(Span::styled(" back  ", cmd_style));
    } else {
        cmd_spans.push(Span::styled(" back  ", cmd_style));
        cmd_spans.push(Span::styled("^Q", key_style));
        cmd_spans.push(Span::styled(" quit", cmd_style));
    }

    f.render_widget(
        Paragraph::new(Line::from(cmd_spans))
            .style(theme::status_bar())
            .alignment(Alignment::Center),
        chunks[7],
    );

    // Apply dither dissolve to status text when flash is fading out
    if let Some(flash_start) = state.llm_status_flash {
        let elapsed = flash_start.elapsed().as_millis() as u64;
        if elapsed >= 1000 && elapsed < 1500 {
            // Dissolve the status text area over 500ms
            let fade_progress = (elapsed - 1000) as f64 / 500.0;
            let info_area = chunks[6];
            for x in info_area.left()..info_area.right() {
                let cell_hash = hash_position(x, info_area.top());
                let threshold = (cell_hash >> 33) as f64 / (u32::MAX as f64);
                if fade_progress > threshold {
                    let cell = &mut f.buffer_mut()[(x, info_area.top())];
                    let sym = cell.symbol().chars().next().unwrap_or(' ');
                    if sym.is_alphabetic() || sym == '.' || sym == '✓' {
                        cell.set_char(' ');
                    }
                }
            }
        }
    }
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

    f.render_widget(
        Paragraph::new("You have unsaved changes.")
            .style(theme::modal_bg())
            .alignment(Alignment::Center),
        msg_chunks[0],
    );

    f.render_widget(
        Paragraph::new(Line::from(vec![
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
        .wrap(Wrap { trim: false }),
        msg_chunks[2],
    );
}

// --- Markdown rich text + [[wiki-link]] styling ---

fn style_markdown(buf: &mut Buffer, area: Rect) {
    for y in area.top()..area.bottom() {
        // Extract line content from buffer
        let width = (area.right() - area.left()) as usize;
        let chars: Vec<char> = (0..width)
            .map(|i| {
                let x = area.left() + i as u16;
                buf[(x, y)].symbol().chars().next().unwrap_or(' ')
            })
            .collect();
        let line: String = chars.iter().collect();
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();

        // Headers: # ## ###
        if trimmed.starts_with("### ") || trimmed.starts_with("#### ") {
            style_line(buf, area, y, indent, theme::GOLD, true);
            hide_range(buf, area, y, indent, indent + trimmed.find(' ').unwrap_or(0));
            continue;
        }
        if trimmed.starts_with("## ") {
            style_line(buf, area, y, indent, theme::TERRACOTTA, true);
            hide_range(buf, area, y, indent, indent + 2);
            continue;
        }
        if trimmed.starts_with("# ") {
            style_line(buf, area, y, indent, theme::MAROON, true);
            hide_range(buf, area, y, indent, indent + 1);
            continue;
        }

        // Blockquote: >
        if trimmed.starts_with("> ") {
            style_line(buf, area, y, indent, theme::CLAY, false);
            // Color the > marker in gold
            let gx = area.left() + indent as u16;
            if gx < area.right() {
                buf[(gx, y)].set_fg(theme::GOLD);
            }
            // Still process inline formatting below
        }

        // List bullets: - or *  (but * not when used for emphasis)
        if trimmed.starts_with("- ") {
            let bx = area.left() + indent as u16;
            if bx < area.right() {
                buf[(bx, y)].set_fg(theme::TERRACOTTA);
                buf[(bx, y)].modifier.insert(Modifier::BOLD);
            }
        }

        // Inline formatting pass
        let mut used = vec![false; width];

        // Wiki-links: [[...]]
        {
            let mut i = 0;
            while i + 3 < width {
                if chars[i] == '[' && chars[i + 1] == '[' && !used[i] {
                    if let Some(end) = find_closing(&chars, i + 2, ']', ']') {
                        // Dim brackets
                        for b in [i, i + 1, end, end + 1] {
                            set_fg(buf, area, y, b, theme::SANDSTONE);
                            used[b] = true;
                        }
                        // Bold maroon content
                        for c in (i + 2)..end {
                            set_fg(buf, area, y, c, theme::MAROON);
                            set_bold(buf, area, y, c);
                            used[c] = true;
                        }
                        i = end + 2;
                        continue;
                    }
                }
                i += 1;
            }
        }

        // Bold: **...**
        {
            let mut i = 0;
            while i + 3 < width {
                if chars[i] == '*' && chars[i + 1] == '*' && !used[i] {
                    if let Some(end) = find_closing(&chars, i + 2, '*', '*') {
                        // Hide ** markers
                        for b in [i, i + 1, end, end + 1] {
                            hide_char(buf, area, y, b);
                            used[b] = true;
                        }
                        // Bold content
                        for c in (i + 2)..end {
                            if !used[c] {
                                set_bold(buf, area, y, c);
                                used[c] = true;
                            }
                        }
                        i = end + 2;
                        continue;
                    }
                }
                i += 1;
            }
        }

        // Italic: *...* (single, not already consumed by bold)
        {
            let mut i = 0;
            while i + 1 < width {
                if chars[i] == '*' && !used[i] {
                    // Find closing single *
                    let mut j = i + 1;
                    while j < width {
                        if chars[j] == '*' && !used[j] {
                            hide_char(buf, area, y, i);
                            hide_char(buf, area, y, j);
                            for c in (i + 1)..j {
                                if !used[c] {
                                    set_italic(buf, area, y, c);
                                }
                            }
                            i = j + 1;
                            break;
                        }
                        j += 1;
                    }
                    if j >= width {
                        i += 1;
                    }
                } else {
                    i += 1;
                }
            }
        }

        // Inline code: `...`
        {
            let mut i = 0;
            while i + 1 < width {
                if chars[i] == '`' && !used[i] {
                    let mut j = i + 1;
                    while j < width {
                        if chars[j] == '`' && !used[j] {
                            hide_char(buf, area, y, i);
                            hide_char(buf, area, y, j);
                            for c in (i + 1)..j {
                                if !used[c] {
                                    set_fg(buf, area, y, c, theme::CLAY);
                                }
                            }
                            i = j + 1;
                            break;
                        }
                        j += 1;
                    }
                    if j >= width {
                        i += 1;
                    }
                } else {
                    i += 1;
                }
            }
        }
    }
}

fn find_closing(chars: &[char], from: usize, c1: char, c2: char) -> Option<usize> {
    let mut i = from;
    while i + 1 < chars.len() {
        if chars[i] == c1 && chars[i + 1] == c2 {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn hide_char(buf: &mut Buffer, area: Rect, y: u16, col: usize) {
    let x = area.left() + col as u16;
    if x < area.right() {
        buf[(x, y)].set_char(' ');
        buf[(x, y)].set_fg(theme::PARCHMENT);
    }
}

fn set_fg(buf: &mut Buffer, area: Rect, y: u16, col: usize, color: Color) {
    let x = area.left() + col as u16;
    if x < area.right() {
        buf[(x, y)].set_fg(color);
    }
}

fn set_bold(buf: &mut Buffer, area: Rect, y: u16, col: usize) {
    let x = area.left() + col as u16;
    if x < area.right() {
        buf[(x, y)].modifier.insert(Modifier::BOLD);
    }
}

fn set_italic(buf: &mut Buffer, area: Rect, y: u16, col: usize) {
    let x = area.left() + col as u16;
    if x < area.right() {
        buf[(x, y)].modifier.insert(Modifier::ITALIC);
    }
}

fn style_line(buf: &mut Buffer, area: Rect, y: u16, from: usize, color: Color, bold: bool) {
    for i in from..area.width as usize {
        let x = area.left() + i as u16;
        if x < area.right() {
            buf[(x, y)].set_fg(color);
            if bold {
                buf[(x, y)].modifier.insert(Modifier::BOLD);
            }
        }
    }
}

fn hide_range(buf: &mut Buffer, area: Rect, y: u16, from: usize, to: usize) {
    for i in from..=to {
        hide_char(buf, area, y, i);
    }
}

// --- CRT post-processing effect ---

fn apply_crt_effect(buf: &mut Buffer, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let w = area.width as f64;
    let h = area.height as f64;
    let cx = w / 2.0;
    let cy = h / 2.0;
    let max_dist = (cx * cx + cy * cy).sqrt();

    for y in area.top()..area.bottom() {
        let row_idx = y - area.top();
        let y_norm = (y - area.top()) as f64 / h;

        for x in area.left()..area.right() {
            let x_norm = (x - area.left()) as f64 / w;

            // Scan lines: dim every other row, stronger at edges
            let edge_boost = 1.0 + (y_norm - 0.5).abs() * 0.08;
            let scanline = if row_idx % 2 == 1 {
                0.92 / edge_boost
            } else {
                1.0
            };

            // Vignette: darkening toward edges and corners
            let dx = (x - area.left()) as f64 - cx;
            let dy = (y - area.top()) as f64 - cy;
            let dist = (dx * dx + dy * dy).sqrt() / max_dist;
            let vignette = 1.0 - (dist * dist * 0.35);

            // Chromatic aberration: smooth gradient across full width
            // Red channel shifts right (stronger on left), blue shifts left (stronger on right)
            // Center of screen = no shift, edges = max shift
            let x_off = x_norm - 0.5; // -0.5 to +0.5
            let aberration = x_off * x_off * 4.0; // 0 at center, 1 at edges
            // Left side: boost red, reduce blue. Right side: boost blue, reduce red.
            let r_shift = 1.0 + x_off * 0.14; // left=0.93, center=1.0, right=1.07
            let g_shift = 1.0 - aberration * 0.03; // slight green reduction at edges
            let b_shift = 1.0 - x_off * 0.14; // left=1.07, center=1.0, right=0.93

            let factor = scanline * vignette;

            let cell = &mut buf[(x, y)];

            // Foreground
            if let Color::Rgb(r, g, b) = cell.fg {
                cell.set_fg(Color::Rgb(
                    ((r as f64) * factor * r_shift).clamp(0.0, 255.0) as u8,
                    ((g as f64) * factor * g_shift).clamp(0.0, 255.0) as u8,
                    ((b as f64) * factor * b_shift).clamp(0.0, 255.0) as u8,
                ));
            }

            // Background
            if let Color::Rgb(r, g, b) = cell.bg {
                let bg_v = 1.0 - (dist * dist * 0.18);
                cell.set_bg(Color::Rgb(
                    ((r as f64) * bg_v * r_shift).clamp(0.0, 255.0) as u8,
                    ((g as f64) * bg_v * g_shift).clamp(0.0, 255.0) as u8,
                    ((b as f64) * bg_v * b_shift).clamp(0.0, 255.0) as u8,
                ));
            }
        }
    }
}

// --- Dither / reveal overlay ---

fn apply_reveal_overlay(
    buf: &mut Buffer,
    area: Rect,
    elapsed_ms: u64,
    total_ms: u64,
    style: &RevealStyle,
) {
    let progress = elapsed_ms as f64 / total_ms as f64;
    let cx = area.width as f64 / 2.0;
    let cy = area.height as f64 / 2.0;
    let max_dist = (cx * cx + cy * cy).sqrt();

    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell_hash = hash_position(x, y);
            let random_part = (cell_hash >> 33) as f64 / (u32::MAX as f64);

            let threshold = match style {
                RevealStyle::TopDown => {
                    let y_ratio = if area.height > 0 {
                        (y - area.top()) as f64 / area.height as f64
                    } else {
                        0.0
                    };
                    y_ratio * 0.55 + random_part * 0.45
                }
                RevealStyle::Scatter => random_part,
                RevealStyle::ZoomIn => {
                    // Center resolves first, edges last
                    let dx = (x - area.left()) as f64 - cx;
                    let dy = (y - area.top()) as f64 - cy;
                    let dist = (dx * dx + dy * dy).sqrt();
                    let dist_ratio = if max_dist > 0.0 {
                        dist / max_dist
                    } else {
                        0.0
                    };
                    dist_ratio * 0.6 + random_part * 0.4
                }
                RevealStyle::ZoomOut => {
                    // Edges resolve first, center last
                    let dx = (x - area.left()) as f64 - cx;
                    let dy = (y - area.top()) as f64 - cy;
                    let dist = (dx * dx + dy * dy).sqrt();
                    let dist_ratio = if max_dist > 0.0 {
                        1.0 - dist / max_dist
                    } else {
                        0.0
                    };
                    dist_ratio * 0.6 + random_part * 0.4
                }
            };

            if progress < threshold {
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
