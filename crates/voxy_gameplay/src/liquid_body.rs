//! Authored finite translating body; scene pose is published by the liquid owner.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiquidBody {
    pub mass_kg: f64,
    pub initial_velocity_m_s: [f64; 3],
}

/// Explicit physical constituents in the root's authored local frame.
/// Collision shapes never supply or duplicate their mass implicitly.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiquidMassDistribution {
    pub parts: Vec<LiquidMassPart>,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiquidMassPart {
    pub mass_kg: f64,
    pub center_m: [f64; 3],
    pub half_edges_m: [[f64; 3]; 3],
}
impl LiquidMassDistribution {
    pub(crate) fn prepare(
        &self,
        mass: f64,
        pose: voxy_scene::Transform,
    ) -> Result<physics::mass_properties::MassProperties, String> {
        if self.parts.is_empty() || self.parts.len() > 128 {
            return Err("mass distribution budget".into());
        }
        pose.matrix().map_err(|e| format!("mass frame: {e:?}"))?;
        let linear = glam::DMat3::from_quat(pose.rotation.as_dquat())
            * glam::DMat3::from_diagonal(pose.scale.as_dvec3());
        let parts: Vec<_> = self
            .parts
            .iter()
            .map(|p| physics::mass_properties::UniformBoxMass {
                mass: p.mass_kg,
                center: (linear * glam::DVec3::from_array(p.center_m)).to_array(),
                half_edges: p
                    .half_edges_m
                    .map(|e| (linear * glam::DVec3::from_array(e)).to_array()),
            })
            .collect();
        let properties =
            physics::mass_properties::from_boxes(&parts, 128).map_err(str::to_string)?;
        if !mass.is_finite() || mass <= 0. || (properties.mass - mass).abs() > 1e-12 * mass {
            return Err("declared mass distribution does not sum to body mass".into());
        }
        Ok(properties)
    }
}
