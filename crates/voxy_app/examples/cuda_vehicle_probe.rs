//! Real NVIDIA chassis integration against the CPU bicycle/voxel controller.
use physics_voxel::{AnchoredAabb, CharacterState, VehicleConfig, VehicleInput, VehicleState};
use voxy_cuda::CudaCompute;
use voxy_world::VoxelPos;

fn initial() -> VehicleState {
    VehicleState {
        chassis: CharacterState {
            body: AnchoredAabb {
                anchor: VoxelPos { x: 8, y: 20, z: 8 },
                min: [-0.6, 0.0, -0.8],
                max: [0.6, 1.2, 0.8],
            },
            velocity: [0.0; 3],
            grounded: false,
        },
        heading: 0.0,
        longitudinal_speed: 0.0,
    }
}
fn config() -> VehicleConfig {
    VehicleConfig {
        max_forward_speed: 4.0,
        max_reverse_speed: 2.0,
        ..VehicleConfig::default()
    }
}
fn input(tick: u32) -> VehicleInput {
    VehicleInput {
        throttle: if (80..120).contains(&tick) {
            0.0
        } else if (120..180).contains(&tick) {
            -1.0
        } else {
            1.0
        },
        steering: if (40..180).contains(&tick) { 0.25 } else { 0.0 },
        brake: (80..120).contains(&tick),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ordinal = std::env::args()
        .nth(1)
        .map_or(Ok(0), |value| value.parse::<usize>())?;
    let compute = CudaCompute::new(ordinal, 1024 * 1024)?;
    println!("CUDA vehicle: {:?}", compute.capabilities()?);
    let scene = voxy_runtime::build_bootstrap_scene(42, 0)?;
    let mut cpu = initial();
    let mut gpu = cpu;
    let mut forward = false;
    let mut reverse = false;
    for tick in 0..240 {
        let expected = physics_voxel::step_vehicle(
            &scene.world,
            scene.world.registry(),
            &mut cpu,
            input(tick),
            1.0 / 60.0,
            config(),
        )?;
        let actual = physics_voxel::step_vehicle_with_chassis_step(
            &mut gpu,
            input(tick),
            1.0 / 60.0,
            config(),
            |chassis, input, dt, config| {
                if tick % 2 == 0 {
                    voxy_app::cuda_motion::step_character_collision(
                        &compute,
                        &scene.world,
                        scene.world.registry(),
                        chassis,
                        input,
                        dt,
                        config,
                        true,
                    )
                    .map_err(voxy_app::cuda_motion::MotionError::Collision)
                } else {
                    voxy_app::cuda_motion::step_character(
                        &compute,
                        &scene.world,
                        scene.world.registry(),
                        chassis,
                        input,
                        dt,
                        config,
                    )
                }
            },
        )?;
        if actual != expected || cpu != gpu {
            return Err(
                format!("CUDA vehicle mismatch at tick {tick}: GPU {gpu:?}, CPU {cpu:?}").into(),
            );
        }
        forward |= gpu.longitudinal_speed > 0.1;
        reverse |= gpu.longitudinal_speed < -0.1;
    }
    if !forward || !reverse {
        return Err("vehicle fixture did not exercise forward/reverse".into());
    }
    println!(
        "CUDA PASS: 240 vehicle ticks, exact CPU state and voxel contacts, forward and reverse exercised"
    );
    println!("CUDA PASS: combined vehicle motion and CUDA contacts exact CPU parity");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cpu_fixture_exercises_forward_reverse_and_braking() {
        let scene = voxy_runtime::build_bootstrap_scene(42, 0).unwrap();
        let mut state = initial();
        let mut forward = false;
        let mut reverse = false;
        let mut stopped_by_brake = false;
        for tick in 0..240 {
            physics_voxel::step_vehicle(
                &scene.world,
                scene.world.registry(),
                &mut state,
                input(tick),
                1.0 / 60.0,
                config(),
            )
            .unwrap();
            forward |= state.longitudinal_speed > 0.1;
            reverse |= state.longitudinal_speed < -0.1;
            stopped_by_brake |= input(tick).brake && state.longitudinal_speed == 0.0;
        }
        assert!(forward && reverse && stopped_by_brake);
    }
}
