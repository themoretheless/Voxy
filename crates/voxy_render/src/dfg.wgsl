struct Params { size: u32, samples: u32, unused0: u32, unused1: u32 }
@group(0) @binding(0) var<uniform> params: Params;
@vertex fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let p = array<vec2<f32>,3>(vec2(-1.,-1.),vec2(3.,-1.),vec2(-1.,3.));
    return vec4(p[i],0.,1.);
}
@fragment fn fs_main(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
    let nv = p.x / f32(params.size);
    let roughness = p.y / f32(params.size);
    let a2 = roughness * roughness * roughness * roughness;
    let v = vec3(sqrt(max(0.,1.-nv*nv)),0.,nv);
    var ab = vec2(0.);
    for(var i=0u; i<params.samples; i++) {
        let xi = vec2(f32(i)/f32(params.samples), f32(reverseBits(i))*2.3283064365386963e-10);
        let phi = 6.283185307179586 * xi.x;
        let nh = sqrt((1.-xi.y)/(1.+(a2-1.)*xi.y));
        let sh = sqrt(max(0.,1.-nh*nh));
        let h = vec3(cos(phi)*sh,sin(phi)*sh,nh);
        let vh = max(dot(v,h),0.);
        let l = 2.*vh*h-v;
        let nl = max(l.z,0.);
        if(nl>0. && vh>0.) {
            let denominator = nl*sqrt(nv*nv*(1.-a2)+a2) + nv*sqrt(nl*nl*(1.-a2)+a2);
            let weight = 2.*nl*vh/(nh*denominator);
            let fc = pow(clamp(1.-vh,0.,1.),5.);
            ab += vec2(1.-fc,fc)*weight;
        }
    }
    return vec4(ab/f32(params.samples),0.,1.);
}
