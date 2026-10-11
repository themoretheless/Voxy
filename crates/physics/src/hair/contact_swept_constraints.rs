//! Affine cuts from witnessed intersections of a proposed joint root/free path.
//! Samples find cuts only; the complete continuous guard still admits motion.
use super::*;

fn point(points: &[V], segment: usize, fraction: f64) -> V {
    add(
        mul(points[segment], 1. - fraction),
        mul(points[segment + 1], fraction),
    )
}
fn add_cut(
    constraints: &mut Vec<Constraint>,
    rods: &[HairRod],
    start: &[HairRod],
    pair: &StrandResponse,
    time: f64,
    radius: f64,
    scales: [f64;2],
) -> Result<(), &'static str> {
    if !time.is_finite() || time <= 0. || time > 1. {
        return Err("invalid swept contact time");
    }
    if scales.iter().any(|s| !s.is_finite() || *s<=0. || *s>1.) {
        return Err("invalid swept witness scale");
    }
    let (a, i, s) = pair.a;
    let (b, j, t) = pair.b;
    let old = dot(
        sub(point(&start[a].x, i, s), point(&start[b].x, j, t)),
        pair.normal,
    );
    let staged = dot(
        sub(point(&rods[a].x, i, s), point(&rods[b].x, j, t)),
        pair.normal,
    );
    // At tau: (1-tau)*old + tau*(staged + J*scaled_free) >= diameter.
    // Staged includes the exact prescribed-root endpoint contribution;
    // pinned DOFs are removed only from J, never from this affine bound.
    // Keep the physical space-time row, including its time Jacobian.
    // Dividing by a very early witness time changes the force multiplier and
    // turns a physical residual into an endpoint residual with another unit
    // scale. FMA avoids cancellation in the affine prescribed-root term.
    let bound = time.mul_add(old-staged,2.*radius-old);
    let ea = entries(a, i, s, mul(pair.normal,time*scales[0]), &rods[a]);
    let eb = entries(b, j, t, mul(pair.normal, -time*scales[1]), &rods[b]);
    add_constraint(constraints, [ea[0], ea[1], eb[0], eb[1]], bound)?
        .ok_or("swept contact has no movable endpoint")?;
    Ok(())
}

pub(super) struct Cut {
    pair: StrandResponse,
    time: f64,
    scales: [f64;2],
}

// Different barycentric witnesses of one segment pair constrain different
// affine points. Replacing by segment identity alone can reopen an earlier
// collision. Retain witnesses for this Newton admission transaction only.
fn retain_cut(cuts: &mut Vec<Cut>, pair: StrandResponse, time: f64, scales: [f64;2]) -> bool {
    if let Some(old)=cuts.iter_mut().find(|old| old.time == time && old.pair.a == pair.a
        && old.pair.b == pair.b && old.pair.normal == pair.normal) {
        let changed=old.scales!=scales;
        old.scales=scales;
        return changed;
    }
    cuts.push(Cut { pair, time, scales });
    true
}

pub(super) fn activate(
    constraints: &mut Vec<Constraint>,
    rods: &[HairRod],
    start: &[HairRod],
    increment: &PositionIncrement,
    groups: &mut Vec<StrandResponse>,
    base: &[StrandResponse],
    cuts: &mut Vec<Cut>,
    radius: f64,
) -> Result<bool, &'static str> {
    activate_with_workers(constraints,rods,start,increment,groups,base,cuts,radius,None)
}

