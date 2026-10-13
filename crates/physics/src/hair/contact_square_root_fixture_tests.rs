//! Replay original, captured joint inequalities without model reconstruction.
use super::*;
use std::io::Read;

// Reconstruct the immutable physical operator, not a rod from geometry. The
// serialized whitening columns are deliberately skipped: native replay must
// derive them again from the original matrices and contact loads.
pub(super) fn read_full_joint_operator(
    path: &std::path::Path,
) -> (Vec<HairResponseSystem>, Vec<f64>, f64) {
    use std::io::{BufReader, Seek, SeekFrom};
    let file = std::fs::File::open(path).unwrap();
    let length = file.metadata().unwrap().len();
    let mut input = BufReader::new(file);
    fn integer(input: &mut impl Read) -> usize {
        let mut bytes = [0; 4];
        input.read_exact(&mut bytes).unwrap();
        u32::from_le_bytes(bytes) as usize
    }
    fn scalar(input: &mut impl Read) -> f64 {
        let mut bytes = [0; 8];
        input.read_exact(&mut bytes).unwrap();
        let value = f64::from_le_bytes(bytes);
        assert!(value.is_finite(), "nonfinite original physical input");
        value
    }
    let mut magic = [0; 4];
    input.read_exact(&mut magic).unwrap();
    assert_eq!(&magic, b"VQC1");
    let rows = integer(&mut input);
    let width = integer(&mut input);
    let systems = integer(&mut input);
    let _refinement = integer(&mut input);
    let tolerance = scalar(&mut input);
    assert!(
        (1..=4096).contains(&rows)
            && (1..=65536).contains(&width)
            && (1..=4096).contains(&systems)
            && tolerance > 0.
    );
    let expected = 28u64
        + 16 * rows as u64
        + 16 * systems as u64
        + 16 * rows as u64 * width as u64
        + 8 * (super::direct::BAND + 1) as u64 * width as u64;
    assert_eq!(
        length, expected,
        "truncated, trailing or inconsistent operator payload"
    );
    let bounds: Vec<_> = (0..rows).map(|_| scalar(&mut input)).collect();
    for _ in 0..rows {
        scalar(&mut input);
    }
    input
        .seek(SeekFrom::Current((rows as u64 * width as u64 * 8) as i64))
        .unwrap();
    let mut requests = Vec::with_capacity(systems);
    let mut actual_width = 0;
    for _ in 0..systems {
        let n = integer(&mut input);
        let band = integer(&mut input);
        let lo = integer(&mut input);
        let hi = integer(&mut input);
        assert!(n > 0 && n <= width && band == super::direct::BAND && lo <= hi && hi <= n);
        actual_width += n;
        assert!(actual_width <= width);
        let matrix = (0..n * band).map(|_| scalar(&mut input)).collect();
        let rhs = (0..n).map(|_| scalar(&mut input)).collect();
        let loads = (0..rows)
            .map(|_| (0..n).map(|_| scalar(&mut input)).collect())
            .collect();
        requests.push(HairResponseSystem {
            system: HairLinearSystem {
                band_width: band,
                matrix,
                rhs,
                active: lo..hi,
            },
            loads,
        });
    }
    assert_eq!(actual_width, width);
    assert_eq!(input.stream_position().unwrap(), length);
    (requests, bounds, tolerance)
}

#[test]
#[ignore = "requires a frozen VQC1 full physical operator; can exceed 512 contacts"]
fn captured_full_joint_operator_preserves_physical_admission() {
    let path = std::env::var_os("VOXY_HAIR_QR_INPUT_FIXTURE").expect("VQC1 operator fixture");
    let (requests, bounds, tolerance) = read_full_joint_operator(std::path::Path::new(&path));
    eprintln!(
        "FULL JOINT REPLAY systems={} rows={} coordinates={} tolerance={tolerance}",
        requests.len(),
        bounds.len(),
        requests.iter().map(|r| r.system.rhs.len()).sum::<usize>()
    );
    let started = std::time::Instant::now();
    let result =
        HairResponseSystem::solve_joint_load_inequalities_native(&requests, &bounds, tolerance);
    eprintln!(
        "FULL JOINT REPLAY elapsed_s={} error={:?}",
        started.elapsed().as_secs_f64(),
        result.as_ref().err()
    );
    let (responses, reactions) = result.expect("original full physical operator admission");
    HairResponseSystem::validate_joint_solution(
        &requests, &bounds, &responses, &reactions, tolerance,
    )
    .expect("original physical loads and inequality residuals");
}

