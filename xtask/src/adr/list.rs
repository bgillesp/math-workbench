//! `adr list`, the ADRs by number or grouped by tag, and `adr tags`, every tag on its own line.

use std::collections::BTreeSet;
use std::fmt::Write;

use super::records::{Adr, Records, Tags};
use super::{ids, list_arg};

/// The column summaries are wrapped at with `--verbose`.
const SUMMARY_WIDTH: usize = 80;

/// The heading for the ADRs with no tags under a bare `--tags`. It can't clash with a real tag,
/// which can't contain parentheses.
const NO_TAGS_HEADING: &str = "(no tags)";

/// How `adr list` arranges its entries.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) enum Grouping {
    /// One list, by number.
    #[default]
    Flat,
    /// Grouped under every tag of the ADRs shown, alphabetically, then any with no tags.
    AllTags,
    /// Grouped under the given tags, in the order given.
    Tags(Vec<String>),
}

/// The options of `adr list`.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Options {
    /// Include superseded ADRs.
    pub(super) all: bool,
    /// Whether and how to group the entries by tag.
    pub(super) grouping: Grouping,
    /// Add each ADR's path and summary below its entry.
    pub(super) verbose: bool,
}

/// Parses the arguments after `adr list`.
pub(super) fn parse_args(mut parser: lexopt::Parser) -> Result<Options, lexopt::Error> {
    use lexopt::prelude::*;

    let mut options = Options::default();
    while let Some(arg) = parser.next()? {
        match arg {
            Long("all") => options.all = true,
            Long("verbose") => options.verbose = true,
            Long("tags") => {
                options.grouping = match parser.optional_value() {
                    Some(value) => Grouping::Tags(list_arg("tags", value, "tag")?),
                    None => Grouping::AllTags,
                };
            },
            // `adr list` takes no other values, so this one is meant for `--tags`.
            Value(value) => {
                let value = value.to_string_lossy();
                return Err(format!(
                    "`{value}` looks like a value for `--tags`; attach it as `--tags={value}`"
                )
                .into());
            },
            arg => return Err(arg.unexpected()),
        }
    }
    Ok(options)
}

/// Parses the arguments after `adr tags`, which takes none.
pub(super) fn parse_tags_args(mut parser: lexopt::Parser) -> Result<(), lexopt::Error> {
    match parser.next()? {
        Some(arg) => Err(arg.unexpected()),
        None => Ok(()),
    }
}

/// The `adr list` output for `records`, or an error naming a tag that doesn't exist.
pub(super) fn render(records: &Records, options: &Options) -> Result<String, String> {
    if records.adrs.is_empty() {
        return Ok("None yet.\n".to_string());
    }
    let shown: Vec<&Adr> =
        records.adrs.iter().filter(|adr| options.all || adr.is_active()).collect();
    let entries = |adrs: &[&Adr]| -> String {
        if adrs.is_empty() {
            return "None in force.\n".to_string();
        }
        adrs.iter()
            .map(|adr| entry(adr, &records.successors(adr.number), options.verbose))
            .collect()
    };
    let tags: Vec<&String> = match &options.grouping {
        Grouping::Flat => return Ok(entries(&shown)),
        Grouping::AllTags if shown.is_empty() => return Ok(entries(&shown)),
        Grouping::AllTags => {
            let tags: BTreeSet<&String> = shown.iter().flat_map(|adr| &adr.tags).collect();
            tags.into_iter().collect()
        },
        Grouping::Tags(tags) => {
            let unknown = tags.iter().find(|tag| !records.tags.contains_key(*tag));
            if let Some(tag) = unknown {
                return Err(format!(
                    "there's no tag `{tag}`; `cargo xtask adr tags` lists the tags"
                ));
            }
            tags.iter().collect()
        },
    };
    let mut groups: Vec<String> = tags
        .iter()
        .map(|tag| {
            let tagged: Vec<&Adr> =
                shown.iter().copied().filter(|adr| adr.tags.contains(tag)).collect();
            format!("## {tag}\n\n{}", entries(&tagged))
        })
        .collect();
    if options.grouping == Grouping::AllTags {
        let untagged: Vec<&Adr> = shown.iter().copied().filter(|adr| adr.tags.is_empty()).collect();
        if !untagged.is_empty() {
            groups.push(format!("## {NO_TAGS_HEADING}\n\n{}", entries(&untagged)));
        }
    }
    Ok(groups.join("\n"))
}

/// The `adr tags` output for `tags`.
pub(super) fn render_tags(tags: &Tags) -> String {
    if tags.is_empty() {
        return "None yet.\n".to_string();
    }
    let mut output = String::new();
    for tag in tags.keys() {
        let _ = writeln!(output, "{tag}");
    }
    output
}

