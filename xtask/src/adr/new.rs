//! `adr new`: creates an ADR from the template, records its tags, and moves the ADRs it
//! supersedes to `superseded/`.
//!
//! The ADR is dated with today's local date. The standard library has no time zones, so the date
//! comes from git, which reports the current time and the local UTC offset.

use std::path::Path;

use super::AdrFile;

/// The template `adr new` copies the body of an ADR from.
pub(super) const TEMPLATE: &str = "docs/templates/adr.md";

/// The arguments of `adr new`.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Args {
    /// The new ADR's title.
    pub(super) title: String,
    /// The new ADR's tags, in the order given.
    pub(super) tags: Vec<String>,
    /// The numbers of the ADRs the new one supersedes.
    pub(super) supersedes: Vec<u32>,
}

/// What `adr new` created.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Created {
    /// The repository-relative path of the new ADR.
    pub(super) path: String,
    /// The new ADR's tags that no other ADR uses.
    pub(super) unused_tags: Vec<String>,
    /// Every tag the other ADRs use, sorted.
    pub(super) existing_tags: Vec<String>,
}

/// Why `adr new` didn't create an ADR. It changes nothing when it refuses.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
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

/// Parses the arguments after `adr new`.
pub(super) fn parse_args(_parser: lexopt::Parser) -> Result<Args, lexopt::Error> {
    todo!()
}

/// Creates the ADR described by `args` under `root`, dated `date`.
pub(super) fn create(_root: &Path, _args: &Args, _date: &str) -> Result<Created, Error> {
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
    use super::*;
    use crate::adr::Dir::{Active, Superseded};
    use crate::adr::check::{Problem, ProblemKind, check};
    use crate::adr::fixtures::{TEMPLATE_TEXT, TempRoot, file, new_args, repository, valid};
    use crate::adr::records::read_adr_files;
    use crate::adr::{ADR_DIR, adr_path};

    fn parse(args: &[&str]) -> Result<Args, String> {
        parse_args(lexopt::Parser::from_args(args)).map_err(|err| err.to_string())
    }

    // Arguments

    #[test]
    fn arguments_parse() {
        assert_eq!(
            parse(&[
                "Wire types: checked on connect",
                "--tags=graph,wires",
                "--supersedes=1,ADR-0002"
            ]),
            Ok(new_args("Wire types: checked on connect", &["graph", "wires"], &[1, 2]))
        );
        assert_eq!(
            parse(&["--tags=graph", "Title first or last"]),
            Ok(new_args("Title first or last", &["graph"], &[]))
        );
    }

    #[test]
    fn option_values_must_be_attached() {
        assert_eq!(
            parse(&["Title", "--tags", "graph"]),
            Err("`graph` looks like a value for `--tags`; attach it as `--tags=graph`".to_string())
        );
        assert_eq!(
            parse(&["Title", "--tags=graph", "--supersedes", "1"]),
            Err("`1` looks like a value for `--supersedes`; attach it as `--supersedes=1`"
                .to_string())
        );
    }

    #[test]
    fn bad_arguments_fail() {
        assert_eq!(
            parse(&["Title"]),
            Err("`adr new` needs `--tags=<tags>` with at least one tag".to_string())
        );
        assert_eq!(
            parse(&["Title", "--tags="]),
            Err("`--tags=` needs at least one tag".to_string())
        );
        assert_eq!(
            parse(&["Title", "--tags=graph", "--supersedes=one"]),
            Err("`one` isn't an ADR number; use a form like 3, 0003, or ADR-0003".to_string())
        );
        assert_eq!(parse(&["--tags=graph"]), Err("`adr new` needs a title".to_string()));
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
    fn create_writes_the_adr_and_its_tags_entry() {
        let root = repository("create");
        let created = create(
            &root.0,
            &new_args("Wire types: checked on connect", &["graph", "wires"], &[]),
            "2026-11-02",
        );
        let new_path = adr_path(Active, "0003-wire-types-checked-on-connect.md");
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
    fn create_moves_the_adrs_it_supersedes() {
        let root = repository("supersede");
        let old = root.read(&adr_path(Active, "0001-decision-1.md"));
        let created =
            create(&root.0, &new_args("Replacement", &["graph"], &[1, 2]), "2026-11-02").unwrap();
        assert_eq!(created.unused_tags, Vec::<String>::new());
        assert_eq!(root.read(&adr_path(Superseded, "0001-decision-1.md")), old);
        assert!(!root.0.join(adr_path(Active, "0001-decision-1.md")).exists());
        assert!(root.0.join(adr_path(Superseded, "0002-decision-2.md")).exists());
        assert!(
            root.read(&created.path)
                .contains("\ndate: 2026-11-02\nsupersedes: ADR-0001, ADR-0002\n---\n")
        );
    }

    #[test]
    fn create_leaves_only_the_summary_to_write() {
        let root = repository("then-check");
        let created =
            create(&root.0, &new_args("Replacement", &["graph"], &[1]), "2026-11-02").unwrap();
        let (files, tags) = read_adr_files(&root.0).unwrap();
        assert_eq!(
            check(&files, tags.as_deref()),
            vec![Problem {
                path: created.path,
                kind: ProblemKind::EmptyField { key: "summary".into() },
            }]
        );
    }

    #[test]
    fn create_works_in_an_empty_repository() {
        let root = TempRoot::new("first");
        root.write(TEMPLATE, TEMPLATE_TEXT);
        let created = create(&root.0, &new_args("First", &["graph"], &[]), "2026-11-02");
        assert_eq!(
            created,
            Ok(Created {
                path: adr_path(Active, "0001-first.md"),
                unused_tags: vec!["graph".into()],
                existing_tags: vec![],
            })
        );
        assert_eq!(root.read(&format!("{ADR_DIR}/tags.yaml")), "ADR-0001: graph\n");
    }

    #[test]
    fn create_refuses_bad_arguments_and_changes_nothing() {
        let root = repository("refuse");
        root.write_adr(&valid(Superseded, 3, &[]));
        root.write_adr(&valid(Active, 4, &[3]));
        let before = root.snapshot();
        let cases = [
            (new_args("", &["graph"], &[]), Error::EmptyTitle),
            (new_args("A \"quoted\" title", &["graph"], &[]), Error::TitleQuoteOrBackslash),
            (new_args("Back\\slash", &["graph"], &[]), Error::TitleQuoteOrBackslash),
            (new_args("Title", &[], &[]), Error::NoTags),
            (new_args("Title", &["Graph"], &[]), Error::BadTag { tag: "Graph".into() }),
            (new_args("Title", &["graph"], &[9]), Error::SupersedesMissing { number: 9 }),
            (new_args("Title", &["graph"], &[3]), Error::AlreadySuperseded { number: 3 }),
        ];
        for (args, error) in cases {
            assert_eq!(create(&root.0, &args, "2026-11-02"), Err(error), "{args:?}");
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
