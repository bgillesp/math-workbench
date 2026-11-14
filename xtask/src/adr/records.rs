//! Loading the ADRs and `tags.yaml`. Each has a step that reads it from disk and a step that parses
//! it, running the structural checks, and `Records` brings the results together.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::Path;

use super::check::{Problem, ProblemKind};
use super::{ADR_DIR, AdrFile, Dir, TAGS_PATH, adr_path, is_tag, parse_id, parse_ids, yaml};

/// The front matter fields every ADR must have. `supersedes` is the only other one.
const REQUIRED_FIELDS: [&str; 4] = ["id", "title", "summary", "date"];

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
    let (mut fields, errors) = yaml::read(block);
    let mut problems: Vec<ProblemKind> = errors.into_iter().map(ProblemKind::Yaml).collect();
    // A value the reader couldn't read is missing from `fields`, so it's reported here too.
    for key in REQUIRED_FIELDS {
        if !fields.contains_key(key) {
            problems.push(ProblemKind::MissingField { key: key.to_string() });
        }
    }
    let [id, title, summary, date] = REQUIRED_FIELDS.map(|key| fields.remove(key));
    let supersedes = fields.remove("supersedes").unwrap_or_default();
    problems.extend(fields.into_keys().map(|key| ProblemKind::UnknownKey { key }));
    let number = id.as_deref().and_then(parse_id);
    if let (Some(id), None) = (&id, number) {
        problems.push(ProblemKind::BadId { value: id.clone() });
    }
    let (supersedes, bad) = parse_ids(&supersedes);
    problems.extend(
        bad.into_iter().map(|entry| ProblemKind::BadSupersedesEntry { entry: entry.to_string() }),
    );
    match (number, title, summary, date) {
        (Some(number), Some(title), Some(summary), Some(date)) if problems.is_empty() => Ok(Adr {
            dir: file.dir,
            name: file.name.clone(),
            number,
            title,
            summary,
            date,
            supersedes,
            tags: Vec::new(),
        }),
        _ => Err(problems),
    }
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
    let mut kinds: Vec<ProblemKind> = errors.into_iter().map(ProblemKind::Yaml).collect();
    let mut tags = Tags::new();
    for (tag, value) in entries {
        if !is_tag(&tag) {
            kinds.push(ProblemKind::BadTag { tag: tag.clone() });
        }
        let (numbers, bad) = parse_ids(&value);
        for entry in bad {
            kinds.push(ProblemKind::BadTagsEntry { tag: tag.clone(), entry: entry.to_string() });
        }
        tags.insert(tag, numbers.into_iter().collect());
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
        BadId, BadSupersedesEntry, BadTag, BadTagsEntry, MissingField, NoFrontMatter, UnknownKey,
        Yaml,
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
                    summary: Every wire has a static\n  type.\ndate: 2026-11-02\n\
                    supersedes: ADR-0001 ADR-0002\n---\n\n# ADR-0003\n";
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
            [Yaml(yaml::Error::MalformedLine {
                line: "not a field".into()
            })]
        );
        assert_eq!(
            kinds(&format!("id: ADR-0001\n{FIELDS}area: graph\n")),
            [UnknownKey { key: "area".into() }]
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
        // Ids are separated by spaces, so a comma stays with the id before it.
        assert_eq!(
            kinds(&format!("id: ADR-0003\n{FIELDS}supersedes: ADR-0001, ADR-0002\n")),
            [BadSupersedesEntry { entry: "ADR-0001,".into() }]
        );
        // An empty `supersedes`, as the template has it, lists no ADRs.
        let adr = record(&format!("id: ADR-0002\n{FIELDS}supersedes:\n"));
        assert_eq!(adr.supersedes, Vec::<u32>::new());
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
            parse_tags_file("stale: ADR-0008\n").unwrap(),
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
        // The header comment, digits after the first letter, and a tag with no ADRs are all fine.
        assert_eq!(
            parse_tags_file("# Each line is a tag.\nutf-8: ADR-0002\nunused:\n").unwrap(),
            Tags::from([
                ("unused".to_string(), BTreeSet::new()),
                ("utf-8".to_string(), BTreeSet::from([2])),
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
        // Two branches that add the same tag in different places merge cleanly, so the repeated
        // key has to fail rather than drop one branch's ADR from the tag.
        assert_eq!(
            tags_kinds("graph: ADR-0001\nwires: ADR-0002\ngraph: ADR-0003\n"),
            [Yaml(yaml::Error::DuplicateKey { key: "graph".into() })]
        );
        // A YAML parser would read `2026` as a number and `on` as `true`.
        assert_eq!(
            tags_kinds("Wires: ADR-0001\n2026: ADR-0001\non: ADR-0001\n"),
            [
                BadTag { tag: "2026".into() },
                BadTag { tag: "Wires".into() },
                BadTag { tag: "on".into() },
            ]
        );
        assert_eq!(
            tags_kinds("graph: ADR-0001, ADR-0002\n"),
            [BadTagsEntry { tag: "graph".into(), entry: "ADR-0001,".into() }]
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
        root.write(TAGS_PATH, "not a line\nWires: ADR-0003\n");
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
                problem(TAGS_PATH, Yaml(yaml::Error::MalformedLine { line: "not a line".into() })),
                problem(TAGS_PATH, BadTag { tag: "Wires".into() }),
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
        root.write(TAGS_PATH, "graph: ADR-0001\n");
        root.write(&format!("{ADR_DIR}/active/notes/ignored.md"), "a directory, not an ADR\n");
        assert_eq!(
            load_adr_files(&root.0).unwrap(),
            vec![
                valid(Active, 2, &[1]),
                valid(Active, 3, &[]),
                valid(Superseded, 1, &[])
            ]
        );
        assert_eq!(load_tags_file(&root.0).unwrap(), "graph: ADR-0001\n");
    }
}
