use ratatui::{
    Frame,
    layout::{Layout, Constraint, Direction, Alignment},
    widgets::{Block, Borders, BorderType, Paragraph, List, ListItem},
    style::{Style, Modifier, Color},
    text::{Line, Span},
};
use crate::launcher::{LauncherItem, ItemType};

pub struct UiState<'a> {
    pub input: &'a str,
    pub results: &'a [LauncherItem],
    pub selected_index: usize,
    pub query_color: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub app_badge_color: Color,
    pub file_badge_color: Color,
    pub border_color: Color,
}

pub fn draw(f: &mut Frame, state: &UiState) {
    // 1. Create layout: Entire screen is bounded by a thin block border.
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(state.border_color))
        .title(Span::styled(" VIEW LAUNCHER ", Style::default().fg(state.query_color).add_modifier(Modifier::BOLD)))
        .title_alignment(Alignment::Center)
        .title_bottom(Line::from(format!("  {} results  ", state.results.len())).alignment(Alignment::Right));

    let area = outer_block.inner(f.size());
    f.render_widget(outer_block, f.size());

    // Divide inner area: Search box at top (1 row + border/padding), remainder is results list
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Input area
            Constraint::Min(1),    // List area
        ])
        .split(area);

    // 2. Draw Input box
    let input_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(state.border_color).add_modifier(Modifier::DIM));

    let input_line = Line::from(vec![
        Span::styled(" ❯ ", Style::default().fg(state.query_color).add_modifier(Modifier::BOLD)),
        Span::styled(state.input, Style::default().fg(Color::White)),
        // Subtle active blinking cursor representation
        Span::styled("▮", Style::default().fg(state.query_color).add_modifier(Modifier::SLOW_BLINK)),
    ]);

    let input_paragraph = Paragraph::new(input_line)
        .block(input_block);
    f.render_widget(input_paragraph, chunks[0]);

    // 3. Draw Results List
    let visible_height = chunks[1].height as usize;
    if state.results.is_empty() {
        let no_results = Paragraph::new(Line::from(vec![
            Span::styled("  No applications or files found.", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
        ])).alignment(Alignment::Left);
        f.render_widget(no_results, chunks[1]);
        return;
    }

    // Scroll mechanism: ensure selected item is always visible
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
        .map(|(idx, item)| {
            let is_selected = idx == state.selected_index;
            
            // Choose badge based on type
            let (badge_text, badge_color) = match item.item_type {
                ItemType::App => (" [App] ", state.app_badge_color),
                ItemType::File => (" [File] ", state.file_badge_color),
                ItemType::Dir => (" [Dir]  ", Color::Green),
            };

            let mut spans = vec![
                Span::styled(badge_text, Style::default().fg(badge_color).add_modifier(Modifier::BOLD)),
                Span::styled(" ", Style::default()),
                Span::styled(&item.name, Style::default().fg(if is_selected { state.selection_fg } else { Color::White })),
            ];

            // Append description/comment if available
            if let Some(desc) = &item.description {
                if !desc.is_empty() {
                    spans.push(Span::styled("  —  ", Style::default().fg(Color::DarkGray)));
                    spans.push(Span::styled(
                        if desc.len() > 50 { format!("{}...", &desc[..47]) } else { desc.to_string() },
                        Style::default().fg(Color::DarkGray)
                    ));
                }
            }

            let style = if is_selected {
                Style::default().bg(state.selection_bg).fg(state.selection_fg).add_modifier(Modifier::BOLD)
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
