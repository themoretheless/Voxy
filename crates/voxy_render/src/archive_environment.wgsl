// Shared analytic daylight radiance (linear working RGB). Both the visible
// environment and specular lighting use this function; this is not baked GI.
fn daylight(direction:vec3<f32>,sun:vec3<f32>)->vec3<f32>{
 let d=normalize(direction);let elevation=max(d.y,0.);
 let horizon=vec3<f32>(720.,800.,850.);let zenith=vec3<f32>(170.,350.,680.);
 let sky=mix(horizon,zenith,pow(elevation,0.42));
 let cosine=clamp(dot(d,sun),-1.,1.);
 let aureole=vec3<f32>(750.,630.,460.)*pow(max(cosine,0.),48.);
 let disk=vec3<f32>(200000.,185000.,150000.)*smoothstep(0.999975,0.99999,cosine);
 return select(vec3<f32>(150.,130.,105.),sky+aureole+disk,d.y>=0.);
}
