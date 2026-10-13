//! Bounded recovery proposal for nonlinear continuous contact nonconvergence.
//! A feasible proposal never establishes tangent completion: the caller must
//! resume the ordinary coupled solver and its independent admission gates.
use super::*;
use crate::hair::{HairSystem, TriangleMesh, contact, mesh_motion};

// Opt-in wall-clock diagnostics. Stage end means scope exit, never admission.
struct RecoveryTiming {
    began:Option<std::time::Instant>,phase:&'static str,
    iteration:Option<usize>,backtrack:Option<usize>,
}
impl RecoveryTiming {
    fn new(enabled:bool,phase:&'static str,iteration:Option<usize>,backtrack:Option<usize>)->Self {
        let began=enabled.then(std::time::Instant::now);
        if enabled {eprintln!("HAIR RECOVERY TIMING event=begin phase={phase} iteration={iteration:?} backtrack={backtrack:?}");}
        Self {began,phase,iteration,backtrack}
    }
}
impl Drop for RecoveryTiming {
    fn drop(&mut self) {
        if let Some(began)=self.began {eprintln!("HAIR RECOVERY TIMING event=end phase={} iteration={:?} backtrack={:?} elapsed_ms={} admission=unclassified",
            self.phase,self.iteration,self.backtrack,began.elapsed().as_secs_f64()*1000.);}
    }
}

#[cfg(test)]
std::thread_local! {static REFRESH_CALLS:std::cell::Cell<usize>=const {std::cell::Cell::new(0)};}
fn refresh(rods:&mut [HairRod],meshes:&[TriangleMesh],radius:f64)->Result<Vec<StrandResponse>,&'static str> {
    #[cfg(test)]
    REFRESH_CALLS.with(|count|count.set(count.get()+1));
    let pairs=contact::refresh_strand_responses(rods,radius,&[]);
    HairSystem::refresh_mesh_geometry(rods,meshes,radius,true)?;
    Ok(pairs)
}

/// Transactional: failed recovery leaves every caller-owned rod untouched.
pub(in crate::hair) fn restore_candidate(rods:&mut [HairRod],meshes:&[TriangleMesh],dt:f64,radius:f64,
    solver:Option<&mut dyn crate::hair::HairLinearSolver>)
    ->Result<bool,&'static str> {
    let preserve=contact::admit_staged_strands(rods,radius).is_ok();
    restore_candidate_mode(rods,meshes,dt,radius,solver,preserve,None)
}

#[cfg(test)]
pub(super) fn restore_preserving_clear_strands(rods:&mut [HairRod],meshes:&[TriangleMesh],dt:f64,radius:f64)
    ->Result<bool,&'static str> {
    restore_candidate_mode(rods,meshes,dt,radius,None,true,None)
}

pub(in crate::hair) fn restore_second_order_candidate(rods:&mut [HairRod],meshes:&[TriangleMesh],dt:f64,radius:f64,
    reference:&[(Vec<V>,Vec<Q>)],solver:Option<&mut dyn crate::hair::HairLinearSolver>)->Result<bool,&'static str> {
    restore_candidate_mode(rods,meshes,dt,radius,solver,true,Some(reference))
}

