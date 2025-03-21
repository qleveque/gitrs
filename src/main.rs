extern crate chrono;
extern crate crossterm;
extern crate git2;
extern crate ratatui;
extern crate syntect;
mod blame_app;
mod config;
mod git;
mod input;
mod log_app;
mod show_app;
mod status_app;
mod ui;

use config::parse_gitrs_config;
use std::{
    env,
    path::{Path, PathBuf},
};

use blame_app::blame_app;
use git::set_git_dir;
use log_app::log_app;
use show_app::show_app;
use status_app::status_app;

use clap::{Arg, ArgMatches, Command};
use ratatui::{backend::CrosstermBackend, Terminal};

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::stdout;

use std::io;

/// Parses a commit-ish and/or path argument, following `git log` behavior.
fn parse_commit_and_path(matches: &ArgMatches) -> (Option<String>, Option<String>) {
    let first = matches.get_one::<String>("first").map(String::as_str);
    let second = matches.get_one::<String>("second").map(String::as_str);

    match (first, second) {
        (Some(f), Some(s)) => {
            let relative_path = Path::new(s);
            let absolute_path: PathBuf = env::current_dir().unwrap().join(relative_path);
            let path_str = absolute_path.to_string_lossy().into_owned();
            (Some(f.to_string()), Some(path_str))
        } // Explicit revision and path
        (Some(f), None) => {
            let relative_path = Path::new(f);
            if Path::new(f).exists() {
                let absolute_path: PathBuf = env::current_dir().unwrap().join(relative_path);
                let path_str = absolute_path.to_string_lossy().into_owned();
                (None, Some(path_str)) // If it's a file, it's a path
            } else {
                (Some(f.to_string()), None) // Otherwise, it's a revision
            }
        }
        _ => (None, None), // No arguments
    }
}

/// Defines the common arguments used by `log`, `show`, and `blame`.
fn add_commit_and_path_args(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("first")
            .help("A commit-ish or a path")
            .index(1)
            .required(false),
    )
    .arg(
        Arg::new("second")
            .help("A path, if the first argument is a revision")
            .index(2)
            .required(false),
    )
}

fn main() -> io::Result<()> {
    // let path = Path::new("/mnt/c/Users/qleve/Documents/prog/git/neovim");
    // let _ = env::set_current_dir(path);
    let config = parse_gitrs_config();

    let matches = Command::new("git-clone")
        .subcommand(
            add_commit_and_path_args(Command::new("blame"))
            .arg(
                Arg::new("line")
                    .short('L')
                    .help("Specify a line")
                    .value_parser(clap::value_parser!(usize)),
            )
        )
        .subcommand(add_commit_and_path_args(Command::new("show")))
        .subcommand(
            add_commit_and_path_args(Command::new("log"))
            .arg(
                Arg::new("author")
                    .long("author")
                    .help("Filter by author name"),
            )
        )
        .subcommand(Command::new("status"))
        .get_matches();

    enable_raw_mode()?;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    let ret = match matches.subcommand() {
        Some(("status", _)) => {
            set_git_dir();
            status_app(&config, &mut terminal)
        }
        Some(("blame", sub_matches)) => {
            let (rev, path) = parse_commit_and_path(sub_matches);

            let line = sub_matches.get_one::<usize>("line").copied().unwrap_or(1);
            set_git_dir();
            if let Some(file) = path {
                // transform absolute to relative so that git.exe would not fail with C:/ files
                let cwd = env::current_dir()
                    .expect("Failed to get current directory")
                    .to_string_lossy()
                    .into_owned();
                let relative = file[cwd.len() + 1..].to_string();
                blame_app(&config, &mut terminal, relative, rev, line)
            } else {
                Ok(())
            }
        }
        Some(("show", sub_matches)) => {
            let (revision, _) = parse_commit_and_path(sub_matches);
            set_git_dir();
            show_app(&config, &mut terminal, revision)
        }
        Some(("log", sub_matches)) => {
            let (rev, path) = parse_commit_and_path(sub_matches);
            set_git_dir();
            let author_filter = sub_matches.get_one::<String>("author");
            log_app(&config, &mut terminal, path, rev, author_filter.cloned())
        }
        _ => Ok(()),
    };

    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    terminal.clear()?;

    disable_raw_mode()?;
    return ret;
}
