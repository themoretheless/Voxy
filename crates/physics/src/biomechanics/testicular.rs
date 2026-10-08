//! Idealized paired solids, suspension assembly and dynamic contact qualification.
//! All dynamics use the shared finite-deformation FEM owner and contact laws.
use super::{Body, Material, Vec3};
use super::urogenital::ellipsoid;

/// Idealized paired testicular solids for the existing nonlinear FEM solver.
/// Dimensions/materials are caller supplied; this is not anatomical calibration.
/// Solids are unpinned by default; attachment vertices must be supplied explicitly.
/// Scrotal layers, cord suspension and inter-body contact are separate systems.
#[derive(Clone, Debug)]
pub struct TesticularGeometry {
    pub centers_m: [Vec3; 2],
    pub radii_m: [Vec3; 2],
    pub sectors: usize,
    pub rings: usize,
}
impl TesticularGeometry {
    /// Construct each solid with its own material and reference geometry.
    /// Invalid dimensions are rejected before mesh allocation.
    pub fn build(&self, materials: [Material; 2]) -> Result<[Body; 2], &'static str> {
        if !(3..=1024).contains(&self.sectors) || !(1..=512).contains(&self.rings)
            || self.centers_m.iter().flatten().any(|x| !x.is_finite())
            || self.radii_m.iter().flatten().any(|x| !x.is_finite() || *x <= 0.) {
            return Err("invalid testicular reference geometry");
        }
        let [left, right] = materials;
        let mut bodies = [
            ellipsoid(self.centers_m[0], self.radii_m[0], self.sectors, self.rings, left)?,
            ellipsoid(self.centers_m[1], self.radii_m[1], self.sectors, self.rings, right)?,
        ];
        // An anatomical suspension is not the generic specimen's fixed cap.
        for body in &mut bodies { body.pinned.fill(false); }
        Ok(bodies)
    }
    /// Build hollow homothetic ellipsoidal walls in the caller's outer geometry.
    /// Inner scales lie strictly in (0,1). These idealized paired walls do not
    /// define a shared scrotum or a calibrated anatomical layer distribution.
    pub fn build_shells(
        &self, inner_scales: [f64;2], materials: [Material;2],
    ) -> Result<[Body;2], &'static str> {
        if inner_scales.iter().any(|s| !s.is_finite() || *s <= 0. || *s >= 1.) {
            return Err("invalid testicular shell inner scale");
        }
        let solids = self.build(materials.clone())?;
        let mut shells = Vec::with_capacity(2);
        for (side, solid) in solids.into_iter().enumerate() {
            // The solid template's final node is its interior fan seed.
            let count = solid.rest.len()-1;
            let mut points = solid.rest[..count].to_vec();
            points.extend(solid.rest[..count].iter().map(|p| std::array::from_fn(|k|
                self.centers_m[side][k] + inner_scales[side]*(p[k]-self.centers_m[side][k]))));
            let mut cells = Vec::new();
            for mut face in solid.surface() {
                // Canonical IDs choose the same diagonal on neighboring prism faces.
                face.sort_unstable();
                let [a,b,c] = face;
                let [u,v,w] = face.map(|i| i+count);
                for cell in [[a,b,c,w],[a,b,v,w],[a,u,v,w]] {
                    cells.push((cell,materials[side].clone()));
                }
            }
            shells.push(Body::new(points,vec![false;count*2],cells)?);
        }
        Ok(shells.try_into().unwrap())
    }

    /// Conforming core/wall solids with shared interface nodes and distinct laws.
    /// A bonded interface transfers deformation continuously; sliding/separation
    /// require a different explicitly configured interface law.
    pub fn build_layered(
        &self, inner_scales: [f64;2], core_materials: [Material;2], wall_materials: [Material;2],
    ) -> Result<[Body;2], &'static str> {
        let shells = self.build_shells(inner_scales,wall_materials)?;
        let mut bodies = Vec::with_capacity(2);
        for (side,shell) in shells.into_iter().enumerate() {
            let count = shell.rest.len()/2;
            let core = ellipsoid(self.centers_m[side],self.radii_m[side].map(|r|r*inner_scales[side]),
                self.sectors,self.rings,core_materials[side].clone())?;
            let mut points = shell.rest;
            let center = points.len();
            points.push(self.centers_m[side]);
            let mut cells: Vec<_> = shell.elements.into_iter().map(|e|(e.nodes,e.material)).collect();
            for element in core.elements {
                let nodes = element.nodes.map(|i|if i == count {center} else {count+i});
                cells.push((nodes,element.material));
            }
            let pins = vec![false;points.len()];
            bodies.push(Body::new(points,pins,cells)?);
        }
        Ok(bodies.try_into().unwrap())
    }

    /// Initialize free finite-deformation dynamics with caller-supplied density.
    /// Both bodies start at rest; suspension/contact must be configured separately.
    pub fn build_dynamic(
        &self, materials: [Material; 2], density_kg_m3: [f64; 2],
    ) -> Result<[super::InertialBody; 2], &'static str> {
        self.build_supported_dynamic(materials, density_kg_m3, [&[], &[]])
    }
    /// Construct dynamics with explicitly selected nodes available to the
    /// existing prescribed-support/reaction-work solver. This does not create cords.
    pub fn build_supported_dynamic(
        &self, materials: [Material; 2], density_kg_m3: [f64; 2], vertices: [&[usize]; 2],
    ) -> Result<[super::InertialBody; 2], &'static str> {
        if density_kg_m3.iter().any(|d| !d.is_finite() || *d <= 0.) {
            return Err("invalid testicular density");
        }
        let [left, right] = self.build_with_attachments(materials, vertices)?;
        let dynamic = |body: Body, density| {
            let densities = vec![density; body.elements.len()];
            let velocities = vec![[0.; 3]; body.positions.len()];
            super::InertialBody::new_with_fixed_supports(body, &densities, velocities)
        };
        Ok([dynamic(left, density_kg_m3[0])?, dynamic(right, density_kg_m3[1])?])
    }

    /// Create a single inertial owner with stable left/right node and cell ranges.
    /// Attachments are authored in each region's local node space. Inter-region
    /// contact and scrotal layers still need explicit assembly configuration.
    pub fn build_assembly(
        &self, materials: [Material; 2], density_kg_m3: [f64; 2], vertices: [&[usize]; 2],
    ) -> Result<super::InertialAssembly, &'static str> {
        let parts = self.build_supported_dynamic(materials, density_kg_m3, vertices)?;
        super::InertialBody::assemble_tissues(&parts)
    }

    /// Assemble the two solids with a caller-authored volumetric suspension.
    /// Link node pairs are [local solid node, local suspension node]; stiffness
    /// is N/m. The third range owns suspension nodes, supports and material state.
    /// Discrete links do not establish a calibrated anatomical cord model.
    pub fn build_suspended_assembly(
        &self, materials: [Material; 2], density_kg_m3: [f64; 2],
        suspension: super::InertialBody, links: [&[([usize; 2], f64)]; 2],
    ) -> Result<super::InertialAssembly, &'static str> {
        let [left, right] = self.build_dynamic(materials, density_kg_m3)?;
        for (solid, side) in [&left, &right].into_iter().zip(links) {
            if side.iter().any(|(nodes, _)| nodes[0] >= solid.body().positions().len()
                || nodes[1] >= suspension.body().positions().len()) {
                return Err("invalid local suspension link node");
            }
        }
        let mut assembly = super::InertialBody::assemble_tissues(&[left,right,suspension])?;
        let mut pairs = Vec::new();
        for (side, links) in links.into_iter().enumerate() {
            for &(nodes, stiffness) in links {
                pairs.push(([
                    assembly.node_ranges[side].start + nodes[0],
                    assembly.node_ranges[2].start + nodes[1],
                ], stiffness));
            }
        }
        assembly.body.add_tissue_bonds(&pairs)?;
        Ok(assembly)
    }

    /// Configure one global surface-contact group, including both solids.
    /// Coefficients are discrete caller controls, not a calibrated tissue fit.
    pub fn build_contact_assembly(
        &self, materials: [Material; 2], density_kg_m3: [f64; 2], vertices: [&[usize]; 2],
        minimum_distance_m: f64, activation_gap_m: f64, pair_stiffness_n_m: f64,
    ) -> Result<super::InertialAssembly, &'static str> {
        let mut assembly = self.build_assembly(materials, density_kg_m3, vertices)?;
        let faces = assembly.body.body().surface();
        assembly.body.set_surface_contacts(vec![super::TissueSurfaceContact {
            faces, minimum_distance_m, activation_gap_m, pair_stiffness_n_m,
        }])?;
        Ok(assembly)
    }

    /// Pin explicitly selected reference vertices for a caller's suspension.
    /// Dynamic cords require coupling through the existing attachment solver.
    pub fn build_with_attachments(
        &self, materials: [Material; 2], vertices: [&[usize]; 2],
    ) -> Result<[Body; 2], &'static str> {
        let mut bodies = self.build(materials)?;
        if bodies.iter().zip(vertices).any(|(body, ids)| ids.iter().any(|&id| id >= body.positions.len())) {
            return Err("invalid testicular attachment vertex");
        }
        for (body, ids) in bodies.iter_mut().zip(vertices) {
            for &id in ids { body.pinned[id] = true; }
        }
        Ok(bodies)
    }
}

