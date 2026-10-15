//! Validated deterministic phase plans. Execution is serial until scoped access
//! enforcement for other resources and worker dispatch are implemented.
//! `run_scene` enforces the scene grant; other declarations describe intent.
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
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("schedule error: {self:?}")]
pub enum ScheduleError {
    Invalid(String),
    Cycle,
    Execution { system: String, reason: String },
}
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
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("system has no declared scene access for this operation")]
pub struct SceneAccessDenied;
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

/// Immutable capability token for a scheduled system invocation in parallel or serial batches.
#[derive(Clone, Copy, Debug)]
pub struct ScopedSystemAccess<'a> {
    grants: &'a BTreeMap<String, bool>,
}

impl<'a> ScopedSystemAccess<'a> {
    /// Checks a domain read declaration; write grants also satisfy read checks.
    /// # Errors
    /// Rejects an undeclared resource read.
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

    /// Checks a domain write declaration.
    /// # Errors
    /// Rejects absent or read-only declarations.
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

    #[must_use]
    pub fn can_read(&self, resource: &str) -> bool {
        self.grants.contains_key(resource)
    }

    #[must_use]
    pub fn can_write(&self, resource: &str) -> bool {
        self.grants.get(resource) == Some(&true)
    }

    /// Iterates over all declared `(resource, write)` grants for this system.
    pub fn declared_resources(&self) -> impl Iterator<Item = (&'a str, bool)> + '_ {
        self.grants.iter().map(|(k, &v)| (k.as_str(), v))
    }
}

/// Parallel-safe scene capability: provides shared read access to [`crate::SceneGraph`]
/// (when `"scene"` is declared) and an isolated per-system [`crate::SceneCommands`] buffer
/// that is committed in deterministic batch order at the end of the batch barrier.
#[derive(Debug)]
pub struct ParallelSceneSystemAccess<'a> {
    scene: &'a crate::SceneGraph,
    grants: &'a BTreeMap<String, bool>,
    commands: crate::SceneCommands,
}

impl ParallelSceneSystemAccess<'_> {
    /// # Errors
    /// Rejects an undeclared resource read.
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
    /// Rejects absent or read-only declarations.
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
    /// Rejects reads when the system declares no `"scene"` access.
    pub fn read(&self) -> Result<&crate::SceneGraph, SceneAccessDenied> {
        self.require_read("scene")
            .map_err(|_| SceneAccessDenied)
            .map(|()| self.scene)
    }

    /// Queues deferred structural/component commands to be applied at the end of the current batch
    /// barrier. Requires declared `"scene"` write or `"commands"` write access.
    /// # Errors
    /// Rejects command queueing when neither `"scene"` write nor `"commands"` write is declared.
    pub fn commands(&mut self) -> Result<&mut crate::SceneCommands, SceneAccessDenied> {
        if self.grants.get("scene") == Some(&true) || self.grants.get("commands") == Some(&true) {
            Ok(&mut self.commands)
        } else {
            Err(SceneAccessDenied)
        }
    }
}

