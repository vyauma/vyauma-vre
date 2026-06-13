//! `vre doc` — Generate documentation for the current project.

use crate::cli::DocArgs;

pub fn run(args: DocArgs) {
    let dir = args.dir.unwrap_or_else(|| ".".to_string());
    crate::doc::generate_docs(&dir);
}
