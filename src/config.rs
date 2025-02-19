use confy::{get_configuration_file_path, ConfyError};
use directories::UserDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{
    constant::{
        APP_CONFIG_NAME, APP_DATA_NAME, APP_METADATA_VERSION_V1, APP_NAME, DEFAULT_WORKSPACE_NAME,
    },
    repo::PjiRepo,
};

#[derive(Serialize, Deserialize)]
pub struct PjiConfig {
    pub roots: Vec<PathBuf>,
}

impl Default for PjiConfig {
    fn default() -> Self {
        Self {
            roots: vec![Self::get_default_root()],
        }
    }
}

impl PjiConfig {
    // load or init with default value
    pub fn load() -> Self {
        confy::load(APP_NAME, APP_CONFIG_NAME).expect("should read config file success")
    }

    pub fn get_config_file_path() -> Result<PathBuf, ConfyError> {
        get_configuration_file_path(APP_NAME, APP_CONFIG_NAME)
    }

    pub fn get_default_root() -> PathBuf {
        UserDirs::new()
            .expect("should get home dir")
            .home_dir()
            .join(DEFAULT_WORKSPACE_NAME)
    }

    pub fn save(&self) -> Result<(), ConfyError> {
        confy::store(APP_NAME, APP_CONFIG_NAME, self)
    }
}

#[derive(Serialize, Deserialize)]
pub struct PjiMetadata {
    pub version: String,
    pub repos: Vec<PjiRepo>,
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
    // load or init with default value
    pub fn load() -> Self {
        confy::load(APP_NAME, APP_DATA_NAME).expect("should read config file success")
    }

    pub fn save(&self) {
        confy::store(APP_NAME, APP_DATA_NAME, self).expect("should write config file success");
    }

    pub fn get_metadata_file_path() -> Result<PathBuf, ConfyError> {
        get_configuration_file_path(APP_NAME, APP_DATA_NAME)
    }

    pub fn add_repo(&mut self, pj_repo: &PjiRepo) -> &mut Self {
        self.repos.push(pj_repo.clone());
        self
    }

    pub fn remove_repo(&mut self, pj_repo: &PjiRepo) -> &mut Self {
        self.repos
            .retain(|repo| repo.git_uri.uri != pj_repo.git_uri.uri && repo.root == pj_repo.root);
        self
    }

    pub fn has_repo(&self, pj_repo: &PjiRepo) -> bool {
        self.repos
            .iter()
            .any(|repo| repo.git_uri.uri == pj_repo.git_uri.uri && repo.root == pj_repo.root)
    }
}
