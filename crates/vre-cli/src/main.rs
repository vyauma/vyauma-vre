//! Vyauma Runtime Engine — CLI
//!
//! Entry point. Delegates all logic to [`cli::run`].

mod cli;
mod commands;
mod config;
mod diagnostics;
mod registry;
mod templates;

// Legacy internal modules (preserved for VM/compiler integration)
mod native;
mod mobile;
mod cloud;
mod embedded;
mod profiler;
mod doc;
mod lsp;
mod dap;
mod test_runner;
mod module_loader;
mod manifest;
mod web;
mod init;

fn main() {
    cli::run();
}
