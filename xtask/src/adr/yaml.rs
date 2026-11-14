//! The flat subset of YAML that ADR front matter and `tags.yaml` use, with rules that make anything
//! it accepts read as the same text in a YAML parser.
//!
//! Each entry is a `key: value` line, and lines indented with spaces continue its value, folding
//! into single spaces. A blank line or a line starting with `#` ends the value and is skipped. A
//! value may be wrapped in double quotes, and means the same either way. No value contains `"` or
//! `\`, so there are no escapes, and an unquoted value that a YAML parser would read differently
//! needs quotes. The rules and their reasons are in `docs/design/0019-simplify-adr-commands.md`.

use std::collections::BTreeMap;
use std::fmt;

/// Something in the text that doesn't read as flat YAML.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Error {
    /// A line, given trimmed, isn't `key: value` or a continuation indented with spaces.
    MalformedLine { line: String },
    /// `key` appears more than once.
    DuplicateKey { key: String },
    /// The value of `key` opens a double quote but doesn't end with one.
    BadClosingQuote { key: String },
    /// The value of `key` contains `"` or `\` once any wrapping quotes are removed.
    QuoteOrBackslash { key: String },
    /// The value of `key` is unquoted, and contains `:` or `#` or doesn't start with a letter or
    /// digit.
    NeedsQuotes { key: String },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedLine { line } => write!(
                f,
                "`{line}` isn't a `key: value` line, or a continuation of one indented with spaces"
            ),
            Self::DuplicateKey { key } => write!(f, "`{key}` appears more than once"),
            Self::BadClosingQuote { key } => write!(
                f,
                "the value of `{key}` opens a double quote but doesn't end with one; a value \
                 continues only on lines indented with spaces"
            ),
            Self::QuoteOrBackslash { key } => {
                write!(f, "the value of `{key}` contains `\"` or `\\`, which no value can")
            },
            Self::NeedsQuotes { key } => write!(
                f,
                "the value of `{key}` needs double quotes, since it contains `:` or `#` or doesn't \
                 start with a letter or digit"
            ),
        }
    }
}

/// Reads each key in `text` with its value, and everything that doesn't read as flat YAML, in
/// order. A value that can't be read is left out, and a repeated key keeps its first value.
pub(super) fn read(text: &str) -> (BTreeMap<String, String>, Vec<Error>) {
    let mut values = BTreeMap::new();
    let mut errors = Vec::new();
    // The entry whose value can still continue: its key, and its lines so far, trimmed.
    let mut open: Option<(String, Vec<&str>)> = None;
    for line in text.lines() {
        let content = line.trim();
        let indent = &line[..line.len() - line.trim_start().len()];
        // A line indented with spaces continues the open value.
        if let Some((_, lines)) = &mut open
            && !content.is_empty()
            && !indent.is_empty()
            && indent.bytes().all(|byte| byte == b' ')
        {
            lines.push(content);
            continue;
        }
        // Any other line ends it.
        if let Some((key, lines)) = open.take() {
            close(key, &lines, &mut values, &mut errors);
        }
        if content.is_empty() || line.starts_with('#') {
            continue;
        }
        match split_entry(line) {
            Some((key, value)) => open = Some((key, vec![value])),
            None => errors.push(Error::MalformedLine { line: content.to_string() }),
        }
    }
    if let Some((key, lines)) = open {
        close(key, &lines, &mut values, &mut errors);
    }
    (values, errors)
}

/// The key and trimmed value of an unindented `key: value` line.
fn split_entry(line: &str) -> Option<(String, &str)> {
    if line.starts_with(char::is_whitespace) {
        return None;
    }
    let (key, value) = line.split_once(':')?;
    let key = key.trim_end();
    if key.is_empty() || !(value.is_empty() || value.starts_with(char::is_whitespace)) {
        return None;
    }
    Some((key.to_string(), value.trim()))
}

/// Adds the value of `key`, folded from its trimmed `lines`, to `values`, or adds the error that
/// keeps it out to `errors`.
fn close(
    key: String,
    lines: &[&str],
    values: &mut BTreeMap<String, String>,
    errors: &mut Vec<Error>,
) {
    // Only the first line can be empty, when the value starts on the line after its key.
    let lines: Vec<&str> = lines.iter().copied().filter(|line| !line.is_empty()).collect();
    let folded = lines.join(" ");
    let unquoted = folded.strip_prefix('"');
    let value = match unquoted {
        Some(rest) => rest.strip_suffix('"'),
        None => Some(folded.as_str()),
    };
    let error = match value {
        _ if values.contains_key(&key) => Error::DuplicateKey { key },
        None => Error::BadClosingQuote { key },
        Some(value) if value.contains(['"', '\\']) => Error::QuoteOrBackslash { key },
        Some(value) if unquoted.is_none() && needs_quotes(value) => Error::NeedsQuotes { key },
        Some(value) => {
            values.insert(key, value.to_string());
            return;
        },
    };
    errors.push(error);
}

