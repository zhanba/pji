use crate::util::parse_git_url;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub enum GitProtocol {
    SSH,
    HTTP,
}

impl GitProtocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            GitProtocol::SSH => "ssh",
            GitProtocol::HTTP => "https",
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GitURI {
    pub hostname: String,
    pub user: String,
    pub repo: String,
    pub protocol: GitProtocol,
    pub uri: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PjiRepo {
    pub git_uri: GitURI,
    pub dir: PathBuf,
    pub root: PathBuf,
    pub create_time: DateTime<Utc>,
    pub last_open_time: DateTime<Utc>,
}

impl PjiRepo {
    pub fn new(repo_uri: &str, root: &PathBuf) -> Self {
        let git_uri =
            parse_git_url(repo_uri).expect(format!("Invalid git repo: {}", repo_uri).as_str());
        let repo_dir = root
            .join(&git_uri.hostname)
            .join(&git_uri.user)
            .join(&git_uri.repo);
        Self {
            git_uri,
            dir: repo_dir,
            root: root.clone(),
            create_time: Utc::now(),
            last_open_time: Utc::now(),
        }
    }

    pub fn update_open_time(&mut self) {
        self.last_open_time = Utc::now();
    }

    pub fn get_home_url(&self) -> Option<String> {
        match self.git_uri.hostname.as_str() {
            "github.com" => Some(format!(
                "https://github.com/{}/{}",
                self.git_uri.user, self.git_uri.repo
            )),
            _ => None,
        }
    }

    pub fn get_issue_url(&self, issue: Option<u32>) -> Option<String> {
        match self.git_uri.hostname.as_str() {
            "github.com" => match issue {
                Some(issue) => Some(format!(
                    "https://github.com/{}/{}/issues/{}",
                    self.git_uri.user, self.git_uri.repo, issue
                )),
                None => Some(format!(
                    "https://github.com/{}/{}/issues",
                    self.git_uri.user, self.git_uri.repo
                )),
            },
            _ => None,
        }
    }

    pub fn get_pr_url(&self, pr: Option<u32>) -> Option<String> {
        match self.git_uri.hostname.as_str() {
            "github.com" => match pr {
                Some(pr) => Some(format!(
                    "https://github.com/{}/{}/pull/{}",
                    self.git_uri.user, self.git_uri.repo, pr
                )),
                None => Some(format!(
                    "https://github.com/{}/{}/pull",
                    self.git_uri.user, self.git_uri.repo
                )),
            },
            _ => None,
        }
    }
}
