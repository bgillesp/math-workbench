//! `adr`: architecture decision records, kept in `active/` and `superseded/` directories.
//!
//! Each ADR's front matter is written once, when the ADR is created, and never refers to later
//! ADRs. Its tags live in `tags.yaml`, which can change as the architecture grows. Authors write
//! these files by hand, following `docs/templates/adr.md`, and this module only reads them. The
//! designs are in `docs/design/0010-adr-directories.md` and
//! `docs/design/0019-simplify-adr-commands.md`.
//!
//! Loading has two stages. `records` reads the ADRs and `tags.yaml` into `Records` through the
//! structural checks, returning every record or every structural problem, using `yaml` for the
//! flat subset of YAML they share. `check` then runs the consistency checks over those records,
//! for `check-docs`. `list` implements the subcommands, with their arguments and output, and
//! needs only the first stage.

mod check;
#[cfg(test)]
mod fixtures;
mod list;
mod records;
mod yaml;

use std::ffi::OsString;
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
    /// `-h` or `--help`, which prints the usage text.
    Help,
}

/// A subcommand, as the usage text describes it.
struct Subcommand {
    /// The name that selects it.
    name: &'static str,
    /// Its arguments, as the usage text shows them.
    args: &'static str,
    /// What it prints.
    summary: &'static str,
    /// Parses the arguments after its name.
    parse: fn(lexopt::Parser) -> Result<Command, lexopt::Error>,
}

/// The subcommands, which both `parse_args` and the usage text read.
const SUBCOMMANDS: [Subcommand; 2] = [
    Subcommand {
        name: "list",
        args: "[--all] [--verbose] [--tags[=<tags>]]",
        summary: "the ADRs in force by number, or grouped by tag",
        parse: |parser| list::parse_args(parser).map(Command::List),
    },
    Subcommand {
        name: "tags",
        args: "",
        summary: "every tag, one per line",
        parse: |parser| list::parse_tags_args(parser).map(|()| Command::Tags),
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
        Err(messages) => {
            for message in &messages {
                eprintln!("error: {message}");
            }
            ExitCode::FAILURE
        },
    }
}

/// The ADR checks that `check-docs` runs, as messages.
pub(crate) fn check_docs(root: &Path) -> io::Result<Vec<String>> {
    let problems = match records::load(root)? {
        Ok(records) => check::check(&records),
        Err(problems) => problems,
    };
    Ok(messages(&problems))
}

/// The message for each of `problems`.
fn messages(problems: &[check::Problem]) -> Vec<String> {
    problems.iter().map(ToString::to_string).collect()
}

/// Runs `command` against the repository at `root`, and returns what it prints, or a message for
/// each thing that went wrong.
fn execute(root: &Path, command: Command) -> Result<String, Vec<String>> {
    Ok(match command {
        Command::List(options) => {
            let records = records::load(root)
                .map_err(|err| vec![err.to_string()])?
                .map_err(|problems| messages(&problems))?;
            list::render(&records, &options).map_err(|message| vec![message])?
        },
        Command::Tags => {
            let tags = records::load_tags(root)
                .map_err(|err| vec![err.to_string()])?
                .map_err(|problems| messages(&problems))?;
            list::render_tags(&tags)
        },
        Command::Help => format!("{}\n", usage()),
    })
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
        "\nOptions take their values attached, as in --tags=graph,wires. To write an ADR, follow\n\
         docs/templates/adr.md.\n\noptions:\n  -h, --help    print this message",
    );
    usage
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

