//! Constant-acceleration sphere sweep: quadratic faces and quartic edge/vertex contacts.
use super::{SphereFilmHit, SurfaceFilm, dot, norm, sub};
fn evaluate(c: &[f64], t: f64) -> f64 {
    c.iter().rev().fold(0.0, |v, c| v * t + c)
}
// Derivative roots partition [0,1] into monotone intervals. This isolates both
// crossings and repeated (tangent) roots without a fixed sampling grid.
fn roots(coefficients: &[f64]) -> Result<Vec<f64>, &'static str> {
    if coefficients.iter().any(|c| !c.is_finite()) {
        return Err("accelerated sphere polynomial overflow");
    }
    let scale = coefficients.iter().fold(0.0_f64, |s, c| s.max(c.abs()));
    if scale == 0.0 {
        return Ok(vec![0.0]);
    }
    let mut c: Vec<_> = coefficients.iter().map(|v| v / scale).collect();
    while c.len() > 1 && c.last() == Some(&0.0) {
        c.pop();
    }
    if c.len() == 1 {
        return Ok(Vec::new());
    }
    if c.len() == 2 {
        let t = -c[0] / c[1];
        return Ok(if t.is_finite() && (0.0..=1.0).contains(&t) {
            vec![t]
        } else {
            Vec::new()
        });
    }
    let derivative: Vec<_> = c
        .iter()
        .enumerate()
        .skip(1)
        .map(|(k, v)| k as f64 * v)
        .collect();
    let mut points = vec![0.0];
    points.extend(roots(&derivative)?);
    points.push(1.0);
    points.sort_by(f64::total_cmp);
    points.dedup();
    let tolerance = 64.0 * f64::EPSILON * c.iter().map(|v| v.abs()).sum::<f64>();
    let mut result = Vec::new();
    for &t in &points {
        if evaluate(&c, t).abs() <= tolerance {
            result.push(t);
        }
    }
    for interval in points.windows(2) {
        let mut lo = interval[0];
        let mut hi = interval[1];
        let mut a = evaluate(&c, lo);
        let b = evaluate(&c, hi);
        if a == 0.0 || b == 0.0 || a.is_sign_negative() == b.is_sign_negative() {
            continue;
        }
        for _ in 0..80 {
            let mid = lo + 0.5 * (hi - lo);
            if mid == lo || mid == hi {
                break;
            }
            let value = evaluate(&c, mid);
            if value == 0.0 {
                lo = mid;
                hi = mid;
                break;
            }
            if value.is_sign_negative() == a.is_sign_negative() {
                lo = mid;
                a = value;
            } else {
                hi = mid;
            }
        }
        result.push(lo + 0.5 * (hi - lo));
    }
    result.sort_by(f64::total_cmp);
    result.dedup_by(|a, b| (*a - *b).abs() < 16.0 * f64::EPSILON);
    Ok(result)
}
fn sphere_roots(w: [f64; 3], d: [f64; 3], q: [f64; 3], r: f64) -> Result<Vec<f64>, &'static str> {
    let scale = w.iter().chain(&d).chain(&q).fold(r, |s, x| s.max(x.abs()));
    let w = w.map(|x| x / scale);
    let d = d.map(|x| x / scale);
    let q = q.map(|x| x / scale);
    let r = r / scale;
    roots(&[
        dot(w, w) - r * r,
        2.0 * dot(w, d),
        dot(d, d) + 2.0 * dot(w, q),
        2.0 * dot(d, q),
        dot(q, q),
    ])
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn inside(p: [f64; 3], tri: &[[f64; 3]; 3], n: [f64; 3]) -> bool {
    (0..3).all(|k| {
        let e = sub(tri[(k + 1) % 3], tri[k]);
        dot(cross(e, sub(p, tri[k])), n) >= -64.0 * f64::EPSILON * dot(e, e)
    })
}
impl SurfaceFilm {
    /// Earliest contact along x(t)=start+velocity*t+acceleration*t²/2, 0<=t<=dt.
    /// Hit time is a fraction of dt. All stationary triangle faces, finite edges
    /// and vertices are tested. Initial overlap/touch and tangent contacts are
    /// included. Substrate geometry only; film height and deformation are absent.
    /// A cell visit and every feature query consume max_feature_checks budget.
    pub fn first_accelerated_sphere_hit(
        &self,
        start: [f64; 3],
        velocity: [f64; 3],
        acceleration: [f64; 3],
        dt: f64,
        radius: f64,
        max_feature_checks: usize,
    ) -> Result<Option<SphereFilmHit>, &'static str> {
        self.accelerated_sphere_hit(
            start,
            velocity,
            acceleration,
            dt,
            radius,
            max_feature_checks,
            false,
        )
    }
    /// Closing contacts only; separating touches are skipped, but deep initial
    /// overlaps remain visible so callers can reject invalid initial geometry.
    pub fn first_closing_accelerated_sphere_hit(
        &self,
        start: [f64; 3],
        velocity: [f64; 3],
        acceleration: [f64; 3],
        dt: f64,
        radius: f64,
        max_feature_checks: usize,
    ) -> Result<Option<SphereFilmHit>, &'static str> {
        self.accelerated_sphere_hit(
            start,
            velocity,
            acceleration,
            dt,
            radius,
            max_feature_checks,
            true,
        )
    }
    fn accelerated_sphere_hit(
        &self,
        start: [f64; 3],
        velocity: [f64; 3],
        acceleration: [f64; 3],
        dt: f64,
        radius: f64,
        max_feature_checks: usize,
        closing_only: bool,
    ) -> Result<Option<SphereFilmHit>, &'static str> {
        if start
            .iter()
            .chain(&velocity)
            .chain(&acceleration)
            .any(|v| !v.is_finite())
            || !dt.is_finite()
            || dt <= 0.0
            || !radius.is_finite()
            || radius <= 0.0
            || max_feature_checks == 0
        {
            return Err("invalid accelerated sphere sweep");
        }
        let d = velocity.map(|v| v * dt);
        let q = acceleration.map(|v| 0.5 * v * dt * dt);
        if d.iter().chain(&q).any(|v| !v.is_finite()) {
            return Err("accelerated sphere trajectory overflow");
        }
        let center = |t: f64| std::array::from_fn::<_, 3, _>(|k| start[k] + t * (d[k] + t * q[k]));
        let end = center(1.0);
        if end.iter().any(|v| !v.is_finite()) {
            return Err("accelerated sphere trajectory overflow");
        }
        let mut lower = std::array::from_fn::<_, 3, _>(|k| start[k].min(end[k]));
        let mut upper = std::array::from_fn::<_, 3, _>(|k| start[k].max(end[k]));
        for k in 0..3 {
            if q[k] != 0.0 {
                let t = -d[k] / (2.0 * q[k]);
                if (0.0..=1.0).contains(&t) {
                    let x = center(t)[k];
                    if !x.is_finite() {
                        return Err("accelerated sphere trajectory overflow");
                    }
                    lower[k] = lower[k].min(x);
                    upper[k] = upper[k].max(x);
                }
            }
        }
        let scale = start
            .iter()
            .chain(&end)
            .chain(&lower)
            .chain(&upper)
            .fold(radius, |s, x| s.max(x.abs()));
        let padding = radius + 128.0 * f64::EPSILON * scale;
        let mut checks = 0usize;
        let mut charge = || -> Result<(), &'static str> {
            checks += 1;
            if checks > max_feature_checks {
                Err("accelerated sphere feature budget")
            } else {
                Ok(())
            }
        };
        let mut nearest: Option<SphereFilmHit> = None;
        for (cell, tri) in self.geometry.iter().enumerate() {
            charge()?;
            if (0..3).any(|k| {
                tri.iter().all(|p| p[k] < lower[k] - padding)
                    || tri.iter().all(|p| p[k] > upper[k] + padding)
            }) {
                continue;
            }
            let n = self.normals[cell];
            let mut consider = |time: f64, point: [f64; 3]| -> Result<(), &'static str> {
                let x = center(time);
                let offset = sub(x, point);
                let distance = norm(offset);
                if x.iter().chain(&point).any(|v| !v.is_finite()) || !distance.is_finite() {
                    return Err("accelerated sphere contact overflow");
                }
                let contact_scale = x.iter().chain(&point).fold(radius, |s, v| s.max(v.abs()));
                // Root arithmetic may identify a near repeated root. Verify actual
                // geometry before accepting it, rather than treating polynomial
                // roundoff tolerance as a physical contact distance.
                if distance > radius + 256.0 * f64::EPSILON * contact_scale
                    || (time > 0.0
                        && (distance - radius).abs() > 256.0 * f64::EPSILON * contact_scale)
                {
                    return Ok(());
                }
                let tangent = std::array::from_fn::<_, 3, _>(|k| d[k] + 2.0 * time * q[k]);
                let normal = if distance > 0.0 {
                    offset.map(|v| v / distance)
                } else if dot(tangent, n) > 0.0 {
                    n.map(|v| -v)
                } else {
                    n
                };
                if time > 0.0 && dot(tangent, normal) > 128.0 * f64::EPSILON * norm(tangent) {
                    return Ok(());
                }
                let penetration = if time == 0.0 {
                    (radius - distance).max(0.0)
                } else {
                    0.0
                };
                if closing_only && penetration <= 64.0 * f64::EPSILON * radius {
                    let normal_speed = dot(tangent, normal);
                    if normal_speed >= 0.0
                        && !(time == 0.0 && normal_speed == 0.0 && dot(q, normal) < 0.0)
                    {
                        return Ok(());
                    }
                }
                if nearest.is_none_or(|old| {
                    time < old.time || (time == old.time && penetration > old.penetration)
                }) {
                    nearest = Some(SphereFilmHit {
                        cell,
                        time,
                        point,
                        normal,
                        penetration,
                    });
                }
                Ok(())
            };
            let signed = dot(sub(start, tri[0]), n);
            let projection = std::array::from_fn(|k| start[k] - signed * n[k]);
            if signed.abs() <= radius && inside(projection, tri, n) {
                consider(0.0, projection)?;
            }
            for side in [-1.0, 1.0] {
                charge()?;
                for t in roots(&[signed - side * radius, dot(d, n), dot(q, n)])? {
                    let x = center(t);
                    let point = std::array::from_fn(|k| x[k] - side * radius * n[k]);
                    let contact_scale = point
                        .iter()
                        .chain(tri.iter().flatten())
                        .fold(radius, |s, v| s.max(v.abs()));
                    if dot(sub(point, tri[0]), n).abs() <= 256.0 * f64::EPSILON * contact_scale
                        && inside(point, tri, n)
                    {
                        consider(t, point)?;
                    }
                }
            }
            for k in 0..3 {
                let a = tri[k];
                let w = sub(start, a);
                charge()?;
                if norm(w) <= radius {
                    consider(0.0, a)?;
                }
                for t in sphere_roots(w, d, q, radius)? {
                    consider(t, a)?;
                }
                let edge = sub(tri[(k + 1) % 3], a);
                let length = norm(edge);
                let unit = edge.map(|v| v / length);
                let along = dot(w, unit);
                let speed = dot(d, unit);
                let accel = dot(q, unit);
                let perpendicular =
                    |v: [f64; 3], s: f64| std::array::from_fn(|k| v[k] - s * unit[k]);
                charge()?;
                let initial = along.clamp(0.0, length);
                let point = std::array::from_fn(|k| a[k] + initial * unit[k]);
                if norm(sub(start, point)) <= radius {
                    consider(0.0, point)?;
                }
                for t in sphere_roots(
                    perpendicular(w, along),
                    perpendicular(d, speed),
                    perpendicular(q, accel),
                    radius,
                )? {
                    let position = along + t * (speed + t * accel);
                    if (0.0..=length).contains(&position) {
                        consider(t, std::array::from_fn(|k| a[k] + position * unit[k]))?;
                    }
                }
            }
        }
        Ok(nearest)
    }
}
