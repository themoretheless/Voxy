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
