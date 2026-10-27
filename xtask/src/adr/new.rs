//! `adr new`: creates an ADR from the template, records its tags, and moves the ADRs it
//! supersedes to `superseded/`.
//!
//! The ADR is dated with today's local date. The standard library has no time zones, so the date
//! comes from git, which reports the current time and the local UTC offset.

use std::fmt::{self, Write};
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use super::records::{at, read_adr_files, read_optional, tag_entries};
use super::{
    AdrFile, Dir, TAGS_PATH, adr_path, detached_value, file_number, is_tag, list_arg, number_arg,
    parse_id,
};

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

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTitle => write!(f, "the title is empty"),
            Self::TitleQuoteOrBackslash => write!(f, "the title can't contain `\"` or `\\`"),
            Self::NoTags => write!(f, "`adr new` needs at least one tag"),
            Self::BadTag { tag } => {
                write!(f, "`{tag}` isn't a tag; tags are lowercase letters, digits, and hyphens")
            },
            Self::SupersedesMissing { number } => {
                write!(f, "there's no ADR-{number:04} to supersede")
            },
            Self::AlreadySuperseded { number } => {
                write!(f, "ADR-{number:04} is already superseded")
            },
            Self::Io(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

/// Parses the arguments after `adr new`.
pub(super) fn parse_args(mut parser: lexopt::Parser) -> Result<Args, lexopt::Error> {
    use lexopt::prelude::*;

    let mut title = None;
    let mut tags = None;
    let mut supersedes = Vec::new();
    // An option given without a value, which it needs.
    let mut bare: Option<&str> = None;
    while let Some(arg) = parser.next()? {
        if let Some(option) = bare.take() {
            return Err(match arg {
                Value(value) => detached_value(option, &value),
                _ => needs_value(option),
            });
        }
        match arg {
            Long("tags") => match parser.optional_value() {
                Some(value) => tags = Some(list_arg("tags", value, "tag")?),
                None => bare = Some("tags"),
            },
            Long("supersedes") => match parser.optional_value() {
                Some(value) => {
                    for number in list_arg("supersedes", value, "ADR number")? {
                        supersedes.push(number_arg(&number)?);
                    }
                },
                None => bare = Some("supersedes"),
            },
            Value(value) if title.is_none() => title = Some(value.string()?),
            arg => return Err(arg.unexpected()),
        }
    }
    if let Some(option) = bare {
        return Err(needs_value(option));
    }
    let title = title.ok_or("`adr new` needs a title")?;
    let tags = tags.ok_or("`adr new` needs `--tags=<tags>` with at least one tag")?;
    Ok(Args { title, tags, supersedes })
}

/// The error for `--option` given without its value.
fn needs_value(option: &str) -> lexopt::Error {
    format!("`--{option}` needs a value, attached as `--{option}=<value>`").into()
}

/// Runs `adr new`, dated today, and returns what it prints.
pub(super) fn run(root: &Path, args: &Args) -> Result<String, Error> {
    let date = local_today(&[]).unwrap_or_else(|| {
        eprintln!(
            "warning: git couldn't be run to find the local date, so the ADR is dated in UTC"
        );
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |time| time.as_secs());
        local_date(i64::try_from(now).unwrap_or_default(), 0)
    });
    let created = create(root, args, &date)?;
    let mut output = format!("Created {}\n", created.path);
    for number in &args.supersedes {
        let _ = writeln!(output, "Moved ADR-{number:04} to `superseded/`");
    }
    output.push_str("\nNext:\n\n1. Write the summary in the front matter, and the body.");
    if !args.supersedes.is_empty() {
        output.push_str(
            " Restate whatever still holds from the ADRs this one supersedes, so the current \
             decision can be read from one document.",
        );
    }
    output.push_str(
        "\n2. Run `cargo xtask check-docs`.\n3. Open a PR. Merging it accepts the ADR.\n",
    );
    if !created.unused_tags.is_empty() && !created.existing_tags.is_empty() {
        let unused: Vec<String> =
            created.unused_tags.iter().map(|tag| format!("`{tag}`")).collect();
        let _ = writeln!(
            output,
            "\nNo other ADR has {} {}. The existing tags are {}. If one of these was meant, \
             change the entry in `tags.yaml`.",
            if unused.len() == 1 { "the tag" } else { "the tags" },
            unused.join(", "),
            created.existing_tags.join(", "),
        );
    }
    Ok(output)
}

/// Creates the ADR described by `args` under `root`, dated `date`.
pub(super) fn create(root: &Path, args: &Args, date: &str) -> Result<Created, Error> {
    let title = args.title.trim();
    if title.is_empty() {
        return Err(Error::EmptyTitle);
    }
    if title.contains(['"', '\\']) {
        return Err(Error::TitleQuoteOrBackslash);
    }
    if args.tags.is_empty() {
        return Err(Error::NoTags);
    }
    if let Some(tag) = args.tags.iter().find(|tag| !is_tag(tag)) {
        return Err(Error::BadTag { tag: tag.clone() });
    }
    let (files, tags_text) = read_adr_files(root)?;
    let mut supersedes = args.supersedes.clone();
    supersedes.sort_unstable();
    supersedes.dedup();
    let mut moves = Vec::new();
    for &number in &supersedes {
        let file = files
            .iter()
            .find(|file| file_number(&file.name) == Some(number))
            .ok_or(Error::SupersedesMissing { number })?;
        if file.dir == Dir::Superseded {
            return Err(Error::AlreadySuperseded { number });
        }
        moves.push(file);
    }
    let template = read_optional(root, TEMPLATE)?
        .ok_or_else(|| Error::Io(format!("{TEMPLATE}: the ADR template is missing")))?;

    let number = next_number(&files);
    let path = adr_path(Dir::Active, &format!("{number:04}-{}.md", slug(title)));
    let mut tags: Vec<String> = Vec::new();
    for tag in &args.tags {
        if !tags.contains(tag) {
            tags.push(tag.clone());
        }
    }
    let mut existing_tags: Vec<String> =
        tag_entries(tags_text.as_deref()).into_iter().flat_map(|(_, tags)| tags).collect();
    existing_tags.sort();
    existing_tags.dedup();
    let unused_tags = tags.iter().filter(|tag| !existing_tags.contains(tag)).cloned().collect();

    write(root, &path, &adr_text(number, title, date, &supersedes, &template))?;
    write(root, TAGS_PATH, &add_entry(tags_text.as_deref().unwrap_or_default(), number, &tags))?;
    for file in moves {
        let to = adr_path(Dir::Superseded, &file.name);
        fs::create_dir_all(root.join(&to).parent().unwrap_or(root)).map_err(|err| at(&to, &err))?;
        fs::rename(root.join(file.path()), root.join(&to)).map_err(|err| at(&to, &err))?;
    }
    Ok(Created { path, unused_tags, existing_tags })
}

