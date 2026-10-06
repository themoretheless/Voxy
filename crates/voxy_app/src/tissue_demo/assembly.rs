//! One global state owner for skeletal tissue regions; shared controller/solver.
use super::*;
use std::ops::Range;

#[derive(Clone, Debug)]
pub(super) struct AssembledRegions {
    pub(super) node_ranges: Vec<Range<usize>>,
    pub(super) cell_ranges: Vec<Range<usize>>,
    source_contacts: Vec<Option<PrescribedTriangleSurface>>,
}

impl TissueDemo {
    /// Replace regional dynamics with one global owner, retaining authored
    /// attachment indices through immutable regional ranges. No source owner
    /// continues to simulate after publication.
    pub(crate) fn assemble_regions(&mut self) -> Result<(), &'static str> {
        if self.assembled_regions.is_some()
            || !self.body_mode
            || self.bodies.len() != self.attachments.len()
        {
            return Err("invalid tissue regional assembly mode");
        }
        let mut parts = Vec::new();
        let mut cells = Vec::new();
        let mut boundary = Vec::new();
        let mut ledger = EnergyLedger::default();
        let mut initial_energy_j = 0.;
        let mut preferred_depth = 0;
        let mut node_offset = 0;
        for (i, part) in self.bodies.iter().enumerate() {
            let DemoTissue::Continuum {
                dynamics,
                ledger: receipt,
                initial_energy_j: initial,
                preferred_depth: depth,
                cells: local_cells,
                ..
            } = part
            else {
                return Err("assembly requires continuum regions");
            };
            let surface = self
                .surfaces
                .get(i)
                .ok_or("missing tissue render embedding")?;
            boundary.extend(surface.deform(dynamics.body().rest_positions())?);
            cells.extend(local_cells.iter().map(|c| c.map(|n| n + node_offset)));
            node_offset += dynamics.body().rest_positions().len();
            ledger.add(*receipt);
            initial_energy_j += initial;
            preferred_depth = preferred_depth.max(*depth);
            parts.push(dynamics.clone());
        }
        let assembled = InertialBody::assemble_tissues(&parts)?;
        let render =
            EmbeddedSurface::bind(assembled.body.body().rest_positions(), &cells, &boundary)?;
        let binding = Arc::new(SolidFilmBinding::new(&assembled.body)?);
        let layout = AssembledRegions {
            node_ranges: assembled.node_ranges,
            cell_ranges: assembled.cell_ranges,
            source_contacts: parts
                .iter()
                .map(|p| p.prescribed_surface().cloned())
                .collect(),
        };
        let global = DemoTissue::Continuum {
            boundary_faces: assembled.body.body().surface().into(),
            dynamics: assembled.body,
            thermal_binding: binding,
            cells,
            ledger,
            initial_energy_j,
            preferred_depth,
        };
        self.bodies = vec![global];
        self.surfaces = vec![render];
        self.assembled_regions = Some(Arc::new(layout));
        Ok(())
    }
    pub(super) fn step_assembled_regions(
        &mut self,
        palette: &[DMat4],
        conductivity: f64,
        next_surfaces: Option<Vec<Arc<PrescribedTriangleSurface>>>,
    ) -> Result<(), &'static str> {
        let layout = self
            .assembled_regions
            .as_ref()
            .ok_or("missing assembled tissue layout")?;
        if self.bodies.len() != 1 || layout.node_ranges.len() != self.attachments.len() {
            return Err("assembled tissue topology changed");
        }
        let mut targets = Vec::new();
        for ((joint, pins), range) in self.attachments.iter().zip(&layout.node_ranges) {
            let matrix = palette
                .get(*joint)
                .ok_or("missing tissue attachment bone")?;
            if !matrix.is_finite() {
                return Err("nonfinite tissue attachment");
            }
            for (node, rest) in pins {
                if *node >= range.len() {
                    return Err("assembled attachment index changed");
                }
                let position_m = matrix.transform_point3(DVec3::from_array(*rest)).to_array();
                if position_m.iter().any(|v| !v.is_finite()) {
                    return Err("nonfinite tissue attachment");
                }
                targets.push(SupportTarget {
                    node: range.start + node,
                    position_m,
                });
            }
        }
        let mut candidate = self.bodies[0].clone();
        let DemoTissue::Continuum { dynamics, .. } = &candidate else {
            return Err("assembly requires continuum body");
        };
        let next = if let Some(surfaces) = next_surfaces {
            if surfaces.len() != layout.source_contacts.len() {
                return Err("tissue contact region count mismatch");
            }
            let points = surfaces
                .first()
                .ok_or("missing assembled contact pose")?
                .positions();
            for (source, staged) in layout.source_contacts.iter().zip(&surfaces) {
                if !source
                    .as_ref()
                    .is_some_and(|s| s.same_contact_owner(staged))
                    || staged.positions() != points
                {
                    return Err("assembled regional contact owner or pose changed");
                }
            }
            Some(Arc::new(
                dynamics
                    .prescribed_surface()
                    .ok_or("surface motion requires installed contact")?
                    .with_positions(points.to_vec())?,
            ))
        } else {
            None
        };
        let next_skin = if let Some(binding) = &self.skin_contact_binding {
            if self
                .attachments
                .iter()
                .map(|(joint, _)| *joint)
                .collect::<Vec<_>>()
                != binding.joints
                || layout.node_ranges != binding.node_ranges
                || dynamics.body().rest_positions() != binding.rest
            {
                return Err("assembled skin owner or topology changed");
            }
            let surface = next
                .as_ref()
                .ok_or("skin motion requires staged source pose")?;
            let current = dynamics
                .body()
                .stationary_embedded_contact()
                .ok_or("missing installed skin contact")?;
            Some(
                current.with_pose(
                    binding.posed_reference(palette)?,
                    surface.positions().to_vec(),
                    Arc::new(
                        current
                            .obstacle()
                            .with_positions(surface.positions().to_vec())?,
                    ),
                )?,
            )
        } else {
            None
        };
        Self::step_continuum_targets_with_skin(
            &mut candidate,
            &targets,
            conductivity,
            next.as_ref(),
            next_skin.as_ref(),
        )?;
        self.bodies[0] = candidate;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn distant_surface() -> Arc<PrescribedTriangleSurface> {
        Arc::new(
            PrescribedTriangleSurface::new(
                vec![[-4., -4., -10.], [4., -4., -10.], [0., 4., -10.]],
                vec![[0, 1, 2]],
                0.0001,
                0.003,
                100.,
            )
            .unwrap(),
        )
    }
    #[test]
    fn assembled_controller_drives_all_regional_supports_in_one_owner() {
        let mut demo = TissueDemo::body();
        let surface = distant_surface();
        demo.bind_contact_surface(surface.clone()).unwrap();
        let sources = demo.bodies.clone();
        demo.assemble_regions().unwrap();
        assert_eq!(demo.bodies.len(), 1);
        let layout = demo.assembled_regions.as_ref().unwrap().clone();
        assert_eq!(layout.node_ranges.len(), 4);
        assert_eq!(layout.cell_ranges.len(), 4);
        let DemoTissue::Continuum { dynamics, .. } = &demo.bodies[0] else {
            panic!()
        };
        for (i, source) in sources.iter().enumerate() {
            let DemoTissue::Continuum {
                dynamics: original, ..
            } = source
            else {
                panic!()
            };
            assert_eq!(
                &dynamics.body().positions()[layout.node_ranges[i].clone()],
                original.body().positions()
            );
            assert_eq!(
                &dynamics.masses()[layout.node_ranges[i].clone()],
                original.masses()
            );
        }
        let before = dynamics.diagnostics().unwrap();
        let mut palette = vec![DMat4::IDENTITY; 8];
        palette[0] = DMat4::from_translation(DVec3::new(0., 1e-5, 0.));
        palette[3] = DMat4::from_translation(DVec3::new(2e-5, 0., 0.));
        palette[7] = DMat4::from_translation(DVec3::new(0., 0., 3e-5));
        let next = Arc::new(
            surface
                .with_positions(
                    surface
                        .positions()
                        .iter()
                        .map(|p| [p[0], p[1], p[2] + 1e-5])
                        .collect(),
                )
                .unwrap(),
        );
        demo.step_body_with_contact64_workers(&palette, 0.5, Some(vec![next; 4]), 4)
            .unwrap();
        let DemoTissue::Continuum {
            dynamics, ledger, ..
        } = &demo.bodies[0]
        else {
            panic!()
        };
        let after = dynamics.diagnostics().unwrap();
        for ((joint, pins), range) in demo.attachments.iter().zip(&layout.node_ranges) {
            for (node, rest) in pins {
                assert_eq!(
                    dynamics.body().positions()[range.start + node],
                    palette[*joint]
                        .transform_point3(DVec3::from_array(*rest))
                        .to_array()
                );
            }
        }
        let independent = after.kinetic_j - before.kinetic_j + after.potential_j
            - before.potential_j
            + ledger.heat_j
            - ledger.support_work_j
            - ledger.surface_work_j;
        assert!(independent.abs() < 1e-5, "independent={independent}");
        assert!(
            (dynamics
                .maxwell_sensible_energy_j()
                .unwrap()
                .iter()
                .sum::<f64>()
                - ledger.heat_j)
                .abs()
                < 1e-10
        );
        assert!(ledger.accepted_steps >= 4);
        assert!(!demo.tissue_mesh().unwrap().vertices().is_empty());
        println!(
            "ASSEMBLED_CONTROLLER regions=4 owners=1 independent_defect_j={independent:.17e} accepted_steps={}",
            ledger.accepted_steps
        );
    }
    #[test]
    fn assembled_controller_rejects_rebound_surface_and_last_joint_atomically() {
        let mut demo = TissueDemo::body();
        let native = distant_surface();
        demo.bind_contact_surface(native.clone()).unwrap();
        demo.assemble_regions().unwrap();
        let before = format!("{demo:?}");
        assert!(demo.bind_contact_surface(native.clone()).is_err());
        assert_eq!(before, format!("{demo:?}"));
        let mut palette = vec![DMat4::IDENTITY; 8];
        palette[7] = DMat4::from_cols_array(&[f64::NAN; 16]);
        assert!(
            demo.step_body_with_contact64_workers(&palette, 0., Some(vec![native.clone(); 4]), 4)
                .is_err()
        );
        assert_eq!(before, format!("{demo:?}"));
        let palette = vec![DMat4::IDENTITY; 8];
        let rebound = distant_surface();
        assert!(
            demo.step_body_with_contact64_workers(&palette, 0., Some(vec![rebound; 4]), 1)
                .is_err()
        );
        assert_eq!(before, format!("{demo:?}"));
        let mut calls = 0;
        assert!(
            demo.advance_with_palette64_and_surfaces(2. / 240., |_| {
                calls += 1;
                if calls == 2 {
                    return Err("fixture second frame failure");
                }
                Ok((palette.clone(), vec![native.clone(); 4]))
            })
            .is_err()
        );
        assert_eq!(calls, 2);
        assert_eq!(before, format!("{demo:?}"));
    }
}

