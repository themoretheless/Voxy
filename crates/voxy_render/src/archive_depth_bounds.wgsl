fn stored_depth_bounds(depth:f32)->vec2<f32> {
 let finite=(bitcast<u32>(depth)&0x7f800000u)!=0x7f800000u;
 let valid=finite&&depth>=0.&&depth<=1.;
 return vec2<f32>(select(0.,depth,valid),select(1.,select(depth,0.,depth==1.),valid));
}
// Level-three forward-Z bound. Every original depth texel is represented;
// invalid values contribute zero, disabling rejection for that entire tile.
@group(0) @binding(0) var full_depth:texture_depth_2d;
@group(0) @binding(1) var minimum_depth:texture_storage_2d<rg32float,write>;
var<workgroup> depths:array<vec2<f32>,64>;
@compute @workgroup_size(8,8)
fn bounds(@builtin(workgroup_id) tile:vec3<u32>,@builtin(local_invocation_id) local:vec3<u32>,@builtin(local_invocation_index) index:u32) {
 let size=textureDimensions(full_depth);
 let pixel=min(tile.xy*8u+local.xy,size-1u);
 let depth=textureLoad(full_depth,vec2<i32>(pixel),0);
 depths[index]=stored_depth_bounds(depth);
 workgroupBarrier();
 for(var stride=32u;stride>0u;stride/=2u) {
  if index<stride {depths[index]=vec2<f32>(min(depths[index].x,depths[index+stride].x),max(depths[index].y,depths[index+stride].y));}
  workgroupBarrier();
 }
 if index==0u {textureStore(minimum_depth,vec2<i32>(tile.xy),vec4<f32>(depths[0],0.,0.));}
}

// Full-resolution leaves preserve the same padded-edge and invalid-depth rules.
@compute @workgroup_size(8,8)
fn pixel_bounds(@builtin(global_invocation_id) id:vec3<u32>) {
 if any(id.xy>=textureDimensions(minimum_depth)) {return;}
 let pixel=min(id.xy,textureDimensions(full_depth)-1u);
 textureStore(minimum_depth,vec2<i32>(id.xy),vec4<f32>(stored_depth_bounds(textureLoad(full_depth,vec2<i32>(pixel),0)),0.,0.));
}
