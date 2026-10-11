//! Native Newton recovery from original constraints, never a GPU fallback.
use super::*;
#[path = "contact_newton_diagnostics.rs"]
mod newton_diagnostics;

// Scoped to an immutable pose and timestep. Lazily assemble each touched
// rod once; contact loads remain fresh for every cut refinement.
struct DualHintRow {entries:[Entry;4],reaction:f64}
impl DualHintRow {
    fn matches(&self,row:&Constraint)->bool {
        self.entries.iter().zip(&row.entries).all(|(a,b)|a.rod==b.rod && a.point==b.point
            && a.mobility.to_bits()==b.mobility.to_bits()
            && a.gradient.iter().zip(b.gradient).all(|(a,b)|a.to_bits()==b.to_bits()))
    }
}
pub(super) struct FrozenSystems<'a> {
    rods:&'a [HairRod],
    dt:f64,
    dual_hints:std::sync::Mutex<Vec<DualHintRow>>,
    loads:Vec<std::sync::Mutex<Option<crate::hair::contact_response_system::NativeLoadCache>>>,
    requests:Vec<std::sync::OnceLock<Result<crate::hair::HairResponseSystem,&'static str>>>,
}
impl<'a> FrozenSystems<'a> {
    pub(super) fn new(rods:&'a [HairRod],dt:f64)->Self {
        Self {rods,dt,dual_hints:std::sync::Mutex::new(Vec::new()),loads:(0..rods.len()).map(|_|std::sync::Mutex::new(None)).collect(),requests:(0..rods.len()).map(|_|std::sync::OnceLock::new()).collect()}
    }
    fn hints_for(&self,rows:&[usize],constraints:&[Constraint])->Vec<f64> {
        let Ok(hints)=self.dual_hints.lock() else {return Vec::new();};
        if hints.is_empty() {return Vec::new();}
        let result:Vec<_>=rows.iter().map(|&row|hints.get(row)
            .filter(|hint|hint.matches(&constraints[row])).map_or(0.,|hint|hint.reaction)).collect();
        if result.iter().any(|v|*v>0.) {result} else {Vec::new()}
    }
    fn publish_hints(&self,constraints:&[Constraint],reactions:&[f64]) {
        if let Ok(mut hints)=self.dual_hints.lock() {
            *hints=constraints.iter().zip(reactions).map(|(row,&reaction)|
                DualHintRow {entries:row.entries,reaction}).collect();
        }
    }
    #[cfg(test)]
    pub(super) fn assembled_count(&self)->usize {self.requests.iter().filter(|slot|slot.get().is_some()).count()}
    #[cfg(test)]
    pub(super) fn computed_load_columns(&self)->usize {
        self.loads.iter().map(|slot|slot.lock().unwrap().as_ref().map_or(0,|cache|cache.computed_columns())).sum()
    }
    fn prepare_loads(&self, index:usize, request:&crate::hair::HairResponseSystem)
        -> Result<(Vec<f64>,Vec<Vec<f64>>), &'static str> {
        let mut slot = self.loads[index].lock().map_err(|_|"hair load cache poisoned")?;
        if slot.is_none() { *slot = Some(crate::hair::contact_response_system::NativeLoadCache::new(request)?); }
        slot.as_mut().unwrap().prepare(request)
    }
    fn request(&self,index:usize)->Result<crate::hair::HairResponseSystem,&'static str> {
        self.requests[index].get_or_init(||response_batches::request_for_rod(&self.rods[index],self.dt,false)).clone()
    }
}
pub(super) fn solve_prepared(
    constraints:&mut [Constraint],systems:&FrozenSystems<'_>,free:PositionIncrement,tolerance:f64,
)->Result<PositionIncrement,&'static str> {
    let work=constraints.len().saturating_mul(constraints.len()).saturating_mul(systems.rods.len());
    let workers=if work>=100_000 {std::thread::available_parallelism().map_or(1,usize::from)} else {1};
    solve_with_workers_prepared(constraints,systems.rods,systems.dt,free,tolerance,workers,Some(systems),None)
}

