use std::fmt;
use std::io::{BufRead, BufReader, Lines};
use std::path::Path;
use std::process::ChildStdout;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::{env, io, thread};

use crate::action::Action;
use crate::app::GitApp;
use crate::app_state::{AppState, NotifChannel};

use crate::config::MappingScope;
use crate::errors::Error;
use crate::git::{git_pager_output, is_valid_git_rev, set_git_dir};
use crate::pager_widget::PagerWidget;
use crate::ui::clean_buggy_characters;

use ratatui::layout::Rect;

use ratatui::widgets::Clear;
use ratatui::Frame;
use ratatui::{backend::CrosstermBackend, Terminal};
use regex::Regex;

struct PagerAppViewModel {
    list: PagerWidget,
    rect: Rect,
    scroll: Option<bool>,
}

#[derive(PartialEq, Debug)]
pub enum LogStyle {
    Standard,
    OneLine,
    Diff,
    Reflog,
    // pagers
    StashPager,
    Unknown,
}

impl fmt::Display for LogStyle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            LogStyle::Standard => "log",
            LogStyle::OneLine => "log (oneline)",
            LogStyle::Reflog => "log (reflog)",
            LogStyle::StashPager => "log (stash)",
            LogStyle::Diff => "diff",
            LogStyle::Unknown => "pager",
        };
        write!(f, "{}", s)
    }
}

pub enum PagerCommand {
    Log(Vec<String>),
    Show(Vec<String>),
    Diff(Vec<String>),
}

pub struct PagerApp {
    state: AppState,
    mapping_scopes: Vec<MappingScope>,
    lines: Arc<Mutex<Vec<String>>>,
    log_style: LogStyle,
    loaded: Arc<AtomicBool>,
    original_dir: std::path::PathBuf,
    graph: bool,
    view_model: PagerAppViewModel,
}

pub enum LogInput {
    Command(Lines<BufReader<ChildStdout>>),
    Stdin,
}

fn remove_graph_symbols(line: &mut String) {
    // remove | and * graph chars
    loop {
        if let Some(first_char) = line.chars().next() {
            let second_char = line.chars().next();
            if first_char == '*' || first_char == '|' || second_char == Some(' ') {
                *line = line.chars().skip(2).collect();
                continue;
            }
        }
        break;
    }
}

fn guess_log_style(line: &mut String) -> LogStyle {
    let mut words = line.split(' ');
    match words.next() {
        Some("commit") => LogStyle::Standard,
        Some("diff") => LogStyle::Diff,
        Some(rev) => {
            if line.contains("HEAD@{0}:") {
                LogStyle::Reflog
            } else if line.starts_with("stash@{0}:") {
                LogStyle::StashPager
            } else if line.contains(" 1) ") {
                LogStyle::Unknown
            } else {
                if words.next().is_some() && is_valid_git_rev(rev) {
                    LogStyle::OneLine
                } else {
                    LogStyle::Unknown
                }
            }
        }
        None => LogStyle::Unknown,
    }
}