/// Writes `text` to the file at `path` under `root`, creating its directory.
fn write(root: &Path, path: &str, text: &str) -> io::Result<()> {
    let target = root.join(path);
    fs::create_dir_all(target.parent().unwrap_or(root)).map_err(|err| at(path, &err))?;
    fs::write(target, text).map_err(|err| at(path, &err))
}

/// A new ADR's text: its front matter, then the template's body after its instructions comment,
/// with the number and title in the heading.
fn adr_text(number: u32, title: &str, date: &str, supersedes: &[u32], template: &str) -> String {
    let mut text =
        format!("---\nid: ADR-{number:04}\ntitle: \"{title}\"\nsummary: \"\"\ndate: {date}\n");
    if !supersedes.is_empty() {
        let ids: Vec<String> = supersedes.iter().map(|number| format!("ADR-{number:04}")).collect();
        let _ = writeln!(text, "supersedes: {}", ids.join(", "));
    }
    text.push_str("---\n\n");
    let body = match template.trim_start().strip_prefix("<!--") {
        Some(comment) => comment.split_once("-->").map_or("", |(_, body)| body),
        None => template,
    };
    let mut heading_done = false;
    for line in body.trim_start().split_inclusive('\n') {
        if !heading_done && line.starts_with("# ") {
            let _ = writeln!(text, "# ADR-{number:04}: {title}");
            heading_done = true;
        } else {
            text.push_str(line);
        }
    }
    text
}

/// `tags`, the text of `tags.yaml`, with an entry for ADR `number` added before the first entry
/// with a higher number, which keeps the entries sorted.
fn add_entry(tags: &str, number: u32, new_tags: &[String]) -> String {
    let entry = format!("ADR-{number:04}: {}\n", new_tags.join(", "));
    let mut text = String::new();
    let mut added = false;
    for line in tags.split_inclusive('\n') {
        let later =
            line.split_once(':').and_then(|(key, _)| parse_id(key)).is_some_and(|n| n > number);
        if later && !added {
            text.push_str(&entry);
            added = true;
        }
        text.push_str(line);
    }
    if !added {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&entry);
    }
    text
}

/// The file name slug for a title: its ASCII letters, lowercased, and digits, with every other
/// run of characters turned into a single hyphen. A title with none of them gives `adr`.
fn slug(title: &str) -> String {
    let mut slug = String::new();
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_end_matches('-');
    if slug.is_empty() { "adr".to_string() } else { slug.to_string() }
}

/// One more than the highest ADR number, or 1 if there are no ADRs.
fn next_number(files: &[AdrFile]) -> u32 {
    files.iter().filter_map(|file| file_number(&file.name)).max().map_or(1, |number| number + 1)
}

/// The Unix time and the UTC offset in seconds, from the end of `git var GIT_COMMITTER_IDENT`'s
/// output.
fn parse_git_ident(line: &str) -> Option<(i64, i32)> {
    let mut fields = line.split_whitespace().rev();
    let offset = fields.next()?;
    let time = fields.next()?.parse().ok()?;
    let (sign, digits) = match offset.split_at_checked(1)? {
        ("+", digits) => (1, digits),
        ("-", digits) => (-1, digits),
        _ => return None,
    };
    if digits.len() != 4 || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let hours: i32 = digits[..2].parse().ok()?;
    let minutes: i32 = digits[2..].parse().ok()?;
    Some((time, sign * (hours * 3600 + minutes * 60)))
}

/// The `YYYY-MM-DD` date at `unix_time`, in the zone `offset` seconds from UTC.
fn local_date(unix_time: i64, offset: i32) -> String {
    // Howard Hinnant's `civil_from_days`, over 400-year eras that start on March 1.
    let days = (unix_time + i64::from(offset)).div_euclid(86_400) + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
    let month = if month_from_march < 10 { month_from_march + 3 } else { month_from_march - 9 };
    let year = era * 400 + year_of_era + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Today's local date from git, with `env` added to git's environment, or `None` if git can't
/// run.
fn local_today(env: &[(&str, &str)]) -> Option<String> {
    // `git var` fails without an identity, and only the time and offset at the end are read.
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=xtask",
            "-c",
            "user.email=xtask",
            "var",
            "GIT_COMMITTER_IDENT",
        ])
        .envs(env.iter().copied())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let (time, offset) = parse_git_ident(String::from_utf8(output.stdout).ok()?.trim())?;
    Some(local_date(time, offset))
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
