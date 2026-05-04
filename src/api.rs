use crate::{
    config::{PjiConfig, PjiMetadata},
    error::PjiError,
    repo::{GitProtocol, GitURI, PjiRepo},
    util::{list_dir, parse_git_url, try_get_repo_from_dir},
    worktree::{
        self, add_worktree, get_default_worktree_path, get_main_repo_from_worktree,
        is_linked_worktree, list_local_branches, list_remote_branches, list_worktrees,
        prune_worktrees, remove_worktree,
    },
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Protocol {
    #[serde(rename = "SSH", alias = "Ssh")]
    Ssh,
    #[serde(rename = "HTTP", alias = "Https")]
    Https,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitUrl {
    pub hostname: String,
    pub owner: String,
    pub name: String,
    pub protocol: Protocol,
    pub original: String,
}

impl GitUrl {
    pub fn parse(url: &str) -> Result<Self, PjiError> {
        parse_git_url(url)
            .map(Self::from)
            .ok_or_else(|| PjiError::InvalidGitUrl(url.to_string()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub git: GitUrl,
    pub dir: PathBuf,
    pub root: PathBuf,
    pub created_at: DateTime<Utc>,
    pub last_opened_at: DateTime<Utc>,
}

impl Repository {
    pub fn home_url(&self) -> Option<String> {
        match self.git.hostname.as_str() {
            "github.com" => Some(format!(
                "https://github.com/{}/{}",
                self.git.owner, self.git.name
            )),
            _ => None,
        }
    }

    pub fn issue_url(&self, issue: Option<u32>) -> Option<String> {
        match self.git.hostname.as_str() {
            "github.com" => match issue {
                Some(issue) => Some(format!(
                    "https://github.com/{}/{}/issues/{}",
                    self.git.owner, self.git.name, issue
                )),
                None => Some(format!(
                    "https://github.com/{}/{}/issues",
                    self.git.owner, self.git.name
                )),
            },
            _ => None,
        }
    }

    pub fn pull_request_url(&self, pr: Option<u32>) -> Option<String> {
        match self.git.hostname.as_str() {
            "github.com" => match pr {
                Some(pr) => Some(format!(
                    "https://github.com/{}/{}/pull/{}",
                    self.git.owner, self.git.name, pr
                )),
                None => Some(format!(
                    "https://github.com/{}/{}/pull",
                    self.git.owner, self.git.name
                )),
            },
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanReport {
    pub added: Vec<Repository>,
    pub invalid_paths: Vec<PathBuf>,
    pub issues: Vec<ScanIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanIssue {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct AddWorktreeRequest {
    pub repo_dir: PathBuf,
    pub branch: String,
    pub path: Option<PathBuf>,
    pub create_branch: bool,
    pub base_branch: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RemoveWorktreeRequest {
    pub repo_dir: PathBuf,
    pub worktree_path: PathBuf,
    pub force: bool,
}

pub type Worktree = worktree::GitWorktree;
pub type WorktreeList = worktree::WorktreeList;

#[derive(Debug, Clone)]
pub struct Pji {
    config: PjiConfig,
    metadata: PjiMetadata,
}

impl Pji {
    pub fn load() -> Result<Self, PjiError> {
        let config = PjiConfig::try_load().map_err(PjiError::Config)?;
        let metadata = PjiMetadata::try_load().map_err(PjiError::Metadata)?;
        Ok(Self { config, metadata })
    }

    pub fn config_file_path() -> Result<PathBuf, PjiError> {
        PjiConfig::get_config_file_path().map_err(PjiError::Config)
    }

    pub fn metadata_file_path() -> Result<PathBuf, PjiError> {
        PjiMetadata::get_metadata_file_path().map_err(PjiError::Metadata)
    }

    pub fn default_root() -> Result<PathBuf, PjiError> {
        PjiConfig::get_default_root()
    }

    pub fn save(&self) -> Result<(), PjiError> {
        self.config.save().map_err(PjiError::Config)?;
        self.metadata.try_save().map_err(PjiError::Metadata)
    }

    pub fn roots(&self) -> &[PathBuf] {
        &self.config.roots
    }

    pub fn add_root(&mut self, root: impl Into<PathBuf>) {
        let root = root.into();
        if !self.config.roots.contains(&root) {
            self.config.roots.push(root);
        }
    }

    pub fn repositories(&self) -> Vec<Repository> {
        self.metadata
            .repos
            .iter()
            .cloned()
            .map(Repository::from)
            .collect()
    }

    pub fn repositories_by_last_opened(&self) -> Vec<Repository> {
        let mut repos = self.repositories();
        repos.sort_by(|a, b| b.last_opened_at.cmp(&a.last_opened_at));
        repos
    }

    pub fn parse_git_url(url: &str) -> Result<GitUrl, PjiError> {
        GitUrl::parse(url)
    }

    pub fn repository_path(root: impl AsRef<Path>, git: &GitUrl) -> PathBuf {
        root.as_ref()
            .join(&git.hostname)
            .join(&git.owner)
            .join(&git.name)
    }

    pub fn is_repository_registered(
        &self,
        url: &str,
        root: impl AsRef<Path>,
    ) -> Result<bool, PjiError> {
        let repo = PjiRepo::try_new(url, root.as_ref())?;
        Ok(self.metadata.has_repo(&repo))
    }

    pub fn unregister_repository(
        &mut self,
        url: &str,
        root: impl AsRef<Path>,
    ) -> Result<(), PjiError> {
        let repo = PjiRepo::try_new(url, root.as_ref())?;
        if !self.metadata.has_repo(&repo) {
            return Err(PjiError::RepositoryNotRegistered(repo.dir));
        }

        self.metadata.remove_repo(&repo);
        Ok(())
    }

    pub fn clone_repository(
        &mut self,
        url: &str,
        root: impl AsRef<Path>,
    ) -> Result<Repository, PjiError> {
        let repo = PjiRepo::try_new(url, root.as_ref())?;
        if self.metadata.has_repo(&repo) {
            return Err(PjiError::RepositoryAlreadyRegistered(repo.dir));
        }

        std::fs::create_dir_all(&repo.dir)?;
        let output = Command::new("git")
            .args(["clone", &repo.git_uri.uri])
            .arg(&repo.dir)
            .output()?;

        if !output.status.success() {
            return Err(PjiError::GitCommand {
                command: format!("git clone {} {}", repo.git_uri.uri, repo.dir.display()),
                stderr: command_error_output(&output),
            });
        }

        self.metadata.add_repo(&repo);
        Ok(repo.into())
    }

    pub fn find_repositories(&self, query: &str) -> Vec<Repository> {
        let query = query.to_lowercase();
        self.repositories()
            .into_iter()
            .filter(|repo| {
                query.is_empty()
                    || repo.git.owner.to_lowercase().contains(&query)
                    || repo.git.name.to_lowercase().contains(&query)
                    || repo.git.original.to_lowercase().contains(&query)
                    || repo.dir.to_string_lossy().to_lowercase().contains(&query)
            })
            .collect()
    }

    pub fn scan(&mut self) -> Result<ScanReport, PjiError> {
        self.metadata.deduplicate();

        let mut added = Vec::new();
        let mut invalid_paths = Vec::new();
        let mut issues = Vec::new();
        for root in self.config.roots.clone() {
            let scan = scan_root(&root)?;
            invalid_paths.extend(scan.invalid_paths);
            issues.extend(scan.issues);

            for repo in scan.added {
                let pji_repo = PjiRepo::from(repo.clone());
                if !self.metadata.has_repo(&pji_repo) {
                    self.metadata.repos.push(pji_repo);
                    added.push(repo);
                }
            }
        }

        Ok(ScanReport {
            added,
            invalid_paths,
            issues,
        })
    }

    pub fn resolve_repository(&self, cwd: impl AsRef<Path>) -> Option<Repository> {
        let cwd = cwd.as_ref();
        let resolved_dir = resolve_git_dir(cwd).unwrap_or_else(|| cwd.to_path_buf());

        self.metadata
            .repos
            .iter()
            .find(|repo| resolved_dir.starts_with(&repo.dir))
            .cloned()
            .map(Repository::from)
    }

    pub fn resolve_git_dir(cwd: impl AsRef<Path>) -> Option<PathBuf> {
        resolve_git_dir(cwd.as_ref())
    }

    pub fn mark_repository_opened(&mut self, dir: impl AsRef<Path>) -> bool {
        let Some(repo) = self
            .metadata
            .repos
            .iter_mut()
            .find(|repo| repo.dir == dir.as_ref())
        else {
            return false;
        };

        repo.update_open_time();
        true
    }

    pub fn list_worktrees(&self, repo_dir: impl AsRef<Path>) -> Result<WorktreeList, PjiError> {
        list_worktrees(&repo_dir.as_ref().to_path_buf())?.ok_or_else(|| {
            PjiError::InvalidWorktree(format!(
                "no worktrees found for {}",
                repo_dir.as_ref().display()
            ))
        })
    }

    pub fn default_worktree_path(repo_dir: impl AsRef<Path>, branch: &str) -> PathBuf {
        get_default_worktree_path(repo_dir.as_ref(), branch)
    }

    pub fn add_worktree(&self, request: AddWorktreeRequest) -> Result<PathBuf, PjiError> {
        add_worktree(
            &request.repo_dir,
            &request.branch,
            request.path,
            request.create_branch,
            request.base_branch.as_deref(),
        )
        .map_err(|stderr| PjiError::GitCommand {
            command: format!(
                "git -C {} worktree add {}",
                request.repo_dir.display(),
                request.branch
            ),
            stderr,
        })
    }

    pub fn remove_worktree(&self, request: RemoveWorktreeRequest) -> Result<(), PjiError> {
        remove_worktree(&request.repo_dir, &request.worktree_path, request.force).map_err(
            |stderr| PjiError::GitCommand {
                command: format!(
                    "git -C {} worktree remove {}",
                    request.repo_dir.display(),
                    request.worktree_path.display()
                ),
                stderr,
            },
        )
    }

    pub fn prune_worktrees(&self, repo_dir: impl AsRef<Path>) -> Result<String, PjiError> {
        prune_worktrees(&repo_dir.as_ref().to_path_buf()).map_err(|stderr| PjiError::GitCommand {
            command: format!("git -C {} worktree prune -v", repo_dir.as_ref().display()),
            stderr,
        })
    }

    pub fn local_branches(&self, repo_dir: impl AsRef<Path>) -> Vec<String> {
        list_local_branches(&repo_dir.as_ref().to_path_buf())
    }

    pub fn remote_branches(&self, repo_dir: impl AsRef<Path>) -> Vec<String> {
        list_remote_branches(&repo_dir.as_ref().to_path_buf())
    }
}

fn scan_root(root: &Path) -> Result<ScanReport, PjiError> {
    if !root.is_dir() {
        return Ok(ScanReport {
            added: Vec::new(),
            invalid_paths: Vec::new(),
            issues: Vec::new(),
        });
    }

    let mut added = Vec::new();
    let mut invalid_paths = Vec::new();
    let mut issues = Vec::new();

    for hostname_dir in list_dir(&root.to_path_buf())? {
        for user_dir in list_dir(&hostname_dir)? {
            for repo_dir in list_dir(&user_dir)? {
                if is_linked_worktree(&repo_dir) || is_worktree_dir(&repo_dir) {
                    continue;
                }

                let repo_url = match try_get_repo_from_dir(&repo_dir) {
                    Ok(Some(repo_url)) => repo_url,
                    Ok(None) => {
                        invalid_paths.push(repo_dir.clone());
                        issues.push(ScanIssue {
                            path: repo_dir,
                            message: "remote.origin.url not found".to_string(),
                        });
                        continue;
                    }
                    Err(PjiError::Io(err)) => return Err(PjiError::Io(err)),
                    Err(err) => {
                        invalid_paths.push(repo_dir.clone());
                        issues.push(ScanIssue {
                            path: repo_dir,
                            message: err.to_string(),
                        });
                        continue;
                    }
                };

                let repo = match PjiRepo::try_new(&repo_url, root) {
                    Ok(repo) => repo,
                    Err(err) => {
                        invalid_paths.push(repo_dir.clone());
                        issues.push(ScanIssue {
                            path: repo_dir,
                            message: err.to_string(),
                        });
                        continue;
                    }
                };
                if repo.dir == repo_dir {
                    added.push(repo.into());
                } else {
                    invalid_paths.push(repo_dir.clone());
                    issues.push(ScanIssue {
                        path: repo_dir,
                        message: "repository remote does not match pji directory layout"
                            .to_string(),
                    });
                }
            }
        }
    }

    Ok(ScanReport {
        added,
        invalid_paths,
        issues,
    })
}

fn command_error_output(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        format!("exit status {}", output.status)
    } else {
        stderr
    }
}

fn is_worktree_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.ends_with(".worktree") || name.ends_with(".worktrees"))
        .unwrap_or(false)
}

fn resolve_git_dir(cwd: &Path) -> Option<PathBuf> {
    let mut check_dir = cwd.to_path_buf();
    loop {
        if is_linked_worktree(&check_dir) {
            return get_main_repo_from_worktree(&check_dir);
        }

        if check_dir.join(".git").is_dir() {
            return Some(check_dir);
        }

        if !check_dir.pop() {
            return None;
        }
    }
}

impl From<GitURI> for GitUrl {
    fn from(git_uri: GitURI) -> Self {
        Self {
            hostname: git_uri.hostname,
            owner: git_uri.user,
            name: git_uri.repo,
            protocol: Protocol::from(git_uri.protocol),
            original: git_uri.uri,
        }
    }
}

impl From<GitUrl> for GitURI {
    fn from(git: GitUrl) -> Self {
        Self {
            hostname: git.hostname,
            user: git.owner,
            repo: git.name,
            protocol: GitProtocol::from(git.protocol),
            uri: git.original,
        }
    }
}

impl From<GitProtocol> for Protocol {
    fn from(protocol: GitProtocol) -> Self {
        match protocol {
            GitProtocol::Ssh => Self::Ssh,
            GitProtocol::Https => Self::Https,
        }
    }
}

impl From<Protocol> for GitProtocol {
    fn from(protocol: Protocol) -> Self {
        match protocol {
            Protocol::Ssh => Self::Ssh,
            Protocol::Https => Self::Https,
        }
    }
}

impl From<PjiRepo> for Repository {
    fn from(repo: PjiRepo) -> Self {
        Self {
            git: repo.git_uri.into(),
            dir: repo.dir,
            root: repo.root,
            created_at: repo.create_time,
            last_opened_at: repo.last_open_time,
        }
    }
}

impl From<Repository> for PjiRepo {
    fn from(repo: Repository) -> Self {
        Self {
            git_uri: repo.git.into(),
            dir: repo.dir,
            root: repo.root,
            create_time: repo.created_at,
            last_open_time: repo.last_opened_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_git_url_for_public_api() {
        let git = Pji::parse_git_url("git@github.com:zhanba/pji.git").unwrap();

        assert_eq!(git.hostname, "github.com");
        assert_eq!(git.owner, "zhanba");
        assert_eq!(git.name, "pji");
        assert_eq!(git.protocol, Protocol::Ssh);
    }

    #[test]
    fn computes_repository_path() {
        let git = GitUrl::parse("https://github.com/zhanba/pji.git").unwrap();
        let path = Pji::repository_path("/tmp/pji", &git);

        assert_eq!(path, PathBuf::from("/tmp/pji/github.com/zhanba/pji"));
    }

    #[test]
    fn identifies_worktree_dirs_by_name() {
        assert!(is_worktree_dir(Path::new(
            "/tmp/pji/github.com/zhanba/pji.worktree"
        )));
        assert!(is_worktree_dir(Path::new(
            "/tmp/pji/github.com/zhanba/pji.worktrees"
        )));
        assert!(!is_worktree_dir(Path::new(
            "/tmp/pji/github.com/zhanba/pji"
        )));
        assert!(!is_worktree_dir(Path::new(
            "/tmp/pji/github.com/zhanba/worktree-tools"
        )));
    }
}
