//! The flat subset of YAML that ADR front matter and `tags.yaml` use.
//!
//! Each entry is a `key: value` line. A double-quoted value may continue on indented lines, and
//! the line breaks fold into single spaces, as in YAML. Quoted values can't contain `"` or `\`,
//! so there are no escapes to handle.

/// A `key: value` entry, with any continuation lines folded in.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Entry {
    /// The text before the colon.
    pub(super) key: String,
    /// The text after the colon.
    pub(super) value: Value,
}

/// An entry's value.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Value {
    /// A value written without quotes, trimmed.
    Plain(String),
    /// A double-quoted value, without its quotes.
    Quoted(String),
}

/// A line that couldn't be read. Entries with a bad quoted value are left out.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    /// A line isn't `key: value` or the continuation of a quoted value.
    MalformedLine { line: String },
    /// The quoted value for `key` has no closing quote.
    UnterminatedQuote { key: String },
    /// The quoted value for `key` contains `"` or `\`.
    QuoteOrBackslash { key: String },
}

/// Reads every entry in `text`, and every line that couldn't be read, in order.
pub(super) fn read(_text: &str) -> (Vec<Entry>, Vec<Error>) {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: &str, value: Value) -> Entry {
        Entry { key: key.to_string(), value }
    }

    fn plain(value: &str) -> Value {
        Value::Plain(value.to_string())
    }

    fn quoted(value: &str) -> Value {
        Value::Quoted(value.to_string())
    }

    #[test]
    fn entries_have_plain_or_quoted_values() {
        assert_eq!(
            read("id: ADR-0003\ntitle: \"Wire types: checked on connect\"\ndate:\n"),
            (
                vec![
                    entry("id", plain("ADR-0003")),
                    entry("title", quoted("Wire types: checked on connect")),
                    entry("date", plain("")),
                ],
                vec![]
            )
        );
        assert_eq!(read(""), (vec![], vec![]));
    }

    #[test]
    fn quoted_values_fold_continuation_lines() {
        assert_eq!(
            read(
                "summary: \"Every wire has a\n  static type,\n    and connecting checks it.\"\nnext: x\n"
            ),
            (
                vec![
                    entry(
                        "summary",
                        quoted("Every wire has a static type, and connecting checks it.")
                    ),
                    entry("next", plain("x")),
                ],
                vec![]
            )
        );
    }

    #[test]
    fn bad_lines_are_reported_in_order() {
        let text = "not a field\nsummary: \"never closed\ntitle: \"a \"b\" c\"\n\
                    path: \"back\\slash\"\nok: yes\n";
        assert_eq!(
            read(text),
            (
                vec![entry("ok", plain("yes"))],
                vec![
                    Error::MalformedLine { line: "not a field".into() },
                    Error::UnterminatedQuote { key: "summary".into() },
                    Error::QuoteOrBackslash { key: "title".into() },
                    Error::QuoteOrBackslash { key: "path".into() },
                ]
            )
        );
    }
}
