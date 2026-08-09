pub mod git;
pub mod jj;

use crate::repo::{Repo, RepoKind};

#[derive(Debug, Clone)]
pub struct LocalStatus {
    pub branch: String,
    pub dirty: bool,
}

#[derive(Debug, Clone)]
pub enum RemoteState {
    NoUpstream,
    UpToDate,
    Ahead(u32),
    Behind(u32),
    Diverged(u32, u32),
    Error(String),
}

pub fn local_status(repo: &Repo) -> anyhow::Result<LocalStatus> {
    match repo.kind {
        RepoKind::Git => git::local_status(&repo.path),
        RepoKind::Jj => jj::local_status(&repo.path),
    }
}

pub async fn remote_status(repo: &Repo) -> RemoteState {
    let result = match repo.kind {
        RepoKind::Git => git::remote_status(&repo.path).await,
        RepoKind::Jj => jj::remote_status(&repo.path).await,
    };
    match result {
        Ok(state) => state,
        Err(e) => RemoteState::Error(e.to_string()),
    }
}
