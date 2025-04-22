use crate::action::Action;
use crate::app::GitApp;
use crate::app_state::AppState;
use crate::errors::Error;
use crate::{config::Config, git::FileStatus};

use std::collections::HashMap;

use crate::git::{git_add_restore, git_status_output, GitFile, StagedStatus};

use ratatui::layout::Rect;
use ratatui::prelude::CrosstermBackend;
use ratatui::style::Style;
use ratatui::widgets::{ListState, Paragraph, StatefulWidget};

use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Color,
    widgets::{Block, Borders, List, ListItem},
};
use ratatui::{Frame, Terminal};

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

fn parse_git_status(files: &mut HashMap<String, GitFile>, config: &Config) -> Result<(), Error> {
    files.clear();
    let git_status = git_status_output(config);
    for line in git_status.lines() {
        let filename: String = line[2..].trim().to_string();
        let second: char = line.chars().nth(1).ok_or_else(|| Error::GitParsingError)?;
        let first: char = line.chars().nth(0).ok_or_else(|| Error::GitParsingError)?;

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
    Ok(())
}

fn list_to_draw<'a>(
    table: &'a Vec<(FileStatus, String)>,
    width: usize,
    color: Color,
    title: String,
    config: &'a Config,
) -> List<'a> {
    let style = Style::from(color);
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
        .block(Block::default().title(title).borders(Borders::TOP))
        .style(Style::from(Color::White))
        .highlight_style(Style::from(Color::Black).bg(color))
        .scroll_padding(config.scroll_off);
}

pub struct StatusApp {
    app_state: AppState,
    staged_status: StagedStatus,
    unstaged_table: Vec<(FileStatus, String)>,
    staged_table: Vec<(FileStatus, String)>,
    git_files: HashMap<String, GitFile>,
    height: usize,
    state: ListState,
    default_state: ListState,
}

impl StatusApp {
    pub fn new() -> Result<Self, Error> {
        let mut state = ListState::default();
        state.select_first();
        let mut instance = Self {
            app_state: AppState::new()?,
            staged_status: StagedStatus::Unstaged, // TODO: should be staged if unstaged empty
            unstaged_table: Vec::new(),
            staged_table: Vec::new(),
            git_files: HashMap::new(),
            height: 0,
            state,
            default_state: ListState::default(),
        };
        instance.reload()?;
        Ok(instance)
    }

    fn get_current_table(&self) -> &Vec<(FileStatus, String)> {
        match self.staged_status {
            StagedStatus::Staged => &self.staged_table,
            StagedStatus::Unstaged => &self.unstaged_table,
        }
    }

    fn get_filename(&self) -> Result<String, Error> {
        let idx = match self.state.selected() {
            Some(idx) => idx,
            None => return Err(Error::StateIndexError),
        };
        let filename = match self.get_current_table().get(idx) {
            Some((_, filename)) => filename,
            None => return Err(Error::StateIndexError),
        };
        Ok(filename.to_string())
    }

    fn get_mut_git_file(&mut self) -> Result<GitFile, Error> {
        let git_file = match self.git_files.get_mut(&self.get_filename()?) {
            Some(git_file) => git_file.clone(),
            None => return Err(Error::StateIndexError),
        };
        Ok(git_file)
    }

    fn tables_are_empty(&self) -> bool {
        return self.unstaged_table.len() == 0 && self.staged_table.len() == 0;
    }
}

impl GitApp for StatusApp {
    fn get_state(&mut self) -> &mut AppState {
        &mut self.app_state
    }

    fn reload(&mut self) -> Result<(), Error> {
        git_add_restore(&mut self.git_files, &self.app_state.config);
        parse_git_status(&mut self.git_files, &self.app_state.config)?;
        compute_tables(
            &self.git_files,
            &mut self.unstaged_table,
            &mut self.staged_table,
        );
        if !self.tables_are_empty() && 0 == self.get_current_table().len() {
            switch_staged_status(&mut self.staged_status, &mut self.state);
        }
        Ok(())
    }

