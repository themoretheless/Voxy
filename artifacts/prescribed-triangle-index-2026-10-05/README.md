# Prescribed triangle contact spatial index

The existing thin-film triangle BVH is now shared with prescribed FEM mesh contact. Immutable obstacle poses own prepared triangle geometry and a refitted index. Motion queries cover endpoint swept bounds; a cached swept index is used only when its source pose is the actual starting pose. Candidate IDs are sorted before narrow-phase accumulation to retain full-scan force ordering.

Qualification: 5 unit tests passed. 1000 endpoint trajectories have the same admission result as an all-pairs CCD reference. A target pose prepared from a different starting pose still rejects a crossing through the actual starting pose. 256 static/refitted obstacle responses have exactly equal energy and body/obstacle gradients against a full-scan reference. 3000 randomized static, refitted and swept queries retain every brute-force overlapping bound. A localized query on 4096 separated triangles returns at most 16 candidates; a sweep across the entire mesh retains all 4096. These candidate counts are not a wall-clock speed claim.

Regression: prescribed mesh dynamics and thin-film contact/mixture tests (27 tests) are recorded in regression-tests.log.

The imported CesiumMan visualization has not yet been connected to this contact solver. No new native or GPU contact demonstration is claimed.
