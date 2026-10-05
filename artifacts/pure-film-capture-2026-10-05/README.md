# Unresolved pure-film capture and actual-volume receipts

The reproduction returned success for absorption of a 1e-30 kg particle into a
filled pure film, claiming 1e-33 m³ deposited although the film volume did not
change. The captured particle was removed. The same integration fixture also
tests a normal particle followed by an unresolved particle in the same batch.

`SurfaceFilm::deposit_batch` now rejects positive additions that cannot increase
the stored cell volume or mass, before publishing any cell. Zero requests still
validate cell indices and preserve inventory. Receipts sum actual representable
volume increments rather than requested volume. Late failure preserves the
entire batch and the incident liquid state.

Mass-underflow admission is stronger: an incoming nonzero volume whose mass
cannot be represented is rejected at raw deposition, before such invalid stock
can enter a FilmMixture. The old constructor-underflow fixture was updated to
assert this earlier rejection and complete preservation of the raw film.

Pure SurfaceFilm still stores volume at constant density; this change does not
enable variable-density mechanics. Execution results and hashes are in the
report. No native graphics or hardware qualification is implied.

The first guarded run passed 50 tests but failed the new mass-underflow check:
the singleton `deposit` still bypassed batch validation. Singleton deposition
and prescribed source-rate integration now delegate to the same atomic batch
boundary. The receipt/rollback fixture exercises all three public entrypoints.
Two pre-existing ignored tests (expensive fold convergence and a release-only
gravity-cache benchmark) are not part of the current executed gate.

The unified production source passed capture and mass-underflow checks. Three
atomic-frame/contact assertions still compared receipts with nominal source
rates. For example, actual growth was 1.0000000000001413e-11 m³ while the nominal
request was 1.0000000000000001e-11 m³. These assertions now require exact equality
with the representable inventory increment. Rollback, exact geometry comparison,
total-mass bounds and the 200-frame contact conservation gate are unchanged.
Only the two affected targets are rerun; production source is unchanged.

Final executed gates passed 51 tests: capture 11, pure film 25, atomic contact 3
and transfer 12. The latter two source-receipt targets passed after updating the
receipt contract; production source was identical across the final runs. Two
existing ignored studies remain unexecuted. Variable-density EOS and pressure
work/energy coupling are still unfinished; the complete engine goal is active.
