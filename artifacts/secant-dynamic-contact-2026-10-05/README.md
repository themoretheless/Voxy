# Shared secant history for admitted midpoint dynamics

The existing static Body L-BFGS recursion and positive-curvature admission now
share private algebra with implicit midpoint dynamics. The dynamics solver
learns curvature from accepted free-node displacement/residual pairs, retains
at most 12 pairs per solve, and applies its inertia + contact-normal inverse
as the initial metric. Invalid/non-descent directions discard history and use
the original positive metric. It does not change force laws, 96-iteration
limits, 48-trial line search, residual/impulse-work budgets, swept contact or
independently checked final energy. Solver history is local and never published
as physical state. Static equilibrium remains transactional.

Independent oracle: the two-loop inverse agrees with explicitly assembled dense
BFGS matrix updates; negative/zero/nonfinite curvature and unbounded reciprocal
are rejected, and memory truncation is tested. A 100 MPa supported tetrahedron
passes its strict 1e-10 J independent midpoint work check and swept admission.

Final physics checks: 92 unit tests passed, 3 manual benchmarks ignored;
8 static L-BFGS, 28 prescribed surface contact, 11 tissue and 8 viscoelastic
dynamics tests passed. TissueDemo integration: 20 passed.

The pre-secant f64 imported provider rejected step 65 (331.06 s trace run).
The secant implementation passed all 68 steps (158.54 s), including independent
end-of-run energy receipts. These timings are diagnostic, not a controlled
performance claim. A separate experiment retrying the original metric after
failed secant line search rejected the full runtime at step 65 and was removed.
The final full render committed steps 1 through 130 and rejected step 131 (t=0.545833333 s) with nonlinear nonconvergence; it did not complete 480 steps.
No full clip, physiological calibration, CUDA or broad hardware qualification
is implied by the 68-step result.

Primary method reference: [Liu and Nocedal, On the limited memory BFGS method
for large scale optimization](https://users.iems.northwestern.edu/~nocedal/Abstracts/limited-memory.html).
Existing repository implementation and RAG archive evidence were inspected
before extracting the shared algebra; no external optimizer code was copied.

## One pose for both prescribed boundaries

`imported_contact_sample64` now supplies support palettes and collision surfaces
from one immutable Pose64, shared by the example runtime and its CPU gates.
Initial-motion verification passed; singular/nonfinite reference matrices,
wrong joint count, empty domains, changed source-face order and invalid phase
reject before publishing a tissue step. Complete state rollback is checked.
The shared-provider 68-step gate passed again (152.82 s). Its regression is enabled by default. The full 480-step CPU gate remains explicitly ignored pending qualification. The pre-provider-refactor render is recorded separately.


`contact-prefix.gif` contains the 11 accepted rendered samples at t=0 through
0.5 s. It is a prefix, not a completed clip. Four blue FEM regions remain
illustrative attachments, not integrated anatomical skin. Rendering used
Apple M4 Max / Metal; physics remains CPU.
