//! The ADR checks that `check-docs` runs: the front matter schema, the supersede links between
//! ADRs, and `tags.yaml`.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt;

use super::records::{Fields, Records, read_front_matter};
use super::{AdrFile, Dir, TAGS_PATH, file_number, ids, is_tag, parse_id, split_tags, yaml};

/// A problem in one file.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Problem {
    /// The repository-relative path of the file to fix.
    pub(super) path: String,
    /// What's wrong with it.
    pub(super) kind: ProblemKind,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.kind)
    }
}

/// What's wrong with an ADR or with `tags.yaml`.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ProblemKind {
    // An ADR on its own
    /// The file name isn't like `0003-short-title.md`.
    BadName,
    /// The file name isn't like `0003-short-title.md`, with the `number` in `id`.
    NameMismatch { number: u32 },
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
    /// A quoted value contains `"` or `\`, or has text after its closing quote.
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
    /// The entry names an ADR that doesn't exist.
    TagsForMissingAdr { number: u32 },
    /// The ADR has more than one entry.
    DuplicateTagsEntry { number: u32 },
    /// A tag isn't lowercase letters, digits, and hyphens.
    BadTag { number: u32, tag: String },
}

impl fmt::Display for ProblemKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadName => write!(f, "ADR file names look like `0003-short-title.md`"),
            Self::NameMismatch { number } => write!(
                f,
                "`id` is ADR-{number:04}, so the file name should look like \
                 `{number:04}-short-title.md`"
            ),
            Self::NoFrontMatter => {
                write!(f, "the file doesn't start with front matter between `---` lines")
            },
            Self::MalformedLine { line } => write!(
                f,
                "front matter line `{line}` isn't `key: value` or an indented continuation of a \
                 quoted value"
            ),
            Self::UnknownKey { key } => write!(f, "`{key}` isn't an ADR front matter field"),
            Self::DuplicateKey { key } => write!(f, "`{key}` appears more than once"),
            Self::MissingField { key } => write!(f, "`{key}` is missing"),
            Self::EmptyField { key } => write!(f, "`{key}` is empty"),
            Self::NotQuoted { key } => write!(f, "`{key}` needs its value in double quotes"),
            Self::QuoteOrBackslash { key } => {
                write!(
                    f,
                    "the quoted value of `{key}` contains `\"` or `\\`, or has text after its \
                     closing quote"
                )
            },
            Self::UnterminatedQuote { key } => write!(
                f,
                "the quoted value of `{key}` has no closing quote; continuation lines are indented"
            ),
            Self::SummaryTooLong { chars } => {
                write!(f, "`summary` is {chars} characters, over the limit of 300")
            },
            Self::BadId { value } => write!(f, "`id` is `{value}`, not an id like `ADR-0003`"),
            Self::IdMismatch { id } => {
                write!(f, "`id` is ADR-{id:04}, which doesn't match the number in the file name")
            },
            Self::DuplicateId { number, other } => write!(f, "{other} is also ADR-{number:04}"),
            Self::BadDate { value } => {
                write!(f, "`date` is `{value}`, not a date like `2026-11-02`")
            },
            Self::BadSupersedesEntry { entry } => {
                write!(f, "`supersedes` lists `{entry}`, which isn't an id like `ADR-0003`")
            },
            Self::SupersedesMissing { number } => {
                write!(f, "supersedes ADR-{number:04}, which doesn't exist")
            },
            Self::SupersedesNotLower { number } => write!(
                f,
                "supersedes ADR-{number:04}, but an ADR can only supersede ones with lower numbers"
            ),
            Self::SupersededByMany { by } => {
                write!(f, "{} all supersede this one; an ADR has one successor", ids(by))
            },
            Self::SupersededButNotListed => {
                write!(f, "no ADR supersedes this one, so it belongs in `active/`")
            },
            Self::ActiveButSuperseded { by } => {
                write!(f, "ADR-{by:04} supersedes this one, so it belongs in `superseded/`")
            },
            Self::TagsMalformedLine { line } => {
                write!(f, "`{line}` isn't an entry like `ADR-0003: graph, wires`")
            },
            Self::TagsForMissingAdr { number } => {
                write!(f, "there's an entry for ADR-{number:04}, which doesn't exist")
            },
            Self::DuplicateTagsEntry { number } => {
                write!(f, "ADR-{number:04} has more than one entry")
            },
            Self::BadTag { number, tag } => write!(
                f,
                "ADR-{number:04} has the tag `{tag}`; tags are lowercase letters, digits, and \
                 hyphens"
            ),
        }
    }
}

