use crate::app::{FileRevLine, GitApp};
use crate::model::action::Action;
use crate::model::app_state::AppState;
use crate::model::config::{Config, MappingScope};
use crate::model::errors::Error;
use crate::model::git::{git_add_restore, git_status_output, FileStatus, GitFile, StagedStatus};

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::CrosstermBackend,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, StatefulWidget},
};
use ratatui::{Frame, Terminal};
use std::collections::HashMap;

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

fn switch_staged_status(staged_status: &mut StagedStatus, list_state: &mut ListState) {
    *staged_status = match staged_status {
        StagedStatus::Unstaged => StagedStatus::Staged,
        StagedStatus::Staged => StagedStatus::Unstaged,
    };
    list_state.select_first();
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
    for line in git_status?.lines() {
        let filename: String = line[2..].trim().to_string();
        let second: char = line.chars().nth(1).ok_or_else(|| Error::GitParsing)?;
        let first: char = line.chars().next().ok_or_else(|| Error::GitParsing)?;

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
    table: &'a [(FileStatus, String)],
    color: Color,
    title: String,
    config: &'a Config,
) -> List<'a> {
    let style = Style::from(color);

    let r: Vec<ListItem> = table
        .iter()
        .map(|item| ListItem::new(format!("{} {}", item.0.character(), item.1)).style(style))
        .collect();
    List::new(r)
        .block(Block::default().title(title).borders(Borders::TOP))
        .style(Style::from(Color::White))
        .highlight_style(Style::from(Color::Black).bg(color))
        .scroll_padding(config.scrolloff)
}

#[derive(Default)]
pub struct StatusAppViewModel {
    top_rect: Rect,
    bottom_rect: Rect,
}

pub struct StatusApp {
    state: AppState,
    staged_status: StagedStatus,
    unstaged_table: Vec<(FileStatus, String)>,
    staged_table: Vec<(FileStatus, String)>,
    git_files: HashMap<String, GitFile>,
    view_model: StatusAppViewModel,
}

impl StatusApp {
    pub fn new() -> Result<Self, Error> {
        let mut state = AppState::new()?;
        state.list_state.select_first();
        let mut instance = Self {
            state,
            staged_status: StagedStatus::Unstaged,
            unstaged_table: Vec::new(),
            staged_table: Vec::new(),
            git_files: HashMap::new(),
            view_model: StatusAppViewModel::default(),
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
        let idx = self.idx()?;
        let filename = match self.get_current_table().get(idx) {
            Some((_, filename)) => filename,
            None => return Err(Error::StateIndex),
        };
        Ok(filename.to_string())
    }

    fn get_git_file(&self) -> Result<GitFile, Error> {
        let git_file = match self.git_files.get(&self.get_filename()?) {
            Some(git_file) => git_file.clone(),
            None => return Err(Error::StateIndex),
        };
        Ok(git_file)
    }

    fn tables_are_empty(&self) -> bool {
        self.unstaged_table.is_empty() && self.staged_table.is_empty()
    }
}

impl GitApp for StatusApp {
    fn state(&mut self) -> &mut AppState {
        &mut self.state
    }
    fn get_state(&self) -> &AppState {
        &self.state
    }

    fn on_click(&mut self) {
        if self.view_model.top_rect.contains(self.state.mouse_position) {
            self.staged_status = StagedStatus::Unstaged;
            let delta = (self.state.mouse_position.y - self.view_model.top_rect.y) as usize;
            if delta > 0 {
                self.state
                    .list_state
                    .select(Some(self.state.list_state.offset() + delta - 1));
            }
        } else if self
            .view_model
            .bottom_rect
            .contains(self.state.mouse_position)
        {
            self.staged_status = StagedStatus::Staged;
            let delta = (self.state.mouse_position.y - self.view_model.bottom_rect.y) as usize;
            if delta > 0 {
                self.state
                    .list_state
                    .select(Some(self.state.list_state.offset() + delta - 1));
            }
        }
    }

    fn get_text_line(&self, idx: usize) -> Option<String> {
        self.get_current_table()
            .get(idx)
            .map(|(_, name)| name.to_string())
    }

    fn reload(&mut self) -> Result<(), Error> {
        git_add_restore(&mut self.git_files, &self.state.config);
        parse_git_status(&mut self.git_files, &self.state.config)?;
        compute_tables(
            &self.git_files,
            &mut self.unstaged_table,
            &mut self.staged_table,
        );
        if !self.tables_are_empty() && self.get_current_table().is_empty() {
            switch_staged_status(&mut self.staged_status, &mut self.state.list_state);
        }
        Ok(())
    }

