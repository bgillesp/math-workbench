//! `adr list`: the ADRs by number, or grouped by tag.

use std::collections::BTreeSet;
use std::fmt;

use super::records::Adr;
use super::{detached_value, list_arg};

/// The column summaries are wrapped at with `--verbose`.
const SUMMARY_WIDTH: usize = 80;

/// How `adr list` arranges its entries.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) enum Grouping {
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
pub(super) struct Options {
    /// Include superseded ADRs.
    pub(super) all: bool,
    /// Whether and how to group the entries by tag.
    pub(super) grouping: Grouping,
    /// Add each ADR's summary below its entry.
    pub(super) verbose: bool,
}

/// `--tags=` named a tag that no ADR has.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct UnknownTag {
    /// The tag that was asked for.
    pub(super) tag: String,
    /// Every tag the ADRs use, sorted.
    pub(super) existing: Vec<String>,
}

impl fmt::Display for UnknownTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.existing.is_empty() {
            return write!(f, "no ADR has the tag `{}`, or any tags yet", self.tag);
        }
        write!(f, "no ADR has the tag `{}`; the tags are {}", self.tag, self.existing.join(", "))
    }
}

impl std::error::Error for UnknownTag {}

/// Parses the arguments after `adr list`.
pub(super) fn parse_args(mut parser: lexopt::Parser) -> Result<Options, lexopt::Error> {
    use lexopt::prelude::*;

    let mut options = Options::default();
    // Whether the previous argument was `--tags` without a value.
    let mut bare_tags = false;
    while let Some(arg) = parser.next()? {
        let after_bare_tags = std::mem::take(&mut bare_tags);
        match arg {
            Long("all") => options.all = true,
            Long("verbose") => options.verbose = true,
            Long("tags") => {
                if let Some(value) = parser.optional_value() {
                    options.grouping = Grouping::Tags(list_arg("tags", value, "tag")?);
                } else {
                    options.grouping = Grouping::AllTags;
                    bare_tags = true;
                }
            },
            Value(value) if after_bare_tags => return Err(detached_value("tags", &value)),
            arg => return Err(arg.unexpected()),
        }
    }
    Ok(options)
}

/// The `adr list` output for `adrs`.
pub(super) fn render(adrs: &[Adr], options: &Options) -> Result<String, UnknownTag> {
    if adrs.is_empty() {
        return Ok("None yet.\n".to_string());
    }
    let shown: Vec<&Adr> = adrs.iter().filter(|adr| options.all || adr.is_active()).collect();
    let entries = |adrs: &[&Adr]| -> String {
        if adrs.is_empty() {
            return "None in force.\n".to_string();
        }
        adrs.iter().map(|adr| entry(adr, options.verbose)).collect()
    };
    let tags: Vec<String> = match &options.grouping {
        Grouping::Flat => return Ok(entries(&shown)),
        Grouping::AllTags => tags_of(&shown),
        Grouping::Tags(tags) => {
            let existing = tags_of(&adrs.iter().collect::<Vec<_>>());
            if let Some(tag) = tags.iter().find(|tag| !existing.contains(tag)) {
                return Err(UnknownTag { tag: tag.clone(), existing });
            }
            tags.clone()
        },
    };
    let groups: Vec<String> = tags
        .iter()
        .map(|tag| {
            let tagged: Vec<&Adr> =
                shown.iter().copied().filter(|adr| adr.tags.contains(tag)).collect();
            format!("## {tag}\n\n{}", entries(&tagged))
        })
        .collect();
    Ok(groups.join("\n"))
}

/// Every tag that `adrs` use, sorted.
fn tags_of(adrs: &[&Adr]) -> Vec<String> {
    let tags: BTreeSet<&String> = adrs.iter().flat_map(|adr| &adr.tags).collect();
    tags.into_iter().cloned().collect()
}

