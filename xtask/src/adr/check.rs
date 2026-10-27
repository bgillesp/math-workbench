//! The ADR checks that `check-docs` runs: the front matter schema, the supersede links between
//! ADRs, and `tags.yaml`.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt;

use super::records::{Fields, read_front_matter};
use super::{AdrFile, Dir, TAGS_PATH, file_number, is_tag, parse_id, split_tags, yaml};

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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadName => write!(f, "ADR file names look like `0003-short-title.md`"),
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
                write!(f, "the quoted value of `{key}` contains `\"` or `\\`, which can't be used")
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
            Self::TagsMissing { number } => {
                write!(f, "ADR-{number:04} has no entry; give it at least one tag")
            },
            Self::TagsForMissingAdr { number } => {
                write!(f, "there's an entry for ADR-{number:04}, which doesn't exist")
            },
            Self::DuplicateTagsEntry { number } => {
                write!(f, "ADR-{number:04} has more than one entry")
            },
            Self::NoTags { number } => {
                write!(f, "ADR-{number:04} has no tags; give it at least one")
            },
            Self::BadTag { number, tag } => write!(
                f,
                "ADR-{number:04} has the tag `{tag}`; tags are lowercase letters, digits, and \
                 hyphens"
            ),
        }
    }
}

/// ADR ids for `numbers`, separated by commas.
fn ids(numbers: &[u32]) -> String {
    let ids: Vec<String> = numbers.iter().map(|number| format!("ADR-{number:04}")).collect();
    ids.join(", ")
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

/// Checks the ADRs and `tags.yaml` (`None` if it's missing).
pub(super) fn check(files: &[AdrFile], tags: Option<&str>) -> Vec<Problem> {
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

/// Checks that `tags.yaml` has exactly one entry for each ADR, and none for an ADR that doesn't
/// exist, and that each entry has at least one well-formed tag.
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
        } else if entry.value.text().is_empty() {
            kinds.push(ProblemKind::NoTags { number });
        } else {
            for tag in split_tags(entry.value.text()).filter(|tag| !is_tag(tag)) {
                kinds.push(ProblemKind::BadTag { number, tag: tag.to_string() });
            }
        }
    }
    for &number in numbers.difference(&entered) {
        kinds.push(ProblemKind::TagsMissing { number });
    }
    kinds.into_iter().map(|kind| Problem { path: TAGS_PATH.to_string(), kind }).collect()
}

#[cfg(test)]
mod tests {
    use super::ProblemKind::{
        ActiveButSuperseded, BadDate, BadId, BadName, BadSupersedesEntry, BadTag, DuplicateId,
        DuplicateKey, DuplicateTagsEntry, EmptyField, IdMismatch, MalformedLine, MissingField,
        NoTags, NotQuoted, QuoteOrBackslash, SummaryTooLong, SupersededButNotListed,
        SupersededByMany, SupersedesMissing, SupersedesNotLower, TagsForMissingAdr,
        TagsMalformedLine, TagsMissing, UnknownKey, UnterminatedQuote,
    };
    use super::*;
    use crate::adr::Dir::{Active, Superseded};
    use crate::adr::fixtures::{file, tags_for, valid, with_front_matter};
    use crate::adr::{TAGS_PATH, adr_path};

    fn problem(path: &str, kind: ProblemKind) -> Problem {
        Problem { path: path.to_string(), kind }
    }

    /// Problems in a fixed order, so tests don't depend on the order checks run in.
    fn sorted(mut problems: Vec<Problem>) -> Vec<Problem> {
        problems.sort();
        problems
    }

    fn key(name: &str) -> String {
        name.to_string()
    }

    #[test]
    fn empty_directories_pass() {
        assert_eq!(check(&[], None), Vec::<Problem>::new());
        assert_eq!(check(&[], Some("")), Vec::<Problem>::new());
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
        let notes = &adr_path(Active, "notes.md");
        let no_slug = &adr_path(Active, "0004.md");
        let unpadded = &adr_path(Superseded, "4-short.md");
        assert_eq!(
            sorted(check(&files, Some(&tags_for(&[1])))),
            sorted(vec![
                problem(notes, BadName),
                problem(no_slug, BadName),
                problem(unpadded, BadName),
            ])
        );
    }

