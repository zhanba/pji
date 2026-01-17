use crate::config::{PjiConfig, PjiMetadata};
use crate::repo::PjiRepo;
use crate::util::{list_dir, try_get_repo_from_dir};
use crate::worktree::{
    self, add_worktree, get_main_repo_from_worktree, is_linked_worktree, list_worktrees,
    prune_worktrees, remove_worktree, GitWorktree,
};
use arboard::Clipboard;
use comfy_table::Table;
use dialoguer::{console::style, Confirm, FuzzySelect, Input, Select};
use std::env;
use std::fs::{create_dir_all, remove_dir_all, remove_file};
use std::io;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};

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
            table.set_header(vec!["dir", "hostname", "user", "repo", "worktrees", "full uri"]);
            self.metadata.repos.iter().for_each(|repo| {
                let worktree_count = match list_worktrees(&repo.dir) {
                    Some(wts) if wts.has_linked() => format!("{}", wts.count()),
                    _ => "-".to_string(),
                };
                table.add_row(vec![
                    &repo.dir.display().to_string(),
                    &repo.git_uri.hostname,
                    &repo.git_uri.user,
                    &repo.git_uri.repo,
                    &worktree_count,
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

        // Save metadata before exec (exec replaces process, so we won't return)
        self.metadata.save();

        // Check if the repository has worktrees
        if let Some(worktrees) = list_worktrees(&repo_dir) {
            if worktrees.has_linked() {
                // Show worktree picker
                if let Some(wt) = self.select_worktree(&worktrees, "") {
                    self.exec_into_dir(&wt.path);
                    return;
                }
            }
        }

        // No worktrees or only main worktree - exec into repo dir
        self.exec_into_dir(&repo_dir);
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
                                // Skip linked worktrees - they belong to their main repo
                                if is_linked_worktree(&repo_dir) {
                                    continue;
                                }

                                // Skip .worktrees directories
                                if repo_dir
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .map(|n| n.ends_with(".worktrees"))
                                    .unwrap_or(false)
                                {
                                    continue;
                                }

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

        // First check if we're in a linked worktree and resolve to main repo
        let resolved_dir = if is_linked_worktree(&cwd) {
            get_main_repo_from_worktree(&cwd)?
        } else {
            // Check if any parent is a linked worktree
            let mut check_dir = cwd.clone();
            let mut found_main = None;
            loop {
                if is_linked_worktree(&check_dir) {
                    found_main = get_main_repo_from_worktree(&check_dir);
                    break;
                }
                if !check_dir.pop() {
                    break;
                }
            }
            found_main.unwrap_or(cwd)
        };

        // Find the repo that matches the resolved directory
        self.metadata
            .repos
            .iter()
            .find(|repo| resolved_dir.starts_with(&repo.dir))
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

    // ==================== Worktree Commands ====================

    /// List worktrees for current or selected repository
    pub fn worktree_list(&mut self, query: Option<String>) {
        let repo_dir = self.get_worktree_repo_dir(query);
        let repo_dir = match repo_dir {
            Some(dir) => dir,
            None => {
                Self::warn_message("No repository found.");
                return;
            }
        };

        match list_worktrees(&repo_dir) {
            Some(worktrees) => {
                let mut table = Table::new();
                table.set_header(vec!["Path", "Branch", "Status"]);

                for wt in worktrees.all() {
                    let status = if wt.is_main {
                        "main".to_string()
                    } else if wt.locked {
                        "locked".to_string()
                    } else if wt.prunable {
                        "prunable".to_string()
                    } else {
                        "".to_string()
                    };

                    table.add_row(vec![
                        wt.path.display().to_string(),
                        wt.branch.clone().unwrap_or_else(|| format!("({})", &wt.commit[..8.min(wt.commit.len())])),
                        status,
                    ]);
                }

                println!("{table}");
                println!("\nTotal: {} worktree(s)", worktrees.count());
            }
            None => {
                println!("No worktrees found for this repository.");
            }
        }
    }

    /// Fuzzy select and switch to a worktree
    pub fn worktree_switch(&mut self, query: Option<String>) {
        let repo_dir = self.get_worktree_repo_dir(None);
        let repo_dir = match repo_dir {
            Some(dir) => dir,
            None => {
                Self::warn_message("No repository found in current directory.");
                return;
            }
        };

        let worktrees = match list_worktrees(&repo_dir) {
            Some(wts) if wts.count() > 1 => wts,
            Some(_) => {
                Self::warn_message("Only one worktree exists. Nothing to switch to.");
                return;
            }
            None => {
                Self::warn_message("No worktrees found for this repository.");
                return;
            }
        };

        let selected = self.select_worktree(&worktrees, query.as_deref().unwrap_or(""));
        if let Some(wt) = selected {
            self.exec_into_dir(&wt.path);
        }
    }

    /// Create a new worktree
    pub fn worktree_add(&mut self, branch: &str, create_branch: bool, path: Option<String>) {
        let repo_dir = self.get_worktree_repo_dir(None);
        let repo_dir = match repo_dir {
            Some(dir) => dir,
            None => {
                Self::warn_message("No repository found in current directory.");
                return;
            }
        };

        let path = path.map(PathBuf::from);

        match add_worktree(&repo_dir, branch, path, create_branch) {
            Ok(worktree_path) => {
                Self::success_message(&format!(
                    "Worktree created at '{}'",
                    worktree_path.display()
                ));
                self.exec_into_dir(&worktree_path);
            }
            Err(e) => {
                eprintln!("Failed to create worktree: {}", e);
            }
        }
    }

    /// Remove a worktree
    pub fn worktree_remove(&mut self, worktree: Option<String>, force: bool) {
        let repo_dir = self.get_worktree_repo_dir(None);
        let repo_dir = match repo_dir {
            Some(dir) => dir,
            None => {
                Self::warn_message("No repository found in current directory.");
                return;
            }
        };

        let worktrees = match list_worktrees(&repo_dir) {
            Some(wts) if !wts.linked.is_empty() => wts,
            Some(_) => {
                Self::warn_message("No linked worktrees to remove.");
                return;
            }
            None => {
                Self::warn_message("No worktrees found for this repository.");
                return;
            }
        };

        // Determine which worktree to remove
        let worktree_path = match worktree {
            Some(wt_str) => {
                // Try to find worktree by path or branch name
                let found = worktrees.linked.iter().find(|wt| {
                    wt.path.to_string_lossy().contains(&wt_str)
                        || wt.branch.as_deref() == Some(&wt_str)
                });
                match found {
                    Some(wt) => wt.path.clone(),
                    None => {
                        Self::warn_message(&format!("Worktree '{}' not found.", wt_str));
                        return;
                    }
                }
            }
            None => {
                // Interactive selection from linked worktrees only
                let items: Vec<String> = worktrees
                    .linked
                    .iter()
                    .map(|wt| {
                        format!(
                            "{} ({})",
                            wt.branch.as_deref().unwrap_or("detached"),
                            wt.path.display()
                        )
                    })
                    .collect();

                let selection = FuzzySelect::new()
                    .with_prompt("Select worktree to remove")
                    .default(0)
                    .highlight_matches(true)
                    .items(&items)
                    .interact()
                    .unwrap();

                worktrees.linked[selection].path.clone()
            }
        };

        // Confirm removal
        if !Self::confirm(&format!(
            "Remove worktree at '{}'?",
            worktree_path.display()
        )) {
            println!("Removal cancelled.");
            return;
        }

        match remove_worktree(&repo_dir, &worktree_path, force) {
            Ok(()) => {
                Self::success_message(&format!(
                    "Worktree '{}' removed successfully.",
                    worktree_path.display()
                ));
            }
            Err(e) => {
                eprintln!("Failed to remove worktree: {}", e);
                if !force {
                    println!("Tip: Use --force to force removal of dirty worktrees.");
                }
            }
        }
    }

    /// Clean up stale worktree information
    pub fn worktree_prune(&self) {
        let repo_dir = self.get_cwd_repo_dir();
        let repo_dir = match repo_dir {
            Some(dir) => dir,
            None => {
                Self::warn_message("No repository found in current directory.");
                return;
            }
        };

        match prune_worktrees(&repo_dir) {
            Ok(output) => {
                if output.is_empty() {
                    println!("No stale worktree entries to prune.");
                } else {
                    println!("{}", output);
                    Self::success_message("Worktree pruning complete.");
                }
            }
            Err(e) => {
                eprintln!("Failed to prune worktrees: {}", e);
            }
        }
    }

    /// Get the repository directory for worktree operations
    /// If in a worktree, returns the main repo directory
    fn get_worktree_repo_dir(&mut self, query: Option<String>) -> Option<PathBuf> {
        match query {
            Some(q) => {
                let repo = self.find_repo("Select repository: ", &q)?;
                Some(repo.dir.clone())
            }
            None => self.get_cwd_repo_dir(),
        }
    }

    /// Get the current working directory's repository directory
    /// Handles both main repos and linked worktrees
    fn get_cwd_repo_dir(&self) -> Option<PathBuf> {
        let cwd = env::current_dir().ok()?;

        // First check if we're in a linked worktree
        if is_linked_worktree(&cwd) {
            return get_main_repo_from_worktree(&cwd);
        }

        // Check if cwd or any parent is a linked worktree
        let mut check_dir = cwd.clone();
        loop {
            if is_linked_worktree(&check_dir) {
                return get_main_repo_from_worktree(&check_dir);
            }
            if check_dir.join(".git").is_dir() {
                return Some(check_dir);
            }
            if !check_dir.pop() {
                break;
            }
        }

        // Fall back to checking metadata repos
        self.metadata
            .repos
            .iter()
            .find(|repo| cwd.starts_with(&repo.dir))
            .map(|repo| repo.dir.clone())
    }

    /// Select a worktree from the list using fuzzy selection
    fn select_worktree<'a>(
        &self,
        worktrees: &'a worktree::WorktreeList,
        query: &str,
    ) -> Option<&'a GitWorktree> {
        let all_worktrees = worktrees.all();
        let items: Vec<String> = all_worktrees
            .iter()
            .map(|wt| {
                let status = if wt.is_main { " (main)" } else { "" };
                format!(
                    "{}{}  {}",
                    wt.branch.as_deref().unwrap_or("detached"),
                    status,
                    wt.path.display()
                )
            })
            .collect();

        let selection = FuzzySelect::new()
            .with_prompt("Select worktree")
            .with_initial_text(query)
            .default(0)
            .highlight_matches(true)
            .max_length(10)
            .items(&items)
            .interact()
            .unwrap();

        all_worktrees.get(selection).copied()
    }

    /// Execute into a directory (replace current process with shell in that directory)
    fn exec_into_dir(&self, dir: &PathBuf) {
        let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

        let err = Command::new(&shell).current_dir(dir).exec();

        eprintln!("Failed to exec shell: {}", err);
    }
}