#[derive(Debug)]
pub(super) struct GlobalSkinBinding {
    embedding: Arc<EmbeddedSurface>,
    owners: Vec<Option<usize>>,
    skin_reference: Vec<[f64; 3]>,
    rest: Vec<[f64; 3]>,
    cells: Vec<[usize; 4]>,
    node_ranges: Vec<Range<usize>>,
    joints: Vec<usize>,
    vertex_count: usize,
    pub(super) bound_count: usize,
}
impl GlobalSkinBinding {
    pub(super) fn tissue_owned_vertices(&self) -> Vec<usize> {
        self.owners
            .iter()
            .enumerate()
            .filter_map(|(vertex, owner)| owner.map(|_| vertex))
            .collect()
    }
}
impl TissueDemo {
    pub(super) fn bind_assembled_skin(
        &self,
        skin: &[[f64; 3]],
    ) -> Result<TissueSkinBinding, &'static str> {
        let layout = self
            .assembled_regions
            .as_ref()
            .ok_or("missing assembled tissue layout")?;
        let Some(DemoTissue::Continuum {
            dynamics, cells, ..
        }) = self.bodies.first()
        else {
            return Err("missing assembled tissue owner");
        };
        if skin.iter().flatten().any(|x| !x.is_finite()) {
            return Err("nonfinite skin binding");
        }
        let rest = dynamics.body().rest_positions();
        let mut owned = vec![false; skin.len()];
        let mut owners = vec![None; skin.len()];
        for (region, range) in layout.cell_ranges.iter().enumerate() {
            let local = cells
                .get(range.clone())
                .ok_or("assembled skin cells changed")?;
            for (i, point) in skin.iter().enumerate() {
                match EmbeddedSurface::bind(rest, local, &[*point]) {
                    Ok(_) => {
                        if owned[i] {
                            return Err("overlapping skin tissue ownership");
                        }
                        owned[i] = true;
                        owners[i] = Some(region);
                    }
                    Err("surface vertex outside tetrahedral mesh") => {}
                    Err(error) => return Err(error),
                }
            }
        }
        let global = GlobalSkinBinding {
            embedding: Arc::new(EmbeddedSurface::bind_relative(rest, cells, skin, &owned)?),
            owners,
            skin_reference: skin.to_vec(),
            rest: rest.to_vec(),
            cells: cells.clone(),
            node_ranges: layout.node_ranges.clone(),
            joints: self.attachments.iter().map(|(joint, _)| *joint).collect(),
            vertex_count: skin.len(),
            bound_count: owned.iter().filter(|&&v| v).count(),
        };
        Ok(TissueSkinBinding {
            regions: Vec::new(),
            vertex_count: skin.len(),
            global: Some(Arc::new(global)),
        })
    }
    pub(super) fn deform_assembled_skin(
        &self,
        binding: &GlobalSkinBinding,
        palette: &[DMat4],
        skin: &[[f64; 3]],
    ) -> Result<Vec<[f64; 3]>, &'static str> {
        let layout = self
            .assembled_regions
            .as_ref()
            .ok_or("missing assembled tissue layout")?;
        let Some(DemoTissue::Continuum {
            dynamics, cells, ..
        }) = self.bodies.first()
        else {
            return Err("missing assembled tissue owner");
        };
        if dynamics.body().rest_positions() != binding.rest
            || cells != &binding.cells
            || layout.node_ranges != binding.node_ranges
            || self.attachments.iter().map(|(j, _)| *j).collect::<Vec<_>>() != binding.joints
        {
            return Err("assembled skin owner or topology changed");
        }
        if skin.len() != binding.vertex_count {
            return Err("invalid posed skin");
        }
        let reference = binding.posed_reference(palette)?;
        let mut output = vec![[0.; 3]; skin.len()];
        binding.embedding.deform_relative_into(
            &reference,
            dynamics.body().positions(),
            skin,
            &mut output,
        )?;
        Ok(output)
    }
}