    fn on_exit(&mut self) -> Result<(), Error> {
        git_add_restore(&mut self.git_files, &self.app_state.config);
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame, rect: Rect) {
        self.height = rect.height as usize;

        if self.tables_are_empty() {
            let paragraph = Paragraph::new("Nothing to commit, working tree clean");
            frame.render_widget(paragraph, rect);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rect);

        let left_list = list_to_draw(
            &self.unstaged_table,
            chunks[0].width as usize,
            Color::Red,
            "Not staged:".to_string(),
            &self.app_state.config,
        );
        StatefulWidget::render(
            &left_list,
            chunks[0],
            frame.buffer_mut(),
            match self.staged_status {
                StagedStatus::Unstaged => &mut self.state,
                StagedStatus::Staged => &mut self.default_state,
            },
        );

        let right_list = list_to_draw(
            &self.staged_table,
            chunks[1].width as usize,
            Color::Green,
            "Staged:".to_string(),
            &self.app_state.config,
        );
        StatefulWidget::render(
            &right_list,
            chunks[1],
            frame.buffer_mut(),
            match self.staged_status {
                StagedStatus::Unstaged => &mut self.default_state,
                StagedStatus::Staged => &mut self.state,
            },
        );
    }

    fn get_mapping_fields(&mut self) -> Vec<(&str, bool)> {
        let git_file = match self.get_mut_git_file() {
            Ok(git_file) => git_file,
            Err(_) => return vec![("status", true)],
        };
        vec![
            (
                "unmerged",
                self.staged_status == StagedStatus::Unstaged
                    && git_file.unstaged_status == FileStatus::Unmerged,
            ),
            (
                "untracked",
                self.staged_status == StagedStatus::Unstaged
                    && git_file.unstaged_status == FileStatus::New,
            ),
            ("staged", self.staged_status == StagedStatus::Staged),
            ("unstaged", self.staged_status == StagedStatus::Unstaged),
            ("status", true),
        ]
    }

    fn get_file_and_rev(&self) -> Result<(Option<String>, Option<String>), Error> {
        Ok((Some(self.get_filename()?), Some("HEAD".to_string())))
    }

    fn run_action(
        &mut self,
        action: &Action,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Error> {

        if self.tables_are_empty() {
            match action {
                Action::Quit | Action::StageUnstageFile | Action::StageUnstageFiles => self.app_state.quit = true,
                Action::Reload => self.reload()?,
                _ => (),
            }
            return Ok(());
        }

        match action {
            Action::StageUnstageFile => {
                let mut git_file = self.git_files.get_mut(&self.get_filename()?).unwrap();
                toggle_stage_git_file(&mut git_file, self.staged_status);
                compute_tables(
                    &self.git_files,
                    &mut self.unstaged_table,
                    &mut self.staged_table,
                );
            }
            Action::StageUnstageFiles => {
                let filenames: Vec<_> = self
                    .get_current_table()
                    .iter()
                    .map(|(_, filename)| filename.clone())
                    .collect();
                for filename in filenames {
                    let mut git_file = match self.git_files.get_mut(&filename) {
                        Some(git_file) => git_file,
                        None => return Err(Error::UnknownFilename(filename)),
                    };
                    toggle_stage_git_file(&mut git_file, self.staged_status);
                }
                compute_tables(
                    &self.git_files,
                    &mut self.unstaged_table,
                    &mut self.staged_table,
                );
            }
            Action::SwitchView => {
                let other_len = match self.staged_status {
                    StagedStatus::Staged => self.unstaged_table.len(),
                    StagedStatus::Unstaged => self.staged_table.len(),
                };
                if other_len > 0 {
                    switch_staged_status(&mut self.staged_status, &mut self.state);
                }
            }
            Action::FocusUnstagedView => {
                self.staged_status = StagedStatus::Unstaged;
                self.state.select_first();
            }
            Action::FocusStagedView => {
                self.staged_status = StagedStatus::Staged;
                self.state.select_first();
            }
            action => {
                if matches!(action, Action::Command(_, _)) {
                    git_add_restore(&mut self.git_files, &self.app_state.config);
                }
                let mut new_state = self.state.clone();
                self.run_generic_action(action, self.height, terminal, &mut new_state)?;
                self.state = new_state;
            }
        }
        if !self.tables_are_empty() && 0 == self.get_current_table().len() {
            switch_staged_status(&mut self.staged_status, &mut self.state);
        }
        return Ok(());
    }
}
