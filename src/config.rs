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
        self.repos.retain(|repo| repo.dir != pj_repo.dir);
        self
    }

    pub fn has_repo(&self, pj_repo: &PjiRepo) -> bool {
        self.repos.iter().any(|repo| repo.dir == pj_repo.dir)
    }

    pub fn deduplicate(&mut self) {
        if self.repos.is_empty() {
            return;
        }

        let mut indices: Vec<(usize, &PathBuf)> = self
            .repos
            .iter()
            .enumerate()
            .map(|(i, r)| (i, &r.dir))
            .collect();
        indices.sort_by(|a, b| a.1.cmp(b.1));

        let mut to_remove = vec![false; self.repos.len()];
        for i in 0..indices.len().saturating_sub(1) {
            if indices[i].1 == indices[i + 1].1 {
                to_remove[indices[i + 1].0] = true;
            }
        }

        let mut current_idx = 0;
        self.repos.retain(|_| {
            let keep = !to_remove[current_idx];
            current_idx += 1;
            keep
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::PjiRepo;
    use std::path::PathBuf;

    #[test]
    fn test_deduplicate() {
        let root = PathBuf::from("/tmp");
        let repo1 = PjiRepo::new("https://github.com/user/repo1.git", &root);
        let repo2 = PjiRepo::new("https://github.com/user/repo2.git", &root);
        let repo1_dup = PjiRepo::new("https://github.com/user/repo1.git", &root);

        let mut metadata = PjiMetadata::default();
        metadata.add_repo(&repo1);
        metadata.add_repo(&repo2);
        metadata.add_repo(&repo1_dup);

        assert_eq!(metadata.repos.len(), 3);

        metadata.deduplicate();

        assert_eq!(metadata.repos.len(), 2);
        assert_eq!(metadata.repos[0].dir, repo1.dir);
        assert_eq!(metadata.repos[1].dir, repo2.dir);
    }

    #[test]
    fn test_deduplicate_order() {
        let root = PathBuf::from("/tmp");
        let repo1 = PjiRepo::new("https://github.com/user/repo1.git", &root);
        let repo2 = PjiRepo::new("https://github.com/user/repo2.git", &root);
        let repo3 = PjiRepo::new("https://github.com/user/repo3.git", &root);

        let mut metadata = PjiMetadata::default();
        // Order: 2, 1, 3, 2, 1
        metadata.add_repo(&repo2);
        metadata.add_repo(&repo1);
        metadata.add_repo(&repo3);
        metadata.add_repo(&repo2);
        metadata.add_repo(&repo1);

        metadata.deduplicate();

        // Expected order: 2, 1, 3
        assert_eq!(metadata.repos.len(), 3);
        assert_eq!(metadata.repos[0].dir, repo2.dir);
        assert_eq!(metadata.repos[1].dir, repo1.dir);
        assert_eq!(metadata.repos[2].dir, repo3.dir);
    }
}
