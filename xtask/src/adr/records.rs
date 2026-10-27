//! Reading ADRs: the files on disk, the front matter schema, and the tags in `tags.yaml`.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io;
use std::path::Path;

use super::check::ProblemKind;
use super::{ADR_DIR, AdrFile, Dir, TAGS_PATH, parse_id, split_tags, yaml};

/// The front matter fields, in the order `adr new` writes them.
const FIELDS: [&str; 5] = ["id", "title", "summary", "date", "supersedes"];

/// The longest a summary can be, in characters.
const SUMMARY_LIMIT: usize = 300;

/// An ADR's front matter, once it has been read.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct FrontMatter {
    /// The number in `id`.
    pub(super) number: u32,
    /// The title, unquoted.
    pub(super) title: String,
    /// The summary, unquoted, with continuation lines folded into single spaces.
    pub(super) summary: String,
    /// The date the ADR was created, as `YYYY-MM-DD`.
    pub(super) date: String,
    /// The numbers of the ADRs this one replaces.
    pub(super) supersedes: Vec<u32>,
}

/// An ADR, as read from its file and `tags.yaml`.
#[derive(Debug)]
pub(super) struct Adr {
    /// The file the ADR was read from, with its full contents.
    pub(super) file: AdrFile,
    /// The ADR's front matter.
    pub(super) front_matter: FrontMatter,
    /// The ADR's tags, in the order `tags.yaml` lists them.
    pub(super) tags: Vec<String>,
    /// The numbers of the ADRs whose `supersedes` lists this one: none for an ADR in force, and
    /// one for a superseded ADR once `check-docs` passes.
    pub(super) successors: Vec<u32>,
}

impl Adr {
    /// Whether the ADR is in force, which its directory decides.
    pub(super) fn is_active(&self) -> bool {
        self.file.dir == Dir::Active
    }

    /// How the ADR stands: in force, or superseded by its successors.
    pub(super) fn status(&self) -> String {
        if self.is_active() {
            return "in force".to_string();
        }
        if self.successors.is_empty() {
            return "superseded".to_string();
        }
        let ids: Vec<String> =
            self.successors.iter().map(|number| format!("ADR-{number:04}")).collect();
        format!("superseded by {}", ids.join(", "))
    }
}

/// The ADRs the views can show, and warnings about files they had to leave out.
#[derive(Debug)]
pub(super) struct Loaded {
    /// The ADRs whose front matter could be read, by number.
    pub(super) adrs: Vec<Adr>,
    /// A message for each file that was left out.
    pub(super) warnings: Vec<String>,
}

/// The front matter fields that could be read, whether or not the rest could.
#[derive(Debug, Default)]
pub(super) struct Fields {
    /// The number in `id`.
    pub(super) number: Option<u32>,
    /// The title, unquoted.
    pub(super) title: Option<String>,
    /// The summary, unquoted.
    pub(super) summary: Option<String>,
    /// The date, as `YYYY-MM-DD`.
    pub(super) date: Option<String>,
    /// The numbers of the well-formed `supersedes` entries.
    pub(super) supersedes: Vec<u32>,
}

/// Reads an ADR's front matter, or every problem that keeps it from being read.
pub(super) fn parse_front_matter(text: &str) -> Result<FrontMatter, Vec<ProblemKind>> {
    let (fields, problems) = read_front_matter(text);
    if problems.is_empty()
        && let (Some(number), Some(title), Some(summary), Some(date)) =
            (fields.number, fields.title, fields.summary, fields.date)
    {
        return Ok(FrontMatter { number, title, summary, date, supersedes: fields.supersedes });
    }
    Err(problems)
}

