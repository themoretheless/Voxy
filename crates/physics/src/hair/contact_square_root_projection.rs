//! Native Newton recovery from original constraints, never a GPU fallback.
use super::*;
#[path = "contact_newton_diagnostics.rs"]
mod newton_diagnostics;

// Scoped to an immutable pose and timestep. Lazily assemble each touched
// rod once; contact loads remain fresh for every cut refinement.
pub(super) struct FrozenSystems<'a> {
    rods:&'a [HairRod],
    dt:f64,
    loads:Vec<std::sync::Mutex<Option<crate::hair::contact_response_system::NativeLoadCache>>>,
    requests:Vec<std::sync::OnceLock<Result<crate::hair::HairResponseSystem,&'static str>>>,
}
impl<'a> FrozenSystems<'a> {
    pub(super) fn new(rods:&'a [HairRod],dt:f64)->Self {
        Self {rods,dt,loads:(0..rods.len()).map(|_|std::sync::Mutex::new(None)).collect(),requests:(0..rods.len()).map(|_|std::sync::OnceLock::new()).collect()}
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
    let results=if let Some(backend)=backend {
        groups.iter().map(|(ids,rows)| {
            let (requests,bounds)=prepare_island(ids,rows,constraints,rods,dt,&free,prepared)?;
            solve_island_increment(&requests,ids,rows,constraints,&free,&bounds,tolerance*scale,prepared,Some(&mut *backend))
        }).collect::<Result<Vec<_>,_>>()?
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
    std::thread::scope(|scope| {
        let mut handles=Vec::new();
        for _ in 0..workers {
            let next=&next;let solve_one=&solve_one;
            handles.push(scope.spawn(move || {
                let mut output=Vec::new();
                loop {
                    let index=next.fetch_add(1,Ordering::Relaxed);
                    if index>=groups.len() {break;}
                    output.push((index,solve_one(&groups[index])));
                }
                output
            }));
        }
        let mut ordered=Vec::with_capacity(groups.len());
        for handle in handles {
            ordered.extend(handle.join().map_err(|_|"native contact island worker panicked")?);
        }
        ordered.sort_unstable_by_key(|(index,_)|*index);
        ordered.into_iter().map(|(_,result)|result).collect()
    })
}

// Refine numerical response addition against original Newton rows. A local
// load inequality is insufficient when adding its response rounds again.
fn solve_island_increment(
    requests:&[crate::hair::HairResponseSystem],ids:&[usize],rows:&[usize],
    constraints:&[Constraint],free:&PositionIncrement,original_bounds:&[f64],tolerance:f64,
    frozen:Option<&FrozenSystems<'_>>,mut backend:Option<&mut dyn crate::hair::HairLinearSolver>,
)->Result<(Vec<Vec<f64>>,Vec<f64>), &'static str> {
    let mut slots=vec![None;free.linear.len()];
    for (slot,&r) in ids.iter().enumerate() {slots[r]=Some(slot);}
    let mut bounds=original_bounds.to_vec();
    let prepared = if let Some(frozen) = frozen {
        crate::hair::contact_response_system::PreparedNativeJoint::new_with_preparation(requests, &bounds, tolerance,
            |index, request| frozen.prepare_loads(ids[index], request))?
    } else { crate::hair::contact_response_system::PreparedNativeJoint::new(requests, &bounds, tolerance)? };
    for refinement in 0..8 {
        let (responses,reactions)=if let Some(backend)=backend.as_deref_mut() {
            let (responses,reactions,accelerated)=prepared.solve_accelerated(&bounds,tolerance,
                |columns,bounds,tolerance|backend.solve_joint_coordinates(columns,bounds,tolerance))?;
            backend.joint_contact_result(accelerated);(responses,reactions)
        } else {prepared.solve(&bounds,tolerance)?};
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
            let correction=requests.iter().zip(&responses)
                .map(|(r,x)|r.loads[i].iter().zip(x).map(|(a,b)|a*b).sum::<f64>()).sum::<f64>();
            // Compensate the rounded original-free + response mapping only.
            // The final check above always uses the original immutable bound.
            bounds[i]=c.bound-(actual-correction);
        }
        if admitted {return Ok((responses,reactions));}
        if refinement==7 || bounds.iter().any(|v| !v.is_finite()) {
            return Err("native square-root island Newton refinement failed");
        }
    }
    unreachable!("bounded Newton refinement returns on final trial")
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
        let serial=solve_with_workers(&mut serial_rows,&rods,1./240.,free.clone(),1e-14,1).unwrap();
        for workers in [2,4] {
            let mut parallel_rows=copy_rows();
            let parallel=solve_with_workers(&mut parallel_rows,&rods,1./240.,free.clone(),1e-14,workers).unwrap();
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
        assert_eq!(solve_with_workers(&mut rows,&rods,1./240.,free,1e-14,2).err(),Some("native square-root contact island exceeds capacity"));
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
