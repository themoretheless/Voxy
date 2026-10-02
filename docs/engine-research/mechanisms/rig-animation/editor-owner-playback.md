# Editor model owner playback

`ModelAnimation` is registered as `editor.model-animation.v1`, selecting a clip
and playback speed. `None` explicitly selects the bind pose, and speed zero
pauses. Invalid clip indices or nonfinite/out-of-range speeds reject before
creating state. No automatic serialization of the model asset or GPU state is
introduced.

`ModelPlayback` holds an immutable shared `Arc<ModelAsset>` and an owner-local
`Animator`. It stages the animator on `advance_with`, validates pose and finite
palette through the existing animation evaluator, and only commits the clock
when its frame consumer returns success. This avoids advancing time on GPU
admission failures. The consumer must preflight its own writes; the closure is
not a transaction over arbitrary scene/GPU side effects. Bind selection also
validates its palette, and all variants validate the timestep.

Tests use the actual animated-triangle GLB: two owners share one asset but have
independent moving/paused clocks, rejected publication retries exactly against
a control state, and explicit bind/invalid selection behavior is covered.
Both playback tests passed, zero failed/ignored, in
`/tmp/voxy-editor-model-playback-tests.log`. Dependency-boundary and whitespace
checks pass.

The editor now attaches playback to extracted generational model owners while
Play or standalone mode is active. Immutable source streams are weak-cached by
model revision and primitive; clocks, palettes and posed streams belong to each
owner. Fixed simulation ticks drive a bounded eight-step catch-up, independent
of how many views render. Paused/bind owners retain their geometry, and speed
changes preserve pose/time without reallocating. Clip selection or accepted
model revision replacement resets that owner's clock; failed replacement
retains the old revision, streams and clock.

The normal SceneDraw path consumes each animated primitive. Compute-unavailable
backends use explicit CPU baking, including rigid animated primitives. Both
routes preserve immutable material coordinates. Admission counts shared sources
once and each owner output, including overlap with a retained old revision;
this is a logical buffer budget, not total driver/in-flight GPU memory. Palette
preflight precedes writes for all existing skin streams. Position validation
still performs CPU arithmetic before GPU deformation; no CPU throughput claim
is made. Default imported animation selects clip zero when clips exist; explicit
`None` uses bind pose. Root motion is not implicitly applied to scene/physics.

Stop immediately clears playback resources and restores the authoring scene.
Inactive/removed generational owners are retired on reconciliation. Selection
outlines currently use bind geometry and are suppressed for animated owners;
a posed selection outline is not implemented.

Final validation: 87 editor tests pass (four explicit hardware tests ignored in
the ordinary suite); two GPU owner/fallback tests were separately executed and
both pass with zero ignored. The GPU test compares rendered pixels against a
CPU reference, verifies source sharing and independent paused/moving owners,
checks repeated ticks/pause/resume do not reset poses or reallocate, retains the
previous revision on budget rejection, and retires removed owners. The fallback
check covers skin plus rigid primitive routing and removal. Native acceptance
uses the ordinary editor duplication, authored component persistence and
Play/Stop paths, with two owners/one GPU source. At presented frame 27, fixed
tick 12, animation logical bytes were 928; at frame 30 after Stop they were zero
and authored transforms/settings matched. Logs and report are saved in
`artifacts/editor-animation-acceptance-2026-10-03/`.

Reproduce native proof with the ordinary application entrypoint:
`cargo run -p voxy_app --bin voxy_app -- --model crates/voxy_render/examples/assets/animated-triangle.glb --animation-native-smoke`.
On macOS, use a correctly named app bundle if direct terminal launch cannot
create a native window. The accepted bundle binary was saved under ignored
`target/editor-animation-native-final/VoxyAnimation.app/Contents/MacOS/VoxyAnimation`.

Production animation remains open: animated LOD and pose-certified residency,
root-motion/physics ownership and pose interpolation, inspector clip/transition
controls, layers/masks, wider skeletal normal verification and genuine limited
backend/hardware acceptance. The separate hand prototype is still rejected.
The broader engine/500-repository/hardware/physics goal remains active.
