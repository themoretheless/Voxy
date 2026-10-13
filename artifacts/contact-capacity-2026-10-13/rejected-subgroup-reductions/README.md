# Subgroup shuffle reduction experiment — not retained in executable source

Tested replacing the five shared-memory/workgroup-barrier steps within each 32-lane reduction with subgroupShuffleDown, preserving the exact binary addition order. Global 256-lane reductions retain the first three shared-memory steps. The experimental shader checked actual subgroup width, invocation mapping and subgroup index at runtime and rejected incompatible mappings before solving. SUBGROUP capability was explicitly requested.

18 regular renderer checks and the actual GPU comparison passed. A batch contains two original 91-row x 882-coordinate operators plus an intervening scalar operator. Both cold and seeded cases were alternated against baseline four times each. Every complete output byte, including QR workspace and all independent headers, matched baseline. All original coordinate inequalities and active gaps passed. This is coordinate transport evidence, not native physical admission.

The elapsed GPU pass samples do not show a robust benefit: cold results are around 30-33 ms for both variants; seeded samples are noisy, including outliers in both variants. Other full-cycle GPU processes remain active, so this is not an isolated benchmark. No speedup, FPS or full-model completion is claimed.

The experiment's source snapshots and logs are retained here. The executable source changes, helper files and subgroup API/tests were removed to avoid adding an unproven optimization or an extra hardware requirement. The previous shared-memory reduction implementation remains the working default. Next investigation should measure arithmetic and occupancy costs rather than assuming barrier transport dominates.

## Restored working source verification

Pre-experiment shader, codec and test source hashes match the previously recorded artifacts. All 17 regular triple checks and actual GPU seeded-original and heterogeneous-batch regressions pass after restoration. Previous uncommitted work was preserved.
