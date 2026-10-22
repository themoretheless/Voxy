# Sufficient temporal-position admission bound

This certificate proves only finiteness of positions, not invertibility or normal
admission. Complete finite/affine palette validation runs first. Mesh construction
already validates finite vertex attributes, canonical joint indices and four
nonnegative UNORM16 weights summing to 65535. Their f32 normalized values are <= 1.

Let M >= 1 bound every absolute palette entry, K >= 1 every model entry, and P >= 1
all absolute authored position components. P is accumulated in the existing mesh
construction validation loop and copied when sharing indices. Vertices are immutable.
M and K are recomputed from the current inputs on every attempted admission.

A deliberately loose rounded-arithmetic envelope suffices. For nonnegative bounds
at least one, allow a factor of two for each multiplication or addition; this
also covers subnormal rounding. The admitted envelope keeps exact intermediates
well below overflow, so these rounding bounds are applicable throughout.

- Each weighted matrix entry has magnitude <= 2M. Summing four entries starting
  from zero with `S_next <= 2(S + 2M)` gives S <= 60M < 64M.
- A model-times-skin four-term dot product has terms <= 128KM. The same recurrence
  gives a sum <= 3840KM < 4096KM.
- A position four-term dot product (including the homogeneous 1) has terms
  <= 8192KMP and a sum <= 245760KMP < 2^18 KMP.

These positive envelopes bound all earlier partial products and sums as well as
any reassociation/FMA evaluation of the four-term expressions. They are much looser
than the usual f32 relative-error bounds. Cancellation does not invalidate an
absolute-magnitude envelope.

The implemented f64 bound B = K*M*P uses two multiplications. Inputs convert exactly
from f32, are >= 1, and their product is below 2^384, so binary64 neither underflows
nor overflows. Its rounding error is far less than a factor of two: actual KMP < 2B.
Admission requires B <= f32::MAX / 2^20. Thus every rounded position/intermediate
has magnitude < 2^18 * 2B <= f32::MAX/2 and must remain finite. The extra margin
covers the binary64 bound calculation without needing directed rounding.

If the certificate is insufficient, the original coherent/plain per-vertex scan
runs and decides admission. A huge but finite translation can therefore be admitted
by the fallback. No approximate transformed coordinates are published, no normal
constraint is added, and invalid palette/model errors retain their original priority.
Meshes below 1024 vertices retain the original path to avoid certificate overhead.

Regressions compare with the original scan for reflection, collapsed normal
transforms, nonfinite/short palettes, overflowing positions, shared index views,
changing valid-invalid-valid inputs, and 125 combinations of position/joint/model
scale exponents with mixed/cancelling weights.
