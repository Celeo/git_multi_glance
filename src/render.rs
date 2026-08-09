use std::io::Write;

use owo_colors::OwoColorize;

use crate::repo::{Repo, RepoKind};
use crate::vcs::{LocalStatus, RemoteState};

pub struct Renderer {
    total_rows: usize,
    name_width: usize,
    branch_width: usize,
}

impl Renderer {
    /// `name_width`/`branch_width` should be the max length of any repo name / branch
    /// name that will be printed, so columns align without knowing rows in advance.
    pub fn new(name_width: usize, branch_width: usize) -> Self {
        Self {
            total_rows: 0,
            name_width,
            branch_width,
        }
    }

    /// Print the next row. Pass `checking = false` when no remote check will ever run.
    pub fn print_row(&mut self, repo: &Repo, local: &LocalStatus, checking: bool) {
        let remote = if checking { RemoteLabel::Checking } else { RemoteLabel::Skipped };
        println!("{}", self.format_line(repo, local, remote));
        self.total_rows += 1;
    }

    /// Rewrite row `index` in place with the resolved remote state.
    pub fn update_row(&self, index: usize, repo: &Repo, local: &LocalStatus, remote: &RemoteState) {
        let up = self.total_rows - index;
        let mut out = std::io::stdout();
        // Move cursor up to the target row, clear it, print, then return to the bottom.
        let _ = write!(
            out,
            "\x1b[{up}A\r\x1b[2K{}\n",
            self.format_line(repo, local, RemoteLabel::Resolved(remote))
        );
        if up > 1 {
            let _ = write!(out, "\x1b[{}B", up - 1);
        }
        let _ = out.flush();
    }

    fn format_line(&self, repo: &Repo, local: &LocalStatus, remote: RemoteLabel) -> String {
        let kind = match repo.kind {
            RepoKind::Git => "git",
            RepoKind::Jj => "jj ",
        };
        let dirty_padded = format!("{:<7}", if local.dirty { "dirty" } else { "clean" });
        let dirty = if local.dirty {
            dirty_padded.red().to_string()
        } else {
            dirty_padded.green().to_string()
        };
        let remote_str = match remote {
            RemoteLabel::Checking => "checking...".dimmed().to_string(),
            RemoteLabel::Skipped => "skipped".dimmed().to_string(),
            RemoteLabel::Resolved(RemoteState::NoUpstream) => "no upstream".dimmed().to_string(),
            RemoteLabel::Resolved(RemoteState::UpToDate) => "up to date".green().to_string(),
            RemoteLabel::Resolved(RemoteState::Ahead(n)) => format!("↑{n}").yellow().to_string(),
            RemoteLabel::Resolved(RemoteState::Behind(n)) => format!("↓{n}").yellow().to_string(),
            RemoteLabel::Resolved(RemoteState::Diverged(a, b)) => {
                format!("↑{a} ↓{b}").red().to_string()
            }
            RemoteLabel::Resolved(RemoteState::Error(e)) => format!("error: {e}").red().to_string(),
        };

        let name_padded = format!("{:<width$}", repo.name, width = self.name_width);
        let branch_padded = format!("{:<width$}", local.branch, width = self.branch_width);

        format!(
            "{} {} {} {} {}",
            name_padded.cyan(),
            kind.dimmed(),
            branch_padded,
            dirty,
            remote_str,
        )
    }
}

enum RemoteLabel<'a> {
    Checking,
    Skipped,
    Resolved(&'a RemoteState),
}
