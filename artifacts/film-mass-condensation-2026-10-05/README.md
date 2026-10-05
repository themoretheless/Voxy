# Condensation through the species mass boundary

Both evaporation and condensation now send species kilograms through the
`FilmMixture` owner. `FilmTransfer` describes actual representable transfer in
either direction; `FilmWithdrawal` remains a compatibility type alias.

Mass deposits stage every cell and species before committing. Repeated cells
share the staged inventory. Sub-resolution species additions report zero rather
than requested mass; species growth without representable bulk growth is an
error. Conversion overflow, underflow, invalid schema and invalid cells cannot
publish partial changes. The coupled vapor adapter retains its staged energy,
momentum, mass and residue balance checks before committing either owner.

This does not yet make kilograms the internal canonical inventory. The current
film still stores component volumes with one constant density. The interface
change removes density conversion from condensation callers, so internal mass
storage and EOS-derived cell geometry can be migrated at the owning boundary.
IAPWS-95, variable density and temperature-dependent mechanical response are
not enabled by this change.

Required checks:

```sh
cargo test -p physics --test surface_film_withdrawal --test surface_film_vapor
```

See `report.json` and retained logs for actual execution results. Local changes
are uncommitted and unpushed.
