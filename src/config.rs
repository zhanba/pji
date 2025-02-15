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
pub struct PJConfig {
    pub root: PathBuf,
}

impl Default for PJConfig {
    fn default() -> Self {
        Self {
            root: UserDirs::new()
                .unwrap()
                .home_dir()
                .join(DEFAULT_WORKSPACE_NAME),
        }
    }
}

impl PJConfig {
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

#[derive(Serialize, Deserialize)]
pub struct PJRepo {
    pub uri: String,
    pub dir: String,
}

impl PJRepo {
    pub fn new(repo_uri: &str, root: &PathBuf) -> Self {
        let (_hostname, user, repo) =
            parse_git_url(repo_uri).expect(format!("Invalid git repo: {}", repo_uri).as_str());
        let repo_dir = root.join(user).join(repo);
        Self {
            uri: repo_uri.to_string(),
            dir: repo_dir.to_string_lossy().to_string(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct PJMetadata {
    version: String,
    repos: Vec<PJRepo>,
}

impl Default for PJMetadata {
    fn default() -> Self {
        Self {
            version: APP_METADATA_VERSION_V1.to_string(),
            repos: vec![],
        }
    }
}

impl PJMetadata {
    pub fn load() -> Self {
        confy::load(APP_NAME, APP_DATA_NAME).expect("should read config file success")
    }

    pub fn save(&self) {
        confy::store(APP_NAME, APP_DATA_NAME, self).expect("should write config file success");
    }

    pub fn add_repo(&mut self, pj_repo: PJRepo) -> &mut Self {
        self.repos.push(pj_repo);
        self
    }

    pub fn remove_repo(&mut self, pj_repo: PJRepo) -> &mut Self {
        self.repos.retain(|repo| repo.uri != pj_repo.uri);
        self
    }

    pub fn list_repos(&self) -> &Vec<PJRepo> {
        &self.repos
    }
}
