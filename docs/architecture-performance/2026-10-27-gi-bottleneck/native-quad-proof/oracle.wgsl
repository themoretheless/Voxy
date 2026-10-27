
@vertex fn vertex(@builtin(vertex_index) i:u32)->@builtin(position) vec4<f32>{
 let p=array<vec2<f32>,3>(vec2<f32>(-1.,-1.),vec2<f32>(3.,-1.),vec2<f32>(-1.,3.));return vec4<f32>(p[i],0.5,1.);
}
@fragment fn fragment(@builtin(position) p:vec4<f32>)->@builtin(frag_depth) f32{
 let index=u32(p.x)+u32(p.y)*1920u;
 if index%6u==0u{return 0.;}if index%6u==1u{return 1.;}if index%6u==2u{return bitcast<f32>(0x80000000u);}
 return bitcast<f32>(0x3f000000u|((index*16777619u)&0x007fffffu));
}
@group(0) @binding(0) var original:texture_depth_2d;
@group(0) @binding(1) var comparison:sampler;
@group(0) @binding(2) var<storage,read_write> result:array<vec2<u32>>;
@compute @workgroup_size(8,8) fn compare(@builtin(global_invocation_id) id:vec3<u32>){
 if any(id.xy>=textureDimensions(original)){return;}
 let p=vec2<i32>(id.xy);let index=id.x+id.y*1920u;
 let z=textureLoad(original,p,0);let bits=bitcast<u32>(z);
 let actual=archive_gi_quad_depth(archive_gi_quad_gather(p),p);
 result[index]=vec2<u32>(select(0u,1u,bits!=bitcast<u32>(actual)),bits);
}



// Gather the unfiltered original Depth32Float words; no depth conversion.

fn archive_gi_quad_gather(pixel:vec2<i32>)->vec4<f32> {
 let base=(pixel/2)*2;
 return textureGather(original,comparison,(vec2<f32>(base)+1.)/vec2<f32>(textureDimensions(original)));
}
fn archive_gi_quad_depth(quad:vec4<f32>,pixel:vec2<i32>)->f32 {
 let x=u32(pixel.x)&1u;let y=u32(pixel.y)&1u;
 return quad[select(3u-x,x,y!=0u)];
}
