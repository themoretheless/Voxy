//! Transactional scene-liquid runtime preparation, binding validation and pose publication.
use super::*;

fn scene_rotation_config() -> physics::spin_path::Config {
    physics::spin_path::Config {
        max_angular_error_rad: 1e-5,
        min_step_s: 1e-9,
        max_arcs: 10000,
        max_trials: 30000,
    }
}

impl SceneLiquidRuntime {
    /// Resolve durable material names once and admit every source, including inactive ones.
    /// # Errors
    /// Duplicate/empty material names, invalid physics/source settings or source budget.
    pub fn new(
        scene: &SceneGraph,
        materials: Vec<(String, Material)>,
        config: Config,
        max_sources: usize,
    ) -> Result<Self, GameplayError> {
        let mut slots = BTreeMap::new();
        for (slot, (name, _)) in materials.iter().enumerate() {
            if name.is_empty() || slots.insert(name.clone(), slot).is_some() {
                return Err("duplicate or empty liquid material identity".into());
            }
        }
        for (node, _) in scene.components::<crate::LiquidMassDistribution>() {
            if scene
                .component::<crate::LiquidBody>(node)
                .map_err(|e| format!("mass owner: {e:?}"))?
                .is_none()
            {
                return Err("mass distribution requires a liquid body owner".into());
            }
        }
        for (node, _) in scene.components::<crate::RigidPointLoads>() {
            if scene
                .component::<crate::LiquidBody>(node)
                .map_err(|e| format!("point load owner: {e:?}"))?
                .is_none()
            {
                return Err("point loads require a liquid body owner".into());
            }
        }
        let owners: Vec<_> = scene.components::<crate::LiquidBody>().collect();
        if owners.len() > 128 {
            return Err("scene liquid translating-body budget exceeded (128)".into());
        }
        let body = owners
            .iter()
            .map(|(node, descriptor)| {
                if !descriptor.mass_kg.is_finite()
                    || descriptor.mass_kg <= 0.
                    || descriptor
                        .initial_velocity_m_s
                        .iter()
                        .any(|v| !v.is_finite())
                {
                    return Err("invalid liquid body mass/velocity".into());
                }
                if scene
                    .parent(*node)
                    .map_err(|e| format!("liquid body parent: {e:?}"))?
                    .is_some()
                    || scene
                        .component::<crate::CharacterBody>(*node)
                        .map_err(|e| format!("liquid body ownership: {e:?}"))?
                        .is_some()
                    || scene
                        .component::<crate::AngularMotion>(*node)
                        .map_err(|e| format!("liquid body ownership: {e:?}"))?
                        .is_some()
                {
                    return Err("liquid body requires an exclusively owned root transform".into());
                }
                let colliders = body_colliders(scene, *node)?;
                let world =
                    crate::static_world(scene).map_err(|e| format!("liquid environment: {e:?}"))?;
                let layers = crate::CollisionLayers::on(scene, *node);
                for own in world
                    .0
                    .iter()
                    .filter(|b| colliders.iter().any(|c| c.node == b.owner))
                {
                    for obstacle in &world.0 {
                        if colliders.iter().any(|c| c.node == obstacle.owner)
                            || obstacle
                                .shape
                                .penetration_affine(own.shape.center, own.shape.edges)
                                .is_none()
                        {
                            continue;
                        }
                        let mut root = obstacle.owner;
                        while let Some(parent) = scene
                            .parent(root)
                            .map_err(|e| format!("liquid collider parent: {e:?}"))?
                        {
                            root = parent;
                        }
                        let obstacle_layers = if owners.iter().any(|(owner, _)| *owner == root) {
                            crate::CollisionLayers::on(scene, root)
                        } else {
                            obstacle.layers
                        };
                        if layers.interacts(obstacle_layers) {
                            return Err("liquid body initially overlaps scene geometry".into());
                        }
                    }
                }
                let published = scene
                    .local(*node)
                    .map_err(|e| format!("liquid body pose: {e:?}"))?;
                let mass_descriptor = scene
                    .component::<crate::LiquidMassDistribution>(*node)
                    .map_err(|e| format!("mass descriptor: {e:?}"))?
                    .cloned();
                let mass_properties = mass_descriptor
                    .as_ref()
                    .map(|d| d.prepare(descriptor.mass_kg, published))
                    .transpose()?;
                let rigid_frame = mass_properties
                    .map(|p| crate::RigidBodyFrame::new(published, p))
                    .transpose()
                    .map_err(|e| format!("mass principal frame: {e:?}"))?;
                let pivot = position(scene, *node)?;
                let state = if let Some(frame) = rigid_frame {
                    frame
                        .prepare_body(pivot, descriptor.initial_velocity_m_s, [0.; 3])
                        .map_err(|e| format!("rigid body seed: {e:?}"))?
                } else {
                    physics::contact::ContactBody {
                        motion: physics::gravity::Body {
                            position: pivot,
                            velocity: descriptor.initial_velocity_m_s,
                            mass: descriptor.mass_kg,
                        },
                        spin: None,
                    }
                };
                let point_loads = scene
                    .component::<crate::RigidPointLoads>(*node)
                    .map_err(|e| format!("point load descriptor: {e:?}"))?
                    .cloned();
                if let Some(loads) = &point_loads {
                    loads.prepare(
                        rigid_frame.ok_or("point loads require an explicit rigid mass frame")?,
                        0.,
                        128,
                    )?;
                }
                let mut owner = BodyOwner {
                    node: *node,
                    descriptor: **descriptor,
                    state,
                    principal_templates: None,
                    published,
                    colliders,
                    mass_descriptor,
                    mass_properties,
                    rigid_frame,
                    point_loads,
                    point_elapsed_s: 0.,
                };
                if let (Some(properties), Some(spin)) = (owner.mass_properties, owner.state.spin) {
                    let inverse = glam::DQuat::from_array(spin.orientation).conjugate();
                    owner.principal_templates = Some(
                        owner
                            .colliders
                            .iter()
                            .map(|collider| {
                                let shape = collider_template(&owner, collider)?;
                                Ok(crate::convex::AffineBox {
                                    center: inverse
                                        * (shape.center
                                            - glam::DVec3::from_array(properties.center)),
                                    edges: shape.edges.map(|edge| inverse * edge),
                                })
                            })
                            .collect::<Result<Vec<_>, GameplayError>>()?,
                    );
                }
                Ok(owner)
            })
            .collect::<Result<Vec<_>, GameplayError>>()?;
        let liquid = Liquid::new(
            Vec::new(),
            materials.into_iter().map(|(_, m)| m).collect(),
            config,
        )
        .map_err(|e| format!("liquid world: {e:?}"))?;
        let mut sources = Vec::new();
        for (node, descriptor) in scene.components::<LiquidSource>() {
            if sources.len() >= max_sources {
                return Err("scene liquid source budget exceeded".into());
            }
            let slot = *slots.get(&descriptor.material_asset).ok_or_else(|| {
                format!("unresolved liquid material: {}", descriptor.material_asset)
            })?;
            let position = position(scene, node)?;
            let emitter = descriptor
                .prepare(position, slot)
                .map_err(|e| format!("liquid source: {e:?}"))?;
            sources.push((
                node,
                Source {
                    descriptor: descriptor.clone(),
                    emitter,
                    position,
                },
            ));
        }
        sources.sort_by_key(|(node, _)| *node);
        Ok(Self {
            scene: scene.identity(),
            sources,
            liquid,
            body,
        })
    }

