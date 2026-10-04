# Composed root velocity

RootRigidSpan::velocity returns origin linear and angular velocity in the interval-start frame, per clip second. It is not a body-coordinate twist; callers must convert origins and coordinate frames explicitly before any velocity blend.

For the existing representation t(u) = a(u) - R(u)p(u), the analytic derivative is a_dot - omega cross (R p) - R p_dot. Bernstein differences supply a_dot and p_dot; RootRotationSpan supplies the normalized quaternion derivative omega. This retains the rotating pivot term, which cannot be recovered by differentiating the translation keys alone. Nonfinite fractions, singular rotations and overflowing velocities reject. Zero-duration STEP events return None, including underlying rotation spans whose interpolation shape is an arc; these events remain in the collision trajectory.

All 77 animation CPU tests pass. Independent linear-pivot and normalized cubic quaternion/moving-pivot formulas check both origin velocity and transformed-point velocity; linear paths also compare central differences along three rotated axes. STEP tests prove no finite velocity and invalid-fraction rejection. Evidence: artifacts/rig-composed-velocity-2026-10-04/.

Moving angular crossfade trajectories remain unimplemented and rejected. This API is a prerequisite, not crossfade acceptance. Next work must define common-frame velocity transport, integrate blended rotations and translations with certified interval error/bounds, preserve simultaneous STEP events, split fade completion tails, and carry interruption/failure atomicity through CharacterPhysics and native rendering.

## Velocity coordinate transport (2026-10-04)

RootRigidVelocity::transformed now applies the same similarity map as RootRigidPath::transformed. At source rotation R, angular velocity becomes B omega and origin linear velocity becomes s B v - (B omega) cross (B R B^-1 c). The second term is required by the translated target origin; multiplying linear velocity by scale alone is incorrect, including at zero scale. Rotation at the sampling time is supplied explicitly. Invalid source velocities/coordinates and overflowing results reject without publishing state.

Coordinate-path regression now checks this velocity transform against differentiation of the actually transported path at 101 fractions of each span for LINEAR, STEP and normalized CUBICSPLINE intervals across loops, with scales zero and 2.5. Both paths must agree on STEP velocity absence. A separate analytic zero-scale offset case and invalid/overflow/retry checks pass. All 78 animation CPU tests pass. Evidence: artifacts/rig-velocity-transport-2026-10-04/. This supplies coordinate transport, not a moving-fade integrator or certified blended collision path.

## Constant spatial twist integration (2026-10-04)

RootRigidVelocity::spatial_twist converts origin velocity at transform (R,t) to fixed-frame twist linear component v = t_dot - omega cross t. RootRigidTwist::increment produces the constant-twist SE(3) exponential; its result must left-compose the starting transform. This avoids confusing origin derivative with spatial linear twist or reversing composition order.