/// An ADR file with a well-formed name, and its front matter as far as it could be read.
struct Checked<'a> {
    /// The file.
    file: &'a AdrFile,
    /// The file's repository-relative path.
    path: String,
    /// The number in the file name, which identifies the ADR in the other checks.
    number: u32,
    /// The front matter fields that could be read.
    fields: Fields,
}

/// Runs the consistency checks.
pub(super) fn check(_records: &Records) -> Vec<Problem> {
    todo!("the consistency checks")
}

/// Checks the ADRs and `tags.yaml` (`None` if it's missing).
pub(super) fn check_files(files: &[AdrFile], tags: Option<&str>) -> Vec<Problem> {
    let mut problems = Vec::new();
    let mut adrs = Vec::new();
    let mut paths: HashMap<u32, String> = HashMap::new();
    for file in files {
        let path = file.path();
        let Some(number) = file_number(&file.name) else {
            problems.push(Problem { path, kind: ProblemKind::BadName });
            continue;
        };
        let (fields, kinds) = read_front_matter(&file.text);
        let mut problem = |kind| problems.push(Problem { path: path.clone(), kind });
        kinds.into_iter().for_each(&mut problem);
        if let Some(id) = fields.number
            && id != number
        {
            problem(ProblemKind::IdMismatch { id });
        }
        match paths.get(&number) {
            Some(other) => problem(ProblemKind::DuplicateId { number, other: other.clone() }),
            None => {
                paths.insert(number, path.clone());
            },
        }
        adrs.push(Checked { file, path, number, fields });
    }
    problems.extend(check_supersedes(&adrs));
    problems.extend(check_tags(&adrs, tags.unwrap_or_default()));
    problems
}

