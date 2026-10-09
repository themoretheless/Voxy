//! Staged structural motion guarded against swept strand collisions.
use super::*;
use crate::hair::contact::continuous::SweptPairCache;
#[path = "contact_swept_constraints.rs"]
mod swept_constraints;
#[path = "contact_line_certificate.rs"]
mod line_certificate;
use crate::hair::{
    CapsuleMotion, CapsuleSweep, CapsuleSweepOptions, sweep_capsules, swept_capsule_pairs,
};

/// Admit a prefix of a proposed rod motion. Prescribed roots are caller-owned.
/// Neighbor exclusions and the permitted follicle corner match static contacts.
pub(in crate::hair) fn strand_fraction(
    rods: &[HairRod],
    end: &[Vec<V>],
    radius: f64,
) -> Result<f64, &'static str> {
    Ok(strand_limits(rods, end, radius)?
        .iter()
        .map(|pair| pair.2)
        .fold(1., f64::min))
}
/// Independently admit the complete staged interval, including every later
/// mesh/contact correction. Safe endpoints alone cannot certify this path.
pub(in crate::hair) fn admit_staged_strands(rods: &[HairRod], radius: f64) -> Result<(), &'static str> {
    let mut start=rods.to_vec();
    for rod in &mut start {rod.x.clone_from(&rod.old_x);}
    let end:Vec<_>=rods.iter().map(|rod|rod.x.clone()).collect();
    if strand_fraction(&start,&end,radius)?!=1. {
        return Err("complete staged strand trajectory is not admitted");
    }
    Ok(())
}
fn strand_limits(
    rods: &[HairRod],
    end: &[Vec<V>],
    radius: f64,
) -> Result<Vec<(usize, usize, f64)>, &'static str> {
    strand_limits_cached(rods,end,radius,None)
}
fn strand_limits_cached(rods:&[HairRod],end:&[Vec<V>],radius:f64,cache:Option<&mut SweptPairCache>)->Result<Vec<(usize,usize,f64)>, &'static str> {
    strand_limits_with_workers(rods,end,radius,cache,None)
}
fn strand_limits_with_workers(rods:&[HairRod],end:&[Vec<V>],radius:f64,cache:Option<&mut SweptPairCache>,workers:Option<usize>)->Result<Vec<(usize,usize,f64)>, &'static str> {
    if rods.len() != end.len() || !radius.is_finite() || radius <= 0. {
        return Err("invalid swept strand motion");
    }
    let mut motions = Vec::new();
    let mut ids = Vec::new();
    for (r, (rod, points)) in rods.iter().zip(end).enumerate() {
        if rod.x.len() != points.len() {
            return Err("swept strand shape mismatch");
        }
        for i in 0..points.len() - 1 {
            ids.push((r, i));
            motions.push(CapsuleMotion {
                start: [rod.x[i], rod.x[i + 1]],
                end: [points[i], points[i + 1]],
                radius,
            });
        }
    }
    // Stop inside the static activation band (1e-10 m), so the next
    // Newton solve owns a newly reached contact instead of stopping forever
    // just outside the band's boundary.
    let options = CapsuleSweepOptions {
        tolerance_m: 5e-11,
        ..Default::default()
    };
    let trace_limits=std::env::var_os("VOXY_HAIR_SWEEP_LIMIT_TRACE").is_some();
    let pairs=match cache {Some(cache)=>cache.query(&motions,options.tolerance_m)?,None=>swept_capsule_pairs(&motions,options.tolerance_m)?};
    let workers=workers.unwrap_or_else(||if pairs.len()>=1024 && !trace_limits {
        std::thread::available_parallelism().map_or(1,usize::from).min(8)
    } else {1});
    ordered_query_results(&pairs,workers,|&(a,b)| {
        let (ra, ia) = ids[a];
        let (rb, ib) = ids[b];
        if ra == rb && ia.abs_diff(ib) <= 2 {
            return Ok(None);
        }
        let ma = motions[a];
        let mb = motions[b];
        let admit = |ma, mb| {
            let value = pair_fraction(ma, mb, options).map_err(|error| {
                eprintln!("HAIR SWEPT PAIR REJECT a=({ra},{ia}) b=({rb},{ib}) reason={error}");
                error
            })?;
            if trace_limits && value<1. {
                eprintln!("HAIR SWEPT LIMIT a=({ra},{ia}) b=({rb},{ib}) fraction={value} start_a={:?} start_b={:?} end_a={:?} end_b={:?}",ma.start,mb.start,ma.end,mb.end);
            }
            if value <= 0. {
                eprintln!(
                    "HAIR SWEPT PAIR ZERO a=({ra},{ia}) b=({rb},{ib}) start_a={:?} start_b={:?} end_a={:?} end_b={:?}",
                    ma.start, mb.start, ma.end, mb.end
                );
            }
            Ok::<f64, &'static str>(value)
        };
        let fraction = if ia == 0 && ib == 0 {
            // The permitted root/root corner is the only excluded region.
            admit(trim(ma), mb)?.min(admit(ma, trim(mb))?)
        } else {
            admit(ma, mb)?
        };
        if !fraction.is_finite() || fraction <= 0. {
            return Err("swept strand motion has no admissible progress");
        }
        Ok((fraction<1.).then_some((ra,rb,fraction)))
    })
}

// Independent geometry queries borrow immutable motion. Join in input order;
// the first rejected pair and every accepted fraction retain serial semantics.
fn ordered_query_results<T:Sync,R:Send>(pairs:&[T],workers:usize,
    evaluate:impl Fn(&T)->Result<Option<R>, &'static str>+Sync,
)->Result<Vec<R>, &'static str> {
    if workers<=1 || pairs.len()<2 {
        return pairs.iter().filter_map(|pair|evaluate(pair).transpose()).collect();
    }
    let chunk=pairs.len().div_ceil(workers.min(pairs.len()));
    let batches=std::thread::scope(|scope| {
        let evaluate=&evaluate;
        let handles:Vec<_>=pairs.chunks(chunk).map(|batch|scope.spawn(move ||
            batch.iter().filter_map(|pair|evaluate(pair).transpose()).collect::<Result<Vec<_>,_>>()
        )).collect();
        handles.into_iter().map(|handle|handle.join().expect("strand query worker panicked")).collect::<Vec<_>>()
    });
    let mut limits=Vec::new();
    for batch in batches {limits.extend(batch?);}
    Ok(limits)
}

