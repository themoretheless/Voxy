//! Browser fixed character step without blocking GPU readback.
use super::browser::error;
use physics::{AnchoredAabb, CharacterConfig, CharacterInput, CharacterState, Origin};
use voxy_gpu::{CharacterGpuQueryError, GpuSweepError, PendingGpuCharacter, VoxelRegionProgram};
use wasm_bindgen::JsValue;

#[derive(Debug)]
pub(crate) struct VoxelGame {
    pub state: CharacterState,
    pub(super) program: Option<VoxelRegionProgram>,
    pub(super) vehicle: Option<super::voxel_vehicle::BrowserVehicle>,
    pending: Option<PendingGpuCharacter>,
    started: Option<f64>,
    intent: Option<CharacterInput>,
    pub ticks: u32,
}
impl VoxelGame {
    pub fn protected_chunks(
        &self,
    ) -> Result<std::collections::BTreeSet<voxy_core::ChunkPos>, JsValue> {
        let mut chunks: std::collections::BTreeSet<_> = voxy_gpu::body_chunks(self.state.body)
            .map_err(error)?
            .into_iter()
            .collect();
        if let Some(pending) = &self.pending {
            chunks.extend(pending.captured_chunks());
        }
        if let Some(vehicle) = &self.vehicle {
            chunks.extend(vehicle.protected_chunks()?);
        }
        Ok(chunks)
    }

    pub fn extend_pause(&mut self, milliseconds: f64) -> u32 {
        let mut pending = 0;
        if let Some(started) = &mut self.started {
            *started += milliseconds;
            pending += 1;
        }
        if let Some(vehicle) = &mut self.vehicle {
            pending += vehicle.extend_pause(milliseconds);
        }
        pending
    }

    pub async fn new(device: &wgpu::Device) -> Result<Self, JsValue> {
        let program = if device.limits().max_compute_workgroups_per_dimension > 0 {
            Some(VoxelRegionProgram::new(device).await.map_err(error)?)
        } else {
            None
        };
        Ok(Self {
            state: CharacterState {
                body: AnchoredAabb {
                    anchor: Origin {
                        x: 16,
                        y: 25,
                        z: 16,
                    },
                    min: [-0.3, 0.0, -0.3],
                    max: [0.3, 1.8, 0.3],
                },
                velocity: [0.0; 3],
                grounded: false,
            },
            program,
            vehicle: None,
            pending: None,
            started: None,
            intent: None,
            ticks: 0,
        })
    }
    pub fn step(
        &mut self,
        queue: &wgpu::Queue,
        world: &voxy_world::World,
        input: CharacterInput,
    ) -> Result<bool, JsValue> {
        let config = CharacterConfig::default();
        if let Some(program) = &self.program {
            let started = self.started.get_or_insert_with(js_sys::Date::now);
            if js_sys::Date::now() - *started >= 30_000.0 {
                return Err(error("browser gameplay character timed out"));
            }
            let input = *self.intent.get_or_insert(input);
            let task = self.pending.get_or_insert_with(|| {
                PendingGpuCharacter::new(self.state, input, 1.0 / 60.0, config, world.registry())
            });
            match task.try_step(program, queue, world) {
                Ok(None) => return Ok(false),
                Ok(Some((state, _))) => {
                    self.state = state;
                    self.pending = None;
                    self.started = None;
                    self.intent = None;
                }
                Err(physics::CharacterError::Sweep(CharacterGpuQueryError::Gpu(
                    GpuSweepError::StaleWorld,
                ))) => {
                    self.pending = None;
                    return Ok(false);
                }
                Err(failure) => return Err(error(failure)),
            }
        } else {
            let collision = physics_voxel::VoxelCollisionWorld {
                view: world,
                registry: world.registry(),
            };
            physics::step_character(&collision, &mut self.state, input, 1.0 / 60.0, config)
                .map_err(error)?;
        }
        self.ticks = self
            .ticks
            .checked_add(1)
            .ok_or_else(|| error("browser character ticks overflow"))?;
        Ok(true)
    }
    pub fn toggle_vehicle(&mut self) -> bool {
        self.pending = None;
        self.started = None;
        self.intent = None;
        if let Some(vehicle) = self.vehicle.take() {
            let body = vehicle.body();
            let center =
                std::array::from_fn::<_, 3, _>(|a| body.min[a] + (body.max[a] - body.min[a]) * 0.5);
            self.state.body = AnchoredAabb {
                anchor: body.anchor,
                min: [center[0] - 0.3, body.min[1], center[2] - 0.3],
                max: [center[0] + 0.3, body.min[1] + 1.8, center[2] + 0.3],
            };
            self.state.velocity = [0.0; 3];
            self.state.grounded = false;
            false
        } else {
            self.vehicle = Some(super::voxel_vehicle::BrowserVehicle::new(self.state));
            true
        }
    }
    pub fn position(&self) -> Result<glam::Vec3, JsValue> {
        let body = self
            .vehicle
            .as_ref()
            .map_or(self.state.body, super::voxel_vehicle::BrowserVehicle::body);
        let anchor = body.anchor;
        let xyz = [anchor.x, anchor.y, anchor.z];
        if xyz.iter().any(|v| v.unsigned_abs() > 2_097_152) {
            return Err(error("browser actor exceeds local render range"));
        }
        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        let position: [f32; 3] = std::array::from_fn(|a| {
            ((xyz[a] as f64 + body.min[a] + (body.max[a] - body.min[a]) * 0.5
                - [16.0, 10.0, 16.0][a])
                / 16.0) as f32
        });
        Ok(glam::Vec3::from_array(position))
    }
}
