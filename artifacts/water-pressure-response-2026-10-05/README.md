# Homogeneous water pressure response

`water_homogeneous_response` returns the existing state and (dp/d rho)_T,
(dp/d T)_rho from one IAPWS potential evaluation. Negative temperature response
is admitted for water's anomalous-density regime. Coexistence Newton now consumes
the already computed density response instead of repeating the potential pass.

New fixtures compare both pressure derivatives with central finite differences
on cold/dense liquid and vapor states, and verify
c_s^2 = (dp/d rho)_T + T*(dp/d T)_rho^2/(rho^2*cv).
Existing official Table 6/7/8 and equilibrium tests remain required.

Validation targets: water_helmholtz, water_coexistence, water_equilibrium and the
water_helmholtz derivative unit tests. Current source is edited and unverified;
prior contact tests predate these changes. This homogeneous response does not
establish equilibrium two-phase acoustics, full stable-fluid domain admission,
particle-mechanics integration or hardware support.
