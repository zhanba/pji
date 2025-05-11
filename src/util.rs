use std::{fs::read_dir, io, path::PathBuf, process::Command};

use crate::repo::{GitProtocol, GitURI};
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
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("config")
        .arg("--get")
        .arg("remote.origin.url")
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if url.is_empty() {
                    None
                } else {
                    Some(url)
                }
            } else {
                None
            }
        }
        Err(_) => None,
    }
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
