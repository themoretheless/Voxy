// Shared overflow-safe HDR->SDR Reinhard & Karis firefly luminance helpers.
fn luma_bt709(rgb: vec3<f32>) -> f32 {
    return dot(rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn tonemap_reinhard_safe(linear_rgb: vec3<f32>, exposure: f32) -> vec3<f32> {
    let exposed = min(max(linear_rgb, vec3<f32>(0.0)) * exposure, vec3<f32>(3.402823e38));
    let low = exposed / (vec3<f32>(1.0) + exposed);
    let high = vec3<f32>(1.0) / (vec3<f32>(1.0) + vec3<f32>(1.0) / exposed);
    return select(low, high, exposed > vec3<f32>(1.0));
}

fn karis_weight_rgb(linear_rgb: vec3<f32>) -> vec3<f32> {
    let c = max(linear_rgb, vec3<f32>(0.0));
    return c / (1.0 + luma_bt709(c));
}
