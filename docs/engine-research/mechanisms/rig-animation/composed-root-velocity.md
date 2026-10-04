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

### Root reference lifetime regression (2026-10-04)

The owner reference survives ordinary preparation with unchanged asset and extraction settings. A new model Arc, changed selected translation axes, or changed rotation extraction invalidates the candidate reference. Independent candidates leave the original owner reference intact. The editor runtime suite passes 12 tests; evidence is in `artifacts/rig-root-reference-lifecycle-2026-10-04`. This does not establish automatic initialization, source-model/retarget lifecycle coverage, or production moving-fade admission.

### Enclosed common-frame assembly (2026-10-04)

`RootRigidFadePlan::integrate_authored_common_enclosure` accepts the complete transported common-frame enclosure and composes both original phase factors and the target tail within that enclosure. The stored-transform entry point delegates to this implementation. The existing common-frame regression verifies identical nominal assembly for the same enclosure; all 148 animation tests pass. This API preserves input uncertainty, but actor reference submission and independent transported-reference integration qualification remain outstanding.

### Owner reference submission (2026-10-04)

`AnimationRuntime::prepare_owner_fade` now stages the original wall-time fade from owner-held extraction axes and authored-to-body enclosure. `PreparedModelFade::bind_common_enclosure` retains the entire enclosure through common-frame assembly. Missing initialization, foreign target models, and unqualified retarget binding are rejected. The free and wall-blocked physical fade regression now uses this owner submission instead of identity source/target factors. All 39 editor runtime/playback/foot tests pass. Ordinary App moving fades remain disabled; retarget-frame certification, extraction/reference snapshot admission, automatic initialization and input/gravity reference policy remain outstanding.

### Owner fade snapshot admission (2026-10-04)

`PreparedOwnerFade` wraps the bound motion with exact target/source asset identity, playback settings, retarget profile and both complete reference enclosures. Acceptance compares this snapshot before rebuilding any displayed frame; the lower-level plan still validates animator clocks and the physical receipt. Free and wall-blocked physical regression attempts now reject changed extraction axes, missing references and changed authored references, preserving scene transform, absent physical publication and the original displayed frame; a valid retry then succeeds. All 39 editor checks pass. Scene-authored changes still need to be admitted into the runtime before this comparison, and ordinary App moving-fade submission remains unconnected.

### Transported fade covariance regression (2026-10-04)

A new animation regression transports an identity authored reference into a translated, exact dyadic half-turn body frame, then assembles a moving/turning fade whose tick includes target-only completion. The independent half-turn inverse contains no trigonometric reference approximation. The resulting nominal motion matches conjugation of the original-frame approximation within the two integration error bounds, a rotation lever-arm term and a 1e-12 allowance for test-only floating composition. All 149 animation tests pass. This checks frame covariance and completion assembly; it is not an independent enclosure proof for imported curve compilation or runtime rounded pose evaluation.

### Authored scene admission (2026-10-04)

Owner fade staging now receives the current scene and loaded model table. It verifies scene identity, logical owner status, asset identity, current imported model Arc, retarget profile and validated/resolved playback settings. `PreparedOwnerFade::admit_scene` repeats this admission before a physics request can be constructed, returning an `AdmittedOwnerFade` used by both request and acceptance. Regressions reject authored extraction-axis changes without runtime synchronization, instance asset replacement and loaded model replacement; all 39 editor checks pass. The scene cannot be cloned and is mutably borrowed during physics, so scene admission occurs before that borrow; the orchestration must refresh admission after scene/model mutations and submit immediately. The token does not itself prohibit intervening edits. Ordinary App orchestration remains outstanding.

### Shared owner fade operation (2026-10-04)

`AnimationRuntime::fixed_step_owner_fades` admits current scene and owner snapshots inside one operation before borrowing the scene for the existing character physics transaction. It stages all receipt-bound accepted animation frames, then performs shared reference/foot preparation once. The returned animation runtime is published by the caller only after success. Free and blocked regression now uses this operation; authored changes after an earlier admission are rejected by its fresh boundary check. A two-owner regression proves that stale settings on the second owner prevent any scene/physics publication, then a corrected retry advances both owners while preserving original runtime frames. All 40 editor checks pass. App dispatch, automatic reference initialization, certified evaluation error, retarget motion and mixed ordinary/certified requests remain outstanding.

### Mixed certified character transaction (2026-10-04)

`CharacterPhysics::fixed_step_with_mixed_certified_fade_preparation` now combines ordinary root translations, ordinary rigid paths and certified fades before the existing single admission/query/publication transaction. The fade-only entry point delegates to it. A three-owner regression verifies all three motion kinds, shared accepted-pose preview, preparation failure rollback of scene/body/input, and successful retry publication. Gameplay passes 60 unit and 45 integration tests. Editor ordinary-frame staging still needs to join this same tick without reusing a previous tick motion buffer; ordinary App moving fades remain unconnected.

### Editor mixed owner staging (2026-10-04)

Ordinary runtime preparation now shares `prepare_deferred`, which clears old motion buffers and retains the already-staged fade owners without advancing them. `fixed_step_owner_fades` prepares all other owners for this tick, submits their new translations/rigid trajectories together with certified fades, and builds accepted fade frames into that candidate before shared pose preparation. The three-owner regression includes two fades and an ordinary translating owner; failed admission preserves all bodies, and retry advances the ordinary owner by the new tick displacement (twice the prior half-duration buffer), updates its frame/clock, and accepts both fades. All 40 editor checks pass. Ordinary clocks retain the existing f32 step convention; fade clocks retain their original f64 wall time. App dispatch, reference initialization, retarget/evaluation qualification remain outstanding.

### Certified planted fade investigation (2026-10-04)

A new three-joint moving/turning fade regression is not yet passing. With zero gravity, the initial no-motion preparation does not establish the retained plant: final sole moves by the body displacement although final physics reports grounded. Enabling gravity makes the initial ordinary preparation return `Physics(InvalidBody)` before the fade. The test now asserts initial grounding and remains failing; evidence is retained in `artifacts/rig-certified-planted-fade-investigation-2026-10-04`. This turn does not prove a moving-fade IK regression: initial support admission must first be established and the failure localized. Do not report the editor suite green until this is resolved.

### Certified planted fade completion (2026-10-04)

The previous investigation is resolved: the test mistakenly assigned positive gravity, while the character descriptor requires nonpositive acceleration. Correcting it to -9.8 establishes initial grounding and the planted contact. The regression now switches between distinct walk/run clips with different root translation and turning rates, then advances eight certified owner ticks through fade completion and its target-only tail. The sole remains within 3e-6 of the initial world anchor at every tick, contact interval endpoint matches accepted playback phase, and the original frame remains immutable. All 41 editor checks pass. Clip selection is explicitly synchronized in the fixture, not through ordinary App dispatch; blocked/frozen moving fades with active feet remain to be qualified. This does not demonstrate a defect in ordinary grounding or authorize zero compiler/evaluation error in production.

### Certified planted wall prefix (2026-10-04)

The active-foot walk/run fade regression now runs both freely and against a wall introduced after the first accepted tick. The wall case proves at least one nonzero accepted partial prefix with phase travel less than the requested tick, retained sole world anchor at every subsequent tick, and contact interval endpoint equality with accepted playback phase. Its transition remains incomplete; the final displayed affine body support along X remains at or before the exact dyadic wall plane. The free case still completes the fade and target-only tail. All 41 editor checks pass. This qualifies the stored-field fixture with caller evaluation radius zero, not arbitrary imported compiler/runtime error; frozen interruption and ordinary App admission remain outstanding.

### Certified planted frozen interruption (2026-10-04)

The moving/turning planted-foot regression now covers free and wall-blocked fades both with and without interruption. Interruption creates a frozen animation source, switches the target back to walk, and retains the owner-held authored reference. A deliberately exhausted angular query budget rejects the interrupted tick without changing scene transform, original frame Arc, playback phase or physical body. Retrying the same staged candidate with the restored budget succeeds and exposes frozen-source tick metadata. The sole remains at its initial world anchor and target contact travel matches accepted playback throughout; free interruption completes while the wall case remains constrained. All 41 editor checks pass. Fixed stance weight is used here; variable contact curves, App selection admission and nonzero compiler/evaluation error remain outstanding.

### Scene-driven owner selection staging (2026-10-04)

`prepare_playback_selection` now owns selection/speed/motion-joint and reload decisions shared by ordinary preparation and `stage_owner_selection`. Selection staging admits scene/model/profile identity, validated/resolved selection and existing foot binding without advancing animation time or replacing the accepted frame/serial. Extraction identity changes invalidate the reference. The planted moving/turning tests now select run and interrupt back to walk through scene settings and this API, rather than mutating owner playback/settings directly. They verify retained running source phase and accepted frame identity; changed foot binding is rejected both at selection and after fade staging. Fade snapshots now include authored foot settings. All 41 editor checks pass. Resource rebinding/foot binding reconstruction stay in ordinary preparation; App dispatch and automatic root-reference initialization remain outstanding.

### Current-phase reference capture (2026-10-04)

`Animator::root_rigid_phase_factor` reads the stored extraction factor from the same bounded phase convention as fade staging, without advancing the clock. An active transition rejects phase-anchor guessing. `ModelPlayback::reference_at_current_phase` composes the caller-selected current-root-to-body enclosure with the enclosed inverse phase factor. `initialize_root_reference_at_phase` admits that reference at an explicit accepted body pose, while retaining stale-model/reset checks and rejecting unqualified retarget binding. The planted free/wall/interruption scenarios now use this phase capture instead of an identity authored reference. Animation passes 150 checks; editor passes 41. Both phase capture and certified owner fade now require rotation extraction because the current rigid field includes angular motion; translation-only fade fields need separate compilation. Automatic body/parent selection and full compiler/evaluation error qualification remain outstanding.

### Published physical pose reference capture (2026-10-04)

`CharacterPhysics::accepted_pose` exposes the published solver center/orientation plus world matrix, velocity and grounding, without reconstructing orientation from narrowed scene values. It returns no accepted pose for unstepped, inactive, detached or supported externally edited bodies; foreign scenes and unsupported transforms still fail validation. A mixed-motion regression checks exact equality with transaction preview and invalidation after position, rotation, descriptor and activity changes. `AnimationRuntime::capture_owner_reference` admits scene selection and uses this published pose for phase reference capture; the caller continues to choose the current-root-to-body enclosure. The planted free/wall/interruption fixtures use this operation and reject capture before the first accepted physical tick. Gameplay passes 45 integration tests; editor passes 41 checks. Automatic root-parent frame selection and ordinary App dispatch remain outstanding.

### Signed common-frame translation units (2026-10-04)

