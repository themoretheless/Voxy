# Imported rig tissue refinement

## Cause and change

The former step-11 rejection was caused by the demo's refinement limit of eight, rather than proof that the imported motion was incompatible. An isolated probe allowed refinement up to twelve without changing the per-time energy tolerance, material, support targets, or transaction semantics; all first 48 frames passed. The bounded demo limit is now twelve.

The current regression samples the complete pinned CesiumMan two-second animation at 240 Hz (480 frames), resolves torso and hip bones by name, and maps supports by current skin palette times inverse initial skin palette. FEM reference support coordinates remain f64. Every frame checks exact prescribed support positions and finite embedded surface vertices. Final mechanical energy change minus support work plus released heat agrees with independently accumulated integration defect within 1e-8 J.

## Evidence

- All 14 tissue tests passed with the initial 48-frame imported regression, including late-frame/second-child rollback and subsequent recovery.
- The expanded full-clip regression passed independently: 480 frames, 42.77 seconds CPU wall time in the release test.
- Per-region accepted/rejected/deepest subdivision receipts: (30778,7070,5), (32994,6870,5), (653257,208797,9), (697506,219254,10).
- Energy admission remains 1e-5 * dt * 240 J, with separate mechanical and relaxation allocations unchanged.
- git diff --check is clean for the changed source.

## Limits

This proves numerical admission for the existing four-region mannequin driven by the imported palette. It does not prove anatomically placed character tissues, complete character surface embedding, realtime performance, or native/GPU visualization of this imported coupling. The measured CPU cost is unsuitable for claiming realtime. No renderer or physics-engine admission thresholds were weakened.

The earlier imported-rig-tissue-admission artifact is historical evidence of the former limit; its incompatibility interpretation is superseded by this controlled refinement experiment.
