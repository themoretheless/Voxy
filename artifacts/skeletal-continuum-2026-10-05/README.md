# Neutral skeleton with viscoelastic continuum tissues

The existing nine-joint shared skeleton drives the attachment nodes of four
rounded FEM regions. Free nodes retain velocity-Verlet inertia. Objective
Ogden-Maxwell memory relaxes exponentially in the reference frame; released
energy is accumulated as a heat receipt. It does not update temperature.

The anatomical material labels select existing illustrative dimensions only.
Material parameters are equilibrium shear 5 kPa, bulk 1 MPa, one Maxwell branch
10 kPa / 0.2 seconds, and density 1000 kg/m³. These are not calibrated human tissue
or anatomical geometry. No absolute-velocity drag is added to settle the bodies.

Four initial subdivisions per 240 Hz pose interval are adaptively refined after
an energy/path rejection. An entire display frame, including clock and energy
receipts, rolls back if any body or substep fails. Render vertices are embedded
in the solved physical boundary.

`smooth-frames`, `poses-smooth.png` and `motion-smooth.gif` are the delivered
version with smooth area-weighted deformed boundary normals, interpolated through
the existing embedding. Position geometry is unchanged.
`final-frames` and `poses-final.png` preserve the invariant-based exponent-two
constitutive implementation and four-subdivision integration before smooth lighting.
`frames` and `poses.png` preserve the initial spectral / sixteen-subdivision run.
`initial-runtime.sample` is a one-second process sample of that initial render,
not a controlled performance benchmark.

Qualification status and source hashes are recorded in report.json after all
owned processes finish. This increment does not complete the engine parity,
hardware/CUDA, engine research, destruction or multiphase-fluid objective.
