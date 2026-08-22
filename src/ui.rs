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

    let trimmed_input = state.input.trim();
    let is_file_mode = trimmed_input.starts_with("@f") || trimmed_input.starts_with("@file");

    // 1. Outer Frame with rounded styling and elegant dynamic title header
    let (title_icon, title_label, title_color) = if is_file_mode {
        (if show_icons { "󰉋 " } else { "" }, "FILE SEARCH", Color::Rgb(125, 207, 255))
    } else {
        (if show_icons { "󰀻 " } else { "" }, "VIEW LAUNCHER", query_color)
    };

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            format!(" {}{} ", title_icon, title_label),
            Style::default().fg(title_color).add_modifier(Modifier::BOLD)
        ))
        .title_alignment(Alignment::Center)
        .title_bottom(
            Line::from(vec![
                Span::styled(
                    format!("  {} {}  ", state.results.len(), if is_file_mode { "files" } else { "apps" }),
                    Style::default().fg(Color::Rgb(122, 162, 247)).add_modifier(Modifier::BOLD)
                )
            ]).alignment(Alignment::Right)
        );

    let area = outer_block.inner(f.size());
    f.render_widget(outer_block, f.size());

    // 2. Layout division: Search input (3 rows), list items, bottom status pill (1 row)
    let mut constraints = vec![
        Constraint::Length(3), // Search box
        Constraint::Min(1),    // Result list
    ];
    if show_status_bar {
        constraints.push(Constraint::Length(1)); // Status bar
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    // 3. Search Box Input
    let input_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(border_color));

    let chars: Vec<char> = state.input.chars().collect();
    let char_count = chars.len();
    let safe_cursor_pos = state.cursor_pos.min(char_count);

    let input_line = if state.input.is_empty() {
        // Placeholder state
        Line::from(vec![
            Span::styled("  󰍉  ", Style::default().fg(query_color).add_modifier(Modifier::BOLD)),
            Span::styled(
                "Search apps, type @f for files, or 500*12 to calculate...",
                Style::default().fg(Color::Rgb(86, 95, 137)).add_modifier(Modifier::ITALIC)
            ),
            Span::styled("▌", Style::default().fg(query_color)),
        ])
    } else {
        let (lens_icon, lens_color) = if is_file_mode {
            ("  󰉋  ", Color::Rgb(125, 207, 255))
        } else {
            ("  󰍉  ", query_color)
        };

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

        let mut spans = vec![
            Span::styled(lens_icon, Style::default().fg(lens_color).add_modifier(Modifier::BOLD)),
            Span::styled(before_cursor, Style::default().fg(Color::Rgb(240, 246, 252)).add_modifier(Modifier::BOLD)),
        ];

        // Cursor representation
        if safe_cursor_pos < char_count {
            spans.push(Span::styled(
                cursor_char,
                Style::default().bg(query_color).fg(Color::Black).add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                "▌",
                Style::default().fg(query_color),
            ));
        }

        spans.push(Span::styled(after_cursor, Style::default().fg(Color::Rgb(240, 246, 252)).add_modifier(Modifier::BOLD)));
        Line::from(spans)
    };

    let input_paragraph = Paragraph::new(input_line).block(input_block);
    f.render_widget(input_paragraph, chunks[0]);

    // 4. Results List
    let visible_height = chunks[1].height as usize;
    if state.results.is_empty() {
        let no_results = Paragraph::new(Line::from(vec![
            Span::styled(
                "   󰋔  No matching applications, files, or calculations found.",
                Style::default().fg(Color::Rgb(86, 95, 137)).add_modifier(Modifier::ITALIC)
            ),
        ])).alignment(Alignment::Left);
        f.render_widget(no_results, chunks[1]);
    } else {
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
                
                // Dynamic semantic category icon & coloring
                let (icon_text, icon_category) = icons::get_icon(
                    &item.item_type,
                    &item.name,
                    &item.exec_or_path,
                    item.icon.as_deref(),
                    show_icons,
                );

                let badge_color = match icon_category {
                    "app_code" => Color::Rgb(125, 207, 255),       // Bright Cyan
                    "app_db" => Color::Rgb(158, 206, 106),         // Emerald Green
                    "app_browser" => Color::Rgb(255, 158, 100),    // Orange/Amber
                    "app_chat" => Color::Rgb(122, 162, 247),       // Soft Blue
                    "app_media" => Color::Rgb(187, 154, 247),      // Violet/Purple
                    "app_graphics" => Color::Rgb(247, 118, 142),   // Rose/Pink
                    "app_office" => Color::Rgb(42, 195, 222),      // Teal
                    "app_system" => Color::Rgb(224, 175, 104),     // Warm Gold
                    "app_settings" => Color::Rgb(122, 162, 247),   // Blue
                    "app_security" => Color::Rgb(224, 175, 104),   // Gold
                    "app_game" => Color::Rgb(187, 154, 247),       // Violet
                    "app_calc" | "calc" => Color::Rgb(158, 206, 106), // Lime Green
                    "dir" => Color::Rgb(122, 162, 247),            // Folder Blue
                    "file_code" => Color::Rgb(125, 207, 255),      // Cyan
                    "file_image" => Color::Rgb(247, 118, 142),     // Pink
                    "file_media" => Color::Rgb(187, 154, 247),     // Purple
                    "file_archive" => Color::Rgb(224, 175, 104),   // Gold
                    _ => Color::Rgb(122, 162, 247),                // Tokyo Night Blue
                };

                // Left selection bar indicator
                let indicator = if is_selected { " ▎ " } else { "   " };
                let indicator_style = if is_selected {
                    Style::default().fg(Color::Rgb(125, 207, 255)).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let mut spans = vec![
                    Span::styled(indicator, indicator_style),
                    Span::styled(format!("{} ", icon_text), Style::default().fg(badge_color).add_modifier(Modifier::BOLD)),
                ];

                // Build highlighted item name
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
                                Style::default().fg(selection_fg).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Color::Rgb(192, 202, 245))
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
                        Style::default().fg(selection_fg).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Rgb(192, 202, 245))
                    };
                    spans.push(Span::styled(current_segment, style));
                }

                // Append secondary description or path
                if let Some(desc) = &item.description {
                    if !desc.is_empty() {
                        let sep_style = if is_selected {
                            Style::default().fg(Color::Rgb(122, 162, 247))
                        } else {
                            Style::default().fg(Color::Rgb(86, 95, 137))
                        };
                        let desc_style = if is_selected {
                            Style::default().fg(Color::Rgb(169, 177, 214))
                        } else {
                            Style::default().fg(Color::Rgb(86, 95, 137))
                        };

                        spans.push(Span::styled("  ·  ", sep_style));
                        spans.push(Span::styled(
                            if desc.len() > 65 { format!("{}...", &desc[..62]) } else { desc.to_string() },
                            desc_style
                        ));
                    }
                }

                let row_style = if is_selected {
                    Style::default().bg(selection_bg)
                } else {
                    Style::default()
                };

                ListItem::new(Line::from(spans)).style(row_style)
            })
            .collect();

        let list = List::new(items);
        f.render_widget(list, chunks[1]);
    }

    // 5. Clean, modern Footer Status Bar
    if show_status_bar && chunks.len() > 2 {
        let key_style = Style::default().fg(Color::Rgb(122, 162, 247)).add_modifier(Modifier::BOLD);
        let label_style = Style::default().fg(Color::Rgb(108, 112, 134));
        let sep_style = Style::default().fg(Color::Rgb(65, 72, 104));

        let status_line = Line::from(vec![
            Span::styled(" [↵] ", key_style),
            Span::styled("Open", label_style),
            Span::styled("  │ ", sep_style),
            Span::styled("[Tab] ", key_style),
            Span::styled("Complete", label_style),
            Span::styled("  │ ", sep_style),
            Span::styled("[Alt+T] ", key_style),
            Span::styled("Terminal", label_style),
            Span::styled("  │ ", sep_style),
            Span::styled("[Alt+C] ", key_style),
            Span::styled("Copy", label_style),
            Span::styled("  │ ", sep_style),
            Span::styled("[Esc] ", key_style),
            Span::styled("Quit", label_style),
        ]).alignment(Alignment::Center);

        let status_bar = Paragraph::new(status_line);
        f.render_widget(status_bar, chunks[2]);
    }
}
