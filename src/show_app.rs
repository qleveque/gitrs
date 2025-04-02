use crate::config::{get_show_command_to_run, run_command, Config};

use crate::git::{git_parse_commit, git_show_output, set_git_dir, FileStatus};
use crate::input::basic_movements;
use crate::ui::{display_commit_metadata, style};

use ratatui::crossterm::event::KeyEvent;
use ratatui::style::Style;
use ratatui::widgets::{ListState, StatefulWidget, Widget};

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::Color,
    widgets::{Block, Borders, List, ListItem},
    Terminal,
};

use std::{env, io};

pub fn show_app(
    config: &Config,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    revision: Option<String>,
) -> io::Result<()> {
    // model
    let original_dir = env::current_dir().unwrap();
    set_git_dir(&config);

    let mut quit = false;
    let mut files_height = 0;

    let output = git_show_output(&revision, &config); // commit hash
    let mut lines = output.lines().map(String::from);
    let (commit, _) = git_parse_commit(&mut lines);

    let mut files = commit.files.clone();
    files.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

    let file_items: Vec<ListItem> = files
        .iter()
        .map(|(file_status, filename)| {
            let label = format!("{} {}", file_status.character(), filename);
            let color = match file_status {
                FileStatus::New => Color::Green,
                FileStatus::Deleted => Color::Red,
                FileStatus::Modified => Color::LightBlue,
                _ => Color::default(),
            };
            ListItem::new(label).style(style(color))
        })
        .collect();
    let files_list = List::new(file_items)
        .block(Block::default().borders(Borders::NONE))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::new().bg(Color::Black))
        .scroll_padding(config.scroll_off);

    let metadata = display_commit_metadata(&commit.metadata);
    let commit_paragraph = metadata.block(Block::default().borders(Borders::NONE));

    let paragraph_len = commit.metadata.lines().count() + 1;

    let mut state = ListState::default();
    state.select_first();

    while !quit {
        // ui
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(paragraph_len as u16), // Fixed size for the top layout
                    Constraint::Min(5), // Minimum size for the bottom layout (5 units)
                ])
                .split(f.area());
            Widget::render(&commit_paragraph, chunks[0], f.buffer_mut());
            StatefulWidget::render(&files_list, chunks[1], f.buffer_mut(), &mut state);
            files_height = chunks[1].height as usize;
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            let Event::Key(KeyEvent {
                kind,
                code,
                modifiers,
                ..
            }) = event::read()? else {
                continue;
            };
            if kind != KeyEventKind::Press {
                continue;
            }

            if let Some(command) = get_show_command_to_run(&config, code) {
                let mut clear = false;

                let file = files[state.selected().unwrap()].1.clone();
                run_command(
                    command,
                    &mut quit,
                    &mut clear,
                    Some(file),
                    Some(commit.hash.clone()),
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
    env::set_current_dir(original_dir).unwrap();
    Ok(())
}
