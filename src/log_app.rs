use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::{env, thread};

use crate::action::Action;
use crate::app::GitApp;
use crate::app_state::{AppState, NotifChannel};

use crate::config::MappingScope;
use crate::errors::Error;
use crate::git::{git_log_output, set_git_dir};
use crate::pager_widget::PagerWidget;

use ratatui::layout::Rect;

use ratatui::widgets::Clear;
use ratatui::Frame;
use ratatui::{backend::CrosstermBackend, Terminal};

struct PagerAppViewModel {
    list: PagerWidget,
    height: usize,
}

#[derive(PartialEq, Debug)]
pub enum LogStyle {
    Standard,
    StandardGraph,
    OneLine,
    OneLineGraph,
}

pub struct PagerApp {
    state: AppState,
    lines: Arc<Mutex<Vec<String>>>,
    log_style: LogStyle,
    loaded: Arc<AtomicBool>,
    original_dir: std::path::PathBuf,
    view_model: PagerAppViewModel,
}

impl PagerApp {
    pub fn new(args: Vec<String>) -> Result<Self, Error> {
        let state = AppState::new()?;
        let original_dir = env::current_dir()?;
        set_git_dir(&state.config);
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

        let lines = Arc::new(Mutex::new(vec![first_line_ansi]));
        let lines_clone = Arc::clone(&lines);

        let loaded = Arc::new(AtomicBool::new(false));
        let loaded_clone = Arc::clone(&loaded);

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
                        None => {
                            lines_clone.lock().unwrap().extend(chunk);
                            loaded_clone.store(true, Ordering::SeqCst);
                            return;
                        },
                    }
                }
                lines_clone.lock().unwrap().extend(chunk);
            }
        });

        let mut r = Self {
            state,
            lines,
            log_style,
            loaded,
            original_dir,
            view_model: PagerAppViewModel {
                list: PagerWidget::default(),
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

    fn remove_graph_symbols(&self, line: &mut String) {
        // remove | and * graph chars
        if self.log_style == LogStyle::StandardGraph || self.log_style == LogStyle::OneLineGraph {
            loop {
                if let Some(first_char) = line.chars().next() {
                    if first_char == '*' || first_char == '|' {
                        *line = line.chars().skip(2).collect();
                        continue;
                    }
                }
                break;
            }
        }
    }

    fn get_line_file(&self, mut line: String) -> Option<String> {
        if self.log_style == LogStyle::OneLine || self.log_style == LogStyle::OneLineGraph {
            return None;
        }
        self.remove_graph_symbols(&mut line);
        if line.starts_with("diff --git a/") {
            if let Some((_, file)) = line.split_once(" b/") {
                return Some(file.to_string());
            }
        }
        return None;
    }

    fn get_line_line_number(&self, mut line: String) -> Option<usize> {
        if self.log_style == LogStyle::OneLine || self.log_style == LogStyle::OneLineGraph {
            return None;
        }
        self.remove_graph_symbols(&mut line);
        if line.starts_with("@@ -") {
            if let Some((_, line)) = line.split_once(" +") {
                let line: String = line.chars().take_while(|c| c.is_ascii_digit()).collect();
                if let Ok(line_number) = line.parse() {
                    return Some(line_number);
                };
            }
        }
        return None;
    }

    fn get_line_commit(&self, mut line: String) -> Option<String> {
        self.remove_graph_symbols(&mut line);
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

impl GitApp for PagerApp {
    fn state(&mut self) -> &mut AppState {
        &mut self.state
    }

    fn get_state(&self) -> &AppState {
        &self.state
    }

    fn reload(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn loaded(&self) -> bool {
        self.loaded.load(Ordering::SeqCst)
    }

    fn get_text_line(&self, idx: usize) -> Option<String> {
        match self.get_stripped_line(idx) {
            Err(_) => None,
            Ok(str) => Some(str),
        }
    }

    fn draw(&mut self, frame: &mut Frame, rect: Rect) {
        if !self.loaded() {
            let message = format!("loading lines {}...", self.lines.lock().unwrap().len());
            self.notif(NotifChannel::Loading, message);
        } else {
            self.state.notif.remove(&NotifChannel::Loading);
        }
        self.view_model.list = PagerWidget::new(
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

    fn get_file_rev_line(&self) -> Result<(Option<String>, Option<String>, Option<usize>), Error> {
        let mut idx = self.idx()?;
        let mut rfile = None;
        let mut rline = None;
        loop {
            let line = self
                .get_stripped_line(idx)
                .map_err(|_| Error::ReachedLastMachted)?;
            if rfile.is_none() {
                rfile = self.get_line_file(line.clone());
            }
            if rline.is_none() {
                rline = self.get_line_line_number(line.clone());
            }
            if let Some(commit) = self.get_line_commit(line) {
                return Ok((rfile, Some(commit), rline));
            }
            if idx == 0 {
                break;
            } else {
                idx -= 1;
            }
        }
        Ok((None, None, None))
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
                    if let Some(_) = self.get_line_commit(line) {
                        self.state.list_state.select(Some(idx));
                        break;
                    }
                    idx += 1;
                }
                *self.state.list_state.offset_mut() = self.idx()?;
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
                    if let Some(_) = self.get_line_commit(line) {
                        self.state.list_state.select(Some(idx));
                        break;
                    }
                }
                *self.state.list_state.offset_mut() = self.idx()?;
            }
            action => {
                self.run_generic_action(action, self.view_model.height, terminal)?;
            }
        }
        return Ok(());
    }

    fn on_exit(&mut self) -> Result<(), Error> {
        env::set_current_dir(self.original_dir.clone()).map_err(|_| {
            Error::GlobalError("could not restore initial working directory".to_string())
        })
    }
}
