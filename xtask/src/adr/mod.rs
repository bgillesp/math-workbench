//! `adr`: architecture decision records, kept in `active/` and `superseded/` directories.
//!
//! Each ADR's front matter is written once, when the ADR is created, and never refers to later
//! ADRs. Its tags live in `tags.yaml`, which can change as the architecture grows. The design is
//! in `docs/design/0010-adr-directories.md`.
//!
//! `yaml` reads the flat subset of YAML that the front matter and `tags.yaml` use, `records`
//! reads the ADRs and their tags, and `check` holds the ADR checks that `check-docs` runs. `list`,
//! `tags`, `print`, and `new` each implement one subcommand, with its arguments and output.

mod check;
#[cfg(test)]
mod fixtures;
mod list;
mod new;
mod print;
mod records;
mod tags;
mod yaml;

use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fmt::Write;
use std::io;
use std::path::Path;
use std::process::ExitCode;

use lexopt::ValueExt;

/// The directory that holds the ADRs and `tags.yaml`.
const ADR_DIR: &str = "docs/architecture/decisions";

/// The repository-relative path of `tags.yaml`.
const TAGS_PATH: &str = "docs/architecture/decisions/tags.yaml";

/// The directory an ADR is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Dir {
    /// `active/`, for ADRs in force.
    Active,
    /// `superseded/`, for ADRs a later ADR replaces.
    Superseded,
}

impl Dir {
    /// The directory's name under `ADR_DIR`.
    fn name(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Superseded => "superseded",
        }
    }
}

/// A file in `active/` or `superseded/`, as read from disk.
#[derive(Clone, Debug, PartialEq, Eq)]
struct AdrFile {
    /// The directory the file is in.
    dir: Dir,
    /// The file name, such as `0003-wire-types-checked-on-connect.md`.
    name: String,
    /// The file's contents.
    text: String,
}

impl AdrFile {
    /// The repository-relative path of the file.
    fn path(&self) -> String {
        adr_path(self.dir, &self.name)
    }
}

/// The repository-relative path of the ADR file named `name` in `dir`.
fn adr_path(dir: Dir, name: &str) -> String {
    format!("{ADR_DIR}/{}/{name}", dir.name())
}

/// An `adr` subcommand and its arguments.
#[derive(Debug, PartialEq, Eq)]
enum Command {
    /// `adr list`, with its options.
    List(list::Options),
    /// `adr tags`.
    Tags,
    /// `adr print`, with the numbers of the ADRs to print.
    Print(Vec<u32>),
    /// `adr new`, with its arguments.
    New(new::Args),
    /// `-h` or `--help`, which prints the usage text.
    Help,
}

/// A subcommand, as the usage text describes it.
struct Subcommand {
    /// The name that selects it.
    name: &'static str,
    /// Its arguments, as the usage text shows them.
    args: &'static str,
    /// What it prints or does.
    summary: &'static str,
    /// Parses the arguments after its name.
    parse: fn(lexopt::Parser) -> Result<Command, lexopt::Error>,
}

/// The subcommands, which both `parse_args` and the usage text read.
const SUBCOMMANDS: [Subcommand; 4] = [
    Subcommand {
        name: "list",
        args: "[--all] [--tags[=<tags>]] [--verbose]",
        summary: "the ADRs in force by number, or grouped by tag",
        parse: |parser| list::parse_args(parser).map(Command::List),
    },
    Subcommand {
        name: "tags",
        args: "",
        summary: "each tag, with the number of ADRs in force that use it",
        parse: |parser| tags::parse_args(parser).map(|()| Command::Tags),
    },
    Subcommand {
        name: "print",
        args: "<N>...",
        summary: "the named ADRs in full",
        parse: |parser| print::parse_args(parser).map(Command::Print),
    },
    Subcommand {
        name: "new",
        args: "<title> --tags=<tags> [--supersedes=<N>,...]",
        summary: "create an ADR, and move the ADRs it supersedes to `superseded/`",
        parse: |parser| new::parse_args(parser).map(Command::New),
    },
];

/// Runs `cargo xtask adr`, given the arguments after `adr`.
pub(crate) fn run(root: &Path, parser: lexopt::Parser) -> ExitCode {
    let command = match parse_args(parser) {
        Ok(command) => command,
        Err(err) => {
            eprintln!("error: {err}\n\n{}", usage());
            return ExitCode::from(2);
        },
    };
    match execute(root, command) {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        },
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        },
    }
}

/// The ADR checks that `check-docs` runs, as messages.
pub(crate) fn check_docs(root: &Path) -> io::Result<Vec<String>> {
    let (files, tags) = records::read_adr_files(root)?;
    Ok(check::check(&files, tags.as_deref()).iter().map(ToString::to_string).collect())
}

/// Runs `command` against the repository at `root`, and returns what it prints.
fn execute(root: &Path, command: Command) -> Result<String, Box<dyn Error>> {
    Ok(match command {
        Command::List(options) => list::render(&load(root)?, &options)?,
        Command::Tags => tags::render(&load(root)?),
        Command::Print(numbers) => print::render(&load(root)?, &numbers)?,
        Command::New(args) => new::run(root, &args)?,
        Command::Help => format!("{}\n", usage()),
    })
}

/// The ADRs for the views, after printing a warning for each file they leave out.
fn load(root: &Path) -> io::Result<Vec<records::Adr>> {
    let (files, tags) = records::read_adr_files(root)?;
    let loaded = records::load(files, tags.as_deref());
    for warning in &loaded.warnings {
        eprintln!("warning: {warning}");
    }
    Ok(loaded.adrs)
}

