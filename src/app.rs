use crate::config::{PJConfig, PJMetadata, PJRepo};
use arboard::Clipboard;
use comfy_table::Table;
use dialoguer::{console::style, Confirm, Input};
use std::io::{self};
use std::process::{Command, Stdio};
use std::{fs::create_dir_all, path::PathBuf};

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
        if self.metadata.has_repo(&repo) {
            println!("{} already exists", repo.git_uri.uri);
            return;
        }
        create_dir_all(&repo.dir).expect("should create repo dir success");
        PJApp::clone_repo(&repo.git_uri.uri, &repo.dir).expect("should clone repo success");
        self.metadata.add_repo(&repo).save();
        Clipboard::new()
            .expect("can't find clipboard")
            .set_text(format!("cd {}", repo.dir))
            .expect("can't set clipboard");
    }

    pub fn list(&self) {
        let mut table = Table::new();
        table.set_header(vec!["dir", "protocol", "user", "repo", "full git uri"]);
        self.metadata.list_repos().iter().for_each(|repo| {
            table.add_row(vec![
                repo.dir.clone(),
                repo.git_uri.protocol.as_str().to_string(),
                repo.git_uri.user.clone(),
                repo.git_uri.repo.clone(),
                repo.git_uri.uri.clone(),
            ]);
        });
        println!("{table}");
    }

    fn clone_repo(repo: &str, dir: &str) -> io::Result<()> {
        let mut cmd = Command::new("git");
        cmd.args(["clone", repo, dir]);
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        // Spawn the command
        let mut child = cmd.spawn()?;

        // Wait for the command to finish
        let status = child.wait()?;
        if !status.success() {
            return Err(io::Error::new(io::ErrorKind::Other, "git clone failed"));
        } else {
            PJApp::success_message("git clone success");
        }
        Ok(())
    }

    fn success_message(message: &str) {
        println!("🚀 {}", style(message).green());
    }
}
