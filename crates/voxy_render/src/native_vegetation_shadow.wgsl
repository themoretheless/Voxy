struct Camera { vp:mat4x4<f32>, inverse_vp:mat4x4<f32>, shadow:mat4x4<f32>, eye:vec4<f32>, sun:vec4<f32>, flags:vec4<f32> }
@group(0) @binding(0) var<uniform> camera:Camera;
@group(1) @binding(0) var scene_depth:texture_depth_2d;
@group(1) @binding(1) var shadow_map:texture_depth_2d;
@group(1) @binding(2) var shadow_sampler:sampler_comparison;
@vertex fn vs_main(@builtin(vertex_index) i:u32)->@builtin(position) vec4<f32> {
 let p=array<vec2<f32>,3>(vec2<f32>(-1.,-1.),vec2<f32>(3.,-1.),vec2<f32>(-1.,3.));return vec4<f32>(p[i],0.,1.);
}
@fragment fn fs_main(@builtin(position) pixel:vec4<f32>)->@location(0) vec4<f32> {
 let xy=vec2<i32>(pixel.xy);let depth=textureLoad(scene_depth,xy,0);if depth==0.{return vec4<f32>(1.);}
 let size=vec2<f32>(textureDimensions(scene_depth));let ndc=pixel.xy/size*vec2<f32>(2.,-2.)+vec2<f32>(-1.,1.);
 let world=camera.inverse_vp*vec4<f32>(ndc,depth,1.);let p=camera.shadow*vec4<f32>(world.xyz/world.w,1.);let q=p.xyz/p.w;
 let uv=q.xy*vec2<f32>(0.5,-0.5)+0.5;if any(uv<vec2<f32>(0.))||any(uv>vec2<f32>(1.))||q.z<0.||q.z>1.{return vec4<f32>(1.);}
 let texel=1./vec2<f32>(textureDimensions(shadow_map));var sum=0.;
 for(var y=-1;y<=1;y++){for(var x=-1;x<=1;x++){sum+=textureSampleCompareLevel(shadow_map,shadow_sampler,uv+vec2<f32>(f32(x),f32(y))*texel,q.z-0.0002);}}
 return vec4<f32>(vec3<f32>(0.55+0.45*sum/9.),1.);
}
