//! `adr tags`: each tag, with the number of ADRs in force that use it.

use std::collections::BTreeMap;
use std::fmt::Write;

use super::records::Adr;

/// Parses the arguments after `adr tags`, which takes none.
pub(super) fn parse_args(mut parser: lexopt::Parser) -> Result<(), lexopt::Error> {
    match parser.next()? {
        Some(arg) => Err(arg.unexpected()),
        None => Ok(()),
    }
}

/// The `adr tags` output for `adrs`.
pub(super) fn render(adrs: &[Adr]) -> String {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for adr in adrs {
        for tag in &adr.tags {
            *counts.entry(tag).or_default() += usize::from(adr.is_active());
        }
    }
    if counts.is_empty() {
        return "None yet.\n".to_string();
    }
    let mut output = String::new();
    for (tag, count) in counts {
        let _ = writeln!(output, "- {tag} ({count})");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adr::fixtures::sample;

    #[test]
    fn no_arguments_are_taken() {
        assert!(parse_args(lexopt::Parser::from_args(Vec::<&str>::new())).is_ok());
        assert!(parse_args(lexopt::Parser::from_args(["extra"])).is_err());
    }

    #[test]
    fn each_tag_shows_its_adrs_in_force() {
        assert_eq!(
            render(&sample()),
            "- dynamic-typing (0)\n- file-format (1)\n- graph (1)\n- wires (1)\n"
        );
    }

    #[test]
    fn tags_says_none_yet_without_adrs() {
        assert_eq!(render(&[]), "None yet.\n");
    }
}
