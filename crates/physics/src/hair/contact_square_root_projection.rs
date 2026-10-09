//! Native Newton recovery from original constraints, never a GPU fallback.
use super::*;
#[path = "contact_newton_diagnostics.rs"]
mod newton_diagnostics;

pub(super) fn solve(
    constraints: &mut [Constraint],
    rods: &[HairRod],
    dt: f64,
    mut free: PositionIncrement,
    tolerance: f64,
) -> Result<PositionIncrement, &'static str> {
    let original=std::env::var_os("VOXY_HAIR_NEWTON_FAILURE_EXPORT").map(|_|free.clone());
    let groups = contact_islands(constraints, rods.len())?;
    let scale = constraints
        .iter()
        .map(|c| c.bound.abs().max(c.speed(&free).abs()))
        .fold(1., f64::max);
    let mut staged_reactions = vec![0.; constraints.len()];
    for (ids, rows) in groups {
        if rows.len() > 512 {
            return Err("native square-root contact island exceeds capacity");
        }
        let mut requests = Vec::with_capacity(ids.len());
        for &r in &ids {
            let mut request = response_batches::request_for_rod(&rods[r], dt, false)?;
            request.loads = vec![vec![0.; request.system.rhs.len()]; rows.len()];
            for (load, &index) in request.loads.iter_mut().zip(&rows) {
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
            .map(|&i| constraints[i].bound - constraints[i].speed(&free))
            .collect();
        let (responses, reactions) = solve_island_increment(
            &requests,&ids,&rows,constraints,&free,&bounds,tolerance*scale)?;
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

// Refine numerical response addition against original Newton rows. A local
// load inequality is insufficient when adding its response rounds again.
fn solve_island_increment(
    requests:&[crate::hair::HairResponseSystem],ids:&[usize],rows:&[usize],
    constraints:&[Constraint],free:&PositionIncrement,original_bounds:&[f64],tolerance:f64,
)->Result<(Vec<Vec<f64>>,Vec<f64>), &'static str> {
    let mut slots=vec![None;free.linear.len()];
    for (slot,&r) in ids.iter().enumerate() {slots[r]=Some(slot);}
    let mut bounds=original_bounds.to_vec();
    for refinement in 0..8 {
        let (responses,reactions)=crate::hair::HairResponseSystem::solve_joint_load_inequalities_native(
            requests,&bounds,tolerance)?;
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
            .filter(|e| dot(e.gradient, e.gradient) > 0.)
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
            std::slice::from_ref(&row),&free,&bounds,tolerance).unwrap();
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
        let result = solve(&mut constraints, &rods, 1. / 240., free, 1e-14).unwrap();
        for (r, constraint) in constraints.iter().enumerate() {
            assert!(constraint.multiplier > 0.);
            assert!(constraint.residual(&result) <= 1e-14);
            assert_eq!(result.linear[r][0], [0.; 3]);
        }
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