/// Systems in a batch are dependency-independent and have no declared write
/// conflicts. Batches are ordered barriers; lower phases always finish first.
/// Scene declarations grant capabilities only through `run_scene` or `run_scene_parallel`.
#[derive(Clone, Debug)]
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
    /// Extends one existing system's resource contract, retaining all barriers.
    /// Newly conflicting systems are split into serial batches. Existing write
    /// grants are never downgraded by an added read declaration.
    /// # Errors
    /// Rejects an unknown system or an empty resource without changing this plan.
    pub fn with_resource_access(
        &self,
        system: &str,
        access: SystemAccess,
    ) -> Result<Self, ScheduleError> {
        if access.resource.is_empty() {
            return Err(ScheduleError::Invalid("empty resource".into()));
        }
        let mut next = self.clone();
        let grants = next
            .resource_grants
            .get_mut(system)
            .ok_or_else(|| ScheduleError::Invalid(format!("unknown system {system}")))?;
        *grants.entry(access.resource).or_default() |= access.write;
        let mut batches = Vec::new();
        for batch in &self.batches {
            let mut pending: Vec<String> = Vec::new();
            for name in batch {
                let grants = &next.resource_grants[name];
                let conflict = pending.iter().any(|other| {
                    grants.iter().any(|(resource, write)| {
                        next.resource_grants[other]
                            .get(resource)
                            .is_some_and(|other_write| *write || *other_write)
                    })
                });
                if conflict {
                    batches.push(std::mem::take(&mut pending));
                }
                pending.push(name.clone());
            }
            if !pending.is_empty() {
                batches.push(pending);
            }
        }
        next.batches = batches;
        Ok(next)
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
    /// Returns the scoped resource grant for a registered system name.
    #[must_use]
    pub fn grant_for(&self, system: &str) -> Option<ScopedSystemAccess<'_>> {
        self.resource_grants
            .get(system)
            .map(|grants| ScopedSystemAccess { grants })
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

    /// Executes conflict-free topological batches concurrently using up to `max_workers`
    /// scoped worker threads per batch (`max_workers == 0` is treated as `1`).
    /// Batch boundaries remain strict barriers, and returned `(system, output)` pairs
    /// preserve the deterministic batch/insertion order regardless of thread completion order.
    /// # Errors
    /// If any system in a batch fails, returns the first failure in deterministic batch order
    /// and does not execute subsequent batches.
    pub fn run_parallel<T, E>(
        &self,
        max_workers: usize,
        execute: impl Fn(&str, ScopedSystemAccess<'_>) -> Result<T, E> + Sync,
    ) -> Result<Vec<(String, T)>, SystemFailure<E>>
    where
        T: Send,
        E: Send,
    {
        let workers = max_workers.max(1);
        let mut total_systems = 0;
        for batch in &self.batches {
            total_systems += batch.len();
        }
        let mut outputs = Vec::with_capacity(total_systems);
        for batch in &self.batches {
            if batch.len() <= 1 || workers == 1 {
                for system in batch {
                    let access = ScopedSystemAccess {
                        grants: &self.resource_grants[system],
                    };
                    let value = execute(system, access).map_err(|error| SystemFailure {
                        system: system.clone(),
                        error,
                    })?;
                    outputs.push((system.clone(), value));
                }
            } else {
                for chunk in batch.chunks(workers) {
                    let chunk_results: Vec<Result<T, E>> = std::thread::scope(|scope| {
                        let handles: Vec<_> = chunk
                            .iter()
                            .map(|system| {
                                let grants = &self.resource_grants[system];
                                let exec = &execute;
                                scope.spawn(move || {
                                    exec(system.as_str(), ScopedSystemAccess { grants })
                                })
                            })
                            .collect();
                        handles
                            .into_iter()
                            .map(|h| h.join().expect("scheduled system thread panicked"))
                            .collect()
                    });
                    for (system, res) in chunk.iter().zip(chunk_results) {
                        let value = res.map_err(|error| SystemFailure {
                            system: system.clone(),
                            error,
                        })?;
                        outputs.push((system.clone(), value));
                    }
                }
            }
        }
        Ok(outputs)
    }

    /// Executes batches in parallel with shared read access to `scene` and per-system
    /// [`crate::SceneCommands`] queues that are applied to `scene` in deterministic
    /// system order at the end of each batch barrier.
    /// # Errors
    /// Stops at the first failed system barrier and returns its typed failure.
    pub fn run_scene_parallel<E>(
        &self,
        scene: &mut crate::SceneGraph,
        max_workers: usize,
        command_capacity_per_system: usize,
        execute: impl Fn(&str, &mut ParallelSceneSystemAccess<'_>) -> Result<(), E> + Sync,
    ) -> Result<Vec<(String, Vec<Result<(), crate::SceneGraphError>>)>, SystemFailure<E>>
    where
        E: Send,
    {
        let workers = max_workers.max(1);
        let mut applied_results = Vec::new();
        for batch in &self.batches {
            let mut batch_commands = Vec::with_capacity(batch.len());
            {
                let shared_scene: &crate::SceneGraph = &*scene;
                if batch.len() <= 1 || workers == 1 {
                    for system in batch {
                        let mut access = ParallelSceneSystemAccess {
                            scene: shared_scene,
                            grants: &self.resource_grants[system],
                            commands: crate::SceneCommands::new(
                                shared_scene,
                                command_capacity_per_system,
                            ),
                        };
                        execute(system, &mut access).map_err(|error| SystemFailure {
                            system: system.clone(),
                            error,
                        })?;
                        batch_commands.push((system.clone(), access.commands));
                    }
                } else {
                    for chunk in batch.chunks(workers) {
                        let chunk_results: Vec<Result<crate::SceneCommands, E>> =
                            std::thread::scope(|scope| {
                                let handles: Vec<_> = chunk
                                    .iter()
                                    .map(|system| {
                                        let grants = &self.resource_grants[system];
                                        let exec = &execute;
                                        scope.spawn(move || {
                                            let mut access = ParallelSceneSystemAccess {
                                                scene: shared_scene,
                                                grants,
                                                commands: crate::SceneCommands::new(
                                                    shared_scene,
                                                    command_capacity_per_system,
                                                ),
                                            };
                                            exec(system.as_str(), &mut access)?;
                                            Ok(access.commands)
                                        })
                                    })
                                    .collect();
                                handles
                                    .into_iter()
                                    .map(|h| h.join().expect("scheduled system thread panicked"))
                                    .collect()
                            });
                        for (system, res) in chunk.iter().zip(chunk_results) {
                            let cmds = res.map_err(|error| SystemFailure {
                                system: system.clone(),
                                error,
                            })?;
                            batch_commands.push((system.clone(), cmds));
                        }
                    }
                }
            }
            for (system, mut commands) in batch_commands {
                let results = commands
                    .apply(scene)
                    .expect("commands created from scene must match scene identity");
                applied_results.push((system, results));
            }
        }
        Ok(applied_results)
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

#[cfg(test)]
mod extended_access_tests {
    use super::*;
    #[test]
    fn extending_grants_splits_conflicts_without_relaxing_existing_plan() {
        let plan = SchedulePlan::build(
            &[
                SystemSpec {
                    name: "a".into(),
                    phase: 0,
                    after: vec![],
                    access: vec![SystemAccess {
                        resource: "animation".into(),
                        write: true,
                    }],
                },
                SystemSpec {
                    name: "b".into(),
                    phase: 0,
                    after: vec![],
                    access: vec![],
                },
            ],
            2,
        )
        .unwrap();
        assert_eq!(plan.batches().len(), 1);
        let extended = plan
            .with_resource_access(
                "b",
                SystemAccess {
                    resource: "animation".into(),
                    write: true,
                },
            )
            .unwrap();
        assert_eq!(
            extended.batches(),
            &[vec!["a".to_string()], vec!["b".to_string()]]
        );
        let reread = extended
            .with_resource_access(
                "b",
                SystemAccess {
                    resource: "animation".into(),
                    write: false,
                },
            )
            .unwrap();
        let mut scene = crate::SceneGraph::new(0);
        reread
            .run_scene(&mut scene, |_, access| access.require_write("animation"))
            .unwrap();
        assert_eq!(plan.batches().len(), 1);
        assert!(
            plan.with_resource_access(
                "missing",
                SystemAccess {
                    resource: "animation".into(),
                    write: true
                }
            )
            .is_err()
        );
        assert!(
            plan.with_resource_access(
                "b",
                SystemAccess {
                    resource: String::new(),
                    write: true
                }
            )
            .is_err()
        );
    }

    #[test]
    fn parallel_batches_enforce_grants_and_commit_commands_deterministically() {
        let plan = SchedulePlan::build(
            &[
                SystemSpec {
                    name: "spawn_alpha".into(),
                    phase: 0,
                    after: vec![],
                    access: vec![
                        SystemAccess {
                            resource: "scene".into(),
                            write: false,
                        },
                        SystemAccess {
                            resource: "commands".into(),
                            write: false,
                        },
                        SystemAccess {
                            resource: "alpha_state".into(),
                            write: true,
                        },
                    ],
                },
                SystemSpec {
                    name: "spawn_beta".into(),
                    phase: 0,
                    after: vec![],
                    access: vec![
                        SystemAccess {
                            resource: "scene".into(),
                            write: false,
                        },
                        SystemAccess {
                            resource: "commands".into(),
                            write: false,
                        },
                        SystemAccess {
                            resource: "beta_state".into(),
                            write: true,
                        },
                    ],
                },
                SystemSpec {
                    name: "verify_scene".into(),
                    phase: 1,
                    after: vec!["spawn_alpha".into(), "spawn_beta".into()],
                    access: vec![SystemAccess {
                        resource: "scene".into(),
                        write: false,
                    }],
                },
            ],
            4,
        )
        .unwrap();
        // Both phase-0 systems only read `scene` and `commands`, and write disjoint resources,
        // so they share the first batch.
        assert_eq!(plan.batches()[0], vec!["spawn_alpha", "spawn_beta"]);

        let results = plan
            .run_parallel(4, |name, access| -> Result<String, ResourceAccessDenied> {
                access.require_read("scene")?;
                if name == "spawn_alpha" {
                    access.require_write("alpha_state")?;
                    assert!(access.require_write("beta_state").is_err());
                } else if name == "spawn_beta" {
                    access.require_write("beta_state")?;
                    assert!(access.require_write("alpha_state").is_err());
                }
                Ok(format!("done:{name}"))
            })
            .unwrap();
        assert_eq!(
            results,
            vec![
                ("spawn_alpha".into(), "done:spawn_alpha".into()),
                ("spawn_beta".into(), "done:spawn_beta".into()),
                ("verify_scene".into(), "done:verify_scene".into()),
            ]
        );

        let plan_with_cmds = plan
            .with_resource_access(
                "spawn_alpha",
                SystemAccess {
                    resource: "commands".into(),
                    write: true,
                },
            )
            .unwrap();
        let mut scene = crate::SceneGraph::new(8);
        let cmd_outcomes = plan_with_cmds
            .run_scene_parallel(&mut scene, 4, 4, |name, access| -> Result<(), String> {
                let graph = access.read().map_err(|e| e.to_string())?;
                if name == "spawn_alpha" {
                    assert_eq!(graph.len(), 0);
                    let cmds = access.commands().map_err(|e| e.to_string())?;
                    cmds.push(crate::SceneCommand::Spawn(
                        None,
                        crate::Transform::default(),
                    ))
                    .map_err(|e| format!("{e:?}"))?;
                } else if name == "verify_scene" {
                    assert_eq!(graph.len(), 1);
                }
                Ok(())
            })
            .unwrap();
        assert_eq!(cmd_outcomes[0].0, "spawn_alpha");
        assert_eq!(cmd_outcomes[0].1.len(), 1);
        assert_eq!(scene.len(), 1);
    }
}
