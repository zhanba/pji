# pji

[![Crates.io](https://img.shields.io/crates/v/pji.svg)](https://crates.io/crates/pji)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Git repos, organized

## Install

```sh
cargo install pji
```

or

```sh
brew install zhanba/tap/pji
```

## Usage

### Global Options

| Option | Description |
|--------|-------------|
| `-n, --non-interactive` | Force non-interactive mode. This is also enabled automatically when stdin, stdout, or stderr is not attached to a terminal |
| `--root <DIR>` | Select a root directory without prompting |

### Commands

| Command | Description |
|---------|-------------|
| `pji [QUERY]` | Fuzzy find and cd into a repository (default) |
| `pji add <URL>` | Clone and register a repository |
| `pji remove <URL> [-y]` | Remove a repository |
| `pji list [-l]` | List repositories (`-l` for detailed view) |
| `pji scan` | Discover and add existing repositories |
| `pji config [ROOT]` | Configure root directories |
| `pji clean` | Remove pji metadata and config |

### Open in Browser

| Command | Description |
|---------|-------------|
| `pji open [REPO]` | Open repository homepage |
| `pji open pr [NUMBER]` | Open pull request page |
| `pji open issue [NUMBER]` | Open issue page |

### Worktree Management (`pji wt`)

| Command | Description |
|---------|-------------|
| `pji wt` | Switch between worktrees (default) |
| `pji wt add [BRANCH]` | Create a worktree |
| `pji wt list` | List all worktrees |
| `pji wt remove [WORKTREE] [-y]` | Remove a worktree |
| `pji wt prune` | Clean up stale worktree info |

### Non-Interactive Mode

pji automatically runs in non-interactive mode when stdin, stdout, or stderr is
not attached to a terminal, such as in scripts, CI, command substitution, pipes,
or redirected output. You can also force this behavior with `-n` or
`--non-interactive`.

In non-interactive mode, pji never opens prompts or fuzzy selectors. Commands
that normally switch into a repository or worktree print the target path instead
of opening a shell, so they can be used from scripts:

```sh
cd "$(pji find pji)"
cd "$(pji wt sw feature/login)"
```

If an operation would require a choice, pji fails and asks for more specific
input. For example, use a narrower query when multiple repositories or
worktrees match. If multiple roots are configured, pass `--root <DIR>` to choose
one explicitly.

Destructive commands still require explicit confirmation. In non-interactive
mode, pass `--yes` for commands such as `pji remove` or `pji wt remove`.

```sh
pji -n find pji
pji -n --root ~/pji add git@github.com:zhanba/pji.git
pji -n wt add feature/login --path ../pji.worktrees/feature-login
pji -n wt remove feature/login --yes
```

## Library API

`pji` also exposes a small, stable API for other Rust apps. Use `Pji` as the
entry point; internal CLI modules are not part of the public contract. The
CLI's interactive and non-interactive behavior is handled by the binary, so
library callers should make their own selection and confirmation decisions.

The API is split into a few groups:

- State: `Pji::load`, `Pji::save`, `Pji::roots`, `Pji::add_root`,
  `Pji::repositories`, and `Pji::repositories_by_last_opened` read and write
  pji's config and metadata.
- Repository helpers: `GitUrl::parse`, `Pji::parse_git_url`, and
  `Pji::repository_path` parse URLs and compute pji's on-disk layout without
  running git.
- Repository operations: `clone_repository`, `unregister_repository`,
  `is_repository_registered`, `find_repositories`, `scan`, `resolve_repository`,
  and `mark_repository_opened` manage repository metadata and discovery.
- Worktree operations: `list_worktrees`, `default_worktree_path`,
  `add_worktree`, `remove_worktree`, `prune_worktrees`, `local_branches`, and
  `remote_branches` wrap git worktree and branch commands.

Methods that mutate config or metadata do not automatically save every change.
Call `pji.save()?` after changes you want to persist. Methods that run git
commands return `PjiError::GitCommand` if git exits unsuccessfully.

```rust
use pji::{AddWorktreeRequest, GitUrl, Pji, PjiError};

fn main() -> Result<(), PjiError> {
    let mut pji = Pji::load()?;

    let root = Pji::default_root()?;
    pji.add_root(&root);
    pji.save()?;

    let git = GitUrl::parse("git@github.com:zhanba/pji.git")?;
    let repo_path = Pji::repository_path(&root, &git);
    println!("{}", repo_path.display());

    if !pji.is_repository_registered(&git.original, &root)? {
        pji.clone_repository(&git.original, &root)?;
        pji.save()?;
    }

    let matches = pji.find_repositories("pji");
    for repo in &matches {
        println!("{}", repo.dir.display());
    }

    if let Some(repo) = matches.first() {
        let worktree_path = Pji::default_worktree_path(&repo.dir, "feature/login");
        pji.add_worktree(AddWorktreeRequest {
            repo_dir: repo.dir.clone(),
            branch: "feature/login".to_string(),
            path: Some(worktree_path),
            create_branch: true,
            base_branch: Some("main".to_string()),
        })?;
    }

    pji.scan()?;
    pji.save()?;

    Ok(())
}
```

### Directory Structure

```
$ROOT/
├── github.com/
│   └── user/
│       └── repo/
└── gitlab.com/
    └── user/
        └── repo/
```

## Inspired By

- [projj](https://github.com/popomore/projj)

## License

MIT
