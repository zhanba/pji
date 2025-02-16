use crate::config::{PjiConfig, PjiMetadata, PjiRepo};
use arboard::Clipboard;
use comfy_table::Table;
use dialoguer::{console::style, Confirm, FuzzySelect, Input};
use std::env;
use std::fs::remove_dir_all;
use std::io::{self};
use std::process::{Command, Stdio};
use std::{fs::create_dir_all, path::PathBuf};

pub struct PjiApp {
    config: PjiConfig,
    metadata: PjiMetadata,
}

impl PjiApp {
    pub fn new() -> Self {
        let config = PjiConfig::load();
        let metadata = PjiMetadata::load();
        Self { config, metadata }
    }

    pub fn init() {
        let config_file_apth =
            PjiConfig::get_config_file_path().expect("should get config file path success");
        if config_file_apth.exists() {
            let confirmation = Self::confirm(&format!(
                "config file {} already exists, do you want to continue?",
                config_file_apth.display()
            ));
            if !confirmation {
                return;
            }
        }

        let name: String = Input::new()
            .with_prompt("Input pji root dir")
            .default(PjiConfig::default().root.to_string_lossy().to_string())
            .interact_text()
            .unwrap();
        let path = PathBuf::new().join(name);

        if !path.exists() {
            print!("{} not exists, creating...", path.display());
            create_dir_all(&path).expect("should create dir success");
            print!("done\n");
        }

        let mut cfg = PjiConfig::default();
        cfg.root = path;
        cfg.save().expect("should save config file success");
    }

    pub fn add(&mut self, repo: &str) {
        let repo = PjiRepo::new(repo, &self.config.root);
        if self.metadata.has_repo(&repo) {
            Self::warn_message(&format!("repo {} already exists", repo.git_uri.uri));
            return;
        }
        create_dir_all(&repo.dir).expect("should create repo dir success");
        Self::clone_repo(&repo.git_uri.uri, &repo.dir).expect("should clone repo success");
        self.metadata.add_repo(&repo).save();
        Self::success_message(&format!(
            "Added repo {} to {} success",
            &repo.git_uri.uri, &repo.dir
        ));
        Self::copy_to_clipboard(&format!("cd {}", repo.dir));
    }

    pub fn remove(&mut self, repo: &str) {
        let repo = PjiRepo::new(repo, &self.config.root);
        if !self.metadata.has_repo(&repo) {
            Self::warn_message(&format!("repo {} not exists", repo.git_uri.uri));
            return;
        }
        let confirmation = Self::confirm(&format!(
            "Are you sure to remove repo {}?",
            repo.git_uri.uri
        ));
        if !confirmation {
            return;
        }
        remove_dir_all(&repo.dir).expect("should remove repo dir success");
        self.metadata.remove_repo(&repo).save();
        Self::success_message(&format!(
            "Removed repo {} from {} success",
            &repo.git_uri.uri, &repo.dir
        ));
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
        let repo = self
            .find_repo("Enter repo name to search: ", query)
            .expect("repo not found");
        let repo_dir = repo.dir.clone();

        println!("You choose: {}", repo_dir);
        Self::copy_to_clipboard(&format!("cd {}", repo_dir));
    }

    pub fn open_home(&self, query: Option<String>) {
        let repo = match query {
            Some(query) => self
                .find_repo("Enter repo name to open: ", &query)
                .expect("repo not found"),
            None => self
                .get_cwd_repo()
                .expect("No repo found in current directory"),
        };

        let url = repo
            .get_home_url()
            .expect(&format!("No home URL found for {}", repo.git_uri.uri));
        Self::open_url(&url);
    }

    pub fn open_pr(&self, pr: Option<u32>) {
        let repo = self
            .get_cwd_repo()
            .expect("No repo found in current directory");

        let url = repo
            .get_pr_url(pr)
            .expect(&format!("No PR found for {}", repo.git_uri.uri));
        Self::open_url(&url);
    }

    pub fn open_issue(&self, issue: Option<u32>) {
        let repo = self
            .get_cwd_repo()
            .expect("No repo found in current directory");
        let url = repo
            .get_issue_url(issue)
            .expect(&format!("No issue found for {}", repo.git_uri.uri));
        Self::open_url(&url);
    }

    fn get_cwd_repo(&self) -> Option<&PjiRepo> {
        let cwd = env::current_dir().ok()?;
        let repo = self
            .metadata
            .list_repos()
            .iter()
            .find(|repo| cwd.starts_with(&repo.dir));
        repo
    }

    fn open_url(url: &str) {
        webbrowser::open(url).expect("Failed to open browser");
        println!("Opening URL: {}", url);
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

    fn find_repo(&self, prompt: &str, query: &str) -> Option<&PjiRepo> {
        let items = self
            .metadata
            .list_repos()
            .iter()
            .map(|repo| repo.dir.clone())
            .collect::<Vec<String>>();

        let selection = FuzzySelect::new()
            .with_prompt(prompt)
            .with_initial_text(query)
            .default(0)
            .highlight_matches(true)
            .max_length(10)
            .items(&items)
            .interact()
            .unwrap();

        let repo_dir = &items[selection];
        self.metadata
            .list_repos()
            .iter()
            .find(|repo| repo.dir == *repo_dir)
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
