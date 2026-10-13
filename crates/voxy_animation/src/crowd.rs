//! Batched evaluation of many [`GraphPlayer`]s sharing one graph, with
//! per-instance level of detail. Failures are isolated per instance; the
//! palette buffer stays instance-major for direct upload.

use std::sync::Arc;

use glam::Mat4;

use crate::anim_graph::{AnimGraph, GraphPlayer, ParamId, ParamValue};
use crate::{AnimationError, GraphFrame};

/// Evaluation detail for one instance.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CrowdLod {
    /// Every tick, full graph.
    #[default]
    Full,
    /// Every other tick; the palette is held in between.
    Half,
    /// Every fourth tick.
    Quarter,
    /// Frozen: no evaluation, palette held.
    Frozen,
}

impl CrowdLod {
    fn period(self) -> u64 {
        match self {
            Self::Full => 1,
            Self::Half => 2,
            Self::Quarter => 4,
            Self::Frozen => u64::MAX,
        }
    }
}

/// One instance's outcome for a tick.
#[derive(Clone, Debug, PartialEq)]
pub enum CrowdOutcome {
    /// Evaluated this tick.
    Updated,
    /// Skipped by LOD; palette held.
    Held,
    /// Evaluation failed; palette held and the player state unchanged.
    Failed(AnimationError),
}

#[derive(Clone, Debug)]
struct Instance {
    player: GraphPlayer,
    lod: CrowdLod,
    /// Accumulated time since the last evaluation, so held ticks are not lost.
    pending_dt: f64,
}

/// Owns the players and an instance-major palette buffer.
#[derive(Clone, Debug)]
pub struct CrowdEvaluator {
    graph: Arc<AnimGraph>,
    joint_count: usize,
    instances: Vec<Instance>,
    palettes: Vec<Mat4>,
    tick: u64,
}

/// Instance-major view of the palettes after a tick.
#[derive(Clone, Copy, Debug)]
pub struct CrowdPalettes<'a> {
    pub joint_count: usize,
    pub matrices: &'a [Mat4],
}

impl<'a> CrowdPalettes<'a> {
    #[must_use]
    pub fn instance(&self, index: usize) -> Option<&'a [Mat4]> {
        let start = index.checked_mul(self.joint_count)?;
        self.matrices.get(start..start + self.joint_count)
    }
}

impl CrowdEvaluator {
    /// # Errors
    /// Propagates the first player's initialization error.
    pub fn new(graph: Arc<AnimGraph>, count: usize) -> Result<Self, AnimationError> {
        let probe = GraphPlayer::new(graph.clone())?;
        let mut probe_clone = probe.clone();
        let joint_count = probe_clone.advance(0.0)?.skin_matrices.len();
        let instances = (0..count)
            .map(|_| Instance {
                player: probe.clone(),
                lod: CrowdLod::Full,
                pending_dt: 0.0,
            })
            .collect();
        Ok(Self {
            graph,
            joint_count,
            instances,
            palettes: vec![Mat4::IDENTITY; joint_count * count],
            tick: 0,
        })
    }

    #[must_use]
    pub fn graph(&self) -> &Arc<AnimGraph> {
        &self.graph
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.instances.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    #[must_use]
    pub fn joint_count(&self) -> usize {
        self.joint_count
    }

    #[must_use]
    pub fn player(&self, index: usize) -> Option<&GraphPlayer> {
        self.instances.get(index).map(|i| &i.player)
    }

    /// # Errors
    /// Rejects an unknown instance or an invalid parameter.
    pub fn set_param(
        &mut self,
        index: usize,
        id: ParamId,
        value: ParamValue,
    ) -> Result<(), AnimationError> {
        self.instances
            .get_mut(index)
            .ok_or(AnimationError::InvalidGraphNode(index))?
            .player
            .set_param(id, value)
    }

    /// # Errors
    /// Rejects an unknown instance.
    pub fn set_lod(&mut self, index: usize, lod: CrowdLod) -> Result<(), AnimationError> {
        self.instances
            .get_mut(index)
            .ok_or(AnimationError::InvalidGraphNode(index))?
            .lod = lod;
        Ok(())
    }

    /// Advances every due instance by its accumulated time and refreshes its
    /// palette. Instances are evaluated on worker threads when there are
    /// enough of them; results are written in instance order.
    ///
    /// # Errors
    /// Rejects a nonfinite or negative `dt`; per-instance failures are
    /// reported in the returned outcomes instead.
    pub fn advance_all(&mut self, dt: f64) -> Result<Vec<CrowdOutcome>, AnimationError> {
        if !dt.is_finite() || dt < 0.0 {
            return Err(AnimationError::InvalidAnimationTimeStep);
        }
        self.tick += 1;
        let tick = self.tick;
        let joint_count = self.joint_count;
        let due: Vec<bool> = self
            .instances
            .iter_mut()
            .map(|instance| {
                instance.pending_dt += dt;
                let period = instance.lod.period();
                period != u64::MAX && tick.is_multiple_of(period)
            })
            .collect();
        let count = self.instances.len();
        let worker_count = std::thread::available_parallelism()
            .map_or(1, usize::from)
            .min(count.max(1));
        let mut outcomes = vec![CrowdOutcome::Held; count];
        let use_workers = worker_count > 1 && due.iter().filter(|d| **d).count() > 8;
        let results: Vec<Option<Result<GraphFrame, AnimationError>>> = if use_workers {
            let chunk = count.div_ceil(worker_count);
            let mut gathered: Vec<Option<Result<GraphFrame, AnimationError>>> =
                Vec::with_capacity(count);
            std::thread::scope(|scope| {
                let handles: Vec<_> = self
                    .instances
                    .chunks_mut(chunk)
                    .zip(due.chunks(chunk))
                    .map(|(instances, due)| {
                        scope.spawn(move || {
                            instances
                                .iter_mut()
                                .zip(due)
                                .map(|(instance, &due)| due.then(|| step_instance(instance)))
                                .collect::<Vec<_>>()
                        })
                    })
                    .collect();
                for handle in handles {
                    gathered.extend(handle.join().expect("crowd worker thread panicked"));
                }
            });
            gathered
        } else {
            self.instances
                .iter_mut()
                .zip(&due)
                .map(|(instance, &due)| due.then(|| step_instance(instance)))
                .collect()
        };
        for (index, result) in results.into_iter().enumerate() {
            match result {
                None => {}
                Some(Ok(frame)) => {
                    let start = index * joint_count;
                    self.palettes[start..start + joint_count].copy_from_slice(&frame.skin_matrices);
                    outcomes[index] = CrowdOutcome::Updated;
                }
                Some(Err(error)) => outcomes[index] = CrowdOutcome::Failed(error),
            }
        }
        Ok(outcomes)
    }

    #[must_use]
    pub fn palettes(&self) -> CrowdPalettes<'_> {
        CrowdPalettes {
            joint_count: self.joint_count,
            matrices: &self.palettes,
        }
    }
}

/// Advances one instance by its pending time, bounded to the graph's
/// per-tick limit by splitting into equal substeps.
fn step_instance(instance: &mut Instance) -> Result<GraphFrame, AnimationError> {
    let total = instance.pending_dt;
    let substeps = (total / 1.0).ceil().max(1.0) as usize;
    let step = total / substeps as f64;
    let mut frame = None;
    for _ in 0..substeps {
        frame = Some(instance.player.advance(step)?);
    }
    instance.pending_dt = 0.0;
    frame.ok_or(AnimationError::InvalidAnimationTimeStep)
}

#[cfg(test)]
mod tests;
