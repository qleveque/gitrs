use std::{
    collections::HashMap,
    env,
    io::{BufRead, BufReader},
    process::{exit, ChildStdout, Command, Stdio},
};

use crate::{config::Config, errors::Error};

#[derive(Debug, Clone, Copy, PartialEq, Ord, PartialOrd)]
#[repr(u8)]
pub enum FileStatus {
    None = 0,
    Unmerged = 1,
    New = 2,
    Modified = 3,
    Deleted = 4,
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

#[derive(PartialEq, Clone, Copy)]
pub enum GitOp {
    Add,
    Restore,
    RmCached,
}

#[derive(Clone)]
pub struct GitFile {
    pub unstaged_status: FileStatus,
    pub staged_status: FileStatus,
    init_unstaged_status: FileStatus,
    init_staged_status: FileStatus,
}

pub struct CommitRef {
    pub hash: String,
    pub author: String,
    pub date: String,
}

pub struct Stash {
    pub date: String,
    pub title: String,
}

impl CommitRef {
    pub fn new(hash: String, author: String, date: String) -> Self {
        CommitRef { hash, author, date }
    }
}

#[derive(Clone)]
pub struct Commit {
    pub metadata: String,
    pub files: Vec<(FileStatus, String)>,
    pub hash: String,
}

impl Commit {
    pub fn new(metadata: String, files: Vec<(FileStatus, String)>, hash: String) -> Self {
        Commit {
            metadata,
            files,
            hash,
        }
    }
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

    fn git_op(&self) -> Option<GitOp> {
        if self.init_unstaged_status != FileStatus::None
            && self.unstaged_status == FileStatus::None
            && self.staged_status != FileStatus::None
        {
            return Some(GitOp::Add);
        } else if self.init_staged_status != FileStatus::None
            && self.staged_status == FileStatus::None
        {
            match self.unstaged_status {
                FileStatus::New => return Some(GitOp::RmCached),
                FileStatus::None => return None,
                _ => return Some(GitOp::Restore),
            }
        }
        return None;
    }

    fn reinit(&mut self) {
        self.init_staged_status = self.staged_status;
        self.init_unstaged_status = self.unstaged_status;
    }
}

pub fn git_status_output(config: &Config) -> String {
    let output = Command::new(config.git_exe.clone())
        .args(["status", "--short", "--no-renames"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute git command");

    let stdout = output.stdout.expect("Failed to capture stdout");
    let reader = BufReader::new(stdout);

    let lines = reader.lines().filter_map(Result::ok);
    lines.collect::<Vec<String>>().join("\n")
}

pub fn git_blame_output(file: String, revision: Option<String>, config: &Config) -> String {
    let mut args: Vec<String> = vec!["blame".to_string(), file.clone()];
    if let Some(rev) = revision {
        args.push(rev.clone());
    }
    let output = Command::new(config.git_exe.clone())
        .args(args)
        .output()
        .expect("Failed to execute git blame");

    String::from_utf8_lossy(&output.stdout)
        .to_string()
        .replace("\t", "    ")
}

pub fn git_parse_commit(output: &String) -> Result<Commit, Error> {
    let mut lines = output.lines().map(String::from);
    let mut metadata: Vec<String> = Vec::new();

    // Parse commit hash
    let line = lines.next().ok_or_else(|| Error::GitParsingError)?;
    let commit_hash = line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| Error::GitParsingError)?;
    metadata.push(line.clone());

    // Read all metadata
    while let Some(line) = lines.next() {
        if line.len() == 0 {
            metadata.push("".to_string());
            break;
        }
        metadata.push(line.to_string());
    }

    // Read commit message and files
    let mut parsing_files = false;
    let mut files: Vec<(FileStatus, String)> = Vec::new();

    while let Some(line) = lines.next() {
        if !parsing_files {
            if !line.chars().next().unwrap_or(' ').is_whitespace() {
                parsing_files = true;
            } else {
                metadata.push(line.to_string());
            }
        }
        if parsing_files {
            let status = match line.chars().next() {
                Some('M') => FileStatus::Modified,
                Some('A') => FileStatus::New,
                Some('D') => FileStatus::Deleted,
                _ => break,
            };
            let filename = line
                .split('\t')
                .nth(1)
                .ok_or_else(|| Error::GitParsingError)?
                .to_string();
            files.push((status, filename));
        }
    }

    let commit = Commit::new(metadata.join("\n"), files, commit_hash.to_string());
    Ok(commit)
}

pub fn git_stash_output(config: &Config) -> Result<String, Error> {
    let args: Vec<String> = vec![
        "stash".to_string(),
        "list".to_string(),
        "--format=%cd\t%s".to_string(),
        "--date=iso-local".to_string(),
    ];
    let output = Command::new(config.git_exe.clone()).args(args).output()?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn git_files_output(revision: &Option<String>, config: &Config) -> Result<String, Error> {
    let mut args: Vec<String> = vec![
        "show".to_string(),
        "--decorate".to_string(),
        "--name-status".to_string(),
        "--stat".to_string(),
        "--no-renames".to_string(),
    ];
    if let Some(rev) = revision {
        args.push(rev.clone());
    }
    let output = Command::new(config.git_exe.clone()).args(args).output()?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn git_pager_output(
    command: &str,
    git_exe: String,
    user_args: Vec<String>,
) -> Result<BufReader<ChildStdout>, Error> {
    let mut args: Vec<String> = vec![command.to_string(), "--color=always".to_string()];
    args.extend(user_args);

    let command = Command::new(git_exe)
        .args(args)
        .stdout(Stdio::piped())
        .spawn()?;

    let stdout = command.stdout.ok_or_else(|| Error::GitParsingError)?;

    Ok(BufReader::new(stdout))
}

#[cfg(target_os = "linux")]
pub fn adapt_repo_root(root: String) -> String {
    if root.starts_with("C:/") {
        root.replacen("C:/", "/mnt/c/", 1)
    } else {
        root
    }
}

#[cfg(target_os = "windows")]
pub fn adapt_repo_root(root: String) -> String {
    root
}

pub fn set_git_dir(config: &Config) {
    // get git repo root dir
    let output = Command::new(config.git_exe.clone())
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .expect("Failed to execute git command");

    if !output.status.success() {
        eprintln!("Not inside a Git repository");
        exit(1);
    }
    let mut repo_root = String::from_utf8_lossy(&output.stdout);
    repo_root = adapt_repo_root(repo_root.to_string().clone()).into();
    env::set_current_dir(repo_root.trim()).expect("Failed to change directory");
}

pub fn git_add_restore(files: &mut HashMap<String, GitFile>, config: &Config) {
    for op in &[GitOp::Add, GitOp::Restore, GitOp::RmCached] {
        let mut files_to_op: Vec<String> = Vec::new();
        for (filename, git_file) in files.iter() {
            if Some(*op) == git_file.git_op() {
                files_to_op.push(filename.clone());
            }
        }
        if files_to_op.len() == 0 {
            continue;
        }
        let args = match *op {
            GitOp::Add => vec!["add"],
            GitOp::Restore => vec!["restore", "--staged"],
            GitOp::RmCached => vec!["rm", "--cached"],
        };
        let mut git_add_output = Command::new(config.git_exe.clone())
            .args(&args)
            .args(files_to_op.iter().map(|s| s.as_str()))
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute git command");
        git_add_output
            .wait()
            .expect("Failed to wait on git command");
    }

    for (_, git_file) in files {
        git_file.reinit();
    }
}
