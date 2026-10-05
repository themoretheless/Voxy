# Mannequin thermal conduction

11 release app tests passed, including full walk/jump/settle, frame rollback,
and a causal conductivity comparison. A 24-frame comparison with conductivities
0.5 and 5000 W/(m K) changes cell temperatures while retaining bit-identical
mechanical positions and velocities. Both thermal inventories match accumulated
Maxwell heat plus the separate conduction defect within 1e-8 J. Invalid NaN
conductivity preserves every body. The high conductivity is a test stimulus,
not default physical data.

Production demonstration uses 0.5 W/(m K), symmetric thermal endpoint half-steps
and the existing frame transaction. The spatial network remains approximate on
non-orthogonal meshes; human calibration and automatic liquid coupling remain.
No new GPU rendering or hardware claims were made.
