# Coupled contact search direction

The implicit local-displacement solver now computes its search direction with bounded diagonally preconditioned conjugate gradients on the positive inertia plus frozen-feature normal operator. Full rank-one contact blocks retain coupling between axes and body vertices; pinned degrees of freedom are eliminated. This is a search metric, not the full material/geometric Hessian. Actual nonlinear line search, continuous collision checks, independent work admission and transactional publication remain unchanged. Nonfinite or non-descent directions fall back to the previous diagonal direction.

An independent Sherman-Morrison inverse regression checks a rotated rank-one block with unequal masses, multiple participating vertices and a pinned vertex: passed. All 21 prescribed-contact, 8 viscoelastic and 18 tissue tests passed.

The expected-success imported regression remains RED at step 52, now with implicit contact quadrature nonconvergence. The previous fatal line-search rejection is absent at that step. Diagnostic initial nearest gap is 4.202982433e-8 m; the final 16-panel work defect is 1.239428077e-6 J versus 3.0517578125e-10 J. A loose-budget diagnostic trial is explicitly not published. Full imported contact animation remains unqualified.

Next: locally adaptive path quadrature with independently measured work error; do not loosen global admission. All changes are local and unpushed.
