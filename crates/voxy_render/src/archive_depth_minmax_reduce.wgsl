// Every child range is represented, including padded edge tiles.
@group(0) @binding(0) var previous:texture_2d<f32>;
@group(0) @binding(1) var next_depth:texture_storage_2d<rg32float,write>;
@compute @workgroup_size(8,8)
fn reduce(@builtin(global_invocation_id) id:vec3<u32>) {
 let size=textureDimensions(next_depth);if any(id.xy>=size){return;}
 let previous_size=textureDimensions(previous);var range=vec2<f32>(1.,0.);
 for(var y=0u;y<2u;y++){for(var x=0u;x<2u;x++){
  let child=textureLoad(previous,vec2<i32>(min(id.xy*2u+vec2<u32>(x,y),previous_size-1u)),0).xy;
  range=vec2<f32>(min(range.x,child.x),max(range.y,child.y));
 }}
 textureStore(next_depth,vec2<i32>(id.xy),vec4<f32>(range,0.,0.));
}
