//! `check-levels`: each crate under `crates/` depends only on crates at strictly lower levels.
//!
//! A crate declares its level in its manifest:
//!
//! ```toml
//! [package.metadata.mw]
//! level = 0
//! ```
//!
//! Every kind of dependency counts, including dev- and build-dependencies. Workspace crates
//! outside `crates/`, such as `xtask`, have no level, so crates under `crates/` can't depend on
//! them.

use std::collections::{HashMap, HashSet};
use std::fmt;
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
    deps: Vec<(String, DependencyKind)>,
}

/// A way a crate breaks the level rules.
#[derive(Debug, PartialEq, Eq)]
enum Problem {
    /// The crate has no valid `[package.metadata.mw] level`.
    MissingLevel { krate: String },
    /// The crate depends on a crate at the same or a higher level.
    DepNotLower { krate: String, level: u64, kind: DependencyKind, dep: String, dep_level: u64 },
    /// The crate depends on a workspace crate outside `crates/`.
    DepOutsideCrates { krate: String, kind: DependencyKind, dep: String },
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingLevel { krate } => write!(
                f,
                "{krate}: declare `[package.metadata.mw] level = N` (a non-negative integer) in \
                 its Cargo.toml"
            ),
            Self::DepNotLower { krate, level, kind, dep, dep_level } => write!(
                f,
                "{krate} (level {level}) has a {} on {dep} (level {dep_level}); crates may depend \
                 only on crates at strictly lower levels",
                kind_name(*kind)
            ),
            Self::DepOutsideCrates { krate, kind, dep } => write!(
                f,
                "{krate} has a {} on {dep}, which is outside crates/ and so has no level",
                kind_name(*kind)
            ),
        }
    }
}

pub(crate) fn run(root: &Path) -> CheckResult {
    // Fetch cargo metadata from Cargo.toml in specified root directory.
    let metadata =
        MetadataCommand::new().manifest_path(root.join("Cargo.toml")).no_deps().exec()?;

    // Member packages of the workspace.
    let members = metadata.workspace_packages();
    let member_names: HashSet<&str> = members.iter().map(|p| p.name.as_str()).collect();

    // Crate structs representing only members in the monitored `crates/` subdirectory.
    let crates_dir = metadata.workspace_root.join("crates");
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
                .map(|d| (d.name.clone(), d.kind))
                .collect(),
        })
        .collect();

    // Report any problems as messages.
    Ok(check(&crates).iter().map(ToString::to_string).collect())
}

fn kind_name(kind: DependencyKind) -> &'static str {
    match kind {
        DependencyKind::Development => "dev-dependency",
        DependencyKind::Build => "build-dependency",
        _ => "dependency",
    }
}

fn check(crates: &[Crate]) -> Vec<Problem> {
    // Dependency levels of all crates in the `crates/` subdirectory.
    let levels: HashMap<&str, Option<u64>> =
        crates.iter().map(|c| (c.name.as_str(), c.level)).collect();

    // Record any dependency problems for crates in `crates/` subdirectory.
    let mut problems = Vec::new();
    for krate in crates {
        let Some(level) = krate.level else {
            // Crate does not declare a dependency level.
            problems.push(Problem::MissingLevel { krate: krate.name.clone() });
            continue;
        };
        for (dep, kind) in &krate.deps {
            match levels.get(dep.as_str()) {
                // Crate depends on a workspace crate with equal or greater dependency level.
                Some(&Some(dep_level)) if dep_level >= level => {
                    problems.push(Problem::DepNotLower {
                        krate: krate.name.clone(),
                        level,
                        kind: *kind,
                        dep: dep.clone(),
                        dep_level,
                    });
                },
                // A lower level is fine, and a missing level is reported for the dependency.
                Some(_) => {},
                // Crate depends on a workspace crate outside of `crates/`.
                None => {
                    problems.push(Problem::DepOutsideCrates {
                        krate: krate.name.clone(),
                        kind: *kind,
                        dep: dep.clone(),
                    });
                },
            }
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;
    use DependencyKind::{Build, Development, Normal};

    fn krate(name: &str, level: Option<u64>, deps: &[(&str, DependencyKind)]) -> Crate {
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
            krate("mw-graph", Some(1), &[("mw-math", Normal)]),
        ];
        assert_eq!(check(&crates), Vec::<Problem>::new());
    }

    #[test]
    fn equal_level_dependency_fails() {
        let crates = [
            krate("mw-a", Some(1), &[("mw-b", Normal)]),
            krate("mw-b", Some(1), &[]),
        ];
        assert_eq!(
            check(&crates),
            [Problem::DepNotLower {
                krate: "mw-a".into(),
                level: 1,
                kind: Normal,
                dep: "mw-b".into(),
                dep_level: 1,
            }]
        );
    }

    #[test]
    fn higher_level_dependency_fails() {
        let crates = [
            krate("mw-math", Some(0), &[("mw-app", Normal)]),
            krate("mw-app", Some(5), &[]),
        ];
        assert_eq!(
            check(&crates),
            [Problem::DepNotLower {
                krate: "mw-math".into(),
                level: 0,
                kind: Normal,
                dep: "mw-app".into(),
                dep_level: 5,
            }]
        );
    }

    #[test]
    fn dev_dependencies_count() {
        let crates = [
            krate("mw-math", Some(0), &[("mw-testkit", Development)]),
            krate("mw-testkit", Some(2), &[]),
        ];
        assert_eq!(
            check(&crates),
            [Problem::DepNotLower {
                krate: "mw-math".into(),
                level: 0,
                kind: Development,
                dep: "mw-testkit".into(),
                dep_level: 2,
            }]
        );
    }

    #[test]
    fn missing_level_is_reported_once() {
        let crates = [
            krate("mw-math", None, &[]),
            krate("mw-graph", Some(1), &[("mw-math", Normal)]),
        ];
        assert_eq!(check(&crates), [Problem::MissingLevel { krate: "mw-math".into() }]);
    }

    #[test]
    fn dependency_outside_crates_dir_is_reported() {
        let crates = [krate("mw-math", Some(0), &[("xtask", Normal)])];
        assert_eq!(
            check(&crates),
            [Problem::DepOutsideCrates {
                krate: "mw-math".into(),
                kind: Normal,
                dep: "xtask".into(),
            }]
        );
    }

    #[test]
    fn problems_display_as_messages() {
        assert_eq!(
            Problem::MissingLevel { krate: "mw-math".into() }.to_string(),
            "mw-math: declare `[package.metadata.mw] level = N` (a non-negative integer) in its \
             Cargo.toml"
        );
        assert_eq!(
            Problem::DepNotLower {
                krate: "mw-math".into(),
                level: 0,
                kind: Development,
                dep: "mw-testkit".into(),
                dep_level: 2,
            }
            .to_string(),
            "mw-math (level 0) has a dev-dependency on mw-testkit (level 2); crates may depend \
             only on crates at strictly lower levels"
        );
        assert_eq!(
            Problem::DepOutsideCrates { krate: "mw-math".into(), kind: Build, dep: "xtask".into() }
                .to_string(),
            "mw-math has a build-dependency on xtask, which is outside crates/ and so has no level"
        );
    }
}
