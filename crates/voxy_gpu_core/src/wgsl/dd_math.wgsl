// Shared double-double (unevaluated hi/lo f32 pair) compensated arithmetic for WGSL solvers.
fn dd_rounded_add(a: f32, b: f32) -> f32 { return fma(a, 1.0, b); }
fn dd_rounded_mul(a: f32, b: f32) -> f32 { return fma(a, b, 0.0); }

fn dd_add(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    let s = dd_rounded_add(a.x, b.x);
    let v = dd_rounded_add(s, -a.x);
    let left = dd_rounded_add(a.x, -dd_rounded_add(s, -v));
    let right = dd_rounded_add(b.x, -v);
    let e = dd_rounded_add(dd_rounded_add(dd_rounded_add(left, right), a.y), b.y);
    let hi = dd_rounded_add(s, e);
    return vec2<f32>(hi, dd_rounded_add(e, -dd_rounded_add(hi, -s)));
}

fn dd_neg(a: vec2<f32>) -> vec2<f32> { return -a; }

fn dd_sub(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    return dd_add(a, -b);
}

fn dd_mul(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    let p = dd_rounded_mul(a.x, b.x);
    let error = fma(a.x, b.x, -p);
    let cross = dd_rounded_add(dd_rounded_add(error, dd_rounded_mul(a.x, b.y)), dd_rounded_mul(a.y, b.x));
    let e = dd_rounded_add(cross, dd_rounded_mul(a.y, b.y));
    let hi = dd_rounded_add(p, e);
    return vec2<f32>(hi, dd_rounded_add(e, -dd_rounded_add(hi, -p)));
}

fn dd_div(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    let q = vec2<f32>(a.x / b.x, 0.0);
    let r = dd_sub(a, dd_mul(q, b));
    return dd_add(q, vec2<f32>((r.x + r.y) / b.x, 0.0));
}

fn dd_sqrt(a: vec2<f32>) -> vec2<f32> {
    let x = vec2<f32>(sqrt(a.x), 0.0);
    return dd_add(x, dd_div(dd_sub(a, dd_mul(x, x)), dd_mul(vec2<f32>(2.0, 0.0), x)));
}
