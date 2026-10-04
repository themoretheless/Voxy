//! Fallible pose preparation inside the character publication transaction.
use super::{
    AppliedCharacterTrajectoryMotion, CharacterPhysics, CharacterRigidTrajectoryMotion, CharacterCertifiedFadeMotion,
    PhysicsError, SupportQueryBudget, SupportWorld,
};
use glam::{DQuat, DVec3, Mat4, Vec3};
use voxy_input::InputMap;
use voxy_scene::{NodeId, SceneGraph};

#[derive(Clone, Copy, Debug)]
pub struct AcceptedCharacterPose {
    pub owner: NodeId,
    /// Exact matrix that rendering will observe after this tick publishes.
    pub world_matrix: Mat4,
    /// Solver precision, before narrowing to the scene's f32 transform.
    pub physical_center: DVec3,
    pub physical_rotation: DQuat,
    pub velocity: DVec3,
    pub grounded: bool,
}
#[derive(Debug)]
pub struct CharacterTickPreview {
    pub characters: Vec<AcceptedCharacterPose>,
    pub support: SupportWorld,
    /// Accepted trajectory prefixes, available before animator preparation.
    pub motions: Vec<AppliedCharacterTrajectoryMotion>,
}
#[derive(Debug, PartialEq)]
pub enum CharacterTickError<E> {
    Physics(PhysicsError),
    Preparation(E),
}
pub(super) type Prepare<'a> =
    dyn FnMut(&CharacterTickPreview, &mut SupportQueryBudget) -> Result<(), PhysicsError> + 'a;

impl CharacterPhysics {
    /// Prepares a candidate (e.g. corrected skin palettes) against accepted physics.
    /// It is returned only after scene, body state and input publish successfully.
    /// The callback receives no mutable scene or physics access. Callers must keep
    /// their own preparation state local and publish only the returned candidate.
    /// # Errors
    /// Physics or preparation failures preserve the scene, body state and input.
    pub fn fixed_step_with_preparation<T, E>(
        &mut self,
        scene: &mut SceneGraph,
        input: &mut InputMap,
        dt: f64,
        translations: &[(NodeId, Vec3)],
        paths: &[CharacterRigidTrajectoryMotion<'_>],
        prepare: impl FnOnce(&CharacterTickPreview, &mut SupportQueryBudget) -> Result<T, E>,
    ) -> Result<(Vec<AppliedCharacterTrajectoryMotion>, T), CharacterTickError<E>> {
        self.fixed_step_preparing(scene, input, dt, translations, paths, &[], prepare)
    }

    /// Admits certified fade fields and prepares their accepted animator prefix
    /// before scene/body/input publication. Callback failure rolls the tick back.
    pub fn fixed_step_with_certified_fade_preparation<T, E>(
        &mut self,
        scene: &mut SceneGraph,
        input: &mut InputMap,
        dt: f64,
        fades: &[CharacterCertifiedFadeMotion<'_>],
        prepare: impl FnOnce(&CharacterTickPreview, &mut SupportQueryBudget) -> Result<T, E>,
    ) -> Result<(Vec<AppliedCharacterTrajectoryMotion>, T), CharacterTickError<E>> {
        self.fixed_step_with_mixed_certified_fade_preparation(
            scene, input, dt, &[], &[], fades, prepare)
    }

    /// Combines ordinary root translations/rigid paths and certified fades in
    /// the same admission, query budget and scene/body/input transaction.
    pub fn fixed_step_with_mixed_certified_fade_preparation<T, E>(
        &mut self,
        scene: &mut SceneGraph,
        input: &mut InputMap,
        dt: f64,
        translations: &[(NodeId, Vec3)],
        ordinary_paths: &[CharacterRigidTrajectoryMotion<'_>],
        fades: &[CharacterCertifiedFadeMotion<'_>],
        prepare: impl FnOnce(&CharacterTickPreview, &mut SupportQueryBudget) -> Result<T, E>,
    ) -> Result<(Vec<AppliedCharacterTrajectoryMotion>, T), CharacterTickError<E>> {
        let path_count = ordinary_paths.len().checked_add(fades.len())
            .filter(|count| *count <= self.max_bodies)
            .ok_or(CharacterTickError::Physics(PhysicsError::InvalidMotion))?;
        let mut paths = Vec::with_capacity(path_count);
        paths.extend_from_slice(ordinary_paths);
        paths.extend(fades.iter().map(|request| CharacterRigidTrajectoryMotion {
            owner: request.owner, trajectory: &request.fade.approximation().path,
            scale: request.scale, basis: request.basis, origin: request.origin,
        }));
        self.fixed_step_preparing(scene, input, dt, translations, &paths, fades, prepare)
    }

    #[allow(clippy::too_many_arguments)]
    fn fixed_step_preparing<T, E>(
        &mut self,
        scene: &mut SceneGraph,
        input: &mut InputMap,
        dt: f64,
        translations: &[(NodeId, Vec3)],
        paths: &[CharacterRigidTrajectoryMotion<'_>],
        certified: &[CharacterCertifiedFadeMotion<'_>],
        prepare: impl FnOnce(&CharacterTickPreview, &mut SupportQueryBudget) -> Result<T, E>,
    ) -> Result<(Vec<AppliedCharacterTrajectoryMotion>, T), CharacterTickError<E>> {
        let mut prepare = Some(prepare);
        let mut candidate = None;
        let mut failure = None;
        let result = self.fixed_step_mixed(
            scene,
            input,
            dt,
            translations,
            paths,
            certified,
            Some(
                &mut |preview, budget| match prepare.take().expect("preparation runs exactly once")(
                    preview, budget,
                ) {
                    Ok(value) => {
                        candidate = Some(value);
                        Ok(())
                    }
                    Err(error) => {
                        failure = Some(error);
                        Err(PhysicsError::Solver)
                    }
                },
            ),
        );
        match result {
            Ok(receipts) => Ok((
                receipts,
                candidate.expect("successful tick prepared a candidate"),
            )),
            Err(error) => Err(failure.map_or(
                CharacterTickError::Physics(error),
                CharacterTickError::Preparation,
            )),
        }
    }
}
