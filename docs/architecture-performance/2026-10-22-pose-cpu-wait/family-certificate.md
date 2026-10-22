# Sufficient convex joint-family certificate

This is admission only. The existing matrices, vertex positions/normals, palette
and GPU writes are not computed through intervals or reordered. Palette finite /
affine validation runs first. Failure of any sufficient bound returns to the old
per-weight-group certificate, then the exact vertex-order fallback.

A family is the sorted set of the nonzero-weight joint IDs of an immutable
influence group. Repeated IDs are removed; inactive slots use a sentinel. Real
normalized weights `integer / 65535` are nonnegative and sum to one. Therefore each
exact blended matrix entry is inside the minimum/maximum of the corresponding
family joint entries, for every possible set of those weights. This removes the
need to recompute one certificate for each of 518,549 distinct quantized weights
in the measured mesh.

## Floating-point enclosures

Let epsilon = 2^-23 (twice binary32 unit roundoff), T = binary32 MIN_POSITIVE,
and J be the largest absolute endpoint of a particular joint-entry interval.

1. Widen that interval by `16 epsilon J + 16 T`. Four rounded UNORM16 decodes,
   products and accumulations have absolute error below this bound, including
   cancellation and underflow. Using T instead of the smallest subnormal is
   deliberately conservative. Duplicate joints and zero slots do not affect
   convexity. The incoming integer sum was validated at mesh creation.
2. Multiply this enclosed 3x3 linear part by the current model's linear part,
   with binary64 interval endpoints. For each dot product let S be the sum of
   `abs(model coefficient) * enclosed skin entry max`. Widen by
   `32 epsilon S + 32 T`. This covers the f32 4-term matrix product (the affine
   fourth term is exactly zero) and the far smaller f64 endpoint rounding.
   Binary32 input values are exactly representable in binary64. Restrict model
   and family matrix magnitudes to at most 2^32; uncertain/extreme inputs use the
   original path. Thus there is no overflow in these intermediate dot products.
3. The actual normal scale is the maximum absolute computed linear entry.
   Its lower bound is the maximum of each entry interval's minimum absolute
   value; its upper bound is the maximum of all endpoint magnitudes. Reject
   uncertain/invalid scales. Enclose the reciprocal, then each scaled entry;
   widen by `8 epsilon` for reciprocal/multiply rounding and binary64 endpoints.
   Intersect with `[-1-4 epsilon, 1+4 epsilon]`, also a sufficient enclosure since
   division uses that same maximum entry. Near-underflow scales fail the interval
   scale bound and use the original path.
4. Evaluate the six-term determinant in interval arithmetic and widen by
   `64 epsilon` for binary32 determinant rounding, including contraction or
   reassociation of its fixed small expression and f64 endpoint arithmetic.
   The absolute-entry bound from step 3 bounds all products and sums here.
   Require the entire determinant interval to lie above +1/512 or below -1/512.
   This implies the existing `NormalTransform` certificate's absolute scaled
   determinant >=1/1024, and exceeds its sign-uncertainty bound
   `16 epsilon * 6 * (1+4 epsilon)^3`.

The existing per-group normal certificate already guarantees finite, nonzero
cofactor-transformed admissible nonzero authored normals under that determinant
margin. The immutable `has_bad_normal` aggregate retains the original checks for
subnormal/extreme authored normals. Zero normals retain their original behavior.
The existing conservative `finite_temporal_positions` bound is additionally
required for the entire mesh/palette/model; it bounds the render matrix product
and subsequent point product as well as the temporal multiplication order. No
position acceptance or normal validation is omitted without a sufficient proof.

## Regression oracles

The tests compare all 65,536 UNORM16 pairs for certified two-joint families against
the existing scaled determinant and per-vertex normal routines. Additional tests
exercise four influences, cancellation, rotations, reflected/singular models,
scales across exponents -120 through +120, extreme authored normals, shared-index
views, valid/invalid/valid pose changes and exact fallback errors. Tests supplement
the conservative bounds above; sampling alone is not their soundness argument.
