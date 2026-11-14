// Original GI may explicitly use the same float contract in compute and raster.
override ARCHIVE_ORIGINAL_GI_RASTER_MATH:bool=false;
override WGPU_METAL_PRECISE_MATH:i32=0;
// Qualification-only injection of the existing unsupported-projection fallback.
override ARCHIVE_RAY_FORCE_FALLBACK:bool=false;
override ARCHIVE_COMPUTE_SSR:bool=false;
override ARCHIVE_TRACE_SSR:bool=false;
override ARCHIVE_COMPUTE_GI:bool=false;
override ARCHIVE_COMPUTE_GI_EXACT_SAMPLE:bool=false;
// Scheduling-only candidate; the query and colour expressions are unchanged.
override ARCHIVE_EARLY_GI:bool=false;
// Opt-in until full-scene correctness and paired performance are qualified.
override ARCHIVE_PAIRED_DDA:bool=false;
override ARCHIVE_RAY_BOUNDS:bool=true;
// Candidate only: paired timings have not established a repeatable gain.
override ARCHIVE_REFINE_CACHE:bool=false;
override ARCHIVE_RAY_DEPTH_REUSE:bool=false;
override ARCHIVE_LIGHT_Z_MASKS:bool=false;
struct Camera { vp:mat4x4<f32>, shadow:mat4x4<f32>, inverse_vp:mat4x4<f32>, eye:vec4<f32>, sun:vec4<f32>, sun_colour:vec4<f32>, viewport:vec4<f32>, weather:vec4<f32>, atmosphere:vec4<f32>, wind:vec4<f32>, previous_vp:mat4x4<f32>, previous_inverse_vp:mat4x4<f32>, surface:vec4<f32> }
struct Light { position_kind:vec4<f32>, colour_intensity:vec4<f32> }
struct Material { diffuse:vec4<f32>, specular_gloss:vec4<f32>, emissive_normal:vec4<f32>, surface:vec4<f32>, workflow:vec4<f32> }
@group(0) @binding(0) var<uniform> camera:Camera;
struct Lights { count:vec4<u32>, values:array<Light,128>, z_masks:array<vec4<u32>,256> }
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
@group(2) @binding(4) var opaque_history_depth:texture_depth_2d;
@group(2) @binding(7) var ray_depth_bounds:texture_2d<f32>;
@group(2) @binding(10) var ray_pixel_depth_bounds:texture_2d<f32>;
@group(2) @binding(8) var computed_gi:texture_2d<f32>;
@group(2) @binding(9) var computed_ssr:texture_2d<f32>;
@group(1) @binding(6) var gi_world:texture_2d<f32>;
@group(1) @binding(7) var gi_normal:texture_2d<f32>;
@group(1) @binding(8) var gi_output:texture_storage_2d<rgba32float,write>;
@group(3) @binding(0) var environment_specular:texture_cube<f32>;
@group(3) @binding(1) var environment_dfg:texture_2d<f32>;
@group(3) @binding(2) var environment_sampler:sampler;
@group(3) @binding(3) var environment_diffuse:texture_cube<f32>;
@group(3) @binding(4) var<uniform> environment_intensity:vec4<f32>;
struct Vertex { @location(0) position:vec3<f32>, @location(1) normal:vec3<f32>, @location(2) tangent:vec4<f32>, @location(3) uv:vec2<f32>, @location(4) w0:vec4<f32>, @location(5) w1:vec4<f32>, @location(6) w2:vec4<f32>, @location(7) w3:vec4<f32>, @location(8) n0:vec4<f32>, @location(9) n1:vec4<f32>, @location(10) n2:vec4<f32> }
// Match clip positions across depth, material, shadow and guide pipelines.
struct Fragment { @invariant @builtin(position) clip:vec4<f32>, @location(0) world:vec3<f32>, @location(1) normal:vec3<f32>, @location(2) tangent:vec4<f32>, @location(3) uv:vec2<f32>, @location(4) shadow:vec4<f32> }
fn vertex(v:Vertex, shadow:bool)->Fragment { let world=mat4x4<f32>(v.w0,v.w1,v.w2,v.w3);let p=world*vec4<f32>(v.position,1.);var f:Fragment;f.world=p.xyz;f.clip=select(camera.vp*p,camera.shadow*p,shadow);f.normal=normalize(mat3x3<f32>(v.n0.xyz,v.n1.xyz,v.n2.xyz)*v.normal);let t=(world*vec4<f32>(v.tangent.xyz,0.)).xyz;let determinant=dot(cross(v.w0.xyz,v.w1.xyz),v.w2.xyz);f.tangent=vec4<f32>(normalize(t),v.tangent.w*sign(determinant));f.uv=v.uv;f.shadow=camera.shadow*p;return f; }
@vertex fn vs_main(v:Vertex)->Fragment{return vertex(v,false);}
@vertex fn vs_shadow(v:Vertex)->Fragment{return vertex(v,true);}
@fragment fn fs_shadow(f:Fragment){if material.surface.w>=0. && textureSample(diffuse_map,material_sampler,surface_uv(f)).a*material.diffuse.a<material.surface.w{discard;}}
fn srgb(x:vec3<f32>)->vec3<f32>{return select(x/12.92,pow((x+0.055)/1.055,vec3<f32>(2.4)),x>vec3<f32>(0.04045));}
fn square(x:f32)->f32{return x*x;}
fn fourth(x:f32)->f32{let s=x*x;return s*s;}
fn fifth(x:f32)->f32{let s=x*x;return s*s*x;}
// Original scene materials: evaluated directly on the GPU, independent of image assets.
fn original_hash(p:vec3<f32>)->f32 {var q=fract(p*0.1031);q+=dot(q,q.yzx+33.33);return fract((q.x+q.y)*q.z);}
fn original_noise(p:vec3<f32>)->f32 {
 let a=floor(p);let f=fract(p);let u=f*f*(3.-2.*f);
 return mix(mix(mix(original_hash(a),original_hash(a+vec3<f32>(1.,0.,0.)),u.x),mix(original_hash(a+vec3<f32>(0.,1.,0.)),original_hash(a+vec3<f32>(1.,1.,0.)),u.x),u.y),mix(mix(original_hash(a+vec3<f32>(0.,0.,1.)),original_hash(a+vec3<f32>(1.,0.,1.)),u.x),mix(original_hash(a+vec3<f32>(0.,1.,1.)),original_hash(a+vec3<f32>(1.,1.,1.)),u.x),u.y),u.z);
}
fn original_surface(p:vec3<f32>,n:vec3<f32>,kind:f32)->vec3<f32>{
 let broad=original_noise(p*1.8);
 if kind>8.5 {
  if kind>12.5 {
   if kind>14.5 {
    let petal=original_noise(p*19.);let fine_fibre=original_noise(p*95.);
    let velvet=0.88+0.18*petal+0.06*(fine_fibre-0.5);
    return vec3<f32>(velvet*(0.94+0.10*broad));
   }
   if kind>13.5 {
    let uv=select(select(p.xz,p.zy,abs(n.x)>0.5),p.xy,abs(n.z)>0.5)/0.15;
    let cell=floor(uv);let f=fract(uv);
    let edge=min(min(f.x,1.-f.x),min(f.y,1.-f.y));
    let fp=max(fwidthFine(uv.x),fwidthFine(uv.y));
    let h=original_hash(vec3<f32>(cell,14.))-0.5;
    let glaze=0.96+0.06*original_noise(p*11.)+0.04*h;
    let grout=mix(0.76,1.,smoothstep(0.015,0.045+fp,edge));
    return vec3<f32>(glaze*grout*(0.98+0.03*original_noise(p*27.)));
   }
   let macro_v=original_noise(p*0.45);let cloud=original_noise(p*1.6);
   let trowel=original_noise(vec3<f32>(p.x*3.5+cloud*0.4,p.y*7.5,p.z*3.5-cloud*0.4));
   let sand=original_noise(p*92.);
   let damp=1.-0.10*(1.-smoothstep(0.08,0.65,p.y))*(0.4+0.6*broad)*smoothstep(0.3,0.8,1.-abs(n.y));
   let warm=mix(vec3<f32>(0.95,0.94,0.92),vec3<f32>(1.04,1.03,1.00),macro_v*0.6+trowel*0.4);
   return warm*(0.96+0.05*original_noise(p*27.)+0.03*sand)*damp;
  }
  if kind>10.5 {
   if kind>11.5 {
    let cell=floor(p/0.7+vec3<f32>(0.01));
    let f_slab=fract(p/0.7+vec3<f32>(0.01));
    let d_chamfer=min(min(f_slab.x,1.-f_slab.x),min(f_slab.z,1.-f_slab.z))*0.7;
    let chamfer_wear=smoothstep(0.018,0.002,d_chamfer)*smoothstep(0.5,0.9,abs(n.y));
    let h=original_hash(cell*13.7);let h2=original_hash(cell*31.3+7.);
    let q=p+vec3<f32>(h*4.3,h2*2.7,h*6.1);
    let vein=original_noise(q*vec3<f32>(2.4,9.,1.6)+original_noise(q*5.5)*0.85);
    let band=smoothstep(0.14,0.86,vein);
    let fine_vein=smoothstep(0.50,0.84,original_noise(q*vec3<f32>(8.,24.,6.)));
    let cross_vein=smoothstep(0.62,0.88,original_noise(q*vec3<f32>(18.,5.,14.)));
    let pore=smoothstep(0.62,0.86,original_noise(q*54.));
    let warm=mix(vec3<f32>(0.73,0.67,0.57),vec3<f32>(1.06,1.02,0.96),band);
    let veined=mix(warm,vec3<f32>(0.64,0.57,0.46),fine_vein*0.38+cross_vein*0.18);
    let tone=0.82+0.30*h+0.11*(broad-0.5);
    let body=veined*tone*(1.-0.18*pore)*(0.93+0.10*original_noise(p*27.));
    return mix(body,vec3<f32>(1.08,1.05,0.99),chamfer_wear*0.16);
   }
   let uv=select(select(p.xz,p.zy,abs(n.x)>0.5),p.xy,abs(n.z)>0.5);
   let footprint=max(length(dpdxFine(uv)),length(dpdyFine(uv)));
   let detail=1.-smoothstep(0.002,0.015,footprint);
   let warp=sin(uv.x*420.)*0.5+0.5;let weft=sin(uv.y*420.)*0.5+0.5;
   let weave=mix(1.,0.88+0.22*(warp*0.5+weft*0.5),detail);
   let fold=0.90+0.16*original_noise(p*4.5)+0.06*broad;
   return vec3<f32>(weave*fold*(0.96+0.07*original_noise(p*27.)));
  }
  if kind>9.5 {
   let turf_broad=original_noise(p*0.42);let medium_patch=original_noise(p*2.1);
   let tuft=original_noise(vec3<f32>(p.x*16.,p.y*4.,p.z*16.));
   let blades=original_noise(vec3<f32>(p.x*58.,p.y*8.,p.z*58.));
   let fine_blades=original_noise(vec3<f32>(p.x*145.,p.y*12.,p.z*145.));
   let clover=smoothstep(0.55,0.85,original_noise(p*4.5));
   let lush=mix(vec3<f32>(0.58,0.86,0.36),vec3<f32>(1.22,1.12,0.60),turf_broad*0.55+medium_patch*0.45);
   let tipped=mix(lush,vec3<f32>(1.36,1.22,0.58),smoothstep(0.52,0.88,tuft)*0.55);
   let blade_ao=0.36+0.64*(blades*0.65+fine_blades*0.35);
   let shaded=mix(vec3<f32>(0.34,0.50,0.24),tipped,blade_ao);
   return mix(shaded,shaded*vec3<f32>(0.80,1.08,0.78),clover*0.42);
  }
  let uv=select(select(p.xz,p.zy,abs(n.x)>0.5),p.xy,abs(n.z)>0.5)/0.025;let cell=floor(uv);let f=fract(uv);let edge=min(min(f.x,1.-f.x),min(f.y,1.-f.y));let footprint=max(fwidthFine(uv.x),fwidthFine(uv.y));let h=original_hash(vec3<f32>(cell,kind));let h2=original_hash(vec3<f32>(cell.yx,kind+3.));let shimmer=vec3<f32>(0.86+0.20*h,0.90+0.16*h2,0.92+0.16*(1.-h*0.5));let tile=mix(vec3<f32>(0.52,0.56,0.60),shimmer,smoothstep(0.05,0.13+footprint,edge));return mix(tile,vec3<f32>(0.90,0.92,0.94),smoothstep(0.25,0.6,footprint));
 }
 if kind>5.5 {
  if kind>7.5 {
   let grooves=original_noise(p*vec3<f32>(85.,2.5,85.));
   let moss=smoothstep(0.54,0.82,original_noise(p*3.8))*(1.-smoothstep(0.15,0.95,p.y))*smoothstep(-0.2,0.5,-n.z);
   let bark_col=vec3<f32>(0.62+0.58*grooves)*(0.9+0.2*broad);
   return mix(bark_col,vec3<f32>(0.42,0.68,0.34),moss*0.55);
  }
  if kind>6.5 {
   let uv=select(p.xy,p.zy,abs(n.x)>0.5);let row=floor(uv.y/0.075);let col=floor(uv.x/0.225+0.5*(row-2.*floor(row*0.5)));
   let cell=fract(vec2<f32>(uv.x/0.225+0.5*(row-2.*floor(row*0.5)),uv.y/0.075));
   let edge=min(min(cell.x,1.-cell.x)*0.225,min(cell.y,1.-cell.y)*0.075);
   let bh=original_hash(vec3<f32>(col,row,7.))-0.5;
   let salt=smoothstep(0.68,0.90,original_noise(p*2.6))*(1.-smoothstep(0.3,1.8,p.y));
   let brick_tone=mix(vec3<f32>(0.71,0.40,0.26)*(0.85+0.25*broad+0.16*bh),vec3<f32>(0.92,0.90,0.86),salt*0.28);
   return mix(vec3<f32>(0.40,0.38,0.34),brick_tone,smoothstep(0.002,0.006,edge));
  }
  let cluster=original_noise(p*4.8);let sub_cluster=original_noise(p*11.5);let leaf_cell=original_noise(p*28.);
  let pocket=smoothstep(0.18,0.78,cluster*0.6+sub_cluster*0.4);
  let hue=mix(vec3<f32>(0.46,0.78,0.30),vec3<f32>(1.22,1.14,0.56),broad*0.45+pocket*0.55);
  return hue*(0.42+0.58*pocket)*(0.72+0.42*leaf_cell)*(0.90+0.16*original_noise(p*27.));
 }
 if kind<2.5 {
  let grain=original_noise(p*27.);
  if kind<1.5 {
   let macro_var=original_noise(p*0.55);let speck=original_noise(p*64.);
   let damp=1.-0.15*(1.-smoothstep(0.08,0.85,p.y))*(0.35+0.65*broad)*smoothstep(0.3,0.8,1.-abs(n.y));
   let gutter=exp(-square((abs(p.x)-4.32)/0.26))*smoothstep(-26.,-24.,p.z)*(1.-smoothstep(81.,83.,p.z))*smoothstep(0.8,0.98,n.y);
   let litter=smoothstep(0.62,0.86,original_noise(p*11.5))*gutter;
   let side_slab=smoothstep(4.65,4.85,abs(p.x))*(1.-smoothstep(6.95,7.25,abs(p.x)))*smoothstep(-26.,-24.,p.z)*(1.-smoothstep(81.,83.,p.z))*smoothstep(0.7,0.95,n.y);
   var joint=1.;
   // All neighbouring lanes need the footprint before the pixel-dependent mask.
   let fp=max(length(dpdxFine(p.xz)),length(dpdyFine(p.xz)));
   if side_slab>0. {
    let gx=abs(fract(p.x/0.72)-0.5)*0.72;
    let gz=abs(fract(p.z/0.90)-0.5)*0.90;
    let d_joint=min(gx,gz);
    let cell_h=original_hash(vec3<f32>(floor(p.x/0.72),floor(p.z/0.90),1.))-0.5;
    joint=mix(1.,(0.74+0.26*smoothstep(0.004,0.011+fp,d_joint))*(1.+0.08*cell_h),side_slab);
   }
   let stone_col=vec3<f32>((0.82+0.16*broad+0.08*macro_var)*(0.92+0.10*grain+0.05*speck)*damp*joint);
   return mix(stone_col,stone_col*vec3<f32>(0.78,0.64,0.44),litter*0.45);
  }
  let damp=1.-0.20*(1.-smoothstep(0.1,1.6,p.y))*(0.3+broad);
  let drip=1.-0.07*smoothstep(0.58,0.88,original_noise(vec3<f32>(p.x*6.5,p.y*0.65,p.z*6.5)))*smoothstep(0.3,0.8,1.-abs(n.y));
  return vec3<f32>((0.86+0.15*broad+0.08*grain)*damp*drip);
 }
 if kind<3.5 {
  let uv=select(p.xy,p.zy,abs(n.x)>0.5);let row=floor(uv.y*12.);let col=floor(uv.x*4.+0.5*(row-2.*floor(row*0.5)));
  let cell=fract(vec2<f32>(uv.x*4.+0.5*(row-2.*floor(row*0.5)),uv.y*12.));
  let tile_h=original_hash(vec3<f32>(col,row,3.));
  let seam=smoothstep(0.02,0.055,cell.x)*smoothstep(0.03,0.085,cell.y);
  let overlap=0.88+0.16*cell.y;
  let slate=mix(vec3<f32>(0.78,0.80,0.83),vec3<f32>(1.06,1.02,0.96),tile_h)*(0.84+0.24*original_noise(p*27.));
  return slate*(0.42+0.58*seam)*overlap;
 }
 if kind<4.5 {
  let an=abs(n);
  let along_z=an.x>0.5||(an.y>0.5&&fract(abs(p.x)*3.7)>0.5);
  let wp=select(vec3<f32>(p.x*3.2,p.y*28.,p.z*28.),vec3<f32>(p.z*3.2,p.y*28.,p.x*28.),along_z);
  let ring=sin(wp.y*1.4+original_noise(wp*0.35)*5.5)*0.5+0.5;
  let fibre=original_noise(wp*vec3<f32>(2.5,3.5,3.5));
  let plank=smoothstep(0.02,0.06, abs(fract(select(p.z,p.x,along_z)*7.5)-0.5));
  let tone=mix(0.68,1.16,ring*0.55+fibre*0.45)*(0.90+0.16*broad);
  return vec3<f32>(tone*(0.78+0.22*plank));
 }
 let streak=original_noise(vec3<f32>(p.x*45.,p.y*12.,p.z*45.));
 let patina=0.92+0.14*original_noise(p*5.5);
 let verdigris=smoothstep(0.60,0.88,original_noise(p*4.2))*(1.-smoothstep(0.1,1.3,p.y));
 let base_metal=vec3<f32>((0.88+0.12*original_noise(p*27.)+0.06*(streak-0.5))*patina);
 return mix(base_metal,base_metal*vec3<f32>(0.76,1.08,0.96),verdigris*0.32);
}
// Band-limited physical height fields in metres. Screen derivatives suppress
// unresolved grain instead of allowing moving-camera specular aliasing.
fn original_height_lod(p:vec3<f32>,kind:f32,fine:f32,medium:f32)->f32 {
 if kind>8.5 {
  if kind>12.5 {
   if kind>14.5 {return 0.0007*medium*original_noise(p*22.)+0.00025*fine*original_noise(p*110.);}
   if kind>13.5 {
    let uv=(p.xz+vec2<f32>(p.y))/0.15;
    let f=fract(uv);let edge=min(min(f.x,1.-f.x),min(f.y,1.-f.y));
    let pillow=smoothstep(0.015,0.08,edge)*medium;
    return 0.0005*pillow+0.00018*medium*original_noise(p*12.);
   }
   return 0.00055*medium*original_noise(p*8.5)+0.00028*fine*original_noise(p*115.);
  }
  if kind>10.5 {
   if kind>11.5 {
    let f_slab=fract(p.xz/0.7+vec2<f32>(0.01));
    let d_chamfer=min(min(f_slab.x,1.-f_slab.x),min(f_slab.y,1.-f_slab.y))*0.7;
    let bevel=(1.-smoothstep(0.002,0.014,d_chamfer))*medium;
    let vein=original_noise(p*vec3<f32>(3.,9.,2.))*medium;
    let pore=smoothstep(0.60,0.85,original_noise(p*54.))*fine;
    return 0.0014*vein-0.00085*pore-0.00075*bevel+0.00028*fine*original_noise(p*160.);
   }
   let weave=(sin(p.x*420.)*sin(p.z*420.)+sin(p.y*420.)*0.5)*fine;
   return 0.0006*medium*original_noise(p*14.)+0.00022*weave;
  }
  if kind>9.5 {
   let tuft=original_noise(vec3<f32>(p.x*16.,p.y*4.,p.z*16.))*medium;
   let blades=original_noise(vec3<f32>(p.x*58.,p.y*8.,p.z*58.))*medium;
   let micro=original_noise(vec3<f32>(p.x*165.,p.y*16.,p.z*165.))*fine;
   return 0.0085*tuft+0.0042*blades+0.0014*micro;
  }
  let uv=p.xz+vec2<f32>(p.y);
  let f=fract(uv/0.025);
  let edge=min(min(f.x,1.-f.x),min(f.y,1.-f.y));
  let pillow=smoothstep(0.04,0.22,edge)*medium;
  return 0.0011*pillow+0.00012*fine*original_noise(p*180.);
 }
 if kind>5.5 {
  if kind>7.5 {
   let bark_groove=original_noise(p*vec3<f32>(65.,2.5,65.))*medium;
   return 0.0028*bark_groove+0.0005*fine*original_noise(p*140.);
  }
  if kind>6.5 {
   let uv=p.xy+vec2<f32>(p.z,0.);let row=floor(uv.y/0.075);
   let cell=fract(vec2<f32>(uv.x/0.225+0.5*(row-2.*floor(row*0.5)),uv.y/0.075));
   let edge=min(min(cell.x,1.-cell.x)*0.225,min(cell.y,1.-cell.y)*0.075);
   let mortar=(1.-smoothstep(0.002,0.008,edge))*medium;
   return 0.0012*medium*original_noise(p*24.)-0.0022*mortar;
  }
  let cluster=original_noise(p*4.8)*medium;
  let sub_cluster=original_noise(p*12.5)*medium;
  let leaf=original_noise(p*38.)*medium;
  let cell=original_noise(p*135.)*fine;
  return 0.011*cluster+0.0052*sub_cluster+0.0022*leaf+0.00055*cell;
 }
 if kind<2.5 {
  if kind<1.5 {
   let side_slab=smoothstep(4.65,4.85,abs(p.x))*(1.-smoothstep(6.95,7.25,abs(p.x)))*smoothstep(-26.,-24.,p.z)*(1.-smoothstep(81.,83.,p.z))*medium;
   let gx=abs(fract(p.x/0.72)-0.5)*0.72;
   let gz=abs(fract(p.z/0.90)-0.5)*0.90;
   let groove=(1.-smoothstep(0.004,0.014,min(gx,gz)))*side_slab;
   return 0.003*medium*original_noise(p*16.)+0.00065*fine*original_noise(p*145.)-0.0018*groove;
  }
  return 0.0012*medium*original_noise(p*23.)+0.00035*fine*original_noise(p*180.);
 }
 if kind<3.5 {
  let course=fract(p.y*12.)*medium;
  return 0.0022*course+0.0014*medium*original_noise(p*32.)+0.0003*fine*original_noise(p*140.);
 }
 if kind<4.5 {return 0.0006*medium*original_noise(p*24.)+0.00055*fine*original_noise(p*vec3<f32>(110.,18.,110.));}
 return 0.00022*medium*original_noise(p*34.)+0.00018*fine*original_noise(p*210.);
}
fn original_height(p:vec3<f32>,kind:f32)->f32 {
 let footprint=max(length(dpdxFine(p)),length(dpdyFine(p)));
 let fine=1.-smoothstep(0.002,0.012,footprint);
 let medium=1.-smoothstep(0.02,0.08,footprint);
 return original_height_lod(p,kind,fine,medium);
}
// Wind-driven micro-flutter for foliage, grass, flower petals, and awning/parasol fabrics
// without altering @invariant clip positions or TAA history depth alignment.
fn wind_flutter_world(p:vec3<f32>,kind:f32)->vec3<f32> {
 let w_len=length(camera.wind.xy);
 if w_len<=0.01 {return p;}
 let is_foliage=(kind>5.5&&kind<6.5)||(kind>9.5&&kind<11.5)||kind>14.5;
 if !is_foliage {return p;}
 let t=camera.atmosphere.x;
 let gust=1.+0.35*camera.wind.z*sin(t*1.4+dot(p.xz,vec2<f32>(0.35,0.28)));
 let phase=dot(p,vec3<f32>(2.4,1.7,2.1))+t*(2.2+0.6*w_len);
 let amp=select(0.0045,0.0085,kind>5.5&&kind<6.5)*min(w_len,3.5)*gust;
 return p+vec3<f32>(sin(phase)*camera.wind.x,cos(phase*1.3)*0.35*w_len,cos(phase)*camera.wind.y)*amp;
}
// 4-step Parallax Occlusion Mapping (POM) in world tangent plane for high-relief materials
// (cobblestone/slabs, roof slate, brick masonry, bark, travertine, ceramic tiles).
fn pom_world(p:vec3<f32>,n:vec3<f32>,view:vec3<f32>,kind:f32)->vec3<f32> {
 let has_relief=(kind>0.5&&kind<1.5)||(kind>2.5&&kind<3.5)||(kind>6.5&&kind<8.5)||(kind>11.5&&kind<12.5)||(kind>13.5&&kind<14.5);
 if !has_relief {return p;}
 let d2=dot(p-camera.eye.xyz,p-camera.eye.xyz);
 if d2>256. {return p;}
 let fade=1.-smoothstep(121.,256.,d2);
 let nv=max(dot(n,view),0.22);
 let tangent_ray=(view-n*dot(n,view))/nv;
 let scale=select(3.2,4.5,(kind>6.5&&kind<7.5)||(kind>2.5&&kind<3.5))*fade;
 var curr_p=p;
 var prev_p=p;
 var prev_diff=0.;
 for(var i=1u;i<=4u;i++) {
  let layer=f32(i)*0.25;
  let test_p=p-tangent_ray*(layer*0.0032*scale);
  let h=clamp(0.5-original_height_lod(test_p,kind,0.5,1.)*180.,0.,1.);
  let diff=layer-h;
  if diff>=0. {
   let t_interp=prev_diff/max(prev_diff-diff,1e-4);
   curr_p=mix(prev_p,test_p,clamp(t_interp,0.,1.));
   break;
  }
  prev_p=test_p;
  prev_diff=diff;
  curr_p=test_p;
 }
 return curr_p;
}
// 2-step POM self-shadowing along the sun direction in the micro-heightfield.
fn pom_self_shadow(p:vec3<f32>,n:vec3<f32>,sun:vec3<f32>,kind:f32)->f32 {
 let has_relief=(kind>0.5&&kind<1.5)||(kind>2.5&&kind<3.5)||(kind>6.5&&kind<8.5)||(kind>11.5&&kind<12.5);
 if !has_relief {return 1.;}
 let d2=dot(p-camera.eye.xyz,p-camera.eye.xyz);
 if d2>225. {return 1.;}
 let nl=dot(n,sun);
 if nl<=0.05 {return 1.;}
 let sun_tan=normalize(sun-n*nl+vec3<f32>(1e-5));
 let h0=original_height_lod(p,kind,0.4,1.);
 let h1=original_height_lod(p+sun_tan*0.006,kind,0.4,1.);
 let h2=original_height_lod(p+sun_tan*0.014,kind,0.4,1.);
 let dh=max(max(h1-h0-0.0005*nl,h2-h0-0.0012*nl),0.);
 return clamp(1.-dh*240.*(1.-smoothstep(121.,225.,d2)),0.42,1.);
}
fn original_bump(p:vec3<f32>,n:vec3<f32>,height:f32)->vec3<f32>{
 let px=dpdxFine(p);let py=dpdyFine(p);let rx=cross(py,n);let ry=cross(n,px);
 let determinant=dot(px,rx);
 let gradient=sign(determinant)*(dpdxFine(height)*rx+dpdyFine(height)*ry);
 return normalize(abs(determinant)*n-gradient+1e-20*n);
}
// Profiling switch: VOXY_FX_OFF bit mask (1 GI, 2 SSR, 4 contact AO, 8 sun shadow, 16 wet layer).
fn fx_off(bit:u32)->bool{return (u32(camera.wind.w)&bit)!=0u;}
fn contact_occlusion(pixel:vec2<f32>,p:vec3<f32>,n:vec3<f32>)->vec4<f32>{
 if fx_off(4u) {return vec4<f32>(n,1.);}
 let view_dist=max(distance(p,camera.eye.xyz),0.5);
 if view_dist>68. {return vec4<f32>(n,1.);}
 let radius=0.55;let projected=clamp(radius*camera.viewport.y/view_dist,2.,96.);
 let ign=fract(52.9829189*fract(dot(floor(pixel),vec2<f32>(0.06711056,0.00583715))))*0.75;
 var cs=vec2<f32>(cos(ign),sin(ign));
 let rot=vec2<f32>(-0.7373688,0.6754903);
 let inv_vp=vec2<f32>(2.,-2.)/camera.viewport.xy;
 let steps=select(12u,select(8u,6u,view_dist>36.),view_dist>18.);
 let inv_steps=1./f32(steps);
 var sum=0.;var weight=0.;var occl_dir=vec3<f32>(0.);
 for(var i=0u;i<steps;i++){
  let r=sqrt((f32(i)+0.5)*inv_steps);
  let uv=pixel+cs*(projected*r);
  cs=vec2<f32>(cs.x*rot.x-cs.y*rot.y,cs.x*rot.y+cs.y*rot.x);
  if any(uv<vec2<f32>(0.))||any(uv>=camera.viewport.xy){continue;}
  let depth=textureLoad(scene_depth,vec2<i32>(uv),0);
  if depth>=1.{continue;}
  let h=camera.inverse_vp*vec4<f32>((floor(uv)+0.5)*inv_vp+vec2<f32>(-1.,1.),depth,1.);
  let delta=h.xyz/h.w-p;let d2=dot(delta,delta);
  if d2>0.000144&&d2<0.3025 {
   let inv_len=inverseSqrt(d2);
   let dir=delta*inv_len;
   let falloff=1.-d2*inv_len*(1./0.55);
   let w_occ=max(dot(n,dir)-0.16,0.)*falloff*falloff;
   sum+=w_occ;
   occl_dir+=dir*w_occ;
  }
  weight+=1.;
 }
 let inv_w=1./max(weight,1.);
 let ao=clamp(1.-2.4*sum*inv_w,0.25,1.);
 let bent_n=normalize(n-occl_dir*(1.65*inv_w)+1e-5*n);
 return vec4<f32>(bent_n,ao);
}
// Screen-space depth occlusion for punctual lights (prevents streetlights/lamps bleeding through walls).
fn point_light_occlusion(p:vec3<f32>,light_pos:vec3<f32>,d2_view:f32,distance2:f32)->f32{
 if d2_view>324.||distance2>64.||distance2<0.12 {return 1.;}
 let to_l=light_pos-p;
 let inv_vp=vec2<f32>(2.,-2.)/camera.viewport.xy;
 let sample_p=p+to_l*0.48;
 let q=camera.vp*vec4<f32>(sample_p,1.);
 if q.w<=0. {return 1.;}
 let uv=q.xy/q.w*vec2<f32>(0.5,-0.5)+0.5;
 if any(uv<=vec2<f32>(0.))||any(uv>=vec2<f32>(1.)){return 1.;}
 let px=vec2<i32>(uv*camera.viewport.xy);
 let z=textureLoad(scene_depth,px,0);
 if z<1.&&q.z/q.w>z+0.00008 {
  let h=camera.inverse_vp*vec4<f32>((vec2<f32>(px)+0.5)*inv_vp+vec2<f32>(-1.,1.),z,1.);
  let blocker_d=distance(h.xyz/h.w,sample_p);
  if blocker_d>0.06&&blocker_d<1.25 {return mix(0.28,1.,smoothstep(196.,324.,d2_view));}
 }
 return 1.;
}
// Karis / LTC Representative Point for linear tube and area lights:
// shifts the specular evaluation vector toward the closest point on the light segment/sphere.
fn area_light_spec_dir(p:vec3<f32>,light_pos:vec3<f32>,reflected:vec3<f32>,radius:f32)->vec3<f32>{
 let l_vec=light_pos-p;
 if radius<4.5 {return normalize(l_vec);}
 let tube_half=clamp((radius-4.0)*0.11,0.12,0.65);
 let axis=select(vec3<f32>(0.,0.,1.),vec3<f32>(1.,0.,0.),abs(light_pos.x)>4.2);
 let l0=l_vec-axis*tube_half;
 let l1=l_vec+axis*tube_half;
 let ld=l1-l0;
 let r_dot_ld=dot(reflected,ld);
 let ld_len2=max(dot(ld,ld),1e-4);
 let t_seg=clamp((dot(reflected,l0)*r_dot_ld-dot(l0,ld))/max(ld_len2-r_dot_ld*r_dot_ld,1e-4),0.,1.);
 let closest_seg=l0+ld*t_seg;
 let center_ray=dot(closest_seg,reflected)*reflected-closest_seg;
 let sphere_r=0.14;
 let closest=closest_seg+center_ray*clamp(sphere_r*inverseSqrt(max(dot(center_ray,center_ray),1e-5)),0.,1.);
 return normalize(closest);
}
// Screen-Space Contact Shadows (UE5 / Unity HDRP SSCS): 4-step short-range depth march
// along the sun vector to ground thin geometry (chair legs, railings, leaves, kerbs).
fn contact_shadow(p:vec3<f32>,sun:vec3<f32>,jitter:f32)->f32{
 if fx_off(8u) {return 1.;}
 let d2=dot(p-camera.eye.xyz,p-camera.eye.xyz);
 if d2>900. {return 1.;}
 let inv_vp=vec2<f32>(2.,-2.)/camera.viewport.xy;
 for(var i=0u;i<4u;i++){
  let t=(f32(i)+0.35+jitter*0.65)*0.075;
  let q=camera.vp*vec4<f32>(p+sun*t,1.);
  if q.w<=0. {break;}
  let uv=q.xy/q.w*vec2<f32>(0.5,-0.5)+0.5;
  if any(uv<=vec2<f32>(0.))||any(uv>=vec2<f32>(1.)){break;}
  let px=vec2<i32>(uv*camera.viewport.xy);
  let z=textureLoad(scene_depth,px,0);
  if z<1. {
   let ray_z=q.z/q.w;
   if ray_z>z+0.00003 {
    let h=camera.inverse_vp*vec4<f32>((vec2<f32>(px)+0.5)*inv_vp+vec2<f32>(-1.,1.),z,1.);
    let thickness=distance(h.xyz/h.w,p+sun*t);
    if thickness<0.22 {return mix(0.18,1.,smoothstep(625.,900.,d2));}
   }
  }
 }
 return 1.;
}
// Trace against the current full-resolution opaque depth and radiance. A miss
// retains the HDR environment; no screen-space result replaces hidden geometry.
// With `history`, the opaque pass reads the previous frame's opaque radiance:
// the hit point is reprojected through that frame's matrix.
// `reach` bounds the march (metres), `growth` scales the step with distance
// and `refines` sets the binary-search depth after a crossing.
fn screen_reflection(origin:vec3<f32>,direction:vec3<f32>,history:bool,reach:f32,growth:f32,refines:u32,bounded:bool)->vec4<f32>{
 if history&&camera.surface.z<0.5 {return vec4<f32>(0.);}
 if ARCHIVE_PAIRED_DDA&&!ARCHIVE_RAY_FORCE_FALLBACK {
  let segment=dda_project_segment(camera.vp,origin,origin+direction*reach,camera.viewport.xy);
  // Unsupported projection remains on the complete original path.
  if segment.state!=2u {return screen_reflection_paired(segment,history);}
 }
 return screen_reflection_original(origin,direction,history,reach,growth,refines,bounded);
}
fn screen_reflection_original(origin:vec3<f32>,direction:vec3<f32>,history:bool,reach:f32,growth:f32,refines:u32,bounded:bool)->vec4<f32>{
 if history&&camera.surface.z<0.5 {return vec4<f32>(0.);}
 var t=0.08;var previous=0.;
 var cached_tile=vec2<i32>(-1);var cached_lower=0.;
 var cached_pixel=vec2<i32>(-1);var cached_depth=1.;
 for(var step=0u;step<80u;step++) {
  let q=camera.vp*vec4<f32>(origin+direction*t,1.);
  if q.w<=0. {return vec4<f32>(0.);}
  let uv=q.xy/q.w*vec2<f32>(0.5,-0.5)+0.5;
  if any(uv<=vec2<f32>(0.))||any(uv>=vec2<f32>(1.)){return vec4<f32>(0.);}
  let ray_pixel=vec2<i32>(uv*camera.viewport.xy);
  // The minimum is a rejection bound only. Potential hits retain the original
  // level-zero read, step recurrence, crossing and all binary refinements.
  var depth=1.;
  if bounded {
   let tile=ray_pixel/8;
   if any(tile!=cached_tile) {cached_lower=textureLoad(ray_depth_bounds,tile,0).x;cached_tile=tile;}
   if q.z/q.w>=cached_lower {
    if any(ray_pixel!=cached_pixel) {cached_depth=textureLoad(scene_depth,ray_pixel,0);cached_pixel=ray_pixel;}
    depth=cached_depth;
   }
  } else if ARCHIVE_RAY_DEPTH_REUSE {
   if any(ray_pixel!=cached_pixel) {cached_depth=textureLoad(scene_depth,ray_pixel,0);cached_pixel=ray_pixel;}
   depth=cached_depth;
  } else {depth=textureLoad(scene_depth,ray_pixel,0);}
  if depth<1.&&q.z/q.w>=depth {
   var lo=previous;var hi=t;
   for(var refine=0u;refine<refines;refine++) {
    let mid=(lo+hi)*0.5;let clip=camera.vp*vec4<f32>(origin+direction*mid,1.);
    let pixel=clamp(vec2<i32>((clip.xy/clip.w*vec2<f32>(0.5,-0.5)+0.5)*camera.viewport.xy),vec2<i32>(0),vec2<i32>(camera.viewport.xy)-1);
    var z=1.;
    if ARCHIVE_RAY_DEPTH_REUSE || (bounded && ARCHIVE_REFINE_CACHE) {
     if any(pixel!=cached_pixel) {cached_depth=textureLoad(scene_depth,pixel,0);cached_pixel=pixel;}
     z=cached_depth;
    } else {z=textureLoad(scene_depth,pixel,0);}
    if z<1.&&clip.z/clip.w>=z {hi=mid;} else {lo=mid;}
   }
   let point=origin+direction*hi;let clip=camera.vp*vec4<f32>(point,1.);
   let coord=clip.xy/clip.w*vec2<f32>(0.5,-0.5)+0.5;
   let pixel=clamp(vec2<i32>(coord*camera.viewport.xy),vec2<i32>(0),vec2<i32>(camera.viewport.xy)-1);
   var z=1.;
   if ARCHIVE_RAY_DEPTH_REUSE || (bounded && ARCHIVE_REFINE_CACHE) {
    if any(pixel!=cached_pixel) {cached_depth=textureLoad(scene_depth,pixel,0);cached_pixel=pixel;}
    z=cached_depth;
   } else {z=textureLoad(scene_depth,pixel,0);}
   let h=camera.inverse_vp*vec4<f32>((vec2<f32>(pixel)+0.5)/camera.viewport.xy*vec2<f32>(2.,-2.)+vec2<f32>(-1.,1.),z,1.);
   if distance(h.xyz/h.w,point)<0.20 {
    var edge=min(min(coord.x,coord.y),min(1.-coord.x,1.-coord.y));
    var source=pixel;
    if history {
     let hit_world=h.xyz/h.w;
     let old=camera.previous_vp*vec4<f32>(hit_world,1.);
     if old.w<=0. {return vec4<f32>(0.);}
     let old_uv=old.xy/old.w*vec2<f32>(0.5,-0.5)+0.5;
     edge=min(edge,min(min(old_uv.x,old_uv.y),min(1.-old_uv.x,1.-old_uv.y)));
     if edge<=0. {return vec4<f32>(0.);}
     source=clamp(vec2<i32>(old_uv*camera.viewport.xy),vec2<i32>(0),vec2<i32>(camera.viewport.xy)-1);
     let stored_depth=textureLoad(opaque_history_depth,source,0);
     if stored_depth>=1. {return vec4<f32>(0.);}
     let old_ndc=(vec2<f32>(source)+0.5)/camera.viewport.xy*vec2<f32>(2.,-2.)+vec2<f32>(-1.,1.);
     let recovered=camera.previous_inverse_vp*vec4<f32>(old_ndc,stored_depth,1.);
     if recovered.w==0. || !archive_history_surface_matches(hit_world,recovered.xyz/recovered.w,old.w,camera.surface.w) {return vec4<f32>(0.);}

    }
    let fade=smoothstep(0.,0.04,edge)*(1.-smoothstep(0.75,1.,hi/reach));
    return vec4<f32>(textureLoad(background,source,0).rgb/camera.viewport.z,fade);
   }
   return vec4<f32>(0.);
  }
  previous=t;t+=growth*(0.04+0.06*t);
  if t>reach {break;}
 }
 return vec4<f32>(0.);
}
fn filter_rough_ssr(reflected_scene:vec4<f32>,rough_eff:f32,fallback_radiance:vec3<f32>)->vec4<f32>{
 if reflected_scene.a<=0. {return vec4<f32>(0.);}
 let luma=dot(reflected_scene.rgb,vec3<f32>(0.2126,0.7152,0.0722));
 let cap=max(dot(fallback_radiance,vec3<f32>(0.2126,0.7152,0.0722))*6.,1800.);
 var rgb=reflected_scene.rgb;
 if luma>cap {rgb*=cap/max(luma,1e-4);}
 let rough_cone=smoothstep(0.06,0.48,rough_eff);
 rgb=mix(rgb,fallback_radiance,rough_cone*0.52);
 return vec4<f32>(rgb,reflected_scene.a*(1.-rough_cone*0.28));
}
// Screen-space one-bounce GI: one cosine-distributed ray per pixel per frame,
// marched against the depth buffer and shaded from the reprojected previous
// frame (which already carries earlier bounces). TAA integrates the noise.
// Returns hit radiance and coverage; misses keep the sky/environment term.
fn clamp_gi_firefly(gi:vec4<f32>)->vec4<f32>{
 let luma=dot(gi.rgb,vec3<f32>(0.2126,0.7152,0.0722));
 let cap=1600.;
 if luma>cap {
  return vec4<f32>(gi.rgb*(cap/max(luma,1e-4)),gi.a);
 }
 return gi;
}
fn load_computed_gi_filtered(pixel:vec2<i32>,center_z:f32)->vec4<f32>{
 let dims=vec2<i32>(textureDimensions(computed_gi));
 let p=clamp(pixel,vec2<i32>(0),dims-vec2<i32>(1));
 let c=clamp_gi_firefly(textureLoad(computed_gi,p,0));
 var sum=c*2.0;
 var w_sum=2.0;
 let offsets=array<vec2<i32>,4>(vec2<i32>(2,0),vec2<i32>(-2,0),vec2<i32>(0,2),vec2<i32>(0,-2));
 for(var i=0;i<4;i++){
  let q=clamp(p+offsets[i],vec2<i32>(0),dims-vec2<i32>(1));
  let dz=abs(textureLoad(scene_depth,q,0)-center_z);
  let w=exp(-dz*650.);
  sum+=clamp_gi_firefly(textureLoad(computed_gi,q,0))*w;
  w_sum+=w;
 }
 return sum/w_sum;
}
fn screen_gi(pixel:vec2<f32>,p:vec3<f32>,n:vec3<f32>)->vec4<f32>{
 if fx_off(1u) {return vec4<f32>(0.);}
 if !ARCHIVE_COMPUTE_GI&&!ARCHIVE_PAIRED_DDA {
  let d2=dot(p-camera.eye.xyz,p-camera.eye.xyz);
  if d2>2704. {return vec4<f32>(0.);}
  let direction=archive_gi_direction(pixel,n);
  let far_fade=1.-smoothstep(1296.,2704.,d2);
  let gi=clamp_gi_firefly(screen_reflection(p+n*0.05,direction,true,select(38.,22.,d2>784.),select(1.26,1.48,d2>784.),select(5u,4u,d2>784.),ARCHIVE_RAY_BOUNDS));
  return vec4<f32>(gi.rgb,gi.a*far_fade);
 }
 let direction=archive_gi_direction(pixel,n);
 // Cosine-distributed diffuse bounce march; probes supplement offscreen light.
 return clamp_gi_firefly(screen_reflection(p+n*0.05,direction,true,42.,1.22,5u,ARCHIVE_RAY_BOUNDS));
}
fn archive_gi_direction(pixel:vec2<f32>,n:vec3<f32>)->vec3<f32>{
 let seed=fract(sin(dot(pixel+vec2<f32>(camera.atmosphere.x*61.7,camera.atmosphere.x*17.3),vec2<f32>(12.9898,78.233)))*43758.5453);
 let seed2=fract(seed*7.1+fract(dot(pixel,vec2<f32>(0.06711056,0.00583715))*52.9829189));
 let r=sqrt(seed);let phi=6.2831853*seed2;
 let t=normalize(select(cross(n,vec3<f32>(0.,1.,0.)),cross(n,vec3<f32>(1.,0.,0.)),abs(n.y)>0.9));
 let b=cross(n,t);
 let direction=normalize(t*r*cos(phi)+b*r*sin(phi)+n*sqrt(max(1.-seed,0.)));
 return direction;
}
// Rain wetting: porous darkening and lower roughness everywhere exposed, plus
// flat low-roughness puddles in noise basins on upward-facing surfaces.
struct WetLayer { darken:f32, roughness:f32, normal:vec3<f32>, puddle:f32, splash:f32 }
// Crown splash of one drop impact: an expanding ring that breaks into beads
// within ~50 ms, lit by the sky. Returns the beads' coverage of the pixel.
fn splash_cell(p:vec2<f32>,size:f32,offset:f32)->f32{
 let cell=floor(p/size);let seed=original_hash(vec3<f32>(cell,offset));
 if seed>camera.weather.z {return 0.;}
 let centre=(cell+0.2+0.6*vec2<f32>(seed,original_hash(vec3<f32>(cell.yx,offset+5.))))*size;
 let life=fract(camera.atmosphere.x*2.3+seed*29.)*8.;
 if life>1. {return 0.;}
 let delta=p-centre;let d=length(delta);
 let ring=exp(-square((d-0.006-0.025*life)/0.0035));
 let beads=smoothstep(0.35,0.9,sin(atan2(delta.y,delta.x)*9.+seed*40.)*0.5+0.5);
 return ring*mix(0.35,1.,beads)*(1.-life);
}
fn ripple_cell(p:vec2<f32>,size:f32,offset:f32)->vec2<f32>{
 let cell=floor(p/size);let seed=original_hash(vec3<f32>(cell,offset));
 let centre=(cell+0.25+0.5*vec2<f32>(seed,original_hash(vec3<f32>(cell.yx,offset+7.))))*size;
 let age=fract(camera.atmosphere.x*1.6+seed*13.);
 if seed>camera.weather.z {return vec2<f32>(0.);}
 let delta=p-centre;let d=length(delta);if d<0.0001 {return vec2<f32>(0.);}
 let x=d-age*size*0.45;
 let slope=cos(x*95.)*exp(-x*x*2500.)*(1.-age)*(1.-age);
 return delta/d*slope;
}
fn wet_layer(p:vec3<f32>,geometric:vec3<f32>,normal:vec3<f32>,roughness:f32,albedo:f32)->WetLayer{
 var w:WetLayer;w.darken=1.;w.roughness=roughness;w.normal=normal;w.puddle=0.;w.splash=0.;
 let wet=camera.surface.x;if wet<=0. {return w;}
 // Pixel footprint, taken in uniform control flow, fades sub-pixel ripples into roughness.
 let footprint=length(fwidthFine(p));
 let up=smoothstep(0.7,0.95,geometric.y);
 // Bevels and joints still hold a level water surface; walls do not.
 let open=smoothstep(0.2,0.6,geometric.y);
 let damp=wet*mix(0.35,1.,up);
 // Water filling pores (Lagarde 2013): bright porous stone loses the most albedo.
 w.darken=1.-damp*mix(0.35,0.7,clamp(albedo,0.,1.));
 // A soaked surface is not yet a water film: it stays glossy-rough, not mirror.
 w.roughness=mix(roughness,clamp(roughness*0.55,0.18,1.),damp);
 // Value-noise sums cluster near 0.5; stretch so coverage maps to area.
 let basin=clamp((original_noise(vec3<f32>(p.x*0.45,3.7,p.z*0.45))*0.7+original_noise(vec3<f32>(p.x*1.6,9.1,p.z*1.6))*0.3-0.5)*2.6+0.5,0.,1.);
 let threshold=1.-camera.surface.y*wet;
 // Saturated rim around each pool: darker and smoother than the soaked surface.
 let halo=smoothstep(threshold-0.16,threshold,basin)*open*wet;
 w.darken*=1.-0.2*halo;
 w.roughness=mix(w.roughness,max(w.roughness*0.5,0.07),halo);
 // Once soaked, joints between street-level stones fill before basins do.
 let street=1.-smoothstep(camera.eye.y-1.3,camera.eye.y-0.9,p.y);
 let joint=(1.-smoothstep(0.82,0.97,geometric.y))*open*street*smoothstep(0.55,0.9,wet);
 let water=max(smoothstep(threshold,threshold+0.05,basin)*open,joint*0.85);
 w.puddle=water;
 let resolved_splash=1.-smoothstep(0.003,0.012,footprint);
 if camera.weather.z>0.&&resolved_splash>0.&&up>0. {
  w.splash=(splash_cell(p.xz,0.23,11.)+splash_cell(p.xz+0.11,0.17,13.))*up*resolved_splash*wet;
 }
 if water>0. {
  let depth=clamp((basin-threshold)*6.,0.,1.);
  let resolved=1.-smoothstep(0.004,0.02,footprint);
  var tilt=vec2<f32>(0.);
  if camera.weather.z>0.&&resolved>0. {
   tilt=(ripple_cell(p.xz,0.19,0.)+ripple_cell(p.xz+0.07,0.14,3.)+ripple_cell(p.xz+0.13,0.11,5.))*0.32*resolved;
  }
  let level=normalize(vec3<f32>(-tilt.x,1.,-tilt.y));
  w.normal=normalize(mix(normal,level,water));
  // Unresolved impacts still scatter the reflection.
  w.roughness=mix(w.roughness,0.012+0.07*camera.weather.z*(1.-resolved),water);
  w.darken*=1.-water*(0.25+0.3*depth);
 }
 return w;
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
// One material evaluation owns the final normal used by lighting and ray inputs.
struct ArchiveSurface {
 base:vec3<f32>,opacity:f32,uv:vec2<f32>,spec:vec4<f32>,f0:vec3<f32>,
 roughness:f32,diffuse:vec3<f32>,normal:vec3<f32>,view:vec3<f32>,geometric:vec3<f32>,splash:f32
}
fn archive_surface(f:Fragment,front:bool,tex:vec4<f32>)->ArchiveSurface {
 let face=select(-1.,1.,front);let geometric=normalize(f.normal)*face;let view=normalize(camera.eye.xyz-f.world);
 var p_mat=f.world;
 if material.workflow.z>0.5 {
  p_mat=pom_world(wind_flutter_world(f.world,material.workflow.z),geometric,view,material.workflow.z);
 }
 var base=tex.rgb*material.diffuse.rgb;if material.workflow.z>0.5{base*=original_surface(p_mat,f.normal,material.workflow.z);}else if material.workflow.w>0.{let damp=1.-0.18*(1.-smoothstep(0.1,1.65,f.world.y))*(0.35+0.65*original_noise(f.world*1.8))*smoothstep(0.3,0.8,1.-abs(f.normal.y));let drip=1.-0.07*smoothstep(0.58,0.88,original_noise(vec3<f32>(f.world.x*6.5,f.world.y*0.65,f.world.z*6.5)))*smoothstep(0.3,0.8,1.-abs(f.normal.y));let macro_v=0.92+0.16*original_noise(f.world*0.35);base*=damp*drip*macro_v;}else if material.workflow.w<0.{let moss=smoothstep(0.54,0.82,original_noise(f.world*3.8))*(1.-smoothstep(0.15,0.95,f.world.y))*smoothstep(-0.2,0.5,-f.normal.z);base=mix(base,vec3<f32>(0.05,0.11,0.03),moss*0.45);}let opacity=select(tex.a,1.,material.workflow.y>1.5)*material.diffuse.a;
 let uv=surface_uv(f);let packed=material.workflow.y>1.5;var packed_normal=vec4<f32>(0.);var spec=vec4<f32>(1.);
 if packed {packed_normal=textureSample(normal_map,material_sampler,uv);spec=vec4<f32>(tex.a,packed_normal.a,0.,1.);}else{spec=textureSample(specular_map,material_sampler,uv);}var f0=spec.rgb*material.specular_gloss.rgb;var roughness=1.-spec.a*material.specular_gloss.a;var diffuse=base*(1.-max(max(f0.r,f0.g),f0.b));
 if material.workflow.x>0.5{let metallic=material.surface.x*spec.b;roughness=material.surface.y*spec.g;f0=mix(vec3<f32>(0.04),base,metallic);diffuse=base*(1.-metallic);}
 var normal=geometric;
 if material.workflow.y>0.5 {
  var tangent=normalize(f.tangent.xyz-normal*dot(normal,f.tangent.xyz))*face;var bitangent=cross(normal,tangent)*f.tangent.w*face;
  if material.workflow.w>0. {let n=abs(f.normal);if n.x>=n.y&&n.x>=n.z {tangent=vec3<f32>(0.,0.,-sign(f.normal.x));bitangent=vec3<f32>(0.,1.,0.);}else if n.y>=n.z {tangent=vec3<f32>(1.,0.,0.);bitangent=vec3<f32>(0.,0.,-sign(f.normal.y));}else{tangent=vec3<f32>(sign(f.normal.z),0.,0.);bitangent=vec3<f32>(0.,1.,0.);}}
  var mapped=vec3<f32>(0.,0.,1.);if packed {mapped=packed_normal.xyz*2.-1.;}else{mapped=textureSample(normal_map,material_sampler,uv).xyz*2.-1.;}mapped.x*=material.emissive_normal.w;mapped.y*=material.emissive_normal.w;normal=normalize(mat3x3<f32>(tangent,bitangent,normal)*mapped);
 }
 if material.workflow.z>0.5{let height=original_height(p_mat,material.workflow.z);normal=original_bump(p_mat,normal,height);let r_amp=select(0.16,0.06,material.workflow.z>12.5);roughness=clamp(roughness+(original_noise(p_mat*7.)-0.5)*r_amp,0.04,1.);}let dndx=dpdxFine(normal);let dndy=dpdyFine(normal);let sigma2=min(dot(dndx,dndx)+dot(dndy,dndy),0.25);roughness=clamp(sqrt(roughness*roughness+0.35*sigma2),0.04,1.);
 var splash=0.;
 if camera.surface.x>0.&&material.surface.z==0.&&!fx_off(16u)&&!(material.workflow.z>5.5&&material.workflow.z<6.5) {
  let wet=wet_layer(f.world,geometric,normal,roughness,dot(base,vec3<f32>(0.2126,0.7152,0.0722)));
  diffuse*=wet.darken;roughness=wet.roughness;normal=wet.normal;f0=mix(f0,max(f0,vec3<f32>(0.02)),wet.puddle);splash=wet.splash;
 }
 if camera.weather.w>0.&&material.surface.z==0. {
  let snow_up=smoothstep(0.45,0.82,geometric.y);
  let snow_drift=0.65+0.55*original_noise(f.world*1.3);
  let snow_cover=clamp(camera.weather.w*snow_up*snow_drift,0.,0.92);
  let crystal=smoothstep(0.92,0.995,original_hash(floor(f.world*95.)));
  diffuse=mix(diffuse,vec3<f32>(0.86,0.91,0.97)+vec3<f32>(0.25)*crystal,snow_cover);
  roughness=mix(roughness,max(0.18,0.46-0.28*crystal),snow_cover);
  normal=normalize(mix(normal,geometric,snow_cover*0.55));
 }
 return ArchiveSurface(base,opacity,uv,spec,f0,roughness,diffuse,normal,view,geometric,splash);
}
// True 3D Ray-Box Interior Mapping (van Dongen 2007 / UE5 Interior Cubemap):
// intersects the refracted view ray against a virtual 3D room cell behind facade glass,
// shading ceiling, hardwood floor, back wall, side walls, pendant lamp, and window curtains.
fn interior_mapping_window(p:vec3<f32>,geometric:vec3<f32>,view:vec3<f32>)->vec3<f32>{
 let cell_size=vec3<f32>(3.2,2.65,3.2);
 let room_cell=floor(p/cell_size);
 let room_seed=original_hash(room_cell);
 let room_lit=smoothstep(0.24,0.58,room_seed);
 if room_lit<=0.001 {return vec3<f32>(0.);}
 // Refract incoming ray (-view) slightly into the room interior (IOR ~ 1.15 effective pane bend).
 let n_out=normalize(vec3<f32>(geometric.x,0.,geometric.z)+vec3<f32>(1e-5,0.,0.));
 let ray=normalize(mix(-view,-n_out,0.18));
 // Local room coordinates: u in [-0.5, 0.5] along facade, v in [0, 1] floor-to-ceiling, d in [0, 1] depth inward.
 let tangent=vec3<f32>(-n_out.z,0.,n_out.x);
 let u0=fract(dot(p,tangent)/3.2)-0.5;
 let v0=fract(p.y/2.65);
 let ru=dot(ray,tangent)/3.2;
 let rv=ray.y/2.65;
 let rd=max(dot(ray,-n_out)/3.4,0.08);
 let t_back=1.0/rd;
 let t_side=select((-0.5-u0)/min(ru,-1e-4),(0.5-u0)/max(ru,1e-4),ru>=0.);
 let t_vert=select(-v0/min(rv,-1e-4),(1.0-v0)/max(rv,1e-4),rv>=0.);
 let t_hit=min(t_back,min(t_side,t_vert));
 let hit=vec3<f32>(u0+ru*t_hit,clamp(v0+rv*t_hit,0.,1.),clamp(rd*t_hit,0.,1.));
 // Room surface albedo by intersected plane (ceiling, hardwood floor, back wall with art/bookshelf, side walls).
 var wall_col=mix(vec3<f32>(0.78,0.72,0.62),vec3<f32>(0.66,0.72,0.74),fract(room_seed*7.3));
 if t_vert<=t_back && t_vert<=t_side {
  if rv>0. {
   let rose=smoothstep(0.22,0.18,length(vec2<f32>(hit.x,hit.z-0.45)));
   wall_col=vec3<f32>(0.92,0.89,0.83)*(1.+0.18*rose);
  } else {
   let plank=0.82+0.18*sin(hit.x*42.+floor(hit.z*8.)*1.7);
   wall_col=vec3<f32>(0.38,0.23,0.12)*plank;
  }
 } else if t_back<=t_side {
  let shelf=step(0.18,abs(hit.x))*step(abs(hit.y-0.48),0.28);
  let stripes=0.7+0.3*sin(hit.y*28.+hit.x*16.);
  wall_col=mix(wall_col,vec3<f32>(0.42,0.26,0.16)*stripes,shelf*0.65);
 }
 // Warm ceiling pendant light at (u = 0, v = 0.82, d = 0.45) with inverse-square radial attenuation.
 let lamp_pos=vec3<f32>(0.0,0.82,0.45);
 let d_lamp= hit-lamp_pos;
 let dist2=dot(d_lamp,d_lamp)+0.09;
 let fixture=smoothstep(0.09,0.02,length(vec2<f32>(hit.x,hit.z-0.45)))*smoothstep(0.72,0.96,hit.y);
 let warm_tint=mix(vec3<f32>(1.55,0.88,0.40),vec3<f32>(1.35,0.96,0.62),fract(room_seed*13.7));
 let illum=warm_tint*(0.28/dist2+fixture*2.2)*smoothstep(1.15,0.15,hit.z);
 // Sheer side curtains near the window glass plane (u0 near +-0.5).
 let curtain_edge=smoothstep(0.24,0.46,abs(u0));
 let folds=0.75+0.25*sin(u0*58.+room_seed*12.);
 let room_radiance=mix(wall_col*illum,warm_tint*0.48*folds,curtain_edge*0.55);
 return room_radiance*room_lit;
}
// 3D world-space volumetric cloud shadow projection onto scene surfaces (UE5 / Unity HDRP Cloud Layer):
// intersects the sun ray from `p` with the mid-cloud slab and evaluates wind-advected cloud optical depth.
fn cloud_sun_transmittance(p:vec3<f32>)->f32 {
 if camera.weather.x<=0.01 || camera.sun.y<=0.02 {return 1.;}
 let t=camera.atmosphere.x;
 let travel=t+camera.wind.z*(0.65*(1.-cos(0.7*t))/0.7+0.35*(1.-cos(1.9*t))/1.9);
 let drift=vec3<f32>(camera.wind.x,0.,camera.wind.y)*travel;
 let slab_y=camera.atmosphere.y+camera.atmosphere.z*0.35;
 let ray_t=max((slab_y-p.y)/max(camera.sun.y,0.05),0.);
 let cp=p+camera.sun.xyz*ray_t;
 let q=(cp-drift)*0.006;
 let n=original_noise(q+vec3<f32>(0.,t*0.003,0.))*0.65
  +original_noise(q*2.1+vec3<f32>(t*0.002,-t*0.009,0.))*0.25;
 let d=max(n-(1.-camera.weather.x),0.)*camera.weather.y;
 return clamp(exp(-d*1.85),0.32,1.);
}
fn shade(f:Fragment,front:bool,tex:vec4<f32>)->vec4<f32>{
 let surface=archive_surface(f,front,tex);
 let base=surface.base;let opacity=surface.opacity;let uv=surface.uv;let spec=surface.spec;
 let f0=surface.f0;let roughness=surface.roughness;let diffuse=surface.diffuse;
 let normal=surface.normal;let view=surface.view;let geometric=surface.geometric;let splash=surface.splash;
 let transmission=material.surface.z;
 var early_bounce=vec4<f32>(0.);
 if ARCHIVE_COMPUTE_GI {
  let pixel=vec2<i32>(f.clip.xy);
  if ARCHIVE_COMPUTE_GI_EXACT_SAMPLE {
   // The original inline ray supplies one pixel's radiance and coverage.
   // A stage crossover must not also introduce a different image filter.
   early_bounce=textureLoad(computed_gi,pixel,0);
  }else{
   early_bounce=load_computed_gi_filtered(pixel,f.clip.z);
  }
 }else if ARCHIVE_EARLY_GI&&transmission<0.9 {early_bounce=screen_gi(f.clip.xy,f.world,normal);}
 let z=material.workflow.z;
 let nl_sun=dot(normal,camera.sun.xyz);
 let translucent_sun=z>5.5&&((z>5.5&&z<6.5)||(z>9.5&&z<11.5)||z>14.5);
 var visibility=0.;
 if nl_sun>0.||translucent_sun {
  visibility=1.;
  let shadow_pos=f.shadow.xyz/f.shadow.w;let shadow_uv=shadow_pos.xy*vec2<f32>(0.5,-0.5)+0.5;
  let shadow_ign=fract(52.9829189*fract(dot(f.clip.xy,vec2<f32>(0.06711056,0.00583715))));
  if all(shadow_uv>=vec2<f32>(0.))&&all(shadow_uv<=vec2<f32>(1.))&&shadow_pos.z>=0.&&shadow_pos.z<=1.&&!fx_off(8u){visibility=soft_sun_visibility(camera.shadow,shadow_uv,shadow_pos.z,shadow_ign);}
  if visibility>0.05&&nl_sun>0.&&material.surface.z<0.9 {
   visibility*=contact_shadow(f.world+geometric*0.015,camera.sun.xyz,shadow_ign);
   if z>0.5 {visibility*=pom_self_shadow(f.world,geometric,camera.sun.xyz,z);}
  }
  visibility*=cloud_sun_transmittance(f.world);
 }
 var ao=1.;var bent_normal=normal;
 if transmission<0.9 {
  let gtao=contact_occlusion(f.clip.xy,f.world,normal);
  bent_normal=gtao.xyz;
  ao=gtao.w*select(1.,spec.r,material.workflow.w!=0.);
 }
 let dust_mask=smoothstep(0.65,0.95,geometric.y)*(1.-ao)*(1.-camera.surface.x)*select(1.,0.,transmission>0.||(z>8.5&&z<10.5));
 let rough_eff=mix(roughness,max(roughness,0.88),dust_mask*0.35);
 let diffuse_eff=mix(diffuse,vec3<f32>(0.54,0.52,0.48),dust_mask*0.14);
 let alpha=max(rough_eff*rough_eff,0.0001);let a2=alpha*alpha;let nv=max(dot(normal,view),0.0001);let dfg=textureSampleLevel(environment_dfg,environment_sampler,vec2<f32>(nv,rough_eff),0.).rg;let shading=Brdf(normal,view,diffuse_eff*(1.-transmission)/3.14159265,f0,a2,nv,sqrt(nv*nv*(1.-a2)+a2),energy_compensation(f0,dfg));var colour=vec3<f32>(0.);if visibility>0.&&nl_sun>0.{
  colour=brdf(shading,camera.sun.xyz)*camera.sun_colour.rgb*camera.sun_colour.a*visibility*micro_shadow(nl_sun,ao);
  if (z>0.5&&z<1.5)||(z>4.5&&z<5.5)||(z>11.5&&z<12.5) {
   let h_sun=normalize(view+camera.sun.xyz);
   let glint_cell=floor(f.world*85.+h_sun*14.);
   let glint=smoothstep(0.985,0.999,original_hash(glint_cell))*pow(max(dot(normal,h_sun),0.),12.);
   colour+=camera.sun_colour.rgb*camera.sun_colour.a*visibility*glint*0.045;
  }
 }
 let halo_weight=clamp(smoothstep(0.25,-0.05,camera.sun.y)*0.032+camera.weather.z*0.024,0.,0.05);
 let d2_view=dot(f.world-camera.eye.xyz,f.world-camera.eye.xyz);
 let n_bend=rough_eff*rough_eff*(1.-rough_eff);
 let reflected=reflect(-view,normalize(mix(normal,view,n_bend)));
 let bent_cone=clamp(dot(reflected,bent_normal)*0.5+0.5,0.35,1.);
 var pt_diffuse=vec3<f32>(0.);
 var candidates=vec4<u32>(0xffffffffu);
 if ARCHIVE_LIGHT_Z_MASKS&&f.world.z>=-512.&&f.world.z<512. {
  candidates=lights.z_masks[u32(i32(floor(f.world.z*0.25))+128)];
 }
 var next_light=0u;
 var word=0u;
 loop {
  var i=next_light;
  if ARCHIVE_LIGHT_Z_MASKS {
   while word<4u {
    if candidates[word]!=0u {break;}
    word+=1u;
   }
   if word==4u {break;}
   let bit=firstTrailingBit(candidates[word]);
   i=word*32u+bit;
   candidates[word]&=candidates[word]-1u;
  }
  if i>=lights.count.x {break;}
  next_light=i+1u;
  let light=lights.values[i];if light.colour_intensity.a==0.{continue;}let r=light.position_kind.w;if r>0.&&abs(light.position_kind.z-f.world.z)>=r{continue;}let offset=light.position_kind.xyz-f.world;if dot(normal,offset)<=0.{continue;}let distance2=max(dot(offset,offset),0.0001);if r>0.&&distance2>=r*r{continue;}var attenuation=1./distance2;if r>0.{let d2_r2=distance2/(r*r);attenuation*=square(clamp(1.-d2_r2*d2_r2,0.,1.));}let l_dir=offset*inverseSqrt(distance2);let l_occ=point_light_occlusion(f.world+geometric*0.02,light.position_kind.xyz,d2_view,distance2);let warm_flicker=select(1.,1.+0.045*sin(camera.atmosphere.x*9.7+f32(i)*2.3)*sin(camera.atmosphere.x*19.3+f32(i)*1.1),light.colour_intensity.r>light.colour_intensity.b*1.45);let l_rad=light.colour_intensity.rgb*light.colour_intensity.a*attenuation*l_occ*warm_flicker;let spec_dir=area_light_spec_dir(f.world,light.position_kind.xyz,reflected,r);let diff_l=shading.diffuse*max(dot(normal,l_dir),0.);let spec_l=(brdf(shading,spec_dir)-shading.diffuse*max(dot(normal,spec_dir),0.))*max(dot(normal,l_dir),0.)/max(dot(normal,spec_dir),0.05);colour+=(diff_l+max(spec_l,vec3<f32>(0.)))*l_rad*micro_shadow(dot(normal,l_dir),ao);pt_diffuse+=l_rad;}
 if halo_weight>0. {colour+=diffuse_eff*(1.-transmission)*pt_diffuse*halo_weight*ao;}
 // Directional sky, contact visibility and view-dependent environment Fresnel.
 if camera.eye.w>0.5 {
  let irradiance=textureSampleLevel(environment_diffuse,environment_sampler,bent_normal,0.).rgb*environment_intensity.x;
  let horizon=smoothstep(-0.15,0.05,dot(reflected,geometric))*mix(1.,bent_cone,1.-ao);
  var radiance=textureSampleLevel(environment_specular,environment_sampler,reflected,rough_eff*f32(textureNumLevels(environment_specular)-1u)).rgb*environment_intensity.x;
  // Skip tracing where Fresnel makes the scene term invisible (steep dielectric views).
  let visible=max(max(f0.r,f0.g),f0.b)+fifth(1.-nv);
  let mirror=select((1.-smoothstep(0.12,0.48,rough_eff))*smoothstep(0.045,0.07,visible)*(1.-smoothstep(900.,2304.,d2_view)),1.,transmission>0.);
  if mirror>0.&&!fx_off(2u) {var reflected_scene=vec4<f32>(0.);if ARCHIVE_COMPUTE_SSR {reflected_scene=textureLoad(computed_ssr,vec2<i32>(f.clip.xy),0);}else{reflected_scene=screen_reflection(f.world+normal*0.03,reflected,transmission==0.,select(54.,28.,d2_view>576.),select(1.08,1.35,d2_view>576.),select(7u,5u,d2_view>576.),ARCHIVE_RAY_BOUNDS);}let filtered_ssr=filter_rough_ssr(reflected_scene,rough_eff,radiance);radiance=mix(radiance,filtered_ssr.rgb,filtered_ssr.a*mirror);}
  var bounce=early_bounce;if !ARCHIVE_EARLY_GI&&!ARCHIVE_COMPUTE_GI&&transmission<0.9 {bounce=screen_gi(f.clip.xy,f.world,normal);}
  let probe=probe_irradiance(f.world,bent_normal);
  let indirect=mix(mix(irradiance,probe.rgb,probe.a),bounce.rgb,bounce.a);
  if transmission>0. {
   let glass_tint=mix(vec3<f32>(1.),vec3<f32>(0.90,0.96,0.95),clamp((1.-nv)*0.75,0.,1.));
   radiance*=glass_tint;
  }
  colour+=diffuse_eff*(1.-transmission)*indirect*multibounce_ao(ao,diffuse_eff)+multiscatter_specular_ibl(f0,dfg,radiance,irradiance)*specular_occlusion(nv,ao,rough_eff)*horizon;
 } else {
  let sky=mix(vec3<f32>(110.,95.,75.),vec3<f32>(170.,230.,330.),clamp(bent_normal.y*0.5+0.5,0.,1.))*camera.viewport.w/3.14159265;
  var bounce=early_bounce;if !ARCHIVE_EARLY_GI&&!ARCHIVE_COMPUTE_GI&&transmission<0.9 {bounce=screen_gi(f.clip.xy,f.world,normal);}
  let probe=probe_irradiance(f.world,bent_normal);
  let indirect=mix(mix(sky,probe.rgb,probe.a),bounce.rgb,bounce.a);
  let horizon=smoothstep(-0.15,0.05,dot(reflected,geometric))*mix(1.,bent_cone,1.-ao);
  var incoming=daylight(reflected,camera.sun.xyz)*(1.-0.7*rough_eff);
  let visible=max(max(f0.r,f0.g),f0.b)+fifth(1.-nv);
  let mirror=select((1.-smoothstep(0.12,0.48,rough_eff))*smoothstep(0.045,0.07,visible)*(1.-smoothstep(900.,2304.,d2_view)),1.,transmission>0.);
  if mirror>0.&&!fx_off(2u) {var reflected_scene=vec4<f32>(0.);if ARCHIVE_COMPUTE_SSR {reflected_scene=textureLoad(computed_ssr,vec2<i32>(f.clip.xy),0);}else{reflected_scene=screen_reflection(f.world+normal*0.03,reflected,transmission==0.,select(54.,28.,d2_view>576.),select(1.08,1.35,d2_view>576.),select(7u,5u,d2_view>576.),ARCHIVE_RAY_BOUNDS);}let filtered_ssr=filter_rough_ssr(reflected_scene,rough_eff,incoming);incoming=mix(incoming,filtered_ssr.rgb,filtered_ssr.a*mirror);}
  if transmission>0. {
   let glass_tint=mix(vec3<f32>(1.),vec3<f32>(0.90,0.96,0.95),clamp((1.-nv)*0.75,0.,1.));
   incoming*=glass_tint;
  }
  colour+=diffuse_eff*(1.-transmission)*indirect*multibounce_ao(ao,diffuse_eff)+multiscatter_specular_ibl(f0,dfg,incoming,sky)*specular_occlusion(nv,ao,rough_eff)*horizon;
 }
 if splash>0. {
  // Water beads scatter the overcast sky; they sit above the ground and ignore its AO.
  var sky_light=mix(vec3<f32>(110.,95.,75.),vec3<f32>(170.,230.,330.),0.75)*camera.viewport.w;
  if camera.eye.w>0.5 {sky_light=textureSampleLevel(environment_diffuse,environment_sampler,vec3<f32>(0.,1.,0.),0.).rgb*environment_intensity.x;}
  colour+=sky_light*min(splash,1.)*0.6;
 }
 if z>0.5 {
  if z>8.5 {
   if z>9.5&&z<10.5 {
    let tip=smoothstep(0.35,0.88,original_noise(vec3<f32>(f.world.x*32.,f.world.y*6.,f.world.z*32.)));
    let graze=pow(1.-nv,2.);
    colour+=base*vec3<f32>(1.2,1.15,0.65)*camera.sun_colour.rgb*camera.sun_colour.a*(0.06+0.12*graze)*tip*visibility;
   } else if z>10.5&&z<11.5 {
    let nl_sun=max(dot(normal,camera.sun.xyz),0.);
    let nh_sun=max(dot(normal,normalize(view+camera.sun.xyz)),0.);
    let sheen=charlie_sheen(nh_sun,nv,max(nl_sun,0.15),roughness)*0.09+pow(1.-nv,3.5)*0.06;
    colour+=sqrt(max(base,vec3<f32>(0.01)))*sheen*camera.sun_colour.rgb*camera.sun_colour.a*max(nl_sun,0.15)*visibility;
   } else if z>14.5 {
    let nl_sun=max(dot(normal,camera.sun.xyz),0.);
    let nh_sun=max(dot(normal,normalize(view+camera.sun.xyz)),0.);
    let velvet=charlie_sheen(nh_sun,nv,max(nl_sun,0.15),0.65)*0.06;
    let sss=max(dot(-normal,camera.sun.xyz)*0.5+0.5,0.)*pow(1.-nv,2.);
    colour+=base*(vec3<f32>(1.18,1.06,0.92)*sss*0.09+velvet)*camera.sun_colour.rgb*camera.sun_colour.a*visibility;
   } else if z>13.5&&z<14.5 {
    let fc=0.04+0.96*fifth(1.-max(dot(geometric,view),0.));
    let r_coat=reflect(-view,normalize(mix(normal,geometric,0.65)));
    var coat_env=daylight(r_coat,camera.sun.xyz);
    if camera.eye.w>0.5 {coat_env=textureSampleLevel(environment_specular,environment_sampler,r_coat,1.2).rgb*environment_intensity.x;}
    colour+=coat_env*fc*0.45*ao*smoothstep(-0.1,0.05,dot(r_coat,geometric));
   }
  } else if z>5.5&&z<6.5 {
   let veins=1.-0.18*exp(-abs(f.uv.x-0.5)*130.);
   let pocket=0.78+0.28*original_noise(f.world*8.5);
   colour*=veins*pocket;
   let trans=max(dot(-normal,camera.sun.xyz)*0.7+0.3,0.);
   let forward=pow(clamp(dot(-view,camera.sun.xyz),0.,1.),3.);
   colour+=base*vec3<f32>(1.15,1.25,0.65)*camera.sun_colour.rgb*camera.sun_colour.a*(trans*0.18+forward*0.16)*visibility;
   if camera.surface.x>0. {
    let wet_fc=0.04+0.96*fifth(1.-nv);
    let r_leaf=reflect(-view,normal);
    var leaf_env=daylight(r_leaf,camera.sun.xyz);
    if camera.eye.w>0.5 {leaf_env=textureSampleLevel(environment_specular,environment_sampler,r_leaf,1.0).rgb*environment_intensity.x;}
    colour+=leaf_env*wet_fc*camera.surface.x*0.32*ao;
   }
  } else if z>4.5&&z<5.5 {
   let film_nm=260.+180.*original_noise(f.world*2.4);
   let irid=thin_film_iridescence(nv,film_nm);
   colour*=mix(vec3<f32>(1.),irid,0.08*smoothstep(0.2,0.85,1.-nv));
   let aniso_t=normalize(cross(normal,vec3<f32>(0.,1.,0.))+vec3<f32>(1e-4,0.,0.));
   let th=dot(aniso_t,normalize(view+camera.sun.xyz));
   let streak=pow(max(1.-th*th,0.),14.)*max(nl_sun,0.)*visibility;
   colour+=f0*camera.sun_colour.rgb*camera.sun_colour.a*streak*0.09;
  } else if z>3.5&&z<4.5 {
   let fc=0.04+0.96*fifth(1.-max(dot(geometric,view),0.));
   let r_coat=reflect(-view,normalize(mix(normal,geometric,0.65)));
   var coat_env=daylight(r_coat,camera.sun.xyz);
   if camera.eye.w>0.5 {coat_env=textureSampleLevel(environment_specular,environment_sampler,r_coat,1.2).rgb*environment_intensity.x;}
   let wood_t=normalize(cross(normal,vec3<f32>(1.,0.,0.))+vec3<f32>(0.,1e-4,0.));
   let th_w=dot(wood_t,normalize(view+camera.sun.xyz));
   let fiber_glint=pow(max(1.-th_w*th_w,0.),10.)*max(nl_sun,0.)*visibility*0.05;
   colour+=coat_env*fc*0.18*ao*smoothstep(-0.1,0.05,dot(r_coat,geometric))+base*camera.sun_colour.rgb*camera.sun_colour.a*fiber_glint;
  }
 } else if transmission>0. {
  let film_nm=320.+140.*original_noise(f.world*0.85);
  let irid=thin_film_iridescence(nv,film_nm);
  colour*=mix(vec3<f32>(1.),irid,0.045*smoothstep(0.25,0.9,1.-nv));
  let dusk_window=smoothstep(0.28,-0.06,camera.sun.y)*smoothstep(0.95,0.2,abs(geometric.y))*smoothstep(86.,76.,f.world.z);
  if dusk_window>0. {
   let warm_room=interior_mapping_window(f.world,geometric,view);
   colour+=warm_room*dusk_window*(1.-f0.r)*camera.viewport.w*420.;
  }
 }
 if any(material.emissive_normal.rgb!=vec3<f32>(0.)){
  let day_suppress=mix(1.,0.16+0.84*fifth(nv),smoothstep(0.05,0.35,camera.sun.y));
  colour+=textureSample(emissive_map,material_sampler,uv).rgb*material.emissive_normal.rgb*day_suppress;
 }
 if transmission>0.{
  // Thin panes do not displace a straight view; only normal-map relief does, with 3-channel RGB dispersion.
  let disp=normal-geometric;
  let bent_r=camera.vp*vec4<f32>(f.world+disp*0.10,1.);
  let bent_g=camera.vp*vec4<f32>(f.world+disp*0.12,1.);
  let bent_b=camera.vp*vec4<f32>(f.world+disp*0.14,1.);
  let base_px=vec2<i32>(f.clip.xy);
  let max_px=vec2<i32>(camera.viewport.xy)-1;
  var px_r=base_px;var px_g=base_px;var px_b=base_px;
  if bent_r.w>0. {let c=clamp(vec2<i32>((bent_r.xy/bent_r.w*vec2<f32>(0.5,-0.5)+0.5)*camera.viewport.xy),vec2<i32>(0),max_px);if textureLoad(scene_depth,c,0)>f.clip.z {px_r=c;}}
  if bent_g.w>0. {let c=clamp(vec2<i32>((bent_g.xy/bent_g.w*vec2<f32>(0.5,-0.5)+0.5)*camera.viewport.xy),vec2<i32>(0),max_px);if textureLoad(scene_depth,c,0)>f.clip.z {px_g=c;}}
  if bent_b.w>0. {let c=clamp(vec2<i32>((bent_b.xy/bent_b.w*vec2<f32>(0.5,-0.5)+0.5)*camera.viewport.xy),vec2<i32>(0),max_px);if textureLoad(scene_depth,c,0)>f.clip.z {px_b=c;}}
  let behind=vec3<f32>(textureLoad(background,px_r,0).r,textureLoad(background,px_g,0).g,textureLoad(background,px_b,0).b);
  let nv=max(dot(normal,view),0.);let fresnel=f0+(1.-f0)*fifth(1.-nv);let path_tint=exp(-vec3<f32>(0.09,0.025,0.035)/max(nv,0.18));colour+=behind/camera.viewport.z*base*path_tint*transmission*(1.-fresnel);
 }
 return vec4<f32>(colour*camera.viewport.z,select(1.,opacity,material.surface.w == -2.));
}

@fragment fn fs_opaque(f:Fragment,@builtin(front_facing) front:bool)->@location(0) vec4<f32>{return shade(f,front,textureSample(diffuse_map,material_sampler,surface_uv(f)));}
@fragment fn fs_main(f:Fragment,@builtin(front_facing) front:bool)->@location(0) vec4<f32>{let tex=textureSample(diffuse_map,material_sampler,surface_uv(f));if material.surface.w>=0.&&tex.a*material.diffuse.a<material.surface.w{discard;}return shade(f,front,tex);}

// Full pixel coverage, with paired coordinates through history reprojection.
fn screen_reflection_paired(segment:DdaProjectedSegment,history:bool)->vec4<f32> {
 if segment.state!=1u {return vec4<f32>(0.);}
 let hit=trace_depth_dda_paired(scene_depth,DdaVectorPair(segment.begin,segment.begin_low),DdaVectorPair(segment.end,segment.end_low));
 return screen_reflection_paired_hit(segment,history,hit);
}
fn screen_reflection_paired_hit(segment:DdaProjectedSegment,history:bool,hit:DepthDdaPairHit)->vec4<f32>{
 if hit.pixel.x<0 {return vec4<f32>(0.);}
 let world=dda_reconstruct_world_paired(segment,hit.parameter);
 let parameter=dda_reconstruct_parameter_paired(segment,hit.parameter);
 return archive_world_history_radiance(world,parameter,hit.pixel,history);
}
// Shared radiance reconstruction for projected-depth and original-source hits.
// Keep world high/low parts through current and previous-camera reprojection.
fn archive_world_history_radiance(world:DdaVectorPair,parameter:vec2<f32>,pixel:vec2<i32>,history:bool)->vec4<f32>{
 let current=dda_project_world_pair(camera.vp,world);
 let coord=dda_clip_ndc(current).xy*vec2<f32>(0.5,-0.5)+0.5;
 var edge=min(min(coord.x,coord.y),min(1.-coord.x,1.-coord.y));
 var source=pixel;
 if history {
  let old=dda_project_world_pair(camera.previous_vp,world);
  if precise_le(vec2<f32>(old.high.w,old.low.w),vec2<f32>(0.)) {return vec4<f32>(0.);}
  let old_uv=dda_clip_ndc(old).xy*vec2<f32>(0.5,-0.5)+0.5;
  edge=min(edge,min(min(old_uv.x,old_uv.y),min(1.-old_uv.x,1.-old_uv.y)));
  if edge<=0. {return vec4<f32>(0.);}
  source=clamp(vec2<i32>(old_uv*camera.viewport.xy),vec2<i32>(0),vec2<i32>(camera.viewport.xy)-1);
  let stored_depth=textureLoad(opaque_history_depth,source,0);
  if stored_depth>=1. {return vec4<f32>(0.);}
  let old_ndc=(vec2<f32>(source)+0.5)/camera.viewport.xy*vec2<f32>(2.,-2.)+vec2<f32>(-1.,1.);
  let recovered=dda_project_endpoint(camera.previous_inverse_vp,vec4<f32>(old_ndc,stored_depth,1.));
  if recovered.high.w==0. {return vec4<f32>(0.);}
  if !archive_paired_history_matches(world,recovered,vec2<f32>(old.high.w,old.low.w),camera.surface.w) {return vec4<f32>(0.);}
 }
 let fade=smoothstep(0.,0.04,edge)*(1.-smoothstep(0.75,1.,parameter.x+parameter.y));
 return vec4<f32>(textureLoad(background,source,0).rgb/camera.viewport.z,fade);
}

// Retain world residuals even when deciding history admission. The threshold
// follows archive_history_surface_matches, with compensated arithmetic.
fn archive_paired_history_matches(point:DdaVectorPair,recovered:DdaVectorPair,clip_w:vec2<f32>,angular:f32)->bool {
 if !dda_finite4(point.high)||!dda_finite4(point.low)||!dda_finite4(recovered.high)||!dda_finite4(recovered.low)||!dda_finite4(vec4<f32>(clip_w,angular,0.))||precise_le(clip_w,vec2<f32>(0.)) {return false;}
 let w=vec2<f32>(recovered.high.w,recovered.low.w);if precise_equal(w,vec2<f32>(0.)){return false;}
 var squared=vec2<f32>(0.);
 for(var axis=0u;axis<3u;axis++){
  let actual=projection_pair_div(vec2<f32>(recovered.high[axis],recovered.low[axis]),w);
  let delta=projection_pair_add(actual,-vec2<f32>(point.high[axis],point.low[axis]));
  squared=projection_pair_add(squared,projection_pair_mul(delta,delta));
 }
 let tolerance=projection_pair_add(vec2<f32>(0.03,0.),projection_pair_mul(vec2<f32>(2.*max(angular,0.),0.),clip_w));
 return precise_le(squared,projection_pair_mul(tolerance,tolerance));
}

fn dda_tile_range(tile:vec2<i32>,level:i32)->vec2<f32>{return textureLoad(ray_depth_bounds,tile,level).xy;}
fn dda_tile_levels()->u32{return textureNumLevels(ray_depth_bounds);}
fn hierarchy_candidate_levels()->u32 {if ARCHIVE_STACK_PIXEL_LEAVES {return textureNumLevels(ray_pixel_depth_bounds);}return dda_tile_levels();}
fn hierarchy_candidate_leaf_extent()->i32 {return select(8,1,ARCHIVE_STACK_PIXEL_LEAVES);}
fn hierarchy_candidate_range(tile:vec2<i32>,level:i32)->vec2<f32> {if ARCHIVE_STACK_PIXEL_LEAVES {return textureLoad(ray_pixel_depth_bounds,tile,level).xy;}return dda_tile_range(tile,level);}


// Same reflection admission as the authored shader, without new ray thresholds.
fn archive_surface_mirror(surface:ArchiveSurface)->f32 {
 let nv=max(dot(surface.normal,surface.view),0.0001);
 if camera.eye.w>0.5 {let visible=max(max(surface.f0.r,surface.f0.g),surface.f0.b)+fifth(1.-nv);return select((1.-smoothstep(0.1,0.4,surface.roughness))*smoothstep(0.045,0.07,visible),1.,material.surface.z>0.);}
 let fresnel=surface.f0+(1.-surface.f0)*fifth(1.-nv);
 return (1.-smoothstep(0.1,0.4,surface.roughness))*smoothstep(0.045,0.07,max(max(fresnel.r,fresnel.g),fresnel.b));
}
// Full precision ray inputs from the same authored surface evaluation.
struct ArchiveGiGuides {@location(0) world:vec4<f32>,@location(1) normal:vec4<f32>}
@fragment fn fs_gi_guides(f:Fragment,@builtin(front_facing) front:bool)->ArchiveGiGuides {
 let tex=textureSample(diffuse_map,material_sampler,surface_uv(f));
 if material.surface.w>=0.&&tex.a*material.diffuse.a<material.surface.w {discard;}
 let surface=archive_surface(f,front,tex);
 var mirror=0.;if ARCHIVE_COMPUTE_SSR {mirror=archive_surface_mirror(surface);}
 return ArchiveGiGuides(vec4<f32>(f.world,mirror),vec4<f32>(surface.normal,select(-1.,1.,material.surface.z==0.)));
}
@compute @workgroup_size(8,8) fn cs_opaque_gi(@builtin(global_invocation_id) id:vec3<u32>) {
 let size=textureDimensions(gi_world);if any(id.xy>=size){return;}
 let pixel=vec2<i32>(id.xy);let world=textureLoad(gi_world,pixel,0);
 var bounce=vec4<f32>(0.);
 let normal=textureLoad(gi_normal,pixel,0);
 if normal.w!=0. {
  if ARCHIVE_TRACE_SSR {if world.w>0.&&!fx_off(2u){let view=normalize(camera.eye.xyz-world.xyz);bounce=screen_reflection(world.xyz+normal.xyz*0.03,reflect(-view,normal.xyz),normal.w>0.,60.,1.,8u,ARCHIVE_RAY_BOUNDS);}}
  else{bounce=screen_gi(vec2<f32>(id.xy)+0.5,world.xyz,normal.xyz);}
 }
 textureStore(gi_output,pixel,bounce);
}

// Qualification-only raw float32 GI export, before native HDR rounding.
@fragment fn fs_gi_reference(f:Fragment,@builtin(front_facing) front:bool)->@location(0) vec4<f32> {
 let tex=textureSample(diffuse_map,material_sampler,surface_uv(f));
 if material.surface.w>=0.&&tex.a*material.diffuse.a<material.surface.w {discard;}
 let surface=archive_surface(f,front,tex);
 return screen_gi(f.clip.xy,f.world,surface.normal);
}
@fragment fn fs_ssr_reference(f:Fragment,@builtin(front_facing) front:bool)->@location(0) vec4<f32> {
 let tex=textureSample(diffuse_map,material_sampler,surface_uv(f));
 if material.surface.w>=0.&&tex.a*material.diffuse.a<material.surface.w {discard;}
 let surface=archive_surface(f,front,tex);
 var ssr=vec4<f32>(0.);
 if archive_surface_mirror(surface)>0.&&!fx_off(2u){ssr=screen_reflection(f.world+surface.normal*0.03,reflect(-surface.view,surface.normal),material.surface.z==0.,60.,1.,8u,ARCHIVE_RAY_BOUNDS);}
 return ssr;
}