    #[test]
    fn front_matter_fields_are_checked() {
        let name = "0001-decision.md";
        let path = &adr_path(Active, name);
        let check_one = |front_matter: &str| {
            check(&[with_front_matter(Active, name, front_matter)], Some(&tags_for(&[1])))
        };
        let fields = "title: \"T\"\nsummary: \"S.\"\ndate: 2026-11-02\n";
        assert_eq!(check_one(fields), [problem(path, MissingField { key: key("id") })]);
        assert_eq!(
            check_one(&format!("id: ADR-0001\n{fields}summary: \"Again.\"\n")),
            [problem(path, DuplicateKey { key: key("summary") })]
        );
        assert_eq!(
            check_one(&format!("id: ADR-0001\n{fields}area: graph\n")),
            [problem(path, UnknownKey { key: key("area") })]
        );
        assert_eq!(
            check_one("id: ADR-0001\ntitle: \"T\"\nsummary: \"\"\ndate: 2026-11-02\n"),
            [problem(path, EmptyField { key: key("summary") })]
        );
        assert_eq!(
            check_one("id: ADR-0001\ntitle: \"T\"\nsummary: \"S.\"\ndate:\n"),
            [problem(path, EmptyField { key: key("date") })]
        );
        assert_eq!(
            check_one(&format!("id: ADR-0001\n{fields}not a field\n")),
            [problem(path, MalformedLine { line: "not a field".into() })]
        );
        assert_eq!(
            check_one(&format!("id: 1\n{fields}")),
            [problem(path, BadId { value: "1".into() })]
        );
        for date in ["2026-11-2", "2026-13-01", "2026-11-32", "November 2, 2026"] {
            assert_eq!(
                check_one(&format!("id: ADR-0001\ntitle: \"T\"\nsummary: \"S.\"\ndate: {date}\n")),
                [problem(path, BadDate { value: date.into() })],
                "{date}"
            );
        }
    }

