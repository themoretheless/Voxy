# Exact triple-coordinate versus f64 boundary audit

Added opt-in VOXY_HAIR_TRIPLE_DECODE_AUDIT_DIR to the test-only triple bridge. It captures at most eight decoded submitted operators per process: original packed input, opaque actual readback, original API coefficient bits, declared gather ranges and decoded coordinates/reactions. Metadata is written last as the completion marker. All files use unique create_new names. Disabled capture performs no file I/O. Diagnostic write errors do not modify numerical output or admission. Five regular app solver checks pass.

This run uses the exact derived 100 x 882 rejected physical input. The enclosing physical test fails, as recorded; snapshots are checked transport intermediates BEFORE original physical admission, not accepted physical states.

The standalone audit uses exact rational arithmetic. It independently checks every packed coefficient and bound against the saved original f64 API values, reconstructs physical coordinate scale from exact binary references, decodes the declared compact gather mapping and compares the exact three-f32 expansion with the delivered f64 coordinates. All original coordinate constraints are checked, including complementarity using the delivered positive reaction set. Raw query/layout interpretation is confined to the offline versioned research script; the app does not duplicate codec offsets. The script explicitly rejects -O rather than silently disabling its validation assertions.

Results across eight snapshots:
- All eight exact three-component coordinate solutions pass the original coordinate-space inequality/complementarity gate.
- Four delivered f64 coordinate vectors fail that same gate after rounding.
- All original coefficients/bounds pack exactly. All nonzero coordinate and reaction conversions match correctly rounded exact sums; this is not an observed scalar decoder bug.
- At row 2, one exact expanded gap is about 6.75e-21 while its f64-coordinate gap is about -2.91e-14, beyond the 1e-14 tolerance. Maximum individual coordinate quantization errors are about 0.86e-17 to 1.37e-17.

This establishes information loss at the current f64-only coordinate boundary. It does NOT prove retaining tails alone fixes the original physical gate: the triangular backtransform and physical output rounding still need validation. Physical case100 remains failing. Do not loosen its tolerance or treat coordinate-space success as physical admission.

Next implementation: carry precision components in an owned typed coordinate proposal through batching, exact cache/aliases and the native backtransform, rather than a side channel or an early f64 sum. Every proposal must still pass original force balance, inequalities and nonnegative reactions. Current executable physics is unchanged by this diagnostic; full jump completion and >160 FPS remain unproven.
