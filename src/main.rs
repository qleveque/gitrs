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

use std::{
    io::{self, stdout, ErrorKind},
    path::Path,
};

use clap::{Parser, Subcommand};

use config::parse_gitrs_config;

use blame_app::blame_app;
use show_app::show_app;
use status_app::status_app;

use ratatui::{backend::CrosstermBackend, Terminal};

use crossterm::{
    execute,
    style::Stylize,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

#[derive(Parser)]
#[command(name = "gitrs", version, about = "A TUI for git status, blame, show", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show git status
    Status,

    /// Show git blame
    Blame {
        /// File to blame
        file: String,

        /// Line number to focus on
        #[arg(default_value_t = 1)]
        line: usize,
    },

    /// Show a git object (commit, tag, etc)
    Show {
        /// Optional object hash or reference
        object: Option<String>,
    },
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let config = parse_gitrs_config();

    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let ret = match cli.command {
        Commands::Status => status_app(&config, &mut terminal),
        Commands::Blame { file, line } => {
            if !Path::new(&file).exists() {
                Err(io::Error::new(
                    ErrorKind::NotFound,
                    format!("error: file '{}' does not exist", file),
                ))
            } else {
                blame_app(&config, &mut terminal, file, None, line)
            }
        }
        Commands::Show { object } => show_app(&config, &mut terminal, object),
    };

    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    disable_raw_mode()?;
    if let Err(err) = ret {
        eprintln!("{} {}", "error:".red().bold(), err.to_string().white());
        std::process::exit(1);
    }
    ret
}
