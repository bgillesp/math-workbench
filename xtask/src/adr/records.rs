//! Loading the ADRs and `tags.yaml`. Each has a step that reads it from disk and a step that parses
//! it, running the structural checks, and `Records` brings the results together.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::io;
use std::path::Path;

use super::check::{Problem, ProblemKind};
use super::{ADR_DIR, AdrFile, Dir, TAGS_PATH, adr_path, is_tag, parse_id, split_tags, yaml};

/// The front matter fields every ADR must have.
const REQUIRED_FIELDS: [&str; 4] = ["id", "title", "summary", "date"];

/// The front matter fields an ADR may leave out.
const OPTIONAL_FIELDS: [&str; 1] = ["supersedes"];

/// An ADR whose file passed the structural checks.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Adr {
    /// The directory it's in, which says whether it's in force.
    pub(super) dir: Dir,
    /// The file name, such as `0003-wire-types-checked-on-connect.md`, which the consistency
    /// checks compare with `number`.
    pub(super) name: String,
    /// The number in `id`, which identifies the ADR.
    pub(super) number: u32,
    /// The value of `title`, which may be empty.
    pub(super) title: String,
    /// The value of `summary`, on one line, which may be empty.
    pub(super) summary: String,
    /// The value of `date`, which the consistency checks require to be a real date.
    pub(super) date: String,
    /// The numbers in `supersedes`.
    pub(super) supersedes: Vec<u32>,
    /// The tags in `tags.yaml` that list this ADR, sorted.
    pub(super) tags: Vec<String>,
}

impl Adr {
    /// The repository-relative path of its file.
    pub(super) fn path(&self) -> String {
        adr_path(self.dir, &self.name)
    }

    /// Whether it's in force, which its directory decides.
    pub(super) fn is_active(&self) -> bool {
        self.dir == Dir::Active
    }
}

/// Each tag in `tags.yaml`, with the numbers of the ADRs it lists.
pub(super) type Tags = BTreeMap<String, BTreeSet<u32>>;

/// Every ADR and tag, once they've all passed the structural checks.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Records {
    /// Every ADR, by number.
    pub(super) adrs: Vec<Adr>,
    /// Each tag in `tags.yaml`, with the numbers of the ADRs it lists.
    pub(super) tags: Tags,
}

impl Records {
    /// The records for `adrs` and `tags`, with the ADRs by number, each given its tags.
    pub(super) fn new(mut adrs: Vec<Adr>, tags: Tags) -> Self {
        for adr in &mut adrs {
            adr.tags = tags
                .iter()
                .filter(|(_, numbers)| numbers.contains(&adr.number))
                .map(|(tag, _)| tag.clone())
                .collect();
        }
        adrs.sort_by_key(|adr| adr.number);
        Self { adrs, tags }
    }

    /// The numbers of the ADRs whose `supersedes` lists `number`, in order.
    pub(super) fn successors(&self, number: u32) -> Vec<u32> {
        let mut successors: Vec<u32> = self
            .adrs
            .iter()
            .filter(|adr| adr.supersedes.contains(&number))
            .map(|adr| adr.number)
            .collect();
        successors.dedup();
        successors
    }
}

/// Reads every ADR and `tags.yaml`, or finds every structural problem with them.
pub(super) fn load_records(root: &Path) -> io::Result<Result<Records, Vec<Problem>>> {
    let adrs = parse_adr_files(&load_adr_files(root)?);
    let tags = parse_tags_file(&load_tags_file(root)?);
    Ok(match (adrs, tags) {
        (Ok(adrs), Ok(tags)) => Ok(Records::new(adrs, tags)),
        (adrs, tags) => Err(adrs.err().into_iter().chain(tags.err()).flatten().collect()),
    })
}

/// Reads just `tags.yaml`, or finds every structural problem with it.
pub(super) fn load_tags(root: &Path) -> io::Result<Result<Tags, Vec<Problem>>> {
    Ok(parse_tags_file(&load_tags_file(root)?))
}

/// Reads the files in `active/` and `superseded/`, in that order and sorted by name. A missing
/// directory counts as empty.
fn load_adr_files(root: &Path) -> io::Result<Vec<AdrFile>> {
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
    Ok(files)
}

