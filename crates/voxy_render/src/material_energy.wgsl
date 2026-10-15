// Shared energy-closure candidate; production pass integration is not yet enabled.
// Corrected Hill 2018 / Kelemen multiple-scattering approximation, not the
// stochastic Smith ground truth. Inputs must come from the same GGX integrator.
// Store lost unit-Fresnel energy directly: subtracting an almost-unit DFG sum
// would discard the small multiple-scattering term near the smooth limit.
struct GgxEnergyMoments {
    view_a: f32, view_loss: f32,
    light_a: f32, light_loss: f32,
    average_a: f32, average_loss: f32,
}
struct LayeredGgxValue { value: vec3<f32>, valid: u32 }
fn energy_moments_valid(e: GgxEnergyMoments) -> bool {
    let a=vec3<f32>(e.view_a,e.light_a,e.average_a);
    let loss=vec3<f32>(e.view_loss,e.light_loss,e.average_loss);
    return all(a>=vec3<f32>(0.0)) && all(loss>=vec3<f32>(0.0))
        && all(a+loss<=vec3<f32>(1.0)) && e.average_loss<1.0;
}
fn normalized_energy_product(a: f32, b: f32, average: f32) -> f32 {
    // Divide before multiplying the small losses; their product may underflow
    // even when the normalized BRDF remains representable.
    return (min(a,b)/average)*(max(a,b)/3.141592653589793);
}
fn layered_ggx_energy(single: vec3<f32>, f0: vec3<f32>, substrate: vec3<f32>, e: GgxEnergyMoments) -> LayeredGgxValue {
    if !energy_moments_valid(e) || !all(f0>=vec3<f32>(0.0)) || !all(f0<=vec3<f32>(1.0))
        || !all(substrate>=vec3<f32>(0.0)) || !all(substrate<=vec3<f32>(1.0))
        || !all(single>=vec3<f32>(0.0)) || !all(single<=vec3<f32>(3.402823466e38)) {
        return LayeredGgxValue(vec3<f32>(0.0),0u);
    }
    let favg=f0+(vec3<f32>(1.0)-f0)/21.0;
    let average_escape=1.0-e.average_loss;
    // The squared Fresnel factor excludes the single bounce already in `single`.
    let multiple_fresnel=favg*favg*average_escape/((vec3<f32>(1.0)-favg)+favg*average_escape);
    let remaining_view=(vec3<f32>(1.0)-f0)*e.view_a+(vec3<f32>(1.0)-multiple_fresnel)*e.view_loss;
    let remaining_light=(vec3<f32>(1.0)-f0)*e.light_a+(vec3<f32>(1.0)-multiple_fresnel)*e.light_loss;
    let remaining_average=(vec3<f32>(1.0)-f0)*e.average_a+(vec3<f32>(1.0)-multiple_fresnel)*e.average_loss;
    var multiple=vec3<f32>(0.0);
    if e.average_loss>0.0 {
        multiple=multiple_fresnel*normalized_energy_product(e.view_loss,e.light_loss,e.average_loss);
    } else if e.view_loss!=0.0 || e.light_loss!=0.0 {
        return LayeredGgxValue(vec3<f32>(0.0),0u);
    }
    var diffuse=vec3<f32>(0.0);
    for(var channel=0u;channel<3u;channel++) {
        if remaining_average[channel]>0.0 {
            diffuse[channel]=substrate[channel]*normalized_energy_product(remaining_view[channel],remaining_light[channel],remaining_average[channel]);
        } else if remaining_view[channel]!=0.0 && remaining_light[channel]!=0.0 {
            return LayeredGgxValue(vec3<f32>(0.0),0u);
        }
    }
    let value=single+multiple+diffuse;
    let valid=all(value>=vec3<f32>(0.0)) && all(value<=vec3<f32>(3.402823466e38));
    return LayeredGgxValue(value,select(0u,1u,valid));
}
