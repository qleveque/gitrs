use std::process::{Command, Stdio};

use crossterm::{
    event::{self, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::CrosstermBackend,
    style::{Color, Style},
    widgets::{Block, Borders, ListState, Paragraph, Widget},
    Frame, Terminal,
};

use crate::{
    action::{Action, CommandType},
    app_state::{AppState, Notif, NotifType},
    errors::Error,
};

pub trait GitApp {
    fn draw(&mut self, frame: &mut Frame, rect: Rect);

    fn on_exit(&mut self) -> Result<(), Error> {
        Ok(())
    }
    fn reload(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn get_state(&mut self) -> &mut AppState;
    fn get_mapping_fields(&mut self) -> Vec<(&str, bool)>;
    fn get_file_and_rev(&self) -> Result<(Option<String>, Option<String>), Error>;

    fn run_action(
        &mut self,
        action: &Action,
        _terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Error>;

    fn notif(&mut self, notif_type: NotifType, message: &str) {
        self.get_state().notif = Some(Notif {
            notif_type,
            message: message.to_string(),
        });
    }
    fn info(&mut self, message: &str) {
        self.notif(NotifType::Info, message);
    }
    fn error(&mut self, message: &str) {
        self.notif(NotifType::Error, message);
    }

    fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Error> {
        enable_raw_mode()?;

        loop {
            terminal.draw(|frame| {
                let mut chunk = frame.area();

                if self.get_state().is_searching || !self.get_state().search_string.is_empty() {
                    let mut searched_string = self.get_state().search_string.clone();
                    if self.get_state().is_searching {
                        searched_string.push_str("│")
                    }
                    let paragraph = Paragraph::new(searched_string)
                        .block(Block::default().borders(Borders::TOP).title("Search"));
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Min(0), Constraint::Length(2)])
                        .split(chunk);
                    Widget::render(&paragraph, chunks[1], frame.buffer_mut());
                    chunk = chunks[0];
                } 

                if let Some(notif) = self.get_state().notif.clone() {
                    let style = match notif.notif_type {
                        NotifType::Info => Style::from(Color::Blue),
                        NotifType::Error => Style::from(Color::Red),
                    };
                    let title = match notif.notif_type {
                        NotifType::Info => "Info",
                        NotifType::Error => "Error",
                    };
                    let paragraph = Paragraph::new(&*notif.message)
                        .block(Block::default().borders(Borders::TOP).title(title))
                        .style(style);

                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Min(0), Constraint::Length(2)])
                        .split(chunk);
                    Widget::render(&paragraph, chunks[1], frame.buffer_mut());
                    chunk = chunks[0];
                }

                self.draw(frame, chunk);
            })?;

            let opt_action = self.handle_user_input()?;
            if let Some(action) = opt_action {
                self.get_state().key_combination = "".to_string();
                match self.run_action(&action, terminal) {
                    Err(err) => self.error(&err.to_string()),
                    Ok(()) => (),
                }
                if self.get_state().quit {
                    break;
                }
            }

            // display key combination if multiple letters
            let key_combination = self.get_state().key_combination.clone();
            if self.get_state().notif.is_none() && !key_combination.is_empty() {
                self.info(&key_combination);
            }
        }
        self.on_exit()?;

        disable_raw_mode()?;
        terminal.show_cursor()?;
        Ok(())
    }

    fn key_pressed(&mut self) -> Result<bool, Error> {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let event::Event::Key(key_event) = event::read()? {
                if key_event.kind == KeyEventKind::Press {

                    if self.get_state().is_searching {

                        match key_event.code {
                            KeyCode::Enter => {
                                self.get_state().is_searching = false;
                            },
                            KeyCode::Esc => {
                                self.get_state().is_searching = false;
                                self.get_state().search_string = "".to_string();
                            },
                            KeyCode::Backspace => {
                                self.get_state().search_string.pop();
                            },
                            KeyCode::Char(char) => {
                                self.get_state().search_string.push(char);
                            },
                            _ => {
                                self.error("error: this char is not handled yet");
                                return Ok(false);
                            }
                        }
                        self.get_state().notif = None;
                        return Ok(false);
                    }

                    let mut key_str = match key_event.code {
                        KeyCode::Up => "up".to_string(),
                        KeyCode::Down => "down".to_string(),
                        KeyCode::Right => "right".to_string(),
                        KeyCode::Left => "left".to_string(),
                        KeyCode::Enter => "cr".to_string(),
                        KeyCode::Tab => "tab".to_string(),
                        KeyCode::Home => "home".to_string(),
                        KeyCode::End => "end".to_string(),
                        KeyCode::Esc => "esc".to_string(),
                        KeyCode::PageUp => "pgup".to_string(),
                        KeyCode::PageDown => "pgdown".to_string(),
                        KeyCode::Char(' ') => "space".to_string(),
                        key_code => key_code.to_string(),
                    };

                    if key_event.modifiers.contains(KeyModifiers::CONTROL) {
                        key_str = format!("<c-{}>", key_str).to_string();
                    } else if key_str.len() > 1 {
                        key_str = format!("<{}>", key_str).to_string();
                    }
                    self.get_state().key_combination.push_str(&key_str);
                    return Ok(true);
                }
            }
        }
        return Ok(false);
    }

    fn run_generic_action(
        &mut self,
        action: &Action,
        height: usize,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
        state: &mut ListState,
    ) -> Result<(), Error> {
        match action {
            Action::Reload => self.reload()?,
            Action::Up => state.select_previous(),
            Action::Down => state.select_next(),
            Action::First => state.select_first(),
            Action::Last => state.select_last(),
            Action::Quit => self.get_state().quit = true,
            Action::HalfPageUp => state.scroll_up_by(height as u16 / 3),
            Action::HalfPageDown => state.scroll_down_by(height as u16 / 3),
            Action::CenterVertically => {
                let mut idx = state.selected().ok_or_else(|| Error::StateIndexError)?;
                *state = if idx > height / 2 {
                    idx = idx - height / 2;
                    state.clone().with_offset(idx)
                } else {
                    state.clone().with_offset(0)
                };
            }
            Action::Command(command_type, command) => {
                match command_type {
                    CommandType::Sync | CommandType::SyncQuit => terminal.clear()?,
                    _ => (),
                }
                let (file, rev) = self.get_file_and_rev()?;
                Self::run_command(terminal, &command_type, command.to_string(), file, rev)?;
                match command_type {
                    CommandType::SyncQuit => self.get_state().quit = true,
                    CommandType::Sync => self.reload()?,
                    _ => (),
                }
            }
            Action::Search => {
                self.get_state().is_searching = true;
            }
            Action::None => (),
            action => {
                return Err(Error::GlobalError(format!(
                    "cannot run `{:?}` in this context",
                    action
                )));
            }
        }
        Ok(())
    }

    fn handle_user_input(&mut self) -> Result<Option<Action>, Error> {
        // TODO: unwrap
        if !self.key_pressed()? {
            return Ok(None);
        }
        self.get_state().notif = None;

        // Compute command to run from config
        let keys = self.get_state().key_combination.clone();
        if keys == "" {
            return Ok(None);
        }

        let bindings = self.get_state().config.bindings.clone();

        let mut potential = false;
        for field in [self.get_mapping_fields().as_slice(), &[("global", true)]].concat() {
            if !field.1 {
                continue;
            }
            if let Some(mode_hotkeys) = bindings.get(field.0) {
                for (key_combination, action) in mode_hotkeys {
                    if *action == Action::None {
                        continue;
                    }
                    if *key_combination == keys {
                        return Ok(Some(action.clone()));
                    }
                    if key_combination.starts_with(&keys) {
                        potential = true;
                    }
                }
            }
        }
        if !potential {
            self.get_state().key_combination = "".to_string();
        }
        Ok(None)
    }

    fn run_command(
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
        command_type: &CommandType,
        mut command: String,
        filename: Option<String>,
        revision: Option<String>,
    ) -> Result<(), Error> {
        if let Some(file) = filename {
            command = command.replace("%(file)", &file);
        }
        if let Some(rev) = revision {
            command = command.replace("%(rev)", &rev);
        }

        let mut bash_proc = Command::new("bash");
        let proc = bash_proc.args(["-c", &command]);

        match command_type {
            CommandType::Async => {
                proc.stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .expect("Failed to execute command");
            }
            _ => {
                disable_raw_mode()?;
                terminal.show_cursor()?;
                let mut child = proc.spawn().expect("Failed to start git commit");
                child.wait().expect("Failed to wait for git commit");
                enable_raw_mode()?;
            }
        }
        Ok(())
    }
}
