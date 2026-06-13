//! `vre debug` — Launch the interactive DAP debugger.

use crate::cli::DebugArgs;

pub fn run(args: DebugArgs) {
    println!();
    println!("  Launching VRE Debugger (DAP)...");
    if let Some(file) = &args.file {
        println!("  Target: {}", file);
    }
    println!("  DAP port: {}", args.port);
    println!();
    // Delegate to existing DAP server
    crate::dap::run_dap_server();
}
