# Full-model jump runtime strain investigation

The original full-model async launch stopped after publishing 0.05 simulated seconds. Guide 53 segment 17 reached 5.3921847% compression; the original limit remains 5%. See `original-rejected-guide.json` and `original-failure-summary.json`.

An application-level bounded temporal retry reruns a rejected interval from its original admitted state, retaining material, roots, collider interval, density and the original strain gate. It doubles substeps up to 32 and only retries the strain-admission error; other failures still return immediately. Accepted output restores the configured base substep count. Accelerator side effects remain external to the transaction.

Refinement alone passed the eight-step full-groom regression but did not complete takeoff. The interactive run published 0.50 seconds and subsequently rejected guide 333 at 7.5877351% strain despite refinement. See `refinement-only-runtime.log` and `refinement-only-rejected-guide.json`.

The runtime previously interpolated scalp roots while testing all substeps against the endpoint body collider. `FemaleHair` now enables the existing sampled-collider-motion path after static groom initialization, so both follow the same physical interval. The paired native/GPU qualification fixture still sets its own motion policy explicitly; its independently running frozen binary is unaffected.

With sampled collider motion, `sampled-collider-regression.log` passes eight actual coupled skin/hair steps on all 469 guides with 20 segments each, without temporal retry. This is an early-contact regression, not qualification of the entire jump, all 720 frames, CCD, all hardware or rendered FPS. The renderer constructs all 2,363,760 follower vertices; these are render fibres, not independent physical rods.

`regression.log`, `rollback.log` and `jump-tests.log` precede the sampled-collider change. `sampled-collider-regression.log`, `hair-tests.log` and `sampled-runtime.log` cover the final sampled policy. Source SHA-256 hashes are in `source-provenance.json`. Runtime work_ms is batch preparation time and must not be reported as rendered FPS.

Live runtime observation: the sampled-collider build has published 0.65 simulated seconds, crossing takeoff at 0.56 seconds, with no failure logged at this observation. The process and full-loop test remain live; this does not establish landing or full-cycle success. See `sampled-runtime-observation.json`.

## Terminal sampled runtime result

The previously live sampled-collider application exited with code 0 after publishing 2.30 simulated seconds, with zero worker/scene errors logged. It crossed takeoff (0.56), landing (1.06), and first-jump recovery (1.65), then began the second crouch. This validates the first native-physics jump in this launch, not the complete six-second loop or the separately running native/GPU 720-frame comparison. See `sampled-runtime-terminal-summary.json`.

The sampled-policy hair tests completed: 11 passed, 2 ignored, including the 360-step arm-rig loop and 120-step animated-surface check. These use the legacy 462-guide asset for those two fixtures; the interactive jump and early-contact regression use the prepared 469-guide model. Do not conflate these scopes.

## Collider workspace reuse

A two-second live CPU sample includes collider refitting, motion sampling, feature normals, skin solver and hair linear solves. It is a call-stack observation under concurrent host load, not a stable percentage profile or a rendered FPS benchmark.

The next source revision retains each timed collider workspace and its vertex scratch across substeps, avoiding repeated full-collider clones. `collider-reuse-tests.log` verifies exact fresh-vs-reused geometry/velocities/BVH/feature-normal state over 128 deforming intervals and retained triangle/node/vertex storage. It also checks atomic rejection of collapsed starting/ending faces. `physics-tests.log`: 370 passed, 39 ignored. App integration after this revision is recorded separately in `reused-collider-app-regression.log`; the interactive runtime above predates this allocation optimization. No measured FPS/speedup claim is made.

Final workspace-reuse checks: `hair-integration-tests.log` passes 21 hair integration tests and 1 rebase test; `reused-collider-app-regression.log` passes all eight coupled steps on the prepared 469-guide model without temporal retry. These checks do not establish a speedup; host load differed between runs.
