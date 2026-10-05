//! Two physical particle reservoirs in the existing native scene pipeline.
use physics::liquid::{
    Config, Container, EmissionPulse, FiniteDropletDragReport, FiniteDropletGasGrid,
    GasGridFlowControl, GasGridFlowReport, GasGridHeatControl, GasGridHeatReport,
    GasGridViscosityControl, GasGridViscosityReport, GasGridWallViscosityControl,
    GasGridWallViscosityReport, Liquid, Material, Particle, ParticleInput, PulsedEmitter,
    SurfaceConfig, SurfaceVolumeControl, VaporCell,
};
use physics::liquid::{
    DepositingImpact, DropletCoalescenceControl, DropletFlight, DropletFlightReport,
    DropletLifecycle, FilmImpactControl, FilmRebound, ImpactSpray, LiquidField, TransportMaterial,
};
use physics::surface_film::{
    FilmMixture, Material as FilmMaterial, SurfaceFilm, ThermalFilmMixture,
};
use voxy_render::{SceneMesh, SceneVertex};

/// Convert physical height once to the renderer's world units. A positive f64
/// height can round to zero in f32; omit only that unrepresentable render cell,
/// retaining its physical volume. Never turn negative/invalid height into dry.
fn optical_film_thickness(height: f64) -> Result<Option<f32>, &'static str> {
    if !height.is_finite() || height < 0. {
        return Err("invalid physical film thickness");
    }
    let world = (height * f64::from(1.1_f32)) as f32;
    if !world.is_finite() {
        return Err("film thickness exceeds render range");
    }
    Ok((world > 0.).then_some(world))
}

const IMPACT_GRAVITY: [f64; 3] = [0.0, -9.81, 0.0];
const IMPACT_GAS: VaporCell = VaporCell {
    mass: 1.44,
    volume: 1.2,
    temperature: 300.0,
    velocity: [0.0; 3],
    specific_heat_cv: 718.0,
};

fn impact_gas_grid() -> Result<FiniteDropletGasGrid, physics::liquid::Error> {
    let cell = VaporCell {
        mass: IMPACT_GAS.mass / 16.0,
        volume: IMPACT_GAS.volume / 16.0,
        ..IMPACT_GAS
    };
    FiniteDropletGasGrid::new(
        [-1.0, 0.0, -0.3],
        [0.5, 0.5, 0.3],
        [4, 2, 2],
        vec![cell; 16],
    )
}

#[derive(Clone, Debug)]
struct ImpactFilm {
    thermal: ThermalFilmMixture,
    captured_sensible_energy: f64,
    points: Vec<[f64; 3]>,
    triangles: Vec<[usize; 3]>,
    captured: usize,
    fragments: usize,
    merges: usize,
    released_surface_energy: f64,
    unresolved_energy: f64,
    unresolved_angular_momentum: [f64; 3],
    gas_drag: FiniteDropletDragReport,
    flight: DropletFlightReport,
    gas_grid: FiniteDropletGasGrid,
    gas_flow: GasGridFlowReport,
    gas_heat: GasGridHeatReport,
    gas_viscosity: GasGridViscosityReport,
    gas_wall_viscosity: GasGridWallViscosityReport,
}

