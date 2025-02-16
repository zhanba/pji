# pji - git repo manager

## Install

```
cargo install pjs
```

## Usage

Pj provide a tree structure to manage your git projects.

```
$ROOT
|- github.com
|  `- zhanba
|     `- pji
`- gitlab.com
   `- zhanba
      `- pji

```

```sh
# init pji, choose root dir
pji init

# add a git project
pji add xxx

# remove a git project
pji remove xxx

# list all git project
pji list

# fuzz search git project
pji find xxx

# scan all git repo in root dir and write repo info to config
pji update

# check root dir and download all missing repos
pji pull

# open a git project in browser
pji open

# open a git project merge request page in browser
pji open mr

```

## Ref

- inspired by [projj](https://github.com/popomore/projj)
