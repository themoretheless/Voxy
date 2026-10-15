struct Camera { vp:mat4x4<f32>, shadow:mat4x4<f32>, inverse_vp:mat4x4<f32>, eye:vec4<f32>, sun:vec4<f32>, sun_colour:vec4<f32>, viewport:vec4<f32>, weather:vec4<f32>, atmosphere:vec4<f32>, wind:vec4<f32> }
struct Light { position_kind:vec4<f32>, colour_intensity:vec4<f32> }
struct Material { diffuse:vec4<f32>, specular_gloss:vec4<f32>, emissive_normal:vec4<f32>, surface:vec4<f32>, workflow:vec4<f32> }
@group(0) @binding(0) var<uniform> camera:Camera;
struct Lights { count:vec4<u32>, values:array<Light,128> }
@group(0) @binding(1) var<uniform> lights:Lights;
@group(1) @binding(0) var<uniform> material:Material;
@group(1) @binding(1) var diffuse_map:texture_2d<f32>;
@group(1) @binding(2) var specular_map:texture_2d<f32>;
@group(1) @binding(3) var normal_map:texture_2d<f32>;
@group(1) @binding(4) var emissive_map:texture_2d<f32>;
@group(1) @binding(5) var material_sampler:sampler;
@group(2) @binding(0) var background:texture_2d<f32>;
@group(2) @binding(1) var shadow_map:texture_depth_2d;
@group(2) @binding(2) var shadow_sampler:sampler_comparison;
@group(2) @binding(3) var scene_depth:texture_depth_2d;
@group(3) @binding(0) var environment_specular:texture_cube<f32>;
@group(3) @binding(1) var environment_dfg:texture_2d<f32>;
@group(3) @binding(2) var environment_sampler:sampler;
@group(3) @binding(3) var environment_diffuse:texture_cube<f32>;
@group(3) @binding(4) var<uniform> environment_intensity:vec4<f32>;
struct Vertex { @location(0) position:vec3<f32>, @location(1) normal:vec3<f32>, @location(2) tangent:vec4<f32>, @location(3) uv:vec2<f32>, @location(4) w0:vec4<f32>, @location(5) w1:vec4<f32>, @location(6) w2:vec4<f32>, @location(7) w3:vec4<f32>, @location(8) n0:vec4<f32>, @location(9) n1:vec4<f32>, @location(10) n2:vec4<f32> }
struct Fragment { @builtin(position) clip:vec4<f32>, @location(0) world:vec3<f32>, @location(1) normal:vec3<f32>, @location(2) tangent:vec4<f32>, @location(3) uv:vec2<f32>, @location(4) shadow:vec4<f32> }
fn vertex(v:Vertex, shadow:bool)->Fragment { let world=mat4x4<f32>(v.w0,v.w1,v.w2,v.w3);let p=world*vec4<f32>(v.position,1.);var f:Fragment;f.world=p.xyz;f.clip=select(camera.vp*p,camera.shadow*p,shadow);f.normal=normalize(mat3x3<f32>(v.n0.xyz,v.n1.xyz,v.n2.xyz)*v.normal);let t=(world*vec4<f32>(v.tangent.xyz,0.)).xyz;let determinant=dot(cross(v.w0.xyz,v.w1.xyz),v.w2.xyz);f.tangent=vec4<f32>(normalize(t),v.tangent.w*sign(determinant));f.uv=v.uv;f.shadow=camera.shadow*p;return f; }
@vertex fn vs_main(v:Vertex)->Fragment{return vertex(v,false);}
@vertex fn vs_shadow(v:Vertex)->Fragment{return vertex(v,true);}
@fragment fn fs_shadow(f:Fragment){if material.surface.w>=0. && textureSample(diffuse_map,material_sampler,surface_uv(f)).a*material.diffuse.a<material.surface.w{discard;}}
fn srgb(x:vec3<f32>)->vec3<f32>{return select(x/12.92,pow((x+0.055)/1.055,vec3<f32>(2.4)),x>vec3<f32>(0.04045));}
fn square(x:f32)->f32{return x*x;}
fn fourth(x:f32)->f32{let s=x*x;return s*s;}
fn fifth(x:f32)->f32{let s=x*x;return s*s*x;}
struct Brdf {n:vec3<f32>,v:vec3<f32>,diffuse:vec3<f32>,f0:vec3<f32>,a2:f32,nv:f32,smith_v:f32}
fn brdf(s:Brdf,l:vec3<f32>)->vec3<f32>{
 let nl=max(dot(s.n,l),0.);if nl==0.{return vec3<f32>(0.);}
 let h=normalize(l+s.v);let nh=max(dot(s.n,h),0.);let vh=max(dot(s.v,h),0.);
 let d=s.a2/(3.14159265*square(nh*nh*(s.a2-1.)+1.));
 let gv=nl*s.smith_v;let gl=s.nv*sqrt(nl*nl*(1.-s.a2)+s.a2);let visibility=0.5/max(gv+gl,0.000001);
 let fresnel=s.f0+(1.-s.f0)*fifth(1.-vh);return (s.diffuse+d*visibility*fresnel)*nl;
}

