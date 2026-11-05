//! `check-docs`: ADRs, design docs, and relative links in Markdown files are consistent.
//!
//! The ADR checks live in the `adr` module. Design docs carry their status in front matter:
//!
//! ```text
//! ---
//! status: implemented
//! ---
//! ```
//!
//! Only flat `key: value` lines are read.

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use pulldown_cmark::{Event, Options, Parser, Tag};

use crate::{CheckResult, adr, display};

pub(crate) const DESIGN_DIR: &str = "docs/design";

const DESIGN_STATUSES: &[&str] = &["draft", "accepted", "implemented"];

/// A problem in one file.
#[derive(Debug, PartialEq, Eq)]
struct Problem {
    /// The repository-relative path of the file to fix.
    path: String,
    /// What's wrong with it.
    kind: ProblemKind,
}

impl Problem {
    fn new(path: impl Into<String>, kind: ProblemKind) -> Self {
        Self { path: path.into(), kind }
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.kind)
    }
}

/// What's wrong with a file.
#[derive(Debug, PartialEq, Eq)]
enum ProblemKind {
    // Design docs
    /// The file name isn't like `0042-short-title.md`.
    BadDesignName,
    /// Another design doc, named `other`, has the same number.
    DuplicateDesignNumber { number: u32, other: String },
    /// `status` is missing or isn't a design doc status.
    BadDesignStatus,

    // Links
    /// A relative link points to something that doesn't exist.
    BrokenLink { dest: String },
    /// A relative link climbs above the repository root.
    LinkOutsideRepository { dest: String },
}

impl fmt::Display for ProblemKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadDesignName => f.write_str(
                "design doc file names look like `0042-short-title.md`, where 0042 is the number \
                 of the PR that first adds it",
            ),
            Self::DuplicateDesignNumber { number, other } => {
                write!(f, "number {number:04} is also used by {other}")
            },
            Self::BadDesignStatus => f.write_str(
                "front matter needs `status:` set to `draft`, `accepted`, or `implemented`",
            ),
            Self::BrokenLink { dest } => write!(f, "broken link `{dest}`"),
            Self::LinkOutsideRepository { dest } => {
                write!(f, "link `{dest}` points outside the repository")
            },
        }
    }
}

pub(crate) fn run(root: &Path) -> CheckResult {
    let mut problems = Vec::new();

    let design_docs = read_files(&root.join(DESIGN_DIR))?;
    problems.extend(check_design_docs(&design_docs));

    for path in markdown_files(root)? {
        let Ok(text) = fs::read_to_string(root.join(&path)) else {
            continue; // deleted from the working tree but still in the index
        };
        problems.extend(check_links(&path, &text, |target| root.join(target).exists()));
    }
    let mut messages: Vec<String> = problems.iter().map(ToString::to_string).collect();
    messages.extend(adr::check_docs(root)?);
    Ok(messages)
}

/// The name and contents of each file directly inside `dir`.
pub(crate) fn read_files(dir: &Path) -> std::io::Result<Vec<(String, String)>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            let name = entry.file_name().to_string_lossy().into_owned();
            files.push((name, fs::read_to_string(entry.path())?));
        }
    }
    files.sort();
    Ok(files)
}

/// Markdown files that git tracks or would track, so ignored and nested-worktree files are
/// skipped.
fn markdown_files(root: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args([
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
            "*.md",
        ])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err(
            format!("git ls-files failed: {}", String::from_utf8_lossy(&output.stderr)).into()
        );
    }
    let mut paths: Vec<PathBuf> = String::from_utf8(output.stdout)?
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .collect();
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn markdown_options() -> Options {
    Options::ENABLE_TABLES | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
}

// Design docs

/// The number of a document named like `0042-short-title.md`.
fn doc_number(file_name: &str) -> Option<u32> {
    let (digits, slug) = file_name.strip_suffix(".md")?.split_once('-')?;
    let valid_slug = slug.split('-').all(|word| {
        !word.is_empty() && word.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    });
    if digits.len() == 4 && digits.bytes().all(|b| b.is_ascii_digit()) && valid_slug {
        digits.parse().ok()
    } else {
        None
    }
}

/// The `key: value` fields of a leading `---` block, or `None` if there isn't one.
pub(crate) fn front_matter(text: &str) -> Option<HashMap<&str, &str>> {
    let mut lines = text.lines();
    if lines.next()? != "---" {
        return None;
    }
    let mut fields = HashMap::new();
    for line in lines {
        if line == "---" {
            return Some(fields);
        }
        if let Some((key, value)) = line.split_once(':') {
            fields.insert(key.trim(), value.trim());
        }
    }
    None
}

/// Checks design doc file names, numbering, and statuses.
fn check_design_docs(files: &[(String, String)]) -> Vec<Problem> {
    let mut problems = Vec::new();
    let mut seen: HashMap<u32, &str> = HashMap::new();
    for (name, text) in files {
        let path = format!("{DESIGN_DIR}/{name}");
        let Some(number) = doc_number(name) else {
            problems.push(Problem::new(path, ProblemKind::BadDesignName));
            continue;
        };
        if let Some(other) = seen.insert(number, name) {
            let other = other.to_string();
            problems
                .push(Problem::new(&path, ProblemKind::DuplicateDesignNumber { number, other }));
        }
        let status = front_matter(text).and_then(|fields| fields.get("status").copied());
        if !status.is_some_and(|s| DESIGN_STATUSES.contains(&s)) {
            problems.push(Problem::new(path, ProblemKind::BadDesignStatus));
        }
    }
    problems
}

