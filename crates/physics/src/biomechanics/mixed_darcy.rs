//! RT0–P0 Darcy response on general tetrahedra, SI units.
//! Positive-definite spatial permeability tensors; exact degree-two flux mass integration.
use super::{Matrix, Vec3, add, columns, cross, det, dot, mv, scale, sub};
use std::collections::BTreeMap;
#[derive(Clone, Debug)]
pub struct DarcyFace {
    pub nodes: [usize; 3],
    pub owner: usize,
    pub neighbor: Option<usize>,
    pub boundary_pressure_pa: Option<f64>,
}
#[derive(Clone, Debug)]
pub struct MixedDarcy {
    faces: Vec<DarcyFace>,
    cells: Vec<[usize; 4]>,
    points: Vec<Vec3>,
    volumes: Vec<f64>,
    local_faces: Vec<[Option<(usize, f64)>; 4]>,
    local_mass: Vec<[[f64; 4]; 4]>,
    diagonal_scale: Vec<f64>,
    reservoir_rank: Option<(f64, Vec<usize>)>,
}
#[derive(Clone, Debug)]
pub struct DarcyResponse {
    /// Signed owner→neighbor or owner→boundary volumetric flux, m³/s.
    pub face_flows_m3_per_s: Vec<f64>,
    pub cell_outflows_m3_per_s: Vec<f64>,
    pub cell_centroid_velocities_m_per_s: Vec<Vec3>,
    pub dissipation_w: f64,
    pub pressure_work_w: f64,
    pub residual_pa: f64,
    pub solver_iterations: usize,
}
/// Exterior protein exchange coefficients for one active membrane port.
#[derive(Clone, Copy, Debug)]
pub struct ProteinMembrane {
    pub concentration_kg_per_m3: f64,
    /// Fraction excluded from donor-advection flux, between zero and one.
    pub reflection: f64,
    /// Permeability times area, m³/s (not permeability in m/s).
    pub diffusive_conductance_m3_per_s: f64,
}
/// Persistent finite fluid/protein compartment; pressure follows linear compliance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PoreReservoir {
    pub reference_volume_m3: f64,
    pub reference_pressure_pa: f64,
    pub compliance_m3_per_pa: f64,
    pub fluid_volume_m3: f64,
    pub protein_kg: f64,
}
impl PoreReservoir {
    /// # Errors
    /// Rejects invalid absolute inventories/compliance or pressure overflow.
    pub fn pressure_pa(&self) -> Result<f64, &'static str> {
        if !self.reference_volume_m3.is_finite()
            || self.reference_volume_m3 <= 0.
            || !self.reference_pressure_pa.is_finite()
            || !self.compliance_m3_per_pa.is_finite()
            || self.compliance_m3_per_pa <= 0.
            || !self.fluid_volume_m3.is_finite()
            || self.fluid_volume_m3 <= 0.
            || !self.protein_kg.is_finite()
            || self.protein_kg < 0.
        {
            return Err("invalid pore reservoir inventory");
        }
        let p = self.reference_pressure_pa
            + (self.fluid_volume_m3 - self.reference_volume_m3) / self.compliance_m3_per_pa;
        if !p.is_finite() {
            return Err("pore reservoir pressure overflow");
        }
        Ok(p)
    }
}
fn factor(a: &[Vec<f64>]) -> Result<(Vec<Vec<f64>>, Vec<f64>), &'static str> {
    let n = a.len();
    let mut d = Vec::with_capacity(n);
    for (i, row) in a.iter().enumerate() {
        if row.len() != n || !row[i].is_finite() || row[i] <= 0. {
            return Err("invalid Darcy SPD matrix");
        }
        d.push(row[i].sqrt());
    }
    let mut l = vec![vec![0.; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let mut value = a[i][j] / d[i] / d[j];
            value -= l[i][..j]
                .iter()
                .zip(&l[j][..j])
                .map(|(a, b)| a * b)
                .sum::<f64>();
            if !value.is_finite() || (i == j && value <= 0.) {
                return Err("Darcy matrix not positive definite");
            }
            l[i][j] = if i == j {
                value.sqrt()
            } else {
                value / l[j][j]
            };
        }
    }
    Ok((l, d))
}
fn solve(l: &[Vec<f64>], d: &[f64], b: &[f64]) -> Result<Vec<f64>, &'static str> {
    let n = b.len();
    let mut x = vec![0.; n];
    for i in 0..n {
        x[i] = b[i] / d[i];
        for j in 0..i {
            x[i] -= l[i][j] * x[j];
        }
        x[i] /= l[i][i];
    }
    for i in (0..n).rev() {
        for j in i + 1..n {
            x[i] -= l[j][i] * x[j];
        }
        x[i] /= l[i][i];
    }
    for (v, scale) in x.iter_mut().zip(d) {
        *v /= scale;
    }
    if x.iter().any(|v| !v.is_finite()) {
        return Err("Darcy solve overflow");
    }
    Ok(x)
}
fn permeability_inverse(k: Matrix) -> Result<Matrix, &'static str> {
    if k.iter().flatten().any(|v| !v.is_finite()) {
        return Err("nonfinite permeability");
    }
    let size = k.iter().flatten().map(|v| v.abs()).fold(0_f64, f64::max);
    for (i, row) in k.iter().enumerate() {
        for (j, value) in row.iter().enumerate() {
            if (*value - k[j][i]).abs() > 1e-12 * size {
                return Err("permeability must be symmetric");
            }
        }
    }
    let a = k.map(|r| r.to_vec()).to_vec();
    let (l, d) = factor(&a)?;
    let mut inverse = [[0.; 3]; 3];
    for j in 0..3 {
        let mut rhs = [0.; 3];
        rhs[j] = 1.;
        let column = solve(&l, &d, &rhs)?;
        for i in 0..3 {
            inverse[i][j] = column[i];
        }
    }
    Ok(inverse)
}
impl MixedDarcy {
    /// Joint persistent backward-Euler tissue/reservoir pressure and protein step.
    /// All supplied inventories and pressures commit together after validation.
    /// Fixed mesh only; cell storage and fluid inventories must describe that mesh.
    /// # Errors
    /// Failure in either solve, depleted fluid or invalid state leaves all inputs unchanged.
    #[allow(clippy::too_many_arguments)]
    pub fn implicit_reservoir_transport_step(
        &self,
        pressures: &mut [f64],
        fluid_volumes: &mut [f64],
        protein_kg: &mut [f64],
        storage: &[f64],
        reservoir: &mut PoreReservoir,
        seconds: f64,
    ) -> Result<DarcyResponse, &'static str> {
        if fluid_volumes.len() != self.cells.len()
            || fluid_volumes.iter().any(|v| !v.is_finite() || *v <= 0.)
        {
            return Err("invalid tissue reservoir fluid inventories");
        }
        let initial_volume = fluid_volumes.iter().sum::<f64>() + reservoir.fluid_volume_m3;
        if !initial_volume.is_finite() {
            return Err("total pore inventory overflow");
        }
        let initial_protein = protein_kg.iter().sum::<f64>() + reservoir.protein_kg;
        if !initial_protein.is_finite() {
            return Err("total pore protein overflow");
        }
        let old_pressure = reservoir.pressure_pa()?;
        let (next_pressure, next_reservoir_pressure, response) = self.implicit_reservoir_step(
            pressures,
            storage,
            seconds,
            old_pressure,
            reservoir.compliance_m3_per_pa,
        )?;
        let next_volumes: Vec<_> = fluid_volumes
            .iter()
            .zip(pressures.iter())
            .zip(&next_pressure)
            .zip(storage)
            .map(|(((v, old), new), s)| v + s * (new - old))
            .collect();
        let mut next_reservoir = *reservoir;
        next_reservoir.fluid_volume_m3 +=
            reservoir.compliance_m3_per_pa * (next_reservoir_pressure - old_pressure);
        next_reservoir.pressure_pa()?;
        let (next_protein, reservoir_protein) = self.implicit_protein_reservoir_step(
            protein_kg,
            &next_volumes,
            &response.face_flows_m3_per_s,
            seconds,
            reservoir.protein_kg,
            next_reservoir.fluid_volume_m3,
        )?;
        next_reservoir.protein_kg = reservoir_protein;
        let final_volume = next_volumes.iter().sum::<f64>() + next_reservoir.fluid_volume_m3;
        if !final_volume.is_finite()
            || (final_volume - initial_volume).abs() > 1e-10 * initial_volume
        {
            return Err("nonconservative joint pore reservoir volume");
        }
        let final_protein = next_protein.iter().sum::<f64>() + next_reservoir.protein_kg;
        if !final_protein.is_finite()
            || (final_protein - initial_protein).abs() > 1e-12 * initial_protein
        {
            return Err("nonconservative joint pore reservoir protein");
        }
        pressures.copy_from_slice(&next_pressure);
        fluid_volumes.copy_from_slice(&next_volumes);
        protein_kg.copy_from_slice(&next_protein);
        *reservoir = next_reservoir;
        Ok(response)
    }
    /// Conservative backward-Euler upwind protein transport on sealed faces.
    /// Fluxes are m³/s; donor concentrations use the new fluid volumes.
    /// Does not mutate input inventories or model state.
    /// # Errors
    /// Rejects invalid inventories/fluxes, overflow and unconverged transport.
    pub fn implicit_protein_step(
        &self,
        old_mass: &[f64],
        new_volumes: &[f64],
        face_flows: &[f64],
        seconds: f64,
    ) -> Result<Vec<f64>, &'static str> {
        if self.faces.iter().any(|f| f.neighbor.is_none()) {
            return Err("sealed protein transport requires sealed faces");
        }
        self.implicit_protein_step_with_boundary(
            old_mass,
            new_volumes,
            face_flows,
            seconds,
            &vec![None; self.faces.len()],
        )
    }
    /// Implicit upwind exchange with external reservoir concentrations in kg/m³.
    /// Supply one optional concentration per face; every external inflow needs
    /// an explicit concentration. Outflow uses the accepted tissue concentration.
    /// # Errors
    /// Rejects invalid/missing boundary concentrations and inaccurate transport.
    pub fn implicit_protein_step_with_boundary(
        &self,
        old_mass: &[f64],
        new_volumes: &[f64],
        face_flows: &[f64],
        seconds: f64,
        boundary_concentrations: &[Option<f64>],
    ) -> Result<Vec<f64>, &'static str> {
        let membranes: Vec<_> = boundary_concentrations
            .iter()
            .map(|c| {
                c.map(|c| ProteinMembrane {
                    concentration_kg_per_m3: c,
                    reflection: 0.,
                    diffusive_conductance_m3_per_s: 0.,
                })
            })
            .collect();
        self.implicit_protein_step_with_membranes(
            old_mass,
            new_volumes,
            face_flows,
            seconds,
            &membranes,
        )
    }
    /// Positive backward-Euler protein transfer with selective exterior membranes.
    /// Exterior flux is (1-sigma)*q*C_donor + D*(C_tissue-C_exterior).
    /// This uses upwind donor concentration, not a fitted membrane pore-average
    /// concentration law. Exterior reservoirs are prescribed and caller-owned.
    /// # Errors
    /// Rejects invalid coefficients, missing inflow state, overflow or failed solve.
    pub fn implicit_protein_step_with_membranes(
        &self,
        old_mass: &[f64],
        new_volumes: &[f64],
        face_flows: &[f64],
        seconds: f64,
        membranes: &[Option<ProteinMembrane>],
    ) -> Result<Vec<f64>, &'static str> {
        if old_mass.len() != self.cells.len()
            || new_volumes.len() != self.cells.len()
            || face_flows.len() != self.faces.len()
            || old_mass.iter().any(|m| !m.is_finite() || *m < 0.)
            || new_volumes.iter().any(|v| !v.is_finite() || *v <= 0.)
            || face_flows.iter().any(|q| !q.is_finite())
            || !seconds.is_finite()
            || seconds <= 0.
            || membranes.len() != self.faces.len()
            || membranes.iter().flatten().any(|m| {
                !m.concentration_kg_per_m3.is_finite()
                    || m.concentration_kg_per_m3 < 0.
                    || !m.reflection.is_finite()
                    || !(0. ..=1.).contains(&m.reflection)
                    || !m.diffusive_conductance_m3_per_s.is_finite()
                    || m.diffusive_conductance_m3_per_s < 0.
            })
        {
            return Err("invalid implicit protein state");
        }
        let mut diagonal = vec![1.; self.cells.len()];
        let mut incoming = vec![Vec::new(); self.cells.len()];
        let mut rhs_mass = old_mass.to_vec();
        let mut external_out = vec![0.; self.cells.len()];
        for ((face, q), membrane) in self.faces.iter().zip(face_flows).zip(membranes) {
            let Some(neighbor) = face.neighbor else {
                let transmission = membrane.map_or(1., |m| 1. - m.reflection);
                let diffusion = membrane.map_or(0., |m| m.diffusive_conductance_m3_per_s);
                let exterior = membrane.map(|m| m.concentration_kg_per_m3);
                if *q < 0. {
                    let c = exterior.ok_or("missing external protein inflow concentration")?;
                    rhs_mass[face.owner] += -seconds * q * transmission * c;
                }
                rhs_mass[face.owner] += seconds * diffusion * exterior.unwrap_or(0.);
                let coefficient =
                    seconds * (q.max(0.) * transmission + diffusion) / new_volumes[face.owner];
                diagonal[face.owner] += coefficient;
                external_out[face.owner] += coefficient;
                if !rhs_mass[face.owner].is_finite() || !diagonal[face.owner].is_finite() {
                    return Err("external protein exchange overflow");
                }
                continue;
            };
            if membrane.is_some() {
                return Err("internal face cannot prescribe external protein");
            }
            let (donor, receiver) = if *q >= 0. {
                (face.owner, neighbor)
            } else {
                (neighbor, face.owner)
            };
            let coefficient = seconds * q.abs() / new_volumes[donor];
            diagonal[donor] += coefficient;
            if !coefficient.is_finite() || !diagonal[donor].is_finite() {
                return Err("implicit protein coefficient overflow");
            }
            incoming[receiver].push((donor, coefficient));
        }
        solve_protein(&rhs_mass, &diagonal, &incoming, &external_out)
    }
    /// Conservative protein transport with one finite, mixed external compartment.
    /// All listed external faces exchange with the same reservoir inventory.
    /// New fluid volumes must correspond to the supplied accepted face fluxes.
    /// # Errors
    /// Rejects invalid inputs, absent ports, overflow or unconverged transport.
    pub fn implicit_protein_reservoir_step(
        &self,
        old_mass: &[f64],
        new_volumes: &[f64],
        face_flows: &[f64],
        seconds: f64,
        old_reservoir_mass_kg: f64,
        new_reservoir_volume_m3: f64,
    ) -> Result<(Vec<f64>, f64), &'static str> {
        let n = self.cells.len();
        if old_mass.len() != n
            || new_volumes.len() != n
            || face_flows.len() != self.faces.len()
            || old_mass.iter().any(|m| !m.is_finite() || *m < 0.)
            || new_volumes.iter().any(|v| !v.is_finite() || *v <= 0.)
            || face_flows.iter().any(|q| !q.is_finite())
            || !seconds.is_finite()
            || seconds <= 0.
            || !old_reservoir_mass_kg.is_finite()
            || old_reservoir_mass_kg < 0.
            || !new_reservoir_volume_m3.is_finite()
            || new_reservoir_volume_m3 <= 0.
            || !self.faces.iter().any(|f| f.neighbor.is_none())
        {
            return Err("invalid finite reservoir protein state");
        }
        let mut rhs = old_mass.to_vec();
        rhs.push(old_reservoir_mass_kg);
        let mut volumes = new_volumes.to_vec();
        volumes.push(new_reservoir_volume_m3);
        let mut diagonal = vec![1.; n + 1];
        let mut incoming = vec![Vec::new(); n + 1];
        for (face, q) in self.faces.iter().zip(face_flows) {
            let neighbor = face.neighbor.unwrap_or(n);
            let (donor, receiver) = if *q >= 0. {
                (face.owner, neighbor)
            } else {
                (neighbor, face.owner)
            };
            let coefficient = seconds * q.abs() / volumes[donor];
            diagonal[donor] += coefficient;
            if !coefficient.is_finite() || !diagonal[donor].is_finite() {
                return Err("finite reservoir protein coefficient overflow");
            }
            incoming[receiver].push((donor, coefficient));
        }
        let mut mass = solve_protein(&rhs, &diagonal, &incoming, &vec![0.; n + 1])?;
        let reservoir_mass = mass.pop().ok_or("missing finite reservoir protein")?;
        Ok((mass, reservoir_mass))
    }
    /// Backward-Euler pressure diffusion on a fixed mesh.
    /// Prescribed boundary-face pressures act as external fluid reservoirs;
    /// unspecified external faces remain sealed.
    /// `storage` is cell fluid-volume compliance in m³/Pa. The returned flux
    /// is evaluated at the returned new pressures, not at the old state.
    /// # Errors
    /// Rejects invalid storage/time and inaccurate solves.
    pub fn implicit_storage_step(
        &self,
        pressures: &[f64],
        storage: &[f64],
        seconds: f64,
    ) -> Result<(Vec<f64>, DarcyResponse), &'static str> {
        self.storage_step(pressures, storage, seconds, None)
            .map(|(p, _, r)| (p, r))
    }
    /// Couple all listed external ports to one finite linear-compliance reservoir.
    /// Returns tissue pressures, accepted reservoir pressure and physical fluxes.
    /// # Errors
    /// Rejects invalid reservoir compliance/pressure, absent ports or failed solve.
    pub fn implicit_reservoir_step(
        &self,
        pressures: &[f64],
        storage: &[f64],
        seconds: f64,
        reservoir_pressure_pa: f64,
        reservoir_compliance_m3_per_pa: f64,
    ) -> Result<(Vec<f64>, f64, DarcyResponse), &'static str> {
        if !reservoir_pressure_pa.is_finite()
            || !reservoir_compliance_m3_per_pa.is_finite()
            || reservoir_compliance_m3_per_pa <= 0.
            || !self.faces.iter().any(|f| f.neighbor.is_none())
        {
            return Err("invalid finite Darcy reservoir");
        }
        self.storage_step(
            pressures,
            storage,
            seconds,
            Some((reservoir_pressure_pa, reservoir_compliance_m3_per_pa)),
        )
    }
    fn storage_step(
        &self,
        pressures: &[f64],
        storage: &[f64],
        seconds: f64,
        reservoir: Option<(f64, f64)>,
    ) -> Result<(Vec<f64>, f64, DarcyResponse), &'static str> {
        if storage.len() != self.cells.len()
            || storage.iter().any(|s| !s.is_finite() || *s <= 0.)
            || !seconds.is_finite()
            || seconds <= 0.
        {
            return Err("invalid implicit Darcy storage step");
        }
        // Eliminate p_new = p_old - dt S^-1 B q. Solve the SPD face system
        // (M + dt B^T S^-1 B) q = B^T p_old with local cell blocks.
        let mut augmented = self.clone();
        let mut diagonal = vec![0.; self.faces.len()];
        for (cell, faces) in self.local_faces.iter().enumerate() {
            let coefficient = seconds / storage[cell];
            for (i, a) in faces.iter().enumerate() {
                if let Some((fi, si)) = a {
                    for (j, b) in faces.iter().enumerate() {
                        if let Some((_, sj)) = b {
                            augmented.local_mass[cell][i][j] += coefficient * si * sj;
                            if !augmented.local_mass[cell][i][j].is_finite() {
                                return Err("implicit Darcy resistance overflow");
                            }
                        }
                    }
                    diagonal[*fi] += augmented.local_mass[cell][i][i];
                }
            }
        }
        if diagonal.iter().any(|d| !d.is_finite() || *d <= 0.) {
            return Err("invalid implicit Darcy diagonal");
        }
        if let Some((pressure, compliance)) = reservoir {
            let coefficient = seconds / compliance;
            let mut ports = Vec::new();
            for (i, face) in augmented.faces.iter_mut().enumerate() {
                if face.neighbor.is_none() {
                    face.boundary_pressure_pa = Some(pressure);
                    diagonal[i] += coefficient;
                    ports.push(i);
                }
            }
            if !coefficient.is_finite() || diagonal.iter().any(|d| !d.is_finite()) {
                return Err("finite reservoir stiffness overflow");
            }
            augmented.reservoir_rank = Some((coefficient, ports));
        }
        augmented.diagonal_scale = diagonal.into_iter().map(f64::sqrt).collect();
        let mut response = augmented.response(pressures)?;
        let next: Vec<_> = pressures
            .iter()
            .zip(&response.cell_outflows_m3_per_s)
            .zip(storage)
            .map(|((p, q), s)| p - seconds * q / s)
            .collect();
        if next.iter().any(|p| !p.is_finite()) {
            return Err("implicit Darcy pressure overflow");
        }
        let next_reservoir = reservoir.map_or(0., |(p, c)| {
            p + seconds / c
                * self
                    .faces
                    .iter()
                    .zip(&response.face_flows_m3_per_s)
                    .filter(|(f, _)| f.neighbor.is_none())
                    .map(|(_, q)| q)
                    .sum::<f64>()
        });
        let applied = self.apply_mass(&response.face_flows_m3_per_s);
        let mut residual = 0_f64;
        let mut drive_scale = 0_f64;
        let mut work = 0.;
        let mut dissipation = 0.;
        for ((face, q), mq) in self
            .faces
            .iter()
            .zip(&response.face_flows_m3_per_s)
            .zip(applied)
        {
            let exterior = face.neighbor.map_or_else(
                || {
                    if reservoir.is_some() {
                        next_reservoir
                    } else {
                        face.boundary_pressure_pa.unwrap_or(0.)
                    }
                },
                |i| next[i],
            );
            let drive = next[face.owner] - exterior;
            residual = residual.max((mq - drive).abs());
            drive_scale = drive_scale.max(drive.abs());
            work += q * drive;
            dissipation += q * mq;
        }
        if !work.is_finite()
            || !dissipation.is_finite()
            || dissipation < 0.
            || work < 0.
            || residual
                > 1e-8 * drive_scale
                    + 64.
                        * f64::EPSILON
                        * pressures
                            .iter()
                            .chain(&next)
                            .map(|p| p.abs())
                            .fold(0., f64::max)
        {
            return Err("inaccurate implicit Darcy physical flux");
        }
        response.residual_pa = residual;
        response.pressure_work_w = work;
        response.dissipation_w = dissipation;
        Ok((next, next_reservoir, response))
    }
    /// Unlisted external faces have zero normal flux. Listed faces prescribe
    /// face-average pressure; internal faces share one globally oriented flux.
    /// # Errors
    /// Rejects invalid/degenerate/nonmanifold topology, non-SPD permeability,
    /// duplicate/internal boundary assignments, overflow or resource limits.
    #[allow(clippy::too_many_lines)]
    pub fn new(
        points: Vec<Vec3>,
        cells: Vec<[usize; 4]>,
        permeabilities: &[Matrix],
        viscosity_pa_s: f64,
        boundaries: &[([usize; 3], f64)],
    ) -> Result<Self, &'static str> {
        if cells.is_empty()
            || cells.len() > 250_000
            || permeabilities.len() != cells.len()
            || points.iter().flatten().any(|v| !v.is_finite())
            || !viscosity_pa_s.is_finite()
            || viscosity_pa_s <= 0.
        {
            return Err("invalid mixed Darcy input");
        }
        let mut map: BTreeMap<[usize; 3], Vec<(usize, usize)>> = BTreeMap::new();
        let mut volumes = Vec::new();
        for (index, nodes) in cells.iter().enumerate() {
            if nodes.iter().any(|i| *i >= points.len()) {
                return Err("invalid Darcy tetrahedron index");
            }
            let [a, b, c, d] = nodes.map(|i| points[i]);
            let volume = det(columns(sub(b, a), sub(c, a), sub(d, a))).abs() / 6.;
            if !volume.is_finite() || volume <= 1e-15 {
                return Err("degenerate Darcy tetrahedron");
            }
            volumes.push(volume);
            for opposite in 0..4 {
                let mut face = [0; 3];
                let mut j = 0;
                for (k, node) in nodes.iter().enumerate() {
                    if k != opposite {
                        face[j] = *node;
                        j += 1;
                    }
                }
                face.sort_unstable();
                map.entry(face).or_default().push((index, opposite));
            }
        }
        let mut boundary_map = BTreeMap::new();
        for &(mut face, p) in boundaries {
            face.sort_unstable();
            if !p.is_finite() || boundary_map.insert(face, p).is_some() {
                return Err("invalid Darcy boundary");
            }
            if map.get(&face).is_none_or(|adj| adj.len() != 1) {
                return Err("pressure boundary must be external");
            }
        }
        let mut faces = Vec::new();
        let mut local_faces = vec![[None; 4]; cells.len()];
        for (nodes, adj) in map {
            if adj.len() > 2 {
                return Err("nonmanifold Darcy face");
            }
            if adj.len() == 2 {
                let origin = points[nodes[0]];
                let normal = cross(sub(points[nodes[1]], origin), sub(points[nodes[2]], origin));
                let side = adj
                    .iter()
                    .map(|&(cell, opposite)| {
                        dot(normal, sub(points[cells[cell][opposite]], origin))
                    })
                    .collect::<Vec<_>>();
                if side[0] * side[1] >= 0. {
                    return Err("overlapping Darcy cells");
                }
            }
            if adj.len() == 1 && !boundary_map.contains_key(&nodes) {
                continue;
            }
            let index = faces.len();
            local_faces[adj[0].0][adj[0].1] = Some((index, 1.));
            if adj.len() == 2 {
                local_faces[adj[1].0][adj[1].1] = Some((index, -1.));
            }
            faces.push(DarcyFace {
                nodes,
                owner: adj[0].0,
                neighbor: adj.get(1).map(|a| a.0),
                boundary_pressure_pa: boundary_map.get(&nodes).copied(),
            });
        }
        if faces.len() > 1_000_000 {
            return Err("mixed Darcy face limit");
        }
        let mut local_mass = vec![[[0.; 4]; 4]; cells.len()];
        let mut diagonal = vec![0.; faces.len()];
        for (cell, nodes) in cells.iter().enumerate() {
            let inverse = permeability_inverse(permeabilities[cell])?;
            let origin = points[nodes[0]];
            let x = nodes.map(|i| sub(points[i], origin));
            let center = scale(x.into_iter().fold([0.; 3], add), 0.25);
            let centered = x.map(|v| sub(v, center));
            let covariance = std::array::from_fn::<_, 3, _>(|i| {
                std::array::from_fn::<_, 3, _>(|j| {
                    centered.iter().map(|v| v[i] * v[j]).sum::<f64>() / 20.
                })
            });
            let trace: f64 = (0..3)
                .flat_map(|i| (0..3).map(move |j| inverse[i][j] * covariance[i][j]))
                .sum();
            for i in 0..4 {
                for j in 0..4 {
                    if let (Some((fi, si)), Some((fj, sj))) =
                        (local_faces[cell][i], local_faces[cell][j])
                    {
                        let value = si * sj * viscosity_pa_s / (9. * volumes[cell])
                            * (trace + dot(centered[i], mv(inverse, centered[j])));
                        if !value.is_finite() {
                            return Err("Darcy local resistance overflow");
                        }
                        local_mass[cell][i][j] = value;
                        if fi == fj {
                            diagonal[fi] += value;
                        }
                    }
                }
            }
        }
        if diagonal.iter().any(|v| !v.is_finite() || *v <= 0.) {
            return Err("invalid Darcy diagonal");
        }
        let diagonal_scale = diagonal.into_iter().map(f64::sqrt).collect();
        Ok(Self {
            faces,
            cells,
            points,
            volumes,
            local_faces,
            local_mass,
            diagonal_scale,
            reservoir_rank: None,
        })
    }
    /// Add passive hydraulic resistances in series with specified exterior ports.
    /// Each value is Pa s/m³: its pressure drop is R*q and loss is R*q².
    /// Resistance augments the physical operator, so implicit storage and finite
    /// reservoir steps retain it. Repeated calls add further series resistance.
    /// # Errors
    /// Rejects negative/nonfinite values, duplicate or inactive/internal ports
    /// and coefficient overflow. The original operator is never modified.
    pub fn with_added_boundary_resistances(
        &self,
        resistances: &[([usize; 3], f64)],
    ) -> Result<Self, &'static str> {
        let exterior: BTreeMap<_, _> = self
            .faces
            .iter()
            .enumerate()
            .filter(|(_, f)| f.neighbor.is_none())
            .map(|(i, f)| {
                let mut key = f.nodes;
                key.sort_unstable();
                (key, i)
            })
            .collect();
        let mut assigned = BTreeMap::new();
        for (nodes, resistance) in resistances {
            let mut key = *nodes;
            key.sort_unstable();
            if !resistance.is_finite()
                || *resistance < 0.
                || assigned.insert(key, *resistance).is_some()
                || !exterior.contains_key(&key)
            {
                return Err("invalid hydraulic boundary resistance");
            }
        }
        let mut trial = self.clone();
        for (key, resistance) in assigned {
            let face = exterior[&key];
            let owner = self.faces[face].owner;
            let local = self.local_faces[owner]
                .iter()
                .position(|f| f.is_some_and(|(i, _)| i == face))
                .ok_or("missing hydraulic boundary face")?;
            trial.local_mass[owner][local][local] += resistance;
            let diagonal = self.diagonal_scale[face].powi(2) + resistance;
            if !trial.local_mass[owner][local][local].is_finite() || !diagonal.is_finite() {
                return Err("hydraulic boundary resistance overflow");
            }
            trial.diagonal_scale[face] = diagonal.sqrt();
        }
        Ok(trial)
    }
    fn apply_mass(&self, x: &[f64]) -> Vec<f64> {
        let mut y = vec![0.; self.faces.len()];
        for (faces, matrix) in self.local_faces.iter().zip(&self.local_mass) {
            for (i, face) in faces.iter().enumerate() {
                if let Some((fi, _)) = face {
                    for (j, other) in faces.iter().enumerate() {
                        if let Some((fj, _)) = other {
                            y[*fi] += matrix[i][j] * x[*fj];
                        }
                    }
                }
            }
        }
        if let Some((coefficient, ports)) = &self.reservoir_rank {
            let total: f64 = ports.iter().map(|i| x[*i]).sum();
            for i in ports {
                y[*i] += coefficient * total;
            }
        }
        y
    }
    fn apply_scaled(&self, x: &[f64]) -> Vec<f64> {
        let q: Vec<_> = x
            .iter()
            .zip(&self.diagonal_scale)
            .map(|(x, d)| x / d)
            .collect();
        self.apply_mass(&q)
            .into_iter()
            .zip(&self.diagonal_scale)
            .map(|(v, d)| v / d)
            .collect()
    }
    fn solve_flux(
        &self,
        rhs: &[f64],
        config: DarcySolve,
    ) -> Result<(Vec<f64>, usize), &'static str> {
        let mut b: Vec<_> = rhs
            .iter()
            .zip(&self.diagonal_scale)
            .map(|(b, d)| b / d)
            .collect();
        if b.iter().any(|v| !v.is_finite()) {
            return Err("Darcy right-hand side overflow");
        }
        let scale = b.iter().map(|v| v.abs()).fold(0_f64, f64::max);
        if scale == 0. {
            return Ok((vec![0.; b.len()], 0));
        }
        for v in &mut b {
            *v /= scale;
        }
        let inner = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(a, b)| a * b).sum::<f64>();
        let threshold = config.relative_tolerance.powi(2) * inner(&b, &b);
        let mut x = vec![0.; b.len()];
        let mut r = b.clone();
        let mut direction = r.clone();
        let mut rr = inner(&r, &r);
        for iteration in 0..config.max_iterations {
            let ad = self.apply_scaled(&direction);
            let curvature = inner(&direction, &ad);
            if !curvature.is_finite() || curvature <= 0. {
                return Err("indefinite Darcy PCG operator");
            }
            let alpha = rr / curvature;
            for i in 0..x.len() {
                x[i] += alpha * direction[i];
                r[i] -= alpha * ad[i];
            }
            let mut next = inner(&r, &r);
            let recompute = next <= threshold || (iteration + 1) % 32 == 0;
            if recompute {
                let ax = self.apply_scaled(&x);
                for i in 0..r.len() {
                    r[i] = b[i] - ax[i];
                }
                next = inner(&r, &r);
            }
            if !next.is_finite() {
                return Err("Darcy PCG overflow");
            }
            if next <= threshold {
                let q: Vec<_> = x
                    .into_iter()
                    .zip(&self.diagonal_scale)
                    .map(|(x, d)| x * scale / d)
                    .collect();
                if q.iter().any(|v| !v.is_finite()) {
                    return Err("Darcy flux overflow");
                }
                return Ok((q, iteration + 1));
            }
            let beta = if recompute { 0. } else { next / rr };
            for i in 0..direction.len() {
                direction[i] = r[i] + beta * direction[i];
            }
            rr = next;
        }
        Err("Darcy PCG did not converge")
    }
    #[must_use]
    pub fn faces(&self) -> &[DarcyFace] {
        &self.faces
    }
    /// # Errors
    /// Rejects wrong/nonfinite pressures and arithmetic overflow.
    pub fn response(&self, pressures: &[f64]) -> Result<DarcyResponse, &'static str> {
        self.response_with_solver(pressures, DarcySolve::default())
    }
    /// # Errors
    /// Also rejects invalid solver limits and nonconverged/indefinite PCG iterations.
    pub fn response_with_solver(
        &self,
        pressures: &[f64],
        config: DarcySolve,
    ) -> Result<DarcyResponse, &'static str> {
        if config.max_iterations == 0
            || config.max_iterations > 100_000
            || !config.relative_tolerance.is_finite()
            || config.relative_tolerance <= 0.
            || config.relative_tolerance > 0.01
        {
            return Err("invalid Darcy solve options");
        }
        if pressures.len() != self.cells.len() || pressures.iter().any(|p| !p.is_finite()) {
            return Err("invalid Darcy cell pressures");
        }
        let rhs: Vec<_> = self
            .faces
            .iter()
            .map(|f| {
                pressures[f.owner]
                    - f.neighbor
                        .map_or_else(|| f.boundary_pressure_pa.unwrap_or(0.), |i| pressures[i])
            })
            .collect();
        let (q, solver_iterations) = self.solve_flux(&rhs, config)?;
        let mut outflows = vec![0.; self.cells.len()];
        let mut velocities = vec![[0.; 3]; self.cells.len()];
        for (i, nodes) in self.cells.iter().enumerate() {
            let origin = self.points[nodes[0]];
            let x = nodes.map(|n| sub(self.points[n], origin));
            let center = scale(x.into_iter().fold([0.; 3], add), 0.25);
            for (j, vertex) in x.iter().enumerate() {
                if let Some((face, sign)) = self.local_faces[i][j] {
                    let flow = sign * q[face];
                    outflows[i] += flow;
                    velocities[i] = add(
                        velocities[i],
                        scale(sub(center, *vertex), flow / (3. * self.volumes[i])),
                    );
                }
            }
        }
        let mut dissipation = 0.;
        let mut residual = 0_f64;
        let applied = self.apply_mass(&q);
        for ((q, mq), b) in q.iter().zip(applied).zip(&rhs) {
            dissipation += q * mq;
            residual = residual.max((mq - b).abs());
        }
        let pressure_work: f64 = q.iter().zip(&rhs).map(|(q, p)| q * p).sum();
        if !dissipation.is_finite()
            || !pressure_work.is_finite()
            || !residual.is_finite()
            || outflows
                .iter()
                .chain(velocities.iter().flatten())
                .any(|x| !x.is_finite())
        {
            return Err("Darcy response overflow");
        }
        let pressure_scale = rhs.iter().map(|v| v.abs()).fold(0_f64, f64::max);
        if residual > 1e-8 * pressure_scale || dissipation < 0. || pressure_work < 0. {
            return Err("inaccurate or nonpassive Darcy solve");
        }
        Ok(DarcyResponse {
            face_flows_m3_per_s: q,
            cell_outflows_m3_per_s: outflows,
            cell_centroid_velocities_m_per_s: velocities,
            dissipation_w: dissipation,
            pressure_work_w: pressure_work,
            residual_pa: residual,
            solver_iterations,
        })
    }
}

