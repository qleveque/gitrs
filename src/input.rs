use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::widgets::ListState;

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
