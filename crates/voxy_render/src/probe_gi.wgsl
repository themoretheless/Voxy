// Irradiance probe baking: project one rendered cube face into order-1
// spherical harmonics and accumulate it into the probe's slot.
struct Bake {
    // forward.xyz, probe index
    forward:vec4<f32>,
    // right.xyz, face index (0 clears the slot)
    right:vec4<f32>,
    // up.xyz, 1 / exposure
    up:vec4<f32>,
    // square origin x, square size, sample step, unused
    square:vec4<f32>,
}
@group(0) @binding(0) var face:texture_2d<f32>;
@group(0) @binding(1) var<uniform> bake:Bake;
@group(0) @binding(2) var<storage,read_write> probes:array<vec4<f32>>;
var<workgroup> partial:array<array<vec3<f32>,4>,128>;

@compute @workgroup_size(128) fn cs_project(@builtin(local_invocation_index) thread:u32) {
    let size=bake.square.y;let step=bake.square.z;
    let count=u32(size/step);
    var sum=array<vec3<f32>,4>(vec3<f32>(0.),vec3<f32>(0.),vec3<f32>(0.),vec3<f32>(0.));
    // Solid angle of one sample on the unit cube face at (u, v).
    let du=2.*step/size;
    for(var i=thread;i<count*count;i+=128u) {
        let px=vec2<f32>(f32(i%count),f32(i/count))*step+step*0.5;
        let uv=px/size*2.-1.;
        let radiance=max(textureLoad(face,vec2<i32>(i32(bake.square.x+px.x),i32(px.y)),0).rgb,vec3<f32>(0.))*bake.up.w;
        let d=bake.forward.xyz+uv.x*bake.right.xyz-uv.y*bake.up.xyz;
        let len2=dot(d,d);
        let weight=du*du/(len2*sqrt(len2));
        let n=d*inverseSqrt(len2);
        sum[0]+=radiance*(0.282095*weight);
        sum[1]+=radiance*(0.488603*n.y*weight);
        sum[2]+=radiance*(0.488603*n.z*weight);
        sum[3]+=radiance*(0.488603*n.x*weight);
    }
    partial[thread]=sum;
    workgroupBarrier();
    for(var stride=64u;stride>0u;stride/=2u) {
        if thread<stride {
            for(var k=0;k<4;k++){partial[thread][k]+=partial[thread+stride][k];}
        }
        workgroupBarrier();
    }
    if thread==0u {
        let base=u32(bake.forward.w)*4u;
        for(var k=0u;k<4u;k++){
            let previous=select(probes[base+k].xyz,vec3<f32>(0.),bake.right.w<0.5);
            probes[base+k]=vec4<f32>(previous+partial[0][k],1.);
        }
    }
}
