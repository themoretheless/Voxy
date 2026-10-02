//! Physical NVIDIA exact f64 continuous box collision against the CPU oracle.
#[path = "support/device_argument.rs"]
mod device_argument;
use voxy_cuda::{CudaBoxSweep, CudaCompute};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ordinal = device_argument::parse(std::env::args().skip(1))?;
    let compute = CudaCompute::new(ordinal, 1024 * 1024)?;
    println!("CUDA box sweep: {:?}", compute.capabilities()?);
    let mut inputs: Vec<_> = (0..257_u32)
        .map(|i| CudaBoxSweep {
            min: [0.1; 3],
            max: [0.9; 3],
            displacement: [
                f64::from(i % 17) * 0.25 - 2.0,
                f64::from(i % 11) * 0.25 - 1.0,
                0.0,
            ],
            obstacle_min: [1.0, 0.0, 0.0],
            obstacle_max: [2.0, 1.0, 1.0],
        })
        .collect();
    let base = inputs[0];
    for displacement in [
        [0.0; 3],
        [-0.0; 3],
        [1.0, 1.0, 0.0],
        [-1.0, 0.0, 0.0],
        [f64::MIN_POSITIVE, 0.0, 0.0],
    ] {
        inputs.push(CudaBoxSweep {
            displacement,
            ..base
        });
        inputs.push(CudaBoxSweep {
            displacement,
            obstacle_min: [0.0; 3],
            obstacle_max: [1.0; 3],
            ..base
        });
    }
    let results = compute.box_sweeps(&inputs)?;
    let bounded = CudaCompute::new(ordinal, 160 * 64)?;
    assert_eq!(bounded.box_sweep_capacity(), 64);
    assert!(matches!(
        bounded.box_sweeps(&inputs),
        Err(voxy_cuda::CudaError::BufferLimit)
    ));
    let mut batched = Vec::new();
    for batch in inputs.chunks(bounded.box_sweep_capacity()) {
        batched.extend(bounded.box_sweeps(batch)?);
    }
    assert_eq!(batched, results);

    for (input, actual) in inputs.iter().zip(results) {
        let expected = physics::sweep_box(
            physics::AnchoredAabb {
                anchor: physics::Origin::default(),
                min: input.min,
                max: input.max,
            },
            input.displacement,
            input.obstacle_min,
            input.obstacle_max,
        );
        match (actual, expected) {
            (None, None) => {}
            (Some((a, an)), Some((e, en))) => {
                assert_eq!(a.to_bits(), e.to_bits());
                assert_eq!(an, en);
            }
            mismatch => panic!("CUDA box sweep differs: {mismatch:?}"),
        }
    }
    assert!(
        compute
            .box_sweeps(&[CudaBoxSweep {
                displacement: [f64::NAN, 0.0, 0.0],
                ..base
            }])
            .is_err()
    );
    println!(
        "PASS: CUDA 267 exact f64 box sweeps, stationary/overlap/tie cases and invalid-input rejection"
    );
    Ok(())
}
