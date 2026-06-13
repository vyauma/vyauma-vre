//! `vre dap` — Start the Debug Adapter Protocol server.

pub fn run() {
    crate::dap::run_dap_server();
}
