//! Shared mass-metric normal projection after mesh and strand friction.
use super::{ContactSource, HairRod, StrandResponse};
use crate::hair::math::*;

#[derive(Clone, Copy)]
struct Entry {
    rod: usize,
    point: usize,
    gradient: V,
    mobility: f64,
}
struct Constraint {
    entries: [Entry; 4],
    bound: f64,
    diagonal: f64,
    multiplier: f64,
    response: Vec<Response>,
}
struct Response {
    rod: usize,
    linear: Vec<V>,
    angular: Vec<V>,
}
// One contact operator acts on either velocities or positional increments.
// This keeps compliance, active-set logic and fixed-root ownership identical.
trait ProjectionVector {
    fn linear(&self,rod:usize,point:usize)->V;
    fn add_linear(&mut self,rod:usize,point:usize,change:V);
    fn add_angular(&mut self,rod:usize,point:usize,change:V);
    fn shape(&self)->Vec<usize>;
    fn finite(&self)->bool;
}
impl ProjectionVector for [HairRod] {
    fn linear(&self,r:usize,p:usize)->V {self[r].velocity[p]}
    fn add_linear(&mut self,r:usize,p:usize,v:V) {self[r].velocity[p]=add(self[r].velocity[p],v);}
    fn add_angular(&mut self,r:usize,p:usize,v:V) {self[r].omega[p]=add(self[r].omega[p],v);}
    fn shape(&self)->Vec<usize> {self.iter().map(|rod|rod.x.len()).collect()}
    fn finite(&self)->bool {self.iter().all(|rod|rod.velocity.iter().chain(&rod.omega).all(|v|finite(*v)))}
}
struct PositionIncrement {linear:Vec<Vec<V>>,angular:Vec<Vec<V>>}
impl ProjectionVector for PositionIncrement {
    fn linear(&self,r:usize,p:usize)->V {self.linear[r][p]}
    fn add_linear(&mut self,r:usize,p:usize,v:V) {self.linear[r][p]=add(self.linear[r][p],v);}
    fn add_angular(&mut self,r:usize,p:usize,v:V) {self.angular[r][p]=add(self.angular[r][p],v);}
    fn shape(&self)->Vec<usize> {self.linear.iter().map(Vec::len).collect()}
    fn finite(&self)->bool {self.linear.iter().chain(&self.angular).all(|points|points.iter().all(|v|finite(*v)))}
}
impl Constraint {
    fn speed<S:ProjectionVector+?Sized>(&self, rods: &S) -> f64 {
        self.entries
            .iter()
            .map(|entry| dot(entry.gradient, rods.linear(entry.rod,entry.point)))
            .sum()
    }
    fn residual<S:ProjectionVector+?Sized>(&self, rods: &S) -> f64 {
        let gap = self.speed(rods) - self.bound;
        if self.multiplier > 0. {
            gap.abs()
        } else {
            (-gap).max(0.)
        }
    }
    fn apply<S:ProjectionVector+?Sized>(&self, rods: &mut S, impulse: f64) {
        if self.response.is_empty() {
            for entry in self.entries {
                rods.add_linear(entry.rod,entry.point,mul(entry.gradient,entry.mobility*impulse));
            }
        } else {
            for response in &self.response {
                for (point,change) in response.linear.iter().enumerate() {
                    rods.add_linear(response.rod,point,mul(*change,impulse));
                }
                for (point,change) in response.angular.iter().enumerate() {
                    rods.add_angular(response.rod,point,mul(*change,impulse));
                }
            }
        }
    }
}
fn entries(rod: usize, segment: usize, fraction: f64, normal: V, hair: &HairRod) -> [Entry; 2] {
    std::array::from_fn(|end| {
        let point = segment + end;
        Entry {
            rod,
            point,
            gradient: mul(normal, if end == 0 { 1. - fraction } else { fraction }),
            // Mobility belongs to a particle, not to an individual plane.
            // Otherwise shared contacts no longer have a symmetric Gram matrix.
            mobility: hair.inv_mass[point],
        }
    })
}
fn add_constraint(
    constraints: &mut Vec<Constraint>,
    entries: [Entry; 4],
    bound: f64,
) -> Result<(), &'static str> {
    let diagonal = entries
        .iter()
        .map(|entry| entry.mobility * dot(entry.gradient, entry.gradient))
        .sum::<f64>();
    if !diagonal.is_finite() || !bound.is_finite() {
        return Err("invalid shared contact velocity constraint");
    }
    if diagonal > 1e-30 {
        constraints.push(Constraint {
            entries,
            bound,
            diagonal,
            multiplier: 0.,
            response: Vec::new(),
        });
    }
    Ok(())
}
fn prepare_implicit_response(
    constraints: &mut [Constraint],
    rods: &[HairRod],
    dt: f64,
) -> Result<(), &'static str> {
    use crate::hair::direct;
    let mut required = vec![false; rods.len()];
    for constraint in constraints.iter() {
        for entry in constraint.entries {
            if dot(entry.gradient, entry.gradient) > 0. {
                required[entry.rod] = true;
            }
        }
    }
    let mut factors = Vec::with_capacity(rods.len());
    for (rod, required) in rods.iter().zip(required) {
        if !required {
            factors.push(None);
            continue;
        }
        let mut staged = rod.clone();
        staged.contacts.clear();
        let (mut matrix, mut rhs) = direct::assemble(&mut staged, dt)?;
        // H=M+dt^2 K couples translations and material-frame rotations.
        for value in &mut matrix {
            *value *= dt * dt;
        }
        rhs.fill(0.);
        let end = rhs.len() - 3;
        direct::cholesky(&mut matrix, &mut rhs, 6..end);
        if matrix.iter().any(|value| !value.is_finite()) {
            return Err("shared contact rod factor overflow");
        }
        factors.push(Some(matrix));
    }
    for constraint in constraints {
        let mut indices: Vec<_> = constraint
            .entries
            .iter()
            .filter(|entry| dot(entry.gradient, entry.gradient) > 0.)
            .map(|entry| entry.rod)
            .collect();
        indices.sort_unstable();
        indices.dedup();
        for index in indices {
            let rod = &rods[index];
            let n = rod.x.len();
            let mut force = vec![0.; n * 6];
            for entry in constraint.entries.iter().filter(|entry| entry.rod == index) {
                if entry.point == 0 {
                    continue;
                }
                for axis in 0..3 {
                    force[entry.point * 6 + axis] += entry.gradient[axis];
                }
            }
            direct::solve_factored(
                factors[index]
                    .as_ref()
                    .ok_or("missing shared contact rod factor")?,
                &mut force,
                6..n * 6 - 3,
            );
            if force.iter().any(|value| !value.is_finite()) {
                return Err("shared contact rod response overflow");
            }
            constraint.response.push(Response {
                rod: index,
                linear: (0..n)
                    .map(|i| std::array::from_fn(|axis| force[i * 6 + axis]))
                    .collect(),
                angular: (0..n - 1)
                    .map(|i| std::array::from_fn(|axis| force[i * 6 + 3 + axis]))
                    .collect(),
            });
        }
        constraint.diagonal = constraint
            .entries
            .iter()
            .map(|entry| {
                constraint
                    .response
                    .iter()
                    .find(|response| response.rod == entry.rod)
                    .map_or(0., |response| {
                        dot(entry.gradient, response.linear[entry.point])
                    })
            })
            .sum();
        if !constraint.diagonal.is_finite() || constraint.diagonal <= 0. {
            return Err("invalid shared contact rod compliance");
        }
    }
    Ok(())
}
fn accelerate_active_set<S:ProjectionVector+?Sized>(
    constraints: &mut [Constraint],
    rods: &mut S,
    tolerance: f64,
) -> Result<bool, &'static str> {
    let mut active: Vec<_> = constraints
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            (c.multiplier > 0. || c.bound - c.speed(rods) > tolerance).then_some(i)
        })
        .collect();
    let mut scratch: Vec<Vec<V>> = rods.shape().into_iter().map(|n|vec![[0.;3];n]).collect();
    // All constraints use the same particle mobility, so J M^-1 J^T is
    // symmetric positive semidefinite. Apply it without a dense Gram matrix.
    for _ in 0..32 {
        if active.is_empty() {
            return Ok(false);
        }
        let mut correction = vec![0.; active.len()];
        let mut residual: Vec<_> = active
            .iter()
            .map(|&i| constraints[i].bound - constraints[i].speed(rods))
            .collect();
        let mut preconditioned: Vec<_> = active
            .iter()
            .zip(&residual)
            .map(|(&i, &r)| r / constraints[i].diagonal)
            .collect();
        let mut direction = preconditioned.clone();
        let mut product = vec![0.; active.len()];
        let mut rz = residual
            .iter()
            .zip(&preconditioned)
            .map(|(r, z)| r * z)
            .sum::<f64>();
        let mut solved = false;
        for _ in 0..(active.len() * 2).min(512) {
            if residual.iter().all(|r| r.abs() <= tolerance * 0.1) {
                solved = true;
                break;
            }
            for points in &mut scratch {
                points.fill([0.; 3]);
            }
            for (&index, &value) in active.iter().zip(&direction) {
                if constraints[index].response.is_empty() {
                    for entry in constraints[index].entries {
                        let point = &mut scratch[entry.rod][entry.point];
                        *point = add(*point, mul(entry.gradient, entry.mobility * value));
                    }
                } else {
                    for response in &constraints[index].response {
                        for (point, change) in
                            scratch[response.rod].iter_mut().zip(&response.linear)
                        {
                            *point = add(*point, mul(*change, value));
                        }
                    }
                }
            }
            for (value, &index) in product.iter_mut().zip(&active) {
                *value = constraints[index]
                    .entries
                    .iter()
                    .map(|entry| dot(entry.gradient, scratch[entry.rod][entry.point]))
                    .sum();
            }
            let curvature = direction
                .iter()
                .zip(&product)
                .map(|(p, q)| p * q)
                .sum::<f64>();
            if !curvature.is_finite() || !rz.is_finite() {
                return Err("shared contact active solve overflow");
            }
            if curvature <= 0. || rz <= 0. {
                break;
            }
            let fraction = rz / curvature;
            for i in 0..active.len() {
                correction[i] += fraction * direction[i];
                residual[i] -= fraction * product[i];
                preconditioned[i] = residual[i] / constraints[active[i]].diagonal;
            }
            let next_rz = residual
                .iter()
                .zip(&preconditioned)
                .map(|(r, z)| r * z)
                .sum::<f64>();
            if !next_rz.is_finite() || correction.iter().any(|value| !value.is_finite()) {
                return Err("shared contact active solve overflow");
            }
            let beta = next_rz / rz;
            for i in 0..active.len() {
                direction[i] = preconditioned[i] + beta * direction[i];
            }
            rz = next_rz;
        }
        if !solved && !residual.iter().all(|r| r.abs() <= tolerance * 0.1) {
            return Ok(false);
        }
        // A zero multiplier whose Newton direction points negative belongs
        // outside the active set. Remove it and resolve, rather than clipping
        // a coupled direction and losing its stationarity.
        if active
            .iter()
            .zip(&correction)
            .any(|(&i, &d)| constraints[i].multiplier == 0. && d < 0.)
        {
            active = active
                .into_iter()
                .zip(correction)
                .filter_map(|(i, d)| (constraints[i].multiplier > 0. || d >= 0.).then_some(i))
                .collect();
            continue;
        }
        let fraction = active
            .iter()
            .zip(&correction)
            .filter(|(_, d)| **d < 0.)
            .map(|(&i, &d)| constraints[i].multiplier / -d)
            .fold(1., f64::min);
        if fraction <= 0. || !fraction.is_finite() {
            return Ok(false);
        }
        for (&index, &delta) in active.iter().zip(&correction) {
            let constraint = &mut constraints[index];
            let next = (constraint.multiplier + fraction * delta).max(0.);
            let change = next - constraint.multiplier;
            constraint.multiplier = next;
            constraint.apply(rods, change);
        }
        return Ok(true);
    }
    Ok(false)
}
pub(in crate::hair) fn stabilize_contact_velocities(
    rods: &mut [HairRod],
    responses: &[StrandResponse],
    dt: f64,
    radius: f64,
) -> Result<(), &'static str> {
    if !dt.is_finite() || dt <= 0. || !radius.is_finite() || radius <= 0. {
        return Err("invalid shared contact velocity step");
    }
    let mut constraints = Vec::new();
    for (index, rod) in rods.iter().enumerate() {
        for contact in &rod.contacts {
            if !matches!(contact.source, ContactSource::Mesh(_)) {
                continue;
            }
            let i = contact.segment;
            let t = contact.fraction;
            let position = add(mul(rod.x[i], 1. - t), mul(rod.x[i + 1], t));
            let gap = dot(sub(position, contact.target), contact.normal);
            let bound = dot(contact.surface_velocity, contact.normal) + (-gap / dt).min(0.);
            let pair = entries(index, i, t, contact.normal, rod);
            let zero = Entry {
                rod: index,
                point: 0,
                gradient: [0.; 3],
                mobility: 0.,
            };
            add_constraint(&mut constraints, [pair[0], pair[1], zero, zero], bound)?;
        }
    }
    for response in responses {
        let (ra, ia, s) = response.a;
        let (rb, ib, t) = response.b;
        let pa = add(mul(rods[ra].x[ia], 1. - s), mul(rods[ra].x[ia + 1], s));
        let pb = add(mul(rods[rb].x[ib], 1. - t), mul(rods[rb].x[ib + 1], t));
        let gap = dot(sub(pa, pb), response.normal) - 2. * radius;
        let a = entries(ra, ia, s, response.normal, &rods[ra]);
        let b = entries(rb, ib, t, mul(response.normal, -1.), &rods[rb]);
        add_constraint(
            &mut constraints,
            [a[0], a[1], b[0], b[1]],
            (-gap / dt).min(0.),
        )?;
    }
    if constraints.is_empty() {
        return Ok(());
    }
    prepare_implicit_response(&mut constraints, rods, dt)?;
    solve_projection(&mut constraints,rods,1e-9)
}
pub(in crate::hair) fn reconcile_contact_positions(rods:&mut [HairRod],responses:&mut [StrandResponse],dt:f64,radius:f64)->Result<(), &'static str> {
    if !dt.is_finite() || dt<=0. || !radius.is_finite() || radius<=0. {return Err("invalid shared contact position step");}
    let mut constraints=Vec::new();
    for (index,rod) in rods.iter().enumerate() {
        for contact in &rod.contacts {
            if !matches!(contact.source,ContactSource::Mesh(_)) {continue;}
            let i=contact.segment;let t=contact.fraction;
            let position=add(mul(rod.x[i],1.-t),mul(rod.x[i+1],t));
            let gap=dot(sub(position,contact.target),contact.normal);
            let pair=entries(index,i,t,contact.normal,rod);
            let zero=Entry {rod:index,point:0,gradient:[0.;3],mobility:0.};
            add_constraint(&mut constraints,[pair[0],pair[1],zero,zero],-gap)?;
        }
    }
    let body_constraints=constraints.len();
    for response in responses.iter() {
        let (ra,ia,s)=response.a;let (rb,ib,t)=response.b;
        let pa=add(mul(rods[ra].x[ia],1.-s),mul(rods[ra].x[ia+1],s));
        let pb=add(mul(rods[rb].x[ib],1.-t),mul(rods[rb].x[ib+1],t));
        let a=entries(ra,ia,s,response.normal,&rods[ra]);
        let b=entries(rb,ib,t,mul(response.normal,-1.),&rods[rb]);
        add_constraint(&mut constraints,[a[0],a[1],b[0],b[1]],2.*radius-dot(sub(pa,pb),response.normal))?;
    }
    if constraints.is_empty() {return Ok(());}
    prepare_implicit_response(&mut constraints,rods,dt)?;
    let mut increment=PositionIncrement {
        linear:rods.iter().map(|rod|vec![[0.;3];rod.x.len()]).collect(),
        angular:rods.iter().map(|rod|vec![[0.;3];rod.q.len()]).collect(),
    };
    solve_projection(&mut constraints,&mut increment,1e-11)?;
    if !increment.finite() || increment.angular.iter().any(|angles|angles.iter().any(|angle|len(*angle)>0.35)) {
        return Err("shared contact position increment exceeds linearization");
    }
    // Publish only a completely solved increment. Roots remain exact zeros.
    for (index,rod) in rods.iter_mut().enumerate() {
        for point in 1..rod.x.len() {
            rod.x[point]=add(rod.x[point],increment.linear[index][point]);
            if point<rod.q.len() {apply(&mut rod.q[point],increment.angular[index][point]);}
        }
    }
    for (response,constraint) in responses.iter_mut().zip(&constraints[body_constraints..]) {
        response.impulse+=constraint.multiplier;
    }
    Ok(())
}
fn solve_projection<S:ProjectionVector+?Sized>(constraints:&mut [Constraint],rods:&mut S,absolute_tolerance:f64)->Result<(), &'static str> {
    let scale = constraints.iter().try_fold(1f64, |scale, constraint| {
        let speed = constraint.speed(rods);
        if !speed.is_finite() {
            return Err("shared contact velocity overflow");
        }
        Ok(scale.max(speed.abs()).max(constraint.bound.abs()))
    })?;
    if !scale.is_finite() {
        return Err("shared contact velocity overflow");
    }
    let tolerance = absolute_tolerance * scale;
    // Projected Gauss-Seidel solves the joint nonnegative impulse problem.
    // A later strand reaction may activate a body plane (and vice versa).
    let mut last_residual = 0.;
    for sweep in 0..4096 {
        for constraint in constraints.iter_mut() {
            let speed = constraint.speed(rods);
            if !speed.is_finite() {
                return Err("shared contact velocity overflow");
            }
            let next = (constraint.multiplier
                + 1.5 * (constraint.bound - speed) / constraint.diagonal)
                .max(0.);
            let change = next - constraint.multiplier;
            if !next.is_finite() {
                return Err("shared contact velocity impulse overflow");
            }
            constraint.multiplier = next;
            constraint.apply(rods, change);
        }
        let residual = constraints.iter().try_fold(0f64, |residual, constraint| {
            let value = constraint.residual(rods);
            if !value.is_finite() {
                return Err("shared contact velocity residual overflow");
            }
            Ok(residual.max(value))
        })?;
        last_residual = residual;
        if residual <= tolerance
            && rods.finite()
        {
            return Ok(());
        }
        if sweep % 16 == 15 {
            accelerate_active_set(constraints, rods, tolerance)?;
        }
    }
    eprintln!(
        "HAIR VELOCITY NONCONVERGENCE constraints={} residual_m_s={} tolerance_m_s={}",
        constraints.len(),
        last_residual,
        tolerance
    );
    Err("shared contact velocity constraints did not converge")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rod(x: f64) -> HairRod {
        HairRod::new(
            vec![[x, 0., 0.], [x, 0.01, 0.], [x, 0.02, 0.]],
            crate::hair::HairMaterial::default(),
        )
        .unwrap()
    }
    #[test]
    fn shared_wall_and_strand_reactions_satisfy_both_constraints() {
        let mut rods = vec![rod(0.), rod(80e-6)];
        rods[0].velocity[1] = [-1., 0., 0.];
        rods[1].velocity[1] = [-2., 0., 0.];
        rods[0].record_point_contact(1, [1., 0., 0.], [0., 0.01, 0.], ContactSource::Mesh(0));
        let response = StrandResponse {
            a: (0, 1, 0.),
            b: (1, 1, 0.),
            normal: [-1., 0., 0.],
            impulse: 0.,
        };
        stabilize_contact_velocities(&mut rods, &[response], 1. / 240., 40e-6).unwrap();
        assert!(rods[0].velocity[1][0].abs() < 2e-9);
        assert!(rods[1].velocity[1][0].abs() < 2e-9);
        assert_eq!(rods[0].velocity[0], [0.; 3]);
    }
    #[test]
    fn separated_strands_can_close_without_an_artificial_early_impulse() {
        let mut rods = vec![rod(0.), rod(0.001)];
        rods[0].velocity[1] = [0.01, 0., 0.];
        let before = rods.clone();
        let response = StrandResponse {
            a: (0, 1, 0.),
            b: (1, 1, 0.),
            normal: [-1., 0., 0.],
            impulse: 0.,
        };
        stabilize_contact_velocities(&mut rods, &[response], 1. / 240., 40e-6).unwrap();
        assert_eq!(rods[0].velocity, before[0].velocity);
        assert_eq!(rods[1].velocity, before[1].velocity);
    }
    #[test]
    fn incompatible_surface_velocities_report_nonconvergence() {
        let mut rods = vec![rod(0.)];
        for normal in [[1., 0., 0.], [-1., 0., 0.]] {
            let index = rods[0].record_point_contact(
                1,
                normal,
                [0., 0.01, 0.],
                ContactSource::Mesh(usize::from(normal[0] < 0.)),
            );
            rods[0].contacts[index].surface_velocity = normal;
        }
        assert!(stabilize_contact_velocities(&mut rods, &[], 1. / 240., 40e-6).is_err());
    }
    #[test]
    fn joint_position_projection_resolves_wall_and_pair_without_moving_roots_or_velocities() {
        let mut rods=vec![rod(0.),rod(40e-6)];
        rods[0].record_point_contact(1,[-1.,0.,0.],[-10e-6,0.01,0.],ContactSource::Mesh(0));
        let mut responses=[StrandResponse {a:(0,1,0.),b:(1,1,0.),normal:[-1.,0.,0.],impulse:0.}];
        let original=rods.clone();
        reconcile_contact_positions(&mut rods,&mut responses,1./240.,40e-6).unwrap();
        assert!(rods[0].x[1][0]<=-10e-6+1e-11);
        assert!(rods[1].x[1][0]-rods[0].x[1][0]>=80e-6-1e-11);
        assert!(responses[0].impulse>0.);
        for (actual,before) in rods.iter().zip(original) {
            assert_eq!(actual.x[0],before.x[0]);assert_eq!(actual.q[0],before.q[0]);
            assert_eq!(actual.velocity,before.velocity);assert_eq!(actual.omega,before.omega);
            assert!(actual.max_relative_stretch()<0.05);
        }
    }
    #[test]
    fn open_position_contact_has_no_attractive_reaction() {
        let mut rods=vec![rod(0.),rod(1e-3)];let original=rods.clone();
        let mut responses=[StrandResponse {a:(0,1,0.),b:(1,1,0.),normal:[-1.,0.,0.],impulse:0.}];
        reconcile_contact_positions(&mut rods,&mut responses,1./240.,40e-6).unwrap();
        for (actual,before) in rods.iter().zip(original) {assert_eq!(actual.x,before.x);assert_eq!(actual.q,before.q);}
        assert_eq!(responses[0].impulse,0.);
    }
    #[test]
    fn active_newton_solve_releases_a_plane_without_a_negative_impulse() {
        let mut rods = vec![rod(0.)];
        rods[0].velocity[1] = [-1., 1., 0.];
        let zero = Entry {
            rod: 0,
            point: 0,
            gradient: [0.; 3],
            mobility: 0.,
        };
        let mut constraints = Vec::new();
        for normal in [[1., 0., 0.], [0.8, 0.6, 0.]] {
            let pair = entries(0, 1, 0., normal, &rods[0]);
            add_constraint(&mut constraints, [pair[0], pair[1], zero, zero], 0.).unwrap();
        }
        assert!(accelerate_active_set(&mut constraints, rods.as_mut_slice(), 1e-9).unwrap());
        assert!(len(sub(rods[0].velocity[1], [0., 1., 0.])) < 1e-12);
        assert!(constraints[0].multiplier > 0.);
        assert_eq!(constraints[1].multiplier, 0.);
        assert!(constraints
            .iter()
            .all(|constraint| constraint.residual(rods.as_slice()) < 1e-12));
    }
    #[test]
    fn overflowing_finite_velocity_does_not_pass_a_nan_residual() {
        let mut rods = vec![rod(0.)];
        rods[0].velocity[1] = [1.7e308; 3];
        rods[0].record_point_contact(1, unit([1.; 3]), [0., 0.01, 0.], ContactSource::Mesh(0));
        assert!(stabilize_contact_velocities(&mut rods, &[], 1. / 240., 40e-6).is_err());
    }
    #[test]
    fn implicit_rod_response_is_reciprocal_and_transmits_rotation() {
        let rods = vec![rod(0.)];
        let mut constraints = Vec::new();
        let zero = Entry {
            rod: 0,
            point: 0,
            gradient: [0.; 3],
            mobility: 0.,
        };
        for (segment, fraction) in [(1, 0.), (1, 1.)] {
            let pair = entries(0, segment, fraction, [1., 0., 0.], &rods[0]);
            add_constraint(&mut constraints, [pair[0], pair[1], zero, zero], 0.).unwrap();
        }
        prepare_implicit_response(&mut constraints, &rods, 1. / 240.).unwrap();
        let a = &constraints[0].response[0];
        let b = &constraints[1].response[0];
        assert!(
            (a.linear[2][0] - b.linear[1][0]).abs()
                < 1e-12 * (constraints[0].diagonal * constraints[1].diagonal).sqrt(),
            "reciprocal responses={} versus {}, diagonals={:?}",
            a.linear[2][0],
            b.linear[1][0],
            constraints
                .iter()
                .map(|constraint| constraint.diagonal)
                .collect::<Vec<_>>()
        );
        assert!(
            a.angular.iter().any(|rotation| len(*rotation) > 0.),
            "contact failed to transmit material-frame rotation"
        );
        assert_eq!(a.linear[0], [0.; 3]);
        assert_eq!(a.angular[0], [0.; 3]);
        assert!(
            constraints[0].diagonal < rods[0].inv_mass[1],
            "implicit response ignored rod stiffness"
        );
        let energy = constraints[0].diagonal + 0.25 * constraints[1].diagonal - a.linear[2][0];
        assert!(energy > 0., "coupled rod compliance lost positive energy");
    }
}