#[cfg(test)]
mod testicular_tests {
    use super::*;
    #[test]
    fn paired_dynamics_follow_free_fall_and_preserve_state_on_invalid_step() {
        let geometry = TesticularGeometry { centers_m: [[-0.02,0.,0.], [0.02,0.,0.]],
            radii_m: [[0.01,0.015,0.02]; 2], sectors: 8, rings: 3 };
        let material = Material { shear_pa: 1000., bulk_pa: 100_000., fibers: vec![] };
        assert!(geometry.build_dynamic([material.clone(), material.clone()], [0.,1000.]).is_err());
        let mut bodies = geometry.build_dynamic([material.clone(), material], [1000.,1100.]).unwrap();
        for body in &mut bodies {
            let rest = body.body().positions().to_vec();
            assert!(body.masses().iter().all(|m| m.is_finite() && *m > 0.));
            body.set_uniform_acceleration([0., -9.81, 0.]).unwrap();
            let dt = 1e-4;
            body.step(dt, 1e-8).unwrap();
            for (p, r) in body.body().positions().iter().zip(&rest) {
                assert!((p[0] - r[0]).abs() < 1e-10);
                assert!((p[1] - r[1] + 0.5 * 9.81 * dt * dt).abs() < 1e-10);
                assert!((p[2] - r[2]).abs() < 1e-10);
            }
            let positions = body.body().positions().to_vec();
            let velocities = body.velocities().to_vec();
            assert!(body.step(0., 1e-8).is_err());
            assert_eq!(body.body().positions(), positions);
            assert_eq!(body.velocities(), velocities);
        }
    }

