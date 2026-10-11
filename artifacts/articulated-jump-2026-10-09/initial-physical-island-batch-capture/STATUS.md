# Initial physical island coordinate batching

Independent physical contact islands can now batch their first coordinate solve through HairLinearSolver::solve_joint_coordinates_batch. All original requests and immutable native preparations are completed before the backend call. The same prepared factors/columns then perform original load/force admission and rounded-free Newton admission per island. A failed batch preserves the previous native fallback. A wrong output count rejects before publishing reactions. Invalid individual coordinates fail original admission and fall back per island. Global checks and hint publication remain after every island succeeds.

Subsequent physical whitening or rounded-Newton corrections still use the scalar path; this is not fully cooperative nested physical refinement. Existing backends default to scalar behavior. GPU opt-in is joint_coordinate_batches; full qualification test reads VOXY_HAIR_JOINT_COORDINATE_BATCHES and logs equality submissions separately without breaking the old frame counter format. Its final gate requires actual shared submissions.

Validation: 342 physics library tests pass (39 ignored), including physical batch success, batch failure/native fallback, malformed individual output/native fallback and wrong count/no publication. Hair integration: 21 pass. Real Metal scene: 4 floor-contacting guides / 4 physical steps with friction-pressure recovery, 181 equality dispatches in 54 submissions, poses and orientations exactly equal serial GPU results, zero native fallbacks. This small physical scene is not full-model/FPS qualification.

## Structural GPU range regression exposed by this scene

Before the new contact path ran, the serial structural GPU solver rejected a packed residual in system 1. Exact original input/output words and diagonal scales are preserved in rejected-structural-decode.json. Independent Python reconstruction in structural-residual-analysis.json found row 23 residual 2.2795332568250115e-38 against an unchanged 1e-8 relative gate with 1e-30 scale floor. The RHS mixes components around 4e-19 and 1e-36, exposing subnormal compensated arithmetic. No tolerance was relaxed.

GpuHairLinearSolver now scales nonzero RHS vectors whose equilibrated maximum is below one by an exact power of two (bounded exponent), and undoes that multiplier before original physical checks. Residual correction RHS vectors are independently scaled while retaining the original coefficient words/factors. Original f64 residuals and final gates are unchanged. Reference-audit back-transformation accounts for the RHS multiplier. The initial conservative exponent threshold did not trigger on this capture; its failed log is retained. The final rule removes that arbitrary threshold.

Real Metal full captured structural batch (all 469 systems) passes original gates and native comparison with fresh/reused factors. This proves only that captured batch, not a whole trajectory or full hardware coverage. The four-guide physical regression subsequently passes the formerly failing structural solve and the actual batch contact path.

An opt-in VOXY_HAIR_BANDED_DECODE_FAILURE_EXPORT saves exact rejected normalized input/output words and scales with create-new semantics; it changes neither admission nor returned results.

The concurrently running same-input drift full trajectory is preserved on its older frozen binary. Its prior early-release counterpart failed at frame 6 with 9.2 micrometres drift; this change has not yet established that the full-trajectory failure is fixed. No >160 rendered FPS result is claimed.
