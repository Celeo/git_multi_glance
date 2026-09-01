//! git_multi_glance

#![deny(unsafe_code, clippy::all)]

mod render;
mod repo;
mod vcs;

use clap::Parser;
use render::Renderer;
use std::path::PathBuf;
use tokio::sync::mpsc;
use vcs::{LocalStatus, RemoteState};

/// Print git/jj status for every immediate subdirectory of a path.
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    /// Directory whose immediate children should be scanned (defaults to cwd)
    #[arg(long)]
    pub path: Option<PathBuf>,

    /// Skip network checks against remotes (local branch/dirty status only)
    #[arg(long)]
    pub no_remote: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let scan_path = match args.path {
        Some(p) => p,
        None => std::env::current_dir()?,
    };

    let repos = repo::discover_repos(&scan_path)?;
    if repos.is_empty() {
        println!("No git or jj repositories found in {}", scan_path.display());
        return Ok(());
    }

    let locals: Vec<LocalStatus> = repos
        .iter()
        .map(|repo| match vcs::local_status(repo) {
            Ok(l) => l,
            Err(e) => LocalStatus {
                branch: format!("error: {e}"),
                dirty: false,
            },
        })
        .collect();

    let name_width = repos.iter().map(|r| r.name.len()).max().unwrap_or(0);
    let branch_width = locals.iter().map(|l| l.branch.len()).max().unwrap_or(0);
    let mut renderer = Renderer::new(name_width, branch_width);
    for (repo, local) in repos.iter().zip(&locals) {
        renderer.print_row(repo, local, !args.no_remote);
    }

    if args.no_remote {
        return Ok(());
    }

    let (tx, mut rx) = mpsc::unbounded_channel::<(usize, RemoteState)>();
    for (index, repo) in repos.iter().cloned().enumerate() {
        let tx = tx.clone();
        tokio::spawn(async move {
            let state = vcs::remote_status(&repo).await;
            let _ = tx.send((index, state));
        });
    }
    drop(tx);

    while let Some((index, state)) = rx.recv().await {
        renderer.update_row(index, &repos[index], &locals[index], &state);
    }

    Ok(())
}
