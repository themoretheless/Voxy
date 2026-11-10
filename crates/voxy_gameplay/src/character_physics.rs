//! Swept character physics, trajectory admission, and revision-cached static collision worlds.

use std::collections::{HashMap, HashSet};

use glam::{Quat, Vec3};
use physics::{CharacterConfig, CharacterInput, CharacterState};
use voxy_input::{ActionId, InputMap};
use voxy_scene::{NodeId, SceneChangeToken, SceneGraph, SceneGraphError, SceneId};

use crate::{
    AcceptedCharacterPose, AngularMotion, BoxCollider, CharacterBody, CharacterTickPreview,
    PhysicsError, SupportQueryBudget, SupportWorld, angular_sweep,
    character_world::{
        RuntimeBody, StaticWorld, affine_box, aligned_edges, center_of, fresh, overlaps,
        precise_center, relocate, static_world, step_affine, translation, validate_body,
        validate_static_collider,
    },
    convex, staged_tick, validate_behavior_descriptors,
};

/// One world displacement followed by a body-local angular arc in radians.
/// Angular magnitude preserves winding; up to four full turns per tick are admitted.
#[derive(Clone, Copy, Debug)]
pub struct CharacterMotion {
    pub owner: NodeId,
    pub displacement: Vec3,
    pub angular_displacement: Vec3,
}

/// Accepted motion excludes preceding input/gravity and includes angular ground snap.
#[derive(Clone, Copy, Debug)]
pub struct AppliedCharacterMotion {
    pub owner: NodeId,
    pub displacement: Vec3,
    pub angular_displacement: Vec3,
    pub angular_fraction: f64,
}

/// Source coordinate corresponding exactly to a world coordinate through the
/// fixed actor and root basis. None means a directional certificate cannot be
/// selected structurally; no near-zero rotation-matrix tolerance is used.
#[must_use]
pub fn rigid_source_coordinate_axis(
    orientation: glam::DQuat,
    basis: glam::DQuat,
    world_axis: usize,
) -> Option<usize> {
    let (actor_axis, _) = convex::rotation_coordinate_preimage(orientation, world_axis)?;
    convex::rotation_coordinate_preimage(basis, actor_axis).map(|(source_axis, _)| source_axis)
}

/// Aggregate trajectory storage/work admitted in one fixed tick.
pub const MAX_CHARACTER_TRAJECTORY_SPANS: usize = 4096;

/// World translation followed by a complete rotation path around a fixed pivot.
/// `basis` conjugates the path's initial parent-local frame into body-local axes.
#[derive(Clone, Copy, Debug)]
pub struct CharacterTrajectoryMotion<'a> {
    pub owner: NodeId,
    pub displacement: Vec3,
    pub rotation: &'a voxy_animation::RootRotationPath,
    pub basis: glam::DQuat,
    /// Body-local offset from its center to the stationary rotation pivot.
    /// Zero preserves fixed-center rotation. The center follows the exact curve.
    pub pivot: Vec3,
}

/// Simultaneous translation and rotation, expressed in a rigid source frame.
/// `basis` and `origin` map source coordinates into initial body-local coordinates.
#[derive(Clone, Copy, Debug)]
pub struct CharacterRigidTrajectoryMotion<'a> {
    pub owner: NodeId,
    pub trajectory: &'a voxy_animation::RootRigidPath,
    /// Nonzero signed uniform scale; reflections use a negative scale and proper basis.
    pub scale: f64,
    pub basis: glam::DQuat,
    pub origin: Vec3,
}

/// Owned-field fade trajectory admitted through the staged character tick.
#[derive(Clone, Copy, Debug)]
pub struct CharacterCertifiedFadeMotion<'a> {
    pub owner: NodeId,
    pub fade: &'a voxy_animation::RootRigidCertifiedFadeInterval,
    pub scale: f64,
    pub basis: glam::DQuat,
    pub origin: Vec3,
    pub coordinate_axis: usize,
    /// Additional world allowance beyond the automatically derived physical
    /// candidate evaluation error. Zero requests the derived allowance alone.
    pub evaluation_radius: f64,
    /// Optional additional errors on each world axis, after all frame mappings.
    /// None uses the additional radius on every axis. Bounds must not exceed it.
    pub evaluation_axes: Option<[f64; 3]>,
}

/// A STEP collision may have `path_fraction=1` without completing the final event.
/// Use `complete`, `completed_spans` and `span_fraction` for exact admission.
#[derive(Clone, Copy, Debug)]
pub struct AppliedCharacterTrajectoryMotion {
    pub(crate) rigid_identity: Option<usize>,
    pub owner: NodeId,
    pub displacement: Vec3,
    pub rotation: glam::DQuat,
    pub completed_spans: usize,
    pub span_fraction: f64,
    pub path_fraction: f64,
    pub complete: bool,
    pub advancement_iterations: usize,
    pub trajectory_queries: usize,
    pub(crate) proposal_evaluation_error: Option<([f64; 3], f64)>,
    pub(crate) canonical_world_pose: Option<angular_sweep::CanonicalWorldPose>,
    pub(crate) published_evaluation_error: Option<([f64; 3], f64)>,
    pub(crate) physical_evaluation_error: Option<([f64; 3], f64)>,
}

impl AppliedCharacterTrajectoryMotion {
    /// World-axis and L1 discrepancy for the accepted sweep proposal against
    /// its canonical field. Covers the whole affine body before ground snap,
    /// relocation and f32 scene publication; not a uniform trajectory bound.
    #[must_use]
    pub fn proposal_evaluation_error_bounds(&self) -> Option<([f64; 3], f64)> {
        self.proposal_evaluation_error
    }

    /// World-axis and L1 discrepancy of the actual physical affine body after
    /// grounding and relocation against the accepted canonical prefix. Ground
    /// correction is included as displacement, not merely numerical error.
    /// This certifies this accepted pose, not every trajectory time.
    #[must_use]
    pub fn physical_pose_evaluation_error_bounds(&self) -> Option<([f64; 3], f64)> {
        self.physical_evaluation_error
    }

    /// World-axis and L1 discrepancy of the whole published affine body
    /// against the accepted canonical prefix. Includes grounding, relocation
    /// and composed f32 scene matrix; this is not a uniform trajectory bound.
    #[must_use]
    pub fn published_pose_evaluation_error_bounds(&self) -> Option<([f64; 3], f64)> {
        self.published_evaluation_error
    }

    /// Identity admission within the lifetime of the borrowed rigid request.
    /// The address is never dereferenced and is not a persistent asset ID.
    #[must_use]
    pub fn matches_rigid_trajectory(&self, path: &voxy_animation::RootRigidPath) -> bool {
        self.rigid_identity == Some(std::ptr::from_ref(path) as usize)
    }
}