/// Reads as much of an ADR's front matter as it can, and every problem with it.
pub(super) fn read_front_matter(text: &str) -> (Fields, Vec<ProblemKind>) {
    let mut fields = Fields::default();
    let Some(block) = front_matter_block(text) else {
        return (fields, vec![ProblemKind::NoFrontMatter]);
    };
    let (entries, errors) = yaml::read(block);
    let mut problems = Vec::new();
    // The keys that appear, including those whose values couldn't be read.
    let mut keys = HashSet::new();
    for error in errors {
        problems.push(match error {
            yaml::Error::MalformedLine { line } => ProblemKind::MalformedLine { line },
            yaml::Error::UnterminatedQuote { key } => {
                keys.insert(key.clone());
                ProblemKind::UnterminatedQuote { key }
            },
            yaml::Error::QuoteOrBackslash { key } => {
                keys.insert(key.clone());
                ProblemKind::QuoteOrBackslash { key }
            },
        });
    }
    for yaml::Entry { key, value } in entries {
        if !FIELDS.contains(&key.as_str()) {
            problems.push(ProblemKind::UnknownKey { key });
            continue;
        }
        if !keys.insert(key.clone()) {
            problems.push(ProblemKind::DuplicateKey { key });
            continue;
        }
        let text = value.text();
        if text.is_empty() {
            problems.push(ProblemKind::EmptyField { key });
            continue;
        }
        match key.as_str() {
            "id" => match parse_id(text) {
                Some(number) => fields.number = Some(number),
                None => problems.push(ProblemKind::BadId { value: text.to_string() }),
            },
            "title" | "summary" if matches!(value, yaml::Value::Plain(_)) => {
                problems.push(ProblemKind::NotQuoted { key });
            },
            "title" => fields.title = Some(text.to_string()),
            "summary" => match text.chars().count() {
                chars if chars > SUMMARY_LIMIT => {
                    problems.push(ProblemKind::SummaryTooLong { chars });
                },
                _ => fields.summary = Some(text.to_string()),
            },
            "date" if is_date(text) => fields.date = Some(text.to_string()),
            "date" => problems.push(ProblemKind::BadDate { value: text.to_string() }),
            _ => {
                for entry in text.split(',').map(str::trim) {
                    match parse_id(entry) {
                        Some(number) => fields.supersedes.push(number),
                        None => problems
                            .push(ProblemKind::BadSupersedesEntry { entry: entry.to_string() }),
                    }
                }
            },
        }
    }
    for key in &FIELDS[..4] {
        if !keys.contains(*key) {
            problems.push(ProblemKind::MissingField { key: (*key).to_string() });
        }
    }
    (fields, problems)
}

/// The text between the `---` line that opens `text` and the next `---` line.
fn front_matter_block(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("---\n").or_else(|| text.strip_prefix("---\r\n"))?;
    let mut end = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end() == "---" {
            return Some(&rest[..end]);
        }
        end += line.len();
    }
    None
}

/// Whether `text` is a real date written as `YYYY-MM-DD`.
fn is_date(text: &str) -> bool {
    let well_formed = text.len() == 10
        && text.bytes().enumerate().all(|(i, byte)| match i {
            4 | 7 => byte == b'-',
            _ => byte.is_ascii_digit(),
        });
    if !well_formed {
        return false;
    }
    let number = |range: std::ops::Range<usize>| text[range].parse::<u32>().unwrap_or_default();
    let (year, month, day) = (number(0..4), number(5..7), number(8..10));
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        1..=12 => 31,
        _ => return false,
    };
    (1..=days).contains(&day)
}

/// Reads the files in `active/` and `superseded/`, in that order and sorted by name, and the
/// contents of `tags.yaml`. A missing directory or tags file counts as empty.
pub(super) fn read_adr_files(root: &Path) -> io::Result<(Vec<AdrFile>, Option<String>)> {
    let mut files = Vec::new();
    for dir in [Dir::Active, Dir::Superseded] {
        let path = format!("{ADR_DIR}/{}", dir.name());
        let entries = match fs::read_dir(root.join(&path)) {
            Ok(entries) => entries,
            Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
            Err(err) => return Err(at(&path, &err)),
        };
        let mut in_dir = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|err| at(&path, &err))?;
            if entry.file_type().map_err(|err| at(&path, &err))?.is_file() {
                let name = entry.file_name().to_string_lossy().into_owned();
                let text = fs::read_to_string(entry.path())
                    .map_err(|err| at(&format!("{path}/{name}"), &err))?;
                in_dir.push(AdrFile { dir, name, text });
            }
        }
        in_dir.sort_by(|a, b| a.name.cmp(&b.name));
        files.extend(in_dir);
    }
    Ok((files, read_optional(root, TAGS_PATH)?))
}