// Links

/// Checks that each relative link in `text`, a Markdown file at the repository-relative `path`,
/// points to something that exists.
fn check_links(path: &Path, text: &str, exists: impl Fn(&Path) -> bool) -> Vec<Problem> {
    let dir = path.parent().unwrap_or(Path::new(""));
    let mut problems = Vec::new();
    for event in Parser::new_ext(text, markdown_options()) {
        let Event::Start(Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. }) = event else {
            continue;
        };
        let Some(target) = local_target(&dest_url) else {
            continue;
        };
        let resolved = match target.strip_prefix('/') {
            Some(from_root) => normalize(Path::new(from_root)),
            None => normalize(&dir.join(target)),
        };
        let kind = match resolved {
            Some(resolved) if exists(&resolved) => continue,
            Some(_) => ProblemKind::BrokenLink { dest: dest_url.to_string() },
            None => ProblemKind::LinkOutsideRepository { dest: dest_url.to_string() },
        };
        problems.push(Problem::new(display(path), kind));
    }
    problems
}

/// The path part of a link destination inside the repository, or `None` for URLs, email links,
/// and links to a heading on the same page.
fn local_target(dest: &str) -> Option<&str> {
    let target = dest.split(['#', '?']).next().unwrap_or_default();
    let has_scheme = target.split_once(':').is_some_and(|(scheme, _)| {
        scheme.starts_with(|c: char| c.is_ascii_alphabetic())
            && scheme.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    });
    (!target.is_empty() && !has_scheme).then_some(target)
}

/// Resolves `.` and `..` without touching the file system. `None` if the path climbs above its
/// starting point.
fn normalize(path: &Path) -> Option<PathBuf> {
    let mut resolved = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {},
            Component::ParentDir => {
                if !resolved.pop() {
                    return None;
                }
            },
            Component::Normal(part) => resolved.push(part),
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(resolved)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn doc_numbers() {
        assert_eq!(doc_number("0042-typed-wire-validation.md"), Some(42));
        assert_eq!(doc_number("0001-a.md"), Some(1));
        assert_eq!(doc_number("42-short.md"), None);
        assert_eq!(doc_number("0042-Typed.md"), None);
        assert_eq!(doc_number("0042-double--dash.md"), None);
        assert_eq!(doc_number("0042-.md"), None);
        assert_eq!(doc_number("0042-slug.txt"), None);
    }

    #[test]
    fn front_matter_fields() {
        let fields =
            front_matter("---\nstatus: accepted\ndate: 2026-10-17\n---\n# Title\n").unwrap();
        assert_eq!(fields.get("status"), Some(&"accepted"));
        assert_eq!(fields.get("date"), Some(&"2026-10-17"));
        assert!(front_matter("# Title\n").is_none());
        assert!(front_matter("---\nstatus: accepted\n").is_none());
    }

    #[test]
    fn design_docs() {
        let files = [
            ("TEMPLATE.md".to_string(), String::new()),
            ("0001-bootstrap.md".to_string(), "---\nstatus: implemented\n---\n".to_string()),
            ("0001-other.md".to_string(), "---\nstatus: draft\n---\n".to_string()),
            ("0007-feature.md".to_string(), "---\nstatus: done\n---\n".to_string()),
            ("0008-feature.md".to_string(), "# No front matter\n".to_string()),
            ("feature.md".to_string(), String::new()),
        ];
        let in_design =
            |name: &str, kind: ProblemKind| Problem::new(format!("{DESIGN_DIR}/{name}"), kind);
        assert_eq!(
            check_design_docs(&files),
            [
                in_design("TEMPLATE.md", ProblemKind::BadDesignName),
                in_design(
                    "0001-other.md",
                    ProblemKind::DuplicateDesignNumber {
                        number: 1,
                        other: "0001-bootstrap.md".into(),
                    },
                ),
                in_design("0007-feature.md", ProblemKind::BadDesignStatus),
                in_design("0008-feature.md", ProblemKind::BadDesignStatus),
                in_design("feature.md", ProblemKind::BadDesignName),
            ]
        );
    }

    #[test]
    fn links() {
        let existing: HashSet<PathBuf> = [
            "docs/guides/workflow.md",
            "docs/README.md",
            "LICENSE",
            "docs/guides",
        ]
        .into_iter()
        .map(PathBuf::from)
        .collect();
        let text = "\
[ok](workflow.md#branch-notes) [up](../README.md) [root](/LICENSE) [dir](./)
[url](https://example.com/x.md) [mail](mailto:a@example.com) [anchor](#top)
[broken](missing.md) [outside](../../../x.md)

[ref]: ../nope.md
Uses [a reference][ref].

`[code](ignored.md)`

```
[fenced](ignored.md)
```
";
        let path = "docs/guides/workflow.md";
        let problems = check_links(Path::new(path), text, |p| existing.contains(p));
        assert_eq!(
            problems,
            [
                Problem::new(path, ProblemKind::BrokenLink { dest: "missing.md".into() }),
                Problem::new(
                    path,
                    ProblemKind::LinkOutsideRepository { dest: "../../../x.md".into() }
                ),
                Problem::new(path, ProblemKind::BrokenLink { dest: "../nope.md".into() }),
            ]
        );
    }

    #[test]
    fn problems_display_as_messages() {
        assert_eq!(
            Problem::new("docs/README.md", ProblemKind::BrokenLink { dest: "x.md#top".into() })
                .to_string(),
            "docs/README.md: broken link `x.md#top`"
        );
    }
}
