//! `check-ready`: the branch is ready to merge into `main`.
//!
//! `docs/wip/` is empty or missing, so branch notes don't reach `main`.

use std::fmt;
use std::fs;
use std::path::Path;

use crate::CheckResult;

const WIP_DIR: &str = "docs/wip";

/// A way the branch isn't ready to merge.
#[derive(Debug, PartialEq, Eq)]
enum Problem {
    /// A file named `name` is in `docs/wip/`.
    BranchNotes { name: String },
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BranchNotes { name } => write!(
                f,
                "{WIP_DIR}/{name}: remove branch notes before merge \
                 (see docs/guides/development.md#branch-notes)"
            ),
        }
    }
}

pub(crate) fn run(root: &Path) -> CheckResult {
    let mut notes = Vec::new();
    let wip_dir = root.join(WIP_DIR);
    if wip_dir.is_dir() {
        for entry in fs::read_dir(wip_dir)? {
            notes.push(entry?.file_name().to_string_lossy().into_owned());
        }
        notes.sort();
    }
    Ok(check(&notes).iter().map(ToString::to_string).collect())
}

/// Checks the names of the files in `docs/wip/`.
fn check(notes: &[String]) -> Vec<Problem> {
    notes.iter().map(|name| Problem::BranchNotes { name: name.clone() }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_file_in_wip_is_reported() {
        let notes = ["README.md".to_string(), "plan.md".to_string()];
        assert_eq!(
            check(&notes),
            [
                Problem::BranchNotes { name: "README.md".into() },
                Problem::BranchNotes { name: "plan.md".into() },
            ]
        );
    }

    #[test]
    fn missing_wip_directory_passes() {
        // The crate's own directory stands in for a repository without `docs/wip/`.
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert_eq!(run(root).unwrap(), Vec::<String>::new());
    }
}