/// The contents of the file at `path` under `root`, or `None` if there's no file there.
pub(super) fn read_optional(root: &Path, path: &str) -> io::Result<Option<String>> {
    match fs::read_to_string(root.join(path)) {
        Ok(text) => Ok(Some(text)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(at(path, &err)),
    }
}

/// `err`, with the repository-relative path it happened at in its message.
pub(super) fn at(path: &str, err: &io::Error) -> io::Error {
    io::Error::new(err.kind(), format!("{path}: {err}"))
}

/// The entries in `tags` whose key is an ADR id, in order, with their tags. The rest is left to
/// `check-docs`.
pub(super) fn tag_entries(tags: Option<&str>) -> Vec<(u32, Vec<String>)> {
    let (entries, _) = yaml::read(tags.unwrap_or_default());
    entries
        .iter()
        .filter_map(|entry| {
            let number = parse_id(&entry.key)?;
            let mut tags: Vec<String> = Vec::new();
            for tag in split_tags(entry.value.text()) {
                if !tag.is_empty() && !tags.iter().any(|known| known == tag) {
                    tags.push(tag.to_string());
                }
            }
            Some((number, tags))
        })
        .collect()
}

/// Reads the ADRs for the views, leaving out any whose front matter can't be read. An ADR with no
/// entry in `tags` has no tags.
pub(super) fn load(files: Vec<AdrFile>, tags: Option<&str>) -> Loaded {
    let mut tags_by_number = HashMap::new();
    for (number, tags) in tag_entries(tags) {
        tags_by_number.entry(number).or_insert(tags);
    }
    let mut adrs = Vec::new();
    let mut warnings = Vec::new();
    for file in files {
        let Ok(front_matter) = parse_front_matter(&file.text) else {
            warnings.push(format!(
                "{}: front matter can't be read, so it's left out; `cargo xtask check-docs` has \
                 the details",
                file.path()
            ));
            continue;
        };
        let tags = tags_by_number.get(&front_matter.number).cloned().unwrap_or_default();
        adrs.push(Adr { file, front_matter, tags, successors: Vec::new() });
    }
    adrs.sort_by_key(|adr| adr.front_matter.number);

    let mut successors: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for adr in &adrs {
        for &old in &adr.front_matter.supersedes {
            let list = successors.entry(old).or_default();
            if !list.contains(&adr.front_matter.number) {
                list.push(adr.front_matter.number);
            }
        }
    }
    for adr in &mut adrs {
        adr.successors = successors.get(&adr.front_matter.number).cloned().unwrap_or_default();
    }
    Loaded { adrs, warnings }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adr::Dir::{Active, Superseded};
    use crate::adr::fixtures::{SAMPLE_TAGS, TempRoot, file, sample_files, valid};
    use crate::adr::{ADR_DIR, adr_path};

    #[test]
    fn front_matter_reads_every_field() {
        let text = "---\nid: ADR-0003\ntitle: \"Wire types: checked on connect\"\n\
                    summary: \"Every wire has a static type.\"\ndate: 2026-11-02\n\
                    supersedes: ADR-0001, ADR-0002\n---\n\n# ADR-0003\n";
        assert_eq!(
            parse_front_matter(text),
            Ok(FrontMatter {
                number: 3,
                title: "Wire types: checked on connect".into(),
                summary: "Every wire has a static type.".into(),
                date: "2026-11-02".into(),
                supersedes: vec![1, 2],
            })
        );
    }

    #[test]
    fn quoted_values_continue_on_indented_lines() {
        let text = "---\nid: ADR-0003\ntitle: \"Wire types\"\nsummary: \"Every wire has a\n  \
                    static type,\n    and connecting checks it.\"\ndate: 2026-11-02\n---\n";
        let front_matter = parse_front_matter(text).unwrap();
        assert_eq!(front_matter.summary, "Every wire has a static type, and connecting checks it.");
    }

    #[test]
    fn front_matter_without_block_fails() {
        assert_eq!(parse_front_matter("# ADR-0003\n"), Err(vec![ProblemKind::NoFrontMatter]));
        assert_eq!(
            parse_front_matter("---\nid: ADR-0003\n"),
            Err(vec![ProblemKind::NoFrontMatter])
        );
    }

    #[test]
    fn missing_directories_and_tags_count_as_empty() {
        let root = TempRoot::new("missing");
        assert_eq!(read_adr_files(&root.0).unwrap(), (vec![], None));
    }

    #[test]
    fn files_are_read_from_both_directories() {
        let root = TempRoot::new("read");
        let files = [
            valid(Superseded, 1, &[]),
            valid(Active, 3, &[]),
            valid(Active, 2, &[1]),
        ];
        for adr in &files {
            root.write_adr(adr);
        }
        root.write(&format!("{ADR_DIR}/tags.yaml"), "ADR-0001: graph\n");
        root.write(&format!("{ADR_DIR}/active/notes/ignored.md"), "a directory, not an ADR\n");
        assert_eq!(
            read_adr_files(&root.0).unwrap(),
            (
                vec![
                    valid(Active, 2, &[1]),
                    valid(Active, 3, &[]),
                    valid(Superseded, 1, &[])
                ],
                Some("ADR-0001: graph\n".to_string())
            )
        );
    }

    #[test]
    fn loading_leaves_out_unreadable_adrs_and_keeps_untagged_ones() {
        let mut files = sample_files();
        files.push(file(Active, "0004-broken.md", "No front matter.\n"));
        files.push(valid(Active, 5, &[]));
        let loaded = load(files, Some(SAMPLE_TAGS));
        assert_eq!(
            loaded.warnings,
            [format!(
                "{}: front matter can't be read, so it's left out; `cargo xtask check-docs` has \
                 the details",
                adr_path(Active, "0004-broken.md")
            )]
        );
        let strings =
            |items: &[&str]| -> Vec<String> { items.iter().map(ToString::to_string).collect() };
        let numbers_and_tags: Vec<(u32, Vec<String>)> =
            loaded.adrs.iter().map(|adr| (adr.front_matter.number, adr.tags.clone())).collect();
        assert_eq!(
            numbers_and_tags,
            [
                (1, strings(&["graph", "dynamic-typing"])),
                (2, strings(&["file-format"])),
                (3, strings(&["graph", "wires"])),
                (5, strings(&[])),
            ]
        );
    }
}
