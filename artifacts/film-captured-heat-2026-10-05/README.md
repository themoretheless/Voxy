# Per-cell captured thermal inventory

The existing chronological droplet lifecycle now returns the receiving triangle
and removed thermal inventory for every captured droplet. None explicitly means
transport was disabled. The inventory includes any latent energy; it cannot be
silently treated as sensible heat if the incoming liquid has phase change enabled.

The original aggregate capture ledger and liquid/film publication transaction
remain in place. The new data identifies where heat must be credited before
migrating the interactive impact demo to the thermal film owner.

Regression coverage includes the aggregate ledger of existing impact tests and
an independent two-drop fixture: 300 K and 400 K droplets deposit on different
triangles, each carries mass * cp * T, and the total equals energy removed from
liquid. These tests are currently running; no pass result is claimed yet.

Command:
`cargo test -p physics --test liquid_impact_events --test liquid_droplet_lifecycle`

This change supplies a ledger, not completed thermal demo integration. No heat
has yet been credited to the interactive demo's nonthermal film. Changes are local.

Ledger validation completed: all 32 tests passed (`ledger-tests.log`).
The later thermal lifecycle wrapper uses the same event engine, credits captured
sensible heat to film cells and commits both owners together. It rejects latent
phase/gas input and inconsistent component heat capacities. Its new regression
checks per-cell temperatures and late-event-budget rollback. This extension
compiled; its test process is still running. The interactive app still uses
the nonthermal lifecycle until that extension is verified and migrated.

Thermal lifecycle validation completed: all 29 tests passed
(`thermal-capture-tests.log`). The later interactive demo migration is now in
source and its app test run is building/running; see
`../native-thermal-film-2026-10-05`. Earlier notes above describe the previous
nonthermal demo snapshot, not the new source state.
