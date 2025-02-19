use std::{fs::read_dir, io, path::PathBuf};

use crate::repo::{GitProtocol, GitURI};
use git2::Repository;
use regex::Regex;

pub fn parse_git_url(url: &str) -> Option<GitURI> {
    let ssh_re = Regex::new(r"^git@(?P<host>[^:]+):(?P<user>[^/]+)/(?P<repo>[^/]+)\.git$")
        .expect("Failed to compile SSH regex");
    let http_re = Regex::new(r"^https?://(?P<host>[^/]+)/(?P<user>[^/]+)/(?P<repo>[^/]+)\.git$")
        .expect("Failed to compile HTTP regex");

    if let Some(caps) = ssh_re.captures(url) {
        let hostname = caps.name("host").map(|m| m.as_str()).unwrap_or("");
        let user = caps.name("user").map(|m| m.as_str()).unwrap_or("");
        let repo = caps.name("repo").map(|m| m.as_str()).unwrap_or("");

        Some(GitURI {
            hostname: hostname.to_string(),
            user: user.to_string(),
            repo: repo.to_string(),
            protocol: GitProtocol::SSH,
            uri: url.to_string(),
        })
    } else if let Some(caps) = http_re.captures(url) {
        let hostname = caps.name("host").map(|m| m.as_str()).unwrap_or("");
        let user = caps.name("user").map(|m| m.as_str()).unwrap_or("");
        let repo = caps.name("repo").map(|m| m.as_str()).unwrap_or("");

        Some(GitURI {
            hostname: hostname.to_string(),
            user: user.to_string(),
            repo: repo.to_string(),
            protocol: GitProtocol::HTTP,
            uri: url.to_string(),
        })
    } else {
        None
    }
}

pub fn try_get_repo_from_dir(dir: &PathBuf) -> Option<String> {
    if let Ok(repo) = Repository::open(&dir) {
        if let Some(remote) = repo.find_remote("origin").ok() {
            if let Some(url) = remote.url() {
                return Some(url.to_string());
            }
        }
    }
    None
}

pub fn list_dir(dir: &PathBuf) -> io::Result<Vec<PathBuf>> {
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
