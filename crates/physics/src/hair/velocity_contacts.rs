//! Shared unilateral contact projection under the rod compliance operator.
use super::{ContactSource, HairRod, StrandResponse};
use crate::hair::math::*;
#[path = "contact_projected_gradient.rs"]
mod projected_gradient;
#[path="contact_dense_active_set.rs"]
mod dense_active_set;
#[path="contact_square_root_projection.rs"]
mod square_root_projection;
#[path="contact_response_batches.rs"]
mod response_batches;
#[path="contact_safe_motion.rs"]
mod safe_motion;
pub(in crate::hair) use safe_motion::{advance as advance_swept_strands,strand_fraction,admit_staged_strands};

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
#[derive(Clone)]
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
    mut entries: [Entry; 4],
    bound: f64,
) -> Result<Option<usize>, &'static str> {
    // Pinned-root entries contribute no increment. Canonical station order
    // removes aliases of a shared endpoint from adjacent capsule pairs.
    for entry in &mut entries {if entry.point==0 {entry.gradient=[0.;3];}}
    entries.sort_by_key(|entry|(entry.rod,entry.point));
    let zero=Entry {rod:0,point:0,gradient:[0.;3],mobility:0.};
    let mut packed=[zero;4];let mut count=0;
    for entry in entries {
        if dot(entry.gradient,entry.gradient)==0. {continue;}
        if count>0 && packed[count-1].rod==entry.rod && packed[count-1].point==entry.point {
            packed[count-1].gradient=add(packed[count-1].gradient,entry.gradient);
        } else {packed[count]=entry;count+=1;}
    }
    let entries=packed;
    let diagonal = entries
        .iter()
        .map(|entry| entry.mobility * dot(entry.gradient, entry.gradient))
        .sum::<f64>();
    if !diagonal.is_finite() || !bound.is_finite() {
        return Err("invalid shared contact velocity constraint");
    }
    if diagonal > 1e-30 {
        if let Some((index,existing))=constraints.iter_mut().enumerate().find(|(_,constraint)| {
            constraint.entries.iter().zip(entries).all(|(a,b)|a.rod==b.rod && a.point==b.point
                && a.mobility==b.mobility && len(sub(a.gradient,b.gradient))<=1e-12)
        }) {
            existing.bound=existing.bound.max(bound);
            return Ok(Some(index));
        }
        constraints.push(Constraint {
            entries,
            bound,
            diagonal,
            multiplier: 0.,
            response: Vec::new(),
        });
        return Ok(Some(constraints.len()-1));
    }
    Ok(None)
}
#[allow(dead_code)]
fn prepare_implicit_response(
    constraints: &mut [Constraint],
    rods: &[HairRod],
    dt: f64,
) -> Result<(), &'static str> {
    prepare_implicit_response_with_solver(constraints,rods,dt,None)
}
fn prepare_implicit_response_with_solver(constraints:&mut [Constraint],rods:&[HairRod],dt:f64,solver:Option<&mut dyn crate::hair::HairLinearSolver>)->Result<(), &'static str> {
    if let Some(solver)=solver {return response_batches::prepare(constraints,rods,dt,solver);}
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
    stabilize_contact_velocities_with_solver(rods,responses,dt,radius,None)
}
pub(in crate::hair) fn stabilize_contact_velocities_with_solver(rods:&mut [HairRod],responses:&[StrandResponse],dt:f64,radius:f64,solver:Option<&mut dyn crate::hair::HairLinearSolver>)->Result<(), &'static str> {
    if !dt.is_finite() || dt <= 0. || !radius.is_finite() || radius <= 0. {
        return Err("invalid shared contact velocity step");
    }
    let mut constraints = Vec::new();
    for (index, rod) in rods.iter().enumerate() {
        for contact in &rod.contacts {
            if contact.trajectory_time.is_some() || !matches!(contact.source, ContactSource::Mesh(_)) {
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
    prepare_implicit_response_with_solver(&mut constraints, rods, dt,solver)?;
    solve_projection(&mut constraints,rods,1e-9)
}
pub(in crate::hair) fn reconcile_contact_positions(rods:&mut [HairRod],responses:&mut [StrandResponse],dt:f64,radius:f64)->Result<bool, &'static str> {
    reconcile_contact_positions_with_solver(rods,responses,dt,radius,None)
}
pub(in crate::hair) fn reconcile_contact_positions_with_solver(rods:&mut [HairRod],responses:&mut [StrandResponse],dt:f64,radius:f64,solver:Option<&mut dyn crate::hair::HairLinearSolver>)->Result<bool, &'static str> {
    reconcile_position_increment(rods,responses,dt,radius,solver,false)
}
pub(in crate::hair) fn reconcile_elastic_contact_positions_with_solver(rods:&mut [HairRod],responses:&mut [StrandResponse],dt:f64,radius:f64,solver:Option<&mut dyn crate::hair::HairLinearSolver>)->Result<bool, &'static str> {
    reconcile_position_increment(rods,responses,dt,radius,solver,true)
}
fn reconcile_position_increment(rods:&mut [HairRod],responses:&mut [StrandResponse],dt:f64,radius:f64,solver:Option<&mut dyn crate::hair::HairLinearSolver>,include_free:bool)->Result<bool, &'static str> {
    if !dt.is_finite() || dt<=0. || !radius.is_finite() || radius<=0. {return Err("invalid shared contact position step");}
    let (mut constraints,pair_constraints)=position_constraints(rods,responses,radius)?;
    if constraints.is_empty() {return Ok(true);}
    // Couple the free implicit elastic step and unilateral reactions under
    // the SAME H. Projecting a zero increment separately fights the preceding
    // elastic solve and can settle into a stretch/contact alternating cycle.
    // Original physical metre rows retain their 1e-11 solve tolerance.
    let increment=if include_free {
        constrained_newton_increment(&mut constraints,rods,dt,solver,1e-11)?
    } else {
        prepare_implicit_response_with_solver(&mut constraints,rods,dt,solver)?;
        let mut increment=PositionIncrement {linear:rods.iter().map(|r|vec![[0.;3];r.x.len()]).collect(),angular:rods.iter().map(|r|vec![[0.;3];r.q.len()]).collect()};
        solve_projection(&mut constraints,&mut increment,1e-11)?;
        increment
    };
    if !increment.finite() {return Err("shared contact position increment overflow");}
    // A shared reaction requires a common fraction within its connected
    // contact component, not across unrelated rods. Reuse the swept-motion
    // component owner so both paths preserve the same pairing invariant.
    let (_,scales)=safe_motion::trust_components(rods,&increment,responses)?;
    // Publish only a completely solved increment. Roots remain exact zeros.
    for (index,rod) in rods.iter_mut().enumerate() {
        let scale=scales[index];
        for point in 1..rod.x.len() {
            rod.x[point]=add(rod.x[point],mul(increment.linear[index][point],scale));
            if point<rod.q.len() {apply(&mut rod.q[point],mul(increment.angular[index][point],scale));}
        }
    }
    let mut multiplicity=vec![0usize;constraints.len()];
    for index in pair_constraints.iter().flatten() {multiplicity[*index]+=1;}
    for (response,index) in responses.iter_mut().zip(pair_constraints) {
        if let Some(index)=index {response.impulse+=constraints[index].multiplier*scales[response.a.0]/multiplicity[index] as f64;}
    }
    Ok(scales.iter().all(|scale|*scale==1.))
}
// Reuse exactly the geometry and alias ownership of positional projection.
fn position_constraints(rods:&[HairRod],responses:&[StrandResponse],radius:f64)->Result<(Vec<Constraint>,Vec<Option<usize>>), &'static str> {
    let mut constraints=Vec::new();
    for (index,rod) in rods.iter().enumerate() {
        for contact in &rod.contacts {
            if !matches!(contact.source,ContactSource::Mesh(_)) {continue;}
            let i=contact.segment;let t=contact.fraction;
            let position=add(mul(rod.x[i],1.-t),mul(rod.x[i+1],t));
            // Use the physical envelope Jacobian, including the time factor
            // for a trajectory witness. Unit-plane rescaling otherwise changes
            // the force multiplier and the residual's physical tolerance.
            if !contact.metric_scale.is_finite() || contact.metric_scale <= 0. {
                return Err("invalid contact physical metric");
            }
            let gap=contact.physical_gap(position);
            let pair=entries(index,i,t,mul(contact.normal,contact.metric_scale),rod);
            let zero=Entry {rod:index,point:0,gradient:[0.;3],mobility:0.};
            add_constraint(&mut constraints,[pair[0],pair[1],zero,zero],-gap)?;
        }
    }
    let mut pair_constraints=Vec::with_capacity(responses.len());
    for response in responses.iter() {
        let (ra,ia,s)=response.a;let (rb,ib,t)=response.b;
        let pa=add(mul(rods[ra].x[ia],1.-s),mul(rods[ra].x[ia+1],s));
        let pb=add(mul(rods[rb].x[ib],1.-t),mul(rods[rb].x[ib+1],t));
        let a=entries(ra,ia,s,response.normal,&rods[ra]);
        let b=entries(rb,ib,t,mul(response.normal,-1.),&rods[rb]);
        pair_constraints.push(add_constraint(&mut constraints,[a[0],a[1],b[0],b[1]],2.*radius-dot(sub(pa,pb),response.normal))?);
    }
    Ok((constraints,pair_constraints))
}

