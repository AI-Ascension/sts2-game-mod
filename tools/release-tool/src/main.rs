// SPDX-License-Identifier: MIT

mod common;
mod manifest;
mod package;
mod receipt;

use std::process::ExitCode;

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let command = args.next().ok_or_else(usage)?;
    let rest: Vec<String> = args.collect();
    match command.as_str() {
        "build-runtime-receipt" => receipt::run(&rest),
        "build-artifact-manifest" => manifest::run(&rest),
        "package-platform-item" => package::run(&rest),
        "--help" | "-h" => {
            println!(
                "sts2-release-tool <command> [args]\n\nCommands:\n  \
build-runtime-receipt       Build one platform receipt\n  \
build-artifact-manifest     Bind paired production receipts\n  \
package-platform-item       Stage one platform Workshop package"
            );
            Ok(())
        }
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "a release command is required: build-runtime-receipt, build-artifact-manifest, or package-platform-item".into()
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("sts2-release-tool: {error}");
            ExitCode::FAILURE
        }
    }
}
