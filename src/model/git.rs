use std::{
    collections::HashMap,
    env,
    io::{BufRead, BufReader},
    process::{ChildStdout, Command, Stdio},
    str::FromStr,
};

use crate::model::{config::Config, errors::Error};

#[derive(Debug, Clone, Copy, PartialEq, Ord, PartialOrd, Hash)]
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

impl FromStr for FileStatus {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "modified" => Ok(FileStatus::Modified),
            "new" => Ok(FileStatus::New),
            "deleted" => Ok(FileStatus::Deleted),
            "conflicted" => Ok(FileStatus::Unmerged),
            _ => Err(Error::ParseMappingScope(s.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Hash, Eq)]
#[repr(u8)]
pub enum StagedStatus {
    Unstaged = 0,
    Staged = 1,
}

impl FromStr for StagedStatus {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "unstaged" => Ok(StagedStatus::Unstaged),
            "staged" => Ok(StagedStatus::Staged),
            _ => Err(Error::ParseMappingScope(s.to_string())),
        }
    }
}

pub struct CommitInBlame {
    pub hash: String,
    pub author: String,
    pub date: String,
}

pub struct Stash {
    pub date: String,
    pub title: String,
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

#[derive(Clone)]
pub struct Commit {
    pub metadata: String,
    pub files: Vec<(FileStatus, String)>,
    pub hash: String,
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
        None
    }

    fn reinit(&mut self) {
        self.init_staged_status = self.staged_status;
        self.init_unstaged_status = self.unstaged_status;
    }
}

pub fn git_status_output(config: &Config) -> Result<String, Error> {
    let mut child = Command::new(config.git_exe.clone())
        .args(["status", "--short", "--no-renames"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute git command");

    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let reader = BufReader::new(stdout);

    let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
    let output_text = lines.join("\n");

    let status = child.wait().expect("Failed to wait on child");

    if !status.success() {
        return Err(Error::GitCommand);
    }
    Ok(output_text)
}

pub fn git_blame_output(
    file: String,
    revision: Option<String>,
    config: &Config,
) -> Result<String, Error> {
    let mut args: Vec<String> = vec!["blame".to_string()];
    if let Some(rev) = revision {
        args.push(rev);
    }
    args.push(file);

    let output = Command::new(config.git_exe.clone())
        .args(args)
        .output()
        .map_err(|_| Error::GitCommand)?;

    if !output.status.success() {
        return Err(Error::GitCommand);
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .to_string()
        .replace('\t', "    "))
}

pub fn git_parse_commit(output: &str) -> Result<Commit, Error> {
    let mut lines = output.lines().map(String::from);
    let mut metadata: Vec<String> = Vec::new();

    // Parse commit hash
    let line = lines.next().ok_or_else(|| Error::GitParsing)?;
    let commit_hash = line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| Error::GitParsing)?;
    metadata.push(line.clone());

    // Read all metadata
    for line in lines.by_ref() {
        if line.is_empty() {
            metadata.push("".to_string());
            break;
        }
        metadata.push(line.to_string());
    }

    // Read commit message and files
    let mut parsing_files = false;
    let mut files: Vec<(FileStatus, String)> = Vec::new();

    for line in lines {
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
                .ok_or_else(|| Error::GitParsing)?
                .to_string();
            files.push((status, filename));
        }
    }

    let commit = Commit {
        metadata: metadata.join("\n"),
        files,
        hash: commit_hash.to_string(),
    };
    Ok(commit)
}

pub fn git_stash_output(config: &Config) -> Result<String, Error> {
    let args = vec![
        "stash".to_string(),
        "list".to_string(),
        "--format=%cd\t%s".to_string(),
        "--date=iso-local".to_string(),
    ];
    let output = Command::new(config.git_exe.clone())
        .args(args)
        .output()
        .map_err(|_| Error::GitCommand)?;

    if !output.status.success() {
        return Err(Error::GitCommand);
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn git_show_output(revision: &Option<String>, config: &Config) -> Result<String, Error> {
    let mut args = vec![
        "show".to_string(),
        "--decorate".to_string(),
        "--name-status".to_string(),
        "--stat".to_string(),
        "--no-renames".to_string(),
    ];
    if let Some(rev) = revision {
        args.push(rev.clone());
    }

    let output = Command::new(config.git_exe.clone())
        .args(args)
        .output()
        .map_err(|_| Error::GitCommand)?;

    if !output.status.success() {
        return Err(Error::GitCommand);
    }

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

    let stdout = command.stdout.ok_or_else(|| Error::GitParsing)?;

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

pub fn set_git_dir(config: &Config) -> Result<(), Error> {
    // get git repo root dir
    let output = Command::new(config.git_exe.clone())
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .expect("Failed to execute git command");

    if !output.status.success() {
        return Err(Error::NotInGitRepo);
    }
    let mut repo_root = String::from_utf8_lossy(&output.stdout);
    repo_root = adapt_repo_root(repo_root.to_string().clone()).into();
    env::set_current_dir(repo_root.trim()).expect("Failed to change directory");
    Ok(())
}

pub fn git_add_restore(files: &mut HashMap<String, GitFile>, config: &Config) {
    for op in &[GitOp::Add, GitOp::Restore, GitOp::RmCached] {
        let mut files_to_op: Vec<String> = Vec::new();
        for (filename, git_file) in files.iter() {
            if Some(*op) == git_file.git_op() {
                files_to_op.push(filename.clone());
            }
        }
        if files_to_op.is_empty() {
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

    for git_file in files.values_mut() {
        git_file.reinit();
    }
}

pub fn get_previous_filename(rev: &str, current_filename: &str) -> Result<String, Error> {
    let output = Command::new("git")
        .args(["diff", "--name-status", &format!("{rev}^"), rev])
        .output()?;

    if !output.status.success() {
        return Err(Error::GitCommand);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        // Look for rename lines like: R100    old_name    new_name
        if line.starts_with('R') {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() == 3 && parts[2] == current_filename {
                return Ok(parts[1].to_string());
            }
        }
    }

    Ok(current_filename.to_string())
}

pub fn is_valid_git_rev(rev: &str) -> bool {
    let output = Command::new("git")
        .args(["rev-parse", "--verify", rev])
        .output();

    matches!(output, Ok(output) if output.status.success())
}
