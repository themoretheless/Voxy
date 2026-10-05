# Closest active contact at imported frame-52 rejection

PrescribedTriangleSurface::nearest_active_contact is a read-only diagnostic query. It shares the authored enabled-face domain and spatial index with the contact solver, returns original body/source-face identities and barycentric features, and reports closed gaps without forcing energy evaluation. Outside activation it returns None. Invalid body geometry still rejects.

12 prescribed-contact tests passed. The new fixture reconstructs closest points from the reported weights, verifies distance/gap and weight bounds, checks closed-gap reporting versus normal response rejection, out-of-range None, invalid-index rejection and unchanged surface state.

Actual rejected step: nearest body face [3,10,15], original character face index 3062. Before: distance 0.00011119479139512532 m, residual gap 0.00001119479139512531 m. After the uncommitted diagnostic trial: distance 0.00011107631605426671 m, residual gap 0.00001107631605426670 m. Body weights [0.055493259734228806,0.9445067402657712,0] before; pinned contribution 5.5493%. Closest identities remain the same at the two endpoints.

These measurements show very small positive separation and predominantly free-feature participation. They do not prove that the entire trajectory uses one feature, nor rule out other active pairs. Combined with local quadratic work-defect convergence, increasing barrier stiffness near the minimum gap is a plausible cause of the explicit step's increasingly small time requirement. A wholly prescribed, immovable nearest feature is not supported by this measurement.

Next investigate a contact-aware implicit/damped mechanical solve with independent work accounting, guarded line search and continuous separation checks; preserve thermal/Maxwell staging and existing atomic rollback. Do not solve admission by disabling triangle 3062 or increasing the energy tolerance. Full imported contact animation remains rejected at frame 52.
