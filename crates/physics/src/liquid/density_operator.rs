//! Immutable liquid density input and original-owner candidate admission.
use super::{Error, Formulation, Material, Particle, density_kernel, finite, norm, positive, sub};

/// A compute backend supplies candidates, never accepted physical state.
pub trait LiquidDensityBackend {
    fn candidate(&mut self, operator: &LiquidDensityOperator<'_>) -> Option<Vec<f64>>;
    /// Opt-in host profiling for native application backends.
    fn profile_pipeline_admission(&self) -> bool {
        false
    }
    fn supports_pipeline(&self) -> bool {
        false
    }
    fn pipeline_candidate(
        &mut self,
        _operator: &super::LiquidPipelineOperator<'_>,
    ) -> Option<super::LiquidPipelineResult> {
        None
    }
    fn supports_density_normal_pressure(&self) -> bool {
        false
    }
    fn supports_density_normals(&self) -> bool {
        false
    }
    fn density_normal_candidate(
        &mut self,
        _operator: &super::LiquidDensityNormalOperator<'_>,
    ) -> Option<super::LiquidDensityNormalResult> {
        None
    }
    fn supports_force_pipeline(&self) -> bool {
        false
    }
    fn force_pipeline_candidate(
        &mut self,
        _operator: &super::LiquidForcePipelineOperator<'_>,
    ) -> Option<super::LiquidForcePipelineResult> {
        None
    }
    fn supports_accumulation(&self) -> bool {
        false
    }
    fn accumulation_candidate(
        &mut self,
        _operator: &super::LiquidAccumulationOperator<'_>,
    ) -> Option<super::LiquidAccumulationResult> {
        None
    }
    fn supports_normals(&self) -> bool {
        false
    }
    fn normal_candidates(
        &mut self,
        _operator: &super::LiquidNormalOperator<'_>,
    ) -> Option<Vec<[f64; 3]>> {
        None
    }
    fn supports_pair_forces(&self) -> bool {
        false
    }
    /// Optional whole-batch fluid-fluid force candidates. Physics retains geometry,
    /// color normals, ordered accumulation and all non-pair contributions.
    fn pair_force_candidates(
        &mut self,
        _inputs: &[super::LiquidPairForceInput],
    ) -> Option<Vec<super::LiquidPairForceResult>> {
        None
    }
}
impl<F: FnMut(&LiquidDensityOperator<'_>) -> Option<Vec<f64>>> LiquidDensityBackend for F {
    fn candidate(&mut self, operator: &LiquidDensityOperator<'_>) -> Option<Vec<f64>> {
        self(operator)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LiquidDensityStats {
    pub calls: usize,
    pub admitted: usize,
    pub native_fallbacks: usize,
    /// Original whole-chain reference admission host time; excludes backend work.
    pub pipeline_admission_ns: u128,
    pub pipeline_calls: usize,
    pub pipeline_admitted: usize,
    pub pipeline_native_fallbacks: usize,
    pub density_normal_pressure_admitted: usize,
    pub density_normal_calls: usize,
    pub density_normal_admitted: usize,
    pub density_normal_native_fallbacks: usize,
    pub force_pipeline_calls: usize,
    pub force_pipeline_admitted: usize,
    pub force_pipeline_native_fallbacks: usize,
    pub accumulation_calls: usize,
    pub accumulation_admitted: usize,
    pub accumulation_native_fallbacks: usize,
    pub normal_calls: usize,
    pub normal_admitted: usize,
    pub normal_native_fallbacks: usize,
    pub pair_force_calls: usize,
    pub pair_force_admitted: usize,
    pub pair_force_native_fallbacks: usize,
}
pub(super) struct NativeDensity;
impl LiquidDensityBackend for NativeDensity {
    fn candidate(&mut self, _: &LiquidDensityOperator<'_>) -> Option<Vec<f64>> {
        None
    }
}

impl super::Liquid {
    /// Transactional ordinary step with admitted density candidates. Forces,
    /// boundaries, images, transport and adaptive time stepping retain ownership.
    /// Tolerances are chosen by the caller/physical policy, not by the backend.
    pub fn step_with_density_backend(
        &mut self,
        dt: f64,
        container: Option<super::Container>,
        backend: &mut impl LiquidDensityBackend,
        absolute: f64,
        relative: f64,
    ) -> Result<(super::StepStats, LiquidDensityStats), Error> {
        self.step_with_compute_backend(dt, container, backend, absolute, relative)
    }

    /// Transactional density and pair force candidates, admitted in whole batches.
    pub fn step_with_compute_backend(
        &mut self,
        dt: f64,
        container: Option<super::Container>,
        backend: &mut impl LiquidDensityBackend,
        absolute: f64,
        relative: f64,
    ) -> Result<(super::StepStats, LiquidDensityStats), Error> {
        if !absolute.is_finite() || absolute < 0. || !relative.is_finite() || relative < 0. {
            return Err(Error::InvalidConfig);
        }
        let mut counts = LiquidDensityStats::default();
        let mut checked = CheckedBackend {
            backend,
            counts: &mut counts,
            absolute,
            relative,
        };
        let mut state = self.clone();
        let stats = state.advance_with_mover_inner(
            dt,
            container,
            |state, particles, time| {
                for particle in particles {
                    for axis in 0..3 {
                        particle.position[axis] += particle.velocity[axis] * time;
                    }
                }
                if state.boundary_coupling.is_some() {
                    state.drift_boundary_body(time)?;
                }
                Ok(())
            },
            &mut checked,
        )?;
        *self = state;
        Ok((stats, counts))
    }
}

/// Fluid-particle density before boundary/image contributions. Backends borrow
/// current inputs; they cannot mutate particles, material fields or pair order.
#[derive(Debug)]
pub struct LiquidDensityOperator<'a> {
    pub particles: &'a [Particle],
    pub pairs: &'a [(usize, usize)],
    pub properties: &'a [Material],
    pub formulation: Formulation,
    pub smoothing_radius: f64,
    pub carrier_droplets: Option<&'a [bool]>,
}

impl LiquidDensityOperator<'_> {
    /// Validate transport inputs before a backend is allowed to read them.
    pub fn validate(&self) -> Result<(), Error> {
        if !positive(self.smoothing_radius)
            || self.properties.len() != self.particles.len()
            || self
                .carrier_droplets
                .is_some_and(|flags| flags.len() != self.particles.len())
        {
            return Err(Error::InvalidConfig);
        }
        for (particle, material) in self.particles.iter().zip(self.properties) {
            if !finite(particle.position) || !positive(particle.mass) {
                return Err(Error::InvalidParticle);
            }
            if !positive(material.rest_density) {
                return Err(Error::InvalidMaterial);
            }
        }
        let mut seen = std::collections::HashSet::with_capacity(self.pairs.len());
        for &(i, j) in self.pairs {
            if i >= j || j >= self.particles.len() || !seen.insert((i, j)) {
                return Err(Error::InvalidParticle);
            }
        }
        Ok(())
    }

    /// Original f64 formula and accumulation order, also used by native stepping.
    pub fn evaluate_native(&self) -> Result<Vec<f64>, Error> {
        self.validate()?;
        let density = self.evaluate_validated();
        if density.iter().any(|&rho| !positive(rho)) {
            return Err(Error::NumericalFailure);
        }
        Ok(density)
    }

    /// Whole-vector admission against current original inputs. No partial result
    /// is published; a failed backend candidate leaves all physical state intact.
    pub fn admit(&self, candidate: &[f64], absolute: f64, relative: f64) -> Result<(), Error> {
        if !absolute.is_finite()
            || absolute < 0.
            || !relative.is_finite()
            || relative < 0.
            || candidate.len() != self.particles.len()
        {
            return Err(Error::InvalidConfig);
        }
        let reference = self.evaluate_native()?;
        for (&actual, expected) in candidate.iter().zip(reference) {
            let error = (actual - expected).abs();
            // Division avoids overflowing a tolerance product and accidentally
            // admitting every finite candidate on extreme valid inputs.
            if !positive(actual)
                || !error.is_finite()
                || (error > absolute && error / expected > relative)
            {
                return Err(Error::NumericalFailure);
            }
        }
        Ok(())
    }

    pub(super) fn evaluate_validated(&self) -> Vec<f64> {
        let h = self.smoothing_radius;
        let self_kernel = density_kernel(self.formulation, h, 0.0);
        let mut density: Vec<_> = self
            .particles
            .iter()
            .map(|p| p.mass * self_kernel)
            .collect();
        for &(i, j) in self.pairs {
            let r = norm(sub(self.particles[i].position, self.particles[j].position));
            let w = density_kernel(self.formulation, h, r);
            let ratio = match self.formulation {
                Formulation::MassDensity => 1.0,
                Formulation::RestVolume | Formulation::RestVolumeWendland => {
                    self.properties[i].rest_density / self.properties[j].rest_density
                }
            };
            density[i] += self.particles[j].mass * w * ratio;
            density[j] += self.particles[i].mass * w / ratio;
        }
        if let Some(flags) = self.carrier_droplets {
            for (i, rho) in density.iter_mut().enumerate() {
                if flags[i] {
                    *rho = self.properties[i].rest_density;
                }
            }
        }
        density
    }
}

struct CheckedBackend<'a, B> {
    backend: &'a mut B,
    counts: &'a mut LiquidDensityStats,
    absolute: f64,
    relative: f64,
}
impl<B: LiquidDensityBackend> LiquidDensityBackend for CheckedBackend<'_, B> {
    fn supports_pipeline(&self) -> bool {
        self.backend.supports_pipeline()
    }
    fn pipeline_candidate(
        &mut self,
        operator: &super::LiquidPipelineOperator<'_>,
    ) -> Option<super::LiquidPipelineResult> {
        self.counts.pipeline_calls += 1;
        if let Some(candidate) = self.backend.pipeline_candidate(operator) {
            let started = self
                .backend
                .profile_pipeline_admission()
                .then(std::time::Instant::now);
            let admitted = operator
                .admit(&candidate, self.absolute, self.relative)
                .is_ok();
            if let Some(started) = started {
                self.counts.pipeline_admission_ns += started.elapsed().as_nanos();
            }
            if admitted {
                self.counts.pipeline_admitted += 1;
                return Some(candidate);
            }
        }
        self.counts.pipeline_native_fallbacks += 1;
        None
    }
    fn supports_density_normal_pressure(&self) -> bool {
        self.backend.supports_density_normal_pressure()
    }
    fn supports_density_normals(&self) -> bool {
        self.backend.supports_density_normals()
    }
    fn density_normal_candidate(
        &mut self,
        operator: &super::LiquidDensityNormalOperator<'_>,
    ) -> Option<super::LiquidDensityNormalResult> {
        self.counts.density_normal_calls += 1;
        if let Some(candidate) = self.backend.density_normal_candidate(operator)
            && operator
                .admit(&candidate, self.absolute, self.relative)
                .is_ok()
        {
            self.counts.density_normal_admitted += 1;
            if operator.pressure_laws.is_some() {
                self.counts.density_normal_pressure_admitted += 1;
            }
            return Some(candidate);
        }
        self.counts.density_normal_native_fallbacks += 1;
        None
    }
    fn supports_force_pipeline(&self) -> bool {
        self.backend.supports_force_pipeline()
    }
    fn force_pipeline_candidate(
        &mut self,
        operator: &super::LiquidForcePipelineOperator<'_>,
    ) -> Option<super::LiquidForcePipelineResult> {
        self.counts.force_pipeline_calls += 1;
        if let Some(candidate) = self.backend.force_pipeline_candidate(operator)
            && operator
                .admit(&candidate, self.absolute, self.relative)
                .is_ok()
        {
            self.counts.force_pipeline_admitted += 1;
            return Some(candidate);
        }
        self.counts.force_pipeline_native_fallbacks += 1;
        None
    }
    fn supports_accumulation(&self) -> bool {
        self.backend.supports_accumulation()
    }
    fn accumulation_candidate(
        &mut self,
        operator: &super::LiquidAccumulationOperator<'_>,
    ) -> Option<super::LiquidAccumulationResult> {
        self.counts.accumulation_calls += 1;
        if let Some(candidate) = self.backend.accumulation_candidate(operator)
            && operator
                .admit(&candidate, self.absolute, self.relative)
                .is_ok()
        {
            self.counts.accumulation_admitted += 1;
            return Some(candidate);
        }
        self.counts.accumulation_native_fallbacks += 1;
        None
    }
    fn supports_normals(&self) -> bool {
        self.backend.supports_normals()
    }
    fn normal_candidates(
        &mut self,
        operator: &super::LiquidNormalOperator<'_>,
    ) -> Option<Vec<[f64; 3]>> {
        self.counts.normal_calls += 1;
        if let Some(candidate) = self.backend.normal_candidates(operator)
            && operator
                .admit(&candidate, self.absolute, self.relative)
                .is_ok()
        {
            self.counts.normal_admitted += 1;
            return Some(candidate);
        }
        self.counts.normal_native_fallbacks += 1;
        None
    }
    fn supports_pair_forces(&self) -> bool {
        self.backend.supports_pair_forces()
    }
    fn candidate(&mut self, operator: &LiquidDensityOperator<'_>) -> Option<Vec<f64>> {
        self.counts.calls += 1;
        if let Some(candidate) = self.backend.candidate(operator)
            && operator
                .admit(&candidate, self.absolute, self.relative)
                .is_ok()
        {
            self.counts.admitted += 1;
            return Some(candidate);
        }
        self.counts.native_fallbacks += 1;
        None
    }
    fn pair_force_candidates(
        &mut self,
        inputs: &[super::LiquidPairForceInput],
    ) -> Option<Vec<super::LiquidPairForceResult>> {
        self.counts.pair_force_calls += 1;
        if let Some(candidate) = self.backend.pair_force_candidates(inputs)
            && candidate.len() == inputs.len()
            && inputs
                .iter()
                .zip(&candidate)
                .all(|(input, &result)| input.admit(result, self.absolute, self.relative).is_ok())
        {
            self.counts.pair_force_admitted += 1;
            return Some(candidate);
        }
        self.counts.pair_force_native_fallbacks += 1;
        None
    }
}
