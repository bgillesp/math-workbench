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

/// The directory that holds the ADRs and `tags.yaml`.
pub(crate) const ADR_DIR: &str = "docs/architecture/decisions";

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
}

/// Parses the arguments after `adr`: a subcommand, then that subcommand's own arguments.
fn parse_args(_parser: lexopt::Parser) -> Result<Command, lexopt::Error> {
    todo!()
}

/// An ADR number given as `3`, `0003`, or `ADR-0003`.
fn parse_number(_text: &str) -> Option<u32> {
    todo!()
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
