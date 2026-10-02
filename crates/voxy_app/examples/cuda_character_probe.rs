//! Headless real-NVIDIA motion + voxel-controller equivalence acceptance.
use physics_voxel::{AnchoredAabb, CharacterConfig, CharacterInput, CharacterState};
use voxy_cuda::CudaCompute;
use voxy_world::VoxelPos;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ordinal = std::env::args()
        .nth(1)
        .map_or(Ok(0), |value| value.parse::<usize>())?;
    let compute = CudaCompute::new(ordinal, 1024 * 1024)?;
    println!("CUDA character: {:?}", compute.capabilities()?);
    let scene = voxy_runtime::build_bootstrap_scene(42, 0)?;
    let mut cpu = initial_state();
    let mut gpu = cpu;
    let config = CharacterConfig::default();
    let dt = 1.0 / 60.0;
    let mut grounded_ticks = 0;
    let mut jumps = 0;
    for tick in 0..240 {
        let input = CharacterInput {
            planar_velocity: [2.0, 0.0],
            jump_pressed: tick == 90 || tick == 180,
        };
        if input.jump_pressed && gpu.grounded {
            jumps += 1;
        }
        let expected = physics_voxel::step_character(
            &scene.world,
            scene.world.registry(),
            &mut cpu,
            input,
            dt,
            config,
        )?;
        let actual = voxy_app::cuda_motion::step_character(
            &compute,
            &scene.world,
            scene.world.registry(),
            &mut gpu,
            input,
            dt,
            config,
        )?;
        if actual != expected || gpu != cpu {
            return Err(format!(
                "CUDA character mismatch at tick {tick}: GPU {gpu:?}, CPU {cpu:?}"
            )
            .into());
        }
        grounded_ticks += usize::from(gpu.grounded);
    }
    if grounded_ticks == 0 || jumps == 0 {
        return Err("character fixture did not exercise ground/jump".into());
    }
    println!(
        "CUDA PASS: 240 character ticks, exact CPU state and voxel contacts, ground and jump exercised"
    );
    Ok(())
}

fn initial_state() -> CharacterState {
    CharacterState {
        body: AnchoredAabb {
            anchor: VoxelPos { x: 8, y: 20, z: 8 },
            min: [-0.35, 0.0, -0.35],
            max: [0.35, 1.8, 0.35],
        },
        velocity: [0.0; 3],
        grounded: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hardware_fixture_exercises_ground_and_jump_on_the_cpu_reference() {
        let scene = voxy_runtime::build_bootstrap_scene(42, 0).unwrap();
        let mut state = initial_state();
        let mut grounded = 0;
        let mut jumps = 0;
        for tick in 0..240 {
            let input = CharacterInput {
                planar_velocity: [2.0, 0.0],
                jump_pressed: tick == 90 || tick == 180,
            };
            jumps += usize::from(input.jump_pressed && state.grounded);
            physics_voxel::step_character(
                &scene.world,
                scene.world.registry(),
                &mut state,
                input,
                1.0 / 60.0,
                CharacterConfig::default(),
            )
            .unwrap();
            grounded += usize::from(state.grounded);
        }
        assert!(grounded > 0);
        assert!(jumps > 0);
    }
}
