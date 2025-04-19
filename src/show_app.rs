use crate::action::Action;
use crate::app::GitApp;
use crate::config::Config;

use crate::git::{git_parse_commit, git_show_output, set_git_dir, Commit, FileStatus};

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Text};
use ratatui::widgets::{ListState, Paragraph, StatefulWidget, Widget};

use ratatui::Frame;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::Color,
    widgets::{Block, Borders, List, ListItem},
    Terminal,
};

use std::env;

pub struct ShowApp {
    commit: Commit,
    files: Vec<(FileStatus, String)>,
    file_list: List<'static>,
    commit_paragraph: Paragraph<'static>,
    state: ListState,
    files_height: usize,
    original_dir: std::path::PathBuf,
}

impl ShowApp {
    pub fn new(config: &Config, revision: Option<String>) -> Self {
        set_git_dir(config);

        let output = git_show_output(&revision, config);
        let mut lines = output.lines().map(String::from);
        let (commit, _) = git_parse_commit(&mut lines);

        let mut files = commit.files.clone();
        files.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

        let file_items: Vec<ListItem> = files
            .iter()
            .map(|(status, name)| {
                let label = format!("{} {}", status.character(), name);
                let color = match status {
                    FileStatus::New => Color::Green,
                    FileStatus::Deleted => Color::Red,
                    FileStatus::Modified => Color::LightBlue,
                    _ => Color::default(),
                };
                ListItem::new(label).style(Style::from(color))
            })
            .collect();

        let file_list = List::new(file_items)
            .block(Block::default().borders(Borders::NONE))
            .style(Style::from(Color::White))
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
            .scroll_padding(config.scroll_off);

        let metadata = Self::display_commit_metadata(commit.metadata.clone());
        let commit_paragraph = metadata.block(Block::default().borders(Borders::NONE));

        let mut state = ListState::default();
        state.select_first();

        return Self {
            commit,
            files,
            file_list,
            commit_paragraph,
            state,
            files_height: 0,
            original_dir: env::current_dir().unwrap(),
        };
    }

    fn display_commit_metadata<'b>(metadata: String) -> Paragraph<'b> {
        let mut lines = metadata.lines();

        let mut styled_lines: Vec<Line<'static>> = Vec::new();

        if let Some(line) = lines.next() {
            styled_lines.push(Line::styled(line.to_string(), Style::from(Color::Blue)));
        }
        if let Some(line) = lines.next() {
            styled_lines.push(Line::styled(line.to_string(), Style::from(Color::Green)));
        }
        if let Some(line) = lines.next() {
            styled_lines.push(Line::styled(line.to_string(), Style::from(Color::Yellow)));
        }
        for line in lines {
            styled_lines.push(Line::styled(
                line.to_string(),
                Style::from(Color::default()),
            ));
        }

        Paragraph::new(Text::from(styled_lines))
    }
}

impl GitApp for ShowApp {
    fn on_exit(&mut self) {
        env::set_current_dir(self.original_dir.clone()).unwrap();
    }

    fn reload(&mut self) {}

    fn draw(&mut self, frame: &mut Frame) {
        let paragraph_len = self.commit.metadata.lines().count() + 1;
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(paragraph_len as u16), Constraint::Min(5)])
            .split(frame.area());

        Widget::render(&self.commit_paragraph, chunks[0], frame.buffer_mut());
        StatefulWidget::render(
            &self.file_list,
            chunks[1],
            frame.buffer_mut(),
            &mut self.state,
        );
        self.files_height = chunks[1].height as usize;
    }

    fn get_config_fields(&mut self) -> Vec<(&str, bool)> {
        vec![("show", true)]
    }

    fn get_file_and_rev(&self) -> (Option<String>, Option<String>) {
        let file = Some(self.files[self.state.selected().unwrap()].1.clone());
        let rev = Some(self.commit.hash.clone());
        (file, rev)
    }

    fn run_action(
        &mut self,
        action: &Action,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> bool {
        let mut new_state = self.state.clone();
        let quit =
            self.run_generic_action(action, self.files_height, terminal, &mut new_state);
        self.state = new_state;
        return quit;
    }
}
