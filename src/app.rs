use crate::config::{PJConfig, PJMetadata, PJRepo};
use arboard::Clipboard;
use comfy_table::Table;
use dialoguer::{console::style, Confirm, FuzzySelect, Input};
use std::fs::remove_dir_all;
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
            let confirmation = PJApp::confirm(&format!(
                "PJ config file {} already exists, do you want to continue?",
                config_file_apth.display()
            ));
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
            PJApp::warn_message(&format!("repo {} already exists", repo.git_uri.uri));
            return;
        }
        create_dir_all(&repo.dir).expect("should create repo dir success");
        PJApp::clone_repo(&repo.git_uri.uri, &repo.dir).expect("should clone repo success");
        self.metadata.add_repo(&repo).save();
        PJApp::success_message(&format!("Added repo {} success", &repo.git_uri.uri));
        PJApp::copy_to_clipboard(&format!("cd {}", repo.dir));
    }

    pub fn remove(&mut self, repo: &str) {
        let repo = PJRepo::new(repo, &self.config.root);
        if !self.metadata.has_repo(&repo) {
            PJApp::warn_message(&format!("repo {} not exists", repo.git_uri.uri));
            return;
        }
        let confirmation = PJApp::confirm(&format!(
            "Are you sure to remove repo {}?",
            repo.git_uri.uri
        ));
        if !confirmation {
            return;
        }
        remove_dir_all(&repo.dir).expect("should remove repo dir success");
        self.metadata.remove_repo(&repo).save();
        PJApp::success_message(&format!("Removed repo {} success", &repo.git_uri.uri));
    }

    pub fn list(&self) {
        let mut table = Table::new();
        table.set_header(vec![
            "dir", "protocol", "hostname", "user", "repo", "full uri",
        ]);
        self.metadata.list_repos().iter().for_each(|repo| {
            table.add_row(vec![
                repo.dir.clone(),
                repo.git_uri.protocol.as_str().to_string(),
                repo.git_uri.hostname.clone(),
                repo.git_uri.user.clone(),
                repo.git_uri.repo.clone(),
                repo.git_uri.uri.clone(),
            ]);
        });
        println!("{table}");
    }

    pub fn find(&self, query: &str) {
        let items = self
            .metadata
            .list_repos()
            .iter()
            .map(|repo| repo.dir.clone())
            .collect::<Vec<String>>();

        let selection = FuzzySelect::new()
            .with_prompt("Input repo name to search: ")
            .with_initial_text(query)
            .highlight_matches(true)
            .max_length(10)
            .items(&items)
            .interact()
            .unwrap();

        println!("You choose: {}", items[selection]);
        PJApp::copy_to_clipboard(&format!("cd {}", items[selection]));
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
        }
        Ok(())
    }

    fn success_message(message: &str) {
        println!("🚀 {}", style(message).green());
    }

    fn warn_message(message: &str) {
        println!("⚠️  {}", style(message).yellow());
    }

    fn copy_to_clipboard(text: &str) {
        Clipboard::new()
            .expect("can't find clipboard")
            .set_text(text)
            .expect("can't set clipboard");

        println!(
            "📋 Copied \"{}\" to clipboard, just paste it in",
            style(text).green()
        )
    }

    fn confirm(message: &str) -> bool {
        let confirmation = Confirm::new()
            .with_prompt(format!("❓ {}", style(message).yellow()))
            .interact()
            .unwrap();
        confirmation
    }
}
