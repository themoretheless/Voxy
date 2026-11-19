# ADR 0005: `physics` is split into `physics`, `physics_bio` and `physics_astro`

Status: accepted; recorded after the fact.
Date: 2026-11-19 (split made in `4f18c0c3`, 2026-11-11)

## Context

The wave 3 refactoring plan (now in git history, `docs/refactor/PLAN.md` item
2.3) made the module layers of `physics` acyclic and concluded that splitting
the crate was not needed yet: the middle of the crate is tightly coupled, and
the first candidate would be `hair`, decided by `cargo build --timings`. Commit
`4f18c0c3` split the crate the same day anyway, along a different line:
biomechanics, skin, tissue, lymph, osmotic, circulation and respiration went to
`physics_bio`, astrophysics to `physics_astro`, and `hair` stayed in `physics`.
No measurement or rationale was written down.

## Decision

Keep the split. `physics_bio` and `physics_astro` depend only on `physics`; the
reverse edge is forbidden by `crates/voxy_depgraph/tests/depgraph.rs`, and the
module layers left in `physics` are checked by `tests/physics_modules.rs`.

## Measurement

Measured 2026-11-19 on Apple M4 Max in a clean worktree at `aaf0e480` with its
own target directory, dev profile (`physics*` at `opt-level = 1`). Another
session was compiling and running tests on the same machine, so absolute times
are inflated and grew from round to round; the ordering held in every round.

| Crate | Source files | Lines |
| --- | --- | --- |
| `physics` | 256 | 103 465 |
| `physics_bio` | 72 | 33 556 |
| `physics_astro` | 21 | 8 038 |

Cold `cargo build -p physics -p physics_bio -p physics_astro --timings`: 59.6 s
wall. `physics` took 55.1 s; `physics_bio` started at 15.3 s, on the pipelined
metadata of `physics`, and took 35.7 s; `physics_astro` started at the same
time and took 4.9 s. Both dependents compile in parallel with the code
generation of `physics`, so the split costs no extra wall time on a cold build.

Incremental rebuild after appending a comment to one crate's `lib.rs`, three
rounds:

| Edited crate | Built | Wall time |
| --- | --- | --- |
| `physics_bio` | `-p physics_bio` | 11.9 s, 19.7 s, 28.4 s |
| `physics` | `-p physics_bio` | 41.5 s, 81.2 s, 74.0 s |
| `physics_astro` | `-p physics_astro` | 5.5 s, 5.9 s, 8.6 s |

An edit in biomechanics or astrophysics rebuilds only its own crate, two to
four times faster than an edit in `physics`, which also rebuilds every
dependent. Before the split every one of these edits cost at least the
`physics` row.

## Consequences

- Work on biomechanics and astrophysics gets the faster loop above; work on
  `hair`, liquids and contact does not. Splitting `hair` remains open and needs
  its own measurement.
- The A/B pairs required for performance decisions (`docs/PLAN.md`, rule 6) were
  not possible with the machine under load; this measurement supports the
  decision qualitatively, not as a performance claim.
