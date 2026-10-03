# Authoring root motion

ModelAnimation keeps the `editor.model-animation.v1` codec. Numeric `root_motion_joint` defaults to zero, `root_motion_bone` defaults to an empty string, and `root_motion_axes` defaults to all false, preserving old scene documents.

The ordinary inspector displays Motion bone index/name and Root motion X/Y/Z. A nonempty authored bone name takes precedence over the numeric index and must resolve uniquely in the accepted imported revision. Scene admission validates descriptor ranges and requires CharacterBody on the model owner when any motion axis is enabled. Runtime repeats checks against the accepted model and rejects animated ancestor bases before physics publication.

ModelPlayback applies bone selection without resetting time. PlaySession's AnimationRuntime owns the staged clock and applies authored settings at fixed ticks. AnimatedModels owns immutable accepted render frames; it does not advance playback. Bind-only playback has zero displacement. Enabled axes are extracted from the pose and applied through character collisions as described in [fixed-animation-play](fixed-animation-play.md). Names remain stable across node-order changes, with the existing clock-reset policy on asset replacement.

Original numeric-authoring verification remains recorded in `artifacts/rig-root-authoring-2026-10-03/`: 93 editor tests, 6 device tests, Fox native Play/Stop and a viewed CUA screenshot labelled Motion bone. Named-selection evidence is in `artifacts/rig-named-bones-2026-10-03/`. Current fixed-tick/physics evidence is in `artifacts/rig-root-fixed-play-2026-10-03/`.

A named hierarchy dropdown, rename retargeting, root rotation, animated-parent world integration, foot contact feedback and skeletal presentation interpolation remain open. See [named-motion-bones](named-motion-bones.md) and [root-motion-physics-port](root-motion-physics-port.md) for specific contracts and limits.