// Original scene materials: evaluated directly on the GPU, independent of image assets.
fn original_hash(p:vec3<f32>)->f32 {var q=fract(p*0.1031);q+=dot(q,q.yzx+33.33);return fract((q.x+q.y)*q.z);}
fn original_noise(p:vec3<f32>)->f32 {
 let a=floor(p);let f=fract(p);let u=f*f*(3.-2.*f);
 return mix(mix(mix(original_hash(a),original_hash(a+vec3<f32>(1.,0.,0.)),u.x),mix(original_hash(a+vec3<f32>(0.,1.,0.)),original_hash(a+vec3<f32>(1.,1.,0.)),u.x),u.y),mix(mix(original_hash(a+vec3<f32>(0.,0.,1.)),original_hash(a+vec3<f32>(1.,0.,1.)),u.x),mix(original_hash(a+vec3<f32>(0.,1.,1.)),original_hash(a+vec3<f32>(1.,1.,1.)),u.x),u.y),u.z);
}
fn original_surface(p:vec3<f32>,n:vec3<f32>,kind:f32)->vec3<f32>{
 let grain=original_noise(p*27.);let broad=original_noise(p*1.8);
 if kind>7.5 {let grooves=original_noise(p*vec3<f32>(85.,2.5,85.));return vec3<f32>(0.62+0.58*grooves)*(0.9+0.2*broad);}
 if kind>6.5 {let uv=select(p.xy,p.zy,abs(n.x)>0.5);let row=floor(uv.y/0.075);let cell=fract(vec2<f32>(uv.x/0.225+0.5*(row-2.*floor(row*0.5)),uv.y/0.075));let edge=min(min(cell.x,1.-cell.x)*0.225,min(cell.y,1.-cell.y)*0.075);return mix(vec3<f32>(0.40,0.38,0.34),vec3<f32>(0.71,0.40,0.26)*(0.85+0.25*broad),smoothstep(0.002,0.006,edge));}
 if kind>5.5 {return mix(vec3<f32>(0.60,0.85,0.38),vec3<f32>(1.10,1.05,0.65),broad);}

 if kind<1.5{return vec3<f32>(0.83+0.18*broad)*(0.94+0.10*grain);}
 if kind<2.5 {let damp=1.-0.20*(1.-smoothstep(0.1,1.6,p.y))*(0.3+broad);return vec3<f32>((0.86+0.15*broad+0.08*grain)*damp);}
 if kind<3.5 {let uv=select(p.xy,p.zy,abs(n.x)>0.5);let row=floor(uv.y*12.);let cell=fract(vec2<f32>(uv.x*4.+0.5*(row-2.*floor(row*0.5)),uv.y*12.));let seam=select(1.,0.5,cell.x<0.045||cell.y<0.07);return vec3<f32>(seam*(0.8+0.3*grain));}
 if kind<4.5 {let line=original_noise(vec3<f32>(p.x*40.,p.y*3.,p.z*3.));return vec3<f32>(0.65+0.55*line);}
 return vec3<f32>(0.90+0.15*grain);
}
// Band-limited physical height fields in metres. Screen derivatives suppress
// unresolved grain instead of allowing moving-camera specular aliasing.
fn original_height(p:vec3<f32>,kind:f32)->f32 {
 let footprint=max(length(dpdx(p)),length(dpdy(p)));
 let fine=1.-smoothstep(0.002,0.012,footprint);
 let medium=1.-smoothstep(0.02,0.08,footprint);
 if kind<1.5 {return 0.003*medium*original_noise(p*16.)+0.0006*fine*original_noise(p*145.);}
 if kind<2.5 {return 0.0012*medium*original_noise(p*23.)+0.00035*fine*original_noise(p*180.);}
 if kind<3.5 {return 0.0015*medium*original_noise(p*32.);}
 if kind<4.5 {return 0.0008*fine*original_noise(p*vec3<f32>(160.,5.,5.));}
 return 0.00008*fine*original_noise(p*230.);
}
fn original_bump(p:vec3<f32>,n:vec3<f32>,height:f32)->vec3<f32>{
 let px=dpdx(p);let py=dpdy(p);let rx=cross(py,n);let ry=cross(n,px);
 let determinant=dot(px,rx);
 let gradient=sign(determinant)*(dpdx(height)*rx+dpdy(height)*ry);
 return normalize(abs(determinant)*n-gradient+1e-20*n);
}
fn contact_occlusion(pixel:vec2<f32>,p:vec3<f32>,n:vec3<f32>)->f32{
 let radius=0.65;let projected=clamp(radius*camera.viewport.y/max(distance(p,camera.eye.xyz),0.5),2.,96.);
 var sum=0.;var weight=0.;
 for(var i=0u;i<12u;i++){
  let a=f32(i)*2.399963;let r=sqrt((f32(i)+0.5)/12.);
  let uv=pixel+vec2<f32>(cos(a),sin(a))*projected*r;
  if any(uv<vec2<f32>(0.))||any(uv>=camera.viewport.xy){continue;}
  let depth=textureLoad(scene_depth,vec2<i32>(uv),0);
  if depth>=1.{continue;}
  let h=camera.inverse_vp*vec4<f32>((floor(uv)+0.5)/camera.viewport.xy*vec2<f32>(2.,-2.)+vec2<f32>(-1.,1.),depth,1.);
  let delta=h.xyz/h.w-p;let len=length(delta);
  if len>0.008&&len<radius {sum+=max(dot(n,delta/len)-0.12,0.)*(1.-len/radius);}
  weight+=1.;
 }
 return clamp(1.-3.*sum/max(weight,1.),0.25,1.);
}
fn surface_uv(f:Fragment)->vec2<f32>{
 if material.workflow.w<0. {return f.uv*vec2<f32>(2.,4.);}
 if material.workflow.w>0. {
  let n=abs(f.normal);
  if n.x>=n.y&&n.x>=n.z {return vec2<f32>(-f.world.z*sign(f.normal.x),f.world.y)*material.workflow.w;}
  if n.y>=n.z {return vec2<f32>(f.world.x,-f.world.z*sign(f.normal.y))*material.workflow.w;}
  return vec2<f32>(f.world.x*sign(f.normal.z),f.world.y)*material.workflow.w;
 }
 return f.uv;
}
fn shade(f:Fragment,front:bool,tex:vec4<f32>)->vec4<f32>{
 var base=tex.rgb*material.diffuse.rgb;if material.workflow.z>0.5{base*=original_surface(f.world,f.normal,material.workflow.z);}let opacity=tex.a*material.diffuse.a;
 let uv=surface_uv(f);let spec=textureSample(specular_map,material_sampler,uv);var f0=spec.rgb*material.specular_gloss.rgb;var roughness=1.-spec.a*material.specular_gloss.a;var diffuse=base*(1.-max(max(f0.r,f0.g),f0.b));
 if material.workflow.x>0.5{let metallic=material.surface.x*spec.b;roughness=material.surface.y*spec.g;f0=mix(vec3<f32>(0.04),base,metallic);diffuse=base*(1.-metallic);}
 let face=select(-1.,1.,front);var normal=normalize(f.normal)*face;var tangent=normalize(f.tangent.xyz-normal*dot(normal,f.tangent.xyz))*face;var bitangent=cross(normal,tangent)*f.tangent.w*face;
 if material.workflow.w>0. {let n=abs(f.normal);if n.x>=n.y&&n.x>=n.z {tangent=vec3<f32>(0.,0.,-sign(f.normal.x));bitangent=vec3<f32>(0.,1.,0.);}else if n.y>=n.z {tangent=vec3<f32>(1.,0.,0.);bitangent=vec3<f32>(0.,0.,-sign(f.normal.y));}else{tangent=vec3<f32>(sign(f.normal.z),0.,0.);bitangent=vec3<f32>(0.,1.,0.);}}var mapped=vec3<f32>(0.,0.,1.);if material.workflow.y>0.5{mapped=textureSample(normal_map,material_sampler,uv).xyz*2.-1.;}mapped.x*=material.emissive_normal.w;mapped.y*=material.emissive_normal.w;normal=normalize(mat3x3<f32>(tangent,bitangent,normal)*mapped);if material.workflow.z>0.5{let height=original_height(f.world,material.workflow.z);normal=original_bump(f.world,normal,height);roughness=clamp(roughness+(original_noise(f.world*7.)-0.5)*0.16,0.04,1.);}let view=normalize(camera.eye.xyz-f.world);
 let shadow_pos=f.shadow.xyz/f.shadow.w;let shadow_uv=shadow_pos.xy*vec2<f32>(0.5,-0.5)+0.5;var visibility=1.;if all(shadow_uv>=vec2<f32>(0.))&&all(shadow_uv<=vec2<f32>(1.))&&shadow_pos.z>=0.&&shadow_pos.z<=1.{let texel=1./vec2<f32>(textureDimensions(shadow_map));visibility=0.;for(var sy=-1;sy<=1;sy++){for(var sx=-1;sx<=1;sx++){visibility+=textureSampleCompareLevel(shadow_map,shadow_sampler,shadow_uv+vec2<f32>(f32(sx),f32(sy))*texel,shadow_pos.z-0.00004)/9.;}}}
 let transmission=material.surface.z;let alpha=max(roughness*roughness,0.0001);let a2=alpha*alpha;let nv=max(dot(normal,view),0.0001);let shading=Brdf(normal,view,diffuse*(1.-transmission)/3.14159265,f0,a2,nv,sqrt(nv*nv*(1.-a2)+a2));var colour=brdf(shading,camera.sun.xyz)*camera.sun_colour.rgb*camera.sun_colour.a*visibility;
 for(var i=0u;i<lights.count.x;i++){let light=lights.values[i];if light.colour_intensity.a==0.{continue;}let offset=light.position_kind.xyz-f.world;let distance2=max(dot(offset,offset),0.0001);let distance=sqrt(distance2);var attenuation=1./distance2;if light.position_kind.w>0.{attenuation*=square(clamp(1.-fourth(distance/light.position_kind.w),0.,1.));}colour+=brdf(shading,offset/distance)*light.colour_intensity.rgb*light.colour_intensity.a*attenuation;}
 // Directional sky, contact visibility and view-dependent environment Fresnel.
 let ao=contact_occlusion(f.clip.xy,f.world,normal)*select(1.,spec.r,material.workflow.w!=0.);
 if camera.eye.w>0.5 {
  let irradiance=textureSampleLevel(environment_diffuse,environment_sampler,normal,0.).rgb*environment_intensity.x;
  let reflected=reflect(-view,normal);
  let radiance=textureSampleLevel(environment_specular,environment_sampler,reflected,roughness*f32(textureNumLevels(environment_specular)-1u)).rgb*environment_intensity.x;
  let dfg=textureSampleLevel(environment_dfg,environment_sampler,vec2<f32>(nv,roughness),0.).rg;
  colour+=(diffuse*(1.-transmission)*irradiance+radiance*(f0*dfg.x+dfg.y))*ao;
 } else {
  let sky=mix(vec3<f32>(110.,95.,75.),vec3<f32>(170.,230.,330.),clamp(normal.y*0.5+0.5,0.,1.));
  colour+=diffuse*(1.-transmission)*sky*camera.viewport.w*ao/3.14159265;
  let reflected=reflect(-view,normal);let fresnel=f0+(1.-f0)*fifth(1.-nv);
  colour+=daylight(reflected,camera.sun.xyz)*fresnel*(1.-0.7*roughness)*ao;
 }
 if material.workflow.z>5.5&&material.workflow.z<6.5 {
  let veins=1.-0.18*exp(-abs(f.uv.x-0.5)*130.);
  colour*=veins;
  colour+=base*camera.sun_colour.rgb*camera.sun_colour.a*max(dot(-normal,camera.sun.xyz),0.)*0.12*visibility;
 }
 if any(material.emissive_normal.rgb!=vec3<f32>(0.)){colour+=textureSample(emissive_map,material_sampler,uv).rgb*material.emissive_normal.rgb;}
 if transmission>0.{let pixel=vec2<i32>(f.clip.xy);let behind=textureLoad(background,pixel,0).rgb;let nv=max(dot(normal,view),0.);let fresnel=f0+(1.-f0)*fifth(1.-nv);colour+=behind/camera.viewport.z*base*transmission*(1.-fresnel);}
 return vec4<f32>(colour*camera.viewport.z,select(1.,opacity,material.surface.w == -2.));
}

@fragment fn fs_opaque(f:Fragment,@builtin(front_facing) front:bool)->@location(0) vec4<f32>{return shade(f,front,textureSample(diffuse_map,material_sampler,surface_uv(f)));}
@fragment fn fs_main(f:Fragment,@builtin(front_facing) front:bool)->@location(0) vec4<f32>{let tex=textureSample(diffuse_map,material_sampler,surface_uv(f));if material.surface.w>=0.&&tex.a*material.diffuse.a<material.surface.w{discard;}return shade(f,front,tex);}
