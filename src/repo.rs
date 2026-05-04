use crate::{error::PjiError, util::parse_git_url};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub(crate) enum GitProtocol {
    #[serde(rename = "SSH", alias = "Ssh")]
    Ssh,
    #[serde(rename = "HTTP", alias = "Https")]
    Https,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub(crate) struct GitURI {
    pub(crate) hostname: String,
    pub(crate) user: String,
    pub(crate) repo: String,
    pub(crate) protocol: GitProtocol,
    pub(crate) uri: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct PjiRepo {
    pub(crate) git_uri: GitURI,
    pub(crate) dir: PathBuf,
    pub(crate) root: PathBuf,
    pub(crate) create_time: DateTime<Utc>,
    pub(crate) last_open_time: DateTime<Utc>,
}

impl PjiRepo {
    pub(crate) fn try_new(repo_uri: &str, root: &Path) -> Result<Self, PjiError> {
        let git_uri =
            parse_git_url(repo_uri).ok_or_else(|| PjiError::InvalidGitUrl(repo_uri.to_string()))?;
        let repo_dir = root
            .join(&git_uri.hostname)
            .join(&git_uri.user)
            .join(&git_uri.repo);
        Ok(Self {
            git_uri,
            dir: repo_dir,
            root: root.to_path_buf(),
            create_time: Utc::now(),
            last_open_time: Utc::now(),
        })
    }

    pub(crate) fn update_open_time(&mut self) {
        self.last_open_time = Utc::now();
    }
}
