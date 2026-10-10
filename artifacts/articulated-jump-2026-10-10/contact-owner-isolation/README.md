# Contact-only motion ownership candidate

A captured full-density frame-2 trace showed extra native reconciliation rounds advancing rod 446 with no strand pairs. An isolated reproduction demonstrated global free elastic motion moving a remote guide during an elastic contact call. That shared elastic call deliberately owns free motion, so its original semantics are retained.

The candidate instead introduces an explicit swept contact-only entry point. It restricts free Newton increments to owners of the current constraints, recalculates owners after newly activated swept rows, and retains full structural free motion in the original swept entry point. No contact admission tolerances are changed.

Regression coverage distinguishes a contact-only pass from structural free advance, and verifies mesh contact separation while a remote guide remains bitwise stationary. The existing unseen-pair runtime fixture now includes the structural advance before reconciliation, matching runtime ordering and retaining its motion/separation/full-path assertions.

Initial broad validation found two ownership-contract failures. The shared elastic API was restored and the runtime-order fixture corrected; subsequent validation is recorded separately. Full-density 720-frame agreement and rendered >160 FPS remain unqualified.

Corrected validation: 366 physics unit tests pass (39 ignored), 21 hair integration tests pass, 222 focused hair units pass (20 ignored). Rebuilt application jump filter passes 6 tests (2 ignored); actual soles check 8280 vertices and knee mesh displacement 0.18173385 m. Actual Metal four-guide/four-frame serial-versus-batch fixture passes exact pose/rotation with 319 equality dispatches, 102 submissions and zero native fallback. Actual GPU body rig comparison is recorded separately. These do not qualify the full-density hair trajectory or FPS.

Additional owner regressions pass: adding a new row after the initial projection restores the new owner's complete elastic translation and rotational compliance (matching the unrestricted reference), while remote free motion remains zero. An actual swept-contact fixture starts with no strand pairs: a mesh correction discovers and moves a neighbor, separates the mesh contact, preserves the entire swept path and roots, and leaves a third remote guide bitwise unchanged. Expanded focused hair suite: 224 passed, 20 ignored. These additions change tests only; the full-density run continues on its original frozen candidate binary.
