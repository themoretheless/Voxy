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
    let scattered=directional_scattering(start,endpoint,metadata);
    for (var channel=0u;channel<3u;channel+=1u) {words[output+channel]=bitcast<u32>(color[channel]*transmission+scattered[channel]);}
    words[output+3u]=bitcast<u32>(color.a);
}


// Stable absorption weight for optically thin subsegments.
fn extinction_weight(tau:f32)->f32 {
    if tau<0.01 {
        return tau*(1.0+tau*(-0.5+tau*(1.0/6.0+tau*(-1.0/24.0+tau/120.0))));
    }
    return 1.0-exp(-tau);
}
fn directional_scattering(start:vec3<f32>,end:vec3<f32>,metadata:u32)->vec3<f32> {
    if words[5]==0u {return vec3<f32>(0.0);}
    let light=metadata+22u;
    let toward_light=vec3<f32>(value(light),value(light+1u),value(light+2u));
    let irradiance=vec3<f32>(value(light+3u),value(light+4u),value(light+5u));
    let albedo=value(light+6u);let g=value(light+7u);let samples=words[light+8u];
    let delta=end-start;let distance=length(delta);
    if distance==0.0 || albedo==0.0 {return vec3<f32>(0.0);}
    let cosine=clamp(dot(toward_light,delta/distance),-1.0,1.0);
    let magnitude=abs(g);
    let denominator=(1.0-magnitude)*(1.0-magnitude)+2.0*magnitude*(1.0-sign(g)*cosine);
    let phase=(1.0-g*g)/(12.566370614359172*denominator*sqrt(denominator));
    let shape=vec3<u32>(words[0],words[1],words[2]);
    let origin=vec3<f32>(value(6u),value(7u),value(8u));
    let spacing=vec3<f32>(value(9u),value(10u),value(11u));
    let extent=vec3<f32>(shape)*spacing;let upper=origin+extent;
    var lo=0.0;var hi=1.0;
    for (var axis=0u;axis<3u;axis+=1u) {
        if delta[axis]==0.0 {
            if start[axis]<origin[axis] || start[axis]>=upper[axis] {return vec3<f32>(0.0);}
        } else {
            let a=(origin[axis]-start[axis])/delta[axis];let b=(upper[axis]-start[axis])/delta[axis];
            lo=max(lo,min(a,b));hi=min(hi,max(a,b));
        }
    }
    if hi<=lo {return vec3<f32>(0.0);}
    var entry=start+delta*lo;var exit=end;
    if hi<1.0 {exit=start+delta*hi;}
    // Clipping identifies exact boundary coordinates. Avoid cancellation in
    // camera-origin interpolation, amplified by dense-medium extinction.
    for (var axis=0u;axis<3u;axis+=1u) {
        if delta[axis]!=0.0 {
            var near_plane=origin[axis];var far_plane=upper[axis];
            if delta[axis]<0.0 {near_plane=upper[axis];far_plane=origin[axis];}
            if lo>0.0 && lo==(near_plane-start[axis])/delta[axis] {entry[axis]=near_plane;}
            if hi<1.0 && hi==(far_plane-start[axis])/delta[axis] {exit[axis]=far_plane;}
        }
    }
    let span=exit-entry;
    let shadow_distance=2.0*length(extent);
    var prefix_tau=0.0;var scattered=0.0;
    for (var i=0u;i<samples;i+=1u) {
        let begin=entry+span*(f32(i)/f32(samples));
        let finish=entry+span*(f32(i+1u)/f32(samples));
        let tau=optical_depth_segment(begin,finish);
        let shadow_a=optical_depth_segment(begin,begin+toward_light*shadow_distance);
        let shadow_b=optical_depth_segment(finish,finish+toward_light*shadow_distance);
        let rate=abs(tau+shadow_b-shadow_a);
        var ratio=1.0;
        if rate>0.0 {ratio=extinction_weight(rate)/rate;}
        let minimum=min(prefix_tau+shadow_a,prefix_tau+tau+shadow_b);
        scattered+=solid_visibility((begin+finish)*0.5)*tau*ratio*exp(-minimum);
        prefix_tau+=tau;
    }
    return irradiance*(albedo*phase)*scattered;
}

fn solid_visibility(world:vec3<f32>)->f32 {return 1.0;}
