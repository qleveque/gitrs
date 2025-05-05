mod action;
mod app;
mod app_state;
mod blame_app;
mod config;
mod errors;
mod git;
mod pager_app;
mod pager_widget;
mod show_app;
mod stash_app;
mod status_app;
mod ui;

use std::io::{self, stdout};

use atty::Stream;
use blame_app::BlameApp;
use clap::{Parser, Subcommand};

use errors::Error;
use pager_app::{PagerApp, PagerCommand};
use show_app::ShowApp;
use stash_app::StashApp;
use status_app::StatusApp;

use app::GitApp;

use ratatui::{backend::CrosstermBackend, Terminal};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
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

    /// Show view
    Show {
        /// Optional revision hash or reference
        revision: Option<String>,
    },
    /// Log view
    #[command(allow_hyphen_values = true)]
    Log {
        /// Arguments passed to git log
        args: Vec<String>,
    },
    /// Diff view
    #[command(allow_hyphen_values = true)]
    Diff {
        /// Arguments passed to git diff
        args: Vec<String>,
    },
    /// Stash view
    Stash,
}

fn app(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>, cli: Cli) -> Result<(), Error> {
    let ret = match cli.command {
        Commands::Status => StatusApp::new()?.run(terminal),
        Commands::Blame { file, line } => BlameApp::new(file, None, line)?.run(terminal),
        Commands::Show { revision } => ShowApp::new(revision)?.run(terminal),
        Commands::Log { args } => PagerApp::new(Some(PagerCommand::Log(args)))?.run(terminal),
        Commands::Diff { args } => PagerApp::new(Some(PagerCommand::Diff(args)))?.run(terminal),
        Commands::Stash => StashApp::new()?.run(terminal),
    };
    ret
}

fn prepare_terminal() -> Result<Terminal<CrosstermBackend<std::io::Stdout>>, io::Error> {
    let backend = CrosstermBackend::new(stdout());
    let terminal = Terminal::new(backend)?;
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    execute!(stdout(), EnableMouseCapture)?;
    Ok(terminal)
}

fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) -> Result<(), io::Error> {
    disable_raw_mode()?;
    terminal.show_cursor()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    execute!(stdout(), DisableMouseCapture)?;
    Ok(())
}

fn main() -> io::Result<()> {
    let ret = if atty::is(Stream::Stdin) {
        let cli = Cli::parse();
        let mut terminal = prepare_terminal()?;
        let ret = app(&mut terminal, cli);
        restore_terminal(&mut terminal)?;
        ret
    } else {
        // use the application as a pager
        let mut terminal = prepare_terminal()?;
        let ret = match PagerApp::new(None) {
            Ok(mut pager_app) => pager_app.run(&mut terminal),
            Err(e) => Err(e),
        };
        restore_terminal(&mut terminal)?;
        ret
    };

    if let Err(err) = ret {
        eprintln!("{} {}", "error:".red().bold(), err.to_string().white());
        std::process::exit(1);
    }
    Ok(())
}
