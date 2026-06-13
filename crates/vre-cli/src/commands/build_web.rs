//! `vre build-web` — Build the project as a WebAssembly module.

use crate::cli::BuildWebArgs;

pub fn run(args: BuildWebArgs) {
    crate::web::build_web(&args.file);
}