pub(super) fn solve(
    constraints: &mut [Constraint],
    rods: &[HairRod],
    dt: f64,
    free: PositionIncrement,
    tolerance: f64,
) -> Result<PositionIncrement, &'static str> {
    let work=constraints.len().saturating_mul(constraints.len()).saturating_mul(rods.len());
    let workers=if work>=100_000 {std::thread::available_parallelism().map_or(1,usize::from)} else {1};
    solve_with_workers(constraints,rods,dt,free,tolerance,workers)
}
fn solve_with_workers(
    constraints:&mut [Constraint],rods:&[HairRod],dt:f64,free:PositionIncrement,tolerance:f64,workers:usize,
)->Result<PositionIncrement, &'static str> {
    solve_with_workers_prepared(constraints,rods,dt,free,tolerance,workers,None,None)
}
fn solve_with_workers_prepared(
    constraints:&mut [Constraint],rods:&[HairRod],dt:f64,mut free:PositionIncrement,tolerance:f64,workers:usize,
    prepared:Option<&FrozenSystems<'_>>,backend:Option<&mut dyn crate::hair::HairLinearSolver>,
)->Result<PositionIncrement, &'static str> {
    let original=std::env::var_os("VOXY_HAIR_NEWTON_FAILURE_EXPORT").map(|_|free.clone());
    let groups = contact_islands(constraints, rods.len())?;
    let scale = constraints.iter().map(|c|c.bound.abs().max(c.speed(&free).abs())).fold(1.,f64::max);
    let use_hints=backend.as_deref().is_some_and(|backend|backend.joint_contact_hints_enabled());
    let results=if let Some(backend)=backend {
        // Preflight the entire independent batch before any accelerator call.
        // A later invalid island must not consume device work for earlier ones.
        let inputs=groups.iter().map(|(ids,rows)|
            prepare_island(ids,rows,constraints,rods,dt,&free,prepared)
        ).collect::<Result<Vec<_>,_>>()?;
        if backend.joint_contact_coordinate_batches_enabled() && groups.len()>1 {
            solve_batched_islands(&groups,&inputs,constraints,&free,tolerance*scale,prepared,backend)?
        } else {
            groups.iter().zip(&inputs).map(|((ids,rows),(requests,bounds))| {
                solve_island_increment(requests,ids,rows,constraints,&free,bounds,tolerance*scale,prepared,Some(&mut *backend))
            }).collect::<Result<Vec<_>,_>>()?
        }
    } else {solve_independent_islands(&groups,constraints,rods,dt,&free,tolerance*scale,workers,prepared)?};
    let mut staged_reactions=vec![0.;constraints.len()];
    for ((ids,rows),(responses,reactions)) in groups.iter().zip(results) {
        for (&r, response) in ids.iter().zip(responses) {
            for p in 0..free.linear[r].len() {
                for axis in 0..3 {
                    free.linear[r][p][axis] += response[p * 6 + axis];
                }
            }
            for p in 0..free.angular[r].len() {
                for axis in 0..3 {
                    free.angular[r][p][axis] += response[p * 6 + 3 + axis];
                }
            }
        }
        for (&index, reaction) in rows.iter().zip(reactions) {
            staged_reactions[index] = reaction;
        }
    }
    if !free.finite()
        || constraints
            .iter()
            .zip(&staged_reactions)
            .any(|(c, reaction)| {
                let gap = c.speed(&free) - c.bound;
                !gap.is_finite()
                    || if *reaction > 0. {
                        gap.abs() > tolerance * scale
                    } else {
                        gap < -tolerance * scale
                    }
            })
    {
        if let Some(original)=&original {
            newton_diagnostics::export(constraints,original,&free,&staged_reactions,tolerance*scale);
        }
        return Err("native square-root Newton admission failed");
    }
    // Publish hints only after every original global physical check passed.
    if use_hints {if let Some(prepared)=prepared {prepared.publish_hints(constraints,&staged_reactions);}}
    for (constraint, reaction) in constraints.iter_mut().zip(staged_reactions) {
        constraint.multiplier = reaction;
    }
    Ok(free)
}

pub(super) fn solve_accelerated(constraints:&mut [Constraint],rods:&[HairRod],dt:f64,free:PositionIncrement,tolerance:f64,
    backend:&mut dyn crate::hair::HairLinearSolver)->Result<PositionIncrement,&'static str> {
    solve_with_workers_prepared(constraints,rods,dt,free,tolerance,1,None,Some(backend))
}
pub(super) fn solve_accelerated_prepared(constraints:&mut [Constraint],systems:&FrozenSystems<'_>,free:PositionIncrement,tolerance:f64,
    backend:&mut dyn crate::hair::HairLinearSolver)->Result<PositionIncrement,&'static str> {
    solve_with_workers_prepared(constraints,systems.rods,systems.dt,free,tolerance,1,Some(systems),Some(backend))
}
fn prepare_island(ids:&[usize],rows:&[usize],constraints:&[Constraint],rods:&[HairRod],dt:f64,
    free:&PositionIncrement,prepared:Option<&FrozenSystems<'_>>)->Result<(Vec<crate::hair::HairResponseSystem>,Vec<f64>),&'static str> {
        if rows.len() > 512 {
            return Err("native square-root contact island exceeds capacity");
        }
        let mut requests = Vec::with_capacity(ids.len());
        for &r in ids {
            let mut request = if let Some(systems)=prepared {systems.request(r)?}
                else {response_batches::request_for_rod(&rods[r], dt, false)?};
            request.loads = vec![vec![0.; request.system.rhs.len()]; rows.len()];
            for (load, &index) in request.loads.iter_mut().zip(rows) {
                for entry in constraints[index].entries {
                    if entry.rod == r && entry.point > 0 {
                        for axis in 0..3 {
                            load[entry.point * 6 + axis] += entry.gradient[axis];
                        }
                    }
                }
            }
            requests.push(request);
        }
        let bounds: Vec<_> = rows
            .iter()
            .map(|&i| constraints[i].bound - constraints[i].speed(free))
            .collect();
    Ok((requests,bounds))
}

type IslandSolution=(Vec<Vec<f64>>,Vec<f64>);
fn solve_independent_islands(
    groups:&[(Vec<usize>,Vec<usize>)],constraints:&[Constraint],rods:&[HairRod],dt:f64,
    free:&PositionIncrement,tolerance:f64,workers:usize,prepared:Option<&FrozenSystems<'_>>,
)->Result<Vec<IslandSolution>, &'static str> {
    let solve_one=|(ids,rows):&(Vec<usize>,Vec<usize>)|->Result<IslandSolution, &'static str> {
        let (requests,bounds)=prepare_island(ids,rows,constraints,rods,dt,free,prepared)?;
        let (responses, reactions) = solve_island_increment(
            &requests,ids,rows,constraints,free,&bounds,tolerance,prepared,None)?;
        Ok((responses,reactions))
    };
    let workers=workers.max(1).min(groups.len());
    if workers<=1 {return groups.iter().map(solve_one).collect();}
    use std::sync::atomic::{AtomicUsize,Ordering};
    let next=AtomicUsize::new(0);
    let observation_frame=crate::hair::phase_diagnostics::observation_frame();
    std::thread::scope(|scope| {
        let mut handles=Vec::new();
        for _ in 0..workers {
            let next=&next;let solve_one=&solve_one;
            handles.push(scope.spawn(move || crate::hair::phase_diagnostics::with_observation_frame(observation_frame,|| {
                let mut output=Vec::new();
                loop {
                    let index=next.fetch_add(1,Ordering::Relaxed);
                    if index>=groups.len() {break;}
                    output.push((index,solve_one(&groups[index])));
                }
                output
            })));
        }
        let mut ordered=Vec::with_capacity(groups.len());
        for handle in handles {
            ordered.extend(handle.join().map_err(|_|"native contact island worker panicked")?);
        }
        ordered.sort_unstable_by_key(|(index,_)|*index);
        ordered.into_iter().map(|(_,result)|result).collect()
    })
}