`RootRigidEnclosure::with_translation_scale` encloses signed uniform translation-unit conversion while retaining proper rotation. `RootRigidFadePlan::integrate_authored_common_similarity` scales both original phase factors and their velocity paths into common/body units; the target-only tail uses the same scaled endpoint enclosure. Common-frame translation remains in body units and is never rescaled. The previous common-frame entry point delegates with scale one. The transported-frame covariance regression now covers scales 1, -2 and 0.5 through fade completion, with zero/nonfinite rejection; all 150 animation tests pass. This is the required unit conversion before storing a scaled root-parent reference in the editor. Editor scale ownership, parent-frame capture and App dispatch remain outstanding.

### Owner-held signed reference scale (2026-10-04)

`RootReference` now owns a nonzero signed uniform scale alongside its body-frame enclosure. Scaled phase capture inverts the already-scaled stored factor; body transport retains the scale while moving the common-frame translation in body units. `PreparedModelFade::bind_common_similarity` submits this scale to the common-frame assembler, and owner snapshot admission compares it before publication. A new imported-linear-clip regression verifies scales 0.5 and -2 over consecutive physical ticks, known asset rate 2 units/second, preserved world anchor enclosure, and invalid scale capture rollback. Existing free/blocked receipt regressions also reject a stale owner scale. All 42 editor checks pass. Root-parent frame/scale discovery and ordinary App dispatch remain outstanding; caller-selected external request scale stays a separate conversion obligation.

### Enclosed ancestor scale products (2026-10-04)

`RootUniformScaleEnclosure` retains outward products of stored signed scales. Translation-unit conversion, mapped root velocity fields and common-frame fade/tail assembly now accept that full enclosure; derivative caps use its maximum absolute endpoint. Existing stored-scalar entry points delegate without losing field bounds. A regression propagates two multi-factor signed products through fade completion. Binary64 input/output bit patterns are checked independently with Python exact rational arithmetic: all eight real products lie inside the emitted bounds without epsilon. Animation passes 151 checks; reusable verifier and logs are in `artifacts/rig-enclosed-scale-products-2026-10-04`. Editor ancestor-chain traversal and scale-enclosure ownership are not yet connected.

### Static root-parent capture (2026-10-04)

`constant_parent_similarity_enclosure` traverses the selected root ancestors, composes stored translations and enclosed normalized rotations, represents sign reflections with exact proper half-turn quaternions, and accumulates outward signed uniform scale products. It requires identical constant ancestor transforms across model clips; moving, differing or nonuniform ancestors reject. Owner reference scale is now the complete `RootUniformScaleEnclosure`, propagated through phase capture, snapshot comparison, body transport and common fade assembly. `capture_owner_reference_from_parent` combines this derived frame with the published physical pose; planted free/wall/interruption fixtures use automatic parent capture. A nested reflected-parent regression checks three points against independent exact dyadic transforms, and rejects nonuniform/moving parents. All 43 editor checks pass. Different static parent frames per clip, retarget frame qualification, compiler/evaluation error and ordinary App dispatch remain outstanding.


### Automatic accepted-pose reference bootstrap (2026-10-04)

The editor now initializes an unreferenced rigid root-motion owner inside accepted-pose preparation, after physics supplies the accepted body pose. Static parent similarity and current playback factor establish the reference without a separate post-step capture. Existing references are transported once; newly initialized references already use the accepted body frame. Retarget owners remain excluded from automatic capture pending qualification of their common frame.

The preparation operation now names both responsibilities: `prepare_accepted_pose` transports or initializes the root reference and then corrects feet. A callback rejection after initialization leaves scene, physics publication, displayed frame, and original reference untouched; retry initializes successfully. The planted free, wall-prefix, and interrupted-fade scenarios use automatic capture. Editor runtime/playback/foot tests: 42 passed. Ordinary App moving-fade dispatch remains incomplete; this verifies bootstrap and transactional preparation, not complete production integration.


### Scene selection and fade batch (2026-10-04)

`prepare_scene_fades` reads active scene owners with initialized references, stages their selections using the existing selection owner, and prepares all active rigid fades before advancing physics. It returns a candidate and owned plans; failure of a later owner exposes no partial candidate or clock publication. Caller-provided integration tolerances remain explicit. This is preparation infrastructure, not yet ordinary App dispatch or a production sampling-error policy.

A two-owner scene-selection regression verifies both plans, unchanged accepted frame and serial, and rejection of an invalid second clip while the original runtime remains untouched. A newly selected target starts at its existing selection-policy phase; staging does not advance time. All 18 animation runtime tests pass, including bootstrap rollback, mixed physics submissions, planted wall prefixes, interruption, and signed scales. Evidence: `artifacts/rig-scene-fade-batch-2026-10-04/`.


### Scene-driven physical fade operation (2026-10-04)

`fixed_step_scene_fades` joins scene selection staging and plan compilation with existing mixed physics admission and accepted-pose publication. It requires an explicit evaluation frame for every active fade and rejects extra frames without active plans. No candidate escapes between selection and physical execution. Production evaluation-error policy and ordinary App dispatch remain incomplete.

The batch regression now executes two scene-selected fades through this operation, verifies two receipts, published physics poses, advanced target clocks and replaced accepted frames, and confirms that the original runtime remains unchanged. A missing second evaluation frame rejects before physics publication; correcting the request succeeds. All 18 runtime regressions pass. Evidence: `artifacts/rig-scene-fade-operation-2026-10-04/`.


### Bind-pose admission and evaluation-margin review (2026-10-04)

An owner with rotation extraction configured but no selected clip has no playback phase factor. Automatic accepted-pose preparation now waits for a selected clip rather than attempting phase capture in bind pose. Existing references still transport normally. A regression executes bind-pose physics without creating a reference, selects a clip, then verifies automatic initialization after its accepted physical tick. All 19 runtime regressions pass.

Review of the actual sweep confirms that the scalar world evaluation radius is added to both whole-body clearance and the directional coordinate certificate margin. The existing `completion_keeps_floor_contact_and_wall_stops_motion_after_fade` regression asserts that radius 0 accepts floor-tangent motion while radius 0.001 prevents completion at exact contact. Therefore a positive arbitrary global allowance is not a workable production contact policy. Direction-specific upstream compilation/evaluation enclosures remain necessary; the current scalar contract cannot infer a zero normal error. Ordinary editor fade dispatch remains unqualified.


### Separate world-axis evaluation margins (2026-10-04)

The shared certified sweep now has an internal entrypoint accepting a whole-body world evaluation radius and three independently proven world-coordinate error bounds. The existing scalar entrypoint delegates with the scalar bound on each axis, retaining its conservative contact behavior. Coordinate certificates accumulate their integration discrepancy with each world-axis error separately; invariant plane admission chooses the actual world normal axis after exact actor/authored coordinate preimage checks. Whole-body clearance still uses the full radius. Nonfinite, negative, or per-axis bounds larger than that radius reject.

The fade-completion floor regression now proves that a radius 0.002 with axis errors [0.001,0,0.001] permits tangent floor motion, while [0.001,0.001,0] prevents completion. It rejects negative, NaN and oversized axis bounds. All 60 gameplay unit tests pass. These bounds remain caller proof obligations; public staged requests still use the scalar interface, and imported compiler/evaluation bounds and ordinary App dispatch remain incomplete. Evidence: `artifacts/rig-world-axis-evaluation-errors-2026-10-04/`.


### World-axis errors through staged requests (2026-10-04)

`CharacterCertifiedFadeMotion` now carries optional caller-proven world-axis evaluation bounds in addition to its whole-body radius. None retains the scalar contract. The staged physics admission rejects nonfinite, negative and oversized axis bounds before publication. The shared certified sweep consumes those bounds, and editor `OwnerFadeFrame` forwards them through scene-driven and explicitly prepared operations.

The existing transactional fade integration regression now uses radius 0.002 with world-axis bounds [0.001,0,0.001] at exact floor contact. It verifies complete movement, callback-rejection rollback of scene/body/input, successful retry, and early rejection of three invalid bound sets without invoking publication preparation. All 45 gameplay integration tests and 27 editor runtime/playback tests pass. This verifies transport and consumption of externally proven bounds, not derivation of imported animation/compiler errors. Ordinary App moving fades remain incomplete. Evidence: `artifacts/rig-axis-error-transaction-2026-10-04/`.


### Enclosed point publication discrepancy (2026-10-04)

`RootRigidEnclosure::enclosed_point_evaluation_error` computes outward world-axis discrepancy between the exact enclosed similarity image of a point box and an already evaluated world coordinate. It also returns an outward L1 radius bounding Euclidean discrepancy, without a square-root or guessed epsilon. The evaluated coordinate may be a widened f32 publication. Invalid/nonfinite inputs reject; exact zero maps retain zero discrepancy.

A regression compares signed scales and an exact dyadic half-turn at translation 16777216 against independently constructed dyadic coordinates. It verifies that the lost unit at f32 publication is enclosed and that all axes and Euclidean discrepancy are covered. All 152 animation tests pass. This derives a bound for a specific point/pose; it is not a uniform bound for all points and times in a physical tick, nor a proof of imported curve compilation. Those obligations and ordinary App fade dispatch remain incomplete. Evidence: `artifacts/rig-point-publication-error-2026-10-04/`.


### Uniform f32 publication bound on a world box (2026-10-04)

`RootRigidEnclosure::enclosed_f32_publication_error` accepts a world-coordinate enclosure and returns world-axis rounding-error bounds plus an outward L1 radius. For varying coordinates, one full adjacent f32 spacing at an upward endpoint magnitude covers round-to-nearest errors at all smaller magnitudes, including subnormals and binade changes. Near f32::MAX it uses the finite preceding spacing and rejects coordinates beyond the finite representable range. Singleton coordinates use enclosed exact subtraction from their actual f32 result, preserving zero for fixed exactly representable coordinates.

The regression checks symmetric large-coordinate ranges, subnormal ranges, a binade boundary, the upper finite range, fixed zero/quarter axes, and singleton loss of a unit at 16777217. Invalid/reversed/overflow ranges reject. All 153 animation tests pass. This is uniform rounding-only coverage on the supplied box, not a derivation of the world box for an entire tick or coverage of earlier path/curve arithmetic. Those obligations remain outstanding. Evidence: `artifacts/rig-f32-publication-box-error-2026-10-04/`.


### Whole canonical path point box (2026-10-04)

`RootScrewEnclosurePath::span_fraction_enclosure` encloses canonical poses throughout a closed fraction interval using an outward elapsed-time interval and the existing interval exponential, composed with the immutable canonical prefix. `whole_path_point_box_bounds` takes the hull of all span images and the initial box. It uses entire intervals, not sampled extrema, and retains existing unsupported-angle/overflow admission boundaries.

A two-span moving/turning regression checks the whole hull against narrower pose images, retains an exactly fixed zero Y coordinate, and derives uniform f32 rounding bounds on that hull. It exercises corners, invalid fraction intervals, out-of-range spans and an empty path. All 154 animation tests pass. This supplies a whole-path box for the stored canonical screw reference; transforming it to the actor world, enclosing earlier numerical evaluation, and proving imported curve compilation remain necessary for ordinary App integration. Evidence: `artifacts/rig-whole-path-point-box-2026-10-04/`.


