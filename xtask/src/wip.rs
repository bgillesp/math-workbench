//! `check-wip`: `docs/wip/` holds nothing but its README, so branch notes don't reach `main`.

use std::fs;
use std::path::Path;

use crate::CheckResult;

const WIP_DIR: &str = "docs/wip";

pub(crate) fn run(root: &Path) -> CheckResult {
    let dir = root.join(WIP_DIR);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(dir)? {
        names.push(entry?.file_name().to_string_lossy().into_owned());
    }
    names.sort();
    Ok(check(&names))
}

fn check(names: &[String]) -> Vec<String> {
    names
        .iter()
        .filter(|name| *name != "README.md")
        .map(|name| {
            format!(
                "{WIP_DIR}/{name}: move branch notes to their permanent homes before merge \
                 (see docs/guides/workflow.md#branch-notes)"
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readme_alone_passes() {
        assert_eq!(check(&["README.md".to_string()]), Vec::<String>::new());
    }

    #[test]
    fn notes_fail() {
        let names = ["README.md".to_string(), "plan.md".to_string()];
        let problems = check(&names);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].starts_with("docs/wip/plan.md:"));
    }
}
