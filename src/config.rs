use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};

use crate::git::{FileStatus, GitFile, StagedStatus};

pub type KeyBindings = HashMap<String, Vec<(String, String)>>;

pub struct Config {
    pub scroll_off: usize,
    pub git_exe: String,
    pub bindings: KeyBindings,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            scroll_off: 2,
            git_exe: "git".to_string(),
            bindings: HashMap::new(),
        }
    }
}

pub fn parse_gitrs_config() -> Config {
    // TODO: better error handling
    let path = std::env::var("HOME").unwrap() + "/.gitrsrc";
    let result = fs::File::open(path);

    let mut config: Config = Config::default();

    if let Ok(file) = result {
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line.unwrap();
            let keyword = line
                .split_once(' ')
                .map(|(first, _)| first.to_string())
                .unwrap_or(line.to_string());

            match keyword.as_str() {
                "map" => {
                    let parts: Vec<&str> = line.splitn(4, ' ').collect();
                    if parts.len() < 4 {
                        continue;
                    }
                    let mode = parts[1].to_string();
                    let key = parts[2].to_string();
                    let command = parts[3].to_string();

                    config
                        .bindings
                        .entry(mode)
                        .or_insert_with(Vec::new)
                        .push((key, command));
                }
                "set" => {
                    let parts: Vec<&str> = line.splitn(3, ' ').collect();
                    if parts.len() < 3 {
                        continue;
                    }
                    let key = parts[1].to_string();
                    let value = parts[2].to_string();
                    match key.as_str() {
                        "scrolloff" => {
                            let number: Result<usize, _> = value.parse();
                            if let Ok(so) = number {
                                config.scroll_off = so
                            }
                        }
                        "git" => config.git_exe = value,
                        _ => (),
                    }
                }
                _ => (),
            }
        }
    }

    config
}

pub fn get_command_to_run(
    config: &Config,
    keys: String,
    fields: &mut Vec<(&str, bool)>,
) -> (Option<String>, bool) {
    if keys == "" {
        return (None, false);
    }
    fields.push(("global", true));
    let mut potential = false;
    for field in fields {
        if !field.1 {
            continue;
        }
        if let Some(mode_hotkeys) = config.bindings.get(field.0) {
            for (key_combination, command) in mode_hotkeys {
                if *key_combination == keys {
                    return (Some(command.clone()), false);
                }
                if key_combination.starts_with(&keys) {
                    potential = true;
                }
            }
        }
    }
    (None, potential)
}

pub fn get_status_command_to_run(
    config: &Config,
    keys: String,
    git_file: &GitFile,
    staged_status: StagedStatus,
) -> (Option<String>, bool) {
    let mut fields: Vec<(&str, bool)> = vec![
        (
            "unmerged",
            staged_status == StagedStatus::Unstaged
                && git_file.unstaged_status == FileStatus::Unmerged,
        ),
        (
            "untracked",
            staged_status == StagedStatus::Unstaged && git_file.unstaged_status == FileStatus::New,
        ),
        ("staged", staged_status == StagedStatus::Staged),
        ("unstaged", staged_status == StagedStatus::Unstaged),
        ("status", true),
    ];
    get_command_to_run(config, keys, &mut fields)
}

pub fn get_show_command_to_run(config: &Config, keys: String) -> (Option<String>, bool) {
    let mut fields: Vec<(&str, bool)> = vec![("show", true)];
    get_command_to_run(config, keys, &mut fields)
}

pub fn get_blame_command_to_run(config: &Config, keys: String) -> (Option<String>, bool) {
    let mut fields: Vec<(&str, bool)> = vec![("blame", true)];
    get_command_to_run(config, keys, &mut fields)
}

pub fn run_command(
    mut command: String,
    filename: Option<String>,
    revision: Option<String>,
) {
    let command_type = command.chars().next().unwrap();
    command = command[1..].to_string();

    if let Some(file) = filename {
        command = command.replace("%(file)", &file);
    }
    if let Some(rev) = revision {
        command = command.replace("%(rev)", &rev);
    }

    let mut bash_proc = Command::new("bash");
    let proc = bash_proc.args(["-c", &command]);

    match command_type {
        '@' => {
            proc.stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .expect("Failed to execute command");
        }
        '!' | '>' => {
            let mut child = proc.spawn().expect("Failed to start git commit");
            child.wait().expect("Failed to wait for git commit");
        }
        _ => (),
    }
}
