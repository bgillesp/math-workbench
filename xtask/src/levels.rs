//! `check-levels`: each crate under `crates/` depends only on crates at strictly lower levels.
//!
//! A crate declares its level in its manifest:
//!
//! ```toml
//! [package.metadata.mw]
//! level = 0
//! ```
//!
//! Every kind of dependency counts, including dev- and build-dependencies.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use cargo_metadata::{DependencyKind, MetadataCommand};

use crate::CheckResult;

/// A workspace crate, reduced to what the level check needs.
#[derive(Debug)]
struct Crate {
    name: String,
    /// `None` if the manifest has no valid `[package.metadata.mw] level`.
    level: Option<u64>,
    /// The workspace crates this one depends on, with the kind of dependency.
    deps: Vec<(String, &'static str)>,
}

pub(crate) fn run(root: &Path) -> CheckResult {
    let metadata =
        MetadataCommand::new().manifest_path(root.join("Cargo.toml")).no_deps().exec()?;
    let crates_dir = metadata.workspace_root.join("crates");
    let members = metadata.workspace_packages();
    let member_names: HashSet<&str> = members.iter().map(|p| p.name.as_str()).collect();
    let crates: Vec<Crate> = members
        .iter()
        .filter(|p| p.manifest_path.starts_with(&crates_dir))
        .map(|p| Crate {
            name: p.name.to_string(),
            level: p.metadata["mw"]["level"].as_u64(),
            deps: p
                .dependencies
                .iter()
                .filter(|d| member_names.contains(d.name.as_str()))
                .map(|d| (d.name.clone(), kind_name(d.kind)))
                .collect(),
        })
        .collect();
    Ok(check(&crates))
}

fn kind_name(kind: DependencyKind) -> &'static str {
    match kind {
        DependencyKind::Development => "dev-dependency",
        DependencyKind::Build => "build-dependency",
        _ => "dependency",
    }
}

fn check(crates: &[Crate]) -> Vec<String> {
    let levels: HashMap<&str, Option<u64>> =
        crates.iter().map(|c| (c.name.as_str(), c.level)).collect();
    let mut problems = Vec::new();
    for krate in crates {
        let Some(level) = krate.level else {
            problems.push(format!(
                "{}: declare `[package.metadata.mw] level = N` (a non-negative integer) in its \
                 Cargo.toml",
                krate.name
            ));
            continue;
        };
        for (dep, kind) in &krate.deps {
            match levels.get(dep.as_str()) {
                Some(Some(dep_level)) if *dep_level >= level => problems.push(format!(
                    "{} (level {level}) has a {kind} on {dep} (level {dep_level}); crates may \
                     depend only on crates at strictly lower levels",
                    krate.name
                )),
                // A lower level is fine, and a missing level is reported for that crate.
                Some(_) => {},
                None => problems.push(format!(
                    "{} has a {kind} on {dep}, which is outside crates/ and has no level",
                    krate.name
                )),
            }
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    fn krate(name: &str, level: Option<u64>, deps: &[(&str, &'static str)]) -> Crate {
        Crate {
            name: name.to_string(),
            level,
            deps: deps.iter().map(|&(d, k)| (d.to_string(), k)).collect(),
        }
    }

    #[test]
    fn lower_level_dependency_passes() {
        let crates = [
            krate("mw-math", Some(0), &[]),
            krate("mw-graph", Some(1), &[("mw-math", "dependency")]),
        ];
        assert_eq!(check(&crates), Vec::<String>::new());
    }

    #[test]
    fn equal_level_dependency_fails() {
        let crates = [
            krate("mw-a", Some(1), &[("mw-b", "dependency")]),
            krate("mw-b", Some(1), &[]),
        ];
        let problems = check(&crates);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].starts_with("mw-a (level 1) has a dependency on mw-b (level 1)"));
    }

    #[test]
    fn higher_level_dependency_fails() {
        let crates = [
            krate("mw-math", Some(0), &[("mw-app", "dependency")]),
            krate("mw-app", Some(5), &[]),
        ];
        assert_eq!(check(&crates).len(), 1);
    }

    #[test]
    fn dev_dependencies_count() {
        let crates = [
            krate("mw-math", Some(0), &[("mw-testkit", "dev-dependency")]),
            krate("mw-testkit", Some(2), &[]),
        ];
        let problems = check(&crates);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("dev-dependency"));
    }

    #[test]
    fn missing_level_is_reported_once() {
        let crates = [
            krate("mw-math", None, &[]),
            krate("mw-graph", Some(1), &[("mw-math", "dependency")]),
        ];
        let problems = check(&crates);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].starts_with("mw-math: declare"));
    }

    #[test]
    fn dependency_outside_crates_dir_is_reported() {
        let crates = [krate("mw-math", Some(0), &[("xtask", "dependency")])];
        let problems = check(&crates);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("outside crates/"));
    }
}
