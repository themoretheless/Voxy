//! Validated deterministic phase plans. Execution is serial until scoped access
//! enforcement for other resources and worker dispatch are implemented.
//! run_scene enforces the scene grant; other declarations describe intent.
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemAccess {
    pub resource: String,
    pub write: bool,
}
#[derive(Clone, Debug)]
pub struct SystemSpec {
    pub name: String,
    pub phase: u16,
    pub after: Vec<String>,
    pub access: Vec<SystemAccess>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScheduleError {
    Invalid(String),
    Cycle,
    Execution { system: String, reason: String },
}
impl std::fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "schedule error: {self:?}")
    }
}
impl std::error::Error for ScheduleError {}
/// A typed failure retains both the system identity and its original error.
#[derive(Debug)]
pub struct SystemFailure<E> {
    pub system: String,
    pub error: E,
}
impl<E: std::fmt::Display> std::fmt::Display for SystemFailure<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "system {} failed: {}", self.system, self.error)
    }
}
impl<E: std::error::Error + 'static> std::error::Error for SystemFailure<E> {}
/// Scene capability borrowed only for one scheduled system invocation.
/// The private grant prevents callbacks from constructing a writable capability.
#[derive(Debug)]
pub struct SceneSystemAccess<'a> {
    scene: &'a mut crate::SceneGraph,
    grants: &'a BTreeMap<String, bool>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SceneAccessDenied;
