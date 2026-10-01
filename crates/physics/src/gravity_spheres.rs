//! Finite solid spheres with mutual gravity and continuous collision detection.
//! Gravity uses kick-drift-kick splitting; during each drift, contacts are solved
//! at their analytic time of impact. Use a fixed step small enough for the orbit.
use crate::gravity::{Body, Error as GravityError, Gravity};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sphere {
    pub body: Body,
    pub radius: f64,
    pub angular_velocity: [f64; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Simulation {
    pub gravity: Gravity,
    /// Coefficient in [0, 1]; one is an elastic contact.
    pub restitution: f64,
    /// Coulomb friction coefficient, nonnegative.
    pub friction: f64,
    /// Maximum fixed substep used to resolve changing gravitational forces.
    pub max_step: f64,
    /// Limits work; exhaustion returns an error without publishing state.
    pub max_substeps: usize,
    pub max_contacts: usize,
}

impl Default for Simulation {
    fn default() -> Self {
        Self {
            gravity: Gravity::default(),
            restitution: 1.0,
            friction: 0.0,
            max_step: 1.0 / 240.0,
            max_substeps: 4096,
            max_contacts: 4096,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidInput,
    Gravity(GravityError),
    BudgetExceeded,
    NumericalOverflow,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StepReport {
    pub substeps: usize,
    pub contacts: usize,
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.into_iter().zip(b).map(|(x, y)| x * y).sum()
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|k| a[k] - b[k])
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn finite(spheres: &[Sphere]) -> bool {
    spheres.iter().all(|s| {
        s.body
            .position
            .iter()
            .chain(&s.body.velocity)
            .chain(&s.angular_velocity)
            .all(|v| v.is_finite())
    })
}

impl Simulation {
    /// Advances finite, freely moving spheres. Equal and opposite impulses
    /// include frictional spin for a homogeneous solid sphere (I = 2mr²/5).
    /// # Errors
    /// Invalid parameters, singular gravity, numerical overflow, and exhausted
    /// contact/substep budgets leave the original spheres unchanged.
    pub fn step(self, spheres: &mut [Sphere], dt: f64) -> Result<StepReport, Error> {
        if !dt.is_finite()
            || dt <= 0.0
            || !self.max_step.is_finite()
            || self.max_step <= 0.0
            || !self.restitution.is_finite()
            || !(0.0..=1.0).contains(&self.restitution)
            || !self.friction.is_finite()
            || self.friction < 0.0
            || spheres.iter().any(|s| {
                !s.radius.is_finite()
                    || s.radius <= 0.0
                    || !s.body.mass.is_finite()
                    || s.body.mass <= 0.0
            })
            || !finite(spheres)
        {
            return Err(Error::InvalidInput);
        }
        let mut next = spheres.to_vec();
        let mut report = StepReport::default();
        // Resolve authored overlaps before evaluating point-mass gravity.
        self.separate(&mut next, &mut report)?;
        let mut remaining = dt;
        while remaining > 0.0 {
            if report.substeps == self.max_substeps {
                return Err(Error::BudgetExceeded);
            }
            let h = remaining.min(self.max_step);
            self.kick(&mut next, h * 0.5)?;
            self.drift(&mut next, h, &mut report)?;
            self.kick(&mut next, h * 0.5)?;
            report.substeps += 1;
            remaining -= h;
        }
        if !finite(&next) {
            return Err(Error::NumericalOverflow);
        }
        spheres.copy_from_slice(&next);
        Ok(report)
    }

    fn kick(self, spheres: &mut [Sphere], dt: f64) -> Result<(), Error> {
        let bodies: Vec<_> = spheres.iter().map(|s| s.body).collect();
        let acceleration = self
            .gravity
            .accelerations(&bodies)
            .map_err(Error::Gravity)?;
        for (s, a) in spheres.iter_mut().zip(acceleration) {
            for (k, v) in a.iter().enumerate() {
                s.body.velocity[k] += v * dt;
            }
        }
        if !finite(spheres) {
            return Err(Error::NumericalOverflow);
        }
        Ok(())
    }

    fn spend(self, report: &mut StepReport) -> Result<(), Error> {
        if report.contacts >= self.max_contacts {
            return Err(Error::BudgetExceeded);
        }
        report.contacts += 1;
        Ok(())
    }

    fn separate(self, spheres: &mut [Sphere], report: &mut StepReport) -> Result<(), Error> {
        let mut had_overlap = false;
        loop {
            let mut corrections = Vec::new();
            let mut degree = vec![0usize; spheres.len()];
            for i in 0..spheres.len() {
                for j in i + 1..spheres.len() {
                    let delta = sub(spheres[j].body.position, spheres[i].body.position);
                    let distance = dot(delta, delta).sqrt();
                    let radius = spheres[i].radius + spheres[j].radius;
                    if !distance.is_finite() || !radius.is_finite() {
                        return Err(Error::NumericalOverflow);
                    }
                    let penetration = radius - distance;
                    if penetration <= radius * 1e-12 {
                        continue;
                    }
                    self.spend(report)?;
                    let normal = if distance == 0.0 {
                        // Relative motion supplies a permutation-equivariant axis.
                        // Identical stationary centres have no physical separating
                        // direction; reject rather than use the array index.
                        let relative = sub(spheres[i].body.velocity, spheres[j].body.velocity);
                        let speed = dot(relative, relative).sqrt();
                        if speed == 0.0 {
                            return Err(Error::InvalidInput);
                        }
                        if !speed.is_finite() {
                            return Err(Error::NumericalOverflow);
                        }
                        relative.map(|v| v / speed)
                    } else {
                        delta.map(|v| v / distance)
                    };
                    corrections.push((i, j, normal, penetration));
                    degree[i] += 1;
                    degree[j] += 1;
                }
            }
            if corrections.is_empty() {
                break;
            }
            had_overlap = true;
            let relaxation = 1.0 / (*degree.iter().max().unwrap_or(&1)).max(1) as f64;
            let mut offsets = vec![[0.0; 3]; spheres.len()];
            for (i, j, normal, penetration) in corrections {
                let inv_a = 1.0 / spheres[i].body.mass;
                let inv_b = 1.0 / spheres[j].body.mass;
                let total = inv_a + inv_b;
                for k in 0..3 {
                    offsets[i][k] -= normal[k] * penetration * relaxation * (inv_a / total);
                    offsets[j][k] += normal[k] * penetration * relaxation * (inv_b / total);
                }
            }
            for (sphere, offset) in spheres.iter_mut().zip(offsets) {
                for k in 0..3 {
                    sphere.body.position[k] += offset[k];
                }
            }
            if !finite(spheres) {
                return Err(Error::NumericalOverflow);
            }
        }
        if had_overlap {
            // Apply collision response once at the repaired geometry, rather
            // than repeating restitution during positional iteration.
            for i in 0..spheres.len() {
                for j in i + 1..spheres.len() {
                    let delta = sub(spheres[j].body.position, spheres[i].body.position);
                    if dot(delta, delta).sqrt()
                        <= (spheres[i].radius + spheres[j].radius) * (1.0 + 1e-12)
                    {
                        return self.contact_group(spheres, (i, j), report);
                    }
                }
            }
        }
        Ok(())
    }

    #[allow(clippy::many_single_char_names)] // Standard quadratic time-of-impact coefficients.
    fn drift(
        self,
        spheres: &mut [Sphere],
        mut remaining: f64,
        report: &mut StepReport,
    ) -> Result<(), Error> {
        // Keep the scale from the start of this drift: a settled inelastic
        // group may retain a sub-tolerance closing velocity after iteration.
        let velocity_tolerance = 1e-12
            * spheres
                .iter()
                .map(|s| dot(s.body.velocity, s.body.velocity).sqrt())
                .fold(0.0, f64::max);
        while remaining > 0.0 {
            let mut earliest = None;
            for i in 0..spheres.len() {
                for j in i + 1..spheres.len() {
                    let p = sub(spheres[j].body.position, spheres[i].body.position);
                    let v = sub(spheres[j].body.velocity, spheres[i].body.velocity);
                    let radius = spheres[i].radius + spheres[j].radius;
                    let a = dot(v, v);
                    let b = dot(p, v);
                    let c = dot(p, p) - radius * radius;
                    if !a.is_finite() || !b.is_finite() || !c.is_finite() {
                        return Err(Error::NumericalOverflow);
                    }
                    if a == 0.0 || b >= 0.0 {
                        continue;
                    }
                    if c.abs() <= radius * radius * 1e-12 && -b <= radius * velocity_tolerance {
                        continue;
                    }
                    let discriminant = b * b - a * c;
                    if !discriminant.is_finite() {
                        return Err(Error::NumericalOverflow);
                    }
                    if discriminant <= 0.0 {
                        continue;
                    }
                    // Stable smaller root; touching and closing pairs hit at t=0.
                    let time = if c <= 0.0 {
                        0.0
                    } else {
                        c / (-b + discriminant.sqrt())
                    };
                    if time <= remaining && earliest.is_none_or(|(t, _, _)| time < t) {
                        earliest = Some((time, i, j));
                    }
                }
            }
            let h = earliest.map_or(remaining, |(t, _, _)| t);
            for s in spheres.iter_mut() {
                for k in 0..3 {
                    s.body.position[k] += s.body.velocity[k] * h;
                }
            }
            if !finite(spheres) {
                return Err(Error::NumericalOverflow);
            }
            let Some((_, i, j)) = earliest else {
                return Ok(());
            };
            self.contact_group(spheres, (i, j), report)?;
            if !finite(spheres) {
                return Err(Error::NumericalOverflow);
            }
            remaining -= h;
        }
        Ok(())
    }

    /// Jacobi impulse iteration: every pair reads the same iteration state.
    /// Restitution targets are frozen at impact, so repeated iterations do not
    /// repeatedly apply restitution. Touching, initially stationary neighbours
    /// participate as well, allowing an impulse to propagate through a chain.
    fn contact_group(
        self,
        spheres: &mut [Sphere],
        hit: (usize, usize),
        report: &mut StepReport,
    ) -> Result<(), Error> {
        let mut pairs = Vec::new();
        let mut degree = vec![0usize; spheres.len()];
        let mut scale: f64 = 0.0;
        for i in 0..spheres.len() {
            for j in i + 1..spheres.len() {
                let delta = sub(spheres[j].body.position, spheres[i].body.position);
                let distance = dot(delta, delta).sqrt();
                let radius = spheres[i].radius + spheres[j].radius;
                if (i, j) != hit && distance > radius * (1.0 + 1e-12) {
                    continue;
                }
                if distance == 0.0 || !distance.is_finite() {
                    return Err(Error::NumericalOverflow);
                }
                let normal = delta.map(|v| v / distance);
                let relative = Self::relative(spheres[i], spheres[j], normal);
                let closing = dot(relative, normal);
                scale = scale.max(dot(relative, relative).sqrt());
                pairs.push((i, j, normal, -self.restitution * closing.min(0.0)));
                degree[i] += 1;
                degree[j] += 1;
            }
        }
        if pairs.len() == 1 {
            self.spend(report)?;
            self.contact(spheres, hit.0, hit.1, pairs[0].2);
            return Ok(());
        }
        let relaxation = 0.5 / (*degree.iter().max().unwrap_or(&1)).max(1) as f64;
        let tolerance = 1e-12 * scale.max(f64::MIN_POSITIVE);
        let mut impulses = vec![[0.0; 3]; pairs.len()];
        loop {
            let mut changes = vec![[0.0; 3]; pairs.len()];
            let mut maximum: f64 = 0.0;
            for (index, &(i, j, normal, target)) in pairs.iter().enumerate() {
                self.spend(report)?;
                let inverse_mass = 1.0 / spheres[i].body.mass + 1.0 / spheres[j].body.mass;
                let relative = Self::relative(spheres[i], spheres[j], normal);
                let closing = dot(relative, normal);
                let old_normal = dot(impulses[index], normal);
                let new_normal =
                    (old_normal + relaxation * (target - closing) / inverse_mass).max(0.0);
                let mut tangent: [f64; 3] = std::array::from_fn(|k| {
                    impulses[index][k]
                        - old_normal * normal[k]
                        - relaxation * (relative[k] - closing * normal[k]) / (3.5 * inverse_mass)
                });
                let length = dot(tangent, tangent).sqrt();
                let limit = self.friction * new_normal;
                if length > limit {
                    tangent = tangent.map(|v| v * (limit / length));
                }
                let next: [f64; 3] = std::array::from_fn(|k| normal[k] * new_normal + tangent[k]);
                changes[index] = sub(next, impulses[index]);
                maximum = maximum
                    .max(dot(changes[index], changes[index]).sqrt() * inverse_mass / relaxation);
                impulses[index] = next;
            }
            // Sum all changes before publishing an iteration to the bodies.
            let mut velocity = vec![[0.0; 3]; spheres.len()];
            let mut spin = velocity.clone();
            for (&(i, j, normal, _), impulse) in pairs.iter().zip(changes) {
                for (body, sign) in [(i, -1.0), (j, 1.0)] {
                    let inverse_mass = 1.0 / spheres[body].body.mass;
                    let radius = spheres[body].radius;
                    let arm = normal.map(|v| -sign * v * radius);
                    let torque = cross(arm, impulse);
                    for k in 0..3 {
                        velocity[body][k] += sign * impulse[k] * inverse_mass;
                        spin[body][k] += sign * torque[k] * 2.5 * inverse_mass / (radius * radius);
                    }
                }
            }
            for (index, sphere) in spheres.iter_mut().enumerate() {
                for k in 0..3 {
                    sphere.body.velocity[k] += velocity[index][k];
                    sphere.angular_velocity[k] += spin[index][k];
                }
            }
            if !finite(spheres) || !maximum.is_finite() {
                return Err(Error::NumericalOverflow);
            }
            if maximum <= tolerance {
                return Ok(());
            }
        }
    }

    fn relative(a: Sphere, b: Sphere, normal: [f64; 3]) -> [f64; 3] {
        let spin_a = cross(a.angular_velocity, normal.map(|v| v * a.radius));
        let spin_b = cross(b.angular_velocity, normal.map(|v| -v * b.radius));
        std::array::from_fn(|k| b.body.velocity[k] + spin_b[k] - a.body.velocity[k] - spin_a[k])
    }

    fn contact(self, spheres: &mut [Sphere], i: usize, j: usize, normal: [f64; 3]) {
        let a = spheres[i];
        let b = spheres[j];
        let inv_a = 1.0 / a.body.mass;
        let inv_b = 1.0 / b.body.mass;
        let inverse_mass = inv_a + inv_b;
        let ra = normal.map(|n| n * a.radius);
        let rb = normal.map(|n| -n * b.radius);
        let spin_a = cross(a.angular_velocity, ra);
        let spin_b = cross(b.angular_velocity, rb);
        let relative: [f64; 3] = std::array::from_fn(|k| {
            b.body.velocity[k] + spin_b[k] - a.body.velocity[k] - spin_a[k]
        });
        let closing = dot(relative, normal);
        if closing >= 0.0 {
            return;
        }
        let normal_impulse = -(1.0 + self.restitution) * closing / inverse_mass;
        let tangent = std::array::from_fn(|k| relative[k] - closing * normal[k]);
        let speed = dot(tangent, tangent).sqrt();
        let friction_impulse = (speed / (3.5 * inverse_mass)).min(self.friction * normal_impulse);
        let impulse: [f64; 3] = std::array::from_fn(|k| {
            normal[k] * normal_impulse
                - if speed > 0.0 {
                    tangent[k] / speed * friction_impulse
                } else {
                    0.0
                }
        });
        let torque_a = cross(ra, impulse);
        let torque_b = cross(rb, impulse);
        for k in 0..3 {
            spheres[i].body.velocity[k] -= impulse[k] * inv_a;
            spheres[j].body.velocity[k] += impulse[k] * inv_b;
            spheres[i].angular_velocity[k] -= torque_a[k] * 2.5 * inv_a / (a.radius * a.radius);
            spheres[j].angular_velocity[k] += torque_b[k] * 2.5 * inv_b / (b.radius * b.radius);
        }
    }
}
