use anyhow::{anyhow, Context, Result};
use arboard::Clipboard;
use comfy_table::Table;
use dialoguer::{
    console::{style, Key, Term},
    Confirm, FuzzySelect, Select,
};
use pji::{
    AddWorktreeRequest, Pji, PjiError, RemoveWorktreeRequest, Repository, Worktree, WorktreeList,
};
use std::env;
use std::fs::{create_dir_all, remove_dir_all, remove_file};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

/// Runtime behavior selected by CLI flags and terminal detection.
pub struct AppOptions {
    /// Whether pji may prompt, fuzzy-select, copy to the clipboard, or open a shell.
    pub interactive: bool,
    /// Explicit root to use when a command cannot safely choose one.
    pub root: Option<PathBuf>,
}

impl Default for AppOptions {
    fn default() -> Self {
        Self {
            interactive: true,
            root: None,
        }
    }
}

pub struct PjiApp {
    pji: Pji,
    options: AppOptions,
}

impl PjiApp {
    pub fn new(options: AppOptions) -> Result<Self> {
        let pji = Pji::load().context("failed to load pji data")?;
        Ok(Self { pji, options })
    }

    pub fn start_config(&mut self, root: Option<PathBuf>) -> Result<()> {
        if let Some(root) = root {
            self.add_root_path(root)?;
        } else {
            self.add_root()?;
        }
        Ok(())
    }

    fn add_root(&mut self) -> Result<Option<PathBuf>> {
        if !self.options.interactive {
            return Err(anyhow!(
                "root path is required in non-interactive mode; pass `pji -n config <ROOT>`"
            ));
        }

        let Some(name) = Self::input_text(
            "Enter the full path for the new pji root directory",
            Some(
                Pji::default_root()
                    .context("failed to determine default pji root")?
                    .display()
                    .to_string(),
            ),
        )?
        else {
            return Ok(None);
        };
        self.add_root_path(PathBuf::from(&name))
    }

    fn add_root_path(&mut self, path: PathBuf) -> Result<Option<PathBuf>> {
        if self.pji.roots().contains(&path) {
            Self::warn_message(&format!(
                "Root '{}' already exists. Please choose another.",
                path.display()
            ));
            return Ok(None);
        }

        if !path.exists() {
            println!("Creating directory '{}'...", path.display());
            create_dir_all(&path)
                .with_context(|| format!("failed to create directory '{}'", path.display()))?;
            Self::success_message(&format!("Directory '{}' created.", path.display()));
        }

        self.pji.add_root(path.clone());
        self.pji
            .save()
            .context("failed to save pji config after adding root")?;
        Self::success_message(&format!("Root '{}' added successfully.", path.display()));
        Ok(Some(path))
    }

    fn get_working_root(&mut self) -> Result<Option<PathBuf>> {
        if let Some(root) = &self.options.root {
            return Ok(Some(root.clone()));
        }

        match self.pji.roots().len() {
            0 => {
                if !self.options.interactive {
                    return Err(anyhow!(
                        "no pji roots found in non-interactive mode; pass `--root <DIR>` or run `pji config <ROOT>`"
                    ));
                }
                Self::warn_message("No pji roots found. Let's add one first.");
                self.add_root()
            }
            1 => Ok(Some(self.pji.roots()[0].clone())),
            _ => {
                if !self.options.interactive {
                    return Err(anyhow!(
                        "multiple pji roots found in non-interactive mode; pass `--root <DIR>`"
                    ));
                }
                let items = self
                    .pji
                    .roots()
                    .iter()
                    .map(|x| x.display().to_string())
                    .collect::<Vec<_>>();
                let selection = Select::new()
                    .with_prompt("Select root directory")
                    .default(0)
                    .items(&items)
                    .interact_opt()
                    .context("failed to select root directory")?;
                Ok(selection.map(|idx| self.pji.roots()[idx].clone()))
            }
        }
    }

