use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader},
};

use crate::action::Action;

pub type KeyBindings = HashMap<String, Vec<(String, Action)>>;

pub struct Config {
    pub scroll_off: usize,
    pub git_exe: String,
    pub bindings: KeyBindings,
}

impl Default for Config {
    fn default() -> Self {
        let bindings: KeyBindings = [
            (
                "global".to_string(),
                vec![
                    ("k".to_string(), Action::Up),
                    ("<up>".to_string(), Action::Up),
                    ("j".to_string(), Action::Down),
                    ("<down>".to_string(), Action::Down),
                    ("r".to_string(), Action::Reload),
                    ("gg".to_string(), Action::First),
                    ("<home>".to_string(), Action::First),
                    ("G".to_string(), Action::Last),
                    ("<end>".to_string(), Action::Last),
                    ("q".to_string(), Action::Quit),
                    ("<esc>".to_string(), Action::Quit),
                    ("<c-u>".to_string(), Action::HalfPageUp),
                    ("<pgup>".to_string(), Action::HalfPageUp),
                    ("<c-d>".to_string(), Action::HalfPageDown),
                    ("<pgdown>".to_string(), Action::HalfPageDown),
                    ("zz".to_string(), Action::CenterVertically),
                ],
            ),
            (
                "status".to_string(),
                vec![
                    ("t".to_string(), Action::StageUnstageFile),
                    ("<space>".to_string(), Action::StageUnstageFile),
                    ("T".to_string(), Action::StageUnstageFiles),
                    ("<cr>".to_string(), Action::StageUnstageFiles),
                    ("<tab>".to_string(), Action::SwitchView),
                    ("K".to_string(), Action::FocusUnstagedView),
                    ("J".to_string(), Action::FocusStagedView),
                ],
            ),
            (
                "blame".to_string(),
                vec![
                    ("l".to_string(), Action::NextCommitBlame),
                    ("<right>".to_string(), Action::NextCommitBlame),
                    ("h".to_string(), Action::PreviousCommitBlame),
                    ("<left>".to_string(), Action::PreviousCommitBlame),
                    ("<cr>".to_string(), Action::ShowCommit),
                ],
            ),
        ]
        .into_iter()
        .collect();
        Config {
            scroll_off: 2,
            git_exe: "git".to_string(),
            bindings,
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
                    let action_str = parts[3].to_string();

                    match action_str.parse::<Action>() {
                        Ok(action) => {
                            config
                                .bindings
                                .entry(mode)
                                .or_insert_with(Vec::new)
                                .push((key, action));
                        }
                        Err(e) => println!("Failed to parse action: {:?}", e),
                    };
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
