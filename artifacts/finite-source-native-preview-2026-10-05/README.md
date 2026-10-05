# Native finite-source preview

The existing native LiquidDemo and liquid_snapshot now have a finite-source mode.
Two point emitters use the mass/energy/recoil transaction; orange source markers
follow explicit source drift. Restart retains the mode. The targeted test proves
emission, recoil, reserve consumption, mesh creation, restart and rollback after
a late second-emitter error. It passes.

Three GPU states were rendered on Apple M4 Max / Metal, exit 0, and the resulting
preview was inspected. Source hashes describe final code; the image was rendered
before a restart-only fix, which leaves render and source dynamics unchanged.
Illustrative mass/geometry does not establish calibrated physical jet breakup.
