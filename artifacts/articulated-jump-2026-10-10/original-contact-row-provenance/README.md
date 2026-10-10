# Original constraint row provenance

Opt-in VQC snapshots now preserve original constraint row ordering, each of four entries' global rod index, point index, gradient f64 bits and mobility f64 bits, plus original bound bits. Original bit patterns are JSON integers; no float formatting loses signed zero or adjacent f64 values. Global row indices identify rows within this snapshot only, not persistent contact features across frames. Physical quantity is still unspecified.

This changes observation only. The live full-density trajectory is not restarted and uses the earlier binary. Selected transverse-separation capture: three two-rod operators; independent Python reconstruction from metadata recovers every original load byte, including the fixed-root exclusion. See row-load-audit.json. Newton/island regressions are recorded separately. No whole-scene trajectory or FPS claim.

`tools/audit_hair_island_rows.py` independently validates 19 captured snapshots and rejects 11 deliberately malformed sidecars. It reconstructs raw non-root load bytes; fixed-root gradients, mobilities and original pre-free bounds are not independently present in VQC1. The initial negative-test attempt wrote an unchanged gradient bit pattern; the corrected corruption flips one original bit and is rejected. This is audit validation, not physical trajectory qualification.
