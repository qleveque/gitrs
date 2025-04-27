mod action;
mod app;
mod app_state;
mod blame_app;
mod config;
mod errors;
mod git;
mod log_app;
mod files_app;
mod status_app;
mod pager_widget;
mod ui;

use std::io::{self, stdout};

use blame_app::BlameApp;
use clap::{Parser, Subcommand};

use errors::Error;
use log_app::{PagerApp, PagerCommand};
use files_app::FilesApp;
use status_app::StatusApp;

use app::GitApp;

use ratatui::{backend::CrosstermBackend, Terminal};

use crossterm::{
    execute,
    style::Stylize,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

#[derive(Parser)]
#[command(name = "gitrs", version, about = "A fast, intuitive Git TUI written in Rust", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Status view
    Status,

    /// Blame view
    Blame {
        /// File to blame
        file: String,

        /// Line number to focus on
        #[arg(default_value_t = 1)]
        line: usize,
    },

    /// Files view
    Files {
        /// Optional revision hash or reference
        revision: Option<String>,
    },
    /// Log view
    #[command(allow_hyphen_values = true)]
    Log {
        /// Arguments passed to git log
        args: Vec<String>
    },
    /// Show view
    #[command(allow_hyphen_values = true)]
    Show {
        /// Arguments passed to git log
        args: Vec<String>
    },
}

fn app(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>, cli: Cli) -> Result<(), Error> {
    let ret = match cli.command {
        Commands::Status => StatusApp::new()?.run(terminal),
        Commands::Blame { file, line } => BlameApp::new(file, None, line)?.run(terminal),
        Commands::Files { revision } => FilesApp::new(revision)?.run(terminal),
        Commands::Log { args } => PagerApp::new(PagerCommand::Log, args)?.run(terminal),
        Commands::Show { args } => PagerApp::new(PagerCommand::Show, args)?.run(terminal),
    };
    ret
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    execute!(stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;

    let ret = app(&mut terminal, cli);

    disable_raw_mode()?;
    terminal.show_cursor()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    if let Err(err) = ret {
        eprintln!("{} {}", "error:".red().bold(), err.to_string().white());
        std::process::exit(1);
    }
    Ok(())
}