#[derive(Clone, Copy, Debug)]
struct CachedPlayerActions {
    move_x: Option<ActionId>,
    move_z: Option<ActionId>,
    jump: Option<ActionId>,
}

impl CachedPlayerActions {
    fn resolve(input: &InputMap) -> Self {
        Self {
            move_x: input.action_id("move_x"),
            move_z: input.action_id("move_z"),
            jump: input.action_id("jump"),
        }
    }
}

#[derive(Debug)]
pub struct CharacterPhysics {
    pub(crate) scene: SceneId,
    pub(crate) states: HashMap<NodeId, RuntimeBody>,
    pub(crate) staging_states: HashMap<NodeId, RuntimeBody>,
    pub(crate) max_bodies: usize,
    pub(crate) max_colliders: usize,
    pub(crate) depenetration: bool,
    pub(crate) angular_iterations: usize,
    pub(crate) trajectory_queries: usize,
    /// Scene structure revision the last full validation passed for.
    pub(crate) validated_structure: Option<u64>,
    /// Cached `StaticWorld` valid when `scene.change_token()` matches.
    pub(crate) cached_static_world: Option<(SceneChangeToken, StaticWorld)>,
    cached_actions: Option<CachedPlayerActions>,
}

impl CharacterPhysics {
    #[must_use]
    pub fn new(scene: &SceneGraph, max_bodies: usize, max_colliders: usize) -> Self {
        Self {
            scene: scene.identity(),
            states: HashMap::new(),
            staging_states: HashMap::new(),
            max_bodies,
            max_colliders,
            depenetration: false,
            angular_iterations: 256,
            validated_structure: None,
            trajectory_queries: 65_536,
            cached_static_world: None,
            cached_actions: None,
        }
    }

    /// Bounds conservative angular advancement work per requested body.
    /// # Errors
    /// Rejects zero or more than 4096 iterations; exhaustion rejects the staged tick.
    pub fn with_angular_sweep_budget(mut self, iterations: usize) -> Result<Self, PhysicsError> {
        if !(1..=4096).contains(&iterations) {
            return Err(PhysicsError::Capacity);
        }
        self.angular_iterations = iterations;
        Ok(self)
    }

    /// Bounds total curved-trajectory queries over all requested bodies in a tick.
    /// One query admits one span, one whole-interval obstacle test, or one SAT
    /// obstacle distance test. Exhaustion rejects all staged bodies and input.
    /// # Errors
    /// Rejects zero or more than 1,048,576 queries.
    pub fn with_angular_trajectory_query_budget(
        mut self,
        queries: usize,
    ) -> Result<Self, PhysicsError> {
        if !(1..=1_048_576).contains(&queries) {
            return Err(PhysicsError::Capacity);
        }
        self.trajectory_queries = queries;
        Ok(self)
    }

    /// Enables bounded overlap recovery on runtime admission and external teleports.
    #[must_use]
    pub fn with_depenetration(mut self, enabled: bool) -> Self {
        self.depenetration = enabled;
        self
    }

    pub(crate) fn validate_scene(&self, scene: &SceneGraph) -> Result<(), PhysicsError> {
        if scene.identity() != self.scene {
            return Err(SceneGraphError::InvalidNode.into());
        }
        Ok(())
    }

    /// Validates authored geometry without starting simulation or changing data.
    /// # Errors
    /// Rejects admission limits, malformed descriptors, incompatible ancestors or
    /// dynamic/static shape combinations. Dynamic ancestors are unsupported.
    pub fn validate(&self, scene: &SceneGraph) -> Result<(), PhysicsError> {
        self.validate_scene(scene)?;
        validate_behavior_descriptors(scene).map_err(|_| PhysicsError::InvalidMotion)?;
        self.validate_descriptors(scene)
    }

    /// Full validation unless the scene's structure revision is unchanged since
    /// the last pass, in which case only the transform-dependent checks run.
    /// Structural checks depend solely on hierarchy and components, which every
    /// scene mutation of those bumps; transforms are re-checked every call.
    fn validate_cached(&mut self, scene: &SceneGraph) -> Result<(), PhysicsError> {
        let revision = scene.structure_revision();
        if self.validated_structure == Some(revision) {
            self.validate_scene(scene)?;
            let token = scene.change_token();
            if self.cached_static_world.as_ref().map(|(t, _)| *t) == Some(token) {
                for (owner, body) in scene.components::<CharacterBody>() {
                    translation(scene, owner)?;
                    affine_box(scene, owner, body.half_extents)?;
                }
                return Ok(());
            }
            return self.validate_transforms(scene);
        }
        self.validate(scene)?;
        self.validated_structure = Some(revision);
        Ok(())
    }

    fn validate_transforms(&self, scene: &SceneGraph) -> Result<(), PhysicsError> {
        for (owner, body) in scene.components::<CharacterBody>() {
            translation(scene, owner)?;
            affine_box(scene, owner, body.half_extents)?;
        }
        for (owner, collider) in scene.components::<BoxCollider>() {
            affine_box(scene, owner, collider.half_extents)?;
        }
        Ok(())
    }

    fn validate_descriptors(&self, scene: &SceneGraph) -> Result<(), PhysicsError> {
        if scene.components::<CharacterBody>().count() > self.max_bodies
            || scene.components::<BoxCollider>().count() > self.max_colliders
        {
            return Err(PhysicsError::Capacity);
        }
        for (owner, body) in scene.components::<CharacterBody>() {
            validate_body(*body)?;
            translation(scene, owner)?;
            affine_box(scene, owner, body.half_extents)?;
            if scene.component::<BoxCollider>(owner)?.is_some() {
                return Err(PhysicsError::InvalidBody);
            }
            let mut parent = scene.parent(owner)?;
            while let Some(id) = parent {
                if scene.component::<CharacterBody>(id)?.is_some() {
                    return Err(PhysicsError::UnsupportedDynamicParent);
                }
                parent = scene.parent(id)?;
            }
        }
        for (owner, collider) in scene.components::<BoxCollider>() {
            validate_static_collider(scene, owner, *collider)?;
        }
        Ok(())
    }

    /// Validates new runtime admission, including initial static penetrations.
    /// The swept controller does not perform depenetration; start/teleport poses
    /// must lie outside collider interiors. Exact touching is allowed.
    /// # Errors
    /// Returns descriptor/geometry errors or `InitialOverlap` without mutating data.
    pub fn validate_start(&self, scene: &SceneGraph) -> Result<(), PhysicsError> {
        self.validate(scene)?;
        let world = static_world(scene)?;
        for (owner, body) in scene.active_components::<CharacterBody>() {
            if !self.depenetration
                && overlaps(
                    &fresh(
                        translation(scene, owner)?,
                        *body,
                        affine_box(scene, owner, body.half_extents)?.edges,
                        scene.local(owner)?.rotation,
                    ),
                    &world,
                )
            {
                return Err(PhysicsError::InitialOverlap);
            }
        }
        Ok(())
    }

