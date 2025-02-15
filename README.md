# pj - git repo manager

## Install

```
cargo install pj
```

## Usage

Pj provide a tree structure to manage your git projects.

```
$ROOT
|- github.com
|  `- zhanba
|     `- pj
`- gitlab.com
   `- zhanba
      `- pj

```


```sh
# init pj, choose root dir, create `~/.pj/config.yaml`
pj init

# add a git project
pj add xxx

# remove a git project
pj remove xxx

# list all git project
pj list

# fuzz search git project
pj find xxx

# scan all git repo in root dir and write repo info into `~/.pj/repo.yaml`
pj update

# check root dir and download all missing repos
pj pull

# open a git project in browser
pj open

# open a git project merge request page in browser
pj open mr

```

## Ref

- inspired by [projj](https://github.com/popomore/projj)