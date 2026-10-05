# Thermal film in the existing impact demo

The existing `LiquidDemo::new_impacts` now owns `ThermalFilmMixture` instead of
an independently mutable nonthermal mixture. It uses the existing chronological
lifecycle with atomic thermal capture and preserves the whole-frame transaction.
Captured sensible heat is credited to receiving cells. The existing shear
spreading path transports energy with the same donor-limited volume/species flux.
Rendering reads the underlying film through the thermal owner's read-only access.

The native acceptance routine compares stored film energy against cumulative
captured sensible heat. Reset returns both to zero. No viscous heating or shear
work is added to this sensible heat ledger. The demo has not yet connected the
finite-vapor interface; this migration does not demonstrate drying.

The thermal lifecycle's 29-test run passed before demo migration. All four app checks passed; one manual timing test was intentionally
ignored. `app-tests.log` records the 35.53-second test run. The GPU snapshot passed on Apple M4 Max / Metal (`gpu.log`). The four
frames contained 0, 0, 161 and 232 rendered film cells. The PNG was visually
inspected and matches the previous nonthermal impact snapshot byte for byte.
This checks that adding heat ownership did not alter this rendered scene; it
does not demonstrate evaporation or prove behavior on other hardware. Source hashes and the live session handle are in report.json.

Command: `cargo test -p voxy_app --lib liquid_demo::tests`.

Changes remain local. Full hardware support and the complete engine goal remain
unverified and unfinished.
