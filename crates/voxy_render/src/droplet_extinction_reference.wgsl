@group(0) @binding(0) var<storage, read_write> words: array<u32>;
fn value(i: u32) -> f32 { return bitcast<f32>(words[i]); }
@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let ray = id.x;
    if ray >= words[4] { return; }
    let n = vec3<u32>(words[0], words[1], words[2]);
    let origin = vec3<f32>(value(6u), value(7u), value(8u));
    let spacing = vec3<f32>(value(9u), value(10u), value(11u));
    let base = 12u + words[3] + 8u * ray;
    let start = vec3<f32>(value(base), value(base+1u), value(base+2u));
    let end = vec3<f32>(value(base+3u), value(base+4u), value(base+5u));
    let delta = end-start;
    let distance = length(delta);
    var tau = 0.0;
    for (var cell = 0u; cell < words[3]; cell += 1u) {
        let sigma = value(12u+cell);
        if sigma == 0.0 || distance == 0.0 { continue; }
        let coordinate = vec3<u32>(cell%n.x, (cell/n.x)%n.y, cell/(n.x*n.y));
        let lower = origin + vec3<f32>(coordinate)*spacing;
        let upper = lower+spacing;
        var lo = 0.0; var hi = 1.0;
        for (var axis = 0u; axis < 3u; axis += 1u) {
            if delta[axis] == 0.0 {
                if start[axis] < lower[axis] || start[axis] >= upper[axis] { hi=lo; break; }
            } else {
                let a = (lower[axis]-start[axis])/delta[axis];
                let b = (upper[axis]-start[axis])/delta[axis];
                lo=max(lo,min(a,b)); hi=min(hi,max(a,b));
                if hi <= lo { break; }
            }
        }
        if hi > lo { tau += sigma*distance*(hi-lo); }
    }
    words[base+6u] = bitcast<u32>(tau);
    words[base+7u] = bitcast<u32>(exp(-tau));
}
