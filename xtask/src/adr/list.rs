//! `adr list`: the ADRs by number, or grouped by tag.

use std::fmt;

use super::records::Adr;

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
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

/// Parses the arguments after `adr list`.
pub(super) fn parse_args(_parser: lexopt::Parser) -> Result<Options, lexopt::Error> {
    todo!()
}

/// The `adr list` output for `adrs`.
pub(super) fn render(_adrs: &[Adr], _options: &Options) -> Result<String, UnknownTag> {
    todo!()
}

/// `text` wrapped greedily at `width` columns, each line starting with `indent`.
fn wrap(_text: &str, _width: usize, _indent: &str) -> String {
    todo!()
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
            render(&load(&files, Some(SAMPLE_TAGS)).adrs, &Options::default()),
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
        assert_eq!(wrap("aaaa bbbb cccc ddd eee", 20, "  "), "  aaaa bbbb cccc ddd\n  eee");
        assert_eq!(wrap("a supercalifragilistic b", 12, "  "), "  a\n  supercalifragilistic\n  b");
        assert_eq!(wrap("spaced  out\n  text", SUMMARY_WIDTH, "  "), "  spaced out text");
    }
}
