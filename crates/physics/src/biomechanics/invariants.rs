//! Stable strain invariant shared by constitutive material laws.
use super::{Matrix, det, mm, transpose};

pub(super) fn isochoric_excess(f: Matrix, q: f64, isochoric_trace: f64) -> f64 {
    isochoric_metric_excess(
        mm(transpose(f), f).map(|r| r.map(|v| q * v)),
        isochoric_trace,
    )
}

/// Reuse an already admitted isochoric metric without rebuilding F-transpose F.
pub(super) fn isochoric_metric_excess(metric: Matrix, isochoric_trace: f64) -> f64 {
    // det(Cbar)=1 implies tr(D)=-I2(D)-det(D), D=Cbar-I.
    // Near rest this avoids subtracting two O(1) values to recover O(strain²).
    let mut deviation = metric;
    for (i, row) in deviation.iter_mut().enumerate() {
        row[i] -= 1.;
    }
    if deviation.iter().flatten().all(|v| v.abs() < 0.1) {
        let second = deviation[0][0] * deviation[1][1]
            + deviation[0][0] * deviation[2][2]
            + deviation[1][1] * deviation[2][2]
            - deviation[0][1] * deviation[1][0]
            - deviation[0][2] * deviation[2][0]
            - deviation[1][2] * deviation[2][1];
        (-second - det(deviation)).max(0.)
    } else {
        isochoric_trace - 3.
    }
}
