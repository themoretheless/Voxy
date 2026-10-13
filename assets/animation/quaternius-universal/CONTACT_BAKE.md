# Contact candidate preparation

`cargo run -p voxy_render --release --example quaternius_gait_contacts -- --bake-contacts`
measures the ankle; `--bake-toe-contacts` measures the toe. Both use the native f64
sampler and the explicit Jog closing-frame recipe before baking. Raw source files
remain unchanged. Output contains named left/right curve maps compatible with
`FootBinding.clip_contact_curves`; it does not select bones, pivots, supports or
install a runtime binding.

The classifier follows the height-plus-vertical-speed approach in MIT
[FootContactCurves](https://github.com/thecodebrozilla/FootContactCurves/tree/866b7e05be3de6ee748b0c72c4e9602f9de8bf9f).
Voxy uses periodic central differences rather than the source's one-sided
neighbor differences. Distance/speed limits are inclusive, in meters and m/s,
with model-space Y up. This is an independently implemented authoring operator;
upstream license and inspected implementation are retained in the research sources.

The current candidates use 24 samples/second, matching the authored source grid,
6cm height range above each observed minimum, and30cm/s vertical-speed limit.
The minimum is a sample statistic, not a detected physical floor. A closing key
repeats phase0's gate. The existing runtime interpolates authored weights with
smoothstep and owns skipped-swing events, mixed fades, anchors and accepted-tick
publication; the baker does not create another runtime state machine.

**These candidates require review.** The UE mannequin defaults do not transfer
cleanly to this source pack. Walk produces coherent windows, but Jog/Sprint still
produce short or absent windows (including zero right-foot Sprint candidates).
Oversampling at240 samples/cycle fragmented the gates further; those diagnostic
outputs are preserved in the dated artifacts. Neither dense sampling nor a valid
serialized curve proves correct stance. Do not automatically activate these files
or silently widen thresholds to manufacture coverage. Per-clip calibration,
manual labels, physical pivot placement and full-character acceptance remain open.

## Threshold analysis

`cargo run -p voxy_render --release --example quaternius_gait_contacts -- --analyze-contact-thresholds`
emits prepared toe-height and periodic vertical-speed samples plus 25 threshold
combinations for each of eight clip/side records. This diagnostic mode does not
select thresholds, write runtime bindings or replace the candidate assets.

The 2026-10-11 analysis contains 200 trials. Screening for exactly two cyclic
transitions and 15–65% coverage on both sides finds shared Jog trials only at
2.4m/s; Sprint additionally needs an 8cm or 12cm height band. This is a sensitivity
result, not a contact-quality criterion: coherent windows can still lock a moving
or airborne foot. Keep explicit contact-point authoring and physical checks before
promotion. Full samples and the screening audit are retained in
`artifacts/articulated-jump-2026-10-11/contact-bake/threshold-analysis.json` and
`threshold-analysis-audit.json`.

## Contact-site kinematics

Both analysis modes now emit full sampled positions and periodic central-difference
velocities. `--analyze-ankle-thresholds` uses the ankle instead of the toe. Each
threshold trial reports the least-squares constant actor XZ translation velocity
that cancels the site's mean velocity, plus the remaining planar RMS velocity.
An empty gate yields a null fit. This is an in-place kinematic diagnostic, not a
root-motion estimate or a physical acceptance gate; it cannot model foot roll,
turns, variable actor speed, support movement or IK corrections.

Across separately screened trials (two cyclic transitions, 15–65% coverage),
minimum toe RMS values for Jog are 0.201/0.181m/s and for Sprint 1.301/1.042m/s.
Ankle Sprint minima are 2.302/1.891m/s. Different sides may attain minima under
different thresholds; these are not selected paired profiles. The large Sprint
residuals rule out treating a coherent gate as a naturally stationary pivot under
constant translation. Author contact-site transfer/roll and verify correction
magnitudes before enabling locks. Native outputs are `toe-kinematics.json` and
`ankle-kinematics.json` in the contact-bake evidence directory. All stored periodic
velocities were independently recomputed from their corresponding position arrays.