    #[test]
    fn paired_prescribed_supports_move_and_reject_missing_targets_atomically() {
        let geometry = TesticularGeometry { centers_m: [[-0.02,0.,0.], [0.02,0.,0.]],
            radii_m: [[0.01,0.015,0.02]; 2], sectors: 8, rings: 3 };
        let material = Material { shear_pa: 1000., bulk_pa: 100_000., fibers: vec![] };
        let mut bodies = geometry.build_supported_dynamic([material.clone(), material],
            [1000.;2], [&[0], &[0]]).unwrap();
        for body in &mut bodies {
            let mut position = body.body().positions()[0];
            position[1] += 1e-7;
            let report = body.step_with_support_targets(
                &[super::super::SupportTarget { node: 0, position_m: position }], 1e-4, 1e-6).unwrap();
            assert_eq!(body.body().positions()[0], position);
            assert!(report.energy_defect_j.is_finite() && report.energy_defect_j.abs() <= 1e-6);
            assert!(body.velocities()[1..].iter().flatten().any(|v| v.abs() > 1e-12));
            let positions = body.body().positions().to_vec();
            let velocities = body.velocities().to_vec();
            assert!(body.step_with_support_targets(&[], 1e-4, 1e-6).is_err());
            assert_eq!(body.body().positions(), positions);
            assert_eq!(body.velocities(), velocities);
        }
    }

