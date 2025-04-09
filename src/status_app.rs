use crate::input::InputManager;
use crate::{config::Config, git::FileStatus};

use std::collections::HashMap;

use crate::config::get_status_command_to_run;

use crate::git::{git_add_restore, git_status_output, GitFile, StagedStatus};

use ratatui::prelude::CrosstermBackend;
use ratatui::style::Style;
use ratatui::widgets::{ListState, Paragraph, StatefulWidget};

use crossterm::event::KeyCode;
use ratatui::Terminal;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Color,
    widgets::{Block, Borders, List, ListItem},
};

use std::io;

fn compute_tables(
    files: &HashMap<String, GitFile>,
    unstaged_table: &mut Vec<(FileStatus, String)>,
    staged_table: &mut Vec<(FileStatus, String)>,
) {
    unstaged_table.clear();
    for (filename, git_file) in files {
        if git_file.unstaged_status != FileStatus::None {
            unstaged_table.push((git_file.unstaged_status, filename.clone()));
        }
    }

    unstaged_table.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

    staged_table.clear();
    for (filename, git_file) in files {
        if git_file.staged_status != FileStatus::None {
            staged_table.push((git_file.staged_status, filename.clone()));
        }
    }
    staged_table.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
}

fn switch_staged_status(staged_status: &mut StagedStatus, state: &mut ListState) {
    *staged_status = match staged_status {
        StagedStatus::Unstaged => StagedStatus::Staged,
        StagedStatus::Staged => StagedStatus::Unstaged,
    };
    state.select_first();
}

fn toggle_stage_git_file(git_file: &mut GitFile, staged_status: StagedStatus) {
    if staged_status == StagedStatus::Unstaged && git_file.unstaged_status == FileStatus::Unmerged {
        git_file.set_status(FileStatus::None, FileStatus::Modified);
        return;
    }
    match staged_status {
        StagedStatus::Unstaged => git_file.set_status(FileStatus::None, git_file.unstaged_status),
        StagedStatus::Staged => git_file.set_status(git_file.staged_status, FileStatus::None),
    }
}

fn parse_git_status(files: &mut HashMap<String, GitFile>, config: &Config) {
    files.clear();
    let git_status = git_status_output(config);
    for line in git_status.lines() {
        let filename: String = line[2..].trim().to_string();
        let second: char = line.chars().nth(1).unwrap();
        let first: char = line.chars().nth(0).unwrap();

        let unstaged_status = match second {
            '?' => FileStatus::New,
            'D' => FileStatus::Deleted,
            'M' => FileStatus::Modified,
            'U' => FileStatus::Unmerged,
            _ => FileStatus::None,
        };

        let staged_status = match first {
            'A' => FileStatus::New,
            'D' => FileStatus::Deleted,
            'M' => FileStatus::Modified,
            _ => FileStatus::None,
        };
        let git_file = GitFile::new(unstaged_status, staged_status);
        files.insert(filename.clone(), git_file);
    }
}

fn list_to_draw<'a>(
    table: &'a Vec<(FileStatus, String)>,
    width: usize,
    color: Color,
    title: String,
    config: &'a Config,
) -> List<'a> {
    let style = Style::default().fg(color);
    // remove margins
    let w = width - 2;

    let r: Vec<ListItem> = table
        .iter()
        .map(|item| {
            let filename = item.1.clone();
            let too_long = w > 5 && filename.len() + "X ".len() > w;
            let displayed_filename: String = if too_long {
                // Add leading "..." if too long
                format!("...{}", &filename[filename.len() - (w - "X ...".len())..])
            } else {
                filename.clone() // Use full filename if it fits
            };

            let file_status = item.0;
            let label = format!("{} {}", file_status.character(), displayed_filename);
            ListItem::new(label.to_string()).style(style)
        })
        .collect();
    return List::new(r)
        .block(Block::default().title(title).borders(Borders::ALL))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::new().bg(color).fg(Color::Black))
        .scroll_padding(config.scroll_off);
}

