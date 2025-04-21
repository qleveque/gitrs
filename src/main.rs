mod action;
mod app;
mod blame_app;
mod config;
mod errors;
mod git;
mod input;
mod show_app;
mod status_app;

use std::io::{self, stdout};

use blame_app::BlameApp;
use clap::{Parser, Subcommand};

use config::parse_gitrs_config;

use errors::Error;
use show_app::ShowApp;
use status_app::StatusApp;

use app::GitApp;

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

    /// Show a git revision (commit, tag, etc)
    Show {
        /// Optional revision hash or reference
        revision: Option<String>,
    },
}

fn app(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<(), Error> {
    let config = parse_gitrs_config()?;
    let cli = Cli::parse();
    let _ = match cli.command {
        Commands::Status => StatusApp::new(&config)?.run(terminal, &config),
        Commands::Blame { file, line } => {
            BlameApp::new(&config, file, None, line)?.run(terminal, &config)
        }
        Commands::Show { revision } => ShowApp::new(&config, revision)?.run(terminal, &config),
    };

    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    disable_raw_mode()?;
    Ok(())
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let ret = app(&mut terminal);

    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    disable_raw_mode()?;

    if let Err(err) = ret {
        eprintln!("{} {}", "error:".red().bold(), err.to_string().white());
        std::process::exit(1);
    }
    Ok(())
}
