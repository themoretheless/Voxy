// Resolve actual spatial samples in linear light, before the display transform.
@group(0) @binding(0) var source:texture_2d<f32>;
// scale, display mode, display headroom, unused
@group(0) @binding(1) var<uniform> settings:vec4<f32>;
@vertex fn vs_main(@builtin(vertex_index) i:u32)->@builtin(position) vec4<f32> {
 return vec4<f32>(f32((i<<1u)&2u)*2.-1.,1.-f32(i&2u)*2.,0.,1.);
}
@fragment fn fs_main(@builtin(position) p:vec4<f32>)->@location(0) vec4<f32> {
 let scale=i32(settings.x);
 let base=vec2<i32>(p.xy)*scale;
 var colour=vec3<f32>(0.);
 for(var y=0;y<scale;y++) {for(var x=0;x<scale;x++) {
  colour+=textureLoad(source,base+vec2<i32>(x,y),0).rgb;
 }}
 colour/=f32(scale*scale);
 return vec4<f32>(display_transform(max(colour,vec3<f32>(0.)),settings.y,settings.z),1.);
}