#[cfg(test)]
mod skin_tests {
    use super::*;
    // Build held-pose fixtures through existing Body construction; do not add
    // a mechanics mutation API just to exercise the render chain rule.
    fn held_pose(dynamics: &mut InertialBody, positions: &[[f64; 3]]) {
        let cells = dynamics
            .body()
            .elements()
            .iter()
            .map(|e| (e.nodes, e.material.clone()))
            .collect::<Vec<_>>();
        let count = cells.len();
        let mut solid = Body::new(
            dynamics.body().rest_positions().to_vec(),
            vec![false; positions.len()],
            cells,
        )
        .unwrap();
        solid.restore_diagnostic_positions(positions).unwrap();
        *dynamics =
            InertialBody::new(solid, &vec![1000.; count], vec![[0.; 3]; positions.len()]).unwrap();
    }
    #[test]
    fn assembled_skin_uses_each_joint_once_and_transfers_all_regional_displacements() {
        let mut demo = TissueDemo::body();
        let mut skin: Vec<_> = demo.bodies.iter().map(|b| b.positions()[1]).collect();
        skin.push([100., 100., 100.]);
        demo.assemble_regions().unwrap();
        let binding = demo.bind_skin(&skin).unwrap();
        assert_eq!(binding.bound_vertex_count(), 4);
        let layout = demo.assembled_regions.as_ref().unwrap().clone();
        let mut palette = vec![DMat4::IDENTITY; 8];
        palette[0] = DMat4::from_translation(DVec3::new(0.2, 0.1, -0.1));
        palette[3] = DMat4::from_rotation_z(0.2);
        palette[7] = DMat4::from_translation(DVec3::new(-0.1, 0.3, 0.2));
        let posed: Vec<_> = skin[..4]
            .iter()
            .zip(&demo.attachments)
            .map(|(p, (joint, _))| {
                palette[*joint]
                    .transform_point3(DVec3::from_array(*p))
                    .to_array()
            })
            .chain([skin[4]])
            .collect();
        let DemoTissue::Continuum { dynamics, .. } = &mut demo.bodies[0] else {
            panic!()
        };
        let mut nodes = dynamics.body().rest_positions().to_vec();
        for ((joint, _), range) in demo.attachments.iter().zip(&layout.node_ranges) {
            for n in range.clone() {
                nodes[n] = palette[*joint]
                    .transform_point3(DVec3::from_array(nodes[n]))
                    .to_array();
            }
        }
        held_pose(dynamics, &nodes);
        assert_eq!(demo.deform_skin(&binding, &palette, &posed).unwrap(), posed);
        for (i, range) in layout.node_ranges.iter().enumerate() {
            nodes[range.start + 1][2] += 0.001 * (i + 1) as f64;
        }
        let DemoTissue::Continuum { dynamics, .. } = &mut demo.bodies[0] else {
            panic!()
        };
        held_pose(dynamics, &nodes);
        let actual = demo.deform_skin(&binding, &palette, &posed).unwrap();
        for i in 0..4 {
            for a in 0..3 {
                let expected = posed[i][a] + if a == 2 { 0.001 * (i + 1) as f64 } else { 0. };
                assert!((actual[i][a] - expected).abs() < 1e-13);
            }
        }
        assert_eq!(actual[4].map(f64::to_bits), posed[4].map(f64::to_bits));
        palette[7] = DMat4::ZERO;
        assert!(demo.deform_skin(&binding, &palette, &posed).is_err());
        palette[7] = DMat4::IDENTITY;
        demo.attachments[3].0 = 6;
        assert!(demo.deform_skin(&binding, &palette, &posed).is_err());
    }
}

