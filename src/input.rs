use std::io;

use crossterm::event::{self, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::{prelude::CrosstermBackend, widgets::ListState, Terminal};

use crate::config::run_command;

pub fn basic_movements(
    code: KeyCode,
    modifiers: KeyModifiers,
    state: &mut ListState,
    height: usize,
    quit: &mut bool
) -> bool {
    let ctrl = modifiers.contains(KeyModifiers::CONTROL);
    match code {
        KeyCode::Char('k') | KeyCode::Up => {
            state.select_previous();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            state.select_next();
        }
        KeyCode::Char('g') | KeyCode::Home => {
            state.select_first();
        }
        KeyCode::Char('G') | KeyCode::End => {
            state.select_last();
        }
        KeyCode::Char('q') | KeyCode::Backspace => {
            *quit = true;
        }
        KeyCode::Char('d') if ctrl => {
            state.scroll_down_by(height as u16 / 3);
        }
        KeyCode::Char('u') if ctrl => {
            state.scroll_up_by(height as u16 / 3);
        }
        KeyCode::Char('z') => {
            *state = if state.selected().unwrap() > height / 2 {
                let idx = state.selected().unwrap() - height / 2;
                state.clone().with_offset(idx)
            } else {
                state.clone().with_offset(0)
            };
        }
        _ => {
            return false;
        }
    }
    true
}

pub struct InputManager {
    pub key_event: KeyEvent,
    pub key_combination: String,
    pub reset_key_combination: bool,
}

impl InputManager {
    pub fn new() -> Self {
        InputManager {
            key_event: KeyEvent {
                code: KeyCode::Null,
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Release,
                state: KeyEventState::NONE,
            },
            key_combination: "".to_string(),
            reset_key_combination: true,
        }
    }
    pub fn key_pressed(&mut self) -> Result<bool, std::io::Error> {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let event::Event::Key(key_event) = event::read()? {
                if key_event.kind == KeyEventKind::Press {
                    match self.reset_key_combination {
                        true => self.key_combination = "".to_string(),
                        false => self.reset_key_combination = true,
                    };
                    self.key_event = key_event;
                    self.key_combination = format!(
                        "{}{}",
                        self.key_combination,
                        self.key_event.code.to_string()
                    );
                    return Ok(true);
                }
            }
        }
        return Ok(false);
    }

    pub fn handle_generic_user_input(
        &mut self,
        state: &mut ListState,
        height: usize,
        quit: &mut bool,
        opt_command: Option<String>,
        file: Option<String>,
        rev: Option<String>,
        potential: bool,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> io::Result<bool> {
        if potential {
            self.reset_key_combination = false;
            return Ok(true);
        }
        if let Some(command) = opt_command {
            let mut clear = false;
            run_command(
                command,
                quit,
                &mut clear,
                file,
                rev,
            );
            if clear {
                terminal.clear()?;
            }
            return Ok(true);
        }
        let r = basic_movements(
            self.key_event.code,
            self.key_event.modifiers,
            state,
            height,
            quit
        );
        return Ok(r);
    }

}
