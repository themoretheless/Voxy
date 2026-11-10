//! Static ancestor similarity and parent-basis proofs for root motion and retargeting.

use voxy_model::ModelAsset;

use crate::{EditorError, ModelAnimation};

// A mapped ancestor must retain its bind transform across source clips and fades.
// Unmapped target ancestors are bind pose by the retarget contract.
pub(super) fn retarget_parent_basis(
    target: &ModelAsset,
    source: &ModelAsset,
    profile: &crate::ModelRetarget,
    root: u16,
    rotation: bool,
) -> Result<(glam::Mat4, glam::DQuat, f64), EditorError> {
    validate_retarget_motion_parents(target, source, profile, root)?;
    constant_parent_basis(
        target,
        &ModelAnimation {
            clip: None,
            root_motion_rotation: rotation,
            ..Default::default()
        },
        root,
    )
}

pub(super) fn validate_retarget_motion_parents(
    target: &ModelAsset,
    source: &ModelAsset,
    profile: &crate::ModelRetarget,
    root: u16,
) -> Result<(), EditorError> {
    let mut parent = target.skeleton.joints()[usize::from(root)].parent;
    while let Some(index) = parent {
        for mapping in &profile.joints {
            if target.resolve_joint_name(&mapping.target)? == index {
                let source_index = usize::from(source.resolve_joint_name(&mapping.source)?);
                if source
                    .animations
                    .iter()
                    .any(|clip| !clip.joint_uses_bind_pose(source_index))
                {
                    return Err(
                        "retarget motion parent is animated; interval basis is unproved".into(),
                    );
                }
            }
        }
        parent = target.skeleton.joints()[usize::from(index)].parent;
    }
    Ok(())
}

/// Compiles a static ancestor similarity from stored TRS values, enclosing
/// every rotation, translation and scale product before matrix rounding.
pub(super) fn constant_parent_similarity_enclosure(
    model: &ModelAsset,
    root: u16,
) -> Result<
    (
        voxy_animation::RootRigidEnclosure,
        voxy_animation::RootUniformScaleEnclosure,
    ),
    EditorError,
> {
    static_parent_similarity_enclosure(model, root, |index, joint| {
        let transform = model
            .animations
            .first()
            .map_or(Some(joint.bind_local), |clip| {
                clip.constant_joint_transform(index)
            })
            .ok_or("root parent reference is animated")?;
        if model
            .animations
            .iter()
            .any(|clip| clip.constant_joint_transform(index) != Some(transform))
        {
            return Err("root parent reference differs between clips".into());
        }
        Ok(transform)
    })
}

pub(super) fn static_parent_similarity_enclosure(
    model: &ModelAsset,
    root: u16,
    mut transform_at: impl FnMut(
        usize,
        &voxy_animation::Joint,
    ) -> Result<voxy_animation::Transform, EditorError>,
) -> Result<
    (
        voxy_animation::RootRigidEnclosure,
        voxy_animation::RootUniformScaleEnclosure,
    ),
    EditorError,
> {
    use voxy_animation::{RootRigidEnclosure, RootRigidTransform, RootUniformScaleEnclosure};
    let mut frame = RootRigidEnclosure::IDENTITY;
    let mut scale = RootUniformScaleEnclosure::ONE;
    let mut parent = model
        .skeleton
        .joints()
        .get(usize::from(root))
        .ok_or("invalid root parent reference joint")?
        .parent;
    while let Some(index) = parent {
        let index = usize::from(index);
        let joint = &model.skeleton.joints()[index];
        let transform = transform_at(index, joint)?;
        let magnitude = transform.scale.abs();
        if magnitude.x == 0. || magnitude.x != magnitude.y || magnitude.x != magnitude.z {
            return Err("root parent reference requires nonzero uniform scale".into());
        }
        let signs = transform.scale.signum();
        let determinant = signs.x * signs.y * signs.z;
        let proper = signs * determinant;
        let reflection = if proper == glam::Vec3::ONE {
            glam::DQuat::IDENTITY
        } else if proper.x == 1. {
            glam::DQuat::from_xyzw(1., 0., 0., 0.)
        } else if proper.y == 1. {
            glam::DQuat::from_xyzw(0., 1., 0., 0.)
        } else {
            glam::DQuat::from_xyzw(0., 0., 1., 0.)
        };
        let authored = RootRigidEnclosure::from_transform(RootRigidTransform {
            translation: transform.translation.as_dvec3(),
            rotation: glam::DQuat::from_array(transform.rotation.to_array().map(f64::from)),
        })?;
        let reflection = RootRigidEnclosure::from_transform(RootRigidTransform {
            rotation: reflection,
            ..RootRigidTransform::IDENTITY
        })?;
        let outer = authored.compose(&reflection)?;
        let outer_scale =
            RootUniformScaleEnclosure::from_scale(f64::from(magnitude.x) * f64::from(determinant))?;
        frame = outer.compose(&frame.with_translation_scale_enclosed(outer_scale)?)?;
        scale = outer_scale.multiplied(scale)?;
        parent = joint.parent;
    }
    Ok((frame, scale))
}

// Only immutable clip admission proofs establish an ancestor for the whole interval.
// For rotation, a uniform signed scale is orthogonal up to magnitude; its axis
// conversion uses the proper pseudovector basis det(S)*S, including reflections.
pub(super) fn constant_parent_basis(
    model: &ModelAsset,
    settings: &ModelAnimation,
    root: u16,
) -> Result<(glam::Mat4, glam::DQuat, f64), EditorError> {
    let mut matrix = glam::Mat4::IDENTITY;
    let mut rotation = glam::DQuat::IDENTITY;
    let mut uniform_scale = 1_f64;
    let mut parent = model.skeleton.joints()[usize::from(root)].parent;
    while let Some(index) = parent {
        let index = usize::from(index);
        let joint = &model.skeleton.joints()[index];
        let transform = if let Some(clip) = settings.clip {
            model.animations[clip]
                .constant_joint_transform(index)
                .ok_or("root motion parent is moving or unproved; choose a fixed locomotion root")?
        } else {
            joint.bind_local
        };
        if settings.root_motion_rotation {
            let scale = transform.scale.abs();
            if scale.x != scale.y || scale.x != scale.z {
                return Err("A parent of the selected motion bone has nonuniform scale; root rotation requires uniform scale".into());
            }
            let signs = transform.scale.signum().as_dvec3();
            let determinant = signs.x * signs.y * signs.z;
            uniform_scale *= f64::from(scale.x) * determinant;
            let reflection =
                glam::DQuat::from_mat3(&glam::DMat3::from_diagonal(signs * determinant))
                    .normalize();
            let authored =
                glam::DQuat::from_array(transform.rotation.to_array().map(f64::from)).normalize();
            rotation = (authored * reflection * rotation).normalize();
        }
        matrix = transform.matrix() * matrix;
        parent = joint.parent;
    }
    if !matrix.is_finite()
        || !rotation.is_finite()
        || !uniform_scale.is_finite()
        || uniform_scale == 0.
    {
        return Err("root motion parent conversion overflow".into());
    }
    Ok((matrix, rotation, uniform_scale))
}
