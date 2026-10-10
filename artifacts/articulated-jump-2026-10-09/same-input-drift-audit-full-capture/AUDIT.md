# Same-input drift localization

The previous early-release full paired run failed at frame 6, rod 227 point 7, position difference 9.198537073512592e-6 m. Frame 5 maximum was 8.239790681013881e-8 m. Zero native fallbacks did not establish trajectory equivalence.

New opt-in VOXY_HAIR_ACCELERATOR_DIFFERENCE_EXPORT audits each physically admitted accelerated joint solve against the native solution of the same immutable prepared physical operator and bounds. It captures the first difference above 1e-9 m translation or 1e-7 rad angular increment, plus both responses and reactions. These are diagnostic trigger thresholds; original admission and trajectory limits remain unchanged. No callback result is replaced. Failed native audit or export does not change admission. Existing input/report files are preserved.

341 physics library tests passed, 39 ignored. The new audit regression checks identical input produces no file, observation preserves response/reaction/matrix state, a differing observation exports VQC1 and both responses, and create-new export preserves existing evidence.

The diagnostic full run will retain 720 frames, all 469 guides and original limits. Phase trace targets rod 227 at frames 5 and 6. This incurs extra CPU reference solves and is not an FPS benchmark. It does not yet integrate cooperative GPU batch submissions.
