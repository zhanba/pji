use std::{fs::read_dir, io, path::PathBuf, process::Command};

use crate::{
    error::PjiError,
    repo::{GitProtocol, GitURI},
};

pub(crate) fn parse_git_url(url: &str) -> Option<GitURI> {
    parse_ssh_git_url(url).or_else(|| parse_http_git_url(url))
}

fn parse_ssh_git_url(url: &str) -> Option<GitURI> {
    let rest = url.strip_prefix("git@")?;
    let (hostname, path) = rest.split_once(':')?;
    let (user, repo) = split_repo_path(path)?;

    Some(GitURI {
        hostname: hostname.to_string(),
        user: user.to_string(),
        repo: repo.to_string(),
        protocol: GitProtocol::Ssh,
        uri: url.to_string(),
    })
}

fn parse_http_git_url(url: &str) -> Option<GitURI> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let (hostname, path) = rest.split_once('/')?;
    let (user, repo) = split_repo_path(path)?;

    Some(GitURI {
        hostname: hostname.to_string(),
        user: user.to_string(),
        repo: repo.to_string(),
        protocol: GitProtocol::Https,
        uri: url.to_string(),
    })
}

fn split_repo_path(path: &str) -> Option<(&str, &str)> {
    let (user, repo) = path.split_once('/')?;
    let repo = repo.strip_suffix(".git")?;
    if user.is_empty() || repo.is_empty() || repo.contains('/') {
        return None;
    }
    Some((user, repo))
}

pub(crate) fn try_get_repo_from_dir(dir: &PathBuf) -> Result<Option<String>, PjiError> {
    let command = format!("git -C {} config --get remote.origin.url", dir.display());
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("config")
        .arg("--get")
        .arg("remote.origin.url")
        .output()?;

    if output.status.success() {
        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if url.is_empty() {
            return Err(PjiError::EmptyGitOutput { command });
        }
        return Ok(Some(url));
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        return Ok(None);
    }

    Err(PjiError::GitCommand { command, stderr })
}

pub(crate) fn list_dir(dir: &PathBuf) -> io::Result<Vec<PathBuf>> {
    let mut dirs = vec![];
    for entry in read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            dirs.push(path);
        }
    }
    Ok(dirs)
}