#[test]
#[ignore = "requires an exact captured VQI1 joint response fixture"]
fn captured_original_load_defect_is_corrected_without_relaxing_admission() {
    let path = std::env::var("VOXY_HAIR_QR_FAILURE_FIXTURE").expect("joint QR fixture");
    let mut input = std::io::Cursor::new(std::fs::read(path).unwrap());
    fn integer(input: &mut std::io::Cursor<Vec<u8>>) -> usize {
        let mut b = [0; 4];
        input.read_exact(&mut b).unwrap();
        u32::from_le_bytes(b) as usize
    }
    fn scalar(input: &mut std::io::Cursor<Vec<u8>>) -> f64 {
        let mut b = [0; 8];
        input.read_exact(&mut b).unwrap();
        let value = f64::from_le_bytes(b);
        assert!(value.is_finite());
        value
    }
    fn values(input: &mut std::io::Cursor<Vec<u8>>, count: usize) -> Vec<f64> {
        assert!(count <= 16_000_000);
        (0..count).map(|_| scalar(input)).collect()
    }
    let mut magic = [0; 4];
    input.read_exact(&mut magic).unwrap();
    assert_eq!(&magic, b"VQI1");
    let rows = integer(&mut input);
    let width = integer(&mut input);
    let count = integer(&mut input);
    let failed = integer(&mut input);
    let tolerance = scalar(&mut input);
    assert!(
        (1..=4096).contains(&rows)
            && (1..=65536).contains(&width)
            && (1..=4096).contains(&count)
            && failed < rows
            && tolerance > 0.
    );
    let bounds = values(&mut input, rows);
    let old_reactions = values(&mut input, rows);
    let skip = (rows + 1) * width * 8;
    assert!(input.position() as usize + skip <= input.get_ref().len());
    input.set_position(input.position() + skip as u64);
    let mut requests = Vec::new();
    let mut previous = Vec::new();
    for _ in 0..count {
        let n = integer(&mut input);
        let band = integer(&mut input);
        let lo = integer(&mut input);
        let hi = integer(&mut input);
        assert!((12..=65536).contains(&n) && band == direct::BAND && lo <= hi && hi <= n);
        let system = HairLinearSystem {
            band_width: band,
            matrix: values(&mut input, n * band),
            rhs: values(&mut input, n),
            active: lo..hi,
        };
        let loads = (0..rows).map(|_| values(&mut input, n)).collect();
        previous.push(values(&mut input, n));
        requests.push(HairResponseSystem { system, loads });
    }
    assert_eq!(input.position() as usize, input.get_ref().len());
    assert_eq!(
        requests.iter().map(|r| r.system.rhs.len()).sum::<usize>(),
        width
    );
    let actual = |responses: &[Vec<f64>], i: usize| {
        requests
            .iter()
            .zip(responses)
            .map(|(r, x)| r.loads[i].iter().zip(x).map(|(a, b)| a * b).sum::<f64>())
            .sum::<f64>()
    };
    let old_gap = actual(&previous, failed) - bounds[failed];
    assert!(
        old_reactions[failed] > 0. && old_gap.abs() > tolerance,
        "fixture must reproduce the original admission failure"
    );
    let (responses, reactions) =
        HairResponseSystem::solve_joint_load_inequalities_native(&requests, &bounds, tolerance)
            .expect("original-load defect refinement");
    for i in 0..rows {
        let gap = actual(&responses, i) - bounds[i];
        assert!(
            reactions[i] >= 0.
                && if reactions[i] > 0. {
                    gap.abs() <= tolerance
                } else {
                    gap >= -tolerance
                }
        );
    }
    for (request, response) in requests.iter().zip(&responses) {
        for (i, &x) in response.iter().enumerate() {
            if !request.system.active.contains(&i) {
                assert_eq!(x, 0.);
            }
        }
    }
    if let Some(path) = std::env::var_os("VOXY_HAIR_QR_RESPONSE_EXPORT") {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"VQR1");
        bytes.extend_from_slice(&(count as u32).to_le_bytes());
        for response in &responses {
            bytes.extend_from_slice(&(response.len() as u32).to_le_bytes());
            for x in response {
                bytes.extend_from_slice(&x.to_le_bytes());
            }
        }
        bytes.extend_from_slice(&(rows as u32).to_le_bytes());
        for x in &reactions {
            bytes.extend_from_slice(&x.to_le_bytes());
        }
        std::fs::write(path, bytes).unwrap();
    }
    eprintln!(
        "ORIGINAL LOAD REFINEMENT rows={rows} old_gap={old_gap:e} new_gap={:e} tolerance={tolerance:e}",
        actual(&responses, failed) - bounds[failed]
    );
}
