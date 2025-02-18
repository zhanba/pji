use crate::config::{PjiConfig, PjiMetadata};
use crate::repo::PjiRepo;
use arboard::Clipboard;
use comfy_table::Table;
use dialoguer::{console::style, Confirm, FuzzySelect, Input, Select};
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

    pub fn start_config(&mut self) {
        self.add_root();
    }

    fn add_root(&mut self) -> &PathBuf {
        let name: String = Input::new()
            .with_prompt("Input pji root dir")
            .default(PjiConfig::get_default_root().display().to_string())
            .interact_text()
            .unwrap();
        let path = PathBuf::from(&name);
        let has_root = self.config.roots.contains(&path);
        if has_root {
            println!("{} already exists, please choose another one", name);
            self.add_root()
        } else {
            if !path.exists() {
                print!("{} not exists, creating...", path.display());
                create_dir_all(&path).expect("should create dir success");
                print!("done\n");
            }
            self.config.roots.push(path);
            self.config.save().expect("should save config file success");
            self.config.roots.last().unwrap()
        }
    }

    fn get_working_root(&mut self) -> &PathBuf {
        let len = self.config.roots.len();
        if len == 0 {
            println!("no root exists, please add one");
            self.add_root()
        } else if len == 1 {
            &self.config.roots[0]
        } else {
            let items = self
                .config
                .roots
                .iter()
                .map(|x| x.display().to_string())
                .collect::<Vec<_>>();
            let selection = Select::new()
                .with_prompt("Choose root to work with:")
                .default(0)
                .items(&items)
                .interact()
                .unwrap();
            &self.config.roots[selection]
        }
    }

    pub fn add(&mut self, repo: &str) {
        let root = self.get_working_root();
        let repo = PjiRepo::new(repo, root);
        if self.metadata.has_repo(&repo) {
            Self::warn_message(&format!("repo {} already exists", repo.git_uri.uri));
            return;
        }
        create_dir_all(&repo.dir).expect("should create repo dir success");
        let repo_dir = repo.dir.display().to_string();
        Self::clone_repo(&repo.git_uri.uri, &repo_dir).expect("should clone repo success");
        self.metadata.add_repo(&repo).save();
        Self::success_message(&format!(
            "Added repo {} to {} success",
            &repo.git_uri.uri, &repo_dir
        ));
        Self::copy_to_clipboard(&format!("cd {}", repo_dir));
    }

    pub fn remove(&mut self, repo: &str) {
        let root = self.get_working_root();
        let repo = PjiRepo::new(repo, root);
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
            &repo.git_uri.uri,
            &repo.dir.display().to_string()
        ));
    }

    pub fn list(&self) {
        let mut table = Table::new();
        table.set_header(vec![
            "dir", "protocol", "hostname", "user", "repo", "full uri",
        ]);
        self.metadata.repos.iter().for_each(|repo| {
            table.add_row(vec![
                &repo.dir.display().to_string(),
                repo.git_uri.protocol.as_str(),
                &repo.git_uri.hostname,
                &repo.git_uri.user,
                &repo.git_uri.repo,
                &repo.git_uri.uri,
            ]);
        });
        println!("{table}");
    }

    pub fn find(&mut self, query: &str) {
        let repo = self
            .find_repo("Enter repo name to search: ", query)
            .expect("repo not found");
        repo.update_open_time();
        let repo_dir = &repo.dir.display().to_string();
        println!("You choose: {}", repo_dir);
        Self::copy_to_clipboard(&format!("cd {}", repo_dir));
    }

    pub fn open_home(&mut self, query: Option<String>) {
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
            .repos
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

    fn find_repo(&mut self, prompt: &str, query: &str) -> Option<&mut PjiRepo> {
        let items = self
            .metadata
            .repos
            .iter_mut()
            .map(|repo| repo.dir.display().to_string())
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
            .repos
            .iter_mut()
            .find(|repo| repo.dir.display().to_string() == *repo_dir)
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
