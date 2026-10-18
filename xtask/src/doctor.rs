//! `doctor`: warns about local setup problems that keep the checks or agent tooling from working.
//!
//! Warnings only, so `just setup` always finishes.

use std::path::Path;
use std::process::{Command, Stdio};

/// Tools the checks run, as a program and the arguments that print its version.
const TOOLS: &[(&str, &[&str])] = &[
    ("just", &["--version"]),
    ("cargo", &["nextest", "--version"]),
    ("cargo", &["deny", "--version"]),
    ("cargo", &["insta", "--version"]),
];

pub(crate) fn run(root: &Path) {
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
    if !root.join(".claude/skills").is_dir() {
        warnings.push(
            "`.claude/skills` isn't a link to `.agents/skills`, so Claude Code won't find the \
             project skills. On Windows, enable Developer Mode and run \
             `git config core.symlinks true`, then delete the `.claude/skills` file and run \
             `git checkout -- .claude/skills`."
                .to_string(),
        );
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
