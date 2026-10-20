//! Repository checks for Math Workbench.
//!
//! Run through the `cargo xtask` alias, usually from a `just` recipe. Each check prints the
//! problems it finds and exits with a failure status if there are any.

mod docs;
mod doctor;
mod levels;
mod wip;

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// The problems a check found, or an error that kept it from running.
type CheckResult = Result<Vec<String>, Box<dyn Error>>;

const USAGE: &str = "\
usage: cargo xtask <command>

commands:
  check-levels  each crate depends only on crates at strictly lower levels
  check-docs    ADRs, design docs, and relative links are consistent
  check-wip     docs/wip/ holds nothing but its README
  doctor        warn about missing tools or a broken skills symlink";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [command] = args.as_slice() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let root = workspace_root();
    let result = match command.as_str() {
        "check-levels" => levels::run(&root),
        "check-docs" => docs::run(&root),
        "check-wip" => wip::run(&root),
        "doctor" => {
            doctor::run(&root);
            return ExitCode::SUCCESS;
        },
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        },
    };
    match result {
        Ok(problems) if problems.is_empty() => {
            println!("{command}: ok");
            ExitCode::SUCCESS
        },
        Ok(problems) => {
            for problem in &problems {
                eprintln!("error: {problem}");
            }
            eprintln!("{command}: {} problem(s)", problems.len());
            ExitCode::FAILURE
        },
        Err(err) => {
            eprintln!("{command}: {err}");
            ExitCode::FAILURE
        },
    }
}

/// The workspace root, which is the parent of this crate's directory.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is a subdirectory of the workspace")
        .to_path_buf()
}

/// A repository-relative path with forward slashes, for messages.
fn display(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