// Canonical minimum-index roots keep grouping independent of pair traversal.
fn component(parent: &[usize], mut index: usize) -> usize {
    while parent[index] != index {
        index = parent[index];
    }
    index
}
fn join(parent: &mut [usize], a: usize, b: usize) -> bool {
    let a = component(parent, a);
    let b = component(parent, b);
    if a == b {
        return false;
    }
    parent[a.max(b)] = a.min(b);
    true
}
fn group_minimum(parent: &[usize], scales: &mut [f64]) {
    let mut minimum = vec![1f64; parent.len()];
    for (rod, &scale) in scales.iter().enumerate() {
        let root = component(parent, rod);
        minimum[root] = minimum[root].min(scale);
    }
    for (rod, scale) in scales.iter_mut().enumerate() {
        *scale = minimum[component(parent, rod)];
    }
}
pub(super) fn trust_components(
    rods: &[HairRod], increment: &PositionIncrement, responses: &[StrandResponse],
) -> Result<(Vec<usize>, Vec<f64>), &'static str> {
    if increment.angular.len()!=rods.len() || increment.linear.len()!=rods.len()
        || rods.iter().enumerate().any(|(r,rod)| increment.angular[r].len()!=rod.q.len()
            || increment.linear[r].len()!=rod.x.len()) {
        return Err("swept structural increment shape mismatch");
    }
    let mut parent: Vec<_>=(0..rods.len()).collect();
    for pair in responses {
        if pair.a.0>=rods.len() || pair.b.0>=rods.len() {
            return Err("swept contact component index out of range");
        }
        join(&mut parent,pair.a.0,pair.b.0);
    }
    let mut scales=Vec::with_capacity(rods.len());
    for angles in &increment.angular {
        if angles.iter().any(|angle| !finite(*angle)) {
            return Err("swept angular trust overflow");
        }
        let maximum=angles.iter().map(|angle|len(*angle)).fold(0.,f64::max);
        if !maximum.is_finite() {return Err("swept angular trust overflow");}
        scales.push(if maximum>0.35 {0.35/maximum} else {1.});
    }
    group_minimum(&parent,&mut scales);
    Ok((parent,scales))
}
/// A reduced component can create a collision with a formerly clear moving
/// neighbor. Re-query every proposed motion after every merge/reduction;
/// publish only when ALL final component trajectories admit their full step.
fn component_scales(
    rods: &[HairRod],
    increment: &PositionIncrement,
    responses: &[StrandResponse],
    radius: f64,
) -> Result<Vec<f64>, &'static str> {
    component_scales_from(rods, rods, increment, responses, radius)
}
// Search Newton scale space, not physical time. Root/staged displacement is
// affine and does not shrink with a free correction. Newly colliding neighbors
// must join the restricted component before another candidate is constructed.
fn checked_component_backtrack(
    rods: &[HairRod], start: &[HairRod], increment: &PositionIncrement,
    radius: f64, parent: &[usize], scales: &[f64],
    initial_limits: &[(usize,usize,f64)],cache:&mut SweptPairCache,
) -> Result<Option<Vec<f64>>, &'static str> {
    let mut parent=parent.to_vec();
    let mut restricted=vec![false;rods.len()];
    for &(a,b,_) in initial_limits {
        restricted[a]=true;restricted[b]=true;
    }
    let mut fraction=0.5;
    for _ in 0..32 {
        let mut groups=vec![false;rods.len()];
        for (r,&active) in restricted.iter().enumerate() {
            if active {groups[component(&parent,r)]=true;}
        }
        let mut candidate=scales.to_vec();
        for r in 0..rods.len() {
            if groups[component(&parent,r)] {candidate[r]*=fraction;}
        }
        group_minimum(&parent,&mut candidate);
        let end:Vec<Vec<V>>=rods.iter().enumerate().map(|(r,rod)|
            rod.x.iter().enumerate().map(|(p,&x)|add(x,mul(increment.linear[r][p],candidate[r]))).collect()).collect();
        let limits=strand_limits_cached(start,&end,radius,Some(cache))?;
        if limits.is_empty() {return Ok(Some(candidate));}
        let mut merged=false;
        for &(a,b,_) in &limits {
            merged|=join(&mut parent,a,b);
            restricted[a]=true;restricted[b]=true;
        }
        // Changing connectivity changes the candidate itself. Requery the
        // same scale after merging; only a rejected fixed graph shrinks it.
        if !merged {fraction*=0.5;}
    }
    Ok(None)
}

fn component_scales_from(
    rods: &[HairRod],
    start: &[HairRod],
    increment: &PositionIncrement,
    responses: &[StrandResponse],
    radius: f64,
) -> Result<Vec<f64>, &'static str> {
    if rods.len() != start.len() {
        return Err("swept structural start shape mismatch");
    }
    let (mut parent,mut scales)=trust_components(rods,increment,responses)?;
    let mut candidates=SweptPairCache::default();
    let mut zero=Vec::new();let mut full=Vec::new();
    for (r,(rod,old)) in rods.iter().zip(start).enumerate() {
        if rod.x.len()!=old.x.len() {return Err("swept strand shape mismatch");}
        for i in 0..rod.x.len()-1 {
            let base=CapsuleMotion {start:[old.x[i],old.x[i+1]],end:[rod.x[i],rod.x[i+1]],radius};
            zero.push(base);
            full.push(CapsuleMotion {end:std::array::from_fn(|p|add(base.end[p],mul(increment.linear[r][i+p],scales[r]))),..base});
        }
    }
    candidates.seed(&zero,&full,5e-11)?;

    let mut tried_scale_search=false;
    let trace_components=std::env::var_os("VOXY_HAIR_SWEEP_COMPONENT_TRACE").is_some();
    for iteration in 0..64 {
        let end: Vec<Vec<V>> = rods
            .iter()
            .enumerate()
            .map(|(r, rod)| {
                rod.x
                    .iter()
                    .enumerate()
                    .map(|(p, x)| add(*x, mul(increment.linear[r][p], scales[r])))
                    .collect()
            })
            .collect();
        let limits = strand_limits_cached(start,&end,radius,Some(&mut candidates))?;
        if trace_components {
            let limiting=limits.iter().min_by(|a,b|a.2.total_cmp(&b.2));
            eprintln!("HAIR COMPONENT CLOCK iteration={iteration} minimum_scale={} limits={} first={limiting:?}",scales.iter().copied().fold(1.,f64::min),limits.len());
        }
        if limits.is_empty() {
            return Ok(scales);
        }
        let mut merged = false;
        for &(a, b, _) in &limits {
            merged |= join(&mut parent, a, b);
        }
        if merged {
            // Previously independent motions changed their time scale. Their
            // old CCD fractions no longer describe the new common path.
            group_minimum(&parent, &mut scales);
            continue;
        }
        if !tried_scale_search {
            tried_scale_search=true;
            if let Some(candidate)=checked_component_backtrack(rods,start,increment,radius,&parent,&scales,&limits,&mut candidates)? {
                return Ok(candidate);
            }
        }
        let mut next = scales.clone();
        for &(a, _, fraction) in &limits {
            next[a] = next[a].min(scales[a] * fraction * 0.9);
        }
        group_minimum(&parent, &mut next);
        if next.iter().any(|value| !value.is_finite() || *value <= 0.) || next == scales {
            let limiting=limits.iter().min_by(|a,b|a.2.total_cmp(&b.2));
            eprintln!("HAIR SWEPT COMPONENT STALLED minimum_scale={} minimum_next={} limiting_pair={limiting:?} restricted_pairs={}",scales.iter().copied().fold(1.,f64::min),next.iter().copied().fold(1.,f64::min),limits.len());
            export_motion_failure(rods, start, increment, responses, radius);
            return Err("swept strand components have no admissible progress");
        }
        scales = next;
    }
    export_motion_failure(rods, start, increment, responses, radius);
    Err("swept strand component admission did not converge")
}