/// Checks that each `supersedes` entry names an existing ADR with a lower number, that each ADR
/// has at most one successor, and that an ADR is in `superseded/` exactly when it has one.
fn check_supersedes(adrs: &[Checked<'_>]) -> Vec<Problem> {
    let mut problems = Vec::new();
    let numbers: HashSet<u32> = adrs.iter().map(|adr| adr.number).collect();
    let mut successors: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for adr in adrs {
        for &old in &adr.fields.supersedes {
            let kind = if numbers.contains(&old) {
                successors.entry(old).or_default().insert(adr.number);
                if old < adr.number {
                    continue;
                }
                ProblemKind::SupersedesNotLower { number: old }
            } else {
                ProblemKind::SupersedesMissing { number: old }
            };
            problems.push(Problem { path: adr.path.clone(), kind });
        }
    }
    for adr in adrs {
        let by: Vec<u32> = successors.get(&adr.number).into_iter().flatten().copied().collect();
        let mut problem = |kind| problems.push(Problem { path: adr.path.clone(), kind });
        if by.len() > 1 {
            problem(ProblemKind::SupersededByMany { by: by.clone() });
        }
        match (adr.file.dir, by.first()) {
            (Dir::Superseded, None) => problem(ProblemKind::SupersededButNotListed),
            (Dir::Active, Some(&by)) => problem(ProblemKind::ActiveButSuperseded { by }),
            _ => {},
        }
    }
    problems
}

/// Checks that `tags.yaml` has at most one entry for each ADR, and none for an ADR that doesn't
/// exist, and that each tag is well formed. An ADR doesn't need an entry, or tags in its entry.
fn check_tags(adrs: &[Checked<'_>], tags: &str) -> Vec<Problem> {
    let mut kinds = Vec::new();
    // The ADRs with an entry, including entries whose quoted value couldn't be read.
    let mut entered = BTreeSet::new();
    let (entries, errors) = yaml::read(tags);
    for error in errors {
        kinds.push(match error {
            yaml::Error::MalformedLine { line } => ProblemKind::TagsMalformedLine { line },
            yaml::Error::UnterminatedQuote { key } => {
                entered.extend(parse_id(&key));
                ProblemKind::UnterminatedQuote { key }
            },
            yaml::Error::QuoteOrBackslash { key } => {
                entered.extend(parse_id(&key));
                ProblemKind::QuoteOrBackslash { key }
            },
        });
    }
    let numbers: BTreeSet<u32> = adrs.iter().map(|adr| adr.number).collect();
    for entry in &entries {
        let Some(number) = parse_id(&entry.key) else {
            let line = match &entry.value {
                yaml::Value::Plain(value) => format!("{}: {value}", entry.key),
                yaml::Value::Quoted(value) => format!("{}: \"{value}\"", entry.key),
            };
            kinds.push(ProblemKind::TagsMalformedLine { line });
            continue;
        };
        if !entered.insert(number) {
            kinds.push(ProblemKind::DuplicateTagsEntry { number });
        } else if !numbers.contains(&number) {
            kinds.push(ProblemKind::TagsForMissingAdr { number });
        } else if !entry.value.text().is_empty() {
            for tag in split_tags(entry.value.text()).filter(|tag| !is_tag(tag)) {
                kinds.push(ProblemKind::BadTag { number, tag: tag.to_string() });
            }
        }
    }
    kinds.into_iter().map(|kind| Problem { path: TAGS_PATH.to_string(), kind }).collect()
}

#[cfg(test)]
mod tests {
    use super::ProblemKind::{
        ActiveButSuperseded, BadDate, BadTag, DuplicateId, EmptyField, NameMismatch,
        SummaryTooLong, SupersededButNotListed, SupersededByMany, SupersedesMissing,
        SupersedesNotLower, TagsForMissingAdr, UnknownKey,
    };
    use super::*;
    use crate::adr::Dir::{Active, Superseded};
    use crate::adr::fixtures::{named, valid, with_front_matter};
    use crate::adr::records::parse;
    use crate::adr::{AdrFile, adr_path};

    /// The file name of an ADR from `with_fields`.
    const NAME: &str = "0001-decision.md";

    fn problem(path: &str, kind: ProblemKind) -> Problem {
        Problem { path: path.to_string(), kind }
    }

    /// Problems in a fixed order, so tests don't depend on the order checks run in.
    fn sorted(mut problems: Vec<Problem>) -> Vec<Problem> {
        problems.sort();
        problems
    }

    /// The consistency problems with `files` and `tags`, which have to pass the structural checks.
    fn problems_in(files: &[AdrFile], tags: Option<&str>) -> Vec<Problem> {
        sorted(check(&parse(files, tags).unwrap()))
    }

    /// ADR-0001, named `NAME`, with these values for its other fields.
    fn with_fields(title: &str, summary: &str, date: &str) -> AdrFile {
        let front_matter =
            format!("id: ADR-0001\ntitle: {title}\nsummary: {summary}\ndate: {date}\n");
        with_front_matter(Active, NAME, &front_matter)
    }

    #[test]
    fn no_adrs_pass() {
        assert_eq!(check(&Records::default()), Vec::<Problem>::new());
    }

    #[test]
    fn consistent_chain_passes() {
        let files = [
            valid(Superseded, 1, &[]),
            valid(Superseded, 2, &[1]),
            valid(Active, 3, &[2]),
        ];
        assert_eq!(problems_in(&files, Some("ADR-0003: graph\n")), Vec::<Problem>::new());
    }

    #[test]
    fn file_names_must_be_numbered_with_the_id() {
        for name in [
            "notes.md",
            "0001.md",
            "1-short.md",
            "0002-short.md",
            "0001-Short.md",
            "0001-short.txt",
        ] {
            assert_eq!(
                problems_in(&[named(Active, name, 1)], None),
                [problem(&adr_path(Active, name), NameMismatch { number: 1 })],
                "{name}"
            );
        }
        // The same goes for `superseded/`.
        assert_eq!(
            problems_in(&[named(Superseded, "notes.md", 1), valid(Active, 2, &[1])], None),
            [problem(
                &adr_path(Superseded, "notes.md"),
                NameMismatch { number: 1 }
            )]
        );
    }

    #[test]
    fn title_and_summary_must_not_be_empty() {
        let path = &adr_path(Active, NAME);
        for empty in ["\"\"", ""] {
            assert_eq!(
                problems_in(&[with_fields(empty, "\"S.\"", "2026-11-02")], None),
                [problem(path, EmptyField { key: "title".into() })]
            );
            assert_eq!(
                problems_in(&[with_fields("\"T\"", empty, "2026-11-02")], None),
                [problem(path, EmptyField { key: "summary".into() })]
            );
        }
    }

    #[test]
    fn summaries_have_a_limit() {
        let summary = |chars: usize| format!("\"{}\"", "x".repeat(chars));
        assert_eq!(
            problems_in(&[with_fields("\"T\"", &summary(300), "2026-11-02")], None),
            Vec::<Problem>::new()
        );
        assert_eq!(
            problems_in(&[with_fields("\"T\"", &summary(301), "2026-11-02")], None),
            [problem(
                &adr_path(Active, NAME),
                SummaryTooLong { chars: 301 }
            )]
        );
    }

    #[test]
    fn dates_must_be_real() {
        for date in [
            "",
            "YYYY-MM-DD",
            "2026-11-2",
            "2026-13-01",
            "2026-11-32",
            "2026-02-29",
            "November 2, 2026",
        ] {
            assert_eq!(
                problems_in(&[with_fields("\"T\"", "\"S.\"", date)], None),
                [problem(
                    &adr_path(Active, NAME),
                    BadDate { value: date.into() }
                )],
                "{date}"
            );
        }
        for date in ["2026-11-02", "2024-02-29"] {
            assert_eq!(
                problems_in(&[with_fields("\"T\"", "\"S.\"", date)], None),
                Vec::<Problem>::new(),
                "{date}"
            );
        }
    }

    #[test]
    fn numbers_must_be_unique_across_directories() {
        let files = [
            valid(Active, 3, &[]),
            named(Superseded, "0003-another.md", 3),
        ];
        let first = adr_path(Active, "0003-decision-3.md");
        let second = &adr_path(Superseded, "0003-another.md");
        assert_eq!(
            problems_in(&files, None),
            sorted(vec![
                problem(second, DuplicateId { number: 3, other: first }),
                problem(second, SupersededButNotListed),
            ])
        );
    }

    #[test]
    fn supersedes_entries_must_name_existing_lower_adrs() {
        let path = &adr_path(Active, "0002-decision-2.md");
        assert_eq!(
            problems_in(&[valid(Active, 2, &[7])], None),
            [problem(path, SupersedesMissing { number: 7 })]
        );
        assert_eq!(
            problems_in(&[valid(Active, 2, &[2])], None),
            sorted(vec![
                problem(path, SupersedesNotLower { number: 2 }),
                problem(path, ActiveButSuperseded { by: 2 }),
            ])
        );
        assert_eq!(
            problems_in(&[valid(Superseded, 3, &[]), valid(Active, 2, &[3])], None),
            [problem(path, SupersedesNotLower { number: 3 })]
        );
    }

    #[test]
    fn superseded_adrs_have_one_successor_and_live_in_superseded() {
        let superseded = &adr_path(Superseded, "0001-decision-1.md");
        let active = &adr_path(Active, "0001-decision-1.md");
        let files = [
            valid(Superseded, 1, &[]),
            valid(Active, 2, &[1]),
            valid(Active, 3, &[1]),
        ];
        assert_eq!(
            problems_in(&files, None),
            [problem(superseded, SupersededByMany { by: vec![2, 3] })]
        );
        assert_eq!(
            problems_in(&[valid(Superseded, 1, &[])], None),
            [problem(superseded, SupersededButNotListed)]
        );
        assert_eq!(
            problems_in(&[valid(Active, 1, &[]), valid(Active, 2, &[1])], None),
            [problem(active, ActiveButSuperseded { by: 2 })]
        );
    }

    #[test]
    fn adrs_dont_need_tags() {
        let files = [valid(Active, 1, &[]), valid(Active, 2, &[])];
        for tags in [
            None,
            Some(""),
            Some("ADR-0001: graph\n"),
            Some("ADR-0001:\n"),
        ] {
            assert_eq!(problems_in(&files, tags), Vec::<Problem>::new(), "{tags:?}");
        }
    }

    #[test]
    fn tags_must_name_existing_adrs() {
        assert_eq!(
            problems_in(
                &[valid(Active, 1, &[])],
                Some("ADR-0001: graph\nADR-0004: graph, wires\n")
            ),
            [problem(TAGS_PATH, TagsForMissingAdr { number: 4 })]
        );
    }

    #[test]
    fn one_run_reports_every_consistency_problem() {
        let files = [
            named(Active, "notes.md", 1),
            with_front_matter(
                Active,
                "0002-decision-2.md",
                "id: ADR-0002\ntitle: \"\"\nsummary: \"S.\"\ndate: soon\n",
            ),
            valid(Active, 3, &[9]),
            valid(Superseded, 4, &[]),
        ];
        let second = &adr_path(Active, "0002-decision-2.md");
        assert_eq!(
            problems_in(&files, Some("ADR-0008: graph\n")),
            sorted(vec![
                problem(&adr_path(Active, "notes.md"), NameMismatch { number: 1 }),
                problem(second, EmptyField { key: "title".into() }),
                problem(second, BadDate { value: "soon".into() }),
                problem(&adr_path(Active, "0003-decision-3.md"), SupersedesMissing { number: 9 }),
                problem(&adr_path(Superseded, "0004-decision-4.md"), SupersededButNotListed),
                problem(TAGS_PATH, TagsForMissingAdr { number: 8 }),
            ])
        );
    }

    #[test]
    fn problems_display_as_messages() {
        let notes = &adr_path(Active, "notes.md");
        let active = &adr_path(Active, "0002-decision-2.md");
        let superseded = &adr_path(Superseded, "0002-decision-2.md");
        assert_eq!(
            problem(notes, NameMismatch { number: 2 }).to_string(),
            "docs/architecture/decisions/active/notes.md: `id` is ADR-0002, so the file name \
             should look like `0002-short-title.md`"
        );
        assert_eq!(
            problem(active, EmptyField { key: "summary".into() }).to_string(),
            "docs/architecture/decisions/active/0002-decision-2.md: `summary` is empty"
        );
        assert_eq!(
            problem(active, UnknownKey { key: "area".into() }).to_string(),
            "docs/architecture/decisions/active/0002-decision-2.md: `area` isn't an ADR front \
             matter field"
        );
        assert_eq!(
            problem(active, ActiveButSuperseded { by: 3 }).to_string(),
            "docs/architecture/decisions/active/0002-decision-2.md: ADR-0003 supersedes this \
             one, so it belongs in `superseded/`"
        );
        assert_eq!(
            problem(superseded, SupersededButNotListed).to_string(),
            "docs/architecture/decisions/superseded/0002-decision-2.md: no ADR supersedes this \
             one, so it belongs in `active/`"
        );
        assert_eq!(
            problem(TAGS_PATH, BadTag { number: 2, tag: "Wires".into() }).to_string(),
            "docs/architecture/decisions/tags.yaml: ADR-0002 has the tag `Wires`; tags are \
             lowercase letters, digits, and hyphens"
        );
    }
}