    pub fn add(&mut self, repo_uri_str: &str) -> Result<()> {
        let Some(root) = self.get_working_root()? else {
            return Ok(());
        };
        if self.pji.is_repository_registered(repo_uri_str, &root)? {
            Self::warn_message(&format!(
                "Repository '{}' already exists in pji.",
                repo_uri_str
            ));
            return Ok(());
        }

        let git = Pji::parse_git_url(repo_uri_str)?;
        let repo_dir = Pji::repository_path(&root, &git);
        println!(
            "Cloning '{}' into '{}'...",
            repo_uri_str,
            repo_dir.display()
        );
        let repo = self
            .pji
            .clone_repository(repo_uri_str, &root)
            .with_context(|| {
                format!(
                    "failed to clone '{}' into '{}'",
                    repo_uri_str,
                    repo_dir.display()
                )
            })?;
        self.pji
            .save()
            .context("failed to save pji metadata after adding repository")?;

        Self::success_message(&format!(
            "✨ Repository '{}' added to '{}'.",
            &repo.git.original,
            &repo.dir.display()
        ));
        if self.options.interactive {
            Self::copy_to_clipboard(
                &format!("cd {}", repo.dir.display()),
                "Paste to navigate to the repository.",
            )?;
        } else {
            println!("{}", repo.dir.display());
        }
        Ok(())
    }

    pub fn remove(&mut self, repo_uri_str: &str, yes: bool) -> Result<()> {
        let Some(root) = self.get_working_root()? else {
            return Ok(());
        };
        if !self.pji.is_repository_registered(repo_uri_str, &root)? {
            Self::warn_message(&format!("Repository '{}' not found in pji.", repo_uri_str));
            return Ok(());
        }

        let git = Pji::parse_git_url(repo_uri_str)?;
        let repo_dir = Pji::repository_path(&root, &git);
        let confirmation = self.confirm_or_require_yes(
            &format!(
                "Are you sure you want to remove the repository '{}' from disk and pji?",
                repo_uri_str
            ),
            yes,
        )?;
        if !confirmation {
            println!("✖️ Removal cancelled.");
            return Ok(());
        }

        println!("Removing directory '{}'...", repo_dir.display());
        remove_dir_all(&repo_dir)
            .with_context(|| format!("failed to remove directory '{}'", repo_dir.display()))?;
        self.pji.unregister_repository(repo_uri_str, &root)?;
        self.pji
            .save()
            .context("failed to save pji metadata after removing repository")?;
        Self::success_message(&format!(
            "🗑️ Repository '{}' removed successfully from '{}'.",
            repo_uri_str,
            repo_dir.display()
        ));
        Ok(())
    }

    pub fn list(&mut self, long_format: bool) -> Result<()> {
        let repos = self.pji.repositories_by_last_opened();
        if long_format {
            self.print_compact_repo_list(&repos, Self::terminal_width())?;
        } else {
            repos.iter().for_each(|repo| {
                println!("{}", repo.dir.display());
            });
        }
        Ok(())
    }

    fn print_compact_repo_list(&self, repos: &[Repository], width: usize) -> Result<()> {
        let width = width.max(48);
        let repo_width = (width / 3).clamp(18, 34);
        let path_width = width.saturating_sub(repo_width + 5).max(12);

        println!(
            "{:<repo_width$} {:>3} {}",
            "repo",
            "wt",
            "path",
            repo_width = repo_width
        );
        println!("{}", "-".repeat(width.min(repo_width + path_width + 5)));

        for repo in repos {
            let repo_name = format!("{}/{}", repo.git.owner, repo.git.name);
            let worktree_count = if repo.dir.exists() {
                match self.pji.list_worktrees(&repo.dir) {
                    Ok(wts) if wts.has_linked() => format!("{}", wts.count()),
                    Ok(_) | Err(PjiError::InvalidWorktree(_)) => "-".to_string(),
                    Err(err) => return Err(err).context("failed to list worktrees"),
                }
            } else {
                "missing".to_string()
            };
            println!(
                "{:<repo_width$} {:>3} {}",
                Self::truncate_middle(&repo_name, repo_width),
                Self::truncate_middle(&worktree_count, 3),
                Self::truncate_middle(&Self::repo_display_path(repo), path_width),
                repo_width = repo_width
            );
        }

        Ok(())
    }

    fn repo_display_path(repo: &Repository) -> String {
        repo.dir
            .strip_prefix(&repo.root)
            .unwrap_or(&repo.dir)
            .display()
            .to_string()
    }

    fn terminal_width() -> usize {
        let term = Term::stdout();
        if term.is_term() {
            usize::from(term.size().1)
        } else if let Ok(columns) = env::var("COLUMNS") {
            columns.parse().unwrap_or(120)
        } else {
            120
        }
    }

