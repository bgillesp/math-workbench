//! `adr tags`: each tag, with the number of ADRs in force that use it.

use super::records::Adr;

/// Parses the arguments after `adr tags`, which takes none.
pub(super) fn parse_args(_parser: lexopt::Parser) -> Result<(), lexopt::Error> {
    todo!()
}

/// The `adr tags` output for `adrs`.
pub(super) fn render(_adrs: &[Adr]) -> String {
    todo!()
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