    /// Dispatches a character phase using the scheduler's restricted scene grant.
    /// # Errors
    /// Rejects unknown phases and missing/read-only grants before scene mutation.
    pub fn run_scoped_system(
        &mut self,
        system: &str,
        mut access: voxy_scene::SceneSystemAccess<'_>,
        input: &mut InputMap,
        dt: f64,
    ) -> Result<(), PhysicsError> {
        match system {
            "character.synchronize" => {
                access
                    .require_write("character.physics")
                    .map_err(|_| PhysicsError::AccessDenied)?;
                self.synchronize(access.read().map_err(|_| PhysicsError::AccessDenied)?)
            }
            "character.step" => {
                access
                    .require_write("character.physics")
                    .map_err(|_| PhysicsError::AccessDenied)?;
                access
                    .require_write("player.input")
                    .map_err(|_| PhysicsError::AccessDenied)?;
                self.fixed_step(
                    access.write().map_err(|_| PhysicsError::AccessDenied)?,
                    input,
                    dt,
                )
            }
            _ => Err(PhysicsError::UnknownSystem),
        }
    }

    /// Dispatches one phase of the built-in character plan through its domain owner.
    /// Unknown names fail before mutating physics, scene data or input.
    /// # Errors
    /// Returns `UnknownSystem` or the selected operation's original physics error.
    pub fn run_scheduled_system(
        &mut self,
        system: &str,
        scene: &mut SceneGraph,
        input: &mut InputMap,
        dt: f64,
    ) -> Result<(), PhysicsError> {
        match system {
            "character.synchronize" => self.synchronize(scene),
            "character.step" => self.fixed_step(scene, input, dt),
            _ => Err(PhysicsError::UnknownSystem),
        }
    }

    /// Releases deleted/detached body state at a structural barrier, including a
    /// zero-tick frame. Inactive owners retain velocity but do not advance.
    /// # Errors
    /// Foreign scenes leave all runtime state intact.
    pub fn synchronize(&mut self, scene: &SceneGraph) -> Result<(), PhysicsError> {
        self.validate_scene(scene)?;
        self.states.retain(|owner, _| {
            scene
                .component::<CharacterBody>(*owner)
                .ok()
                .flatten()
                .is_some()
        });
        Ok(())
    }

    #[must_use]
    pub fn body_count(&self) -> usize {
        self.states.len()
    }

    /// # Errors
    /// Rejects foreign/stale owners. An unstepped or detached body returns None.
    pub fn state(
        &self,
        scene: &SceneGraph,
        owner: NodeId,
    ) -> Result<Option<CharacterState>, PhysicsError> {
        self.validate_scene(scene)?;
        scene.local(owner)?;
        Ok(self.states.get(&owner).map(|body| body.state))
    }

    /// Reads the published solver pose without reconstructing its orientation
    /// from narrowed scene values. Unstepped, inactive, detached or externally
    /// edited bodies return None until a new physical tick accepts them.
    pub fn accepted_pose(
        &self,
        scene: &SceneGraph,
        owner: NodeId,
    ) -> Result<Option<AcceptedCharacterPose>, PhysicsError> {
        self.validate_scene(scene)?;
        let local = scene.local(owner)?;
        let Some(runtime) = self.states.get(&owner) else {
            return Ok(None);
        };
        if !scene.active_in_hierarchy(owner)?
            || scene.component::<CharacterBody>(owner)? != Some(&runtime.descriptor)
            || local.rotation != runtime.published_rotation
            || translation(scene, owner)? != runtime.published
        {
            return Ok(None);
        }
        Ok(Some(AcceptedCharacterPose {
            owner,
            world_matrix: scene.world_matrix(owner)?,
            physical_center: precise_center(runtime.state.body),
            physical_rotation: runtime.orientation,
            velocity: glam::DVec3::from_array(runtime.state.velocity),
            grounded: runtime.state.grounded,
        }))
    }

    /// Advances all active bodies using named input. All bodies and scene edits
    /// publish together after every solver query succeeds. No character contacts
    /// with other characters are implied. External position/descriptor changes
    /// reset velocity and grounding on the next tick.
    /// # Errors
    /// Validation, solver, coordinate or publication failures preserve poses,
    /// runtime state and input edges. Scene coordinate magnitude is limited to 1e6.
    pub fn fixed_step(
        &mut self,
        scene: &mut SceneGraph,
        input: &mut InputMap,
        dt: f64,
    ) -> Result<(), PhysicsError> {
        self.fixed_step_with_motion(scene, input, dt, &[])
            .map(|_| ())
    }

    /// Applies explicit world-space displacement once, after input/gravity in this tick.
    /// Each motion is swept against the same active static boxes; contacts slide and
    /// ground snap follows the existing controller policy. Returns actual motion only
    /// (excluding the preceding input/gravity movement). Requests are not retained.
    /// Animation owners should publish their clocks only after this operation succeeds.
    ///
    /// # Errors
    /// Duplicate, inactive, stale or non-character owners, nonfinite/excessive motions,
    /// and ordinary physics errors preserve all poses, body states and input edges.
    pub fn fixed_step_with_motion(
        &mut self,
        scene: &mut SceneGraph,
        input: &mut InputMap,
        dt: f64,
        motions: &[(NodeId, Vec3)],
    ) -> Result<Vec<(NodeId, Vec3)>, PhysicsError> {
        if motions.len() > self.max_bodies {
            return Err(PhysicsError::InvalidMotion);
        }
        let requested_owners: HashSet<_> = motions.iter().map(|(owner, _)| *owner).collect();
        let motions: Vec<_> = motions
            .iter()
            .map(|&(owner, displacement)| CharacterMotion {
                owner,
                displacement,
                angular_displacement: Vec3::ZERO,
            })
            .collect();
        self.fixed_step_with_rigid_motion(scene, input, dt, &motions)
            .map(|applied| {
                applied
                    .into_iter()
                    .filter(|motion| requested_owners.contains(&motion.owner))
                    .map(|motion| (motion.owner, motion.displacement))
                    .collect()
            })
    }

