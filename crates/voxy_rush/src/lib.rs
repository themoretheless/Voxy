//! Rush scripting backed by the tokenizer repository's parser and evaluator.
//! Compile once, then execute on the owning thread for each scene tick.
use glam::Vec3;
pub use rush;
use rush::{CancellationToken, ExecutionLimits, Program, RuntimeError, Value};
use voxy_scene::{NodeId, SceneGraph, SceneGraphError};

#[derive(Debug)]
pub enum ScriptError {
    Runtime(RuntimeError),
    Scene(SceneGraphError),
    InvalidTime,
    InvalidPosition,
}
impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Runtime(error) => write!(f, "Rush at {:?}: {}", error.span, error.message),
            other => write!(f, "Rush scene error: {other:?}"),
        }
    }
}
impl std::error::Error for ScriptError {}

/// A position script returns `[x, y, z]`. Inputs are the current local position,
/// frame delta and elapsed time in seconds. Variables are fresh on each run.
/// The borrowing program is thread-local because the Rush runtime uses Rc.
pub struct PositionScript<'s> {
    program: Program<'s>,
    pub limits: ExecutionLimits,
}
impl std::fmt::Debug for PositionScript<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PositionScript")
            .field("limits", &self.limits)
            .finish_non_exhaustive()
    }
}
impl<'s> PositionScript<'s> {
    /// # Errors
    /// Returns syntax/binding diagnostics with source byte spans.
    pub fn compile(source: &'s str) -> Result<Self, ScriptError> {
        Ok(Self {
            program: Program::compile(source).map_err(ScriptError::Runtime)?,
            limits: ExecutionLimits {
                steps: 100_000,
                max_depth: 32,
                max_collection_items: 4096,
                max_string_bytes: 65_536,
            },
        })
    }

    /// # Errors
    /// Invalid time, cancellation, exhausted budgets, stale handles and invalid
    /// results leave the scene unchanged. Limits are not a total heap bound.
    pub fn update(
        &self,
        scene: &mut SceneGraph,
        owner: NodeId,
        delta: f64,
        time: f64,
        cancellation: &CancellationToken,
    ) -> Result<(), ScriptError> {
        if !delta.is_finite() || delta < 0.0 || !time.is_finite() || time < 0.0 {
            return Err(ScriptError::InvalidTime);
        }
        let mut transform = scene.local(owner).map_err(ScriptError::Scene)?;
        let p = transform.translation;
        let result = self
            .program
            .run_with_limits(
                self.limits,
                cancellation,
                &[
                    ("x", f64::from(p.x)),
                    ("y", f64::from(p.y)),
                    ("z", f64::from(p.z)),
                    ("delta", delta),
                    ("time", time),
                ],
                &[],
                &[],
            )
            .map_err(ScriptError::Runtime)?;
        let Value::List(values) = result else {
            return Err(ScriptError::InvalidPosition);
        };
        let [Value::Number(x), Value::Number(y), Value::Number(z)] = values.as_slice() else {
            return Err(ScriptError::InvalidPosition);
        };
        #[allow(clippy::cast_possible_truncation)]
        let position = Vec3::new(*x as f32, *y as f32, *z as f32);
        if !position.is_finite() {
            return Err(ScriptError::InvalidPosition);
        }
        transform.translation = position;
        scene
            .set_locals(&[(owner, transform)])
            .map_err(ScriptError::Scene)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_scene::Transform;
    #[test]
    fn executes_rush_and_preserves_other_transform_fields() {
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        let script = PositionScript::compile(include_str!("../examples/move.r")).unwrap();
        script
            .update(&mut scene, owner, 0.5, 1.0, &CancellationToken::default())
            .unwrap();
        let result = scene.local(owner).unwrap();
        assert_eq!(result.translation, Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(result.scale, Vec3::ONE);
    }
    #[test]
    fn errors_do_not_mutate_scene() {
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        for source in [
            "[1, 2]",
            "[1, 2, 1e100]",
            "unknown()",
            "while true { 1 }\n[1, 2, 3]",
        ] {
            let mut script = PositionScript::compile(source).unwrap();
            script.limits.steps = 100;
            assert!(
                script
                    .update(&mut scene, owner, 0.1, 1.0, &CancellationToken::default())
                    .is_err()
            );
            assert_eq!(scene.local(owner).unwrap(), Transform::default());
        }
        assert!(PositionScript::compile("let x = ;").is_err());
        let script = PositionScript::compile("[x, y, z]").unwrap();
        let cancellation = CancellationToken::default();
        cancellation.cancel();
        assert!(
            script
                .update(&mut scene, owner, 0.1, 1.0, &cancellation)
                .is_err()
        );
        assert!(
            script
                .update(
                    &mut scene,
                    owner,
                    f64::NAN,
                    1.0,
                    &CancellationToken::default()
                )
                .is_err()
        );
    }
}

mod manager;
pub use manager::*;
