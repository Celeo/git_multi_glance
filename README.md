# git_multi_glance

A small CLI that scans the immediate subdirectories of a path (default: current
directory) and prints, for each one that's a git or [jj](https://github.com/jj-vcs/jj)
repo, its branch, dirty status, and whether it's ahead/behind/diverged from its remote —
a quick "glance" across a directory of repos without `cd`-ing into each one.

Local status (branch, dirty) prints immediately. Remote status requires a network fetch,
so it's checked concurrently in the background and each row updates in place as its
result comes in — without clearing the screen, so the final output stays in your
terminal scrollback.

## Install / build

Requires the `git` and (optionally) `jj` binaries to be on your `PATH`.

```sh
cargo build --release
```

## Usage

```sh
git_multi_glance [--path <dir>] [--no-remote]
```

- `--path <dir>` — scan a directory other than the current one.
- `--no-remote` — skip network checks; local branch/dirty status only, fast.
- `-h, --help` / `-V, --version`

### Example

```sh
$ git_multi_glance
project-a            git my-feature       dirty   checking...
project-b            git main             clean   checking...
project-c            jj  my-bookmark      clean   checking...
project-d            git main             clean   checking...

$ # a few seconds later, in place:
project-a            git my-feature       dirty   ↑2
project-b            git main             clean   up to date
project-c            jj  my-bookmark      clean   no upstream
project-d            git main             clean   ↓1
```

## Design notes

See [CLAUDE.md](CLAUDE.md) for the architecture, module layout, and the constraints
(no `git2`/`jj-lib`, no alt-screen rendering) that shaped this project.
