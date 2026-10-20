//! `check-docs`: ADRs, design docs, and relative links in Markdown files are consistent.
//!
//! ADRs and design docs carry their status in front matter:
//!
//! ```text
//! ---
//! status: accepted
//! supersedes: ADR-0002
//! ---
//! ```
//!
//! Only flat `key: value` lines are read.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::{CheckResult, display};

const ADR_DIR: &str = "docs/architecture/decisions";
const DESIGN_DIR: &str = "docs/design";

/// Files in the ADR directory that aren't ADRs.
const ADR_OTHER_FILES: &[&str] = &["README.md", "TEMPLATE.md"];
/// Files in the design doc directory that aren't design docs.
const DESIGN_OTHER_FILES: &[&str] = &["TEMPLATE.md"];

const DESIGN_STATUSES: &[&str] = &["draft", "accepted", "implemented", "abandoned"];

/// The index section that must list exactly the accepted ADRs.
const IN_FORCE: &str = "In force";

pub(crate) fn run(root: &Path) -> CheckResult {
    let mut problems = Vec::new();

    let adrs = read_files(&root.join(ADR_DIR))?;
    let index = fs::read_to_string(root.join(ADR_DIR).join("README.md"))?;
    problems.extend(check_adrs(&adrs, &index));

    let design_docs = read_files(&root.join(DESIGN_DIR))?;
    problems.extend(check_design_docs(&design_docs));

    for path in markdown_files(root)? {
        let Ok(text) = fs::read_to_string(root.join(&path)) else {
            continue; // deleted from the working tree but still in the index
        };
        problems.extend(check_links(&path, &text, |target| root.join(target).exists()));
    }
    Ok(problems)
}

/// The name and contents of each file directly inside `dir`.
fn read_files(dir: &Path) -> std::io::Result<Vec<(String, String)>> {
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

// ADRs and design docs

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AdrStatus {
    Proposed,
    Accepted,
    Rejected,
    SupersededBy(u32),
    Deprecated,
}

impl AdrStatus {
    fn parse(text: &str) -> Option<Self> {
        match text {
            "proposed" => Some(Self::Proposed),
            "accepted" => Some(Self::Accepted),
            "rejected" => Some(Self::Rejected),
            "deprecated" => Some(Self::Deprecated),
            _ => {
                text.strip_prefix("superseded by ").and_then(parse_adr_ref).map(Self::SupersededBy)
            },
        }
    }

    /// Whether the decision took effect at some point, even if it has since been replaced.
    fn took_effect(self) -> bool {
        matches!(self, Self::Accepted | Self::SupersededBy(_) | Self::Deprecated)
    }
}

impl fmt::Display for AdrStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Proposed => f.write_str("proposed"),
            Self::Accepted => f.write_str("accepted"),
            Self::Rejected => f.write_str("rejected"),
            Self::SupersededBy(n) => write!(f, "superseded by ADR-{n:04}"),
            Self::Deprecated => f.write_str("deprecated"),
        }
    }
}

#[derive(Debug)]
struct Adr {
    number: u32,
    name: String,
    status: AdrStatus,
    supersedes: Vec<u32>,
}

impl Adr {
    fn path(&self) -> String {
        format!("{ADR_DIR}/{}", self.name)
    }
}

