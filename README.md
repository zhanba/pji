<!-- filepath: /Users/ryannz/pji/github.com/zhanba/pji/README.md -->

# pji - A CLI for managing, finding, and opening Git repositories.

[![Crates.io](https://img.shields.io/crates/v/pji.svg)](https://crates.io/crates/pji)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

<!-- Add a build status badge if you have CI/CD setup e.g. [![Build Status](https://travis-ci.org/zhanba/pji.svg?branch=master)](https://travis-ci.org/zhanba/pji) -->

`pji` is a command-line tool designed to simplify managing, finding, and opening your Git repositories. It helps you keep your projects organized and quickly access them.

## Features

- **Centralized Repository Management**: Configure root directories to store your projects.
- **Quick Add & Remove**: Easily clone new repositories or remove existing ones.
- **List Repositories**: View all managed repositories, with an option for detailed output.
- **Fuzzy Find**: Quickly search and find repositories, copying the `cd` command to your clipboard for instant navigation.
- **Scan**: Automatically discover and add existing Git repositories within your root directories.
- **Clean**: Remove `pji`'s metadata and configuration files.
- **Browser Integration**: Open repository pages (homepage, pull requests, issues) directly in your web browser.

## Install

```sh
cargo install pji
```

or use brew

```sh
brew install zhanba/tap/pji
```

## Usage

### General Commands

```
pji [COMMAND]
```

**Available Commands:**

- `config`: Configure the root directory (or directories) where your repositories are stored. `pji` will prompt for the path.
- `add <GIT_URL>`: Clones the specified Git repository into your configured root directory and registers it with `pji`.
  - Example: `pji add https://github.com/zhanba/pji.git`
- `remove <GIT_URL>`: Removes a registered Git repository from `pji`'s tracking and deletes its directory.
  - Example: `pji remove https://github.com/zhanba/pji.git`
- `list [-l, --long]`: Lists all Git repositories managed by `pji`.
  - Use `-l` or `--long` for a detailed table view including last opened time.
- `find [QUERY]`: Fuzzy search for a Git repository by its name or path.
  - If a query is provided, it filters the search.
  - Upon selection, the command to `cd` into the repository's directory is copied to your clipboard.
  - Example: `pji find myproject`
- `scan`: Scans all configured root directories for Git repositories and adds any new ones to `pji`'s managed list.
- `clean`: Removes all `pji` metadata and configuration files from your system. Use with caution.
- `open ...`: Contains subcommands to open repository pages in the browser. See below.
- `help`: Print help message or the help of a given subcommand.

### `pji open` Commands

The `open` command helps you quickly navigate to your repository's web interface (e.g., GitHub, GitLab).

```
pji open [SUBCOMMAND] [OPTIONS]
```

**Subcommands & Options:**

- `pji open [REPO_NAME_OR_URL]` or `pji open home [REPO_NAME_OR_URL]`: Opens the homepage of the specified repository.
  - If `REPO_NAME_OR_URL` is omitted, `pji` attempts to open the repository corresponding to the current working directory.
  - Example: `pji open pji` (if 'pji' is a known repo)
  - Example: `pji open` (when inside a managed git repository)
- `pji open pr [NUMBER]`: Opens a specific pull request page.
  - If `NUMBER` is omitted, it may open the general pull requests page (behavior might depend on the hosting platform's URL structure for the repo).
  - This command typically operates on the repository of the current working directory if no specific repo is named.
  - Example: `pji open pr 123`
- `pji open issue [NUMBER]`: Opens a specific issue page.
  - If `NUMBER` is omitted, it may open the general issues page.
  - Operates on the repository of the current working directory if no specific repo is named.
  - Example: `pji open issue 456`

**Note on `open` context:** For `pr` and `issue` subcommands, if you don't specify a repository name/URL directly and are not inside a managed repository's directory, the command might not find a repository to act upon.

### Project Directory Structure Example

`pji` organizes cloned repositories under the configured root path, typically like this:

```
$ROOT/
|- github.com/
|  `- zhanba/
|     `- pji/
`- gitlab.com/
   `- another-user/
      `- another-project/
```

## How it Works

`pji` maintains a configuration file (usually `~/.config/pji/config.json` or platform equivalent) to store your root directories and metadata about your repositories (usually `~/.config/pji/metadata.json`).

## Inspired By

- [projj](https://github.com/popomore/projj)

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
