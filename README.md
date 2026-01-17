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