/// Matrix-free diagonally scaled conjugate-gradient controls.
#[derive(Clone, Copy, Debug)]
pub struct DarcySolve {
    pub max_iterations: usize,
    pub relative_tolerance: f64,
}
impl Default for DarcySolve {
    fn default() -> Self {
        Self {
            max_iterations: 2000,
            relative_tolerance: 1e-13,
        }
    }
}

impl MixedDarcy {
    /// Allocated operator payload, excluding vector headers and constructor scratch.
    #[must_use]
    pub fn operator_storage_bytes(&self) -> usize {
        use std::mem::size_of;
        self.faces.capacity() * size_of::<DarcyFace>()
            + self.cells.capacity() * size_of::<[usize; 4]>()
            + self.points.capacity() * size_of::<Vec3>()
            + self.volumes.capacity() * size_of::<f64>()
            + self.local_faces.capacity() * size_of::<[Option<(usize, f64)>; 4]>()
            + self.local_mass.capacity() * size_of::<[[f64; 4]; 4]>()
            + self.diagonal_scale.capacity() * size_of::<f64>()
            + self
                .reservoir_rank
                .as_ref()
                .map_or(0, |(_, ports)| ports.capacity() * size_of::<usize>())
    }
}

// Positive sparse M-matrix solve shared by sealed, prescribed and finite reservoirs.
pub(crate) fn solve_protein(
    rhs_mass: &[f64],
    diagonal: &[f64],
    incoming: &[Vec<(usize, f64)>],
    external_out: &[f64],
) -> Result<Vec<f64>, &'static str> {
    let scale = rhs_mass.iter().copied().fold(0_f64, f64::max);
    if scale == 0. {
        return Ok(vec![0.; rhs_mass.len()]);
    }
    let rhs: Vec<_> = rhs_mass.iter().map(|m| m / scale).collect();
    let total: f64 = rhs.iter().sum();
    let mut mass = rhs.clone();
    // Positive Gauss-Seidel iteration for the conservative transport M-matrix.
    // No clipping or post-hoc mass redistribution is used.
    for _ in 0..20_000 {
        for i in 0..mass.len() {
            mass[i] =
                (rhs[i] + incoming[i].iter().map(|(j, c)| c * mass[*j]).sum::<f64>()) / diagonal[i];
        }
        let residual: f64 = (0..mass.len())
            .map(|i| {
                (diagonal[i] * mass[i]
                    - incoming[i].iter().map(|(j, c)| c * mass[*j]).sum::<f64>()
                    - rhs[i])
                    .abs()
            })
            .sum();
        let drift = (mass
            .iter()
            .zip(external_out)
            .map(|(m, c)| m * (1. + c))
            .sum::<f64>()
            - total)
            .abs();
        if !residual.is_finite() || mass.iter().any(|m| !m.is_finite() || *m < 0.) {
            return Err("implicit protein iteration overflow");
        }
        if residual <= 1e-14 * total && drift <= 1e-14 * total {
            let output: Vec<_> = mass.into_iter().map(|m| m * scale).collect();
            if output.iter().any(|m| !m.is_finite()) {
                return Err("implicit protein inventory overflow");
            }
            return Ok(output);
        }
    }
    Err("implicit protein transport did not converge")
}