### World body vertex publication bounds (2026-10-04)

Gameplay now derives rounding-only bounds for body vertices throughout the canonical path. It encloses inverse source-frame vertices with the existing collision helper, takes each whole-path point box, maps it through actor/source composition and signed scale, and derives f32 publication bounds. The actor/source frame construction was extracted and reused by the actual collision sweep so both operations use the same outward coordinate model. Axis bounds and whole-point radii are maxima across vertices.

The regression exercises two moving/turning spans, exact actor/authored half-turns, a translated pivot, three signed scales and actor translation 16777216. It checks all body vertices across sample fractions against the derived uniform bounds. All 61 gameplay unit tests pass. The helper is not yet used to supply automatic admission: earlier evaluation and compiler errors must be included, and a rounding-only result must not be presented as a complete error certificate. Evidence: `artifacts/rig-world-body-publication-error-2026-10-04/`.


### Stored-key translation compilation discrepancy (2026-10-04)

Each `RootCurve` knot now retains an outward coefficient-compilation discrepancy from the exact stored f32 keys/tangents and times. The interval compiler encloses subtraction of the root origin and STEP/linear/cubic power coefficients, then sums coefficient discrepancies per coordinate; because every normalized monomial has magnitude at most one on [0,1], that sum bounds position discrepancy at equal normalized parameter. Failures remain explicit in the knot proof and are propagated by the query rather than erased. `RootRigidCurve::translation_compilation_error_bounds` exposes the maximum over knot intervals.

A regression uses origin 2^100 and a final key of 1, where the relative cache loses a whole unit, and verifies coverage for all three interpolation modes while preserving exact zero on unrelated coordinates. Empty curves return zero. All 155 animation tests pass. This proves only translation coefficient compilation at equal normalized parameter. Runtime parameter evaluation, power-to-Bernstein conversion/restriction, rotation compilation, extraction/factors and loop composition remain separate obligations, so no complete imported-curve or ordinary App acceptance claim follows. Evidence: `artifacts/rig-translation-compilation-errors-2026-10-04/`.


### Restricted translation Bernstein discrepancy (2026-10-04)

`RootRigidCurve::translation_piece_compilation_error_bounds` now bounds exact stored-key translation against the rounded restricted Bernstein piece. It widens cached power coefficients by the retained compilation discrepancy, encloses exact normalized u/v using outward subtraction and interval division of stored key times, then encloses restriction and power-to-Bernstein conversion. Maximum control discrepancy per coordinate bounds the whole Bernstein curve by convexity. Requests crossing a translation key or invalid time interval reject; held regions retain their corresponding compilation proof.

The compilation regression now exercises a restricted interval of all three channel modes, confirms preserved zero orthogonal axes, rejects cross-key and NaN intervals, and distinguishes a STEP's exact pre-jump zero from linear/cubic lost-unit coverage. All 155 animation tests pass. The interval times are interpreted exactly as supplied: phase selection, rotation, extraction frames and cycle composition remain separate obligations. Evidence: `artifacts/rig-translation-piece-errors-2026-10-04/`.


### Quaternion key normalization discrepancy (2026-10-04)

`RootRotationCurve` now retains componentwise outward discrepancy between its rounded normalized quaternion keys/bind fallback and real normalization of the original stored f32 components. The proof uses outward squares, sum, square root and interval division, then compares against the cached normalized values. `RootRigidCurve::rotation_key_normalization_error_bounds` exposes that retained proof. Invalid zero/nonfinite normalization rejects rather than publishing a missing proof.

A regression covers STEP/linear/cubic modes with axial keys, verifies bounded normalization discrepancy against the independent equal-component reference, and preserves exactly zero X/Z components. It also rejects zero and NaN quaternion inputs. All 156 animation tests pass. This covers key normalization only; arc logarithms, cubic control construction/restriction, relative origin/cycle compositions and extraction remain outstanding. Evidence: `artifacts/rig-rotation-key-normalization-proof-2026-10-04/`.


### Quaternion cubic control compilation discrepancy (2026-10-04)

`RootRotationCurve` now retains componentwise outward discrepancy for raw cubic Bernstein controls compiled from exact stored quaternion keys, tangents and key times. Outward elapsed time, division by three and tangent/control arithmetic cover the rounded cached controls. Maximum control error bounds raw polynomial discrepancy by Bernstein convexity. `RootRigidCurve::rotation_cubic_control_compilation_error_bounds` exposes it; non-cubic channels return None rather than a misleading zero cubic certificate.

A regression checks nonzero tangent/duration rounding with exact zero X/Z and unchanged W components, distinguishes a linear channel, and rejects reversed times. All 157 animation tests pass. Normalized cubic pose error requires a positive norm lower bound and normalization amplification; restriction, relative frames, arc logarithms and cycle composition remain outstanding. Evidence: `artifacts/rig-quaternion-cubic-control-proof-2026-10-04/`.


### Normalized cubic compilation discrepancy (2026-10-04)

`RootRotationCurve::cubic_normalized_compilation_error_bounds` now derives a uniform source/cached normalized-polynomial discrepancy over complete cubic key spans. Bernstein component hulls supply a positive norm lower bound L for the cached polynomial. The retained raw control error yields an outward L1 discrepancy E; requiring L>E proves that the exact source polynomial remains nonsingular too. Normalization discrepancy is bounded outward by 2E/L. Exactly zero components shared by source/cached polynomials retain zero error. Key/bind normalization discrepancies are included for held endpoints; non-cubic channels remain None.

The cubic regression verifies small finite normalized bounds and exact zero axial components, and rejects a hull containing the zero quaternion and an error large enough to invalidate the norm proof. All 157 animation tests pass. The hull proof can conservatively reject nonsingular curves requiring subdivision. Runtime rounded normalization/evaluation, restricted cubic controls, relative frame/cycle composition and arc logarithm proof remain outstanding. Evidence: `artifacts/rig-cubic-normalized-compilation-proof-2026-10-04/`.


### Restricted normalized cubic compilation discrepancy (2026-10-04)

`RootRotationCurve::cubic_piece_compilation_error_bounds` carries exact-source control discrepancy through restriction of one key interval. It encloses elapsed/key-time division, de Casteljau splitting at the requested end, and the second split at the exact start/end ratio. The restricted source controls are compared with the actual rounded cached restriction; Bernstein control discrepancy and a positive norm proof then bound normalized source/cached curves uniformly. It rejects cross-key intervals and invalid times. Relative left/right frames and runtime pose arithmetic remain outside this certificate.

Stored-key compilation proofs now live together in `root_rigid/enclosure/compilation.rs`, reusing existing outward scalar arithmetic and interpolation. The cubic regression verifies finite small restricted error, preserved zero X/Z components and rejection of a cross-key interval. All 157 animation tests pass. Runtime evaluation, arc compilation and relative/cycle compositions remain outstanding before ordinary App integration. Evidence: `artifacts/rig-cubic-restriction-compilation-proof-2026-10-04/`.


### Pointwise cubic runtime evaluation discrepancy (2026-10-04)

`RootRotationCurve::cubic_phase_evaluation_error_bounds` now compares its actual local cubic quaternion sample against an enclosure of the normalized exact-source polynomial. It starts from cached controls widened by source compilation discrepancy, encloses exact key-time parameter division and de Casteljau evaluation, proves positive norm, then encloses normalized components and subtracts the actual quaternion produced by the existing `local`/`unit` implementation. Thus earlier rounded polynomial evaluation and normalization are included by comparison with the source enclosure, not by a guessed operation count. Held endpoints use retained key normalization bounds.

The cubic regression queries 33 supplied phases, verifies small finite bounds and exact zero axial components, and rejects invalid phases. All 157 animation tests pass. This is pointwise at the exact supplied phase, not a uniform error bound for a whole tick; phase/loop selection, relative quaternion frames and arc proof remain outstanding. Evidence: `artifacts/rig-cubic-phase-evaluation-proof-2026-10-04/`.


### Pointwise translation runtime discrepancy and imported proof retention (2026-10-04)

`RootCurve::phase_evaluation_error_bounds` now encloses exact supplied key-time parameter division and Horner evaluation from cached power coefficients widened by source compilation discrepancy, then compares to the actual existing local position evaluator. `RootRigidCurve` exposes the pointwise source-to-runtime translation error. Held knots use retained compilation bounds; invalid phases reject. The lost-unit regression exercises this evaluation for all interpolation modes while preserving zero orthogonal coordinates.

An editor regression loads the existing animated GLB model through the normal parser, obtains the shared selected-joint rigid curve and verifies retained translation/quaternion-key proofs plus finite small translation evaluation error at 33 local phases. Its non-cubic rotation channel explicitly returns None for cubic evaluation. All 157 animation tests and 45 editor runtime/playback/foot-placement tests pass. These are pointwise phase proofs, not uniform tick or relative-frame/cycle certificates; arc compilation and complete App dispatch remain outstanding. Evidence: `artifacts/rig-translation-evaluation-import-proof-2026-10-04/`.


### Uniform translation runtime discrepancy (2026-10-04)

`RootCurve::interval_evaluation_error_bounds` now supplies source-to-runtime translation error uniformly throughout one closed key interval. It encloses key-time denominator subtraction, numerator rounding and quotient sensitivity, then propagates a rounding-error bound through each Horner multiplication/addition using interval magnitudes and complete f64 spacing bounds. Source coefficient compilation discrepancy is added at equal exact normalized parameter. The right endpoint includes the next key's proof explicitly, respecting STEP right-continuity. Exact zero coordinate polynomials retain zero error; invalid/cross-key intervals reject.

A dyadic cubic regression compares actual evaluations with an independently expanded exactly representable polynomial across the interval. The lost-unit regression covers all interpolation modes and the STEP endpoint. All 158 animation tests pass. The supplied phase interval is interpreted exactly; clock/loop mapping, uniform quaternion runtime error, relative extraction frames and arc compilation remain outstanding before ordinary App moving fades. Evidence: `artifacts/rig-uniform-translation-evaluation-proof-2026-10-04/`.


### Uniform cubic runtime discrepancy (2026-10-04)

`RootRotationCurve::cubic_interval_evaluation_error_bounds` now derives a uniform source-to-runtime quaternion error on one closed key interval. A shared rounded-parameter owner encloses time subtraction/division. Rounded-range propagation follows the actual de Casteljau lerp multiply/add operations, while retaining exact nominal convex interpolation ranges. Source control compilation discrepancy is combined with the raw runtime error; a positive source norm and a discrepancy smaller than that norm prove normalization remains nonsingular. Scaled normalization error follows the inspected glam f64 divide-by-max, dot, sqrt, reciprocal and multiplication path, with full-spacing rounding bounds. Exact zero source/cached components retain zero error; right-key normalization is included separately at the endpoint.