pub fn status_app(
    config: &Config,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) -> io::Result<()> {
    // model
    let mut staged_status = StagedStatus::Unstaged;
    let mut unstaged_table: Vec<(FileStatus, String)> = Vec::new();
    let mut staged_table: Vec<(FileStatus, String)> = Vec::new();
    let mut files: HashMap<String, GitFile> = HashMap::new();
    let mut quit = false;
    let mut reload = true;
    let mut height = 0;

    let mut state = ListState::default();
    state.select_first();
    let mut default_state = ListState::default();

    let mut input_manager = InputManager::new();

    while !quit {
        if reload {
            parse_git_status(&mut files, &config);
            compute_tables(&files, &mut unstaged_table, &mut staged_table);
            reload = false;
        }

        let empty_tables = unstaged_table.len() == 0 && staged_table.len() == 0;

        // switch view if empty
        if !empty_tables
            && 0 == match staged_status {
                StagedStatus::Staged => staged_table.len(),
                StagedStatus::Unstaged => unstaged_table.len(),
            }
        {
            switch_staged_status(&mut staged_status, &mut state);
        }

        // ui
        terminal.draw(|f| {
            let size = f.area();
            height = size.height as usize;

            if empty_tables {
                let paragraph = Paragraph::new("Nothing to commit, working tree clean");
                f.render_widget(paragraph, size);
                return;
            }

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(f.area());

            let left_list = list_to_draw(
                &unstaged_table,
                chunks[0].width as usize,
                Color::Red,
                "Not staged:".to_string(),
                &config,
            );
            StatefulWidget::render(
                &left_list,
                chunks[0],
                f.buffer_mut(),
                match staged_status {
                    StagedStatus::Unstaged => &mut state,
                    StagedStatus::Staged => &mut default_state,
                },
            );

            let right_list = list_to_draw(
                &staged_table,
                chunks[1].width as usize,
                Color::Green,
                "Staged:".to_string(),
                &config,
            );
            StatefulWidget::render(
                &right_list,
                chunks[1],
                f.buffer_mut(),
                match staged_status {
                    StagedStatus::Unstaged => &mut default_state,
                    StagedStatus::Staged => &mut state,
                },
            );
        })?;

        if !input_manager.key_pressed()? {
            continue;
        }

        if empty_tables {
            match input_manager.key_event.code {
                KeyCode::Char('q') | KeyCode::Enter | KeyCode::Char(' ') => quit = true,
                KeyCode::Char('r') => {
                    git_add_restore(&mut files, &config, &mut reload);
                }
                _ => (),
            }
            continue;
        }

        let table = match staged_status {
            StagedStatus::Staged => &staged_table,
            StagedStatus::Unstaged => &unstaged_table,
        };
        let idx = state.selected().unwrap();
        let (_, filename) = &table.get(idx).unwrap();
        let git_file = files.get_mut(filename).unwrap().clone();

        let (opt_command, potential) = get_status_command_to_run(
            &config,
            input_manager.key_combination.clone(),
            &git_file,
            staged_status,
        );
        if opt_command.is_some() {
            git_add_restore(&mut files, &config, &mut reload);
        }

        let file = Some(filename.to_string());
        let rev = Some("HEAD".to_string());

        if input_manager.handle_generic_user_input(
            &mut state,
            height,
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
            KeyCode::Char('r') => {
                git_add_restore(&mut files, &config, &mut reload);
            }
            KeyCode::Char('t') | KeyCode::Char(' ') => {
                let mut git_file = files.get_mut(filename).unwrap();
                toggle_stage_git_file(&mut git_file, staged_status);
                compute_tables(&files, &mut unstaged_table, &mut staged_table);
            }
            KeyCode::Char('T') | KeyCode::Enter => {
                for (_, filename) in table {
                    let mut git_file = files.get_mut(filename).unwrap();
                    toggle_stage_git_file(&mut git_file, staged_status);
                }
                compute_tables(&files, &mut unstaged_table, &mut staged_table);
            }
            KeyCode::Tab => {
                let other_len = match staged_status {
                    StagedStatus::Staged => unstaged_table.len(),
                    StagedStatus::Unstaged => staged_table.len(),
                };
                if other_len > 0 {
                    switch_staged_status(&mut staged_status, &mut state);
                }
            }
            KeyCode::Char('K') => {
                if unstaged_table.len() > 0 && staged_status == StagedStatus::Staged {
                    switch_staged_status(&mut staged_status, &mut state);
                }
            }
            KeyCode::Char('J') => {
                if staged_table.len() > 0 && staged_status == StagedStatus::Unstaged {
                    switch_staged_status(&mut staged_status, &mut state);
                }
            }
            _ => (),
        }
    }
    git_add_restore(&mut files, &config, &mut reload);
    Ok(())
}
