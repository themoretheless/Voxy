//! Replay actual GPU-returned coefficient words through the physical owner.
use super::*;
#[test]
#[ignore = "requires original VQC1 matrices and a real-GPU VPA2 five-word roundtrip artifact"]
fn five_gpu_roundtrip_preserves_native_coefficients_and_inverse_columns() {
    use crate::hair::HairResponseCoefficient as C;
    let fixture = std::env::var_os("VOXY_HAIR_QR_INPUT_FIXTURE").unwrap();
    let (requests, bounds, _tolerance) =
        square_root_fixture_tests::read_full_joint_operator(std::path::Path::new(&fixture));
    let artifact =
        std::fs::read(std::env::var_os("VOXY_EXACT_FIVE_GPU_ROUNDTRIP").unwrap()).unwrap();
    assert!(artifact.len() >= 24 && &artifact[..4] == b"VPA2");
    let rows = u32::from_le_bytes(artifact[4..8].try_into().unwrap()) as usize;
    let width = u32::from_le_bytes(artifact[8..12].try_into().unwrap()) as usize;
    let scale = f64::from_le_bytes(artifact[12..20].try_into().unwrap());
    assert_eq!(u32::from_le_bytes(artifact[20..24].try_into().unwrap()), 5);
    assert_eq!(rows, bounds.len());
    assert_eq!(
        width,
        requests.iter().map(|r| r.system.rhs.len()).sum::<usize>()
    );
    assert_eq!(artifact.len(), 24 + rows * width * 36);
    let mut recovered = vec![Vec::with_capacity(width); rows];
    for (index, record) in artifact[24..].chunks_exact(36).enumerate() {
        let source = [
            f64::from_le_bytes(record[..8].try_into().unwrap()),
            f64::from_le_bytes(record[8..16].try_into().unwrap()),
        ];
        let words = std::array::from_fn::<_, 5, _>(|i| {
            f64::from(f32::from_bits(u32::from_le_bytes(
                record[16 + i * 4..20 + i * 4].try_into().unwrap(),
            )))
        });
        let value = C::from_expansion_terms(words, [scale, 1.]).unwrap();
        let expected = C::from_parts(source).unwrap();
        assert_eq!(
            value.parts().map(f64::to_bits),
            expected.parts().map(f64::to_bits),
            "GPU coefficient {index}"
        );
        recovered[index / width].push(value);
    }
    let mut offset = 0;
    let mut checked_inverse = 0;
    for request in &requests {
        let local_width = request.system.rhs.len();
        let fresh = request.whiten_loads_expanded().unwrap();
        assert_eq!(fresh.len(), rows);
        for (row, source) in fresh.iter().enumerate() {
            let from_gpu = &recovered[row][offset..offset + local_width];
            for (coordinate, (original, restored)) in source.iter().zip(from_gpu).enumerate() {
                assert_eq!(
                    original.parts().map(f64::to_bits),
                    restored.parts().map(f64::to_bits),
                    "artifact differs from current physical owner row={row} coordinate={coordinate}"
                );
            }
            let original = request.backtransform_expanded(source).unwrap();
            let restored = request.backtransform_expanded(from_gpu).unwrap();
            for (original, restored) in original.iter().zip(&restored) {
                assert_eq!(
                    original.parts().map(f64::to_bits),
                    restored.parts().map(f64::to_bits)
                );
                checked_inverse += 1;
            }
        }
        offset += local_width;
    }
    assert_eq!(offset, width);
    assert_eq!(checked_inverse, rows * width);
    eprintln!(
        "FIVE GPU NATIVE RECOVERY rows={rows} width={width} coefficients={} inverse_coefficients={checked_inverse} bit_exact=true scope=coefficient_and_inverse_column_replay_not_GPU_contact_arithmetic_or_trajectory",
        rows * width
    );
}