    #[test]
    fn paired_assembly_preserves_mass_and_maps_supports_to_one_owner() {
        let geometry = TesticularGeometry { centers_m: [[-0.02,0.,0.], [0.02,0.,0.]],
            radii_m: [[0.01,0.015,0.02]; 2], sectors: 8, rings: 3 };
        let material = Material { shear_pa: 1000., bulk_pa: 100_000., fibers: vec![] };
        let parts = geometry.build_supported_dynamic([material.clone(), material.clone()],
            [1000.,1100.], [&[0], &[1]]).unwrap();
        let mut assembly = geometry.build_assembly([material.clone(), material],
            [1000.,1100.], [&[0], &[1]]).unwrap();
        assert_eq!(assembly.node_ranges.len(), 2);
        assert_eq!(assembly.cell_ranges.len(), 2);
        assert_eq!(assembly.node_ranges[0].end, assembly.node_ranges[1].start);
        for (part, range) in parts.iter().zip(&assembly.node_ranges) {
            assert_eq!(&assembly.body.body().positions()[range.clone()], part.body().positions());
            assert_eq!(&assembly.body.masses()[range.clone()], part.masses());
        }
        let targets: Vec<_> = [0,1].into_iter().enumerate().map(|(side, local)| {
            let node = assembly.node_ranges[side].start + local;
            let mut position = assembly.body.body().positions()[node];
            position[1] += if side == 0 {1e-7} else {-1e-7};
            super::super::SupportTarget {node, position_m: position}
        }).collect();
        assembly.body.step_with_support_targets(&targets, 1e-4, 1e-6).unwrap();
        for target in targets { assert_eq!(assembly.body.body().positions()[target.node], target.position_m); }
    }

    #[test]
    #[ignore = "manual sustained nonlinear contact qualification and timing"]
    fn sustained_pair_contact_preserves_energy_gap_and_momentum() {
        let geometry = TesticularGeometry { centers_m: [[-0.0106,0.,0.], [0.0106,0.,0.]],
            radii_m: [[0.01,0.015,0.02];2], sectors: 8, rings: 3 };
        let material = Material { shear_pa: 1000., bulk_pa: 100_000., fibers: vec![] };
        let mut assembly = geometry.build_contact_assembly([material.clone(),material],
            [1000.;2], [&[],&[]], 0.0001, 0.002, 1.).unwrap();
        let initial = assembly.body.diagnostics().unwrap();
        let mut max_defect = 0.0_f64;
        let mut minimum_gap = f64::INFINITY;
        let started = std::time::Instant::now();
        let mut solve_duration = std::time::Duration::ZERO;
        let mut diagnostic_duration = std::time::Duration::ZERO;
        for step in 0..10_000 {
            let solve_started = std::time::Instant::now();
            let defect = assembly.body.step(2.5e-5,1e-8)
                .unwrap_or_else(|error| panic!("contact qualification failed at step {step}: {error}"));
            solve_duration += solve_started.elapsed();
            max_defect = max_defect.max(defect.abs());
            if step % 20 == 0 {
                let diagnostic_started = std::time::Instant::now();
                minimum_gap = minimum_gap.min(assembly.body.body().minimum_surface_contact_distance().unwrap().unwrap());
                diagnostic_duration += diagnostic_started.elapsed();
            }
        }
        let elapsed_ms = started.elapsed().as_secs_f64()*1000.;
        let solve_ms = solve_duration.as_secs_f64()*1000.;
        let diagnostic_ms = diagnostic_duration.as_secs_f64()*1000.;
        let final_state = assembly.body.diagnostics().unwrap();
        let drift = final_state.kinetic_j + final_state.potential_j
            - initial.kinetic_j - initial.potential_j;
        assert!(minimum_gap > 0.0001);
        assert!(drift.abs() < 1e-7, "sustained energy drift {drift:e}");
        assert!(final_state.momentum_kg_m_s.iter().all(|p| p.abs() < 1e-10));
        eprintln!("SUSTAINED PAIR CONTACT steps=10000 simulated_s=0.25 elapsed_ms={elapsed_ms:.3} solve_ms={solve_ms:.3} diagnostic_ms={diagnostic_ms:.3} mean_step_ms={:.6} min_gap_m={minimum_gap:e} max_defect_j={max_defect:e} drift_j={drift:e}", solve_ms/10_000.);
    }

