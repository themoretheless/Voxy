# Large original-operator QR reuse and early release qualification

Original captured frame-2 operator native-407: 174 rows, 14 systems, 1764 coordinates, tolerance 1e-14. Units remain unassigned. Frozen binary and input hashes are recorded in provenance.json.

Seven warmed alternating pairs compare fresh QR with exact-prefix reuse, full readback/grouped passes/support compaction in both modes. Median complete physical projection: 0.313036708 s fresh, 0.142754292 s reuse, ratio 2.1928357012201074. All seven reuse pairs are faster. Response and reaction bits match, original physical admission passes, native fallback is zero. This is a loaded selected-operator benchmark; concurrent full-model GPU work and other system load are uncontrolled. No FPS claim follows.

A separate early-release/prefix run reduces equality dispatches from 119 to 106 with original physical admission and zero fallback. Baseline and candidate native/GPU response and reaction bit comparisons are in summary.json. Four cooperative coordinate operators match serial GPU bits exactly (318 dispatches, 106 submissions). Invalid later input is rejected before any earlier-owner submit. This does not qualify a complete physical trajectory.

run_full_candidate.sh is prepared with QR prefix reuse and both release options enabled, preserving full density, frame count and all original tolerances. It has NOT been launched: the existing owner-isolation full run remains live and unchanged. Production defaults remain unchanged pending full qualification.
