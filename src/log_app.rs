use std::sync::{Arc, Mutex};
use std::thread;

use crate::action::Action;
use crate::app::GitApp;
use crate::app_state::AppState;

use crate::config::MappingScope;
use crate::errors::Error;
use crate::git::git_log_output;
use crate::view_list::ViewList;

use ratatui::layout::Rect;

use ratatui::widgets::Clear;
use ratatui::Frame;
use ratatui::{backend::CrosstermBackend, Terminal};

struct LogAppViewModel {
    list: ViewList,
    height: usize,
}

#[derive(PartialEq, Debug)]
pub enum LogStyle {
    Standard,
    StandardGraph,
    OneLine,
    OneLineGraph,
}

pub struct LogApp {
    state: AppState,
    lines: Arc<Mutex<Vec<String>>>,
    log_style: LogStyle,
    view_model: LogAppViewModel,
}

impl LogApp {
    pub fn new(args: Vec<String>) -> Result<Self, Error> {
        let state = AppState::new()?;
        let git_exe = state.config.git_exe.clone();
        let mut iterator = git_log_output(git_exe, args).unwrap();
        let first_line_ansi = iterator
            .by_ref()
            .next()
            .ok_or_else(|| Error::GitParsingError)??
            .replace("\t", "    ");

        let bytes = strip_ansi_escapes::strip(&first_line_ansi.as_bytes());
        let first_line = String::from_utf8(bytes)?;

        let (first_word, other_words) = first_line
            .split_once(' ')
            .ok_or_else(|| Error::GitParsingError)?;
        // Hopefully this is enough
        let log_style = match first_word {
            "commit" => LogStyle::Standard,
            "*" => {
                let (second_word, _) = other_words
                    .split_once(' ')
                    .ok_or_else(|| Error::GitParsingError)?;
                match second_word {
                    "commit" => LogStyle::StandardGraph,
                    _ => LogStyle::OneLineGraph,
                }
            }
            _ => LogStyle::OneLine,
        };
        println!("{:?}", log_style);

        let lines = Arc::new(Mutex::new(vec![first_line_ansi]));
        let list_data_clone = Arc::clone(&lines);
        thread::spawn(move || {
            let n = 100;
            loop {
                let mut chunk = Vec::with_capacity(n);
                for _ in 0..n {
                    let next = iterator.by_ref().next();
                    match next {
                        Some(res_line) => {
                            chunk.push(match res_line {
                                Ok(line) => line.replace("\t", "    "),
                                Err(_) => "\x1b[31m/!\\ *** ERROR *** /!\\: gitrs could not read that line\x1b[0m".to_string(),
                            })
                        }
                        None => break,
                    }
                }
                list_data_clone.lock().unwrap().extend(chunk);
            }
        });

        let mut r = Self {
            state,
            lines,
            log_style,
            view_model: LogAppViewModel {
                list: ViewList::default(),
                height: 0,
            },
        };
        r.state.list_state.select_first();
        Ok(r)
    }

    fn get_stripped_line(&self, idx: usize) -> Result<String, Error> {
        let s = self
            .lines
            .lock()
            .unwrap()
            .get(idx)
            .cloned()
            .ok_or_else(|| Error::StateIndexError)?;
        let bytes = strip_ansi_escapes::strip(&s.as_bytes());
        let str = String::from_utf8(bytes)?;
        Ok(str)
    }

    fn read_log_line_rev(&self, mut line: String) -> Option<String> {
        // remove | and * graph chars
        if self.log_style == LogStyle::StandardGraph || self.log_style == LogStyle::OneLineGraph {
            loop {
                if let Some(first_char) = line.chars().next() {
                    if first_char == '*' || first_char == '|' {
                        line = line.chars().skip(2).collect();
                        continue;
                    }
                } else {
                    return None;
                }
                break;
            }
        }
        match self.log_style {
            LogStyle::StandardGraph | LogStyle::Standard => {
                let (first, rest) = line.split_once(' ').unwrap_or(("", ""));
                if first == "commit" {
                    let (commit, _) = rest.split_once(' ').unwrap_or((rest, ""));
                    if !commit.is_empty() {
                        return Some(commit.to_string());
                    }
                }
            }
            LogStyle::OneLineGraph | LogStyle::OneLine => {
                // assume this is the first word
                let (commit, _) = line.split_once(' ').unwrap_or(("", ""));
                if !commit.is_empty() {
                    return Some(commit.to_string());
                }
            }
        }
        return None;
    }
}

impl GitApp for LogApp {
    fn state(&mut self) -> &mut AppState {
        &mut self.state
    }

    fn get_state(&self) -> &AppState {
        &self.state
    }

    fn reload(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn get_text_line(&self, idx: usize) -> Option<String> {
        match self.get_stripped_line(idx) {
            Err(_) => None,
            Ok(str) => Some(str),
        }
    }

    fn draw(&mut self, frame: &mut Frame, rect: Rect) {
        // self.state.notif.clear();
        // self.info(&format!("lines: {}", self.lines.lock().unwrap().len()));
        self.view_model.list = ViewList::new(
            &self.lines.lock().unwrap(),
            self.view_model.height,
            &mut self.state,
        );
        self.view_model.height = rect.height as usize;
        frame.render_widget(Clear, rect);
        self.view_model.list.render(rect, frame.buffer_mut());
        self.highlight_search(frame, rect);
    }

    fn get_mapping_fields(&mut self) -> Vec<(MappingScope, bool)> {
        vec![(MappingScope::Log, true)]
    }

    fn get_file_and_rev(&self) -> Result<(Option<String>, Option<String>), Error> {
        let mut idx = self.idx()?;
        loop {
            let line = self
                .get_stripped_line(idx)
                .map_err(|_| Error::ReachedLastMachted)?;
            if let Some(commit) = self.read_log_line_rev(line) {
                return Ok((None, Some(commit)));
            }
            if idx == 0 {
                break;
            } else {
                idx -= 1;
            }
        }
        Ok((None, None))
    }

    fn run_action(
        &mut self,
        action: &Action,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Error> {
        match action {
            Action::NextCommit => {
                let mut idx = self.idx()? + 1;
                loop {
                    let line = self
                        .get_stripped_line(idx)
                        .map_err(|_| Error::ReachedLastMachted)?;
                    if let Some(_) = self.read_log_line_rev(line) {
                        self.state.list_state.select(Some(idx));
                        break;
                    }
                    idx += 1;
                }
            }
            Action::PreviousCommit => {
                let mut idx = self.idx()?;
                loop {
                    if idx == 0 {
                        break;
                    }
                    idx -= 1;
                    let line = self
                        .get_stripped_line(idx)
                        .map_err(|_| Error::ReachedLastMachted)?;
                    if let Some(_) = self.read_log_line_rev(line) {
                        self.state.list_state.select(Some(idx));
                        break;
                    }
                }
            }
            action => {
                self.run_generic_action(action, self.view_model.height, terminal)?;
            }
        }
        return Ok(());
    }
}
