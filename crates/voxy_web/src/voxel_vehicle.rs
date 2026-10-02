//! Browser vehicle with atomic asynchronous chassis contacts.
use super::browser::error;
use physics_voxel::{VehicleConfig, VehicleInput, VehicleState};
use voxy_gpu::{
    CharacterGpuQueryError, GpuSweepError, PendingGpuVehicle, VehicleGpuError, VoxelRegionProgram,
};
use wasm_bindgen::JsValue;

#[derive(Debug)]
pub(super) struct BrowserVehicle {
    state: VehicleState,
    pending: Option<PendingGpuVehicle>,
    intent: Option<VehicleInput>,
    started: Option<f64>,
}
impl BrowserVehicle {
    pub fn protected_chunks(
        &self,
    ) -> Result<std::collections::BTreeSet<voxy_core::ChunkPos>, JsValue> {
        let mut chunks: std::collections::BTreeSet<_> = voxy_gpu::body_chunks(self.body())
            .map_err(error)?
            .into_iter()
            .collect();
        if let Some(pending) = &self.pending {
            chunks.extend(pending.captured_chunks());
        }
        Ok(chunks)
    }

    pub fn extend_pause(&mut self, milliseconds: f64) -> u32 {
        let mut pending = 0;
        if let Some(started) = &mut self.started {
            *started += milliseconds;
            pending += 1;
        }
        pending
    }

    pub fn new(character: physics::CharacterState) -> Self {
        let center = std::array::from_fn::<_, 3, _>(|axis| {
            character.body.min[axis] + (character.body.max[axis] - character.body.min[axis]) * 0.5
        });
        Self {
            state: VehicleState {
                chassis: physics_voxel::CharacterState {
                    body: physics_voxel::AnchoredAabb {
                        anchor: voxy_world::VoxelPos {
                            x: character.body.anchor.x,
                            y: character.body.anchor.y,
                            z: character.body.anchor.z,
                        },
                        min: [center[0] - 0.6, character.body.min[1], center[2] - 0.8],
                        max: [
                            center[0] + 0.6,
                            character.body.min[1] + 1.2,
                            center[2] + 0.8,
                        ],
                    },
                    velocity: [0.0; 3],
                    grounded: false,
                },
                heading: 0.0,
                longitudinal_speed: 0.0,
            },
            pending: None,
            intent: None,
            started: None,
        }
    }
    #[allow(clippy::cast_possible_truncation)]
    pub fn heading(&self) -> f32 {
        self.state.heading as f32
    }
    pub fn body(&self) -> physics::AnchoredAabb {
        let body = self.state.chassis.body;
        physics::AnchoredAabb {
            anchor: physics::Origin {
                x: body.anchor.x,
                y: body.anchor.y,
                z: body.anchor.z,
            },
            min: body.min,
            max: body.max,
        }
    }
    pub fn step(
        &mut self,
        program: Option<&VoxelRegionProgram>,
        queue: &wgpu::Queue,
        world: &voxy_world::World,
        input: VehicleInput,
    ) -> Result<bool, JsValue> {
        let config = VehicleConfig {
            max_forward_speed: 4.0,
            max_reverse_speed: 2.0,
            ..VehicleConfig::default()
        };
        if let Some(program) = program {
            let started = self.started.get_or_insert_with(js_sys::Date::now);
            if js_sys::Date::now() - *started >= 30_000.0 {
                return Err(error("browser vehicle timed out"));
            }
            let input = *self.intent.get_or_insert(input);
            let pending = self.pending.get_or_insert_with(|| {
                PendingGpuVehicle::new(self.state, input, 1.0 / 60.0, config, world.registry())
            });
            match pending.try_step(program, queue, world) {
                Ok(None) => return Ok(false),
                Ok(Some((state, _))) => {
                    self.state = state;
                    self.pending = None;
                    self.started = None;
                    self.intent = None;
                }
                Err(VehicleGpuError::Character(physics::CharacterError::Sweep(
                    CharacterGpuQueryError::Gpu(GpuSweepError::StaleWorld),
                ))) => {
                    self.pending = None;
                    return Ok(false);
                }
                Err(failure) => return Err(error(failure)),
            }
        } else {
            physics_voxel::step_vehicle(
                world,
                world.registry(),
                &mut self.state,
                input,
                1.0 / 60.0,
                config,
            )
            .map_err(error)?;
        }
        Ok(true)
    }
}
