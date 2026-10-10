//! Barycentric embedding of a render mesh in a coarse tetrahedral simulation.
//! Binding reuses a rest-space bounds index; deformation is O(surface vertices).
type Point = [f64; 3];
use std::sync::Arc;
mod search;
pub use search::TetrahedralEmbedding;
fn sub(a: Point, b: Point) -> Point {
    std::array::from_fn(|i| a[i] - b[i])
}
fn determinant(a: Point, b: Point, c: Point) -> f64 {
    a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
        + a[2] * (b[0] * c[1] - b[1] * c[0])
}
#[derive(Clone, Debug)]
struct Binding {
    indices: [usize; 4],
    weights: [f64; 4],
}
/// Immutable rest-space attachment. The simulation must retain its vertex order.
#[derive(Clone, Debug)]
pub struct EmbeddedSurface {
    bindings: Vec<Option<Binding>>,
    vertex_count: usize,
    rest: Arc<[Point]>,
    cells: Arc<[[usize; 4]]>,
    surface: Arc<[Point]>,
}
/// Forces conjugate to x_skin = x_base + W (x_nodes - x_reference).
/// The prescribed reference/base loads are separate from mechanical nodal loads.
#[derive(Clone, Debug)]
pub struct RelativeSurfaceLoads {
    nodal: Vec<Point>,
    reference: Vec<Point>,
    base: Vec<Point>,
}
impl RelativeSurfaceLoads {
    #[must_use]
    pub fn nodal_forces_n(&self) -> &[Point] {
        &self.nodal
    }
    #[must_use]
    pub fn reference_forces_n(&self) -> &[Point] {
        &self.reference
    }
    #[must_use]
    pub fn base_forces_n(&self) -> &[Point] {
        &self.base
    }
    /// Work done by an actuator against the skin load during prescribed motion.
    /// Use path-averaged forces for finite steps; instantaneous forces alone do
    /// not establish a finite-step energy balance for a nonlinear potential.
    /// # Errors
    /// Rejects mismatched/nonfinite displacements and overflowing work.
    pub fn actuator_work_j(
        &self,
        reference_delta: &[Point],
        base_delta: &[Point],
    ) -> Result<f64, &'static str> {
        if reference_delta.len() != self.reference.len()
            || base_delta.len() != self.base.len()
            || reference_delta
                .iter()
                .chain(base_delta)
                .flatten()
                .any(|x| !x.is_finite())
        {
            return Err("invalid prescribed skin displacement");
        }
        let work: f64 = self
            .reference
            .iter()
            .zip(reference_delta)
            .chain(self.base.iter().zip(base_delta))
            .flat_map(|(f, d)| (0..3).map(move |i| -f[i] * d[i]))
            .sum();
        if !work.is_finite() {
            return Err("prescribed skin work overflow");
        }
        Ok(work)
    }
    /// Actuator work between prescribed poses, without allocating displacement arrays.
    /// As with `actuator_work_j`, finite motion requires path-averaged loads.
    /// # Errors
    /// Rejects mismatched poses, nonfinite displacements and overflowing work.
    pub fn actuator_work_between_j(
        &self,
        reference_start: &[Point],
        reference_end: &[Point],
        base_start: &[Point],
        base_end: &[Point],
    ) -> Result<f64, &'static str> {
        if reference_start.len() != self.reference.len()
            || reference_end.len() != self.reference.len()
            || base_start.len() != self.base.len()
            || base_end.len() != self.base.len()
        {
            return Err("invalid prescribed skin displacement");
        }
        let deltas = || {
            reference_start
                .iter()
                .zip(reference_end)
                .chain(base_start.iter().zip(base_end))
                .map(|(&a, &b)| sub(b, a))
        };
        if deltas().flatten().any(|x| !x.is_finite()) {
            return Err("invalid prescribed skin displacement");
        }
        let work: f64 = self
            .reference
            .iter()
            .chain(&self.base)
            .zip(deltas())
            .flat_map(|(f, d)| (0..3).map(move |i| -f[i] * d[i]))
            .sum();
        if !work.is_finite() {
            return Err("prescribed skin work overflow");
        }
        Ok(work)
    }
    /// Physical load on the prescribed rig, as resultant N and moment N m.
    /// Reference/base coordinates must correspond to this force evaluation.
    /// # Errors
    /// Rejects invalid positions, sizes, origin or overflowing wrench.
    pub fn rig_wrench_about(
        &self,
        reference: &[Point],
        base: &[Point],
        origin: Point,
    ) -> Result<(Point, Point), &'static str> {
        if reference.len() != self.reference.len()
            || base.len() != self.base.len()
            || reference
                .iter()
                .chain(base)
                .flatten()
                .chain(origin.iter())
                .any(|x| !x.is_finite())
        {
            return Err("invalid skin wrench geometry");
        }
        let mut resultant = [0.; 3];
        let mut moment = [0.; 3];
        for (f, p) in self
            .reference
            .iter()
            .zip(reference)
            .chain(self.base.iter().zip(base))
        {
            let r = sub(*p, origin);
            let torque = [
                r[1] * f[2] - r[2] * f[1],
                r[2] * f[0] - r[0] * f[2],
                r[0] * f[1] - r[1] * f[0],
            ];
            for i in 0..3 {
                resultant[i] += f[i];
                moment[i] += torque[i];
            }
        }
        if resultant.iter().chain(&moment).any(|x| !x.is_finite()) {
            return Err("skin wrench overflow");
        }
        Ok((resultant, moment))
    }
}
impl EmbeddedSurface {
    /// All render vertices must lie inside the tetrahedral mesh (boundary allowed).
    /// Shared-face ties select the first cell. No extrapolation or nearest-cell fallback.
    /// # Errors
    /// Rejects nonfinite positions, invalid or degenerate cells, and exterior vertices.
    pub fn bind(
        rest: &[Point],
        cells: &[[usize; 4]],
        surface: &[Point],
    ) -> Result<Self, &'static str> {
        Self::bind_relative(rest, cells, surface, &vec![true; surface.len()])
    }
    /// Bind the tissue-owned vertices of a mixed skeletal/tissue surface.
    /// False entries retain their prescribed base pose and transfer no force to
    /// this tissue. True entries must be contained; exterior extrapolation is
    /// rejected. Ownership is authored and immutable, not inferred during motion.
    /// Absolute deformation needs a base pose; use `deform_relative_into`.
    /// # Errors
    /// Invalid ownership size, geometry, cells or exterior tissue-owned vertices.
    pub fn bind_relative(
        rest: &[Point],
        cells: &[[usize; 4]],
        surface: &[Point],
        tissue_owned: &[bool],
    ) -> Result<Self, &'static str> {
        if tissue_owned.len() != surface.len() {
            return Err("invalid relative embedding ownership");
        }
        if rest.is_empty()
            || cells.is_empty()
            || rest.iter().chain(surface).flatten().any(|x| !x.is_finite())
        {
            return Err("invalid embedding positions");
        }
        TetrahedralEmbedding::new(rest, cells)?.bind_relative(surface, tissue_owned)
    }
    pub(crate) fn tissue_owned_vertex(&self, vertex: usize) -> bool {
        self.bindings[vertex].is_some()
    }
    pub(crate) fn reference_geometry(&self) -> (&[Point], &[[usize; 4]], &[Point]) {
        (&self.rest, &self.cells, &self.surface)
    }
    /// Immutable sparse displacement map for resident renderers. Prescribed
    /// vertices have no tissue coefficients; no simulation state is exported.
    pub fn displacement_bindings(&self) -> impl ExactSizeIterator<Item = Option<([usize; 4], [f64; 4])>> + '_ {
        self.bindings.iter().map(|b| b.as_ref().map(|b| (b.indices, b.weights)))
    }
    /// Produces new render positions; caller-owned buffers remain unchanged on failure.
    /// Vertex order/count must match rest geometry. Normals must be recomputed by renderer.
    /// # Errors
    /// Rejects a changed vertex count, nonfinite positions, or arithmetic overflow.
    pub fn deform(&self, positions: &[Point]) -> Result<Vec<Point>, &'static str> {
        let mut output = vec![[0.0; 3]; self.bindings.len()];
        self.deform_into(positions, &mut output)?;
        Ok(output)
    }
    /// Updates an existing render buffer without allocating. Two passes ensure atomicity.
    /// # Errors
    /// Rejects wrong buffer sizes, nonfinite simulation positions or arithmetic overflow.
    /// The output remains unchanged on any error.
    pub fn deform_into(
        &self,
        positions: &[Point],
        output: &mut [Point],
    ) -> Result<(), &'static str> {
        if self.bindings.iter().any(Option::is_none) {
            return Err("mixed embedding requires prescribed base pose");
        }
        self.deform_displacements_into(positions, output)
    }
    /// Apply the linear tissue-displacement map W, with zero at prescribed vertices.
    /// This also maps trial directions for contact preconditioning; it does not
    /// supply absolute world positions for a mixed surface.
    /// # Errors
    /// Incompatible/nonfinite input or arithmetic overflow; output stays unchanged.
    pub fn deform_displacements(
        &self,
        displacements: &[Point],
    ) -> Result<Vec<Point>, &'static str> {
        let mut output = vec![[0.; 3]; self.bindings.len()];
        self.deform_displacements_into(displacements, &mut output)?;
        Ok(output)
    }
    fn deform_displacements_into(
        &self,
        positions: &[Point],
        output: &mut [Point],
    ) -> Result<(), &'static str> {
        if positions.len() != self.vertex_count || output.len() != self.bindings.len() {
            return Err("invalid deformed embedding positions");
        }
        let mut may_overflow = false;
        for &v in positions.iter().flatten() {
            if !v.is_finite() {
                return Err("invalid deformed embedding positions");
            }
            if v.abs() > 1e300 {
                may_overflow = true;
            }
        }
        let evaluate = |binding: &Option<Binding>| -> Point {
            let Some(binding) = binding else {
                return [0.; 3];
            };
            std::array::from_fn(|axis| {
                (0..4)
                    .map(|i| positions[binding.indices[i]][axis] * binding.weights[i])
                    .sum()
            })
        };
        if may_overflow {
            for binding in &self.bindings {
                if evaluate(binding).iter().any(|x| !x.is_finite()) {
                    return Err("embedding overflow");
                }
            }
        }
        for (binding, point) in self.bindings.iter().zip(output) {
            *point = evaluate(binding);
        }
        Ok(())
    }
    /// Adds embedded tissue displacement to an already posed render surface.
    /// `reference` is the simulation mesh transported by the same skeletal pose;
    /// `positions` is its physical state. Both retain the bind-time vertex order.
    /// The skeletal transformation must already be present in `surface_reference`.
    /// No extrapolation or closest-cell substitution is performed.
    /// # Errors
    /// Rejects incompatible buffers, nonfinite input and overflow atomically.
    pub fn deform_relative_into(
        &self,
        reference: &[Point],
        positions: &[Point],
        surface_reference: &[Point],
        output: &mut [Point],
    ) -> Result<(), &'static str> {
        if reference.len() != self.vertex_count
            || positions.len() != self.vertex_count
            || surface_reference.len() != self.bindings.len()
            || output.len() != self.bindings.len()
        {
            return Err("invalid relative embedding positions");
        }
        let mut may_overflow = false;
        for &x in reference
            .iter()
            .chain(positions)
            .chain(surface_reference)
            .flatten()
        {
            if !x.is_finite() {
                return Err("invalid relative embedding positions");
            }
            if x.abs() > 1e300 {
                may_overflow = true;
            }
        }
        let evaluate = |binding: &Option<Binding>, base: Point| -> Point {
            let Some(binding) = binding else {
                return base;
            };
            std::array::from_fn(|axis| {
                let displacement: f64 = (0..4)
                    .map(|i| {
                        let node = binding.indices[i];
                        (positions[node][axis] - reference[node][axis]) * binding.weights[i]
                    })
                    .sum();
                base[axis] + displacement
            })
        };
        if may_overflow {
            for (binding, &base) in self.bindings.iter().zip(surface_reference) {
                if evaluate(binding, base).iter().any(|x| !x.is_finite()) {
                    return Err("relative embedding overflow");
                }
            }
        }
        for ((binding, &base), point) in self.bindings.iter().zip(surface_reference).zip(output) {
            *point = evaluate(binding, base);
        }
        Ok(())
    }
    /// Adds surface forces to simulation nodes using the transpose of the
    /// displacement embedding. For fixed binding and skeletal reference this
    /// preserves virtual work: f_surface dot dx_surface = f_nodes dot dx_nodes.
    /// Existing nodal forces are retained. Every surface force is applied once.
    /// This does not compute contact forces or the reaction on a moving rig.
    /// # Errors
    /// Rejects incompatible buffers, nonfinite forces and overflow. The nodal
    /// buffer remains unchanged on any failure, including late accumulation overflow.
    pub fn accumulate_forces_into(
        &self,
        surface_forces: &[Point],
        nodal_forces: &mut [Point],
    ) -> Result<(), &'static str> {
        if surface_forces.len() != self.bindings.len() || nodal_forces.len() != self.vertex_count {
            return Err("invalid embedded surface forces");
        }
        let mut may_overflow = false;
        for &x in surface_forces.iter().chain(nodal_forces.iter()).flatten() {
            if !x.is_finite() {
                return Err("invalid embedded surface forces");
            }
            if x.abs() > 1e290 {
                may_overflow = true;
            }
        }
        if may_overflow {
            // Shared nodes receive several contributions; a private candidate makes
            // publication atomic even when only the final contribution overflows.
            let mut candidate = nodal_forces.to_vec();
            for (binding, force) in self.bindings.iter().zip(surface_forces) {
                let Some(binding) = binding else {
                    continue;
                };
                for (&node, &weight) in binding.indices.iter().zip(&binding.weights) {
                    for (value, &component) in candidate[node].iter_mut().zip(force) {
                        *value = weight.mul_add(component, *value);
                        if !value.is_finite() {
                            return Err("embedded force accumulation overflow");
                        }
                    }
                }
            }
            nodal_forces.copy_from_slice(&candidate);
        } else {
            for (binding, force) in self.bindings.iter().zip(surface_forces) {
                let Some(binding) = binding else {
                    continue;
                };
                for (&node, &weight) in binding.indices.iter().zip(&binding.weights) {
                    for (value, &component) in nodal_forces[node].iter_mut().zip(force) {
                        *value = weight.mul_add(component, *value);
                    }
                }
            }
        }
        Ok(())
    }
    /// Complete force chain rule for relative skin composition. Reference loads
    /// are -W^T f, base loads are f, and mechanical nodal loads are W^T f.
    /// # Errors
    /// Rejects incompatible/nonfinite forces or overflowing transferred loads.
    pub fn relative_loads(
        &self,
        surface_forces: &[Point],
    ) -> Result<RelativeSurfaceLoads, &'static str> {
        self.relative_loads_owned(surface_forces.to_vec())
    }
    pub(crate) fn vertex_count(&self) -> usize {
        self.vertex_count
    }
    pub(crate) fn relative_loads_owned(
        &self,
        surface_forces: Vec<Point>,
    ) -> Result<RelativeSurfaceLoads, &'static str> {
        let mut nodal = vec![[0.; 3]; self.vertex_count];
        self.accumulate_forces_into(&surface_forces, &mut nodal)?;
        let reference = nodal.iter().map(|p| p.map(|x| -x)).collect();
        Ok(RelativeSurfaceLoads {
            nodal,
            reference,
            base: surface_forces,
        })
    }
}
