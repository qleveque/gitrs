use crate::config::{get_show_command_to_run, run_command, Config};

use crate::input::basic_movements;
use crate::ui::{delta_char, delta_color, delta_value, style};

use ratatui::crossterm::event::KeyEvent;
use ratatui::style::Style;
use ratatui::text::{Line, Text};
use ratatui::widgets::{ListState, Paragraph, StatefulWidget, Widget};

use crossterm::event;
use git2::{Delta, DiffOptions, ObjectType, Repository};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::Color,
    widgets::{Block, Borders, List, ListItem},
    Terminal,
};

use std::io;

pub fn show_app(
    config: &Config,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    revision: Option<String>,
) -> io::Result<()> {
    let mut quit = false;
    let mut files_height = 0;
    let mut state = ListState::default();
    state.select_first();

    let rev = match revision {
        Some(rev) => rev.clone(),
        None => "HEAD".to_string(),
    };

    let repo = Repository::open(".").unwrap();
    let obj = repo.revparse_single(rev.as_str()).unwrap();
    let commit = obj.as_commit().unwrap();

    // COMMIT
    let oid = commit.id();
    let message = commit.message().unwrap_or("No commit message").to_string();
    let author = commit.author();
    let author_name = author.name().unwrap_or("Unknown");
    let author_email = author.email().unwrap_or("Unknown");

    let seconds = commit.time().seconds();
    let commit_datetime = chrono::DateTime::from_timestamp(seconds, 0).unwrap();
    let date = commit_datetime.format("%Y-%m-%d %H:%M:%S").to_string();

    let mut references: Vec<String> = Vec::new();
    let refs = repo.references().unwrap();
    for reference in refs {
        let reference = reference.unwrap();
        if let Some(ref_name) = reference.shorthand() {
            // Check if the reference points to the same commit
            if reference.peel_to_commit().unwrap().id() == oid {
                references.push(ref_name.to_string());
            }
        }
    }
    let mut lines = vec![
        Line::styled(format!("commit  {}", oid.to_string()), style(Color::Blue)),
        Line::styled(
            format!("Author: {} <{}>", author_name, author_email),
            style(Color::Green),
        ),
        Line::styled(format!("Date:   {}", date), style(Color::Yellow)),
        Line::raw(""),
    ];

    if references.len() > 0 {
        let reference_line = Line::styled(
            format!("Refs:   {}", references.join(" ")),
            style(Color::Red),
        );
        lines.insert(1, reference_line);
    }

    let mut message_lines = message.lines();
    while let Some(line) = message_lines.next() {
        lines.push(Line::styled(
            format!("    {}", line),
            style(Color::default()),
        ));
    }
    let len_lines = lines.len() + 1;
    let text = Text::from(lines);
    let paragraph = Paragraph::new(text).block(Block::default().borders(Borders::NONE));

    // FILES
    let mut files: Vec<(Delta, String)> = Vec::new();
    if commit.parent_count() == 0 {
        let tree = commit.tree().unwrap(); // Get the tree of the commit
        for entry in tree.iter() {
            match entry.kind() {
                Some(ObjectType::Blob) => {
                    if let Some(file_name) = entry.name() {
                        files.push((Delta::Added, file_name.to_string()));
                    }
                },
                Some(ObjectType::Tree) => {
                    if let Some(dir_name) = entry.name() {
                        files.push((Delta::Added, format!("{}/", dir_name))); // Indicate it's a directory
                    }
                },
                _ => (),
            }
        }
    } else {
        let parent_commit = commit.parent(0).unwrap();
        let mut diff_options = DiffOptions::new();
        let diff = repo
            .diff_tree_to_tree(
                Some(&parent_commit.tree().unwrap()), // Parent tree
                Some(&commit.tree().unwrap()),        // Current tree
                Some(&mut diff_options),              // Diff options
            )
            .unwrap();
        diff.foreach(
            &mut |delta, _| {
                if let Some(path) = delta.new_file().path().or(delta.old_file().path()) {
                    let status = delta.status();
                    let file_path = path.to_string_lossy()
                        .into_owned();
                    files.push((status.clone(), file_path.clone()));
                }
                true
            },
            None,
            None,
            None,
        )
        .unwrap();
        files.sort_by(|a, b| {
            delta_value(a.0)
                .cmp(&delta_value(b.0))
                .then_with(|| a.1.cmp(&b.1))
        });
    }

    let file_items: Vec<ListItem> = files
        .iter()
        .map(|(status, path)| {
            let label = format!("{} {}", delta_char(*status), path);
            ListItem::new(label).style(style(delta_color(*status)))
        })
        .collect();

    let files_list = List::new(file_items)
        .block(Block::default().borders(Borders::NONE))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::new().bg(Color::Black));

    while !quit {
        // ui
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(len_lines as u16), // Fixed size for the top layout
                    Constraint::Min(5), // Minimum size for the bottom layout (5 units)
                ])
                .split(f.area());
            Widget::render(&paragraph, chunks[0], f.buffer_mut());
            StatefulWidget::render(&files_list, chunks[1], f.buffer_mut(), &mut state);
            files_height = chunks[1].height as usize;
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            let event::Event::Key(KeyEvent {
                code, modifiers, ..
            }) = event::read()?
            else {
                continue;
            };

            if let Some(command) = get_show_command_to_run(&config, code) {
                let mut clear = false;

                let file = files[state.selected().unwrap()].1.clone();
                run_command(
                    command,
                    &mut quit,
                    &mut clear,
                    Some(file),
                    Some(oid.to_string()),
                );

                if clear {
                    let _ = terminal.clear()?;
                }
                continue;
            }

            if basic_movements(code, modifiers, &mut state, files_height, &mut quit) {
                continue;
            }

            match code {
                _ => (),
            }
        }
    }
    Ok(())
}
