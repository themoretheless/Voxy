# Streamed exact coordinate support

GPU contact packing discovers nonzero coordinate IDs by streaming each column contiguously. A dense-first-column fast path and the existing simple scan for fewer than four columns avoid unnecessary masks. Shape/finite preflight, ascending coordinate ordering and exact-zero semantics remain unchanged; signed zero is excluded, finite nonzero values of any magnitude remain included. No packing coefficients, shader arithmetic, physical tolerance or publication gate changed.

The captured original 61-row x 2646-coordinate operator measured 296.705 to 40.115 microseconds median for sparse support discovery, paired 12 x 64 calls. The same operator with a dense first column measured 6.206 to 2.416 microseconds. These include allocation and identity assertions and are concurrent CPU diagnostics, not full packing, physics or rendered FPS benchmarks.

Eleven codec/shader/support tests passed. Real Metal original physical admission passed for 21 systems at 1e-14 tolerance with zero native fallback. Four cooperative coordinate operators matched serial output exactly, with original physical checks retained.

The separately running full 720-frame bounded-readback experiment uses its frozen pre-streaming binary and does not qualify this patch. Full trajectory and >160 FPS remain unproven. A workgroup-cache shader candidate was measured separately and rejected for slower timings; production equality shader is unchanged.
