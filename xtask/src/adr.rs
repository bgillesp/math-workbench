//! `adr`: architecture decision records, kept in `active/` and `superseded/` directories.
//!
//! Each ADR's front matter is written once, when the ADR is created, and never refers to later
//! ADRs. Its tags live in `tags.yaml`, which can change as the architecture grows. This module
//! checks both for `check-docs`, prints views of them, and creates new ADRs. The design is in
//! `docs/design/0010-adr-directories.md`.

use std::fmt;
use std::io;
use std::path::Path;

/// The directory that holds the ADRs and `tags.yaml`.
pub(crate) const ADR_DIR: &str = "docs/architecture/decisions";

/// The template `adr new` copies the body of an ADR from.
const TEMPLATE: &str = "docs/templates/adr.md";

/// The column summaries are wrapped at in `adr list --verbose`.
const SUMMARY_WIDTH: usize = 80;

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

/// An ADR's front matter, once it has been read.
#[derive(Debug, PartialEq, Eq)]
struct FrontMatter {
    /// The number in `id`.
    number: u32,
    /// The title, unquoted.
    title: String,
    /// The summary, unquoted, with continuation lines folded into single spaces.
    summary: String,
    /// The date the ADR was created, as `YYYY-MM-DD`.
    date: String,
    /// The numbers of the ADRs this one replaces.
    supersedes: Vec<u32>,
}

/// An ADR with everything the views need.
#[derive(Debug)]
struct Adr {
    /// The ADR's number.
    number: u32,
    /// The repository-relative path of the ADR's file.
    path: String,
    /// The title from the front matter.
    title: String,
    /// The summary from the front matter.
    summary: String,
    /// The ADR's tags, in the order `tags.yaml` lists them.
    tags: Vec<String>,
    /// Whether the ADR is in `superseded/`.
    superseded: bool,
    /// The number of the ADR that supersedes this one, if any.
    successor: Option<u32>,
    /// The file's full contents, for `adr print`.
    text: String,
}

/// The ADRs the views can show, and warnings about files they had to leave out.
#[derive(Debug)]
struct Loaded {
    /// The ADRs whose front matter could be read, by number.
    adrs: Vec<Adr>,
    /// A message for each file that was left out.
    warnings: Vec<String>,
}

/// A problem in one file.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Problem {
    /// The repository-relative path of the file to fix.
    path: String,
    /// What's wrong with it.
    kind: ProblemKind,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.kind)
    }
}

/// What's wrong with an ADR or with `tags.yaml`.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ProblemKind {
    // An ADR on its own
    /// The file name isn't like `0003-short-title.md`.
    BadName,
    /// The file doesn't start with a front matter block.
    NoFrontMatter,
    /// A front matter line isn't `key: value` or the continuation of a quoted value.
    MalformedLine { line: String },
    /// The key isn't one of the front matter fields.
    UnknownKey { key: String },
    /// The key appears more than once.
    DuplicateKey { key: String },
    /// A required field is missing.
    MissingField { key: String },
    /// A required field is present but empty.
    EmptyField { key: String },
    /// `title` or `summary` isn't double-quoted.
    NotQuoted { key: String },
    /// A quoted value contains `"` or `\`.
    QuoteOrBackslash { key: String },
    /// A quoted value has no closing quote.
    UnterminatedQuote { key: String },
    /// `summary` is longer than 300 characters.
    SummaryTooLong { chars: usize },
    /// `id` isn't like `ADR-0003`.
    BadId { value: String },
    /// `id` names a different number than the file name.
    IdMismatch { id: u32 },
    /// Another ADR, at path `other`, has the same number.
    DuplicateId { number: u32, other: String },
    /// `date` isn't a `YYYY-MM-DD` date.
    BadDate { value: String },
    /// A `supersedes` entry isn't like `ADR-0001`.
    BadSupersedesEntry { entry: String },

    // Supersede links between ADRs
    /// `supersedes` names an ADR that doesn't exist.
    SupersedesMissing { number: u32 },
    /// `supersedes` names an ADR whose number isn't lower than this one's.
    SupersedesNotLower { number: u32 },
    /// More than one ADR lists this one in `supersedes`.
    SupersededByMany { by: Vec<u32> },
    /// The ADR is in `superseded/`, but no ADR supersedes it.
    SupersededButNotListed,
    /// The ADR is in `active/`, but ADR `by` supersedes it.
    ActiveButSuperseded { by: u32 },

    // tags.yaml
    /// A line isn't `ADR-NNNN: tag, tag`.
    TagsMalformedLine { line: String },
    /// The ADR has no entry.
    TagsMissing { number: u32 },
    /// The entry names an ADR that doesn't exist.
    TagsForMissingAdr { number: u32 },
    /// The ADR has more than one entry.
    DuplicateTagsEntry { number: u32 },
    /// The entry lists no tags.
    NoTags { number: u32 },
    /// A tag isn't lowercase letters, digits, and hyphens.
    BadTag { number: u32, tag: String },
}

