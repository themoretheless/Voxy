#!/usr/bin/env python3
"""Prepare an isolated before/after hinge diagnostic for a supplied rig candidate.

This compares three no-object poses; it is not a rig acceptance test.
Run the printed Cargo command separately. The active application rig is untouched.
"""
import argparse
from pathlib import Path
import shutil

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("candidate", type=Path)
parser.add_argument("--output", type=Path, default=Path("target/hand-hinge-comparison"))
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
out = args.output.resolve()
out.mkdir(parents=True, exist_ok=True)
source = args.candidate.read_text()
start = source.index("                    let angle = |points: [Vec3; 4]| {", source.index("fn correct_grasp_skin"))
end = source.index("                    let denominator: f32", start)
current = (root / "crates/voxy_app/src/female_rig.rs").read_text()
a = current.index("                    let Some(angle) =", current.index("fn correct_grasp_skin"))
b = current.index("                    let denominator: f32", a)
updated = source[:start] + current[a:b] + source[end:]
test = r'''
#[cfg(test)] mod hinge_comparison {
use super::*;
#[test] fn sampled_prototype_fold_diagnostic() {
let asset=voxy_render::ObjAsset::parse(include_str!("/Users/themoretheless/Documents/ChatGPT/Voxy/assets/characters/blender-female/body.obj"),voxy_render::ObjLimits::default()).unwrap();
let rest=asset.mesh.vertices();let mut rig=FemaleRig::new(rest).unwrap();rig.bind_hand_surface(rest,asset.mesh.indices()).unwrap();rig.set_grasp_object(None);
for fraction in [0.5,0.75,1.0] {
rig.set_grasp(fraction).unwrap();let mut posed=rest.to_vec();rig.deform(&mut posed,0.);
let mut worst=0f32;let mut area=f32::INFINITY;
for &(ids,rest_angle) in &rig.thumb_web_hinges {
let p=ids.map(|i|Vec3::from_array(posed[i].position));let rp=ids.map(|i|Vec3::from_array(rest[i].position));
let a=(p[1]-p[0]).cross(p[2]-p[0]);let b=(p[0]-p[1]).cross(p[3]-p[1]);
let extra=a.normalize().dot(b.normalize()).clamp(-1.,1.).acos()-rest_angle;assert!(extra.is_finite());worst=worst.max(extra);
area=area.min(a.length()/(rp[1]-rp[0]).cross(rp[2]-rp[0]).length());
}
println!("{} fraction {fraction}: extra_crease {worst}, min_area {area}",module_path!());
}
}
}
'''
test += r'''#[cfg(test)] mod articulation_isolation {
use super::*;
#[test] fn raw_thumb_articulation_isolation() {
let asset=voxy_render::ObjAsset::parse(include_str!("/Users/themoretheless/Documents/ChatGPT/Voxy/assets/characters/blender-female/body.obj"),voxy_render::ObjLimits::default()).unwrap();
let rest=asset.mesh.vertices();let mut rig=FemaleRig::new(rest).unwrap();rig.bind_hand_surface(rest,asset.mesh.indices()).unwrap();rig.set_grasp_object(None);rig.set_body_motion(false);
for case in 0..8 {
let mut worst=0f32;let mut area=f32::INFINITY;let mut culprit=([0usize;4],0f32);
for frame in 0..=120 {
rig.set_grasp(frame as f32/120.).unwrap();let mut pose=rig.pose(0.);
for side in 0..2 {let start=24+side*15;
for j in 0..3 {if match case {1=>true,2=>j==0,3=>j==1,4=>j==2,5=>j!=0,6=>j!=1,7=>j!=2,_=>false} {pose.set_joint_rotation(start+j,Quat::IDENTITY).unwrap();}}
if case>=5 {for i in 16..24 {pose.set_joint_rotation(i,Quat::IDENTITY).unwrap();}for i in start+3..start+15 {pose.set_joint_rotation(i,Quat::IDENTITY).unwrap();}}
}
let matrices=pose.skin_matrices(&rig.skeleton).unwrap();let left=matrices[6].inverse();let right=matrices[12].inverse();
let palette:Vec<_>=matrices.iter().enumerate().map(|(i,m)|crate::rig_skinning::RigidSkinTransform::from_matrix(if i<16 {Mat4::IDENTITY} else { (if (16..20).contains(&i)||(24..39).contains(&i){left}else{right}) * *m })).collect();
let points:Vec<_>=rest.iter().enumerate().map(|(i,v)|{let p=Vec3::from_array(v.position);rig.finger_weights[i].as_ref().map_or(p,|w|crate::rig_skinning::deform_point(p,w,&palette))}).collect();
for &(ids,rest_angle) in &rig.thumb_web_hinges {
let p=ids.map(|i|points[i]);let rp=ids.map(|i|Vec3::from_array(rest[i].position));let a=(p[1]-p[0]).cross(p[2]-p[0]);let b=(p[0]-p[1]).cross(p[3]-p[1]);let extra=a.normalize().dot(b.normalize()).clamp(-1.,1.).acos()-rest_angle;assert!(extra.is_finite());if extra>worst {worst=extra;culprit=(ids,frame as f32/120.);}
area=area.min(a.length()/(rp[1]-rp[0]).cross(rp[2]-rp[0]).length());
}
}
println!("ISOLATION case {case}: crease {worst}, min_area {area}, worst_ids {:?}, fraction {}",culprit.0,culprit.1);
if case==0 {for i in culprit.0 {println!("CULPRIT vertex {i} rest {:?} influences {:?}",rest[i].position,rig.finger_weights[i]);}}
}
}
}
'''
test += r'''#[cfg(test)] mod base_channel_isolation {
use super::*;
#[test] fn raw_thumb_base_channel_isolation() {
let asset=voxy_render::ObjAsset::parse(include_str!("/Users/themoretheless/Documents/ChatGPT/Voxy/assets/characters/blender-female/body.obj"),voxy_render::ObjLimits::default()).unwrap();
let rest=asset.mesh.vertices();let mut rig=FemaleRig::new(rest).unwrap();rig.bind_hand_surface(rest,asset.mesh.indices()).unwrap();rig.set_grasp_object(None);rig.set_body_motion(false);
for case in 8..12 {
let mut worst=0f32;let mut area=f32::INFINITY;let mut culprit=([0usize;4],0f32);
for frame in 0..=120 {
rig.set_grasp(frame as f32/120.).unwrap();let mut pose=rig.pose(0.);
for side in 0..2 {let start=24+side*15;
for j in 0..3 {if match case {1=>true,2=>j==0,3=>j==1,4=>j==2,5=>j!=0,6=>j!=1,7=>j!=2,_=>false} {pose.set_joint_rotation(start+j,Quat::IDENTITY).unwrap();}}
if case>=5 {for i in 16..24 {pose.set_joint_rotation(i,Quat::IDENTITY).unwrap();}for i in start+3..start+15 {pose.set_joint_rotation(i,Quat::IDENTITY).unwrap();}}
}
for side in 0..2 {
let sign=if side==0{1.}else{-1.};let start=24+side*15;let axes=FingerJointAxes::for_segment(FINGER_CHAINS[0],0,sign);let amount=frame as f32/120.;
let flex=if case==8||case==11{0.20*amount}else{0.};let spread=if case==9||case==11{0.70*amount}else{0.};let twist=if case==10{0.20*amount}else{0.};
pose.set_joint_rotation(start,axes.rotation(flex,spread,twist)).unwrap();for j in 1..3{pose.set_joint_rotation(start+j,Quat::IDENTITY).unwrap();}
}
let matrices=pose.skin_matrices(&rig.skeleton).unwrap();let left=matrices[6].inverse();let right=matrices[12].inverse();
let palette:Vec<_>=matrices.iter().enumerate().map(|(i,m)|crate::rig_skinning::RigidSkinTransform::from_matrix(if i<16 {Mat4::IDENTITY} else { (if (16..20).contains(&i)||(24..39).contains(&i){left}else{right}) * *m })).collect();
let points:Vec<_>=rest.iter().enumerate().map(|(i,v)|{let p=Vec3::from_array(v.position);rig.finger_weights[i].as_ref().map_or(p,|w|crate::rig_skinning::deform_point(p,w,&palette))}).collect();
for &(ids,rest_angle) in &rig.thumb_web_hinges {
let p=ids.map(|i|points[i]);let rp=ids.map(|i|Vec3::from_array(rest[i].position));let a=(p[1]-p[0]).cross(p[2]-p[0]);let b=(p[0]-p[1]).cross(p[3]-p[1]);let extra=a.normalize().dot(b.normalize()).clamp(-1.,1.).acos()-rest_angle;assert!(extra.is_finite());if extra>worst {worst=extra;culprit=(ids,frame as f32/120.);}
area=area.min(a.length()/(rp[1]-rp[0]).cross(rp[2]-rp[0]).length());
}
}
println!("ISOLATION case {case}: crease {worst}, min_area {area}, worst_ids {:?}, fraction {}",culprit.0,culprit.1);
if case==0 {for i in culprit.0 {println!("CULPRIT vertex {i} rest {:?} influences {:?}",rest[i].position,rig.finger_weights[i]);}}
}
}
}
'''
test += r'''#[cfg(test)] mod weight_conditioning {
use super::*;
#[test] fn thumb_weight_conditioning_diagnostic() {
let asset=voxy_render::ObjAsset::parse(include_str!("/Users/themoretheless/Documents/ChatGPT/Voxy/assets/characters/blender-female/body.obj"),voxy_render::ObjLimits::default()).unwrap();
let rest=asset.mesh.vertices();let mut rig=FemaleRig::new(rest).unwrap();rig.bind_hand_surface(rest,asset.mesh.indices()).unwrap();rig.set_grasp_object(None);rig.set_body_motion(false);
let original=rig.finger_weights.clone();let mut neighbors=vec![std::collections::BTreeSet::new();rest.len()];
for triangle in asset.mesh.indices().chunks_exact(3) {for j in 0..3 {let a=triangle[j]as usize;let b=triangle[(j+1)%3]as usize;neighbors[a].insert(b);neighbors[b].insert(a);}}
let mut dense=vec![[0f32;4];rest.len()];let mut region=vec![false;rest.len()];let mut pin=vec![false;rest.len()];
for (i,v) in rest.iter().enumerate() {let p=Vec3::from_array(v.position);let base=if p.x>=0.{24}else{39};let w=original[i].as_deref().unwrap_or(&[]);let thumb:f32=w.iter().filter(|(b,_)|(*b>=base)&&(*b<base+3)).map(|(_,w)|*w).sum();
region[i]=thumb>0.001 && (0.325..0.375).contains(&p.x.abs())&&(-0.06..0.03).contains(&p.y)&&p.z>0.07;
for &(b,w) in w {dense[i][if (base..base+3).contains(&b){1+b-base}else{0}]+=w;}
pin[i]=!region[i]||dense[i][3]>0.5||dense[i][0]>0.98;
}
let baseline_palettes:Vec<_>=(0..=120).map(|frame|{rig.set_grasp(frame as f32/120.).unwrap();let ms=rig.pose(0.).skin_matrices(&rig.skeleton).unwrap();let li=ms[6].inverse();let ri=ms[12].inverse();ms.iter().enumerate().map(|(i,m)|crate::rig_skinning::RigidSkinTransform::from_matrix(if i<16{Mat4::IDENTITY}else{(if(16..20).contains(&i)||(24..39).contains(&i){li}else{ri}) * *m})).collect::<Vec<_>>()}).collect();
for rounds in [0,4,16,64,256] {let mut weights=dense.clone();for _ in 0..rounds {let prev=weights.clone();for i in 0..rest.len(){if pin[i]{continue;}let adjacent:Vec<_>=neighbors[i].iter().copied().filter(|&j|region[j]).collect();if adjacent.is_empty(){continue;}for c in 0..4{let mean:f32=adjacent.iter().map(|&j|prev[j][c]).sum::<f32>()/adjacent.len()as f32;weights[i][c]=0.5*prev[i][c]+0.5*mean;}}}
rig.finger_weights=original.clone();for i in 0..rest.len(){if !region[i]{continue;}let base=if rest[i].position[0]>=0.{24}else{39};let sum:f32=weights[i].iter().sum();rig.finger_weights[i]=Some(weights[i].iter().enumerate().filter_map(|(c,w)|(*w>0.).then_some((if c==0{0}else{base+c-1},*w/sum))).collect::<Vec<_>>().into_boxed_slice());}
let mut worst=0f32;let mut area=f32::INFINITY;let mut deviation=0f32;let mut tip_deviation=0f32;
for palette in &baseline_palettes {let mut points=Vec::with_capacity(rest.len());for (i,v)in rest.iter().enumerate(){let p=Vec3::from_array(v.position);let old=original[i].as_ref().map_or(p,|w|crate::rig_skinning::deform_point(p,w,palette));let new=rig.finger_weights[i].as_ref().map_or(p,|w|crate::rig_skinning::deform_point(p,w,palette));deviation=deviation.max(old.distance(new));if dense[i][3]>0.5{tip_deviation=tip_deviation.max(old.distance(new));}points.push(new);}
for &(ids,rest_angle)in &rig.thumb_web_hinges{let p=ids.map(|i|points[i]);let rp=ids.map(|i|Vec3::from_array(rest[i].position));let a=(p[1]-p[0]).cross(p[2]-p[0]);let b=(p[0]-p[1]).cross(p[3]-p[1]);let extra=a.normalize().dot(b.normalize()).clamp(-1.,1.).acos()-rest_angle;assert!(extra.is_finite());worst=worst.max(extra);area=area.min(a.length()/(rp[1]-rp[0]).cross(rp[2]-rp[0]).length());}
}
println!("WEIGHT CONDITIONING rounds {rounds}: crease {worst}, min_area {area}, max_deviation_m {deviation}, tip_deviation_m {tip_deviation}");
}
}
}
'''
test += r'''#[cfg(test)] mod pivot_isolation {
use super::*;
#[test] fn thumb_bind_preserving_pivot_diagnostic() {
let asset=voxy_render::ObjAsset::parse(include_str!("/Users/themoretheless/Documents/ChatGPT/Voxy/assets/characters/blender-female/body.obj"),voxy_render::ObjLimits::default()).unwrap();let rest=asset.mesh.vertices();let mut rig=FemaleRig::new(rest).unwrap();rig.bind_hand_surface(rest,asset.mesh.indices()).unwrap();rig.set_grasp_object(None);rig.set_body_motion(false);
let original=rig.skeleton.joints().to_vec();let mut offsets=vec![Vec3::ZERO];for axis in 0..3{for shift in [-0.01,-0.005,0.005,0.01]{let mut offset=Vec3::ZERO;offset[axis]=shift;offsets.push(offset);}}
for offset in offsets {let mut joints=original.clone();for side in 0..2 {let root=24+side*15;let mirrored=offset*Vec3::new(if side==0{1.}else{-1.},1.,1.);joints[root].bind_local.translation+=mirrored;joints[root+1].bind_local.translation-=mirrored;joints[root].inverse_bind=Mat4::from_translation(-mirrored)*joints[root].inverse_bind;}
rig.skeleton=Skeleton::new(joints).unwrap();for m in rig.skeleton.bind_pose().skin_matrices(&rig.skeleton).unwrap(){assert!(m.abs_diff_eq(Mat4::IDENTITY,1e-6),"pivot change moved the bind mesh");}
let mut worst=0f32;let mut area=f32::INFINITY;let mut tip_displacement=0f32;
for frame in 0..=120 {rig.set_grasp(frame as f32/120.).unwrap();let ms=rig.pose(0.).skin_matrices(&rig.skeleton).unwrap();let li=ms[6].inverse();let ri=ms[12].inverse();let palette:Vec<_>=ms.iter().enumerate().map(|(i,m)|crate::rig_skinning::RigidSkinTransform::from_matrix(if i<16{Mat4::IDENTITY}else{(if(16..20).contains(&i)||(24..39).contains(&i){li}else{ri}) * *m})).collect();
let points:Vec<_>=rest.iter().enumerate().map(|(i,v)|{let p=Vec3::from_array(v.position);rig.finger_weights[i].as_ref().map_or(p,|w|crate::rig_skinning::deform_point(p,w,&palette))}).collect();
for &(ids,rest_angle)in &rig.thumb_web_hinges{let p=ids.map(|i|points[i]);let rp=ids.map(|i|Vec3::from_array(rest[i].position));let a=(p[1]-p[0]).cross(p[2]-p[0]);let b=(p[0]-p[1]).cross(p[3]-p[1]);let extra=a.normalize().dot(b.normalize()).clamp(-1.,1.).acos()-rest_angle;assert!(extra.is_finite());worst=worst.max(extra);area=area.min(a.length()/(rp[1]-rp[0]).cross(rp[2]-rp[0]).length());}
if frame==120 {for(i,v)in rest.iter().enumerate(){let base=if v.position[0]>=0.{24}else{39};if rig.finger_weights[i].as_ref().is_some_and(|w|w.iter().any(|(b,w)|*b==base+2&&*w>0.5)){tip_displacement=tip_displacement.max(points[i].distance(Vec3::from_array(v.position)));}}}
}
println!("PIVOT offset {:?}: crease {worst}, min_area {area}, tip_motion_m {tip_displacement}",offset.to_array());
}
}
}
'''
test += r'''#[cfg(test)] mod refined_pivot_isolation {
use super::*;
#[test] fn thumb_refined_pivot_diagnostic() {
let asset=voxy_render::ObjAsset::parse(include_str!("/Users/themoretheless/Documents/ChatGPT/Voxy/assets/characters/blender-female/body.obj"),voxy_render::ObjLimits::default()).unwrap();let rest=asset.mesh.vertices();let mut rig=FemaleRig::new(rest).unwrap();rig.bind_hand_surface(rest,asset.mesh.indices()).unwrap();rig.set_grasp_object(None);rig.set_body_motion(false);
let original=rig.skeleton.joints().to_vec();let mut offsets=vec![];for y in [-0.015,-0.02,-0.025]{for z in [0.0,0.01,0.02]{offsets.push(Vec3::new(0.,y,z));}}
for offset in offsets {let mut joints=original.clone();for side in 0..2 {let root=24+side*15;let mirrored=offset*Vec3::new(if side==0{1.}else{-1.},1.,1.);joints[root].bind_local.translation+=mirrored;joints[root+1].bind_local.translation-=mirrored;joints[root].inverse_bind=Mat4::from_translation(-mirrored)*joints[root].inverse_bind;}
rig.skeleton=Skeleton::new(joints).unwrap();for m in rig.skeleton.bind_pose().skin_matrices(&rig.skeleton).unwrap(){assert!(m.abs_diff_eq(Mat4::IDENTITY,1e-6),"pivot change moved the bind mesh");}
let mut worst=0f32;let mut area=f32::INFINITY;let mut tip_displacement=0f32;
for frame in 0..=120 {rig.set_grasp(frame as f32/120.).unwrap();let ms=rig.pose(0.).skin_matrices(&rig.skeleton).unwrap();let li=ms[6].inverse();let ri=ms[12].inverse();let palette:Vec<_>=ms.iter().enumerate().map(|(i,m)|crate::rig_skinning::RigidSkinTransform::from_matrix(if i<16{Mat4::IDENTITY}else{(if(16..20).contains(&i)||(24..39).contains(&i){li}else{ri}) * *m})).collect();
let points:Vec<_>=rest.iter().enumerate().map(|(i,v)|{let p=Vec3::from_array(v.position);rig.finger_weights[i].as_ref().map_or(p,|w|crate::rig_skinning::deform_point(p,w,&palette))}).collect();
for &(ids,rest_angle)in &rig.thumb_web_hinges{let p=ids.map(|i|points[i]);let rp=ids.map(|i|Vec3::from_array(rest[i].position));let a=(p[1]-p[0]).cross(p[2]-p[0]);let b=(p[0]-p[1]).cross(p[3]-p[1]);let extra=a.normalize().dot(b.normalize()).clamp(-1.,1.).acos()-rest_angle;assert!(extra.is_finite());worst=worst.max(extra);area=area.min(a.length()/(rp[1]-rp[0]).cross(rp[2]-rp[0]).length());}
if frame==120 {for(i,v)in rest.iter().enumerate(){let base=if v.position[0]>=0.{24}else{39};if rig.finger_weights[i].as_ref().is_some_and(|w|w.iter().any(|(b,w)|*b==base+2&&*w>0.5)){tip_displacement=tip_displacement.max(points[i].distance(Vec3::from_array(v.position)));}}}
}
println!("PIVOT offset {:?}: crease {worst}, min_area {area}, tip_motion_m {tip_displacement}",offset.to_array());
}
}
}
'''
test = test.replace("/Users/themoretheless/Documents/ChatGPT/Voxy", str(root))
(out / "before.rs").write_text(source + test)
(out / "after.rs").write_text(updated + test)
(out / "lib.rs").write_text(f'#[path="{root}/crates/voxy_app/src/rig_skinning.rs"] mod rig_skinning;\nmod before;\nmod after;\n')
(out / "Cargo.toml").write_text(f'''[package]
name="voxy_hand_hinge_comparison"
version="0.0.0"
edition="2024"
[workspace]
[lib]
path="lib.rs"
[dependencies]
glam="0.33.6"
voxy_animation={{path="{root}/crates/voxy_animation"}}
voxy_render={{path="{root}/crates/voxy_render"}}
physics={{path="{root}/crates/physics"}}
[patch.crates-io]
wgpu={{path="{root}/vendor/wgpu"}}
wgpu-hal={{path="{root}/vendor/wgpu-hal"}}
''')
shutil.copyfile(root / "Cargo.lock", out / "Cargo.lock")
print(f'CARGO_TARGET_DIR="{root}/target" cargo test --manifest-path "{out}/Cargo.toml" sampled_prototype_fold_diagnostic -- --nocapture --test-threads=1')
