# Coupled finite substrate and solution vapor exchange

`exchange_solution_vapor_with_substrate` composes contact half-step, shared vapor full-step and contact half-step. It stages film, borrowed substrate energies, cell velocities and shared vapor before publication. Contact uses the existing exact pair heat kernel; vapor uses the existing adaptive solution integrator and symmetric shared-reservoir sweeps. It does not duplicate their physical laws or storage owners.

Tests cover hot substrate driving more evaporation than an insulated control, total mass and sensible+latent+kinetic energy, momentum, nonvolatile residue, late vapor rejection rolling back prior substrate heating, and temporal refinement against 256 subdivisions. Thermodynamic parameters are synthetic, not calibrated water/oil. Complete solvent exhaustion, dry-surface nucleation, spatial solid heat diffusion and interactive demo integration remain unfinished. Inspect report/log for actual test results.