    #[test]
    fn quoting_is_checked() {
        let name = "0001-decision.md";
        let path = &adr_path(Active, name);
        let check_one = |title: &str, summary: &str| {
            let front_matter =
                format!("id: ADR-0001\ntitle: {title}\nsummary: {summary}\ndate: 2026-11-02\n");
            check(&[with_front_matter(Active, name, &front_matter)], Some(&tags_for(&[1])))
        };
        assert_eq!(
            check_one("Unquoted", "\"S.\""),
            [problem(path, NotQuoted { key: key("title") })]
        );
        assert_eq!(
            check_one("\"T\"", "Unquoted."),
            [problem(path, NotQuoted { key: key("summary") })]
        );
        assert_eq!(
            check_one("\"T\"", "\"A \"quoted\" word.\""),
            [problem(path, QuoteOrBackslash { key: key("summary") })]
        );
        assert_eq!(
            check_one("\"Back\\slash\"", "\"S.\""),
            [problem(path, QuoteOrBackslash { key: key("title") })]
        );
        assert_eq!(
            check_one("\"T\"", "\"Never closed."),
            [problem(path, UnterminatedQuote { key: key("summary") })]
        );
        let long = format!("\"{}\"", "x".repeat(301));
        assert_eq!(check_one("\"T\"", &long), [problem(path, SummaryTooLong { chars: 301 })]);
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
        let path = &adr_path(Active, "0002-decision.md");
        assert_eq!(
            check(&[mismatched], Some(&tags_for(&[2]))),
            [problem(path, IdMismatch { id: 3 })]
        );

        let first = valid(Active, 3, &[]);
        let mut second = valid(Superseded, 3, &[]);
        second.name = "0003-another.md".into();
        let first_path = adr_path(Active, "0003-decision-3.md");
        let second_path = &adr_path(Superseded, "0003-another.md");
        assert_eq!(
            sorted(check(&[first, second], Some(&tags_for(&[3])))),
            sorted(vec![
                problem(second_path, DuplicateId { number: 3, other: first_path }),
                problem(second_path, SupersededButNotListed),
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
        let path = &adr_path(Active, "0002-decision-2.md");
        assert_eq!(
            check(&[valid(Active, 1, &[]), bad_entry], Some(&tags_for(&[1, 2]))),
            [problem(path, BadSupersedesEntry { entry: "1".into() })]
        );
        assert_eq!(
            check(&[valid(Active, 2, &[7])], Some(&tags_for(&[2]))),
            [problem(path, SupersedesMissing { number: 7 })]
        );
        assert_eq!(
            sorted(check(&[valid(Active, 2, &[2])], Some(&tags_for(&[2])))),
            sorted(vec![
                problem(path, SupersedesNotLower { number: 2 }),
                problem(path, ActiveButSuperseded { by: 2 }),
            ])
        );
        assert_eq!(
            check(&[valid(Superseded, 3, &[]), valid(Active, 2, &[3])], Some(&tags_for(&[2, 3]))),
            [problem(path, SupersedesNotLower { number: 3 })]
        );
    }

    #[test]
    fn superseded_adrs_have_one_successor_and_live_in_superseded() {
        let files = [
            valid(Superseded, 1, &[]),
            valid(Active, 2, &[1]),
            valid(Active, 3, &[1]),
        ];
        let superseded = &adr_path(Superseded, "0001-decision-1.md");
        let active = &adr_path(Active, "0001-decision-1.md");
        assert_eq!(
            check(&files, Some(&tags_for(&[1, 2, 3]))),
            [problem(superseded, SupersededByMany { by: vec![2, 3] })]
        );
        assert_eq!(
            check(&[valid(Superseded, 1, &[])], Some(&tags_for(&[1]))),
            [problem(superseded, SupersededButNotListed)]
        );
        assert_eq!(
            check(&[valid(Active, 1, &[]), valid(Active, 2, &[1])], Some(&tags_for(&[1, 2]))),
            [problem(active, ActiveButSuperseded { by: 2 })]
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
                problem(TAGS_PATH, DuplicateTagsEntry { number: 1 }),
                problem(TAGS_PATH, TagsMissing { number: 2 }),
                problem(TAGS_PATH, NoTags { number: 3 }),
                problem(TAGS_PATH, TagsForMissingAdr { number: 4 }),
            ])
        );
        assert_eq!(
            check(&files[..1], Some("ADR-0001: graph, Wires, two words\n")),
            [
                problem(TAGS_PATH, BadTag { number: 1, tag: "Wires".into() }),
                problem(TAGS_PATH, BadTag { number: 1, tag: "two words".into() }),
            ]
        );
        assert_eq!(
            check(&files[..1], Some("ADR-0001: graph\ngraph: ADR-0001\n")),
            [problem(
                TAGS_PATH,
                TagsMalformedLine { line: "graph: ADR-0001".into() }
            )]
        );
        assert_eq!(
            sorted(check(&files[..2], None)),
            sorted(vec![
                problem(TAGS_PATH, TagsMissing { number: 1 }),
                problem(TAGS_PATH, TagsMissing { number: 2 }),
            ])
        );
    }

    #[test]
    fn problems_display_as_messages() {
        let notes = &adr_path(Active, "notes.md");
        let active = &adr_path(Active, "0002-decision-2.md");
        let superseded = &adr_path(Superseded, "0002-decision-2.md");
        assert_eq!(
            problem(notes, BadName).to_string(),
            "docs/architecture/decisions/active/notes.md: ADR file names look like \
             `0003-short-title.md`"
        );
        assert_eq!(
            problem(active, EmptyField { key: key("summary") }).to_string(),
            "docs/architecture/decisions/active/0002-decision-2.md: `summary` is empty"
        );
        assert_eq!(
            problem(active, UnknownKey { key: key("area") }).to_string(),
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
            problem(TAGS_PATH, TagsMissing { number: 2 }).to_string(),
            "docs/architecture/decisions/tags.yaml: ADR-0002 has no entry; give it at least one \
             tag"
        );
        assert_eq!(
            problem(TAGS_PATH, BadTag { number: 2, tag: "Wires".into() }).to_string(),
            "docs/architecture/decisions/tags.yaml: ADR-0002 has the tag `Wires`; tags are \
             lowercase letters, digits, and hyphens"
        );
    }
}
