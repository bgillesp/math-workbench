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
const ADR_OTHER_FILES: &[&str] = &["README.md"];

const DESIGN_STATUSES: &[&str] = &["draft", "accepted", "implemented", "abandoned"];

/// The index section that must list exactly the accepted ADRs.
const IN_FORCE: &str = "In force";

/// A problem in one file.
#[derive(Debug, PartialEq, Eq)]
struct Problem {
    /// The repository-relative path of the file to fix.
    path: String,
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
    // An ADR on its own
    /// The file name isn't like `0001-short-title.md`.
    BadAdrName,
    /// Another ADR, named `other`, has the same number.
    DuplicateAdrNumber { number: u32, other: String },
    /// No line starts with `# ADR-NNNN: `.
    BadAdrTitle { number: u32 },
    /// `status` is missing or isn't an ADR status.
    BadAdrStatus,
    /// A `supersedes` entry isn't like `ADR-0001`.
    BadSupersedesEntry { entry: String },

    // Supersede links between ADRs
    /// `supersedes` lists an ADR that doesn't exist.
    SupersedesMissingAdr { old: u32 },
    /// `supersedes` lists the ADR itself.
    SupersedesItself,
    /// An ADR that took effect supersedes this one, but this one's status doesn't say so.
    MissingSupersededBy { new: u32 },
    /// The status says it's superseded by an ADR that doesn't exist.
    SupersededByMissingAdr { new: u32 },
    /// An ADR's status says this one supersedes it, but `supersedes` doesn't list it.
    MissingSupersedes { old: u32 },
    /// The status says it's superseded by an ADR that never took effect.
    SupersedingAdrNotAccepted { new: u32, status: AdrStatus },

