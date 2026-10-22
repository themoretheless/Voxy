struct Camera { vp:mat4x4<f32>, inverse_vp:mat4x4<f32>, shadow:mat4x4<f32>, eye:vec4<f32>, sun:vec4<f32>, flags:vec4<f32> }
struct Material { diffuse:vec4<f32>, specular:vec4<f32>, emissive_normal:vec4<f32>, surface:vec4<f32>, workflow:vec4<f32>, canopy:vec4<f32>, extent_density:vec4<f32> }
@group(0) @binding(0) var<uniform> camera:Camera;
@group(1) @binding(0) var<uniform> material:Material;
@group(1) @binding(1) var colour_map:texture_2d<f32>;
@group(1) @binding(2) var normal_map:texture_2d<f32>;
@group(1) @binding(3) var sampler_colour:sampler;
@group(1) @binding(4) var roughness_map:texture_2d<f32>;
@group(2) @binding(0) var shadow_map:texture_depth_2d;
@group(2) @binding(1) var shadow_sampler:sampler_comparison;
struct Vertex { @location(0) position:vec3<f32>, @location(1) normal:vec3<f32>, @location(2) tangent:vec4<f32>, @location(3) uv:vec2<f32>, @location(4) w0:vec4<f32>, @location(5) w1:vec4<f32>, @location(6) w2:vec4<f32>, @location(7) w3:vec4<f32>, @location(8) n0:vec4<f32>, @location(9) n1:vec4<f32>, @location(10) n2:vec4<f32> }
struct Fragment { @builtin(position) clip:vec4<f32>, @location(0) world:vec3<f32>, @location(1) normal:vec3<f32>, @location(2) tangent:vec4<f32>, @location(3) uv:vec2<f32>, @location(4) shadow:vec4<f32>, @location(5) local:vec3<f32> }
// Hierarchical wind (after Sousa, GPU Gems 3 ch.16): whole-plant bend grows with
// height squared, limbs sway with distance from the stem, leaf blades flutter.
// Each instance gets its own phase from its world origin so a stand never moves in lockstep.
fn wind(local:vec3<f32>,origin:vec3<f32>)->vec3<f32> {
 let time=camera.flags.y;let wind_dir=normalize(vec3<f32>(1.,0.,0.35));
 let top=max(material.canopy.y+material.extent_density.y,0.5);
 let phase=dot(origin,vec3<f32>(0.37,0.,0.61));
 let gust=0.6+0.4*sin(time*0.31+phase*0.2)*sin(time*0.83+phase*0.07);
 let h=max(local.y,0.)/top;
 let bend=h*h*(0.035*gust+0.012*sin(time*1.1+phase));
 let reach=length(local.xz);let limb=reach*h*0.025*sin(time*2.3+phase+local.y*1.7+reach*2.1);
 var offset=wind_dir*bend*top+vec3<f32>(0.,limb,0.);
 if material.workflow.w>0. {
  let flutter=sin(time*9.+dot(local,vec3<f32>(37.,23.,41.)))*0.012*gust*min(reach+h,1.);
  offset+=vec3<f32>(flutter*0.5,flutter,flutter*0.5);
 }
 // Keep stems rooted: no motion at the ground.
 return offset*smoothstep(0.,0.4,local.y);
}
@vertex fn vs_main(v:Vertex)->Fragment {
 let world=mat4x4<f32>(v.w0,v.w1,v.w2,v.w3);let p=world*vec4<f32>(v.position+wind(v.position,v.w3.xyz),1.);var f:Fragment;
 f.local=v.position;f.world=p.xyz;f.clip=camera.vp*p;f.normal=normalize(mat3x3<f32>(v.n0.xyz,v.n1.xyz,v.n2.xyz)*v.normal);
 f.tangent=vec4<f32>(normalize((world*vec4<f32>(v.tangent.xyz,0.)).xyz),v.tangent.w);f.uv=v.uv;f.shadow=camera.shadow*p;return f;
}
struct ShadowFragment { @builtin(position) clip:vec4<f32>, @location(0) uv:vec2<f32> }
@vertex fn vs_shadow(v:Vertex)->ShadowFragment {
 var f:ShadowFragment;f.clip=camera.shadow*mat4x4<f32>(v.w0,v.w1,v.w2,v.w3)*vec4<f32>(v.position+wind(v.position,v.w3.xyz),1.);f.uv=v.uv;return f;
}
// Leaf cards cast their blade silhouette, not their quad.
@fragment fn fs_shadow(f:ShadowFragment) {
 if material.surface.w>=0. && textureSampleLevel(colour_map,sampler_colour,f.uv,0.).a<material.surface.w {discard;}
}
fn visibility(p:vec4<f32>)->f32 {
 let q=p.xyz/p.w;let uv=q.xy*vec2<f32>(0.5,-0.5)+0.5;
 if any(uv<vec2<f32>(0.))||any(uv>vec2<f32>(1.))||q.z<0.||q.z>1.{return 1.;}
 let texel=1./vec2<f32>(textureDimensions(shadow_map));var sum=0.;
 for(var y=-2;y<=2;y++){for(var x=-2;x<=2;x++){sum+=textureSampleCompareLevel(shadow_map,shadow_sampler,uv+vec2<f32>(f32(x),f32(y))*texel,q.z-0.0001);}}
 return sum/25.;
}
// Approximate sky accessibility from the generated crown's leaf-area density.
// This supplements real sun shadow maps; it is not a full indirect-light solver.
fn crown_accessibility(local:vec3<f32>)->f32 {
 let extent=material.extent_density.xyz;let density=material.extent_density.w;
 let p=(local-material.canopy.xyz)/extent;let d=vec3<f32>(0.,1.,0.)/extent;
 let a=dot(d,d);let b=dot(p,d);let c=dot(p,p)-1.;let disc=b*b-a*c;
 if disc<=0.||density<=0.{return 1.;}
 let root=sqrt(disc);let entry=max((-b-root)/a,0.);let exit=max((-b+root)/a,0.);
 return max(exp(-0.5*density*max(exit-entry,0.)),0.22);
}
@fragment fn fs_main(f:Fragment,@builtin(front_facing) front:bool)->@location(0) vec4<f32> {
 let texel=textureSample(colour_map,sampler_colour,f.uv);
 if material.surface.w>=0. && texel.a<material.surface.w {discard;}
 let base=texel.rgb*material.diffuse.rgb;
 let geometric=normalize(f.normal);
 var n=geometric;let t=normalize(f.tangent.xyz-n*dot(n,f.tangent.xyz));let b=cross(n,t)*f.tangent.w;
 let mapped=textureSample(normal_map,sampler_colour,f.uv).xyz*2.-1.;
 n=normalize(mat3x3<f32>(t,b,n)*vec3<f32>(mapped.xy*material.emissive_normal.w,mapped.z));if !front {n=-n;}
 let mr=textureSample(roughness_map,sampler_colour,f.uv);
 let v=normalize(camera.eye.xyz-f.world);let l=camera.sun.xyz;let h=normalize(v+l);
 let leaf=material.workflow.w;
 let nl_raw=dot(n,l);
 // Foliage: wrapped diffuse stands in for multiple scattering inside the blade.
 let wrap=leaf*0.35;let nl=max((nl_raw+wrap)/(1.+wrap),0.);
 let nv=max(dot(n,v),0.001);let nh=max(dot(n,h),0.);let vh=max(dot(v,h),0.);
 let rough=clamp(material.surface.y*mr.g,0.08,1.);let r2=rough*rough;let a2=r2*r2;
 let q=nh*nh*(a2-1.)+1.;
 let d=a2/(3.14159265*q*q);
 let spec_nl=max(nl_raw,0.);
 let smith_l=2.*spec_nl/max(spec_nl+sqrt(a2+(1.-a2)*spec_nl*spec_nl),0.00001);
 let smith_v=2.*nv/max(nv+sqrt(a2+(1.-a2)*nv*nv),0.00001);
 let ft=1.-vh;let ft2=ft*ft;
 let fresnel=0.04+0.96*(ft2*ft2*ft);
 let spec=d*smith_l*smith_v*fresnel/max(4.*spec_nl*nv,0.001);
 let shadow=visibility(f.shadow);let accessibility=crown_accessibility(f.local);
 let sky=mix(vec3<f32>(0.24,0.21,0.15),vec3<f32>(0.55,0.72,1.0),n.y*0.5+0.5);
 // Light through a blade is filtered by chlorophyll: yellower and more saturated.
 let filtered=pow(max(base,vec3<f32>(0.0001)),vec3<f32>(0.8))*vec3<f32>(1.05,1.25,0.45);
 let back=max(-nl_raw,0.);let fs_dot=max(dot(v,-l),0.);let fs2=fs_dot*fs_dot;let forward_scatter=fs2*fs2*fs2;
 let transmit=leaf*(back*0.55+forward_scatter*1.4)/3.14159265;
 let diffuse=(1.-fresnel)*nl/3.14159265;
 let sunlight=vec3<f32>(2.95,2.76,2.48);
 // Inner crown sees a sky already filtered by the outer leaves.
 let ambient=base*sky*accessibility*mix(1.,0.85+0.3*accessibility,leaf);
 var colour=ambient+(base*diffuse+filtered*transmit+vec3<f32>(spec*spec_nl))*sunlight*shadow;
 colour=select(colour/(colour+vec3<f32>(1.)),colour,camera.flags.x>0.5);
 return vec4<f32>(colour,1.);
}
