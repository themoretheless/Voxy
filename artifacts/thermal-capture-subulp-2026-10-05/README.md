# Preserve sub-resolution droplets during thermal capture

A regression deposits a 1e-30 kg droplet into an already wet film. The receiving
component volumes and sensible energy cannot increment at that scale. The
previous implementation returned success and committed particle removal.
`repro-failure.log` records the failed rollback expectation before the fix.

Captured mixture admission now requires an actual bulk/component inventory
increase in every receiving cell, and a representable energy increase whenever
positive heat is supplied. Invalid or overflowing captured heat also rejects.
The surrounding lifecycle transaction preserves the liquid and thermal film on
failure. No compensating sub-resolution inventory buffer is implemented yet;
these transfers are rejected rather than silently lost.

All 35 lifecycle, thermal film and vapor tests passed (`tests.log`); Cargo
exited successfully. The native impact app regression is now being rerun to
check ordinary jet deposition after this admission change. The regression compares both complete owner
states after rejection. Source hashes and the live handle are in report.json.

Command:
`cargo test -p physics --test liquid_droplet_lifecycle --test surface_film_thermal --test surface_film_vapor`

This is a numerical admission fix, not complete drying or production physics
qualification. Changes remain local.
