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
            value_to_lines(&val, 0, &mut out, false);
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

fn value_to_lines(val: &Value, indent: usize, out: &mut Vec<Line<'static>>, skip_brackets: bool) {
    match val {
        Value::Object(map) => {
            if !skip_brackets {
                out.push(Line::from(Span::raw(format!("{}{{", " ".repeat(indent * 2)))));
            }
            for (i, (k, v)) in map.iter().enumerate() {
                let mut spans: Vec<Span<'static>> = Vec::new();
                spaces(indent + 1, &mut spans);
                spans.push(Span::styled(
                    format!("\"{k}\""),
                    Style::default().fg(Color::Cyan),
                ));
                spans.push(Span::raw(": "));
                match v {
                    Value::Object(_) | Value::Array(_) => {
                        let ch = if matches!(v, Value::Object(_)) { '{' } else { '[' };
                        spans.push(Span::raw(ch.to_string()));
                        if i < map.len() - 1 { spans.push(Span::raw(",")); }
                        out.push(Line::from(spans));
                        value_to_lines(v, indent + 2, out, true);
                        let mut close_spans = Vec::new();
                        spaces(indent + 1, &mut close_spans);
                        let close_ch = if matches!(v, Value::Object(_)) { '}' } else { ']' };
                        close_spans.push(Span::raw(close_ch.to_string()));
                        out.push(Line::from(close_spans));
                    }
                    _ => {
                        value_spans(v, &mut spans);
                        if i < map.len() - 1 { spans.push(Span::raw(",")); }
                        out.push(Line::from(spans));
                    }
                }
            }
            if !skip_brackets {
                out.push(Line::from(Span::raw(format!("{}}}", " ".repeat(indent * 2)))));
            }
        }
        Value::Array(arr) => {
            if !skip_brackets {
                out.push(Line::from(Span::raw(format!("{}[", " ".repeat(indent * 2)))));
            }
            for (i, v) in arr.iter().enumerate() {
                let mut spans: Vec<Span<'static>> = Vec::new();
                spaces(indent + 1, &mut spans);
                match v {
                    Value::Object(_) | Value::Array(_) => {
                        let ch = if matches!(v, Value::Object(_)) { '{' } else { '[' };
                        spans.push(Span::raw(ch.to_string()));
                        if i < arr.len() - 1 { spans.push(Span::raw(",")); }
                        out.push(Line::from(spans));
                        value_to_lines(v, indent + 2, out, true);
                        let mut close_spans = Vec::new();
                        spaces(indent + 1, &mut close_spans);
                        let close_ch = if matches!(v, Value::Object(_)) { '}' } else { ']' };
                        close_spans.push(Span::raw(close_ch.to_string()));
                        out.push(Line::from(close_spans));
                    }
                    _ => {
                        value_spans(v, &mut spans);
                        if i < arr.len() - 1 { spans.push(Span::raw(",")); }
                        out.push(Line::from(spans));
                    }
                }
            }
            if !skip_brackets {
                out.push(Line::from(Span::raw(format!("{}]", " ".repeat(indent * 2)))));
            }
        }
        other => {
            let mut spans = Vec::new();
            spaces(indent, &mut spans);
            value_spans(other, &mut spans);
            out.push(Line::from(spans));
        }
    }
}

fn value_spans(val: &Value, spans: &mut Vec<Span<'static>>) {
    match val {
        Value::Null => spans.push(Span::styled(
            "null",
            Style::default().fg(Color::Magenta).add_modifier(Modifier::ITALIC),
        )),
        Value::Bool(b) => spans.push(Span::styled(
            b.to_string(),
            Style::default().fg(Color::Magenta),
        )),
        Value::Number(n) => spans.push(Span::styled(
            n.to_string(),
            Style::default().fg(Color::Yellow),
        )),
        Value::String(s) => spans.push(Span::styled(
            format!("\"{s}\""),
            Style::default().fg(Color::Green),
        )),
        _ => unreachable!(),
    }
}

fn spaces(n: usize, spans: &mut Vec<Span<'static>>) {
    if n > 0 {
        spans.push(Span::raw(" ".repeat(n * 2)));
    }
}
