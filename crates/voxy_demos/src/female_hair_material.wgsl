// Directional fibre lighting. UV.y is arc position along the guide.
struct Transform { mvp: mat4x4<f32>, previous_mvp: mat4x4<f32>, camera: vec4<f32> }
@group(0) @binding(0) var<uniform> transform: Transform;
struct Output {
 @builtin(position) position: vec4<f32>,
 @location(0) color: vec4<f32>,
 @location(1) point: vec3<f32>,
 @location(2) normal: vec3<f32>,
 @location(3) arc: f32,
 @location(4) to_camera: vec3<f32>,
}
fn unit(v: vec3<f32>) -> vec3<f32> { return v * inverseSqrt(max(dot(v,v),1e-12)); }
@vertex fn vs_main(@location(0) position: vec3<f32>, @location(1) uv: vec2<f32>,
 @location(2) color: vec4<f32>, @location(3) normal: vec3<f32>) -> Output {
 var out: Output;
 out.position = transform.mvp*vec4<f32>(position,1.0);
 out.color = color;
 out.point = position;
 out.normal = normal;
 out.arc = uv.y;
 out.to_camera = transform.camera.xyz-position;
 return out;
}
@fragment fn fs_main(in: Output) -> @location(0) vec4<f32> {
 let n = unit(in.normal);
 let gradient = dpdx(in.point)*dpdx(in.arc)+dpdy(in.point)*dpdy(in.arc);
 let projected = gradient-n*dot(gradient,n);
 let fallback = cross(n,vec3<f32>(0.31,0.57,0.76));
 let t = unit(select(fallback,projected,dot(projected,projected)>1e-12));
 let l = unit(vec3<f32>(-0.4,0.7,0.6));
 let v = unit(in.to_camera);
 let h = unit(l+v);
 let primary = unit(t+n*0.12);
 let secondary = unit(t-n*0.22);
 let primary_dot = dot(primary,h);
 let p = max(0.0,1.0-primary_dot*primary_dot);
 let p2=p*p; let p4=p2*p2; let p8=p4*p4; let p16=p8*p8; let p32=p16*p16;
 let secondary_dot = dot(secondary,h);
 let s = max(0.0,1.0-secondary_dot*secondary_dot);
 let s2=s*s; let s4=s2*s2; let s8=s4*s4;
 let light_dot = dot(t,l);
 let diffuse = sqrt(max(0.0,1.0-light_dot*light_dot));
 let facing = max(dot(n,l),0.0);
 let specular = (vec3<f32>(0.12)*(p32*p2*p)
  +vec3<f32>(0.065,0.035,0.018)*(s8*s4))*facing;
 return vec4<f32>(in.color.rgb*(0.28+0.72*diffuse)+specular,in.color.a);
}
