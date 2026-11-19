//! `doctor`: warns about missing tools that the checks run.
//!
//! Warnings only, so `just setup` always finishes.

use std::process::{Command, Stdio};

/// Tools the checks run, as a program and the arguments that print its version.
const TOOLS: &[(&str, &[&str])] = &[
    ("just", &["--version"]),
    ("cargo", &["nextest", "--version"]),
    ("cargo", &["deny", "--version"]),
    ("cargo", &["insta", "--version"]),
];

pub(crate) fn run() {
    let mut warnings = Vec::new();
    for (program, args) in TOOLS {
        if !runs(program, args) {
            let name = if *program == "cargo" {
                format!("cargo-{}", args[0])
            } else {
                (*program).to_string()
            };
            warnings.push(format!("`{name}` isn't installed; run `just setup`"));
        }
    }
    if warnings.is_empty() {
        println!("doctor: ok");
    }
    for warning in &warnings {
        eprintln!("warning: {warning}");
    }
}

fn runs(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}
