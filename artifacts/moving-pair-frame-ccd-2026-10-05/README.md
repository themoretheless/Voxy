# Moving-pair-frame conservative advancement

CCD interpolates both triangles after subtracting the first body vertex at each authored endpoint. This is the same relative linear trajectory with its rigid common translation removed. The speed bound is computed in that relative frame. Original world endpoint distances remain independently guarded; the first sampled rejection time is preserved for quadrature refinement rather than prematurely returning an endpoint-only rejection.

An analytic binary-coordinate test certifies a constant open gap of 2^-40 m above the separation floor while both triangles translate by 2^20 m in each axis. A closed authored endpoint remains rejected, and existing swept crossing/time-localization regressions pass. 88 library tests pass (3 manual benchmarks ignored), 24 prescribed-contact, 8 viscoelastic, 52 related contact/geometry and 19 tissue tests pass. git diff --check passes.

The full imported render still rejects step 65 at 0.270833333 s, now with closed surface contact gap. Only six prefix frames exist; there is no qualified complete contact clip. Local solver geometry, force-integrated work and actual committed world endpoints still require consistency qualification. No admission budget or law coefficient changed. All changes remain local and uncommitted.