    #[test]
    fn paired_contact_motion_converges_under_timestep_refinement() {
        let geometry = TesticularGeometry { centers_m: [[-0.0106,0.,0.], [0.0106,0.,0.]],
            radii_m: [[0.01,0.015,0.02]; 2], sectors: 8, rings: 3 };
        let material = Material { shear_pa: 1000., bulk_pa: 100_000., fibers: vec![] };
        let initial = geometry.build_contact_assembly([material.clone(),material],
            [1000.;2], [&[],&[]], 0.0001, 0.002, 1.).unwrap();
        let mut endpoints = Vec::new();
        for steps in [4,8,16] {
            let mut trial = initial.body.clone();
            let dt = 0.0004 / steps as f64;
            for _ in 0..steps {
                trial.step(dt, 1e-8).unwrap();
                assert!(trial.body().minimum_surface_contact_distance().unwrap().unwrap() > 0.0001);
            }
            endpoints.push(trial.body().positions().to_vec());
        }
        let distance = |a: &[[f64;3]], b: &[[f64;3]]| -> f64 {
            a.iter().zip(b).flat_map(|(a,b)| (0..3).map(move |k| (a[k]-b[k]).powi(2)))
                .sum::<f64>().sqrt()
        };
        let coarse = distance(&endpoints[0], &endpoints[1]);
        let fine = distance(&endpoints[1], &endpoints[2]);
        eprintln!("PAIRED CONTACT REFINEMENT coarse_l2_m={coarse:e} fine_l2_m={fine:e}");
        assert!(coarse > 0. && fine < coarse * 0.5,
            "contact trajectory must converge as the step is halved: {coarse:e} {fine:e}");
    }

    #[test]
    fn dynamic_contact_repels_with_balanced_momentum_and_open_gap() {
        let geometry = TesticularGeometry { centers_m: [[-0.0106,0.,0.], [0.0106,0.,0.]],
            radii_m: [[0.01,0.015,0.02]; 2], sectors: 8, rings: 3 };
        let material = Material { shear_pa: 1000., bulk_pa: 100_000., fibers: vec![] };
        let mut assembly = geometry.build_contact_assembly([material.clone(),material],
            [1000.;2], [&[],&[]], 0.0001, 0.002, 1.).unwrap();
        let defect = assembly.body.step(1e-4, 1e-8).unwrap();
        assert!(defect.is_finite() && defect.abs() <= 1e-8);
        let gap = assembly.body.body().minimum_surface_contact_distance().unwrap().unwrap();
        assert!(gap > 0.0001);
        for _ in 0..3 {
            let defect = assembly.body.step(1e-4, 1e-8).unwrap();
            assert!(defect.abs() <= 1e-8);
            assert!(assembly.body.body().minimum_surface_contact_distance().unwrap().unwrap() > 0.0001);
        }
        let momentum = |range: std::ops::Range<usize>| -> [f64;3] {
            std::array::from_fn(|axis| range.clone().map(|i|
                assembly.body.masses()[i] * assembly.body.velocities()[i][axis]).sum())
        };
        let left = momentum(assembly.node_ranges[0].clone());
        let right = momentum(assembly.node_ranges[1].clone());
        assert!(left[0] < 0. && right[0] > 0., "contact must repel both dynamic regions");
        for axis in 0..3 {
            assert!((left[axis] + right[axis]).abs() <= 1e-10);
        }
    }

