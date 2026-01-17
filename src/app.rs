use crate::config::{PjiConfig, PjiMetadata};
use crate::repo::PjiRepo;
use crate::util::{list_dir, try_get_repo_from_dir};
use arboard::Clipboard;
use comfy_table::Table;
use dialoguer::{console::style, Confirm, FuzzySelect, Input, Select};
use std::env;
use std::fs::{remove_dir_all, remove_file};
use std::io;
use std::os::unix::process::CommandExt;
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
            .with_prompt("Enter the full path for the new pji root directory")
            .default(PjiConfig::get_default_root().display().to_string())
            .interact_text()
            .unwrap();
        let path = PathBuf::from(&name);
        let has_root = self.config.roots.contains(&path);
        if has_root {
            Self::warn_message(&format!(
                "Root '{}' already exists. Please choose another.",
                name
            ));
            self.add_root()
        } else {
            if !path.exists() {
                println!("Creating directory '{}'...", path.display());
                create_dir_all(&path).expect("should create dir success");
                Self::success_message(&format!("Directory '{}' created.", path.display()));
            }
            self.config.roots.push(path);
            self.config.save().expect("should save config file success");
            Self::success_message(&format!(
                "Root '{}' added successfully.",
                self.config.roots.last().unwrap().display()
            ));
            self.config.roots.last().unwrap()
        }
    }

    fn get_working_root(&mut self) -> &PathBuf {
        let len = self.config.roots.len();
        if len == 0 {
            Self::warn_message("No pji roots found. Let's add one first.");
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
                .with_prompt("Select root directory")
                .default(0)
                .items(&items)
                .interact()
                .unwrap();
            &self.config.roots[selection]
        }
    }

    pub fn add(&mut self, repo_uri_str: &str) {
        let root = self.get_working_root();
        let repo = PjiRepo::new(repo_uri_str, root);
        if self.metadata.has_repo(&repo) {
            Self::warn_message(&format!(
                "Repository '{}' already exists in pji.",
                repo.git_uri.uri
            ));
            return;
        }
        create_dir_all(&repo.dir).expect("should create repo dir success");
        let repo_dir = repo.dir.display().to_string();
        println!("Cloning '{}' into '{}'...", repo.git_uri.uri, repo_dir);
        Self::clone_repo(&repo.git_uri.uri, &repo_dir).expect("should clone repo success");
        self.metadata.add_repo(&repo).save();
        Self::success_message(&format!(
            "✨ Repository '{}' added to '{}'.",
            &repo.git_uri.uri, &repo_dir
        ));
        Self::copy_to_clipboard(
            &format!("cd {}", repo_dir),
            "Paste to navigate to the repository.",
        );
    }

    pub fn remove(&mut self, repo_uri_str: &str) {
        let root = self.get_working_root();
        let repo = PjiRepo::new(repo_uri_str, root);
        if !self.metadata.has_repo(&repo) {
            Self::warn_message(&format!(
                "Repository '{}' not found in pji.",
                repo.git_uri.uri
            ));
            return;
        }
        let confirmation = Self::confirm(&format!(
            "Are you sure you want to remove the repository '{}' from disk and pji?",
            repo.git_uri.uri
        ));
        if !confirmation {
            println!("✖️ Removal cancelled.");
            return;
        }
        println!("Removing directory '{}'...", repo.dir.display());
        remove_dir_all(&repo.dir).expect("should remove repo dir success");
        self.metadata.remove_repo(&repo).save();
        Self::success_message(&format!(
            "🗑️ Repository '{}' removed successfully from '{}'.",
            &repo.git_uri.uri,
            &repo.dir.display()
        ));
    }

    pub fn list(&mut self, long_format: bool) {
        self.metadata
            .repos
            .sort_by(|a, b| b.last_open_time.cmp(&a.last_open_time));
        if long_format {
            let mut table = Table::new();
            table.set_header(vec!["dir", "hostname", "user", "repo", "full uri"]);
            self.metadata.repos.iter().for_each(|repo| {
                table.add_row(vec![
                    &repo.dir.display().to_string(),
                    &repo.git_uri.hostname,
                    &repo.git_uri.user,
                    &repo.git_uri.repo,
                    &repo.git_uri.uri,
                ]);
            });
            println!("{table}");
        } else {
            self.metadata.repos.iter().for_each(|repo| {
                println!("{}", repo.dir.display());
            });
        }
    }

    pub fn find(&mut self, query: &str) {
        let repo = self
            .find_repo("🔍 Search and select repository: ", query)
            .expect("repo not found");
        repo.update_open_time();
        let repo_dir = repo.dir.clone();

        // Get the user's shell from SHELL env var, default to /bin/sh
        let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

        // Save metadata before exec (exec replaces process, so we won't return)
        self.metadata.save();

        // Replace current process with a new shell in the target directory
        let err = Command::new(&shell)
            .current_dir(&repo_dir)
            .exec();

        // exec() only returns if there was an error
        eprintln!("Failed to exec shell: {}", err);
    }

    pub fn scan(&mut self) {
        let mut total_new_repos_added = 0;
        // Clean up duplicates before scanning
        self.metadata.deduplicate();

        for root in self.config.roots.as_slice() {
            println!("🔍 Scanning {}...", root.display());
            if let Some(repos) = Self::get_repos_from_root(&root) {
                for repo in repos {
                    if !(self.metadata.has_repo(&repo)) {
                        println!("  ✨ Added: {}", repo.dir.display());
                        self.metadata.repos.push(repo);
                        total_new_repos_added += 1;
                    }
                }
            }
        }
        self.metadata.save();
        if total_new_repos_added > 0 {
            let repo_str = if total_new_repos_added == 1 {
                "repository"
            } else {
                "repositories"
            };
            Self::success_message(&format!(
                "Scan complete. {} new {} added.",
                total_new_repos_added, repo_str
            ));
        } else {
            Self::success_message("Scan complete. No new repositories found.");
        }
    }

    pub fn clean() {
        if let Ok(config_path) = PjiConfig::get_config_file_path() {
            remove_file(config_path).expect("Failed to remove config file");
        }

        if let Ok(metadata_path) = PjiMetadata::get_metadata_file_path() {
            remove_file(metadata_path).expect("Failed to remove metadata file");
        }

        Self::success_message("🧹 Project data cleaned successfully.");
    }

    fn get_repos_from_root(root: &PathBuf) -> Option<Vec<PjiRepo>> {
        if !root.is_dir() {
            return None;
        }
        let mut repos = vec![];
        let mut invalid_repo_paths: Vec<String> = Vec::new();

        if let Ok(hostname_dirs) = list_dir(root) {
            for hostname_dir in hostname_dirs {
                if let Ok(user_dirs) = list_dir(&hostname_dir) {
                    for user_dir in user_dirs {
                        if let Ok(repo_dirs) = list_dir(&user_dir) {
                            for repo_dir in repo_dirs {
                                if let Some(repo_url) = try_get_repo_from_dir(&repo_dir) {
                                    let repo = PjiRepo::new(&repo_url, root);
                                    if repo.dir == repo_dir {
                                        repos.push(repo);
                                    } else {
                                        invalid_repo_paths.push(repo_dir.display().to_string());
                                    }
                                } else {
                                    invalid_repo_paths.push(repo_dir.display().to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        if !invalid_repo_paths.is_empty() {
            Self::warn_message("The following paths were found but are not valid pji repositories or have an unexpected structure:");
            for path_str in invalid_repo_paths {
                println!("  - {}", path_str);
            }
        }
        Some(repos)
    }

    pub fn open_home(&mut self, query: Option<String>) {
        let repo = match query {
            Some(query) => self
                .find_repo("Open repo: ", &query)
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
        println!("🌐 Opening URL in browser: {}", style(url).cyan());
        webbrowser::open(url).expect("Failed to open browser");
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
        self.metadata
            .repos
            .sort_by(|a, b| b.last_open_time.cmp(&a.last_open_time));

        let mut counts = std::collections::HashMap::new();
        for repo in &self.metadata.repos {
            let key = format!("{}/{}", repo.git_uri.user, repo.git_uri.repo);
            *counts.entry(key).or_insert(0) += 1;
        }

        let items = self
            .metadata
            .repos
            .iter()
            .map(|repo| {
                let key = format!("{}/{}", repo.git_uri.user, repo.git_uri.repo);
                // if there are multiple repos with the same user/repo, show the full path
                if *counts.get(&key).unwrap_or(&0) > 1 {
                    repo.dir.display().to_string()
                } else {
                    key
                }
            })
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

        self.metadata.repos.get_mut(selection)
    }

    fn success_message(message: &str) {
        println!("🚀 {}", style(message).green());
    }

    fn warn_message(message: &str) {
        println!("⚠️  {}", style(message).yellow());
    }

    fn copy_to_clipboard(text: &str, context_message: &str) {
        Clipboard::new()
            .expect("can't find clipboard")
            .set_text(text)
            .expect("can't set clipboard");

        println!(
            "📋 Copied \"{}\" to clipboard. {}", // Changed
            style(text).green(),
            context_message
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
