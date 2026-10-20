//! Original section-volume union. Removes intersecting loft seams before grooming.
use super::{AnatomySurface, BreedAnatomy, surface};
use glam::Vec3;
use std::collections::HashMap;
struct Segment {
    a: Vec3,
    delta: Vec3,
    radii: [f32; 4],
}
struct Volume {
    segments: Vec<Segment>,
    min: Vec3,
    max: Vec3,
    end: Vec3,
    end_axis: Vec3,
    flat_end: bool,
    longitudinal_axis: usize,
    feline_head: bool,
}
impl Volume {
    fn distance(&self, p: Vec3) -> f32 {
        // Anatomical sweeps are monotone along Y (limbs/neck) or Z
        // (torso/head/tail). Select the cross-section by that coordinate,
        // not Euclidean centreline ownership or a union of capped slices.
        // Adjacent sections therefore meet exactly without frontal annuli.
        let axis=self.longitudinal_axis;
        let first=&self.segments[0];
        let last=self.segments.last().unwrap();
        let sign=(self.end[axis]-first.a[axis]).signum();
        let before=(p[axis]-first.a[axis])*sign<0.0;
        let after=(p[axis]-self.end[axis])*sign>0.0;
        let (segment,u)=if before {(first,0.0)} else if after {(last,1.0)} else {
            self.segments.iter().find_map(|segment| {
                let t=(p[axis]-segment.a[axis])/segment.delta[axis];
                (t>=0.0&&t<=1.0).then_some((segment,t))
            }).unwrap_or((last,1.0))
        };
        let centre=segment.a+segment.delta*u;
        let q=p-centre;
        let w=segment.radii[0]+(segment.radii[2]-segment.radii[0])*u;
        let d=segment.radii[1]+(segment.radii[3]-segment.radii[1])*u;
        let transverse=if axis==1 {2} else {1};
        let axial=if before||after {q[axis]/w.min(d)} else {0.0};
        let progress=((p[axis]-first.a[axis])/(self.end[axis]-first.a[axis])).clamp(0.,1.);
        let exponent=if self.feline_head {
            let muzzle=((progress-0.68)/0.20).clamp(0.,1.);
            let skull=if q[transverse]<0. {1.45} else {2.0};
            skull+(2.6-skull)*muzzle*muzzle*(3.-2.*muzzle)
        } else {2.0};
        let radial=((q.x/w).abs().powf(exponent)
            +(q[transverse]/d).abs().powf(exponent)).powf(1./exponent);
        let distance=((radial*radial+axial*axial).sqrt()-1.)*w.min(d);
        if self.flat_end {
            distance.max((p - self.end).dot(self.end_axis))
        } else {
            distance
        }
    }
}
struct Field {
    volumes: Vec<Volume>,
    blend: f32,
    paw_blend: f32,
    paw_height: f32,
    sockets: Vec<(Vec3, Vec3, Vec3)>,
    nasal_plane: f32,
}
impl Field {
    fn value(&self, p: Vec3) -> f32 {
        // Small digit volumes need a much tighter blend than torso muscles.
        let t=((p.y-self.paw_height*0.6)/(self.paw_height*0.8)).clamp(0.,1.);
        let blend=self.paw_blend+(self.blend-self.paw_blend)*t*t*(3.-2.*t);
        let body = self.volumes.iter().fold(f32::INFINITY, |best, v| {
            let bound = (v.min - p).max(p - v.max).max(Vec3::ZERO).length();
            if bound > best.max(0.) + blend {
                return best;
            }
            let d = v.distance(p);
            if !best.is_finite() {
                return d;
            }
            let h = (0.5 + 0.5 * (d - best) / blend).clamp(0., 1.);
            d * (1. - h) + best * h - blend * h * (1. - h)
        });
        // Whisker-pad blends must remain behind the separately authored nasal
        // planum; otherwise their smooth union buries the visible nose.
        let body = body.max(p.z - self.nasal_plane);
        self.sockets
            .iter()
            .fold(body, |distance, (center, radii, normal)| {
                let right = Vec3::new(normal.z, 0.0, -normal.x).normalize();
                let up = normal.cross(right);
                let q = p - *center;
                let local = Vec3::new(q.dot(right), q.dot(up), q.dot(*normal));
                let cut = (local / *radii).length() - 1.0;
                distance.max(-cut * radii.min_element())
            })
    }

}
fn section_volume(rings: &[surface::Section], flat_end: bool, feline_head: bool) -> Volume {
    let mut min=Vec3::splat(f32::INFINITY);
    let mut max=Vec3::splat(f32::NEG_INFINITY);
    for r in rings {
        let radius=r.w.max(r.d);
        min=min.min(r.p-Vec3::splat(radius));max=max.max(r.p+Vec3::splat(radius));
    }
    Volume {
        segments:rings.windows(2).map(|r|Segment{a:r[0].p,delta:r[1].p-r[0].p,
            radii:[r[0].w,r[0].d,r[1].w,r[1].d]}).collect(),
        min,max,end:rings.last().unwrap().p,
        end_axis:(rings[rings.len()-1].p-rings[rings.len()-2].p).normalize(),
        flat_end,feline_head,
        longitudinal_axis:if (rings.last().unwrap().p.y-rings[0].p.y).abs()
            >(rings.last().unwrap().p.z-rings[0].p.z).abs() {1} else {2},
    }
}
pub(super) fn eye_anchor(a: BreedAnatomy, provisional: Vec3, normal: Vec3) -> Vec3 {
    let Ok(source)=surface::build(a) else {return provisional;};
    let head=section_volume(&source.volumes[2],a.breed.species()==super::Species::Dog,
        a.breed.species()==super::Species::Cat && a.breed!=super::Breed::BritishShorthair);
    let mut lo=-a.skull_width*0.5;let mut hi=a.skull_width*0.5;
    if head.distance(provisional+normal*lo)>=0. || head.distance(provisional+normal*hi)<=0. {
        return provisional;
    }
    for _ in 0..24 {
        let mid=(lo+hi)*0.5;
        if head.distance(provisional+normal*mid)<0. {lo=mid;} else {hi=mid;}
    }
    provisional+normal*((lo+hi)*0.5)
}
fn orient_closed_surface(mesh: &mut AnatomySurface) -> Result<(),String> {
    let count=mesh.indices.len()/3;
    let mut neighbours=vec![[(usize::MAX,0_i8);3];count];
    let mut edges:HashMap<(u32,u32),(usize,usize,i8)>=HashMap::new();
    for (face,tri) in mesh.indices.chunks_exact(3).enumerate() {
        for slot in 0..3 {
            let (a,b)=(tri[slot],tri[(slot+1)%3]);
            let direction=if a<b {1_i8}else{-1_i8};
            let key=(a.min(b),a.max(b));
            if let Some(&(other,other_slot,other_direction))=edges.get(&key) {
                let relation=-direction*other_direction;
                if neighbours[other][other_slot].0!=usize::MAX {return Err("nonmanifold sculpt edge".into());}
                neighbours[face][slot]=(other,relation);
                neighbours[other][other_slot]=(face,relation);
            } else {edges.insert(key,(face,slot,direction));}
        }
    }
    let mut signs=vec![0_i8;count];
    for seed in 0..count {
        if signs[seed]!=0 {continue;}
        signs[seed]=1;
        let mut component=vec![seed];let mut cursor=0;
        while cursor<component.len() {
            let face=component[cursor];cursor+=1;
            for &(other,relation) in &neighbours[face] {
                if other==usize::MAX {return Err("open sculpt edge".into());}
                let required=signs[face]*relation;
                if signs[other]==0 {signs[other]=required;component.push(other);}
                else if signs[other]!=required {return Err("nonorientable sculpt component".into());}
            }
        }
        let mut volume=0.0_f64;
        for &face in &component {
            let p: [glam::DVec3;3]=std::array::from_fn(|k|
                Vec3::from(mesh.positions[mesh.indices[face*3+k] as usize]).as_dvec3());
            volume+=p[0].dot(p[1].cross(p[2]))*signs[face] as f64;
        }
        let outward=if volume<0. {-1}else{1};
        for face in component {
            if signs[face]*outward<0 {mesh.indices.swap(face*3+1,face*3+2);}
        }
    }
    Ok(())
}
pub(super) fn build(a: BreedAnatomy) -> Result<AnatomySurface, String> {
    let source = surface::build(a)?;
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    let volumes: Vec<Volume> = source
        .volumes
        .iter()
        .enumerate()
        .map(|(index,rings)| {
            let volume=section_volume(rings,index==2 && a.breed.species()==super::Species::Dog,
                index==2 && a.breed.species()==super::Species::Cat && a.breed!=super::Breed::BritishShorthair);
            min=min.min(volume.min);max=max.max(volume.max);volume
        }).collect();
    if volumes.iter().any(|v| {
        let axis=v.longitudinal_axis;
        let sign=(v.end[axis]-v.segments[0].a[axis]).signum();
        v.segments.iter().any(|s|s.delta[axis]*sign<=0.0)
    }) {return Err("anatomical sweep coordinate must be strictly monotone".into());}
    let step = a.shoulder_height / 180.;
    let margin = step * 3.;
    min -= Vec3::splat(margin);
    max += Vec3::splat(margin);
    let dims = ((max - min) / step).ceil().as_uvec3() + glam::UVec3::ONE;
    let count = dims.x as usize * dims.y as usize * dims.z as usize;
    if count > 24_000_000 {
        return Err("sculpt grid budget exceeded".into());
    }
    let sockets: Vec<_> = [-1.0, 1.0]
        .into_iter()
        .map(|side| {
            let normal = a.eye_normal(side);
            let center = a.eye_center(side);
            let r = a.eye_radius();
            (center, Vec3::new(r * 1.08, r * 0.62, r * 0.75), normal)
        })
        .collect();
    let field = Field {
        volumes,
        // Muscle attachments need a broad skin transition; a narrow union
        // leaves a visible seam around each separately authored shoulder.
        blend: a.shoulder_height * 0.065,
        paw_blend: a.paw_half_width*0.10,
        paw_height: a.paw_half_width*2.0,
        sockets,
        nasal_plane: a.nose_center().z - 0.001,
    };
    let index = |x: u32, y: u32, z: u32| (x + dims.x * (y + dims.y * z)) as usize;
    let point = |i: usize| {
        min + Vec3::new(
            (i % dims.x as usize) as f32,
            ((i / dims.x as usize) % dims.y as usize) as f32,
            (i / (dims.x * dims.y) as usize) as f32,
        ) * step
    };
    let values: Vec<_> = (0..count).map(|i| field.value(point(i))).collect();
    let mut out = AnatomySurface::default();
    let mut edges = HashMap::new();
    let tetra = [
        [0, 1, 3, 7],
        [0, 3, 2, 7],
        [0, 2, 6, 7],
        [0, 6, 4, 7],
        [0, 4, 5, 7],
        [0, 5, 1, 7],
    ];
    for z in 0..dims.z - 1 {
        for y in 0..dims.y - 1 {
            for x in 0..dims.x - 1 {
                let cube = [
                    index(x, y, z),
                    index(x + 1, y, z),
                    index(x, y + 1, z),
                    index(x + 1, y + 1, z),
                    index(x, y, z + 1),
                    index(x + 1, y, z + 1),
                    index(x, y + 1, z + 1),
                    index(x + 1, y + 1, z + 1),
                ];
                if cube.iter().all(|&i| values[i] > 0.) || cube.iter().all(|&i| values[i] <= 0.) {
                    continue;
                }
                for t in tetra {
                    let mut inside = Vec::new();
                    let mut outside = Vec::new();
                    for k in t {
                        let i = cube[k];
                        if values[i] <= 0. {
                            inside.push(i);
                        } else {
                            outside.push(i);
                        }
                    }
                    if inside.is_empty() || outside.is_empty() {
                        continue;
                    }
                    let outward=outside.iter().map(|&i|point(i)).sum::<Vec3>()/outside.len() as f32
                        -inside.iter().map(|&i|point(i)).sum::<Vec3>()/inside.len() as f32;
                    let mut crossing = |a: usize, b: usize| {
                        let t=(values[a]/(values[a]-values[b])).clamp(0.0,1.0);
                        // Snap crossings within numerical noise of a grid vertex
                        // to one shared topological vertex, across every edge.
                        let (key,p)=if t<1e-5 {((a,a),point(a))}
                            else if t>1.0-1e-5 {((b,b),point(b))}
                            else {((a.min(b),a.max(b)),point(a).lerp(point(b),t))};
                        *edges.entry(key).or_insert_with(|| {
                            let i = out.positions.len() as u32;
                            out.positions.push(p.to_array());
                            out.normals.push([0.0;3]);
                            i
                        })
                    };
                    let tris = if inside.len() == 1 {
                        vec![[
                            crossing(inside[0], outside[0]),
                            crossing(inside[0], outside[1]),
                            crossing(inside[0], outside[2]),
                        ]]
                    } else if outside.len() == 1 {
                        vec![[
                            crossing(outside[0], inside[0]),
                            crossing(outside[0], inside[1]),
                            crossing(outside[0], inside[2]),
                        ]]
                    } else {
                        let a = crossing(inside[0], outside[0]);
                        let b = crossing(inside[0], outside[1]);
                        let c = crossing(inside[1], outside[1]);
                        let d = crossing(inside[1], outside[0]);
                        vec![[a, b, c], [a, c, d]]
                    };
                    for mut tri in tris {
                        if tri[0]==tri[1] || tri[1]==tri[2] || tri[2]==tri[0] {continue;}
                        let p = tri.map(|i| Vec3::from(out.positions[i as usize]));
                        // Orient from the tetrahedron's inside samples toward
                        // outside samples, not a separately evaluated gradient.
                        // The latter can disagree with the sampled surface and
                        // flip adjacent faces, exposing holes under backface culling.
                        if (p[1] - p[0]).cross(p[2] - p[0]).dot(outward) < 0. {
                            tri.swap(1, 2);
                        }
                        out.indices.extend(tri);
                    }
                }
            }
        }
    }
    if out.positions.is_empty() {
        return Err("empty sculpt".into());
    }
    let mut used=vec![false;out.positions.len()];
    for &i in &out.indices {used[i as usize]=true;}
    let mut remap=vec![0;out.positions.len()];
    let mut positions=Vec::with_capacity(out.positions.len());
    for (i,&p) in out.positions.iter().enumerate() {
        if used[i] {remap[i]=positions.len() as u32;positions.push(p);}
    }
    out.positions=positions;
    out.normals=vec![[0.0;3];out.positions.len()];
    for i in &mut out.indices {*i=remap[*i as usize];}
    orient_closed_surface(&mut out)?;
    // Nonshrinking relaxation removes the sub-grid span/cell ripple before
    // grooming. Keep distal digits nearly fixed; preserve the nasal limit.
    let mut degrees=vec![0u32;out.positions.len()];
    for tri in out.indices.chunks_exact(3) {for &i in tri {degrees[i as usize]+=2;}}
    let mut sums=vec![Vec3::ZERO;out.positions.len()];
    for _ in 0..6 {
        for coefficient in [0.5,-0.53] {
            sums.fill(Vec3::ZERO);
            for tri in out.indices.chunks_exact(3) {
                let [i,j,k]=[tri[0] as usize,tri[1] as usize,tri[2] as usize];
                let [p,q,r]=[Vec3::from(out.positions[i]),Vec3::from(out.positions[j]),Vec3::from(out.positions[k])];
                sums[i]+=q+r; sums[j]+=p+r; sums[k]+=p+q;
            }
            for (i,p) in out.positions.iter_mut().enumerate() {
                let original=Vec3::from(*p);
                let protect=if original.y<a.paw_half_width*2.5 {0.2} else {1.0};
                let delta=((sums[i]/degrees[i] as f32-original)*coefficient*protect).clamp_length_max(step*0.45);
                let mut relaxed=original+delta;
                relaxed.z=relaxed.z.min(field.nasal_plane);
                *p=relaxed.to_array();
            }
        }
    }
    for tri in out.indices.chunks_exact(3) {
        let p: [Vec3;3]=std::array::from_fn(|k|Vec3::from(out.positions[tri[k] as usize]));
        let normal=(p[1]-p[0]).cross(p[2]-p[0]);
        for &i in tri {out.normals[i as usize]=(Vec3::from(out.normals[i as usize])+normal).to_array();}
    }
    for normal in &mut out.normals {*normal=Vec3::from(*normal).normalize_or_zero().to_array();}
    super::ears::append(&mut out, a);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn eyes_are_seated_on_the_authored_cranium() {
        for breed in super::super::Breed::ALL {
            let a=breed.anatomy();let source=surface::build(a).unwrap();
            let head=section_volume(&source.volumes[2],breed.species()==super::super::Species::Dog,
                breed.species()==super::super::Species::Cat && breed!=super::super::Breed::BritishShorthair);
            for side in [-1.,1.] {
                let p=a.eye_center(side);let n=a.eye_normal(side);
                assert!(head.distance(p).abs()<1e-5,"floating or buried {breed:?} eye: {}",head.distance(p));
                assert!(head.distance(p-n*0.002)<0. && head.distance(p+n*0.002)>0.);
            }
        }
    }
    #[test]
    fn every_gameplay_sculpt_has_finite_normals_and_preserves_the_nasal_plane() {
        for breed in super::super::Breed::ALL {
            let anatomy = breed.anatomy();
            let mesh = build(anatomy).unwrap_or_else(|error| panic!("{breed:?}: {error}"));
            assert_eq!(mesh.positions.len(), mesh.normals.len());
            let mut topology=std::collections::HashMap::new();
            for tri in mesh.indices.chunks_exact(3) {
                for k in 0..3 {
                    let (i,j)=(tri[k],tri[(k+1)%3]);
                    let entry=topology.entry((i.min(j),i.max(j))).or_insert((0usize,0i32));
                    entry.0+=1; entry.1+=if i<j {1} else {-1};
                }
            }
            assert!(topology.values().all(|&(count,winding)|count==2&&winding==0),
                "{breed:?}: boundary {}, nonmanifold {}, wrong winding {}, examples {:?}",
                topology.values().filter(|&&(c,_)|c==1).count(),
                topology.values().filter(|&&(c,_)|c>2).count(),
                topology.values().filter(|&&(c,w)|c==2&&w!=0).count(),
                topology.iter().filter(|(_,v)|v.0!=2||v.1!=0).take(4).map(|(&(i,j),v)| (mesh.positions[i as usize],mesh.positions[j as usize],*v)).collect::<Vec<_>>());

            assert_eq!(mesh.positions.len(), mesh.inner_ear.len());
            for (&p, &n) in mesh.positions.iter().zip(&mesh.normals) {
                assert!(Vec3::from(p).is_finite());
                assert!(Vec3::from(n).is_finite());
                assert!(Vec3::from(n).length_squared() > 0.9);
                assert!(p[2] <= anatomy.nose_center().z - 0.0009);
            }
            assert!(mesh.indices.iter().all(|&i| (i as usize) < mesh.positions.len()));
            if breed == super::super::Breed::MaineCoon {
                let cap_half_width=mesh.positions.iter().zip(&mesh.normals)
                    .filter(|(p,n)| (p[2]-(anatomy.nose_center().z-0.001)).abs()<0.00001 && n[2]>0.95)
                    .map(|(p,_)|p[0].abs()).fold(0.0,f32::max);
                assert!(cap_half_width < anatomy.skull_width*0.11,
                    "broad flat muzzle cap forms a facial disk: {cap_half_width}");
            }

        }
    }

    #[test]
    fn overlapping_spans_have_no_internal_zero_membrane() {
        let segment=|z:f32| Segment{a:Vec3::new(0.0,0.0,z),delta:Vec3::Z,radii:[0.2;4]};
        let volume=Volume{segments:vec![segment(0.0),segment(1.0)],min:Vec3::splat(-1.),max:Vec3::splat(3.),
            end:Vec3::Z*2.0,end_axis:Vec3::Z,flat_end:false,feline_head:false,longitudinal_axis:2};
        for z in [0.99999,1.0,1.00001] {
            assert!(volume.distance(Vec3::new(0.1,0.0,z)) < -0.09);
        }
        assert!((volume.distance(Vec3::new(0.1,0.0,0.99999))-volume.distance(Vec3::new(0.1,0.0,1.00001))).abs()<0.0001);
    }

    #[test]
    fn rounded_section_end_is_continuous_and_overlaps_its_neighbour() {
        let volume = Volume {
            min: Vec3::splat(-1.),
            max: Vec3::splat(2.),
            end: Vec3::Z,
            end_axis: Vec3::Z,
            flat_end: false,
            feline_head: false,
            longitudinal_axis: 2,
            segments: vec![Segment {
                a: Vec3::ZERO,
                delta: Vec3::Z,
                radii: [1.; 4],
            }],
        };
        let before = volume.distance(Vec3::new(0.5, 0., 1. - 1e-5));
        let after = volume.distance(Vec3::new(0.5, 0., 1. + 1e-5));
        assert!(
            (before - after).abs() < 0.0001,
            "a disc cap would create a surface discontinuity"
        );
        assert!(after < 0.);
        assert!(volume.distance(Vec3::new(0.5, 0., 2.5)) > 0.);
    }
}
