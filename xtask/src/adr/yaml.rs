//! The flat subset of YAML that ADR front matter and `tags.yaml` use.
//!
//! Each entry is a `key: value` line. A double-quoted value may continue on indented lines, and
//! the line breaks fold into single spaces, as in YAML. An unindented line ends the value, so a
//! missing closing quote is reported there rather than swallowing the lines after it. Quoted
//! values can't contain `"` or `\`, so there are no escapes to handle.

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

impl Value {
    /// The value's text, without quotes.
    pub(super) fn text(&self) -> &str {
        match self {
            Self::Plain(text) | Self::Quoted(text) => text,
        }
    }
}

/// A line that couldn't be read. Entries with a bad quoted value are left out.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    /// A line isn't `key: value` or the continuation of a quoted value.
    MalformedLine { line: String },
    /// The quoted value for `key` has no closing quote.
    UnterminatedQuote { key: String },
    /// The quoted value for `key` contains `"` or `\`, or has text after its closing quote.
    QuoteOrBackslash { key: String },
}

/// Reads every entry in `text`, and every line that couldn't be read, in order.
pub(super) fn read(text: &str) -> (Vec<Entry>, Vec<Error>) {
    let mut entries = Vec::new();
    let mut errors = Vec::new();
    // The key of a quoted value whose closing quote hasn't been reached, and its lines so far.
    let mut open: Option<(String, Vec<String>)> = None;
    for line in text.lines() {
        if let Some((key, mut parts)) = open.take() {
            if line.starts_with([' ', '\t']) && !line.trim().is_empty() {
                let line = line.trim();
                if let Some((last, rest)) = line.split_once('"') {
                    parts.push(last.trim_end().to_string());
                    push_quoted(key, &parts, rest, &mut entries, &mut errors);
                } else {
                    parts.push(line.to_string());
                    open = Some((key, parts));
                }
                continue;
            }
            errors.push(Error::UnterminatedQuote { key });
        }
        if line.trim().is_empty() {
            continue;
        }
        let Some((key, value)) = split_entry(line) else {
            errors.push(Error::MalformedLine { line: line.trim().to_string() });
            continue;
        };
        let Some(quoted) = value.strip_prefix('"') else {
            entries.push(Entry { key, value: Value::Plain(value.to_string()) });
            continue;
        };
        match quoted.split_once('"') {
            Some((content, rest)) => {
                push_quoted(key, &[content.to_string()], rest, &mut entries, &mut errors);
            },
            None => open = Some((key, vec![quoted.trim_end().to_string()])),
        }
    }
    if let Some((key, _)) = open {
        errors.push(Error::UnterminatedQuote { key });
    }
    (entries, errors)
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

/// Adds the quoted value made of `parts`, whose closing quote is followed by `rest`, or the error
/// that keeps it out.
fn push_quoted(
    key: String,
    parts: &[String],
    rest: &str,
    entries: &mut Vec<Entry>,
    errors: &mut Vec<Error>,
) {
    if !rest.trim().is_empty() || parts.iter().any(|part| part.contains('\\')) {
        errors.push(Error::QuoteOrBackslash { key });
        return;
    }
    let parts: Vec<&str> =
        parts.iter().map(String::as_str).filter(|part| !part.is_empty()).collect();
    entries.push(Entry { key, value: Value::Quoted(parts.join(" ")) });
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
