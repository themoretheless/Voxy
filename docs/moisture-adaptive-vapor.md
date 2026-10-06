# Adaptive thermal moisture exchange

`Body::advance_thermal_vapor_adaptive` wraps the existing conservative
first-order activity exchange with one finite `ThermalVapor` owner. Each trial
freezes saturation capacity, exchanges mass and latent energy, then updates
capacity from temperature. One full trial and two half trials estimate local
error in every material-cell water mass, vapor water mass and vapor temperature.
`ThermalVaporAccuracy` controls relative and absolute tolerances and an attempt
budget. The common step-doubling controller uses the second-power local error
for this first-order law; existing midpoint vapor integration retains its
third-power controller.

All input links are admitted before refinement. Only an accepted complete
elapsed-time trajectory publishes the material and vapor states. Domain,
accuracy-budget and conservation failures preserve both original owners. Final
mass and energy balance checks retain the existing relative conservation gate.

The independent regression integrates the continuous activity/latent-energy ODE
with RK4 (4096 steps) for both an initially wet material drying into an empty
reservoir and an initially dry material receiving condensation. At local mass
tolerances 1e-7, 1e-9 and 1e-11 kg, endpoint mass errors decrease from roughly
7.87e-7 to 7.96e-9 kg for drying, and 3.22e-7 to 3.17e-9 kg for condensation.
The test also checks closed total water, latent energy and temperature, and full
rollback on attempt exhaustion and duplicate links. All 31 moisture/particle/
film-vapor regressions passed; artifacts are in
`artifacts/adaptive-thermal-moisture-2026-10-06`.

These are local numerical tolerances, not a global-error certificate. The law
retains fixed material moisture capacities, constant latent heat and a lumped
thermal vapor reservoir. It does not add droplets, supersaturation condensation,
gas motion, temperature-dependent sorption, completely dry film nucleation,
material calibration or renderer integration.

`advance_thermal_vapor_adaptive_with_receipt` returns `ThermalVaporStep`: the
transfer, fully admitted elapsed seconds, attempted/accepted/rejected interval
counts and minimum accepted interval. Each accepted interval consists of two
half-step solves. The transfer-only API delegates to this same path and retains
its result type. Receipts are returned only after complete time and conservation
admission; rejected partial trajectories publish neither owner nor a receipt.

The 0.5-second regression with strict tolerances admits 1020 intervals in 1025
attempts, rejects 5 attempts and uses a minimum interval of approximately
0.00014986 seconds. Its final state and transfer match the transfer-only API.
An initially lower-domain-boundary reservoir rejects the requested drying
interval without changing either state. A zero-flux case admits the complete
interval in one attempt and reports zero transferred mass/latent energy.
These tests measure numerical work, not physical execution speed.

Adaptive trajectories now prepare the admitted material/vapor link network once.
Each private trial owns its network state and revalidates mutable cell masses,
capacities and aggregate capacity before solving; duplicate/index/conductance
admission stays at preparation. Ordinary calls move their prepared network into
the solver rather than cloning it. No public mutable topology cache was added.

The four-cell opposing-flux regression matches an independent continuous network
ODE to a maximum mass error of 2.23e-9 kg. 21 ordinary moisture tests and one
explicit manual comparison passed. On a 32-cell/100-step local release comparison,
reconstructing the network took 787.458 us and reusing preparation
took 590.708 us, with exact Debug equality of final owners. This is
one microbenchmark, not a scene or general scaling result. That measurement used the dense 32-cell path. The current sparse extension and
its limits are described below.

## Sparse large networks

Body now admits up to 16384 cells and 262144 internal links. Networks of at most
128 cells retain the direct factorization path and its arithmetic. Larger
networks assemble a positive mass-space M-matrix with O(cells + links) storage.
The conservative positive iteration formerly private to Darcy protein transport
is now a shared crate-private numerical core; Darcy retains its external error
messages. No moisture dependency on biomechanical geometry was introduced.

The sparse solve admits only finite operators, nonnegative masses and converged
residual/global balance, with a 20000-iteration budget. Existing saturation and
final material/external mass gates still apply before publication. Failure keeps
the owner unchanged; mass is not redistributed to hide residual error.

The 1024-cell closed-chain regression checks analytic discrete Neumann-mode
attenuation. Additional tests compare heterogeneous open baths with direct
solutions, reject overflowing operators even with zero initial mass, preserve
isolated dry cells, and compare sparse adaptive thermal exchange with a small
direct network. 51 distinct moisture/Darcy tests passed. This is bounded CPU
transport support, not large-world performance, thermodynamic calibration or
complete fluid/renderer/hardware qualification.

Large networks now split into components connected by positive conductance.
Components of at most 128 cells use the direct solve; larger components retain
the sparse solve. Zero-conductance links do not join independent blocks. All
blocks are staged before the existing global conservation and publication gates.
A stiff two-cell pair inside a 129-cell disconnected network previously exhausted
the sparse iteration budget; the regression now matches its analytic solution,
an independent reservoir bath, and atomic rejection of a late overflow. The 25
focused moisture tests pass. Large connected stiff components can still exhaust
the sparse iteration budget.

Connected acyclic components larger than 128 cells now use positive leaf
elimination and forward substitution in O(cells + links) storage and operations.
The Schur update adds positive effective capacities instead of subtracting large
diagonals, retaining the material capacity on stiff edges. Original small blocks
retain their dense compatibility path. Cyclic components retain the bounded
sparse iteration; stiff cyclic networks can still exhaust its iteration budget.
All operator overflow, saturation and global balance gates remain transactional.

