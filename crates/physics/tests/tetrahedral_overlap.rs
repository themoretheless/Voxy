use physics::biomechanics::{Material, TetraMesh};
fn pair(shift: [f64; 3], scale: f64) -> TetraMesh {
    let base = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    let mut points = base.to_vec();
    points.extend(base.map(|p| std::array::from_fn(|axis| shift[axis] + scale * p[axis])));
    TetraMesh {
        points,
        cells: vec![[0, 1, 2, 3], [4, 5, 6, 7]],
        boundary: vec![
            [1, 2, 3],
            [0, 3, 2],
            [0, 1, 3],
            [0, 2, 1],
            [5, 6, 7],
            [4, 7, 6],
            [4, 5, 7],
            [4, 6, 5],
        ],
    }
}
#[test]
fn overlapping_disconnected_cells_are_rejected_before_mass_or_embedding() {
    for (shift, scale) in [
        ([0.; 3], 1.),
        ([0.1; 3], 1.),
        ([0.1; 3], 0.1),
        ([-0.1; 3], 1.),
    ] {
        let mesh = pair(shift, scale);
        // Construct the untrusted payload from a valid header/connectivity;
        // the engine writer itself correctly refuses this geometry.
        let mut payload = pair([2., 0., 0.], 1.).to_bytes().unwrap();
        for (point_index, point) in mesh.points.iter().enumerate().skip(4) {
            for (axis, value) in point.iter().enumerate() {
                let offset = 20 + 24 * point_index + 8 * axis;
                payload[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
            }
        }
        assert_eq!(
            TetraMesh::from_bytes(&payload).unwrap_err(),
            "overlapping tetrahedral cells"
        );
        assert_eq!(
            mesh.to_bytes().unwrap_err(),
            "overlapping tetrahedral cells"
        );
        assert_eq!(
            mesh.into_body(
                vec![false; 8],
                &Material {
                    shear_pa: 10.,
                    bulk_pa: 1000.,
                    fibers: vec![]
                }
            )
            .unwrap_err(),
            "overlapping tetrahedral cells"
        );
    }
}
#[test]
fn separated_and_touching_cells_remain_admissible() {
    for shift in [[2., 0., 0.], [1., 0., 0.], [0.5, 0.5, 0.], [0.4, 0.4, 0.4]] {
        let mesh = pair(shift, 1.);
        let bytes = mesh.to_bytes().unwrap();
        assert_eq!(TetraMesh::from_bytes(&bytes).unwrap().cells, mesh.cells);
    }
    // Opposite cells sharing a complete canonical interface are valid.
    let mesh = TetraMesh {
        points: vec![
            [0.; 3],
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [0., 0., -1.],
        ],
        cells: vec![[0, 1, 2, 3], [0, 2, 1, 4]],
        boundary: vec![
            [1, 2, 3],
            [0, 3, 2],
            [0, 1, 3],
            [2, 1, 4],
            [0, 4, 1],
            [0, 2, 4],
        ],
    };
    mesh.to_bytes().unwrap();
}
#[test]
fn overlap_detection_survives_rotation_translation_and_uniform_scale() {
    for scale in [0.001, 1., 1000.] {
        let mut mesh = pair([0.1; 3], 0.25);
        for point in &mut mesh.points {
            let [x, y, z] = *point;
            *point = [1000. + scale * y, -500. - scale * x, 25. + scale * z];
        }
        assert_eq!(
            mesh.to_bytes().unwrap_err(),
            "overlapping tetrahedral cells"
        );
    }
}

#[test]
fn thin_positive_volume_overlap_is_not_hidden_by_unrelated_wide_axes() {
    for height in [1e-14, 1e-12, 1e-6] {
        let mut mesh = pair([0.1; 3], 1.);
        for point in &mut mesh.points {
            point[2] *= height;
        }
        // Each positive volume = height/6 > the existing 1e-15 admission floor.
        assert_eq!(
            mesh.to_bytes().unwrap_err(),
            "overlapping tetrahedral cells"
        );
        // Overlapping boxes alone do not imply overlapping tetrahedra.
        let mut separated = pair([0.4; 3], 1.);
        for point in &mut separated.points {
            point[2] *= height;
        }
        separated.to_bytes().unwrap();
    }
}