/// Recover absolute normal reactions from the current physical force balance.
/// The virtual free Newton increment is projected, but never applied to poses.
/// This avoids treating accumulated nonlinear solver corrections as pressure.
pub(in crate::hair) fn recover_friction_pressure(rods:&[HairRod],responses:&mut [StrandResponse],dt:f64,radius:f64,solver:Option<&mut dyn crate::hair::HairLinearSolver>)->Result<(), &'static str> {
    if !dt.is_finite() || dt<=0. || !radius.is_finite() || radius<=0. {return Err("invalid friction pressure step");}
    let (mut constraints,aliases)=position_constraints(rods,responses,radius)?;
    if constraints.is_empty() {for response in responses {response.impulse=0.;} return Ok(());}
    let _free = constrained_newton_increment(&mut constraints, rods, dt, solver, 1e-14)?;
    let mut multiplicity=vec![0usize;constraints.len()];
    for index in aliases.iter().flatten() {multiplicity[*index]+=1;}
    for (response,index) in responses.iter_mut().zip(aliases) {
        response.impulse=index.map_or(0.,|index|constraints[index].multiplier/multiplicity[index] as f64);
    }
    Ok(())
}
/// Solve the structural increment and unilateral reactions under one H.
/// Returns a candidate only: nonlinear clearance and swept contact admission
/// must complete before a caller may publish this motion.
fn constrained_newton_increment(
    constraints: &mut [Constraint], rods: &[HairRod], dt: f64,
    solver: Option<&mut dyn crate::hair::HairLinearSolver>, tolerance: f64,
) -> Result<PositionIncrement, &'static str> {
    constrained_newton_increment_prepared(constraints,rods,dt,solver,tolerance,None)
}