// Independent islands advance in ready rounds through original Newton-row
// admission. Both outer Newton and inner whitening/load corrections batch only
// unfinished owners while retaining the original per-owner physical admission.
fn solve_batched_islands(
    groups:&[(Vec<usize>,Vec<usize>)],inputs:&[(Vec<crate::hair::HairResponseSystem>,Vec<f64>)],
    constraints:&[Constraint],free:&PositionIncrement,tolerance:f64,
    frozen:Option<&FrozenSystems<'_>>,backend:&mut dyn crate::hair::HairLinearSolver,
)->Result<Vec<IslandSolution>,&'static str> {
    let joints=groups.iter().zip(inputs).map(|((ids,_),(requests,bounds))| {
        prepare_joint(requests,ids,bounds,tolerance,frozen)
    }).collect::<Result<Vec<_>,_>>()?;
    let use_hints=backend.joint_contact_hints_enabled();
    let mut hints:Vec<_>=groups.iter().map(|(_,rows)| {
        if use_hints {frozen.map_or_else(Vec::new,|f|f.hints_for(rows,constraints))} else {Vec::new()}
    }).collect();
    let mut bounds:Vec<_>=inputs.iter().map(|(_,bounds)|bounds.clone()).collect();
    let slots:Vec<_>=groups.iter().map(|(ids,_)| {
        let mut slots=vec![None;free.linear.len()];
        for (slot,&r) in ids.iter().enumerate() {slots[r]=Some(slot);}
        slots
    }).collect();
    for (i,(ids,_)) in groups.iter().enumerate() {observe_island_input(&joints[i],ids,&groups[i].1,constraints,&bounds[i],tolerance,true);}
    let mut finished:Vec<Option<IslandSolution>>=(0..groups.len()).map(|_|None).collect();
    for refinement in 0..8 {
        let ready:Vec<_>=(0..groups.len()).filter(|&i|finished[i].is_none()).collect();
        if ready.is_empty() {break;}
        let selected:Vec<_>=ready.iter().map(|&i|&joints[i]).collect();
        let selected_bounds:Vec<_>=ready.iter().map(|&i|bounds[i].as_slice()).collect();
        let mut selected_hints:Vec<_>=ready.iter().map(|&i|std::mem::take(&mut hints[i])).collect();
        let solutions=crate::hair::contact_response_system::PreparedNativeJoint::solve_accelerated_batch(
            &selected,&selected_bounds,tolerance,&mut selected_hints,backend)?;
        for (&i,hint) in ready.iter().zip(selected_hints) {hints[i]=hint;}
        for (i,(responses,reactions,accelerated)) in ready.into_iter().zip(solutions) {
            backend.joint_contact_result(accelerated);
            let admitted=admit_island_response(&inputs[i].0,&groups[i].1,constraints,free,&slots[i],
                &responses,&reactions,&mut bounds[i],tolerance);
            if admitted {finished[i]=Some((responses,reactions));}
            else if refinement==7 || bounds[i].iter().any(|v|!v.is_finite()) {
                return Err("native square-root island Newton refinement failed");
            }
        }
    }
    finished.into_iter().map(|result|result.ok_or("native square-root island Newton refinement failed")).collect()
}

fn prepare_joint<'a>(requests:&'a [crate::hair::HairResponseSystem],ids:&[usize],bounds:&[f64],tolerance:f64,
    frozen:Option<&FrozenSystems<'_>>)->Result<crate::hair::contact_response_system::PreparedNativeJoint<'a>,&'static str> {
    if let Some(frozen)=frozen {
        crate::hair::contact_response_system::PreparedNativeJoint::new_with_preparation(requests,bounds,tolerance,
            |index,request|frozen.prepare_loads(ids[index],request))
    } else {crate::hair::contact_response_system::PreparedNativeJoint::new(requests,bounds,tolerance)}
}

// Refine numerical response addition against original Newton rows. A local
// load inequality is insufficient when adding its response rounds again.
fn solve_island_increment(
    requests:&[crate::hair::HairResponseSystem],ids:&[usize],rows:&[usize],
    constraints:&[Constraint],free:&PositionIncrement,original_bounds:&[f64],tolerance:f64,
    frozen:Option<&FrozenSystems<'_>>,backend:Option<&mut dyn crate::hair::HairLinearSolver>,
)->Result<(Vec<Vec<f64>>,Vec<f64>), &'static str> {
    let joint=prepare_joint(requests,ids,original_bounds,tolerance,frozen)?;
    solve_prepared_island_increment(&joint,requests,ids,rows,constraints,free,original_bounds,tolerance,frozen,backend)
}
fn solve_prepared_island_increment(
    prepared:&crate::hair::contact_response_system::PreparedNativeJoint<'_>,
    requests:&[crate::hair::HairResponseSystem],ids:&[usize],rows:&[usize],
    constraints:&[Constraint],free:&PositionIncrement,original_bounds:&[f64],tolerance:f64,
    frozen:Option<&FrozenSystems<'_>>,mut backend:Option<&mut dyn crate::hair::HairLinearSolver>,
)->Result<IslandSolution,&'static str> {
    let mut slots=vec![None;free.linear.len()];
    for (slot,&r) in ids.iter().enumerate() {slots[r]=Some(slot);}
    let mut bounds=original_bounds.to_vec();
    observe_island_input(prepared,ids,rows,constraints,&bounds,tolerance,backend.is_some());
    // Immutable-step hints are remapped by exact contact row identity.
    // The coordinate owner reconstructs its primal from current columns.
    let use_hints=backend.as_deref().is_some_and(|backend|backend.joint_contact_hints_enabled());
    let mut coordinate_seeds=if use_hints {
        frozen.map_or_else(Vec::new,|frozen|frozen.hints_for(rows,constraints))
    } else {Vec::new()};
    for refinement in 0..8 {
        let (responses,reactions)=if let Some(backend)=backend.as_deref_mut() {
            let (responses,reactions,accelerated)=prepared.solve_accelerated(&bounds,tolerance,
                |columns,bounds,tolerance| {
                    let output=if !use_hints {backend.solve_joint_coordinates(columns,bounds,tolerance)}
                        else {backend.solve_joint_coordinates_seeded(columns,bounds,tolerance,&coordinate_seeds)};
                    if use_hints {if let Some((_,reactions))=&output {coordinate_seeds.clone_from(reactions);}}
                    output
                })?;
            backend.joint_contact_result(accelerated);(responses,reactions)
        } else {prepared.solve(&bounds,tolerance)?};
        let admitted=admit_island_response(requests,rows,constraints,free,&slots,&responses,&reactions,&mut bounds,tolerance);
        if admitted {return Ok((responses,reactions));}
        if refinement==7 || bounds.iter().any(|v| !v.is_finite()) {
            return Err("native square-root island Newton refinement failed");
        }
    }
    unreachable!("bounded Newton refinement returns on final trial")
}

