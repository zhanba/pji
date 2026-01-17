use std::path::PathBuf;
use std::process::Command;

/// Represents a single git worktree
#[derive(Debug, Clone)]
pub struct GitWorktree {
    /// Path to the worktree directory
    pub path: PathBuf,
    /// Branch name (None for detached HEAD)
    pub branch: Option<String>,
    /// Current commit hash
    pub commit: String,
    /// Whether this is the main worktree (where .git is a directory)
    pub is_main: bool,
    /// Whether the worktree is locked
    pub locked: bool,
    /// Whether the worktree can be pruned
    pub prunable: bool,
}

impl GitWorktree {
    /// Get a display name for this worktree
    pub fn display_name(&self) -> String {
        if self.is_main {
            format!(
                "{} (main)",
                self.branch.as_deref().unwrap_or("detached")
            )
        } else {
            self.branch
                .as_deref()
                .unwrap_or(&self.commit[..8.min(self.commit.len())])
                .to_string()
        }
    }
}

/// Collection of worktrees for a repository
#[derive(Debug, Clone)]
pub struct WorktreeList {
    /// The main worktree (where .git is a directory)
    pub main: GitWorktree,
    /// Linked worktrees
    pub linked: Vec<GitWorktree>,
}

impl WorktreeList {
    /// Returns all worktrees (main + linked)
    pub fn all(&self) -> Vec<&GitWorktree> {
        let mut result = vec![&self.main];
        result.extend(self.linked.iter());
        result
    }

    /// Returns true if there are any linked worktrees
    pub fn has_linked(&self) -> bool {
        !self.linked.is_empty()
    }

    /// Total count of worktrees
    pub fn count(&self) -> usize {
        1 + self.linked.len()
    }
}

/// Parse the porcelain output of `git worktree list --porcelain`
///
/// Example output:
/// ```text
/// worktree /path/to/main
/// HEAD abc123
/// branch refs/heads/main
///
/// worktree /path/to/feature
/// HEAD def456
/// branch refs/heads/feature
/// ```
fn parse_worktree_porcelain(output: &str) -> Vec<GitWorktree> {
    let mut worktrees = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_commit: Option<String> = None;
    let mut current_branch: Option<String> = None;
    let mut is_bare = false;
    let mut is_locked = false;
    let mut is_prunable = false;

    for line in output.lines() {
        if line.starts_with("worktree ") {
            // Save previous worktree if exists
            if let (Some(path), Some(commit)) = (current_path.take(), current_commit.take()) {
                if !is_bare {
                    worktrees.push(GitWorktree {
                        path,
                        branch: current_branch.take(),
                        commit,
                        is_main: worktrees.is_empty(),
                        locked: is_locked,
                        prunable: is_prunable,
                    });
                }
            }
            // Start new worktree
            current_path = Some(PathBuf::from(&line[9..]));
            current_branch = None;
            is_bare = false;
            is_locked = false;
            is_prunable = false;
        } else if line.starts_with("HEAD ") {
            current_commit = Some(line[5..].to_string());
        } else if line.starts_with("branch ") {
            // Branch format is "refs/heads/branch-name"
            let branch_ref = &line[7..];
            if let Some(branch) = branch_ref.strip_prefix("refs/heads/") {
                current_branch = Some(branch.to_string());
            } else {
                current_branch = Some(branch_ref.to_string());
            }
        } else if line == "bare" {
            is_bare = true;
        } else if line == "locked" || line.starts_with("locked ") {
            is_locked = true;
        } else if line == "prunable" || line.starts_with("prunable ") {
            is_prunable = true;
        } else if line == "detached" {
            current_branch = None;
        }
    }

    // Don't forget the last worktree
    if let (Some(path), Some(commit)) = (current_path, current_commit) {
        if !is_bare {
            worktrees.push(GitWorktree {
                path,
                branch: current_branch,
                commit,
                is_main: worktrees.is_empty(),
                locked: is_locked,
                prunable: is_prunable,
            });
        }
    }

    worktrees
}

/// List all worktrees for a repository
///
/// # Arguments
/// * `repo_dir` - Path to the repository (can be main worktree or linked worktree)
///
/// # Returns
/// * `Some(WorktreeList)` if worktrees are found
/// * `None` if the command fails or no worktrees exist
pub fn list_worktrees(repo_dir: &PathBuf) -> Option<WorktreeList> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .arg("worktree")
        .arg("list")
        .arg("--porcelain")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let worktrees = parse_worktree_porcelain(&stdout);

    if worktrees.is_empty() {
        return None;
    }

    let mut iter = worktrees.into_iter();
    let main = iter.next()?;
    let linked: Vec<GitWorktree> = iter.collect();

    Some(WorktreeList { main, linked })
}

/// Check if a directory is a linked worktree (not the main worktree)
///
/// A linked worktree has a `.git` file (not directory) that points to the main repo.
pub fn is_linked_worktree(dir: &PathBuf) -> bool {
    let git_path = dir.join(".git");
    // A linked worktree has .git as a file, not a directory
    git_path.is_file()
}

/// Get the main repository path from a worktree directory
///
/// For a linked worktree, this reads the `.git` file and follows the gitdir reference.
/// For a main worktree, this returns the same path.
pub fn get_main_repo_from_worktree(worktree_dir: &PathBuf) -> Option<PathBuf> {
    let git_path = worktree_dir.join(".git");

    if git_path.is_dir() {
        // This is the main worktree
        return Some(worktree_dir.clone());
    }

    if git_path.is_file() {
        // This is a linked worktree, read the .git file
        let content = std::fs::read_to_string(&git_path).ok()?;
        // Format: "gitdir: /path/to/main/.git/worktrees/name"
        let gitdir = content.trim().strip_prefix("gitdir: ")?;
        let gitdir_path = PathBuf::from(gitdir);

        // Navigate up from .git/worktrees/name to the main repo
        // .git/worktrees/name -> .git/worktrees -> .git -> repo
        let main_git_dir = gitdir_path.parent()?.parent()?.parent()?;
        return Some(main_git_dir.to_path_buf());
    }

    None
}

