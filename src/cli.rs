use std::path::PathBuf;

use clap::Parser;

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