    fn truncate_middle(value: &str, max_chars: usize) -> String {
        let len = value.chars().count();
        if len <= max_chars {
            return value.to_string();
        }
        if max_chars <= 3 {
            return ".".repeat(max_chars);
        }

        let keep = max_chars - 3;
        let front = (keep + 1) / 2;
        let back = keep / 2;
        let prefix: String = value.chars().take(front).collect();
        let suffix: String = value
            .chars()
            .rev()
            .take(back)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("{prefix}...{suffix}")
    }

    pub fn find(&mut self, query: &str) -> Result<()> {
        let Some(repo) = self.find_repo("🔍 Search and select repository", query)? else {
            return Ok(());
        };
        self.pji.mark_repository_opened(&repo.dir);
        self.pji
            .save()
            .context("failed to save pji metadata before opening repository")?;

        if !self.options.interactive {
            println!("{}", repo.dir.display());
            return Ok(());
        }

        match self.pji.list_worktrees(&repo.dir) {
            Ok(worktrees) if worktrees.has_linked() => {
                if let Some(wt) = self.select_worktree(&worktrees, "")? {
                    return self.exec_into_dir(&wt.path);
                }
            }
            Ok(_) | Err(PjiError::InvalidWorktree(_)) => {}
            Err(err) => return Err(err).context("failed to list worktrees"),
        }

        self.exec_into_dir(&repo.dir)
    }

    pub fn scan(&mut self) -> Result<()> {
        for root in self.pji.roots() {
            println!("🔍 Scanning {}...", root.display());
        }

        let report = self.pji.scan().context("failed to scan repositories")?;
        for repo in &report.added {
            println!("  ✨ Added: {}", repo.dir.display());
        }

        if !report.issues.is_empty() {
            Self::warn_message("The following paths were found but are not valid pji repositories or have an unexpected structure:");
            for issue in &report.issues {
                println!("  - {} ({})", issue.path.display(), issue.message);
            }
        }

        self.pji
            .save()
            .context("failed to save pji metadata after scanning repositories")?;
        if !report.added.is_empty() {
            let repo_str = if report.added.len() == 1 {
                "repository"
            } else {
                "repositories"
            };
            Self::success_message(&format!(
                "Scan complete. {} new {} added.",
                report.added.len(),
                repo_str
            ));
        } else {
            Self::success_message("Scan complete. No new repositories found.");
        }
        Ok(())
    }

    pub fn clean() -> Result<()> {
        if let Ok(config_path) = Pji::config_file_path() {
            if config_path.exists() {
                remove_file(&config_path).with_context(|| {
                    format!("failed to remove config file '{}'", config_path.display())
                })?;
            }
        }

        if let Ok(metadata_path) = Pji::metadata_file_path() {
            if metadata_path.exists() {
                remove_file(&metadata_path).with_context(|| {
                    format!(
                        "failed to remove metadata file '{}'",
                        metadata_path.display()
                    )
                })?;
            }
        }

        Self::success_message("🧹 Project data cleaned successfully.");
        Ok(())
    }

    pub fn open_home(&mut self, query: Option<String>) -> Result<()> {
        let repo = match query {
            Some(query) => {
                let Some(repo) = self.find_repo("Open repo: ", &query)? else {
                    return Ok(());
                };
                repo
            }
            None => self
                .get_cwd_repo()
                .context("No repo found in current directory")?,
        };

        let url = repo
            .home_url()
            .ok_or_else(|| anyhow!("No home URL found for {}", repo.git.original))?;
        Self::open_url(&url)
    }

    pub fn open_pr(&self, pr: Option<u32>) -> Result<()> {
        let repo = self
            .get_cwd_repo()
            .context("No repo found in current directory")?;

        let url = repo
            .pull_request_url(pr)
            .ok_or_else(|| anyhow!("No PR found for {}", repo.git.original))?;
        Self::open_url(&url)
    }

    pub fn open_issue(&self, issue: Option<u32>) -> Result<()> {
        let repo = self
            .get_cwd_repo()
            .context("No repo found in current directory")?;
        let url = repo
            .issue_url(issue)
            .ok_or_else(|| anyhow!("No issue found for {}", repo.git.original))?;
        Self::open_url(&url)
    }

    fn get_cwd_repo(&self) -> Option<Repository> {
        let cwd = env::current_dir().ok()?;
        let repo_dir = Pji::resolve_git_dir(&cwd).unwrap_or(cwd);
        self.pji.resolve_repository(&repo_dir)
    }

