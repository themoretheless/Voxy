# Imported skeleton with neutral FEM pads

The existing body_motion_snapshot example accepts --cesium. It reuses the pinned licensed CesiumMan asset, samples its full two-second clip at the existing 240 Hz physics clock, and draws the imported CPU-skinned character and four independently solved FEM surfaces through the existing SceneRenderer. It creates no parallel animation clock or renderer.

Initial region centers are explicitly placed near the world-space torso/hip bone origins in the first sampled pose. The current skin palette times inverse first-pose palette moves the three supports per region; free nodes retain solved momentum. The initial material coordinates remain double precision. The four placements are illustrative neutral pads, not anatomically calibrated tissue. Imported geometry uses neutral vertex lighting for diagnostic readability; its original textures/materials are not displayed in this example.

TissueDemo now exposes an external palette callback routed through its existing transactional frame update. A callback failure after a successful first substep preserves bodies, histories, heat, clock, accumulator and attachments; the regression verifies successful recovery. Secondary displacement is computed against each region's actual initial center, rather than the original mannequin centers. Only the physics surfaces are added to the imported character; no procedural mannequin limbs are drawn.

Validation:
- 15 tissue tests passed, including full earlier imported-motion regression, shifted placement, external-palette failure, clock rollback and recovery.
- Actual Apple M4 Max / Metal offscreen render: 41 distinct sampled frames at 20 fps over 2 seconds, with GPU validation scope clean. Four-pose strip at 0, 0.5, 1 and 2 seconds was visually inspected.
- Full 480-step simulation plus 41 rendered/read-back/saved frames: 9.421672500 seconds. This is a different correctly placed fixture from the earlier arbitrary-center 36.35-second CPU regression, not a controlled solver speedup comparison.
- Region maximum free-center displacement: 3.7656, 3.7636, 8.9315, 7.3004 mm. Sampled relative volume changes: at most 0.0033284. CSV energy closure differs from independently accumulated defect by about 1e-12 J (printed decimal rounding).
- gif is assembled from the actual rendered PNGs; no motion is synthesized.
- git diff --check clean for changed source.

Run:

```sh
CARGO_TARGET_DIR=/tmp/voxy-moving-supports-final-20261005 cargo run --release -p voxy_app --example body_motion_snapshot -- OUTPUT.png FRAME_DIRECTORY --cesium
```

Limitations: offscreen proof only; no native imported-pad demo was opened. Realtime remains unproved and the measured end-to-end duration exceeds the simulated duration. Pads have no contact coupling against the imported character surface, so their surfaces may intersect it. Whole-character soft deformation, correct material textures, calibrated anatomy and collision coupling remain work to do.