/// Parses `ADR-0003` into `3`.
fn parse_adr_ref(text: &str) -> Option<u32> {
    let digits = text.strip_prefix("ADR-")?;
    if digits.len() == 4 && digits.bytes().all(|b| b.is_ascii_digit()) {
        digits.parse().ok()
    } else {
        None
    }
}

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
fn front_matter(text: &str) -> Option<HashMap<&str, &str>> {
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

/// Checks ADR file names, titles, statuses, supersede links, and the index.
fn check_adrs(files: &[(String, String)], index: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let mut adrs: BTreeMap<u32, Adr> = BTreeMap::new();
    for (name, text) in files {
        if ADR_OTHER_FILES.contains(&name.as_str()) {
            continue;
        }
        let path = format!("{ADR_DIR}/{name}");
        let Some(number) = doc_number(name) else {
            problems.push(format!("{path}: ADR file names look like `0001-short-title.md`"));
            continue;
        };
        if let Some(other) = adrs.get(&number) {
            problems.push(format!("{path}: ADR-{number:04} is also used by {}", other.path()));
            continue;
        }
        let title = format!("# ADR-{number:04}: ");
        if !text.lines().any(|line| line.starts_with(&title)) {
            problems.push(format!("{path}: the title should start with `{}`", title.trim_end()));
        }
        let fields = front_matter(text).unwrap_or_default();
        let Some(status) = fields.get("status").copied().and_then(AdrStatus::parse) else {
            problems.push(format!(
                "{path}: front matter needs `status:` set to `proposed`, `accepted`, `rejected`, \
                 `superseded by ADR-NNNN`, or `deprecated`"
            ));
            continue;
        };
        let mut supersedes = Vec::new();
        for item in fields.get("supersedes").into_iter().flat_map(|list| list.split(',')) {
            match parse_adr_ref(item.trim()) {
                Some(old) => supersedes.push(old),
                None => problems.push(format!(
                    "{path}: `supersedes` entries look like `ADR-0001`, not `{}`",
                    item.trim()
                )),
            }
        }
        adrs.insert(number, Adr { number, name: name.clone(), status, supersedes });
    }
    problems.extend(check_supersede_links(&adrs));
    problems.extend(check_index(&adrs, index));
    problems
}

fn check_supersede_links(adrs: &BTreeMap<u32, Adr>) -> Vec<String> {
    let mut problems = Vec::new();
    for adr in adrs.values() {
        for &old in &adr.supersedes {
            match adrs.get(&old) {
                None => problems
                    .push(format!("{}: supersedes ADR-{old:04}, which doesn't exist", adr.path())),
                Some(_) if old == adr.number => {
                    problems.push(format!("{}: an ADR can't supersede itself", adr.path()));
                },
                Some(old_adr)
                    if adr.status.took_effect()
                        && old_adr.status != AdrStatus::SupersededBy(adr.number) =>
                {
                    problems.push(format!(
                        "{}: status should be `superseded by ADR-{:04}`, since that ADR replaces it",
                        old_adr.path(),
                        adr.number
                    ));
                },
                Some(_) => {},
            }
        }
        if let AdrStatus::SupersededBy(new) = adr.status {
            match adrs.get(&new) {
                None => problems.push(format!(
                    "{}: superseded by ADR-{new:04}, which doesn't exist",
                    adr.path()
                )),
                Some(new_adr) if !new_adr.supersedes.contains(&adr.number) => {
                    problems.push(format!(
                        "{}: front matter should say `supersedes: ADR-{:04}`",
                        new_adr.path(),
                        adr.number
                    ));
                },
                Some(new_adr) if !new_adr.status.took_effect() => problems.push(format!(
                    "{}: marked superseded by ADR-{new:04}, which is {}",
                    adr.path(),
                    new_adr.status
                )),
                Some(_) => {},
            }
        }
    }
    problems
}

/// Checks that the index lists every ADR, with accepted ADRs, and only those, under "In force".
fn check_index(adrs: &BTreeMap<u32, Adr>, index: &str) -> Vec<String> {
    let index_path = format!("{ADR_DIR}/README.md");
    let mut in_force_links = HashSet::new();
    let mut other_links = HashSet::new();
    let mut has_in_force = false;
    let mut in_force = false;
    let mut heading: Option<String> = None;
    for event in Parser::new_ext(index, markdown_options()) {
        match event {
            Event::Start(Tag::Heading { level: HeadingLevel::H2, .. }) => {
                heading = Some(String::new());
            },
            Event::Text(text) | Event::Code(text) => {
                if let Some(heading) = heading.as_mut() {
                    heading.push_str(&text);
                }
            },
            Event::End(TagEnd::Heading(HeadingLevel::H2)) => {
                in_force = heading.take().is_some_and(|h| h.trim() == IN_FORCE);
                has_in_force |= in_force;
            },
            Event::Start(Tag::Link { dest_url, .. }) => {
                let target = dest_url.split('#').next().unwrap_or_default();
                let target = target.strip_prefix("./").unwrap_or(target).to_string();
                if in_force {
                    in_force_links.insert(target);
                } else {
                    other_links.insert(target);
                }
            },
            _ => {},
        }
    }

    let mut problems = Vec::new();
    if !has_in_force {
        problems.push(format!("{index_path}: needs a `## {IN_FORCE}` section"));
    }
    for adr in adrs.values() {
        let listed_in_force = in_force_links.contains(&adr.name);
        let accepted = adr.status == AdrStatus::Accepted;
        if !listed_in_force && !other_links.contains(&adr.name) {
            problems.push(format!("{index_path}: doesn't link {}", adr.name));
        } else if accepted && !listed_in_force {
            problems.push(format!(
                "{index_path}: ADR-{:04} is accepted, so it belongs under `## {IN_FORCE}`",
                adr.number
            ));
        } else if !accepted && listed_in_force {
            problems.push(format!(
                "{index_path}: ADR-{:04} is {}, so it belongs below `## {IN_FORCE}`",
                adr.number, adr.status
            ));
        }
    }
    problems
}

/// Checks design doc file names, numbering, and statuses.
fn check_design_docs(files: &[(String, String)]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut seen: HashMap<u32, &str> = HashMap::new();
    for (name, text) in files {
        if DESIGN_OTHER_FILES.contains(&name.as_str()) {
            continue;
        }
        let path = format!("{DESIGN_DIR}/{name}");
        let Some(number) = doc_number(name) else {
            problems.push(format!(
                "{path}: design doc file names look like `0042-short-title.md`, where 0042 is \
                 the GitHub issue number"
            ));
            continue;
        };
        if let Some(other) = seen.insert(number, name) {
            problems.push(format!("{path}: number {number:04} is also used by {other}"));
        }
        let status = front_matter(text).and_then(|fields| fields.get("status").copied());
        if !status.is_some_and(|s| DESIGN_STATUSES.contains(&s)) {
            problems.push(format!(
                "{path}: front matter needs `status:` set to `draft`, `accepted`, \
                 `implemented`, or `abandoned`"
            ));
        }
    }
    problems
}

// Links

/// Checks that each relative link in `text`, a Markdown file at the repository-relative `path`,
/// points to something that exists.
fn check_links(path: &Path, text: &str, exists: impl Fn(&Path) -> bool) -> Vec<String> {
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
        match resolved {
            Some(resolved) if exists(&resolved) => {},
            Some(_) => problems.push(format!("{}: broken link `{dest_url}`", display(path))),
            None => problems.push(format!(
                "{}: link `{dest_url}` points outside the repository",
                display(path)
            )),
        }
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

    fn adr(number: u32, status: &str, supersedes: Option<&str>) -> (String, String) {
        let supersedes = supersedes.map(|s| format!("supersedes: {s}\n")).unwrap_or_default();
        (
            format!("{number:04}-decision.md"),
            format!("---\nstatus: {status}\n{supersedes}---\n\n# ADR-{number:04}: Decision\n"),
        )
    }

    fn index(in_force: &[u32], other: &[u32]) -> String {
        let links = |numbers: &[u32]| {
            numbers
                .iter()
                .map(|n| format!("- [ADR-{n:04}: Decision]({n:04}-decision.md)\n"))
                .collect::<Vec<_>>()
                .concat()
        };
        format!(
            "# ADRs\n\n## In force\n\n### Area\n\n{}\n## History\n\n{}",
            links(in_force),
            links(other)
        )
    }

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
    fn adr_statuses() {
        assert_eq!(AdrStatus::parse("accepted"), Some(AdrStatus::Accepted));
        assert_eq!(AdrStatus::parse("superseded by ADR-0007"), Some(AdrStatus::SupersededBy(7)));
        assert_eq!(AdrStatus::parse("superseded by 7"), None);
        assert_eq!(AdrStatus::parse("provisional"), None);
        assert_eq!(AdrStatus::SupersededBy(7).to_string(), "superseded by ADR-0007");
    }

    #[test]
    fn consistent_adrs_pass() {
        let files = [
            adr(1, "superseded by ADR-0003", None),
            adr(2, "rejected", None),
            adr(3, "accepted", Some("ADR-0001")),
            adr(4, "proposed", Some("ADR-0003")),
        ];
        let problems = check_adrs(&files, &index(&[3], &[1, 2, 4]));
        assert_eq!(problems, Vec::<String>::new());
    }

    #[test]
    fn empty_index_passes() {
        let index = "# ADRs\n\n## In force\n\nNone yet.\n";
        assert_eq!(check_adrs(&[], index), Vec::<String>::new());
    }

    #[test]
    fn index_needs_in_force_section() {
        let problems = check_adrs(&[], "# ADRs\n\n## Current\n");
        assert_eq!(
            problems,
            ["docs/architecture/decisions/README.md: needs a `## In force` section"]
        );
    }

    #[test]
    fn duplicate_numbers_fail() {
        let mut other = adr(1, "accepted", None);
        other.0 = "0001-other.md".to_string();
        let problems = check_adrs(&[adr(1, "accepted", None), other], &index(&[1], &[]));
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("ADR-0001 is also used by"));
    }

    #[test]
    fn bad_names_titles_and_statuses_fail() {
        let mut untitled = adr(2, "accepted", None);
        untitled.1 = untitled.1.replace("# ADR-0002: ", "# ");
        let files = [
            ("notes.md".to_string(), String::new()),
            untitled,
            adr(3, "provisional", None),
        ];
        let problems = check_adrs(&files, &index(&[2], &[]));
        assert_eq!(problems.len(), 3, "{problems:?}");
    }

    #[test]
    fn accepted_superseding_adr_requires_old_status() {
        let files = [
            adr(1, "accepted", None),
            adr(2, "accepted", Some("ADR-0001")),
        ];
        let problems = check_adrs(&files, &index(&[1, 2], &[]));
        assert_eq!(problems.len(), 1);
        assert!(
            problems[0].contains("0001-decision.md: status should be `superseded by ADR-0002`")
        );
    }

    #[test]
    fn superseded_by_requires_matching_link_and_acceptance() {
        let files = [
            adr(1, "superseded by ADR-0002", None),
            adr(2, "accepted", None),
            adr(3, "superseded by ADR-0004", None),
            adr(4, "proposed", Some("ADR-0003")),
            adr(5, "superseded by ADR-0009", None),
        ];
        let problems = check_adrs(&files, &index(&[2], &[1, 3, 4, 5]));
        assert_eq!(problems.len(), 3, "{problems:?}");
        assert!(
            problems[0]
                .contains("0002-decision.md: front matter should say `supersedes: ADR-0001`")
        );
        assert!(problems[1].contains("which is proposed"));
        assert!(problems[2].contains("ADR-0009, which doesn't exist"));
    }

    #[test]
    fn supersedes_must_name_existing_other_adrs() {
        let files = [adr(1, "accepted", Some("ADR-0001, ADR-0008, 7"))];
        let problems = check_adrs(&files, &index(&[1], &[]));
        assert_eq!(problems.len(), 3, "{problems:?}");
    }

    #[test]
    fn index_sections_must_match_status() {
        let files = [
            adr(1, "accepted", None),
            adr(2, "rejected", None),
            adr(3, "accepted", None),
        ];
        let problems = check_adrs(&files, &index(&[2], &[1]));
        assert_eq!(problems.len(), 3, "{problems:?}");
        assert!(problems[0].contains("ADR-0001 is accepted, so it belongs under"));
        assert!(problems[1].contains("ADR-0002 is rejected, so it belongs below"));
        assert!(problems[2].contains("doesn't link 0003-decision.md"));
    }

    #[test]
    fn design_docs() {
        let files = [
            ("TEMPLATE.md".to_string(), String::new()),
            ("0001-bootstrap.md".to_string(), "---\nstatus: implemented\n---\n".to_string()),
            ("0007-feature.md".to_string(), "---\nstatus: done\n---\n".to_string()),
            ("0008-feature.md".to_string(), "# No front matter\n".to_string()),
            ("feature.md".to_string(), String::new()),
        ];
        let problems = check_design_docs(&files);
        assert_eq!(problems.len(), 3, "{problems:?}");
        assert!(problems[0].starts_with("docs/design/0007-feature.md"));
        assert!(problems[1].starts_with("docs/design/0008-feature.md"));
        assert!(problems[2].starts_with("docs/design/feature.md"));
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
        let problems =
            check_links(Path::new("docs/guides/workflow.md"), text, |p| existing.contains(p));
        assert_eq!(
            problems,
            [
                "docs/guides/workflow.md: broken link `missing.md`",
                "docs/guides/workflow.md: link `../../../x.md` points outside the repository",
                "docs/guides/workflow.md: broken link `../nope.md`",
            ]
        );
    }
}
