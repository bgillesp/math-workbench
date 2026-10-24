//! The ADR checks that `check-docs` runs: the front matter schema, the supersede links between
//! ADRs, and `tags.yaml`.

use std::fmt;

use super::AdrFile;

/// A problem in one file.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Problem {
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
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

/// Checks the ADRs and `tags.yaml` (`None` if it's missing).
pub(super) fn check(_files: &[AdrFile], _tags: Option<&str>) -> Vec<Problem> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adr::ADR_DIR;
    use crate::adr::Dir::{self, Active, Superseded};
    use crate::adr::fixtures::{file, path, tags_for, valid, with_front_matter};

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
}
