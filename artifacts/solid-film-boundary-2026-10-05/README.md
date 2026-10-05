# Solid boundary / film binding

39 focused release tests passed across binding, solid conduction, film thermal,
vapor and viscoelastic inertia targets. Boundary binding uses actual exterior
face ownership, supports negatively oriented reference cells through normalized
topology validation, and derives conductance from area / areal resistance.

Tests check boundary completeness, stale geometry rejection, explicit geometry
refresh preserving mass and heat, closed thermal exchange, incompatible cell or
triangle order, invalid resistance, and overlapping cell rejection.

This is reference-mesh compatibility, not unique runtime object identity.
Mechanical work of the deforming wet surface and automatic mannequin hookup
remain unfinished. No fresh GPU rendering is claimed.
