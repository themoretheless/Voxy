# Cubic glTF TRS animation

The importer accepts CUBICSPLINE channels alongside LINEAR and STEP. Each input
key decodes an incoming derivative, value and outgoing derivative. Derivatives
remain vectors: quaternion derivatives are not normalized. All three output
elements count toward ModelLimits.keys before decoding. Output counts must be
exactly three times input counts; cubic channels require at least two keys.

JointTrack and existing constructors retain their previous layouts and LINEAR
behavior. The additive JointTangents payload and new_with_tangents constructor
validate finite derivatives, channel mode/count correspondence, and reject unused
streams attached to non-cubic channels. Derivatives are stored in value units per
second and multiplied by each segment's duration in the Hermite formula.

Rotation interpolation uses component-wise Hermite followed by normalization,
without LINEAR's shortest-path sign flip. Exact keys and out-of-range channels
retain endpoint values. Quaternion normalization rescales finite components to
avoid squared-length overflow. A zero or non-finite interpolated quaternion is
invalid, rather than replaced by an invented orientation.

Runtime admission uses try_sample through ModelAsset.sample_pose and Animator.
It rejects invalid interpolated local TRS before constructing a GPU palette.
Animator retains its previous time/transition on failure. ModelPlayback does not
invoke its publication consumer for an invalid pose and retains the owner's
clock. The legacy infallible AnimationClip.sample API can return an invalid cubic
pose; runtime callers must use try_sample or the validated asset/animator path.
Skeleton compatibility validation checks joint count, not skeleton identity.

Analytic regressions cover nonzero translation/rotation derivatives, segment
length in seconds, normalized quaternion output, mixed cubic/LINEAR channels,
clamping, exact endpoints, malformed derivative/count streams, key budget,
scale overshoot, a zero scale, a zero quaternion, failed active transitions, and
failed owner publication without clock advancement.

GPU acceptance derives a cubic fixture from the pinned RiggedFigure by retaining
key values and inserting zero derivatives. This is a synthetic cubic rig, not
an unchanged third-party authored cubic clip. All 65 samples compare positions
and normals with CPU and require visible rendered pixels. Error maxima on Metal:
positions 2.3841858e-7, normals 1.9082798e-7. The fixture has 370 vertices and a
22-node palette; its original provenance remains under examples/assets/rigged-figure.

Primary source:
https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#_cubic_spline_interpolation

Signed nonzero scales are supported; see [mirrored rigs](mirrored-rigs.md).
A singular zero scale rejects at runtime and preserves the previous pose. Morph-weight channels, retargeting, editor blend controls, arbitrary rigs,
unchanged authored cubic asset acceptance, native-window cubic screenshots and
non-Metal/CUDA hardware acceptance remain incomplete. Sampling tests are discrete
and do not certify every point of an entire continuous curve.

235 ordinary tests (15 animation, 91 editor, 129 renderer) and the 65-pose GPU
acceptance passed. Logs and source hashes are retained under
artifacts/rig-cubic-interpolation-2026-10-03/.
