//! `vre profile` — Profile a Vyauma program's execution.

use crate::cli::ProfileArgs;

pub fn run(args: ProfileArgs) {
    crate::profiler::run_profiler(&args.file);
}
