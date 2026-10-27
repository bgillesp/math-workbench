//! Repository checks for Math Workbench.
//!
//! Run through the `cargo xtask` alias, usually from a `just` recipe. Each check prints the
//! problems it finds and exits with a failure status if there are any. `adr` instead prints views
//! of the ADRs, or creates one, and takes a subcommand of its own.

mod adr;
mod docs;
mod doctor;
mod ready;

use std::error::Error;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// The problems a check found, or an error that kept it from running.
type CheckResult = Result<Vec<String>, Box<dyn Error>>;

/// A top-level command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Command {
    /// `adr`, which parses its own subcommand and arguments.
    Adr,
    /// `check-docs`.
    CheckDocs,
    /// `check-ready`.
    CheckReady,
    /// `doctor`.
    Doctor,
}

impl Command {
    const ALL: [Self; 4] = [Self::Adr, Self::CheckDocs, Self::CheckReady, Self::Doctor];

    fn name(self) -> &'static str {
        match self {
            Self::Adr => "adr",
            Self::CheckDocs => "check-docs",
            Self::CheckReady => "check-ready",
            Self::Doctor => "doctor",
        }
    }

    fn summary(self) -> &'static str {
        match self {
            Self::Adr => "list, print, and create ADRs (`cargo xtask adr --help` for more)",
            Self::CheckDocs => "ADRs, design docs, and relative links are consistent",
            Self::CheckReady => "no branch notes, and every design doc is implemented",
            Self::Doctor => "warn about missing tools or a broken skills symlink",
        }
    }
}

fn main() -> ExitCode {
    let mut parser = lexopt::Parser::from_env();
    let command = match parse_args(&mut parser) {
        Ok(Some(command)) => command,
        Ok(None) => {
            println!("{}", usage());
            return ExitCode::SUCCESS;
        },
        Err(err) => {
            eprintln!("error: {err}\n\n{}", usage());
            return ExitCode::from(2);
        },
    };
    let root = workspace_root();
    let result = match command {
        Command::Adr => return adr::run(&root, parser),
        Command::CheckDocs => docs::run(&root),
        Command::CheckReady => ready::run(&root),
        Command::Doctor => {
            doctor::run(&root);
            return ExitCode::SUCCESS;
        },
    };
    let name = command.name();
    match result {
        Ok(problems) if problems.is_empty() => {
            println!("{name}: ok");
            ExitCode::SUCCESS
        },
        Ok(problems) => {
            for problem in &problems {
                eprintln!("error: {problem}");
            }
            eprintln!("{name}: {} problem(s)", problems.len());
            ExitCode::FAILURE
        },
        Err(err) => {
            eprintln!("{name}: {err}");
            ExitCode::FAILURE
        },
    }
}

/// The requested command, or `None` if help was requested. The arguments after `adr` are left in
/// `parser` for it to parse.
fn parse_args(parser: &mut lexopt::Parser) -> Result<Option<Command>, lexopt::Error> {
    use lexopt::prelude::*;

    let command = match parser.next()? {
        Some(Short('h') | Long("help")) => return Ok(None),
        Some(Value(name)) => {
            let name = name.string()?;
            Command::ALL
                .into_iter()
                .find(|command| command.name() == name)
                .ok_or_else(|| format!("unknown command `{name}`"))?
        },
        Some(arg) => return Err(arg.unexpected()),
        None => return Err("missing command".into()),
    };
    if command == Command::Adr {
        return Ok(Some(command));
    }
    match parser.next()? {
        Some(Short('h') | Long("help")) => Ok(None),
        Some(arg) => Err(arg.unexpected()),
        None => Ok(Some(command)),
    }
}

fn usage() -> String {
    let mut usage = String::from("usage: cargo xtask <command>\n\ncommands:\n");
    for command in Command::ALL {
        let _ = writeln!(usage, "  {:<14}{}", command.name(), command.summary());
    }
    usage.push_str("\noptions:\n  -h, --help    print this message");
    usage
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Option<Command>, String> {
        parse_args(&mut lexopt::Parser::from_args(args)).map_err(|err| err.to_string())
    }

    #[test]
    fn commands_parse_by_name() {
        for command in Command::ALL {
            assert_eq!(parse(&[command.name()]), Ok(Some(command)));
        }
    }

    #[test]
    fn help_flags_request_usage() {
        assert_eq!(parse(&["--help"]), Ok(None));
        assert_eq!(parse(&["-h"]), Ok(None));
        assert_eq!(parse(&["check-docs", "--help"]), Ok(None));
    }

    #[test]
    fn bad_arguments_fail() {
        assert_eq!(parse(&[]), Err("missing command".to_string()));
        assert_eq!(parse(&["check-all"]), Err("unknown command `check-all`".to_string()));
        assert!(parse(&["check-docs", "extra"]).is_err());
        assert!(parse(&["--verbose"]).is_err());
    }

    #[test]
    fn usage_lists_every_command() {
        let usage = usage();
        for command in Command::ALL {
            assert!(usage.contains(command.name()));
        }
    }
}
