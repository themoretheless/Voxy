// Character shading terms shared by the photoreal skinned path.

// Pre-integrated subsurface scattering (Penner 2011), analytic form: light
// diffuses further in red than blue, so the terminator wraps per channel and
// narrower (more curved) features scatter more. curvature is 1/radius in 1/m.
fn skin_diffuse(n_dot_l:f32,curvature:f32)->vec3<f32>{
 let scatter=clamp(curvature*0.012,0.,1.);
 let wrap=vec3<f32>(0.60,0.24,0.14)*(0.35+0.65*scatter);
 let lit=clamp((vec3<f32>(n_dot_l)+wrap)/(1.+wrap),vec3<f32>(0.),vec3<f32>(1.));
 // Energy-normalised wrap with a gentle red bleed across the terminator.
 return lit*lit*(1.+wrap*0.5)/(1.+wrap*0.5*0.6667);
}

// Marschner-style fibre scattering (Karis 2016 approximation). tangent runs
// along the strand. R is the surface reflection, TT the forward transmission
// and TRT the tinted internal reflection, each a shifted longitudinal lobe.
fn hair_lobe(sin_sum:f32,shift:f32,beta:f32)->f32{
 let x=sin_sum-shift;return exp(-x*x/(2.*beta*beta))/(beta*2.5066283);
}
fn hair_shading(tangent:vec3<f32>,light:vec3<f32>,view:vec3<f32>,colour:vec3<f32>,roughness:f32)->vec3<f32>{
 let sin_l=clamp(dot(tangent,light),-1.,1.);let sin_v=clamp(dot(tangent,view),-1.,1.);
 let cos_l=sqrt(1.-sin_l*sin_l);
 let lp=normalize(light-tangent*sin_l+vec3<f32>(1e-5));let vp=normalize(view-tangent*sin_v+vec3<f32>(1e-5));
 let cos_phi=dot(lp,vp);
 let beta=max(roughness*roughness,0.02);let shift=0.035;
 let fresnel=0.04+0.96*pow(1.-sqrt(clamp(0.5+0.5*dot(light,view),0.,1.)),5.);
 let r=hair_lobe(sin_l+sin_v,-2.*shift,beta)*0.25*sqrt(clamp(0.5+0.5*cos_phi,0.,1.))*fresnel;
 // Absorption grows with path length through the pigmented fibre.
 let absorb=pow(max(colour,vec3<f32>(0.001)),vec3<f32>(0.5/max(cos_l,0.2)));
 let tt=hair_lobe(sin_l+sin_v,shift,beta*0.5)*absorb*exp(-3.65*cos_phi-3.98)*(1.-fresnel)*(1.-fresnel);
 let trt=hair_lobe(sin_l+sin_v,4.*shift,beta*2.)*absorb*absorb*fresnel*0.6;
 // Kajiya-Kay style multiple-scattering diffuse keeps dark hair from going flat.
 let diffuse=colour*(0.25+0.75*cos_l)/3.14159265;
 return (vec3<f32>(r)+tt+trt)*cos_l+diffuse;
}

// Wet corneal layer: a near-mirror dielectric coat (n = 1.376, F0 ~ 0.025)
// over the diffuse sclera/iris, which loses the light the coat reflects.
fn cornea_coat(n:vec3<f32>,view:vec3<f32>,light:vec3<f32>,base:vec3<f32>)->vec3<f32>{
 let h=normalize(view+light);let nh=max(dot(n,h),0.);let nl=max(dot(n,light),0.);let nv=max(dot(n,view),1e-4);
 let a2=0.0004;let q=nh*nh*(a2-1.)+1.;let d=a2/(3.14159265*q*q);
 let f=0.025+0.975*pow(1.-max(dot(view,h),0.),5.);
 let coat_view=0.025+0.975*pow(1.-nv,5.);
 return base*(1.-coat_view)+vec3<f32>(min(d*f*0.25/nv,60.)*nl);
}
