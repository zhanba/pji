# pji - git repo manager

## Install

```
cargo install pji
```

## Usage

pji commands

```
pji provide a tree structure to manage your git projects.

Usage: pji [COMMAND]

Commands:
  init    select root directory for your repos and create a pji config file
  add     add a git project
  remove  remove a git project
  list    list all git projects
  find    fuzz search git projects
  update  scan all git repo in root dir and save repo info
  pull    check root dir and download all missing repos
  open    open a git project home page in browser
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

pji open commands

```
open a git project home page in browser

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