    #[test]
    fn assembled_contact_is_active_and_invalid_replacement_is_atomic() {
        let geometry = TesticularGeometry { centers_m: [[-0.0106,0.,0.], [0.0106,0.,0.]],
            radii_m: [[0.01,0.015,0.02]; 2], sectors: 8, rings: 3 };
        let material = Material { shear_pa: 1000., bulk_pa: 100_000., fibers: vec![] };
        let mut assembly = geometry.build_contact_assembly([material.clone(),material],
            [1000.;2], [&[],&[]], 0.0001, 0.002, 1.).unwrap();
        let body = assembly.body.body();
        assert_eq!(body.surface_contacts().len(), 1);
        let mut exact = f64::INFINITY;
        for contact in body.surface_contacts() {
            for (i, a) in contact.faces.iter().enumerate() {
                for b in &contact.faces[i+1..] {
                    if a.iter().any(|v| b.contains(v)) { continue; }
                    exact = exact.min(super::super::surface_distance::triangle_distance(
                        a.map(|i| body.positions()[i]), b.map(|i| body.positions()[i])).unwrap().distance);
                }
            }
        }
        assert_eq!(body.minimum_surface_contact_distance().unwrap().unwrap().to_bits(), exact.to_bits());
        let pairs = body.active_surface_pairs_at(body.positions()).unwrap();
        let left = &assembly.node_ranges[0];
        let right = &assembly.node_ranges[1];
        assert!(pairs.iter().any(|(_, a, b, _, energy)| *energy > 0.
            && ((a.iter().all(|i| left.contains(i)) && b.iter().all(|i| right.contains(i)))
                || (a.iter().all(|i| right.contains(i)) && b.iter().all(|i| left.contains(i))))));
        let positions = body.positions().to_vec();
        let energy = assembly.body.diagnostics().unwrap().potential_j;
        assert!(assembly.body.set_surface_contacts(vec![super::super::TissueSurfaceContact {
            faces: vec![[0,1,2]], minimum_distance_m: -1., activation_gap_m: 0.002,
            pair_stiffness_n_m: 1.,
        }]).is_err());
        assert_eq!(assembly.body.body().positions(), positions);
        assert_eq!(assembly.body.body().surface_contacts().len(), 1);
        assert_eq!(assembly.body.diagnostics().unwrap().potential_j.to_bits(), energy.to_bits());
    }

    #[test]
    fn dynamic_elastic_link_retains_state_and_matches_extension_energy() {
        let geometry = TesticularGeometry { centers_m: [[-0.02,0.,0.], [0.02,0.,0.]],
            radii_m: [[0.01,0.015,0.02];2], sectors: 8, rings: 3 };
        let material = Material { shear_pa: 1000., bulk_pa: 100_000., fibers: vec![] };
        let mut assembly = geometry.build_assembly([material.clone(),material],
            [1000.;2], [&[],&[]]).unwrap();
        let unlinked = assembly.body.clone();
        let nodes = [assembly.node_ranges[0].start,assembly.node_ranges[1].start];
        assert!(assembly.body.add_tissue_bonds(&[(nodes,10.)]).unwrap().abs() < 1e-12);
        assert_eq!(assembly.body.masses(), unlinked.masses());
        assert_eq!(assembly.body.velocities(), unlinked.velocities());
        let mut displaced = assembly.body.body().positions().to_vec();
        for i in assembly.node_ranges[1].clone() { displaced[i][0] += 0.001; }
        let linked = assembly.body.body().evaluate(&displaced).unwrap().0;
        let free = unlinked.body().evaluate(&displaced).unwrap().0;
        assert!((linked - free - 0.5*10.*0.001_f64.powi(2)).abs() < 1e-12);
        let positions = assembly.body.body().positions().to_vec();
        assert!(assembly.body.add_tissue_bonds(&[(nodes,10.)]).is_err());
        assert_eq!(assembly.body.body().positions(), positions);
        assert_eq!(assembly.body.body().tissue_bonds().len(),1);
    }