/// Parses each of `files` into an `Adr`, or finds every structural problem with them.
pub(super) fn parse_adr_files(files: &[AdrFile]) -> Result<Vec<Adr>, Vec<Problem>> {
    let mut adrs = Vec::new();
    let mut problems = Vec::new();
    for file in files {
        match parse_adr(file) {
            Ok(adr) => adrs.push(adr),
            Err(kinds) => {
                problems.extend(kinds.into_iter().map(|kind| Problem { path: file.path(), kind }));
            },
        }
    }
    if problems.is_empty() { Ok(adrs) } else { Err(problems) }
}

/// Reads an ADR's front matter, or every structural problem with it.
fn parse_adr(file: &AdrFile) -> Result<Adr, Vec<ProblemKind>> {
    let Some(block) = front_matter_block(&file.text) else {
        return Err(vec![ProblemKind::NoFrontMatter]);
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
    let mut number = None;
    let mut title = None;
    let mut summary = None;
    let mut date = None;
    let mut supersedes = Vec::new();
    for yaml::Entry { key, value } in entries {
        let name = key.as_str();
        if !REQUIRED_FIELDS.contains(&name) && !OPTIONAL_FIELDS.contains(&name) {
            problems.push(ProblemKind::UnknownKey { key });
            continue;
        }
        if !keys.insert(key.clone()) {
            problems.push(ProblemKind::DuplicateKey { key });
            continue;
        }
        let text = value.text();
        match key.as_str() {
            "id" => match parse_id(text) {
                Some(parsed) => number = Some(parsed),
                None => problems.push(ProblemKind::BadId { value: text.to_string() }),
            },
            "title" | "summary" if !text.is_empty() && matches!(value, yaml::Value::Plain(_)) => {
                problems.push(ProblemKind::NotQuoted { key });
            },
            "title" => title = Some(text.to_string()),
            "summary" => summary = Some(text.to_string()),
            "date" => date = Some(text.to_string()),
            _ if text.is_empty() => problems.push(ProblemKind::EmptyField { key }),
            _ => {
                for entry in text.split(',').map(str::trim) {
                    match parse_id(entry) {
                        Some(parsed) => supersedes.push(parsed),
                        None => problems
                            .push(ProblemKind::BadSupersedesEntry { entry: entry.to_string() }),
                    }
                }
            },
        }
    }
    for key in REQUIRED_FIELDS {
        if !keys.contains(key) {
            problems.push(ProblemKind::MissingField { key: key.to_string() });
        }
    }
    if problems.is_empty()
        && let (Some(number), Some(title), Some(summary), Some(date)) =
            (number, title, summary, date)
    {
        return Ok(Adr {
            dir: file.dir,
            name: file.name.clone(),
            number,
            title,
            summary,
            date,
            supersedes,
            tags: Vec::new(),
        });
    }
    Err(problems)
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

/// Reads `tags.yaml`. A missing file counts as empty.
fn load_tags_file(root: &Path) -> io::Result<String> {
    match fs::read_to_string(root.join(TAGS_PATH)) {
        Ok(text) => Ok(text),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(err) => Err(at(TAGS_PATH, &err)),
    }
}

/// Parses the text of `tags.yaml` into a map from tag to ADR numbers, or finds every structural
/// problem with it.
pub(super) fn parse_tags_file(text: &str) -> Result<Tags, Vec<Problem>> {
    let (entries, errors) = yaml::read(text);
    let mut kinds: Vec<ProblemKind> = errors
        .into_iter()
        .map(|error| match error {
            yaml::Error::MalformedLine { line } => ProblemKind::TagsMalformedLine { line },
            yaml::Error::UnterminatedQuote { key } => ProblemKind::UnterminatedQuote { key },
            yaml::Error::QuoteOrBackslash { key } => ProblemKind::QuoteOrBackslash { key },
        })
        .collect();
    let mut tags = Tags::new();
    let mut entered = BTreeSet::new();
    for entry in &entries {
        let Some(number) = parse_id(&entry.key) else {
            let line = match &entry.value {
                yaml::Value::Plain(value) => format!("{}: {value}", entry.key),
                yaml::Value::Quoted(value) => format!("{}: \"{value}\"", entry.key),
            };
            kinds.push(ProblemKind::TagsMalformedLine { line });
            continue;
        };
        if !entered.insert(number) {
            kinds.push(ProblemKind::DuplicateTagsEntry { number });
        } else if !entry.value.text().is_empty() {
            for tag in split_tags(entry.value.text()) {
                if is_tag(tag) {
                    tags.entry(tag.to_string()).or_default().insert(number);
                } else {
                    kinds.push(ProblemKind::BadTag { number, tag: tag.to_string() });
                }
            }
        }
    }
    if kinds.is_empty() {
        return Ok(tags);
    }
    Err(kinds.into_iter().map(|kind| Problem { path: TAGS_PATH.to_string(), kind }).collect())
}

/// `err`, with the repository-relative path it happened at in its message.
fn at(path: &str, err: &io::Error) -> io::Error {
    io::Error::new(err.kind(), format!("{path}: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adr::Dir::{Active, Superseded};
    use crate::adr::adr_path;
    use crate::adr::check::ProblemKind::{
        BadId, BadSupersedesEntry, BadTag, DuplicateKey, DuplicateTagsEntry, EmptyField,
        MalformedLine, MissingField, NoFrontMatter, NotQuoted, QuoteOrBackslash, TagsMalformedLine,
        UnknownKey, UnterminatedQuote,
    };
    use crate::adr::fixtures::{
        SAMPLE_TAGS, TempRoot, file, records_from, sample_files, valid, with_front_matter,
    };

    /// The file name of the ADR that `record` and `kinds` read.
    const NAME: &str = "0001-decision.md";

    /// Well-formed values for the required fields other than `id`.
    const FIELDS: &str = "title: \"T\"\nsummary: \"S.\"\ndate: 2026-11-02\n";

    fn problem(path: &str, kind: ProblemKind) -> Problem {
        Problem { path: path.to_string(), kind }
    }

    /// Problems in a fixed order, so tests don't depend on the order checks run in.
    fn sorted(mut problems: Vec<Problem>) -> Vec<Problem> {
        problems.sort();
        problems
    }

    /// The record for one ADR with this front matter, which has to pass the structural checks.
    fn record(front_matter: &str) -> Adr {
        let file = with_front_matter(Active, NAME, front_matter);
        parse_adr_files(&[file]).unwrap().remove(0)
    }

    /// The kinds of the structural problems in one ADR with this front matter, each of which has
    /// to name its file.
    fn kinds(front_matter: &str) -> Vec<ProblemKind> {
        let file = with_front_matter(Active, NAME, front_matter);
        let problems = sorted(parse_adr_files(&[file]).unwrap_err());
        assert!(problems.iter().all(|problem| problem.path == adr_path(Active, NAME)));
        problems.into_iter().map(|problem| problem.kind).collect()
    }

    /// The kinds of the structural problems in this `tags.yaml`, each of which has to name it.
    fn tags_kinds(tags: &str) -> Vec<ProblemKind> {
        let problems = sorted(parse_tags_file(tags).unwrap_err());
        assert!(problems.iter().all(|problem| problem.path == TAGS_PATH));
        problems.into_iter().map(|problem| problem.kind).collect()
    }

    #[test]
    fn front_matter_reads_every_field() {
        let name = "0003-wire-types.md";
        let text = "---\nid: ADR-0003\ntitle: \"Wire types: checked on connect\"\n\
                    summary: \"Every wire has a static type.\"\ndate: 2026-11-02\n\
                    supersedes: ADR-0001, ADR-0002\n---\n\n# ADR-0003\n";
        assert_eq!(
            parse_adr_files(&[file(Active, name, text)]).unwrap(),
            [Adr {
                dir: Active,
                name: name.into(),
                number: 3,
                title: "Wire types: checked on connect".into(),
                summary: "Every wire has a static type.".into(),
                date: "2026-11-02".into(),
                supersedes: vec![1, 2],
                tags: vec![],
            }]
        );
    }

    #[test]
    fn quoted_values_continue_on_indented_lines() {
        let adr = record(
            "id: ADR-0001\ntitle: \"T\"\nsummary: \"Every wire has a\n  static type,\n    and \
             connecting checks it.\"\ndate: 2026-11-02\n",
        );
        assert_eq!(adr.summary, "Every wire has a static type, and connecting checks it.");
    }

    #[test]
    fn files_need_a_front_matter_block() {
        for text in ["# ADR-0001\n", "---\nid: ADR-0001\n"] {
            assert_eq!(
                parse_adr_files(&[file(Active, NAME, text)]),
                Err(vec![problem(&adr_path(Active, NAME), NoFrontMatter)]),
                "{text:?}"
            );
        }
    }

    #[test]
    fn reader_problems_and_unknown_keys_are_structural() {
        assert_eq!(
            kinds(&format!("id: ADR-0001\n{FIELDS}not a field\n")),
            [MalformedLine { line: "not a field".into() }]
        );
        assert_eq!(
            kinds(&format!("id: ADR-0001\n{FIELDS}area: graph\n")),
            [UnknownKey { key: "area".into() }]
        );
        assert_eq!(
            kinds(&format!("id: ADR-0001\n{FIELDS}summary: \"Again.\"\n")),
            [DuplicateKey { key: "summary".into() }]
        );
    }

    #[test]
    fn quoting_is_structural() {
        let front_matter = |title: &str, summary: &str| {
            format!("id: ADR-0001\ntitle: {title}\nsummary: {summary}\ndate: 2026-11-02\n")
        };
        assert_eq!(kinds(&front_matter("Unquoted", "\"S.\"")), [NotQuoted { key: "title".into() }]);
        assert_eq!(
            kinds(&front_matter("\"T\"", "Unquoted.")),
            [NotQuoted { key: "summary".into() }]
        );
        assert_eq!(
            kinds(&front_matter("\"T\"", "\"A \"quoted\" word.\"")),
            [QuoteOrBackslash { key: "summary".into() }]
        );
        assert_eq!(
            kinds(&front_matter("\"Back\\slash\"", "\"S.\"")),
            [QuoteOrBackslash { key: "title".into() }]
        );
        // One mistake is one problem, so the unreadable value doesn't also count as missing.
        assert_eq!(
            kinds(&front_matter("\"T\"", "\"Never closed.")),
            [UnterminatedQuote { key: "summary".into() }]
        );
    }

    #[test]
    fn missing_fields_are_structural() {
        let lines = [
            ("id", "id: ADR-0001\n"),
            ("title", "title: \"T\"\n"),
            ("summary", "summary: \"S.\"\n"),
            ("date", "date: 2026-11-02\n"),
        ];
        for (key, _) in lines {
            let front_matter: String =
                lines.iter().filter(|(other, _)| *other != key).map(|(_, line)| *line).collect();
            assert_eq!(kinds(&front_matter), [MissingField { key: key.into() }], "{key}");
        }
    }

    #[test]
    fn ids_have_to_parse() {
        for id in ["", "1", "0001", "ADR-1", "ADR-NNNN"] {
            assert_eq!(
                kinds(&format!("id: {id}\n{FIELDS}")),
                [BadId { value: id.into() }],
                "{id:?}"
            );
        }
    }

    #[test]
    fn supersedes_entries_have_to_be_ids() {
        assert_eq!(
            kinds(&format!("id: ADR-0002\n{FIELDS}supersedes: 1\n")),
            [BadSupersedesEntry { entry: "1".into() }]
        );
        assert_eq!(
            kinds(&format!("id: ADR-0002\n{FIELDS}supersedes: ADR-0001, x\n")),
            [BadSupersedesEntry { entry: "x".into() }]
        );
        // A blank `supersedes` stays a problem until the format allows it, and only a structural
        // check can see it, since an `Adr` with no successors looks the same.
        assert_eq!(
            kinds(&format!("id: ADR-0002\n{FIELDS}supersedes:\n")),
            [EmptyField { key: "supersedes".into() }]
        );
    }

    #[test]
    fn what_the_consistency_checks_cover_still_parses() {
        let long = "x".repeat(301);
        let files = [
            with_front_matter(
                Active,
                "notes.md",
                "id: ADR-0001\ntitle: \"\"\nsummary: \"\"\ndate: YYYY-MM-DD\nsupersedes: ADR-0009\n",
            ),
            with_front_matter(
                Superseded,
                "0001-again.md",
                &format!("id: ADR-0001\ntitle:\nsummary: \"{long}\"\ndate:\n"),
            ),
        ];
        assert_eq!(
            parse_adr_files(&files).unwrap(),
            [
                Adr {
                    dir: Active,
                    name: "notes.md".into(),
                    number: 1,
                    title: String::new(),
                    summary: String::new(),
                    date: "YYYY-MM-DD".into(),
                    supersedes: vec![9],
                    tags: vec![],
                },
                Adr {
                    dir: Superseded,
                    name: "0001-again.md".into(),
                    number: 1,
                    title: String::new(),
                    summary: long,
                    date: String::new(),
                    supersedes: vec![],
                    tags: vec![],
                },
            ]
        );
        assert_eq!(
            parse_tags_file("ADR-0008: stale\n").unwrap(),
            Tags::from([("stale".to_string(), BTreeSet::from([8]))])
        );
    }

    #[test]
    fn tags_read_as_a_map_from_tag_to_adrs() {
        assert_eq!(
            parse_tags_file(SAMPLE_TAGS).unwrap(),
            Tags::from([
                ("dynamic-typing".to_string(), BTreeSet::from([1])),
                ("file-format".to_string(), BTreeSet::from([2])),
                ("graph".to_string(), BTreeSet::from([1, 3])),
                ("wires".to_string(), BTreeSet::from([3])),
            ])
        );
    }

    #[test]
    fn adrs_come_back_by_number_with_sorted_tags() {
        let mut files = sample_files();
        files.push(valid(Active, 5, &[]));
        let records = records_from(&files, SAMPLE_TAGS);
        let strings =
            |items: &[&str]| -> Vec<String> { items.iter().map(ToString::to_string).collect() };
        let numbers_and_tags: Vec<(u32, Vec<String>)> =
            records.adrs.iter().map(|adr| (adr.number, adr.tags.clone())).collect();
        assert_eq!(
            numbers_and_tags,
            [
                (1, strings(&["dynamic-typing", "graph"])),
                (2, strings(&["file-format"])),
                (3, strings(&["graph", "wires"])),
                (5, strings(&[])),
            ]
        );
    }

    #[test]
    fn problems_in_tags_yaml_are_structural() {
        assert_eq!(tags_kinds("not a line\n"), [TagsMalformedLine { line: "not a line".into() }]);
        assert_eq!(
            tags_kinds("graph: ADR-0001\n"),
            [TagsMalformedLine { line: "graph: ADR-0001".into() }]
        );
        assert_eq!(
            tags_kinds("ADR-0001: \"never closed\n"),
            [UnterminatedQuote { key: "ADR-0001".into() }]
        );
        assert_eq!(
            tags_kinds("ADR-0001: \"a \"b\" c\"\n"),
            [QuoteOrBackslash { key: "ADR-0001".into() }]
        );
        assert_eq!(
            tags_kinds("ADR-0001: graph\nADR-0001: wires\n"),
            [DuplicateTagsEntry { number: 1 }]
        );
        assert_eq!(
            tags_kinds("ADR-0001: graph, Wires, two words\n"),
            [
                BadTag { number: 1, tag: "Wires".into() },
                BadTag { number: 1, tag: "two words".into() },
            ]
        );
    }

    #[test]
    fn one_run_reports_every_structural_problem() {
        let files = [
            file(Active, "0001-no-block.md", "# ADR-0001\n"),
            with_front_matter(
                Active,
                "0002-two-problems.md",
                "id: 2\ntitle: \"T\"\nsummary: \"S.\"\n",
            ),
            valid(Active, 3, &[]),
            with_front_matter(
                Superseded,
                "0004-unknown-key.md",
                &format!("id: ADR-0004\n{FIELDS}area: graph\n"),
            ),
        ];
        let root = TempRoot::new("every-problem");
        for adr in &files {
            root.write_adr(adr);
        }
        root.write(TAGS_PATH, "not a line\nADR-0003: Wires\n");
        let second = &adr_path(Active, "0002-two-problems.md");
        assert_eq!(
            sorted(load_records(&root.0).unwrap().unwrap_err()),
            sorted(vec![
                problem(&adr_path(Active, "0001-no-block.md"), NoFrontMatter),
                problem(second, BadId { value: "2".into() }),
                problem(second, MissingField { key: "date".into() }),
                problem(
                    &adr_path(Superseded, "0004-unknown-key.md"),
                    UnknownKey { key: "area".into() }
                ),
                problem(TAGS_PATH, TagsMalformedLine { line: "not a line".into() }),
                problem(TAGS_PATH, BadTag { number: 3, tag: "Wires".into() }),
            ])
        );
    }

    #[test]
    fn missing_directories_and_tags_count_as_empty() {
        let root = TempRoot::new("missing");
        assert_eq!(load_adr_files(&root.0).unwrap(), vec![]);
        assert_eq!(load_tags_file(&root.0).unwrap(), "");
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
        root.write(TAGS_PATH, "ADR-0001: graph\n");
        root.write(&format!("{ADR_DIR}/active/notes/ignored.md"), "a directory, not an ADR\n");
        assert_eq!(
            load_adr_files(&root.0).unwrap(),
            vec![
                valid(Active, 2, &[1]),
                valid(Active, 3, &[]),
                valid(Superseded, 1, &[])
            ]
        );
        assert_eq!(load_tags_file(&root.0).unwrap(), "ADR-0001: graph\n");
    }
}
