//! Shared rigid-body contact fixture (plan step B3): the same body the
//! normal-contact and liquid-body-world tests used to define each on their own.
use physics::{astrophysics_spin::Spin, contact::ContactBody, gravity::Body};

/// Unit-mass body at `position` with `velocity` and angular momentum `z` about Z.
pub fn body(position: [f64; 3], velocity: [f64; 3], z: f64) -> ContactBody {
    ContactBody {
        motion: Body {
            mass: 1.,
            position,
            velocity,
        },
        spin: Some(Spin {
            orientation: [0., 0., 0., 1.],
            angular_momentum: [0., 0., z],
            inertia: [1.; 3],
        }),
    }
}
