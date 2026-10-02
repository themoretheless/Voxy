use physics::moisture::{Body, Cell, Link};
#[test]
fn exhausted_cells_are_removed_and_changed_graph_conserves_retained_water() {
    let body = Body::new(
        vec![
            Cell {
                capacity_kg: 2.,
                water_kg: 1.,
            },
            Cell {
                capacity_kg: 1.,
                water_kg: 1.,
            },
            Cell {
                capacity_kg: 4.,
                water_kg: 0.,
            },
        ],
        vec![],
    )
    .unwrap();
    let mut p = body
        .partition_material(
            &[0.5, 0., 1.],
            &[Link {
                cells: [0, 2],
                conductance_kg_s: 1.,
            }],
        )
        .unwrap();
    assert_eq!(p.remap, vec![Some(0), None, Some(1)]);
    assert_eq!(p.removed_water_kg, vec![0.5, 1., 0.]);
    assert_eq!(p.removed_capacity_kg, vec![1., 1., 0.]);
    let remaining = p.remaining.as_mut().unwrap();
    assert_eq!(remaining.cells()[0].saturation().unwrap(), 0.5);
    let transfer = remaining.advance(1., &[]).unwrap();
    assert!(transfer.mass_defect_kg.abs() < 1e-14);
    assert!((remaining.cells().iter().map(|c| c.water_kg).sum::<f64>() - 0.5).abs() < 1e-14);
    assert!((remaining.cells()[0].water_kg - 5. / 18.).abs() < 1e-14);
    assert_eq!(body.cells().len(), 3);
    assert_eq!(body.cells()[1].water_kg, 1.);
    let all = body.partition_material(&[0.; 3], &[]).unwrap();
    assert!(all.remaining.is_none());
    assert_eq!(all.removed_water_kg.iter().sum::<f64>(), 2.);
}
#[test]
fn geometry_must_explicitly_disconnect_exhausted_material() {
    let body = Body::new(
        vec![
            Cell {
                capacity_kg: 1.,
                water_kg: 0.5
            };
            2
        ],
        vec![],
    )
    .unwrap();
    assert!(
        body.partition_material(
            &[0., 1.],
            &[Link {
                cells: [0, 1],
                conductance_kg_s: 1.
            }]
        )
        .is_err()
    );
    assert!(body.partition_material(&[1., f64::NAN], &[]).is_err());
    assert_eq!(body.cells()[0].water_kg, 0.5);
}
