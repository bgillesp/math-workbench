//! Reading ADRs: the files on disk, the front matter schema, and the tags in `tags.yaml`.

use std::io;
use std::path::Path;

use super::check::ProblemKind;
use super::{AdrFile, Dir};

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
    /// The directory the file is in.
    pub(super) dir: Dir,
    /// The repository-relative path of the file.
    pub(super) path: String,
    /// The ADR's front matter.
    pub(super) front_matter: FrontMatter,
    /// The ADR's tags, in the order `tags.yaml` lists them.
    pub(super) tags: Vec<String>,
    /// The numbers of the ADRs whose `supersedes` lists this one: none for an ADR in force, and
    /// one for a superseded ADR once `check-docs` passes.
    pub(super) successors: Vec<u32>,
    /// The file's full contents, front matter included.
    pub(super) text: String,
}

impl Adr {
    /// Whether the ADR is in force, which its directory decides.
    pub(super) fn is_active(&self) -> bool {
        self.dir == Dir::Active
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

/// Reads an ADR's front matter, or every problem that keeps it from being read.
pub(super) fn parse_front_matter(_text: &str) -> Result<FrontMatter, Vec<ProblemKind>> {
    todo!()
}

/// Reads the files in `active/` and `superseded/`, in that order and sorted by name, and the
/// contents of `tags.yaml`. A missing directory or tags file counts as empty.
pub(super) fn read_adr_files(_root: &Path) -> io::Result<(Vec<AdrFile>, Option<String>)> {
    todo!()
}

/// Reads the ADRs for the views, leaving out any whose front matter can't be read. An ADR with no
/// entry in `tags` has no tags.
pub(super) fn load(_files: &[AdrFile], _tags: Option<&str>) -> Loaded {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adr::ADR_DIR;
    use crate::adr::Dir::{Active, Superseded};
    use crate::adr::fixtures::{SAMPLE_TAGS, TempRoot, file, path, sample_files, valid};

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
        let loaded = load(&files, Some(SAMPLE_TAGS));
        assert_eq!(
            loaded.warnings,
            [format!(
                "{}: front matter can't be read, so it's left out; `cargo xtask check-docs` has \
                 the details",
                path(Active, "0004-broken.md")
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
