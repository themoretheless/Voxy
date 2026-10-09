//! Replays an actual captured contact-connected component without warm-up.
use super::*;
use crate::hair::{HairLinearSolver, HairLinearSystem, HairResponseSystem, RodContact};
use std::io::{Cursor, Read};

fn integer(input: &mut Cursor<Vec<u8>>) -> usize {
    let mut bytes = [0; 4];
    input.read_exact(&mut bytes).unwrap();
    u32::from_le_bytes(bytes) as usize
}
fn scalar(input: &mut Cursor<Vec<u8>>) -> f64 {
    let mut bytes = [0; 8];
    input.read_exact(&mut bytes).unwrap();
    let value = f64::from_le_bytes(bytes);
    assert!(value.is_finite());
    value
}
fn vectors<const N: usize>(input: &mut Cursor<Vec<u8>>, count: usize) -> Vec<[f64; N]> {
    (0..count)
        .map(|_| std::array::from_fn(|_| scalar(input)))
        .collect()
}
struct Snapshot {
    ids: Vec<usize>,
    rods: Vec<HairRod>,
    pairs: Vec<StrandResponse>,
    expected_positions: Vec<Vec<V>>,
    expected_orientations: Vec<Vec<Q>>,
}
fn snapshot(input: &mut Cursor<Vec<u8>>) -> Snapshot {
    let count = integer(input);
    let pairs = integer(input);
    let mut output = Snapshot {
        ids: Vec::new(),
        rods: Vec::new(),
        pairs: Vec::new(),
        expected_positions: Vec::new(),
        expected_orientations: Vec::new(),
    };
    for _ in 0..count {
        let id = integer(input);
        let n = integer(input);
        let contacts = integer(input);
        assert!(!output.ids.contains(&id));
        output.ids.push(id);
        let rest = vectors::<3>(input, n);
        let mut rod = HairRod::new(rest, Default::default()).unwrap();
        rod.x = vectors::<3>(input, n);
        rod.q = vectors::<4>(input, n - 1);
        assert!(
            rod.q
                .iter()
                .all(|q| (q.iter().map(|x| x * x).sum::<f64>() - 1.).abs() < 1e-5)
        );
        output.expected_positions.push(vectors::<3>(input, n));
        output
            .expected_orientations
            .push(vectors::<4>(input, n - 1));
        for _ in 0..contacts {
            let segment = integer(input);
            let fraction = scalar(input);
            let normal = vectors::<3>(input, 1)[0];
            let target = vectors::<3>(input, 1)[0];
            let velocity = vectors::<3>(input, 1)[0];
            let mesh = integer(input);
            assert!(segment < n - 1 && (0.0..=1.0).contains(&fraction));
            assert!((len(normal) - 1.).abs() < 1e-5);
            rod.contacts.push(RodContact {
                segment,
                fraction,
                normal,
                target,
                surface_velocity: velocity,
                source: ContactSource::Mesh(mesh),
            });
        }
        output.rods.push(rod);
    }
    for _ in 0..pairs {
        let mut endpoint = || {
            let rod = integer(input);
            let segment = integer(input);
            let fraction = scalar(input);
            (rod, segment, fraction)
        };
        let a = endpoint();
        let b = endpoint();
        let normal = vectors::<3>(input, 1)[0];
        for (rod, segment, fraction) in [a, b] {
            assert!(
                rod < count
                    && segment < output.rods[rod].x.len() - 1
                    && (0.0..=1.0).contains(&fraction)
            );
        }
        assert!((len(normal) - 1.).abs() < 1e-5);
        output.pairs.push(StrandResponse {
            a,
            b,
            normal,
            impulse: 0.,
        });
    }
    output
}

