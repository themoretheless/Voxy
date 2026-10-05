//! Quasistatic tetrahedral finite elements in metres, newtons and pascals.
//! Constitutive verification is distinct from anatomical/experimental validation.
#![allow(clippy::many_single_char_names)]
mod cell_poroelastic;
mod implicit_pore;
mod vascular_pore;
pub use implicit_pore::{ImplicitPoreConfig, ImplicitPoreReport};
pub use vascular_pore::{
    OsmoticPressureLaw, VascularPoreConfig, VascularPorePort, VascularPoreReport,
};
mod mixed_darcy;
pub(crate) use mixed_darcy::solve_protein;
mod tetra_mesh;
pub use mixed_darcy::{
    DarcyFace, DarcyResponse, DarcySolve, MixedDarcy, PoreReservoir, ProteinMembrane,
};
pub use tetra_mesh::TetraMesh;
mod coupling;
mod poroelastic;
pub use cell_poroelastic::{AdaptiveTissueExchangeConfig, CellPoreTissue, LymphaticWallAttachment};
pub use poroelastic::{PoreFluid, PoreTissue};
mod inertia;
pub use inertia::{
    DrivenMuscleStep, DrivenSupportStep, InertialBody, InertialDiagnostics, MuscleDynamicStep,
    PlaneContact, SolidFilmBinding, SupportTarget, ViscoelasticDynamicStep,
};
mod invariants;
mod myocardium;
pub use coupling::{CouplingConfig, FemChamber};
mod viscoelastic;
pub use viscoelastic::{MaxwellBranch, OgdenTerm, ViscoelasticOgden};
mod stress_memory;
pub use stress_memory::{FrozenStressPotential, ReferenceStressMemory};
mod hgo;
pub use hgo::HgoMaterial;
mod viscoelastic_hgo;
pub use viscoelastic_hgo::ViscoelasticHgo;
mod muscle_velocity;
pub use muscle_velocity::{ActiveFiberVelocityLaw, MuscleVelocityResponse};
mod muscle_activation;
pub use muscle_activation::{ActivationKinetics, ActiveFiberLengthLaw, MuscleRegionDrive};
mod perfusion;
pub use perfusion::{PerfusionCellSolute, PerfusionPort, PorePerfusion};
mod contact_mollifier;
mod lbfgs;
mod surface_contact;
pub use contact_mollifier::edge_contact_mollifier;
mod contact_precision;
mod prescribed_surface;
mod surface_distance;
pub use prescribed_surface::{
    PrescribedContactFeature, PrescribedContactPathResponse, PrescribedContactStencil,
    PrescribedSurfaceResponse, PrescribedTriangleSurface,
};
pub use surface_contact::{SurfaceContactLaw, SurfacePrimitive, TissueSurfaceContact};
mod tissue_gaps;
pub use tissue_gaps::TissueGap;
mod tissue_bonds;
pub use tissue_bonds::{TissueAssembly, TissueBond};
mod urogenital;
pub use urogenital::{ClitoralComplex, ClitoralGeometry, UrogenitalWallGeometry};
mod genital_pads;
pub use genital_pads::{LabiaMajoraGeometry, LabialPad};
mod specimens;
mod stress;
pub use myocardium::{ExponentialTerm, Myocardium};
pub use specimens::*;
pub use stress::{ElementStress, Stress};
pub type Vec3 = [f64; 3];
pub type Matrix = [[f64; 3]; 3];
pub const IDENTITY: Matrix = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
fn add(a: Vec3, b: Vec3) -> Vec3 {
    std::array::from_fn(|i| a[i] + b[i])
}
fn sub(a: Vec3, b: Vec3) -> Vec3 {
    std::array::from_fn(|i| a[i] - b[i])
}
fn scale(a: Vec3, s: f64) -> Vec3 {
    a.map(|x| x * s)
}
fn dot(a: Vec3, b: Vec3) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn transpose(a: Matrix) -> Matrix {
    std::array::from_fn(|i| std::array::from_fn(|j| a[j][i]))
}
fn mv(a: Matrix, x: Vec3) -> Vec3 {
    a.map(|r| dot(r, x))
}
fn mm(a: Matrix, b: Matrix) -> Matrix {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| a[i][k] * b[k][j]).sum()))
}
fn outer(a: Vec3, b: Vec3) -> Matrix {
    std::array::from_fn(|i| b.map(|x| a[i] * x))
}
fn det(a: Matrix) -> f64 {
    dot(a[0], cross(a[1], a[2]))
}
fn inverse(a: Matrix) -> Result<Matrix, &'static str> {
    let d = det(a);
    if !d.is_finite() || d.abs() < 1e-18 {
        return Err("singular deformation");
    }
    Ok(transpose([
        scale(cross(a[1], a[2]), 1. / d),
        scale(cross(a[2], a[0]), 1. / d),
        scale(cross(a[0], a[1]), 1. / d),
    ]))
}
fn columns(a: Vec3, b: Vec3, c: Vec3) -> Matrix {
    transpose([a, b, c])
}
/// Tension-only aligned exponential reinforcement, plus constant nominal active
/// tension. Direction is in the reference configuration; no dispersion/Hill law.
#[derive(Clone, Copy, Debug)]
pub struct Fiber {
    pub direction: Vec3,
    pub stiffness_pa: f64,
    pub exponent: f64,
    pub active_pa: f64,
}
#[derive(Clone, Debug)]
pub struct Material {
    pub shear_pa: f64,
    pub bulk_pa: f64,
    pub fibers: Vec<Fiber>,
}
#[derive(Clone, Copy, Debug)]
pub struct Response {
    pub energy_density: f64,
    pub first_piola: Matrix,
    pub volume_ratio: f64,
}
impl Material {
    /// Isochoric neo-Hookean matrix + volumetric penalty + exponential fibers.
    /// # Errors
    /// Rejects invalid parameters, activation, inversion and numerical overflow.
    pub fn response(&self, f: Matrix, activation: f64) -> Result<Response, &'static str> {
        self.response_with_active_length(f, activation, None)
    }
    /// Aligned-fiber response with an optional explicit active force/stretch curve.
    /// # Errors
    /// Invalid material, length law, deformation or numerical overflow.
    pub fn response_with_active_length(
        &self,
        f: Matrix,
        activation: f64,
        length_law: Option<ActiveFiberLengthLaw>,
    ) -> Result<Response, &'static str> {
        if let Some(law) = length_law {
            law.validate()?;
        }

        if !activation.is_finite()
            || !(0.0..=1.0).contains(&activation)
            || !self.shear_pa.is_finite()
            || self.shear_pa <= 0.
            || !self.bulk_pa.is_finite()
            || self.bulk_pa <= 0.
            || f.iter().flatten().any(|x| !x.is_finite())
        {
            return Err("invalid material input");
        }
        let j = det(f);
        if j <= 0. || !j.is_finite() {
            return Err("inverted element");
        }
        let inv_t = transpose(inverse(f)?);
        let i1: f64 = f.iter().flatten().map(|x| x * x).sum();
        let q = j.powf(-2. / 3.);
        let excess = invariants::isochoric_excess(f, q, q * i1);
        let mut energy = self.shear_pa / 2. * excess + self.bulk_pa / 2. * (j - 1.).powi(2);
        let mut p: Matrix = std::array::from_fn(|i| {
            std::array::from_fn(|k| {
                self.shear_pa * q * (f[i][k] - i1 / 3. * inv_t[i][k])
                    + self.bulk_pa * (j - 1.) * j * inv_t[i][k]
            })
        });
        for fiber in &self.fibers {
            if fiber.direction.iter().any(|x| !x.is_finite())
                || (dot(fiber.direction, fiber.direction) - 1.).abs() > 1e-8
                || !fiber.stiffness_pa.is_finite()
                || fiber.stiffness_pa < 0.
                || !fiber.exponent.is_finite()
                || fiber.exponent <= 0.
                || !fiber.active_pa.is_finite()
                || fiber.active_pa < 0.
            {
                return Err("invalid fiber");
            }
            let a = mv(f, fiber.direction);
            let l2 = dot(a, a);
            let l = l2.sqrt();
            if l < 1e-12 {
                return Err("collapsed fiber");
            }
            let e = (l2 - 1.).max(0.);
            let exp = (fiber.exponent * e * e).exp();
            let (active_factor, active_potential) = match length_law {
                Some(law) => law.response(l)?,
                None => (1., l - 1.),
            };
            energy += fiber.stiffness_pa / (2. * fiber.exponent)
                * (fiber.exponent * e * e).exp_m1()
                + activation * fiber.active_pa * active_potential;
            let factor = 2. * fiber.stiffness_pa * e * exp
                + activation * fiber.active_pa * active_factor / l;
            let pa = outer(a, fiber.direction);
            for i in 0..3 {
                for k in 0..3 {
                    p[i][k] += factor * pa[i][k];
                }
            }
        }
        if !energy.is_finite() || p.iter().flatten().any(|x| !x.is_finite()) {
            return Err("material overflow");
        }
        Ok(Response {
            energy_density: energy,
            first_piola: p,
            volume_ratio: j,
        })
    }
}
#[derive(Clone, Debug)]
pub struct Element {
    /// Application material region; tube specimens use radial layer indices.
    pub region: usize,
    pub nodes: [usize; 4],
    pub material: Material,
    pub activation: f64,
    active_length_law: Option<ActiveFiberLengthLaw>,
    myocardium: Option<Myocardium>,
    viscoelastic: Option<ViscoelasticOgden>,
    viscoelastic_hgo: Option<ViscoelasticHgo>,
    inv_rest: Matrix,
    volume: f64,
    gradients: [Vec3; 4],
}
impl Element {
    /// Constitutive response of this element, including an assigned cardiac law.
    /// # Errors
    /// Rejects invalid deformation and constitutive overflow.
    pub fn response(&self, deformation: Matrix) -> Result<Response, &'static str> {
        if let Some(law) = &self.viscoelastic_hgo {
            return if law.trial_seconds == 0. {
                law.committed_response(deformation)
            } else {
                law.trial(deformation, law.trial_seconds)
            };
        }
        if let Some(law) = &self.viscoelastic {
            return law.response(deformation, law.trial_seconds);
        }
        match self.myocardium {
            Some(law) => law.response(deformation, self.activation),
            None => self.material.response_with_active_length(
                deformation,
                self.activation,
                self.active_length_law,
            ),
        }
    }
}
/// Oriented, closed triangular cavity boundary (outward from cavity).
#[derive(Clone, Debug)]
pub struct Cavity {
    pub faces: Vec<[usize; 3]>,
    /// Prescribed pressure difference. Signed values require the gauge-pressure API.
    pub pressure_pa: f64,
}
#[derive(Clone, Debug)]
pub struct Body {
    positions: Vec<Vec3>,
    rest: Vec<Vec3>,
    pinned: Vec<bool>,
    elements: Vec<Element>,
    cavities: Vec<Cavity>,
    forces: Vec<Vec3>,
    diagonal: Vec<f64>,
    pore_fluid: Option<PoreFluid>,
    cell_pore_fluids: Vec<PoreFluid>,
    tissue_bonds: Vec<TissueBond>,
    tissue_gaps: Vec<TissueGap>,
    surface_contacts: Vec<TissueSurfaceContact>,
    surface_contact_law: SurfaceContactLaw,
}
#[derive(Clone, Copy, Debug)]
pub struct Equilibrium {
    pub iterations: usize,
    pub residual_n: f64,
    pub converged: bool,
    pub min_j: f64,
    pub max_j: f64,
}
impl Body {
    /// # Errors
    /// Rejects invalid geometry/materials and degenerate tetrahedra.
    pub fn new(
        points: Vec<Vec3>,
        pinned: Vec<bool>,
        cells: Vec<([usize; 4], Material)>,
    ) -> Result<Self, &'static str> {
        if points.is_empty()
            || pinned.len() != points.len()
            || points.iter().flatten().any(|x| !x.is_finite())
        {
            return Err("invalid mesh");
        }
        let mut elements = Vec::new();
        let mut diagonal = vec![0.; points.len()];
        for (nodes, material) in cells {
            if nodes.iter().any(|i| *i >= points.len()) {
                return Err("invalid element index");
            }
            material.response(IDENTITY, 0.)?;
            let [a, b, c, d] = nodes.map(|i| points[i]);
            let dm = columns(sub(b, a), sub(c, a), sub(d, a));
            let volume = det(dm).abs() / 6.;
            if volume < 1e-15 {
                return Err("degenerate element");
            }
            let inv_rest = inverse(dm)?;
            let gradients = [
                scale(add(add(inv_rest[0], inv_rest[1]), inv_rest[2]), -1.),
                inv_rest[0],
                inv_rest[1],
                inv_rest[2],
            ];
            let stiffness = material.shear_pa
                + material.bulk_pa
                + material
                    .fibers
                    .iter()
                    .map(|f| 4. * f.stiffness_pa + f.active_pa)
                    .sum::<f64>();
            for k in 0..4 {
                diagonal[nodes[k]] += volume * stiffness * dot(gradients[k], gradients[k]);
            }
            elements.push(Element {
                region: 0,
                nodes,
                material,
                activation: 0.,
                active_length_law: None,
                myocardium: None,
                viscoelastic: None,
                viscoelastic_hgo: None,
                inv_rest,
                volume,
                gradients,
            });
        }
        if diagonal.iter().zip(&pinned).any(|(d, p)| !*p && *d <= 0.) {
            return Err("unconnected free vertex");
        }
        Ok(Self {
            forces: vec![[0.; 3]; points.len()],
            rest: points.clone(),
            positions: points,
            pinned,
            elements,
            cavities: Vec::new(),
            pore_fluid: None,
            cell_pore_fluids: Vec::new(),
            tissue_bonds: Vec::new(),
            tissue_gaps: Vec::new(),
            surface_contacts: Vec::new(),
            surface_contact_law: SurfaceContactLaw::TriangleMinimum,
            diagonal,
        })
    }
    #[must_use]
    pub fn positions(&self) -> &[Vec3] {
        &self.positions
    }
    #[must_use]
    pub fn rest_positions(&self) -> &[Vec3] {
        &self.rest
    }
    #[must_use]
    pub fn elements(&self) -> &[Element] {
        &self.elements
    }
    #[must_use]
    pub fn cavities(&self) -> &[Cavity] {
        &self.cavities
    }
    /// # Errors
    /// Rejects invalid cavity indices and nonfinite/negative pressures.
    pub fn set_pressure(&mut self, index: usize, pressure_pa: f64) -> Result<(), &'static str> {
        if index >= self.cavities.len() || !pressure_pa.is_finite() || pressure_pa < 0. {
            return Err("invalid cavity pressure");
        }
        self.cavities[index].pressure_pa = pressure_pa;
        Ok(())
    }
    /// Signed pressure difference (inside minus outside) on a closed boundary.
    /// Negative values produce an inward follower load through potential -p V.
    /// This is a prescribed pressure difference, not a fluid-flow model.
    pub fn set_gauge_pressure(
        &mut self,
        index: usize,
        pressure_pa: f64,
    ) -> Result<(), &'static str> {
        if index >= self.cavities.len() || !pressure_pa.is_finite() {
            return Err("invalid cavity gauge pressure");
        }
        self.cavities[index].pressure_pa = pressure_pa;
        Ok(())
    }
    /// Install a closed outward-oriented boundary with a signed pressure difference.
    /// Retains the topology and geometry validation of `add_cavity`.
    pub fn add_gauge_pressure_cavity(&mut self, mut cavity: Cavity) -> Result<(), &'static str> {
        let pressure = cavity.pressure_pa;
        cavity.pressure_pa = pressure.abs();
        self.add_cavity(cavity)?;
        self.cavities.last_mut().unwrap().pressure_pa = pressure;
        Ok(())
    }
    /// # Errors
    /// Rejects invalid element indices or activation outside [0,1].
    pub fn set_activation(&mut self, index: usize, activation: f64) -> Result<(), &'static str> {
        if index >= self.elements.len()
            || !activation.is_finite()
            || !(0.0..=1.0).contains(&activation)
            || (self.elements[index].viscoelastic.is_some() && activation != 0.)
            || (self.elements[index].viscoelastic_hgo.is_some() && activation != 0.)
        {
            return Err("invalid element activation");
        }
        self.elements[index].activation = activation;
        Ok(())
    }
    /// Assign an orthotropic cardiac material to a tetrahedron.
    /// Each element can carry its own fiber/sheet frame.
    /// # Errors
    /// Rejects invalid indices/materials before changing the body.
    pub fn set_myocardium(&mut self, index: usize, law: Myocardium) -> Result<(), &'static str> {
        self.set_myocardium_batch(&[(index, law)])
    }
    /// Assign an element-wise cardiac field with one preconditioner rebuild.
    /// Unlisted elements retain their existing constitutive laws and histories.
    /// # Errors
    /// Rejects invalid/duplicate indices, invalid laws or overflow atomically.
    pub fn set_myocardium_batch(
        &mut self,
        updates: &[(usize, Myocardium)],
    ) -> Result<(), &'static str> {
        if updates.is_empty() {
            return Ok(());
        }
        let mut replacements = vec![None; self.elements.len()];
        for &(index, law) in updates {
            let slot = replacements
                .get_mut(index)
                .ok_or("invalid cardiac element index")?;
            if slot.is_some() {
                return Err("duplicate cardiac element index");
            }
            law.response(IDENTITY, 0.)?;
            *slot = Some(law);
        }
        let mut diagonal = vec![0.; self.diagonal.len()];
        for (element_index, e) in self.elements.iter().enumerate() {
            let assigned = replacements[element_index].or(e.myocardium);
            let stiffness = if replacements[element_index].is_none()
                && let Some(hgo) = &e.viscoelastic_hgo
            {
                hgo.stiffness()
            } else if replacements[element_index].is_none()
                && let Some(viscoelastic) = &e.viscoelastic
            {
                viscoelastic.stiffness()
            } else {
                match assigned {
                    Some(m) => {
                        m.bulk_pa
                            + m.matrix.scale_pa
                            + 4. * (m.fiber.scale_pa + m.sheet.scale_pa + m.fiber_sheet.scale_pa)
                            + m.active_tension_pa
                    }
                    None => {
                        e.material.shear_pa
                            + e.material.bulk_pa
                            + e.material
                                .fibers
                                .iter()
                                .map(|f| 4. * f.stiffness_pa + f.active_pa)
                                .sum::<f64>()
                    }
                }
            };
            for k in 0..4 {
                diagonal[e.nodes[k]] += e.volume * stiffness * dot(e.gradients[k], e.gradients[k]);
            }
        }
        if diagonal.iter().any(|v| !v.is_finite()) {
            return Err("cardiac preconditioner overflow");
        }
        for (element, replacement) in self.elements.iter_mut().zip(replacements) {
            if let Some(law) = replacement {
                element.myocardium = Some(law);
                element.viscoelastic = None;
                element.viscoelastic_hgo = None;
            }
        }
        self.diagonal = diagonal;
        Ok(())
    }
    /// Nodal external force in newtons.
    /// # Errors
    /// Rejects invalid vertex indices or nonfinite loads.
    pub fn set_force(&mut self, index: usize, force: Vec3) -> Result<(), &'static str> {
        if index >= self.forces.len() || force.iter().any(|x| !x.is_finite()) {
            return Err("invalid nodal load");
        }
        self.forces[index] = force;
        Ok(())
    }
    /// Validates that every edge occurs twice with opposite winding.
    /// # Errors
    /// Rejects open/nonmanifold, nonpositive-volume or invalid pressure boundaries.
    pub fn add_cavity(&mut self, cavity: Cavity) -> Result<(), &'static str> {
        use std::collections::BTreeMap;
        if !cavity.pressure_pa.is_finite() || cavity.pressure_pa < 0. || cavity.faces.is_empty() {
            return Err("invalid cavity");
        }
        let mut edges = BTreeMap::<[usize; 2], (usize, i32)>::new();
        let mut v = 0.;
        for face in &cavity.faces {
            if face.iter().any(|i| *i >= self.positions.len()) {
                return Err("invalid cavity vertex");
            }
            let [a, b, c] = face.map(|i| self.positions[i]);
            if dot(cross(sub(b, a), sub(c, a)), cross(sub(b, a), sub(c, a))) < 1e-24 {
                return Err("degenerate cavity face");
            }
            v += dot(a, cross(b, c)) / 6.;
            for k in 0..3 {
                let a = face[k];
                let b = face[(k + 1) % 3];
                let key = [a.min(b), a.max(b)];
                let e = edges.entry(key).or_default();
                e.0 += 1;
                e.1 += if a < b { 1 } else { -1 };
            }
        }
        if edges.values().any(|&(n, s)| n != 2 || s != 0) || v <= 0. {
            return Err("cavity must be closed and outward oriented");
        }
        self.cavities.push(cavity);
        Ok(())
    }
    /// Potential energy and its nodal gradient, including -pressure * cavity volume.
    /// # Errors
    /// Rejects invalid loads, inverted elements and nonfinite results.
    pub fn evaluate(&self, positions: &[Vec3]) -> Result<(f64, Vec<Vec3>), &'static str> {
        if positions.len() != self.positions.len()
            || positions.iter().flatten().any(|x| !x.is_finite())
            || self.forces.iter().flatten().any(|x| !x.is_finite())
        {
            return Err("invalid evaluation");
        }
        let (pore_pressure, mut energy) = self.pore_fields_at(positions)?;
        let mut g = vec![[0.; 3]; positions.len()];
        for (index, e) in self.elements.iter().enumerate() {
            let [a, b, c, d] = e.nodes.map(|i| positions[i]);
            let f = mm(columns(sub(b, a), sub(c, a), sub(d, a)), e.inv_rest);
            let mut r = e.response(f)?;
            let pore = Self::pore_piola(f, pore_pressure[index])?;
            for (row, pore_row) in r.first_piola.iter_mut().zip(pore) {
                for (value, correction) in row.iter_mut().zip(pore_row) {
                    *value += correction;
                }
            }
            energy += e.volume * r.energy_density;
            for k in 0..4 {
                let i = e.nodes[k];
                g[i] = add(g[i], scale(mv(r.first_piola, e.gradients[k]), e.volume));
            }
        }
        energy += self.bond_energy_gradient(positions, &mut g)?;
        energy += self.gap_energy_gradient(positions, &mut g)?;
        energy += self.surface_energy_gradient(positions, &mut g)?;
        for cavity in &self.cavities {
            if !cavity.pressure_pa.is_finite() {
                return Err("invalid pressure");
            }
            for &[a, b, c] in &cavity.faces {
                energy -=
                    cavity.pressure_pa * dot(positions[a], cross(positions[b], positions[c])) / 6.;
                let s = -cavity.pressure_pa / 6.;
                for (i, v) in [
                    (a, cross(positions[b], positions[c])),
                    (b, cross(positions[c], positions[a])),
                    (c, cross(positions[a], positions[b])),
                ] {
                    g[i] = add(g[i], scale(v, s));
                }
            }
        }
        for i in 0..positions.len() {
            energy -= dot(self.forces[i], sub(positions[i], self.rest[i]));
            g[i] = sub(g[i], self.forces[i]);
        }
        if !energy.is_finite() || g.iter().flatten().any(|x| !x.is_finite()) {
            return Err("load overflow");
        }
        Ok((energy, g))
    }
    /// Bounded preconditioned nonlinear conjugate gradient with Armijo line search. This solves static
    /// equilibrium: iteration count is NOT physiological time. Reports residual.
    /// # Errors
    /// Invalid inputs/evaluation errors leave the positions unchanged.
    #[allow(clippy::too_many_lines)] // Keep bounded line search and transactional commit together.
    pub fn equilibrate(
        &mut self,
        max_iterations: usize,
        tolerance_n: f64,
    ) -> Result<Equilibrium, &'static str> {
        if max_iterations == 0
            || max_iterations > 100_000
            || !tolerance_n.is_finite()
            || tolerance_n <= 0.
        {
            return Err("invalid equilibrium options");
        }
        // Sealed fluid storage adds undrained volumetric stiffness. Leaving it
        // out can make accepted CG steps oscillate near the stability boundary.
        let diagonal = self.pore_preconditioner()?;
        let mut x = self.positions.clone();
        let origin = self.rest[0];
        let radius_squared = self
            .rest
            .iter()
            .map(|p| dot(sub(*p, origin), sub(*p, origin)))
            .fold(0_f64, f64::max);
        let tiny_step_squared = radius_squared * f64::EPSILON;
        let roundoff_energy = 64.
            * f64::EPSILON
            * diagonal
                .iter()
                .zip(&self.rest)
                .map(|(d, p)| d * dot(sub(*p, origin), sub(*p, origin)))
                .sum::<f64>();
        if !radius_squared.is_finite() || !roundoff_energy.is_finite() {
            return Err("equilibrium scale overflow");
        }
        let mut iterations = 0;
        let mut previous_z: Vec<Vec3> = Vec::new();
        let mut previous_direction: Vec<Vec3> = Vec::new();
        let mut previous_norm = 0.0;
        // Preserve conjugate directions across enough iterations to resolve
        // large-mesh low-frequency modes; short specimens retain the old period.
        let restart_period = self.positions.len().clamp(32, 256);
        for _ in 0..max_iterations {
            let (energy, g) = self.evaluate(&x)?;
            let residual = g
                .iter()
                .zip(&self.pinned)
                .filter(|(_, p)| !**p)
                .map(|(v, _)| dot(*v, *v))
                .sum::<f64>()
                .sqrt();
            if residual <= tolerance_n {
                break;
            }
            let z: Vec<_> = g
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    if self.pinned[i] {
                        [0.; 3]
                    } else {
                        scale(*v, 1. / diagonal[i])
                    }
                })
                .collect();
            let norm: f64 = g.iter().zip(&z).map(|(a, b)| dot(*a, *b)).sum();
            let beta = if previous_norm > 0. && iterations % restart_period != 0 {
                (g.iter()
                    .zip(z.iter().zip(&previous_z))
                    .map(|(g, (z, old))| dot(*g, sub(*z, *old)))
                    .sum::<f64>()
                    / previous_norm)
                    .clamp(0., 2.)
            } else {
                0.
            };
            let mut direction: Vec<_> = z
                .iter()
                .enumerate()
                .map(|(i, z)| {
                    if beta > 0. {
                        add(scale(*z, -1.), scale(previous_direction[i], beta))
                    } else {
                        scale(*z, -1.)
                    }
                })
                .collect();
            let mut slope: f64 = g.iter().zip(&direction).map(|(a, b)| dot(*a, *b)).sum();
            if slope >= -0.01 * norm {
                direction = z.iter().map(|z| scale(*z, -1.)).collect();
                slope = -norm;
            }
            previous_z = z;
            previous_direction.clone_from(&direction);
            previous_norm = norm;
            let mut alpha = 8.;
            let mut accepted = false;
            for _ in 0..40 {
                let trial: Vec<_> = x
                    .iter()
                    .zip(&direction)
                    .map(|(a, b)| add(*a, scale(*b, alpha)))
                    .collect();
                if !self.gap_path_is_open(&x, &trial) {
                    alpha *= 0.5;
                    continue;
                }
                if let Ok((e, trial_gradient)) = self.evaluate(&trial) {
                    let armijo = e <= energy + 1e-4 * alpha * slope;
                    // At the floating-point energy floor, a geometrically tiny
                    // step is accepted only if its independently evaluated force
                    // residual decreases and the energy change is within roundoff.
                    let tiny = direction
                        .iter()
                        .all(|d| alpha * alpha * dot(*d, *d) <= tiny_step_squared);
                    let trial_residual_squared = trial_gradient
                        .iter()
                        .zip(&self.pinned)
                        .filter(|(_, p)| !**p)
                        .map(|(g, _)| dot(*g, *g))
                        .sum::<f64>();
                    let roundoff_progress = tiny
                        && (e - energy).abs() <= roundoff_energy
                        && trial_residual_squared < residual * residual * (1. - 1e-4);
                    if armijo || roundoff_progress {
                        x = trial;
                        accepted = true;
                        break;
                    }
                }
                alpha *= 0.5;
            }
            if !accepted {
                break;
            }
            iterations += 1;
        }
        let (_, g) = self.evaluate(&x)?;
        let residual_n = g
            .iter()
            .zip(&self.pinned)
            .filter(|(_, p)| !**p)
            .map(|(v, _)| dot(*v, *v))
            .sum::<f64>()
            .sqrt();
        let mut min_j = f64::INFINITY;
        let mut max_j = f64::NEG_INFINITY;
        for e in &self.elements {
            let [a, b, c, d] = e.nodes.map(|i| x[i]);
            let j = det(mm(columns(sub(b, a), sub(c, a), sub(d, a)), e.inv_rest));
            min_j = min_j.min(j);
            max_j = max_j.max(j);
        }
        self.positions = x;
        Ok(Equilibrium {
            iterations,
            residual_n,
            converged: residual_n <= tolerance_n,
            min_j,
            max_j,
        })
    }
    /// Boundary faces oriented away from the solid. Internal shared faces removed.
    #[must_use]
    pub fn surface(&self) -> Vec<[usize; 3]> {
        use std::collections::BTreeMap;
        let mut faces = BTreeMap::<[usize; 3], ([usize; 3], usize)>::new();
        for e in &self.elements {
            for (mut face, opposite) in [
                ([e.nodes[1], e.nodes[2], e.nodes[3]], e.nodes[0]),
                ([e.nodes[0], e.nodes[3], e.nodes[2]], e.nodes[1]),
                ([e.nodes[0], e.nodes[1], e.nodes[3]], e.nodes[2]),
                ([e.nodes[0], e.nodes[2], e.nodes[1]], e.nodes[3]),
            ] {
                let [a, b, c] = face.map(|i| self.rest[i]);
                if dot(cross(sub(b, a), sub(c, a)), sub(self.rest[opposite], a)) > 0. {
                    face.swap(1, 2);
                }
                let mut key = face;
                key.sort_unstable();
                let entry = faces.entry(key).or_insert((face, 0));
                entry.1 += 1;
            }
        }
        faces
            .into_values()
            .filter(|(_, n)| *n == 1)
            .map(|(f, _)| f)
            .collect()
    }
}

mod calibration;
pub use calibration::{FitError, TensilePoint, fit_tensile, tensile_error, tensile_stress};
