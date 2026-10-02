use physics::biomechanics::*;
fn specimen(offset: f64, load: f64, steps: usize) -> Body {
    let material = Material::from_young_poisson(1500., 0.35).unwrap();
    let mut b = Body::new(
        vec![
            [offset, 0., 0.],
            [offset + 0.02, 0., 0.],
            [offset, 0.02, 0.],
            [offset, 0., 0.02],
        ],
        vec![true, false, true, true],
        vec![([0, 1, 2, 3], material)],
    )
    .unwrap();
    b.set_viscoelastic_ogden(
        0,
        ViscoelasticOgden::new(
            vec![OgdenTerm {
                shear_pa: 500.,
                exponent: 2.,
            }],
            5000.,
            vec![MaxwellBranch {
                shear_pa: 1000.,
                relaxation_seconds: 0.1,
            }],
        )
        .unwrap(),
    )
    .unwrap();
    b.set_force(1, [load, 0., 0.]).unwrap();
    for _ in 0..steps {
        b.relax_step(0.02, 10000, 1e-9).unwrap();
    }
    b
}
fn probe() -> Matrix {
    [[1.03, 0.01, 0.], [0., 0.99, 0.], [0., 0., 1.]]
}
#[test]
fn assembly_preserves_deformation_and_distinct_viscous_histories_exactly() {
    let a = specimen(0., 0.002, 2);
    let b = specimen(0.04, 0.003, 5);
    let assembly = Body::assemble_tissues(&[a.clone(), b.clone()]).unwrap();
    assert_eq!(&assembly.body.positions()[0..4], a.positions());
    assert_eq!(&assembly.body.positions()[4..8], b.positions());
    for (i, source) in [a.clone(), b.clone()].iter().enumerate() {
        let old = source.elements()[0].response(probe()).unwrap();
        let new = assembly.body.elements()[i].response(probe()).unwrap();
        assert_eq!(old.first_piola, new.first_piola);
        assert_eq!(old.energy_density, new.energy_density);
    }
    assert_ne!(
        assembly.body.elements()[0]
            .response(probe())
            .unwrap()
            .first_piola,
        assembly.body.elements()[1]
            .response(probe())
            .unwrap()
            .first_piola
    );
    let (energy, gradient) = assembly.body.evaluate(assembly.body.positions()).unwrap();
    let (ea, ga) = a.evaluate(a.positions()).unwrap();
    let (eb, gb) = b.evaluate(b.positions()).unwrap();
    assert!((energy - ea - eb).abs() < 1e-14);
    assert_eq!(&gradient[..4], ga);
    assert_eq!(&gradient[4..], gb);
}
#[test]
fn merged_relaxation_matches_separate_advances_and_bonded_failure_preserves_memory() {
    let mut a = specimen(0., 0.002, 2);
    let mut b = specimen(0.04, 0.003, 5);
    let mut assembly = Body::assemble_tissues(&[a.clone(), b.clone()]).unwrap();
    a.relax_step(0.03, 10000, 1e-10).unwrap();
    b.relax_step(0.03, 10000, 1e-10).unwrap();
    assembly.body.relax_step(0.03, 20000, 1e-10).unwrap();
    for (i, source) in [a, b].iter().enumerate() {
        for (p, q) in assembly.body.positions()[i * 4..i * 4 + 4]
            .iter()
            .zip(source.positions())
        {
            for k in 0..3 {
                assert!((p[k] - q[k]).abs() < 1e-9);
            }
        }
        let x = assembly.body.elements()[i]
            .response(probe())
            .unwrap()
            .first_piola;
        let y = source.elements()[0].response(probe()).unwrap().first_piola;
        for row in 0..3 {
            for col in 0..3 {
                assert!((x[row][col] - y[row][col]).abs() < 1e-5);
            }
        }
    }
    assembly.body.add_tissue_bonds(&[([1, 5], 10.)]).unwrap();
    let original = assembly.body.clone();
    assert!(assembly.body.relax_step(0.05, 1, 1e-16).is_err());
    assert_eq!(assembly.body.positions(), original.positions());
    for i in 0..2 {
        assert_eq!(
            assembly.body.elements()[i]
                .response(probe())
                .unwrap()
                .first_piola,
            original.elements()[i]
                .response(probe())
                .unwrap()
                .first_piola
        );
    }
    assert_eq!(
        assembly.body.evaluate(assembly.body.positions()).unwrap(),
        original.evaluate(original.positions()).unwrap()
    );
    assert!(
        assembly
            .body
            .relax_step(0.05, 20000, 1e-9)
            .unwrap()
            .converged
    );
    assert_ne!(
        assembly.body.elements()[0]
            .response(probe())
            .unwrap()
            .first_piola,
        original.elements()[0]
            .response(probe())
            .unwrap()
            .first_piola
    );
}