/// An ADR's entry in the list, with its summary below it if `verbose`.
fn entry(adr: &Adr, verbose: bool) -> String {
    let mut entry = format!("- ADR-{:04}: {}", adr.front_matter.number, adr.front_matter.title);
    if !adr.tags.is_empty() {
        entry = format!("{entry} ({})", adr.tags.join("/"));
    }
    if !adr.is_active() {
        entry = format!("{entry}, {}", adr.status());
    }
    entry.push('\n');
    if verbose {
        entry.push_str(&wrap(&adr.front_matter.summary, SUMMARY_WIDTH, "  "));
        entry.push('\n');
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
    use crate::adr::Dir::Active;
    use crate::adr::fixtures::{SAMPLE_TAGS, sample, sample_files, valid};
    use crate::adr::records::load;

    fn parse(args: &[&str]) -> Result<Options, String> {
        parse_args(lexopt::Parser::from_args(args)).map_err(|err| err.to_string())
    }

    #[test]
    fn options_parse() {
        assert_eq!(parse(&[]), Ok(Options::default()));
        assert_eq!(
            parse(&["--all", "--tags", "--verbose"]),
            Ok(Options { all: true, grouping: Grouping::AllTags, verbose: true })
        );
        assert_eq!(
            parse(&["--tags=wires,graph"]),
            Ok(Options {
                grouping: Grouping::Tags(vec!["wires".into(), "graph".into()]),
                ..Options::default()
            })
        );
    }

    #[test]
    fn bad_options_fail() {
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
            render(&sample(), &Options::default()),
            Ok("- ADR-0002: Versioned project files (file-format)\n\
                - ADR-0003: Wire types: checked on connect (graph/wires)\n"
                .to_string())
        );
    }

    #[test]
    fn all_adds_superseded_adrs() {
        let options = Options { all: true, ..Options::default() };
        assert_eq!(
            render(&sample(), &options),
            Ok("- ADR-0001: Wires are dynamically typed (graph/dynamic-typing), superseded by \
                ADR-0003\n\
                - ADR-0002: Versioned project files (file-format)\n\
                - ADR-0003: Wire types: checked on connect (graph/wires)\n"
                .to_string())
        );
    }

    #[test]
    fn untagged_adrs_are_listed_without_tags() {
        let mut files = sample_files();
        files.push(valid(Active, 5, &[]));
        assert_eq!(
            render(&load(files, Some(SAMPLE_TAGS)).adrs, &Options::default()),
            Ok("- ADR-0002: Versioned project files (file-format)\n\
                - ADR-0003: Wire types: checked on connect (graph/wires)\n\
                - ADR-0005: Decision 5\n"
                .to_string())
        );
    }

    #[test]
    fn tags_groups_by_every_tag() {
        let options = Options { grouping: Grouping::AllTags, ..Options::default() };
        assert_eq!(
            render(&sample(), &options),
            Ok("## file-format\n\n\
                - ADR-0002: Versioned project files (file-format)\n\n\
                ## graph\n\n\
                - ADR-0003: Wire types: checked on connect (graph/wires)\n\n\
                ## wires\n\n\
                - ADR-0003: Wire types: checked on connect (graph/wires)\n"
                .to_string())
        );

        let options = Options { all: true, grouping: Grouping::AllTags, verbose: false };
        assert_eq!(
            render(&sample(), &options),
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
    fn tags_with_values_groups_by_those_tags_in_order() {
        let options = Options {
            grouping: Grouping::Tags(vec!["wires".into(), "file-format".into()]),
            ..Options::default()
        };
        assert_eq!(
            render(&sample(), &options),
            Ok("## wires\n\n\
                - ADR-0003: Wire types: checked on connect (graph/wires)\n\n\
                ## file-format\n\n\
                - ADR-0002: Versioned project files (file-format)\n"
                .to_string())
        );
    }

    #[test]
    fn unknown_tags_fail() {
        let options =
            Options { grouping: Grouping::Tags(vec!["graphs".into()]), ..Options::default() };
        let error = UnknownTag {
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
        assert_eq!(render(&sample(), &options), Err(error));
    }

    #[test]
    fn verbose_adds_wrapped_summaries() {
        let options = Options { verbose: true, ..Options::default() };
        assert_eq!(
            render(&sample(), &options),
            Ok("- ADR-0002: Versioned project files (file-format)\n  \
                Project files carry a format version, and older versions are migrated when\n  \
                loaded.\n\
                - ADR-0003: Wire types: checked on connect (graph/wires)\n  \
                Every wire has a static type, and connecting two ports checks compatibility,\n  \
                so a mistyped connection is rejected before anything is evaluated.\n"
                .to_string())
        );

        let options = Options {
            grouping: Grouping::Tags(vec!["file-format".into()]),
            verbose: true,
            ..Options::default()
        };
        assert_eq!(
            render(&sample(), &options),
            Ok("## file-format\n\n\
                - ADR-0002: Versioned project files (file-format)\n  \
                Project files carry a format version, and older versions are migrated when\n  \
                loaded.\n"
                .to_string())
        );
    }

    #[test]
    fn list_says_none_yet_without_adrs() {
        assert_eq!(render(&[], &Options::default()), Ok("None yet.\n".to_string()));
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
}