pub(super) fn activate_with_workers(
    constraints: &mut Vec<Constraint>,
    rods: &[HairRod],
    start: &[HairRod],
    increment: &PositionIncrement,
    groups: &mut Vec<StrandResponse>,
    base: &[StrandResponse],
    cuts: &mut Vec<Cut>,
    radius: f64,
    workers:Option<usize>,
) -> Result<bool, &'static str> {
    let mut motions = Vec::new();
    let mut ids = Vec::new();
    let (_,scales)=trust_components(rods,increment,groups)?;
    for (r, rod) in rods.iter().enumerate() {
        for i in 0..rod.x.len() - 1 {
            ids.push((r, i));
            motions.push(CapsuleMotion {
                start: [start[r].x[i], start[r].x[i + 1]],
                end: [
                    add(rod.x[i], mul(increment.linear[r][i],scales[r])),
                    add(rod.x[i + 1], mul(increment.linear[r][i + 1],scales[r])),
                ],
                radius,
            });
        }
    }
    let pairs=swept_capsule_pairs(&motions,1e-10)?;
    // Captured witness assembly has not established a stable parallel win.
    // Keep production serial; explicit worker counts qualify the experiment.
    let workers=workers.unwrap_or(1);
    let witnesses=ordered_query_results(&pairs,workers,|&(a,b)| {
        let (ra, ia) = ids[a];
        let (rb, ib) = ids[b];
        if ra == rb && ia.abs_diff(ib) <= 2 {
            return Ok(None);
        }
        let mut worst: Option<(f64, f64, StrandResponse, f64, f64)> = None;
        let mut times: Vec<_> = (1..=32).map(|step| step as f64 / 32.).collect();
        let regions = if ia == 0 && ib == 0 {
            vec![
                (trim(motions[a]), motions[b]),
                (motions[a], trim(motions[b])),
            ]
        } else {
            vec![(motions[a], motions[b])]
        };
        let mut may_intersect=false;
        for (ma, mb) in regions {
            if !matches!(
                sweep_capsules(
                    ma,
                    mb,
                    CapsuleSweepOptions {
                        tolerance_m: 5e-11,
                        ..Default::default()
                    }
                )?,
                CapsuleSweep::Clear
            ) {
                may_intersect=true;
                let time = localized_minimum_time(ma, mb);
                if time > 0. {
                    times.push(time);
                }
            }
        }
        // Clear is a full continuous certificate, not an endpoint/sample
        // estimate. Only unresolved regions need counterexample sampling.
        if !may_intersect {return Ok(None);}
        times.sort_by(f64::total_cmp);
        times.dedup();
        for time in times {
            let at = |m: CapsuleMotion| {
                std::array::from_fn::<_, 2, _>(|p| {
                    add(m.start[p], mul(sub(m.end[p], m.start[p]), time))
                })
            };
            let aa = at(motions[a]);
            let bb = at(motions[b]);
            let regions = if ia == 0 && ib == 0 {
                vec![
                    (
                        trim(CapsuleMotion {
                            start: aa,
                            end: aa,
                            radius,
                        })
                        .start,
                        bb,
                        0.2,
                        0.,
                    ),
                    (
                        aa,
                        trim(CapsuleMotion {
                            start: bb,
                            end: bb,
                            radius,
                        })
                        .start,
                        0.,
                        0.2,
                    ),
                ]
            } else {
                vec![(aa, bb, 0., 0.)]
            };
            for (aa, bb, offset_a, offset_b) in regions {
                let (s, t, p, q) = super::super::super::segment_pair(aa[0], aa[1], bb[0], bb[1]);
                let delta = sub(p, q);
                let distance = len(delta);
                if distance >= 2. * radius - 1e-10 {
                    continue;
                }
                let s = offset_a + (1. - offset_a) * s;
                let t = offset_b + (1. - offset_b) * t;
                let normal = if distance > 1e-12 {
                    mul(delta, 1. / distance)
                } else {
                    let old = sub(point(&start[ra].x, ia, s), point(&start[rb].x, ib, t));
                    if len(old) <= 1e-12 {
                        return Err("swept contact has no separating direction");
                    }
                    unit(old)
                };
                if worst.as_ref().is_none_or(|w| distance < w.0) {
                    worst = Some((
                        distance,
                        time,
                        StrandResponse {
                            a: (ra, ia, s),
                            b: (rb, ib, t),
                            normal,
                            impulse: 0.,
                        },
                        offset_a,
                        offset_b,
                    ));
                }
            }
        }
        if let Some((_, time, mut pair, offset_a, offset_b)) = worst {
            let ma = if offset_a > 0. {
                trim(motions[a])
            } else {
                motions[a]
            };
            let mb = if offset_b > 0. {
                trim(motions[b])
            } else {
                motions[b]
            };
            let first = match sweep_capsules(
                ma,
                mb,
                CapsuleSweepOptions {
                    tolerance_m: 5e-11,
                    ..Default::default()
                },
            )? {
                CapsuleSweep::Clear => {
                    return Err("sampled intersection contradicts capsule sweep");
                }
                CapsuleSweep::InitialContact { .. } => 0.,
                CapsuleSweep::Approach { fraction, .. }
                | CapsuleSweep::IterationLimit { fraction, .. } => fraction,
            };
            let at = |m: CapsuleMotion| {
                std::array::from_fn::<_, 2, _>(|p| {
                    add(m.start[p], mul(sub(m.end[p], m.start[p]), first))
                })
            };
            let aa = at(ma);
            let bb = at(mb);
            let (_, _, p, q) = super::super::super::segment_pair(aa[0], aa[1], bb[0], bb[1]);
            let delta = sub(p, q);
            let distance = len(delta);
            if distance <= 1e-12 {
                return Err("first swept contact has no separating direction");
            }
            // The side comes from first approach, but the barycentric
            // feature belongs to the witnessed collision. Reusing the first
            // feature would constrain a different point after sliding.
            pair.normal = mul(delta, 1. / distance);
            return Ok(Some((pair,time,[scales[ra],scales[rb]])));
        }
        Ok(None)
    })?;
    // Geometry is read-only in workers. Publish cuts canonically only after
    // every query succeeds, then rebuild the same physical constraint owner.
    let mut changed=false;
    for (pair,time,scales) in witnesses {changed|=retain_cut(cuts,pair,time,scales);}
    if changed {
        // Rebuild from the canonical base and all witnessed affine planes.
        // add_constraint retains its canonical coalescing and strongest bound.
        *constraints = position_constraints(rods, base, radius)?.0;
        *groups = base.to_vec();
        groups.extend(cuts.iter().map(|cut|cut.pair.clone()));
        let (_,scales)=trust_components(rods,increment,groups)?;
        for cut in cuts {
            // Contact connectivity may change the common angular clock.
            // Relinearize every retained feature on that same current path;
            // obsolete clock paths are not extra physical constraints.
            cut.scales=[scales[cut.pair.a.0],scales[cut.pair.b.0]];
            let _ = add_cut(constraints, rods, start, &cut.pair, cut.time, radius, cut.scales)?;
        }
    }
    Ok(changed)
}

