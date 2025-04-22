mod action;
mod app;
mod blame_app;
mod config;
mod errors;
mod git;
mod show_app;
mod status_app;
mod app_state;

use std::io::{self, stdout};

use blame_app::BlameApp;
use clap::{Parser, Subcommand};

use errors::Error;
use show_app::ShowApp;
use status_app::StatusApp;

use app::GitApp;

use ratatui::{backend::CrosstermBackend, Terminal};

use crossterm::{
    execute,
    style::Stylize,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
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

    /// Show a git revision (commit, tag, etc)
    Show {
        /// Optional revision hash or reference
        revision: Option<String>,
    },
}

fn app(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<(), Error> {
    let cli = Cli::parse();
    let _ = match cli.command {
        Commands::Status => StatusApp::new()?.run(terminal),
        Commands::Blame { file, line } => {
            BlameApp::new(file, None, line)?.run(terminal)
        }
        Commands::Show { revision } => ShowApp::new(revision)?.run(terminal),
    };
    Ok(())
}

fn main() -> io::Result<()> {
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let ret = app(&mut terminal);

    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    if let Err(err) = ret {
        eprintln!("{} {}", "error:".red().bold(), err.to_string().white());
        std::process::exit(1);
    }
    Ok(())
}
