# Midpoint impulse/drift precision and independent path work

A loose diagnostic energy budget previously also made nonlinear residual admission arbitrarily loose. The residual threshold is now capped independently: norm <= min(energy_tolerance,1e-8)*1e-4. A regression proves a 1e100 diagnostic energy budget retains the same actual solved positions/velocities as 1e-8 for the fixture, including nonzero contact impulse.

Free velocities now use v_new=v_old-dt*(gradient_mid/m-acceleration), rather than dividing a difference of nearby world coordinates by a tiny dt. The endpoint drift is reconstructed as x_old+dt*(v_old+v_new)/2, then the actual corrected endpoint is checked for continuous contact and volume safety and independently admitted energy. Prescribed pin velocities keep their specified endpoint-motion definition.

The new analytic tiny-step fixture uses dt=1e-10 s. Drift is below world-coordinate f64 resolution, but the nonzero acceleration impulse remains representable and is retained. This catches the old zero-velocity result. 19 prescribed contact and 8 viscoelastic tests passed.

Independent read-only contact work sampling uses eight Simpson subintervals along the uncommitted actual trial. Before the impulse fix, measured contact midpoint quadrature error was 1.2264e-12 J, versus Simpson error 1.1518e-16 J; the overall implicit defect was larger. These measurements do not establish a complete root cause.

The corrected actual imported Metal run still rejects frame 52 (0.216666667 s), with `implicit contact work defect`. Five corrected prefix frames are diagnostics, not a qualified complete animation. Energy tolerance, masks and refinement limit remain unchanged. The precision defects are fixed and tested independently; they are not claimed to have solved the imported fixture. Follow-up corrected path-work measurements are recorded separately when available.

Corrected measurements: rejected midpoint mechanical defect 3.1391979299394857e-10 J; independent contact midpoint path-work error 3.1394504956656585e-10 J. Their difference is about 2.526e-14 J. Eight-subinterval Simpson contact error is 4.443782174916658e-15 J. Actual nearest residual gap is 13.1701 micrometres with 5.6517% pinned feature weight. Corrected subdivisions recover quadratic convergence: total defects 7.850314483889576e-11 J (2) and 1.9625008945957417e-11 J (4). For this trial, midpoint contact quadrature explains almost all remaining work defect. Full tissue regression: 18 passed, 44.66 seconds.

Next use consistently path-averaged contact forces and obstacle/support work in the nonlinear solve. Replacing only the reported actuator work would break the force/energy relationship; do not infer work from energy or change admission tolerance.