fn restore_candidate_mode(rods:&mut [HairRod],meshes:&[TriangleMesh],dt:f64,radius:f64,
    solver:Option<&mut dyn crate::hair::HairLinearSolver>,preserve_clear_strands:bool,
    energy_reference:Option<&[(Vec<V>,Vec<Q>)]>)
    ->Result<bool,&'static str> {
    restore_candidate_mode_impl::<true>(rods,meshes,dt,radius,solver,preserve_clear_strands,energy_reference)
}
fn restore_candidate_mode_impl<const REUSE:bool>(rods:&mut [HairRod],meshes:&[TriangleMesh],dt:f64,radius:f64,
    mut solver:Option<&mut dyn crate::hair::HairLinearSolver>,preserve_clear_strands:bool,
    energy_reference:Option<&[(Vec<V>,Vec<Q>)]>)->Result<bool,&'static str> {
    let timing=std::env::var_os("VOXY_HAIR_RECOVERY_TIMING_TRACE").is_some();
    let _total=RecoveryTiming::new(timing,"recovery",None,None);
    let origin=rods.to_vec();
    let before=energy_reference.map_or_else(||origin.iter().map(|rod|(rod.x.clone(),rod.q.clone())).collect::<Vec<_>>(),|reference|reference.to_vec());
    let mut start=origin.clone();
    for rod in &mut start {rod.x.clone_from(&rod.old_x);rod.q.clone_from(&rod.old_q);}
    let mut working=origin.clone();
    let mut refreshed={let _stage=RecoveryTiming::new(timing,"initial_refresh",None,None);
        Some(refresh(&mut working,meshes,radius)?)};
    if energy_reference.is_none() {
        let _stage=RecoveryTiming::new(timing,"initial_advancement",None,None);
        safe_motion::advance_contacts_with_pairs(&mut working,dt,radius,None,&mut Vec::new(),Some(&start),&[])?;
        refreshed=None; // Advancement can change the motion; no cached geometry survives.
    }
    if preserve_clear_strands {
        contact::admit_staged_strands(&working,radius)?;
    }
    let trace=std::env::var_os("VOXY_HAIR_NONLINEAR_MERIT_TRACE").is_some();
    let phases:&[bool]=if energy_reference.is_some() {&[true]} else {&[false,true]};
    let limit=if energy_reference.is_some() {8} else {64};
    for &zero_free in phases {
        let mut merits=Vec::with_capacity(limit+1);
        let mut changes=Vec::with_capacity(limit+1);
        for iteration in 0..=limit {
            let _iteration=RecoveryTiming::new(timing,if zero_free {"geometry_iteration"} else {"elastic_iteration"},Some(iteration),None);
            let pairs={let _stage=RecoveryTiming::new(timing,"iteration_refresh",Some(iteration),None);
                if REUSE {match refreshed.take() {
                Some(pairs)=>pairs,
                None=>refresh(&mut working,meshes,radius)?,
            }} else {refresh(&mut working,meshes,radius)?}};
            let (mut rows,path_merit)={let _stage=RecoveryTiming::new(timing,"restoration_model",Some(iteration),None);
                safe_motion::restoration_model(&working,&start,&pairs,radius)?};
            let merit=HairSystem::contact_merit(&working,&pairs,radius)?+path_merit;
            let work=HairSystem::elastic_energy_change(&working,&before,dt)?;
            let strain=working.iter().map(HairRod::max_relative_stretch).fold(0.,f64::max);
            if trace {eprintln!("HAIR CONTACT RECOVERY zero_free={zero_free} iteration={iteration} merit={merit:e} work_j={work:e}");}
            let admitted={let _stage=RecoveryTiming::new(timing,"whole_motion_admission",Some(iteration),None);
                merit==0. && (energy_reference.is_none() || work<0.) && strain<=0.05
                && contact::admit_staged_strands(&working,radius).is_ok()
                && mesh_motion::admit(&working,meshes,radius).is_ok()};
            if admitted {
                if working.iter().zip(&origin).any(|(rod,old)|rod.x[0]!=old.x[0]) {
                    return Err("contact restoration moved a prescribed root");
                }
                rods.clone_from_slice(&working);
                return Ok(true);
            }
            // A feasibility-only SOC cannot qualify an objective-increasing
            // feasible trial. Leave minimization to the original step owner.
            if energy_reference.is_some() && merit==0. && work>=0. {return Ok(false);}
            if iteration==limit {break;}
            // Recovery supplies feasibility, then the caller resumes physical
            // minimization. If elastic motion stops contracting violations,
            // hand off to the existing geometry-only phase instead of using
            // its remaining work budget on effectively unchanged contacts.
            // This is a phase transition, never a relaxed admission test.
            merits.push(merit);changes.push(None);
            if !zero_free && energy_reference.is_none()
                && !HairSystem::nonlinear_progress(&merits,&changes) {
                if trace {eprintln!("HAIR CONTACT RECOVERY phase_transition=geometry_only iteration={iteration} reason=feasibility_stagnation");}
                break;
            }
            let projection_timing=RecoveryTiming::new(timing,"direction_projection",Some(iteration),None);
            let free=if zero_free {PositionIncrement {
                linear:working.iter().map(|rod|vec![[0.;3];rod.x.len()]).collect(),
                angular:working.iter().map(|rod|vec![[0.;3];rod.q.len()]).collect(),
            }} else {NativeNewtonStep::new(&working,dt,1e-14)?.free};
            // Coordinate-capable backends consume the same canonical physical
            // system and retain the native global admission checks. A legacy
            // response backend must not replace the zero-free feasibility load.
            let guarded=if preserve_clear_strands {
                Some(match solver.as_mut() {
                    Some(backend)=>safe_motion::project_guarded_restoration_direction(&mut rows,&working,&start,&pairs,dt,radius,&free,Some(&mut **backend))?,
                    None=>safe_motion::project_guarded_restoration_direction(&mut rows,&working,&start,&pairs,dt,radius,&free,None)?,
                })
            } else {None};
            let (direction,groups)=if let Some((direction,groups))=guarded {(direction,Some(groups))} else {(match solver.as_mut().filter(|backend|backend.joint_contact_coordinates_enabled()) {
                Some(backend)=>square_root_projection::solve_accelerated(&mut rows,&working,dt,free,1e-14,&mut **backend)?,
                None=>square_root_projection::solve(&mut rows,&working,dt,free,1e-14)?,
            },None)};
            drop(projection_timing);
            let (_,trust)=safe_motion::trust_components(&working,&direction,groups.as_deref().unwrap_or(&pairs))?;
            let mut accepted=false;
            for backtrack in 0..32 {
                let _trial=RecoveryTiming::new(timing,"backtrack",Some(iteration),Some(backtrack));
                let scale=2f64.powi(-(backtrack as i32));
                let mut trial=working.clone();
                for (r,rod) in trial.iter_mut().enumerate() {
                    for i in 1..rod.x.len() {rod.x[i]=add(rod.x[i],mul(direction.linear[r][i],scale*trust[r]));}
                    for i in 1..rod.q.len() {apply(&mut rod.q[i],mul(direction.angular[r][i],scale*trust[r]));}
                }
                let fresh={let _stage=RecoveryTiming::new(timing,"trial_refresh",Some(iteration),Some(backtrack));
                    refresh(&mut trial,meshes,radius)?};
                let (_,path)={let _stage=RecoveryTiming::new(timing,"trial_model",Some(iteration),Some(backtrack));
                    safe_motion::restoration_model(&trial,&start,&fresh,radius)?};
                let next=HairSystem::contact_merit(&trial,&fresh,radius)?+path;
                let progress=if merit>0. {next<merit} else {
                    next==0. && HairSystem::elastic_energy_change(&trial,&before,dt)?<work
                };
                if progress && trial.iter().map(HairRod::max_relative_stretch).fold(0.,f64::max)<=0.05
                    && (!preserve_clear_strands || contact::admit_staged_strands(&trial,radius).is_ok()) {
                    // Trial contacts describe this exact accepted geometry.
                    refreshed=Some(fresh);working=trial;accepted=true;break;
                }
            }
            if !accepted {break;}
        }
    }
    if let Some(path)=std::env::var_os("VOXY_HAIR_CONTACT_REPLAY_EXPORT") {
        let path=std::path::Path::new(&path).with_extension("recovery-rejected.vhr");
        if let Err(error)=crate::hair::contact_replay::save_measured(&path,&working,meshes,dt,radius,true,true) {
            eprintln!("HAIR CONTACT RECOVERY EXPORT ERROR rejected: {error}");
        }
    }
    Ok(false)
}

