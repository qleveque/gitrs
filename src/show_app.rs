use crate::config::{get_show_command_to_run, Config};

use crate::git::{git_parse_commit, git_show_output, set_git_dir, FileStatus};
use crate::input::InputManager;
use crate::ui::{display_commit_metadata, style};

use ratatui::style::Style;
use ratatui::widgets::{ListState, StatefulWidget, Widget};

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

    let mut input_manager = InputManager::new();

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


        if !input_manager.key_pressed()? {
            continue;
        }

        let file = Some(files[state.selected().unwrap()].1.clone());
        let rev = Some(commit.hash.clone());
        let (opt_command, potential) = get_show_command_to_run(
            &config,
            input_manager.key_combination.clone(),
        );
        if input_manager.handle_generic_user_input(
            &mut state,
            files_height,
            &mut quit,
            opt_command,
            file,
            rev,
            potential,
            terminal,
        )? {
            continue;
        }
        match input_manager.key_event.code {
            _ => (),
        }
    }
    env::set_current_dir(original_dir).unwrap();
    Ok(())
}
