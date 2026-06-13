//! `vre flash` — Flash firmware to an embedded device.

use crate::cli::FlashArgs;

pub fn run(args: FlashArgs) {
    let target = args.target.to_string();
    crate::embedded::flash(&args.file, &target);
}