#[cfg(test)]
mod refresh_reuse_tests {
    use super::*;
    #[test]
    fn accepted_geometry_refresh_reuse_matches_fresh_recovery() {
        let mesh=TriangleMesh::new(&[[-1.,-1.,0.],[1.,-1.,0.],[0.,1.,0.]],&[[0,1,2]]).unwrap();
        let mut saved_refreshes=0;
        for height in [1e-5,3e-5,6e-5] {for second_order in [false,true] {
            let mut rod=HairRod::new(vec![[0.,0.,0.2],[0.,0.,0.1],[0.,0.,0.001]],crate::hair::HairMaterial::default()).unwrap();
            rod.x[2][2]=height;
            let mut cached=vec![rod.clone()];let mut fresh=vec![rod];
            let before=fresh.iter().map(|rod|(rod.x.clone(),rod.q.clone())).collect::<Vec<_>>();
            let reference=second_order.then_some(before.as_slice());
            REFRESH_CALLS.with(|count|count.set(0));
            let a=restore_candidate_mode_impl::<true>(&mut cached,std::slice::from_ref(&mesh),1./240.,40e-6,None,true,reference);
            let cached_calls=REFRESH_CALLS.with(std::cell::Cell::get);
            REFRESH_CALLS.with(|count|count.set(0));
            let b=restore_candidate_mode_impl::<false>(&mut fresh,std::slice::from_ref(&mesh),1./240.,40e-6,None,true,reference);
            let fresh_calls=REFRESH_CALLS.with(std::cell::Cell::get);
            assert!(cached_calls<=fresh_calls);
            saved_refreshes+=fresh_calls-cached_calls;
            assert_eq!(a,b);
            for (a,b) in cached.iter().zip(&fresh) {
                assert_eq!(a.x,b.x);assert_eq!(a.q,b.q);
                assert_eq!(a.old_x,b.old_x);assert_eq!(a.old_q,b.old_q);
                assert_eq!(a.x[0],[0.,0.,0.2]);
                assert_eq!(a.q[0],before[0].1[0]);
            }
        }}
        assert!(saved_refreshes>0,"comparison must exercise skipped identical refreshes");
        eprintln!("RECOVERY REFRESH REUSE saved_refreshes={saved_refreshes} cases=6 pose_and_result_identical=true scope=small_fixture_not_full_groom");
    }
}
