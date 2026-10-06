//! Durable liquid source configuration; runtime inventories stay in physics owners.
use physics::liquid::{EmissionPulse, Particle, ParticleInput, PulsedEmitter};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiquidPulse {
    pub start_s: f64,
    pub duration_s: f64,
    pub volume_m3: f64,
    pub speed_m_s: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiquidSource {
    pub pulses: Vec<LiquidPulse>,
    pub density_kg_m3: f64,
    pub particle_volume_m3: f64,
    pub nozzle_radius_m: f64,
    /// World-space exhaust direction; non-unit vectors are normalized by physics.
    pub direction: [f64; 3],
    /// Durable asset identity, resolved to a runtime material slot by the liquid owner.
    pub material_asset: String,
}
impl LiquidSource {
    /// Prepare the existing physics emitter at an explicit world-space origin.
    /// Thermal/species fields and finite source inventory are admitted by their
    /// physics owners when the prepared emitter is attached to a liquid world.
    /// # Errors
    /// Invalid geometry, empty material identity, pulse budget or pulse values.
    pub fn prepare(
        &self,
        position_m: [f64; 3],
        material_slot: usize,
    ) -> Result<PulsedEmitter, physics::liquid::Error> {
        let length = self.direction.iter().map(|v| v * v).sum::<f64>();
        if self.pulses.len() > 4096
            || !self.density_kg_m3.is_finite()
            || self.density_kg_m3 <= 0.
            || !self.particle_volume_m3.is_finite()
            || self.particle_volume_m3 <= 0.
            || !self.nozzle_radius_m.is_finite()
            || self.nozzle_radius_m < 0.
            || !length.is_finite()
            || length <= 0.
            || position_m.iter().any(|v| !v.is_finite())
            || self.material_asset.is_empty()
        {
            return Err(physics::liquid::Error::InvalidConfig);
        }
        let mut emitter = PulsedEmitter::new(
            self.pulses
                .iter()
                .map(|p| EmissionPulse {
                    start: p.start_s,
                    duration: p.duration_s,
                    volume: p.volume_m3,
                    speed: p.speed_m_s,
                })
                .collect(),
            ParticleInput {
                particle: Particle {
                    position: position_m,
                    velocity: [0.; 3],
                    mass: 1.,
                    material: material_slot,
                },
                field: None,
                phase_fraction: None,
            },
        )?;
        emitter.direction = self.direction;
        emitter.density = self.density_kg_m3;
        emitter.particle_volume = self.particle_volume_m3;
        emitter.nozzle_radius = self.nozzle_radius_m;
        Ok(emitter)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn durable_source_prepares_existing_emission_and_rejects_invalid_configuration() {
        let source = LiquidSource {
            pulses: vec![LiquidPulse {
                start_s: 0.,
                duration_s: 1.,
                volume_m3: 0.001,
                speed_m_s: 2.,
            }],
            density_kg_m3: 1000.,
            particle_volume_m3: 0.0001,
            nozzle_radius_m: 0.,
            direction: [1., 0., 0.],
            material_asset: "liquids/water".into(),
        };
        let mut registry = voxy_scene::ComponentRegistry::default();
        crate::register_components(&mut registry).unwrap();
        let mut scene = voxy_scene::SceneGraph::new(1);
        let node = scene.spawn(None, voxy_scene::Transform::default()).unwrap();
        scene.insert_component(node, source.clone()).unwrap();
        let document = registry
            .capture_registered_components(&scene, node)
            .unwrap();
        assert!(
            serde_json::to_string(&document)
                .unwrap()
                .contains("game.liquid-source.v1")
        );
        assert!(
            crate::validate_game_descriptors(&scene, 1)
                .unwrap_err()
                .contains("attached scene liquid runtime")
        );
        let encoded = serde_json::to_string(&source).unwrap();
        let restored: LiquidSource = serde_json::from_str(&encoded).unwrap();
        assert_eq!(restored, source);
        let mut emitter = restored.prepare([0.; 3], 0).unwrap();
        let mut liquid = physics::liquid::Liquid::new(
            vec![],
            vec![physics::liquid::Material::WATER],
            physics::liquid::Config::default(),
        )
        .unwrap();
        let receipt = emitter.advance(&mut liquid, 1.).unwrap();
        assert!((receipt.added.mass - 1.).abs() < 1e-12);
        assert!((receipt.added.momentum[0] - 2.).abs() < 1e-12);
        let mut invalid = source.clone();
        invalid.pulses[0].duration_s = 0.;
        assert!(invalid.prepare([0.; 3], 0).is_err());
        invalid = source.clone();
        invalid.direction = [0.; 3];
        assert!(invalid.prepare([0.; 3], 0).is_err());
        invalid = source;
        invalid.particle_volume_m3 = f64::NAN;
        assert!(invalid.prepare([0.; 3], 0).is_err());
    }
}