impl GlobalSkinBinding {
    fn posed_reference(&self, palette: &[DMat4]) -> Result<Vec<[f64; 3]>, &'static str> {
        let mut reference = self.rest.clone();
        for (range, joint) in self.node_ranges.iter().zip(&self.joints) {
            let matrix = palette.get(*joint).ok_or("missing skin attachment joint")?;
            let determinant = matrix.determinant();
            if !matrix.is_finite() || !determinant.is_finite() || determinant == 0. {
                return Err("invalid skin attachment matrix");
            }
            for node in range.clone() {
                reference[node] = matrix
                    .transform_point3(DVec3::from_array(self.rest[node]))
                    .to_array();
            }
        }
        Ok(reference)
    }
}

impl TissueDemo {
    /// Choose imported skin as the physical boundary; disable the native FEM
    /// envelope so the same body is not charged a second contact potential.
    pub(crate) fn bind_skin_contact(
        &mut self,
        binding: &TissueSkinBinding,
    ) -> Result<usize, &'static str> {
        use physics::biomechanics::{EmbeddedTriangleContact, StationaryEmbeddedContact};
        use std::collections::{BTreeMap, BTreeSet};
        if self.time != 0.
            || self.accumulator != 0.
            || self
                .body_step_counts()
                .iter()
                .any(|(accepted, _, _)| *accepted != 0)
        {
            return Err("skin contact authoring requires initial animation state");
        }
        let global = binding
            .global
            .as_ref()
            .ok_or("skin contact requires assembled binding")?;
        let layout = self
            .assembled_regions
            .as_ref()
            .ok_or("missing assembled tissue layout")?;
        let source = layout
            .source_contacts
            .first()
            .and_then(Option::as_ref)
            .ok_or("skin contact requires prescribed source geometry")?;
        if source.positions() != global.skin_reference || self.skin_contact_binding.is_some() {
            return Err("invalid skin contact reference or existing binding");
        }
        let faces: Vec<_> = source
            .faces()
            .iter()
            .copied()
            .filter(|f| f.iter().any(|&n| global.owners[n].is_some()))
            .collect();
        if faces.is_empty() {
            return Err("no tissue-owned skin contact triangles");
        }
        if std::env::var_os("VOXY_CONTACT_REJECTION_TRACE").is_some() {
            let owned: Vec<_> = global
                .owners
                .iter()
                .enumerate()
                .filter_map(|(vertex, owner)| owner.map(|region| (vertex, region)))
                .collect();
            eprintln!("PHYSICAL_SKIN_VERTEX_OWNERS {owned:?}");
        }
        let key = |p: [f64; 3]| p.map(|v| if v == 0. { 0 } else { v.to_bits() });
        let mut aliases = BTreeMap::<[u64; 3], Vec<usize>>::new();
        for (i, &p) in global.skin_reference.iter().enumerate() {
            aliases.entry(key(p)).or_default().push(i);
        }
        let mut groups = BTreeMap::<Vec<bool>, Vec<[usize; 3]>>::new();
        for &face in &faces {
            let owners: BTreeSet<_> = face.iter().filter_map(|&n| global.owners[n]).collect();
            let neighbours: BTreeSet<_> = face
                .iter()
                .flat_map(|&n| aliases[&key(global.skin_reference[n])].iter().copied())
                .collect();
            let enabled = source
                .faces()
                .iter()
                .enumerate()
                .map(|(i, other)| {
                    // Dynamic skin is not a prescribed obstacle copy. Its reciprocal
                    // self-contact needs a two-sided dynamic interaction separately.
                    !other
                        .iter()
                        .any(|&n| global.owners[n].is_some() || neighbours.contains(&n))
                        && owners.iter().all(|&region| {
                            layout.source_contacts[region]
                                .as_ref()
                                .is_some_and(|s| s.contact_faces()[i])
                        })
                })
                .collect();
            groups.entry(enabled).or_default().push(face);
        }
        let obstacle = Arc::new(
            source
                .with_contact_faces(vec![true; source.faces().len()])?
                .with_body_contact_domains(
                    groups
                        .into_iter()
                        .map(|(mask, faces)| (faces, mask))
                        .collect(),
                )?,
        );
        let contact = Arc::new(EmbeddedTriangleContact::from_embedding(
            global.embedding.clone(),
            faces.clone(),
        )?);
        let start = StationaryEmbeddedContact::new(
            contact,
            global.rest.clone(),
            global.skin_reference.clone(),
            obstacle,
        )?;
        let mut candidate = self
            .bodies
            .first()
            .ok_or("missing assembled tissue owner")?
            .clone();
        let DemoTissue::Continuum {
            dynamics,
            initial_energy_j,
            ..
        } = &mut candidate
        else {
            return Err("skin contact requires continuum body");
        };
        let native = dynamics
            .prescribed_surface()
            .ok_or("missing native contact integration owner")?
            .with_body_contact_domains(vec![(
                dynamics.body().surface(),
                vec![false; source.faces().len()],
            )])?;
        let work = dynamics.set_prescribed_surface(Some(Arc::new(native)))?
            + dynamics.set_stationary_embedded_contact(Some(start))?;
        *initial_energy_j += work;
        self.bodies[0] = candidate;
        self.skin_contact_binding = Some(global.clone());
        Ok(faces.len())
    }
}

