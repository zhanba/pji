# pji - A CLI for managing, finding, and opening Git repositories.

## Install

```
cargo install pji
```

## Usage

pji commands

```
A CLI for managing, finding, and opening Git repositories.

Usage: pji [COMMAND]

Commands:
  config  Configure the root directory for your repositories
  add     Add a git repository
  remove  Remove a git repository
  list    List all git repositories
  find    Fuzzy search for git repositories
  scan    Scan all git repositories in the root directory and save their information
  clean   Clean pji metadata and configuration
  open    Open a git repository page (e.g., home, PR, issue) in the browser
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

pji open commands

```
Open a git repository page (e.g., home, PR, issue) in the browser

Usage: pji open [URL]
       pji open home [URL]
       pji open pr [NUMBER]
       pji open issue [NUMBER]
       pji open help [COMMAND]...
```

pji tree structure

```
$ROOT
|- github.com
|  `- zhanba
|     `- pji
`- gitlab.com
   `- zhanba
      `- pji

```

## Ref

- inspired by [projj](https://github.com/popomore/projj)
