use crate::action::Action;
use crate::app::GitApp;
use crate::{config::Config, git::FileStatus};

use std::collections::HashMap;

use crate::git::{git_add_restore, git_status_output, GitFile, StagedStatus};

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
        .block(Block::default().title(title).borders(Borders::ALL))
        .style(Style::from(Color::White))
        .highlight_style(Style::from(Color::Black).bg(color))
        .scroll_padding(config.scroll_off);
}

pub struct StatusApp<'a> {
    staged_status: StagedStatus,
    unstaged_table: Vec<(FileStatus, String)>,
    staged_table: Vec<(FileStatus, String)>,
    files: HashMap<String, GitFile>,
    height: usize,
    state: ListState,
    default_state: ListState,
    config: &'a Config,
}

impl<'a> StatusApp<'a> {
    pub fn new(config: &'a Config) -> Self {
        let mut state = ListState::default();
        state.select_first();
        let mut instance = Self {
            staged_status: StagedStatus::Unstaged,
            unstaged_table: Vec::new(),
            staged_table: Vec::new(),
            files: HashMap::new(),
            height: 0,
            state,
            default_state: ListState::default(),
            config,
        };
        instance.reload();
        instance
    }

    fn get_current_table(&self) -> &Vec<(FileStatus, String)> {
        match self.staged_status {
            StagedStatus::Staged => &self.staged_table,
            StagedStatus::Unstaged => &self.unstaged_table,
        }
    }

    fn get_filename(&self) -> String {
        let idx = self.state.selected().unwrap();
        let (_, filename) = self.get_current_table().get(idx).unwrap();
        filename.to_string()
    }

    fn get_mut_git_file(&mut self) -> GitFile {
        self.files.get_mut(&self.get_filename()).unwrap().clone()
    }

    fn tables_are_empty(&self) -> bool {
        return self.unstaged_table.len() == 0 && self.staged_table.len() == 0;
    }
}

impl GitApp for StatusApp<'_> {
    fn reload(&mut self) {
        git_add_restore(&mut self.files, &self.config);
        parse_git_status(&mut self.files, &self.config);
        compute_tables(
            &self.files,
            &mut self.unstaged_table,
            &mut self.staged_table,
        );
    }

    fn on_exit(&mut self) {
        git_add_restore(&mut self.files, &self.config);
    }

    fn draw(&mut self, frame: &mut Frame) {
        let size = frame.area();
        self.height = size.height as usize;

        if self.tables_are_empty() {
            let paragraph = Paragraph::new("Nothing to commit, working tree clean");
            frame.render_widget(paragraph, size);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(frame.area());

        let left_list = list_to_draw(
            &self.unstaged_table,
            chunks[0].width as usize,
            Color::Red,
            "Not staged:".to_string(),
            &self.config,
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
            &self.config,
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

    fn get_config_fields(&mut self) -> Vec<(&str, bool)> {
        if self.tables_are_empty() {
            return vec![("status", true)];
        }
        let git_file = self.get_mut_git_file();
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

    fn get_file_and_rev(&self) -> (Option<String>, Option<String>) {
        let file = Some(self.get_filename());
        let rev = Some("HEAD".to_string());
        (file, rev)
    }

    fn run_action(
        &mut self,
        action: &Action,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> bool {
        let mut quit = false;

        if self.tables_are_empty() {
            match action {
                Action::Quit | Action::StageUnstageFile | Action::StageUnstageFiles => quit = true,
                Action::Reload => self.reload(),
                _ => (),
            }
            return quit;
        }

        let filename = self.get_filename();

        match action {
            Action::StageUnstageFile => {
                let mut git_file = self.files.get_mut(&filename).unwrap();
                toggle_stage_git_file(&mut git_file, self.staged_status);
                compute_tables(
                    &self.files,
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
                    let mut git_file = self.files.get_mut(&filename).unwrap();
                    toggle_stage_git_file(&mut git_file, self.staged_status);
                }
                compute_tables(
                    &self.files,
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
                if self.unstaged_table.len() > 0 && self.staged_status == StagedStatus::Staged {
                    switch_staged_status(&mut self.staged_status, &mut self.state);
                }
            }
            Action::FocusStagedView => {
                if self.staged_table.len() > 0 && self.staged_status == StagedStatus::Unstaged {
                    switch_staged_status(&mut self.staged_status, &mut self.state);
                }
            }
            _ => {
                let mut new_state = self.state.clone();
                quit =
                    self.run_generic_action(action, self.height, terminal, &mut new_state);
                self.state = new_state;
            }
        }
        if !self.tables_are_empty() && 0 == self.get_current_table().len() {
            switch_staged_status(&mut self.staged_status, &mut self.state);
        }
        return quit;
    }
}
