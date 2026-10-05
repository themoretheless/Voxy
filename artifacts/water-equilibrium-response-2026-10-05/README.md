# Equilibrium water caloric and acoustic response

`water_equilibrium_response` extends the existing state query with fixed-density
cv, isentropic sound speed and pressure responses. Strictly inside the two-phase
region p_rho=0 and p_T=(s_v-s_l)/(v_v-v_l). Saturation branch derivatives use
rho_i'=(p_T-p_T_i)/p_rho_i and s_i'=cv_i/T-p_T_i*rho_i'/rho_i^2.
Differentiating the fixed-volume lever rule gives x_T; then
cv=T*((1-x)*s_l'+x*s_v'+x_T*(s_v-s_l)) and
c^2=T*p_T^2/(rho^2*cv). Outside the mixture, homogeneous response is selected.
At phase endpoints the selected homogeneous branch gives a one-sided response;
no unique derivative across the phase boundary is claimed.

All derivatives come from the same potential. The coexistence solver returns
its already evaluated branch responses internally, so mixture response does
not solve coexistence again or re-evaluate both saturated branch potentials.
The existing public coexistence and equilibrium-state APIs retain their outputs.

Fixtures compare cv against a fixed-density energy derivative and c^2 against
a fixed-entropy pressure/density derivative at three mixtures and one vapor
state. Prior pressure/state/coexistence/heat/volume-work gates remain required.

This assumes instantaneous phase/thermal/mechanical equilibrium. It does not
model frozen composition, interphase slip or finite-relaxation acoustics, and
is not yet connected to particle mechanics/CFL. Exact critical/ice and global
stable-fluid domain qualification remain unfinished.
Primary assumption reference: https://onlinelibrary.wiley.com/doi/full/10.1155/2018/3087051
The derivative formulas above are derived from the shared EOS thermodynamic
identities and tested numerically; the cited paper is not a validation of this
implementation or its fixtures.

Initial validation failed the pressure-temperature finite-difference gate at
T=300 K, rho=0.1 kg/m3: 207.90640426957907 vs 207.90345997879604 Pa/K.
The diagnostic is archived. Phase-equality tolerances have been strengthened
from 1e-4 Pa + 1e-8 relative to 1e-6 Pa + 1e-12 relative, and chemical equality
from 1e-10 R T to 1e-12 R T. Table 8 residual gates are strengthened accordingly.
The derivative gate remains 1e-5 relative with the original 0.001 K step.
This is a proposed numerical consistency fix; consult the report for executed
results, rather than treating the changed tolerances as proof of correctness.

The stricter equality solve initially returned NumericalFailure on a Table 8
state with log-density coordinates. At rho=999.887406 kg/m3, one native density
ULP is 1.1368683772161603e-13; one log-coordinate increment maps through exp to
9.094947017729282e-13. The next candidate stores densities directly and retains
relative Newton scaling and positive damped-trial admission. Strict pressure,
chemical and derivative gates are unchanged. Execution status remains separate
from this precision analysis.

The direct-density candidate also failed at 275 K. Scaling the line-search merit
by publication tolerances alone did not fix it: the last residual was
-4.958175509273133e-6 Pa, while the chemical residual was already admissible.
Compensated summation of all six residual-potential channels reduced the pressure
residual to -4.126915314373036e-6 Pa but still failed the unchanged gate.

The current candidate additionally searches at most 64 adjacent representable
liquid densities in either direction when Newton stalls with relative updates
below 1e-10. Every candidate must improve the same tolerance-scaled merit;
publication still requires both original strict equality bounds and distinct,
positive-pressure phases. The Table 8 and unsupported-domain tests passed on
this candidate. Mixture derivative and homogeneous-response results must be
read from the current report; they are not implied by the Table 8 pass.

Final execution passed 17 integration tests (2 coexistence, 9 equilibrium,
6 homogeneous state/response) and both potential-derivative unit checks. The
original 0.001 K finite-difference step and derivative tolerance were retained.
Temporary failure logging was removed after the integration run; the numerical
algorithm was unchanged, and the potential unit checks ran on the final source.
These are CPU f64 checks on local macOS, not GPU/CUDA or complete-domain proof.
