use confy::{get_configuration_file_path, ConfyError};
use directories::UserDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{
    constant::{
        APP_CONFIG_NAME, APP_DATA_NAME, APP_METADATA_VERSION_V1, APP_NAME, DEFAULT_WORKSPACE_NAME,
    },
    error::PjiError,
    repo::PjiRepo,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PjiConfig {
    pub roots: Vec<PathBuf>,
}

impl Default for PjiConfig {
    fn default() -> Self {
        Self {
            roots: vec![Self::fallback_default_root()],
        }
    }
}

impl PjiConfig {
    pub(crate) fn try_load() -> Result<Self, ConfyError> {
        confy::load_path(config_file_path(APP_CONFIG_NAME)?)
    }

    pub(crate) fn get_config_file_path() -> Result<PathBuf, ConfyError> {
        config_file_path(APP_CONFIG_NAME)
    }

    pub(crate) fn get_default_root() -> Result<PathBuf, PjiError> {
        let user_dirs = UserDirs::new().ok_or(PjiError::HomeDirectoryNotFound)?;
        Ok(user_dirs.home_dir().join(DEFAULT_WORKSPACE_NAME))
    }

    fn fallback_default_root() -> PathBuf {
        UserDirs::new()
            .map(|dirs| dirs.home_dir().join(DEFAULT_WORKSPACE_NAME))
            .unwrap_or_else(|| PathBuf::from(DEFAULT_WORKSPACE_NAME))
    }

    pub(crate) fn save(&self) -> Result<(), ConfyError> {
        confy::store_path(config_file_path(APP_CONFIG_NAME)?, self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PjiMetadata {
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
    pub(crate) fn try_load() -> Result<Self, ConfyError> {
        confy::load_path(config_file_path(APP_DATA_NAME)?)
    }

    pub(crate) fn try_save(&self) -> Result<(), ConfyError> {
        confy::store_path(config_file_path(APP_DATA_NAME)?, self)
    }

    pub(crate) fn get_metadata_file_path() -> Result<PathBuf, ConfyError> {
        config_file_path(APP_DATA_NAME)
    }

    pub(crate) fn add_repo(&mut self, pj_repo: &PjiRepo) -> &mut Self {
        self.repos.push(pj_repo.clone());
        self
    }

    pub(crate) fn remove_repo(&mut self, pj_repo: &PjiRepo) -> &mut Self {
        self.repos.retain(|repo| repo.dir != pj_repo.dir);
        self
    }

    pub(crate) fn has_repo(&self, pj_repo: &PjiRepo) -> bool {
        self.repos.iter().any(|repo| repo.dir == pj_repo.dir)
    }

    pub(crate) fn deduplicate(&mut self) {
        let mut seen = std::collections::HashSet::new();
        self.repos.retain(|repo| seen.insert(repo.dir.clone()));
    }
}

fn config_file_path(config_name: &str) -> Result<PathBuf, ConfyError> {
    get_configuration_file_path(APP_NAME, config_name)
}
