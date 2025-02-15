use std::{fs::create_dir_all, path::PathBuf};

use dialoguer::{Confirm, Input};

use crate::config::{PJConfig, PJMetadata, PJRepo};

pub struct PJApp {
    config: PJConfig,
    metadata: PJMetadata,
}

impl PJApp {
    pub fn new() -> Self {
        let config = PJConfig::load();
        let metadata = PJMetadata::load();
        Self { config, metadata }
    }

    pub fn init() {
        let config_file_apth =
            PJConfig::get_config_file_path().expect("should get config file path success");
        if config_file_apth.exists() {
            let confirmation = Confirm::new()
                .with_prompt(format!(
                    "PJ config file {} already exists, do you want to continue?",
                    config_file_apth.display()
                ))
                .interact()
                .unwrap();
            if !confirmation {
                return;
            }
        }

        let name: String = Input::new()
            .with_prompt("Input pj root dir")
            .default(PJConfig::default().root.to_string_lossy().to_string())
            .interact_text()
            .unwrap();
        let path = PathBuf::new().join(name);

        if !path.exists() {
            print!("{} not exists, creating...", path.display());
            create_dir_all(&path).expect("should create dir success");
            print!("done\n");
        }

        let mut cfg = PJConfig::default();
        cfg.root = path;
        cfg.save().expect("should save config file success");
    }

    pub fn add(&mut self, repo: &str) {
        let repo = PJRepo::new(repo, &self.config.root);
        create_dir_all(&repo.dir).expect("should create repo dir success");
        self.metadata.add_repo(repo).save();
    }
}
