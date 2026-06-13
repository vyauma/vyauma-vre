//! `vre mobile` — Mobile platform build commands.

use crate::cli::{MobileArgs, MobileCommand, MobilePlatform};

pub fn run(args: MobileArgs) {
    match args.subcommand {
        MobileCommand::Build(build_args) => {
            let file = build_args.file.unwrap_or_else(|| "src/main.vya".to_string());
            let target = build_args.platform.to_string();
            crate::mobile::pack(&file, &target);
        }
        MobileCommand::Sign(sign_args) => {
            println!();
            println!("  Signing {} for {}...", sign_args.package, sign_args.platform);
            println!();
            println!("  (Code signing will be implemented in a future release)");
            println!("  Package: {}", sign_args.package);
            println!("  Platform: {}", sign_args.platform);
            println!();
        }
    }
}
