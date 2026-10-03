# Fixed-tick animation and root motion in Play

## One playback authority

PlaySession owns AnimationRuntime, keyed by generational scene owner. Each accepted model instance has one ModelPlayback and an immutable accepted AnimatorFrame. The existing character phase stages animation once at each fixed tick, then performs physics; the staged animation clocks/frames publish only when that character transaction succeeds. The editor extends the shared gameplay plan with explicit write access to `animation.playback`; the grant is checked before staging. SchedulePlan extension preserves barriers and splits newly conflicting batches rather than inventing duplicate systems.

AnimatedModels owns rendering resources and accepted frame snapshots. It contains no ModelPlayback and does not call advance. Skin palettes and LOD preparation read the snapshot, including when rendering multiple views. GPU admission failure retains the prior geometry/pose, while accepted simulation continues; the next rendering attempt consumes the latest simulation frame rather than replaying elapsed ticks. A missing fixed frame retains previous geometry silently.

Models are bound from the GPU-accepted revision when graphics exists and the accepted CPU catalog revision in headless Play. Missing imported models wait. New bindings start at the first available fixed tick; model revision or clip changes reset playback according to the existing replacement policy. Speed and motion-joint edits preserve the clock. Inactive owners retain clock state; deleted owners are removed at synchronization, even on a zero-tick frame. Stop releases all playback frames and restores authoring.

## Authored root motion

ModelAnimation persists `root_motion_axes: [bool; 3]`, defaulting to all false for old scenes. The ordinary component inspector exposes Root motion X/Y/Z alongside the selected motion bone index/name. Enabled axes require a CharacterBody on the same model owner.

Axes refer to the selected joint's parent-local coordinates. Extraction returns those pose coordinates to bind translation and rebuilds the skin palette, leaving rotation, scale and other axes intact. The displacement is transformed through the proved constant ancestor basis and the model owner's world basis. Constant authored channels may differ from bind pose. Proof is cached at clip construction: values must match exactly; LINEAR/STEP quaternion antipodes are accepted, while CUBICSPLINE also requires zero participating tangents and identical quaternion components. Unused boundary tangents are ignored. Moving or unproved ancestors are rejected; normalized quaternion cubic curves with nonzero tangents remain unproved.

The existing character-motion API applies this world displacement after input/gravity, sweeps against static colliders, slides, and returns actual motion. The renderer sees the in-place palette plus the collision-limited scene transform. Requests are produced once per accepted fixed tick, never by presentation. A failed physics tick preserves animation clocks/frames and pending input edges. Previously completed ticks in a catch-up frame remain accepted and are counted even when a later tick fails. Behavior changes before the failed system follow the existing SceneSimulation partial-failure policy; this is not rollback of arbitrary game scripts.

## Evidence and limits

CPU tests cover resource waiting, inactive/resumed/paused owners, read-only frame queries, invalid settings, foreign scenes, deletion/Stop, nonuniform rotated ancestor conversion, rejection of animated ancestors, real editor Play wall collisions, zero-tick frames, failure/retry and a failure after one completed catch-up tick. Real-device tests cover immutable-frame skin publication, shared sources, material failure retention, CPU fallback, LOD and waiting-frame retention.

Native root-motion smoke authors the component fields through the ordinary inspector, runs two owners, and verifies a wall-limited X position near 0.05 with an in-place palette. Native Fox smoke separately verifies all three original clips, shared texture bindings, a paused owner and Stop. Both use the ordinary Play/Stop path and actual presented frames.

Remaining production work includes rotation extraction, animated-parent world integration, long-running clock precision, skeletal render interpolation, model/clip reimport continuity beyond reset, character/platform contacts, contact feedback and foot locking. This does not establish CUDA support, general dynamic rigid-body physics or completion of the broader engine goal.