// Borrow the frozen pose for the complete cut-refinement transaction. This
// context cannot survive a mutable pose update or change its timestep.
struct NativeNewtonStep<'a> {
    rods:&'a [HairRod],
    dt:f64,
    free:PositionIncrement,
    systems:square_root_projection::FrozenSystems<'a>,
}
impl<'a> NativeNewtonStep<'a> {
    fn new(rods:&'a [HairRod],dt:f64,tolerance:f64)->Result<Self,&'static str> {
        let free=constrained_newton_increment(&mut [],rods,dt,None,tolerance)?;
        Ok(Self {rods,dt,free,systems:square_root_projection::FrozenSystems::new(rods,dt)})
    }
    fn project(&self,constraints:&mut [Constraint],tolerance:f64)->Result<PositionIncrement,&'static str> {
        self.project_with_solver(constraints,tolerance,None)
    }
    fn project_with_solver(&self,constraints:&mut [Constraint],tolerance:f64,solver:Option<&mut dyn crate::hair::HairLinearSolver>)->Result<PositionIncrement,&'static str> {
        constrained_newton_increment_prepared(constraints,self.rods,self.dt,solver,tolerance,Some((&self.free,&self.systems)))
    }
}
fn constrained_newton_increment_prepared(
    constraints:&mut [Constraint],rods:&[HairRod],dt:f64,
    mut solver:Option<&mut dyn crate::hair::HairLinearSolver>,tolerance:f64,
    prepared:Option<(&PositionIncrement,&square_root_projection::FrozenSystems<'_>)>,
)->Result<PositionIncrement,&'static str> {
    let prepared_free=prepared.map(|(free,_)|free);
    if !dt.is_finite() || dt <= 0. || !tolerance.is_finite() || tolerance <= 0. {
        return Err("invalid constrained Newton step");
    }
    let mut free = prepared_free.cloned().unwrap_or_else(||PositionIncrement {
        linear: rods.iter().map(|rod| vec![[0.;3];rod.x.len()]).collect(),
        angular: rods.iter().map(|rod| vec![[0.;3];rod.q.len()]).collect(),
    });
    struct Native;
    impl crate::hair::HairLinearSolver for Native {
        fn solve(&mut self, _: &[crate::hair::HairLinearSystem]) -> Result<Vec<Vec<f64>>, &'static str> {
            Err("constrained Newton step requires response loads")
        }
    }
    let joint_projection=solver.as_ref().is_some_and(|backend|backend.joint_contact_coordinates_enabled());
    let native_projection=solver.is_none() || joint_projection;
    let mut native = Native;
    if native_projection {
        // QR owns contact response columns. Solve only the elastic free load;
        // Gram compliances are needed solely by the legacy recovery path.
        if prepared_free.is_none() {
            response_batches::prepare_with_free(&mut [],rods,dt,&mut native,&mut free)?;
        }
    } else {
        response_batches::prepare_with_free(constraints,rods,dt,solver.as_deref_mut().unwrap(),&mut free)?;
    }
    if native_projection {
        // Original square-root columns preserve directions lost by forming a
        // rounded Gram matrix. Try that canonical native solve first instead
        // of exhausting iterative Gram recovery before using the same owner.
        let projected=if joint_projection {
            if let Some((_,systems))=prepared {
                square_root_projection::solve_accelerated_prepared(constraints,systems,free.clone(),tolerance,solver.take().unwrap())
            } else {square_root_projection::solve_accelerated(constraints,rods,dt,free.clone(),tolerance,solver.take().unwrap())}
        } else if let Some((_,systems))=prepared {
            square_root_projection::solve_prepared(constraints,systems,free.clone(),tolerance)
        } else {square_root_projection::solve(constraints,rods,dt,free.clone(),tolerance)};
        match projected {
            Ok(candidate)=>free=candidate,
            Err(square_root_error)=> {
                // Failed square-root solves stage reactions without publication.
                // The legacy native projection still starts from original free.
                response_batches::prepare(constraints,rods,dt,&mut native)?;
                if solve_projection(constraints,&mut free,tolerance).is_err() {
                    return Err(square_root_error);
                }
            }
        }
    } else {
        solve_projection(constraints,&mut free,tolerance)?;
    }
    if !free.finite() { return Err("constrained Newton increment overflow"); }
    Ok(free)
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
    if dense_active_set::solve(constraints,rods,tolerance)? {
        return Ok(());
    }
    if projected_gradient::solve(constraints, rods, tolerance)? {
        return Ok(());
    }
    eprintln!(
        "HAIR CONTACT PROJECTION NONCONVERGENCE constraints={} residual={} tolerance={}",
        constraints.len(),
        last_residual,
        tolerance
    );
    export_projection_failure(constraints,rods,tolerance);
    Err("shared contact projection constraints did not converge")
}