All 158 animation tests pass. An independent Decimal calculation at 100-digit precision compares 132 quaternion components over 33 phases against the uniform error without extra tolerance; the verifier and raw actual samples are saved. The phase interval is exact as supplied. Relative rotation/extraction frames, loop mapping/composition and arc logarithm compilation remain unqualified; ordinary App dispatch remains incomplete. Evidence: `artifacts/rig-uniform-cubic-evaluation-proof-2026-10-04/`.


### Pointwise relative cubic rotation discrepancy (2026-10-04)

`RootRotationCurve::cubic_relative_phase_evaluation_error_bounds` now includes the initial-key inverse frame: it encloses the normalized exact-source local sample and initial quaternion through retained component errors, forms their real unit Hamilton product with the shared outward rigid-frame composition, then compares to the actual normalized `phase_rotation` output. This accounts for rounded product/normalization by comparison with an enclosed source product. It is pointwise at the supplied local phase; a unit-source proof is required for each input.

A regression starts from a nonidentity axial quaternion, samples a zero-tangent cubic midpoint and independently derives relative rotation (0,1,0,2)/sqrt(5). It verifies coverage and exact zero X/Z errors, and rejects NaN phase. All 159 animation tests pass. Uniform relative-frame evaluation, cycle selection/composition, translation extraction coupling and arc compilation remain outstanding. Evidence: `artifacts/rig-relative-cubic-phase-proof-2026-10-04/`.


### Explicit cubic cycle composition discrepancy (2026-10-04)

`RootRotationCurve::cubic_cycle_phase_evaluation_error_bounds` now carries source key normalization, cycle construction and binary-power composition errors through an explicit cycle count and local phase. `power_with_error` mirrors the actual rounded quaternion operations and propagates source unit-product enclosures at each used multiply/square. The final query matches sample's unnormalized intermediate product and its final normalization; unused final square proofs are omitted because they cannot affect the returned orientation. Nonzero cycle counts reject for Clamp playback.

The nonidentity-origin regression now uses Loop playback and checks four cycles plus midpoint against the independently derived negative quaternion (0,-1,0,-2)/sqrt(5), preserving zero X/Z errors and rejecting invalid phase. All 159 animation tests pass. This does not prove cycle/phase selection from wall time, uniform relative-frame error, translation/rotation extraction coupling or arc compilation. Ordinary App fades remain incomplete. Evidence: `artifacts/rig-cubic-cycle-composition-proof-2026-10-04/`.

### Exact cycle selection in rigid path partitioning (2026-10-04)

Rigid path loop budgeting and segment cycle selection now reuse the exact dyadic clock helper instead of rounded division. Animation library: 161 tests passed. This qualifies cycle selection only: subtraction used for local segment times and translation-only weighted integration remain separate numerical obligations. Ordinary App moving fades remain disabled. Evidence: `artifacts/rig-exact-cycle-clock-2026-10-04/path-tests.log`.

### Local rigid segment phase extraction (2026-10-04)

Rigid translation segment endpoints now use the exact dyadic cycle remainder rather than subtracting a rounded cycle origin. A right endpoint exactly at the next seam maps to the clip duration (left limit); a segment crossing an interior seam rejects and requires partitioning. Added regression coverage for the large-clock remainder, adjacent seam endpoint, interior seam rejection, and clamp. Animation library: 162 tests passed. Cut construction and addition of relative time to the path origin still require numerical qualification; ordinary App moving fades remain disabled.

### Translation partition admission (2026-10-04)

Rigid path construction now validates finite ordered local segment endpoints and rejects any segment extending past its next translation key before restricting the cached polynomial. The endpoint exactly at the key is admitted as the pre-event endpoint; the next segment starts on the right-continuous key. Regression checks cover STEP, LINEAR and CUBICSPLINE, including a one-ULP overshoot. Animation library: 162 tests passed. This prevents silent cross-key extrapolation; it does not yet enclose the rounding of generated cut times or recover an unrepresentable cut. Ordinary App moving fades remain disabled.

### Outward time-cut enclosures (2026-10-04)

RootTimeCutEnclosure encloses the exact stored-source expression cycle*duration+key-origin using outward basic arithmetic and bounds its discrepancy from the evaluated cut. Rigid translation cut generation shares this evaluator for keys and seams. Tests cover a large dyadic cycle, complete loss of a small key offset through cancellation, and invalid inputs. Animation library: 163 tests passed. Bounds are currently queryable infrastructure: rigid path spans do not yet retain or propagate time-cut errors into spatial velocity/error certificates. Ordinary App moving fades remain disabled.

### Continuous displacement from clock discrepancy (2026-10-04)

RootTimeCutEnclosure::continuous_motion_error computes outward displacement <= dt*(linear_speed+angular_speed*point_radius) and angular discrepancy <= dt*angular_speed. Caller bounds must cover the whole exact/evaluated time corridor in a fixed spatial frame. STEP impulses are excluded and need explicit event handling. Zero-speed and zero-radius cases preserve exact zero; invalid bounds reject. Animation library: 163 tests passed. Automatic corridor-speed acquisition, path metadata retention through retiming/appending, and collision admission remain unconnected. Ordinary App moving fades remain disabled.

### Translation cut metadata retention (2026-10-04)

RootRigidPath retains enclosures for admitted translation-key and loop cuts. Coordinate transformation preserves these clocks; retiming maps exact bounds through outward division/multiplication; spatial append shifts following clocks outward by the stored preceding duration. Regression exercises a turning loop path, retiming, frame transformation and append. Animation library: 164 tests passed. Empty metadata on synthetic/integrated twist paths does not certify their clocks. Dropped/unrepresentable cuts, rotation partition clocks, initial elapsed/local-origin rounding, and preceding-duration uncertainty are not covered. Collision consumers still do not automatically derive corridor speeds or spatial allowances from this metadata. Ordinary App moving fades remain disabled.

### Automatic stored-field cut corridor speeds (2026-10-04)

RootRigidPath::translation_cut_motion_errors encloses stored spatial velocities across the hull of each cut source interval and evaluated clock, including both continuous neighbor limits. Outward L1 bounds feed the continuous clock-to-displacement conversion. Unsupported fields return None; STEP corridors or out-of-path clocks reject. A two-speed key regression verifies use of the faster neighbor. Animation library: 165 tests passed. These are per-cut allowances for the stored field, not aggregate source-motion certificates: source field compilation error, cut/event topology, accumulated pose uncertainty and collision integration remain separate obligations. Ordinary App moving fades remain disabled.

### Exact stored-key translation derivative bounds (2026-10-04)

RootRigidCurve::translation_source_velocity_bounds encloses the derivative of exact-source relative translation on one key interval. Cached coefficients are widened by their proved compilation discrepancy; key duration, normalized parameter and derivative evaluation use outward arithmetic. This bounds the source polynomial, rather than merely rounded runtime velocities. STEP derivative is zero with impulses handled separately; out-of-key domains reject. Regression compares exact dyadic linear and cubic derivative formulas over 65 parameters, and preserves zero orthogonal axes. Animation library: 166 tests passed. Rotation coupling, angular source derivatives, loop prefix errors and collision policy remain unqualified. Ordinary App moving fades remain disabled.

### Exact-source cubic angular speed bound (2026-10-04)

RootRotationCurve::cubic_source_angular_speed_bound and the RootRigidCurve delegate bound normalized source angular speed by 2*|raw derivative|/|raw quaternion|. Raw Bernstein controls are widened by proved compilation discrepancy; derivative controls and a positive quaternion-norm lower bound use outward arithmetic. Whole-key bounds cover requested single-key subintervals; uncertified singular hulls reject and noncubic modes return None. An analytic smoothstep rotation regression checks 65 phases. Animation library: 167 tests passed. LINEAR arc source/log proofs, extraction/pivot coupling, loop frame uncertainty and collision consumers remain unfinished. Ordinary App moving fades remain disabled.

### Coupled local source point speed (2026-10-04)

RootRigidCurve::source_point_speed_bound combines source translation derivatives, outward source position ranges and normalized cubic source angular speed. It bounds a local authored point using extraction masks: selected axes retain bind pivots, unselected axes carry the moving authored position, whose derivative contributes separately. Caller radius bounds the fixed point about the local origin. Regression uses an accepted animation clip with unit rotation keys, nonzero bind pivot and both extraction masks. Animation library: 168 tests passed. Bounds cover one shared key interval only; parent/loop prefixes, field discrepancy accumulation, STEP events and collision admission remain incomplete. Ordinary App moving fades remain disabled.

### Fixed similarity source speed mapping (2026-10-04)

RootRigidEnclosure::inverse_similarity_point_box_bounds_enclosed bounds inverse point coordinates with fixed enclosed real-unit rotation/translation and an invertible signed scale interval. RootRigidCurve::mapped_source_point_speed_bound derives an outward local point radius, computes the coupled source speed, and multiplies by the maximum absolute enclosed scale. Regression covers translated frames, positive scale, reflection and zero-scale rejection. Animation library: 168 tests passed. Frame ownership must prove a normalized real rotation and fixed mapping; dynamic parents, source loop prefix construction and collision admission remain unfinished. Ordinary App moving fades remain disabled.

### Exact-source rigid phase and loop enclosures (2026-10-04)

RootRigidCurve source_phase_enclosure combines source translation coefficient bounds, cubic relative-quaternion evaluation discrepancy and extraction pivot algebra. source_cycle_prefix_enclosure composes the resulting exact-source cycle by binary exponentiation, retaining real-unit rotation semantics without rounded normalization. source_sample_enclosure uses exact dyadic clock selection and requires a singleton exact remainder. Regression checks exact linear translation with cubic identity rotation over several cycles and preserves zero orthogonal coordinates. Animation library: 169 tests passed. Noncubic rotations, tight enclosure growth, turning-cycle independent reference validation, dynamic frames and automatic collision admission remain incomplete. Ordinary App moving fades remain disabled.

### Independent turning-cycle source validation (2026-10-04)

Added a clip with exact identity/half-turn Y quaternion keys, zero cubic tangents, linear X translation and a nonzero bind pivot. At each cycle midpoint, exact translation is (5/2,0,+/-2); quaternion Y/W components are signed sqrt(1/2), with period four including quaternion sign. A separate Fraction verifier checks all 63 components over nine cycles using rational inequalities and squared square-root bounds, with no tolerance and no production quaternion operations. Animation library: 170 tests passed. This fixture validates source loop composition/pivot order; arbitrary turning cubic paths, LINEAR rotations, source-field bounds and automatic collision admission still require qualification. Ordinary App moving fades remain disabled.

### Subdivided source quaternion nonsingularity (2026-10-04)

Cubic source angular-speed compilation now uses outward Bernstein subdivision to establish a positive norm lower bound over every cell. This admits a valid identity-to-half-turn cubic whose unsplit coordinate hull includes zero. Depth is bounded at 12 and total visited cells at 8191; unresolved/singular source families reject. The global derivative control hull remains conservative. The half-turn fixture now checks angular speed against its analytic normalized smoothstep expression over 65 parameters and verifies coupled point-speed admission. Animation library: 170 tests passed. This improves source speed qualification; collision consumers and ordinary App moving fades remain unconnected.

