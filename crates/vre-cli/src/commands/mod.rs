//! Command dispatch router.
//!
//! All command handler modules live here. The `dispatch()` function
//! matches the parsed [`crate::cli::Command`] variant and invokes
//! the appropriate handler.

pub mod new;
pub mod init;
pub mod run;
pub mod check;
pub mod build;
pub mod build_web;
pub mod test;
pub mod package;
pub mod publish;
pub mod install;
pub mod uninstall;
pub mod upgrade;
pub mod search;
pub mod info;
pub mod mobile;
pub mod deploy;
pub mod flash;
pub mod debug;
pub mod lsp;
pub mod dap;
pub mod doc;
pub mod profile;
pub mod doctor;
pub mod version;

use crate::cli::Command;

/// Match a parsed CLI command to its handler function.
pub fn dispatch(command: Command) {
    match command {
        Command::New(args)       => new::run(args),
        Command::Init(args)      => init::run(args),
        Command::Run(args)       => run::run(args),
        Command::Check(args)     => check::run(args),
        Command::Build(args)     => build::run(args),
        Command::BuildWeb(args)  => build_web::run(args),
        Command::Test(args)      => test::run(args),
        Command::Package(args)   => package::run(args),
        Command::Publish(args)   => publish::run(args),
        Command::Install(args)   => install::run(args),
        Command::Uninstall(args) => uninstall::run(args),
        Command::Upgrade         => upgrade::run(),
        Command::Search(args)    => search::run(args),
        Command::Info(args)      => info::run(args),
        Command::Mobile(args)    => mobile::run(args),
        Command::Deploy(args)    => deploy::run(args),
        Command::Flash(args)     => flash::run(args),
        Command::Debug(args)     => debug::run(args),
        Command::Lsp             => lsp::run(),
        Command::Dap             => dap::run(),
        Command::Doc(args)       => doc::run(args),
        Command::Profile(args)   => profile::run(args),
        Command::Doctor          => doctor::run(),
        Command::Version         => version::run(),
    }
}