fn observe_island_input(prepared:&crate::hair::contact_response_system::PreparedNativeJoint<'_>,
    ids:&[usize],rows:&[usize],constraints:&[Constraint],bounds:&[f64],tolerance:f64,external:bool) {
    // Opt-in diagnostic snapshots of complete original islands. Observation
    // never changes selection, bounds, precision or physical publication.
    if let Some(directory)=std::env::var_os("VOXY_HAIR_ISLAND_INPUT_EXPORT") {
        let frame=crate::hair::phase_diagnostics::observation_frame();
        if let Ok(selected)=std::env::var("VOXY_HAIR_ISLAND_INPUT_FRAME") {
            let selected=selected.parse::<usize>().expect("invalid observed island frame");
            assert!(selected>0,"observed island frame must be one-based");
            if frame!=Some(selected) {return;}
        }
        let frame_json=frame.map_or_else(||"null".to_owned(),|value|value.to_string());
        let target=std::env::var("VOXY_HAIR_ISLAND_INPUT_ROD").ok().and_then(|v|v.parse::<usize>().ok()).unwrap_or(53);
        let minimum=std::env::var("VOXY_HAIR_ISLAND_INPUT_MIN_TOLERANCE").ok().map(|v|v.parse::<f64>().expect("invalid observed island minimum tolerance")).unwrap_or(0.);
        assert!(minimum.is_finite() && minimum>=0.,"invalid observed island minimum tolerance");
        if tolerance>=minimum && ids.contains(&target) {
            use std::sync::atomic::{AtomicUsize,Ordering};
            static NATIVE:AtomicUsize=AtomicUsize::new(0);
            static EXTERNAL:AtomicUsize=AtomicUsize::new(0);
            let (label,counter)=if external {("external",&EXTERNAL)} else {("native",&NATIVE)};
            let index=counter.fetch_add(1,Ordering::Relaxed);
            let limit=std::env::var("VOXY_HAIR_ISLAND_INPUT_LIMIT").ok().map(|v|v.parse::<usize>().expect("invalid observed island input limit")).unwrap_or(16);
            assert!((1..=1024).contains(&limit),"observed island input limit outside 1..1024");
            // Skip earlier matching physical islands without overwriting the
            // late precursor we are diagnosing. Native and external counters
            // remain independent; filenames retain the original call index.
            let skip=std::env::var("VOXY_HAIR_ISLAND_INPUT_SKIP").ok()
                .map(|v|v.parse::<usize>().expect("invalid observed island input skip")).unwrap_or(0);
            if index>=skip && index-skip<limit {
                let directory=std::path::Path::new(&directory);
                match std::fs::create_dir_all(directory) {
                    Ok(())=> {
                        let path=directory.join(format!("{label}-{index:02}.vqc"));
                        if prepared.capture_observed_input(&path,&bounds,tolerance) {
                            // Record global rod ordering only after the immutable
                            // input was successfully written. Sidecar failure is
                            // diagnostic and cannot change physical admission.
                            use std::io::Write;
                            let metadata=std::fs::OpenOptions::new().write(true).create_new(true)
                                .open(path.with_extension("metadata.json"));
                            // Original row ownership makes near-parallel witnesses
                            // distinguishable without assuming observation ordinals
                            // or matching matrix dimensions imply row identity.
                            // Integer bit patterns preserve signed zero and every
                            // original f64 bit without JSON float conversion.
                            let original_rows=rows.iter().map(|&row| {
                                let constraint=&constraints[row];
                                let entries=constraint.entries.iter().map(|entry|format!(
                                    "{{\"rod\":{},\"point\":{},\"gradient_bits\":{:?},\"mobility_bits\":{}}}",
                                    entry.rod,entry.point,entry.gradient.map(f64::to_bits),entry.mobility.to_bits()
                                )).collect::<Vec<_>>().join(",");
                                format!("{{\"global_row_index\":{row},\"bound_bits\":{},\"entries\":[{entries}]}}",constraint.bound.to_bits())
                            }).collect::<Vec<_>>().join(",");
                            let result=metadata.and_then(|mut out|writeln!(out,
                                "{{\"schema\":\"voxy-observed-island-v2\",\"backend\":\"{label}\",\"observation_index\":{index},\"frame\":{frame_json},\"rod_ids\":{ids:?},\"tolerance\":{tolerance:e},\"original_rows\":[{original_rows}]}}"));
                            if let Err(error)=result {eprintln!("HAIR ISLAND METADATA EXPORT ERROR {error}");}
                        }
                    },
                    Err(error)=>eprintln!("HAIR ISLAND INPUT EXPORT ERROR {error}"),
                }
            }
        }
    }
 }

