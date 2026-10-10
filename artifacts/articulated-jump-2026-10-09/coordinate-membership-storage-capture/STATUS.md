# Coordinate owner membership and gap storage

UnilateralContinuation now owns one reusable full gap vector and an exact active-row membership flag vector. Gap products retain the original column order, compensated/FMA arithmetic, finite checks and tolerance. Flags initialize from positive validated seed reactions, clear on the exact released row, and set on the exact entering row; active ordering/tie selection and original512 limit remain unchanged. This removes a new gap-vector allocation on each gap check and repeated linear active-list searches per candidate. No precision reduction, shader change or measured FPS improvement claimed.

345 physics library tests passed,39 ignored, including equality/native-reference comparisons and resumable/seeded/release/cold-retry cases. Real Metal heterogeneous seeded-release and dependent-hint cold-retry test passed with exact serial match. git diff --check passed.

Current full original-column velocity run13189 stays on its frozen pre-storage-change binary. First frame passed original pose/rotation limits,9131 coordinate calls and9131 admitted,zero native fallbacks,26663 equality dispatches in7458 submissions. Full720 and >160 rendered FPS remain incomplete. This small host allocation change is not represented as the primary real-time solution; GPU residency/selection/geometry and whole-model solver cost remain open.
