struct Camera { vp:mat4x4<f32>, inverse_vp:mat4x4<f32>, shadow:mat4x4<f32>, eye:vec4<f32>, sun:vec4<f32>, flags:vec4<f32> }
struct Material { diffuse:vec4<f32>, specular:vec4<f32>, emissive_normal:vec4<f32>, surface:vec4<f32>, workflow:vec4<f32> }
@group(0) @binding(0) var<uniform> camera:Camera;
@group(1) @binding(0) var<uniform> material:Material;
@group(1) @binding(1) var colour_map:texture_2d<f32>;
@group(1) @binding(2) var normal_map:texture_2d<f32>;
@group(1) @binding(3) var sampler_colour:sampler;
@group(2) @binding(0) var shadow_map:texture_depth_2d;
@group(2) @binding(1) var shadow_sampler:sampler_comparison;
struct Vertex { @location(0) position:vec3<f32>, @location(1) normal:vec3<f32>, @location(2) tangent:vec4<f32>, @location(3) uv:vec2<f32>, @location(4) w0:vec4<f32>, @location(5) w1:vec4<f32>, @location(6) w2:vec4<f32>, @location(7) w3:vec4<f32>, @location(8) n0:vec4<f32>, @location(9) n1:vec4<f32>, @location(10) n2:vec4<f32> }
struct Fragment { @builtin(position) clip:vec4<f32>, @location(0) world:vec3<f32>, @location(1) normal:vec3<f32>, @location(2) tangent:vec4<f32>, @location(3) uv:vec2<f32>, @location(4) shadow:vec4<f32> }
@vertex fn vs_main(v:Vertex)->Fragment {
 let world=mat4x4<f32>(v.w0,v.w1,v.w2,v.w3);let p=world*vec4<f32>(v.position,1.);var f:Fragment;
 f.world=p.xyz;f.clip=camera.vp*p;f.normal=normalize(mat3x3<f32>(v.n0.xyz,v.n1.xyz,v.n2.xyz)*v.normal);
 f.tangent=vec4<f32>(normalize((world*vec4<f32>(v.tangent.xyz,0.)).xyz),v.tangent.w);f.uv=v.uv;f.shadow=camera.shadow*p;return f;
}
@vertex fn vs_shadow(v:Vertex)->@builtin(position) vec4<f32> {
 return camera.shadow*mat4x4<f32>(v.w0,v.w1,v.w2,v.w3)*vec4<f32>(v.position,1.);
}
fn visibility(p:vec4<f32>)->f32 {
 let q=p.xyz/p.w;let uv=q.xy*vec2<f32>(0.5,-0.5)+0.5;
 if any(uv<vec2<f32>(0.))||any(uv>vec2<f32>(1.))||q.z<0.||q.z>1.{return 1.;}
 let texel=1./vec2<f32>(textureDimensions(shadow_map));var sum=0.;
 for(var y=-1;y<=1;y++){for(var x=-1;x<=1;x++){sum+=textureSampleCompareLevel(shadow_map,shadow_sampler,uv+vec2<f32>(f32(x),f32(y))*texel,q.z-0.0001);}}
 return sum/9.;
}
@fragment fn fs_main(f:Fragment,@builtin(front_facing) front:bool)->@location(0) vec4<f32> {
 let base=textureSample(colour_map,sampler_colour,f.uv).rgb*material.diffuse.rgb;
 var n=normalize(f.normal);let t=normalize(f.tangent.xyz-n*dot(n,f.tangent.xyz));let b=cross(n,t)*f.tangent.w;
 let mapped=textureSample(normal_map,sampler_colour,f.uv).xyz*2.-1.;
 n=normalize(mat3x3<f32>(t,b,n)*vec3<f32>(mapped.xy*material.emissive_normal.w,mapped.z));if !front {n=-n;}
 let v=normalize(camera.eye.xyz-f.world);let l=camera.sun.xyz;let h=normalize(v+l);
 let nl=max(dot(n,l),0.);let nv=max(dot(n,v),0.001);let nh=max(dot(n,h),0.);let vh=max(dot(v,h),0.);
 let rough=max(material.surface.y,0.08);let a2=pow(rough,4.);let d=a2/(3.14159265*pow(nh*nh*(a2-1.)+1.,2.));
 let k=pow(rough+1.,2.)/8.;let g=nl/(nl*(1.-k)+k)*nv/(nv*(1.-k)+k);let fresnel=0.04+0.96*pow(1.-vh,5.);
 let spec=d*g*fresnel/max(4.*nl*nv,0.001);let shadow=visibility(f.shadow);
 let sky=mix(vec3<f32>(0.22,0.17,0.12),vec3<f32>(0.4,0.52,0.7),n.y*0.5+0.5);
 let thin=material.workflow.w*max(dot(-n,l),0.)*0.6;
 var colour=base*(sky+(nl+thin)*3.*shadow)+vec3<f32>(spec*nl*shadow*3.);
 colour=select(colour/(colour+vec3<f32>(1.)),colour,camera.flags.x>0.5);
 return vec4<f32>(colour,1.);
}
