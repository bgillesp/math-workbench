//! Fixtures shared by the tests of the `adr` modules.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::Dir::{Active, Superseded};
use super::new::{Args, TEMPLATE};
use super::records::{Adr, load};
use super::{ADR_DIR, AdrFile, Dir};

pub(super) fn file(dir: Dir, name: &str, text: &str) -> AdrFile {
    AdrFile { dir, name: name.to_string(), text: text.to_string() }
}

/// An ADR file with the given front matter lines.
pub(super) fn with_front_matter(dir: Dir, name: &str, front_matter: &str) -> AdrFile {
    file(dir, name, &format!("---\n{front_matter}---\n\n# Body\n"))
}

/// A well-formed ADR numbered `n`, named `000n-decision-n.md`.
pub(super) fn valid(dir: Dir, n: u32, supersedes: &[u32]) -> AdrFile {
    let mut front_matter = format!(
        "id: ADR-{n:04}\ntitle: \"Decision {n}\"\nsummary: \"Summary {n}.\"\ndate: 2026-11-02\n"
    );
    if !supersedes.is_empty() {
        let ids: Vec<String> = supersedes.iter().map(|s| format!("ADR-{s:04}")).collect();
        front_matter.push_str(&format!("supersedes: {}\n", ids.join(", ")));
    }
    with_front_matter(dir, &format!("{n:04}-decision-{n}.md"), &front_matter)
}

/// `tags.yaml` giving each numbered ADR the tag `graph`.
pub(super) fn tags_for(numbers: &[u32]) -> String {
    numbers.iter().map(|n| format!("ADR-{n:04}: graph\n")).collect()
}

/// Three ADRs: 0001 superseded by 0003, and 0002 and 0003 in force.
pub(super) fn sample_files() -> Vec<AdrFile> {
    vec![
        with_front_matter(
            Active,
            "0002-versioned-project-files.md",
            "id: ADR-0002\ntitle: \"Versioned project files\"\nsummary: \"Project files carry a \
             format version, and older versions are migrated when loaded.\"\ndate: 2026-11-01\n",
        ),
        with_front_matter(
            Active,
            "0003-wire-types-checked-on-connect.md",
            "id: ADR-0003\ntitle: \"Wire types: checked on connect\"\nsummary: \"Every wire has a \
             static type, and connecting two ports checks compatibility, so a mistyped \
             connection is rejected before anything is evaluated.\"\ndate: 2026-11-02\n\
             supersedes: ADR-0001\n",
        ),
        with_front_matter(
            Superseded,
            "0001-wires-are-dynamically-typed.md",
            "id: ADR-0001\ntitle: \"Wires are dynamically typed\"\nsummary: \"Wires carry values \
             of any type, checked when a tile reads them.\"\ndate: 2026-10-30\n",
        ),
    ]
}

/// The tags of the sample ADRs.
pub(super) const SAMPLE_TAGS: &str =
    "ADR-0001: graph, dynamic-typing\nADR-0002: file-format\nADR-0003: graph, wires\n";

/// The sample ADRs, loaded for the views.
pub(super) fn sample() -> Vec<Adr> {
    let loaded = load(sample_files(), Some(SAMPLE_TAGS));
    assert_eq!(loaded.warnings, Vec::<String>::new());
    loaded.adrs
}

/// A scratch repository under the system temp directory, removed when dropped.
pub(super) struct TempRoot(
    /// The root directory.
    pub(super) PathBuf,
);

impl TempRoot {
    /// A new, empty scratch repository; `name` keeps tests from sharing one.
    pub(super) fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("xtask-adr-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    /// Writes a file at a repository-relative path, creating its directories.
    pub(super) fn write(&self, relative: &str, text: &str) {
        let target = self.0.join(relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, text).unwrap();
    }

    /// Reads the file at a repository-relative path.
    pub(super) fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.0.join(relative)).unwrap()
    }

    /// Every file under the root, by repository-relative path.
    pub(super) fn snapshot(&self) -> BTreeMap<String, String> {
        fn walk(base: &Path, dir: &Path, files: &mut BTreeMap<String, String>) {
            for entry in fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap().path();
                if entry.is_dir() {
                    walk(base, &entry, files);
                } else {
                    let relative = entry.strip_prefix(base).unwrap();
                    let relative = relative.to_string_lossy().replace('\\', "/");
                    files.insert(relative, fs::read_to_string(&entry).unwrap());
                }
            }
        }
        let mut files = BTreeMap::new();
        walk(&self.0, &self.0, &mut files);
        files
    }

    /// Writes an ADR file at its place under `ADR_DIR`.
    pub(super) fn write_adr(&self, adr: &AdrFile) {
        self.write(&adr.path(), &adr.text);
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A short ADR template, in the shape of `docs/templates/adr.md`.
pub(super) const TEMPLATE_TEXT: &str = "<!--\nCreate ADRs with `cargo xtask adr new`.\n-->\n\n\
                                        # ADR-NNNN: Decision title\n\n## Context\n\n\
                                        The problem.\n";

/// A scratch repository with the ADR template, ADRs 0001 and 0002 in force, and their tags.
pub(super) fn repository(name: &str) -> TempRoot {
    let root = TempRoot::new(name);
    root.write(TEMPLATE, TEMPLATE_TEXT);
    root.write_adr(&valid(Active, 1, &[]));
    root.write_adr(&valid(Active, 2, &[]));
    root.write(&format!("{ADR_DIR}/tags.yaml"), "ADR-0001: graph\nADR-0002: file-format\n");
    root
}

/// Arguments for `adr new`.
pub(super) fn new_args(title: &str, tags: &[&str], supersedes: &[u32]) -> Args {
    Args {
        title: title.to_string(),
        tags: tags.iter().map(ToString::to_string).collect(),
        supersedes: supersedes.to_vec(),
    }
}