The closed-form translational Jacobian follows the rigid exponential in [Modern Robotics MatrixExp6](https://github.com/NxRLab/ModernRobotics/blob/master/packages/Python/modern_robotics/core.py). This implementation uses unit-axis cross products, scaled robust angular norm and small-angle coefficient series; it does not discard a nonzero small rotation. Invalid duration/nonfinite inputs and overflowing output reject. This floating-point formula is not an interval error certificate.

Independent offset-pivot circular motion plus axial translation checks rates 0, 1e-9, 1e-4, 0.3 and 7 over durations 0, .001, .2 and 1. Tests also prove two half increments match one full increment and verify fixed-frame left composition from a nonidentity starting transform. Invalid duration/velocity, overflow and valid retry are covered. All 79 animation CPU tests pass. Evidence: artifacts/rig-spatial-twist-2026-10-04/.

This is exact integration for constant twist in real arithmetic, with tested floating-point evaluation. Time-varying mixed velocities, noncommuting angular fields, certified collision envelopes, fade tails/interruption and editor/native acceptance remain required before enabling general moving angular fades.

## Ordered constant-twist collision paths (2026-10-04)

RootRigidPath::from_twists now stores positive-duration constant-spatial-twist segments in the existing path/span API. Each segment samples its SE(3) exponential left-composed with the preceding accepted endpoint. Constant angular spans retain winding and speed bounds; origin velocity is omega cross current_translation + spatial_linear. This adds no separate playback clock or physical trajectory system. Existing imported polynomial/STEP spans retain their representation.

For a fixed point, constant spatial twist gives x_dot = omega cross x + v and x_ddot = omega cross x_dot, hence its speed magnitude is constant. The new span supplies that speed times duration with a floating guard. A midpoint projection enclosure uses half of the full-span point travel bound times normal length, plus a floating guard. The bound is conservative in real arithmetic; these are tested floating-point bounds, not interval-certified integration of a changing velocity field. Similarity conversion transports both the initial transform and spatial twist, retaining span order.

The regression checks two noncommuting angular segments, endpoint composition and boundary continuity, coordinate/velocity transport at scales zero and 2.5, projection/speed enclosures at 101 fractions per span, span capacity, invalid duration and empty-path admission. All 80 animation CPU tests pass. Evidence: artifacts/rig-ordered-screw-path-2026-10-04/. CharacterPhysics collision acceptance for these new spans is still required, as are certified approximation of varying blended fields and native verification.

## CharacterPhysics acceptance of ordered screw spans (2026-10-04)

The ordinary CharacterPhysics rigid-trajectory API now has a regression using RootRigidPath::from_twists: a 0.1-unit X translation followed by a yaw screw around external pivot X=0.6. The body's front support is independently .9*sin(angle)+.02*cos(angle); wall first contact gives angle asin(.23/hypot(.9,.02))-atan2(.02,.9). The accepted normalized time is (.1+angle/2)/.6, with center (.6-.5*cos(angle),0,.5*sin(angle)). Path time, displacement and rotation agree within 1e-7 and the first span is completed before collision.

A one-query trajectory budget rejects without changing scene pose or consuming jump input. Retry with normal budget accepts the analytic contact and consumes input; the next ordinary physical tick retains the accepted pose. All 42 gameplay integration tests pass. Evidence: artifacts/rig-screw-physics-2026-10-04/. This proves the new constant-twist spans run through existing physical collision acceptance; time-varying blended-field integration, authoring use and native rendering remain unimplemented/unverified.

## Bounded varying spatial twist integration (2026-10-04)

RootRigidPath::integrate_spatial constructs ordered constant-twist approximations to a continuous fixed-frame field. Uniform refinement doubles span count up to the existing global cap. It returns RootRigidApproximation with the path and origin/angular discretization error bounds, or rejects when tolerances cannot be met within capacity. Constant fields need one span. STEP events must be split separately. The callback must be deterministic and derivative rate bounds must be proved by the caller over the whole interval; samples do not establish these bounds.

For a span of duration h with initial approximate origin t0 and frozen spatial linear velocity v0, the local origin discrepancy is at most h^2/2*(Lv + Lw*|t0|) + Lw*|v0|*h^3/3. This follows from e_dot = omega_true cross e + delta_omega cross t_approx + delta_v: skew dynamics preserve the norm of propagated prior error, and |t_approx(s)| <= |t0| + |v0|*s. Angular discrepancy is bounded by Lw*h^2/2. Summing bounds therefore covers every prefix. These are conditional real-arithmetic discretization bounds; numerical exponential evaluation/roundoff is not certified.

Independent tests cover xi(t)=t*xi0 (exact endpoint exp(xi0/2)) and a noncommuting authored rotation Ry(t^2/2)*Rx(t/2) with translation (sin(t),.3*t^2,0). Returned envelopes cover origin/orientation at span boundaries and midpoints. Capacity rejection, invalid derivative bounds, callback failure, constant-field admission and zero duration are verified. All 82 animation CPU tests pass. Evidence: artifacts/rig-varying-twist-2026-10-04/.

Approximated varying fields are not enabled in physical animation playback. Before that, error envelopes must participate in collision queries, numerical evaluation must receive adequate conservative guards, source/target clip derivative bounds must be constructed, fade completion/interruption and STEP events must retain atomic publication, and native presentation must be verified.

## Approximation envelopes through retarget coordinates (2026-10-04)

RootRigidApproximation::transformed transports path and discretization errors together. With origin error e and angular error a, the target origin discrepancy is bounded by scale*e + 2*sin(min(a,pi)/2)*|offset|. Rotation conjugation preserves angular discrepancy. RootRigidApproximation::point_error_bound supplies e + 2*sin(min(a,pi)/2)*|point| for a fixed collision point. The angular contribution is capped by the full rotation chord at pi, not dropped when similarity scale is zero. Nonfinite/negative error metadata and invalid coordinates reject. These remain conditional mathematical discretization bounds; numerical evaluation is not certified.

A independently integrated commuting ramp checks target origin/body-point errors at every span boundary and midpoint for scales zero and 2.5, a nonidentity basis and large shifted origin. Tests cover the large-angle chord cap and invalid metadata/point/scale rejection. All 83 animation CPU tests pass. Evidence: artifacts/rig-approximation-error-transport-2026-10-04/. Physical consumption of these envelopes, derivative proofs for actual blended clips and numerical evaluation certification remain required.

## Compiled angular derivative bounds (2026-10-04)

RootRotationSpan::angular_acceleration_bound supplies a within-span rate proof for compiled rotation channels. Held and constant angular arcs return zero, STEP/zero-duration events return no finite bound. Cubic spans use the existing positive raw quaternion norm lower bound m and derivative Bernstein hulls M1, M2 per clip second. The normalized angular derivative magnitude is bounded by 2*M2/m + 4*(M1/m)^2, plus a floating guard for coefficient evaluation. Nonpositive norm proofs and overflow reject.

The derivation writes angular velocity as twice the imaginary part of q_dot*q^-1. Differentiating gives q_double_dot*q^-1 minus (q_dot*q^-1)^2; the latter imaginary part is bounded by twice the product of its scalar and vector parts. Unit left/right coordinate changes preserve these norms. This bound applies within one compiled span; finite velocity jumps at keys and loops require integration interval splitting, even when pose is continuous.

The closed cubic yaw regression checks the independent acceleration of q=normalize(0,8*t*(1-t),0,1) across every split span. Constant linear turn and STEP regression checks are also extended. All 83 animation CPU tests pass. Evidence: artifacts/rig-angular-acceleration-bound-2026-10-04/. Full spatial linear derivative bounds, blended-field rates and key/fade interval partitioning remain required.

## Full spatial derivative bounds from compiled spans (2026-10-04)

RootRigidSpan::twist_rate_bounds now supplies both fixed-frame spatial linear and angular derivative bounds. For t=a-Rp, spatial linear twist is a_dot - omega cross a - R*p_dot. Its derivative is a_double_dot - alpha cross a - omega cross a_dot - omega cross (R*p_dot) - R*p_double_dot. Bernstein position/first/second derivative hulls plus compiled rotation speed/acceleration bounds give the corresponding norm enclosure. This cancellation avoids introducing irrelevant constant-pivot radii into spatial linear bounds. A floating guard is included, but full numerical certification remains absent. STEP returns no finite derivative; constant spatial screw spans have exactly zero derivative.

Independent normalized cubic yaw/moving-pivot derivatives are covered for selected and unselected translation axes. A new integration regression consumes actual compiled linear-turn clip velocities and its computed derivative bounds, comparing the integrated result with the original root path and independent quarter-turn pivot equation within the returned error. All 84 animation CPU tests pass. Evidence: artifacts/rig-spatial-rate-bound-2026-10-04/. Key/fade partitioning, common-frame blended derivative bounds and physical error-envelope consumption remain required.

## Shared blend interval partition (2026-10-04)

RootRigidPath::partition_for_blend produces shared normalized-progress intervals and separate STEP records from two ordered paths. Each interval identifies its original source/target non-STEP span, or None for a stationary gap. Differing path durations remain explicit; callers must retime velocity and derivative bounds to the shared wall interval. All span boundaries are unioned without tolerance snapping. Positive spans that collapse during normalization reject instead of silently disappearing.

Simultaneous STEP records retain both original index lists and their per-path ordering; this API does not infer how source/target events physically compose. Combined interval plus event count obeys the existing global cap. After sorting shared cuts, span attribution uses advancing cursors rather than repeatedly scanning all spans.

Tests cover .25/.5 shared boundaries from paths of durations one and two, original STEP identity/order, stationary paths, capacity exhaustion and extreme-duration normalization collapse. All 85 animation CPU tests pass. Evidence: artifacts/rig-blend-partition-2026-10-04/. Blended field construction, STEP policy, fade completion tails, error-aware physical acceptance and native verification remain unfinished.

## Common-frame blend field bounds and playback retiming (2026-10-04)

RootRigidSpan::twist_bounds now pairs magnitude and derivative bounds. Spatial linear magnitude follows |a_dot| + |omega|*|a| + |p_dot|; constant screw spans use their stored twist magnitudes. RootRigidTwist and RootSpatialTwistBounds support nonnegative constant playback retiming: velocity scales by speed and derivatives by speed squared.

Linear-weight common-frame blending is explicit. For xi=(1-w)*xi_source+w*xi_target, its derivative contains (1-w)*xi_source_dot+w*xi_target_dot + w_dot*(xi_target-xi_source). Bounds use the maximum endpoint-weighted derivative enclosure plus |w_dot| times source/target magnitude bounds. Coordinate alignment and key-free intervals are caller obligations; this API does not infer them or modify Animator clocks. Invalid/nonfinite weights, speeds, durations and overflow reject.

The integration regression starts from compiled constant screw bounds, retimes source by .5 and target by 2, and integrates the resulting linear fade. Its commuting field (.5+5.5*t)*xi0 has independent endpoint exp(3.25*xi0); integrated errors fit returned envelopes. Existing independent cubic moving-pivot tests now check spatial speed magnitude bounds too. All 86 animation CPU tests pass. Evidence: artifacts/rig-blended-twist-bound-2026-10-04/. Animator fade tails/interruption, common-frame clip alignment, STEP handling, numerical/error-aware physical admission and native rendering remain unfinished.

## Spatial twist and bound alignment across rig coordinates (2026-10-04)

RootRigidTwist::transformed maps angular velocity to B*omega and spatial linear velocity to scale*B*v - (B*omega) cross offset. RootSpatialTwistBounds::transformed transports linear speed/rate bounds as scale*Mv + Momega*|offset| and scale*Lv + Lomega*|offset|, preserving angular bounds. This differs from origin-velocity transport and is the fixed-frame operation required before blending. RootRigidPath's screw coordinate conversion now reuses the shared twist transformation rather than its own duplicate formula.

Across existing LINEAR/STEP/CUBICSPLINE loop-path coordinate tests, continuous samples compare directly transported spatial twists with origin derivatives converted back from the already-transformed path, and transferred magnitude bounds cover samples. Independent zero-scale shifted-origin tests verify nonzero linear twist and derivative coupling. Invalid basis/scale/offset and overflow reject. All 87 animation CPU tests pass. Evidence: artifacts/rig-twist-common-frame-2026-10-04/. Actual Animator source/target frame choice, fade lifecycle, event policy, physical approximation envelopes and numerical certification remain incomplete.

## Animator-owned staged rigid fade plans (2026-10-04)

Animator::prepare_root_rigid_fade now exposes a staged active-fade candidate from the sole existing source/target clocks. RootRigidFadePlan contains the candidate Animator, full-tick displayed frame, source/target fade paths, target-only completion tail, weight endpoints, wall durations and source/target/tail root extraction factors. Factors retain each curve's authored-origin convention; they are explicitly not inferred actor-world frames. Consumers must choose common coordinates and account for the tail's separate start factor.

Captured interruption poses have zero source motion and no guessed source clip interval/factor. The method uses immutable Animator state, compiles all candidate paths, checks their aggregate span count, then advances a clone for the displayed frame. Failure leaves the accepted clocks untouched. The returned candidate must only publish after trajectory integration/physical acceptance. Existing moving angular advance APIs still reject unsupported transitions.

Regression covers different source/target start phases, completion prefix/tail, pose equality with ordinary advancement, unchanged clock after preparation and capacity failure, frozen-source interruption, invalid dt and paused completion. All 88 animation CPU tests pass. Evidence: artifacts/rig-animator-fade-plan-2026-10-04/. Common actor-frame alignment, assembled blended path, STEP policy, physical numerical/error-envelope checks and native presentation remain unfinished.

Spatial concatenation now also has regression evidence for simultaneous and separate STEP translation/rotation events after a noncommuting screw prefix. Event timestamps shift, per-event order and intermediate swept transforms survive, and instantaneous velocity remains undefined. All 89 animation CPU tests pass; evidence: `artifacts/rig-spatial-step-append-2026-10-04/`. This does not enable moving fades in physical runtime.


`RootRigidPath::blend_spatial` now assembles a linear-weight spatial velocity fade from two compiled paths in an explicitly shared frame. It retimes each complete path to a common wall interval, partitions at both paths' keys, derives speed/rate bounds from the selected spans, integrates each partition and left-composes the resulting screw paths. A missing stationary source contributes zero twist. STEP inputs reject with `RootRotationTransitionUnsupported`; a discrete-event blend policy remains required.

The origin envelope allocation reserves half the requested tolerance for local origin errors and half for angular errors transported through preceding translations. The sum of interval linear-speed bounds times duration bounds the approximate prefix radius; each local angular tolerance is reduced accordingly. Aggregate span capacity includes the unprocessed intervals, so refinement cannot silently bypass the caller budget. Returned errors remain conditional real-arithmetic discretization bounds: roundoff and collision acceptance are separate outstanding requirements.

Independent tests integrate a commuting piecewise fade analytically (different clip durations and a source key at wall progress 0.4), including frozen-source motion. A zero-error constant-weight test compares two noncommuting rotations against explicit quaternion exponentials in their correct order. This API does not yet select actor frames, compose the Animator's completion tail or enable moving physical fades. Evidence: `artifacts/rig-compiled-spatial-blend-2026-10-04/`.


`RootRigidFadePlan::integrate_spatial` now transports the staged source/target paths through explicitly supplied rigid frames, assembles the blended fade and continues the target completion tail in the same fixed frame. If the target fade-local path is D and its chosen frame is P, the tail-local path E is conjugated by P D(end), then left-composed with the accepted approximation of the blend. This preserves the target spatial field at completion; blindly restarting E at the blended actor axes would change that field. Frozen sources require no invented frame. The runtime still has to choose these frames and physically accept the candidate before publishing its Animator.

`RootRigidPath::retimed` preserves polynomial, screw and STEP geometry while replacing wall timestamps and rescaling speed/twist for each positive-duration span. It rejects collapsing moving spans and allows an empty stationary path to carry elapsed wall time. The target tail is retimed from clip seconds to wall seconds before concatenation. Exact tail geometry adds no discretization envelope; evaluation roundoff remains uncertified.

All 93 animation CPU tests pass. A real Animator plan at 2x playback independently checks the conjugated tail samples, wall duration, tail angular speed and unchanged original clock, with nonidentity source/target frames. Missing source frames and insufficient span budgets reject. Evidence: `artifacts/rig-fade-common-frame-tail-2026-10-04/`. Moving physical fades and STEP blend policies remain disabled.


`RootRigidApproximation::span_projection_bounds` exposes per-span projection enclosures expanded by `|normal| * (origin_error + 2*sin(min(angular_error,pi)/2)*|point|)`. This accounts for offset body vertices and nonunit separating axes without pretending the nominal trajectory is the original velocity field. A regression places a stationary nominal vertex outside the true translated/rotated projection: the nominal bound misses it and the expanded enclosure contains it. Invalid span indices, points, normals and error metadata reject. All 94 animation CPU tests pass; evidence: `artifacts/rig-approximation-projection-2026-10-04/`.

Gameplay's sweep still consumes exact `RootRigidPath` spans. Using this projection expansion for broadphase alone would be insufficient: narrowphase distances and conservative advancement also need error-aware treatment. Numerical evaluation uncertainty remains a separate requirement. No approximate physical path is admitted by this addition.


The shared gameplay advancement kernel now has `advance_with_clearance`: a caller-proved uniform Hausdorff radius is subtracted from each unit-axis SAT gap before conservative advancement. Inflating each nominal pose by a fixed radius does not require a derivative bound for approximation error: that fixed envelope moves with the nominal point speed. The hit is an envelope contact, not evidence that the original body truly touches the obstacle. A stationary nominal pose with positive clearance still needs an initial overlap check.

Existing exact-trajectory callers use the same kernel with zero clearance. Tests independently locate linear envelope contact at `(1.8 - 0.2)/3`, verify stationary overlap/clearance, and retain existing angular arc cases. All 44 gameplay unit tests pass (5 sweep tests); evidence: `artifacts/rig-sweep-clearance-kernel-2026-10-04/`. Broadphase envelope wiring, corner-radius derivation, numerical bounds and candidate acceptance remain outstanding; no approximate physical path is admitted yet.


The rigid-path sweep now uses a shared clearance-aware implementation. Positive clearance expands the broadphase support test and routes candidates into the same clearance-aware advancement kernel; unlike exact zero-clearance paths, stationary nominal spans still query possible envelope overlap. Positive-envelope separation requires a strictly positive gap beyond the numerical guard, preserving candidate contacts at envelope tangency.

The regression uses a nominal translation of 0.7 toward a wall whose initial body gap is 0.8: the nominal path clears it, but a 0.2 envelope stops at fraction 0.6/0.7. It also checks stationary overlap, query exhaustion and invalid clearance. All 45 gameplay unit tests and 42 integration tests pass; evidence: `artifacts/rig-sweep-clearance-broadphase-2026-10-04/`. Public physical requests still select zero clearance. Automatic whole-body discretization/numerical envelope derivation and approximate candidate admission remain outstanding.


Gameplay now derives whole-body clearance from all eight collider corners mapped into the root path's source coordinates. The radius is `abs(scale) * max_corner(point_error_bound) + evaluation_radius`, with a separately supplied world-space numerical uncertainty bound. The shared inverse mapping validates the basis, signed nonzero scale, origin and finite corner coordinates. This handles a displaced pivot and negative scale without losing the angular lever arm.

The internal read-only `sweep_rigid_approximation` connects that derived radius to broadphase and advancement. It remains disconnected from runtime candidate publication because its numerical evaluation bound is still a caller proof obligation. Empty stationary paths with positive clearance now check overlap rather than bypassing the envelope. Tests compare all corners against independently perturbed transforms at scales -2, 0.5 and 2, check combined origin/evaluation contact and empty stationary overlap. All 46 gameplay unit tests and 42 integration tests pass; evidence: `artifacts/rig-whole-body-envelope-2026-10-04/`.


`RootRigidTwist::increment_enclosure` now encloses the real exponential of exact stored f64 twist/duration inputs for a small-angle increment. Scalar operations round outward with adjacent representable values; the angular squared norm's interval upper bound must be at most one. Borderline inputs can reject conservatively. Coefficients use eight alternating Taylor terms and the interval magnitude of the ninth term as a remainder bound: A=(1-cos(theta))/theta², B=(theta-sin(theta))/theta³, sin(theta/2)/theta and cos(theta/2). Cross products then enclose translation and quaternion components. No platform transcendental functions, floating quaternion normalization or sampled discrepancy enters the enclosure.

The arithmetic model requires IEEE-754 round-to-nearest basic operations, gradual underflow and no fast-math reassociation. Adjacent value operations are specified in the [Rust f64 documentation](https://doc.rust-lang.org/std/primitive.f64.html#method.next_up). Nonfinite inputs, overflow and larger-angle intervals reject. This is one increment's enclosure; accumulated prefix error, similarity/frame conversion and collision evaluation remain unproved.

All 95 animation CPU tests pass. The saved independent Decimal verifier uses 400-digit Rodrigues/quaternion evaluation of exact f64 inputs; all five zero/axial/general/tiny-angle cases are inside the computed component intervals with relative width below 1e-12. Evidence: `artifacts/rig-increment-enclosure-2026-10-04/`. Neither these examples nor this increment-only API certify the entire moving fade or enable runtime admission.


The numerical enclosure owner is now `RootRigidEnclosure`, with private translation/quaternion bounds, exact identity, ordered `compose` and `transform_point`. Private fields retain the unit-rotation invariant supplied by true exponential increments and their real compositions. Quaternion action is enclosed algebraically without normalizing floating approximations. All scalar operations in composition and point transformation round outward; overflow or invalid points reject. This accumulates numerical uncertainty of stored increments, not the derivative/coordinate extraction errors that produced those increments.

All 96 animation CPU tests pass. An independent 400-digit verifier composes Rodrigues matrices for 2, 16 and 64 noncommuting increments, derives quaternion components from the matrix and transforms an offset point. Translation, rotation and point references lie within the accumulated intervals. Evidence: `artifacts/rig-prefix-enclosure-2026-10-04/`. Linking these intervals to every actual path span/time, transformed frame and collision calculation remains outstanding.


`RootRigidPath::screw_field_enclosure` links numerical enclosures to the canonical ordered field represented by stored screw rates and stored timestamps. Interval subtraction encloses exact endpoint-time differences; interval multiplication encloses the sampled fraction. Prefix transforms are recomputed outward from identity, rather than treating stored floating initial prefixes as exact. The API admits only screw prefixes within the small-angle enclosure domain and explicit span-work budget. Polynomial/STEP prefixes and invalid samples reject.

This certificate's reference is the canonical field of stored data. It does not certify that curve extraction or velocity blending produced exact input rates, nor the whole runtime evaluation pipeline. All 98 animation CPU tests pass; an independent 400-digit Rodrigues matrix oracle checks 12 span/fraction cases using exact differences of the actual stored timestamps, including transformed point coordinates. Evidence: `artifacts/rig-screw-field-enclosure-2026-10-04/`. Runtime moving-fade admission remains disabled.


`RootRigidEnclosure` now supports a frame initialized by exact stored translation and real normalization of a validated unit-near stored quaternion, plus inverse and signed uniform similarity transformation. Quaternion norm sqrt and division round outward. The [Rust sqrt precision guarantee](https://doc.rust-lang.org/std/primitive.f64.html#method.sqrt) specifies rounded infinite-precision square root; adjacent representable values then bound it under the same arithmetic model. Inverse and conjugation retain private unit-rotation enclosure invariants.

Similarity is evaluated as F * (R,scale*t) * F^-1, so translated-origin angular coupling remains present at zero scale, and negative scale reflects translation without changing proper quaternion rotation. All 99 animation CPU tests pass. Independent 400-digit matrix conjugation of normalized exact stored frame inputs verifies transform and point bounds at scales -2, 0, 0.5 and 2. Evidence: `artifacts/rig-frame-enclosure-2026-10-04/`. This closes numerical frame arithmetic for this defined reference; upstream rate extraction and collision arithmetic still require their own bounds before physical moving-fade admission.


`RootRigidTwistEnclosure` now owns outward fixed-frame velocity arithmetic: exact stored twist initialization, clip/wall ratio retiming, linear endpoint-weight interpolation at stored progress, and similarity transport through a `RootRigidEnclosure`. The same scalar implementation encloses division, weight subtraction, vector rotation and the shifted-origin term `-omega' cross offset`; signed and zero scales retain the angular coupling. Fields remain private. Source velocity extraction and any uncertainty preceding these stored inputs are not inferred or certified by this API.

All 100 animation CPU tests pass. An independent 400-digit verifier evaluates exact stored durations, weights and progress, then normalized-frame matrix transport at scales -2, 0, 0.5 and 2. Every linear/angular reference lies within the computed intervals. Evidence: `artifacts/rig-twist-arithmetic-enclosure-2026-10-04/`. General imported curve derivative enclosures and collision arithmetic remain outstanding before physical moving-fade admission.


`RootRotationSpan::cubic_angular_velocity_bounds` encloses the rational derivative of the normalized stored cubic quaternion. Outward De Casteljau evaluation gives the polynomial and its clip-time derivative; the imaginary part of `2*q_dot*conjugate(q)/|q|²` gives angular velocity without assuming floating quaternion normalization is exact. The normalized real stored left frame transports it to the span frame; the constant right frame cancels. An unproved positive denominator rejects. Non-cubic shapes return None, rather than claiming a certificate for another interpolation rule.

All 101 animation CPU tests pass. The independent 400-digit verifier differentiates the normalized rotation matrix directly, multiplies R_dot by R_transpose and extracts its skew angular vector, then applies the left frame matrix. It covers five fractions of each compiled test span. Evidence: `artifacts/rig-cubic-angular-enclosure-2026-10-04/`. This encloses stored-coefficient angular arithmetic only; compilation error before those coefficients, spatial linear/pivot derivative arithmetic and collision evaluation remain outstanding.


`RootRigidSpan::spatial_twist_enclosure` now evaluates stored screw fields or the complete cubic-rotation polynomial spatial field. Shared outward polynomial evaluation gives additive/pivot values and clip-time derivatives. Normalized real cubic rotation, both constant frames and angular velocity are enclosed together; linear spatial velocity uses `a_dot - omega cross a - R*p_dot`. This retains moving-pivot and translated-origin coupling rather than substituting origin velocity for a spatial twist. Unsupported interpolation and instantaneous spans return None.

All 102 animation CPU tests pass. An independent 400-digit verifier differentiates the rotation matrix and evaluates polynomial additive/pivot terms for all eight extraction-axis masks in shifted, rotated, scaled coordinates: 280 span/fraction references are enclosed. Evidence: `artifacts/rig-cubic-spatial-enclosure-2026-10-04/`. These are stored-coefficient arithmetic enclosures, not bounds on compilation from original keys. Held/arc field enclosures, propagation of sampled rate uncertainty into integration and full collision arithmetic remain outstanding before runtime moving-fade admission.


The same stored-field spatial enclosure now supports held and LINEAR arc rotation. Angular velocity transports the exact stored axis divided by the exact timestamp difference through normalized real left/from frames. Arc orientation uses ordered outward small-angle exponentials; subdivision is selected by an outward L1 angular bound and capped by existing 4096-span work capacity. It preserves winding instead of reducing the arc to endpoint orientation. The shared additive/pivot derivative formula remains unchanged. `angular_velocity_bounds` covers all continuous interpolation modes; STEP and zero-duration events return None.

All 103 animation CPU tests pass. The independent 400-digit Rodrigues/matrix verifier checks 40 LINEAR spatial-field cases across all eight extraction masks, while held translation and STEP regressions pass in the CPU suite. Evidence: `artifacts/rig-arc-spatial-enclosure-2026-10-04/`. Continuous stored-field velocity arithmetic is now covered; propagating these sampled uncertainties into integration and bounding the collision pipeline remain outstanding. Runtime moving fades stay disabled.


`RootRigidTwistEnclosure::error_bounds` computes outward L1 discrepancy bounds from a finite nominal twist; these also bound Euclidean discrepancies. Exact singleton equality remains exactly zero. `RootTwistErrorBounds` keeps fields private and distinguishes sampled velocity error from temporal derivative bounds.

`RootRigidPath::integrate_spatial_enclosed` now shares the existing integration kernel while adding sampled errors. For a frozen nominal sample, per-span origin error adds `ev*h + ew*(|prefix.translation|*h + |v_nominal|*h²/2)`; angular error adds `ew*h`. The original field's derivative bounds still cover variation after the sample. These terms follow the skew dynamics error inequality and cannot be removed merely by refining a biased sample. Existing exact-sample callers supply zero sample error through the same kernel.

All 104 animation CPU tests pass. An analytic commuting constant field versus a 10-percent biased nominal field is contained by the returned envelopes, and a tighter tolerance rejects after capacity exhaustion instead of reporting false accuracy. Evidence: `artifacts/rig-sampled-velocity-error-2026-10-04/`. Floating accumulation of these errors, clock mapping and the full collision arithmetic remain separate numerical obligations; runtime moving fades are not enabled.


`RootRigidErrorAccumulator` now encloses the conditional integration inequality arithmetic itself. Exact stored start/end timestamps are subtracted outward, prefix translation radius is bounded from its rigid enclosure, and nominal linear speed uses an outward L1 upper bound. Products, divisions and accumulated origin/angular sums round outward. Supplied derivative-rate validity remains a caller obligation. Failed updates return no new owner; exact zero variation/error and zero angular uncertainty remain zero.

All 105 animation CPU tests pass. An independent 400-digit evaluator of three cumulative inequality updates stays below the emitted bounds. Evidence: `artifacts/rig-outward-error-accumulation-2026-10-04/`. The accumulator is not yet substituted into the full Animator blend builder: canonical prefix/clock alignment, proven derivative-rate inputs and collision arithmetic still need wiring and verification before runtime admission.


`RootRotationSpan::enclosed_angular_derivative_bounds` provides private whole-span angular speed/acceleration bounds for the normalized stored-field reference. Bernstein component hulls prove a positive quaternion norm lower bound; a hull that cannot exclude zero rejects without sampled fallback. Outward L1 bounds on first/second derivative controls give `speed <= 2*M1/m` and `acceleration <= 2*M2/m + 4*(M1/m)^2`. Constant normalized frames preserve these magnitude bounds. Held/arc acceleration is exactly zero; STEP and zero-duration events return None.

All 105 animation CPU tests pass with expanded independent analytic cubic speed/acceleration checks. The 400-digit Bernstein reference verifies seven whole-span emitted bounds. Evidence: `artifacts/rig-angular-rate-enclosure-2026-10-04/`. Compilation preceding stored coefficients and complete spatial linear rate bounds remain separate; the full builder/collision pipeline is not admitted yet.


`RootRigidSpan::enclosed_twist_bounds` now combines outward angular bounds with Bernstein position/first/second derivative hulls for additive and pivot polynomials. For canonical spatial `v=a_dot-omega cross a-R*p_dot`, its speed bound is `A1+omega*A+P1` and derivative bound is `A2+alpha*A+omega*(A1+P1)+P2`; all arithmetic rounds outward. Screw fields have exact zero derivative rates. STEP/zero-duration events return None and an unproved rotation norm rejects.

All 105 animation CPU tests pass with expanded independent moving-pivot derivative checks. A 400-digit reference verifies 56 whole-span speed/rate inequalities across all eight extraction masks. Evidence: `artifacts/rig-spatial-rate-enclosure-2026-10-04/`. These bounds describe canonical stored coefficients. Outward rate transport, blend-builder wiring, prefix/clock alignment and full collision arithmetic remain outstanding before runtime moving-fade admission.

### Outward bound transport (2026-10-04)

`RootSpatialTwistBounds::enclosed_retimed_between` encloses the exact stored
clip/wall duration ratio and its square, avoiding a rounded ratio as a proof
input. `enclosed_transformed` transports magnitude/rate caps through a constant
normalized real frame, including signed scale and the L1 cap of the enclosed
origin offset. `enclosed_blend` bounds both endpoint weighted caps and the
weight-derivative contribution using outward subtraction, division, addition
and multiplication. The caps remain conditional on the source fields already
being enclosed in the same frame and wall clock; these APIs do not certify
source compilation or automatically enable physical moving fades.

Validation: 106 animation tests pass, including reflected offset transport,
nontrivial playback ratios, increasing/decreasing/constant weights, a stopped
clip, invalid times/weights and overflow rejection. Evidence is saved in
`artifacts/rig-bound-transport-enclosure-2026-10-04/`. The existing fast APIs and
blend builder still use their earlier arithmetic contract; wiring the new caps
and outward error accumulator into that builder remains outstanding.

### Outward frozen-field integrator (2026-10-04)

`RootRigidPath::integrate_spatial_outward` connects sampled velocity enclosures,
`RootRigidErrorAccumulator` and outward canonical prefix composition. It stores
selected start/end clocks directly in the output spans rather than rebuilding
those clocks by summing rounded durations. The callback reference is the exact
stored start time, and the interval reference duration is the real difference
of the stored endpoints. Large angular increments trigger uniform refinement;
other callback/numeric failures propagate without returning a candidate path.

Its envelope compares the original field to the canonical real ordered screw
field. Floating pose sampling, collision arithmetic, original curve extraction
and composed approximation transport still require their own guarantees. The
older integration API keeps its documented arithmetic contract. The fade builder
has not yet switched to the new integrator.

All 108 animation tests pass. New regressions cover angular subdivision with
zero field error, exact output clocks, sampled bias and its nonzero error floor,
and independently analytic accelerating translation at intermediate fractions.
Evidence: `artifacts/rig-outward-integrator-2026-10-04/`.

### Clock intervals carried into velocity enclosures (2026-10-04)

`RootRigidSpan::spatial_twist_enclosure_range` now encloses a closed interval of
progress values, retaining De Casteljau, normalization and moving-pivot coupling
throughout the interval. The arc exponential accepts interval progress and
subdivides using its upper angular cap. `spatial_twist_enclosure_at_times`
converts checked in-span timestamps by outward subtraction/division. Intersection
with [0,1] relies on the validated timestamp domain; out-of-span times reject
rather than being silently clamped. Cubic norm positivity must still be proved;
a box that cannot exclude zero rejects and requires smaller intervals.

This supplies a necessary building block for certified clock conversion in the
fade builder; that builder still uses its old contract. 109 animation tests pass.
Independent 400-digit matrix references verify 840 cubic and 120 arc values at
interval endpoints and interior points over all eight extraction masks, shifted
coordinates and moving pivots. These checks support the implementation but do
not alone constitute a whole collision or upstream extraction certificate.
Evidence: `artifacts/rig-phase-range-enclosure-2026-10-04/`.

### Outward world point envelope (2026-10-04)

`RootRigidApproximation::enclosed_world_point_error_bound` converts caller-proven
origin/angular error metadata to a world point clearance. Euclidean point norm,
rotation chord, absolute signed scale and world evaluation radius are accumulated
outward. The chord uses an alternating sin(z)/z polynomial with a bounded ninth
term below stored pi, and the universal diameter bound 2 at/above it. This sine
series remains decreasing for z squared <= 2.5 (first ratio <= 2.5/6); it uses
no platform transcendental error assumptions. Exact zero error/scale cases avoid
inventing an artificial clearance.

The conditional gameplay whole-body sweep now uses this API for every mapped
collider corner. The computed stored corners are still a separate mapping proof
obligation, as are SAT/contact arithmetic and support-contact behavior. This
change does not enable runtime moving fades or certify upstream old error caps.
Independent 400-digit references cover 96 combinations of angular error, point,
signed scale and world radius, including tiny angles and the pi transition.
Evidence: `artifacts/rig-world-point-enclosure-2026-10-04/`.

### Enclosed inverse corner coordinates (2026-10-04)

`RootRigidEnclosure::inverse_similarity_point_sum_bounds` encloses inverse
similarity coordinates of an exact sum of stored vectors. Sum, enclosed origin,
normalized real inverse rotation and signed scale division all use outward
arithmetic. `RootRigidApproximation::enclosed_world_point_box_error_bound` accepts
these coordinate boxes and bounds their Euclidean radius before error transport.

The conditional whole-body clearance now feeds all eight signed sums of collider
edges through this mapping instead of treating rounded inverse corners as exact.
This closes the coordinate-sum/mapping obligation for the canonical stored-edge,
normalized real-frame model. Runtime floating pose evaluation, broadphase/SAT
roundoff and support contacts remain separate obligations; moving fades remain
disabled. Zero scale and nonfinite data reject before returning a clearance.

111 animation tests pass; 24 independent 400-digit matrix reference cases verify
skewed collider corners, shifted/rotated frames, positive/reflected scales and the
resulting world clearance. Evidence:
`artifacts/rig-inverse-point-enclosure-2026-10-04/`.

### Directed projection gaps for inflated poses (2026-10-04)

The conditional positive-clearance advancement path now obtains each projection
gap from `angular_sweep/gap.rs`. It encloses center subtraction, dot products,
absolute support sums and clearance dilation with directed adjacent-float
arithmetic. Because a generated stored axis need not have exact unit norm, a
positive projection gap is divided downward by an outward axis norm upper bound.
A nonpositive/inconclusive gap returns zero; it is not a penetration or true
contact certificate. Any arbitrary nonzero separating axis is sufficient for
this positive-gap certificate; completeness of the SAT axis set is unnecessary
for safety of that particular claim.

The legacy zero-clearance path retains its existing contract. Floating pose
sampling, broadphase rejection, advancement-time arithmetic, speed bounds and
support contacts remain obligations before certifying a complete swept result.
47 gameplay unit tests and 42 integration tests pass, including existing
inflated-wall/stationary-overlap tests. Nine independent 400-digit projection
references verify large-coordinate subtraction, nonunit/skew axes and different
clearance radii. Evidence: `artifacts/rig-projection-gap-enclosure-2026-10-04/`.

### Directed advancement and excursion-based rejection (2026-10-04)

The positive-clearance path now rounds multiplication, division and time addition
downward when advancing by a fraction of the proven gap/speed ratio. Underflow
may yield no progress (budget rejection); overflowing positive ratios clamp to
the unit endpoint only after downward rounding. Finite nonnegative gap, valid
unit time and finite positive speed are required.

Its broadphase now rejects an obstacle only when a directed initial world gap
exceeds the supplied whole-unit-interval point excursion bound. This preserves
near obstacles without depending on the old floating projection-extrema guard.
Completeness/performance may be conservative; a positive gap beyond the entire
point excursion is sufficient to prove this rejection. Source point speed bounds
and pose evaluation remain caller proof obligations: current legacy speed caps
are not yet fully certified outward. Legacy zero-clearance behavior is unchanged.

Validation: 48 gameplay unit and 42 integration tests pass. Six independent
400-digit advancement references cover ordinary ratios, endpoint completion,
underflow and overflow. Near/far obstacle regressions verify broadphase decisions
with zero advancement budget. Evidence:
`artifacts/rig-directed-advancement-2026-10-04/`.

### Canonical ordered screw point speeds (2026-10-04)

`RootRigidPath::enclosed_screw_point_speed_bounds` recomputes the canonical real
prefix once and bounds each mapped point-box velocity `omega cross position + v`
at the start of each constant field. Its norm stays constant because its time
derivative is `omega cross velocity`. Outward cross products, Euclidean norm,
exact stored endpoint duration and absolute world scale give a cap per normalized
span. Exact stationary fields preserve zero speed. Invalid boxes, unsupported
non-screw fields, exhausted span capacity and large unsubdivided angles reject.
The outward integrator already subdivides its angular increments appropriately.

Conditional `sweep_rigid_approximation` now passes these caps to the shared
advancement/broadphase kernel. Collider inverse-coordinate boxes share one owner
with clearance calculation. Exact/legacy paths keep their separate speed contract.
This covers canonical stored screw speed arithmetic; runtime pose evaluation and
support-contact admission still need completion. No moving fade is enabled.

112 animation tests pass. 27 independent 400-digit ordered matrix/exponential
references verify point speed after noncommuting prefixes, signed scales and a
point 1000 units from the origin. Evidence:
`artifacts/rig-screw-speed-enclosure-2026-10-04/`.

### Canonical enclosed corner geometry in gap queries (2026-10-04)

`RootRigidEnclosure::{transform_point_box_bounds,similarity_point_box_bounds}`
propagate coordinate boxes through rigid/signed similarity frames. Conditional
approximation sweeps now recompute the canonical ordered screw field at each
sample, transform all eight inverse corner boxes through that field and the
composed normalized real actor/source frame, and pass the resulting world boxes
to a directed corner-projection gap calculation. The rounded pose selects arbitrary
candidate axes only; a positive gap is established from enclosed geometry.
The same corner certificate feeds the whole-excursion broadphase rejection.
Empty/stationary conditional paths also check enclosed initial geometry, including
zero-error metadata. Legacy queries retain their documented contract.

This avoids requiring the rounded sampled pose to be exact for a gap certificate.
It does not prove that a subsequently published floating pose has no evaluation
error: the supplied world evaluation radius still covers that publication/model
obligation. Error metadata, upstream curve extraction and support contacts remain
separate proof obligations. Prefix recomputation currently costs linear work in
span index per sample; caching canonical prefixes is a required performance step
before enabling long production fades. Moving fades remain disabled.

Validation: 113 animation, 49 gameplay unit and 42 integration tests pass.
Independent 400-digit matrix/projection references cover 32 transformed coordinate
box corners and three directed world-corner gaps, including nonunit/skew axes.
Evidence: `artifacts/rig-enclosed-pose-gap-2026-10-04/`.

### Path-borrowed canonical prefix cache (2026-10-04)

`RootRigidPath::prepare_screw_enclosures` returns `RootScrewEnclosurePath`, an
immutable path-borrowed owner with one prepared enclosure per prefix, including
the final endpoint. Preparation uses one ordered pass and rejects unsupported
fields/angles, invalid intervals, overflow or exhausted capacity before publishing
an owner. Each interior sample evaluates one local exponential and composes one
cached prefix; endpoint samples return prepared boxes directly. The borrow binds
cache lifetime to the actual immutable path; the gameplay kernel additionally
checks source path pointer identity. Speed caps and corner-gap queries share this
prepared owner, eliminating replay of prior spans on every query.

114 animation tests pass. Twelve independent 400-digit canonical timestamp/field
references confirm cached translation, quaternion and point boxes. A paired debug
CPU measurement of 64 partial queries, including cache preparation, took median
41.01 ms by replay and 2.62 ms with preparation/cache (median per-round ratio
15.71x). This is a local microbenchmark, not a claim about whole game performance.
Evidence: `artifacts/rig-prefix-cache-2026-10-04/`. Pose publication error and
support-contact admission remain outstanding; moving fades remain disabled.

### Separate admission of the rounded proposed pose (2026-10-04)

Conditional approximation sweeps now validate the returned proposal independently
of the canonical trajectory: proposed center is the actual stored `center + hit
displacement`; orientation/edges mirror the physical controller's stored quaternion
composition/normalization and edge update. Directed projection gaps prove separation from every obstacle; the exact dyadic
fallback below can additionally prove projection touching. Each obstacle consumes the shared query
budget, including checks for previously discarded broadphase obstacles, and the
receipt includes those queries. Overlap, unproved touching, invalid geometry
or exhausted work reject without returning an accepted proposal.

This establishes a certificate for that exact rounded proposed center/edge model.
The check occurs before the runtime's later grounding/relocation operation; it
is not proof of the entire published scene/GPU transform. Ground-support touching
needs explicit directional/support admission rather than silently ignoring a floor.
The conditional API remains a read-only query, and moving fades remain disabled.

50 gameplay unit and 42 integration tests pass. New regressions distinguish a
separated proposal, a changed overlapping proposal and a touching proposal, and
check invalid data and budget exhaustion. Existing conditional inflated sweeps
also exercise the connected gate. Evidence:
`artifacts/rig-proposed-pose-admission-2026-10-04/`.


### Exact endpoint touching predicate (2026-10-04)

`angular_sweep/exact_gap.rs` computes the exact sign of the stored binary64
projection gap. Each finite product is decoded into a signed dyadic integer in
units 2^-2148. Products occupy at most 4196 bits; the 24 terms of one projection
gap require at most 4201 bits. A fixed 66-limb (4224-bit) accumulator therefore
supports every finite input exponent, including subnormals, without heap
allocation or rounded intermediate products. Center subtraction occurs through
exact dot subtraction, never through a possibly overflowing floating difference.

Proposed-pose admission uses directed positive separation as its fast path and
exact nonnegative gap as fallback, including interval-arithmetic overflow. Exact
zero proves disjoint interiors on that projection and permits touching. Any
nonzero stored direction can prove this condition; failure to find a certificate
still rejects. This is endpoint nonpenetration, not proof of a supported continuous
trajectory, floor snapping or later scene/GPU publication. Isotropic error
clearance still blocks supported moving paths until directional admission is
implemented. Moving fades remain disabled.

51 gameplay unit and 42 integration tests pass. Independent arbitrary-precision
rational references verify 21 exact sign cases across the full binary64 exponent
range and touching versus adjacent-float penetration. Connected pose tests admit
an exactly touching proposal and reject its next-float penetrating neighbor.
Evidence: `artifacts/rig-exact-contact-sign-2026-10-04/`.

### Whole-path coordinate-plane support certificate (2026-10-04)

`RootScrewEnclosurePath::coordinate_velocity_range` proves a structural projection
property over every stored field: angular components orthogonal to the selected
coordinate must be exactly zero. Then that coordinate of every moving point has
derivative equal to the stored coordinate of linear spatial velocity. The returned
whole-path velocity extrema support exact sign comparisons without rounded cross
products or sampled invariants. Tiny nonzero orthogonal rotation rejects the proof.

Conditional approximation broadphase can now retain an exact initial coordinate
plane contact when both normalized real actor/source rotations fix that coordinate,
the signed-scale velocity is everywhere tangent or directed away from the obstacle,
and an exact initial SAT sign proves nonpenetration. The proof is for all times,
not an endpoint guess; nearby walls still require ordinary certified sweeping.
This certificate is used only with zero total clearance. Nonzero isotropic error
cannot be treated as zero directional error; varying fades require separately
proved projection error before this can admit their floor contact. General tilted
support planes, grounding/relocation and runtime moving fade publication remain
outstanding. Moving fades stay disabled.

Evidence is saved in `artifacts/rig-coordinate-support-admission-2026-10-04/`.

The connected floor test exposed a real rounded-pose defect: the expanded
quaternion/vector norm-coefficient formula can change the fixed coordinate of a
pure coordinate-axis rotation. That turns exact support touching into a tiny
penetration. `convex::rotate_vector` now owns the cross-product unit-rotation form
for physical controller rest-edge recovery/updates and sweep point/frame vectors.
It preserves the fixed coordinate bit for bit when quaternion orthogonal vector
components are exactly zero. This corrects the physical calculation rather than
weakening the exact endpoint predicate. Sweep samples use the same normalized
combined orientation as the controller's proposed edge update.

Validation: 115 animation, 53 gameplay unit and 42 integration tests pass.
Connected conditional sweeps cover tangent/away/into-plane motion, positive and
reflected scales, actor/source yaw frames and an offset origin, plus a near wall
that the support certificate must not discard. The shared physical rotation test
checks bitwise fixed-coordinate preservation on all three coordinate axes. The
full integration suite retains tilted/sheared collider, jump, winding, rollback
and query-budget behavior. These are CPU checks; native editor proof is separate.

### Signed coordinate frame mappings preserve support (2026-10-04)

`convex::rotation_coordinate_preimage` recognizes an exact signed coordinate row
of the normalized real quaternion rotation from linear stored-component identities.
Diagonal rows follow zero orthogonal quaternion components (fixed direction), or
zero scalar/selected-vector components (reversed direction). Off-diagonal rows
use `q_i = sign*q_j` and `q_k = -epsilon_ijk*sign*q_w`; the corresponding rational
matrix row is exactly a signed coordinate selector. No rounded matrix or near-zero
tolerance enters the certificate. `rotate_vector` applies these known rows exactly
and uses the unit cross form for other coordinates.

Support proof now transports its world coordinate through actor and source rows,
including their signs and reflected scale, instead of requiring both frames to
fix the same coordinate. `reframe_rotation` conjugates a motion quaternion by
rotating its imaginary vector and retaining its scalar, avoiding needless rounded
quaternion products that destroy these structural identities. Controller and sweep
rotation ownership remains shared.

55 gameplay unit and 42 integration tests pass. Nine independent exact rational
matrix rows validate coordinate selectors, including quarter turns, signed cycle
permutations and partial rows with arbitrary spin. A one-ulp component perturbation
rejects the claimed row. Connected sweeps preserve floor support through source
or actor quarter turns and reflected scale. This still requires a world coordinate
plane and zero total clearance; directional error bounds, general tilted planes
and full moving fade publication remain outstanding. Evidence:
`artifacts/rig-permuted-support-admission-2026-10-04/`.

### Prepared support direction ranges (2026-10-04)

Canonical screw preparation now accumulates all three structural coordinate
velocity ranges in the same pass as prefix enclosures. `coordinate_velocity_range`
is a constant-work lookup instead of replaying the whole path for every support
axis/obstacle/span. The source borrow still binds this metadata to the immutable
path. Orthogonal angular motion invalidates a coordinate permanently, late linear
velocity reversal remains in its min/max, and empty paths return exact zero.

116 animation tests pass, including late reversal after 64 earlier fields and
empty-path direction queries. This removes the repeated path traversal from the
support certificate; no whole-game speedup is asserted. Evidence:
`artifacts/rig-cached-support-ranges-2026-10-04/`. Nonzero directional approximation
error and arbitrary tilted support planes remain outstanding.

### Exact zero identities in velocity enclosures

Interval arithmetic now preserves exact algebraic zero for multiplication,
positive division, squaring, addition of zero, and subtraction of identical
finite singleton values. Non-singleton self-subtraction retains its uncertainty;
zero multiplication does not conceal infinite or NaN operands. These rules
avoid introducing artificial subnormal off-axis angular velocity or vertical
linear velocity in planar fields.

`RootRigidTwistEnclosure::coordinate_velocity_range` reports a coordinate
velocity interval only when both orthogonal angular intervals are exactly zero.
Its certificate covers the enclosure's represented domain: a sample certificate
cannot establish invariance between samples. Retimed and blended planar fields
retain exact zero normal velocity. This is groundwork for directional error
bounds; it does not enable runtime moving fades or replace editor acceptance.

Whole-span planar verification exposed dependency loss in interval De Casteljau
interpolation: independently evaluated `u` and `1-u` allowed an artificial zero
quaternion norm despite positive scalar control points. Each interpolation now
intersects the arithmetic enclosure with the convex hull of its two controls
when the complete weight interval lies in [0,1]. Both independently contain the
real interpolation, so their intersection remains conservative. Extrapolation
retains the arithmetic enclosure. This proves planar coordinate constraints on
whole cubic spans rather than just at samples, without weakening norm rejection
for genuinely unproved curves.

### Directional displacement error over stored-time intervals

`RootRigidSpan::enclosed_coordinate_displacement_error_between` obtains the
source velocity enclosure for the entire checked time interval and compares it
with a finite frozen spatial field. Both angular fields must be exactly parallel
to the selected coordinate axis. For every material point, the corresponding
component of `omega cross x` then vanishes, so the coordinate discrepancy at any
prefix is bounded by the maximum enclosed linear velocity discrepancy times an
outward duration. Initial coordinate error remains separate.

This certificate allows different yaw speeds and arbitrarily different horizontal
velocities while proving exact zero height error. Nonzero normal velocity bias
produces a positive bound; even a subnormal-scale orthogonal angular component
rejects the directional certificate. The API checks the full clock interval
against the source span and cannot accept an unrelated point-sample enclosure.
It does not yet transport this certificate through fade frame mapping or attach
it to gameplay collision queries. Floating pose publication remains a separate
admission check.

### Whole-progress velocity blend enclosures

`RootRigidTwistEnclosure::blended_over_progress` encloses all linear-weight
blends over a closed progress interval. It uses the shared convex interpolation
owner for both the weight and the field components; the point-progress API now
calls the same implementation with a singleton interval. Increasing, decreasing
and constant weights share the contract. Complete source and target time-domain
enclosures must already use the same frame and playback clock; upstream clock
conversion uncertainty must be included in the supplied progress interval.

Exact planar normal velocity survives the whole blend interval, whereas a
nonzero orthogonal angular component prevents the coordinate certificate.
This operation supplies the whole-field input needed for directional fade
errors. It does not on its own couple progress to clip clocks, prove common
frame ownership, handle STEP events, or enable moving fades in the runtime.

### Retiming complete clip domains from stored clocks

`RootRigidTwistEnclosure::retimed_between_times` computes both durations from
stored finite endpoints with outward subtraction, then divides outward.
`RootRigidSpan::retimed_spatial_twist_enclosure_between` couples that operation
with the complete checked source interval. Source and target spans can now be
mapped independently onto the same wall interval before whole-progress blending.

Equal clip endpoints represent a paused clip and retain exact zero velocity.
The wall interval must be strictly increasing; decreasing clip intervals,
nonfinite clocks, out-of-span clip times, and an unprovably positive numerical
wall-duration enclosure reject. This includes tiny adjacent-float wall intervals
rather than silently treating an uncertain denominator as positive. Long elapsed
clocks are tested with adjacent stored endpoints. The contract uses exact stored
endpoint differences; upstream timestamp acquisition error remains separate.
Frame ownership, progress-clock coupling and runtime admission remain outstanding.

### Fade progress from the stored wall clock

`RootRigidTwistEnclosure::blended_between_times` derives a whole progress
interval by outward subtraction and division of stored fade/query clocks.
The checked query must remain inside a strictly positive continuous fade;
completion and target tails must be split by the caller. Increasing and
reversing weights use the shared whole-progress blend owner. Clamping the
outward progress enclosure to [0,1] relies on that checked domain, not a
numerical tolerance. Complete source and target field coverage in the same
frame/wall clock remains a precondition. Clock acquisition error, event
partitioning and runtime collision admission remain separate.

The shared interpolation owner also intersects its result with the outward
`a+(b-a)*u` expression, which retains the dependence of weight and complement
for fixed controls. If this optional expression overflows, the original valid
enclosure remains. An independent exact-rational oracle checks 16,000 corner
values against the intersection of both expressions and the convex hull.

### Owned whole-domain fade field snapshots

`RootRigidFieldInterval::from_span` snapshots the complete checked clip domain,
retimes it to stored wall endpoints, and maps it through an enclosed normalized
frame and signed scale. `frozen` represents the zero-velocity interruption source.
These snapshots have private field/clock state; callers cannot construct them
from unrelated point samples. `RootRigidFadeFieldInterval` requires identical
source/target wall endpoints and derives the blend from the stored fade clock.
Both supplied mappings must target the caller-selected common coordinate frame.

The fade owner provides a uniform coordinate displacement error bound against a
frozen field for every prefix and every material point, sharing the existing
single-span error arithmetic owner. Shifted yaw frames, reflected scales and a
frozen interruption source preserve zero normal displacement error; a tilted
mapping invalidates the coordinate certificate. Initial discrepancies, proof
transport to arbitrary planes, whole-path accumulation and gameplay admission
remain separate. These values are immutable snapshots of stored source data;
subsequent source edits require rebuilding the fade snapshot.

### Path-bound directional fade certificate

`RootRigidPath::enclose_fade_coordinate_error` requires one complete fade field
interval per stored screw span, with exactly matching endpoints and order. It
compares each whole fade domain against that span's own stored twist, rather
than a separately supplied nominal velocity, then accumulates coordinate error
outward. Because both fields remain parallel to the selected axis, initial
coordinate discrepancy propagates unchanged and local displacement discrepancy
bounds add for every prefix and every material point.

The resulting `RootRigidCoordinateCertificate` privately borrows the exact path,
records its axis and uniform error bound, and cannot survive mutation of that
path. Missing/reordered domains and exhausted span budgets reject; a late
orthogonal angular component or unsupported non-screw span returns no complete
certificate. Exact zero directional error remains zero across all spans.
Initial coordinate error, floating pose evaluation and collider frame transport
are still separate. Gameplay has not yet consumed this certificate.

### Directional certificate consumed by the conditional collision query

The conditional approximate-motion sweep now accepts an optional path-bound
coordinate certificate. It verifies exact path identity before any query and
uses a zero-error certificate only for the matching coordinate after exact
actor/source coordinate-row transport. Thus a conservative nonzero isotropic
approximation margin need not block touching a floor when the complete fade
field has provably identical normal displacement to the nominal trajectory.
The existing whole-path monotone-away condition and exact initial SAT contact
predicate remain required. Other obstacle axes retain the isotropic margin.

A positive scalar floating-evaluation radius disables this directional bypass,
since it supplies no exact zero normal-error proof. The proposed rounded pose
still passes the separate exact-contact admission gate. The read-only query
remains disconnected from runtime moving-fade publication; the caller must use
certified fade domains for the actual original field. General positive
coordinate-error margins and tilted support planes are not yet handled.

### Positive directional margins at support planes

The directional collision path now also handles nonzero coordinate error when
there is a proven initial gap. The certificate computes an outward world margin
`abs(scale)*coordinate_error + evaluation_radius`; exact coordinate-row frame
transport supplies only a sign and does not enlarge the magnitude. A monotone
nominal field may discard a support candidate only when its initial directed
projection gap exceeds this margin. Zero margin retains the exact touching
predicate. Other axes continue to use the complete isotropic approximation
margin, and the rounded proposed pose still passes its separate admission gate.

This supersedes the earlier blanket disabling of directional support for a
positive numerical evaluation radius: that radius now participates in the
required gap and cannot permit exact touching. Reflected scales and insufficient
support gaps are checked. Runtime publication and general tilted support remain
outstanding.

### Derivative caps use the same stored-clock mapping

Audit of the staged fade builder found that using rounded duration subtraction
for derivative retiming would break the endpoint-clock reference used by
velocity enclosures. `RootSpatialTwistBounds::enclosed_retimed_between_times`
now shares the outward endpoint-difference ratio owner with velocity retiming.
Speed caps scale by the enclosed ratio; derivative caps scale by its outward
square. Existing exact-duration retiming shares the same bound scaling owner.
Paused clip clocks retain exact zero caps; invalid or unprovably positive wall
intervals reject. The legacy `blend_spatial` builder still uses its earlier
real-arithmetic contract and is not silently promoted to a certified builder.

### Integrated certified continuous fade interval

`RootRigidMappedField` borrows an authoritative continuous source span and
records its clip endpoints, enclosed common-frame mapping and signed scale.
`RootRigidCertifiedFadeInterval::integrate` derives whole-field derivative caps
from those same sources, retimes/transforms them outward, and integrates with
`integrate_spatial_outward`. Point samples and whole output-span certificates
share one field evaluator. That evaluator maps stored wall time to an outward
clip-time interval, preserving mapping uncertainty and the original global
clip/wall rate rather than recomputing it from rounded local durations.

The nominal stored twist is the finite componentwise midpoint of its enclosure;
its discrepancy is included by the existing outward integrator. The immutable
result owns both approximation and complete fade domains, and constructs its
path-bound coordinate certificate on demand. A frozen interruption source is
supported. This builds one key-free continuous interval only; multi-key and STEP
partitioning, composed tail error transport, Animator publication and native
acceptance remain outstanding.

Canonical yaw verification sums stored angular speed times exact stored endpoint
differences using independent rational arithmetic and checks the analytic half
radian phase against the emitted angular bound over 64 spans. A rounded pose's
`angle_between` is not substituted for this canonical comparison: floating pose
evaluation uncertainty remains separate from the discretization certificate.

### Assembled fade passed directly to collision acceptance

`sweep_certified_rigid_fade` consumes the immutable assembled interval and builds
its coordinate certificate internally from that object's source domains. The
caller no longer supplies unrelated approximation metadata and a separate fade
certificate. The existing enclosed sweep, directional support margin, shared
query/iteration budgets and rounded proposed-pose admission gate are reused.

The verification path assembles increasing-weight translation plus yaw from the
original continuous source, then checks floor contact and a wall in the same
query. This is still a read-only staged acceptance query; it does not publish
Animator clocks, grounding/relocation updates or editor state. Multi-key/tail
assembly and native acceptance remain outstanding.

### One global integrator across stored key-domain cuts

`integrate_spatial_outward_partitioned` accepts strictly ordered stored domain
endpoints and separate whole-domain derivative caps. It keeps one global nominal
prefix, canonical prefix enclosure and outward error accumulator across all
cuts. The original single-interval API delegates to this owner. Every output
span stays inside its selected domain, and the final subdivision uses that
domain's exact stored endpoint. No rounded local-duration summation or post-hoc
path append transports the error bound.

The callback receives both active domain index and stored wall time, so a
velocity discontinuity at a cut selects the next domain explicitly. Instantaneous
pose STEP events still require a separate policy. Subdivision uses one bounded
global span budget and publishes only a complete candidate. A translation then
rotation test verifies preserved order, stored cut identity and budget rejection
before sampling. Mapping actual clip keys onto these wall cuts and assembling
the complete Animator fade/tail remain outstanding.

### Certified fade assembly across explicit clip-pair domains

`RootRigidFadeDomain` pairs checked source/target span mappings with a stored
wall endpoint. `RootRigidCertifiedFadeInterval::integrate_partitioned` retimes
each mapping from its own exact endpoint differences, uses global fade weights
and weight derivative bounds, and passes all domains to the single partitioned
outward integrator. The one-domain API delegates to this owner. Whole-span
coordinate certificates are assembled from the same global-time field evaluator
and remain attached to the complete owned path.

The global weight does not restart at a key. A frozen-source example changes
target speed from 1 to 3 at wall time 0.25; its analytic fade displacement is
1.4375. Restarting the weight locally would produce a different trajectory and
is rejected by the error-bound comparison. Stored key cuts are retained during
refinement, and all domains retain zero normal discrepancy. This API requires
explicit checked clip-to-wall domain mappings; automatic extraction from both
Animator key streams, pose STEP policy, completion/tail assembly and publication
remain outstanding.

A paired-key verification uses a source key at 0.5 and a target key at 0.25,
with explicit common cuts [0.25, 0.5, 1]. Both fields retain the global weight
`t`; the analytic blended displacement is (2.1875, 0, 0.5). All stored cuts and
zero normal discrepancy survive adaptive integration. Automatic discovery and
certified temporal mapping of arbitrary imported key streams remain separate.

### Outward wall-clock key partition with explicit uncertainty domains

`RootRigidPath::partition_wall_outward` computes interval bounds for each exact
stored `key_time / clip_duration * wall_duration`. Outer endpoints preserve the
algebraic identities zero and the complete wall duration. Every non-singleton
key-time enclosure contributes a marked uncertainty domain. Outside their union,
source/target span indices are selected only when the entire stored wall interval
is proved between the enclosed key boundaries. Marked domains supply no invented
single-span derivative cap and require a future whole-field discrepancy bound.

The partition borrows both exact input paths, preventing stale indices after
mutation. Overlapping guards merge; sorted guards and source spans are traversed
with monotone cursors. A nonrepresentable 1/3 key verifies guarded timing, proved
outside indices and contiguous wall coverage. Pose STEP events reject explicitly.
This prepares automatic certified key mapping; consuming the marked domains in
fade integration remains outstanding, so the older rounded normalized partition
is not promoted to a numerical certificate.

### Complete field hull over a key-uncertainty clip domain

`RootRigidPath::spatial_twist_enclosure_between` bounds all intersecting continuous
span limits on a checked clip-time interval. A key boundary contributes both
neighbors, stationary gaps contribute zero, and any intersecting instantaneous
pose STEP rejects. Componentwise `RootRigidTwistEnclosure::hull` preserves exact
planar constraints only if every included field satisfies them; even a tiny
orthogonal angular component in a neighboring span invalidates the certificate.
No finite derivative cap across the hull's key boundaries is implied.

A binary search locates the first intersecting ordered span, followed by a scan
of only the required spans and gap boundaries. This supplies the velocity
range needed for marked uncertain wall-key domains. Uniform whole-field
integration of that range, automatic complete fade assembly and Animator/native
publication remain outstanding.

### Whole-field integration domains without fictitious derivative bounds

`RootRigidIntegrationDomain` distinguishes derivative-bounded smooth domains
from `WholeField` domains. Smooth callbacks enclose the stored start instant;
whole-field callbacks receive the complete stored [start,end] interval and must
bound every velocity there. `append_bounded_field_interval` has no derivative
precondition: the skew term in the discrepancy dynamics preserves norm, giving
`ev*h + ew*(prefix_radius*h + nominal_linear_speed*h*h/2)` and angular error
`ew*h`. Both modes share the private outward error arithmetic owner.

The global integrator retains whole-field domains as one span while refining
smooth domains under the remaining shared budget. This avoids subdividing an
adjacent-float key guard into nonrepresentable timestamps. A mixed ramp/guard/ramp
case verifies successful refinement, one exact adjacent-clock guard span and
bounded displacement without assigning zero original acceleration at a key.
The original partitioned and single-domain APIs delegate to this owner.
Automatic assembly from original path key guards remains outstanding.

### Certified staged-plan fade portion (2026-10-04)

`RootRigidFadePlan::integrate_certified_fade` now routes the staged source and
 target paths directly through the automatic original-clock assembler. Explicit
 source/target frames retain their outward normalization enclosure; mismatched
 frozen/live source frames are rejected. The returned interval covers exactly
 `fade_wall_seconds`, with the original global weights and discovered key guards.
 It does not publish the candidate animator. Completion tails still require an
 enclosed original-target endpoint frame and a single whole-tick accumulator;
 the existing `integrate_spatial` whole-tick API remains the legacy arithmetic
 reference rather than a certificate for runtime publication.

### Continuous endpoint pose enclosure (2026-10-04)

`RootRigidSpan::continuous_pose_enclosure` evaluates the stored continuous
reference directly with outward arithmetic. Normalized cubic and arc rotations
share the existing rotation enclosure owner; translation retains both additive
controls and the rotated moving pivot. Screw spans enclose their exact stored
time difference and their stored initial frame. This span API does not replace
canonical ordered screw-prefix preparation: cached floating initial frames have
a different reference. STEP returns unsupported; unproved normalization or
exponential range fails instead of using a sampled rounded pose. This supplies
the pose arithmetic needed for an enclosed target continuation frame, which is
still not assembled into a whole-tick runtime fade.

### Enclosed original target continuation mapping (2026-10-04)

`continuous_end_enclosure` uses canonical prefixes for an ordered screw path and
its final absolute stored span for a compiled polynomial path. Empty paths are
identity; mixed reference kinds and STEP events reject. A cache-corruption test
confirms that screw endpoint enclosures do not depend on cached floating poses.
`RootRigidMappedPath::from_enclosed_frame` retains this frame enclosure, and
`RootRigidFadePlan::certified_target_tail_mapping` composes it with the original
common target frame. A completion-tail integration test exercises that mapping.
Fade and tail still need one global integration/error accumulator; independently
certified intervals cannot be concatenated with the legacy zero-error append and
called a certified whole tick. Runtime publication remains disabled.

### One-accumulator explicit fade completion (2026-10-04)

`integrate_partitioned_with_completion` now integrates explicitly mapped fade
and target-tail domains through the same outward partitioned integrator. Fade
weights use the fade clock, completion uses target-only weight one, and both
share the canonical prefix and accumulated origin/angular error. A completion
requires an explicit cut at fade end and final fade weight one; live completion
sources and missing cuts reject. The regression covers a fade cut at 0.5,
completion at 1, tail through 3, analytic translation 7, retained cuts, zero
height error, and global budget rejection. Original-clock automatic key guards
and staged-plan tail mappings still need to be routed through this whole-tick
owner before enabling runtime publication.

### Automatic whole-tick completion assembly (2026-10-04)

`integrate_paths_with_completion` retains automatic outward source/target fade
key domains and adds target-only tail domains to the same integrator invocation.
The tail's original clip clock is mapped from the stored whole-tick endpoints
using enclosed subtraction and division, never a rounded local duration. Full
field queries enclose every intersected original tail key. Tail subdivisions
increase on budget failure, sharing the fixed global span/error budget with the
fade; no prefix or error is appended or reset. Stored subdivision cuts define
integration domains only, so their rounding does not reparameterize the clip.
`RootRigidFadePlan::integrate_certified_tick` supplies the enclosed original-target
continuation mapping. Tests cover staged completion within a tick and an
original tail key at one third of its clip, with total analytic translation 16,
zero height discrepancy and insufficient-capacity rejection. All 144 animation
tests passed in 6.98 seconds. Whole-field tail refinement is conservative and
currently more expensive than outward shifted key guards; production performance
and physical/runtime publication remain unverified. Zero-time fade handling
still needs an explicit runtime policy.

### Whole-tick fade collision regression (2026-10-04)

The gameplay collision consumer now has an end-to-end regression using automatic
whole-tick assembly, an enclosed original-target endpoint frame, translating yaw
and a target tail with an internal original key. The floor-only query accepts the
complete tick with exactly zero height displacement. Adding a wall stops the
trajectory after the fade boundary and before the wall; a positive evaluation
radius still rejects exact touching floor contact rather than dropping that
radius. All 60 gameplay unit tests and 42 integration tests passed. This proves
the read-only collision path for this fixture, not animator clock publication,
post-grounding/relocation acceptance, or native editor runtime fades. Those
runtime boundaries remain to be wired and verified.

### Accepted wall-time pose staging (2026-10-04)

A fade plan retains a private pre-tick animator snapshot and original stored wall
endpoint. `prepare_accepted_frame` rebuilds a candidate animator and displayed
frame for a finite accepted wall time inside that interval, without publishing
anything. The internal advancement owner now accepts f64 wall time; public f32
advancement delegates to it unchanged, avoiding an upward f32 conversion of a
physics-accepted prefix. Whole-tick assembly uses the saved original wall endpoint.
Transition completion now checks elapsed time against duration, not rounded f32
pose weight. A regression at the preceding f64 value below fade end produces
weight 1 but retains the unfinished transition, then completes exactly at end.
Zero acceptance and invalid-prefix rejection preserve the original and full-tick
staged candidates. All 145 animation tests passed. The consumer still must bind
the accepted time to its physical path, validate current asset identity and final
post-grounding/relocation actor pose, and atomically publish scene and animator;
this helper alone does not enable runtime moving fades.

### Final prepared rigid-pose publication gate (2026-10-04)

Character ticks with rigid trajectories and a preparation callback now certify
both the final physical box and the box represented by the proposed scene world
matrix. This runs after grounding, relocation and f32 narrowing, before callback
execution or any scene/body/input publication, using the shared collision query
budget. The exact stored-projection owner permits touching but rejects interior
intersection. A wall-contact narrowing regression rejects publication, leaves the
callback uncalled, retains the scene, absent runtime state and pending jump; a
subsequent safe trajectory prepares and publishes the exact preview matrix.
All 60 gameplay unit and 43 integration tests passed. This gate currently applies
to prepared rigid trajectories; certified fade requests and accepted-clock
receipts still need to be introduced into that transactional entry point. A
rejected rounded contact currently rolls back the tick rather than finding an
alternate representable contact pose; production contact continuation remains
incomplete.

### Certified fade character transaction (2026-10-04)

`CharacterCertifiedFadeMotion` and `fixed_step_with_certified_fade_preparation`
route owned fade fields through the existing character transaction rather than a
second solver. Admission requires the exact path identity, matching stored tick
duration, valid coordinate axis and nonnegative finite world evaluation radius.
Existing rigid-frame, owner, writer and aggregate budgets still apply. The sweep
constructs the directional certificate from the owned field snapshots; final
physical/displayed pose checks run before preparation. `CharacterTickPreview`
now carries accepted trajectory receipts so an animator owner can stage its
accepted wall prefix in the callback. Preparation dispatch is shared with the
existing rigid preparation entry point. The new grounded-yaw regression rejects
a preparation error with scene/body/input intact, then publishes the successful
preview and rejects duration mismatch atomically. All 60 gameplay unit and 44
integration tests passed. The model editor still uses the legacy moving-fade
rejection path; actual animator/asset identity binding and accepted-clock
publication remain to be connected. Supplied evaluation-radius validity remains
a caller proof obligation, not inferred from these fixture tests.

### Model-bound accepted fade staging (2026-10-04)

`PreparedModelFade` retains the exact imported model Arc and immutable original
fade plan. Accepted-pose preparation requires that model identity and the original
animator snapshot still match. `RootRigidFadePlan::matches_animator` checks current
clip/root-curve identity, motion selection, clock, speed, source clip/curve identity,
source clock, transition elapsed/duration and frozen-source pose identity. The
candidate is rebuilt from the retained snapshot only after those checks; this
helper does not mutate or publish playback. Contact intervals, retargeting and
editor character-loop publication still require integration with the certified
physical receipts; the existing moving-fade runtime rejection remains enabled.

Editor verification: compilation passed after adding empty accepted-motion
receipts to the two manually constructed foot-placement preview fixtures.
Playback (9), animation-runtime (8) and foot-placement (17) targeted tests passed.
The new asset-bound staging regression checks a successful accepted prefix,
invalid prefix, changed speed and a separately imported equivalent model Arc.
Native editor acceptance and end-to-end moving-fade publication remain unproven.

### Receipt-bound model fade preparation (2026-10-04)

`PreparedModelFadeMotion` owns the model-bound plan and its certified whole-tick
interval. Its borrowed physics request references that exact trajectory. Rigid
receipts carry a private transient trajectory identity (address comparison only,
never dereferenced or persisted); rotation-only receipts cannot match it. Accepted
model preparation requires both the owner and this live trajectory identity, then
uses `accepted_wall_time` to convert the completed-span/fraction receipt with
outward interpolation. Partial acceptance uses the lower wall-time endpoint;
complete acceptance requires all spans and fraction one. The prepared snapshot
and model gates remain in force. These bridge the physical preparation callback
to a candidate animator; ordinary model runtime dispatch, precise contact
intervals and foot/retarget publication are still not switched over.

Verification: 10 playback, 8 animation-runtime and 17 foot-placement tests passed,
including the physical callback bridge, wrong live trajectory, preparation rollback
and stale snapshot replay rejection. The animation suite passed all 145 tests,
with endpoint/partial wall-prefix and malformed completion checks. Partial time
is conservatively bounded; precise correspondence of contact intervals to that
accepted wall prefix and ordinary editor dispatch are still outstanding.

### Accepted-prefix contact candidate (2026-10-04)

`phase_interval_wall`, `source_phase_interval_wall` and `frozen_source_tick_wall`
share the existing phase owners while accepting stored f64 wall time. Legacy f32
APIs delegate without changing their contract. `accepted_playback` now stages a
complete ModelPlayback candidate: accepted animator plus target contact interval,
source interval/active fraction, or frozen-source snapshot/active fraction, all
computed from the same pre-tick state and accepted wall prefix. Nothing mutates
the live owner in preparation. The physical callback regression checks that target
contact end matches the accepted pose phase, live source metadata is present and
the original owner retains no contact interval until candidate publication.
Ordinary editor runtime dispatch and root extraction/retarget/foot correction of
this candidate remain to be connected before moving fades can be enabled.

Verification passed: 145 animation tests and 10 playback / 8 runtime / 17 foot
placement editor checks. The adjacent-f64-prefix regression confirms that target
and source contact travel preserve the accepted time instead of rounding up to
the f32 fade completion boundary. These are staged candidate proofs, not native
editor moving-fade acceptance.

### Accepted displayed-frame owner (2026-10-04)

Ordinary runtime preparation and accepted fade staging now share
`prepare_displayed_frame`: retargeting, selected-axis in-place translation and
root rotation removal occur in one owner. `AnimationRuntime::accept_fade` rebuilds
an immutable runtime candidate from the receipt-bound playback, validates the
target model Arc, and replaces only the staged owner's playback/frame. The
physical preparation regression routes it through the existing foot-correction
callback, confirms the original accepted frame remains unchanged, and checks that
movement is applied to the body while the displayed root retains bind translation
and zero extracted translation delta. This fixture has no active foot settings;
it does not prove moving-fade planted-foot behavior. Existing retarget and foot
checks passed: 10 playback, 9 animation-runtime and 17 foot tests. The initial
zero-step test fixture was corrected to a valid fixed tick after runtime admission
rejected it. Ordinary App dispatch still rejects moving fades. Original f64 fixed
wall time, canonical common-frame selection and bounded floating evaluation error
must be carried through that dispatch before enabling it broadly.

### Original fixed-step wall staging (2026-10-04)

`prepare_root_rigid_fade_wall` and `prepare_certified_fade_wall` retain the
scheduler's stored f64 wall endpoint through path staging, candidate advancement
and owned whole-tick integration. The existing f32 methods delegate as compatibility
wrappers. The physical runtime-candidate regression now uses the actual 1/60 f64
step; certified duration admission succeeds without narrowing. Its initial
ordinary setup tick is 1/120, so accepted target-phase progress remains observable.
All 10 playback, 9 runtime and 17 foot-placement tests passed. Ordinary App dispatch
still calls the legacy animation preparation path; this change supplies the exact
wall staging needed for its future certified fade branch. Common-frame policy,
upstream compilation/evaluation error enclosure and active-foot fade acceptance
remain required before enabling that branch in production.

### Partial fade runtime-candidate admission (2026-10-04)

The physical runtime-candidate regression now runs both free and wall-clipped
1/60 ticks. A dyadic wall/body fixture keeps final stored touching representable,
so partial-path admission can be tested independently of the separate contact
narrowing rejection. The clipped receipt is strictly between zero and one;
accepted target/contact phase and transition weight stop before full-tick values,
body translation stays at or before the wall, and displayed root extraction remains
in-place. The original runtime frame remains unchanged until candidate publication.
All 9 targeted animation-runtime tests passed. This verifies partial accepted
candidate staging for a translating fixture, not ordinary App fade dispatch,
noncommuting common-frame policy, active-foot contact behavior or general rounded
contact recovery.

### Explicit authored-origin common-frame assembly (2026-10-04)

`integrate_authored_common_frame` maps each interval-local path through its stored
original-phase root factor, then through a caller-selected authored-to-common
frame. Both compositions stay enclosed rather than first rounding composed poses.
The continuation frame follows the original mapped target endpoint. Missing
live-source factors reject. A distinct source/target phase regression checks
agreement with the explicit-frame reference, original f64 tick duration and
unchanged live animator; all 146 animation tests passed. The agreement tolerance
is a regression check, not an independent arithmetic enclosure proof. The method
formalizes a chosen stored-data mapping; it does not infer the actor's correct
common frame or enclose errors accumulated before imported/compiled factors were
stored. Actor-frame policy and those upstream errors remain production obligations.

### Root-origin transport contract (2026-10-04)

Inspected reference: phase rotation is R(t)*R(0)^-1 in parent coordinates; extracted
translation subtracts the original authored offset on selected axes. Consequently
F(0)=identity even with a nonidentity authored root rotation. An interval path is
rebased by F(start)^-1; restoring it through F(start) yields the parent-frame
increment F(end)*F(start)^-1. A regression verifies this order for a nonidentity
root origin, noncommuting turn, translating pivot, all-selected/all-unselected
and mixed-axis extraction. All 147 animation tests passed. The 1e-12 pose agreement
is a compatibility regression, not a proof that floating path compilation error
is enclosed. This establishes the clip-to-parent mapping; actor reference
transport across accepted fades, reloads and physical corrections still requires
explicit persistent ownership before ordinary dispatch can choose its common
frame automatically.

### Enclosed body-reference transport (2026-10-04)

`RootRigidEnclosure::transported_body_reference` implements
C_next = B_next^-1 * B_previous * C_previous using existing outward inverse and
composition owners. Thus the represented authored reference retains its world
anchor when body coordinates change. The regression uses dyadic positions and an
exact half-turn quaternion for an independent world-point reference, then returns
to the previous body and checks the same anchor. All 148 animation tests passed.
This is the transport primitive, not persistent runtime state ownership: callers
still must decide which body pose boundaries to use and when player input or
physics corrections should move/re-anchor the reference. No automatic common-frame
policy or ordinary App moving-fade dispatch is enabled by this addition.

### Owner-held root reference candidate (2026-10-04)

Animation owners now optionally retain an enclosed authored-to-body reference and
its accepted body-to-world pose. `accept_fade` requires the matching physical pose
owner and transports the reference while staging accepted playback/frame; failed
candidate preparation cannot mutate the original runtime. Ordinary owner rebuilds
retain a reference only for identical model/source-model Arcs, retarget profile and
root extraction settings; otherwise it is invalidated. The free/partial physical
fixture explicitly initializes the reference and verifies its world anchor stays
at zero while the original owner retains the unchanged reference. All 9 runtime,
10 playback and 17 foot-placement tests passed. Initialization/re-anchoring policy
is still not supplied by ordinary App dispatch, and non-fade movement paths do not
yet update this reference. Consequently this optional state cannot be treated as
an automatically valid common frame for every live tick or reload.

### Shared accepted-pose reference preparation (2026-10-04)

Reference transport now runs in the shared physical preparation callback before
optional foot IK, so it applies to ordinary movement as well as accepted fades.
The App selects that callback whenever an owner has foot settings or retained
root-reference state; absence of physics rejects either requirement. Fade staging
no longer performs a separate transport, avoiding duplicate interval growth for
the same accepted pose. A no-foot ordinary movement regression retains the
reference through owner preparation, transports it after a 0.25 displacement and
checks the fixed world anchor and unchanged original runtime. All 10 runtime,
10 playback and 17 foot tests passed. Reference initialization/re-anchoring remains
explicit and unwired in ordinary owner creation; moving-fade dispatch and compiler/
evaluation-error proof remain outstanding.

### Explicit root-reference initialization admission (2026-10-04)

`initialize_root_reference` provides the owner entry point for a chosen enclosed
authored-to-body frame at a known body-to-world pose. Invalid pose, disappeared
owner, different model Arc and repeat initialization reject before changing owner
state. Ordinary and fade transport fixtures now use this admission API rather than
writing private fields. The new regression verifies foreign model, nonfinite pose,
unchanged absent state, successful capture and rejection of silent reset. All
11 runtime, 10 playback and 17 foot tests passed. The caller still must choose a
physically justified initial authored frame and body snapshot; ordinary App owner
creation does not invent that choice. Input re-anchoring, full compiler/evaluation
error accounting and ordinary moving-fade dispatch remain outstanding.
