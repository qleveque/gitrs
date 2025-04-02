extern crate crossterm;
extern crate ratatui;
extern crate syntect;

mod blame_app;
mod config;
mod git;
mod input;
mod show_app;
mod status_app;
mod ui;

use std::env;

use config::parse_gitrs_config;

use blame_app::blame_app;
use show_app::show_app;
use status_app::status_app;

use ratatui::{backend::CrosstermBackend, Terminal};

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::stdout;

use std::io;

fn main() -> io::Result<()> {
    // let path = Path::new("/mnt/c/Users/qleve/Documents/prog/git/neovim");
    // let _ = env::set_current_dir(path);
    let mut args = env::args();
    args.next();

    // init
    let config = parse_gitrs_config();

    let command = match args.next() {
        Some(first_param) => first_param,
        None => "status".to_string(),
    };

    enable_raw_mode()?;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    let ret = match command.as_str() {
        "status" => status_app(&config, &mut terminal),
        "blame" => {
            if let Some(file) = args.next() {
                let line = match args.next() {
                    Some(str) => str.parse::<usize>().unwrap_or(1),
                    None => 1,
                };
                blame_app(&config, &mut terminal, file, None, line)
            } else {
                Ok(())
            }
        }
        "show" => {
            show_app(&config, &mut terminal, args.next())
        },
        _ => Ok(()),
    };

    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    disable_raw_mode()?;
    return ret;
}