    fn on_exit(&mut self) -> Result<(), Error> {
        git_add_restore(&mut self.git_files, &self.state.config);
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame, rect: Rect) {
        if self.tables_are_empty() {
            let paragraph = Paragraph::new("Nothing to commit, working tree clean");
            frame.render_widget(paragraph, rect);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rect);
        self.view_model.top_rect = chunks[0];
        self.view_model.bottom_rect = chunks[1];

        let top_list = list_to_draw(
            &self.unstaged_table,
            Color::Red,
            "Not staged:".to_string(),
            &self.state.config,
        );
        let mut default = ListState::default();
        StatefulWidget::render(
            &top_list,
            chunks[0],
            frame.buffer_mut(),
            match self.staged_status {
                StagedStatus::Unstaged => &mut self.state.list_state,
                StagedStatus::Staged => &mut default,
            },
        );

        let bottom_list = list_to_draw(
            &self.staged_table,
            Color::Green,
            "Staged:".to_string(),
            &self.state.config,
        );
        let mut default = ListState::default();
        StatefulWidget::render(
            &bottom_list,
            chunks[1],
            frame.buffer_mut(),
            match self.staged_status {
                StagedStatus::Unstaged => &mut default,
                StagedStatus::Staged => &mut self.state.list_state,
            },
        );

        let chunk = match self.staged_status {
            StagedStatus::Unstaged => chunks[0],
            StagedStatus::Staged => chunks[1],
        };
        self.highlight_search(
            frame,
            Rect {
                x: rect.x + chunk.x + 2,
                y: chunk.y + 1,
                width: chunk.width - 1,
                height: chunk.height - 1,
            },
        );
    }

    fn get_mapping_fields(&self) -> Vec<MappingScope> {
        let git_file = match self.get_git_file() {
            Ok(git_file) => git_file,
            Err(_) => return vec![MappingScope::Status(None, None)],
        };
        let file_status = match self.staged_status {
            StagedStatus::Staged => git_file.staged_status,
            StagedStatus::Unstaged => git_file.unstaged_status,
        };
        vec![
            MappingScope::Status(Some(self.staged_status), Some(file_status)),
            MappingScope::Status(Some(self.staged_status), None),
            MappingScope::Status(None, None),
        ]
    }

    fn get_file_rev_line(&self) -> Result<FileRevLine, Error> {
        let filename = self.get_filename().ok();
        Ok((filename, Some("HEAD".to_string()), None))
    }

    fn on_scroll(&mut self, down: bool) {
        if self.view_model.top_rect.contains(self.state.mouse_position) {
            self.staged_status = StagedStatus::Unstaged;
        } else if self
            .view_model
            .bottom_rect
            .contains(self.state.mouse_position)
        {
            self.staged_status = StagedStatus::Staged;
        };
        let rect = match self.staged_status {
            StagedStatus::Unstaged => self.view_model.top_rect,
            StagedStatus::Staged => self.view_model.bottom_rect,
        };
        let table = self.get_current_table();
        self.on_scroll_generic(down, rect.height as usize, table.len());
    }

    fn run_action(
        &mut self,
        action: &Action,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Error> {
        match action {
            Action::StageUnstageFile => {
                let git_file = self.git_files.get_mut(&self.get_filename()?).unwrap();
                toggle_stage_git_file(git_file, self.staged_status);
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
                    let git_file = match self.git_files.get_mut(&filename) {
                        Some(git_file) => git_file,
                        None => return Err(Error::UnknownFilename(filename)),
                    };
                    toggle_stage_git_file(git_file, self.staged_status);
                }
                compute_tables(
                    &self.git_files,
                    &mut self.unstaged_table,
                    &mut self.staged_table,
                );
            }
            Action::StatusSwitchView => {
                let other_len = match self.staged_status {
                    StagedStatus::Staged => self.unstaged_table.len(),
                    StagedStatus::Unstaged => self.staged_table.len(),
                };
                if other_len > 0 {
                    switch_staged_status(&mut self.staged_status, &mut self.state.list_state);
                }
            }
            Action::FocusUnstagedView => {
                self.staged_status = StagedStatus::Unstaged;
                self.state().list_state.select_first();
            }
            Action::FocusStagedView => {
                self.staged_status = StagedStatus::Staged;
                self.state().list_state.select_first();
            }
            action => {
                if matches!(action, Action::Command(_, _)) {
                    git_add_restore(&mut self.git_files, &self.state.config);
                }
                let rect = match self.staged_status {
                    StagedStatus::Unstaged => self.view_model.top_rect,
                    StagedStatus::Staged => self.view_model.bottom_rect,
                };
                self.run_action_generic(action, rect.height as usize, terminal)?;
            }
        }
        if !self.tables_are_empty() && self.get_current_table().is_empty() {
            switch_staged_status(&mut self.staged_status, &mut self.state.list_state);
        }
        Ok(())
    }
}
