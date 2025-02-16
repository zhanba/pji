use confy::{get_configuration_file_path, ConfyError};
use directories::UserDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{
    constant::{
        APP_CONFIG_NAME, APP_DATA_NAME, APP_METADATA_VERSION_V1, APP_NAME, DEFAULT_WORKSPACE_NAME,
    },
    util::parse_git_url,
};

#[derive(Serialize, Deserialize)]
pub struct PjiConfig {
    pub root: PathBuf,
}

impl Default for PjiConfig {
    fn default() -> Self {
        Self {
            root: UserDirs::new()
                .unwrap()
                .home_dir()
                .join(DEFAULT_WORKSPACE_NAME),
        }
    }
}

impl PjiConfig {
    pub fn load() -> Self {
        confy::load(APP_NAME, APP_CONFIG_NAME).expect("should read config file success")
    }

    pub fn get_config_file_path() -> Result<PathBuf, ConfyError> {
        get_configuration_file_path(APP_NAME, APP_CONFIG_NAME)
    }

    pub fn save(&self) -> Result<(), ConfyError> {
        confy::store(APP_NAME, APP_CONFIG_NAME, self)
    }
}

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
    pub dir: String,
}

impl PjiRepo {
    pub fn new(repo_uri: &str, root: &PathBuf) -> Self {
        let git_uri =
            parse_git_url(repo_uri).expect(format!("Invalid git repo: {}", repo_uri).as_str());
        let repo_dir = root.join(&git_uri.user).join(&git_uri.repo);
        Self {
            git_uri,
            dir: repo_dir.to_string_lossy().to_string(),
        }
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

    pub fn get_issue_url(&self, issue: Option<&str>) -> Option<String> {
        match self.git_uri.hostname.as_str() {
            "github.com" => match issue {
                Some(issue) => Some(format!(
                    "https://github.com/{}/{}/issues/{}",
                    self.git_uri.user, self.git_uri.repo, issue
                )),
                None => Some(format!(
                    "https://github.com/{}/{}",
                    self.git_uri.user, self.git_uri.repo
                )),
            },
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct PjiMetadata {
    version: String,
    repos: Vec<PjiRepo>,
}

impl Default for PjiMetadata {
    fn default() -> Self {
        Self {
            version: APP_METADATA_VERSION_V1.to_string(),
            repos: vec![],
        }
    }
}

impl PjiMetadata {
    pub fn load() -> Self {
        confy::load(APP_NAME, APP_DATA_NAME).expect("should read config file success")
    }

    pub fn save(&self) {
        confy::store(APP_NAME, APP_DATA_NAME, self).expect("should write config file success");
    }

    pub fn add_repo(&mut self, pj_repo: &PjiRepo) -> &mut Self {
        self.repos.push(pj_repo.clone());
        self
    }

    pub fn remove_repo(&mut self, pj_repo: &PjiRepo) -> &mut Self {
        self.repos
            .retain(|repo| repo.git_uri.uri != pj_repo.git_uri.uri);
        self
    }

    pub fn has_repo(&self, pj_repo: &PjiRepo) -> bool {
        self.repos
            .iter()
            .any(|repo| repo.git_uri.uri == pj_repo.git_uri.uri)
    }

    pub fn list_repos(&self) -> &Vec<PjiRepo> {
        &self.repos
    }
}