#[cold]
fn export_motion_failure(
    rods: &[HairRod],
    start: &[HairRod],
    increment: &PositionIncrement,
    responses: &[StrandResponse],
    radius: f64,
) {
    let Some(path) = std::env::var_os("VOXY_HAIR_ROOT_MOTION_FAILURE_EXPORT") else {
        return;
    };
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"VJR1");
    bytes.extend_from_slice(&(rods.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&radius.to_le_bytes());
    for (r, rod) in rods.iter().enumerate() {
        bytes.extend_from_slice(&(rod.x.len() as u32).to_le_bytes());
        for points in [
            &start[r].x,
            &rod.x,
            &increment.linear[r],
            &increment.angular[r],
        ] {
            for point in points {
                for value in point {
                    bytes.extend_from_slice(&value.to_le_bytes());
                }
            }
        }
    }
    bytes.extend_from_slice(&(responses.len() as u32).to_le_bytes());
    // Only rod connectivity enters component admission; geometry and full
    // free/angular candidate increments are captured above without rounding.
    for response in responses {
        bytes.extend_from_slice(&(response.a.0 as u32).to_le_bytes());
        bytes.extend_from_slice(&(response.b.0 as u32).to_le_bytes());
    }
    match std::fs::write(&path, &bytes) {
        Ok(()) => eprintln!(
            "HAIR ROOT MOTION FAILURE EXPORT {:?} rods={} bytes={}",
            path,
            rods.len(),
            bytes.len()
        ),
        Err(error) => eprintln!("HAIR ROOT MOTION FAILURE EXPORT ERROR {error}"),
    }
}

fn trim(mut motion: CapsuleMotion) -> CapsuleMotion {
    motion.start[0] = add(
        motion.start[0],
        mul(sub(motion.start[1], motion.start[0]), 0.2),
    );
    motion.end[0] = add(motion.end[0], mul(sub(motion.end[1], motion.end[0]), 0.2));
    motion
}
fn pair_fraction(
    a: CapsuleMotion,
    b: CapsuleMotion,
    options: CapsuleSweepOptions,
) -> Result<f64, &'static str> {
    let (outcome, certificate) = crate::hair::contact::continuous::sweep_capsules_selective(
        a, b, options, || line_certificate::clear(a, b, 512),
    )?;
    if outcome == CapsuleSweep::Clear { return Ok(1.); }
    // A failed certificate belongs to this exact immutable pair and must not
    // be evaluated again after resuming conservative advancement.
    if (certificate.is_none() && line_certificate::clear(a, b, 512))
        || interval_planes_clear(a, b, 32768)? { return Ok(1.); }
    match outcome {
        CapsuleSweep::Approach { fraction, .. } | CapsuleSweep::IterationLimit { fraction, .. }
            if fraction > 0. => Ok(fraction),
        _ => initial_support_fraction(a, b),
    }
}

#[cfg(test)]
fn pair_fraction_reference(
    a: CapsuleMotion,
    b: CapsuleMotion,
    options: CapsuleSweepOptions,
) -> Result<f64, &'static str> {
    let outcome = sweep_capsules(a, b, options)?;
    if outcome == CapsuleSweep::Clear {
        return Ok(1.);
    }
    // One initial plane can reject safe rotating near-contact trajectories.
    // A union of certified intervals may prove the complete path instead.
    if line_certificate::clear(a, b, 512) || interval_planes_clear(a, b, 32768)? {
        return Ok(1.);
    }
    match outcome {
        CapsuleSweep::Approach { fraction, .. } | CapsuleSweep::IterationLimit { fraction, .. }
            if fraction > 0. =>
        {
            Ok(fraction)
        }
        _ => initial_support_fraction(a, b),
    }
}
fn interval_planes_clear(
    a: CapsuleMotion,
    b: CapsuleMotion,
    budget: usize,
) -> Result<bool, &'static str> {
    let scale = a
        .start
        .iter()
        .chain(&a.end)
        .chain(&b.start)
        .chain(&b.end)
        .flatten()
        .map(|v| v.abs())
        .fold(a.radius + b.radius, f64::max);
    let margin = 512. * f64::EPSILON * scale;
    if margin >= 1e-10 {
        return Err("interval contact precision requires recentering");
    }
    let at = |m: CapsuleMotion, t: f64| {
        std::array::from_fn::<_, 2, _>(|p| add(m.start[p], mul(sub(m.end[p], m.start[p]), t)))
    };
    let mut stack = vec![(0f64, 1f64)];
    let mut visited = 0;
    while let Some((lo, hi)) = stack.pop() {
        visited += 1;
        if visited > budget {
            return Ok(false);
        }
        let mid = (lo + hi) * 0.5;
        let aa = at(a, mid);
        let bb = at(b, mid);
        let (_, _, p, q) = super::super::segment_pair(aa[0], aa[1], bb[0], bb[1]);
        let delta = sub(p, q);
        let distance = len(delta);
        if !distance.is_finite() {
            return Err("interval contact geometry overflow");
        }
        if distance < a.radius + b.radius - 1e-10 {
            return Ok(false);
        }
        if distance <= 0. {
            return Ok(false);
        }
        let normal = mul(delta, 1. / distance);
        let norm_upper = len(normal) * (1. + 8. * f64::EPSILON);
        let threshold = (a.radius + b.radius - 1e-10 + margin) * norm_upper;
        let certified = [lo, hi].iter().all(|&time| {
            let aa = at(a, time);
            let bb = at(b, time);
            aa.iter()
                .all(|p| bb.iter().all(|q| dot(sub(*p, *q), normal) >= threshold))
        });
        if certified {
            continue;
        }
        if mid <= lo || mid >= hi {
            return Ok(false);
        }
        // Each endpoint pair projection is affine in time; passing at both
        // ends bounds every barycentric capsule point throughout this interval.
        stack.push((mid, hi));
        stack.push((lo, mid));
    }
    Ok(true)
}
fn initial_support_fraction(a: CapsuleMotion, b: CapsuleMotion) -> Result<f64, &'static str> {
    let (_, _, p, q) = super::super::segment_pair(a.start[0], a.start[1], b.start[0], b.start[1]);
    let delta = sub(p, q);
    let distance = len(delta);
    if distance - a.radius - b.radius < -1e-10 {
        eprintln!("HAIR SWEPT INITIAL PENETRATION gap_m={} start_a={:?} start_b={:?}", distance-a.radius-b.radius,a.start,b.start);
        return Err("swept strand motion starts in penetration");
    }
    if distance <= 0. {
        return Err("swept strand initial contact has no separating normal");
    }
    let normal = mul(delta, 1. / distance);
    let scale = a
        .start
        .iter()
        .chain(&a.end)
        .chain(&b.start)
        .chain(&b.end)
        .flatten()
        .map(|v| v.abs())
        .fold(a.radius + b.radius, f64::max);
    let threshold = a.radius + b.radius - 1e-10 + 64. * f64::EPSILON * scale;
    // This plane bounds every convex barycentric point for the complete
    // linear prefix, rather than testing only the closest point at its end.
    let mut fraction = 1f64;
    for i in 0..2 {
        for j in 0..2 {
            let start = dot(sub(a.start[i], b.start[j]), normal);
            let end = dot(sub(a.end[i], b.end[j]), normal);
            if start < threshold {
                return Err("initial strand separating plane cannot certify motion");
            }
            if end < threshold {
                fraction = fraction.min((start - threshold) / (start - end));
            }
        }
    }
    Ok(fraction)
}

fn elastic_motion_limited(rods:&[HairRod],increment:&PositionIncrement,scales:&[f64])->Result<bool, &'static str> {
    for (r,rod) in rods.iter().enumerate() {
        if scales[r]>=1. {continue;}
        for (i,&rest) in rod.lengths.iter().enumerate() {
            let a=add(rod.x[i],mul(increment.linear[r][i],scales[r]));
            let b=add(rod.x[i+1],mul(increment.linear[r][i+1],scales[r]));
            let strain=len(sub(b,a))/rest-1.;
            if !strain.is_finite() {return Err("swept elastic candidate overflow");}
            if strain.abs()>0.05 {return Ok(true);}
        }
    }
    Ok(false)
}