#[cfg(test)]
mod contact_tests {
    use super::*;
    #[test]
    fn physical_render_skin_contact_drives_fem_and_books_obstacle_work_once() {
        let mut demo = TissueDemo::body();
        let DemoTissue::Continuum { dynamics, .. } = &demo.bodies[0] else {
            panic!()
        };
        let rest = dynamics.body().rest_positions();
        let blend = |a: f64, b: f64, c: f64, node: usize| {
            std::array::from_fn(|axis| a * rest[4][axis] + b * rest[0][axis] + c * rest[node][axis])
        };
        let skin = [
            blend(0.95, 0.05, 0., 1),
            blend(0.94, 0.04, 0.02, 1),
            blend(0.94, 0.04, 0.02, 2),
        ];
        let cx = skin[0][0];
        let y = skin[0][1] - 0.0015;
        let points: Vec<_> = skin
            .into_iter()
            .chain([[cx - 0.4, y, -0.4], [cx + 0.4, y, -0.4], [cx, y, 0.4]])
            .collect();
        let native = Arc::new(
            PrescribedTriangleSurface::new(
                points.clone(),
                vec![[0, 1, 2], [3, 4, 5]],
                0.0001,
                0.003,
                100.,
            )
            .unwrap()
            .with_contact_faces(vec![false, true])
            .unwrap()
            .with_body_contact_domains(vec![(dynamics.body().surface(), vec![false, false])])
            .unwrap(),
        );
        demo.bind_contact_surface(native.clone()).unwrap();
        demo.assemble_regions().unwrap();
        let binding = demo.bind_skin(&points).unwrap();
        assert_eq!(binding.bound_vertex_count(), 3);
        assert_eq!(demo.bind_skin_contact(&binding).unwrap(), 1);
        let palette = vec![DMat4::IDENTITY; 8];
        assert_eq!(
            demo.deform_skin(&binding, &palette, &points).unwrap(),
            points
        );
        let DemoTissue::Continuum { dynamics, .. } = &demo.bodies[0] else {
            panic!()
        };
        let initial = dynamics.diagnostics().unwrap();
        let response = dynamics
            .body()
            .stationary_embedded_contact()
            .unwrap()
            .response(dynamics.body().positions())
            .unwrap();
        assert!(response.potential_j > 0.);
        assert!(
            response
                .skin_loads
                .nodal_forces_n()
                .iter()
                .flatten()
                .any(|f| f.abs() > 1e-6)
        );
        assert_eq!(
            dynamics
                .prescribed_surface()
                .unwrap()
                .response(dynamics.body().positions(), &dynamics.body().surface())
                .unwrap()
                .potential_j,
            0.
        );
        let mut moved = points.clone();
        for p in &mut moved[3..] {
            p[1] -= 1e-5;
        }
        let next = Arc::new(native.with_positions(moved.clone()).unwrap());
        demo.step_body_with_contact64_workers(&palette, 0.5, Some(vec![next; 4]), 4)
            .unwrap();
        let DemoTissue::Continuum {
            dynamics, ledger, ..
        } = &demo.bodies[0]
        else {
            panic!()
        };
        let end = dynamics.diagnostics().unwrap();
        let independent = end.kinetic_j - initial.kinetic_j + end.potential_j - initial.potential_j
            + ledger.heat_j
            - ledger.support_work_j
            - ledger.surface_work_j;
        assert!(ledger.surface_work_j.abs() > 1e-9);
        assert!(independent.abs() < 1e-8, "independent={independent}");
        let visible = demo.deform_skin(&binding, &palette, &moved).unwrap();
        assert_ne!(visible[0], moved[0]);
        let surface_work_j = ledger.surface_work_j;
        let saved = format!("{demo:?}");
        let mut crossing = moved;
        for p in &mut crossing[3..] {
            p[1] = skin[0][1] + 0.01;
        }
        let crossing = Arc::new(native.with_positions(crossing).unwrap());
        assert!(
            demo.step_body_with_contact64_workers(&palette, 0.5, Some(vec![crossing; 4]), 1)
                .is_err()
        );
        assert_eq!(saved, format!("{demo:?}"));
        println!(
            "PHYSICAL_SKIN_CONTROLLER independent_defect_j={independent:.17e} surface_work_j={:.17e}",
            surface_work_j
        );
    }
    #[test]
    fn authored_convex_volume_uses_complete_physical_skin_and_existing_controller() {
        use physics::biomechanics::TetraMesh;
        let points = vec![
            [-0.01, 0.8, -0.01],
            [0.01, 0.8, -0.01],
            [0.01, 0.82, -0.01],
            [-0.01, 0.82, -0.01],
            [-0.01, 0.8, 0.01],
            [0.01, 0.8, 0.01],
            [0.01, 0.82, 0.01],
            [-0.01, 0.82, 0.01],
        ];
        let faces = vec![
            [0, 2, 1],
            [0, 3, 2],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ];
        let mesh =
            TetraMesh::from_convex_surface(points.clone(), faces.clone(), [0., 0.81, 0.]).unwrap();
        assert!(TissueDemo::body_from_regions(vec![(mesh.clone(), [0, 0, 1], 0)]).is_err());
        assert!(TissueDemo::body_from_regions(vec![(mesh.clone(), [0, 1, 99], 0)]).is_err());
        let mut malformed = mesh.clone();
        malformed.boundary.pop();
        assert!(TissueDemo::body_from_regions(vec![(malformed, [0, 1, 3], 0)]).is_err());
        let mut demo = TissueDemo::body_from_regions(vec![(mesh, [0, 1, 3], 0)]).unwrap();
        let mut model_points = points;
        model_points.extend([[-0.4, 0.7985, -0.4], [0.4, 0.7985, -0.4], [0., 0.7985, 0.4]]);
        let mut model_faces = faces;
        model_faces.push([8, 9, 10]);
        let mut domain = vec![false; model_faces.len()];
        *domain.last_mut().unwrap() = true;
        let source = Arc::new(
            PrescribedTriangleSurface::new(model_points.clone(), model_faces, 0.0001, 0.003, 100.)
                .unwrap()
                .with_contact_faces(domain)
                .unwrap(),
        );
        demo.bind_contact_surfaces(&[source.clone()]).unwrap();
        demo.assemble_regions().unwrap();
        let binding = demo.bind_skin(&model_points).unwrap();
        assert_eq!(binding.bound_vertex_count(), 8);
        assert_eq!(demo.bind_skin_contact(&binding).unwrap(), 12);
        let before = demo.body_energy_receipts().unwrap()[0];
        let mut moved = model_points.clone();
        for point in &mut moved[8..] {
            point[1] -= 1e-5;
        }
        let next = Arc::new(source.with_positions(moved.clone()).unwrap());
        demo.step_body_with_contact64_workers(&[DMat4::IDENTITY], 0.5, Some(vec![next]), 1)
            .unwrap();
        let after = demo.body_energy_receipts().unwrap()[0];
        let independent = (after[0] - before[0]) - (after[1] - before[1]) + (after[2] - before[2]);
        assert!(
            independent.abs() < 1e-5,
            "independent frame balance={independent:.17e}"
        );
        let visible = demo
            .deform_skin(&binding, &[DMat4::IDENTITY], &moved)
            .unwrap();
        assert_eq!(&visible[8..], &moved[8..]);
        assert!(visible[..8].iter().zip(&moved[..8]).any(|(a, b)| a != b));
        eprintln!(
            "AUTHORED_CONVEX_CONTROLLER bound_vertices=8 responsive_triangles=12 independent_frame_balance_j={independent:.17e}"
        );
    }
    #[test]
    fn authored_material_and_variable_supports_drive_shared_mechanical_thermal_owner() {
        use physics::biomechanics::TetraMesh;
        let mesh = TetraMesh::ellipsoid([0., 1., 0.], [0.02; 3], 0).unwrap();
        let mut spec = TissueRegionSpec::illustrative(mesh.clone(), vec![1, 2, 3, 4], 0);
        spec.density_kg_m3 = 1200.;
        spec.specific_heat_j_kg_k = 2000.;
        spec.temperature_kelvin = 299.;
        spec.ogden_terms[0].shear_pa = 15000.;
        spec.bulk_pa = 2e6;
        spec.maxwell_branches[0].shear_pa = 20000.;
        spec.maxwell_branches[0].relaxation_seconds = 0.5;
        let mut demo = TissueDemo::body_from_region_specs(vec![spec.clone()]).unwrap();
        assert_eq!(demo.attachments[0].1.len(), 4);
        let DemoTissue::Continuum { dynamics, .. } = &demo.bodies[0] else {
            panic!()
        };
        // Inscribed octahedron volume = 4/3 * radius^3, independently of mesh internals.
        let expected_mass = 1200. * 4. / 3. * 0.02_f64.powi(3);
        assert!((dynamics.masses().iter().sum::<f64>() - expected_mass).abs() < 1e-14);
        assert_eq!(
            dynamics.maxwell_temperatures_kelvin().unwrap(),
            vec![299.; mesh.cells.len()]
        );
        demo.assemble_regions().unwrap();
        let before = demo.body_energy_receipts().unwrap()[0];
        let palette = [DMat4::from_translation(DVec3::new(1e-6, 0., 0.))];
        demo.step_body_with_contact64_workers(&palette, 0.5, None, 1)
            .unwrap();
        let after = demo.body_energy_receipts().unwrap()[0];
        let independent = (after[0] - before[0]) - (after[1] - before[1]) + (after[2] - before[2]);
        assert!(independent.abs() < 1e-5);
        let DemoTissue::Continuum {
            dynamics, ledger, ..
        } = &demo.bodies[0]
        else {
            panic!()
        };
        for &node in &spec.supports {
            assert_eq!(
                dynamics.body().positions()[node],
                palette[0]
                    .transform_point3(DVec3::from_array(mesh.points[node]))
                    .to_array()
            );
        }
        assert!(
            (dynamics
                .maxwell_sensible_energy_j()
                .unwrap()
                .iter()
                .sum::<f64>()
                - ledger.heat_j)
                .abs()
                < 1e-12
        );
        assert!(ledger.heat_j > 0.);
        let temperature_energy: f64 = dynamics
            .maxwell_temperatures_kelvin()
            .unwrap()
            .iter()
            .map(|&temperature| {
                expected_mass / mesh.cells.len() as f64 * 2000. * (temperature - 299.)
            })
            .sum();
        let kelvin_rounding_j = 4. * f64::EPSILON * 299. * expected_mass * 2000.;
        assert!(
            ledger.heat_j > 100. * kelvin_rounding_j,
            "insufficient heat to discriminate authored capacity"
        );
        assert!((temperature_energy - ledger.heat_j).abs() <= kelvin_rounding_j);
        eprintln!(
            "AUTHORED_HEAT released_j={:.17e} from_temperatures_j={temperature_energy:.17e} rounding_j={kelvin_rounding_j:.17e}",
            ledger.heat_j
        );
        eprintln!(
            "AUTHORED_MATERIAL mass_kg={expected_mass:.17e} supports=4 independent_frame_balance_j={independent:.17e} cells={}",
            mesh.cells.len()
        );
        for variant in 0..7 {
            let mut invalid = spec.clone();
            match variant {
                0 => invalid.density_kg_m3 = 0.,
                1 => invalid.specific_heat_j_kg_k = 0.,
                2 => invalid.temperature_kelvin = 0.,
                3 => invalid.bulk_pa = -1.,
                4 => invalid.ogden_terms[0].exponent = 0.,
                5 => invalid.maxwell_branches[0].relaxation_seconds = 0.,
                _ => invalid.supports = vec![1, 1],
            }
            assert!(TissueDemo::body_from_region_specs(vec![invalid]).is_err());
        }
    }
    #[test]
    fn nonradial_volume_boundary_normals_ignore_internal_faces_and_cell_order() {
        use physics::biomechanics::TetraMesh;
        let points = vec![
            [0., 0., 0.],
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [0., 0., -1.],
        ];
        let cells = vec![[0, 1, 2, 3], [0, 2, 1, 4]];
        let body = Body::new(
            points.clone(),
            vec![false; 5],
            cells
                .iter()
                .map(|&c| {
                    (
                        c,
                        Material {
                            shear_pa: 100.,
                            bulk_pa: 1000.,
                            fibers: vec![],
                        },
                    )
                })
                .collect(),
        )
        .unwrap();
        let boundary = body.surface();
        assert_eq!(boundary.len(), 6);
        let mesh = TetraMesh {
            points: points.clone(),
            cells: cells.clone(),
            boundary: boundary.clone(),
        };
        let mut demo = TissueDemo::body_from_regions(vec![(mesh.clone(), [0, 1, 2], 0)]).unwrap();
        let normals = demo.bodies[0].smooth_boundary_normals().unwrap();
        let diagonal = -std::f64::consts::FRAC_1_SQRT_2;
        assert!((normals[0][0] - diagonal).abs() < 1e-14);
        assert!((normals[0][1] - diagonal).abs() < 1e-14);
        assert_eq!(normals[0][2], 0.);
        assert_eq!(demo.tissue_mesh().unwrap().vertices().len(), 6 * 16 * 3);
        let alternate_cells = vec![[1, 2, 0, 3], [2, 1, 0, 4]];
        let alternate = TissueDemo::body_from_regions(vec![(
            TetraMesh {
                points,
                cells: alternate_cells,
                boundary,
            },
            [0, 1, 2],
            0,
        )])
        .unwrap();
        let alternate_normals = alternate.bodies[0].smooth_boundary_normals().unwrap();
        for (a, b) in normals
            .iter()
            .flatten()
            .zip(alternate_normals.iter().flatten())
        {
            assert!((a - b).abs() < 1e-14);
        }
        let refined = mesh.refined_once().unwrap().refined_once().unwrap();
        let interior = refined
            .points
            .iter()
            .position(|p| p.iter().all(|&v| (v - 0.25).abs() < 1e-14))
            .unwrap();
        let refined_faces = refined.boundary.len();
        let refined_demo = TissueDemo::body_from_regions(vec![(refined, [0, 1, 2], 0)]).unwrap();
        assert_eq!(
            refined_demo.bodies[0].smooth_boundary_normals().unwrap()[interior],
            [0.; 3]
        );
        assert_eq!(
            refined_demo.tissue_mesh().unwrap().vertices().len(),
            refined_faces * 16 * 3
        );
        demo.assemble_regions().unwrap();
        assert_eq!(demo.bodies[0].smooth_boundary_normals().unwrap(), normals);
        assert_eq!(demo.tissue_mesh().unwrap().vertices().len(), 6 * 16 * 3);
    }
}
