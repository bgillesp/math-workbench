//! `check-docs`: relative links in Markdown files point to files in the repository.

use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use pulldown_cmark::{Event, Options, Parser, Tag};

use crate::{CheckResult, display};

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
    /// A relative link points to something that doesn't exist.
    BrokenLink { dest: String },
    /// A relative link climbs above the repository root.
    LinkOutsideRepository { dest: String },
}

impl fmt::Display for ProblemKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BrokenLink { dest } => write!(f, "broken link `{dest}`"),
            Self::LinkOutsideRepository { dest } => {
                write!(f, "link `{dest}` points outside the repository")
            },
        }
    }
}

pub(crate) fn run(root: &Path) -> CheckResult {
    let mut problems = Vec::new();
    for path in markdown_files(root)? {
        let Ok(text) = fs::read_to_string(root.join(&path)) else {
            continue; // deleted from the working tree but still in the index
        };
        problems.extend(check_links(&path, &text, |target| root.join(target).exists()));
    }
    Ok(problems.iter().map(ToString::to_string).collect())
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
