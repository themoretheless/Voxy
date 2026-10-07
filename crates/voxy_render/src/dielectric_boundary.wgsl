// Smooth lossless interface; power fractions, not radiance transport weights.
// Caller admits unit incident/normal, dot(incident,normal)<=0, positive finite
// indices with representable ratio. Normal points into the incident medium.
struct DielectricBoundary {
    reflected: vec3f,
    transmitted: vec3f,
    reflectance: f32,
    transmittance: f32,
    has_transmission: bool,
    incident_over_transmitted_ior: f32,
}
fn dielectric_polarization(a: f32, b: f32) -> vec2f {
    let denominator = a+b;
    let r = (a-b)/denominator;
    return vec2f(r*r,(2.0*a/denominator)*(2.0*b/denominator));
}
fn dielectric_boundary(incident: vec3f, normal: vec3f, ni: f32, nt: f32) -> DielectricBoundary {
    let cosine = clamp(-dot(incident,normal),0.0,1.0);
    let reflected = incident+2.0*cosine*normal;
    if ni == nt {
        return DielectricBoundary(reflected,incident,0.0,1.0,true,1.0);
    }
    let eta = ni/nt;
    var tangent = incident+cosine*normal;
    // Exact opposite vectors have zero tangent. Rounded dot(n,n) otherwise
    // leaves a residual that a large index ratio would magnify into refraction.
    // Do not clamp genuinely oblique rays into this normal-incidence case.
    if all(incident == -normal) { tangent = vec3f(0.0); }
    let sine_t = eta*length(tangent);
    if sine_t >= 1.0 {
        return DielectricBoundary(reflected,vec3f(0.0),1.0,0.0,false,eta);
    }
    let cosine_t = sqrt(max(0.0,1.0-sine_t*sine_t));
    let transmitted = eta*tangent-cosine_t*normal;
    let scale = max(ni,nt);
    let s = dielectric_polarization((ni/scale)*cosine,(nt/scale)*cosine_t);
    let p = dielectric_polarization((nt/scale)*cosine,(ni/scale)*cosine_t);
    let fractions = 0.5*(s+p);
    return DielectricBoundary(reflected,transmitted,fractions.x,fractions.y,true,eta);
}
// Radiance returning along camera branches to the incident-medium viewer.
// The sampled index ratio belongs to the boundary result; these
// deterministic weights include no sampling probabilities or cosine factors.
fn dielectric_camera_radiance_weights(boundary: DielectricBoundary) -> vec2f {
    let eta = boundary.incident_over_transmitted_ior;
    return vec2f(boundary.reflectance,(boundary.transmittance*eta)*eta);
}
fn dielectric_camera_radiance(boundary: DielectricBoundary, reflected: vec3f, transmitted: vec3f) -> vec3f {
    let weights = dielectric_camera_radiance_weights(boundary);
    return weights.x*reflected+weights.y*transmitted;
}