/// Whether `value`, written without quotes, would read differently in a YAML parser, as it can when
/// it contains `:` or `#` or starts with something other than a letter or digit.
fn needs_quotes(value: &str) -> bool {
    !value.is_empty() && (value.contains([':', '#']) || !value.starts_with(char::is_alphanumeric))
}

#[cfg(test)]
mod tests {
    use super::Error::{
        BadClosingQuote, DuplicateKey, MalformedLine, NeedsQuotes, QuoteOrBackslash,
    };
    use super::*;

    /// The errors in `text`.
    fn errors(text: &str) -> Vec<Error> {
        read(text).1
    }

    #[test]
    fn values_read_the_same_quoted_or_not() {
        let text = concat!(
            "# A comment, then a blank line.\n",
            "\n",
            "id: ADR-0003\n",
            "title: \"Wire types: checked on connect\"\n",
            "summary: Every wire has a static type,\n",
            "  and connecting two ports\n",
            "    checks it.\n",
            "quoted: \"Every wire has a static type,\n",
            "  and connecting two ports checks it.\"\n",
            "next-line:\n",
            "  Starts on the line after its key.\n",
            "issue: \"Follows up on\n",
            "  #16\"\n",
            "date: \"2026-11-02\"\n",
            "empty:\n",
            "empty-quoted: \"\"\n",
        );
        let values = [
            ("id", "ADR-0003"),
            ("title", "Wire types: checked on connect"),
            ("summary", "Every wire has a static type, and connecting two ports checks it."),
            ("quoted", "Every wire has a static type, and connecting two ports checks it."),
            ("next-line", "Starts on the line after its key."),
            ("issue", "Follows up on #16"),
            ("date", "2026-11-02"),
            ("empty", ""),
            ("empty-quoted", ""),
        ];
        let values = values.map(|(key, value)| (key.to_string(), value.to_string()));
        assert_eq!(read(text), (BTreeMap::from(values), vec![]));
    }

    #[test]
    fn values_a_yaml_parser_would_read_differently_are_rejected() {
        let key = |key: &str| key.to_string();
        let cases = [
            // A YAML parser rejects these, or ends the value at ` #`.
            ("title: Wire types: checked on connect\n", NeedsQuotes { key: key("title") }),
            ("title: Follows up on #16\n", NeedsQuotes { key: key("title") }),
            ("title: `wgpu` for rendering\n", NeedsQuotes { key: key("title") }),
            // An indented `#` line continues the value, so the value needs quotes.
            ("summary: Follows up on\n  #16\n", NeedsQuotes { key: key("summary") }),
            // Quotes wrap the whole value, and nothing inside them can need an escape.
            ("title: \"Never closed\n", BadClosingQuote { key: key("title") }),
            ("title: \"Closed\" early\n", BadClosingQuote { key: key("title") }),
            ("title: \"A \"quoted\" word\"\n", QuoteOrBackslash { key: key("title") }),
            ("title: Back\\slash\n", QuoteOrBackslash { key: key("title") }),
            // YAML 1.2 requires keys to be unique.
            ("title: A\ntitle: B\n", DuplicateKey { key: key("title") }),
        ];
        for (text, error) in cases {
            assert_eq!(errors(text), [error], "{text:?}");
        }
    }

    #[test]
    fn lines_must_be_entries_or_continuations() {
        let malformed = |line: &str| MalformedLine { line: line.to_string() };
        assert_eq!(errors("not an entry\n"), [malformed("not an entry")]);
        // An indented line continues a value that's still open, and only with spaces.
        assert_eq!(errors("  id: ADR-0001\n"), [malformed("id: ADR-0001")]);
        assert_eq!(errors("summary: Every wire\n\tchecks it.\n"), [malformed("checks it.")]);
        // A blank or comment line ends the value, even inside quotes.
        assert_eq!(errors("summary: Every wire\n\n  checks it.\n"), [malformed("checks it.")]);
        assert_eq!(
            errors("summary: Every wire\n# Note\n  checks it.\n"),
            [malformed("checks it.")]
        );
        assert_eq!(
            errors("summary: \"Every wire\n# Note\n  checks it.\"\n"),
            [
                BadClosingQuote { key: "summary".to_string() },
                malformed("checks it.\"")
            ]
        );
    }
}