/// An ADR's entry in the list, noting its `successors` if it's superseded, with its path and
/// summary below it if `verbose`.
fn entry(adr: &Adr, successors: &[u32], verbose: bool) -> String {
    let mut entry = format!("- ADR-{:04}: {}", adr.number, adr.title);
    if !adr.tags.is_empty() {
        entry = format!("{entry} ({})", adr.tags.join("/"));
    }
    if !adr.is_active() {
        entry.push_str(", superseded");
        if !successors.is_empty() {
            entry = format!("{entry} by {}", ids(successors));
        }
    }
    entry.push('\n');
    if verbose {
        entry = format!("{entry}  {}\n", adr.path());
        if !adr.summary.is_empty() {
            entry.push_str(&wrap(&adr.summary, SUMMARY_WIDTH, "  "));
            entry.push('\n');
        }
    }
    entry
}

/// `text` wrapped greedily into lines of at most `width` columns, each starting with `indent`,
/// which counts toward the width. A word too long to fit gets a line of its own.
fn wrap(text: &str, width: usize, indent: &str) -> String {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
            lines.push(std::mem::take(&mut line));
        }
        if line.is_empty() {
            line.push_str(indent);
        } else {
            line.push(' ');
        }
        line.push_str(word);
    }
    lines.push(line);
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adr::Dir::{Active, Superseded};
    use crate::adr::fixtures::{SAMPLE_TAGS, records_from, sample, sample_files, valid};

    fn parse(args: &[&str]) -> Result<Options, String> {
        parse_args(lexopt::Parser::from_args(args)).map_err(|err| err.to_string())
    }

    /// Options that group the entries under `tags`.
    fn grouped(tags: &[&str]) -> Options {
        Options {
            grouping: Grouping::Tags(tags.iter().map(ToString::to_string).collect()),
            ..Options::default()
        }
    }

    /// The sample ADRs, plus ADR-0005, which is in force and has no tags.
    fn with_untagged() -> Records {
        let mut files = sample_files();
        files.push(valid(Active, 5, &[]));
        records_from(&files, Some(SAMPLE_TAGS))
    }

    #[test]
    fn options_parse() {
        assert_eq!(parse(&[]), Ok(Options::default()));
        assert_eq!(
            parse(&["--all", "--tags", "--verbose"]),
            Ok(Options { all: true, grouping: Grouping::AllTags, verbose: true })
        );
        assert_eq!(parse(&["--tags=wires,graph"]), Ok(grouped(&["wires", "graph"])));
    }

    #[test]
    fn bad_arguments_fail() {
        assert_eq!(
            parse(&["--tags", "graph"]),
            Err("`graph` looks like a value for `--tags`; attach it as `--tags=graph`".to_string())
        );
        assert_eq!(parse(&["--tags="]), Err("`--tags=` needs at least one tag".to_string()));
        assert!(parse(&["--width=100"]).is_err());
    }

    #[test]
    fn list_shows_adrs_in_force_by_number() {
        assert_eq!(
            render(&with_untagged(), &Options::default()),
            Ok("- ADR-0002: Versioned project files (file-format)\n\
                - ADR-0003: Wire types: checked on connect (graph/wires)\n\
                - ADR-0005: Decision 5\n"
                .to_string())
        );
    }

    #[test]
    fn all_adds_superseded_adrs() {
        let options = Options { all: true, ..Options::default() };
        assert_eq!(
            render(&sample(), &options),
            Ok("- ADR-0001: Wires are dynamically typed (dynamic-typing/graph), superseded by \
                ADR-0003\n\
                - ADR-0002: Versioned project files (file-format)\n\
                - ADR-0003: Wire types: checked on connect (graph/wires)\n"
                .to_string())
        );
    }

    #[test]
    fn verbose_adds_paths_and_wrapped_summaries() {
        let options = Options { verbose: true, ..Options::default() };
        assert_eq!(
            render(&sample(), &options),
            Ok("- ADR-0002: Versioned project files (file-format)\n  \
                docs/architecture/decisions/active/0002-versioned-project-files.md\n  \
                Project files carry a format version, and older versions are migrated when\n  \
                loaded.\n\
                - ADR-0003: Wire types: checked on connect (graph/wires)\n  \
                docs/architecture/decisions/active/0003-wire-types-checked-on-connect.md\n  \
                Every wire has a static type, and connecting two ports checks compatibility,\n  \
                so a mistyped connection is rejected before anything is evaluated.\n"
                .to_string())
        );
    }

    #[test]
    fn tags_groups_by_every_tag() {
        let options = Options { grouping: Grouping::AllTags, ..Options::default() };
        assert_eq!(
            render(&with_untagged(), &options),
            Ok("## file-format\n\n\
                - ADR-0002: Versioned project files (file-format)\n\n\
                ## graph\n\n\
                - ADR-0003: Wire types: checked on connect (graph/wires)\n\n\
                ## wires\n\n\
                - ADR-0003: Wire types: checked on connect (graph/wires)\n\n\
                ## (no tags)\n\n\
                - ADR-0005: Decision 5\n"
                .to_string())
        );
    }

    #[test]
    fn tags_groups_by_the_given_tags() {
        assert_eq!(
            render(&with_untagged(), &grouped(&["wires", "graph"])),
            Ok("## wires\n\n\
                - ADR-0003: Wire types: checked on connect (graph/wires)\n\n\
                ## graph\n\n\
                - ADR-0003: Wire types: checked on connect (graph/wires)\n"
                .to_string())
        );
    }

    #[test]
    fn options_combine() {
        let files = vec![valid(Superseded, 1, &[]), valid(Active, 2, &[1])];
        let records = records_from(&files, Some("ADR-0002: graph\n"));
        assert_eq!(
            render(&records, &parse(&["--all", "--tags", "--verbose"]).unwrap()),
            Ok("## graph\n\n\
                - ADR-0002: Decision 2 (graph)\n  \
                docs/architecture/decisions/active/0002-decision-2.md\n  \
                Summary 2.\n\n\
                ## (no tags)\n\n\
                - ADR-0001: Decision 1, superseded by ADR-0002\n  \
                docs/architecture/decisions/superseded/0001-decision-1.md\n  \
                Summary 1.\n"
                .to_string())
        );
        // Without `--all`, the superseded ADR stays out of `(no tags)`.
        assert_eq!(
            render(&records, &parse(&["--tags"]).unwrap()),
            Ok("## graph\n\n- ADR-0002: Decision 2 (graph)\n".to_string())
        );
        assert_eq!(
            render(&sample(), &parse(&["--all", "--tags=dynamic-typing", "--verbose"]).unwrap()),
            Ok("## dynamic-typing\n\n\
                - ADR-0001: Wires are dynamically typed (dynamic-typing/graph), superseded by \
                ADR-0003\n  \
                docs/architecture/decisions/superseded/0001-wires-are-dynamically-typed.md\n  \
                Wires carry values of any type, checked when a tile reads them.\n"
                .to_string())
        );
    }

    #[test]
    fn unknown_tags_point_to_adr_tags() {
        assert_eq!(
            render(&sample(), &grouped(&["graphs"])),
            Err("there's no tag `graphs`; `cargo xtask adr tags` lists the tags".to_string())
        );
    }

    #[test]
    fn says_none_when_nothing_is_shown() {
        let none_in_force = Ok("None in force.\n".to_string());
        let superseded = records_from(&[valid(Superseded, 1, &[])], Some("ADR-0001: graph\n"));
        let by_tag = Options { grouping: Grouping::AllTags, ..Options::default() };
        assert_eq!(render(&Records::default(), &Options::default()), Ok("None yet.\n".to_string()));
        assert_eq!(render(&superseded, &Options::default()), none_in_force);
        assert_eq!(render(&superseded, &by_tag), none_in_force);
        assert_eq!(
            render(&superseded, &grouped(&["graph"])),
            Ok("## graph\n\nNone in force.\n".to_string())
        );
    }

    #[test]
    fn summaries_wrap_greedily() {
        assert_eq!(wrap("Short summary.", SUMMARY_WIDTH, "  "), "  Short summary.");
        // A line of exactly `width` columns fits, and the indent counts toward the width.
        assert_eq!(wrap("aaaa bbbb cccc ddd eee", 20, "  "), "  aaaa bbbb cccc ddd\n  eee");
        assert_eq!(wrap("aaaa bbbb cccc dddd", 20, "  "), "  aaaa bbbb cccc\n  dddd");
        assert_eq!(wrap("a supercalifragilistic b", 12, "  "), "  a\n  supercalifragilistic\n  b");
        assert_eq!(wrap("spaced  out\n  text", SUMMARY_WIDTH, "  "), "  spaced out text");
    }

    #[test]
    fn no_arguments_are_taken_by_adr_tags() {
        assert!(parse_tags_args(lexopt::Parser::from_args(Vec::<&str>::new())).is_ok());
        assert!(parse_tags_args(lexopt::Parser::from_args(["extra"])).is_err());
    }

    #[test]
    fn each_tag_prints_once_on_its_own_line_sorted() {
        assert_eq!(render_tags(&sample().tags), "dynamic-typing\nfile-format\ngraph\nwires\n");
    }

    #[test]
    fn tags_says_none_yet_without_adrs() {
        assert_eq!(render_tags(&Tags::new()), "None yet.\n");
    }
}
