//! `adr tags`: every tag, one per line.

use std::fmt::Write;

use super::records::Tags;

/// Parses the arguments after `adr tags`, which takes none.
pub(super) fn parse_args(mut parser: lexopt::Parser) -> Result<(), lexopt::Error> {
    match parser.next()? {
        Some(arg) => Err(arg.unexpected()),
        None => Ok(()),
    }
}

/// The `adr tags` output for `tags`.
pub(super) fn render(tags: &Tags) -> String {
    if tags.is_empty() {
        return "None yet.\n".to_string();
    }
    let mut output = String::new();
    for tag in tags.keys() {
        let _ = writeln!(output, "{tag}");
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
    fn each_tag_prints_once_on_its_own_line_sorted() {
        assert_eq!(render(&sample().tags), "dynamic-typing\nfile-format\ngraph\nwires\n");
    }

    #[test]
    fn tags_says_none_yet_without_adrs() {
        assert_eq!(render(&Tags::new()), "None yet.\n");
    }
}
