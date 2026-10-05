# Adaptive tissue and prescribed mesh motion

TissueDemo now accepts a combined animation palette and prescribed contact surface sample. Binding an obstacle to all continuum regions is atomic and accounts for the initial contact potential in the energy reference. The outer frame owns positions, velocities, Maxwell state, thermal state, contact pose, receipt and clock publication.

Each initial subdivision receives a linearly interpolated obstacle pose. Rejected trials subdivide both the bone-support targets and obstacle positions; both children remain transactional. Endpoint obstacle owner identity is retained. Mesh actuator work is accumulated separately and included in the existing aggregate external-work receipt. Energy tolerance and refinement limits are unchanged.

Verified: 16 tissue tests passed in 36.96 seconds, including imported two-second rig regression and a new actual-contact work/energy/crossing rollback fixture. A separate frame test passed for all-region binding, obstacle pose publication, wrong-owner rollback and sampling failure after an accepted substep.

This provides the simulation integration interface. The CesiumMan snapshot example has not yet supplied a collision mesh; existing GIFs do not demonstrate these contacts. Material values remain illustrative and contact is frictionless.
