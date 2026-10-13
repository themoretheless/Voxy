# Rejected per-invocation QR vector cache

Each lane owns 30 paired values (indices lane + 64*k); original operation order, barriers and wider-input storage fallback remain. Actual Metal/M4 Max execution passed the original 174-row, 14-system, 1764-coordinate physical operator at 1e-14 with zero native fallback. Nine alternating timed pairs of the selected 39-row active equality preserved entire GPU storage bitwise. Uncollapsed widths 1919/1920/1921/4096 and QR positive tiny pivot/dependent-column gates passed.

Baseline median initial QR 5.570083 ms; candidate 6.331959 ms (1.137 times baseline), zero faster pairs. Reject. No production shader change. This eliminates a second current-vector cache strategy on this measured adapter/operator; it does not establish universal hardware behavior. Concurrent load uncontrolled; no full trajectory, complete solver speed or rendered FPS claim.
