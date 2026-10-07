@group(0) @binding(1) var scene_color: texture_2d<f32>;
@group(0) @binding(2) var scene_depth: texture_depth_2d;
@compute @workgroup_size(8,8)
fn cs_main(@builtin(global_invocation_id) id:vec3<u32>) {
    let metadata=12u+words[3]+12u*words[4];
    let width=words[metadata+20u];let height=words[metadata+21u];
    if id.x>=width || id.y>=height {return;}
    let ray=id.y*width+id.x;
    let pixel=id.xy;
    let base=12u+words[3]+8u*ray;
    let output=12u+words[3]+8u*words[4]+4u*ray;
    if any(textureDimensions(scene_color)!=vec2<u32>(width,height)) || any(textureDimensions(scene_depth)!=vec2<u32>(width,height)) {
        words[output]=0x7fc00000u;return;
    }
    var columns:array<vec4<f32>,4>;
    for (var column=0u;column<4u;column+=1u) {
        let at=metadata+4u*column;
        columns[column]=vec4<f32>(value(at),value(at+1u),value(at+2u),value(at+3u));
    }
    let inverse=mat4x4<f32>(columns[0],columns[1],columns[2],columns[3]);
    let xy=vec2<f32>(2.0*(f32(pixel.x)+0.5)/f32(width)-1.0,1.0-2.0*(f32(pixel.y)+0.5)/f32(height));
    let depth=textureLoad(scene_depth,vec2<i32>(pixel),0);
    if !(depth>=0.0 && depth<=1.0) {words[output]=0x7fc00000u;return;}
    let position=inverse*vec4<f32>(xy,depth,1.0);
    let endpoint=position.xyz/position.w;
    var start=vec3<f32>(value(metadata+16u),value(metadata+17u),value(metadata+18u));
    if words[metadata+19u]==1u {let near=inverse*vec4<f32>(xy,0.0,1.0);start=near.xyz/near.w;}
    if !(all(abs(start)<=vec3<f32>(3.402823e38)) && all(abs(endpoint)<=vec3<f32>(3.402823e38))) {
        words[output]=0x7fc00000u;return;
    }
    let delta=endpoint-start;
    if !(dot(delta,delta)<=3.402823e38) {words[output]=0x7fc00000u;return;}
    for (var axis=0u;axis<3u;axis+=1u) {words[base+axis]=bitcast<u32>(start[axis]);words[base+3u+axis]=bitcast<u32>(endpoint[axis]);}
    let tau=optical_depth(ray);let transmission=exp(-tau);
    words[base+6u]=bitcast<u32>(tau);words[base+7u]=bitcast<u32>(transmission);
    let color=textureLoad(scene_color,vec2<i32>(pixel),0);
    for (var channel=0u;channel<3u;channel+=1u) {words[output+channel]=bitcast<u32>(color[channel]*transmission);}
    words[output+3u]=bitcast<u32>(color.a);
}
