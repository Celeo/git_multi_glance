# git_multi_glance

A CLI that scans the immediate subdirectories of a path (default: cwd) and prints the
git/jj branch, dirty status, and ahead/behind-vs-remote for each — a "glance" across a
directory of repos without `cd`-ing into each one.

## Design constraints (read before changing architecture)

- **No `git2`/`jj-lib`.** Both backends work by shelling out to the `git` and `jj`
  binaries (`std::process::Command` for cheap local calls, `tokio::process::Command` for
  the network-bound remote check). This was a deliberate choice over `git2` bindings: one
  interaction pattern for both VCSs, no libgit2/CLI config divergence, and no pinning to
  jj's internal (non-public-API) `jj-lib` crate.
- **No alt-screen / full-screen clear.** Output is a static list printed top-to-bottom in
  discovery order; rows are rewritten in place using plain ANSI cursor-movement escapes
  (`render.rs`) as remote checks resolve. This means the final output stays in normal
  terminal scrollback after the program exits. Do not introduce `ratatui`/`crossterm`
  alt-screen usage — that would defeat this on purpose.
- Local status (branch, dirty) is synchronous and cheap; remote status is async and
  network-bound (does a real `git fetch` / `jj git fetch`, then diffs against the
  resulting remote-tracking ref). Local rows print immediately; remote results stream in
  and update their row whenever they finish, out of order relative to each other but
  never corrupting other rows (row position is fixed at print time).

## Module layout

- `src/cli.rs` — clap `Args`: `--path`, `--no-remote`.
- `src/repo.rs` — `discover_repos`: lists immediate children of a directory, classifies
  each as `RepoKind::Jj` (has `.jj/`) or `RepoKind::Git` (has `.git`, checked after `.jj`
  since jj repos can colocate a git backend), skips anything else.
- `src/vcs/` — `local_status`/`remote_status` dispatch to `git.rs` or `jj.rs` based on
  `RepoKind`. `git.rs` parses `git status --porcelain=v2 --branch`. `jj.rs` parses
  `jj log`/`jj status`/`jj bookmark list` output and counts ahead/behind via revset
  ranges (`bookmark@origin..bookmark`).
- `src/render.rs` — `Renderer`: prints each row once (with `checking...`/`skipped`
  placeholder), then rewrites a specific row in place (`update_row`) using cursor-up +
  clear-line + cursor-down escapes. Row count is fixed before any async work starts.
- `src/main.rs` — orchestration: discover repos, print local rows sequentially, spawn one
  `tokio` task per repo for the remote check (unless `--no-remote`), apply updates as
  they arrive over an `mpsc` channel.

## Running

```sh
cargo run -- --path /some/dir   # scan a directory other than cwd
cargo run -- --no-remote        # skip network checks, local-only, fast
```

## Verifying changes

- `cargo build`, `cargo clippy --all-targets` should be clean.
- Manual test: build a scratch directory with a clean git repo, a dirty git repo ahead of
  its upstream, a git repo with no upstream, a jj repo, and a plain non-repo directory;
  run the binary against it and confirm each row is correct and updates land on the right
  row. Run under `script -qec '<cmd>' /dev/null | cat -v` to inspect the raw ANSI escapes
  if debugging row-update math (row updates are index-based: `up = total_rows - index`
  lines to move up, then `up - 1` lines back down — must special-case `up == 1` since
  `\x1b[0B` is ambiguous across terminals).