#[derive(Clone, Debug)]
pub(crate) struct LiquidDemo {
    liquids: [Liquid; 2],
    emitters: [PulsedEmitter; 2],
    finite_sources: Option<[(physics::liquid::TranslatingBody, f64); 2]>,
    emitted_mass: [f64; 2],
    films: Option<[ImpactFilm; 2]>,
    accumulator: f64,
    pub(crate) steps: u64,
}
impl LiquidDemo {
    pub(crate) fn new() -> Result<Self, physics::liquid::Error> {
        let make = |material: Material| {
            let mut particles = Vec::new();
            let spacing: f64 = 0.08;
            for x in 0..5 {
                for y in 0..6 {
                    for z in 0..4 {
                        particles.push(Particle {
                            position: [
                                -0.7 + f64::from(x) * spacing,
                                0.1 + f64::from(y) * spacing,
                                -0.12 + f64::from(z) * spacing,
                            ],
                            velocity: [0.0; 3],
                            mass: material.rest_density * spacing.powi(3),
                            material: 0,
                        });
                    }
                }
            }
            Liquid::new(
                particles,
                vec![material],
                Config {
                    gravity: IMPACT_GRAVITY,
                    smoothing_radius: 0.16,
                    particle_radius: 0.03,
                    ..Config::default()
                },
            )
        };
        let source = |material: Material| -> Result<PulsedEmitter, physics::liquid::Error> {
            let mut emitter = PulsedEmitter::new(
                vec![EmissionPulse {
                    start: 0.1,
                    duration: 0.3,
                    volume: 0.005,
                    speed: 1.5,
                }],
                ParticleInput {
                    particle: Particle {
                        position: [0.55, 0.8, 0.0],
                        velocity: [0.0; 3],
                        mass: 1.0,
                        material: 0,
                    },
                    field: None,
                    phase_fraction: None,
                },
            )?;
            emitter.direction = [-0.6, -0.8, 0.0];
            emitter.nozzle_radius = 0.04;
            emitter.particle_volume = 0.000512;
            emitter.density = material.rest_density;
            Ok(emitter)
        };
        Ok(Self {
            liquids: [make(Material::WATER)?, make(Material::OIL)?],
            emitters: [source(Material::WATER)?, source(Material::OIL)?],
            finite_sources: None,
            emitted_mass: [0.0; 2],
            films: None,
            accumulator: 0.0,
            steps: 0,
        })
    }
    pub(crate) fn new_finite_sources() -> Result<Self, physics::liquid::Error> {
        let mut demo = Self::new()?;
        demo.finite_sources = Some(std::array::from_fn(|index| {
            (
                physics::liquid::TranslatingBody {
                    position: demo.emitters[index].template.particle.position,
                    velocity: [0.; 3],
                    mass: 100.,
                },
                1000.,
            )
        }));
        for emitter in &mut demo.emitters {
            emitter.nozzle_radius = 0.;
        }
        Ok(demo)
    }
    pub(crate) fn new_impacts() -> Result<Self, physics::liquid::Error> {
        let mut demo = Self::new()?;
        let make = |material: Material| -> Result<(Liquid, ImpactFilm), physics::liquid::Error> {
            let mut liquid = Liquid::new(
                vec![],
                vec![material],
                Config {
                    smoothing_radius: 0.16,
                    particle_radius: 0.005,
                    ..Config::default()
                },
            )?;
            liquid.configure_transport(
                vec![],
                vec![TransportMaterial {
                    conductivity: 0.0,
                    diffusivity: 0.0,
                    ..TransportMaterial::default()
                }],
            )?;
            liquid.configure_species(vec!["fluid".into()], vec![])?;
            liquid.configure_droplet_population(Some(vec![]))?;
            let mut points = Vec::new();
            for z in 0..=4 {
                for x in 0..=16 {
                    points.push([-1.0 + f64::from(x) / 8.0, 0.0, -0.3 + f64::from(z) * 0.15]);
                }
            }
            let mut triangles = Vec::new();
            for z in 0..4 {
                for x in 0..16 {
                    let a = z * 17 + x;
                    triangles.extend([[a, a + 1, a + 18], [a, a + 18, a + 17]]);
                }
            }
            let film = SurfaceFilm::new(
                &points,
                triangles.clone(),
                FilmMaterial {
                    density: material.rest_density,
                    viscosity: material.viscosity,
                    wetting: 1e-5,
                    surface_tension: 0.0,
                },
            )
            .map_err(|_| physics::liquid::Error::InvalidConfig)?;
            let mixture =
                FilmMixture::new(film, vec!["fluid".into()], vec![vec![1.0]; triangles.len()])
                    .map_err(|_| physics::liquid::Error::InvalidConfig)?;
            Ok((
                liquid,
                ImpactFilm {
                    thermal: ThermalFilmMixture::new(
                        mixture,
                        vec![TransportMaterial::default().specific_heat],
                        &vec![300.; triangles.len()],
                    )
                    .map_err(|_| physics::liquid::Error::InvalidConfig)?,
                    captured_sensible_energy: 0.,
                    points,
                    triangles,
                    captured: 0,
                    fragments: 0,
                    merges: 0,
                    released_surface_energy: 0.0,
                    unresolved_energy: 0.0,
                    unresolved_angular_momentum: [0.0; 3],
                    gas_drag: FiniteDropletDragReport::default(),
                    flight: DropletFlightReport::default(),
                    gas_grid: impact_gas_grid()?,
                    gas_flow: GasGridFlowReport::default(),
                    gas_heat: GasGridHeatReport::default(),
                    gas_viscosity: GasGridViscosityReport::default(),
                    gas_wall_viscosity: GasGridWallViscosityReport::default(),
                },
            ))
        };
        let (water, water_film) = make(Material::WATER)?;
        let (oil, oil_film) = make(Material::OIL)?;
        demo.liquids = [water, oil];
        demo.films = Some([water_film, oil_film]);
        for emitter in &mut demo.emitters {
            emitter.template.field = Some(LiquidField {
                temperature: 300.0,
                concentration: 0.0,
            });
        }
        Ok(demo)
    }
    pub(crate) fn restart(&self) -> Result<Self, physics::liquid::Error> {
        if self.finite_sources.is_some() {
            Self::new_finite_sources()
        } else if self.films.is_some() {
            Self::new_impacts()
        } else {
            Self::new()
        }
    }
    pub(crate) fn advance(&mut self, dt: f64) -> Result<(), physics::liquid::Error> {
        if !dt.is_finite() || dt < 0.0 {
            return Err(physics::liquid::Error::InvalidTimeStep);
        }
        let accumulated = self.accumulator + dt.min(0.1);
        if accumulated < 1.0 / 120.0 {
            // No physical subsystem runs yet, so only the clock is published.
            self.accumulator = accumulated;
            return Ok(());
        }
        let mut candidate = self.clone();
        candidate.advance_candidate(dt)?;
        *self = candidate;
        Ok(())
    }

