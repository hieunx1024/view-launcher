use ratatui::{
    Frame,
    layout::{Layout, Constraint, Direction, Alignment},
    widgets::{Block, Borders, BorderType, Paragraph, List, ListItem},
    style::{Style, Modifier, Color},
    text::{Line, Span},
};
use crate::launcher::LauncherItem;
use crate::config::{ThemeConfig, parse_color};
use crate::icons;

pub struct UiState<'a> {
    pub input: &'a str,
    pub cursor_pos: usize,
    pub results: &'a [(LauncherItem, Vec<usize>)],
    pub selected_index: usize,
    pub theme: &'a ThemeConfig,
}

pub fn draw(f: &mut Frame, state: &UiState) {
    let show_icons = state.theme.show_icons.unwrap_or(true);
    let show_status_bar = state.theme.show_status_bar.unwrap_or(true);
    let border_color = parse_color(&state.theme.border_color);
    let query_color = parse_color(&state.theme.query_color);
    let selection_bg = parse_color(&state.theme.selection_bg);
    let selection_fg = parse_color(&state.theme.selection_fg);
    let highlight_color = parse_color(&state.theme.highlight_color);

    // 1. Outer Block Frame
    let title_text = if show_icons { " 󰍉 VIEW LAUNCHER " } else { " VIEW LAUNCHER " };
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(title_text, Style::default().fg(query_color).add_modifier(Modifier::BOLD)))
        .title_alignment(Alignment::Center)
        .title_bottom(Line::from(format!("  {} results  ", state.results.len())).alignment(Alignment::Right));

    let area = outer_block.inner(f.size());
    f.render_widget(outer_block, f.size());

    // 2. Divide layout: Search Box (3) + Results List (Min 1) + Status Bar (1 optional)
    let mut constraints = vec![
        Constraint::Length(3), // Input area
        Constraint::Min(1),    // List area
    ];
    if show_status_bar {
        constraints.push(Constraint::Length(1)); // Status bar
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    // 3. Draw Input box with proper cursor handling
    let input_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(border_color).add_modifier(Modifier::DIM));

    // Handle cursor position based on char indices
    let chars: Vec<char> = state.input.chars().collect();
    let char_count = chars.len();
    let safe_cursor_pos = state.cursor_pos.min(char_count);

    let before_cursor: String = chars[..safe_cursor_pos].iter().collect();
    let cursor_char = if safe_cursor_pos < char_count {
        chars[safe_cursor_pos].to_string()
    } else {
        " ".to_string()
    };
    let after_cursor: String = if safe_cursor_pos < char_count {
        chars[safe_cursor_pos + 1..].iter().collect()
    } else {
        String::new()
    };

    let mut input_spans = vec![
        Span::styled(" ❯ ", Style::default().fg(query_color).add_modifier(Modifier::BOLD)),
        Span::styled(before_cursor, Style::default().fg(Color::White)),
    ];

    // Highlight cursor block
    if safe_cursor_pos < char_count {
        input_spans.push(Span::styled(
            cursor_char,
            Style::default().bg(query_color).fg(Color::Black).add_modifier(Modifier::BOLD),
        ));
    } else {
        input_spans.push(Span::styled(
            "▮",
            Style::default().fg(query_color).add_modifier(Modifier::SLOW_BLINK),
        ));
    }

    input_spans.push(Span::styled(after_cursor, Style::default().fg(Color::White)));

    let input_paragraph = Paragraph::new(Line::from(input_spans)).block(input_block);
    f.render_widget(input_paragraph, chunks[0]);

    // 4. Draw Results List
    let visible_height = chunks[1].height as usize;
    if state.results.is_empty() {
        let no_results = Paragraph::new(Line::from(vec![
            Span::styled("  No applications, files, or calculations found.", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
        ])).alignment(Alignment::Left);
        f.render_widget(no_results, chunks[1]);
    } else {
        // Scroll mechanism
        let start_idx = if state.selected_index >= visible_height {
            state.selected_index - visible_height + 1
        } else {
            0
        };

        let items: Vec<ListItem> = state.results
            .iter()
            .enumerate()
            .skip(start_idx)
            .take(visible_height)
            .map(|(idx, (item, matched_indices))| {
                let is_selected = idx == state.selected_index;
                
                // Choose icon / badge
                let (icon_text, icon_category) = icons::get_icon(&item.item_type, &item.name, &item.exec_or_path, show_icons);
                let badge_color = match icon_category {
                    "calc" => parse_color(&state.theme.calc_badge_color),
                    "dir" => parse_color(&state.theme.dir_badge_color),
                    "file" | "file_code" | "file_config" | "file_text" | "file_pdf" | "file_image" | "file_media" | "file_archive" => parse_color(&state.theme.file_badge_color),
                    _ => parse_color(&state.theme.app_badge_color),
                };

                let mut spans = vec![
                    Span::styled(format!(" {}", icon_text), Style::default().fg(badge_color).add_modifier(Modifier::BOLD)),
                ];

                // Build highlighted name
                let name_chars: Vec<char> = item.name.chars().collect();
                let mut current_segment = String::new();
                let mut is_current_highlighted = false;

                for (c_idx, &c) in name_chars.iter().enumerate() {
                    let is_highlighted = matched_indices.contains(&c_idx);
                    if is_highlighted != is_current_highlighted {
                        if !current_segment.is_empty() {
                            let style = if is_current_highlighted {
                                Style::default().fg(highlight_color).add_modifier(Modifier::BOLD)
                            } else if is_selected {
                                Style::default().fg(selection_fg)
                            } else {
                                Style::default().fg(Color::White)
                            };
                            spans.push(Span::styled(current_segment.clone(), style));
                            current_segment.clear();
                        }
                        is_current_highlighted = is_highlighted;
                    }
                    current_segment.push(c);
                }

                if !current_segment.is_empty() {
                    let style = if is_current_highlighted {
                        Style::default().fg(highlight_color).add_modifier(Modifier::BOLD)
                    } else if is_selected {
                        Style::default().fg(selection_fg)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    spans.push(Span::styled(current_segment, style));
                }

                // Append description or path
                if let Some(desc) = &item.description {
                    if !desc.is_empty() {
                        spans.push(Span::styled("  —  ", Style::default().fg(Color::DarkGray)));
                        spans.push(Span::styled(
                            if desc.len() > 60 { format!("{}...", &desc[..57]) } else { desc.to_string() },
                            Style::default().fg(Color::DarkGray)
                        ));
                    }
                }

                let style = if is_selected {
                    Style::default().bg(selection_bg).fg(selection_fg).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                ListItem::new(Line::from(spans)).style(style)
            })
            .collect();

        let list = List::new(items)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));
            
        f.render_widget(list, chunks[1]);
    }

    // 5. Draw Footer Status Bar
    if show_status_bar && chunks.len() > 2 {
        let status_line = Line::from(vec![
            Span::styled(" [↵] ", Style::default().fg(query_color).add_modifier(Modifier::BOLD)),
            Span::styled("Open  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Tab] ", Style::default().fg(query_color).add_modifier(Modifier::BOLD)),
            Span::styled("Complete  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Alt+T] ", Style::default().fg(query_color).add_modifier(Modifier::BOLD)),
            Span::styled("Terminal  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Alt+C] ", Style::default().fg(query_color).add_modifier(Modifier::BOLD)),
            Span::styled("Copy  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Esc] ", Style::default().fg(query_color).add_modifier(Modifier::BOLD)),
            Span::styled("Quit", Style::default().fg(Color::DarkGray)),
        ]).alignment(Alignment::Center);

        let status_bar = Paragraph::new(status_line);
        f.render_widget(status_bar, chunks[2]);
    }
}
