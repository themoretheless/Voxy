//! Current native scene/archive and DDS source qualification, no renderer claim.
#[derive(Debug)]pub struct ModelError(pub String);
impl std::fmt::Display for ModelError{fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{self.0.fmt(f)}}
pub use scene_archive::{SceneArchivePrimitive,SceneArchiveTangents};
#[path="../../crates/voxy_model/src/scene_tangents.rs"]mod scene_tangents;
#[path="../../crates/voxy_model/src/scene_archive.rs"]mod scene_archive;
#[path="../../crates/voxy_render/src/compressed_image.rs"]mod compressed_image;
fn main(){
 let root=std::path::PathBuf::from(std::env::args().nth(1).expect("asset cache directory"));
 let source=std::fs::read(root.join("bistro.gltf")).unwrap();let doc:serde_json::Value=serde_json::from_slice(&source).unwrap();
 let buffers=doc["buffers"].as_array().unwrap().iter().map(|b|std::sync::Arc::<[u8]>::from(std::fs::read(root.join(b["uri"].as_str().unwrap())).unwrap())).collect::<Vec<_>>();
 let mut scene=scene_archive::SceneArchive::decode(&source,buffers,Default::default()).unwrap();
 let mut repaired_tangents=0;
 for primitive in scene.meshes.iter_mut().flatten(){
  let before=primitive.indices.iter().map(|&i|primitive.positions[i as usize].map(f32::to_bits)).collect::<Vec<_>>();
  repaired_tangents+=primitive.repair_tangents(0).unwrap();
  let after=primitive.indices.iter().map(|&i|primitive.positions[i as usize].map(f32::to_bits)).collect::<Vec<_>>();assert_eq!(before,after,"tangent repair changed triangle geometry/order");
 }
 let mut textures=0;let mut mip_levels=0;let mut texture_bytes=0;let mut pending=0;
 for texture in scene.document["textures"].as_array().unwrap(){
  let id=texture["extensions"]["MSFT_texture_dds"]["source"].as_u64().unwrap() as usize;let uri=scene.document["images"][id]["uri"].as_str().unwrap();let path=root.join(uri);
  if !path.exists(){pending+=1;continue;}
  let image=compressed_image::Bc7ImageAsset::decode_dds(std::fs::read(path).unwrap().into(),512*1024*1024,16384).unwrap();textures+=1;mip_levels+=image.mip_ranges.len();texture_bytes+=image.byte_len();
 }
 println!("{}",serde_json::json!({"scope":"native geometry/hierarchy import and available unchanged BC7 DDS mip validation; not rendered or walking","nodes":scene.nodes.len(),"repaired_tangents":repaired_tangents,"tangent_vertices_requiring_repair":scene.meshes.iter().flatten().map(|p|match &p.tangents{Some(scene_archive::SceneArchiveTangents::NeedsRepair{invalid_vertices,..})=>invalid_vertices.len(),_=>0}).sum::<usize>(),"meshes":scene.meshes.len(),"vertices":scene.meshes.iter().flatten().map(|p|p.positions.len()).sum::<usize>(),"indices":scene.meshes.iter().flatten().map(|p|p.indices.len()).sum::<usize>(),"material_sources":scene.document["materials"].as_array().unwrap().len(),"textures_checked":textures,"texture_mip_levels_checked":mip_levels,"texture_payload_bytes_checked":texture_bytes,"textures_pending":pending}));
}
