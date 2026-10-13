# Strength of materials (сопромат) validation suite

Status: 2026-10-13, plan step E. `crates/physics/tests/sopromat.rs` checks the
small-strain solvers against closed-form results. Numbers below are the
printed values from `cargo test -p physics --test sopromat -- --nocapture`
(E = 10 MPa, ν = 0.3, bar 1 m × 0.1 m × 0.1 m unless stated).

| Case | Solver | Reference | Coarse error | Fine error | Bound |
| --- | --- | --- | --- | --- | --- |
| Cantilever, tip load | T10 (`QuadraticBody`), 4 → 16 hexes along the length | Euler–Bernoulli + Timoshenko shear, κ = 5/6 | 5.7 % | 1.8 % | 5 % |
| Simply supported, uniform load | T10, 4 → 16 | 5qL⁴/384EI + qL²/8κGA | 3.6 % | 0.18 % | 5 % |
| Three-point bending | T10, 4 → 16 | PL³/48EI + PL/4κGA | 4.6 % | 0.65 % | 5 % |
| Square shaft torsion | T10, section 1 → 2 hexes across (length 4 hexes) | φ = TL/GJ, J = 0.1406 a⁴ | 15.2 % | 3.9 % | 10 % |
| Uniaxial tension | linear tets (`Body`), 4×2×2 | δ = PL/EA, lateral −ν ε | < 1e-5 | — | 1e-5 |
| Displacement-controlled J2 coupon (E = 210 GPa, σ_y = 250 MPa, H = 1 GPa) | single linear tet | elastic σ = Eε, post-yield slope EH/(E+H), onset at σ_y | slope error 1e-11 | — | 1 % |

Observations:
- Linear tetrahedra lock in bending: with two elements through the thickness
  the cantilever deflection is 29 % of the reference and 65 % with three;
  `tests/beam_accuracy.rs` already documents the monotone (slow) convergence.
  The suite therefore uses the quadratic body for bending and torsion.
- Torsional stiffness converges with the section discretization (warping),
  not with the number of elements along the length: 16 elements along the bar
  with one across give the same 15 % as 4 elements.
- The quadratic body's 512-node budget bounds the meshes; a finer torsion
  section would need the node limit raised.

Not covered yet: plates (clamped circular/square), buckling, combined loading,
and stress-field checks on the quadratic body (quadrature point positions are
not exposed, so only deflections are compared there).
