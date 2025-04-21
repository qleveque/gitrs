use std::process::{Command, Stdio};

use ratatui::{prelude::CrosstermBackend, widgets::ListState, Frame, Terminal};

use crate::{
    action::{Action, CommandType},
    config::Config,
    errors::Error,
    input::InputManager,
};

pub trait GitApp {
    fn draw(&mut self, frame: &mut Frame);

    fn on_exit(&mut self) -> Result<(), Error> {
        Ok(())
    }
    fn reload(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn get_config_fields(&mut self) -> Vec<(&str, bool)>;
    fn get_file_and_rev(&self) -> (Option<String>, Option<String>);

    fn run_action(
        &mut self,
        action: &Action,
        _terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<bool, Error>;

    fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
        config: &Config,
    ) -> Result<(), Error> {
        let mut input_manager = InputManager::new();
        loop {
            let _ = terminal.draw(|f| {
                self.draw(f);
            });
            let opt_action = self.read_user_action(&mut input_manager, config)?;
            if let Some(action) = opt_action {
                let quit = self.run_action(&action, terminal)?;
                if quit {
                    break;
                }
            }
        }
        self.on_exit()?;
        Ok(())
    }

    fn run_generic_action(
        &mut self,
        action: &Action,
        height: usize,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
        state: &mut ListState,
    ) -> Result<bool, Error> {
        let mut quit = false;
        match action {
            Action::Reload => self.reload()?,
            Action::Up => state.select_previous(),
            Action::Down => state.select_next(),
            Action::First => state.select_first(),
            Action::Last => state.select_last(),
            Action::Quit => quit = true,
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
                //TODO: remove unwrap
                match command_type {
                    CommandType::Sync | CommandType::SyncQuit => terminal.clear()?,
                    _ => (),
                }
                let (file, rev) = self.get_file_and_rev();
                // TODO: improve
                self.reload()?;
                Self::run_command(&command_type, command.to_string(), file, rev);
                // TODO: improve
                self.reload()?;
                match command_type {
                    CommandType::SyncQuit => quit = true,
                    _ => (),
                }
            }
            Action::None => (),
            _ => {
                println!("Unknown command in this context");
            }
        }
        return Ok(quit);
    }

    fn read_user_action(
        &mut self,
        input_manager: &mut InputManager,
        config: &Config,
    ) -> Result<Option<Action>, Error> {
        // TODO: unwrap
        if !input_manager.key_pressed()? {
            return Ok(None);
        }

        // Compute command to run from config
        let keys = input_manager.key_combination.clone();
        if keys == "" {
            return Ok(None);
        }
        for field in [self.get_config_fields().as_slice(), &[("global", true)]].concat() {
            if !field.1 {
                continue;
            }
            if let Some(mode_hotkeys) = config.bindings.get(field.0) {
                for (key_combination, action) in mode_hotkeys {
                    if *action == Action::None {
                        continue;
                    }
                    if *key_combination == keys {
                        return Ok(Some(action.clone()));
                    }
                    if key_combination.starts_with(&keys) {
                        input_manager.reset_key_combination = false;
                    }
                }
            }
        }
        Ok(None)
    }

    fn run_command(
        command_type: &CommandType,
        mut command: String,
        filename: Option<String>,
        revision: Option<String>,
    ) {
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
                let mut child = proc.spawn().expect("Failed to start git commit");
                child.wait().expect("Failed to wait for git commit");
            }
        }
    }
}
