# Cooperative coordinate equality rounds

HairResponseSystem::solve_contact_coordinate_batch_with_proposals now collects all ready independent active-set owners into one callback round. The backend receives stable operator indices and borrowed current equality rows. Entire input preflight precedes dispatch. Owners finish independently, retain original arithmetic and KKT checks, and retry unusable hints once from zero duals. A cold failure, missing batch or wrong reply count rejects all outputs. No worker thread or GPU device is created by the coordinator.

Current source: 340 physics library tests passed, 39 ignored. The new regression compares mixed operators exactly to serial solves, covers negative-reaction release, early completion without a callback, hinted backend rejection and cold retry, cold failure, reply count mismatch, malformed later inputs before dispatch and empty batches.

This API is not yet wired to batched GPU encoding or independent physical island publication. It does not replace original physical admission and does not establish throughput or full-trajectory equivalence.

During this work the previously launched early-release full trajectory terminated: frame 6 position error 9.198537073512592e-6 m exceeds 1e-6 m, quaternion error 0.00019552032000652586 exceeds 5e-5, despite 534558 admitted coordinate calls and zero native fallbacks. Its original log, launch provenance and terminal summary remain in ../early-release-full-capture. That older process did not use this source revision. Do not infer that cooperative scheduling fixes this numerical divergence. Full trajectory and >160 rendered FPS remain unqualified.

Current-source hair integration suite: 21 passed, zero failed. git diff --check passed.
