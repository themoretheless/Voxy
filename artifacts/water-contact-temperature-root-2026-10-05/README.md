# Direct temperature coordinate for water contact

The profiled contact path previously evaluated an equilibrium energy inversion
inside each scalar heat-root trial. The new formulation solves directly in
water temperature: q(T)=m*u(T,rho)-U_old, with residual
q(T)/(G*dt) - ((E_reservoir-q(T))/C_reservoir-T).
It keeps the same implicit contact law and staged energy publication. Every
root trial uses one forward EOS equilibrium query. The common monotone bracket
solver is shared with legacy latent-phase heat conduction; adjacent-float and
exact-zero termination avoid repeating an identical trial.

Required validation: water_equilibrium, liquid_phase, liquid_transport and
implicit_heat_tests. Existing heat-law, conservation, no-overshoot, rollback and
unchanged temporal-convergence gates apply. A faster debug fixture is not a
production benchmark or evidence of all-device support.

Status and source hashes are in report.json. The current running volume-work
test process predates these contact/root changes and cannot validate them.
