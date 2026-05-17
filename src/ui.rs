use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
};
use serde_json::Value;

use crate::app::{App, Focus};

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Outer container with title bar
    let outer_block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" live_jq ")
        .title_bottom(Line::from(" Tab: focus | q: quit ").right_aligned());
    frame.render_widget(&outer_block, area);
    let inner_area = outer_block.inner(area);

    // Split layout: output (fill) and query (fixed height)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(app.query_area_height(inner_area.height)),
        ])
        .split(inner_area);

    let output_area = chunks[0];
    let query_area = chunks[1];

    // --- Output preview ---
    let output_is_focused = app.focus == Focus::OutputPreview;
    let output_style = if output_is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let (output_lines, total_lines) = if let Some(ref err) = app.error {
        (
            vec![Line::from(Span::styled(
                err.clone(),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ))],
            1,
        )
    } else if app.output.is_empty() {
        (
            vec![Line::from(Span::styled(
                "Type a jq query to see results".to_owned(),
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM),
            ))],
            1,
        )
    } else {
        let lines = colorize_json(&app.output);
        let count = lines.len();
        (lines, count)
    };

    let output_block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Output ")
        .border_style(output_style);
    let output_inner = output_block.inner(output_area);
    let output_paragraph =
        Paragraph::new(output_lines).scroll((app.output_scroll, 0)).block(output_block);
    frame.render_widget(output_paragraph, output_area);

    // Scrollbar for output
    let viewport_height = output_inner.height as usize;
    let scrollbar = Scrollbar::default()
        .orientation(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None);
    let mut scrollbar_state = ScrollbarState::default()
        .content_length(total_lines)
        .viewport_content_length(viewport_height)
        .position(app.output_scroll as usize);
    frame.render_stateful_widget(scrollbar, output_area, &mut scrollbar_state);

    // --- Query input ---
    let query_is_focused = app.focus == Focus::QueryInput;
    let query_style = if query_is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let query_block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Query ")
        .border_style(query_style);
    let query_inner = query_block.inner(query_area);
    let query_paragraph =
        Paragraph::new(format!("> {}", app.query_buf)).block(query_block);
    frame.render_widget(query_paragraph, query_area);

    if query_is_focused {
        let cursor_x = query_inner.x + 2 + app.query_buf.len() as u16;
        let cursor_y = query_inner.y;
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}

fn colorize_json(text: &str) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    let mut buf = String::new();
    for line in text.lines() {
        buf.push_str(line);
        buf.push('\n');
        if let Ok(val) = serde_json::from_str::<Value>(buf.trim()) {
            let pretty =
                serde_json::to_string_pretty(&val).unwrap_or_else(|_| buf.trim().to_owned());
            for pline in pretty.lines() {
                out.push(colorize_line(pline));
            }
            buf.clear();
        }
    }
    if !buf.trim().is_empty() {
        for line in buf.lines() {
            out.push(Line::from(Span::raw(line.to_owned())));
        }
    }
    out
}

fn colorize_line(line: &str) -> Line<'static> {
    let trimmed = line.trim_end_matches(',');
    let has_comma = trimmed.len() < line.len();
    let trimmed_val = trimmed.trim();

    if trimmed_val.is_empty() || matches!(trimmed_val, "{" | "}" | "[" | "]") {
        return Line::from(Span::raw(line.to_owned()));
    }

    if let Some(sep_start) = trimmed.find(r#""": "#) {
        let before = &trimmed[..sep_start + 1];
        let after = &trimmed[sep_start + 3..];

        if before.trim().starts_with('"') {
            let indent = &line[..line.len() - line.trim_start().len()];
            let mut spans: Vec<Span<'static>> = vec![
                Span::raw(indent.to_owned()),
                Span::styled(before.trim().to_owned(), Style::default().fg(Color::Cyan)),
                Span::raw(": "),
            ];
            spans.extend(colorize_value_str(after.trim()));
            if has_comma {
                spans.push(Span::raw(","));
            }
            return Line::from(spans);
        }
    }

    let indent = &line[..line.len() - line.trim_start().len()];
    let mut spans: Vec<Span<'static>> = vec![Span::raw(indent.to_owned())];
    spans.extend(colorize_value_str(trimmed_val));
    if has_comma {
        spans.push(Span::raw(","));
    }
    Line::from(spans)
}

fn colorize_value_str(val: &str) -> Vec<Span<'static>> {
    if val.is_empty() || matches!(val, "{" | "}" | "[" | "]") {
        return vec![Span::raw(val.to_owned())];
    }
    match serde_json::from_str::<Value>(val) {
        Ok(Value::String(s)) => {
            vec![Span::styled(format!("\"{s}\""), Style::default().fg(Color::Green))]
        }
        Ok(Value::Number(n)) => {
            vec![Span::styled(n.to_string(), Style::default().fg(Color::Yellow))]
        }
        Ok(Value::Bool(b)) => {
            vec![Span::styled(b.to_string(), Style::default().fg(Color::Magenta))]
        }
        Ok(Value::Null) => vec![Span::styled(
            "null".to_owned(),
            Style::default().fg(Color::Magenta).add_modifier(Modifier::ITALIC),
        )],
        Ok(Value::Array(_) | Value::Object(_)) => {
            vec![Span::styled(val.to_owned(), Style::default().fg(Color::White))]
        }
        Err(_) => vec![Span::raw(val.to_owned())],
    }
}