    #[must_use]
    pub fn liquid(&self) -> &Liquid {
        &self.liquid
    }
    #[must_use]
    pub fn source_elapsed(&self, node: NodeId) -> Option<f64> {
        self.sources
            .iter()
            .find(|(id, _)| *id == node)
            .map(|(_, source)| source.emitter.elapsed())
    }

    /// Validate the runtime belongs to this scene and still owns its admitted sources.
    /// # Errors
    /// Foreign scenes, source addition/removal or descriptor edits require rebind.
    pub fn validate_bindings(&self, scene: &SceneGraph) -> Result<(), GameplayError> {
        if scene.identity() != self.scene {
            return Err("foreign scene liquid runtime".into());
        }
        let descriptors: BTreeMap<_, _> = scene.components::<LiquidSource>().collect();
        if descriptors.len() != self.sources.len()
            || self.sources.iter().any(|(node, source)| {
                descriptors
                    .get(node)
                    .is_none_or(|d| **d != source.descriptor)
            })
        {
            return Err("liquid source ownership or descriptor changed; rebind runtime".into());
        }
        let point_descriptors: BTreeMap<_, _> =
            scene.components::<crate::RigidPointLoads>().collect();
        if point_descriptors.len()
            != self
                .body
                .iter()
                .filter(|body| body.point_loads.is_some())
                .count()
            || self.body.iter().any(|body| {
                point_descriptors.get(&body.node).map(|v| &**v) != body.point_loads.as_ref()
            })
        {
            return Err("point load ownership or descriptor changed; rebind runtime".into());
        }
        let bodies: Vec<_> = scene.components::<crate::LiquidBody>().collect();
        if bodies.len() != self.body.len() {
            return Err("liquid body ownership changed; rebind runtime".into());
        }
        if scene.components::<crate::LiquidMassDistribution>().count()
            != self
                .body
                .iter()
                .filter(|b| b.mass_descriptor.is_some())
                .count()
        {
            return Err("mass distribution ownership changed; rebind runtime".into());
        }
        for body in &self.body {
            if scene
                .component::<crate::LiquidMassDistribution>(body.node)
                .map_err(|e| format!("mass binding: {e:?}"))?
                != body.mass_descriptor.as_ref()
            {
                return Err("mass distribution changed; rebind runtime".into());
            }
            if scene
                .component::<crate::CharacterBody>(body.node)
                .map_err(|e| format!("liquid body ownership: {e:?}"))?
                .is_some()
                || scene
                    .component::<crate::AngularMotion>(body.node)
                    .map_err(|e| format!("liquid body ownership: {e:?}"))?
                    .is_some()
            {
                return Err("liquid body has another transform owner".into());
            }
            if body_colliders(scene, body.node)? != body.colliders {
                return Err("compound collider bindings changed; rebind runtime".into());
            }
            if !bodies
                .iter()
                .any(|(node, descriptor)| *node == body.node && **descriptor == body.descriptor)
                || scene
                    .local(body.node)
                    .map_err(|e| format!("liquid body pose: {e:?}"))?
                    != body.published
                || scene
                    .parent(body.node)
                    .map_err(|e| format!("liquid body parent: {e:?}"))?
                    .is_some()
            {
                return Err("liquid body descriptor/pose changed; rebind runtime".into());
            }
        }
        Ok(())
    }

