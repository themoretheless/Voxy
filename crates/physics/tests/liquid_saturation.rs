use physics::liquid::{
    Config, Liquid, LiquidField, Material, Particle, PhaseChange, SaturationCurve,
    TransportMaterial,
};
fn curve() -> SaturationCurve {
    SaturationCurve {
        reference_temperature: 373.15,
        reference_pressure: 101325.0,
        latent_heat: 2257000.0,
        vapor_gas_constant: 461.5,
        min_temperature: 300.0,
        max_temperature: 450.0,
    }
}
fn fluid() -> Liquid {
    let mut l = Liquid::new(
        vec![
            Particle {
                position: [0.0; 3],
                velocity: [0.0; 3],
                mass: 1.0,
                material: 0,
            },
            Particle {
                position: [0.4, 0.0, 0.0],
                velocity: [0.0; 3],
                mass: 2.0,
                material: 0,
            },
        ],
        vec![Material::WATER],
        Config {
            gravity: [0.0; 3],
            smoothing_radius: 1.0,
            ..Config::default()
        },
    )
    .unwrap();
    l.configure_transport(
        vec![
            LiquidField {
                temperature: 373.15,
                concentration: 0.0
            };
            2
        ],
        vec![TransportMaterial {
            specific_heat: 4200.0,
            conductivity: 0.0,
            diffusivity: 0.0,
            mixing_group: 0,
        }],
    )
    .unwrap();
    l.configure_phase_change(
        vec![Some(PhaseChange {
            temperature: 373.15,
            latent_heat: curve().latent_heat,
            high_phase: Material {
                rest_density: 1.0,
                sound_speed: 100.0,
                viscosity: 0.0,
            },
        })],
        vec![0.1, 0.3],
    )
    .unwrap();
    l
}
#[test]
fn saturation_roundtrip_monotone_and_domain_validation() {
    let c = curve();
    for t in [300.0, 330.0, 360.0, 373.15, 400.0, 450.0] {
        assert!((c.temperature(c.pressure(t).unwrap()).unwrap() - t).abs() < 1e-10);
    }
    assert!(c.pressure(360.0).unwrap() < c.reference_pressure);
    assert!(c.temperature(2.0 * c.reference_pressure).unwrap() > c.reference_temperature);
    assert!(c.temperature(0.0).is_err());
    assert!(c.temperature(f64::NAN).is_err());
    assert!(c.pressure(299.0).is_err());
    assert!(c.pressure(451.0).is_err());
    assert!(c.temperature(c.pressure(450.0).unwrap() * 2.0).is_err());
}
#[test]
fn local_pressure_remap_preserves_energy_and_reverses_phase_fraction() {
    let mut l = fluid();
    let initial = l.transport_totals().unwrap().unwrap().0;
    l.configure_saturation(
        vec![Some(curve())],
        vec![curve().reference_pressure, curve().pressure(360.0).unwrap()],
    )
    .unwrap();
    assert!((l.fields().unwrap()[1].temperature - 360.0).abs() < 1e-10);
    let expected = 0.3 + 4200.0 * (373.15 - 360.0) / curve().latent_heat;
    assert!((l.phase_fractions().unwrap()[1] - expected).abs() < 1e-12);
    assert!((l.transport_totals().unwrap().unwrap().0 - initial).abs() < 1e-8);
    l.set_saturation_pressures(vec![curve().pressure(400.0).unwrap(); 2])
        .unwrap();
    assert!(l.phase_fractions().unwrap()[0] < 0.1);
    l.set_saturation_pressures(vec![curve().pressure(330.0).unwrap(); 2])
        .unwrap();
    assert!(l.phase_fractions().unwrap()[0] > 0.1);
    l.set_saturation_pressures(vec![curve().reference_pressure; 2])
        .unwrap();
    assert!((l.phase_fractions().unwrap()[0] - 0.1).abs() < 1e-12);
    assert!((l.phase_fractions().unwrap()[1] - 0.3).abs() < 1e-12);
    assert!((l.transport_totals().unwrap().unwrap().0 - initial).abs() < 1e-8);
}
#[test]
fn local_pressure_plateau_drives_reservoir_heat_and_survives_drains() {
    let mut l = fluid();
    let pressure = curve().pressure(360.0).unwrap();
    l.configure_saturation(
        vec![Some(curve())],
        vec![curve().reference_pressure, pressure],
    )
    .unwrap();
    let before = l.transport_totals().unwrap().unwrap().0;
    let heat = l.exchange_reservoir_heat(2.0, 400.0, &[0.0, 10.0]).unwrap();
    assert!((heat - 800.0).abs() < 1e-7);
    assert!((l.fields().unwrap()[1].temperature - 360.0).abs() < 1e-10);
    assert!((l.transport_totals().unwrap().unwrap().0 - before - heat).abs() < 1e-8);
    l.exchange_particles(&[0], &[]).unwrap();
    assert_eq!(l.saturation_pressures().unwrap(), &[pressure]);
    assert!((l.fields().unwrap()[0].temperature - 360.0).abs() < 1e-10);
    let heat = l.exchange_reservoir_heat(2.0, 400.0, &[10.0]).unwrap();
    assert!((heat - 800.0).abs() < 1e-7);
}
#[test]
fn invalid_pressure_and_curve_changes_are_atomic() {
    let mut l = fluid();
    l.configure_saturation(vec![Some(curve())], vec![curve().reference_pressure; 2])
        .unwrap();
    let before = l.clone();
    assert!(l.set_saturation_pressures(vec![0.0; 2]).is_err());
    assert_eq!(l, before);
    assert!(
        l.set_saturation_pressures(vec![curve().reference_pressure])
            .is_err()
    );
    assert_eq!(l, before);
    assert!(
        l.configure_saturation(
            vec![Some(SaturationCurve {
                latent_heat: 1.0,
                ..curve()
            })],
            vec![curve().reference_pressure; 2]
        )
        .is_err()
    );
    assert_eq!(l, before);
    let p = l.particles()[0];
    assert!(
        l.exchange_particles(
            &[],
            &[physics::liquid::ParticleInput {
                particle: p,
                field: Some(l.fields().unwrap()[0]),
                phase_fraction: Some(0.1)
            }]
        )
        .is_err()
    );
    assert_eq!(l, before);
}