### Source interval delta and evaluated point discrepancy (2026-10-04)

RootRigidCurve::source_delta_enclosure evaluates source_start inverse composed with source_end, including loop powers and pivot extraction. Equal endpoints preserve exact identity. source_delta_point_error compares its exact enclosed point image to a supplied evaluated point. The turning-cycle fixture at [0.5,1.5] has exact relative translation (4,0,0), half-turn rotation, and image (3,0,0) for point X; the actual path endpoint discrepancy is bounded with radius below 1e-10. Animation library: 170 tests passed. These are pointwise endpoint proofs; uniform intermediate pose bounds, fade source-field discrepancy and collision admission remain incomplete. Ordinary App moving fades remain disabled.

### Uniform continuous source pose intervals (2026-10-04)

source_phase_interval_enclosure integrates proved source origin-speed and quaternion angular-speed caps over a checked single-key interval, widening the starting source factor outward. Quaternion component variation is bounded by angular_speed*dt/2. source_delta_interval_enclosure transports this whole interval through a source loop prefix and a fixed source-reference inverse; the interval must occupy one loop cell (right seam allowed). Turning-fixture intermediate regressions check separate point-enclosure midpoints; the uniform proof comes from the analytic speed caps, not sampling. Animation library: 170 tests passed. Bounds are conservative and require key/loop partitioning; automatic splitting, uniform source-candidate discrepancy, STEP handling and collision admission remain incomplete. Ordinary App moving fades remain disabled.

### Source motion key/cycle partition operation (2026-10-04)

RootRigidCurve::source_delta_partition unions translation/rotation key clocks and loop seams, limits work/storage, and emits continuous source-motion enclosures relative to a common reference. A represented loop boundary is admitted only when exact dyadic cycle/phase matches its authored key/seam; unrepresentable boundaries reject. The turning fixture partitions [0.5,2.5] into [0.5,1], [1,2], [2,2.5] and rejects insufficient capacity. Animation library: 170 tests passed. Separate interior-channel key regressions and nonrepresentable-boundary regressions remain to be added; automatic source-candidate comparison and collision admission remain incomplete. Ordinary App moving fades remain disabled.

### Separate-channel and lost-boundary partition regressions (2026-10-04)

Added an accepted clip with a translation key at 1/4 and a rotation key at 1/2. The partition emits all three cells in both the initial and following cycles. A separate exact f32 key at 2^-60 becomes unrepresentable after shifting by one unit-duration cycle; the operation returns RootRigidBudget instead of losing that interior key through deduplication. NaN reference clocks reject even on empty intervals. Animation library: 171 tests passed. Broad source-candidate comparison and physical collision admission remain unfinished; ordinary App moving fades remain disabled.

### Uniform source/canonical screw geometric discrepancy (2026-10-04)

RootRigidCurve::source_screw_point_error compares the hull of partitioned source point images with the canonical screw reference whole-path point box over matching durations. Outward independent hull subtraction bounds every geometric point discrepancy, with axis caps and L1 radius. A deliberate doubled-speed candidate regression verifies nonzero discrepancy coverage and duration mismatch rejection. Animation library: 171 tests passed. This first bound is intentionally conservative: synchronized interval refinement is needed for useful tolerances. Runtime evaluation, world/f32 publication, blended source fade fields and physical collision admission remain separate obligations. Ordinary App moving fades remain disabled.

### Synchronized source/reference discrepancy refinement (2026-10-04)

Canonical screw caches now enclose point boxes over stored-time corridors, including stationary gaps and endpoint holds. Source/candidate comparison maps each source cell affinely into candidate time using outward exact-duration arithmetic and supports bounded uniform subdivision. The existing comparison uses one subdivision per source cell. A matching linear-reference regression verifies that 16 subdivisions shrink the radius by more than fourfold; deliberate mismatched velocity remains covered. Animation library: 171 tests passed. This qualifies geometric discrepancy for a single source clip versus a canonical reference; runtime/world/f32 errors, fade fields and physical admission remain incomplete. Ordinary App moving fades remain disabled.

### Budgeted borrowed source/reference geometric certificate (2026-10-04)

certify_source_screw_point_error doubles synchronized subdivisions until the supplied radius tolerance is proved, counting all comparison cells across attempts. RootSourceScrewPointCertificate borrows both immutable source and prepared reference owners and retains the domain, point box, extraction axes, axis/radius bounds, subdivision count and work count. Insufficient capacity rejects without relaxing tolerance. Regression certifies a matching linear path within radius 0.2, confirms owner identity, and rejects a doubled-speed path at that tolerance. Animation library: 171 tests passed. This is a geometric single-clip proof; runtime evaluation/world/f32 errors, blended source fields and physical collision admission are not included. Ordinary App moving fades remain disabled.

### Fixed-frame geometric error and direct f32 rounding (2026-10-04)

RootSourceScrewPointCertificate::mapped_f32_geometric_error rotates/scales its uniform local axis errors into a fixed common frame and adds a uniform direct-f32 conversion bound from the mapped candidate whole-path box. Similarity image supports enclosed signed scale products. Regression uses a reflected scale and frame origin 100,000,000, requiring the coarse f32 spacing to remain covered. Animation library: 171 tests passed. This covers geometric discrepancy and rounding of exact mapped coordinates only. Runtime matrix/quaternion evaluation before conversion, scene ownership/admission and blended source fades remain incomplete. Ordinary App moving fades remain disabled.

### Stored similarity arithmetic rounding bounds (2026-10-04)

RootRigidEnclosure::stored_similarity_evaluation_error mirrors local glam 0.33.7 DQuat::mul_vec3: w²-b·b, p·b, b×p and ordered vector sums, followed by scale and offset. RoundedRange propagates uniform basic-operation errors over stored-input boxes. The reference is the exact homogeneous quaternion polynomial of the stored inputs, not independently normalized real source rotation. An independent Fraction verifier validates six components, including lost unit offset at 1e16, with no tolerance. Animation library: 172 tests passed. Quaternion/source compilation discrepancy, transcendental pose evaluation, full consumer assembly and collision admission remain separate obligations. Ordinary App moving fades remain disabled.

### Uniform relative quaternion runtime composition (2026-10-04)

RootRotationCurve::cubic_relative_interval_evaluation_error_bounds combines local cubic runtime/source error and origin normalization error through actual ordered Hamilton multiplication and DQuat normalization. The uniform product discrepancy must remain <=1/4, giving a raw norm [3/4,5/4]; dot/sqrt/reciprocal/multiply rounding are bounded analytically. Regression compares 33 pointwise proofs for a nonidentity origin to the uniform cap and checks excessive-error rejection. Animation library: 173 tests passed. Uniform cycle powers, extraction/pose publication assembly, source fade fields and collision consumers remain incomplete. Ordinary App moving fades remain disabled.

### Uniform runtime loop quaternion discrepancy (2026-10-04)

RootRotationCurve::cubic_cycle_interval_evaluation_error_bounds carries fixed cycle-power error and local uniform cubic error through the actual sample order: raw power*local, origin multiplication, final normalization. A separate raw Hamilton uniform rounding bound avoids inventing intermediate normalization. Regression covers 33 phases at cycles 0,3,7 and rejects unrepresentable cycle indices. Animation library: 173 tests passed. Explicit cycle/phase input remains separate from wall-clock policy; rigid translation/pivot and cycle-prefix runtime error, publication assembly, fade fields and collision admission remain incomplete. Ordinary App moving fades remain disabled.

### Coupled local phase runtime point evaluation (2026-10-04)

RootRigidCurve::phase_point_evaluation_error_bounds propagates source translation/runtime coefficient error and relative cubic quaternion error through origin addition, extraction adjustment, moving/bind pivot rotation, final point rotation and translation. Shared RoundedRange quaternion-vector arithmetic mirrors local glam ordering. The half-turn midpoint fixture checks exact point image (5/2,0,1) and a finite axis cap below 1e-9. Source continuous interval admission now explicitly rejects nonzero translation STEP jumps at its right endpoint; a regression covers this previously missing guard. Animation library: 173 tests passed. Runtime rigid loop prefix evaluation, world/f32 assembly, blended sources and collision admission remain incomplete. Ordinary App moving fades remain disabled.

### Runtime rigid prefix and composed point discrepancy (2026-10-04)

cycle_prefix_evaluation_error_bounds compares the fixed actual rigid binary power to its exact-source enclosure, retaining translation and quaternion errors. cycle_point_evaluation_error_bounds propagates those caps and local phase errors through actual rigid translation composition, quaternion multiplication/normalization and final point mapping. Uniform normalization rounding now uses a shared scalar-discrepancy helper instead of an invented identity multiplication. Analytic midpoint regressions cover cycles 0,1,4 with finite axis caps below 1e-8. Animation library: 173 tests passed. Explicit cycle/local-phase domains remain separate from wall-clock selection and root reference transport; nondegenerate interval independent validation, world/f32 assembly, fade fields and collision admission remain unfinished. Ordinary App moving fades remain disabled.

### Independent nondegenerate runtime interval validation (2026-10-04)

The turning fixture now emits actual point coordinates over [1/4,3/4] at cycles 0,1,4 under a single uniform runtime cap per cycle. The independent Fraction verifier derives source point coordinates from s=3u²-2u³, d=s²+(1-s)²: x=u+2-((1-s)²-s²)/d and z=2s(1-s)/d; odd cycles apply (5-x,-z). All 153 components are covered without tolerance. These finite regressions validate the selected fixture; whole-interval coverage relies on the outward arithmetic proof, not the samples. Animation library: 173 tests passed. World mapping/publication assembly, general fade sources and physical admission remain incomplete. Ordinary App moving fades remain disabled.

### Source-to-published fixed-frame point error assembly (2026-10-04)

cycle_world_point_evaluation_error_bounds combines local source/runtime point error with an enclosed fixed source frame/scale and actual stored mapping parameters. RoundedRange propagates frame quaternion/translation/scale discrepancies and actual mapping arithmetic; the final f32 rounding domain is expanded by the pre-cast runtime error. Independent Fraction validation covers 153 published components with a half-turn parent frame, scale -1/2 and origin 100,000,000. Animation library: 173 tests passed. This proves the documented explicit-cycle sample-point-map-cast operation, not the editor body/frame/contact publication path or a blended source fade. Root-reference transport, wall-clock domain ownership and physical consumers remain unconnected. Ordinary App moving fades remain disabled.

### Absolute clock publication error domain (2026-10-04)

