//! `check-ready`: the branch is ready to merge into `main`.
//!
//! `docs/wip/` is empty or missing, so branch notes don't reach `main`, and every design doc is
//! `implemented`, since a design doc is frozen once its PR merges.

use std::fmt;
use std::fs;
use std::path::Path;

use crate::CheckResult;
use crate::docs::{self, DESIGN_DIR};

const WIP_DIR: &str = "docs/wip";

/// A way the branch isn't ready to merge.
#[derive(Debug, PartialEq, Eq)]
enum Problem {
    /// A file named `name` is in `docs/wip/`.
    BranchNotes { name: String },
    /// The design doc named `name` has a status other than `implemented`, or none.
    DesignNotImplemented { name: String, status: Option<String> },
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BranchNotes { name } => write!(
                f,
                "{WIP_DIR}/{name}: move branch notes to their permanent homes before merge \
                 (see docs/guides/development.md#branch-notes)"
            ),
            Self::DesignNotImplemented { name, status: Some(status) } => write!(
                f,
                "{DESIGN_DIR}/{name}: status is `{status}`; revise the doc to describe what was \
                 built and set it to `implemented` before merge"
            ),
            Self::DesignNotImplemented { name, status: None } => write!(
                f,
                "{DESIGN_DIR}/{name}: no status; revise the doc to describe what was built and \
                 set it to `implemented` before merge"
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
    let design_docs = docs::read_files(&root.join(DESIGN_DIR))?;
    Ok(check(&notes, &design_docs).iter().map(ToString::to_string).collect())
}

/// Checks the names of the files in `docs/wip/`, and the name and contents of each design doc.
fn check(notes: &[String], design_docs: &[(String, String)]) -> Vec<Problem> {
    let mut problems: Vec<Problem> =
        notes.iter().map(|name| Problem::BranchNotes { name: name.clone() }).collect();
    for (name, text) in design_docs {
        let status = docs::front_matter(text).and_then(|fields| fields.get("status").copied());
        if status != Some("implemented") {
            problems.push(Problem::DesignNotImplemented {
                name: name.clone(),
                status: status.map(str::to_string),
            });
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    fn design_doc(name: &str, status: &str) -> (String, String) {
        (name.to_string(), format!("---\nstatus: {status}\n---\n"))
    }

    #[test]
    fn ready_branch_passes() {
        let design_docs = [design_doc("0002-conventions.md", "implemented")];
        assert_eq!(check(&[], &design_docs), Vec::<Problem>::new());
    }

    #[test]
    fn branch_notes_fail() {
        let notes = ["README.md".to_string(), "plan.md".to_string()];
        assert_eq!(
            check(&notes, &[]),
            [
                Problem::BranchNotes { name: "README.md".into() },
                Problem::BranchNotes { name: "plan.md".into() },
            ]
        );
    }

    #[test]
    fn unimplemented_design_docs_fail() {
        let design_docs = [
            design_doc("0002-conventions.md", "implemented"),
            design_doc("0003-draft.md", "draft"),
            design_doc("0004-accepted.md", "accepted"),
            ("0005-no-status.md".to_string(), "# No front matter\n".to_string()),
        ];
        assert_eq!(
            check(&[], &design_docs),
            [
                Problem::DesignNotImplemented {
                    name: "0003-draft.md".into(),
                    status: Some("draft".into()),
                },
                Problem::DesignNotImplemented {
                    name: "0004-accepted.md".into(),
                    status: Some("accepted".into()),
                },
                Problem::DesignNotImplemented { name: "0005-no-status.md".into(), status: None },
            ]
        );
    }

    #[test]
    fn problems_display_as_messages() {
        assert_eq!(
            Problem::BranchNotes { name: "plan.md".into() }.to_string(),
            "docs/wip/plan.md: move branch notes to their permanent homes before merge (see \
             docs/guides/development.md#branch-notes)"
        );
        assert_eq!(
            Problem::DesignNotImplemented {
                name: "0003-draft.md".into(),
                status: Some("draft".into()),
            }
            .to_string(),
            "docs/design/0003-draft.md: status is `draft`; revise the doc to describe what was \
             built and set it to `implemented` before merge"
        );
        assert_eq!(
            Problem::DesignNotImplemented { name: "0005-no-status.md".into(), status: None }
                .to_string(),
            "docs/design/0005-no-status.md: no status; revise the doc to describe what was built \
             and set it to `implemented` before merge"
        );
    }
}
