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