world_point_evaluation_error_bounds partitions absolute clocks at exact represented source keys and seams, refines unresolved quaternion norm corridors, and aggregates fixed-frame sample/map/f32 error caps. Exact seams additionally certify the actual next-cycle/phase-zero runtime branch; checking only the preceding phase-duration branch would miss different rounding. The cubic uniform evaluator now reports unresolved nonpositive norm bounds as InvalidRootRotationCurve before sqrt, allowing bounded refinement instead of a misleading NumericalOverflow. Regression spans [1/4,11/4], validates 18 published components including seams with Fraction arithmetic, and rejects insufficient capacity. Animation library: 173 tests passed. Physics/editor trajectories are relative to accepted body/source-reference frames; this absolute sample proof is not yet interchangeable with that path. Scene ownership, source fade fields and collision admission remain incomplete. Ordinary App moving fades remain disabled.

### Single body-local frame ownership in owner fade admission (2026-10-04)

Owner-bound fade preparation already maps source fields through the retained common reference into body-local coordinates. OwnerFadeAdmission now contains collision-axis/error policy only; AdmittedOwnerFade::request fixes identity basis, zero origin and unit scale, preventing a second caller-provided coordinate transform. Lower-level explicit frame requests remain available for separately bound model plans. The signed-scale/transport regression verifies the emitted request uses identity mapping while the source scale is preserved in preparation. Editor build passed; runtime 20, playback 8, foot placement 17 tests passed. Automatic publication error policy/source fade numerical proofs and ordinary App dispatch remain incomplete; ordinary App moving fades remain disabled.

### Exact source spatial field domains (2026-10-04)

source_phase_twist_enclosure bounds the exact authored spatial field on one continuous translation/cubic-rotation key interval. For source pose (t,Q), it uses v=t'−omega×t, the proved source-origin speed cap, angular speed cap and outward L1 translation radius. Symmetric component boxes preserve validity but intentionally do not assert an angular-axis constraint. RootRigidFieldInterval::from_source_phase retimes this field and applies the existing fixed enclosed signed similarity adjoint before fade blending. The exact half-turn fixture has t=(5/2,0,2), t'=(13,0,0), omega=(0,6,0), v=(1,0,15) at u=1/2; signed scale −2, half-turn Y frame, X offset 10, doubled clock speed and equal blend with a frozen source give v=(2,0,90), omega=(0,6,0). All 173 animation tests pass. These are source-field domains, not a proof of the integrated blended trajectory, runtime publication or physical admission. Loop-prefix assembly, tighter componentwise angular bounds, numerical source-to-candidate fade discrepancy and editor dispatch remain incomplete. Ordinary App moving fades remain disabled.

### Original-clock source field assembly (2026-10-04)

source_delta_twist_enclosure shares exact key/seam partition construction with source_delta_partition and maps each local source field through the fixed reference inverse and exact-source loop prefix. It aggregates complete field boxes, retaining clamp completion tails as zero velocity. A point domain retains the source derivative; from_source_domain supplies zero retiming for paused playback. RootRigidFieldInterval::from_source_domain retimes the complete original-clock hull once and applies the fixed common signed similarity, so rounded retimed cuts cannot omit an intermediate source field. This intentionally sacrifices local tightness and does not certify field derivatives. Regressions cover multiple half-turn loops, distinct translation/rotation keys, pause, clamp tails, insufficient budget, lost tiny keys and translation STEP rejection. Source-field hulls still require source-to-integrated-fade discrepancy and body/publication qualification before physical admission. Ordinary App moving fades remain disabled.

### Uniform fade material-point certificate (2026-10-04)

RootRigidFadePointCertificate borrows the immutable canonical screw cache and every complete fade-field interval. It rejects gaps, reordered or missing fields, mismatched clocks, invalid point boxes and insufficient capacity. The existing bounded-field error accumulator compares each whole velocity domain against its canonical frozen twist and prefix, without assuming derivative caps or sampled extrema. For a material-point box of L1 radius r, the uniform Euclidean discrepancy is origin_error + min(2, angular_error)*r; it covers every prefix for equal identity initial poses. Playback/body/publication errors remain separate. Regression verifies exact linear discrepancy 1, retained prefix coupling after translation then rotation, and immutable owner identity.

The exact-source linear field now uses algebraic cancellation before interval arithmetic: for t=a−Q*b, v=p'−Q*b'−omega×a. Selected axes have a=p_relative+bind and b'=0; unselected axes have a=p_absolute and b'=p'. This replaces the initial symmetric scalar linear cap described above and retains signed source translation derivatives. The original linear-source fixture connects from_source_domain through fade blending to the new point certificate: a matching canonical trajectory has error below 1e−9, while a trajectory with doubled speed has error at least 1. All 174 animation tests pass. Angular source boxes remain conservative scalar caps; adaptive source-fade construction, componentwise angular bounds and physical/editor admission remain incomplete. Ordinary App moving fades remain disabled.

### Componentwise exact-source angular fields (2026-10-04)