    /// Prepare emission, fluid/body motion and scene publication as one transaction.
    /// # Errors
    /// Any preparation/publication failure preserves this runtime and the scene pose.
    pub fn tick_and_publish(
        &mut self,
        scene: &mut SceneGraph,
        dt: f64,
        container: Option<Container>,
    ) -> Result<SceneLiquidStep, GameplayError> {
        self.tick_and_publish_with_dynamics(
            scene,
            dt,
            container,
            Default::default(),
            scene_rotation_config(),
        )
    }

    /// Prepare and atomically publish with explicit restitution/contact and angular budgets.
    /// The normal angular solver currently requires zero tangential friction.
    /// # Errors
    /// Invalid settings, unresolved contacts or publication errors preserve both owners and scene.
    pub fn tick_and_publish_with_dynamics(
        &mut self,
        scene: &mut SceneGraph,
        dt: f64,
        container: Option<Container>,
        dynamics: physics::liquid::DynamicWorldConfig,
        rotation: physics::spin_path::Config,
    ) -> Result<SceneLiquidStep, GameplayError> {
        let mut candidate = self.clone();
        let report = candidate.tick_with_dynamics(scene, dt, container, dynamics, rotation)?;
        candidate.publish_body_pose(scene)?;
        *self = candidate;
        Ok(report)
    }

