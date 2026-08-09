use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoKind {
    Git,
    Jj,
}

#[derive(Debug, Clone)]
pub struct Repo {
    pub name: String,
    pub path: PathBuf,
    pub kind: RepoKind,
}

/// List immediate subdirectories of `root` that are git or jj repos.
/// A `.jj` marker takes priority over `.git` since jj repos can colocate a git backend.
pub fn discover_repos(root: &Path) -> anyhow::Result<Vec<Repo>> {
    let mut entries: Vec<_> = std::fs::read_dir(root)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut repos = Vec::new();
    for entry in entries {
        let path = entry.path();
        let kind = if path.join(".jj").is_dir() {
            RepoKind::Jj
        } else if path.join(".git").exists() {
            RepoKind::Git
        } else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        repos.push(Repo { name, path, kind });
    }
    Ok(repos)
}