/// Add a new worktree
///
/// # Arguments
/// * `repo_dir` - Path to the repository
/// * `branch` - Branch name to checkout (or create with -b)
/// * `path` - Optional path for the worktree (defaults to {repo}.worktrees/{branch})
/// * `create_branch` - If true, create a new branch
///
/// # Returns
/// * `Ok(PathBuf)` - Path to the created worktree
/// * `Err(String)` - Error message
pub fn add_worktree(
    repo_dir: &PathBuf,
    branch: &str,
    path: Option<PathBuf>,
    create_branch: bool,
) -> Result<PathBuf, String> {
    // Determine worktree path
    let worktree_path = match path {
        Some(p) => p,
        None => {
            // Default: {repo}.worktrees/{branch}
            let repo_name = repo_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("repo");
            let worktrees_dir = repo_dir
                .parent()
                .ok_or("Cannot determine parent directory")?
                .join(format!("{}.worktrees", repo_name));

            // Sanitize branch name for filesystem
            let safe_branch = branch.replace('/', "-");
            worktrees_dir.join(&safe_branch)
        }
    };

    // Build the git worktree add command
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(repo_dir)
        .arg("worktree")
        .arg("add");

    if create_branch {
        cmd.arg("-b").arg(branch);
    }

    cmd.arg(&worktree_path);

    if !create_branch {
        cmd.arg(branch);
    }

    let output = cmd.output().map_err(|e| e.to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(stderr.to_string());
    }

    Ok(worktree_path)
}

/// Remove a worktree
///
/// # Arguments
/// * `repo_dir` - Path to the repository
/// * `worktree_path` - Path to the worktree to remove
/// * `force` - If true, force removal even if the worktree is dirty
pub fn remove_worktree(repo_dir: &PathBuf, worktree_path: &PathBuf, force: bool) -> Result<(), String> {
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(repo_dir)
        .arg("worktree")
        .arg("remove");

    if force {
        cmd.arg("--force");
    }

    cmd.arg(worktree_path);

    let output = cmd.output().map_err(|e| e.to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(stderr.to_string());
    }

    Ok(())
}

/// Prune stale worktree information
pub fn prune_worktrees(repo_dir: &PathBuf) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .arg("worktree")
        .arg("prune")
        .arg("-v")
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(stderr.to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_worktree_porcelain_single() {
        let output = r#"worktree /home/user/repo
HEAD abc123def456
branch refs/heads/main
"#;
        let worktrees = parse_worktree_porcelain(output);
        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0].path, PathBuf::from("/home/user/repo"));
        assert_eq!(worktrees[0].branch, Some("main".to_string()));
        assert_eq!(worktrees[0].commit, "abc123def456");
        assert!(worktrees[0].is_main);
    }

    #[test]
    fn test_parse_worktree_porcelain_multiple() {
        let output = r#"worktree /home/user/repo
HEAD abc123
branch refs/heads/main

worktree /home/user/repo.worktrees/feature
HEAD def456
branch refs/heads/feature
"#;
        let worktrees = parse_worktree_porcelain(output);
        assert_eq!(worktrees.len(), 2);
        assert!(worktrees[0].is_main);
        assert!(!worktrees[1].is_main);
        assert_eq!(worktrees[1].branch, Some("feature".to_string()));
    }

    #[test]
    fn test_parse_worktree_porcelain_detached() {
        let output = r#"worktree /home/user/repo
HEAD abc123
branch refs/heads/main

worktree /home/user/repo.worktrees/detached
HEAD def456
detached
"#;
        let worktrees = parse_worktree_porcelain(output);
        assert_eq!(worktrees.len(), 2);
        assert!(worktrees[1].branch.is_none());
    }

    #[test]
    fn test_parse_worktree_porcelain_locked() {
        let output = r#"worktree /home/user/repo
HEAD abc123
branch refs/heads/main

worktree /home/user/repo.worktrees/locked-wt
HEAD def456
branch refs/heads/locked-branch
locked
"#;
        let worktrees = parse_worktree_porcelain(output);
        assert_eq!(worktrees.len(), 2);
        assert!(worktrees[1].locked);
    }

    #[test]
    fn test_worktree_display_name() {
        let main_wt = GitWorktree {
            path: PathBuf::from("/repo"),
            branch: Some("main".to_string()),
            commit: "abc123".to_string(),
            is_main: true,
            locked: false,
            prunable: false,
        };
        assert_eq!(main_wt.display_name(), "main (main)");

        let linked_wt = GitWorktree {
            path: PathBuf::from("/repo.worktrees/feature"),
            branch: Some("feature".to_string()),
            commit: "def456".to_string(),
            is_main: false,
            locked: false,
            prunable: false,
        };
        assert_eq!(linked_wt.display_name(), "feature");

        let detached_wt = GitWorktree {
            path: PathBuf::from("/repo.worktrees/detached"),
            branch: None,
            commit: "abc123def456".to_string(),
            is_main: false,
            locked: false,
            prunable: false,
        };
        assert_eq!(detached_wt.display_name(), "abc123de");
    }
}
