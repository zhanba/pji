use confy::ConfyError;
use std::{io, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PjiError {
    #[error("failed to read or write pji config: {0}")]
    Config(#[source] ConfyError),
    #[error("failed to read or write pji metadata: {0}")]
    Metadata(#[source] ConfyError),
    #[error("invalid git url: {0}")]
    InvalidGitUrl(String),
    #[error("home directory not found")]
    HomeDirectoryNotFound,
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("repository is already registered: {}", .0.display())]
    RepositoryAlreadyRegistered(PathBuf),
    #[error("repository is not registered: {}", .0.display())]
    RepositoryNotRegistered(PathBuf),
    #[error("git command failed (`{command}`): {}", stderr.trim())]
    GitCommand { command: String, stderr: String },
    #[error("git command produced no usable output: {command}")]
    EmptyGitOutput { command: String },
    #[error("invalid worktree: {0}")]
    InvalidWorktree(String),
}