// Counterexample locator only. Closest-feature changes can create multiple
// minima, so this never replaces the final continuous admission query.
fn localized_minimum_time(a: CapsuleMotion, b: CapsuleMotion) -> f64 {
    let distance = |time: f64| {
        let at = |m: CapsuleMotion| {
            std::array::from_fn::<_, 2, _>(|p| {
                add(m.start[p], mul(sub(m.end[p], m.start[p]), time))
            })
        };
        let aa = at(a);
        let bb = at(b);
        let (_, _, p, q) = super::super::super::segment_pair(aa[0], aa[1], bb[0], bb[1]);
        len(sub(p, q))
    };
    let ratio = (5f64.sqrt() - 1.) * 0.5;
    let (mut lo, mut hi) = (0., 1.);
    let mut left = hi - ratio * (hi - lo);
    let mut right = lo + ratio * (hi - lo);
    let mut dl = distance(left);
    let mut dr = distance(right);
    for _ in 0..64 {
        if dl <= dr {
            hi = right;
            right = left;
            dr = dl;
            left = hi - ratio * (hi - lo);
            dl = distance(left);
        } else {
            lo = left;
            left = right;
            dl = dr;
            right = lo + ratio * (hi - lo);
            dr = distance(right);
        }
    }
    if dl <= dr { left } else { right }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn captured_blocking_pair_locator_retains_short_intersection() {
        let a = CapsuleMotion {start:[[-0.10571503383239862, 0.5353139616408988, 0.017056329341362284], [-0.10806919706770535, 0.5299472118400107, 0.023043577335470203]],end:[[-0.10577149379928885, 0.5351591661098012, 0.01705744597213609], [-0.10812570262070621, 0.5297880864893493, 0.02304079376955749]],radius:4e-5};
        let b = CapsuleMotion {start:[[-0.10800291278035071, 0.5311111101854139, 0.022075529636794787], [-0.1083593551281725, 0.5185706833323395, 0.03356630216649008]],end:[[-0.10806758277419319, 0.5309456355605204, 0.022085131983851477], [-0.10842155264723373, 0.5183892560338552, 0.03355856313709558]],radius:4e-5};
        let distance=|time:f64| {
            let at=|m:CapsuleMotion| std::array::from_fn::<_,2,_>(|i|add(m.start[i],mul(sub(m.end[i],m.start[i]),time)));
            let aa=at(a);let bb=at(b);
            let (_,_,p,q)=super::super::super::super::segment_pair(aa[0],aa[1],bb[0],bb[1]);
            len(sub(p,q))
        };
        let located=localized_minimum_time(a,b);
        let sampled=(0..=4096).map(|i| {let t=i as f64/4096.;(t,distance(t))}).min_by(|a,b|a.1.total_cmp(&b.1)).unwrap();
        eprintln!("BLOCKING PAIR located_time={located} located_distance={} sampled={sampled:?}",distance(located));
        eprintln!("BLOCKING PAIR certificates line={} planes={:?} sweep={:?} admitted_fraction={:?}",super::super::line_certificate::clear(a,b,512),super::super::interval_planes_clear(a,b,32768),sweep_capsules(a,b,CapsuleSweepOptions {tolerance_m:5e-11,..Default::default()}),super::super::pair_fraction(a,b,CapsuleSweepOptions {tolerance_m:5e-11,..Default::default()}));
        assert!(distance(located)<=sampled.1+1e-14,"locator missed a shorter interior feature");
        assert!(super::super::line_certificate::clear(a,b,512),"finite endpoint feature must certify the complete captured path");
        assert_eq!(super::super::pair_fraction(a,b,CapsuleSweepOptions {tolerance_m:5e-11,..Default::default()}).unwrap(),1.);
    }
    #[test]
    fn early_witness_residual_and_gradient_remain_in_physical_metres() {
        let make=|x|HairRod::new(vec![[x,0.,0.],[x,0.01,0.],[x,0.02,0.]],crate::hair::HairMaterial::default()).unwrap();
        let start=vec![make(0.),make(80e-6+3e-10)];
        let mut staged=start.clone();staged[0].x[0][0]+=1e-5;staged[0].x[1][0]+=4e-5;
        let pair=StrandResponse {a:(0,0,0.5),b:(1,0,0.5),normal:[-1.,0.,0.],impulse:0.};
        let scales=[0.25,0.5];
        let mut free=PositionIncrement {linear:vec![vec![[0.;3];3];2],angular:vec![vec![[0.;3];2];2]};
        free.linear[0][1]=[2e-5,0.,0.];free.linear[1][1]=[-1e-5,0.,0.];
        for time in [1e-8,0.25,0.5,1.] {
            let mut rows=Vec::new();add_cut(&mut rows,&staged,&start,&pair,time,40e-6,scales).unwrap();
            let at=|r:usize|add(mul(point(&start[r].x,0,0.5),1.-time),mul(
                add(point(&staged[r].x,0,0.5),mul(free.linear[r][1],0.5*scales[r])),time));
            let actual=dot(sub(at(0),at(1)),pair.normal)-80e-6;
            assert!((rows[0].speed(&free)-rows[0].bound-actual).abs()<1e-18);
            for entry in rows[0].entries.iter().filter(|entry|entry.point!=0) {
                let sign=if entry.rod==0 {-1.} else {1.};
                assert_eq!(entry.gradient,[sign*time*scales[entry.rod]*0.5,0.,0.]);
            }
            assert!(rows[0].entries.iter().all(|e|e.point!=0 || e.gradient==[0.;3]));
        }
    }
    #[test]
    fn angular_trust_collision_is_found_on_the_path_that_will_be_admitted() {
        let make=|x| HairRod::new(vec![[x,0.,0.],[x,0.01,0.],[x,0.02,0.]],
            crate::hair::HairMaterial::default()).unwrap();
        let start=vec![make(0.),make(80e-6+5e-10)];
        let mut staged=start.clone();
        for rod in &mut staged {rod.x[0][0]+=0.004;}
        let mut free=PositionIncrement {
            linear:vec![vec![[0.;3];3];2],angular:vec![vec![[0.;3];2];2],
        };
        for points in &mut free.linear {for p in &mut points[1..] {p[0]=0.004;}}
        free.angular[1][1]=[0.,0.,0.7];
        let full:Vec<Vec<V>>=staged.iter().enumerate().map(|(r,rod)| rod.x.iter().enumerate()
            .map(|(p,x)|add(*x,free.linear[r][p])).collect()).collect();
        assert_eq!(strand_fraction(&start,&full,40e-6).unwrap(),1.,"full rigid transport is clear");
        let base=vec![StrandResponse {a:(0,1,0.5),b:(1,1,0.5),normal:[-1.,0.,0.],impulse:0.}];
        let (_,scales)=trust_components(&staged,&free,&base).unwrap();
        let restricted:Vec<Vec<V>>=staged.iter().enumerate().map(|(r,rod)| rod.x.iter().enumerate()
            .map(|(p,x)|add(*x,mul(free.linear[r][p],scales[r]))).collect()).collect();
        assert!(strand_fraction(&start,&restricted,40e-6).unwrap()<1.);
        let mut constraints=Vec::new(); let mut groups=base.clone(); let mut cuts=Vec::new();
        assert!(activate(&mut constraints,&staged,&start,&free,&mut groups,&base,&mut cuts,40e-6).unwrap());
        assert!(constraints.iter().any(|c|c.speed(&free)<c.bound-1e-10),
            "scaled witness must reject the actual restricted trajectory");
        for cut in &cuts {
            let mut row=Vec::new();
            add_cut(&mut row,&staged,&start,&cut.pair,cut.time,40e-6,cut.scales).unwrap();
            let (a,i,s)=cut.pair.a; let (b,j,t)=cut.pair.b;
            let at=|r:usize,segment:usize,fraction:f64|add(
                mul(point(&start[r].x,segment,fraction),1.-cut.time),
                mul(point(&restricted[r],segment,fraction),cut.time));
            let actual=dot(sub(at(a,i,s),at(b,j,t)),cut.pair.normal)-80e-6;
            assert!(((row[0].speed(&free)-row[0].bound)-actual).abs()<1e-15,
                "affine load must include the same clocks and prescribed roots as geometry");
        }
    }
    #[test]
    fn shifted_witness_cannot_reopen_a_previously_constrained_crossing() {
        let make = |x| HairRod::new(
            vec![[x,0.,0.],[x,0.01,0.],[x,0.02,0.]],
            crate::hair::HairMaterial::default()).unwrap();
        let rods=vec![make(0.),make(0.001)];
        let old=StrandResponse { a:(0,1,0.1),b:(1,1,0.1),normal:[-1.,0.,0.],impulse:0. };
        let mut new=old.clone(); new.a.2=0.9; new.b.2=0.9;
        let mut free=PositionIncrement {
            linear:vec![vec![[0.;3];3];2],angular:vec![vec![[0.;3];2];2],
        };
        free.linear[0][1][0]=0.002;
        free.linear[0][2][0]=-0.002;
        let mut replacement=Vec::new();
        add_cut(&mut replacement,&rods,&rods,&new,1.,40e-6,[1.,1.]).unwrap();
        assert!(replacement[0].speed(&free)>=replacement[0].bound);
        let old_end=dot(sub(
            add(point(&rods[0].x,1,0.1),add(mul(free.linear[0][1],0.9),mul(free.linear[0][2],0.1))),
            point(&rods[1].x,1,0.1)),old.normal);
        assert!(old_end<0., "replacement admits crossing the old contact side");
        let mut cuts=Vec::new();
        assert!(retain_cut(&mut cuts,old.clone(),1.,[1.,1.]));
        assert!(retain_cut(&mut cuts,new,1.,[1.,1.]));
        assert!(!retain_cut(&mut cuts,old,1.,[1.,1.]));
        let mut retained=Vec::new();
        for cut in &cuts { add_cut(&mut retained,&rods,&rods,&cut.pair,cut.time,40e-6,cut.scales).unwrap(); }
        assert!(retained.iter().any(|c| c.speed(&free)<c.bound-1e-10));
        assert!(retained.iter().all(|c| c.entries.iter().all(|e| e.point!=0 || e.gradient==[0.;3])));
    }
    #[test]
    fn localized_witness_finds_captured_penetration_between_coarse_samples() {
        let a = CapsuleMotion {
            radius: 40e-6,
            start: [
                [
                    -0.05320794855818693,
                    0.8135695376889923,
                    0.038037514531762524,
                ],
                [
                    -0.05448365126943721,
                    0.8122917787067718,
                    0.03580298606584595,
                ],
            ],
            end: [
                [
                    -0.053139853296977235,
                    0.8137719008823939,
                    0.03808365478321574,
                ],
                [
                    -0.054390995942093075,
                    0.812423006562546,
                    0.03587609922408957,
                ],
            ],
        };
        let b = CapsuleMotion {
            radius: 40e-6,
            start: [
                [
                    -0.05364587941295736,
                    0.8132718340966427,
                    0.03860107685139109,
                ],
                [
                    -0.05451064581400578,
                    0.8123308423697174,
                    0.035916939966533064,
                ],
            ],
            end: [
                [
                    -0.05340466461219224,
                    0.812612857539943,
                    0.038785437810288384,
                ],
                [
                    -0.05425001225440255,
                    0.8116082804903335,
                    0.036117569023514415,
                ],
            ],
        };
        let gap = |time| {
            let at = |m: CapsuleMotion| {
                std::array::from_fn::<_, 2, _>(|p| {
                    add(m.start[p], mul(sub(m.end[p], m.start[p]), time))
                })
            };
            let aa = at(a);
            let bb = at(b);
            let (_, _, p, q) = super::super::super::super::segment_pair(aa[0], aa[1], bb[0], bb[1]);
            len(sub(p, q)) - a.radius - b.radius
        };
        for step in 0..=32 {
            assert!(gap(step as f64 / 32.) >= -1e-10);
        }
        let time = localized_minimum_time(a, b);
        assert!(time > 0. && time < 1. / 32.);
        assert!(gap(time) < -1e-7);
    }
    #[test]
    fn swept_cut_keeps_the_first_contact_side_after_candidate_crosses() {
        let make = |x| {
            HairRod::new(
                vec![[x, 0., 0.], [x, 0.01, 0.], [x, 0.02, 0.]],
                crate::hair::HairMaterial::default(),
            )
            .unwrap()
        };
        let start = vec![make(0.), make(0.001)];
        let mut staged = start.clone();
        staged[0].x[0][0] = 0.0033;
        let mut free = PositionIncrement {
            linear: vec![vec![[0.; 3]; 3]; 2],
            angular: vec![vec![[0.; 3]; 2]; 2],
        };
        for p in 1..3 {
            free.linear[0][p][0] = 0.0033;
        }
        let mut constraints = Vec::new();
        let mut groups = Vec::new();
        let mut cuts = Vec::new();
        assert!(
            activate(
                &mut constraints,
                &staged,
                &start,
                &free,
                &mut groups,
                &[],
                &mut cuts,
                40e-6
            )
            .unwrap()
        );
        assert!(groups.iter().all(|pair| pair.normal[0] < -0.99));
        let previous_count = cuts.len();
        for p in 1..3 {
            free.linear[0][p][0] = 0.0034;
        }
        assert!(
            activate(
                &mut constraints,
                &staged,
                &start,
                &free,
                &mut groups,
                &[],
                &mut cuts,
                40e-6
            )
            .unwrap()
        );
        assert!(cuts.len() > previous_count, "new witnesses must preserve prior planes");
        assert_eq!(groups.len(), cuts.len());
        assert!(constraints.iter().any(|c| c.speed(&free) < c.bound - 1e-10));
        assert!(constraints.iter().all(|c| {
            c.entries
                .iter()
                .all(|e| e.point != 0 || e.gradient == [0.; 3])
        }));
    }
    #[test]
    fn affine_cut_includes_prescribed_root_and_time_interpolation() {
        let make = |x| {
            HairRod::new(
                vec![[x, 0., 0.], [x, 0.01, 0.], [x, 0.02, 0.]],
                crate::hair::HairMaterial::default(),
            )
            .unwrap()
        };
        let start = vec![make(0.), make(0.001)];
        let mut staged = start.clone();
        staged[0].x[0][0] = 0.001;
        let pair = StrandResponse {
            a: (0, 0, 0.5),
            b: (1, 0, 0.5),
            normal: [-1., 0., 0.],
            impulse: 0.,
        };
        let mut constraints = Vec::new();
        add_cut(&mut constraints, &staged, &start, &pair, 0.5, 40e-6,[1.,1.]).unwrap();
        let mut free = PositionIncrement {
            linear: vec![vec![[0.; 3]; 3]; 2],
            angular: vec![vec![[0.; 3]; 2]; 2],
        };
        free.linear[0][1][0] = 0.001;
        let old = dot(
            sub(point(&start[0].x, 0, 0.5), point(&start[1].x, 0, 0.5)),
            pair.normal,
        );
        let end = dot(
            sub(
                add(point(&staged[0].x, 0, 0.5), mul(free.linear[0][1], 0.5)),
                point(&staged[1].x, 0, 0.5),
            ),
            pair.normal,
        );
        let actual = 0.5 * old + 0.5 * end - 80e-6;
        assert!(
            ((constraints[0].speed(&free) - constraints[0].bound) - actual).abs() < 1e-15
        );
        assert!(
            constraints[0]
                .entries
                .iter()
                .all(|e| e.point != 0 || e.gradient == [0.; 3])
        );
    }
}