    fn open_url(url: &str) -> Result<()> {
        println!("🌐 Opening URL in browser: {}", style(url).cyan());
        webbrowser::open(url).with_context(|| format!("failed to open browser for '{url}'"))?;
        Ok(())
    }

    fn find_repo(&self, prompt: &str, query: &str) -> Result<Option<Repository>> {
        let repos = self.pji.repositories_by_last_opened();
        if repos.is_empty() {
            return Ok(None);
        }

        if !self.options.interactive {
            // Scripts need deterministic output; fuzzy selection is only safe
            // when the query resolves to exactly one repository.
            let matches = repos
                .into_iter()
                .filter(|repo| Self::repo_matches(repo, query))
                .collect::<Vec<_>>();
            return Self::select_single_repository(matches, query);
        }

        let mut counts = std::collections::HashMap::new();
        for repo in &repos {
            let key = format!("{}/{}", repo.git.owner, repo.git.name);
            *counts.entry(key).or_insert(0) += 1;
        }

        let items = repos
            .iter()
            .map(|repo| {
                let key = format!("{}/{}", repo.git.owner, repo.git.name);
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
            .interact_opt()
            .context("failed to select repository")?;

        Ok(selection.and_then(|idx| repos.get(idx).cloned()))
    }

    fn success_message(message: &str) {
        println!("🚀 {}", style(message).green());
    }

    fn warn_message(message: &str) {
        println!("⚠️  {}", style(message).yellow());
    }

    fn copy_to_clipboard(text: &str, context_message: &str) -> Result<()> {
        Clipboard::new()
            .context("can't find clipboard")?
            .set_text(text)
            .context("can't set clipboard")?;

        println!(
            "📋 Copied \"{}\" to clipboard. {}",
            style(text).green(),
            context_message
        );
        Ok(())
    }

    fn confirm(message: &str) -> Result<bool> {
        let confirmation = Confirm::new()
            .with_prompt(format!("❓ {}", style(message).yellow()))
            .interact_opt()
            .context("failed to read confirmation")?;
        Ok(confirmation.unwrap_or(false))
    }

    fn confirm_or_require_yes(&self, message: &str, yes: bool) -> Result<bool> {
        if yes {
            return Ok(true);
        }

        if !self.options.interactive {
            return Err(anyhow!(
                "{} Pass `--yes` to confirm in non-interactive mode.",
                message
            ));
        }

        Self::confirm(message)
    }

    fn input_text(prompt: &str, default: Option<String>) -> Result<Option<String>> {
        let term = Term::stderr();
        let prompt_text = match default.as_deref() {
            Some(default) => format!("{} [{}]: ", prompt, default),
            None => format!("{}: ", prompt),
        };

        term.write_str(&prompt_text)?;
        term.flush()?;

        let mut input = String::new();
        loop {
            match term.read_key()? {
                Key::Enter => {
                    term.write_line("")?;
                    if input.is_empty() {
                        return Ok(default);
                    }
                    return Ok(Some(input));
                }
                Key::Escape => {
                    term.write_line("")?;
                    return Ok(None);
                }
                Key::Backspace => {
                    if input.pop().is_some() {
                        term.clear_chars(1)?;
                    }
                }
                Key::Char(ch) if !ch.is_ascii_control() => {
                    input.push(ch);
                    term.write_str(&ch.to_string())?;
                }
                _ => {}
            }
            term.flush()?;
        }
    }

    pub fn worktree_list(&mut self, query: Option<String>) -> Result<()> {
        let repo_dir = match self.get_worktree_repo_dir(query)? {
            Some(dir) => dir,
            None => {
                Self::warn_message("No repository found.");
                return Ok(());
            }
        };

        match self.pji.list_worktrees(&repo_dir) {
            Ok(worktrees) => {
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
                        wt.branch.clone().unwrap_or_else(|| {
                            format!("({})", &wt.commit[..8.min(wt.commit.len())])
                        }),
                        status,
                    ]);
                }

                println!("{table}");
                println!("\nTotal: {} worktree(s)", worktrees.count());
            }
            Err(PjiError::InvalidWorktree(_)) => {
                println!("No worktrees found for this repository.");
            }
            Err(err) => return Err(err).context("failed to list worktrees"),
        }
        Ok(())
    }

    pub fn worktree_switch(&mut self, query: Option<String>) -> Result<()> {
        let repo_dir = match self.get_worktree_repo_dir(None)? {
            Some(dir) => dir,
            None => {
                Self::warn_message("No repository found in current directory.");
                return Ok(());
            }
        };

        let worktrees = match self.pji.list_worktrees(&repo_dir) {
            Ok(wts) if wts.count() > 1 => wts,
            Ok(_) => {
                Self::warn_message("Only one worktree exists. Nothing to switch to.");
                return Ok(());
            }
            Err(PjiError::InvalidWorktree(_)) => {
                Self::warn_message("No worktrees found for this repository.");
                return Ok(());
            }
            Err(err) => return Err(err).context("failed to list worktrees"),
        };

        if let Some(wt) = self.select_worktree(&worktrees, query.as_deref().unwrap_or(""))? {
            self.exec_into_dir(&wt.path)?;
        }
        Ok(())
    }

    pub fn worktree_add(
        &mut self,
        branch: Option<String>,
        path: Option<PathBuf>,
        base_branch: Option<String>,
        create_new: bool,
    ) -> Result<()> {
        let repo_dir = match self.get_worktree_repo_dir(None)? {
            Some(dir) => dir,
            None => {
                Self::warn_message("No repository found in current directory.");
                return Ok(());
            }
        };

        let branch_from_args = branch.is_some();
        let (final_branch, create_new, base_branch) = match branch {
            Some(branch) => (branch, create_new || base_branch.is_some(), base_branch),
            None if self.options.interactive => match self.select_branch_for_worktree(&repo_dir)? {
                Some(result) => result,
                None => return Ok(()),
            },
            None => {
                return Err(anyhow!(
                    "branch is required in non-interactive mode; pass `pji -n wt add <BRANCH>`"
                ))
            }
        };

        let default_path = Pji::default_worktree_path(&repo_dir, &final_branch);
        let worktree_path = match path {
            Some(path) => path,
            None if branch_from_args || !self.options.interactive => default_path,
            None => {
                let Some(worktree_path) =
                    Self::input_text("Worktree path", Some(default_path.display().to_string()))?
                else {
                    return Ok(());
                };
                PathBuf::from(worktree_path)
            }
        };

        let created_path = self.pji.add_worktree(AddWorktreeRequest {
            repo_dir,
            branch: final_branch,
            path: Some(worktree_path),
            create_branch: create_new,
            base_branch,
        })?;

        Self::success_message(&format!("Worktree created at '{}'", created_path.display()));
        self.exec_into_dir(&created_path)
    }

    fn select_branch_for_worktree(
        &self,
        repo_dir: &PathBuf,
    ) -> Result<Option<(String, bool, Option<String>)>> {
        let source_options = vec!["Local branch", "Remote branch", "New branch"];
        let source_selection = Select::new()
            .with_prompt("Select branch source")
            .default(0)
            .items(&source_options)
            .interact_opt()
            .context("failed to select branch source")?;

        let result = match source_selection {
            Some(0) => {
                let local_branches = self.pji.local_branches(repo_dir);
                if local_branches.is_empty() {
                    Self::warn_message("No local branches found.");
                    return Ok(None);
                }

                let local_branches = Self::prioritize_main_branch(local_branches, "main");
                let selection = FuzzySelect::new()
                    .with_prompt("Select local branch")
                    .default(0)
                    .highlight_matches(true)
                    .max_length(10)
                    .items(&local_branches)
                    .interact_opt()
                    .context("failed to select local branch")?;

                selection.map(|idx| (local_branches[idx].clone(), false, None))
            }
            Some(1) => {
                let remote_branches = self.pji.remote_branches(repo_dir);
                if remote_branches.is_empty() {
                    Self::warn_message("No remote branches found. Try running 'git fetch' first.");
                    return Ok(None);
                }

                let remote_branches = Self::prioritize_main_branch(remote_branches, "origin/main");
                let selection = FuzzySelect::new()
                    .with_prompt("Select remote branch")
                    .default(0)
                    .highlight_matches(true)
                    .max_length(10)
                    .items(&remote_branches)
                    .interact_opt()
                    .context("failed to select remote branch")?;

                selection.map(|idx| (remote_branches[idx].clone(), true, None))
            }
            Some(2) => {
                let Some(new_branch_name) = Self::input_text("Enter new branch name", None)? else {
                    return Ok(None);
                };

                if new_branch_name.is_empty() {
                    Self::warn_message("Branch name cannot be empty.");
                    return Ok(None);
                }

                let mut all_branches: Vec<String> = Vec::new();
                all_branches.extend(self.pji.remote_branches(repo_dir));
                all_branches.extend(self.pji.local_branches(repo_dir));

                if all_branches.is_empty() {
                    return Ok(Some((new_branch_name, true, None)));
                }

                let all_branches = Self::prioritize_main_branch(all_branches, "origin/main");
                let selection = FuzzySelect::new()
                    .with_prompt("Select base branch")
                    .default(0)
                    .highlight_matches(true)
                    .max_length(10)
                    .items(&all_branches)
                    .interact_opt()
                    .context("failed to select base branch")?;

                selection.map(|idx| (new_branch_name, true, Some(all_branches[idx].clone())))
            }
            _ => None,
        };
        Ok(result)
    }

    fn prioritize_main_branch(mut branches: Vec<String>, preferred: &str) -> Vec<String> {
        if let Some(idx) = branches.iter().position(|b| b == preferred) {
            let branch = branches.remove(idx);
            branches.insert(0, branch);
        } else if preferred == "origin/main" {
            if let Some(idx) = branches.iter().position(|b| b == "main") {
                let branch = branches.remove(idx);
                branches.insert(0, branch);
            }
        }
        branches
    }

    pub fn worktree_remove(
        &mut self,
        worktree: Option<String>,
        force: bool,
        yes: bool,
    ) -> Result<()> {
        let repo_dir = match self.get_worktree_repo_dir(None)? {
            Some(dir) => dir,
            None => {
                Self::warn_message("No repository found in current directory.");
                return Ok(());
            }
        };

        let worktrees = match self.pji.list_worktrees(&repo_dir) {
            Ok(wts) if !wts.linked.is_empty() => wts,
            Ok(_) => {
                Self::warn_message("No linked worktrees to remove.");
                return Ok(());
            }
            Err(PjiError::InvalidWorktree(_)) => {
                Self::warn_message("No worktrees found for this repository.");
                return Ok(());
            }
            Err(err) => return Err(err).context("failed to list worktrees"),
        };

        let worktree_path = match worktree {
            Some(wt_str) => {
                let found = worktrees
                    .linked
                    .iter()
                    .filter(|wt| {
                        wt.path.to_string_lossy().contains(&wt_str)
                            || wt.branch.as_deref() == Some(&wt_str)
                    })
                    .collect::<Vec<_>>();
                match found.len() {
                    0 => {
                        Self::warn_message(&format!("Worktree '{}' not found.", wt_str));
                        return Ok(());
                    }
                    1 => found[0].path.clone(),
                    _ => {
                        return Err(anyhow!(
                            "worktree '{}' matched multiple worktrees: {}",
                            wt_str,
                            found
                                .iter()
                                .map(|wt| wt.path.display().to_string())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }
                }
            }
            None => {
                if !self.options.interactive {
                    return Err(anyhow!(
                        "worktree path or branch is required in non-interactive mode"
                    ));
                }
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
                    .interact_opt()
                    .context("failed to select worktree to remove")?;

                let Some(selection) = selection else {
                    return Ok(());
                };
                worktrees.linked[selection].path.clone()
            }
        };

        if !self.confirm_or_require_yes(
            &format!("Remove worktree at '{}'?", worktree_path.display()),
            yes,
        )? {
            println!("Removal cancelled.");
            return Ok(());
        }

        self.pji.remove_worktree(RemoveWorktreeRequest {
            repo_dir,
            worktree_path: worktree_path.clone(),
            force,
        })?;
        Self::success_message(&format!(
            "Worktree '{}' removed successfully.",
            worktree_path.display()
        ));
        Ok(())
    }

    pub fn worktree_prune(&self) -> Result<()> {
        let repo_dir = match self.get_cwd_repo_dir() {
            Some(dir) => dir,
            None => {
                Self::warn_message("No repository found in current directory.");
                return Ok(());
            }
        };

        let output = self.pji.prune_worktrees(&repo_dir)?;
        if output.is_empty() {
            println!("No stale worktree entries to prune.");
        } else {
            println!("{}", output);
            Self::success_message("Worktree pruning complete.");
        }
        Ok(())
    }

    fn get_worktree_repo_dir(&self, query: Option<String>) -> Result<Option<PathBuf>> {
        let repo_dir = match query {
            Some(q) => self
                .find_repo("Select repository: ", &q)?
                .map(|repo| repo.dir),
            None => self.get_cwd_repo_dir(),
        };
        Ok(repo_dir)
    }

    fn get_cwd_repo_dir(&self) -> Option<PathBuf> {
        let cwd = env::current_dir().ok()?;
        let git_dir = Pji::resolve_git_dir(&cwd).unwrap_or_else(|| cwd.clone());
        if git_dir != cwd {
            return Some(git_dir);
        }

        self.pji.resolve_repository(&cwd).map(|repo| repo.dir)
    }

    fn select_worktree<'a>(
        &self,
        worktrees: &'a WorktreeList,
        query: &str,
    ) -> Result<Option<&'a Worktree>> {
        let all_worktrees = worktrees.all();

        if !self.options.interactive {
            let matches = all_worktrees
                .iter()
                .copied()
                .filter(|wt| Self::worktree_matches(wt, query))
                .collect::<Vec<_>>();
            return Self::select_single_worktree(matches, query);
        }

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
            .interact_opt()
            .context("failed to select worktree")?;

        Ok(selection.and_then(|idx| all_worktrees.get(idx).copied()))
    }

    #[cfg(unix)]
    fn exec_into_dir(&self, dir: &PathBuf) -> Result<()> {
        if !self.options.interactive {
            // In non-interactive mode, return a path the caller can consume
            // instead of replacing the process with an interactive shell.
            println!("{}", dir.display());
            return Ok(());
        }

        let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let err = Command::new(&shell).current_dir(dir).exec();

        Err(anyhow!(
            "failed to exec shell in '{}': {}",
            dir.display(),
            err
        ))
    }

    #[cfg(windows)]
    fn exec_into_dir(&self, dir: &PathBuf) -> Result<()> {
        if !self.options.interactive {
            // In non-interactive mode, return a path the caller can consume
            // instead of spawning an interactive shell.
            println!("{}", dir.display());
            return Ok(());
        }

        let shell = env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string());

        Command::new(&shell)
            .current_dir(dir)
            .spawn()
            .and_then(|mut child| child.wait())
            .with_context(|| format!("failed to spawn shell in '{}'", dir.display()))?;
        Ok(())
    }

    fn repo_matches(repo: &Repository, query: &str) -> bool {
        let query = query.to_lowercase();
        query.is_empty()
            || repo.git.owner.to_lowercase().contains(&query)
            || repo.git.name.to_lowercase().contains(&query)
            || repo.git.original.to_lowercase().contains(&query)
            || repo.dir.to_string_lossy().to_lowercase().contains(&query)
    }

    fn select_single_repository(repos: Vec<Repository>, query: &str) -> Result<Option<Repository>> {
        match repos.len() {
            0 => Ok(None),
            1 => Ok(repos.into_iter().next()),
            _ => {
                let matches = repos
                    .iter()
                    .map(|repo| repo.dir.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                if query.is_empty() {
                    Err(anyhow!(
                        "repository query is required in non-interactive mode; multiple repositories are available: {}",
                        matches
                    ))
                } else {
                    Err(anyhow!(
                        "repository query '{}' matched multiple repositories: {}",
                        query,
                        matches
                    ))
                }
            }
        }
    }

    fn worktree_matches(worktree: &Worktree, query: &str) -> bool {
        let query = query.to_lowercase();
        query.is_empty()
            || worktree
                .branch
                .as_deref()
                .unwrap_or("detached")
                .to_lowercase()
                .contains(&query)
            || worktree
                .path
                .to_string_lossy()
                .to_lowercase()
                .contains(&query)
            || worktree.display_name().to_lowercase().contains(&query)
    }

    fn select_single_worktree<'a>(
        worktrees: Vec<&'a Worktree>,
        query: &str,
    ) -> Result<Option<&'a Worktree>> {
        match worktrees.len() {
            0 => Ok(None),
            1 => Ok(worktrees.into_iter().next()),
            _ => {
                let matches = worktrees
                    .iter()
                    .map(|wt| wt.path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                if query.is_empty() {
                    Err(anyhow!(
                        "worktree query is required in non-interactive mode; multiple worktrees are available: {}",
                        matches
                    ))
                } else {
                    Err(anyhow!(
                        "worktree query '{}' matched multiple worktrees: {}",
                        query,
                        matches
                    ))
                }
            }
        }
    }
}
