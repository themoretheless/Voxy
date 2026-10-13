# Frame-three guide 232 diagnostic capture

Session 78498 runs frozen /tmp/voxy-contact-owner-gpu-tests, same full 469 guides x 20 segments and 720 requested frames with unchanged 1e-6 m / 5e-5 quaternion gates. The previous run terminated with exit 101 at frame 3; this is a diagnostic follow-up, not a restart on timeout. Phase trace selects guide 232 at frame 3; original island operators are captured for frame 3. Production solver defaults and shader are unchanged. This run does not measure FPS.

## Operator capture correction

The live launch selected phase-trace rod 232 and frame 3, but omitted VOXY_HAIR_ISLAND_INPUT_ROD and VOXY_HAIR_ISLAND_INPUT_LIMIT. Consequently the operator exporter uses its defaults: target rod 53, limit 16 per backend. These sidecars must not be presented as a complete guide-232 operator capture. Frame-three phase/projection traces still select rod 232 correctly. Preserve the live session and these operators; no restart was made to change environment settings. Any later guide-232 operator capture must explicitly set rod=232 and an appropriate bounded limit, retaining original full simulation settings.

## Terminal diagnostic result

Session 78498 exited 101 after 4824.37 seconds at frame 3. It reproduced the previous terminal position drift 0.00022248771285753033 m and quaternion component drift 0.006089104733330297 at guide 232. Gates remain 1e-6 m and 5e-5. No trajectory qualification or FPS success is established.

`phase-comparison.json` localizes the first over-gate guide-232 phase to frame 3, substep 0, contact-positions at structural iteration 3: position component difference 4.360276746667724e-5 m and quaternion difference 0.0013343173844616585. Immediately preceding structural-mesh phase differs by only 1.2094880652568918e-11 m / 5.590240770292354e-10. The endpoint mesh-contact counts after this projection are both zero, but that does not prove absence of strand-connected or preceding mesh constraints. `first-failing-phase-projections.json` inventories observed projection snapshots; it does not establish semantic equivalence of rows.

`terminal-summary.json` and `terminal-frame-work.json` summarize the unchanged full-run counters. The captured guide-53 operators remain insufficient for reproducing this guide-232 projection; do not relabel them. The original paired phase log is preserved.

## Operator-directory ownership audit

`tools/audit_hair_island_directory.py` verified all 32 original VQC load arrays against their v2 row sidecars with frame 3 / rod 53 selection. Requiring rod 232 rejected all 32 operators (exit 1). This confirms these historical operators cannot serve as the rod-232 replay. See operator-directory-audit.json. This is an input ownership audit, not solver parity.