/// ADR ids for `numbers`, separated by commas.
fn ids(numbers: &[u32]) -> String {
    let ids: Vec<String> = numbers.iter().map(|number| format!("ADR-{number:04}")).collect();
    ids.join(", ")
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
    use super::Dir::{Active, Superseded};
    use super::check::ProblemKind::{
        BadDate, BadTag, NoFrontMatter, SupersedesMissing, TagsForMissingAdr,
    };
    use super::check::{Problem, ProblemKind};
    use super::fixtures::{TempRoot, file, valid, with_front_matter};
    use super::*;

    fn parse(args: &[&str]) -> Result<Command, String> {
        parse_args(lexopt::Parser::from_args(args)).map_err(|err| err.to_string())
    }

    /// A scratch repository with these ADRs, and `tags` as `tags.yaml`.
    fn repository(name: &str, files: &[AdrFile], tags: &str) -> TempRoot {
        let root = TempRoot::new(name);
        for adr in files {
            root.write_adr(adr);
        }
        root.write(TAGS_PATH, tags);
        root
    }

    /// The message for a problem at `path`.
    fn message(path: &str, kind: ProblemKind) -> String {
        Problem { path: path.to_string(), kind }.to_string()
    }

    /// Messages in a fixed order, so tests don't depend on the order checks run in.
    fn sorted(mut messages: Vec<String>) -> Vec<String> {
        messages.sort();
        messages
    }

    /// ADR-0001, whose `date` is a consistency problem.
    fn bad_date() -> AdrFile {
        with_front_matter(
            Active,
            "0001-decision-1.md",
            "id: ADR-0001\ntitle: \"T\"\nsummary: \"S.\"\ndate: soon\n",
        )
    }

    /// ADR-0002, which has no front matter, a structural problem.
    fn no_block() -> AdrFile {
        file(Active, "0002-no-block.md", "# ADR-0002\n")
    }

    #[test]
    fn check_docs_passes_without_adrs() {
        let root = TempRoot::new("no-adrs");
        assert_eq!(check_docs(&root.0).unwrap(), Vec::<String>::new());
        for dir in [Active, Superseded] {
            std::fs::create_dir_all(root.0.join(ADR_DIR).join(dir.name())).unwrap();
        }
        root.write(TAGS_PATH, "");
        assert_eq!(check_docs(&root.0).unwrap(), Vec::<String>::new());
    }

    #[test]
    fn check_docs_passes_a_consistent_chain() {
        let files = [
            valid(Superseded, 1, &[]),
            valid(Superseded, 2, &[1]),
            valid(Active, 3, &[2]),
        ];
        let root = repository("chain", &files, "ADR-0003: graph\n");
        assert_eq!(check_docs(&root.0).unwrap(), Vec::<String>::new());
    }

    #[test]
    fn check_docs_reports_consistency_problems_once_the_structure_is_sound() {
        let files = [bad_date(), valid(Active, 3, &[9])];
        let root = repository("consistency", &files, "ADR-0008: graph\n");
        assert_eq!(
            sorted(check_docs(&root.0).unwrap()),
            sorted(vec![
                message(&bad_date().path(), BadDate { value: "soon".into() }),
                message(&valid(Active, 3, &[9]).path(), SupersedesMissing { number: 9 }),
                message(TAGS_PATH, TagsForMissingAdr { number: 8 }),
            ])
        );
    }

    #[test]
    fn check_docs_reports_no_consistency_problems_alongside_structural_ones() {
        let files = [bad_date(), no_block(), valid(Active, 3, &[9])];
        let root = repository("structural", &files, "ADR-0008: Wires\n");
        assert_eq!(
            sorted(check_docs(&root.0).unwrap()),
            sorted(vec![
                message(&no_block().path(), NoFrontMatter),
                message(TAGS_PATH, BadTag { number: 8, tag: "Wires".into() }),
            ])
        );
        // A structural problem in `tags.yaml` alone is enough.
        let files = [bad_date(), valid(Active, 3, &[9])];
        let root = repository("structural-tags", &files, "ADR-0003: Wires\n");
        assert_eq!(
            check_docs(&root.0).unwrap(),
            [message(
                TAGS_PATH,
                BadTag { number: 3, tag: "Wires".into() }
            )]
        );
    }

    #[test]
    fn adr_list_fails_on_structural_problems() {
        let list = || Command::List(list::Options::default());
        let root = repository("list-adr", &[valid(Active, 1, &[]), no_block()], "");
        assert_eq!(execute(&root.0, list()), Err(vec![message(&no_block().path(), NoFrontMatter)]));
        let root = repository("list-tags", &[valid(Active, 1, &[])], "ADR-0001: Wires\n");
        assert_eq!(
            execute(&root.0, list()),
            Err(vec![message(
                TAGS_PATH,
                BadTag { number: 1, tag: "Wires".into() }
            )])
        );
    }

    #[test]
    fn adr_list_shows_adrs_with_only_consistency_problems() {
        let files = [
            // An empty summary, which `--verbose` leaves out rather than printing a blank line.
            with_front_matter(
                Active,
                "0001-decision-1.md",
                "id: ADR-0001\ntitle: \"Decision 1\"\nsummary: \"\"\ndate: 2026-11-02\n",
            ),
            // A file name with the wrong number.
            with_front_matter(
                Active,
                "0009-wrong-number.md",
                "id: ADR-0002\ntitle: \"Decision 2\"\nsummary: \"Summary 2.\"\ndate: 2026-11-02\n",
            ),
            // An ADR in `superseded/` that nothing supersedes.
            valid(Superseded, 3, &[]),
        ];
        let root = repository("list-consistency", &files, "");
        assert_eq!(check_docs(&root.0).unwrap().len(), 3);
        let options = list::Options { all: true, verbose: true, ..list::Options::default() };
        assert_eq!(
            execute(&root.0, Command::List(options)),
            Ok("- ADR-0001: Decision 1\n  \
                docs/architecture/decisions/active/0001-decision-1.md\n\
                - ADR-0002: Decision 2\n  \
                docs/architecture/decisions/active/0009-wrong-number.md\n  \
                Summary 2.\n\
                - ADR-0003: Decision 3, superseded\n  \
                docs/architecture/decisions/superseded/0003-decision-3.md\n  \
                Summary 3.\n"
                .to_string())
        );
    }

    #[test]
    fn adr_tags_reads_only_tags_yaml() {
        // ADR-0002 can't be read and ADR-0009 doesn't exist, and their tags still print.
        let files = [valid(Active, 1, &[]), no_block()];
        let tags = "ADR-0001: graph\nADR-0002: wires\nADR-0009: stale\n";
        let root = repository("tags-only", &files, tags);
        assert_eq!(execute(&root.0, Command::Tags), Ok("graph\nstale\nwires\n".to_string()));
    }

    #[test]
    fn adr_tags_fails_on_structural_problems_in_tags_yaml() {
        let root = repository("tags-bad", &[valid(Active, 1, &[])], "ADR-0001: Wires\n");
        assert_eq!(
            execute(&root.0, Command::Tags),
            Err(vec![message(
                TAGS_PATH,
                BadTag { number: 1, tag: "Wires".into() }
            )])
        );
    }

    #[test]
    fn subcommands_parse_their_own_arguments() {
        assert_eq!(
            parse(&["list", "--all"]),
            Ok(Command::List(list::Options { all: true, ..list::Options::default() }))
        );
        assert_eq!(parse(&["tags"]), Ok(Command::Tags));
    }

    #[test]
    fn unknown_or_missing_subcommands_fail() {
        assert!(parse(&[]).is_err());
        assert!(parse(&["index"]).is_err());
        assert!(parse(&["active"]).is_err());
    }

    #[test]
    fn new_and_print_are_unknown_subcommands() {
        assert_eq!(
            parse(&["new", "Title", "--tags=graph"]),
            Err("unknown subcommand `new`".to_string())
        );
        assert_eq!(parse(&["print", "3"]), Err("unknown subcommand `print`".to_string()));
    }

    #[test]
    fn usage_describes_the_remaining_subcommands() {
        let usage = usage();
        assert!(usage.contains("list [--all] [--verbose] [--tags[=<tags>]]"), "{usage}");
        assert!(usage.contains("\n  tags\n"), "{usage}");
        for removed in ["\n  new ", "\n  print ", "ADR numbers can be given"] {
            assert!(!usage.contains(removed), "{removed:?} in {usage}");
        }
    }

    #[test]
    fn the_just_recipe_passes_its_arguments_through() {
        let output = std::process::Command::new("just")
            .args(["--dry-run", "adr", "list", "--tags=graph,wires"])
            .current_dir(crate::workspace_root())
            .output()
            .expect("`just` runs this repository's recipes, so it should be installed");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr).trim(),
            "cargo xtask adr list --tags=graph,wires"
        );
    }
}
