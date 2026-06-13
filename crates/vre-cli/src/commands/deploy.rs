//! `vre deploy` — Deploy the project to a cloud or container target.

use crate::cli::{DeployArgs, DeployTarget};

pub fn run(args: DeployArgs) {
    let file = args.file.unwrap_or_else(|| "src/main.vya".to_string());
    let target = args.target.to_string();
    crate::cloud::deploy(&file, &target);
}
