![git-like-vcs banner](assets/banner.png)

# git-like-vcs

A small git-like VCS developed for learnining porpuses

## Features

- init: initialize a repository.
- hash-object: compute SHA-1 of a file (optionally store it).
- cat-file -p: pretty-print an object by its hash.
- write-tree: write a tree object from the working directory state.
- ls-tree [--name-only]: list a tree’s contents.
- commit-tree -m: create a commit for a tree (author/committer from env).
- clone <url> <dir>: clone a remote repository (via libgit2).

## Build

```bash
cargo build
```

## Usage

Run subcommands with cargo:

```bash
cargo run -- <command> [options]
```

Examples:

- Initialize a repo:
```bash
cargo run -- init
```

- Hash a file (compute only):
```bash
cargo run -- hash-object ./path/to/file
```

- Hash and store object:
```bash
cargo run -- hash-object -w ./path/to/file
```

- Pretty-print an object:
```bash
cargo run -- cat-file -p <object_hash>
```

- Write a tree:
```bash
cargo run -- write-tree
```

- List a tree:
```bash
cargo run -- ls-tree <tree_hash>
# names only
cargo run -- ls-tree --name-only <tree_hash>
```

- Create a commit:
```bash
# optional: set author/committer
export GIT_AUTHOR_NAME="Your Name"
export GIT_AUTHOR_EMAIL="you@example.com"
export GIT_COMMITTER_NAME="$GIT_AUTHOR_NAME"
export GIT_COMMITTER_EMAIL="$GIT_AUTHOR_EMAIL"

cargo run -- commit-tree -m "message" <tree_hash>
# with parent
cargo run -- commit-tree -m "message" -p <parent_commit_hash> <tree_hash>
```

- Clone a repository:
```bash
cargo run -- clone https://github.com/user/repo.git ./my-repo
```

## Environment variables (commit)

- GIT_AUTHOR_NAME, GIT_AUTHOR_EMAIL
- GIT_COMMITTER_NAME, GIT_COMMITTER_EMAIL

If not set, defaults are used. Date/time and timezone are taken from the system.