// Diagnostic dielectric surface lighting; not calibrated skin/subsurface transport.
struct Transform { mvp: mat4x4<f32>, previous_mvp: mat4x4<f32>, camera: vec4<f32> }
@group(0) @binding(0) var<uniform> transform: Transform;
struct Output {
 @builtin(position) position: vec4<f32>,
 @location(0) color: vec4<f32>,
 @location(1) normal: vec3<f32>,
 @location(2) to_camera: vec3<f32>,
 @location(3) reference: vec3<f32>,
 @location(4) parameters: vec4<f32>,
}
fn unit(v: vec3<f32>) -> vec3<f32> { return v*inverseSqrt(max(dot(v,v),1e-12)); }
@vertex fn vs_main(@location(0) position: vec3<f32>, @location(1) uv: vec2<f32>,
 @location(2) color: vec4<f32>, @location(3) normal: vec3<f32>,
 @location(4) parameters: vec4<f32>, @location(5) reference: vec3<f32>) -> Output {
 var out: Output;
 out.position=transform.mvp*vec4<f32>(position,1.0);
 out.reference=reference; out.parameters=parameters;
 out.color=color; out.normal=normal; out.to_camera=transform.camera.xyz-position;
 return out;
}
@fragment fn fs_main(in: Output) -> @location(0) vec4<f32> {
 let n=unit(in.normal); let v=unit(in.to_camera); let l=unit(vec3<f32>(-0.4,0.7,0.6));
 let h=unit(v+l); let nl=max(dot(n,l),0.0); let nv=max(dot(n,v),0.0);
 let nh=max(dot(n,h),0.0); let vh=max(dot(v,h),0.0);
 let roughness=select(0.6,clamp(in.parameters.x,0.05,1.0),in.parameters.x>0.0);
 let alpha=roughness*roughness; let a2=alpha*alpha;
 let denom=nh*nh*(a2-1.0)+1.0;
 let d=a2/(3.14159265*denom*denom);
 let k=(roughness+1.0)*(roughness+1.0)/8.0;
 let g=(nl/(nl*(1.0-k)+k))*(nv/(nv*(1.0-k)+k));
 let one=1.0-vh; let one2=one*one;
 let f=0.04+0.96*one2*one2*one;
 let specular=d*g*f/max(4.0*nl*nv,1e-6);
 // Explicit diagnostic stripes in metre-valued material coordinates.
 let stripe=smoothstep(0.4,0.6,fract(in.reference.y*max(in.parameters.z,1.0)));
 let contrast=select(0.0,clamp(in.parameters.w,0.0,1.0),in.parameters.y>0.5);
 let base=in.color.rgb*(1.0-contrast*stripe);
 let diffuse=base*(1.0-f)/3.14159265;
 return vec4<f32>(base*0.14+(diffuse+vec3<f32>(specular))*nl*3.0,in.color.a);
}