#[cfg(test)]
pub(super) fn activate_reference(
    constraints: &mut Vec<Constraint>,
    rods: &[HairRod],
    start: &[HairRod],
    increment: &PositionIncrement,
    groups: &mut Vec<StrandResponse>,
    base: &[StrandResponse],
    cuts: &mut Vec<Cut>,
    radius: f64,
) -> Result<bool, &'static str> {
    let mut motions = Vec::new();
    let mut ids = Vec::new();
    let (_,scales)=trust_components(rods,increment,groups)?;
    for (r, rod) in rods.iter().enumerate() {
        for i in 0..rod.x.len() - 1 {
            ids.push((r, i));
            motions.push(CapsuleMotion {
                start: [start[r].x[i], start[r].x[i + 1]],
                end: [
                    add(rod.x[i], mul(increment.linear[r][i],scales[r])),
                    add(rod.x[i + 1], mul(increment.linear[r][i + 1],scales[r])),
                ],
                radius,
            });
        }
    }
    let mut changed = false;
    for (a, b) in swept_capsule_pairs(&motions, 1e-10)? {
        let (ra, ia) = ids[a];
        let (rb, ib) = ids[b];
        if ra == rb && ia.abs_diff(ib) <= 2 {
            continue;
        }
        let mut worst: Option<(f64, f64, StrandResponse, f64, f64)> = None;
        let mut times: Vec<_> = (1..=32).map(|step| step as f64 / 32.).collect();
        let regions = if ia == 0 && ib == 0 {
            vec![
                (trim(motions[a]), motions[b]),
                (motions[a], trim(motions[b])),
            ]
        } else {
            vec![(motions[a], motions[b])]
        };
        for (ma, mb) in regions {
            if !matches!(
                sweep_capsules(
                    ma,
                    mb,
                    CapsuleSweepOptions {
                        tolerance_m: 5e-11,
                        ..Default::default()
                    }
                )?,
                CapsuleSweep::Clear
            ) {
                let time = localized_minimum_time(ma, mb);
                if time > 0. {
                    times.push(time);
                }
            }
        }
        times.sort_by(f64::total_cmp);
        times.dedup();
        for time in times {
            let at = |m: CapsuleMotion| {
                std::array::from_fn::<_, 2, _>(|p| {
                    add(m.start[p], mul(sub(m.end[p], m.start[p]), time))
                })
            };
            let aa = at(motions[a]);
            let bb = at(motions[b]);
            let regions = if ia == 0 && ib == 0 {
                vec![
                    (
                        trim(CapsuleMotion {
                            start: aa,
                            end: aa,
                            radius,
                        })
                        .start,
                        bb,
                        0.2,
                        0.,
                    ),
                    (
                        aa,
                        trim(CapsuleMotion {
                            start: bb,
                            end: bb,
                            radius,
                        })
                        .start,
                        0.,
                        0.2,
                    ),
                ]
            } else {
                vec![(aa, bb, 0., 0.)]
            };
            for (aa, bb, offset_a, offset_b) in regions {
                let (s, t, p, q) = super::super::super::segment_pair(aa[0], aa[1], bb[0], bb[1]);
                let delta = sub(p, q);
                let distance = len(delta);
                if distance >= 2. * radius - 1e-10 {
                    continue;
                }
                let s = offset_a + (1. - offset_a) * s;
                let t = offset_b + (1. - offset_b) * t;
                let normal = if distance > 1e-12 {
                    mul(delta, 1. / distance)
                } else {
                    let old = sub(point(&start[ra].x, ia, s), point(&start[rb].x, ib, t));
                    if len(old) <= 1e-12 {
                        return Err("swept contact has no separating direction");
                    }
                    unit(old)
                };
                if worst.as_ref().is_none_or(|w| distance < w.0) {
                    worst = Some((
                        distance,
                        time,
                        StrandResponse {
                            a: (ra, ia, s),
                            b: (rb, ib, t),
                            normal,
                            impulse: 0.,
                        },
                        offset_a,
                        offset_b,
                    ));
                }
            }
        }
        if let Some((_, time, mut pair, offset_a, offset_b)) = worst {
            let ma = if offset_a > 0. {
                trim(motions[a])
            } else {
                motions[a]
            };
            let mb = if offset_b > 0. {
                trim(motions[b])
            } else {
                motions[b]
            };
            let first = match sweep_capsules(
                ma,
                mb,
                CapsuleSweepOptions {
                    tolerance_m: 5e-11,
                    ..Default::default()
                },
            )? {
                CapsuleSweep::Clear => {
                    return Err("sampled intersection contradicts capsule sweep");
                }
                CapsuleSweep::InitialContact { .. } => 0.,
                CapsuleSweep::Approach { fraction, .. }
                | CapsuleSweep::IterationLimit { fraction, .. } => fraction,
            };
            let at = |m: CapsuleMotion| {
                std::array::from_fn::<_, 2, _>(|p| {
                    add(m.start[p], mul(sub(m.end[p], m.start[p]), first))
                })
            };
            let aa = at(ma);
            let bb = at(mb);
            let (_, _, p, q) = super::super::super::segment_pair(aa[0], aa[1], bb[0], bb[1]);
            let delta = sub(p, q);
            let distance = len(delta);
            if distance <= 1e-12 {
                return Err("first swept contact has no separating direction");
            }
            // The side comes from first approach, but the barycentric
            // feature belongs to the witnessed collision. Reusing the first
            // feature would constrain a different point after sliding.
            pair.normal = mul(delta, 1. / distance);
            changed |= retain_cut(cuts, pair, time, [scales[ra],scales[rb]]);
        }
    }
    if changed {
        // Rebuild from the canonical base and all witnessed affine planes.
        // add_constraint retains its canonical coalescing and strongest bound.
        *constraints = position_constraints(rods, base, radius)?.0;
        *groups = base.to_vec();
        groups.extend(cuts.iter().map(|cut|cut.pair.clone()));
        let (_,scales)=trust_components(rods,increment,groups)?;
        for cut in cuts {
            // Contact connectivity may change the common angular clock.
            // Relinearize every retained feature on that same current path;
            // obsolete clock paths are not extra physical constraints.
            cut.scales=[scales[cut.pair.a.0],scales[cut.pair.b.0]];
            let _ = add_cut(constraints, rods, start, &cut.pair, cut.time, radius, cut.scales)?;
        }
    }
    Ok(changed)
}