fn admit_island_response(
    requests:&[crate::hair::HairResponseSystem],rows:&[usize],constraints:&[Constraint],
    free:&PositionIncrement,slots:&[Option<usize>],responses:&[Vec<f64>],reactions:&[f64],bounds:&mut [f64],tolerance:f64,
)->bool {
        let mut admitted=true;
        for (i,&row) in rows.iter().enumerate() {
            let c=&constraints[row];
            let actual=c.entries.iter().map(|entry| {
                let old=free.linear[entry.rod][entry.point];
                let candidate=if let Some(slot)=slots[entry.rod] {
                    std::array::from_fn(|axis|old[axis]+responses[slot][entry.point*6+axis])
                } else {old};
                dot(entry.gradient,candidate)
            }).sum::<f64>();
            let gap=actual-c.bound;
            if !gap.is_finite() || if reactions[i]>0. {gap.abs()>tolerance} else {gap< -tolerance} {
                admitted=false;
            }
            let correction=requests.iter().zip(responses)
                .map(|(r,x)|r.loads[i].iter().zip(x).map(|(a,b)|a*b).sum::<f64>()).sum::<f64>();
            // Compensate the rounded original-free + response mapping only.
            // The final check above always uses the original immutable bound.
            bounds[i]=c.bound-(actual-correction);
        }
        admitted
}

