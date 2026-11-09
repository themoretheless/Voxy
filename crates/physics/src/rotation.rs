//! Rigid-body principal-axis rotation, quaternion utilities, and torque laws.
//!
//! Provides a domain-neutral rotation boundary for rigid bodies, contacts, and fluid-rigid
//! coupling without requiring callers to import `astrophysics`.

pub(crate) use crate::astrophysics::spin::{
    ArcTorque, RotatingArmForce, RotatingMaterialMoment, cross, rotate, rotating_arm_integrals,
};
pub use crate::astrophysics::spin::{Spin, SpinArc, TorquePolynomial};