fn export_projection_failure<S:ProjectionVector+?Sized>(constraints:&[Constraint],state:&S,tolerance:f64) {
    let Ok(path)=std::env::var("VOXY_HAIR_PROJECTION_FAILURE_EXPORT") else {return;};
    let write=||->std::io::Result<()> {
        use std::io::Write;
        let n=constraints.len();
        let mut gram=vec![0.;n*n];
        for (i,row) in constraints.iter().enumerate() {
            for (j,column) in constraints.iter().enumerate() {
                gram[i*n+j]=row.entries.iter().map(|entry| {
                    let response=if column.response.is_empty() {
                        column.entries.iter().filter(|other|other.rod==entry.rod&&other.point==entry.point)
                            .fold([0.;3],|v,other|add(v,mul(other.gradient,other.mobility)))
                    } else {
                        column.response.iter().filter(|response|response.rod==entry.rod)
                            .fold([0.;3],|v,response|add(v,response.linear[entry.point]))
                    };
                    dot(entry.gradient,response)
                }).sum();
            }
        }
        let mut file=std::fs::File::create(path)?;
        file.write_all(b"VQP1")?;
        file.write_all(&(n as u32).to_le_bytes())?;
        file.write_all(&tolerance.to_le_bytes())?;
        for value in &gram {file.write_all(&value.to_le_bytes())?;}
        // Recover b - J*v_free from the final response, retaining the original
        // unnormalised operator. This diagnostic never changes the solve.
        for (i,row) in constraints.iter().enumerate() {
            let reaction=gram[i*n..(i+1)*n].iter().zip(constraints).map(|(g,c)|g*c.multiplier).sum::<f64>();
            file.write_all(&(row.bound-row.speed(state)+reaction).to_le_bytes())?;
        }
        for row in constraints {file.write_all(&row.multiplier.to_le_bytes())?;}
        let shape=state.shape();
        file.write_all(&(shape.len() as u32).to_le_bytes())?;
        for count in shape {file.write_all(&(count as u32).to_le_bytes())?;}
        for row in constraints {
            for entry in row.entries {
                file.write_all(&(entry.rod as u32).to_le_bytes())?;
                file.write_all(&(entry.point as u32).to_le_bytes())?;
                for value in entry.gradient {file.write_all(&value.to_le_bytes())?;}
            }
        }
        Ok(())
    };
    if let Err(error)=write() {eprintln!("HAIR CONTACT PROJECTION EXPORT FAILED {error}");}
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
    fn recovered_pressure_matches_inertial_force_and_discards_solver_history() {
        let dt=1./240.; let radius=40e-6; let approach=1e-6;
        let mut rods=vec![rod(0.),rod(2.*radius)];
        for point in 1..3 {
            rods[0].predicted_x[point][0]+=approach;
            rods[1].predicted_x[point][0]-=approach;
        }
        let original=rods.clone();
        let make=|history|[0,1].map(|segment|StrandResponse {
            a:(0,segment,1.),b:(1,segment,1.),normal:[-1.,0.,0.],impulse:history,
        });
        let mut responses=make(0.);
        recover_friction_pressure(&rods,&mut responses,dt,radius,None).unwrap();
        let expected=rods[0].inv_mass[1..].iter().map(|inverse|1./inverse).sum::<f64>()*approach;
        let actual=responses.iter().map(|response|response.impulse).sum::<f64>();
        assert!((actual-expected).abs()<expected*1e-4,"reaction {actual:e}, inertial load {expected:e}");
        assert!(responses.iter().all(|response|response.impulse>0.));
        let mut stale=make(1.);
        recover_friction_pressure(&rods,&mut stale,dt,radius,None).unwrap();
        assert_eq!(responses.iter().map(|r|r.impulse).collect::<Vec<_>>(),stale.iter().map(|r|r.impulse).collect::<Vec<_>>());
        for (a,b) in rods.iter().zip(&original) {
            assert_eq!(a.x,b.x);assert_eq!(a.q,b.q);assert_eq!(a.velocity,b.velocity);
        }
        // Resting pressure must still generate tangential friction even with
        // zero closing normal velocity; dropping positional history cannot
        // silently remove Coulomb support friction.
        for point in 1..3 {rods[0].velocity[point][1]=0.01;rods[1].velocity[point][1]=-0.01;}
        crate::hair::contact::finish_strand_contacts(&mut rods,&responses,dt);
        for point in 1..3 {
            assert!(rods[0].velocity[point][1]<0.01);
            assert!(rods[1].velocity[point][1]>-0.01);
            assert!((rods[0].velocity[point][1]+rods[1].velocity[point][1]).abs()<1e-14);
        }
        for point in 1..3 {
            rods[0].predicted_x[point][0]-=2.*approach;
            rods[1].predicted_x[point][0]+=2.*approach;
        }
        recover_friction_pressure(&rods,&mut stale,dt,radius,None).unwrap();
        assert!(stale.iter().all(|response|response.impulse==0.),"separating strands must release pressure");
    }
    #[test]
    fn dual_projection_handles_more_than_256_dependent_contacts() {
        let zero=Entry {rod:0,point:0,gradient:[0.;3],mobility:0.};
        let mut constraints:Vec<_>=(0..300).map(|i|Constraint {
            entries:[Entry {rod:0,point:i/2+1,gradient:[1.,0.,0.],mobility:1.},zero,zero,zero],
            bound:(i%2+1) as f64*1e-6,diagonal:1.,multiplier:0.,response:Vec::new(),
        }).collect();
        let mut increment=PositionIncrement {linear:vec![vec![[0.;3];151]],angular:vec![vec![[0.;3];150]]};
        assert!(projected_gradient::solve(&mut constraints,&mut increment,1e-11).unwrap());
        assert!(increment.linear[0][1..].iter().all(|p|(p[0]-2e-6).abs()<1e-11));
        assert_eq!(increment.linear[0][0],[0.;3]);
        assert!(constraints.iter().step_by(2).all(|c|c.multiplier==0.));
    }
    #[test]
    fn dual_projection_resolves_dependent_rows_with_different_bounds() {
        let zero = Entry {rod:0,point:0,gradient:[0.;3],mobility:0.};
        let entry = Entry {rod:0,point:1,gradient:[1.,0.,0.],mobility:1.};
        let mut constraints: Vec<_> = [1e-6, 2e-6].into_iter().map(|bound| Constraint {
            entries:[entry,zero,zero,zero], bound, diagonal:1., multiplier:0., response:Vec::new(),
        }).collect();
        let mut increment = PositionIncrement {linear:vec![vec![[0.;3];2]],angular:vec![vec![[0.;3];1]]};
        assert!(projected_gradient::solve(&mut constraints,&mut increment,1e-11).unwrap());
        assert!((increment.linear[0][1][0]-2e-6).abs()<1e-11);
        assert_eq!(constraints[0].multiplier,0.);
        assert!(constraints[1].multiplier>0.);
        assert_eq!(increment.linear[0][0],[0.;3]);
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
    fn submicron_wall_projection_does_not_admit_linear_residual_as_separation() {
        let mut rods = vec![rod(0.)];
        let target = [0.6e-6, 0.01, 0.];
        rods[0].record_point_contact(1, [1., 0., 0.], target, ContactSource::Mesh(0));
        let before = rods[0].clone();
        assert!(reconcile_contact_positions(&mut rods, &mut [], 1. / 240., 40e-6).unwrap());
        // For one plane the minimum-compliance solution is exactly on its
        // boundary. A coarse stopping rule also admits an overshooting impulse.
        assert!((rods[0].x[1][0] - target[0]).abs() < 1e-10);
        assert_eq!(rods[0].x[0], before.x[0]);
        assert_eq!(rods[0].velocity, before.velocity);
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
    fn shared_endpoint_constraints_have_one_canonical_row_and_strongest_bound() {
        let hair=rod(0.);let zero=Entry {rod:0,point:0,gradient:[0.;3],mobility:0.};
        let a=entries(0,0,1.,[1.,0.,0.],&hair);
        let b=entries(0,1,0.,[1.,0.,0.],&hair);
        let mut constraints=Vec::new();
        let first=add_constraint(&mut constraints,[a[0],a[1],zero,zero],1e-6).unwrap();
        let second=add_constraint(&mut constraints,[zero,b[1],b[0],zero],2e-6).unwrap();
        assert_eq!(first,second);assert_eq!(constraints.len(),1);assert_eq!(constraints[0].bound,2e-6);
    }
    #[test]
    fn trajectory_position_row_uses_physical_time_jacobian() {
        let mut hair=rod(0.);
        hair.contacts.push(crate::hair::RodContact {
            segment:1, fraction:0.4, normal:[1.,0.,0.],
            target:[1e-5,0.014,0.], surface_velocity:[0.;3],
            source:ContactSource::Mesh(0), metric_scale:0.25,
            trajectory_time:Some(0.25),
        });
        let (rows,_)=position_constraints(&[hair],&[],40e-6).unwrap();
        assert_eq!(rows.len(),1);
        assert!((rows[0].bound-2.5e-6).abs()<1e-20);
        let gradient: V=rows[0].entries.iter().fold([0.;3],|a,e|add(a,e.gradient));
        assert_eq!(gradient,[0.25,0.,0.]);
        let direction=PositionIncrement {linear:vec![vec![[0.;3],[2e-6,0.,0.],[-1e-6,0.,0.]]],angular:vec![vec![[0.;3];2]]};
        assert!((rows[0].speed(&direction)-2e-7).abs()<1e-20);
    }
    #[test]
    fn shared_position_entrypoint_applies_free_elastic_motion_with_contact_reactions() {
        let radius=40e-6;
        let mut rods=vec![rod(0.),rod(2.*radius),rod(0.01)];
        for point in 1..3 {
            rods[0].predicted_x[point][0]+=1e-3;
            rods[1].predicted_x[point][0]-=1e-3;
            rods[2].predicted_x[point][0]+=1e-3;
        }
        let roots:Vec<_>=rods.iter().map(|rod|rod.x[0]).collect();
        let mut pairs=[StrandResponse {a:(0,1,0.),b:(1,1,0.),normal:[-1.,0.,0.],impulse:0.}];
        reconcile_elastic_contact_positions_with_solver(&mut rods,&mut pairs,1./240.,radius,None).unwrap();
        assert!(rods[2].x[1][0]>0.01+1e-9,"a zero-only projection froze the independent elastic guide");
        let separation=dot(sub(rods[0].x[1],rods[1].x[1]),pairs[0].normal);
        assert!(separation>=2.*radius-1e-11,"coupled elastic motion crossed the active contact");
        for (rod,root) in rods.iter().zip(roots) {assert_eq!(rod.x[0],root);}
    }
    #[test]
    fn coupled_newton_motion_releases_separating_contact() {
        let radius=40e-6;
        let mut rods=vec![rod(0.),rod(2.*radius)];
        for point in 1..3 {
            rods[0].predicted_x[point][0] -= 1e-3;
            rods[1].predicted_x[point][0] += 1e-3;
        }
        let pairs=[StrandResponse {a:(0,1,0.),b:(1,1,0.),normal:[-1.,0.,0.],impulse:0.}];
        let (mut constraints,_)=position_constraints(&rods,&pairs,radius).unwrap();
        let free=constrained_newton_increment(&mut [],&rods,1./240.,None,1e-14).unwrap();
        let coupled=constrained_newton_increment(&mut constraints,&rods,1./240.,None,1e-14).unwrap();
        assert_eq!(constraints[0].multiplier,0.);
        assert!(constraints[0].speed(&coupled)>1e-9);
        assert_eq!(coupled.linear,free.linear);
        assert_eq!(coupled.angular,free.angular);
    }
    #[test]
    fn coupled_newton_motion_stops_closing_contact_and_preserves_free_guide() {
        let radius = 40e-6;
        let mut rods = vec![rod(0.), rod(2.*radius), rod(0.01)];
        for point in 1..3 {
            rods[0].predicted_x[point][0] += 1e-3;
            rods[1].predicted_x[point][0] -= 1e-3;
            rods[2].predicted_x[point][0] += 1e-3;
        }
        let originals = rods.clone();
        let pairs = [StrandResponse {a:(0,1,0.),b:(1,1,0.),normal:[-1.,0.,0.],impulse:0.}];
        let (mut constraints, _) = position_constraints(&rods, &pairs, radius).unwrap();
        let free = constrained_newton_increment(&mut [], &rods, 1./240., None, 1e-14).unwrap();
        assert!(dot(sub(free.linear[0][1],free.linear[1][1]),pairs[0].normal) < -1e-9);
        struct Legacy;
        impl crate::hair::HairLinearSolver for Legacy {
            fn solve(&mut self,_:&[crate::hair::HairLinearSystem])->Result<Vec<Vec<f64>>, &'static str> {Err("response-only reference")}
        }
        let (mut legacy_constraints,_)=position_constraints(&rods,&pairs,radius).unwrap();
        let mut legacy_free=free.clone();
        response_batches::prepare_with_free(&mut legacy_constraints,&rods,1./240.,&mut Legacy,&mut legacy_free).unwrap();
        let legacy=square_root_projection::solve(&mut legacy_constraints,&rods,1./240.,legacy_free,1e-14).unwrap();
        let coupled = constrained_newton_increment(&mut constraints, &rods, 1./240., None, 1e-14).unwrap();
        struct UnavailableJoint {calls:usize,fallbacks:usize}
        impl crate::hair::HairLinearSolver for UnavailableJoint {
            fn joint_contact_coordinates_enabled(&self)->bool {true}
            fn solve_joint_coordinates(&mut self,_:&[Vec<f64>],_:&[f64],_:f64)->Option<(Vec<f64>,Vec<f64>)> {self.calls+=1;None}
            fn joint_contact_result(&mut self,accelerated:bool) {assert!(!accelerated);self.fallbacks+=1;}
            fn solve(&mut self,_:&[crate::hair::HairLinearSystem])->Result<Vec<Vec<f64>>, &'static str> {panic!("joint route must retain native free owner")}
            fn solve_responses(&mut self,_:&[crate::hair::HairResponseSystem])->Result<Vec<Vec<Vec<f64>>>, &'static str> {panic!("joint route must not form Gram compliances")}
        }
        let mut backend=UnavailableJoint {calls:0,fallbacks:0};
        let (mut accelerated_rows,_)=position_constraints(&rods,&pairs,radius).unwrap();
        let fallback=constrained_newton_increment(&mut accelerated_rows,&rods,1./240.,Some(&mut backend),1e-14).unwrap();
        assert!(backend.calls>0 && backend.fallbacks>0);
        assert_eq!(fallback.linear,coupled.linear);assert_eq!(fallback.angular,coupled.angular);
        assert_eq!(accelerated_rows[0].multiplier.to_bits(),constraints[0].multiplier.to_bits());
        assert!(accelerated_rows.iter().all(|c|c.response.is_empty()));
        assert_eq!(coupled.linear,legacy.linear);assert_eq!(coupled.angular,legacy.angular);
        assert!(constraints.iter().all(|c|c.response.is_empty()),"successful QR must not prepare unused Gram responses");
        assert_eq!(constraints[0].multiplier.to_bits(),legacy_constraints[0].multiplier.to_bits());
        let prepared=NativeNewtonStep::new(&rods,1./240.,1e-14).unwrap();
        assert_eq!(prepared.systems.assembled_count(),0);
        let mut computed_columns = None;
        for offset in [0.,1e-8,-1e-8] {
            let (mut cached_rows,_)=position_constraints(&rods,&pairs,radius).unwrap();
            let (mut fresh_rows,_)=position_constraints(&rods,&pairs,radius).unwrap();
            cached_rows[0].bound+=offset;fresh_rows[0].bound+=offset;
            let cached=prepared.project(&mut cached_rows,1e-14).unwrap();
            let fresh=constrained_newton_increment(&mut fresh_rows,&rods,1./240.,None,1e-14).unwrap();
            let bits=|step:&PositionIncrement|step.linear.iter().chain(&step.angular)
                .flat_map(|points|points.iter().flatten()).map(|v|v.to_bits()).collect::<Vec<_>>();
            assert_eq!(bits(&cached),bits(&fresh));
            assert_eq!(cached_rows.iter().map(|c|c.multiplier.to_bits()).collect::<Vec<_>>(),fresh_rows.iter().map(|c|c.multiplier.to_bits()).collect::<Vec<_>>());
            assert_eq!(prepared.systems.assembled_count(),2,"contact matrices must be assembled only once for touched rods");
            let actual_columns=prepared.systems.computed_load_columns();
            assert!(actual_columns>0);
            if let Some(previous)=computed_columns {assert_eq!(actual_columns,previous,"changed bounds must reuse exact rod load columns");}
            computed_columns=Some(actual_columns);
        }
        if std::env::var_os("VOXY_HAIR_FROZEN_OPERATOR_BENCH").is_some() {
            let mut paired = Vec::new();
            for round in 0..7 {
                let measure = |cached:bool| {
                    let began = std::time::Instant::now();
                    for _ in 0..100 {
                        let (mut rows,_) = position_constraints(std::hint::black_box(&rods), &pairs, radius).unwrap();
                        let step = if cached { prepared.project(&mut rows, 1e-14) } else {
                            square_root_projection::solve(&mut rows, &rods, 1./240., free.clone(), 1e-14)
                        }.unwrap();
                        for (a,b) in step.linear.iter().chain(&step.angular).flat_map(|v|v.iter().flatten())
                            .zip(coupled.linear.iter().chain(&coupled.angular).flat_map(|v|v.iter().flatten())) {
                            assert_eq!(a.to_bits(), b.to_bits());
                        }
                        assert_eq!(rows[0].multiplier.to_bits(),constraints[0].multiplier.to_bits());
                        std::hint::black_box(step);
                    }
                    began.elapsed().as_secs_f64()
                };
                let times = if round%2==0 {let fresh=measure(false);(fresh,measure(true))}
                    else {let cached=measure(true);(measure(false),cached)};
                paired.push(times);
            }
            eprintln!("FROZEN OPERATOR fresh_cached_seconds={paired:?} scope=complete_immutable_contact_projection");
        }
        assert!(constraints[0].multiplier > 0.);
        assert!(constraints[0].residual(&coupled) <= 1e-14);
        assert_eq!(coupled.linear[2],free.linear[2]);
        assert_eq!(coupled.angular[2],free.angular[2]);
        for (actual,before) in rods.iter().zip(originals) {
            assert_eq!(actual.x,before.x); assert_eq!(actual.q,before.q);
        }
        for index in 0..rods.len() {
            assert_eq!(coupled.linear[index][0],[0.;3]);
            assert_eq!(coupled.angular[index][0],[0.;3]);
        }
    }
    #[test]
    fn free_newton_batch_keeps_unconstrained_guides_moving() {
        struct Native;
        impl crate::hair::HairLinearSolver for Native {
            fn solve(&mut self, _: &[crate::hair::HairLinearSystem]) -> Result<Vec<Vec<f64>>, &'static str> { unreachable!() }
        }
        let dt = 1. / 240.;
        let mut rods = vec![rod(0.), rod(0.01)];
        for rod in &mut rods {
            for point in 1..rod.x.len() {
                rod.predicted_x[point][0] += 1e-3;
            }
        }
        let original = rods.clone();
        let mut free = PositionIncrement {
            linear: rods.iter().map(|rod| vec![[0.;3];rod.x.len()]).collect(),
            angular: rods.iter().map(|rod| vec![[0.;3];rod.q.len()]).collect(),
        };
        response_batches::prepare_with_free(&mut [], &rods, dt, &mut Native, &mut free).unwrap();
        for (index, rod) in rods.iter_mut().enumerate() {
            let (matrix, rhs) = crate::hair::direct::assemble(rod, dt).unwrap();
            let expected = crate::hair::HairLinearSystem {
                band_width: 9, matrix, rhs, active: 6..rod.x.len()*6-3,
            }.solve_native().unwrap();
            assert!(free.linear[index].iter().any(|value| len(*value) > 1e-9));
            assert_eq!(free.linear[index][0], [0.;3]);
            assert_eq!(free.angular[index][0], [0.;3]);
            for point in 1..rod.x.len() {
                for axis in 0..3 {
                    assert!((free.linear[index][point][axis] - expected[point*6+axis]).abs() < 1e-12);
                }
            }
            assert_eq!(rod.x, original[index].x);
            assert_eq!(rod.q, original[index].q);
        }
    }
    #[test]
    fn duplicate_pair_aliases_share_one_positional_reaction_load() {
        let original=vec![rod(0.),rod(40e-6)];let mut single=original.clone();let mut duplicate=original;
        let make=|segment,fraction|StrandResponse {a:(0,segment,fraction),b:(1,segment,fraction),normal:[-1.,0.,0.],impulse:0.};
        let mut one=[make(1,0.)];let mut two=[make(0,1.),make(1,0.)];
        assert!(reconcile_contact_positions(&mut single,&mut one,1./240.,40e-6).unwrap());
        assert!(reconcile_contact_positions(&mut duplicate,&mut two,1./240.,40e-6).unwrap());
        for (a,b) in single.iter().zip(duplicate) {assert_eq!(a.x,b.x);assert_eq!(a.q,b.q);}
        assert!((two.iter().map(|r|r.impulse).sum::<f64>()-one[0].impulse).abs()<1e-20);
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
    #[test]
    fn contact_backend_batches_match_native_and_reject_partial_results() {
        use crate::hair::{HairLinearSolver,HairLinearSystem,HairResponseSystem};
        struct Backend {calls:usize,corrupt:bool}
        impl HairLinearSolver for Backend {
            fn solve(&mut self,_:&[HairLinearSystem])->Result<Vec<Vec<f64>>, &'static str> {panic!("unexpected structural call")}
            fn solve_responses(&mut self,batch:&[HairResponseSystem])->Result<Vec<Vec<Vec<f64>>>, &'static str> {
                self.calls+=1;assert_eq!(batch.len(),2);
                let mut result=batch.iter().map(HairResponseSystem::solve_native).collect::<Result<Vec<_>,_>>()?;
                if self.corrupt {result[1].pop();}
                Ok(result)
            }
        }
        let rods=vec![rod(0.),rod(0.00004)];
        let make=|| {
            let mut constraints=Vec::new();
            for index in 0..2 {
                let zero=Entry {rod:index,point:0,gradient:[0.;3],mobility:0.};
                for fraction in [0.,1.] {
                    let pair=entries(index,1,fraction,[1.,0.,0.],&rods[index]);
                    add_constraint(&mut constraints,[pair[0],pair[1],zero,zero],0.).unwrap();
                }
            }
            constraints
        };
        let mut native=make();let mut batched=make();
        prepare_implicit_response(&mut native,&rods,1./240.).unwrap();
        let mut backend=Backend {calls:0,corrupt:false};
        prepare_implicit_response_with_solver(&mut batched,&rods,1./240.,Some(&mut backend)).unwrap();
        assert_eq!(backend.calls,1);
        for (a,b) in native.iter().zip(&batched) {
            assert_eq!(a.diagonal,b.diagonal);
            for (a,b) in a.response.iter().zip(&b.response) {assert_eq!(a.rod,b.rod);assert_eq!(a.linear,b.linear);assert_eq!(a.angular,b.angular);}
        }
        let mut rejected=make();backend.corrupt=true;
        assert!(prepare_implicit_response_with_solver(&mut rejected,&rods,1./240.,Some(&mut backend)).is_err());
        assert!(rejected.iter().all(|c|c.response.is_empty()));
    }

}

#[cfg(test)]
#[path="contact_position_fixture_tests.rs"]
mod captured_position_tests;

#[cfg(test)]
#[path="contact_component_fixture_tests.rs"]
mod captured_component_tests;