    #[test]
    fn volumetric_suspension_links_transfer_force_without_moving_fixed_supports() {
        let geometry = TesticularGeometry { centers_m: [[-0.02,0.,0.], [0.02,0.,0.]],
            radii_m: [[0.01,0.015,0.02];2], sectors: 8, rings: 3 };
        let material = Material { shear_pa: 1000., bulk_pa: 100_000., fibers: vec![] };
        let support = Body::new(vec![[0.,0.04,0.],[0.005,0.04,0.],
            [0.,0.045,0.],[0.,0.04,0.005]], vec![true;4],
            vec![([0,1,2,3],material.clone())]).unwrap();
        let suspension = super::super::InertialBody::new_with_fixed_supports(
            support, &[1000.], vec![[0.;3];4]).unwrap();
        let links = [([0,0],10.)];
        let mut assembly = geometry.build_suspended_assembly(
            [material.clone(),material.clone()], [1000.;2], suspension.clone(), [&links,&links]).unwrap();
        assert_eq!(assembly.node_ranges.len(),3);
        assert_eq!(assembly.body.body().tissue_bonds().len(),2);
        let support_range = assembly.node_ranges[2].clone();
        let rest_support = assembly.body.body().positions()[support_range.clone()].to_vec();
        assembly.body.set_uniform_acceleration([0.,-9.81,0.]).unwrap();
        assembly.body.step(1e-5,1e-8).unwrap();
        assert_eq!(&assembly.body.body().positions()[support_range.clone()], rest_support);
        assert!(assembly.body.velocities()[support_range].iter().flatten().all(|v| *v == 0.));
        let posed = assembly.body.body().positions();
        let (_, gradient) = assembly.body.body().evaluate(posed).unwrap();
        assert!(gradient[assembly.node_ranges[2].start].iter().any(|g| g.abs() > 1e-12));
        let bad = [([usize::MAX,0],10.)];
        assert!(geometry.build_suspended_assembly([material.clone(),material],
            [1000.;2], suspension, [&bad,&links]).is_err());
    }

    #[test]
    fn outer_wall_motion_transfers_to_core_with_layered_mass_and_energy_guard() {
        let geometry = TesticularGeometry {centers_m:[[-0.02,0.,0.],[0.02,0.,0.]],
            radii_m:[[0.015,0.02,0.025];2],sectors:8,rings:3};
        let core = Material {shear_pa:1000.,bulk_pa:100_000.,fibers:vec![]};
        let wall = Material {shear_pa:3000.,bulk_pa:200_000.,fibers:vec![]};
        for body in geometry.build_layered([0.8,0.9],[core.clone(),core],[wall.clone(),wall]).unwrap() {
            let outer_count = (body.rest.len()-1)/2;
            // These fixture laws identify its authored regions; no density is
            // inferred from stiffness in the production solver.
            let density: Vec<_> = body.elements.iter().map(|e|
                if e.material.shear_pa == 1000. {1000.} else {1100.}).collect();
            let expected_mass: f64 = body.elements.iter().zip(&density).map(|(e,d)|e.volume*d).sum();
            let mut velocity = vec![[0.;3];body.rest.len()];
            velocity[..outer_count].fill([0.001,0.,0.]);
            let mut dynamic = super::super::InertialBody::new(body,&density,velocity).unwrap();
            assert!((dynamic.masses().iter().sum::<f64>()-expected_mass).abs() < 1e-14);
            for _ in 0..4 { assert!(dynamic.step(1e-5,1e-8).unwrap().abs() <= 1e-8); }
            assert!(dynamic.velocities()[outer_count..].iter().flatten().any(|v|v.abs()>1e-12));
            assert!(dynamic.body().positions().iter().flatten().all(|p|p.is_finite()));
        }
    }

