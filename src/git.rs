use std::{collections::HashMap, env, path::Path, process::Command};

use git2::{Commit, Repository, Status, StatusOptions};

use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Ord, PartialOrd)]
#[repr(u8)]
pub enum FileStatus {
    None = 0,
    Unmerged = 1,
    Modified = 2,
    Deleted = 3,
    New = 4,
}
impl Eq for FileStatus {}

impl FileStatus {
    pub fn character(&self) -> char {
        match self {
            FileStatus::Modified => '>',
            FileStatus::Deleted => '-',
            FileStatus::New => '+',
            FileStatus::Unmerged => '@',
            FileStatus::None => panic!("None file status should not be displayed"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum StagedStatus {
    Unstaged = 0,
    Staged = 1,
}

pub struct GitFile {
    pub unstaged_status: FileStatus,
    pub staged_status: FileStatus,
    pub init_unstaged_status: FileStatus,
    pub init_staged_status: FileStatus,
}

impl GitFile {
    pub fn new(unstaged_status: FileStatus, staged_status: FileStatus) -> Self {
        GitFile {
            unstaged_status,
            staged_status,
            init_unstaged_status: unstaged_status,
            init_staged_status: staged_status,
        }
    }

    pub fn set_status(&mut self, new_unstaged_status: FileStatus, new_staged_status: FileStatus) {
        self.unstaged_status = new_unstaged_status;
        self.staged_status = new_staged_status;
    }
}

pub fn git_blame_output(file: String, revision: Option<String>, config: &Config) -> String {
    let mut args: Vec<String> = vec!["blame".to_string(), file.clone(), "--no-renames".to_string()];
    if let Some(rev) = revision {
        args.push(rev.clone());
    }
    let output = Command::new(config.git_exe.clone())
        .args(args)
        .output()
        .expect("Failed to execute git blame");

    String::from_utf8_lossy(&output.stdout).to_string()
}

pub fn set_git_dir() {
    let repo = Repository::discover(".").unwrap();
    let repo_root = repo.path().parent().unwrap_or_else(|| Path::new("."));
    let _ = env::set_current_dir(repo_root);
}

pub fn ref_to_commit(repo: &Repository, commit_ref: Option<String>) -> Option<Commit> {
    commit_ref
        .as_ref()
        .and_then(|hash| repo.revparse_single(hash).ok())
        .and_then(|obj| obj.as_commit().map(|commit| Some(commit.clone())))
        .unwrap_or(None)
}


pub fn compute_git_files(repo: &Repository, files: &mut HashMap<String, GitFile>, config: &Config) {
    if false {
        // TODO
        // too slow on windows index
        let mut status_options = StatusOptions::new();
        status_options.include_untracked(true);
        let statuses = repo.statuses(Some(&mut status_options)).unwrap();
        for entry in statuses.iter() {
            let status = entry.status();
            let path = entry.path().unwrap();

            let unstaged_status = if status.contains(Status::CONFLICTED) {
                FileStatus::Unmerged
            } else if status.contains(Status::WT_NEW) {
                FileStatus::New
            } else if status.contains(Status::WT_MODIFIED) {
                FileStatus::Modified
            } else if status.contains(Status::WT_DELETED) {
                FileStatus::Deleted
            } else {
                FileStatus::None
            };

            let staged_status = if status.contains(Status::INDEX_NEW) {
                FileStatus::New
            } else if status.contains(Status::INDEX_MODIFIED) {
                FileStatus::Modified
            } else if status.contains(Status::INDEX_DELETED) {
                FileStatus::Deleted
            } else {
                FileStatus::None
            };

            let git_file = GitFile::new(unstaged_status, staged_status);
            files.insert(path.to_string(), git_file);
        }
    } else {
        let output = Command::new(config.git_exe.clone())
            .args(["status", "--short", "--no-renames"])
            .output()
            .expect("Failed to execute git status");
        let stdout = std::str::from_utf8(&output.stdout).expect("Invalid UTF-8 output");
        
        for line in stdout.lines() {
            let mut line = line.to_string();
            let first_char = line.remove(0);
            let second_char = line.remove(0);
            let _ = line.remove(0);
            let unstaged_status = match second_char {
                'U' => FileStatus::Unmerged,
                'M' => FileStatus::Modified,
                'D' => FileStatus::Deleted,
                '?' => FileStatus::New,
                _ => FileStatus::None,
            };
            let staged_status = match first_char {
                'M' => FileStatus::Modified,
                'D' => FileStatus::Deleted,
                'A' => FileStatus::New,
                _ => FileStatus::None,
            };
            let git_file = GitFile::new(unstaged_status, staged_status);
            files.insert(line, git_file);
        }
    }
}
