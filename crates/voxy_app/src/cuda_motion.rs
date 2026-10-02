//! Hybrid CUDA integration with authoritative CPU voxel collision.
use physics_voxel::{
    CharacterConfig, CharacterError, CharacterInput, CharacterState, CharacterStep, VehicleError,
};
use voxy_cuda::{CudaCompute, CudaError, CudaProjectileInput};
use voxy_world::{BlockRegistry, VoxelView};

#[derive(Debug)]
pub enum MotionError {
    Cuda(CudaError),
    Character(CharacterError),
    Vehicle(VehicleError),
    Collision(String),
}
impl From<CudaError> for MotionError {
    fn from(value: CudaError) -> Self {
        Self::Cuda(value)
    }
}
impl From<CharacterError> for MotionError {
    fn from(value: CharacterError) -> Self {
        Self::Character(value)
    }
}
impl From<VehicleError> for MotionError {
    fn from(value: VehicleError) -> Self {
        Self::Vehicle(value)
    }
}
impl std::fmt::Display for MotionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "hardware motion error: {self:?}")
    }
}
impl std::error::Error for MotionError {}

/// Integrates local f64 velocity/displacement on CUDA, then executes the complete
/// CPU voxel controller. Input/jump selection and terminal falling clamp remain
/// on the CPU. No coordinate anchors are converted to device floats.
/// # Errors
/// CUDA or controller failures leave the original character/chassis unchanged.
#[allow(clippy::too_many_arguments)]
pub fn step_character(
    compute: &CudaCompute,
    view: &impl VoxelView,
    registry: &BlockRegistry,
    state: &mut CharacterState,
    input: CharacterInput,
    dt: f64,
    config: CharacterConfig,
) -> Result<CharacterStep, MotionError> {
    let motion = character_motion(compute, state, input, dt, config)?;
    Ok(physics_voxel::step_character_with_motion(
        view,
        registry,
        state,
        input,
        dt,
        config,
        motion.velocity,
        motion.displacement,
    )?)
}

/// Computes CUDA motion once so asynchronous collision can retain the exact result.
/// # Errors
/// Returns CUDA integration errors without changing the supplied character.
pub fn character_motion(
    compute: &CudaCompute,
    state: &CharacterState,
    input: CharacterInput,
    dt: f64,
    config: CharacterConfig,
) -> Result<voxy_cuda::CudaProjectileMotion, MotionError> {
    let mut velocity = state.velocity;
    velocity[0] = input.planar_velocity[0];
    velocity[2] = input.planar_velocity[1];
    if input.jump_pressed && state.grounded {
        velocity[1] = config.jump_speed;
    }
    let mut motion = compute
        .euler_motion(
            &[CudaProjectileInput {
                velocity,
                acceleration: [0.0, config.gravity, 0.0],
            }],
            dt,
        )?
        .into_iter()
        .next()
        .ok_or(CudaError::InvalidProjectileInput)?;
    motion.velocity[1] = motion.velocity[1].max(-config.terminal_fall_speed);
    motion.displacement[1] = motion.velocity[1] * dt;
    Ok(motion)
}

/// Executes the shared controller with CUDA broadphase and f64 contact kernels.
/// # Errors
/// CUDA/controller failures leave the original state unchanged; no CPU fallback.
#[allow(clippy::too_many_arguments)]
pub fn step_character_collision(
    compute: &CudaCompute,
    view: &impl VoxelView,
    registry: &BlockRegistry,
    state: &mut CharacterState,
    input: CharacterInput,
    dt: f64,
    config: CharacterConfig,
    cuda_motion: bool,
) -> Result<CharacterStep, String> {
    let mut general = physics::CharacterState {
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
    };
    let world = voxy_gpu::CudaVoxelCollisionWorld {
        compute,
        view,
        registry,
    };
    let report = if cuda_motion {
        let motion =
            character_motion(compute, state, input, dt, config).map_err(|e| e.to_string())?;
        physics::step_character_with_motion(
            &world,
            &mut general,
            input,
            dt,
            config,
            motion.velocity,
            motion.displacement,
        )
    } else {
        physics::step_character(&world, &mut general, input, dt, config)
    }
    .map_err(|e| e.to_string())?;
    *state = CharacterState {
        body: physics_voxel::AnchoredAabb {
            anchor: voxy_world::VoxelPos {
                x: general.body.anchor.x,
                y: general.body.anchor.y,
                z: general.body.anchor.z,
            },
            min: general.body.min,
            max: general.body.max,
        },
        velocity: general.velocity,
        grounded: general.grounded,
    };
    Ok(report)
}
