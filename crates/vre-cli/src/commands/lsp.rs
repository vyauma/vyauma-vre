//! `vre lsp` — Start the Language Server Protocol server.

pub fn run() {
    crate::lsp::run_lsp_server();
}
