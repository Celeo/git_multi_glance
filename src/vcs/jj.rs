use std::path::Path;
use std::process::Command;

use anyhow::{Context, anyhow};
use tokio::process::Command as AsyncCommand;

use super::{LocalStatus, RemoteState};

fn run_log(path: &Path, revset: &str, template: &str) -> anyhow::Result<String> {
    let output = Command::new("jj")
        .args(["log", "-r", revset, "--no-graph", "-T", template])
        .current_dir(path)
        .output()
        .context("failed to run jj log")?;
    if !output.status.success() {
        return Err(anyhow!(
            "jj log failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// The bookmark associated with `@`, or (if `@` is a bare empty commit with no bookmark
/// of its own, as jj leaves after `jj new`/checking out a bookmark) the bookmark on `@-`
/// instead — that's the commit the user actually thinks of themselves as "on".
fn current_bookmark(path: &Path) -> anyhow::Result<String> {
    let head = run_log(path, "@", "bookmarks ++ \"\\n\" ++ empty")?;
    let mut lines = head.lines();
    let head_bookmarks = lines.next().unwrap_or("").trim();
    let head_empty = lines.next().unwrap_or("").trim() == "true";

    if !head_bookmarks.is_empty() {
        return Ok(head_bookmarks.to_string());
    }
    if head_empty {
        let parent_bookmarks = run_log(path, "@-", "bookmarks")?;
        let parent_bookmarks = parent_bookmarks.trim();
        if !parent_bookmarks.is_empty() {
            return Ok(parent_bookmarks.to_string());
        }
    }
    Ok("(no bookmark)".to_string())
}

pub fn local_status(path: &Path) -> anyhow::Result<LocalStatus> {
    let branch = current_bookmark(path)?;

    let status_output = Command::new("jj")
        .args(["status"])
        .current_dir(path)
        .output()
        .context("failed to run jj status")?;
    if !status_output.status.success() {
        return Err(anyhow!(
            "jj status failed: {}",
            String::from_utf8_lossy(&status_output.stderr)
        ));
    }
    let stdout = String::from_utf8_lossy(&status_output.stdout);
    let dirty = stdout.contains("Working copy changes:");

    Ok(LocalStatus { branch, dirty })
}

fn bookmark_name_no_marks(branch: &str) -> Option<&str> {
    let name = branch.trim_end_matches(['*']);
    if name.is_empty() || name == "(no bookmark)" {
        None
    } else {
        Some(name)
    }
}

pub async fn remote_status(path: &Path) -> anyhow::Result<RemoteState> {
    let local = local_status(path)?;
    let Some(bookmark) = bookmark_name_no_marks(&local.branch) else {
        return Ok(RemoteState::NoUpstream);
    };

    let list_before = AsyncCommand::new("jj")
        .args(["bookmark", "list", bookmark, "--all-remotes"])
        .current_dir(path)
        .output()
        .await
        .context("failed to run jj bookmark list")?;
    let before = String::from_utf8_lossy(&list_before.stdout);
    if !before.contains("@origin") || before.contains("(not created yet)") {
        return Ok(RemoteState::NoUpstream);
    }

    let fetch = AsyncCommand::new("jj")
        .args(["git", "fetch"])
        .current_dir(path)
        .output()
        .await
        .context("failed to run jj git fetch")?;
    if !fetch.status.success() {
        return Ok(RemoteState::Error(
            String::from_utf8_lossy(&fetch.stderr).trim().to_string(),
        ));
    }

    let count = |revset: String| -> anyhow::Result<u32> {
        let output = Command::new("jj")
            .args(["log", "--no-graph", "-T", "commit_id ++ \"\\n\"", "-r", &revset])
            .current_dir(path)
            .output()
            .context("failed to run jj log")?;
        if !output.status.success() {
            return Err(anyhow!(
                "jj log failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.lines().filter(|l| !l.trim().is_empty()).count() as u32)
    };

    let ahead = count(format!("{bookmark}@origin..{bookmark}"))?;
    let behind = count(format!("{bookmark}..{bookmark}@origin"))?;

    Ok(match (ahead, behind) {
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
    fn strips_trailing_asterisk() {
        assert_eq!(bookmark_name_no_marks("main*"), Some("main"));
    }

    #[test]
    fn passes_through_plain_bookmark() {
        assert_eq!(bookmark_name_no_marks("main"), Some("main"));
    }

    #[test]
    fn no_bookmark_placeholder_is_none() {
        assert_eq!(bookmark_name_no_marks("(no bookmark)"), None);
    }

    #[test]
    fn empty_string_is_none() {
        assert_eq!(bookmark_name_no_marks(""), None);
    }
}