    /// Publish an already prepared body pose after the host's other preparation succeeds.
    /// # Errors
    /// Foreign/edited scene bindings or unrepresentable pose reject before mutation.
    pub fn publish_body_pose(&mut self, scene: &mut SceneGraph) -> Result<(), GameplayError> {
        self.validate_bindings(scene)?;
        let poses = self
            .body
            .iter()
            .map(|body| {
                let mut pose = body.published;
                if let Some(frame) = body.rigid_frame {
                    let radius = body
                        .principal_templates
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|s| {
                            s.center.length() + s.edges.iter().map(|e| e.length()).sum::<f64>()
                        })
                        .fold(0., f64::max);
                    pose = frame
                        .prepare_pose(body.state, radius, 1e-5)
                        .map_err(|e| format!("rigid pose publication: {e:?}"))?
                        .pose;
                } else {
                    pose.translation =
                        glam::DVec3::from_array(body.state.motion.position).as_vec3();
                }
                pose.matrix()
                    .map_err(|e| format!("liquid body publication: {e:?}"))?;
                Ok((body.node, pose))
            })
            .collect::<Result<Vec<_>, GameplayError>>()?;
        scene
            .set_locals(&poses)
            .map_err(|e| format!("liquid body publication: {e:?}"))?;
        for (body, (_, pose)) in self.body.iter_mut().zip(poses) {
            body.published = pose;
        }
        Ok(())
    }
    /// First admitted body for compatibility; use `body_states` for the entire world.
    #[must_use]
    pub fn body_state(&self) -> Option<(NodeId, physics::liquid::TranslatingBody)> {
        self.body_states().next()
    }

    /// All admitted COM states (legacy bodies use the root pivot), including paused
    /// inactive bodies, in stable owner order. Rotation is exposed by `body_rigid_states`.
    pub fn body_states(
        &self,
    ) -> impl Iterator<Item = (NodeId, physics::liquid::TranslatingBody)> + '_ {
        self.body.iter().map(|b| {
            (
                b.node,
                physics::liquid::TranslatingBody {
                    position: b.state.motion.position,
                    velocity: b.state.motion.velocity,
                    mass: b.state.motion.mass,
                },
            )
        })
    }

    /// Admitted mass tensor and COM offset in the root-relative world-oriented frame.
    /// Legacy bodies without an explicit distribution have no inferred inertia.
    pub fn body_mass_properties(
        &self,
    ) -> impl Iterator<Item = (NodeId, Option<physics::mass_properties::MassProperties>)> + '_ {
        self.body.iter().map(|b| (b.node, b.mass_properties))
    }

    /// Persistent mechanical COM states, including intrinsic rotation when mass is authored.
    pub fn body_rigid_states(
        &self,
    ) -> impl Iterator<Item = (NodeId, physics::contact::ContactBody)> + '_ {
        self.body.iter().map(|b| (b.node, b.state))
    }

    /// Admitted COM/principal adapters.
    pub fn body_rigid_frames(
        &self,
    ) -> impl Iterator<Item = (NodeId, Option<crate::RigidBodyFrame>)> + '_ {
        self.body.iter().map(|b| (b.node, b.rigid_frame))
    }

    /// Build the active rigid geometry world once for both dynamics and support queries.
    fn mechanical_world(
        &self,
        scene: &SceneGraph,
    ) -> Result<
        (
            SceneBodyWorld,
            Vec<usize>,
            Vec<physics::contact::ContactBody>,
        ),
        GameplayError,
    > {
        let mut geometry = SceneGeometry(
            crate::static_world(scene).map_err(|e| format!("liquid collider: {e:?}"))?,
        );
        let mut active = Vec::new();
        let mut templates = Vec::new();
        let mut states = Vec::new();
        for (owner, body) in self.body.iter().enumerate() {
            if !scene
                .active_in_hierarchy(body.node)
                .map_err(|e| format!("liquid body activity: {e:?}"))?
            {
                continue;
            }
            let mut shapes = Vec::new();
            for (slot, collider) in body.colliders.iter().enumerate() {
                if let Some(index) = geometry.0.0.iter().position(|b| b.owner == collider.node) {
                    geometry.0.0.remove(index);
                    shapes.push(if let Some(principal) = &body.principal_templates {
                        principal[slot]
                    } else {
                        collider_template(body, collider)?
                    });
                }
            }
            active.push(owner);
            templates.push(shapes);
            states.push(body.state);
        }
        let body_layers: Vec<_> = active
            .iter()
            .map(|&owner| crate::CollisionLayers::on(scene, self.body[owner].node))
            .collect();
        let body_environments = body_layers
            .iter()
            .map(|&layers| match geometry.0.filtered(layers) {
                std::borrow::Cow::Borrowed(_) => None,
                std::borrow::Cow::Owned(world) => Some(SceneGeometry(world)),
            })
            .collect();
        let particle_environment = match geometry.0.filtered(crate::CollisionLayers::default()) {
            std::borrow::Cow::Borrowed(_) => None,
            std::borrow::Cow::Owned(world) => Some(SceneGeometry(world)),
        };
        Ok((
            SceneBodyWorld {
                boundaries: Vec::new(),
                environment: geometry,
                body_environments,
                particle_environment,
                body_layers,
                templates,
            },
            active,
            states,
        ))
    }

    /// Gather current supporting patches and resolve coupled reactions under
    /// configured gravity plus additional COM loads. This is read-only and does
    /// not certify or advance a constrained trajectory. Returned indices follow owners.
    /// # Errors
    /// Invalid/edited scene bindings, no active bodies, duplicate/foreign/inactive
    /// load owners, geometry overlap, limits or reaction nonconvergence.
    pub fn support_reactions(
        &self,
        scene: &SceneGraph,
        additional: &[(NodeId, physics::contact::ContactWrench)],
        limits: physics::liquid::DynamicWorldConfig,
        config: physics::contact::ReactionConfig,
    ) -> Result<SceneSupportReactions, GameplayError> {
        self.validate_bindings(scene)?;
        let (world, active, states) = self.mechanical_world(scene)?;
        let owners: Vec<_> = active.iter().map(|i| self.body[*i].node).collect();
        let mut wrenches = vec![physics::contact::ContactWrench::default(); owners.len()];
        let mut used = Vec::new();
        for (node, wrench) in additional {
            let index = owners
                .iter()
                .position(|owner| owner == node)
                .ok_or_else(|| "support load has a foreign or inactive owner".to_string())?;
            if used.contains(&index) {
                return Err("duplicate support load owner".into());
            }
            used.push(index);
            wrenches[index] = *wrench;
        }
        for (index, owner_index) in active.iter().enumerate() {
            let owner = &self.body[*owner_index];
            if let Some(loads) = &owner.point_loads {
                let points = loads.prepare(
                    owner.rigid_frame.ok_or("point load mass frame missing")?,
                    owner.point_elapsed_s,
                    128,
                )?;
                let moment = physics::rigid_motion::MaterialForceMoment::aggregate(points)
                    .map_err(|e| format!("point load resultant: {e:?}"))?;
                let orientation = states[index]
                    .spin
                    .ok_or("point loads require intrinsic rotation")?
                    .orientation;
                let torque = moment
                    .torque_at(orientation, 0.)
                    .map_err(|e| format!("point load moment: {e:?}"))?;
                let old = wrenches[index];
                let total = physics::rigid_motion::MotionLoad::aggregate([
                    physics::rigid_motion::MotionLoad {
                        force: old.force,
                        torque: physics::rotation::TorquePolynomial::constant(old.torque),
                        ..physics::rigid_motion::MotionLoad::zero()
                    },
                    physics::rigid_motion::MotionLoad {
                        force: moment.force,
                        torque: physics::rotation::TorquePolynomial::constant(torque),
                        ..physics::rigid_motion::MotionLoad::zero()
                    },
                ])
                .map_err(|e| format!("support load aggregation: {e:?}"))?;
                wrenches[index] = physics::contact::ContactWrench {
                    force: total.force,
                    torque: total.torque.value,
                };
            }
        }
        let reactions = self
            .liquid
            .rigid_world_reactions(&states, &world, &wrenches, limits, config)
            .map_err(|e| format!("scene support reactions: {e:?}"))?;
        Ok(SceneSupportReactions { owners, reactions })
    }

    /// Emit then simulate on an explicitly supplied fixed interval. No caller state changes.
    /// World positions follow scene transforms; nozzle velocity is displacement / interval.
    /// Inactive sources track position without advancing their pulse clock.
    /// # Errors
    /// Foreign scene, descriptor/ownership edits, invalid time, emission or physics failure.
    /// The entire world and every source clock remain unchanged on any error.
    pub fn tick(
        &mut self,
        scene: &SceneGraph,
        dt: f64,
        container: Option<Container>,
    ) -> Result<SceneLiquidStep, GameplayError> {
        self.tick_with_dynamics(
            scene,
            dt,
            container,
            Default::default(),
            scene_rotation_config(),
        )
    }

    /// Whether the shared solver owns finite bodies in this scene.
    #[must_use]
    pub fn couples_kinematic_boundaries(&self) -> bool {
        !self.body.is_empty()
    }

    /// Simulate against the platform trajectories admitted for this fixed step.
    /// # Errors
    /// Rejects stale platform receipts or unresolved continuous contacts.
    pub fn tick_with_platform_motion(
        &mut self,
        scene: &SceneGraph,
        dt: f64,
        motion: &crate::PlatformMotion,
    ) -> Result<SceneLiquidStep, GameplayError> {
        if motion.boundaries.is_empty() {
            return self.tick(scene, dt, None);
        }
        if motion.step_s != dt || motion.scene_revision != Some(scene.change_token()) {
            return Err("invalid platform motion receipt".into());
        }
        self.tick_impl(
            scene,
            dt,
            None,
            Default::default(),
            scene_rotation_config(),
            None,
            Some(motion),
        )
        .map(|report| report.step)
    }

    /// Advance coupled bodies and shorten a conflicting platform trajectory.
    /// Each trial starts from the same poses and a private body candidate.
    /// # Errors
    /// A rejected trial leaves the original body state and platform poses intact.
    pub fn resolve_platform_tick(
        &mut self,
        scene: &mut SceneGraph,
        characters: Option<&crate::CharacterPhysics>,
        dt: f64,
        motion: &mut crate::PlatformMotion,
    ) -> Result<(), GameplayError> {
        let first = match self.tick_with_platform_motion(scene, dt, motion) {
            Ok(_) => return Ok(()),
            Err(error) => {
                motion.rollback(scene)?;
                match error {
                    GameplayError::KinematicConstraint(owner) => owner,
                    other => return Err(other),
                }
            }
        };
        let probe = |scene: &mut SceneGraph, limits: &[(NodeId, f64)]| {
            let trial =
                crate::kinematic::step_kinematic_with_limits(scene, characters, dt, true, limits)?;
            let result = self.clone().tick_with_platform_motion(scene, dt, &trial);
            trial.rollback(scene)?;
            result.map(|_| ())
        };
        // Establish a feasible baseline by pausing only movers reported by the
        // solver. Other movers keep their original authored velocity.
        let mut limits = vec![(first, 0.)];
        loop {
            match probe(scene, &limits) {
                Ok(()) => break,
                Err(GameplayError::KinematicConstraint(owner))
                    if !limits.iter().any(|&(known, _)| known == owner) =>
                {
                    limits.push((owner, 0.));
                }
                Err(error) => return Err(error),
            }
        }
        for index in 0..limits.len() {
            let mut low = 0.;
            let mut high = 1.;
            for _ in 0..20 {
                let fraction = f64::midpoint(low, high);
                limits[index].1 = fraction;
                match probe(scene, &limits) {
                    Ok(()) => low = fraction,
                    Err(GameplayError::KinematicConstraint(_)) => high = fraction,
                    Err(error) => return Err(error),
                }
            }
            limits[index].1 = low;
        }
        *motion =
            crate::kinematic::step_kinematic_with_limits(scene, characters, dt, true, &limits)?;
        if let Err(error) = self.tick_with_platform_motion(scene, dt, motion) {
            motion.rollback(scene)?;
            return Err(error);
        }
        Ok(())
    }

    /// Simulate using caller-specified mechanical response and angular admission budgets.
    /// # Errors
    /// Same transactional binding, emission and geometry guarantees as tick.
    pub fn tick_with_dynamics(
        &mut self,
        scene: &SceneGraph,
        dt: f64,
        container: Option<Container>,
        dynamics: physics::liquid::DynamicWorldConfig,
        rotation: physics::spin_path::Config,
    ) -> Result<SceneLiquidStep, GameplayError> {
        self.tick_impl(scene, dt, container, dynamics, rotation, None, None)
            .map(|report| report.step)
    }

    /// Advance and publish through the existing scene owner/event loop with
    /// geometry-admitted finite normal reactions. Unsupported changing branches
    /// reject with complete runtime/scene rollback, including emitted sources.
    /// # Errors
    /// Invalid budgets/bindings, unresolved support evolution or pose publication.
    pub fn tick_and_publish_with_supported_dynamics(
        &mut self,
        scene: &mut SceneGraph,
        dt: f64,
        container: Option<Container>,
        dynamics: physics::liquid::DynamicWorldConfig,
        rotation: physics::spin_path::Config,
        supports: physics::liquid::SupportedWorldConfig,
    ) -> Result<SceneSupportedLiquidStep, GameplayError> {
        supports
            .validate()
            .map_err(|e| format!("supported liquid config: {e:?}"))?;
        let mut candidate = self.clone();
        let report = candidate.tick_impl(
            scene,
            dt,
            container,
            dynamics,
            rotation,
            Some(supports),
            None,
        )?;
        candidate.publish_body_pose(scene)?;
        *self = candidate;
        Ok(report)
    }

    fn tick_impl(
        &mut self,
        scene: &SceneGraph,
        dt: f64,
        container: Option<Container>,
        dynamics: physics::liquid::DynamicWorldConfig,
        rotation: physics::spin_path::Config,
        supports: Option<physics::liquid::SupportedWorldConfig>,
        platform: Option<&crate::PlatformMotion>,
    ) -> Result<SceneSupportedLiquidStep, GameplayError> {
        self.validate_bindings(scene)?;
        if !dt.is_finite() || dt <= 0. {
            return Err("invalid liquid fixed interval".into());
        }
        let mut candidate = self.clone();
        let mut emissions = Vec::new();
        for (node, source) in &mut candidate.sources {
            let current = position(scene, *node)?;
            source.emitter.template.particle.position = current;
            source.emitter.source_velocity =
                std::array::from_fn(|axis| (current[axis] - source.position[axis]) / dt);
            source.position = current;
            if scene
                .active_in_hierarchy(*node)
                .map_err(|e| format!("liquid source activity: {e:?}"))?
            {
                let exchange = source
                    .emitter
                    .advance(&mut candidate.liquid, dt)
                    .map_err(|e| format!("liquid source {node:?}: {e:?}"))?;
                emissions.push((*node, exchange));
            }
        }
        if !candidate.body.is_empty() && container.is_some() {
            return Err(
                "authored liquid bodies use scene colliders, not an extra container".into(),
            );
        }
        let (mut world, active, mut states) = candidate.mechanical_world(scene)?;
        if let Some(platform) = platform {
            world
                .boundaries
                .extend(
                    platform
                        .boundaries
                        .iter()
                        .map(|&(owner, displacement)| MovingBoundary {
                            owner,
                            displacement,
                            duration: dt,
                        }),
                );
        }
        let point_loads = active
            .iter()
            .map(|index| {
                let owner = &candidate.body[*index];
                owner.point_loads.as_ref().map_or(Ok(Vec::new()), |loads| {
                    loads.prepare(
                        owner.rigid_frame.ok_or("point load mass frame missing")?,
                        owner.point_elapsed_s,
                        128,
                    )
                })
            })
            .collect::<Result<Vec<_>, GameplayError>>()?;
        let has_points = point_loads.iter().any(|points| !points.is_empty());
        let mut support_report = None;
        let (physics, dynamics) = if states.is_empty() && world.boundaries.is_empty() {
            (
                candidate
                    .liquid
                    .step_with_geometry(
                        dt,
                        container,
                        world.particle_environment(),
                        dynamics.contact,
                    )
                    .map_err(|e| format!("scene liquid step: {e:?}"))?,
                None,
            )
        } else {
            let report = if let Some(config) = supports {
                let loads = vec![physics::contact::ContactWrench::default(); states.len()];
                let report = candidate
                    .liquid
                    .step_with_supported_rigid_body_point_forces(
                        dt,
                        &mut states,
                        &world,
                        dynamics,
                        128,
                        rotation,
                        &loads,
                        &point_loads,
                        config,
                    )
                    .map_err(|e| format!("scene supported liquid body step: {e:?}"))?;
                support_report = Some(report);
                report.rigid.world
            } else if has_points {
                let additional = vec![physics::contact::ContactWrench::default(); states.len()];
                candidate
                    .liquid
                    .step_with_rigid_body_point_forces(
                        dt,
                        &mut states,
                        &world,
                        dynamics,
                        128,
                        rotation,
                        &additional,
                        &point_loads,
                    )
                    .map_err(|error| world.impact_error(error, "scene point load step"))?
                    .world
            } else {
                candidate
                    .liquid
                    .step_with_rigid_body_world(dt, &mut states, &world, dynamics, 128, rotation)
                    .map_err(|error| world.impact_error(error, "scene liquid body step"))?
            };
            for (owner, state) in active.into_iter().zip(states) {
                candidate.body[owner].state = state;
                if candidate.body[owner].point_loads.is_some() {
                    candidate.body[owner].point_elapsed_s += dt;
                    if !candidate.body[owner].point_elapsed_s.is_finite() {
                        return Err("point load clock overflow".into());
                    }
                }
            }
            (report.dynamics.fluid, Some(report))
        };
        *self = candidate;
        Ok(SceneSupportedLiquidStep {
            step: SceneLiquidStep {
                emissions,
                physics,
                dynamics,
            },
            support: support_report,
        })
    }
}
