//! `check-wip`: `docs/wip/` is empty or missing, so branch notes don't reach `main`.

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
        .map(|name| {
            format!(
                "{WIP_DIR}/{name}: move branch notes to their permanent homes before merge \
                 (see docs/guides/development.md#branch-notes)"
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_directory_passes() {
        assert_eq!(check(&[]), Vec::<String>::new());
    }

    #[test]
    fn every_file_fails() {
        let names = ["README.md".to_string(), "plan.md".to_string()];
        let problems = check(&names);
        assert_eq!(problems.len(), 2);
        assert!(problems[0].starts_with("docs/wip/README.md:"));
        assert!(problems[1].starts_with("docs/wip/plan.md:"));
    }
}