    // The ADR index
    /// There's no `## In force` section.
    MissingInForceSection,
    /// An ADR isn't linked anywhere in the index.
    UnlistedAdr { name: String },
    /// An accepted ADR is listed outside `## In force`.
    AcceptedOutsideInForce { number: u32 },
    /// An ADR that isn't accepted is listed under `## In force`.
    UnacceptedInForce { number: u32, status: AdrStatus },

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
            Self::BadAdrName => f.write_str("ADR file names look like `0001-short-title.md`"),
            Self::DuplicateAdrNumber { number, other } => {
                write!(f, "ADR-{number:04} is also used by {other}")
            },
            Self::BadAdrTitle { number } => {
                write!(f, "the title should start with `# ADR-{number:04}:`")
            },
            Self::BadAdrStatus => f.write_str(
                "front matter needs `status:` set to `proposed`, `accepted`, `rejected`, \
                 `superseded by ADR-NNNN`, or `deprecated`",
            ),
            Self::BadSupersedesEntry { entry } => {
                write!(f, "`supersedes` entries look like `ADR-0001`, not `{entry}`")
            },
            Self::SupersedesMissingAdr { old } => {
                write!(f, "supersedes ADR-{old:04}, which doesn't exist")
            },
            Self::SupersedesItself => f.write_str("an ADR can't supersede itself"),
            Self::MissingSupersededBy { new } => write!(
                f,
                "status should be `superseded by ADR-{new:04}`, since that ADR replaces it"
            ),
            Self::SupersededByMissingAdr { new } => {
                write!(f, "superseded by ADR-{new:04}, which doesn't exist")
            },
            Self::MissingSupersedes { old } => {
                write!(f, "front matter should say `supersedes: ADR-{old:04}`")
            },
            Self::SupersedingAdrNotAccepted { new, status } => {
                write!(f, "marked superseded by ADR-{new:04}, which is {status}")
            },
            Self::MissingInForceSection => write!(f, "needs a `## {IN_FORCE}` section"),
            Self::UnlistedAdr { name } => write!(f, "doesn't link {name}"),
            Self::AcceptedOutsideInForce { number } => {
                write!(f, "ADR-{number:04} is accepted, so it belongs under `## {IN_FORCE}`")
            },
            Self::UnacceptedInForce { number, status } => {
                write!(f, "ADR-{number:04} is {status}, so it belongs below `## {IN_FORCE}`")
            },
            Self::BadDesignName => f.write_str(
                "design doc file names look like `0042-short-title.md`, where 0042 is the GitHub \
                 issue number",
            ),
            Self::DuplicateDesignNumber { number, other } => {
                write!(f, "number {number:04} is also used by {other}")
            },
            Self::BadDesignStatus => f.write_str(
                "front matter needs `status:` set to `draft`, `accepted`, `implemented`, or \
                 `abandoned`",
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
    Ok(problems.iter().map(ToString::to_string).collect())
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
fn check_adrs(files: &[(String, String)], index: &str) -> Vec<Problem> {
    let mut problems = Vec::new();
    let mut adrs: BTreeMap<u32, Adr> = BTreeMap::new();
    for (name, text) in files {
        if ADR_OTHER_FILES.contains(&name.as_str()) {
            continue;
        }
        let path = format!("{ADR_DIR}/{name}");
        let Some(number) = doc_number(name) else {
            problems.push(Problem::new(path, ProblemKind::BadAdrName));
            continue;
        };
        if let Some(other) = adrs.get(&number) {
            let other = other.name.clone();
            problems.push(Problem::new(path, ProblemKind::DuplicateAdrNumber { number, other }));
            continue;
        }
        let title = format!("# ADR-{number:04}: ");
        if !text.lines().any(|line| line.starts_with(&title)) {
            problems.push(Problem::new(&path, ProblemKind::BadAdrTitle { number }));
        }
        let fields = front_matter(text).unwrap_or_default();
        let Some(status) = fields.get("status").copied().and_then(AdrStatus::parse) else {
            problems.push(Problem::new(path, ProblemKind::BadAdrStatus));
            continue;
        };
        let mut supersedes = Vec::new();
        for item in fields.get("supersedes").into_iter().flat_map(|list| list.split(',')) {
            let item = item.trim();
            if let Some(old) = parse_adr_ref(item) {
                supersedes.push(old);
            } else {
                let entry = item.to_string();
                problems.push(Problem::new(&path, ProblemKind::BadSupersedesEntry { entry }));
            }
        }
        adrs.insert(number, Adr { number, name: name.clone(), status, supersedes });
    }
    problems.extend(check_supersede_links(&adrs));
    problems.extend(check_index(&adrs, index));
    problems
}

fn check_supersede_links(adrs: &BTreeMap<u32, Adr>) -> Vec<Problem> {
    let mut problems = Vec::new();
    for adr in adrs.values() {
        for &old in &adr.supersedes {
            match adrs.get(&old) {
                None => {
                    let kind = ProblemKind::SupersedesMissingAdr { old };
                    problems.push(Problem::new(adr.path(), kind));
                },
                Some(_) if old == adr.number => {
                    problems.push(Problem::new(adr.path(), ProblemKind::SupersedesItself));
                },
                Some(old_adr)
                    if adr.status.took_effect()
                        && old_adr.status != AdrStatus::SupersededBy(adr.number) =>
                {
                    let kind = ProblemKind::MissingSupersededBy { new: adr.number };
                    problems.push(Problem::new(old_adr.path(), kind));
                },
                Some(_) => {},
            }
        }
        if let AdrStatus::SupersededBy(new) = adr.status {
            match adrs.get(&new) {
                None => {
                    let kind = ProblemKind::SupersededByMissingAdr { new };
                    problems.push(Problem::new(adr.path(), kind));
                },
                Some(new_adr) if !new_adr.supersedes.contains(&adr.number) => {
                    let kind = ProblemKind::MissingSupersedes { old: adr.number };
                    problems.push(Problem::new(new_adr.path(), kind));
                },
                Some(new_adr) if !new_adr.status.took_effect() => {
                    let kind =
                        ProblemKind::SupersedingAdrNotAccepted { new, status: new_adr.status };
                    problems.push(Problem::new(adr.path(), kind));
                },
                Some(_) => {},
            }
        }
    }
    problems
}

/// Checks that the index lists every ADR, with accepted ADRs, and only those, under "In force".
fn check_index(adrs: &BTreeMap<u32, Adr>, index: &str) -> Vec<Problem> {
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
        problems.push(Problem::new(&index_path, ProblemKind::MissingInForceSection));
    }
    for adr in adrs.values() {
        let listed_in_force = in_force_links.contains(&adr.name);
        let accepted = adr.status == AdrStatus::Accepted;
        if !listed_in_force && !other_links.contains(&adr.name) {
            let kind = ProblemKind::UnlistedAdr { name: adr.name.clone() };
            problems.push(Problem::new(&index_path, kind));
        } else if accepted && !listed_in_force {
            let kind = ProblemKind::AcceptedOutsideInForce { number: adr.number };
            problems.push(Problem::new(&index_path, kind));
        } else if !accepted && listed_in_force {
            let kind = ProblemKind::UnacceptedInForce { number: adr.number, status: adr.status };
            problems.push(Problem::new(&index_path, kind));
        }
    }
    problems
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

    /// A problem in the ADR that `adr` creates for `number`.
    fn in_adr(number: u32, kind: ProblemKind) -> Problem {
        Problem::new(format!("{ADR_DIR}/{number:04}-decision.md"), kind)
    }

    /// A problem in the ADR index.
    fn in_index(kind: ProblemKind) -> Problem {
        Problem::new(format!("{ADR_DIR}/README.md"), kind)
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
        assert_eq!(problems, Vec::<Problem>::new());
    }

    #[test]
    fn empty_index_passes() {
        let index = "# ADRs\n\n## In force\n\nNone yet.\n";
        assert_eq!(check_adrs(&[], index), Vec::<Problem>::new());
    }

    #[test]
    fn index_needs_in_force_section() {
        let problems = check_adrs(&[], "# ADRs\n\n## Current\n");
        assert_eq!(problems, [in_index(ProblemKind::MissingInForceSection)]);
    }

    #[test]
    fn duplicate_numbers_fail() {
        let mut other = adr(1, "accepted", None);
        other.0 = "0001-other.md".to_string();
        let problems = check_adrs(&[adr(1, "accepted", None), other], &index(&[1], &[]));
        assert_eq!(
            problems,
            [Problem::new(
                format!("{ADR_DIR}/0001-other.md"),
                ProblemKind::DuplicateAdrNumber { number: 1, other: "0001-decision.md".into() },
            )]
        );
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
        assert_eq!(
            problems,
            [
                Problem::new(format!("{ADR_DIR}/notes.md"), ProblemKind::BadAdrName),
                in_adr(2, ProblemKind::BadAdrTitle { number: 2 }),
                in_adr(3, ProblemKind::BadAdrStatus),
            ]
        );
    }

    #[test]
    fn accepted_superseding_adr_requires_old_status() {
        let files = [
            adr(1, "accepted", None),
            adr(2, "accepted", Some("ADR-0001")),
        ];
        let problems = check_adrs(&files, &index(&[1, 2], &[]));
        assert_eq!(problems, [in_adr(1, ProblemKind::MissingSupersededBy { new: 2 })]);
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
        assert_eq!(
            problems,
            [
                in_adr(2, ProblemKind::MissingSupersedes { old: 1 }),
                in_adr(
                    3,
                    ProblemKind::SupersedingAdrNotAccepted { new: 4, status: AdrStatus::Proposed }
                ),
                in_adr(5, ProblemKind::SupersededByMissingAdr { new: 9 }),
            ]
        );
    }

    #[test]
    fn supersedes_must_name_existing_other_adrs() {
        let files = [adr(1, "accepted", Some("ADR-0001, ADR-0008, 7"))];
        let problems = check_adrs(&files, &index(&[1], &[]));
        assert_eq!(
            problems,
            [
                in_adr(1, ProblemKind::BadSupersedesEntry { entry: "7".into() }),
                in_adr(1, ProblemKind::SupersedesItself),
                in_adr(1, ProblemKind::SupersedesMissingAdr { old: 8 }),
            ]
        );
    }

    #[test]
    fn index_sections_must_match_status() {
        let files = [
            adr(1, "accepted", None),
            adr(2, "rejected", None),
            adr(3, "accepted", None),
        ];
        let problems = check_adrs(&files, &index(&[2], &[1]));
        assert_eq!(
            problems,
            [
                in_index(ProblemKind::AcceptedOutsideInForce { number: 1 }),
                in_index(ProblemKind::UnacceptedInForce { number: 2, status: AdrStatus::Rejected }),
                in_index(ProblemKind::UnlistedAdr { name: "0003-decision.md".into() }),
            ]
        );
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
            in_adr(1, ProblemKind::MissingSupersededBy { new: 2 }).to_string(),
            "docs/architecture/decisions/0001-decision.md: status should be \
             `superseded by ADR-0002`, since that ADR replaces it"
        );
        assert_eq!(
            in_index(ProblemKind::UnacceptedInForce { number: 2, status: AdrStatus::Rejected })
                .to_string(),
            "docs/architecture/decisions/README.md: ADR-0002 is rejected, so it belongs below \
             `## In force`"
        );
        assert_eq!(
            Problem::new("docs/README.md", ProblemKind::BrokenLink { dest: "x.md#top".into() })
                .to_string(),
            "docs/README.md: broken link `x.md#top`"
        );
    }
}
