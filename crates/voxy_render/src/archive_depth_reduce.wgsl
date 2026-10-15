// Exact minimum over each 2x2 footprint of a finite [0,1] depth image.
// Allocate the next level with ceil(width/2), ceil(height/2), as separate
// R32Float textures: ordinary floor-sized mip chains lose odd border texels.
// This is an acceleration bound, never a replacement for level-zero hit tests.
@group(0) @binding(0) var source_depth:texture_2d<f32>;
@group(0) @binding(1) var reduced_depth:texture_storage_2d<r32float,write>;
@compute @workgroup_size(8,8)
fn reduce_depth(@builtin(global_invocation_id) invocation:vec3<u32>) {
 let target=textureDimensions(reduced_depth);
 if any(invocation.xy>=target) {return;}
 let last=vec2<i32>(textureDimensions(source_depth))-1;
 let p=vec2<i32>(invocation.xy*2u);
 let a=textureLoad(source_depth,min(p,last),0).x;
 let b=textureLoad(source_depth,min(p+vec2<i32>(1,0),last),0).x;
 let c=textureLoad(source_depth,min(p+vec2<i32>(0,1),last),0).x;
 let d=textureLoad(source_depth,min(p+vec2<i32>(1,1),last),0).x;
 textureStore(reduced_depth,vec2<i32>(invocation.xy),vec4<f32>(min(min(a,b),min(c,d)),0.,0.,0.));
}
