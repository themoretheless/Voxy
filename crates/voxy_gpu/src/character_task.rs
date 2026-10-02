//! Restartable pure controller evaluation with cached asynchronous collision queries.
use crate::{GpuSweepError, PendingVoxelSweep, VoxelRegionProgram};
use physics::{
    CharacterConfig, CharacterError, CharacterInput, CharacterState, CharacterStep, CollisionWorld,
};
use std::cell::{Cell, RefCell};
use voxy_world::{BlockRegistry, VoxelView};

#[derive(Debug)]
pub enum CharacterGpuQueryError {
    Pending,
    Gpu(GpuSweepError),
}
impl std::fmt::Display for CharacterGpuQueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GPU character query: {self:?}")
    }
}
impl std::error::Error for CharacterGpuQueryError {}
#[derive(Debug)]
struct Query {
    body: physics::AnchoredAabb,
    displacement: [f64; 3],
    budget: usize,
    pending: PendingVoxelSweep,
    result: Option<physics::SweepResult<physics_voxel::SweepObstacle>>,
}
/// A character step whose output is published atomically after every query completes.
#[derive(Debug)]
pub struct PendingGpuCharacter {
    initial: CharacterState,
    input: CharacterInput,
    dt: f64,
    config: CharacterConfig,
    registry: BlockRegistry,
    queries: Vec<Query>,
    consumed: bool,
    motion: Option<([f64; 3], [f64; 3])>,
}
impl PendingGpuCharacter {
    /// Footprints of every retained sweep, including completed queries used during replay.
    /// Positions may repeat; callers can collect them into a streaming pin set.
    pub fn captured_chunks(&self) -> impl Iterator<Item = voxy_core::ChunkPos> + '_ {
        self.queries
            .iter()
            .flat_map(|query| query.pending.captured_chunks())
    }

    #[must_use]
    pub fn new(
        state: CharacterState,
        input: CharacterInput,
        dt: f64,
        config: CharacterConfig,
        registry: &BlockRegistry,
    ) -> Self {
        Self {
            initial: state,
            input,
            dt,
            config,
            registry: registry.clone(),
            queries: Vec::new(),
            consumed: false,
            motion: None,
        }
    }
    /// Supplies externally integrated local velocity and displacement, retained across polls.
    /// The shared controller validates this motion before any collision query.
    #[must_use]
    pub fn with_motion(mut self, velocity: [f64; 3], displacement: [f64; 3]) -> Self {
        self.motion = Some((velocity, displacement));
        self
    }

    /// Number of completed sweeps retained for deterministic controller replay.
    #[must_use]
    pub fn completed_sweeps(&self) -> usize {
        self.queries
            .iter()
            .filter(|query| query.result.is_some())
            .count()
    }

    /// Replays only pure controller arithmetic; completed GPU queries are reused.
    /// Callers poll the native device or yield the browser event loop between calls.
    /// # Errors
    /// Rejects malformed controller inputs, stale world data and consumed results.
    pub fn try_step(
        &mut self,
        program: &VoxelRegionProgram,
        queue: &wgpu::Queue,
        view: &impl VoxelView,
    ) -> Result<
        Option<(CharacterState, CharacterStep<physics_voxel::SweepObstacle>)>,
        CharacterError<CharacterGpuQueryError>,
    > {
        if self.consumed {
            return Err(CharacterError::Sweep(CharacterGpuQueryError::Gpu(
                voxy_render::ComputeError::Consumed.into(),
            )));
        }
        if self.queries.iter().any(|q| !q.pending.is_current(view)) {
            self.consumed = true;
            return Err(CharacterError::Sweep(CharacterGpuQueryError::Gpu(
                GpuSweepError::StaleWorld,
            )));
        }
        let replay = Replay {
            queries: RefCell::new(&mut self.queries),
            cursor: Cell::new(0),
            program,
            queue,
            view,
            registry: &self.registry,
        };
        let mut next = self.initial;
        let result = if let Some((velocity, displacement)) = self.motion {
            physics::step_character_with_motion(
                &replay,
                &mut next,
                self.input,
                self.dt,
                self.config,
                velocity,
                displacement,
            )
        } else {
            physics::step_character(&replay, &mut next, self.input, self.dt, self.config)
        };
        match result {
            Err(CharacterError::Sweep(CharacterGpuQueryError::Pending)) => Ok(None),
            result => {
                self.consumed = true;
                result.map(|report| Some((next, report)))
            }
        }
    }
}
struct Replay<'a, V> {
    queries: RefCell<&'a mut Vec<Query>>,
    cursor: Cell<usize>,
    program: &'a VoxelRegionProgram,
    queue: &'a wgpu::Queue,
    view: &'a V,
    registry: &'a BlockRegistry,
}
impl<V: VoxelView> CollisionWorld for Replay<'_, V> {
    type Obstacle = physics_voxel::SweepObstacle;
    type Error = CharacterGpuQueryError;
    fn sweep_aabb(
        &self,
        body: physics::AnchoredAabb,
        displacement: [f64; 3],
        budget: usize,
    ) -> Result<physics::SweepResult<Self::Obstacle>, Self::Error> {
        let index = self.cursor.get();
        self.cursor.set(index + 1);
        let mut queries = self.queries.borrow_mut();
        if index == queries.len() {
            let pending = self
                .program
                .begin_sweep(
                    self.queue,
                    self.view,
                    self.registry,
                    physics_voxel::AnchoredAabb {
                        anchor: voxy_core::VoxelPos {
                            x: body.anchor.x,
                            y: body.anchor.y,
                            z: body.anchor.z,
                        },
                        min: body.min,
                        max: body.max,
                    },
                    displacement,
                    physics_voxel::SweepConfig {
                        max_candidate_voxels: budget,
                    },
                )
                .map_err(CharacterGpuQueryError::Gpu)?;
            queries.push(Query {
                body,
                displacement,
                budget,
                pending,
                result: None,
            });
        }
        let query = &mut queries[index];
        if query.body != body
            || query.displacement.map(f64::to_bits) != displacement.map(f64::to_bits)
            || query.budget != budget
        {
            return Err(CharacterGpuQueryError::Gpu(
                voxy_render::ComputeError::InvalidBuffer.into(),
            ));
        }
        if query.result.is_none() {
            query.result = query
                .pending
                .try_sweep(self.view)
                .map_err(CharacterGpuQueryError::Gpu)?
                .map(|r| physics::SweepResult {
                    fraction: r.fraction,
                    normal: r.normal,
                    obstacle: r.obstacle,
                });
        }
        query.result.ok_or(CharacterGpuQueryError::Pending)
    }
}
