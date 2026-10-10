# Cooperative inner physical refinement

Both original outer Newton rounds and inner whitening/load defect corrections now batch unfinished physical owners. Finished owners leave later batches. Original matrices, original bounds, summation order, physical admission and native recovery are retained. Local hints remain private until global admission.

The new cancellation-sensitive inner-refinement regression requires more than one physical trial, removes the completed owner from later batches, compares every response/reaction bitwise with serial scheduling, and checks original immutable bounds. Full-column shape/finite preflight occurs before any backend callback.

Validation: 344 physics library tests passed, 39 ignored. Actual Metal four-guide/four-step physical scene passed with 181 equality dispatches in 54 submissions, exact serial positions and orientations, zero native fallbacks. Four jump checks passed, two hardware/full-model checks ignored in that selection. The final projection source edit after library/Metal validation changes a comment only; source hashes record that comment revision.

These are bounded checks, not complete full-model trajectory or FPS qualification. Frozen earlier full drift-audit process 7636 was confirmed live at 48:52 elapsed, with frames 1-5 within original drift gates and frame 6 in progress. It does not contain this batching implementation and was not restarted. Earlier full early-release capture failed at frame 6. Full 720-frame dynamics, rendered secondary-motion capture, >160 FPS and broader engine/hardware scope remain incomplete.

Final current-source hair integration: 21 passed, zero failures. git diff --check passed.
