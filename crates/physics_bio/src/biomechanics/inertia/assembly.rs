//! Assemble existing regional state without resetting inertia or material memory.
use super::{Body, InertialBody, thermal::CellThermalState};
use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct InertialAssembly {
    pub body: InertialBody,
    pub node_ranges: Vec<Range<usize>>,
    pub cell_ranges: Vec<Range<usize>>,
}

impl InertialBody {
    /// Assemble disjoint regional node spaces using the existing Body assembly.
    /// Preserves positions, support velocities, exact nodal/cell masses, material
    /// memories and complete compensated thermal inventory. Stable ranges map
    /// authored regional node/cell IDs into the resulting global owner.
    /// Regional masks on a common obstacle geometry are remapped to global body
    /// triangle identities. Source obstacle-face indices are retained. Distinct
    /// regional domains create a new global contact owner; unchanged common
    /// domains retain their original owner. Sources stay unchanged.
    /// Install global embedded skin contact after assembly, rather than losing
    /// or freezing another dynamic region's force transfer.
    /// # Errors
    /// Empty input, incompatible acceleration/plane/surface/thermal ownership,
    /// embedded contacts needing rebinding, or Body assembly/evaluation failure.
    pub fn assemble_tissues(parts: &[Self]) -> Result<InertialAssembly, &'static str> {
        let first = parts.first().ok_or("empty inertial tissue assembly")?;
        for part in parts {
            match (&part.search_backend, &first.search_backend) {
                (None, None) => {}
                (Some(a), Some(b)) if Arc::ptr_eq(a, b) => {}
                _ => return Err("mixed inertial tissue search backend ownership"),
            }
            if part.acceleration != first.acceleration || part.plane != first.plane {
                return Err("mixed inertial tissue acceleration or plane");
            }
            if part.thermal.is_some() != first.thermal.is_some() {
                return Err("mixed inertial tissue thermal ownership");
            }
            match (&part.prescribed_surface, &first.prescribed_surface) {
                (None, None) => {}
                (Some(a), Some(b)) => {
                    a.same_geometry_owner(b)?;
                    if a.positions() != b.positions() {
                        return Err("mixed inertial tissue surface poses");
                    }
                }
                _ => return Err("mixed inertial tissue surface ownership"),
            }
            part.diagnostics()?;
        }
        let assembly =
            Body::assemble_tissues(&parts.iter().map(|p| p.body.clone()).collect::<Vec<_>>())?;
        let prescribed_surface = if let Some(source) = &first.prescribed_surface {
            let remap = parts.iter().any(|p| {
                let surface = p.prescribed_surface.as_ref().unwrap();
                surface.has_body_contact_domains() || surface.same_owner(source).is_err()
            });
            if remap {
                let mut domains = Vec::new();
                for (part, range) in parts.iter().zip(&assembly.node_ranges) {
                    let surface = part.prescribed_surface.as_ref().unwrap();
                    let mut groups = BTreeMap::<Vec<bool>, Vec<[usize; 3]>>::new();
                    for face in part.body.surface() {
                        let row = surface.body_contact_faces(face);
                        let enabled = surface
                            .contact_faces()
                            .iter()
                            .enumerate()
                            .map(|(i, &active)| active && row.is_none_or(|mask| mask[i]))
                            .collect::<Vec<_>>();
                        groups
                            .entry(enabled)
                            .or_default()
                            .push(face.map(|i| i + range.start));
                    }
                    domains.extend(groups.into_iter().map(|(mask, faces)| (faces, mask)));
                }
                Some(Arc::new(
                    source
                        .with_contact_faces(vec![true; source.faces().len()])?
                        .with_body_contact_domains(domains)?,
                ))
            } else {
                Some(source.clone())
            }
        } else {
            None
        };
        let thermal = if first.thermal.is_some() {
            Some(CellThermalState::assemble(
                &parts
                    .iter()
                    .filter_map(|p| p.thermal.as_ref())
                    .collect::<Vec<_>>(),
            ))
        } else {
            None
        };
        let body = Self {
            body: assembly.body,
            masses: parts
                .iter()
                .flat_map(|p| p.masses.iter().copied())
                .collect(),
            cell_masses: parts
                .iter()
                .flat_map(|p| p.cell_masses.iter().copied())
                .collect(),
            velocities: parts
                .iter()
                .flat_map(|p| p.velocities.iter().copied())
                .collect(),
            thermal,
            acceleration: first.acceleration,
            search_backend: first.search_backend.clone(),
            plane: first.plane,
            prescribed_surface,
        };
        body.diagnostics()?;
        Ok(InertialAssembly {
            body,
            node_ranges: assembly.node_ranges,
            cell_ranges: assembly.cell_ranges,
        })
    }
}
