use std::sync::{Arc, Mutex};
use std::thread;

use crate::action::Action;
use crate::app::GitApp;
use crate::app_state::AppState;

use crate::errors::Error;
use crate::git::git_log_output;
use crate::view_list::ViewList;

use ratatui::layout::Rect;

use ratatui::Frame;
use ratatui::{backend::CrosstermBackend, Terminal};

pub struct LogAppViewModel {
    list: ViewList,
    height: usize,
}

pub struct LogApp {
    state: AppState,
    lines: Arc<Mutex<Vec<String>>>,
    view_model: LogAppViewModel,
}

impl LogApp {
    pub fn new(args: Vec<String>) -> Result<Self, Error> {
        let state = AppState::new()?;

        let lines = Arc::new(Mutex::new(Vec::new()));

        let list_data_clone = Arc::clone(&lines);
        let git_exe = state.config.git_exe.clone();
        thread::spawn(move || {
            let n = 100;
            // TODO: unwrap
            let mut iterator = git_log_output(git_exe, args).unwrap();
            loop {
                // TODO: unwrap
                let chunk: Vec<_> = match iterator.by_ref().take(n).collect::<Result<_, _>>() {
                    Err(_) => continue, // invalid UTF-8 data ?,
                    Ok(chunk) => chunk,
                };
                if chunk.is_empty() {
                    break;
                }
                list_data_clone.lock().unwrap().extend(chunk);
            }
        });

        let mut r = Self {
            state,
            lines,
            view_model: LogAppViewModel {
                list: ViewList::default(),
                height: 0,
            },
        };
        r.state.list_state.select_first();
        Ok(r)
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

    fn get_text_line(&mut self, idx: usize) -> Option<String> {
        self.lines.lock().unwrap().get(idx).cloned()
    }

    fn draw(&mut self, frame: &mut Frame, rect: Rect) {
        self.view_model.list = ViewList::new(
            &self.lines.lock().unwrap(),
            self.view_model.height,
            &mut self.state,
        );
        self.view_model.height = rect.height as usize;
        self.view_model.list.render(rect, frame.buffer_mut());
        self.highlight_search(frame, &self.lines.lock().unwrap(), rect);
    }

    fn get_mapping_fields(&mut self) -> Vec<(&str, bool)> {
        vec![("log", true)]
    }

    fn get_file_and_rev(&self) -> Result<(Option<String>, Option<String>), Error> {
        Ok((None, None))
    }

    fn run_action(
        &mut self,
        action: &Action,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Error> {
        self.run_generic_action(action, self.view_model.height, terminal)?;
        return Ok(());
    }
}
