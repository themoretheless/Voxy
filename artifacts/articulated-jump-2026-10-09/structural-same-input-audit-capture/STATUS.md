# Same-input structural audit

Frozen diagnostic full run terminated FAILED at frame 6. Phase comparison shows rod227 error 78.068 nm at prediction, then 15.836351 micrometres at the first structural phase, before contact counts differ. Final frame error is 9.198537 micrometres. This localizes amplification, but does not distinguish structural arithmetic error on identical inputs from nonlinear sensitivity to inherited pose differences. No same-input joint audit capture was generated. Terminal phase analysis is preserved in ../same-input-drift-audit-full-capture/terminal-phase-analysis.json.

New opt-in VOXY_HAIR_STRUCTURAL_DIFFERENCE_EXPORT observes final post-refinement corrections on the identical immutable original matrix/RHS. It compares native correction per active translational/angular degree of freedom and exports the first difference above 1e-9 m or 1e-7 rad. Thresholds only trigger diagnosis; outputs, admission, refinement counts and fallback decisions remain unchanged. Matrix/RHS/native/GPU values use exact u64 float bits. create_new preserves existing evidence; native/export errors are logged without rejecting GPU results. No extra native solve occurs without opt-in.

Regression passed: identical corrections emit nothing; perturbed correction captures exact native/accelerated bits without changing matrix/RHS/result; existing evidence is preserved. Actual Metal full469 captured structural control passed with the observer enabled. This control is not the failed frame6 operator and does not resolve the full-trajectory divergence.

Current full cooperative trajectory process86606 remains on its frozen pre-audit binary, passed frames1-2, and was not restarted. Full720, rendered physics and >160 FPS remain unverified.
