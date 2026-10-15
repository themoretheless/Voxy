// Shared GGX microfacet BSDF: height-correlated Smith visibility, Schlick
// Fresnel and multiple-scattering energy compensation from the split-sum
// DFG lookup (rg = scale/bias, so single-scatter albedo Ess = r + g).
fn bsdf_fifth(x:f32)->f32{let s=x*x;return s*s*x;}
struct Brdf {n:vec3<f32>,v:vec3<f32>,diffuse:vec3<f32>,f0:vec3<f32>,a2:f32,nv:f32,smith_v:f32,energy:vec3<f32>}
fn brdf(s:Brdf,l:vec3<f32>)->vec3<f32>{
 let nl=max(dot(s.n,l),0.);if nl==0.{return vec3<f32>(0.);}
 let h=normalize(l+s.v);let nh=max(dot(s.n,h),0.);let vh=max(dot(s.v,h),0.);
 let q=nh*nh*(s.a2-1.)+1.;let d=s.a2/(3.14159265*q*q);
 let gv=nl*s.smith_v;let gl=s.nv*sqrt(nl*nl*(1.-s.a2)+s.a2);let visibility=0.5/max(gv+gl,0.000001);
 let fresnel=s.f0+(1.-s.f0)*bsdf_fifth(1.-vh);return (s.diffuse+d*visibility*fresnel*s.energy)*nl;
}
// Kulla-Conty style scale for analytic lights (Filament 4.7.2).
fn energy_compensation(f0:vec3<f32>,dfg:vec2<f32>)->vec3<f32>{
 let ess=max(dfg.x+dfg.y,0.0001);return 1.+f0*(1./ess-1.);
}
// Fdez-Aguera 2019: single + multiple scattering for image-based lighting.
fn multiscatter_specular_ibl(f0:vec3<f32>,dfg:vec2<f32>,radiance:vec3<f32>,irradiance:vec3<f32>)->vec3<f32>{
 let single=f0*dfg.x+dfg.y;
 let ems=1.-(dfg.x+dfg.y);
 let favg=f0+(1.-f0)/21.;
 let multiple=single*favg/(1.-ems*favg);
 // irradiance is the cosine-convolved environment already divided by pi.
 return single*radiance+multiple*ems*irradiance;
}
// Jimenez 2016: light inter-reflected by bright surroundings brightens AO.
fn multibounce_ao(ao:f32,albedo:vec3<f32>)->vec3<f32>{
 let a=2.0404*albedo-0.3324;let b=-4.7951*albedo+0.6417;let c=2.7552*albedo+0.6903;
 return max(vec3<f32>(ao),((ao*a+b)*ao+c)*ao);
}
// Lagarde & de Rousiers 2014: occlusion of the specular lobe from diffuse AO.
fn specular_occlusion(nv:f32,ao:f32,roughness:f32)->f32{
 return clamp(pow(nv+ao,exp2(-16.*roughness-1.))-1.+ao,0.,1.);
}