impl fmt::Display for ProblemKind {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

/// How `adr list` arranges its entries.
#[derive(Debug, Default, PartialEq, Eq)]
enum Grouping {
    /// One list, by number.
    #[default]
    Flat,
    /// Grouped under every tag, alphabetically.
    AllTags,
    /// Grouped under the given tags, in the order given.
    Tags(Vec<String>),
}

/// The options of `adr list`.
#[derive(Debug, Default, PartialEq, Eq)]
struct ListOptions {
    /// Include superseded ADRs.
    all: bool,
    /// Whether and how to group the entries by tag.
    grouping: Grouping,
    /// Add each ADR's summary below its entry.
    verbose: bool,
}

/// The arguments of `adr new`.
#[derive(Debug, PartialEq, Eq)]
struct NewArgs {
    /// The new ADR's title.
    title: String,
    /// The new ADR's tags, in the order given.
    tags: Vec<String>,
    /// The numbers of the ADRs the new one supersedes.
    supersedes: Vec<u32>,
}

/// An `adr` subcommand and its arguments.
#[derive(Debug, PartialEq, Eq)]
enum AdrCommand {
    /// `adr list`, with its options.
    List(ListOptions),
    /// `adr tags`.
    Tags,
    /// `adr print`, with the numbers of the ADRs to print.
    Print(Vec<u32>),
    /// `adr new`, with its arguments.
    New(NewArgs),
}

/// Why a view couldn't be printed.
#[derive(Debug, PartialEq, Eq)]
enum ViewError {
    /// No ADR has the tag; `existing` lists the tags in use.
    UnknownTag { tag: String, existing: Vec<String> },
    /// No ADR has the number.
    NoSuchAdr { number: u32 },
}

impl fmt::Display for ViewError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

/// What `adr new` created.
#[derive(Debug, PartialEq, Eq)]
struct Created {
    /// The repository-relative path of the new ADR.
    path: String,
    /// The new ADR's tags that no other ADR uses.
    unused_tags: Vec<String>,
    /// Every tag the other ADRs use, sorted.
    existing_tags: Vec<String>,
}

/// Why `adr new` refused to create an ADR. It changes nothing when it refuses.
#[derive(Debug, PartialEq, Eq)]
enum NewError {
    /// The title is empty.
    EmptyTitle,
    /// The title contains `"` or `\`.
    TitleQuoteOrBackslash,
    /// No tags were given.
    NoTags,
    /// A tag isn't lowercase letters, digits, and hyphens.
    BadTag { tag: String },
    /// The ADR to supersede doesn't exist.
    SupersedesMissing { number: u32 },
    /// The ADR to supersede is already in `superseded/`.
    AlreadySuperseded { number: u32 },
    /// Reading or writing a file failed.
    Io(String),
}

/// Parses the arguments after `adr`.
fn parse_args(_parser: lexopt::Parser) -> Result<AdrCommand, lexopt::Error> {
    todo!()
}

/// An ADR number given as `3`, `0003`, or `ADR-0003`.
fn parse_number(_text: &str) -> Option<u32> {
    todo!()
}

/// Reads an ADR's front matter, or every problem that keeps it from being read.
fn parse_front_matter(_text: &str) -> Result<FrontMatter, Vec<ProblemKind>> {
    todo!()
}

/// Reads the files in `active/` and `superseded/`, in that order and sorted by name, and the
/// contents of `tags.yaml`. A missing directory or tags file counts as empty.
fn read_adr_files(_root: &Path) -> io::Result<(Vec<AdrFile>, Option<String>)> {
    todo!()
}

/// Checks the ADRs and `tags.yaml` (`None` if it's missing).
fn check(_files: &[AdrFile], _tags: Option<&str>) -> Vec<Problem> {
    todo!()
}

/// Reads the ADRs for the views, leaving out any whose front matter can't be read.
fn load(_files: &[AdrFile], _tags: Option<&str>) -> Loaded {
    todo!()
}

/// `adr list`.
fn list(_adrs: &[Adr], _options: &ListOptions) -> Result<String, ViewError> {
    todo!()
}

/// `adr tags`.
fn tags(_adrs: &[Adr]) -> String {
    todo!()
}

/// `adr print`, for the ADRs in the order given.
fn print(_adrs: &[Adr], _numbers: &[u32]) -> Result<String, ViewError> {
    todo!()
}

/// `adr new`, dating the ADR `date`.
fn new_adr(_root: &Path, _args: &NewArgs, _date: &str) -> Result<Created, NewError> {
    todo!()
}

/// The file name slug for a title.
fn slug(_title: &str) -> String {
    todo!()
}

/// One more than the highest ADR number, or 1 if there are no ADRs.
fn next_number(_files: &[AdrFile]) -> u32 {
    todo!()
}

/// `text` wrapped greedily at `width` columns, each line starting with `indent`.
fn wrap(_text: &str, _width: usize, _indent: &str) -> String {
    todo!()
}

/// The Unix time and the UTC offset in seconds, from the end of `git var GIT_COMMITTER_IDENT`'s
/// output.
fn parse_git_ident(_line: &str) -> Option<(i64, i32)> {
    todo!()
}

/// The `YYYY-MM-DD` date at `unix_time`, in the zone `offset` seconds from UTC.
fn local_date(_unix_time: i64, _offset: i32) -> String {
    todo!()
}

/// Today's local date from git, with `env` added to git's environment, or `None` if git can't
/// run.
fn local_today(_env: &[(&str, &str)]) -> Option<String> {
    todo!()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;

    use super::*;
    use Dir::{Active, Superseded};

    // Fixtures

    fn file(dir: Dir, name: &str, text: &str) -> AdrFile {
        AdrFile { dir, name: name.to_string(), text: text.to_string() }
    }

    /// An ADR file with the given front matter lines.
    fn with_front_matter(dir: Dir, name: &str, front_matter: &str) -> AdrFile {
        file(dir, name, &format!("---\n{front_matter}---\n\n# Body\n"))
    }

    /// A well-formed ADR numbered `n`, named `000n-decision-n.md`.
    fn valid(dir: Dir, n: u32, supersedes: &[u32]) -> AdrFile {
        let mut front_matter = format!(
            "id: ADR-{n:04}\ntitle: \"Decision {n}\"\nsummary: \"Summary {n}.\"\ndate: 2026-11-02\n"
        );
        if !supersedes.is_empty() {
            let ids: Vec<String> = supersedes.iter().map(|s| format!("ADR-{s:04}")).collect();
            front_matter.push_str(&format!("supersedes: {}\n", ids.join(", ")));
        }
        with_front_matter(dir, &format!("{n:04}-decision-{n}.md"), &front_matter)
    }

    /// `tags.yaml` giving each numbered ADR the tag `graph`.
    fn tags_for(numbers: &[u32]) -> String {
        numbers.iter().map(|n| format!("ADR-{n:04}: graph\n")).collect()
    }

    fn path(dir: Dir, name: &str) -> String {
        format!("{ADR_DIR}/{}/{name}", dir.name())
    }

    fn in_adr(dir: Dir, name: &str, kind: ProblemKind) -> Problem {
        Problem { path: path(dir, name), kind }
    }

    fn in_tags(kind: ProblemKind) -> Problem {
        Problem { path: format!("{ADR_DIR}/tags.yaml"), kind }
    }

    /// Problems in a fixed order, so tests don't depend on the order checks run in.
    fn sorted(mut problems: Vec<Problem>) -> Vec<Problem> {
        problems.sort();
        problems
    }

    fn key(name: &str) -> String {
        name.to_string()
    }

    /// Three ADRs: 0001 superseded by 0003, and 0002 and 0003 in force.
    fn sample_files() -> Vec<AdrFile> {
        vec![
            with_front_matter(
                Active,
                "0002-versioned-project-files.md",
                "id: ADR-0002\ntitle: \"Versioned project files\"\nsummary: \"Project files \
                 carry a format version, and older versions are migrated when loaded.\"\n\
                 date: 2026-11-01\n",
            ),
            with_front_matter(
                Active,
                "0003-wire-types-checked-on-connect.md",
                "id: ADR-0003\ntitle: \"Wire types: checked on connect\"\nsummary: \"Every wire \
                 has a static type, and connecting two ports checks compatibility, so a mistyped \
                 connection is rejected before anything is evaluated.\"\ndate: 2026-11-02\n\
                 supersedes: ADR-0001\n",
            ),
            with_front_matter(
                Superseded,
                "0001-wires-are-dynamically-typed.md",
                "id: ADR-0001\ntitle: \"Wires are dynamically typed\"\nsummary: \"Wires carry \
                 values of any type, checked when a tile reads them.\"\ndate: 2026-10-30\n",
            ),
        ]
    }

    const SAMPLE_TAGS: &str =
        "ADR-0001: graph, dynamic-typing\nADR-0002: file-format\nADR-0003: graph, wires\n";

    fn sample() -> Vec<Adr> {
        let loaded = load(&sample_files(), Some(SAMPLE_TAGS));
        assert_eq!(loaded.warnings, Vec::<String>::new());
        loaded.adrs
    }

    /// A scratch repository under the system temp directory, removed when dropped.
    struct TempRoot(
        /// The root directory.
        PathBuf,
    );

    impl TempRoot {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("xtask-adr-test-{}-{name}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn write(&self, relative: &str, text: &str) {
            let target = self.0.join(relative);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::write(target, text).unwrap();
        }

        fn read(&self, relative: &str) -> String {
            fs::read_to_string(self.0.join(relative)).unwrap()
        }

        /// Every file under the root, by repository-relative path.
        fn snapshot(&self) -> BTreeMap<String, String> {
            fn walk(base: &Path, dir: &Path, files: &mut BTreeMap<String, String>) {
                for entry in fs::read_dir(dir).unwrap() {
                    let entry = entry.unwrap().path();
                    if entry.is_dir() {
                        walk(base, &entry, files);
                    } else {
                        let relative = entry.strip_prefix(base).unwrap();
                        let relative = relative.to_string_lossy().replace('\\', "/");
                        files.insert(relative, fs::read_to_string(&entry).unwrap());
                    }
                }
            }
            let mut files = BTreeMap::new();
            walk(&self.0, &self.0, &mut files);
            files
        }

        /// Writes an ADR file at its place under `ADR_DIR`.
        fn write_adr(&self, adr: &AdrFile) {
            self.write(&path(adr.dir, &adr.name), &adr.text);
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    const TEMPLATE_TEXT: &str = "<!--\nCreate ADRs with `cargo xtask adr new`.\n-->\n\n\
                                 # ADR-NNNN: Decision title\n\n## Context\n\nThe problem.\n";

    /// A scratch repository with the ADR template, ADRs 0001 and 0002 in force, and their tags.
    fn repository(name: &str) -> TempRoot {
        let root = TempRoot::new(name);
        root.write(TEMPLATE, TEMPLATE_TEXT);
        root.write_adr(&valid(Active, 1, &[]));
        root.write_adr(&valid(Active, 2, &[]));
        root.write(&format!("{ADR_DIR}/tags.yaml"), "ADR-0001: graph\nADR-0002: file-format\n");
        root
    }

    fn new_args(title: &str, tags: &[&str], supersedes: &[u32]) -> NewArgs {
        NewArgs {
            title: title.to_string(),
            tags: tags.iter().map(ToString::to_string).collect(),
            supersedes: supersedes.to_vec(),
        }
    }

    // Arguments

    fn parse(args: &[&str]) -> Result<AdrCommand, String> {
        parse_args(lexopt::Parser::from_args(args)).map_err(|err| err.to_string())
    }

    #[test]
    fn list_options_parse() {
        assert_eq!(parse(&["list"]), Ok(AdrCommand::List(ListOptions::default())));
        assert_eq!(
            parse(&["list", "--all", "--tags", "--verbose"]),
            Ok(AdrCommand::List(ListOptions {
                all: true,
                grouping: Grouping::AllTags,
                verbose: true
            }))
        );
        assert_eq!(
            parse(&["list", "--tags=wires,graph"]),
            Ok(AdrCommand::List(ListOptions {
                grouping: Grouping::Tags(vec!["wires".into(), "graph".into()]),
                ..ListOptions::default()
            }))
        );
    }

    #[test]
    fn other_subcommands_parse() {
        assert_eq!(parse(&["tags"]), Ok(AdrCommand::Tags));
        assert_eq!(
            parse(&["print", "3", "0007", "ADR-0012"]),
            Ok(AdrCommand::Print(vec![3, 7, 12]))
        );
        assert_eq!(
            parse(&[
                "new",
                "Wire types: checked on connect",
                "--tags=graph,wires",
                "--supersedes=1,ADR-0002"
            ]),
            Ok(AdrCommand::New(new_args(
                "Wire types: checked on connect",
                &["graph", "wires"],
                &[1, 2]
            )))
        );
        assert_eq!(
            parse(&["new", "--tags=graph", "Title first or last"]),
            Ok(AdrCommand::New(new_args("Title first or last", &["graph"], &[])))
        );
    }

    #[test]
    fn option_values_must_be_attached() {
        assert_eq!(
            parse(&["list", "--tags", "graph"]),
            Err("`graph` looks like a value for `--tags`; attach it as `--tags=graph`".to_string())
        );
        assert_eq!(
            parse(&["new", "Title", "--tags", "graph"]),
            Err("`graph` looks like a value for `--tags`; attach it as `--tags=graph`".to_string())
        );
        assert_eq!(
            parse(&["new", "Title", "--tags=graph", "--supersedes", "1"]),
            Err("`1` looks like a value for `--supersedes`; attach it as `--supersedes=1`"
                .to_string())
        );
    }

    #[test]
    fn bad_arguments_fail() {
        assert_eq!(
            parse(&["new", "Title"]),
            Err("`adr new` needs `--tags=<tags>` with at least one tag".to_string())
        );
        assert_eq!(
            parse(&["new", "Title", "--tags="]),
            Err("`--tags=` needs at least one tag".to_string())
        );
        assert_eq!(
            parse(&["list", "--tags="]),
            Err("`--tags=` needs at least one tag".to_string())
        );
        assert_eq!(
            parse(&["print", "x"]),
            Err("`x` isn't an ADR number; use a form like 3, 0003, or ADR-0003".to_string())
        );
        assert_eq!(
            parse(&["new", "Title", "--tags=graph", "--supersedes=one"]),
            Err("`one` isn't an ADR number; use a form like 3, 0003, or ADR-0003".to_string())
        );
        assert_eq!(parse(&["print"]), Err("`adr print` needs at least one ADR number".to_string()));
        assert_eq!(parse(&["new", "--tags=graph"]), Err("`adr new` needs a title".to_string()));
        assert!(parse(&[]).is_err());
        assert!(parse(&["index"]).is_err());
        assert!(parse(&["list", "--width=100"]).is_err());
        assert!(parse(&["tags", "extra"]).is_err());
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

    // Front matter

    #[test]
    fn front_matter_reads_every_field() {
        let text = "---\nid: ADR-0003\ntitle: \"Wire types: checked on connect\"\n\
                    summary: \"Every wire has a static type.\"\ndate: 2026-11-02\n\
                    supersedes: ADR-0001, ADR-0002\n---\n\n# ADR-0003\n";
        assert_eq!(
            parse_front_matter(text),
            Ok(FrontMatter {
                number: 3,
                title: "Wire types: checked on connect".into(),
                summary: "Every wire has a static type.".into(),
                date: "2026-11-02".into(),
                supersedes: vec![1, 2],
            })
        );
    }

    #[test]
    fn quoted_values_continue_on_indented_lines() {
        let text = "---\nid: ADR-0003\ntitle: \"Wire types\"\nsummary: \"Every wire has a\n  \
                    static type,\n    and connecting   checks it.\"\ndate: 2026-11-02\n---\n";
        let front_matter = parse_front_matter(text).unwrap();
        assert_eq!(front_matter.summary, "Every wire has a static type, and connecting checks it.");
    }

    #[test]
    fn front_matter_without_block_fails() {
        assert_eq!(parse_front_matter("# ADR-0003\n"), Err(vec![ProblemKind::NoFrontMatter]));
        assert_eq!(
            parse_front_matter("---\nid: ADR-0003\n"),
            Err(vec![ProblemKind::NoFrontMatter])
        );
    }

    // Checks

    #[test]
    fn empty_directories_pass() {
        assert_eq!(check(&[], None), Vec::<Problem>::new());
        assert_eq!(check(&[], Some("")), Vec::<Problem>::new());
    }

    #[test]
    fn missing_directories_and_tags_count_as_empty() {
        let root = TempRoot::new("missing");
        assert_eq!(read_adr_files(&root.0).unwrap(), (vec![], None));
    }

    #[test]
    fn files_are_read_from_both_directories() {
        let root = TempRoot::new("read");
        let files = [
            valid(Superseded, 1, &[]),
            valid(Active, 3, &[]),
            valid(Active, 2, &[1]),
        ];
        for adr in &files {
            root.write_adr(adr);
        }
        root.write(&format!("{ADR_DIR}/tags.yaml"), "ADR-0001: graph\n");
        root.write(&format!("{ADR_DIR}/active/notes/ignored.md"), "a directory, not an ADR\n");
        assert_eq!(
            read_adr_files(&root.0).unwrap(),
            (
                vec![
                    valid(Active, 2, &[1]),
                    valid(Active, 3, &[]),
                    valid(Superseded, 1, &[])
                ],
                Some("ADR-0001: graph\n".to_string())
            )
        );
    }

    #[test]
    fn consistent_chain_passes() {
        let files = [
            valid(Superseded, 1, &[]),
            valid(Superseded, 2, &[1]),
            valid(Active, 3, &[2]),
        ];
        assert_eq!(check(&files, Some(&tags_for(&[1, 2, 3]))), Vec::<Problem>::new());
    }

    #[test]
    fn file_names_must_be_numbered() {
        let files = [
            valid(Active, 1, &[]),
            file(Active, "notes.md", "---\n---\n"),
            file(Active, "0004.md", "---\n---\n"),
            file(Superseded, "4-short.md", "---\n---\n"),
        ];
        assert_eq!(
            sorted(check(&files, Some(&tags_for(&[1])))),
            sorted(vec![
                in_adr(Active, "notes.md", ProblemKind::BadName),
                in_adr(Active, "0004.md", ProblemKind::BadName),
                in_adr(Superseded, "4-short.md", ProblemKind::BadName),
            ])
        );
    }

    #[test]
    fn front_matter_fields_are_checked() {
        let name = "0001-decision.md";
        let check_one = |front_matter: &str| {
            check(&[with_front_matter(Active, name, front_matter)], Some(&tags_for(&[1])))
        };
        let fields = "title: \"T\"\nsummary: \"S.\"\ndate: 2026-11-02\n";
        assert_eq!(
            check_one(fields),
            vec![in_adr(
                Active,
                name,
                ProblemKind::MissingField { key: key("id") }
            )]
        );
        assert_eq!(
            check_one(&format!("id: ADR-0001\n{fields}summary: \"Again.\"\n")),
            vec![in_adr(
                Active,
                name,
                ProblemKind::DuplicateKey { key: key("summary") }
            )]
        );
        assert_eq!(
            check_one(&format!("id: ADR-0001\n{fields}area: graph\n")),
            vec![in_adr(
                Active,
                name,
                ProblemKind::UnknownKey { key: key("area") }
            )]
        );
        assert_eq!(
            check_one("id: ADR-0001\ntitle: \"T\"\nsummary: \"\"\ndate: 2026-11-02\n"),
            vec![in_adr(
                Active,
                name,
                ProblemKind::EmptyField { key: key("summary") }
            )]
        );
        assert_eq!(
            check_one("id: ADR-0001\ntitle: \"T\"\nsummary: \"S.\"\ndate:\n"),
            vec![in_adr(
                Active,
                name,
                ProblemKind::EmptyField { key: key("date") }
            )]
        );
        assert_eq!(
            check_one(&format!("id: ADR-0001\n{fields}not a field\n")),
            vec![in_adr(
                Active,
                name,
                ProblemKind::MalformedLine { line: "not a field".into() }
            )]
        );
        assert_eq!(
            check_one(&format!("id: 1\n{fields}")),
            vec![in_adr(
                Active,
                name,
                ProblemKind::BadId { value: "1".into() }
            )]
        );
        for date in ["2026-11-2", "2026-13-01", "2026-11-32", "November 2, 2026"] {
            assert_eq!(
                check_one(&format!("id: ADR-0001\ntitle: \"T\"\nsummary: \"S.\"\ndate: {date}\n")),
                vec![in_adr(
                    Active,
                    name,
                    ProblemKind::BadDate { value: date.into() }
                )],
                "{date}"
            );
        }
    }

    #[test]
    fn quoting_is_checked() {
        let name = "0001-decision.md";
        let check_one = |title: &str, summary: &str| {
            let front_matter =
                format!("id: ADR-0001\ntitle: {title}\nsummary: {summary}\ndate: 2026-11-02\n");
            check(&[with_front_matter(Active, name, &front_matter)], Some(&tags_for(&[1])))
        };
        assert_eq!(
            check_one("Unquoted", "\"S.\""),
            vec![in_adr(
                Active,
                name,
                ProblemKind::NotQuoted { key: key("title") }
            )]
        );
        assert_eq!(
            check_one("\"T\"", "\"A \"quoted\" word.\""),
            vec![in_adr(
                Active,
                name,
                ProblemKind::QuoteOrBackslash { key: key("summary") }
            )]
        );
        assert_eq!(
            check_one("\"Back\\slash\"", "\"S.\""),
            vec![in_adr(
                Active,
                name,
                ProblemKind::QuoteOrBackslash { key: key("title") }
            )]
        );
        assert_eq!(
            check_one("\"T\"", "\"Never closed."),
            vec![in_adr(
                Active,
                name,
                ProblemKind::UnterminatedQuote { key: key("summary") }
            )]
        );
        let long = format!("\"{}\"", "x".repeat(301));
        assert_eq!(
            check_one("\"T\"", &long),
            vec![in_adr(
                Active,
                name,
                ProblemKind::SummaryTooLong { chars: 301 }
            )]
        );
        let at_limit = format!("\"{}\"", "x".repeat(300));
        assert_eq!(check_one("\"T\"", &at_limit), Vec::<Problem>::new());
    }

    #[test]
    fn ids_must_match_names_and_be_unique() {
        let mismatched = with_front_matter(
            Active,
            "0002-decision.md",
            "id: ADR-0003\ntitle: \"T\"\nsummary: \"S.\"\ndate: 2026-11-02\n",
        );
        assert_eq!(
            check(&[mismatched], Some(&tags_for(&[2]))),
            vec![in_adr(
                Active,
                "0002-decision.md",
                ProblemKind::IdMismatch { id: 3 }
            )]
        );

        let first = valid(Active, 3, &[]);
        let mut second = valid(Superseded, 3, &[]);
        second.name = "0003-another.md".into();
        assert_eq!(
            sorted(check(&[first, second], Some(&tags_for(&[3])))),
            sorted(vec![
                in_adr(
                    Superseded,
                    "0003-another.md",
                    ProblemKind::DuplicateId {
                        number: 3,
                        other: path(Active, "0003-decision-3.md"),
                    }
                ),
                in_adr(Superseded, "0003-another.md", ProblemKind::SupersededButNotListed),
            ])
        );
    }

    #[test]
    fn supersedes_entries_must_be_ids_of_existing_lower_adrs() {
        let bad_entry = with_front_matter(
            Active,
            "0002-decision-2.md",
            "id: ADR-0002\ntitle: \"T\"\nsummary: \"S.\"\ndate: 2026-11-02\nsupersedes: 1\n",
        );
        assert_eq!(
            check(&[valid(Active, 1, &[]), bad_entry], Some(&tags_for(&[1, 2]))),
            vec![in_adr(
                Active,
                "0002-decision-2.md",
                ProblemKind::BadSupersedesEntry { entry: "1".into() }
            )]
        );
        assert_eq!(
            check(&[valid(Active, 2, &[7])], Some(&tags_for(&[2]))),
            vec![in_adr(
                Active,
                "0002-decision-2.md",
                ProblemKind::SupersedesMissing { number: 7 }
            )]
        );
        assert_eq!(
            sorted(check(&[valid(Active, 2, &[2])], Some(&tags_for(&[2])))),
            sorted(vec![
                in_adr(Active, "0002-decision-2.md", ProblemKind::SupersedesNotLower { number: 2 }),
                in_adr(Active, "0002-decision-2.md", ProblemKind::ActiveButSuperseded { by: 2 }),
            ])
        );
        assert_eq!(
            sorted(check(
                &[valid(Superseded, 3, &[]), valid(Active, 2, &[3])],
                Some(&tags_for(&[2, 3]))
            )),
            vec![in_adr(
                Active,
                "0002-decision-2.md",
                ProblemKind::SupersedesNotLower { number: 3 }
            )]
        );
    }

    #[test]
    fn superseded_adrs_have_one_successor_and_live_in_superseded() {
        let files = [
            valid(Superseded, 1, &[]),
            valid(Active, 2, &[1]),
            valid(Active, 3, &[1]),
        ];
        assert_eq!(
            check(&files, Some(&tags_for(&[1, 2, 3]))),
            vec![in_adr(
                Superseded,
                "0001-decision-1.md",
                ProblemKind::SupersededByMany { by: vec![2, 3] }
            )]
        );
        assert_eq!(
            check(&[valid(Superseded, 1, &[])], Some(&tags_for(&[1]))),
            vec![in_adr(
                Superseded,
                "0001-decision-1.md",
                ProblemKind::SupersededButNotListed
            )]
        );
        assert_eq!(
            check(&[valid(Active, 1, &[]), valid(Active, 2, &[1])], Some(&tags_for(&[1, 2]))),
            vec![in_adr(
                Active,
                "0001-decision-1.md",
                ProblemKind::ActiveButSuperseded { by: 2 }
            )]
        );
    }

    #[test]
    fn every_adr_needs_one_well_formed_tags_entry() {
        let files = [
            valid(Active, 1, &[]),
            valid(Active, 2, &[]),
            valid(Active, 3, &[]),
        ];
        let tags = "ADR-0001: graph\nADR-0001: wires\nADR-0003:\nADR-0004: graph\n";
        assert_eq!(
            sorted(check(&files, Some(tags))),
            sorted(vec![
                in_tags(ProblemKind::DuplicateTagsEntry { number: 1 }),
                in_tags(ProblemKind::TagsMissing { number: 2 }),
                in_tags(ProblemKind::NoTags { number: 3 }),
                in_tags(ProblemKind::TagsForMissingAdr { number: 4 }),
            ])
        );
        assert_eq!(
            check(&files[..1], Some("ADR-0001: graph, Wires, two words\n")),
            vec![
                in_tags(ProblemKind::BadTag { number: 1, tag: "Wires".into() }),
                in_tags(ProblemKind::BadTag { number: 1, tag: "two words".into() }),
            ]
        );
        assert_eq!(
            sorted(check(&files[..1], Some("ADR-0001: graph\ngraph: ADR-0001\n"))),
            vec![in_tags(ProblemKind::TagsMalformedLine {
                line: "graph: ADR-0001".into()
            })]
        );
        assert_eq!(
            sorted(check(&files[..2], None)),
            sorted(vec![
                in_tags(ProblemKind::TagsMissing { number: 1 }),
                in_tags(ProblemKind::TagsMissing { number: 2 }),
            ])
        );
    }

    #[test]
    fn problems_display_as_messages() {
        let name = "0002-decision-2.md";
        assert_eq!(
            in_adr(Active, "notes.md", ProblemKind::BadName).to_string(),
            "docs/architecture/decisions/active/notes.md: ADR file names look like \
             `0003-short-title.md`"
        );
        assert_eq!(
            in_adr(Active, name, ProblemKind::EmptyField { key: key("summary") }).to_string(),
            "docs/architecture/decisions/active/0002-decision-2.md: `summary` is empty"
        );
        assert_eq!(
            in_adr(Active, name, ProblemKind::UnknownKey { key: key("area") }).to_string(),
            "docs/architecture/decisions/active/0002-decision-2.md: `area` isn't an ADR front \
             matter field"
        );
        assert_eq!(
            in_adr(Active, name, ProblemKind::ActiveButSuperseded { by: 3 }).to_string(),
            "docs/architecture/decisions/active/0002-decision-2.md: ADR-0003 supersedes this \
             one, so it belongs in `superseded/`"
        );
        assert_eq!(
            in_adr(Superseded, name, ProblemKind::SupersededButNotListed).to_string(),
            "docs/architecture/decisions/superseded/0002-decision-2.md: no ADR supersedes this \
             one, so it belongs in `active/`"
        );
        assert_eq!(
            in_tags(ProblemKind::TagsMissing { number: 2 }).to_string(),
            "docs/architecture/decisions/tags.yaml: ADR-0002 has no entry; give it at least one \
             tag"
        );
        assert_eq!(
            in_tags(ProblemKind::BadTag { number: 2, tag: "Wires".into() }).to_string(),
            "docs/architecture/decisions/tags.yaml: ADR-0002 has the tag `Wires`; tags are \
             lowercase letters, digits, and hyphens"
        );
    }

    // Views

    #[test]
    fn list_shows_adrs_in_force_by_number() {
        assert_eq!(
            list(&sample(), &ListOptions::default()),
            Ok("- ADR-0002: Versioned project files (file-format)\n\
                - ADR-0003: Wire types: checked on connect (graph/wires)\n"
                .to_string())
        );
    }

    #[test]
    fn list_all_adds_superseded_adrs() {
        let options = ListOptions { all: true, ..ListOptions::default() };
        assert_eq!(
            list(&sample(), &options),
            Ok("- ADR-0001: Wires are dynamically typed (graph/dynamic-typing), superseded by \
                ADR-0003\n\
                - ADR-0002: Versioned project files (file-format)\n\
                - ADR-0003: Wire types: checked on connect (graph/wires)\n"
                .to_string())
        );
    }

    #[test]
    fn list_groups_by_every_tag() {
        let options = ListOptions { grouping: Grouping::AllTags, ..ListOptions::default() };
        assert_eq!(
            list(&sample(), &options),
            Ok("## file-format\n\n\
                - ADR-0002: Versioned project files (file-format)\n\n\
                ## graph\n\n\
                - ADR-0003: Wire types: checked on connect (graph/wires)\n\n\
                ## wires\n\n\
                - ADR-0003: Wire types: checked on connect (graph/wires)\n"
                .to_string())
        );

        let options = ListOptions { all: true, grouping: Grouping::AllTags, verbose: false };
        assert_eq!(
            list(&sample(), &options),
            Ok("## dynamic-typing\n\n\
                - ADR-0001: Wires are dynamically typed (graph/dynamic-typing), superseded by \
                ADR-0003\n\n\
                ## file-format\n\n\
                - ADR-0002: Versioned project files (file-format)\n\n\
                ## graph\n\n\
                - ADR-0001: Wires are dynamically typed (graph/dynamic-typing), superseded by \
                ADR-0003\n\
                - ADR-0003: Wire types: checked on connect (graph/wires)\n\n\
                ## wires\n\n\
                - ADR-0003: Wire types: checked on connect (graph/wires)\n"
                .to_string())
        );
    }

    #[test]
    fn list_groups_by_given_tags_in_order() {
        let options = ListOptions {
            grouping: Grouping::Tags(vec!["wires".into(), "file-format".into()]),
            ..ListOptions::default()
        };
        assert_eq!(
            list(&sample(), &options),
            Ok("## wires\n\n\
                - ADR-0003: Wire types: checked on connect (graph/wires)\n\n\
                ## file-format\n\n\
                - ADR-0002: Versioned project files (file-format)\n"
                .to_string())
        );
    }

    #[test]
    fn list_rejects_unknown_tags() {
        let options = ListOptions {
            grouping: Grouping::Tags(vec!["graphs".into()]),
            ..ListOptions::default()
        };
        let error = ViewError::UnknownTag {
            tag: "graphs".into(),
            existing: vec![
                "dynamic-typing".into(),
                "file-format".into(),
                "graph".into(),
                "wires".into(),
            ],
        };
        assert_eq!(
            error.to_string(),
            "no ADR has the tag `graphs`; the tags are dynamic-typing, file-format, graph, wires"
        );
        assert_eq!(list(&sample(), &options), Err(error));
    }

    #[test]
    fn verbose_list_adds_wrapped_summaries() {
        let options = ListOptions { verbose: true, ..ListOptions::default() };
        assert_eq!(
            list(&sample(), &options),
            Ok("- ADR-0002: Versioned project files (file-format)\n  \
                Project files carry a format version, and older versions are migrated when\n  \
                loaded.\n\
                - ADR-0003: Wire types: checked on connect (graph/wires)\n  \
                Every wire has a static type, and connecting two ports checks compatibility,\n  \
                so a mistyped connection is rejected before anything is evaluated.\n"
                .to_string())
        );

        let options = ListOptions {
            grouping: Grouping::Tags(vec!["file-format".into()]),
            verbose: true,
            ..ListOptions::default()
        };
        assert_eq!(
            list(&sample(), &options),
            Ok("## file-format\n\n\
                - ADR-0002: Versioned project files (file-format)\n  \
                Project files carry a format version, and older versions are migrated when\n  \
                loaded.\n"
                .to_string())
        );
    }

    #[test]
    fn tags_lists_each_tag_with_adrs_in_force() {
        assert_eq!(
            tags(&sample()),
            "- dynamic-typing (0)\n- file-format (1)\n- graph (1)\n- wires (1)\n"
        );
    }

    #[test]
    fn views_say_none_yet_without_adrs() {
        assert_eq!(list(&[], &ListOptions::default()), Ok("None yet.\n".to_string()));
        assert_eq!(tags(&[]), "None yet.\n");
    }

    #[test]
    fn print_shows_files_with_their_status() {
        let files = sample_files();
        let header_3 = format!("{} (in force)", path(Active, &files[1].name));
        let header_1 = format!("{} (superseded by ADR-0003)", path(Superseded, &files[2].name));
        assert_eq!(
            print(&sample(), &[3, 1]),
            Ok(format!("{header_3}\n\n{}\n{header_1}\n\n{}", files[1].text, files[2].text))
        );
        let error = ViewError::NoSuchAdr { number: 9 };
        assert_eq!(error.to_string(), "there's no ADR-0009");
        assert_eq!(print(&sample(), &[3, 9]), Err(error));
    }

    #[test]
    fn views_leave_out_unreadable_adrs_and_show_untagged_ones() {
        let mut files = sample_files();
        files.push(file(Active, "0004-broken.md", "No front matter.\n"));
        files.push(valid(Active, 5, &[]));
        let loaded = load(&files, Some(SAMPLE_TAGS));
        assert_eq!(
            loaded.warnings,
            [format!(
                "{}: front matter can't be read, so it's left out; `cargo xtask check-docs` has \
                 the details",
                path(Active, "0004-broken.md")
            )]
        );
        assert_eq!(
            list(&loaded.adrs, &ListOptions::default()),
            Ok("- ADR-0002: Versioned project files (file-format)\n\
                - ADR-0003: Wire types: checked on connect (graph/wires)\n\
                - ADR-0005: Decision 5\n"
                .to_string())
        );
    }

    #[test]
    fn summaries_wrap_greedily() {
        assert_eq!(wrap("Short summary.", 80, "  "), "  Short summary.");
        assert_eq!(wrap("aaaa bbbb cccc ddd eee", 20, "  "), "  aaaa bbbb cccc ddd\n  eee");
        assert_eq!(wrap("a supercalifragilistic b", 12, "  "), "  a\n  supercalifragilistic\n  b");
        assert_eq!(wrap("spaced  out\n  text", 80, "  "), "  spaced out text");
    }

    // Creating ADRs

    #[test]
    fn slugs_keep_ascii_letters_and_digits() {
        assert_eq!(slug("Wire types: checked on connect"), "wire-types-checked-on-connect");
        assert_eq!(slug("  C++ & Rust 2024!  "), "c-rust-2024");
        assert_eq!(slug("Café über"), "caf-ber");
    }

    #[test]
    fn next_number_follows_the_highest_in_either_directory() {
        assert_eq!(next_number(&[]), 1);
        let files = [
            valid(Active, 1, &[]),
            valid(Superseded, 3, &[]),
            file(Active, "notes.md", ""),
        ];
        assert_eq!(next_number(&files), 4);
    }

    #[test]
    fn new_creates_an_adr_and_its_tags_entry() {
        let root = repository("create");
        let created = new_adr(
            &root.0,
            &new_args("Wire types: checked on connect", &["graph", "wires"], &[]),
            "2026-11-02",
        );
        let new_path = path(Active, "0003-wire-types-checked-on-connect.md");
        assert_eq!(
            created,
            Ok(Created {
                path: new_path.clone(),
                unused_tags: vec!["wires".into()],
                existing_tags: vec!["file-format".into(), "graph".into()],
            })
        );
        assert_eq!(
            root.read(&new_path),
            "---\nid: ADR-0003\ntitle: \"Wire types: checked on connect\"\nsummary: \"\"\n\
             date: 2026-11-02\n---\n\n# ADR-0003: Wire types: checked on connect\n\n\
             ## Context\n\nThe problem.\n"
        );
        assert_eq!(
            root.read(&format!("{ADR_DIR}/tags.yaml")),
            "ADR-0001: graph\nADR-0002: file-format\nADR-0003: graph, wires\n"
        );
    }

    #[test]
    fn new_moves_the_adrs_it_supersedes() {
        let root = repository("supersede");
        let old = root.read(&path(Active, "0001-decision-1.md"));
        let created =
            new_adr(&root.0, &new_args("Replacement", &["graph"], &[1, 2]), "2026-11-02").unwrap();
        assert_eq!(created.unused_tags, Vec::<String>::new());
        assert_eq!(root.read(&path(Superseded, "0001-decision-1.md")), old);
        assert!(!root.0.join(path(Active, "0001-decision-1.md")).exists());
        assert!(root.0.join(path(Superseded, "0002-decision-2.md")).exists());
        assert!(
            root.read(&created.path)
                .contains("\ndate: 2026-11-02\nsupersedes: ADR-0001, ADR-0002\n---\n")
        );
    }

    #[test]
    fn new_leaves_only_the_summary_to_write() {
        let root = repository("then-check");
        let created =
            new_adr(&root.0, &new_args("Replacement", &["graph"], &[1]), "2026-11-02").unwrap();
        let (files, tags) = read_adr_files(&root.0).unwrap();
        assert_eq!(
            check(&files, tags.as_deref()),
            vec![Problem {
                path: created.path,
                kind: ProblemKind::EmptyField { key: key("summary") },
            }]
        );
    }

    #[test]
    fn new_works_in_an_empty_repository() {
        let root = TempRoot::new("first");
        root.write(TEMPLATE, TEMPLATE_TEXT);
        let created = new_adr(&root.0, &new_args("First", &["graph"], &[]), "2026-11-02");
        assert_eq!(
            created,
            Ok(Created {
                path: path(Active, "0001-first.md"),
                unused_tags: vec!["graph".into()],
                existing_tags: vec![],
            })
        );
        assert_eq!(root.read(&format!("{ADR_DIR}/tags.yaml")), "ADR-0001: graph\n");
    }

    #[test]
    fn new_refuses_bad_arguments_and_changes_nothing() {
        let root = repository("refuse");
        root.write_adr(&valid(Superseded, 3, &[]));
        root.write_adr(&valid(Active, 4, &[3]));
        let before = root.snapshot();
        let cases = [
            (new_args("", &["graph"], &[]), NewError::EmptyTitle),
            (new_args("A \"quoted\" title", &["graph"], &[]), NewError::TitleQuoteOrBackslash),
            (new_args("Back\\slash", &["graph"], &[]), NewError::TitleQuoteOrBackslash),
            (new_args("Title", &[], &[]), NewError::NoTags),
            (new_args("Title", &["Graph"], &[]), NewError::BadTag { tag: "Graph".into() }),
            (new_args("Title", &["graph"], &[9]), NewError::SupersedesMissing { number: 9 }),
            (new_args("Title", &["graph"], &[3]), NewError::AlreadySuperseded { number: 3 }),
        ];
        for (args, error) in cases {
            assert_eq!(new_adr(&root.0, &args, "2026-11-02"), Err(error), "{args:?}");
            assert_eq!(root.snapshot(), before, "{args:?}");
        }
    }

    // Dates

    #[test]
    fn git_ident_ends_with_time_and_offset() {
        assert_eq!(
            parse_git_ident("xtask <xtask> 1792787808 -0600"),
            Some((1_792_787_808, -21_600))
        );
        assert_eq!(
            parse_git_ident("A Name <a@b.c> 1700000000 +0530"),
            Some((1_700_000_000, 19_800))
        );
        assert_eq!(parse_git_ident("Odd > Name <x> 5 +0000"), Some((5, 0)));
        assert_eq!(parse_git_ident("xtask <xtask> 5 0600"), None);
        assert_eq!(parse_git_ident("xtask <xtask>"), None);
    }

    #[test]
    fn local_dates_apply_the_offset() {
        assert_eq!(local_date(0, 0), "1970-01-01");
        assert_eq!(local_date(0, -60), "1969-12-31");
        assert_eq!(local_date(-3_600, 0), "1969-12-31");
        // 2026-01-02 23:30 at UTC-8 is already January 3 in UTC.
        assert_eq!(local_date(1_767_425_400, -28_800), "2026-01-02");
        assert_eq!(local_date(1_767_425_400, 0), "2026-01-03");
    }

    #[test]
    fn local_dates_handle_leap_days() {
        assert_eq!(local_date(951_825_600, 0), "2000-02-29");
        assert_eq!(local_date(1_709_208_000, 0), "2024-02-29");
        assert_eq!(local_date(4_107_544_200, 0), "2100-03-01");
        assert_eq!(local_date(4_107_544_200, -3_600), "2100-02-28");
    }

    #[test]
    fn todays_date_comes_from_git() {
        assert_eq!(
            local_today(&[("GIT_COMMITTER_DATE", "2026-01-02T23:30:00-0800")]),
            Some("2026-01-02".to_string())
        );
    }
}
