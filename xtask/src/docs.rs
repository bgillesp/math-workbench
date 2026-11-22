//! `check-docs`: relative links in Markdown files, apart from design docs, point to files in the
//! repository.

use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use pulldown_cmark::{Event, Options, Parser, Tag};

use crate::{CheckResult, display};

/// Design docs, which are left as a historical record, so their links aren't kept in step with
/// later moves. Git history shows what a link pointed to when its doc was written.
const UNCHECKED_DIR: &str = "docs/design";

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

/// Markdown files to check: those that git tracks or would track, so ignored and nested-worktree
/// files are skipped, outside [`UNCHECKED_DIR`].
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
        .filter(|p| is_checked(p))
        .collect();
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn is_checked(path: &Path) -> bool {
    !path.starts_with(UNCHECKED_DIR)
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
    use super::*;

    /// The file whose links each test checks.
    const PAGE: &str = "docs/guides/page.md";

    /// The problems in `text`, checked as the file at [`PAGE`] in a repository holding only the
    /// files and directories in `existing`.
    fn check_page(text: &str, existing: &[&str]) -> Vec<Problem> {
        check_links(Path::new(PAGE), text, |target| {
            existing.iter().any(|path| Path::new(path) == target)
        })
    }

    fn broken(dest: &str) -> Problem {
        Problem::new(PAGE, ProblemKind::BrokenLink { dest: dest.into() })
    }

    #[test]
    fn relative_links_resolve_from_the_linking_files_directory() {
        // Only the last link's target is missing from `docs/guides/`; it exists at the root.
        let text = "[sibling](sibling.md) [parent](../README.md) [root](README.md)";
        let existing = ["docs/guides/sibling.md", "docs/README.md", "README.md"];
        assert_eq!(check_page(text, &existing), [broken("README.md")]);
    }

    #[test]
    fn leading_slash_resolves_from_the_repository_root() {
        // Only the last link's target is missing from the root; it exists in `docs/guides/`.
        let text = "[license](/LICENSE) [sibling](/sibling.md)";
        let existing = ["LICENSE", "docs/guides/sibling.md"];
        assert_eq!(check_page(text, &existing), [broken("/sibling.md")]);
    }

    #[test]
    fn links_above_the_root_are_reported_as_outside_the_repository() {
        assert_eq!(
            check_page("[outside](../../../x.md)", &[]),
            [Problem::new(
                PAGE,
                ProblemKind::LinkOutsideRepository { dest: "../../../x.md".into() }
            )]
        );
    }

    #[test]
    fn anchors_and_queries_are_ignored() {
        let text = "[anchor](sibling.md#part) [query](sibling.md?plain=1)";
        assert_eq!(check_page(text, &["docs/guides/sibling.md"]), Vec::<Problem>::new());
    }

    #[test]
    fn external_and_same_page_links_are_skipped() {
        let text = "[url](https://example.com/x.md) [mail](mailto:a@example.com) [heading](#top)";
        assert_eq!(check_page(text, &[]), Vec::<Problem>::new());
    }

    #[test]
    fn images_are_checked_like_links() {
        assert_eq!(check_page("![diagram](diagram.svg)", &[]), [broken("diagram.svg")]);
    }

    #[test]
    fn design_docs_are_not_checked() {
        assert!(!is_checked(Path::new("docs/design/2026-01-01-topic.md")));
        assert!(is_checked(Path::new("docs/architecture/overview.md")));
    }
}