cubic_source_angular_velocity_bounds evaluates the source controls widened by their compilation errors and uses omega=2*vec(raw' * conjugate(raw))/|raw|². De Casteljau value/derivative evaluation is shared with existing compiled cubic enclosures. Bounded subdivision proves a positive norm throughout each domain; unresolved/singular domains reject before division. Both angular APIs share key-domain admission. The fixed right origin rotation cancels from spatial angular velocity. source_phase_twist_enclosure now uses these componentwise angular bounds instead of the earlier symmetric speed box, preserving exact zero axes. The turning fixture proves Y-coordinate velocity remains zero and the midpoint linear/angular component widths are below 1e−10. An independent Fraction verifier checks 51 angular components of raw(u)=(u,u²,u³,1) at 17 dyadic phases against rational formulas without tolerance, including nonzero cross-product terms. All 175 animation tests pass. These finite checks validate the fixture; uniform coverage comes from outward interval arithmetic. Adaptive source-fade trajectory construction, runtime publication and editor physical admission remain incomplete. Ordinary App moving fades remain disabled.

### Adaptive authored-source fade construction (2026-10-04)

RootRigidSourceField borrows an immutable source curve and fixes its reference, full original clock, extraction axes, enclosed common frame and signed scale. Queries first enclose the exact global affine clock, split that enclosing clip corridor at source keys/seams, and retain the full-domain retiming factor. Recomputing speed from rounded query endpoint differences would change playback rate; this assembly avoids that error. integrate_sources feeds complete source/target blend field boxes into the existing WholeField integrator, refining stored wall domains under a fixed maximum span capacity until both requested error tolerances are met. It reuses one global error/prefix accumulator and stores the same source fields for point and coordinate certificates. No source derivative estimate or sample-based extrema are substituted.

Regressions cover frozen-to-linear weight ramp (exact displacement 1/2), source speed 1 to target speed 2 across a target loop (exact displacement 3/2), and the authored half-turn/pivot cubic (endpoint point image 4X, uniform point budget 0.3, exact zero height discrepancy). Insufficient capacity rejects without relaxing tolerance. All 175 animation tests pass. This integrates the documented continuous global fade interval in fixed common frames. In-tick fade completion policy, source/body snapshot binding, runtime publication errors and editor/physical dispatch remain incomplete. Ordinary App moving fades remain disabled.

### Original-source fade completion within a tick (2026-10-04)

integrate_sources_with_completion cuts exactly at the stored fade endpoint and continues with the original target field. Clip clocks remain mapped over the full wall interval, while weights use the shorter fade interval; neither clock nor initial weight restarts at subdivision cuts. One canonical trajectory and error accumulator span both phases. Refinement allocates the fixed global span capacity across fade and tail, preserving the exact completion cut. Ending weights other than 1, invalid fade durations and insufficient capacity reject. integrate_sources delegates to this implementation with no completion tail.

The two-clock fixture has source speed 1 and target speed 2, fade end 1/4 and tick end 1. The exact displacement is 15/8 for weights [0,1] and 31/16 for weights [1/2,1]. Both lie inside the returned origin error; full point certificates remain below 0.01 and accepted_wall_time returns the exact completion cut 1/4. All 175 animation tests pass. Body/source snapshot binding, actual runtime/f32 publication errors and editor physical dispatch remain incomplete. Ordinary App moving fades remain disabled.

### Animator and asset snapshot binding for original-source fades (2026-10-04)

RootRigidFadePlan retains extraction axes with its private pre-tick Animator. integrate_original_sources_common_similarity derives curves, original phases, full-tick speed, active weights and completion directly from that immutable snapshot. Mutable diagnostic path/factor/weight fields do not redirect the source compiler. Signed common similarity and interrupted frozen sources are covered by the regression; changed animator speed fails existing exact snapshot admission. All 176 animation tests pass.

PreparedModelFade::bind_original_sources_common_similarity returns the existing asset-bound PreparedModelFadeMotion, sharing owner identity, immutable motion receipts, accepted playback and contact preparation. The imported-model fixture replaces its animation with a known cubic-identity root rotation and linear X source, then exercises the existing physical transaction. Palette preparation failure restores the scene/body state, a separately prepared identical motion cannot consume the receipt, and successful acceptance moves the body exactly 1/16 and advances only the bound playback. Model playback 11, runtime 20 and foot placement 17 tests pass. This lower-level transaction fixture supplies zero publication margin conditionally; it does not qualify general runtime/f32/world publication errors. The ordinary owner/scene compiler still uses the prior compiled-path binding. General LINEAR root rotation, source/body numeric publication policy and ordinary App dispatch remain incomplete. Ordinary App moving fades remain disabled.

### Exact authored constant-rotation support (2026-10-04)

The source compiler now supports held root orientation in noncubic channels and absent rotation channels. Constancy is proved from pairwise proportionality of the original f32 quaternion components, independent of rounded cached logs. Products of finite f32 components have at most 48 significant bits and remain normal finite f64, so the cross-product equality test is exact. Antipodal quaternion representatives describe the same constant orientation. This proof is intentionally not applied to keyed cubics with potentially moving tangents.

A proved constant source rotation relative to its authored origin is identity and has zero angular velocity. The source field preserves selected translation derivatives and sets unselected axes exactly to zero before interval arithmetic. Phase validation still rejects out-of-domain queries. Regression covers nonidentity antipodal keys, multiple loops, masked axes, affine-clock fade assembly, and a LINEAR key perturbation of 1e−20 that must not be treated as held. All 177 animation tests pass. Moving LINEAR source rotation still rejects rather than inheriting cached log/angle approximations. Ordinary owner dispatch and general publication error qualification remain incomplete; ordinary App moving fades remain disabled.

### Original LINEAR source angular velocity (2026-10-04)

Retained f32 quaternion keys now supply outward short-arc angular velocity bounds without cached logs or platform atan/atan2. The angle enclosure uses the alternating arctangent series with an explicit remainder and a rational Machin enclosure for pi. Independent exact-rational checks cover 18 components, including tiny angles, half-turns, antipodal choices and noncommuting keys. All 178 animation tests pass. Moving LINEAR source pose/fade support and general publication qualification remain incomplete; ordinary App moving fades remain disabled.

### Original LINEAR source phase pose (2026-10-04)

linear_relative_source_phase_rotation_bounds encloses source-key normalization, the short-arc exponential and composition with the inverse authored source origin. source_phase_enclosure now accepts moving LINEAR poses. Sine and cosine use alternating Taylor series through 32 terms with explicit first-omitted-term remainders on angles at most 2 radians; no platform trigonometric result is used as proof. Source fractions use outward affine arithmetic inside a proved key cell. The independent rational verifier covers 120 quaternion components across six source key pairs and five fractions, including tiny angles, half-turns, negative hemisphere and noncommuting rotations. All 179 animation tests pass. Whole-interval moving LINEAR source fields still require the speed-cap and componentwise-field paths to be connected; ordinary App moving fades and general runtime publication qualification remain incomplete.

### Whole-interval original LINEAR fields and source fades (2026-10-04)

The shared source angular-field selector now admits moving LINEAR channels alongside cubic channels. Their speed cap is the outward L1 norm of the certified source angular field; cubic retains its Bernstein cap. Source phase interval enclosures, pivot-aware spatial velocity, loop-prefix adjoints and original-clock fade integration use these shared operations. Moving STEP channels remain unsupported.

The exact half-turn fixture rotates around bind pivot 2X with angular velocity pi*Y and spatial linear velocity 2*pi*Z. Both local and two-loop field hulls contain independent rational Machin pi bounds for all 12 components. A source fade from clock [0,1] to [1/2,3/2], completing at wall time 1/4, preserves the same pivot field and ends at translation 4X. Its uniform material-point certificate for box [-1,1]^3 is 3.4601442930920067e-13. The fixed capacity rejection and exact completion receipt cut are checked. All 180 animation tests pass. These source trajectory certificates do not qualify general runtime/f32/world publication or ordinary App dispatch; ordinary App moving fades remain disabled.

### Editor owner preparation selects original source compiler (2026-10-04)

prepare_owner_fade now calls bind_original_sources_common_similarity. The asset/Animator snapshot, owner identity, accepted-motion receipt and transactional publication pipeline are retained. The unused cached-path bind_common_similarity wrapper was removed. All 20 editor runtime tests and both physical receipt tests pass; they cover scene batches, rollback, signed root-reference transport and planted-foot preservation.

This is owner preparation integration, not unconditional production admission. The editor root-reference initialization still encloses its evaluated stored factor rather than qualifying source-to-cached factor discrepancy. CharacterCertifiedFadeMotion still accepts a caller-provided evaluation radius. General source/reference numeric error, canonical evaluation and f32 body/world publication margins require separate proof; ordinary App moving fades remain disabled.

### Source phase reference anchoring (2026-10-04)

Animator::root_rigid_source_phase_factor_enclosure encloses the original source factor at the stored current clip phase and rejects anchoring an active blend. ModelPlayback::reference_at_current_phase uses this source enclosure before signed scaling and inverse composition. Ordinary unsupported source channels retain the previous evaluated-factor fallback; moving unsupported channels cannot enter original-source fade integration. Supported LINEAR/cubic phase anchors no longer treat a rounded cache factor as an exact source pose.

The regression retains a nonzero authored turn of 1e-20 at half phase, composes its source inverse back to a box containing the identity action, preserves the Animator clock and rejects an active blend. The cached factor also retains the small turn; this change supplies a source enclosure rather than repairing a lost cached turn. All 181 animation tests and 20 editor runtime tests pass, including physical receipt acceptance, signed reference transport and planted sole preservation. Source anchoring at the stored phase does not qualify runtime phase arithmetic, canonical path evaluation, or f32 body/world publication. General numeric admission and ordinary App moving fade dispatch remain incomplete.

### Canonical evaluated poses in physical sweep (2026-10-04)

RootScrewEnclosurePath::sample_evaluated selects a finite stored pose from the outward canonical prefix/increment enclosure, with explicit actual quaternion normalization. It returns that pose and its source enclosure so the existing enclosed_point_evaluation_error operation can bound the actual computed point discrepancy. Invalid fractions, missing spans, nonfinite or vanishing midpoint quaternions reject.

The enclosed physical sweep uses this same evaluator for intermediate proposals, partial accepted motion and complete endpoint motion. It no longer samples rounded screw-prefix caches or path.end for a nonempty enclosed canonical path. Cache/path owner identity remains checked. The regression deliberately corrupts stored rounded prefix/end translations, then verifies canonical selection and point-error coverage at five fractions. Point bounds in this fixture are below 1e-10. All 182 animation tests, 61 gameplay tests and 20 editor runtime tests pass, including wall stops, floor contact, in-tick fade completion, transactional acceptance and planted soles.

This connects evaluated local proposals to their canonical point enclosure. It does not yet construct a uniform automatic numeric margin for the actual body/world mapping, orientation/edge update or f32 publication. Caller-supplied evaluation margins remain conditional proof obligations; ordinary App moving fades remain disabled.

### Physical publication admission without a pose callback (2026-10-04)

The post-snap physical pose and composed f32 scene shape are now certified against every obstacle for every requested rigid trajectory, regardless of whether a preparation callback exists. The previous prepare.is_some gate allowed unprepared trajectories to bypass these checks. Exact dyadic separation still permits true touching and rejects overlapping interiors before scene/body/input publication.

Ordinary unenclosed zero-clearance trajectories now use an early-stop contact reserve of half f32 epsilon times max(1, anchor L-infinity magnitude plus body edge radius). Advancement subtracts that reserve and the stopping threshold includes it. This is candidate-selection policy, not a proof of uniform numeric error. Explicit clearance/enclosed queries retain their supplied numeric-envelope contract without a second reserve; every physical rigid tick still undergoes final actual-pose admission.

The regression at coordinate 65536 independently proves that a precisely touching center rounds to f32 center 65536.015625, which places the body's right face inside the wall. The previous f32 lattice center 65536.0078125 has disjoint interiors. Ordinary no-callback movement now selects and publishes a safe earlier candidate, consumes input once, and remains clipped by the wall. The preparation regression likewise obtains a safe candidate, then deliberately fails its callback to verify full rollback before retrying a smaller successful motion. All 61 library, 46 integration and 20 editor runtime tests pass. General uniform body/world numeric error and ordinary App moving-fade dispatch remain unqualified; ordinary App moving fades remain disabled.

### Accepted canonical proposal world error (2026-10-04)

The enclosed rigid approximation sweep now compares all eight canonical world corner enclosures against the exact affine box defined by its actual stored proposed center and rotated edges. gap::world_pose_error uses directed interval corner sums/subtractions, rejects invalid boxes and nonfinite poses, and returns componentwise maxima plus a directed L1 radius. Affine corner interpolation extends these bounds to every material point of the body.

The resulting private optional metric is carried through PathHit into AppliedCharacterTrajectoryMotion::proposal_evaluation_error_bounds. It belongs to the accepted canonical sweep proposal, before grounding, relocation and f32 scene publication. Ordinary unenclosed paths report None rather than a manufactured zero bound. The source-fade transaction integration test requires the metric to be present, finite, internally consistent and below 1e-8 in its fixture. An independent exact-rational affine-shift fixture at world coordinate 65536 checks all three axes and the exact 7/8 L1 displacement with no tolerance. All 62 library, 46 integration and 20 editor runtime tests pass.

This is a per-pose numerical discrepancy, not a uniform trajectory bound and not a post-snap/published-scene certificate. It must not replace CharacterCertifiedFadeMotion's whole-interval numeric proof obligation. General uniform body/world numerical admission and ordinary App moving-fade dispatch remain incomplete; ordinary App moving fades remain disabled.

### Accepted published world-pose discrepancy (2026-10-04)

The canonical accepted prefix now retains an immutable world-corner enclosure through the physical receipt. After grounding, relocation and composed f32 scene publication, the controller compares that same witness against the displayed center and affine edges. AppliedCharacterTrajectoryMotion::published_pose_evaluation_error_bounds exposes componentwise and L1 bounds for every material point of the published body. The estimate is available before the preparation callback; unsupported unenclosed paths retain None. Actual collision admission remains independent and mandatory.

The large-coordinate dyadic fixture distinguishes the negligible proposal error from a published X displacement of exactly 1/512, then adds a 1/8 ground relocation. This is a per-accepted-prefix discrepancy, not a uniform trajectory margin. It does not close the automatic numeric-envelope obligation or enable ordinary App moving fades.

Validation: all 63 gameplay library, 46 gameplay integration and 20 editor runtime tests pass. The independent exact-rational verifier checks three axis caps and the L1 cap against the exact 1/512 displacement without tolerance. Formatting and diff checks pass. Evidence is retained in artifacts/rig-published-world-pose-error-2026-10-04/.

### Uniform stored quaternion normalization rounding (2026-10-04)

RootRigidEnclosure::stored_quaternion_normalization_error_bounds now bounds glam 0.33.7 DQuat::normalize over an entire supplied stored-input box. The inspected implementation evaluates four squared products, three left-associated additions, sqrt, reciprocal and component multiplication. Directed nominal intervals plus separate operation-rounding caps propagate the error through positive norm and reciprocal sensitivity bounds. This estimates rounding relative to exact normalization of each stored quaternion, without charging the motion variation within the input box as a numeric error.

Singular/ambiguous-zero boxes, nonfinite values and unprovable finite operation ranges reject. The regression uses a nontrivial input box with component sign changes and a positive fourth-component norm floor; four stored quaternions, including a 1e-20 component, are checked independently using exact rational 384-bit square-root brackets. This primitive does not yet bound canonical-enclosure midpoint selection, actor composition, runtime phase arithmetic or the whole physical trajectory. Ordinary App moving fades remain disabled.

Validation: 183 animation library tests pass; all 16 independently checked normalized components lie within their caps against rational square-root brackets, without tolerance. Formatting and diff checks pass. Evidence: artifacts/rig-stored-quaternion-normalization-2026-10-04/.

### Shared normalization rounding in source composition (2026-10-04)

quaternion_normalized_composition_uniform_error now calls the same operation-level normalization rounding helper as stored_quaternion_normalization_error_bounds. Its raw product discrepancy supplies a private relational squared-norm proof via [1-discrepancy, 1+discrepancy]. That proof is retained even when all component boxes contain zero. The helper accounts for the actual dot/sqrt/reciprocal/multiply sequence; a separate normalization sensitivity term carries the source-to-raw-product discrepancy.

This replaces the duplicated coarse normalization rounding formula in source composition. The regression checks sign-changing unit-source products, monotonic growth for perturbed input bounds, and rejection above the established raw-discrepancy limit. Independent exact rational Hamilton products qualify 12 stored result components without tolerance. This is composition error propagation, not qualification of canonical midpoint selection, world/body mapping or a complete physical-trajectory numeric envelope. Ordinary App moving fades remain disabled.

Validation: 184 animation, 63 gameplay library, 46 gameplay integration and 20 editor runtime tests pass. Formatting and diff checks pass. Evidence: artifacts/rig-shared-normalization-rounding-2026-10-04/.

### Frozen canonical midpoint evaluation with component errors (2026-10-04)

RootRigidEvaluatedPose retains the selected stored transform, canonical source enclosure, and translation/quaternion component discrepancies in private immutable fields. RootRigidEnclosure::evaluate_midpoint computes the midpoint and actual quaternion normalization once, then directed differences compare the exact stored result against every component in the source enclosure. RootScrewEnclosurePath::sample_evaluated_with_errors selects through the borrowed canonical cache; the existing physical sample_evaluated interface delegates to that same implementation.

This preserves the physical proposal while making source selection/normalization discrepancy available for downstream body-frame error propagation. The poisoned cached-prefix regression checks identical selected transforms, finite small component caps and the existing point discrepancy across five fractions. Two independent normalized-source fixtures check selected quaternion components against exact-rational square-root brackets. The bounds belong to their sampled canonical enclosures; they are not a whole-trajectory numeric certificate. General world/body propagation and phase arithmetic remain unfinished, and ordinary App moving fades remain disabled.

Validation: 185 animation, 63 gameplay library, 46 gameplay integration and 20 editor runtime tests pass. Independent rational checks cover eight normalized-source quaternion components without tolerance. Formatting and diff checks pass. Evidence: artifacts/rig-midpoint-component-error-2026-10-04/.

### Evaluated-pose point-box and fixed world publication error (2026-10-04)

RootRigidEvaluatedPose::point_evaluation_error_bounds propagates the frozen source-to-stored pose discrepancy through the actual glam f64 quaternion point rotation and offset for every stored point in a supplied coordinate box. mapped_f32_point_error_bounds carries that local error through a fixed source/actual world frame, signed scale, offset and direct f32 conversion. Both use the existing operation-level RoundedRange propagation; invalid point boxes, nonfinite scale and unprovable finite evaluation reject.

The exact fixture combines a Y half-turn, a cyclic-axis world frame, signed scale -2, a body box with nonzero extents and world coordinate 65536+5/512. All eight stored corners are compared independently against rational dyadic world coordinates, checking each axis and L1 cap without tolerance. Bounds cover every point in the box by interval algebra. The APIs qualify the stated local/map/direct-cast operation sequence at one frozen canonical pose; composed scene matrices, grounding and trajectory-time uncertainty still require their own qualification. Ordinary App moving fades remain disabled.

Validation: 186 animation library tests pass. Independent exact-rational checks pass for 24 axis and eight L1 discrepancies. Formatting and diff checks pass. Evidence: artifacts/rig-evaluated-point-box-world-error-2026-10-04/.

### Uniform translation-path midpoint selection error (2026-10-04)

RootScrewEnclosurePath::translation_selection_error_bounds supplies local translation component caps over every fraction of every translation-only screw span, including cached endpoint selections. Nonzero angular rates return None. The private EnclosureFamily arithmetic stores two different quantities: an endpoint domain across the time family and an upper bound on each member enclosure's diameter. Variable exact fractions have domain [0,1] and width zero. Directed addition/product width propagation includes four adjacent spacings for outward endpoint rounding; midpoint selection adds four spacings for halving/addition. Fixed prepared prefix widths are retained.

Consequently the cap does not charge the physical path excursion as numerical error. The two-span regression traverses hundreds of units and reverses direction, while its uniform local selection caps remain below 1e-9. Per-pose component caps at endpoints and interior fractions lie within the whole-path caps; exact rational references qualify the computed coordinates, including stored fractions such as 0.1 and 0.3 that expose actual coordinate rounding. This is the translation-only stage of temporal selection qualification, not a substitute for angular trajectories or world/body/clock/scene publication error. Ordinary App moving fades remain disabled.

Validation: 187 animation tests pass. Independent exact-rational verification passes 42 component comparisons, including 12 nonzero observed coordinate-rounding discrepancies. The fixture uniform caps are [7.032152637975749e-13, 4.662936703425663e-15, 3.851086116668516e-16]. Formatting and diff checks pass. Evidence: artifacts/rig-uniform-translation-selection-2026-10-04/.

### Uniform angular screw-path selection error (2026-10-04)

RootScrewEnclosurePath::selection_error_bounds now propagates pointwise enclosure diameters over every fraction of each prepared angular screw span. EnclosureArithmetic provides a shared expression tree for Scalar and EnclosureFamily: cross products, point rotation, Hamilton composition and the SE(3) increment's alternating series use exactly the same operation order. Family division, square, signed subtraction and explicit symmetric Taylor remainder bounds retain endpoint domains independently of per-member width. Fixed canonical prefix enclosures are included, and midpoint discrepancy is carried through the shared real-unit normalization sensitivity/rounding calculation.

The new regression combines moving Y and X screw spans with noncommuting prefix composition, endpoints and interior stored fractions. Whole-path local translation/quaternion caps enclose each sampled component discrepancy and remain below 1e-9 in the fixture. The independent verifier evaluates SE(3) increments and Hamilton products using exact rational Taylor bounds, checking 84 components without tolerance. Existing scalar operation behavior is preserved by the shared implementation, and the prior translation-only API remains available. This closes local temporal midpoint/normalization selection for the supported prepared screw domain; uniform actual physical body/world mapping, runtime phase arithmetic and scene publication still require qualification. Ordinary App moving fades remain disabled.

Validation: 188 animation, 63 gameplay library, 46 gameplay integration and 20 editor runtime tests pass. Independent rational checks cover 84 components with explicit Taylor remainder intervals and no tolerance. Formatting and diff checks pass. Evidence: artifacts/rig-uniform-angular-selection-2026-10-04/.

### Uniform screw-path point-box and fixed world publication error (2026-10-04)

RootScrewEnclosurePath::point_selection_error_bounds carries its uniform translation/quaternion selection caps through the inspected runtime quaternion point polynomial and translation. It evaluates operation-error domains over every prepared prefix and whole-span pose enclosure, while temporal motion remains nominal interval variation rather than an error term. The result covers every stored point in the supplied box over every fraction of the same immutable canonical screw path.

mapped_f32_point_selection_error_bounds propagates that local error through a fixed source/actual world frame, signed scale, offset and direct f32 conversion. Invalid point boxes, nonfinite actual scale and unprovable finite domains reject. This qualifies the explicit local point/map/direct-cast sequence, not the different physical controller orientation/edge update, composed scene matrix, grounding, upstream source-field approximation or runtime phase arithmetic.

The regression uses noncommuting moving Y/X screw spans, a body box, cyclic-axis frame, scale -2 and world coordinate 65536+5/512. Uniform local error stays below 1e-9 and covers the independently obtained per-pose point caps. The rational verifier evaluates the real screw exponentials, transforms all eight corners at five fractions of each span, and checks 240 axis plus 80 L1 errors without tolerance. All 189 animation tests, formatting and diff checks pass. Evidence: artifacts/rig-uniform-world-point-error-2026-10-04/. Ordinary App moving fades remain disabled pending actual physical/controller/publication qualification.

### Uniform actual physical orientation error (2026-10-04)

RootScrewEnclosurePath::physical_rotation_selection_error_bounds follows the controller's actual rotation arithmetic. It rotates the selected quaternion's imaginary vector through the fixed stored basis using the cross form, retains the scalar component, normalizes, then multiplies by the fixed actor orientation and normalizes again. Source bases/orientations are the exact real normalizations of those same stored inputs. Directed operation-error propagation, the existing uniform canonical selection cap, raw Hamilton composition error and the shared normalization sensitivity/rounding bound qualify the whole prepared path.

The controller's structural coordinate-row replacement is exact for the real normalization of the same stored basis; it retains only input-vector error, already covered by the conservative cross-form bound. The regression calls the actual gameplay reframe_rotation function, not a substitute evaluator, over noncommuting moving Y/X screw spans and a cyclic-axis basis. The independent rational verifier computes source quaternion exponentials, basis conjugation and actor multiplication, checking 48 actual result components without tolerance.

This qualifies the rotation chain itself. Center displacement, separately rotated affine edges, grounding/relocation, composed f32 scene matrices and runtime phase arithmetic remain separate obligations. The bound is not yet an automatic production request margin, and ordinary App moving fades remain disabled. Evidence: artifacts/rig-physical-rotation-uniform-error-2026-10-04/.

Validation: 64 gameplay library and 46 gameplay integration tests pass. Independent rational checks cover 48 actual controller quaternion components without tolerance. Formatting and diff checks pass.

### Uniform actual pre-grounding physical body error (2026-10-04)

RootScrewEnclosurePath::physical_body_selection_error_bounds follows the stored controller body arithmetic over every canonical screw fraction: basis/pivot displacement, actor rotation, center addition, normalized body-orientation update, independently rotated rest edges and affine corner sums. The bound covers every material point by affine corner interpolation. Source body/frame inputs are the exact real normalizations of the same stored actor/basis inputs; signed translation scale and the stored pivot are retained.

The shared rotation helper now returns both reframe and body-orientation errors. Coordinate-row overrides require an additional geometric normalization cap: the real normalization of a stored quaternion within raw L1 discrepancy d of a canonical unit source differs by at most 2d/(1-d). This component inflation covers both the ordinary cross polynomial and its exact structural-row branch, with no near-zero geometry tolerance. Directed operation errors propagate through center and edge evaluation. Invalid frames, nonfinite inputs, zero scale or unproved positive normalization domains reject.

The regression calls actual gameplay reframe_rotation and rotate_vector across moving noncommuting Y/X screw spans, a nonzero pivot, signed scale -2, unequal body extents and world coordinate 65536+5/512. Independent rational SE(3) exponentials and quaternion products transform all eight corners at five fractions per span, checking 240 axis and 80 L1 discrepancies without tolerance. This qualifies the body before grounding and relocation. Input clocks, source-field approximation, grounding and composed f32 scene publication remain separate obligations; ordinary App moving fades remain disabled. Evidence: artifacts/rig-physical-whole-body-uniform-error-2026-10-04/.

Validation: 65 gameplay library and 46 gameplay integration tests pass. Independent rational checks pass for 240 axis and 80 L1 actual corner discrepancies. Formatting and diff checks pass.

### Uniform post-snap reconstruction error (2026-10-04)

RootScrewEnclosurePath::physical_post_snap_selection_error_bounds extends the pre-grounding physical-body bound through the stored snap-vector multiplication, center addition and current zero-anchor AABB reconstruction. It covers every selected stored snap fraction in [0,1]. The snap displacement is treated as physical intent; the bound qualifies numerical error relative to that selected displacement, not collision fraction selection or composed f32 scene publication.

The gameplay regression calls the actual relocate and precise_center functions at five screw fractions and four snap fractions. Independent rational screw exponentials plus the exact selected stored displacement verify 60 axis and 20 L1 center discrepancies without tolerance. Gameplay validation passes 66 library and 46 integration tests. Evidence: artifacts/rig-post-snap-relocation-uniform-error-2026-10-04/.
