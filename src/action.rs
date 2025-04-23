use std::str::FromStr;

use crate::errors::Error;

#[derive(Clone, PartialEq, Debug)]
pub enum CommandType {
    Async,
    Sync,
    SyncQuit,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Action {
    Reload,
    Up,
    Down,
    First,
    Last,
    Quit,
    HalfPageUp,
    HalfPageDown,
    CenterVertically,
    Search,
    SearchReverse,
    NextSearchResult,
    PreviousSearchResult,
    TypeCommand,
    Command(CommandType, String),
    StageUnstageFile,
    StageUnstageFiles,
    SwitchView,
    FocusUnstagedView,
    FocusStagedView,
    ShowCommit,
    NextCommitBlame,
    PreviousCommitBlame,
    None,
}

impl FromStr for Action {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "up" => Ok(Action::Up),
            "down" => Ok(Action::Down),
            "reload" => Ok(Action::Reload),
            "first" => Ok(Action::First),
            "last" => Ok(Action::Last),
            "quit" => Ok(Action::Quit),
            "half_page_up" => Ok(Action::HalfPageUp),
            "half_page_down" => Ok(Action::HalfPageDown),
            "center" => Ok(Action::CenterVertically),
            "next_search_result" => Ok(Action::NextSearchResult),
            "previous_search_result" => Ok(Action::PreviousSearchResult),
            "type_command" => Ok(Action::TypeCommand),
            "search" => Ok(Action::Search),
            "search_reverse" => Ok(Action::SearchReverse),
            "stage_unstage_file" => Ok(Action::StageUnstageFile),
            "stage_unstage_files" => Ok(Action::StageUnstageFiles),
            "switch_view" => Ok(Action::SwitchView),
            "focus_unstaged_view" => Ok(Action::FocusUnstagedView),
            "focus_staged_view" => Ok(Action::FocusStagedView),
            "show_commit" => Ok(Action::ShowCommit),
            "next_commit_blame" => Ok(Action::NextCommitBlame),
            "previous_commit_blame" => Ok(Action::PreviousCommitBlame),
            "nop" => Ok(Action::None),
            cmd => {
                let command_type = match cmd.chars().next() {
                    Some('!') => CommandType::Sync,
                    Some('>') => CommandType::SyncQuit,
                    Some('@') => CommandType::Async,
                    _ => return Err(Error::ParseActionError(cmd.to_string())),
                };

                Ok(Action::Command(command_type, cmd[1..].to_string()))
            }
        }
    }
}
