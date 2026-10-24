//! `adr print`: ADRs in full, by number.

use std::fmt;

use super::records::Adr;

/// A number given to `adr print` has no ADR.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct NoSuchAdr {
    /// The number that was asked for.
    pub(super) number: u32,
}

impl fmt::Display for NoSuchAdr {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

/// Parses the arguments after `adr print`: one or more ADR numbers.
pub(super) fn parse_args(_parser: lexopt::Parser) -> Result<Vec<u32>, lexopt::Error> {
    todo!()
}

/// The `adr print` output for the ADRs numbered `numbers`, in the order given.
pub(super) fn render(_adrs: &[Adr], _numbers: &[u32]) -> Result<String, NoSuchAdr> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adr::Dir::{Active, Superseded};
    use crate::adr::fixtures::{path, sample, sample_files};

    fn parse(args: &[&str]) -> Result<Vec<u32>, String> {
        parse_args(lexopt::Parser::from_args(args)).map_err(|err| err.to_string())
    }

    #[test]
    fn numbers_parse() {
        assert_eq!(parse(&["3", "0007", "ADR-0012"]), Ok(vec![3, 7, 12]));
        assert_eq!(
            parse(&["x"]),
            Err("`x` isn't an ADR number; use a form like 3, 0003, or ADR-0003".to_string())
        );
        assert_eq!(parse(&[]), Err("`adr print` needs at least one ADR number".to_string()));
    }

    #[test]
    fn files_print_with_their_status() {
        let files = sample_files();
        let header_3 = format!("{} (in force)", path(Active, &files[1].name));
        let header_1 = format!("{} (superseded by ADR-0003)", path(Superseded, &files[2].name));
        assert_eq!(
            render(&sample(), &[3, 1]),
            Ok(format!("{header_3}\n\n{}\n{header_1}\n\n{}", files[1].text, files[2].text))
        );
    }

    #[test]
    fn unknown_numbers_fail() {
        let error = NoSuchAdr { number: 9 };
        assert_eq!(error.to_string(), "there's no ADR-0009");
        assert_eq!(render(&sample(), &[3, 9]), Err(error));
    }
}
