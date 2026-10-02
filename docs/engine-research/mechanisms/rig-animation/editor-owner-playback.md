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
preflight precedes writes for all skin streams. Prepared immutable pose tokens
avoid repeating that vertex validation during encoding. Position validation
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

Production animation remains open: production skeletal LOD quality/performance acceptance,
root-motion/physics ownership and pose interpolation, inspector transition
controls, layers/masks, wider skeletal normal verification and genuine limited
backend/hardware acceptance. The separate hand prototype is still rejected.
The broader engine/500-repository/hardware/physics goal remains active.

## Authored playback controls

The Component fields inspector exposes an Add/Remove model animation action.
The registered `clip` field accepts a zero-based clip index or `null` (bind pose);
`speed` accepts 0 through 8, with zero pausing the owner clock. Changes use the
existing stable component bindings, scene history and scene serialization.
Invalid edits leave the accepted document unchanged. Clip bounds are checked
against available imported data; loading a scene before asynchronous import
publication remains supported, and playback admission checks the actual asset
revision again. Removing the component restores automatic default playback.

The integration regression exercises the inspector action, rejected clip and
speed edits, bind/clip switching, pause, undo/redo and document round trip.
Native controller acceptance now uses the ordinary inspector action and text
entry/Enter path, with bind/clip switching, undo/redo and per-owner pause before
Play. At frame 28/tick 12, two owners shared one source (928 logical animation
bytes); after Stop at frame 31, bytes were zero and the authored document was
restored. The 88-test editor suite passes, with four hardware tests ignored in
that suite. Evidence is retained under
`artifacts/editor-animation-inspector-acceptance-2026-10-03/`.
Screenshot and pointer hit-testing verification remain outstanding.

## Animated camera LOD

Imported skeletal LOD recipes now reach the Play/standalone owner controller.
The candidate pose is recertified before publishing animation time or writing
existing GPU streams. Source attributes, palette size and base topology must
match the model's single skinned primitive. Camera selection recertifies in the
current world transform, with per-owner/per-view hysteresis; rest-space metadata
is never treated as an all-animation quality guarantee.

The owner caches admitted index-only GPU levels. Compute deformation updates
the shared streams once, and views borrow their independently selected level.
CPU fallback bakes the selected local-space pose with immutable material
coordinates and clears stale optional baked levels after a pose change.
Admission accounts for currently retained levels, other geometry and staged
allocations. Invalid camera/viewport or failed admission leaves selection
history unchanged. Stop/removal retires both base and optional residency.

Unused optional levels retire after all current views select; closing a view
removes its history before admission. Levels used by another view remain pinned,
including the accepted selection after a rejected update. Base geometry stays
resident. Pose/camera certification still requires CPU work; no runtime
performance target is claimed. Complex production skeletal LOD acceptance
remains outstanding.

Verification: 88 ordinary editor tests pass (five hardware tests ignored);
all three owner GPU/fallback tests separately pass. The new LOD test uses a
six-index source and three-index equivalent variant: the distant orthographic
view selects the variant and a perspective near-plane crossing selects base.
Budget and invalid viewport failures preserve accepted history; the GPU level
adds 12 bytes. Both GPU and explicit CPU selected renders match a CPU reference
with more than 50 colored pixels. Stop/clear retires all logical animation bytes.
The native inspector/Play/Stop regression also passes after integration, but its
GLB has no LOD recipe and does not prove native camera LOD selection. Reports are
saved in `artifacts/editor-animated-lod-acceptance-2026-10-03/`.

Native skeletal LOD acceptance now loads an observed `.vmodel` recipe generated
by `certify_skinned_lod`. The fixture duplicates one animated triangle in the
base and removes the duplicate in its certified variant, so it proves resource
and camera routing with an equivalent surface, not perceptual simplification.
Two far-view owners select level 1/three indices (976 animation bytes); a
near-plane crossing selects base/six indices and retires 24 optional bytes,
leaving 952 bytes. Stop at frame 34 retires all animation bytes and restores the
authored document. Three GPU tests include second-view pinning, closed-view
retirement and exact-peak re-admission on both GPU and CPU paths. Evidence and
reproducible model inputs are in
`artifacts/editor-skeletal-lod-native-2026-10-03/`.

The additional pinned RiggedFigure fixture now supplies a 19-skin-bone/22-node
palette regression, sampled CPU/GPU position comparison and native owner
Play/Stop acceptance. See `reference-rig-preflight.md` for bounded evidence and
debug host timing limits; it does not close reduced complex-rig LOD quality or
full production animation acceptance.

An actually reduced RiggedFigure recipe (768 to 510 indices) now also passes
native camera selection/retirement and 258 sampled pose/world certifications.
Its release CPU certificate timing and remaining visual/material/continuous-time
limits are documented in `reference-rig-reduced-lod.md`.