#[test]
#[ignore = "requires an actual captured contact component binary"]
fn captured_contact_component_replays_without_full_model_warmup() {
    let path = std::env::var("VOXY_HAIR_CONTACT_COMPONENT_FIXTURE")
        .expect("captured component fixture path");
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(&bytes[..4], b"VCP1");
    let mut input = Cursor::new(bytes);
    input.set_position(4);
    assert_eq!(integer(&mut input), 2);
    let selected = integer(&mut input);
    let dt = scalar(&mut input);
    let radius = scalar(&mut input);
    assert_eq!(dt, 1. / 240.);
    assert_eq!(radius, 40e-6);
    let mut snapshots = vec![snapshot(&mut input), snapshot(&mut input)];
    assert_eq!(input.position(), input.get_ref().len() as u64);
    struct Capture {
        requests: Vec<HairResponseSystem>,
    }
    impl HairLinearSolver for Capture {
        fn solve(&mut self, _: &[HairLinearSystem]) -> Result<Vec<Vec<f64>>, &'static str> {
            Err("response-only captured component")
        }
        fn solve_responses(
            &mut self,
            requests: &[HairResponseSystem],
        ) -> Result<Vec<Vec<Vec<f64>>>, &'static str> {
            self.requests = requests.to_vec();
            requests
                .iter()
                .map(HairResponseSystem::solve_native)
                .collect()
        }
    }
    let mut exports = Vec::new();
    for (index, state) in snapshots.iter_mut().enumerate() {
        let before = state.rods.clone();
        let (constraints, _) = position_constraints(&state.rods, &state.pairs, radius).unwrap();
        let mut capture = Capture {
            requests: Vec::new(),
        };
        let complete = reconcile_contact_positions_with_solver(
            &mut state.rods,
            &mut state.pairs,
            dt,
            radius,
            Some(&mut capture),
        )
        .unwrap();
        let mut maximum_error = 0f64;
        let mut maximum_rotation_error = 0f64;
        for (((actual, before), positions), orientations) in state
            .rods
            .iter()
            .zip(&before)
            .zip(&state.expected_positions)
            .zip(&state.expected_orientations)
        {
            assert_eq!(actual.x[0], before.x[0]);
            assert_eq!(actual.q[0], before.q[0]);
            assert_eq!(actual.velocity, before.velocity);
            assert_eq!(actual.omega, before.omega);
            assert!(actual.x.iter().all(|point| finite(*point)));
            maximum_error = maximum_error.max(
                actual
                    .x
                    .iter()
                    .zip(positions)
                    .flat_map(|(a, b)| a.iter().zip(b).map(|(a, b)| (a - b).abs()))
                    .fold(0., f64::max),
            );
            maximum_rotation_error = maximum_rotation_error.max(
                actual
                    .q
                    .iter()
                    .zip(orientations)
                    .flat_map(|(a, b)| a.iter().zip(b).map(|(a, b)| (a - b).abs()))
                    .fold(0., f64::max),
            );
        }
        if complete {
            for constraint in &constraints {
                let change = constraint
                    .entries
                    .iter()
                    .map(|entry| {
                        dot(
                            entry.gradient,
                            sub(
                                state.rods[entry.rod].x[entry.point],
                                before[entry.rod].x[entry.point],
                            ),
                        )
                    })
                    .sum::<f64>();
                assert!(
                    change >= constraint.bound - 1.1e-11,
                    "captured tangent constraint failed: {}",
                    change - constraint.bound
                );
            }
        }
        eprintln!(
            "CAPTURED COMPONENT case={index} rods={} constraints={} complete={complete} replay_vs_captured_position_error_m={maximum_error:e} rotation_error={maximum_rotation_error:e}",
            state.rods.len(),
            constraints.len()
        );
        exports.push((capture.requests, constraints));
    }
    let a = &snapshots[0];
    let b = &snapshots[1];
    let a = &a.rods[a.ids.iter().position(|id| *id == selected).unwrap()];
    let b = &b.rods[b.ids.iter().position(|id| *id == selected).unwrap()];
    let delta =
        a.x.iter()
            .zip(&b.x)
            .flat_map(|(a, b)| a.iter().zip(b).map(|(a, b)| (a - b).abs()))
            .fold(0., f64::max);
    eprintln!(
        "CAPTURED COMPONENT selected_rod={selected} native_replay_input_perturbation_delta_m={delta:e}"
    );
    if let Ok(path) = std::env::var("VOXY_HAIR_COMPONENT_RESPONSE_EXPORT") {
        assert!(std::path::Path::new(&path).is_absolute());
        use std::fmt::Write;
        let mut json = String::from("[");
        for (case, (requests, constraints)) in exports.iter().enumerate() {
            if case > 0 {
                json.push(',');
            }
            write!(&mut json, "{{\"case\":{case},\"requests\":[").unwrap();
            for (index, request) in requests.iter().enumerate() {
                if index > 0 {
                    json.push(',');
                }
                write!(
                    &mut json,
                    "{{\"matrix\":{:?},\"rhs\":{:?},\"loads\":{:?},\"first\":{},\"end\":{}}}",
                    request.system.matrix,
                    request.system.rhs,
                    request.loads,
                    request.system.active.start,
                    request.system.active.end
                )
                .unwrap();
            }
            json.push_str("],\"constraints\":[");
            for (index, constraint) in constraints.iter().enumerate() {
                if index > 0 {
                    json.push(',');
                }
                write!(&mut json, "{{\"bound\":{},\"entries\":[", constraint.bound).unwrap();
                for (entry_index, entry) in constraint.entries.iter().enumerate() {
                    if entry_index > 0 {
                        json.push(',');
                    }
                    write!(
                        &mut json,
                        "{{\"rod\":{},\"point\":{},\"gradient\":{:?}}}",
                        entry.rod, entry.point, entry.gradient
                    )
                    .unwrap();
                }
                json.push_str("]}");
            }
            json.push_str("]}");
        }
        json.push(']');
        std::fs::write(path, json).unwrap();
    }
}
