use physics::skin::{SkinEmbedding, SurfaceBinding};
fn binding(vertex: usize) -> SurfaceBinding {
    SurfaceBinding {
        vertex,
        triangle: 0,
        weights: [0.2, 0.3, 0.5],
    }
}
#[test]
fn preserves_detail_and_moves_both_sides_of_a_render_seam() {
    let embedding =
        SkinEmbedding::new(2, 3, vec![[0, 1, 2]], vec![binding(0), binding(1)], true).unwrap();
    let shell = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
    let detail = [[0.3, 0.5, 0.01]; 2];
    assert_eq!(embedding.deform(&shell, &shell, &detail).unwrap(), detail);
    let moved = shell.map(|p| [p[0] + 0.02, p[1] - 0.03, p[2] + 0.04]);
    let result = embedding.deform(&moved, &shell, &detail).unwrap();
    assert_eq!(result[0], result[1]);
    assert!((result[0][2] - 0.05).abs() < 1e-12);
    // When the skeleton and shell move together, there is no second displacement.
    assert_eq!(embedding.deform(&moved, &moved, &detail).unwrap(), detail);
}
#[test]
fn rejects_partial_duplicate_invalid_and_overflowing_bindings() {
    assert!(SkinEmbedding::new(2, 3, vec![[0, 1, 2]], vec![binding(0)], true).is_err());
    assert!(
        SkinEmbedding::new(2, 3, vec![[0, 1, 2]], vec![binding(0), binding(0)], false).is_err()
    );
    let mut invalid = binding(0);
    invalid.weights[0] = f64::NAN;
    assert!(SkinEmbedding::new(1, 3, vec![[0, 1, 2]], vec![invalid], true).is_err());
    let embedding = SkinEmbedding::new(1, 3, vec![[0, 1, 2]], vec![binding(0)], true).unwrap();
    assert!(
        embedding
            .deform(&[[f64::MAX; 3]; 3], &[[-f64::MAX; 3]; 3], &[[0.; 3]])
            .is_err()
    );
    assert!(
        embedding
            .deform(&[[0.; 3]; 2], &[[0.; 3]; 3], &[[0.; 3]])
            .is_err()
    );
}
