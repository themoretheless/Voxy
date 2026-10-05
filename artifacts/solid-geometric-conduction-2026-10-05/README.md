# Geometric solid conduction qualification

13 focused release tests passed. Shared face conductance uses actual deformed
area and normal centroid distances, with heterogeneous half-cell resistances
in series. Analytic octahedral-sector links, heterogeneous two-cell conductance
and isotropic deformation scaling are independently checked.

Centroid distances use relative coordinates to avoid unnecessary cancellation
from absolute translation; face normal length uses hypot to avoid squaring
overflow. The mechanical owner admits geometry. Shared-face collapse or same-side
centroids are rejected by the contact builder.

This monotone network is not a qualified arbitrary-mesh continuum discretization:
non-orthogonal consistency correction and automatic demo integration remain.