    fn advance_candidate(&mut self, dt: f64) -> Result<(), physics::liquid::Error> {
        self.accumulator += dt.min(0.1);
        while self.accumulator >= 1.0 / 120.0 {
            for (index, liquid) in self.liquids.iter_mut().enumerate() {
                let emitted = if let Some(sources) = &mut self.finite_sources {
                    let (body, reserve) = &mut sources[index];
                    let old_velocity = body.velocity;
                    let receipt = self.emitters[index].advance_from_translating_source(
                        liquid,
                        1. / 120.,
                        body,
                        90.,
                        reserve,
                        None,
                    )?;
                    for axis in 0..3 {
                        body.position[axis] +=
                            (0.5 * old_velocity[axis] + 0.5 * body.velocity[axis]) / 120.;
                    }
                    if body.position.iter().any(|p| !p.is_finite()) {
                        return Err(physics::liquid::Error::NumericalFailure);
                    }
                    receipt.particles
                } else if self.films.is_some() {
                    self.emitters[index].advance_with_species(liquid, 1.0 / 120.0, &[1.0])?
                } else {
                    self.emitters[index].advance(liquid, 1.0 / 120.0)?
                };
                self.emitted_mass[index] += emitted.added.mass;
                if let Some(films) = &mut self.films {
                    let radii = liquid.equivalent_sphere_radii()?;
                    let drag = liquid.exchange_marked_droplet_drag_grid(
                        1.0 / 120.0,
                        &mut films[index].gas_grid,
                        &radii,
                        0.47,
                    )?;
                    let drag = drag.drag;
                    let gas = &mut films[index].gas_drag;
                    for k in 0..3 {
                        gas.gas_impulse[k] += drag.gas_impulse[k];
                    }
                    gas.dissipated_heat += drag.dissipated_heat;
                    gas.gas_kinetic_energy_change += drag.gas_kinetic_energy_change;
                    gas.pair_steps += drag.pair_steps;
                    let flow = films[index]
                        .gas_grid
                        .advance_euler(1.0 / 120.0, GasGridFlowControl::default())?;
                    films[index].gas_flow.substeps += flow.substeps;
                    films[index].gas_flow.face_updates += flow.face_updates;
                    for k in 0..3 {
                        films[index].gas_flow.wall_impulse[k] += flow.wall_impulse[k];
                    }
                    let viscosity = films[index].gas_grid.relax_viscosity_with_walls(
                        1.0 / 120.0,
                        GasGridViscosityControl::default(),
                        GasGridWallViscosityControl::default(),
                    )?;
                    let wall = viscosity.walls;
                    let wall_total = &mut films[index].gas_wall_viscosity;
                    wall_total.substeps += wall.substeps;
                    wall_total.face_steps += wall.face_steps;
                    wall_total.work_on_gas += wall.work_on_gas;
                    wall_total.dissipated_heat += wall.dissipated_heat;
                    for k in 0..3 {
                        wall_total.wall_impulse[k] += wall.wall_impulse[k];
                        for face in 0..6 {
                            wall_total.face_impulses[face][k] += wall.face_impulses[face][k];
                        }
                    }
                    let viscosity = viscosity.interior;
                    films[index].gas_viscosity.substeps += viscosity.substeps;
                    films[index].gas_viscosity.block_steps += viscosity.block_steps;
                    films[index].gas_viscosity.dissipated_heat += viscosity.dissipated_heat;
                    let heat = films[index]
                        .gas_grid
                        .conduct_heat(1.0 / 120.0, GasGridHeatControl::default())?;
                    films[index].gas_heat.substeps += heat.substeps;
                    films[index].gas_heat.pair_steps += heat.pair_steps;
                    films[index].gas_heat.absolute_transferred_heat +=
                        heat.absolute_transferred_heat;
                }
                let initial_velocities: Vec<_> =
                    liquid.particles().iter().map(|p| p.velocity).collect();
                let previous: Vec<_> = liquid.particles().iter().map(|p| p.position).collect();
                let advance = if self.films.is_some() {
                    Liquid::step_carrier_and_droplets
                } else {
                    Liquid::step
                };
                advance(
                    liquid,
                    1.0 / 120.0,
                    if self.films.is_some() {
                        None
                    } else {
                        Some(Container {
                            min: [-1.0, 0.0, -0.3],
                            max: [1.0, 1.0, 0.3],
                            restitution: 0.0,
                            friction: 0.02,
                        })
                    },
                )?;
                if let Some(films) = &mut self.films {
                    let film = &mut films[index];
                    let radii = liquid.equivalent_sphere_radii()?;
                    let report = liquid
                        .depositing_impact_spheres_thermal_surface_mixture_lifecycle(
                            &previous,
                            &mut film.thermal,
                            DepositingImpact {
                                capture_speed: 3.0,
                                spray: ImpactSpray {
                                    rebound: FilmRebound {
                                        restitution: 0.4,
                                        friction: 0.1,
                                    },
                                    children: 2,
                                    position_radius: 0.015,
                                    surface_tension: 0.072,
                                    fragmentation_fraction: 0.2,
                                },
                            },
                            &radii,
                            FilmImpactControl {
                                dt: 1.0 / 120.0,
                                max_contacts_per_lineage: 16,
                                max_events: 4096,
                            },
                            &DropletLifecycle {
                                mass_fractions: Some(vec![0.2, 0.8]),
                                splash_onset: None,
                                capture_growth_contact: true,
                                free_surface_side: Some(-1.0),
                                capture_film_immersion: true,
                                flight: Some(DropletFlight {
                                    initial_velocities,
                                    acceleration: IMPACT_GRAVITY,
                                    max_feature_checks: 10000,
                                }),
                                coalescence: Some(DropletCoalescenceControl {
                                    dt: 1.0 / 120.0,
                                    surface_tension: 0.072,
                                    maximum_normal_speed: 0.5,
                                    max_events: 4096,
                                }),
                            },
                        )
                        .map_err(|reason| {
                            eprintln!("liquid lifecycle: {reason}");
                            physics::liquid::Error::InvalidCollision
                        })?;
                    film.flight.work += report.flight.work;
                    for k in 0..3 {
                        film.flight.impulse[k] += report.flight.impulse[k];
                    }
                    film.merges += report.coalescence.events.len();
                    film.released_surface_energy += report.coalescence.released_surface_energy;
                    film.unresolved_energy += report.coalescence.unresolved_kinetic_energy;
                    for k in 0..3 {
                        film.unresolved_angular_momentum[k] +=
                            report.coalescence.unresolved_angular_momentum[k];
                    }
                    let report = report.impact;
                    film.captured += report.deposition.capture.particles;
                    film.captured_sensible_energy += report
                        .deposition
                        .capture
                        .absorbed
                        .thermal_energy
                        .ok_or(physics::liquid::Error::InvalidTransport)?;

                    film.fragments += report.spray.fragments_created;
                    film.thermal
                        .step_with_surface_shear(
                            1.0 / 120.0,
                            [0.0; 3],
                            &vec![[0.02, 0.0, 0.0]; film.triangles.len()],
                            0.001,
                        )
                        .map_err(|_| physics::liquid::Error::NumericalFailure)?;
                }
            }
            self.accumulator -= 1.0 / 120.0;
            self.steps += 1;
        }
        Ok(())
    }
    pub(crate) fn verify(&self) -> Result<(), &'static str> {
        if self.steps < 120 {
            return Err("liquid smoke did not advance enough fixed steps");
        }
        if let Some(sources) = &self.finite_sources {
            for (index, (body, reserve)) in sources.iter().enumerate() {
                if !body.mass.is_finite()
                    || (body.mass + self.emitted_mass[index] - 100.).abs() > 1e-9
                    || body.mass < 90.
                    || !reserve.is_finite()
                    || *reserve < 0.
                    || *reserve >= 1000.
                    || body.velocity.iter().any(|v| !v.is_finite())
                    || body.velocity[0] <= 0.
                    || body.velocity[1] <= 0.
                {
                    return Err("finite source mass, recoil or energy reserve failed");
                }
            }
        }
        for (index, liquid) in self.liquids.iter().enumerate() {
            if let Some(films) = &self.films {
                let film = &films[index];
                let expected = if index == 0 { 5.0 } else { 4.0 };
                if (self.emitted_mass[index] - expected).abs() > 1e-9
                    || (liquid.mass() + film.thermal.mixture().film().total_mass() - expected).abs()
                        > 1e-9
                {
                    return Err("native impact mass ledger failed");
                }
                let stored_heat = film.thermal.energies_j().iter().sum::<f64>();
                if !stored_heat.is_finite()
                    || !film.captured_sensible_energy.is_finite()
                    || (stored_heat - film.captured_sensible_energy).abs() > 1e-6
                {
                    return Err("native captured film heat ledger failed");
                }
                film.thermal
                    .temperatures()
                    .map_err(|_| "native film thermal state failed")?;
                let gas = film
                    .gas_grid
                    .totals()
                    .map_err(|_| "native gas-grid totals failed")?;
                if (gas.mass - IMPACT_GAS.mass).abs() > 1e-14
                    || (gas.volume - IMPACT_GAS.volume).abs() > 1e-14
                    || (gas.kinetic_energy + gas.thermal_energy
                        - IMPACT_GAS.mass * IMPACT_GAS.specific_heat_cv * IMPACT_GAS.temperature
                        - film.gas_drag.dissipated_heat
                        - film.gas_drag.gas_kinetic_energy_change
                        - film.gas_wall_viscosity.work_on_gas)
                        .abs()
                        > 1e-7
                    || (0..3).any(|k| {
                        (gas.momentum[k]
                            + film.gas_flow.wall_impulse[k]
                            + film.gas_wall_viscosity.wall_impulse[k]
                            - film.gas_drag.gas_impulse[k])
                            .abs()
                            > 1e-10
                    })
                {
                    return Err("native finite gas grid ledger failed");
                }
                if liquid
                    .droplet_population()
                    .is_none_or(|flags| flags.len() != liquid.particles().len())
                    || [film.released_surface_energy, film.unresolved_energy]
                        .iter()
                        .chain(&film.unresolved_angular_momentum)
                        .chain(&film.flight.impulse)
                        .chain([&film.flight.work])
                        .chain([
                            &film.gas_heat.absolute_transferred_heat,
                            &film.gas_viscosity.dissipated_heat,
                            &film.gas_wall_viscosity.dissipated_heat,
                            &film.gas_wall_viscosity.work_on_gas,
                        ])
                        .chain(&film.gas_flow.wall_impulse)
                        .chain(&film.gas_wall_viscosity.wall_impulse)
                        .chain(film.gas_wall_viscosity.face_impulses.iter().flatten())
                        .chain(&film.gas_drag.gas_impulse)
                        .chain([
                            &film.gas_drag.dissipated_heat,
                            &film.gas_drag.gas_kinetic_energy_change,
                        ])
                        .any(|v| !v.is_finite())
                {
                    return Err("native droplet population or merge ledger failed");
                }
                if film.captured == 0 || film.fragments == 0 {
                    return Err("native impact did not both spray and deposit");
                }
                continue;
            }
            let expected = if index == 0 { 61.44 } else { 49.152 };
            if (self.emitted_mass[index] - if index == 0 { 5.0 } else { 4.0 }).abs() > 1e-9 {
                return Err("native liquid jet did not emit its complete pulse");
            }
            if (liquid.mass() - expected - self.emitted_mass[index]).abs() > 1e-9 {
                return Err("liquid mass changed");
            }
            let min = liquid
                .particles()
                .iter()
                .map(|p| p.position[0])
                .fold(f64::INFINITY, f64::min);
            let max = liquid
                .particles()
                .iter()
                .map(|p| p.position[0])
                .fold(f64::NEG_INFINITY, f64::max);
            if max - min < 0.5 {
                return Err("liquid did not spread");
            }
        }
        Ok(())
    }
    #[allow(clippy::cast_possible_truncation)]
    /// Optical snapshots borrow the simulation; they never create or remove liquid mass.
    pub(crate) fn optical_scene(
        &self,
    ) -> Result<(SceneMesh, Vec<voxy_render::FluidRenderParticle>), Box<dyn std::error::Error>>
    {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut samples = Vec::new();
        for (group, liquid) in self.liquids.iter().enumerate() {
            let offset = if group == 0 { -1.25 } else { 1.25 };
            let world = |p: [f64; 3]| {
                [
                    offset + p[0] as f32 * 1.1,
                    p[1] as f32 * 1.1 - 0.9,
                    p[2] as f32 * 1.1,
                ]
            };
            // A contrasting reference grid makes background refraction visible.
            for y in 0..10 {
                for x in 0..20 {
                    let x0 = -1.0 + x as f64 * 0.1;
                    let y0 = y as f64 * 0.1;
                    for quad in [
                        [
                            [x0, y0, -0.3],
                            [x0 + 0.1, y0, -0.3],
                            [x0 + 0.1, y0 + 0.1, -0.3],
                            [x0, y0 + 0.1, -0.3],
                        ],
                        [
                            [x0, 0.0, y0 * 0.6 - 0.3],
                            [x0 + 0.1, 0.0, y0 * 0.6 - 0.3],
                            [x0 + 0.1, 0.0, (y0 + 0.1) * 0.6 - 0.3],
                            [x0, 0.0, (y0 + 0.1) * 0.6 - 0.3],
                        ],
                    ] {
                        let base = vertices.len() as u32;
                        let c = if (x + y) % 2 == 0 {
                            [0.32, 0.4, 0.48, 1.0]
                        } else {
                            [0.08, 0.12, 0.17, 1.0]
                        };
                        for point in quad {
                            vertices.push(SceneVertex {
                                position: world(point),
                                uv: [0.0; 2],
                                color: c,
                            });
                        }
                        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
                    }
                }
            }
            let materials = liquid.effective_materials()?;
            for (particle, material) in liquid.particles().iter().zip(materials) {
                let radius =
                    (3.0 * particle.mass / material.rest_density / (4.0 * std::f64::consts::PI))
                        .cbrt() as f32
                        * 1.1;
                let p = world(particle.position);
                samples.push(voxy_render::FluidRenderParticle {
                    position_radius: [p[0], p[1], p[2], radius],
                    absorption_ior: if group == 0 {
                        [0.1, 0.04, 0.02, 1.333]
                    } else {
                        [0.3, 3.0, 12.0, 1.47]
                    },
                });
            }
        }
        Ok((SceneMesh::new(vertices, indices)?, samples))
    }

    /// Borrow exact per-cell film thickness; never mutate physical inventories.
    pub(crate) fn optical_film_triangles(
        &self,
    ) -> Result<Vec<voxy_render::FluidRenderFilmTriangle>, &'static str> {
        let mut result = Vec::new();
        if let Some(films) = &self.films {
            for (group, film) in films.iter().enumerate() {
                let offset = if group == 0 { -1.25 } else { 1.25 };
                let material = if group == 0 {
                    [0.1, 0.04, 0.02, 1.333]
                } else {
                    [0.3, 3.0, 12.0, 1.47]
                };
                let thickness = film.thermal.mixture().film().thickness();
                if thickness.len() != film.triangles.len() {
                    return Err("film optical cell count mismatch");
                }
                for (cell, (face, h)) in film.triangles.iter().zip(thickness).enumerate() {
                    let Some(world_thickness) = optical_film_thickness(h)? else {
                        continue;
                    };
                    // The existing horizontal substrate indices point downward.
                    // Optical prisms must extrude above the owned floor.
                    let points = [face[0], face[2], face[1]].map(|i| {
                        let p = film.points[i];
                        [
                            offset + p[0] as f32 * 1.1,
                            p[1] as f32 * 1.1 - 0.9,
                            p[2] as f32 * 1.1,
                        ]
                    });
                    result.push(voxy_render::FluidRenderFilmTriangle::new(
                        points,
                            world_thickness,
                        material,
                    ).map_err(|error| {
                        eprintln!("OPTICAL FILM rejected group={group} cell={cell} thickness_m={h} points={points:?}: {error}");
                        error
                    })?);
                }
            }
        }
        Ok(result)
    }

    pub(crate) fn mesh(&self) -> Result<SceneMesh, Box<dyn std::error::Error>> {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut triangle = |points: [[f64; 3]; 3], color: [f32; 4]| {
            let base = vertices.len() as u32;
            for position in points {
                vertices.push(SceneVertex {
                    position: position.map(|v| v as f32),
                    uv: [0.0; 2],
                    color,
                });
            }
            indices.extend([base, base + 1, base + 2]);
        };
        for (group, liquid) in self.liquids.iter().enumerate() {
            let offset = if group == 0 { -1.25 } else { 1.25 };
            let world = |p: [f64; 3]| [offset + p[0] * 1.1, p[1] * 2.0 - 0.9, p[2] * 1.5];
            if let Some(sources) = &self.finite_sources {
                let center = sources[group].0.position;
                let points: [[f64; 3]; 6] = std::array::from_fn(|i| {
                    let mut p = center;
                    p[i / 2] += if i % 2 == 0 { 0.08 } else { -0.08 };
                    world(p)
                });
                for face in [
                    [0, 2, 4],
                    [2, 1, 4],
                    [1, 3, 4],
                    [3, 0, 4],
                    [2, 0, 5],
                    [1, 2, 5],
                    [3, 1, 5],
                    [0, 3, 5],
                ] {
                    triangle(face.map(|i| points[i]), [0.95, 0.55, 0.1, 1.]);
                }
            }
            // Opaque floor and rear wall leave the particle movement visible.
            for quad in [
                [
                    [-1.0, 0.0, -0.3],
                    [1.0, 0.0, -0.3],
                    [1.0, 0.0, 0.3],
                    [-1.0, 0.0, 0.3],
                ],
                [
                    [-1.0, 0.0, -0.3],
                    [-1.0, 1.0, -0.3],
                    [1.0, 1.0, -0.3],
                    [1.0, 0.0, -0.3],
                ],
            ] {
                let p = quad.map(world);
                let color = [0.1, 0.14, 0.19, 1.0];
                triangle([p[0], p[1], p[2]], color);
                triangle([p[0], p[2], p[3]], color);
            }
            let color = if group == 0 {
                [0.08, 0.55, 0.95, 1.0]
            } else {
                [0.95, 0.62, 0.12, 1.0]
            };
            if self.films.is_some() && liquid.mass() > 1e-10 {
                let radii = liquid.equivalent_sphere_radii()?;
                let flags = liquid
                    .droplet_population()
                    .ok_or("missing droplet population")?;
                let supports: Vec<_> = radii
                    .iter()
                    .enumerate()
                    .map(|(i, r)| if flags[i] { 2.5 * r } else { 0.16 })
                    .collect();
                let mut min = [f64::INFINITY; 3];
                let mut max = [f64::NEG_INFINITY; 3];
                for (p, h) in liquid.particles().iter().zip(&supports) {
                    for k in 0..3 {
                        min[k] = min[k].min(p.position[k] - h - 0.025);
                        max[k] = max[k].max(p.position[k] + h + 0.025);
                    }
                }
                let matched = liquid.surface_volume_matched(
                    SurfaceConfig {
                        min,
                        max,
                        cell_size: 0.025,
                        isovalue: 0.25,
                        material: None,
                        max_samples: 1_000_000,
                        max_checks: 2_000_000,
                        max_triangles: 100_000,
                    },
                    &supports,
                    SurfaceVolumeControl {
                        relative_tolerance: 0.035,
                        max_iterations: 48,
                        max_checks: 40_000_000,
                    },
                )?;
                // Threshold trials may be larger than the volume-matched result.
                // Keep the final draw within the existing reserved GPU capacity.
                if matched.surface.triangles.len() > 8500 {
                    return Err(physics::liquid::Error::SurfaceBudget.into());
                }
                for face in matched.surface.triangles {
                    triangle(face.map(world), color);
                }
            }
            // Isolated airborne samples may fall below the isosurface threshold.
            // Draw their volume-equivalent spheres as eight-face approximations.
            let materials = liquid.effective_materials()?;
            for (index, particle) in liquid
                .particles()
                .iter()
                .enumerate()
                .filter(|(_, p)| self.films.is_none() && p.position[1] > 0.55)
            {
                let radius = (3.0 * particle.mass
                    / materials[index].rest_density
                    / (4.0 * std::f64::consts::PI))
                    .cbrt();
                let points: [[f64; 3]; 6] = std::array::from_fn(|i| {
                    let mut p = particle.position;
                    p[i / 2] += if i % 2 == 0 { radius } else { -radius };
                    world(p)
                });
                for face in [
                    [0, 2, 4],
                    [2, 1, 4],
                    [1, 3, 4],
                    [3, 0, 4],
                    [2, 0, 5],
                    [1, 2, 5],
                    [3, 1, 5],
                    [0, 3, 5],
                ] {
                    triangle(face.map(|i| points[i]), color);
                }
            }
            if let Some(films) = &self.films {
                let film = &films[group];
                let heights = film
                    .thermal
                    .mixture()
                    .film()
                    .vertex_thickness(film.points.len())?;
                for face in &film.triangles {
                    // Hide numerical wetting traces thinner than 10 micrometres.
                    if face.iter().map(|&i| heights[i]).sum::<f64>() / 3.0 >= 1e-5 {
                        triangle(
                            face.map(|i| {
                                let mut p = film.points[i];
                                p[1] = heights[i] + 1e-5;
                                world(p)
                            }),
                            color,
                        );
                    }
                }
                continue;
            }
            let surface = liquid.surface(SurfaceConfig {
                min: [-1.2, -0.2, -0.5],
                max: [1.2, 1.2, 0.5],
                cell_size: 0.08,
                isovalue: 0.45,
                material: None,
                max_samples: 20_000,
                max_checks: 3_000_000,
                max_triangles: 9_500,
            })?;
            for face in surface.triangles {
                let face = face.map(world);
                let a: [f64; 3] = std::array::from_fn(|axis| face[1][axis] - face[0][axis]);
                let b: [f64; 3] = std::array::from_fn(|axis| face[2][axis] - face[0][axis]);
                let normal = [
                    a[1] * b[2] - a[2] * b[1],
                    a[2] * b[0] - a[0] * b[2],
                    a[0] * b[1] - a[1] * b[0],
                ];
                let length = normal.iter().map(|v| v * v).sum::<f64>().sqrt();
                let shade = (0.4
                    + 0.6 * ((normal[1] + normal[2]) / length / std::f64::consts::SQRT_2).max(0.0))
                    as f32;
                let mut tint = color;
                for channel in &mut tint[..3] {
                    *channel *= shade;
                }
                triangle(face, tint);
            }
        }
        Ok(SceneMesh::new(vertices, indices)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finite_source_visual_demo_emits_recoils_and_preserves_failed_frame() {
        let mut demo = LiquidDemo::new_finite_sources().unwrap();
        for _ in 0..15 {
            demo.advance(1. / 120.).unwrap();
        }
        for (body, reserve) in demo.finite_sources.unwrap() {
            assert!(body.mass < 100. && body.mass > 90.);
            assert!(body.velocity[0] > 0. && body.velocity[1] > 0.);
            assert!(reserve < 1000.);
        }
        demo.mesh().unwrap();
        let restarted = demo.restart().unwrap();
        assert!(restarted.finite_sources.is_some());
        assert_eq!(restarted.steps, 0);
        for (body, reserve) in restarted.finite_sources.unwrap() {
            assert_eq!(body.mass, 100.);
            assert_eq!(reserve, 1000.);
        }
        demo.emitters[1].direction = [f64::NAN; 3];
        let before = format!("{demo:?}");
        assert!(demo.advance(1. / 120.).is_err());
        assert_eq!(before, format!("{demo:?}"));
    }
    #[test]
    fn optical_film_conversion_omits_only_zero_quantized_height() {
        assert_eq!(optical_film_thickness(0.).unwrap(), None);
        assert_eq!(
            optical_film_thickness(3.1547334616506297e-83).unwrap(),
            None
        );
        let smallest = f32::from_bits(1);
        assert_eq!(
            optical_film_thickness(f64::from(smallest) / f64::from(1.1_f32)).unwrap(),
            Some(smallest)
        );
        assert_eq!(
            optical_film_thickness(0.003).unwrap(),
            Some((0.003 * f64::from(1.1_f32)) as f32)
        );
        for invalid in [-3.1547334616506297e-83, f64::NAN, f64::INFINITY, f64::MAX] {
            assert!(optical_film_thickness(invalid).is_err());
        }
    }
    #[test]
    #[ignore = "manual timing control; no performance threshold"]
    fn measure_native_liquid_transaction_cost() {
        use std::{hint::black_box, time::Instant};
        for (mode, initial) in [
            ("reservoirs", LiquidDemo::new().unwrap()),
            ("impacts", LiquidDemo::new_impacts().unwrap()),
        ] {
            let mut clone_ns = Vec::new();
            for _ in 0..9 {
                let start = Instant::now();
                for _ in 0..100 {
                    drop(black_box(initial.clone()));
                }
                clone_ns.push(start.elapsed().as_nanos() / 100);
            }
            let mut direct_ns = Vec::new();
            let mut atomic_ns = Vec::new();
            for _ in 0..9 {
                let mut direct = initial.clone();
                let start = Instant::now();
                direct.advance_candidate(1. / 120.).unwrap();
                direct_ns.push(start.elapsed().as_nanos());
                let mut atomic = initial.clone();
                let start = Instant::now();
                atomic.advance(1. / 120.).unwrap();
                atomic_ns.push(start.elapsed().as_nanos());
                assert_eq!(format!("{direct:?}"), format!("{atomic:?}"));
            }
            clone_ns.sort_unstable();
            direct_ns.sort_unstable();
            atomic_ns.sort_unstable();
            println!(
                "liquid_transaction_timing={{\"mode\":\"{mode}\",\"clone_median_ns\":{},\"direct_step_median_ns\":{},\"atomic_step_median_ns\":{},\"trials\":9,\"state\":\"initial\"}}",
                clone_ns[4], direct_ns[4], atomic_ns[4]
            );
        }
    }

    #[test]
    fn native_liquid_failure_preserves_both_reservoirs_and_clock() {
        for mut demo in [
            LiquidDemo::new().unwrap(),
            LiquidDemo::new_impacts().unwrap(),
        ] {
            for dt in [f64::NAN, f64::INFINITY, -0.01] {
                assert!(matches!(
                    demo.advance(dt),
                    Err(physics::liquid::Error::InvalidTimeStep)
                ));
                assert_eq!(demo.accumulator, 0.);
                assert_eq!(demo.steps, 0);
            }
            let liquids = demo.liquids.clone();
            let emitters = demo.emitters.clone();
            demo.advance(1. / 480.).unwrap();
            assert_eq!(demo.steps, 0);
            assert_eq!(demo.accumulator, 1. / 480.);
            assert_eq!(demo.liquids, liquids);
            assert_eq!(demo.emitters, emitters);
            demo.emitters[1].direction = [f64::NAN; 3];
            let before = format!("{demo:?}");
            assert!(demo.advance(1. / 120.).is_err());
            assert_eq!(format!("{demo:?}"), before);
            demo.emitters[1].direction = [-0.6, -0.8, 0.];
            demo.advance(1. / 120.).unwrap();
            assert_eq!(demo.steps, 1);
        }
    }

    #[test]
    fn native_jets_spray_deposit_and_preserve_mass_and_reset_mode() {
        let mut demo = LiquidDemo::new_impacts().unwrap();
        for _ in 0..120 {
            demo.advance(1.0 / 120.0).unwrap();
            assert!(demo.mesh().unwrap().vertices().len() <= 60_024);
        }
        demo.verify().unwrap();
        assert!(
            demo.films
                .as_ref()
                .unwrap()
                .iter()
                .any(|film| film.merges > 0)
        );
        assert!(
            demo.films
                .as_ref()
                .unwrap()
                .iter()
                .any(|film| film.gas_drag.dissipated_heat > 0.0)
        );
        assert!(
            demo.films
                .as_ref()
                .unwrap()
                .iter()
                .all(|film| film.flight.impulse[1] < 0.0)
        );
        assert!(demo.films.as_ref().unwrap().iter().all(|film| {
            film.gas_grid.cells().iter().any(|cell| {
                cell.temperature > IMPACT_GAS.temperature
                    && cell.velocity.iter().any(|v| v.abs() > 0.0)
            })
        }));
        for film in demo.films.as_ref().unwrap() {
            assert!(film.gas_flow.substeps > 0 && film.gas_flow.face_updates > 0);
            assert!(film.gas_heat.substeps > 0 && film.gas_heat.pair_steps > 0);
            assert!(film.gas_heat.absolute_transferred_heat > 0.0);
            assert!(film.gas_viscosity.substeps > 0 && film.gas_viscosity.block_steps > 0);
            assert!(film.gas_viscosity.dissipated_heat > 0.0);
            assert!(film.gas_wall_viscosity.substeps > 0 && film.gas_wall_viscosity.face_steps > 0);
            assert!(film.gas_wall_viscosity.dissipated_heat > 0.0);
            assert_eq!(film.gas_wall_viscosity.work_on_gas, 0.0);
            assert!(
                film.gas_wall_viscosity
                    .wall_impulse
                    .iter()
                    .any(|v| *v != 0.0)
            );
        }
        let reset = demo.restart().unwrap();
        assert!(reset.films.is_some());
        assert_eq!(reset.steps, 0);
        for film in reset.films.unwrap() {
            assert_eq!(film.thermal.mixture().film().total_mass(), 0.0);
            assert_eq!(film.captured_sensible_energy, 0.);
            assert!(film.thermal.energies_j().iter().all(|e| *e == 0.));
            assert_eq!(film.captured, 0);
            assert_eq!(film.fragments, 0);
            assert_eq!(film.merges, 0);
            assert_eq!(film.released_surface_energy, 0.0);
            assert_eq!(film.unresolved_energy, 0.0);
            assert_eq!(film.unresolved_angular_momentum, [0.0; 3]);
            assert_eq!(film.gas_drag, FiniteDropletDragReport::default());
            assert_eq!(film.gas_grid, impact_gas_grid().unwrap());
            assert_eq!(film.gas_flow, GasGridFlowReport::default());
            assert_eq!(film.gas_heat, GasGridHeatReport::default());
            assert_eq!(film.gas_viscosity, GasGridViscosityReport::default());
            assert_eq!(
                film.gas_wall_viscosity,
                GasGridWallViscosityReport::default()
            );
            assert_eq!(film.flight, DropletFlightReport::default());
        }
    }
    #[test]
    fn both_reservoirs_spread_and_surface_stays_within_reserved_capacity() {
        let mut demo = LiquidDemo::new().unwrap();
        assert!(!demo.mesh().unwrap().vertices().is_empty());
        for _ in 0..30 {
            demo.advance(1.0 / 120.0).unwrap();
        }
        assert!(
            demo.liquids
                .iter()
                .all(|liquid| liquid.particles().iter().any(|p| p.position[1] > 0.55))
        );
        assert!(demo.emitted_mass[0] > 0.0 && demo.emitted_mass[1] > 0.0);
        assert!(demo.mesh().unwrap().vertices().len() <= 60_024);
        for _ in 0..90 {
            demo.advance(1.0 / 120.0).unwrap();
        }
        demo.verify().unwrap();
        assert!(demo.mesh().unwrap().vertices().len() <= 60_024);
    }
}
