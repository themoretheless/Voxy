//! Repair invalid authored tangents with upstream MikkTSpace, preserving triangles.
use super::{ModelError,SceneArchivePrimitive,SceneArchiveTangents};
use std::collections::{BTreeMap,BTreeSet};
struct Geometry<'a>{positions:&'a[[f32;3]],normals:&'a[[f32;3]],uv:&'a[[f32;2]],indices:&'a[u32],output:Vec<Option<[f32;4]>>}
impl bevy_mikktspace::Geometry for Geometry<'_>{
 fn num_faces(&self)->usize{self.indices.len()/3}
 fn num_vertices_of_face(&self,_:usize)->usize{3}
 fn position(&self,face:usize,vert:usize)->[f32;3]{self.positions[self.indices[face*3+vert] as usize]}
 fn normal(&self,face:usize,vert:usize)->[f32;3]{self.normals[self.indices[face*3+vert] as usize]}
 fn tex_coord(&self,face:usize,vert:usize)->[f32;2]{self.uv[self.indices[face*3+vert] as usize]}
 fn set_tangent(&mut self,tangent:Option<bevy_mikktspace::TangentSpace>,face:usize,vert:usize){self.output[face*3+vert]=tangent.map(|v|v.tangent_encoded());}
}
impl SceneArchivePrimitive{
 /// Transactional repair of invalid vertices only. Different corner tangents get
 /// distinct vertices; never average across mirrored UV seams or change triangles.
 pub fn repair_tangents(&mut self,uv_set:u32)->Result<usize,ModelError>{
  let Some(SceneArchiveTangents::NeedsRepair{source,invalid_vertices})=&self.tangents else{return Ok(0)};
  if self.source["attributes"].as_object().is_some_and(|a|a.keys().any(|k|!matches!(k.as_str(),"POSITION"|"NORMAL"|"TANGENT")&&!k.starts_with("TEXCOORD_")))||self.source.get("targets").is_some(){return Err(ModelError("tangent repair requires explicit copying of all vertex attributes".into()));}
  let normals=self.normals.as_ref().ok_or_else(||ModelError("tangent repair requires authored normals".into()))?;
  let uv=self.texcoords.iter().find(|(set,_)|*set==uv_set).map(|(_,v)|v.as_slice()).ok_or_else(||ModelError("tangent repair missing material UV set".into()))?;
  let mut geometry=Geometry{positions:&self.positions,normals,uv,indices:&self.indices,output:vec![None;self.indices.len()]};
  bevy_mikktspace::generate_tangents(&mut geometry).map_err(|e|ModelError(e.to_string()))?;
  let bad=invalid_vertices.iter().copied().collect::<BTreeSet<_>>();let count=bad.len();
  let mut repaired=self.clone();let mut tangents=source.clone();let mut chosen=BTreeMap::<usize,[u32;4]>::new();let mut variants=BTreeMap::<(usize,[u32;4]),u32>::new();
  for(corner,&index)in self.indices.iter().enumerate(){
   let id=index as usize;if !bad.contains(&id){continue;}
   let tangent=if let Some(value)=geometry.output[corner]{value}else{
    // An isolated degenerate UV/geometry face has no defined tangent space.
    // Use an explicit geometric basis only after proving exact degeneracy;
    // source positions, normals, UVs and triangle membership stay unchanged.
    let face=corner/3;let ids=&self.indices[face*3..face*3+3];
    let p=ids.iter().map(|&i|glam::DVec3::from_array(self.positions[i as usize].map(f64::from))).collect::<Vec<_>>();
    let e1=p[1]-p[0];let e2=p[2]-p[0];
    let t=ids.iter().map(|&i|uv[i as usize].map(f64::from)).collect::<Vec<_>>();
    let determinant=(t[1][0]-t[0][0])*(t[2][1]-t[0][1])-(t[1][1]-t[0][1])*(t[2][0]-t[0][0]);
    let area=e1.cross(e2).length_squared();
    let mut n=glam::DVec3::from_array(normals[id].map(f64::from));
    if n.length_squared()==0.{if area!=0.{return Err(ModelError("undefined normal on visible tangent repair face".into()));}n=glam::DVec3::Y;}else{n=n.normalize();}
    let (mut tangent,bitangent)=if determinant!=0. && area!=0.{
     // Some authored near-degenerate UV frames are unresolved by the f32
     // reference. Recover the SAME UV differential in f64, without an epsilon.
     ((e1*(t[2][1]-t[0][1])-e2*(t[1][1]-t[0][1]))/determinant,
      (e2*(t[1][0]-t[0][0])-e1*(t[2][0]-t[0][0]))/determinant)
    }else{(if e1.length_squared()>=e2.length_squared(){e1}else{e2},glam::DVec3::ZERO)};
    tangent-=n*n.dot(tangent);
    if tangent.length_squared()==0.{
     if determinant!=0. && area!=0.{return Err(ModelError("UV tangent is parallel to authored normal".into()));}
     tangent=n.any_orthonormal_vector();
    }else{tangent=tangent.normalize();}
    let sign=if n.cross(tangent).dot(bitangent)<0.{-1.}else{1.};
    let tangent=tangent.as_vec3().to_array();[tangent[0],tangent[1],tangent[2],sign]
   };
   if tangent.iter().any(|x|!x.is_finite())||!(tangent[3]==1.||tangent[3]==-1.){return Err(ModelError("MikkTSpace produced invalid tangent".into()));}
   let bits=tangent.map(f32::to_bits);let target=if let Some(&target)=variants.get(&(id,bits)){target}else{
    let target=if chosen.contains_key(&id){
     let target=u32::try_from(repaired.positions.len()).map_err(|_|ModelError("tangent split vertex capacity".into()))?;
     repaired.positions.push(self.positions[id]);repaired.normals.as_mut().unwrap().push(normals[id]);
     for((_,output),(_,input))in repaired.texcoords.iter_mut().zip(&self.texcoords){output.push(input[id]);}
     tangents.push(tangent);target
    }else{chosen.insert(id,bits);tangents[id]=tangent;index};variants.insert((id,bits),target);target
   };repaired.indices[corner]=target;
  }
  if chosen.len()!=count{return Err(ModelError("unreferenced invalid tangent requires explicit unused-vertex processing".into()));}
  if tangents.iter().flatten().any(|v|!v.is_finite()){return Err(ModelError("unresolved invalid tangent".into()));}
  // Original source attributes remain in the archive's immutable source buffers.
  repaired.tangents=Some(SceneArchiveTangents::Finite(tangents));*self=repaired;Ok(count)
 }
}