impl std::fmt::Display for SceneAccessDenied {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("system has no declared scene access for this operation")
    }
}
impl std::error::Error for SceneAccessDenied {}
/// Domain access rejection; no resource mutation is authorized by this error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceAccessDenied {
    pub resource: String,
    pub write: bool,
}
impl std::fmt::Display for ResourceAccessDenied {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "undeclared {} access to {}",
            if self.write { "write" } else { "read" },
            self.resource
        )
    }
}
impl std::error::Error for ResourceAccessDenied {}
impl SceneSystemAccess<'_> {
    /// Checks a domain declaration before its owner exposes an operation.
    /// # Errors
    /// Rejects an undeclared resource read; writes also imply read access.
    pub fn require_read(&self, resource: &str) -> Result<(), ResourceAccessDenied> {
        if self.grants.contains_key(resource) {
            Ok(())
        } else {
            Err(ResourceAccessDenied {
                resource: resource.into(),
                write: false,
            })
        }
    }
    /// # Errors
    /// Rejects absent or read-only declarations before domain mutation.
    pub fn require_write(&self, resource: &str) -> Result<(), ResourceAccessDenied> {
        if self.grants.get(resource) == Some(&true) {
            Ok(())
        } else {
            Err(ResourceAccessDenied {
                resource: resource.into(),
                write: true,
            })
        }
    }

    /// # Errors
    /// Rejects reads when the system declares no scene access.
    pub fn read(&self) -> Result<&crate::SceneGraph, SceneAccessDenied> {
        self.require_read("scene")
            .map_err(|_| SceneAccessDenied)
            .map(|()| &*self.scene)
    }
    /// # Errors
    /// Rejects mutation unless the system declares scene write access.
    pub fn write(&mut self) -> Result<&mut crate::SceneGraph, SceneAccessDenied> {
        if self.grants.get("scene") == Some(&true) {
            Ok(self.scene)
        } else {
            Err(SceneAccessDenied)
        }
    }
}
/// Systems in a batch are dependency-independent and have no declared write
/// conflicts. Batches are ordered barriers; lower phases always finish first.
/// Scene declarations grant capabilities only through run_scene. Other resource
/// declarations and the legacy run methods do not enforce borrowing.
#[derive(Debug)]
pub struct SchedulePlan {
    batches: Vec<Vec<String>>,
    resource_grants: BTreeMap<String, BTreeMap<String, bool>>,
}
fn conflicts(a: &SystemSpec, b: &SystemSpec) -> bool {
    a.access.iter().any(|a| {
        b.access
            .iter()
            .any(|b| a.resource == b.resource && (a.write || b.write))
    })
}
impl SchedulePlan {
    /// # Errors
    /// Rejects exceeded system limits, duplicate names/access entries, unknown
    /// dependencies, later-phase dependencies and cycles before execution.
    pub fn build(specs: &[SystemSpec], max_systems: usize) -> Result<Self, ScheduleError> {
        if specs.len() > max_systems {
            return Err(ScheduleError::Invalid("system limit exceeded".into()));
        }
        let mut names = BTreeMap::new();
        for (index, spec) in specs.iter().enumerate() {
            if spec.name.is_empty() || names.insert(spec.name.as_str(), index).is_some() {
                return Err(ScheduleError::Invalid(
                    "empty or duplicate system name".into(),
                ));
            }
            let mut resources = BTreeSet::new();
            for access in &spec.access {
                if access.resource.is_empty() || !resources.insert(access.resource.as_str()) {
                    return Err(ScheduleError::Invalid(format!(
                        "duplicate/empty resource in {}",
                        spec.name
                    )));
                }
            }
        }
        let mut dependencies = vec![Vec::new(); specs.len()];
        for (index, spec) in specs.iter().enumerate() {
            for dependency in &spec.after {
                let parent = *names.get(dependency.as_str()).ok_or_else(|| {
                    ScheduleError::Invalid(format!("unknown dependency {dependency}"))
                })?;
                if specs[parent].phase > spec.phase {
                    return Err(ScheduleError::Invalid("dependency in later phase".into()));
                }
                dependencies[index].push(parent);
            }
        }
        let mut done = vec![false; specs.len()];
        let mut remaining = specs.len();
        let mut batches = Vec::new();
        while remaining > 0 {
            let phase = specs
                .iter()
                .enumerate()
                .filter(|(i, _)| !done[*i])
                .map(|(_, s)| s.phase)
                .min()
                .ok_or(ScheduleError::Cycle)?;
            let mut batch = Vec::new();
            for (index, spec) in specs.iter().enumerate() {
                if done[index]
                    || spec.phase != phase
                    || !dependencies[index].iter().all(|parent| done[*parent])
                {
                    continue;
                }
                if batch.iter().all(|chosen| !conflicts(spec, &specs[*chosen])) {
                    batch.push(index);
                }
            }
            if batch.is_empty() {
                return Err(ScheduleError::Cycle);
            }
            remaining -= batch.len();
            for index in &batch {
                done[*index] = true;
            }
            batches.push(
                batch
                    .into_iter()
                    .map(|index| specs[index].name.clone())
                    .collect(),
            );
        }
        let resource_grants = specs
            .iter()
            .map(|spec| {
                (
                    spec.name.clone(),
                    spec.access
                        .iter()
                        .map(|access| (access.resource.clone(), access.write))
                        .collect(),
                )
            })
            .collect();
        Ok(Self {
            batches,
            resource_grants,
        })
    }
    /// Composes validated plans as ordered barriers, without merging batches.
    /// # Errors
    /// Rejects total system capacity overflow and duplicate system names.
    pub fn compose(plans: &[&Self], max_systems: usize) -> Result<Self, ScheduleError> {
        let mut batches = Vec::new();
        let mut resource_grants = BTreeMap::new();
        for plan in plans {
            for (system, grants) in &plan.resource_grants {
                if resource_grants.len() >= max_systems {
                    return Err(ScheduleError::Invalid(
                        "composed system limit exceeded".into(),
                    ));
                }
                if resource_grants
                    .insert(system.clone(), grants.clone())
                    .is_some()
                {
                    return Err(ScheduleError::Invalid("duplicate composed system".into()));
                }
            }
            batches.extend(plan.batches.iter().cloned());
        }
        Ok(Self {
            batches,
            resource_grants,
        })
    }
    #[must_use]
    pub fn batches(&self) -> &[Vec<String>] {
        &self.batches
    }
    /// Runs serially with scene access restricted to each system's declaration.
    /// Other resources and captures remain the caller's responsibility.
    /// # Errors
    /// Stops on the first typed system failure, preserving earlier effects.
    pub fn run_scene<E>(
        &self,
        scene: &mut crate::SceneGraph,
        mut execute: impl FnMut(&str, SceneSystemAccess<'_>) -> Result<(), E>,
    ) -> Result<(), SystemFailure<E>> {
        self.run_typed(|system| {
            execute(
                system,
                SceneSystemAccess {
                    scene,
                    grants: &self.resource_grants[system],
                },
            )
        })
    }
    /// Executes serially in deterministic batch/insertion order. Stops on the
    /// first failure; already completed system effects are not rolled back.
    /// # Errors
    /// Reports the failing system and its supplied error message.
    pub fn run(
        &self,
        mut execute: impl FnMut(&str) -> Result<(), String>,
    ) -> Result<(), ScheduleError> {
        self.run_typed(&mut execute)
            .map_err(|failure| ScheduleError::Execution {
                system: failure.system,
                reason: failure.error,
            })
    }
    /// Executes the validated plan serially, preserving the original error type.
    /// # Errors
    /// Stops at the first failed system; previous effects remain committed.
    pub fn run_typed<E>(
        &self,
        mut execute: impl FnMut(&str) -> Result<(), E>,
    ) -> Result<(), SystemFailure<E>> {
        for batch in &self.batches {
            for system in batch {
                execute(system).map_err(|error| SystemFailure {
                    system: system.clone(),
                    error,
                })?;
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn system(name: &str, phase: u16, after: &[&str], write: bool) -> SystemSpec {
        SystemSpec {
            name: name.into(),
            phase,
            after: after.iter().map(|s| (*s).into()).collect(),
            access: vec![SystemAccess {
                resource: "health".into(),
                write,
            }],
        }
    }
    #[test]
    fn phases_dependencies_and_conflicts_form_deterministic_batches() {
        let plan = SchedulePlan::build(
            &[
                system("read", 1, &[], false),
                system("write", 0, &[], true),
                system("read2", 1, &[], false),
                system("extract", 2, &["read"], false),
            ],
            4,
        )
        .unwrap();
        assert_eq!(
            plan.batches(),
            &[
                vec!["write".to_owned()],
                vec!["read".to_owned(), "read2".to_owned()],
                vec!["extract".to_owned()]
            ]
        );
        let plan = SchedulePlan::build(&[system("a", 0, &[], true), system("b", 0, &[], true)], 2)
            .unwrap();
        assert_eq!(plan.batches().len(), 2);
    }
    #[test]
    fn cycles_unknown_dependencies_and_errors_do_not_execute_downstream() {
        assert!(SchedulePlan::build(&[system("a", 0, &["missing"], false)], 1).is_err());
        assert!(matches!(
            SchedulePlan::build(
                &[system("a", 0, &["b"], false), system("b", 0, &["a"], false)],
                2
            ),
            Err(ScheduleError::Cycle)
        ));
        let plan = SchedulePlan::build(&[system("a", 0, &[], true), system("b", 1, &[], true)], 2)
            .unwrap();
        let mut calls = Vec::new();
        assert!(
            plan.run(|name| {
                calls.push(name.to_owned());
                Err("failed".into())
            })
            .is_err()
        );
        assert_eq!(calls, vec!["a"]);
    }
}
