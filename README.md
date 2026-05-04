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

### Commands

| Command | Description |
|---------|-------------|
| `pji [QUERY]` | Fuzzy find and cd into a repository (default) |
| `pji add <URL>` | Clone and register a repository |
| `pji remove <URL>` | Remove a repository |
| `pji list [-l]` | List repositories (`-l` for detailed view) |
| `pji scan` | Discover and add existing repositories |
| `pji config` | Configure root directories |
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
| `pji wt add` | Create worktree interactively |
| `pji wt list` | List all worktrees |
| `pji wt remove` | Remove a worktree |
| `pji wt prune` | Clean up stale worktree info |

## Library API

`pji` also exposes a small, stable API for other Rust apps. Use `Pji` as the
entry point; internal CLI modules are not part of the public contract.

```rust
use pji::{GitUrl, Pji, PjiError};

fn main() -> Result<(), PjiError> {
    let mut pji = Pji::load()?;
    let git = GitUrl::parse("git@github.com:zhanba/pji.git")?;

    for repo in pji.find_repositories("pji") {
        println!("{}", repo.dir.display());
    }

    let path = Pji::repository_path("/Users/me/pji", &git);
    println!("{}", path.display());

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