    #[test]
    fn bonded_core_wall_shares_nodes_preserves_volume_and_distinct_laws() {
        let geometry = TesticularGeometry {centers_m:[[-0.02,0.,0.],[0.02,0.,0.]],
            radii_m:[[0.015,0.02,0.025];2],sectors:8,rings:3};
        let core = Material {shear_pa:1000.,bulk_pa:100_000.,fibers:vec![]};
        let wall = Material {shear_pa:3000.,bulk_pa:200_000.,fibers:vec![]};
        let solids = geometry.build([core.clone(),core.clone()]).unwrap();
        let layered = geometry.build_layered([0.8,0.9],[core.clone(),core], [wall.clone(),wall]).unwrap();
        for (side,body) in layered.iter().enumerate() {
            let total: f64 = body.elements.iter().map(|e|e.volume).sum();
            let reference: f64 = solids[side].elements.iter().map(|e|e.volume).sum();
            assert!((total-reference).abs() < 1e-15);
            assert_eq!(body.surface().len(),solids[side].surface().len());
            let core_volume: f64 = body.elements.iter().filter(|e|e.material.shear_pa == 1000.).map(|e|e.volume).sum();
            assert!((core_volume-reference*[0.8_f64,0.9][side].powi(3)).abs() < 1e-15);
            assert!(body.elements.iter().any(|e|e.material.shear_pa == 3000.));
            let interface = (body.rest.len()-1)/2;
            for node in interface..2*interface {
                assert!(body.elements.iter().any(|e|e.material.shear_pa == 1000. && e.nodes.contains(&node)));
                assert!(body.elements.iter().any(|e|e.material.shear_pa == 3000. && e.nodes.contains(&node)));
            }
        }
    }

    #[test]
    fn hollow_shells_preserve_wall_volume_and_have_no_internal_boundary_faces() {
        let geometry = TesticularGeometry { centers_m: [[-0.02,0.,0.],[0.02,0.,0.]],
            radii_m: [[0.015,0.02,0.025];2],sectors: 8,rings: 3 };
        let material = Material {shear_pa: 1000.,bulk_pa: 100_000.,fibers: vec![]};
        let solids = geometry.build([material.clone(),material.clone()]).unwrap();
        let shells = geometry.build_shells([0.8,0.9],[material.clone(),material.clone()]).unwrap();
        for (side,shell) in shells.iter().enumerate() {
            let solid_volume: f64 = solids[side].elements.iter().map(|e| e.volume).sum();
            let shell_volume: f64 = shell.elements.iter().map(|e| e.volume).sum();
            let scale = [0.8_f64,0.9][side];
            assert!((shell_volume-solid_volume*(1.-scale.powi(3))).abs() < 1e-15);
            assert_eq!(shell.surface().len(),solids[side].surface().len()*2);
            assert!(shell.elements.iter().all(|e| e.volume > 0.));
        }
        for invalid in [0.,1.,-1.,f64::NAN] {
            assert!(geometry.build_shells([invalid,0.8],[material.clone(),material.clone()]).is_err());
        }
    }

    #[test]
    fn paired_solids_use_validated_caller_geometry() {
        let geometry = TesticularGeometry { centers_m: [[-0.02,0.,0.], [0.02,0.,0.]],
            radii_m: [[0.01,0.015,0.02], [0.012,0.015,0.02]], sectors: 8, rings: 3 };
        let material = Material { shear_pa: 1000., bulk_pa: 100_000., fibers: vec![] };
        let free = geometry.build([material.clone(), material.clone()]).unwrap();
        assert!(free.iter().all(|body| body.pinned.iter().all(|pin| !pin)));
        let attached = geometry.build_with_attachments(
            [material.clone(), material.clone()], [&[0, 1], &[2]]).unwrap();
        assert_eq!(attached[0].pinned.iter().filter(|pin| **pin).count(), 2);
        assert_eq!(attached[1].pinned.iter().filter(|pin| **pin).count(), 1);
        assert!(attached[0].pinned[0] && attached[0].pinned[1] && attached[1].pinned[2]);
        assert!(geometry.build_with_attachments([material.clone(), material.clone()],
            [&[], &[usize::MAX]]).is_err());
        for invalid in [0., -0.01, f64::NAN, f64::INFINITY] {
            let mut bad = geometry.clone(); bad.radii_m[1][2] = invalid;
            assert!(bad.build([material.clone(), material.clone()]).is_err());
        }
        let mut bad = geometry; bad.rings = 0;
        assert!(bad.build([material.clone(), material]).is_err());
    }
}