fn add_store(body: &mut Body, pressure: f64) {
    let volumes: Vec<_> = body
        .stresses_at(body.positions())
        .unwrap()
        .iter()
        .map(|e| e.reference_volume_m3)
        .collect();
    body.set_cell_pore_fluids(
        volumes
            .iter()
            .map(|&v| PoreFluid {
                reference_fluid_volume_m3: 0.3 * v,
                fluid_volume_m3: 0.3 * v + pressure * v * 1e-4,
                biot_coefficient: 0.8,
                storage_m3_per_pa: v * 1e-4,
            })
            .collect(),
    )
    .unwrap();
}
#[test]
fn cell_pore_assembly_preserves_inventory_pressure_energy_gradient_and_memory() {
    let mut a = specimen(0., 0.002, 2);
    let mut b = specimen(0.04, 0.003, 5);
    add_store(&mut a, 20.);
    add_store(&mut b, 40.);
    let assembly = Body::assemble_tissues(&[a.clone(), b.clone()]).unwrap();
    assert_eq!(assembly.cell_ranges, vec![0..1, 1..2]);
    let (pa, ea) = a.cell_pore_response_at(a.positions()).unwrap();
    let (pb, eb) = b.cell_pore_response_at(b.positions()).unwrap();
    let (pressures, energy) = assembly
        .body
        .cell_pore_response_at(assembly.body.positions())
        .unwrap();
    assert_eq!(pressures, [pa[0], pb[0]]);
    assert!((energy - ea - eb).abs() < 1e-14);
    let mut sum = 0.;
    for (i, part) in [&a, &b].iter().enumerate() {
        let original = part.cell_pore_fluids()[0];
        let merged = assembly.body.cell_pore_fluids()[i];
        assert_eq!(original.fluid_volume_m3, merged.fluid_volume_m3);
        assert_eq!(
            original.reference_fluid_volume_m3,
            merged.reference_fluid_volume_m3
        );
        assert_eq!(original.storage_m3_per_pa, merged.storage_m3_per_pa);
        assert_eq!(original.biot_coefficient, merged.biot_coefficient);
        assert_eq!(
            part.elements()[0].response(probe()).unwrap().first_piola,
            assembly.body.elements()[i]
                .response(probe())
                .unwrap()
                .first_piola
        );
        sum += original.fluid_volume_m3;
    }
    assert_eq!(
        sum,
        assembly
            .body
            .cell_pore_fluids()
            .iter()
            .map(|f| f.fluid_volume_m3)
            .sum::<f64>()
    );
    let (e, g) = assembly.body.evaluate(assembly.body.positions()).unwrap();
    let (ea, ga) = a.evaluate(a.positions()).unwrap();
    let (eb, gb) = b.evaluate(b.positions()).unwrap();
    assert!((e - ea - eb).abs() < 1e-14);
    assert_eq!(&g[..4], ga);
    assert_eq!(&g[4..], gb);
    let mut bonded = assembly.body;
    bonded.add_tissue_bonds(&[([1, 5], 10.)]).unwrap();
    let inventories: Vec<_> = bonded
        .cell_pore_fluids()
        .iter()
        .map(|f| f.fluid_volume_m3)
        .collect();
    bonded.relax_step(0.02, 20000, 1e-9).unwrap();
    assert_eq!(
        inventories,
        bonded
            .cell_pore_fluids()
            .iter()
            .map(|f| f.fluid_volume_m3)
            .collect::<Vec<_>>()
    );
    assert!(
        bonded
            .cell_pore_response_at(bonded.positions())
            .unwrap()
            .0
            .iter()
            .all(|p| p.is_finite())
    );
}
#[test]
fn cell_pore_assembly_rejects_implicit_dry_inventories_and_uniform_pressure_merging() {
    let mut a = specimen(0., 0.002, 1);
    let b = specimen(0.04, 0.003, 1);
    add_store(&mut a, 20.);
    assert!(Body::assemble_tissues(&[a.clone(), b.clone()]).is_err());
    let reference = a.reference_volume();
    a.set_pore_fluid(PoreFluid {
        reference_fluid_volume_m3: 0.3 * reference,
        fluid_volume_m3: 0.3 * reference,
        biot_coefficient: 0.8,
        storage_m3_per_pa: reference * 1e-4,
    })
    .unwrap();
    assert!(Body::assemble_tissues(&[a, b]).is_err());
}