impl PagerApp {
    pub fn new(pager_command: Option<PagerCommand>) -> Result<Self, Error> {
        let state = AppState::new()?;
        let git_exe = state.config.git_exe.clone();
        let mut log_style = LogStyle::Unknown;

        let mut iterator = match pager_command {
            Some(pager_command) => {
                let (git_command, args, style) = match pager_command {
                    PagerCommand::Log(args) => ("log", args, LogStyle::Unknown),
                    PagerCommand::Show(args) => ("show", args, LogStyle::Standard),
                    PagerCommand::Diff(args) => ("diff", args, LogStyle::Diff),
                };
                log_style = style;
                let bufreader: BufReader<ChildStdout> =
                    git_pager_output(git_command, git_exe, args)?;
                LogInput::Command(bufreader.lines())
            }
            None => LogInput::Stdin,
        };
        let mut first_line_ansi = match iterator {
            LogInput::Command(ref mut lines) => lines.by_ref().next(),
            LogInput::Stdin => {
                let stdin = io::stdin();
                let handle = stdin.lock();
                let mut lines = handle.lines();
                lines.next()
            }
        }
        .ok_or_else(|| Error::GlobalError("no data provided to the pager".to_string()))??;
        first_line_ansi = clean_buggy_characters(&first_line_ansi);

        let first_line = String::from_utf8(strip_ansi_escapes::strip(&first_line_ansi.as_bytes()))?;

        // Test if there is a graph mode
        let graph = Some("*") == first_line.split(' ').next();

        let mut line = first_line.clone();
        if graph {
            remove_graph_symbols(&mut line);
        }
        if log_style == LogStyle::Unknown {
            log_style = guess_log_style(&mut line);
        }

        let mapping_scope = match log_style {
            LogStyle::Diff => MappingScope::Diff,
            LogStyle::Reflog => MappingScope::Log,
            LogStyle::Standard => MappingScope::Log,
            LogStyle::OneLine => MappingScope::Log,
            LogStyle::StashPager => MappingScope::Log,
            _ => MappingScope::Pager,
        };
        let mapping_scopes = vec![mapping_scope];

        let lines = Arc::new(Mutex::new(vec![first_line_ansi]));
        let lines_clone = Arc::clone(&lines);

        let loaded = Arc::new(AtomicBool::new(false));
        let loaded_clone = Arc::clone(&loaded);

        thread::spawn(move || {
            let n = 100;
            let mut stdin_lines = match iterator {
                LogInput::Stdin => Some(io::stdin().lock().lines()),
                LogInput::Command(_) => None,
            };
            loop {
                let mut chunk = Vec::with_capacity(n);
                for _ in 0..n {
                    let next = match iterator {
                        LogInput::Command(ref mut lines) => lines.by_ref().next(),
                        LogInput::Stdin => stdin_lines.as_mut().unwrap().next(),
                    };
                    match next {
                        Some(res_line) => {
                            chunk.push(match res_line {
                                Ok(line) => clean_buggy_characters(&line),
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

        let original_dir = env::current_dir()?;
        set_git_dir(&state.config)?;

        let mut r = Self {
            state,
            mapping_scopes,
            lines,
            log_style,
            loaded,
            original_dir,
            graph,
            view_model: PagerAppViewModel {
                list: PagerWidget::default(),
                rect: Rect::default(),
                scroll: None,
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

    fn file_in_line(&self, mut line: String) -> Option<String> {
        if self.log_style == LogStyle::OneLine {
            return None;
        }
        if self.graph {
            remove_graph_symbols(&mut line);
        }
        if line.starts_with("diff --git a/") {
            if let Some((_, file)) = line.split_once(" b/") {
                return Some(file.to_string());
            }
        }
        return None;
    }

    fn line_number_in_line(&self, mut line: String) -> Option<usize> {
        if self.log_style == LogStyle::OneLine {
            return None;
        }
        if self.graph {
            remove_graph_symbols(&mut line);
        }
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

    fn commit_in_line(&self, mut line: String) -> Option<String> {
        if self.graph {
            remove_graph_symbols(&mut line);
        }
        match self.log_style {
            LogStyle::Standard => {
                let (first, rest) = line.split_once(' ').unwrap_or(("", ""));
                if first == "commit" {
                    let (commit, _) = rest.split_once(' ').unwrap_or((rest, ""));
                    if !commit.is_empty() {
                        return Some(commit.to_string());
                    }
                }
            }
            LogStyle::OneLine => {
                // assume this is the first word
                if let Some((commit, _)) = line.split_once(' ') {
                    return Some(commit.to_string());
                }
            }
            LogStyle::StashPager => {
                if line.starts_with("stash@{") {
                    if let Some((commit, _)) = line.split_once(':') {
                        return Some(commit.to_string());
                    }
                }
                return None;
            }
            LogStyle::Reflog => {
                if line.contains("HEAD@{") {
                    if let Some((commit, _)) = line.split_once(' ') {
                        return Some(commit.to_string());
                    }
                }
                return None;
            }
            LogStyle::Diff => {
                let (first, rest) = line.split_once(' ').unwrap_or(("", ""));
                if first == "index" {
                    let (commit, _) = rest.split_once(' ').unwrap_or((rest, ""));
                    if !commit.is_empty() {
                        return Some(commit.to_string());
                    }
                }
            }
            LogStyle::Unknown => {
                return None;
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
        self.view_model.rect = rect;
        let idx = self.idx().unwrap_or(0);
        let idx = idx.checked_add(1).unwrap_or(0);
        let message = format!(
            "{} - line {} of {}",
            self.log_style,
            idx,
            self.lines.lock().unwrap().len(),
        );
        self.notif(NotifChannel::Line, Some(message));
        let scroll_step = self.state.config.scroll_step;
        self.view_model.list = PagerWidget::new(
            &self.lines.lock().unwrap(),
            rect.height as usize,
            &mut self.state,
            self.view_model.scroll,
            scroll_step,
        );
        self.view_model.scroll = None;
        frame.render_widget(Clear, rect);
        self.view_model.list.render(rect, frame.buffer_mut());
        self.highlight_search(frame, rect);
    }

    fn get_mapping_fields(&self) -> Vec<MappingScope> {
        self.mapping_scopes.clone()
    }

    fn get_file_rev_line(&self) -> Result<(Option<String>, Option<String>, Option<usize>), Error> {
        let mut idx = self.idx()?;
        let mut file = None;
        let mut commit = None;
        let mut line_number = None;

        // Test if current line describes a file
        if self.log_style == LogStyle::Standard {
            let idx = self.idx()?;
            let mut line = self
                .get_stripped_line(idx)
                .map_err(|_| Error::GitParsingError)?;
            if self.graph {
                remove_graph_symbols(&mut line);
            }
            let stat_re =
                Regex::new(r"^\s*(?P<file>[^|]+)\s+\|\s+(?P<changes>\d+)\s+(?P<diff>[+\-]+)")
                    .unwrap();
            if Path::new(&line).is_file() {
                file = Some(line);
            } else if let Some(caps) = stat_re.captures(&line) {
                file = match caps.name("file") {
                    None => None,
                    Some(file) => Some(file.as_str().trim().to_string()),
                }
            }
        }

        loop {
            let line = self
                .get_stripped_line(idx)
                .map_err(|_| Error::GitParsingError)?;
            if file.is_none() {
                if let Some(line_file) = self.file_in_line(line.clone()) {
                    file = Some(line_file);
                    if self.log_style == LogStyle::Diff {
                        break;
                    }
                }
            }
            if line_number.is_none() {
                line_number = self.line_number_in_line(line.clone());
            }
            if let Some(line_commit) = self.commit_in_line(line) {
                commit = Some(line_commit);
                if self.log_style != LogStyle::Diff {
                    break;
                }
            }
            if idx == 0 {
                break;
            } else {
                idx -= 1;
            }
        }
        Ok((file, commit, line_number))
    }

    fn run_action(
        &mut self,
        action: &Action,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Error> {
        match action {
            Action::PagerNextCommit => {
                let mut idx = self.idx()? + 1;
                loop {
                    let line = self
                        .get_stripped_line(idx)
                        .map_err(|_| Error::ReachedLastMachted)?;
                    if let Some(_) = self.commit_in_line(line) {
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
                    if let Some(_) = self.commit_in_line(line) {
                        self.state.list_state.select(Some(idx));
                        break;
                    }
                }
                *self.state.list_state.offset_mut() = self.idx()?;
            }
            action => {
                self.run_action_generic(action, self.view_model.rect.height as usize, terminal)?;
            }
        }
        return Ok(());
    }

    fn on_exit(&mut self) -> Result<(), Error> {
        env::set_current_dir(self.original_dir.clone()).map_err(|_| {
            Error::GlobalError("could not restore initial working directory".to_string())
        })
    }

    fn on_scroll(&mut self, down: bool) {
        self.view_model.scroll = Some(down);
    }

    fn on_click(&mut self) {
        let rect = self.view_model.rect;
        if rect.contains(self.state.mouse_position) {
            let delta = (self.state.mouse_position.y - rect.y) as usize;
            self.state
                .list_state
                .select(Some(self.state.list_state.offset() + delta));
        }
    }
}
