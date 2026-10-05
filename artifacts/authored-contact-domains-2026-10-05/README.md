# Authored contact domains

PrescribedTriangleSurface::with_contact_faces accepts an explicit immutable boolean domain in original triangle order. Geometry and vertex/face indices remain intact; response gradients retain the complete original vertex array. Excluded faces contribute neither barrier energy nor continuous collision constraints. All geometry, including excluded faces, is still validated.

Changing the mask creates a new contact owner, which must be bound explicitly. Animated with_positions retains the mask identity. Attempts to change domain during motion/work evaluation reject before state publication. Candidate filtering is shared by static response and swept admission.

11 prescribed surface integration tests passed. The new regression proves: nearby excluded contact contributes zero forces; a distant enabled face still contributes; an excluded crossing is allowed while an enabled crossing rejects; source indices and animated domain remain intact; changed-domain identity, wrong-length/all-disabled masks and malformed excluded geometry reject.

No mask is inferred from current intersections. CesiumMan attachment domains have not yet been authored, and its existing full-mesh binding still fails initial admission. No successful contact GIF is claimed.