#[test]
fn source_pressure_and_body_heat_use_local_plateau() {
    use physics::liquid::{ParticleInput, ThermalTranslatingBody, TranslatingBody};
    let mut l = fluid();
    l.configure_saturation(vec![Some(curve())], vec![curve().reference_pressure; 2])
        .unwrap();
    let source = ParticleInput {
        particle: l.particles()[0],
        field: Some(LiquidField {
            temperature: 360.0,
            concentration: 0.0,
        }),
        phase_fraction: Some(0.2),
    };
    let pressure = curve().pressure(360.0).unwrap();
    l.exchange_particles_at_pressure(&[0, 1], &[source], &[pressure], None)
        .unwrap();
    assert_eq!(l.saturation_pressures().unwrap(), &[pressure]);
    let mut body = ThermalTranslatingBody {
        mechanics: TranslatingBody {
            position: [0.0; 3],
            velocity: [0.0; 3],
            mass: 1.0,
        },
        specific_heat: 1000.0,
        temperature: 400.0,
    };
    let initial = l.transport_totals().unwrap().unwrap().0 + body.thermal_energy().unwrap();
    let heat = l.exchange_body_heat(2.0, &mut body, &[10.0]).unwrap();
    let expected = 1000.0 * 40.0 * (-(-0.02_f64).exp_m1());
    assert!((heat - expected).abs() < 1e-7);
    assert!((l.fields().unwrap()[0].temperature - 360.0).abs() < 1e-10);
    assert!(
        (l.transport_totals().unwrap().unwrap().0 + body.thermal_energy().unwrap() - initial).abs()
            < 1e-8
    );
    let before = l.clone();
    let body_before = body;
    assert!(
        l.exchange_particles_at_pressure(&[], &[source], &[0.0], None)
            .is_err()
    );
    assert_eq!(l, before);
    assert_eq!(body, body_before);
}