A connected 257-cell chain with conductance 10000 previously failed to converge.
It now matches the analytic backward-Euler Neumann mode. A heterogeneous
129-cell branching network with two baths matches independent dense Gaussian
elimination; a periodic cyclic network verifies the sparse fallback. Late bath
overflow preserves the complete owner. 28 focused moisture tests pass. These
checks qualify the numerical transport model, not material calibration or full
liquid mechanics. Evidence: `artifacts/moisture-tree-2026-10-06/`.

Cyclic networks that exhaust the positive Gauss-Seidel budget now retry a
matrix-free, diagonally preconditioned conjugate-gradient solve of the symmetric
activity operator `diag(capacity + bath exchange) + weighted graph Laplacian`.
Positive capacities make this operator SPD. The PCG algorithm follows
[Netlib Templates](https://www.netlib.org/templates/templates.html); it is specific
to this symmetric moisture law and is not applied to arbitrary Darcy transport.

Acceptance recomputes the actual residual, checks global inventory independently,
and checks finite saturation bounds before the existing outer publication gates.
The residual threshold includes an evaluation roundoff estimate with a hard
relative ceiling of 1e-10; the independent inventory threshold remains 1e-12.
A drifting recursive residual triggers a restart. No mass redistribution is used.
The retry is bounded by min(4 * cells, 20000) iterations; extreme conditioning
can still reject atomically. It currently runs after the original 20000 positive
iterations, so these results do not establish real-time performance.

A stiff 257-cell periodic ring previously failed and now matches its analytic
backward-Euler diffusion mode. A heterogeneous 129-cell cyclic graph with baths
matches independent dense Gaussian elimination. The initial heterogeneous PCG
failure and residual diagnosis are retained alongside the final 30 passing
moisture tests in `artifacts/moisture-cyclic-2026-10-06/`. This is numerical
qualification of the calibrated activity model, not complete fluid physics.

Moisture now limits its preliminary positive solve to 256 iterations before the
PCG retry. The shared kernel exposes an internal budget parameter; Darcy keeps
its original 20000-iteration behavior and numerical acceptance thresholds. The
PCG and outer conservation/saturation gates are unchanged.

On this host, the 257-cell stiff periodic-ring workload measured eight repetitions
after one warmup: median 50.721583 ms before versus 0.697375 ms after, with the
same 1.2212453270876722e-15 analytic error, 2.609024107869118e-14 kg mass defect
and recorded checksum. This qualifies this workload only; it does not establish
world-scale or real-time fluid performance. A localized unit-water inventory in
an otherwise dry stiff ring matches the complete discrete Fourier solution and
remains positive and conservative. 58 focused moisture/Darcy tests pass, with
one timing test ignored in normal runs and executed separately. Evidence:
`artifacts/moisture-cyclic-cost-2026-10-06/`.

`FiniteQuadraticDynamics::apply_thermal_moisture_with_cohesion` applies accepted
water and prescribed calibrated temperature fields to bulk inertia/elasticity
and cohesive histories in one transaction. Cell temperatures and face
temperatures are separate explicit inputs; they are not silently averaged.
It delegates to the existing accepted-history migration and wet momentum/work
accounting. Invalid late face migration rolls back all bulk changes as well.

At identical wet inventory and opening, the regression remains one connected
fragment under the cold interface law and splits into two under the calibrated
hot law. Fracture dissipation is preserved; cooling/drying that would heal the
accepted history is rejected with the complete state unchanged. Joint accepted
state/reports match the existing individual operations. 23 focused wet-solid,
cohesive and wear tests pass; evidence is in
`artifacts/joint-thermal-wet-solid-2026-10-06/`. Temperature is prescribed here.
Parameter work is reported explicitly rather than automatically charged to a
thermal owner; this is not thermal expansion or a closed thermomechanical solve.

`advance_heated_vapor_loaded_calibrated` connects the material thermal store
to the joint temperature/water constitutive update and loaded motion. It shares
one staged heat/inventory/motion implementation with the original API, whose
laws remain temperature-independent. Every cell and interface uses the lumped
material temperature after heat and water exchange, before subsequent free-body
kinetic mixing-loss heat deposition. The final post-mixing temperature is also
checked against every calibration domain before publication.

An independent two-store conduction formula checks the temperature feedback
and resulting elastic parameter work at a deformed pose. Out-of-domain laws,
late invalid motion, and a post-mixing temperature-domain violation restore
solid, material water, vapor and material heat together. The same mixing step
commits with a wider valid calibration range. 29 focused wet-solid/vapor/cohesive
tests pass; evidence: `artifacts/calibrated-heated-solid-2026-10-06/`.
This remains first-order lumped splitting with explicit parameter work; there is
no implicit debit of parameter work from heat, thermal expansion, or monolithic
thermomechanical solve.

The accumulated sparse transport, temperature-coupled wet solid and FEM preview
changes were qualified together against the complete release physics suite:
1485 passed, zero failed, 11 ignored. All targets of
voxy_app, voxy_editor and voxy_render also pass release cargo check. Source
digests and logs are retained in
`artifacts/current-physics-qualification-2026-10-06/`. Full module regression and
compilation do not establish editor/game feature parity or cross-hardware proof.

The local research corpus audit verifies 1428 pinned identity records, 500
classified engine repositories including 52 derived engines, and 24 source files
across five selected mechanism manifests. Root/README verification does not
establish architectural review of every engine. The RAG wiki
`voxy-engine-roadmap` (b8cc7f31-f4ef-45a1-9ea8-83eaead6189f, revision 1) describes
the 2026-09-05 checkout at 4fb0f45 and is historical context rather than current
acceptance evidence. The overall requested engine goal remains incomplete.