/// Parses the arguments after `adr`: a subcommand, then that subcommand's own arguments.
fn parse_args(mut parser: lexopt::Parser) -> Result<Command, lexopt::Error> {
    use lexopt::prelude::*;

    match parser.next()? {
        Some(Short('h') | Long("help")) => Ok(Command::Help),
        Some(Value(name)) => {
            let name = name.string()?;
            let subcommand = SUBCOMMANDS
                .iter()
                .find(|subcommand| subcommand.name == name)
                .ok_or_else(|| format!("unknown subcommand `{name}`"))?;
            (subcommand.parse)(parser)
        },
        Some(arg) => Err(arg.unexpected()),
        None => Err("missing subcommand".into()),
    }
}

/// The usage text for `adr`.
fn usage() -> String {
    let mut usage = String::from("usage: cargo xtask adr <subcommand> [<args>]\n\nsubcommands:\n");
    for subcommand in &SUBCOMMANDS {
        let line = format!("{} {}", subcommand.name, subcommand.args);
        let _ = writeln!(usage, "  {}\n      {}", line.trim_end(), subcommand.summary);
    }
    usage.push_str(
        "\nADR numbers can be given as 3, 0003, or ADR-0003. Options take their values attached,\n\
         as in --tags=graph,wires.\n\noptions:\n  -h, --help    print this message",
    );
    usage
}

/// An ADR number given as `3`, `0003`, or `ADR-0003`.
fn parse_number(text: &str) -> Option<u32> {
    let digits = text.strip_prefix("ADR-").unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// An ADR number given as an argument, or an error saying what forms one takes.
fn number_arg(text: &str) -> Result<u32, lexopt::Error> {
    parse_number(text).ok_or_else(|| {
        format!("`{text}` isn't an ADR number; use a form like 3, 0003, or ADR-0003").into()
    })
}

/// The comma-separated items attached to `--option=`, or an error if there are none, naming
/// them as `noun`.
fn list_arg(option: &str, value: OsString, noun: &str) -> Result<Vec<String>, lexopt::Error> {
    let items: Vec<String> = value
        .string()?
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(String::from)
        .collect();
    if items.is_empty() {
        return Err(format!("`--{option}=` needs at least one {noun}").into());
    }
    Ok(items)
}

/// The error for `value` given after `--option` with a space, where it has to be attached.
fn detached_value(option: &str, value: &OsStr) -> lexopt::Error {
    let value = value.to_string_lossy();
    format!("`{value}` looks like a value for `--{option}`; attach it as `--{option}={value}`")
        .into()
}

/// The number in an ADR id like `ADR-0003`.
fn parse_id(id: &str) -> Option<u32> {
    canonical_number(id.strip_prefix("ADR-")?)
}

/// The number in an ADR file name like `0003-short-title.md`, whose slug is lowercase letters,
/// digits, and hyphens.
fn file_number(name: &str) -> Option<u32> {
    let (digits, rest) = name.split_once('-')?;
    let slug = rest.strip_suffix(".md")?;
    let number = canonical_number(digits)?;
    (!slug.is_empty() && slug.bytes().all(is_slug_byte)).then_some(number)
}

/// The number written as `digits`, if it's written as ADRs write it: at least four digits, with
/// leading zeros only to make up four.
fn canonical_number(digits: &str) -> Option<u32> {
    let number: u32 = digits.parse().ok()?;
    (format!("{number:04}") == digits).then_some(number)
}

/// Whether `tag` is well formed: lowercase letters, digits, and hyphens.
fn is_tag(tag: &str) -> bool {
    !tag.is_empty() && tag.bytes().all(is_slug_byte)
}

/// Whether `byte` can appear in a tag or a file name's slug.
fn is_slug_byte(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
}

/// The tags in a `tags.yaml` value, which separates them with commas.
fn split_tags(value: &str) -> impl Iterator<Item = &str> {
    value.split(',').map(str::trim)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adr::fixtures::new_args;

    fn parse(args: &[&str]) -> Result<Command, String> {
        parse_args(lexopt::Parser::from_args(args)).map_err(|err| err.to_string())
    }

    #[test]
    fn subcommands_parse_their_own_arguments() {
        assert_eq!(
            parse(&["list", "--all"]),
            Ok(Command::List(list::Options { all: true, ..list::Options::default() }))
        );
        assert_eq!(parse(&["tags"]), Ok(Command::Tags));
        assert_eq!(parse(&["print", "3"]), Ok(Command::Print(vec![3])));
        assert_eq!(
            parse(&["new", "Title", "--tags=graph"]),
            Ok(Command::New(new_args("Title", &["graph"], &[])))
        );
    }

    #[test]
    fn unknown_or_missing_subcommands_fail() {
        assert!(parse(&[]).is_err());
        assert!(parse(&["index"]).is_err());
        assert!(parse(&["active"]).is_err());
    }

    #[test]
    fn numbers_parse_in_each_form() {
        assert_eq!(parse_number("3"), Some(3));
        assert_eq!(parse_number("0003"), Some(3));
        assert_eq!(parse_number("ADR-0003"), Some(3));
        assert_eq!(parse_number("ADR-0120"), Some(120));
        for bad in ["", "x", "ADR-", "3a", "-3", "ADR-x"] {
            assert_eq!(parse_number(bad), None, "{bad:?}");
        }
    }
}