fn contact_motion_limited(rods:&[HairRod],increment:&PositionIncrement,scales:&[f64],groups:&[StrandResponse])->Result<bool, &'static str> {
    if elastic_motion_limited(rods,increment,scales)? {return Ok(true);}
    let (_,trust)=trust_components(rods,increment,groups)?;
    // A collision prefix is not the end of the implicit solve. Activate the
    // blocking space-time witness even when its capped pose has little strain;
    // otherwise every later iteration stops at the same activation band.
    Ok(scales.iter().zip(trust).any(|(scale,trust)|*scale<trust))
}

pub(in crate::hair) fn advance(
    rods: &mut [HairRod],
    dt: f64,
    radius: f64,
    mut solver: Option<&mut dyn crate::hair::HairLinearSolver>,
    history: &mut Vec<StrandResponse>,
    start: Option<&[HairRod]>,
) -> Result<f64, &'static str> {
    let profile_started=std::env::var_os("VOXY_HAIR_SWEEP_REFINEMENT_TRACE").map(|_|std::time::Instant::now());
    let mut responses = super::super::refresh_strand_responses(rods, radius, &[]);
    let (mut constraints, aliases) = position_constraints(rods, &responses, radius)?;
    let native_step=if solver.as_ref().is_none_or(|backend|backend.joint_contact_coordinates_enabled()) {Some(NativeNewtonStep::new(rods,dt,1e-14)?)} else {None};
    let mut increment = if let Some(backend) = solver.as_mut() {
        if let Some(step)=&native_step {step.project_with_solver(&mut constraints,1e-14,Some(&mut **backend))?}
        else {constrained_newton_increment(&mut constraints, rods, dt, Some(&mut **backend), 1e-14)?}
    } else {
        native_step.as_ref().unwrap().project(&mut constraints,1e-14)?
    };
    let initial_projection_ms=profile_started.map(|started|started.elapsed().as_secs_f64()*1000.);
    let mut groups = responses.clone();
    let mut cuts = Vec::new();
    let mut admission = if let Some(previous) = start {
        component_scales_from(rods, previous, &increment, &groups, radius)
    } else {
        component_scales(rods, &increment, &groups, radius)
    };
    let initial_admission_ms=profile_started.map(|started|started.elapsed().as_secs_f64()*1000.-initial_projection_ms.unwrap());
    let mut refinement_count = 0;
    {
        let previous=start.unwrap_or(rods);
        for _ in 0..32 {
            let limited=match &admission {
                Ok(scales)=>contact_motion_limited(rods,&increment,scales,&groups)?,
                Err(_)=>false,
            };
            if !limited && !matches!(
                &admission,
                Err("swept strand component admission did not converge"
                    | "swept strand components have no admissible progress"
                    | "swept strand motion has no admissible progress")
            ) {
                break;
            }
            if !swept_constraints::activate(
                &mut constraints,
                rods,
                previous,
                &increment,
                &mut groups,
                &responses,
                &mut cuts,
                radius,
            )? {
                if limited {
                    export_motion_failure(rods,previous,&increment,&groups,radius);
                    if profile_started.is_some() {
                        eprintln!("HAIR SWEPT WITNESS STALLED cuts={} constraints={} groups={}",cuts.len(),constraints.len(),groups.len());
                    }
                }
                break;
            }
            refinement_count += 1;
            // Each projection starts from the free Newton solution. Old dual
            // reactions must not remain without their corresponding primal load.
            for constraint in &mut constraints {
                constraint.multiplier = 0.;
            }
            increment = if let Some(backend) = solver.as_mut() {
                if let Some(step)=&native_step {step.project_with_solver(&mut constraints,1e-14,Some(&mut **backend))?}
                else {constrained_newton_increment(
                    &mut constraints,
                    rods,
                    dt,
                    Some(&mut **backend),
                    1e-14,
                )?}
            } else {
                native_step.as_ref().unwrap().project(&mut constraints,1e-14)?
            };
            admission = component_scales_from(rods, previous, &increment, &groups, radius);
            if std::env::var_os("VOXY_HAIR_SWEEP_REFINEMENT_TRACE").is_some() {
                let status=match &admission {
                    Ok(scales)=>format!("fraction={}",scales.iter().copied().fold(1.,f64::min)),
                    Err(error)=>format!("error={error}"),
                };
                eprintln!("HAIR SWEPT REFINEMENT count={refinement_count} cuts={} constraints={} {status}",cuts.len(),constraints.len());
            }
        }
    }
    if refinement_count == 32 && match &admission {
        Ok(scales)=>contact_motion_limited(rods,&increment,scales,&groups)?,
        Err(_)=>true,
    } {
        export_motion_failure(rods,start.unwrap_or(rods),&increment,&groups,radius);
        return Err("swept contact refinement budget exhausted");
    }
    let scales = admission?;
    drop(native_step);
    if let Some(started)=profile_started {
        let (_,trust)=trust_components(rods,&increment,&groups)?;
        let collision_limited=scales.iter().zip(&trust).any(|(scale,limit)|scale<limit);
        let elastic_limited=elastic_motion_limited(rods,&increment,&scales)?;
        let maximum_free_translation=increment.linear.iter().flatten().map(|value|len(*value)).fold(0.,f64::max);
        let maximum_accepted_translation=increment.linear.iter().zip(&scales).flat_map(|(values,scale)|values.iter().map(move |value|len(*value)*scale)).fold(0.,f64::max);
        eprintln!("HAIR SWEPT COMPLETION collision_limited={collision_limited} elastic_limited={elastic_limited} minimum_trust={} maximum_free_translation_m={maximum_free_translation} maximum_accepted_translation_m={maximum_accepted_translation}",trust.iter().copied().fold(1.,f64::min));
        eprintln!("HAIR SWEPT SOLVE PROFILE elapsed_ms={} initial_projection_ms={} initial_admission_ms={} refinements={refinement_count} cuts={} constraints={} groups={} minimum_scale={}",
            started.elapsed().as_secs_f64()*1000.,initial_projection_ms.unwrap(),initial_admission_ms.unwrap(),cuts.len(),constraints.len(),groups.len(),scales.iter().copied().fold(1.,f64::min));
    }
    // Everything has been checked before publishing any pose. The enclosing
    // HairSystem transaction still owns nonlinear mesh/contact admission.
    for (r, rod) in rods.iter_mut().enumerate() {
        for p in 1..rod.x.len() {
            rod.x[p] = add(rod.x[p], mul(increment.linear[r][p], scales[r]));
            if p < rod.q.len() {
                apply(&mut rod.q[p], mul(increment.angular[r][p], scales[r]));
            }
        }
    }
    let mut multiplicity = vec![0usize; constraints.len()];
    for index in aliases.iter().flatten() {
        multiplicity[*index] += 1;
    }
    for (response, index) in responses.iter_mut().zip(aliases) {
        response.impulse = index.map_or(0., |index| {
            constraints[index].multiplier * scales[response.a.0] / multiplicity[index] as f64
        });
    }
    history.extend(responses);
    Ok(scales.into_iter().fold(1., f64::min))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hair::{HairLinearSolver, HairLinearSystem, HairMaterial, HairSystem, RootPose};
    #[test]
    fn safe_newton_segments_do_not_certify_the_chord_of_one_physical_interval() {
        let a=HairRod::new(vec![[-0.001,-0.001,0.],[-0.001,-0.001,0.1],[-0.001,-0.001,0.2]],HairMaterial::default()).unwrap();
        let b=HairRod::new(vec![[0.,0.,0.1],[0.,0.,0.16],[0.,0.,0.2]],HairMaterial::default()).unwrap();
        let start=vec![a,b];let mut waypoint=start.clone();
        for p in &mut waypoint[0].x[1..] {p[1]=0.001;}
        let mut end:Vec<_>=waypoint.iter().map(|r|r.x.clone()).collect();
        for p in &mut end[0][1..] {p[0]=0.001;}
        let middle:Vec<_>=waypoint.iter().map(|r|r.x.clone()).collect();
        assert_eq!(strand_fraction(&start,&middle,40e-6).unwrap(),1.);
        assert_eq!(strand_fraction(&waypoint,&end,40e-6).unwrap(),1.);
        assert!(strand_fraction(&start,&end,40e-6).unwrap()<1.,"iteration paths cannot replace the physical clock");
        let increment=PositionIncrement {
            linear:waypoint.iter().zip(&end).map(|(r,end)|r.x.iter().zip(end).map(|(a,b)|sub(*b,*a)).collect()).collect(),
            angular:waypoint.iter().map(|r|vec![[0.;3];r.q.len()]).collect(),
        };
        assert!(component_scales(&waypoint,&increment,&[],40e-6).unwrap().iter().all(|s|*s==1.));
        let scales=component_scales_from(&waypoint,&start,&increment,&[],40e-6).unwrap();
        assert!(scales.iter().any(|s|*s<1.));
        assert_eq!(strand_fraction(&start,&scaled_end(&waypoint,&increment,&scales),40e-6).unwrap(),1.);
    }
    #[test]
    fn collision_limited_low_strain_step_activates_reactions_before_publication() {
        let radius=40e-6;
        let mut rods:Vec<_>=[-0.5e-3,0.5e-3].into_iter().map(|x|HairRod::new(
            vec![[x,0.,0.],[x,0.,0.1],[x,0.,0.2]],HairMaterial::default()).unwrap()).collect();
        for (r,rod) in rods.iter_mut().enumerate() {for p in &mut rod.predicted_x[1..] {p[0]+=if r==0 {0.55e-3} else {-0.55e-3};}}
        let start=rods.clone();
        let free=constrained_newton_increment(&mut [],&rods,1./240.,None,1e-14).unwrap();
        let scales=component_scales(&rods,&free,&[],radius).unwrap();
        assert!(scales.iter().any(|scale|*scale<1.),"fixture must expose an unseen closing pair");
        assert!(!elastic_motion_limited(&rods,&free,&scales).unwrap(),"the old strain-only refinement rule misses this case");
        let mut history=Vec::new();
        assert_eq!(advance(&mut rods,1./240.,radius,None,&mut history,None).unwrap(),1.,"a collision prefix is not a solved contact step");
        let end:Vec<_>=rods.iter().map(|rod|rod.x.clone()).collect();
        assert_eq!(strand_fraction(&start,&end,radius).unwrap(),1.);
        let pairs=crate::hair::contact::refresh_strand_responses(&mut rods,radius,&[]);
        assert!(crate::hair::contact::strand_geometry_admitted(&rods,&pairs,radius).unwrap());
        for (rod,old) in rods.iter().zip(&start) {
            assert_eq!(rod.x[0],old.x[0]);
            assert!((rod.x[2][0]-old.x[2][0]).abs()>0.3e-3);
        }
    }
    fn rod(x: f64) -> HairRod {
        HairRod::new(
            vec![[x, 0., 0.], [x, 0.01, 0.], [x, 0.02, 0.]],
            HairMaterial::default(),
        )
        .unwrap()
    }
    #[test]
    fn checked_scale_search_merges_new_neighbors_and_preserves_independent_motion() {
        let rods=vec![rod(0.),rod(0.001),rod(-0.001),rod(1.)];
        let mut increment=increment_for(&rods);
        for (r,dx) in [0.01,0.006,0.006,0.01].into_iter().enumerate() {
            for p in 1..3 {increment.linear[r][p][0]=dx;}
        }
        let original=strand_limits(&rods,&scaled_end(&rods,&increment,&[1.;4]),40e-6).unwrap();
        assert!(original.iter().any(|&(a,b,_)|a==0&&b==1));
        assert!(original.iter().all(|&(a,b,_)|a!=2&&b!=2));
        let parent:Vec<_>=(0..4).collect();
        let scales=checked_component_backtrack(&rods,&rods,&increment,40e-6,&parent,&[1.;4],&original,&mut SweptPairCache::default()).unwrap().unwrap();
        assert!(scales[2]<1.,"reducing the first pair introduces a moving neighbor");
        assert_eq!(scales[3],1.,"disconnected guide retains its free motion");
        assert_eq!(strand_fraction(&rods,&scaled_end(&rods,&increment,&scales),40e-6).unwrap(),1.);
        for (r,end) in scaled_end(&rods,&increment,&scales).iter().enumerate() {assert_eq!(end[0],rods[r].x[0]);}
    }
    #[test]
    fn reduced_free_motion_cannot_hide_prescribed_root_compression() {
        let mut rods=vec![rod(0.)];
        rods[0].x[0][1]+=0.002;
        let increment=PositionIncrement {
            linear:vec![vec![[0.;3],[0.,0.002,0.],[0.,0.002,0.]]],
            angular:vec![vec![[0.;3];2]],
        };
        assert!(elastic_motion_limited(&rods,&increment,&[0.001]).unwrap());
        assert!(!elastic_motion_limited(&rods,&increment,&[0.9]).unwrap());
        assert!(!elastic_motion_limited(&rods,&increment,&[1.]).unwrap());
        assert_eq!(increment.linear[0][0],[0.;3]);
    }
    fn roots(system: &HairSystem) -> Vec<RootPose> {
        system
            .rods()
            .iter()
            .map(|rod| RootPose {
                position: rod.x[0],
                rotation: [0., 0., 0., 1.],
            })
            .collect()
    }
    fn increment_for(rods: &[HairRod]) -> PositionIncrement {
        PositionIncrement {
            linear: rods.iter().map(|r| vec![[0.; 3]; r.x.len()]).collect(),
            angular: rods.iter().map(|r| vec![[0.; 3]; r.q.len()]).collect(),
        }
    }
    fn scaled_end(rods: &[HairRod], increment: &PositionIncrement, scales: &[f64]) -> Vec<Vec<V>> {
        rods.iter()
            .enumerate()
            .map(|(r, rod)| {
                rod.x
                    .iter()
                    .enumerate()
                    .map(|(p, x)| add(*x, mul(increment.linear[r][p], scales[r])))
                    .collect()
            })
            .collect()
    }
    #[test]
    #[ignore = "requires a captured joint-root motion fixture"]
    fn captured_witness_sampling_matches_before_change_reference() {
        use std::io::Read;
        fn integer(reader: &mut std::io::Cursor<Vec<u8>>) -> usize {
            let mut b = [0; 4];
            reader.read_exact(&mut b).unwrap();
            u32::from_le_bytes(b) as usize
        }
        fn scalar(reader: &mut std::io::Cursor<Vec<u8>>) -> f64 {
            let mut b = [0; 8];
            reader.read_exact(&mut b).unwrap();
            let value = f64::from_le_bytes(b);
            assert!(value.is_finite());
            value
        }
        fn points(reader: &mut std::io::Cursor<Vec<u8>>, count: usize) -> Vec<V> {
            (0..count)
                .map(|_| std::array::from_fn(|_| scalar(reader)))
                .collect()
        }
        let path =
            std::env::var("VOXY_HAIR_ROOT_MOTION_FAILURE_FIXTURE").expect("root motion fixture");
        let mut reader = std::io::Cursor::new(std::fs::read(path).unwrap());
        let mut magic = [0; 4];
        reader.read_exact(&mut magic).unwrap();
        assert_eq!(&magic, b"VJR1");
        let count = integer(&mut reader);
        assert!((1..=8192).contains(&count));
        let radius = scalar(&mut reader);
        assert!(radius > 0.);
        let mut rods = Vec::new();
        let mut start = Vec::new();
        let mut increment = PositionIncrement {
            linear: Vec::new(),
            angular: Vec::new(),
        };
        for _ in 0..count {
            let n = integer(&mut reader);
            assert!((3..=1024).contains(&n));
            let previous = points(&mut reader, n);
            let current = points(&mut reader, n);
            let rod = HairRod::new(current, HairMaterial::default()).unwrap();
            let mut old = rod.clone();
            old.x = previous;
            rods.push(rod);
            start.push(old);
            increment.linear.push(points(&mut reader, n));
            increment.angular.push(points(&mut reader, n - 1));
        }
        let pairs = integer(&mut reader);
        assert!(pairs <= 1_000_000);
        let mut responses = Vec::new();
        for _ in 0..pairs {
            let a = integer(&mut reader);
            let b = integer(&mut reader);
            assert!(a < count && b < count);
            responses.push(StrandResponse {
                a: (a, 1, 0.),
                b: (b, 1, 0.),
                normal: [1., 0., 0.],
                impulse: 0.,
            });
        }
        assert_eq!(reader.position() as usize, reader.get_ref().len());
        let mut timings=Vec::new();
        for repeat in 0..3 {
            let run=|mode| {
                let mut groups=responses.clone();
                let base=Vec::new();let mut cuts=Vec::new();
                let mut constraints=Vec::new();let began=std::time::Instant::now();
                let changed=match mode {
                    0=>swept_constraints::activate_reference(&mut constraints,&rods,&start,&increment,&mut groups,&base,&mut cuts,radius),
                    1=>swept_constraints::activate_with_workers(&mut constraints,&rods,&start,&increment,&mut groups,&base,&mut cuts,radius,Some(1)),
                    _=>swept_constraints::activate_with_workers(&mut constraints,&rods,&start,&increment,&mut groups,&base,&mut cuts,radius,Some(8)),
                }.unwrap();
                let seconds=began.elapsed().as_secs_f64();
                let signature:Vec<_>=constraints.iter().map(|c|(c.bound.to_bits(),c.entries.iter().map(|e|
                    (e.rod,e.point,e.gradient.map(f64::to_bits))).collect::<Vec<_>>())).collect();
                let groups:Vec<_>=groups.iter().map(|g|
                    ((g.a.0,g.a.1,g.a.2.to_bits()),(g.b.0,g.b.1,g.b.2.to_bits()),g.normal.map(f64::to_bits),g.impulse.to_bits())).collect();
                (changed,(signature,groups),seconds)
            };
            let (old,new)=if repeat%2==0 {(run(0),run(2))} else {let new=run(2);(run(0),new)};
            let serial=run(1);
            assert_eq!(old.0,new.0);assert_eq!(old.1,new.1);
            assert_eq!(serial.0,new.0);assert_eq!(serial.1,new.1);
            timings.push((old.2,serial.2,new.2));
        }
        eprintln!("full-groom witness original/current-serial/current-parallel seconds={timings:?}");
    }
    #[test]
    fn parallel_pair_queries_preserve_order_and_first_rejection() {
        let pairs:Vec<_>=(0..4096).collect();
        let evaluate=|&id:&usize|Ok((id%3==0).then_some((id,id+1,(id as f64+1.)/4097.)));
        let expected=ordered_query_results(&pairs,1,evaluate).unwrap();
        for workers in [2,4,8] {
            assert_eq!(ordered_query_results(&pairs,workers,evaluate).unwrap(),expected);
            let rejected=ordered_query_results(&pairs,workers,|&id|
                if id==31 {Err("first rejection")} else if id==2048 {Err("later rejection")} else {evaluate(&id)});
            assert_eq!(rejected,Err("first rejection"));
        }
    }
    #[test]
    #[ignore = "requires a captured joint-root motion fixture"]
    fn captured_joint_root_motion_admission() {
        use std::io::Read;
        fn integer(reader: &mut std::io::Cursor<Vec<u8>>) -> usize {
            let mut b = [0; 4];
            reader.read_exact(&mut b).unwrap();
            u32::from_le_bytes(b) as usize
        }
        fn scalar(reader: &mut std::io::Cursor<Vec<u8>>) -> f64 {
            let mut b = [0; 8];
            reader.read_exact(&mut b).unwrap();
            let value = f64::from_le_bytes(b);
            assert!(value.is_finite());
            value
        }
        fn points(reader: &mut std::io::Cursor<Vec<u8>>, count: usize) -> Vec<V> {
            (0..count)
                .map(|_| std::array::from_fn(|_| scalar(reader)))
                .collect()
        }
        let path =
            std::env::var("VOXY_HAIR_ROOT_MOTION_FAILURE_FIXTURE").expect("root motion fixture");
        let mut reader = std::io::Cursor::new(std::fs::read(path).unwrap());
        let mut magic = [0; 4];
        reader.read_exact(&mut magic).unwrap();
        assert_eq!(&magic, b"VJR1");
        let count = integer(&mut reader);
        assert!((1..=8192).contains(&count));
        let radius = scalar(&mut reader);
        assert!(radius > 0.);
        let mut rods = Vec::new();
        let mut start = Vec::new();
        let mut increment = PositionIncrement {
            linear: Vec::new(),
            angular: Vec::new(),
        };
        for _ in 0..count {
            let n = integer(&mut reader);
            assert!((3..=1024).contains(&n));
            let previous = points(&mut reader, n);
            let current = points(&mut reader, n);
            let rod = HairRod::new(current, HairMaterial::default()).unwrap();
            let mut old = rod.clone();
            old.x = previous;
            rods.push(rod);
            start.push(old);
            increment.linear.push(points(&mut reader, n));
            increment.angular.push(points(&mut reader, n - 1));
        }
        let pairs = integer(&mut reader);
        assert!(pairs <= 1_000_000);
        let mut responses = Vec::new();
        for _ in 0..pairs {
            let a = integer(&mut reader);
            let b = integer(&mut reader);
            assert!(a < count && b < count);
            responses.push(StrandResponse {
                a: (a, 1, 0.),
                b: (b, 1, 0.),
                normal: [1., 0., 0.],
                impulse: 0.,
            });
        }
        assert_eq!(reader.position() as usize, reader.get_ref().len());
        if std::env::var_os("VOXY_HAIR_SWEEP_COMPONENT_TRACE").is_some() {
            for fraction in [0.,0.000001,0.00001,0.001,0.1,0.5,1.] {
                let end=scaled_end(&rods,&increment,&vec![fraction;rods.len()]);
                let limits=strand_limits(&start,&end,radius);
                eprintln!("HAIR CAPTURE SCALE {fraction} limits={:?}",limits.as_ref().map(|v| (v.len(),v.iter().min_by(|a,b|a.2.total_cmp(&b.2)))));
            }
        }
        // Pair-list equality alone does not qualify the physical clock.
        // Compare complete narrow-phase fractions (or identical rejection)
        // with fresh queries on the captured old/staged/free trajectories.
        let full_end=scaled_end(&rods,&increment,&vec![1.;rods.len()]);
        let mut zero=Vec::new();let mut full=Vec::new();
        for (r,rod) in rods.iter().enumerate() {
            for i in 0..rod.x.len()-1 {
                let motion=CapsuleMotion {start:[start[r].x[i],start[r].x[i+1]],end:[rod.x[i],rod.x[i+1]],radius};
                zero.push(motion);full.push(CapsuleMotion {end:[full_end[r][i],full_end[r][i+1]],..motion});
            }
        }
        let mut cache=SweptPairCache::default();cache.seed(&zero,&full,5e-11).unwrap();
        for fraction in [1.,0.5,0.25,0.125,0.0625,0.03125,0.00001,0.] {
            let end=scaled_end(&rods,&increment,&vec![fraction;rods.len()]);
            let began=std::time::Instant::now();
            let serial=strand_limits_with_workers(&start,&end,radius,None,Some(1));
            let serial_seconds=began.elapsed().as_secs_f64();
            for workers in [2,4,8] {
                let began=std::time::Instant::now();
                let parallel=strand_limits_with_workers(&start,&end,radius,Some(&mut cache),Some(workers));
                assert_eq!(parallel,serial,"parallel physical clock changed at scale {fraction} workers {workers}");
                let parallel_seconds=began.elapsed().as_secs_f64();
                let began=std::time::Instant::now();
                let cached_serial=strand_limits_with_workers(&start,&end,radius,Some(&mut cache),Some(1));
                let cached_serial_seconds=began.elapsed().as_secs_f64();
                assert_eq!(cached_serial,serial);
                eprintln!("captured physical clock scale={fraction} workers={workers} serial_seconds={serial_seconds} cached_serial_seconds={cached_serial_seconds} parallel_cached_seconds={parallel_seconds}");
            }
        }
        let scales = component_scales_from(&rods, &start, &increment, &responses, radius)
            .expect("captured joint root motion must admit a full checked path");
        let end = scaled_end(&rods, &increment, &scales);
        assert_eq!(strand_fraction(&start, &end, radius).unwrap(), 1.);
        for (r, points) in end.iter().enumerate() {
            assert_eq!(points[0], rods[r].x[0]);
        }
    }
    #[test]
    fn interval_planes_do_not_admit_a_tunnel_or_exhausted_query() {
        let a = CapsuleMotion {
            start: [[-0.001, 0., 0.], [-0.001, 0.01, 0.]],
            end: [[0.001, 0., 0.], [0.001, 0.01, 0.]],
            radius: 40e-6,
        };
        let b = CapsuleMotion {
            start: [[0., 0., 0.], [0., 0.01, 0.]],
            end: [[0., 0., 0.], [0., 0.01, 0.]],
            radius: 40e-6,
        };
        assert!(!interval_planes_clear(a, b, 32768).unwrap());
        assert!(!interval_planes_clear(a, b, 0).unwrap());
        let mut clear = a;
        clear.end = clear.start;
        assert!(interval_planes_clear(clear, b, 1).unwrap());
    }
    #[test]
    fn root_and_free_motion_are_admitted_as_one_path() {
        let start = vec![rod(0.), rod(0.001)];
        let mut staged = start.clone();
        for rod in &mut staged {
            rod.x[0][0] += 0.2;
        }
        let root_only: Vec<_> = staged.iter().map(|rod| rod.x.clone()).collect();
        assert!(strand_fraction(&start, &root_only, 40e-6).unwrap() < 1.);
        let mut increment = increment_for(&staged);
        for r in 0..2 {
            for p in 1..3 {
                increment.linear[r][p][0] = 0.2;
            }
        }
        let scales = component_scales_from(&staged, &start, &increment, &[], 40e-6).unwrap();
        assert_eq!(scales, vec![1., 1.]);
        let end = scaled_end(&staged, &increment, &scales);
        assert_eq!(strand_fraction(&start, &end, 40e-6).unwrap(), 1.);
        for (r, points) in end.iter().enumerate() {
            assert_eq!(points[0], staged[r].x[0]);
        }
    }
    #[test]
    fn runtime_moving_roots_remain_exact_and_elastic_nodes_follow() {
        let mut system = HairSystem::new(vec![rod(0.), rod(0.05)]).unwrap();
        system.joint_contact_positions = true;
        system.swept_strand_positions = true;
        system.substeps = 2;
        system.iterations = 6;
        let mut attachment = roots(&system);
        for frame in 1..=8 {
            attachment[0].position[0] = frame as f64 * 0.00005;
            attachment[1].position[0] = 0.05 + frame as f64 * 0.00005;
            system
                .step(1. / 120., &attachment, [0.; 3], [0.; 3], &[])
                .unwrap();
            for (rod, root) in system.rods().iter().zip(&attachment) {
                assert_eq!(rod.x[0], root.position);
                assert!(rod.max_relative_stretch() < 0.05);
            }
        }
        assert!(system.rods()[0].x[1][0] > 0.0003);
    }
    #[test]
    fn contact_group_does_not_slow_an_independent_guide() {
        let rods = vec![rod(0.), rod(0.001), rod(0.05)];
        let mut increment = increment_for(&rods);
        for p in 1..3 {
            increment.linear[0][p][0] = 0.002;
            increment.linear[2][p][0] = 0.002;
        }
        let scales = component_scales(&rods, &increment, &[], 40e-6).unwrap();
        assert!(scales[0] > 0. && scales[0] < 1.);
        assert_eq!(scales[0], scales[1]);
        assert_eq!(scales[2], 1.);
        assert_eq!(
            strand_fraction(&rods, &scaled_end(&rods, &increment, &scales), 40e-6).unwrap(),
            1.
        );
    }
    #[test]
    fn reducing_one_group_rechecks_and_merges_a_new_neighbor_collision() {
        let rods = vec![rod(0.), rod(0.001), rod(0.0012)];
        let mut increment = increment_for(&rods);
        for p in 1..3 {
            increment.linear[1][p][0] = -0.002;
            increment.linear[2][p][0] = -0.001;
        }
        let full = scaled_end(&rods, &increment, &[1.; 3]);
        let initial = strand_limits(&rods, &full, 40e-6).unwrap();
        assert!(initial.iter().any(|&(a, b, _)| a == 0 && b == 1));
        assert!(!initial.iter().any(|&(a, b, _)| a == 1 && b == 2));
        let blocked = strand_fraction(&rods[..2], &full[..2], 40e-6).unwrap() * 0.9;
        let reduced = scaled_end(&rods, &increment, &[blocked, blocked, 1.]);
        assert!(
            strand_limits(&rods, &reduced, 40e-6)
                .unwrap()
                .iter()
                .any(|&(a, b, _)| a == 1 && b == 2)
        );
        let scales = component_scales(&rods, &increment, &[], 40e-6).unwrap();
        assert_eq!(scales[0], scales[1]);
        assert_eq!(scales[1], scales[2]);
        assert!(scales[2] < 1.);
        assert_eq!(
            strand_fraction(&rods, &scaled_end(&rods, &increment, &scales), 40e-6).unwrap(),
            1.
        );
    }
    #[test]
    fn angular_trust_region_is_local_to_the_contact_component() {
        let rods = vec![rod(0.), rod(0.05), rod(0.1)];
        let mut increment = increment_for(&rods);
        increment.angular[2][1] = [0., 0., 1.];
        let scales = component_scales(&rods, &increment, &[], 40e-6).unwrap();
        assert_eq!(scales, vec![1., 1., 0.35]);
        let pairs = [StrandResponse {
            a: (1, 1, 0.),
            b: (2, 1, 0.),
            normal: [-1., 0., 0.],
            impulse: 0.,
        }];
        let scales = component_scales(&rods, &increment, &pairs, 40e-6).unwrap();
        assert_eq!(scales, vec![1., 0.35, 0.35]);
    }
    #[test]
    fn initial_plane_allows_opening_and_sliding_but_bounds_closing() {
        let a = CapsuleMotion {
            start: [[0., 0., 0.], [0., 0.01, 0.]],
            end: [[-0.001, 0.001, 0.], [-0.001, 0.011, 0.]],
            radius: 40e-6,
        };
        let b = CapsuleMotion {
            start: [[80e-6, 0., 0.], [80e-6, 0.01, 0.]],
            end: [[80e-6, 0., 0.], [80e-6, 0.01, 0.]],
            radius: 40e-6,
        };
        assert_eq!(pair_fraction(a, b, Default::default()).unwrap(), 1.);
        let mut sliding = a;
        sliding.end = [[0., 0.001, 0.], [0., 0.011, 0.]];
        assert_eq!(pair_fraction(sliding, b, Default::default()).unwrap(), 1.);
        let mut closing = a;
        closing.end = [[0.001, 0., 0.], [0.001, 0.01, 0.]];
        let fraction = pair_fraction(closing, b, Default::default()).unwrap();
        assert!(fraction > 0. && fraction < 1e-6);
        let delta = sub(closing.end[0], closing.start[0]);
        let moved = add(closing.start[0], mul(delta, fraction));
        assert!(b.start[0][0] - moved[0] >= 80e-6 - 1e-10);
    }
    #[test]
    fn opening_pair_above_initial_band_does_not_return_zero_progress() {
        let a = CapsuleMotion {
            start: [[0., 0., 0.], [0., 0.01, 0.]],
            end: [[-0.001, 0., 0.], [-0.001, 0.01, 0.]],
            radius: 40e-6,
        };
        let b = CapsuleMotion {
            start: [[80e-6 + 9e-11, 0., 0.], [80e-6 + 9e-11, 0.01, 0.]],
            end: [[80e-6 + 9e-11, 0., 0.], [80e-6 + 9e-11, 0.01, 0.]],
            radius: 40e-6,
        };
        let options = CapsuleSweepOptions {
            tolerance_m: 5e-11,
            ..Default::default()
        };
        assert!(matches!(
            sweep_capsules(a, b, options).unwrap(),
            CapsuleSweep::Approach { fraction: 0., .. }
        ));
        assert_eq!(pair_fraction(a, b, options).unwrap(), 1.);
    }
    #[test]
    fn stopped_new_collision_enters_the_next_contact_active_set() {
        let mut rods = vec![rod(0.), rod(0.001)];
        let mut end: Vec<_> = rods.iter().map(|r| r.x.clone()).collect();
        for point in 1..3 {
            end[0][point][0] += 0.002;
        }
        let fraction = strand_fraction(&rods, &end, 40e-6).unwrap();
        assert!(fraction > 0. && fraction < 1.);
        for (rod, target) in rods.iter_mut().zip(end) {
            for (point, t) in rod.x.iter_mut().zip(target) {
                *point = add(*point, mul(sub(t, *point), fraction));
            }
        }
        let contacts = super::super::super::refresh_strand_responses(&mut rods, 40e-6, &[]);
        assert!(
            !contacts.is_empty(),
            "CCD must stop inside the next contact activation band"
        );
        let (constraints, _) = position_constraints(&rods, &contacts, 40e-6).unwrap();
        assert!(!constraints.is_empty());
    }
    #[test]
    fn follicle_corner_permission_keeps_distal_crossings_visible() {
        let a = HairRod::new(
            vec![[0., 0., 0.], [-0.01, 0.01, 0.], [-0.02, 0.02, 0.]],
            HairMaterial::default(),
        )
        .unwrap();
        let b = HairRod::new(
            vec![[0., 0., 0.], [0.01, 0.01, 0.], [0.02, 0.02, 0.]],
            HairMaterial::default(),
        )
        .unwrap();
        let rods = vec![a, b];
        let mut end: Vec<_> = rods.iter().map(|r| r.x.clone()).collect();
        assert_eq!(strand_fraction(&rods, &end, 40e-6).unwrap(), 1.);
        end[0][1] = [0.015, 0.01, 0.];
        let fraction = strand_fraction(&rods, &end, 40e-6).unwrap();
        assert!(fraction > 0. && fraction < 1.);
    }
    #[test]
    fn runtime_swept_mode_uses_response_backend_and_preserves_exact_roots() {
        struct ResponsesOnly;
        impl HairLinearSolver for ResponsesOnly {
            fn solve(&mut self, _: &[HairLinearSystem]) -> Result<Vec<Vec<f64>>, &'static str> {
                panic!("unconstrained structural bypass");
            }
            fn contact_responses_enabled(&self) -> bool {
                true
            }
        }
        let mut native = HairSystem::new(vec![rod(0.), rod(0.05)]).unwrap();
        native.joint_contact_positions = true;
        native.swept_strand_positions = true;
        native.joint_contact_velocities = true;
        native.iterations = 6;
        native.substeps = 2;
        let attachment = roots(&native);
        let mut external = native.clone();
        for _ in 0..8 {
            native
                .step(1. / 120., &attachment, [0., -9.81, 0.], [0.1, 0., 0.], &[])
                .unwrap();
            external
                .step_with_solver(
                    1. / 120.,
                    &attachment,
                    [0., -9.81, 0.],
                    [0.1, 0., 0.],
                    &[],
                    &mut ResponsesOnly,
                )
                .unwrap();
            for ((a, b), root) in native.rods().iter().zip(external.rods()).zip(&attachment) {
                assert_eq!(a.x, b.x);
                assert_eq!(a.q, b.q);
                assert_eq!(a.x[0], root.position);
                assert_eq!(a.q[0], a.rest_q[0]);
                assert!(a.max_relative_stretch() < 0.05);
            }
        }
        assert!(native.rods()[0].x[2][0] > 1e-9);
        let rebased = native
            .rebased(native.rods().iter().map(|r| r.rest_x.clone()).collect())
            .unwrap();
        assert!(rebased.swept_strand_positions);
    }
    #[test]
    fn penetrating_start_and_invalid_modes_roll_back_complete_runtime_state() {
        let mut system = HairSystem::new(vec![rod(0.), rod(20e-6)]).unwrap();
        system.joint_contact_positions = true;
        system.swept_strand_positions = true;
        let original = system.clone();
        let attachment = roots(&system);
        assert!(
            system
                .step(1. / 120., &attachment, [0., -9.81, 0.], [0.; 3], &[])
                .is_err()
        );
        for (a, b) in system.rods().iter().zip(original.rods()) {
            assert_eq!(a.x, b.x);
            assert_eq!(a.q, b.q);
            assert_eq!(a.velocity, b.velocity);
            assert_eq!(a.old_x, b.old_x);
        }
        system.terminal_contact_iterations = 1;
        assert_eq!(
            system
                .step(1. / 120., &attachment, [0.; 3], [0.; 3], &[])
                .unwrap_err(),
            "swept strand motion requires joint self contacts and no terminal structural bypass"
        );
    }
}
