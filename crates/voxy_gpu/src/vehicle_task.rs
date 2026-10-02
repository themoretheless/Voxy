//! Atomic vehicle evaluation around asynchronous chassis collision queries.
use crate::{CharacterGpuQueryError, PendingGpuCharacter, VoxelRegionProgram};
use physics_voxel::{VehicleConfig, VehicleError, VehicleInput, VehicleState, VehicleStep};
use voxy_world::{BlockRegistry, VoxelView};

/// Local velocity and displacement supplied by a chassis integrator.
pub type ChassisMotion = ([f64; 3], [f64; 3]);

#[derive(Debug)]
pub enum VehicleGpuError {
    Vehicle(VehicleError),
    Character(physics::CharacterError<CharacterGpuQueryError>),
    Consumed,
    Pending,
    Integration(String),
}
impl From<VehicleError> for VehicleGpuError {
    fn from(value: VehicleError) -> Self {
        Self::Vehicle(value)
    }
}
impl std::fmt::Display for VehicleGpuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GPU vehicle step: {self:?}")
    }
}
impl std::error::Error for VehicleGpuError {}

/// Retains the original vehicle until all chassis contacts complete.
#[derive(Debug)]
pub struct PendingGpuVehicle {
    initial: VehicleState,
    input: VehicleInput,
    dt: f64,
    config: VehicleConfig,
    registry: BlockRegistry,
    chassis: Option<PendingGpuCharacter>,
    consumed: bool,
}
impl PendingGpuVehicle {
    /// Chunks retained by the asynchronous chassis controller.
    pub fn captured_chunks(&self) -> impl Iterator<Item = voxy_core::ChunkPos> + '_ {
        self.chassis
            .iter()
            .flat_map(PendingGpuCharacter::captured_chunks)
    }

    #[must_use]
    pub fn new(
        state: VehicleState,
        input: VehicleInput,
        dt: f64,
        config: VehicleConfig,
        registry: &BlockRegistry,
    ) -> Self {
        Self {
            initial: state,
            input,
            dt,
            config,
            registry: registry.clone(),
            chassis: None,
            consumed: false,
        }
    }

    /// Polls without waiting. The returned candidate is the only published state.
    /// # Errors
    /// Rejects invalid vehicle input, stale collision data and repeated consumption.
    pub fn try_step(
        &mut self,
        program: &VoxelRegionProgram,
        queue: &wgpu::Queue,
        view: &impl VoxelView,
    ) -> Result<Option<(VehicleState, VehicleStep)>, VehicleGpuError> {
        self.try_step_with_integrator(program, queue, view, |_, _, _, _| Ok(None))
    }

    /// Integrates chassis motion once, retaining it across collision polls.
    /// Returning `None` uses the controller's ordinary Euler integration.
    /// # Errors
    /// Reports integration, vehicle, collision and consumption errors.
    pub fn try_step_with_integrator(
        &mut self,
        program: &VoxelRegionProgram,
        queue: &wgpu::Queue,
        view: &impl VoxelView,
        mut integrate: impl FnMut(
            &physics_voxel::CharacterState,
            physics::CharacterInput,
            f64,
            physics::CharacterConfig,
        ) -> Result<Option<ChassisMotion>, String>,
    ) -> Result<Option<(VehicleState, VehicleStep)>, VehicleGpuError> {
        if self.consumed {
            return Err(VehicleGpuError::Consumed);
        }
        let mut candidate = self.initial;
        let result = physics_voxel::step_vehicle_with_chassis_step(
            &mut candidate,
            self.input,
            self.dt,
            self.config,
            |state, input, dt, config| {
                if self.chassis.is_none() {
                    let motion = integrate(state, input, dt, config)
                        .map_err(VehicleGpuError::Integration)?;
                    let mut task = PendingGpuCharacter::new(
                        physics::CharacterState {
                            body: physics::AnchoredAabb {
                                anchor: physics::Origin {
                                    x: state.body.anchor.x,
                                    y: state.body.anchor.y,
                                    z: state.body.anchor.z,
                                },
                                min: state.body.min,
                                max: state.body.max,
                            },
                            velocity: state.velocity,
                            grounded: state.grounded,
                        },
                        input,
                        dt,
                        config,
                        &self.registry,
                    );
                    if let Some((velocity, displacement)) = motion {
                        task = task.with_motion(velocity, displacement);
                    }
                    self.chassis = Some(task);
                }
                let task = self.chassis.as_mut().ok_or(VehicleGpuError::Consumed)?;
                let Some((next, report)) = task
                    .try_step(program, queue, view)
                    .map_err(VehicleGpuError::Character)?
                else {
                    return Err(VehicleGpuError::Pending);
                };
                *state = physics_voxel::CharacterState {
                    body: physics_voxel::AnchoredAabb {
                        anchor: voxy_core::VoxelPos {
                            x: next.body.anchor.x,
                            y: next.body.anchor.y,
                            z: next.body.anchor.z,
                        },
                        min: next.body.min,
                        max: next.body.max,
                    },
                    velocity: next.velocity,
                    grounded: next.grounded,
                };
                Ok(report)
            },
        );
        match result {
            Err(VehicleGpuError::Pending) => Ok(None),
            result => {
                self.consumed = true;
                result.map(|report| Some((candidate, report)))
            }
        }
    }
}