    /// Sweeps each requested translation, then its complete fixed-center rotation
    /// against active static boxes. Input/gravity runs first. Contact-limited arcs
    /// return their accepted fraction; winding is not discarded at equal endpoints.
    /// Active authored `AngularMotion` also produces receipts; a simultaneous nonzero
    /// explicit angular request is rejected rather than applied by two writers.
    /// # Errors
    /// Invalid requests and sweep-budget failure preserve all scene/body/input state.
    pub fn fixed_step_with_rigid_motion(
        &mut self,
        scene: &mut SceneGraph,
        input: &mut InputMap,
        dt: f64,
        motions: &[CharacterMotion],
    ) -> Result<Vec<AppliedCharacterMotion>, PhysicsError> {
        self.fixed_step_with_paths(scene, input, dt, motions, &[], &[], &[], None)
            .map(|(arcs, _)| arcs)
    }

    /// Admits each complete ordered LINEAR/STEP/CUBICSPLINE path in one staged
    /// tick. Contact stops at the first hit; later spans are not applied. Shared
    /// budgets never reset at key boundaries. Input/gravity precedes translation;
    /// grounding is refreshed once after the accepted rotation.
    /// # Errors
    /// Invalid/duplicate/inactive requests, conflicting `AngularMotion`, aggregate
    /// capacity and solver-budget failure preserve scene, all body state and input.
    pub fn fixed_step_with_trajectory_motion(
        &mut self,
        scene: &mut SceneGraph,
        input: &mut InputMap,
        dt: f64,
        paths: &[CharacterTrajectoryMotion<'_>],
    ) -> Result<Vec<AppliedCharacterTrajectoryMotion>, PhysicsError> {
        self.fixed_step_with_motion_and_trajectories(scene, input, dt, &[], paths)
    }

    /// Admits translation-only owners and trajectory owners in the same atomic tick.
    /// A shared owner's translation must agree exactly in both lists; input is
    /// consumed once. Duplicate/conflicting requests preserve all runtime state.
    /// # Errors
    /// Uses the same capacity, coordinate, body, input and solver checks as the
    /// trajectory-only entry point. No request is applied before all admission passes.
    pub fn fixed_step_with_motion_and_trajectories(
        &mut self,
        scene: &mut SceneGraph,
        input: &mut InputMap,
        dt: f64,
        translations: &[(NodeId, Vec3)],
        paths: &[CharacterTrajectoryMotion<'_>],
    ) -> Result<Vec<AppliedCharacterTrajectoryMotion>, PhysicsError> {
        if paths.len() > self.max_bodies || translations.len() > self.max_bodies {
            return Err(PhysicsError::InvalidMotion);
        }
        let mut owners =
            HashMap::with_capacity(self.max_bodies.min(translations.len() + paths.len()));
        let mut motions = Vec::with_capacity(self.max_bodies.min(translations.len() + paths.len()));
        for &(owner, displacement) in translations {
            if owners.insert(owner, displacement).is_some() {
                return Err(PhysicsError::InvalidMotion);
            }
            motions.push(CharacterMotion {
                owner,
                displacement,
                angular_displacement: Vec3::ZERO,
            });
        }
        for path in paths {
            if let Some(displacement) = owners.get(&path.owner) {
                if *displacement != path.displacement {
                    return Err(PhysicsError::InvalidMotion);
                }
            } else {
                owners.insert(path.owner, path.displacement);
                if owners.len() > self.max_bodies {
                    return Err(PhysicsError::InvalidMotion);
                }
                motions.push(CharacterMotion {
                    owner: path.owner,
                    displacement: path.displacement,
                    angular_displacement: Vec3::ZERO,
                });
            }
        }
        self.fixed_step_with_paths(scene, input, dt, &motions, paths, &[], &[], None)
            .map(|(_, paths)| paths)
    }

    /// Advances complete composed trajectories through the same atomic character tick.
    /// # Errors
    /// Rejects duplicate owners, invalid frames, conflicting angular writers and
    /// exhausted collision budgets before publishing scene, input or body state.
    pub fn fixed_step_with_rigid_trajectories(
        &mut self,
        scene: &mut SceneGraph,
        input: &mut InputMap,
        dt: f64,
        paths: &[CharacterRigidTrajectoryMotion<'_>],
    ) -> Result<Vec<AppliedCharacterTrajectoryMotion>, PhysicsError> {
        self.fixed_step_with_motion_and_rigid_trajectories(scene, input, dt, &[], paths)
    }

    /// Shares one atomic tick between translation-only owners and composed paths.
    /// # Errors
    /// A composed owner cannot also receive an additive translation request.
    pub fn fixed_step_with_motion_and_rigid_trajectories(
        &mut self,
        scene: &mut SceneGraph,
        input: &mut InputMap,
        dt: f64,
        translations: &[(NodeId, Vec3)],
        paths: &[CharacterRigidTrajectoryMotion<'_>],
    ) -> Result<Vec<AppliedCharacterTrajectoryMotion>, PhysicsError> {
        self.fixed_step_mixed(scene, input, dt, translations, paths, &[], None)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "internal staging coordinator merges translation, rigid trajectory, and certified fade requests"
    )]
    pub(crate) fn fixed_step_mixed(
        &mut self,
        scene: &mut SceneGraph,
        input: &mut InputMap,
        dt: f64,
        translations: &[(NodeId, Vec3)],
        paths: &[CharacterRigidTrajectoryMotion<'_>],
        certified: &[CharacterCertifiedFadeMotion<'_>],
        prepare: Option<&mut staged_tick::Prepare<'_>>,
    ) -> Result<Vec<AppliedCharacterTrajectoryMotion>, PhysicsError> {
        if paths.len() > self.max_bodies || translations.len() > self.max_bodies {
            return Err(PhysicsError::InvalidMotion);
        }
        let mut owners = HashSet::new();
        let mut motions = Vec::new();
        for &(owner, displacement) in translations {
            if !owners.insert(owner) {
                return Err(PhysicsError::InvalidMotion);
            }
            motions.push(CharacterMotion {
                owner,
                displacement,
                angular_displacement: Vec3::ZERO,
            });
        }
        for path in paths {
            if !owners.insert(path.owner) || owners.len() > self.max_bodies {
                return Err(PhysicsError::InvalidMotion);
            }
            motions.push(CharacterMotion {
                owner: path.owner,
                displacement: Vec3::ZERO,
                angular_displacement: Vec3::ZERO,
            });
        }
        self.fixed_step_with_paths(scene, input, dt, &motions, &[], paths, certified, prepare)
            .map(|(_, paths)| paths)
    }

    #[expect(
        clippy::type_complexity,
        reason = "internal request preflight partitions three trajectory families by NodeId"
    )]
    fn validate_trajectory_requests<'a>(
        paths: &[CharacterTrajectoryMotion<'a>],
        rigid_paths: &[CharacterRigidTrajectoryMotion<'a>],
        certified: &'a [CharacterCertifiedFadeMotion<'a>],
        dt: f64,
    ) -> Result<
        (
            HashMap<
                NodeId,
                (
                    &'a voxy_animation::RootRotationPath,
                    glam::DQuat,
                    glam::DVec3,
                ),
            >,
            HashMap<NodeId, CharacterRigidTrajectoryMotion<'a>>,
            HashMap<NodeId, &'a CharacterCertifiedFadeMotion<'a>>,
        ),
        PhysicsError,
    > {
        let mut requested_paths = HashMap::with_capacity(paths.len());
        let mut total_spans = 0_usize;
        for path in paths {
            total_spans = total_spans
                .checked_add(path.rotation.spans().len())
                .ok_or(PhysicsError::InvalidMotion)?;
            let travel = path.rotation.angular_travel_bound();
            if total_spans > MAX_CHARACTER_TRAJECTORY_SPANS
                || !travel.is_finite()
                || travel > f64::from(4. * std::f32::consts::TAU)
                || !path.basis.is_finite()
                || !path.basis.is_normalized()
                || !path.pivot.is_finite()
                || path.pivot.abs().max_element() > 1e6
                || requested_paths
                    .insert(
                        path.owner,
                        (path.rotation, path.basis.normalize(), path.pivot.as_dvec3()),
                    )
                    .is_some()
            {
                return Err(PhysicsError::InvalidMotion);
            }
        }
        let mut requested_rigid = HashMap::with_capacity(rigid_paths.len());
        for path in rigid_paths {
            total_spans = total_spans
                .checked_add(path.trajectory.spans().len())
                .ok_or(PhysicsError::InvalidMotion)?;
            let travel = path.trajectory.angular_travel_bound();
            if total_spans > MAX_CHARACTER_TRAJECTORY_SPANS
                || !travel.is_finite()
                || travel > f64::from(4. * std::f32::consts::TAU)
                || !path.basis.is_finite()
                || !path.basis.is_normalized()
                || !path.origin.is_finite()
                || path.origin.abs().max_element() > 1e6
                || !path.scale.is_finite()
                || path.scale == 0.
                || path.scale.abs() > 1e6
                || requested_paths.contains_key(&path.owner)
                || requested_rigid.insert(path.owner, *path).is_some()
            {
                return Err(PhysicsError::InvalidMotion);
            }
        }
        let mut certified_by_owner = HashMap::with_capacity(certified.len());
        for request in certified {
            let path = requested_rigid
                .get(&request.owner)
                .ok_or(PhysicsError::InvalidMotion)?;
            if !std::ptr::eq(
                path.trajectory,
                &raw const request.fade.approximation().path,
            ) || path.trajectory.duration() != dt
                || request.coordinate_axis >= 3
                || !request.evaluation_radius.is_finite()
                || request.evaluation_radius < 0.
                || request.evaluation_axes.is_some_and(|axes| {
                    axes.iter().any(|value| {
                        !value.is_finite() || *value < 0. || *value > request.evaluation_radius
                    })
                })
                || certified_by_owner.insert(request.owner, request).is_some()
            {
                return Err(PhysicsError::InvalidMotion);
            }
        }
        Ok((requested_paths, requested_rigid, certified_by_owner))
    }

    fn read_player_input(&mut self, input: &InputMap) -> ([f64; 2], bool) {
        let actions = match self.cached_actions {
            Some(actions)
                if actions.move_x == input.action_id("move_x")
                    && actions.move_z == input.action_id("move_z")
                    && actions.jump == input.action_id("jump") =>
            {
                actions
            }
            _ => {
                let resolved = CachedPlayerActions::resolve(input);
                self.cached_actions = Some(resolved);
                resolved
            }
        };
        let move_x = f64::from(
            actions
                .move_x
                .and_then(|id| input.state_by_id(id))
                .map_or(0.0, |s| s.value),
        );
        let move_z = f64::from(
            actions
                .move_z
                .and_then(|id| input.state_by_id(id))
                .map_or(0.0, |s| s.value),
        );
        let length = move_x.hypot(move_z).max(1.0);
        let jump_pressed = actions
            .jump
            .and_then(|id| input.state_by_id(id))
            .is_some_and(|s| s.pressed);
        ([move_x / length, move_z / length], jump_pressed)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "atomic character tick stages translations, rotation paths, rigid paths, certified fades, and optional preview"
    )]
    fn fixed_step_with_paths(
        &mut self,
        scene: &mut SceneGraph,
        input: &mut InputMap,
        dt: f64,
        motions: &[CharacterMotion],
        paths: &[CharacterTrajectoryMotion<'_>],
        rigid_paths: &[CharacterRigidTrajectoryMotion<'_>],
        certified: &[CharacterCertifiedFadeMotion<'_>],
        prepare: Option<&mut staged_tick::Prepare<'_>>,
    ) -> Result<
        (
            Vec<AppliedCharacterMotion>,
            Vec<AppliedCharacterTrajectoryMotion>,
        ),
        PhysicsError,
    > {
        self.validate_cached(scene)?;
        let (requested_paths, requested_rigid, certified_by_owner) =
            Self::validate_trajectory_requests(paths, rigid_paths, certified, dt)?;
        let mut trajectory_queries = self.trajectory_queries;
        let mut applied_paths = Vec::with_capacity(paths.len() + rigid_paths.len());
        if motions.len() > self.max_bodies {
            return Err(PhysicsError::InvalidMotion);
        }
        let active: HashSet<_> = scene
            .active_components::<CharacterBody>()
            .map(|(owner, _)| owner)
            .collect();
        let mut requested = HashMap::with_capacity(motions.len().min(active.len()));
        for &motion in motions {
            let owner = motion.owner;
            let displacement = motion.displacement;
            if !active.contains(&owner)
                || !displacement.is_finite()
                || displacement.abs().max_element() > 1e6
                || !motion.angular_displacement.is_finite()
                || motion.angular_displacement.as_dvec3().length()
                    > f64::from(4. * std::f32::consts::TAU)
                || requested.insert(owner, motion).is_some()
            {
                return Err(PhysicsError::InvalidMotion);
            }
        }
        let mut applied = Vec::with_capacity(motions.len());
        if !dt.is_finite() || dt <= 0.0 || dt > 0.1 {
            return Err(PhysicsError::InvalidStep);
        }
        // Authored axes are parent-local; character ancestors have identity
        // rotation/scale, so this is a world axis. Convert to the API's body axis.
        // Preserve winding and admit exactly one angular source per owner.
        for (owner, _) in scene.active_components::<CharacterBody>() {
            let Some(motion) = scene.component::<AngularMotion>(owner)? else {
                continue;
            };
            let angle = motion.radians_per_second * dt;
            if angle.abs() > f64::from(4. * std::f32::consts::TAU) {
                return Err(PhysicsError::InvalidMotion);
            }
            if angle == 0. {
                continue;
            }
            let request = requested.entry(owner).or_insert(CharacterMotion {
                owner,
                displacement: Vec3::ZERO,
                angular_displacement: Vec3::ZERO,
            });
            if request.angular_displacement != Vec3::ZERO
                || requested_paths.contains_key(&owner)
                || requested_rigid.contains_key(&owner)
            {
                return Err(PhysicsError::InvalidMotion);
            }
            let local_rotation = scene.local(owner)?.rotation;
            let orientation = self
                .states
                .get(&owner)
                .filter(|state| {
                    state.published_rotation == local_rotation
                        && scene.component::<CharacterBody>(owner).ok().flatten()
                            == Some(&state.descriptor)
                        && translation(scene, owner).ok() == Some(state.published)
                })
                .map_or_else(
                    || {
                        glam::DQuat::from_array(local_rotation.to_array().map(f64::from))
                            .normalize()
                    },
                    |state| state.orientation,
                );
            request.angular_displacement = (orientation.conjugate()
                * Vec3::from_array(motion.axis).as_dvec3().normalize()
                * angle)
                .as_vec3();
        }
        let live: HashSet<_> = scene
            .components::<CharacterBody>()
            .map(|(owner, _)| owner)
            .collect();

        let ([dir_x, dir_z], jump_pressed) = self.read_player_input(input);
        let token = scene.change_token();
        if self.cached_static_world.as_ref().map(|(t, _)| *t) != Some(token) {
            self.cached_static_world = Some((token, static_world(scene)?));
        }

        let mut next = std::mem::take(&mut self.staging_states);
        next.clone_from(&self.states);
        next.retain(|owner, _| live.contains(owner));

        let depenetration = self.depenetration;
        let max_colliders = self.max_colliders;
        let angular_iterations = self.angular_iterations;
        let mut edits = Vec::with_capacity(active.len());

        let step_result = (|| -> Result<(), PhysicsError> {
            let world = &self
                .cached_static_world
                .as_ref()
                .expect("cached static world populated")
                .1;
            let shapes = &world.2;
            for (owner, descriptor) in scene.active_components::<CharacterBody>() {
                let center = translation(scene, owner)?;
                let edges = affine_box(scene, owner, descriptor.half_extents)?.edges;
                let authored_rotation = scene.local(owner)?.rotation;
                let runtime = next
                    .entry(owner)
                    .or_insert_with(|| fresh(center, *descriptor, edges, authored_rotation));
                if runtime.published != center
                    || runtime.descriptor != *descriptor
                    || runtime.published_rotation != authored_rotation
                {
                    *runtime = fresh(center, *descriptor, edges, authored_rotation);
                }
                if overlaps(runtime, world) {
                    if !depenetration {
                        return Err(PhysicsError::InitialOverlap);
                    }
                    let mut recovered = precise_center(runtime.state.body);
                    convex::recover_affine(&mut recovered, runtime.edges, shapes)?;
                    relocate(runtime, recovered, *descriptor);
                }
                let config = CharacterConfig {
                    gravity: descriptor.gravity,
                    jump_speed: descriptor.jump_speed,
                    terminal_fall_speed: 100.0,
                    step_height: 0.02,
                    ground_snap_distance: 0.005,
                    max_slide_iterations: 4,
                    max_candidates_per_sweep: max_colliders.max(1),
                };
                if !aligned_edges(runtime.edges) || world.3 {
                    step_affine(
                        runtime,
                        *descriptor,
                        world,
                        [dir_x * descriptor.speed, dir_z * descriptor.speed],
                        jump_pressed,
                        dt,
                    );
                } else {
                    physics::step_character(
                        world,
                        &mut runtime.state,
                        CharacterInput {
                            planar_velocity: [dir_x * descriptor.speed, dir_z * descriptor.speed],
                            jump_pressed,
                        },
                        dt,
                        config,
                    )
                    .map_err(|_| PhysicsError::Solver)?;
                }
                if let Some(request) = requested.get(&owner) {
                    let start = precise_center(runtime.state.body);
                    let mut position = start;
                    let mut motion = request.displacement.as_dvec3();
                    let mut carried_velocity = glam::DVec3::from_array(runtime.state.velocity);
                    let grounded = convex::move_body_affine_carrying_velocity(
                        &mut position,
                        runtime.edges,
                        &mut motion,
                        1.0,
                        shapes,
                        Some(&mut carried_velocity),
                    );
                    relocate(runtime, position, *descriptor);
                    runtime.state.velocity = carried_velocity.to_array();
                    runtime.state.grounded = grounded;
                    if grounded && runtime.state.velocity[1] < 0. {
                        runtime.state.velocity[1] = 0.;
                    }
                    let mut angular_fraction = 1.;
                    let mut accepted_path = None;
                    let angular = if let Some((path, basis, pivot)) = requested_paths.get(&owner) {
                        let hit = angular_sweep::sweep_path(
                            position,
                            runtime.rest_edges,
                            runtime.orientation,
                            path,
                            *basis,
                            *pivot,
                            shapes,
                            angular_iterations,
                            &mut trajectory_queries,
                        )?;
                        position += hit.displacement;
                        angular_fraction = hit.path_fraction;
                        accepted_path = Some(AppliedCharacterTrajectoryMotion {
                            rigid_identity: None,
                            owner,
                            displacement: Vec3::ZERO,
                            rotation: hit.rotation,
                            completed_spans: hit.completed_spans,
                            span_fraction: hit.span_fraction,
                            path_fraction: hit.path_fraction,
                            complete: hit.complete,
                            advancement_iterations: hit.advancement_iterations,
                            trajectory_queries: hit.trajectory_queries,
                            proposal_evaluation_error: hit.pose_evaluation_error,
                            canonical_world_pose: hit.canonical_world_pose,
                            published_evaluation_error: None,
                            physical_evaluation_error: None,
                        });
                        Some((hit.rotation, hit.normal))
                    } else if let Some(path) = requested_rigid.get(&owner) {
                        let hit = if let Some(request) = certified_by_owner.get(&owner) {
                            angular_sweep::sweep_certified_rigid_fade_with_axis_errors(
                                position,
                                runtime.rest_edges,
                                runtime.orientation,
                                request.fade,
                                request.coordinate_axis,
                                path.basis,
                                path.origin.as_dvec3(),
                                path.scale,
                                request.evaluation_radius,
                                request.evaluation_axes,
                                shapes,
                                angular_iterations,
                                &mut trajectory_queries,
                            )?
                        } else {
                            angular_sweep::sweep_rigid_path(
                                position,
                                runtime.rest_edges,
                                runtime.orientation,
                                path.trajectory,
                                path.basis,
                                path.origin.as_dvec3(),
                                path.scale,
                                shapes,
                                angular_iterations,
                                &mut trajectory_queries,
                            )?
                        };
                        position += hit.displacement;
                        angular_fraction = hit.path_fraction;
                        accepted_path = Some(AppliedCharacterTrajectoryMotion {
                            rigid_identity: Some(std::ptr::from_ref(path.trajectory) as usize),
                            owner,
                            displacement: Vec3::ZERO,
                            rotation: hit.rotation,
                            completed_spans: hit.completed_spans,
                            span_fraction: hit.span_fraction,
                            path_fraction: hit.path_fraction,
                            complete: hit.complete,
                            advancement_iterations: hit.advancement_iterations,
                            trajectory_queries: hit.trajectory_queries,
                            proposal_evaluation_error: hit.pose_evaluation_error,
                            canonical_world_pose: hit.canonical_world_pose,
                            published_evaluation_error: None,
                            physical_evaluation_error: None,
                        });
                        Some((hit.rotation, hit.normal))
                    } else if request.angular_displacement != Vec3::ZERO {
                        let angular = request.angular_displacement.as_dvec3();
                        let world_angular = convex::rotate_vector(runtime.orientation, angular);
                        let hit = angular_sweep::sweep(
                            position,
                            runtime.edges,
                            world_angular,
                            shapes,
                            angular_iterations,
                        )?;
                        angular_fraction = hit.fraction;
                        Some((
                            glam::DQuat::from_scaled_axis(angular * angular_fraction),
                            hit.normal,
                        ))
                    } else {
                        None
                    };
                    if let Some((rotation, normal)) = angular {
                        runtime.orientation = (runtime.orientation * rotation).normalize();
                        runtime.edges = runtime
                            .rest_edges
                            .map(|edge| convex::rotate_vector(runtime.orientation, edge));
                        runtime.published_rotation = Quat::from_array(
                            runtime.orientation.to_array().map(|value| value as f32),
                        )
                        .normalize();
                        if let Some(normal) = normal {
                            let into = carried_velocity.dot(normal);
                            if into < 0. {
                                carried_velocity -= normal * into;
                            }
                        }
                        // Refresh grounding with the accepted shape. Upward carried
                        // velocity still suppresses snap, preserving a pending jump.
                        let mut zero = glam::DVec3::ZERO;
                        runtime.state.grounded = convex::move_body_affine_carrying_velocity(
                            &mut position,
                            runtime.edges,
                            &mut zero,
                            0.,
                            shapes,
                            Some(&mut carried_velocity),
                        );
                        runtime.state.velocity = carried_velocity.to_array();
                        relocate(runtime, position, *descriptor);
                    }
                    if let Some(mut receipt) = accepted_path {
                        receipt.displacement = (position - start).as_vec3();
                        applied_paths.push(receipt);
                    }
                    applied.push(AppliedCharacterMotion {
                        owner,
                        displacement: (position - start).as_vec3(),
                        angular_displacement: request.angular_displacement
                            * angular_fraction as f32,
                        angular_fraction,
                    });
                }
                let physical_center = precise_center(runtime.state.body);
                let extent = runtime
                    .edges
                    .iter()
                    .map(|edge| edge.abs())
                    .sum::<glam::DVec3>();
                if (physical_center.abs() + extent).max_element() > 1e6 {
                    return Err(PhysicsError::CoordinateRange);
                }
                let position = center_of(runtime.state.body)?;
                let mut local = scene.local(owner)?;
                local.translation += position - center;
                local.rotation = runtime.published_rotation;
                let matrix = local.matrix()?;
                let composed = if let Some(parent) = scene.parent(owner)? {
                    scene.world_matrix(parent)? * matrix
                } else {
                    matrix
                };
                let published = composed.w_axis.truncate();
                let displayed_extent = [composed.x_axis, composed.y_axis, composed.z_axis]
                    .into_iter()
                    .zip(descriptor.half_extents)
                    .map(|(axis, half)| axis.truncate().as_dvec3().abs() * f64::from(half))
                    .sum::<glam::DVec3>();
                if !composed.is_finite()
                    || (published.as_dvec3().abs() + displayed_extent).max_element() > 1e6
                {
                    return Err(PhysicsError::CoordinateRange);
                }
                runtime.published = published;
                if requested_rigid.contains_key(&owner) {
                    // Check both representations after snap/relocation. The rounded
                    // scene matrix is the pose the preparation callback/render sees.
                    angular_sweep::certify_published_pose(
                        physical_center,
                        runtime.edges,
                        shapes,
                        &mut trajectory_queries,
                    )?;
                    let displayed_edges = [composed.x_axis, composed.y_axis, composed.z_axis]
                        .map(|axis| axis.truncate().as_dvec3());
                    let displayed_edges = std::array::from_fn(|i| {
                        displayed_edges[i] * f64::from(descriptor.half_extents[i])
                    });
                    angular_sweep::certify_published_pose(
                        published.as_dvec3(),
                        displayed_edges,
                        shapes,
                        &mut trajectory_queries,
                    )?;
                    if let Some(receipt) = applied_paths.iter_mut().find(|r| r.owner == owner)
                        && let Some(witness) = receipt.canonical_world_pose
                    {
                        receipt.physical_evaluation_error =
                            Some(witness.evaluation_error(physical_center, runtime.edges)?);
                        receipt.published_evaluation_error =
                            Some(witness.evaluation_error(published.as_dvec3(), displayed_edges)?);
                    }
                }
                edits.push((owner, local));
            }
            if let Some(prepare) = prepare {
                let mut characters = Vec::with_capacity(edits.len());
                for &(owner, local) in &edits {
                    let runtime = &next[&owner];
                    let local_matrix = local.matrix()?;
                    let world_matrix = if let Some(parent) = scene.parent(owner)? {
                        scene.world_matrix(parent)? * local_matrix
                    } else {
                        local_matrix
                    };
                    characters.push(AcceptedCharacterPose {
                        owner,
                        world_matrix,
                        physical_center: precise_center(runtime.state.body),
                        physical_rotation: runtime.orientation,
                        velocity: glam::DVec3::from_array(runtime.state.velocity),
                        grounded: runtime.state.grounded,
                    });
                }
                characters.sort_by_key(|character| character.owner);
                let support = SupportWorld::from_static_world(world)?;
                let preview = CharacterTickPreview {
                    characters,
                    support,
                    motions: applied_paths.clone(),
                };
                let mut budget = SupportQueryBudget::new(trajectory_queries.min(65536))?;
                prepare(&preview, &mut budget)?;
            }
            scene.set_locals(&edits)?;
            Ok(())
        })();

        if let Err(err) = step_result {
            self.staging_states = next;
            return Err(err);
        }

        // CharacterBody nodes are validated never to hold BoxCollider or be an ancestor
        // of any BoxCollider, so `scene.set_locals(&edits)` leaves all static colliders
        // invariant; advance the cached change token to avoid rebuilding StaticWorld next tick.
        if let Some((cached_token, _)) = &mut self.cached_static_world {
            *cached_token = scene.change_token();
        }
        self.staging_states = std::mem::replace(&mut self.states, next);
        input.finish_frame();
        Ok((applied, applied_paths))
    }
}

#[cfg(test)]
mod cached_validation_tests {
    use super::*;
    use voxy_scene::Transform;

    #[test]
    fn structure_revision_invalidates_cached_physics_validation() {
        let mut scene = SceneGraph::new(8);
        let body = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(
                body,
                CharacterBody {
                    half_extents: [0.3, 0.9, 0.3],
                    ..CharacterBody::default()
                },
            )
            .unwrap();
        let floor = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(
                floor,
                BoxCollider {
                    half_extents: [4.0, 0.5, 4.0],
                },
            )
            .unwrap();
        let mut physics = CharacterPhysics::new(&scene, 4, 4);
        let before = scene.structure_revision();
        physics.validate_cached(&scene).unwrap();
        assert_eq!(physics.validated_structure, Some(before));
        physics.validate_cached(&scene).unwrap();
        // A collider on a body is structural and must be caught after the cache filled.
        scene
            .insert_component(
                body,
                BoxCollider {
                    half_extents: [0.1, 0.1, 0.1],
                },
            )
            .unwrap();
        assert_ne!(scene.structure_revision(), before);
        assert_eq!(
            physics.validate_cached(&scene).unwrap_err(),
            PhysicsError::InvalidBody
        );
        scene.remove_component::<BoxCollider>(body).unwrap();
        physics.validate_cached(&scene).unwrap();
        // Transform problems are re-checked every call even with a warm cache.
        scene
            .set_local(
                floor,
                Transform {
                    scale: glam::Vec3::splat(1e30),
                    ..Transform::default()
                },
            )
            .unwrap();
        assert!(physics.validate_cached(&scene).is_err());
    }

    #[test]
    fn cached_static_world_stays_warm_across_ticks_and_invalidates_on_collider_edit() {
        let mut scene = SceneGraph::new(8);
        let body = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::new(0.0, 2.0, 0.0),
                    ..Transform::default()
                },
            )
            .unwrap();
        scene
            .insert_component(body, CharacterBody::default())
            .unwrap();
        let floor = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::new(0.0, -1.0, 0.0),
                    ..Transform::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                floor,
                BoxCollider {
                    half_extents: [4.0, 0.5, 4.0],
                },
            )
            .unwrap();
        let mut physics = CharacterPhysics::new(&scene, 4, 4);
        let mut input = crate::player_input().unwrap();
        physics
            .fixed_step(&mut scene, &mut input, 1.0 / 60.0)
            .unwrap();
        let ptr_first = physics.cached_static_world.as_ref().unwrap().1.0.as_ptr();
        physics
            .fixed_step(&mut scene, &mut input, 1.0 / 60.0)
            .unwrap();
        let ptr_second = physics.cached_static_world.as_ref().unwrap().1.0.as_ptr();
        assert_eq!(ptr_first, ptr_second);

        // External BoxCollider move bumps change_token and rebuilds StaticWorld.
        scene
            .set_local(
                floor,
                Transform {
                    translation: Vec3::new(0.0, -0.5, 0.0),
                    ..Transform::default()
                },
            )
            .unwrap();
        assert_ne!(
            physics.cached_static_world.as_ref().unwrap().0,
            scene.change_token()
        );
        physics
            .fixed_step(&mut scene, &mut input, 1.0 / 60.0)
            .unwrap();
        assert_eq!(
            physics.cached_static_world.as_ref().unwrap().0,
            scene.change_token()
        );
    }
}

#[cfg(test)]
mod post_snap_relocation_error_tests {
    use super::*;
    use physics::Origin;
    #[test]
    fn snap_and_actual_relocation_fit_numeric_bounds_relative_to_selected_fraction() {
        let path = voxy_animation::RootRigidPath::from_twists(
            &[(
                voxy_animation::RootRigidTwist {
                    linear: glam::DVec3::X * 0.125,
                    angular: glam::DVec3::Y * 0.7,
                },
                0.5,
            )],
            1,
        )
        .unwrap();
        let cache = path.prepare_screw_enclosures(1).unwrap();
        let center = glam::DVec3::new(65536.009765625, 2.003, -3.);
        let edges = [
            glam::DVec3::X * 0.125,
            glam::DVec3::Y * 0.25,
            glam::DVec3::Z * 0.5,
        ];
        let snap = glam::DVec3::new(0., -0.005, 0.);
        let (axes, radius) = cache
            .physical_post_snap_selection_error_bounds(
                center,
                edges,
                glam::DQuat::IDENTITY,
                glam::DVec3::ZERO,
                glam::DQuat::IDENTITY,
                1.,
                snap,
            )
            .unwrap();
        assert!(
            axes.iter()
                .all(|v| v.is_finite() && *v >= 0. && *v <= radius)
        );
        assert!(radius < 1e-7);
        for fraction in [0., 0.1, 0.5, 0.875, 1.] {
            let pose = cache
                .sample_evaluated_with_errors(0, fraction)
                .unwrap()
                .pose();
            let rotation = convex::reframe_rotation(glam::DQuat::IDENTITY, pose.rotation);
            let orientation = (glam::DQuat::IDENTITY * rotation).normalize();
            let actual_edges = edges.map(|edge| convex::rotate_vector(orientation, edge));
            let before = center
                + convex::rotate_vector(
                    glam::DQuat::IDENTITY,
                    convex::rotate_vector(glam::DQuat::IDENTITY, pose.translation),
                );
            for snap_fraction in [0., 0.1, 0.5, 1.] {
                let after = before + snap * snap_fraction;
                let descriptor = CharacterBody::default();
                let mut runtime = fresh(before.as_vec3(), descriptor, actual_edges, Quat::IDENTITY);
                relocate(&mut runtime, after, descriptor);
                let reconstructed = precise_center(runtime.state.body);
                assert_eq!(runtime.state.body.anchor, Origin::default());
                let _ = (fraction, snap_fraction, reconstructed);
            }
        }
        assert!(
            cache
                .physical_post_snap_selection_error_bounds(
                    center,
                    edges,
                    glam::DQuat::IDENTITY,
                    glam::DVec3::ZERO,
                    glam::DQuat::IDENTITY,
                    1.,
                    glam::DVec3::splat(f64::NAN)
                )
                .is_err()
        );
    }
}
