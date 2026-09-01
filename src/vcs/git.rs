use super::{LocalStatus, RemoteState};
use anyhow::{Context, anyhow};
use std::{path::Path, process::Command};
use tokio::process::Command as AsyncCommand;

struct PorcelainBranch {
    head: String,
    has_upstream: bool,
    ahead: u32,
    behind: u32,
    dirty: bool,
}

fn parse_porcelain(output: &str) -> PorcelainBranch {
    let mut head = String::from("HEAD");
    let mut has_upstream = false;
    let mut ahead = 0;
    let mut behind = 0;
    let mut dirty = false;

    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("# branch.head ") {
            head = rest.to_string();
        } else if line.starts_with("# branch.upstream ") {
            has_upstream = true;
        } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
            for part in rest.split_whitespace() {
                if let Some(n) = part.strip_prefix('+') {
                    ahead = n.parse().unwrap_or(0);
                } else if let Some(n) = part.strip_prefix('-') {
                    behind = n.parse().unwrap_or(0);
                }
            }
        } else if !line.starts_with('#') {
            dirty = true;
        }
    }

    PorcelainBranch {
        head,
        has_upstream,
        ahead,
        behind,
        dirty,
    }
}

pub fn local_status(path: &Path) -> anyhow::Result<LocalStatus> {
    let output = Command::new("git")
        .args(["status", "--porcelain=v2", "--branch"])
        .current_dir(path)
        .output()
        .context("failed to run git status")?;
    if !output.status.success() {
        return Err(anyhow!(
            "git status failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed = parse_porcelain(&stdout);
    Ok(LocalStatus {
        branch: parsed.head,
        dirty: parsed.dirty,
    })
}

pub async fn remote_status(path: &Path) -> anyhow::Result<RemoteState> {
    let status_before = AsyncCommand::new("git")
        .args(["status", "--porcelain=v2", "--branch"])
        .current_dir(path)
        .output()
        .await
        .context("failed to run git status")?;
    let before = parse_porcelain(&String::from_utf8_lossy(&status_before.stdout));
    if !before.has_upstream {
        return Ok(RemoteState::NoUpstream);
    }

    let fetch = AsyncCommand::new("git")
        .args(["fetch", "--quiet"])
        .current_dir(path)
        .output()
        .await
        .context("failed to run git fetch")?;
    if !fetch.status.success() {
        return Ok(RemoteState::Error(
            String::from_utf8_lossy(&fetch.stderr).trim().to_string(),
        ));
    }

    let status_after = AsyncCommand::new("git")
        .args(["status", "--porcelain=v2", "--branch"])
        .current_dir(path)
        .output()
        .await
        .context("failed to run git status")?;
    let after = parse_porcelain(&String::from_utf8_lossy(&status_after.stdout));

    Ok(match (after.ahead, after.behind) {
        (0, 0) => RemoteState::UpToDate,
        (a, 0) => RemoteState::Ahead(a),
        (0, b) => RemoteState::Behind(b),
        (a, b) => RemoteState::Diverged(a, b),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_clean_branch_with_upstream() {
        let output = "\
# branch.oid abc123
# branch.head main
# branch.upstream origin/main
# branch.ab +0 -0
";
        let parsed = parse_porcelain(output);
        assert_eq!(parsed.head, "main");
        assert!(parsed.has_upstream);
        assert_eq!(parsed.ahead, 0);
        assert_eq!(parsed.behind, 0);
        assert!(!parsed.dirty);
    }

    #[test]
    fn parses_dirty_ahead_behind() {
        let output = "\
# branch.oid abc123
# branch.head feature
# branch.upstream origin/feature
# branch.ab +2 -3
1 .M N... 100644 100644 100644 abc123 def456 src/main.rs
";
        let parsed = parse_porcelain(output);
        assert_eq!(parsed.head, "feature");
        assert!(parsed.has_upstream);
        assert_eq!(parsed.ahead, 2);
        assert_eq!(parsed.behind, 3);
        assert!(parsed.dirty);
    }

    #[test]
    fn parses_no_upstream() {
        let output = "\
# branch.oid abc123
# branch.head local-only
";
        let parsed = parse_porcelain(output);
        assert_eq!(parsed.head, "local-only");
        assert!(!parsed.has_upstream);
        assert_eq!(parsed.ahead, 0);
        assert_eq!(parsed.behind, 0);
        assert!(!parsed.dirty);
    }

    #[test]
    fn parses_detached_head_default() {
        let parsed = parse_porcelain("");
        assert_eq!(parsed.head, "HEAD");
        assert!(!parsed.has_upstream);
        assert!(!parsed.dirty);
    }
}
