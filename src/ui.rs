use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::Paragraph,
};

use crate::git::CommitRef;

pub fn style(color: Color) -> Style {
    Style::default().fg(color)
}

pub fn display_commit_metadata<'a>(metadata: &'a String) -> Paragraph<'a> {
    let mut mlines = metadata.lines();
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::styled(mlines.next().unwrap(), style(Color::Blue)));
    lines.push(Line::styled(mlines.next().unwrap(), style(Color::Green)));
    lines.push(Line::styled(mlines.next().unwrap(), style(Color::Yellow)));
    while let Some(line) = mlines.next() {
        lines.push(Line::styled(line, style(Color::default())));
    }
    let text = Text::from(lines);
    let paragraph = Paragraph::new(text);
    paragraph
}

pub fn display_blame_line<'a>(
    opt_commit: &'a Option<CommitRef>,
    idx: usize,
    max_author_len: usize,
    max_line_len: usize,
    max_blame_len: &mut usize,
) -> Line<'a> {
    match opt_commit {
        Some(commit) => {
            let displayed_hash: String = commit.hash.chars().take(4).collect();
            let spans = vec![
                Span::styled(displayed_hash, style(Color::Blue)),
                Span::raw(" "),
                Span::styled(
                    format!("{:<max_author_len$}", commit.author.clone()),
                    style(Color::Yellow),
                ),
                Span::raw(" "),
                Span::styled(commit.date.clone(), style(Color::Blue)),
                Span::raw(" "),
                Span::styled(
                    format!("{:>max_line_len$}", idx),
                    style(Color::Yellow)
                ),
            ];
            let line = Line::from(spans);
            if *max_blame_len < line.width() {
                *max_blame_len = line.width()
            }
            line
        },
        _ => Line::from("Not Committed Yet".to_string()),
    }
}