// A rod's compliance couples its stations; partition by rods, never particles.
fn contact_islands(
    constraints: &[Constraint],
    count: usize,
) -> Result<Vec<(Vec<usize>, Vec<usize>)>, &'static str> {
    fn root(parent: &[usize], mut i: usize) -> usize {
        while parent[i] != i {
            i = parent[i];
        }
        i
    }
    let mut parent: Vec<_> = (0..count).collect();
    let mut owners = Vec::with_capacity(constraints.len());
    let mut involved = vec![false; count];
    for constraint in constraints {
        let ids: Vec<_> = constraint
            .entries
            .iter()
            .filter(|e| e.gradient.iter().any(|&value| value != 0.))
            .map(|e| e.rod)
            .collect();
        let Some(&first) = ids.first() else {
            return Err("square-root contact has no movable owner");
        };
        for &id in &ids {
            involved[id] = true;
            let a = root(&parent, first);
            let b = root(&parent, id);
            parent[a.max(b)] = a.min(b);
        }
        owners.push(first);
    }
    let mut groups: std::collections::BTreeMap<usize, (Vec<usize>, Vec<usize>)> =
        Default::default();
    for (r, used) in involved.iter().enumerate() {
        if *used {
            groups.entry(root(&parent, r)).or_default().0.push(r);
        }
    }
    for (i, owner) in owners.into_iter().enumerate() {
        groups.entry(root(&parent, owner)).or_default().1.push(i);
    }
    Ok(groups.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_physical_islands_batch_initial_trials_and_preserve_admission() {
        struct Backend {mode:usize,batches:usize,admitted:Vec<bool>}
        impl crate::hair::HairLinearSolver for Backend {
            fn solve(&mut self,_:&[crate::hair::HairLinearSystem])->Result<Vec<Vec<f64>>,&'static str> {panic!("unexpected structural solve")}
            fn joint_contact_coordinates_enabled(&self)->bool {true}
            fn joint_contact_coordinate_batches_enabled(&self)->bool {true}
            fn solve_joint_coordinates(&mut self,_:&[Vec<f64>],_:&[f64],_:f64)->Option<(Vec<f64>,Vec<f64>)> {
                panic!("initial physical trials were submitted serially")
            }
            fn solve_joint_coordinates_batch(&mut self,requests:&[crate::hair::HairContactCoordinateRequest<'_>])
                ->Option<Vec<(Vec<f64>,Vec<f64>)>> {
                self.batches+=1;assert_eq!(requests.len(),2);
                if self.mode==1 {return None;}
                if self.mode==2 {return Some(vec![]);}
                Some(requests.iter().enumerate().map(|(i,r)| {
                    assert_eq!(r.columns.len(),1);
                    let norm=r.columns[0].iter().map(|v|v*v).sum::<f64>();
                    let reaction=r.bounds[0]/norm;
                    let mut x:Vec<_>=r.columns[0].iter().map(|v|v*reaction).collect();
                    if self.mode==3 && i==0 {x.pop();}
                    (x,vec![reaction])
                }).collect())
            }
            fn joint_contact_result(&mut self,accelerated:bool) {self.admitted.push(accelerated);}
        }
        let rods:Vec<_>=[0.,0.1].into_iter().map(|x|HairRod::new(
            vec![[x,0.,0.],[x,0.01,0.],[x,0.02,0.]],Default::default()).unwrap()).collect();
        let frozen=FrozenSystems::new(&rods,1./240.);
        let zero=Entry {rod:0,point:0,gradient:[0.;3],mobility:0.};
        let make_rows=|| {
            let mut rows=Vec::new();
            for r in 0..2 {
                let a=entries(r,1,0.,[1.,0.,0.],&rods[r]);
                add_constraint(&mut rows,[a[0],a[1],zero,zero],(r+1) as f64*1e-6).unwrap();
                rows[r].multiplier=17.+r as f64;
            }
            rows
        };
        let free=PositionIncrement {linear:vec![vec![[0.;3];3];2],angular:vec![vec![[0.;3];2];2]};
        let mut native_rows=make_rows();
        let native=solve_with_workers_prepared(&mut native_rows,&rods,1./240.,free.clone(),1e-14,1,Some(&frozen),None).unwrap();
        for mode in 0..4 {
            let mut rows=make_rows();let mut backend=Backend {mode,batches:0,admitted:Vec::new()};
            let result=solve_accelerated_prepared(&mut rows,&frozen,free.clone(),1e-14,&mut backend);
            assert_eq!(backend.batches,1);
            if mode==2 {
                assert_eq!(result.err(),Some("joint coordinate batch result count mismatch"));
                assert_eq!(rows.iter().map(|r|r.multiplier).collect::<Vec<_>>(),vec![17.,18.]);
                assert!(backend.admitted.is_empty());continue;
            }
            let result=result.unwrap();
            assert_eq!(backend.admitted,match mode {0=>vec![true,true],1=>vec![false,false],3=>vec![false,true],_=>unreachable!()});
            for row in &rows {assert!((row.speed(&result)-row.bound).abs()<=1e-14);assert!(row.multiplier>=0.);}
            let difference=result.linear.iter().flatten().flatten().zip(native.linear.iter().flatten().flatten())
                .map(|(a,b)|(a-b).abs()).fold(0f64,f64::max);
            assert!(difference<1e-18,"batched physical response drift: {difference}");
        }
    }

    #[test]
    fn frozen_dual_hints_follow_exact_rows_and_only_opt_in_backends() {
        struct Backend {seen:Vec<Vec<f64>>}
        impl crate::hair::HairLinearSolver for Backend {
            fn joint_contact_coordinates_enabled(&self)->bool {true}
            fn joint_contact_hints_enabled(&self)->bool {true}
            fn solve(&mut self,_:&[crate::hair::HairLinearSystem])->Result<Vec<Vec<f64>>,&'static str> {panic!("native free owner")}
            fn solve_joint_coordinates_seeded(&mut self,columns:&[Vec<f64>],bounds:&[f64],tolerance:f64,seeds:&[f64])->Option<(Vec<f64>,Vec<f64>)> {
                self.seen.push(seeds.to_vec());
                crate::hair::HairResponseSystem::solve_contact_coordinates_seeded_with_equality_accelerator(columns,bounds,tolerance,seeds,
                    |columns,bounds,_| {
                        assert_eq!(columns.len(),1);
                        let norm=columns[0].iter().map(|v|v*v).sum::<f64>();
                        let reaction=bounds[0]/norm;
                        Some((columns[0].iter().map(|v|v*reaction).collect(),vec![reaction]))
                    })
            }
            fn joint_contact_result(&mut self,accelerated:bool) {assert!(accelerated);}
        }
        let rods=vec![HairRod::new(vec![[0.,0.,0.],[0.,0.01,0.],[0.,0.02,0.]],Default::default()).unwrap()];
        let frozen=FrozenSystems::new(&rods,1./240.);
        let zero=Entry {rod:0,point:0,gradient:[0.;3],mobility:0.};
        let a=entries(0,1,0.,[1.,0.,0.],&rods[0]);
        let mut constraints=Vec::new();
        add_constraint(&mut constraints,[a[0],a[1],zero,zero],1e-6).unwrap();
        let free=PositionIncrement {linear:vec![vec![[0.;3];3]],angular:vec![vec![[0.;3];2]]};
        let mut backend=Backend {seen:Vec::new()};
        for (iteration,bound) in [1e-6,1.1e-6].into_iter().enumerate() {
            let first_callback=backend.seen.len();
            constraints[0].bound=bound;
            constraints[0].multiplier=0.; // hint storage must be independent
            let result=solve_accelerated_prepared(&mut constraints,&frozen,free.clone(),1e-14,&mut backend).unwrap();
            assert!((constraints[0].speed(&result)-bound).abs()<1e-14);
            if iteration==0 {assert!(backend.seen[first_callback].is_empty());}
            else {assert!(backend.seen[first_callback].iter().any(|v|*v>0.));}
        }
        let accepted=frozen.hints_for(&[0],&constraints)[0];
        assert!(accepted>0.);
        let mut invalid_free=free.clone();invalid_free.linear[0][1][0]=f64::NAN;
        assert!(solve_accelerated_prepared(&mut constraints,&frozen,invalid_free,1e-14,&mut backend).is_err());
        assert_eq!(frozen.hints_for(&[0],&constraints),vec![accepted],"failed solve replaced admitted hints");
        constraints.push(Constraint {entries:constraints[0].entries,bound:0.,diagonal:constraints[0].diagonal,
            multiplier:0.,response:Vec::new()});
        assert_eq!(frozen.hints_for(&[1,0],&constraints),vec![0.,accepted],"new row or island permutation misassigned hints");
        constraints[0].entries[0].gradient[0]=constraints[0].entries[0].gradient[0].next_up();
        assert!(frozen.hints_for(&[0],&constraints).is_empty(),"ULP-changed row reused an identity hint");
        let another=FrozenSystems::new(&rods,1./240.);
        assert!(another.hints_for(&[0],&constraints).is_empty(),"hint escaped immutable Newton step");
    }

    #[test]
    fn cooperative_newton_rounds_retry_only_unfinished_original_rows() {
        struct Backend {rounds:Vec<usize>}
        impl crate::hair::HairLinearSolver for Backend {
            fn solve(&mut self,_:&[crate::hair::HairLinearSystem])->Result<Vec<Vec<f64>>,&'static str> {panic!("unexpected structural solve")}
            fn solve_joint_coordinates(&mut self,_:&[Vec<f64>],_:&[f64],_:f64)->Option<(Vec<f64>,Vec<f64>)> {
                panic!("outer Newton correction escaped cooperative rounds")
            }
            fn solve_joint_coordinates_batch(&mut self,requests:&[crate::hair::HairContactCoordinateRequest<'_>])
                ->Option<Vec<(Vec<f64>,Vec<f64>)>> {
                self.rounds.push(requests.len());
                if self.rounds.len()>1 {assert!(requests.iter().all(|r|r.columns[0][7]==1.));}
                Some(requests.iter().map(|r| {
                    let norm=r.columns[0].iter().map(|v|v*v).sum::<f64>();let reaction=r.bounds[0]/norm;
                    (r.columns[0].iter().map(|v|v*reaction).collect(),vec![reaction])
                }).collect())
            }
            fn joint_contact_result(&mut self,accelerated:bool) {assert!(accelerated);}
        }
        let zero=Entry {rod:0,point:0,gradient:[0.;3],mobility:0.};
        let constraints:Vec<_>=[[1.,1.,0.],[1.,0.,0.]].into_iter().enumerate().map(|(r,gradient)|Constraint {
            entries:[Entry {rod:r,point:1,gradient,mobility:1.},zero,zero,zero],
            bound:if r==0 {12e-15} else {1e-6},diagonal:if r==0 {2.} else {1.},multiplier:0.,response:Vec::new()
        }).collect();
        let free=PositionIncrement {linear:vec![vec![[0.;3],[16f64.next_up(),-16.,0.],[0.;3]],vec![[0.;3];3]],
            angular:vec![vec![[0.;3];2];2]};
        let inputs:Vec<_>=(0..2).map(|r| {
            let mut matrix=vec![0.;18*crate::hair::direct::BAND];for i in 0..18 {matrix[i*crate::hair::direct::BAND]=1.;}
            let mut load=vec![0.;18];load[6]=1.;if r==0 {load[7]=1.;}
            (vec![crate::hair::HairResponseSystem {system:crate::hair::HairLinearSystem {
                band_width:crate::hair::direct::BAND,matrix,rhs:vec![0.;18],active:6..15},loads:vec![load]}],
                vec![constraints[r].bound-constraints[r].speed(&free)])
        }).collect();
        let groups=vec![(vec![0],vec![0]),(vec![1],vec![1])];
        let mut backend=Backend {rounds:Vec::new()};
        let solutions=solve_batched_islands(&groups,&inputs,&constraints,&free,1e-15,None,&mut backend).unwrap();
        assert_eq!(backend.rounds[0],2);assert!(backend.rounds.len()>1,"fixture missed rounded-free correction");
        assert!(backend.rounds[1..].iter().all(|&count|count==1));
        for (r,(responses,reactions)) in solutions.into_iter().enumerate() {
            let mut candidate=free.clone();
            for p in 0..3 {for axis in 0..3 {candidate.linear[r][p][axis]+=responses[0][p*6+axis];}}
            assert!((constraints[r].speed(&candidate)-constraints[r].bound).abs()<=1e-15);
            assert!(reactions[0]>0.);
        }
    }
    #[test]
    fn rounded_free_plus_response_is_checked_in_original_newton_coordinates() {
        let zero=Entry {rod:0,point:0,gradient:[0.;3],mobility:0.};
        let row=Constraint {entries:[Entry {rod:0,point:1,gradient:[1.,1.,0.],mobility:1.},zero,zero,zero],
            bound:12e-15,diagonal:2.,multiplier:0.,response:Vec::new()};
        let free=PositionIncrement {linear:vec![vec![[0.;3],[16f64.next_up(),-16.,0.],[0.;3]]],
            angular:vec![vec![[0.;3];2]]};
        let mut matrix=vec![0.;18*crate::hair::direct::BAND];
        for i in 0..18 {matrix[i*crate::hair::direct::BAND]=1.;}
        let mut load=vec![0.;18];load[6]=1.;load[7]=1.;
        let requests=vec![crate::hair::HairResponseSystem {
            system:crate::hair::HairLinearSystem {band_width:crate::hair::direct::BAND,
                matrix,rhs:vec![0.;18],active:6..15},loads:vec![load],
        }];
        let bounds=vec![row.bound-row.speed(&free)];let tolerance=1e-15;
        let add_response=|response:&[f64]| {
            let mut candidate=free.clone();
            for p in 0..3 {for axis in 0..3 {candidate.linear[0][p][axis]+=response[p*6+axis];}}
            candidate
        };
        let (local,_) = crate::hair::HairResponseSystem::solve_joint_load_inequalities_native(
            &requests,&bounds,tolerance).unwrap();
        assert!((row.speed(&add_response(&local[0]))-row.bound).abs()>tolerance,
            "admitted local load response can fail after rounded free addition");
        let (responses,reactions)=solve_island_increment(&requests,&[0],&[0],
            std::slice::from_ref(&row),&free,&bounds,tolerance,None,None).unwrap();
        let candidate=add_response(&responses[0]);
        assert!((row.speed(&candidate)-row.bound).abs()<=tolerance);
        assert!(reactions[0]>0.);
        assert_eq!(candidate.linear[0][0],[0.;3]);
    }
    #[test]
    fn independent_islands_exceed_old_global_capacity_without_dropping_rows() {
        let rods: Vec<_> = (0..300)
            .map(|i| {
                let x = i as f64 * 0.01;
                HairRod::new(
                    vec![[x, 0., 0.], [x, 0.01, 0.], [x, 0.02, 0.]],
                    Default::default(),
                )
                .unwrap()
            })
            .collect();
        let mut constraints = Vec::new();
        let zero = Entry {
            rod: 0,
            point: 0,
            gradient: [0.; 3],
            mobility: 0.,
        };
        for (r, rod) in rods.iter().enumerate() {
            let a = entries(r, 1, 0., [1., 0., 0.], rod);
            add_constraint(&mut constraints, [a[0], a[1], zero, zero], 1e-6).unwrap();
        }
        let groups = contact_islands(&constraints, rods.len()).unwrap();
        assert_eq!(groups.len(), 300);
        assert!(
            groups
                .iter()
                .all(|(ids, rows)| ids.len() == 1 && rows.len() == 1)
        );
        let free = PositionIncrement {
            linear: vec![vec![[0.; 3]; 3]; 300],
            angular: vec![vec![[0.; 3]; 2]; 300],
        };
        let copy_rows=||constraints.iter().map(|row| {
            assert!(row.response.is_empty());
            Constraint {entries:row.entries,bound:row.bound,diagonal:row.diagonal,multiplier:row.multiplier,response:Vec::new()}
        }).collect::<Vec<_>>();
        let signature=|values:&[Vec<V>]|values.iter().flatten().flatten().map(|v|v.to_bits()).collect::<Vec<_>>();
        let mut serial_rows=copy_rows();
        let serial=crate::hair::observe_contact_frame(4,||solve_with_workers(&mut serial_rows,&rods,1./240.,free.clone(),1e-14,1)).unwrap();
        for workers in [2,4] {
            let mut parallel_rows=copy_rows();
            let parallel=crate::hair::observe_contact_frame(4,||solve_with_workers(&mut parallel_rows,&rods,1./240.,free.clone(),1e-14,workers)).unwrap();
            assert_eq!(signature(&serial.linear),signature(&parallel.linear));
            assert_eq!(signature(&serial.angular),signature(&parallel.angular));
            assert_eq!(serial_rows.iter().map(|c|c.multiplier.to_bits()).collect::<Vec<_>>(),
                parallel_rows.iter().map(|c|c.multiplier.to_bits()).collect::<Vec<_>>());
        }
        let result = solve(&mut constraints, &rods, 1. / 240., free, 1e-14).unwrap();
        for (r, constraint) in constraints.iter().enumerate() {
            assert!(constraint.multiplier > 0.);
            assert!(constraint.residual(&result) <= 1e-14);
            assert_eq!(result.linear[r][0], [0.; 3]);
        }
    }
    #[test]
    fn tiny_nonzero_jacobians_cannot_split_coupled_rod_owners() {
        let zero=Entry {rod:0,point:0,gradient:[0.;3],mobility:0.};
        let a=Entry {rod:0,point:1,gradient:[1e-200,0.,0.],mobility:1.};
        let b=Entry {rod:1,point:1,gradient:[-1e-200,0.,0.],mobility:1.};
        assert_eq!(dot(a.gradient,a.gradient),0.,"squaring a nonzero row can underflow");
        let row=Constraint {entries:[a,b,zero,zero],bound:0.,diagonal:0.,multiplier:0.,response:Vec::new()};
        assert_eq!(contact_islands(&[row],2).unwrap(),vec![(vec![0,1],vec![0])]);
    }
    #[test]
    fn rejected_parallel_island_does_not_publish_other_reactions() {
        let rods:Vec<_>=(0..2).map(|r|HairRod::new(vec![[r as f64,0.,0.],[r as f64,0.01,0.],[r as f64,0.02,0.]],Default::default()).unwrap()).collect();
        let zero=Entry {rod:0,point:0,gradient:[0.;3],mobility:0.};
        let row=|r|Constraint {entries:[Entry {rod:r,point:1,gradient:[1.,0.,0.],mobility:1.},zero,zero,zero],bound:1e-6,diagonal:1.,multiplier:0.25,response:Vec::new()};
        let mut rows=vec![row(0)];rows.extend((0..513).map(|_|row(1)));
        let free=PositionIncrement {linear:vec![vec![[0.;3];3];2],angular:vec![vec![[0.;3];2];2]};
        assert_eq!(solve_with_workers(&mut rows,&rods,1./240.,free.clone(),1e-14,2).err(),Some("native square-root contact island exceeds capacity"));
        struct Count {calls:usize}
        impl crate::hair::HairLinearSolver for Count {
            fn solve(&mut self,_:&[crate::hair::HairLinearSystem])->Result<Vec<Vec<f64>>, &'static str> {panic!("unexpected structural solve")}
            fn solve_joint_coordinates(&mut self,_:&[Vec<f64>],_:&[f64],_:f64)->Option<(Vec<f64>,Vec<f64>)> {self.calls+=1;None}
        }
        let mut backend=Count {calls:0};
        assert_eq!(solve_accelerated(&mut rows,&rods,1./240.,free,1e-14,&mut backend).err(),Some("native square-root contact island exceeds capacity"));
        assert_eq!(backend.calls,0,"invalid later island launched earlier device work");
        assert!(rows.iter().all(|row|row.multiplier==0.25));
    }
    #[test]
    fn rod_compliance_keeps_different_stations_in_the_same_island() {
        let rods: Vec<_> = (0..3)
            .map(|i| {
                HairRod::new(
                    vec![
                        [i as f64, 0., 0.],
                        [i as f64, 0.01, 0.],
                        [i as f64, 0.02, 0.],
                    ],
                    Default::default(),
                )
                .unwrap()
            })
            .collect();
        let mut constraints = Vec::new();
        for (a, b, point) in [(0, 1, 1), (1, 2, 2)] {
            let aa = entries(
                a,
                1,
                if point == 1 { 0. } else { 1. },
                [1., 0., 0.],
                &rods[a],
            );
            let bb = entries(b, 1, 0., [-1., 0., 0.], &rods[b]);
            add_constraint(&mut constraints, [aa[0], aa[1], bb[0], bb[1]], 1e-6).unwrap();
        }
        assert_eq!(
            contact_islands(&constraints, 3).unwrap(),
            vec![(vec![0, 1, 2], vec![0, 1])]
        );
    }
    #[test]
    fn joint_newton_recovery_preserves_free_guides_and_releases_opening_motion() {
        let rods: Vec<_> = [0., 80e-6, 0.02]
            .iter()
            .map(|&x| {
                HairRod::new(
                    vec![[x, 0., 0.], [x, 0.01, 0.], [x, 0.02, 0.]],
                    Default::default(),
                )
                .unwrap()
            })
            .collect();
        for movement in [-1e-5, 1e-5] {
            let mut free = PositionIncrement {
                linear: vec![vec![[0.; 3]; 3]; 3],
                angular: vec![vec![[0.; 3]; 2]; 3],
            };
            free.linear[0][1][0] = movement;
            free.linear[0][2][0] = movement;
            free.linear[2][1] = [2e-5, 3e-5, -1e-5];
            let untouched = free.linear[2].clone();
            let a = entries(0, 1, 0.5, [-1., 0., 0.], &rods[0]);
            let b = entries(1, 1, 0.5, [1., 0., 0.], &rods[1]);
            let mut constraints = Vec::new();
            add_constraint(&mut constraints, [a[0], a[1], b[0], b[1]], 0.).unwrap();
            let result = solve(&mut constraints, &rods, 1. / 240., free, 1e-14).unwrap();
            assert_eq!(result.linear[2], untouched);
            for rod in &result.linear {
                assert_eq!(rod[0], [0.; 3]);
            }
            assert!(constraints[0].residual(&result) <= 1e-14);
            if movement < 0. {
                assert_eq!(constraints[0].multiplier, 0.);
                assert_eq!(result.linear[0][1][0], movement);
                assert_eq!(result.linear[1], vec![[0.; 3]; 3]);
            } else {
                assert!(constraints[0].multiplier > 0.);
                assert!((result.linear[0][1][0] + result.linear[1][1][0] - movement).abs() < 1e-14);
            }
        }
    }
}
