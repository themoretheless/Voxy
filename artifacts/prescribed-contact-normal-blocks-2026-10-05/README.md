# Prescribed-contact normal barrier blocks

PrescribedTriangleSurface::normal_stencils returns positive normal stiffness blocks for active, enabled original face pairs. Each block retains body and obstacle node IDs, closest weights, normal and scalar d2E/ddistance2. The curvature function is shared with the existing internal primitive barrier rather than duplicated.

PrescribedContactStencil::apply implements k J^T J over joint body/obstacle displacements with frozen closest features and normal. This is a PSD preconditioner, not the full Hessian of deforming triangle distance: derivatives of the closest features and normal are omitted explicitly. It is intended for a guarded nonlinear implicit mechanical solve, which is not implemented here.

13 prescribed-contact integration tests passed. New qualification compares scalar curvature against independent central differences of forces at gaps 14 mm, 1 mm and 11 micrometres. Relative tolerance is 1e-6. Joint rigid translation is in the nullspace, tested block quadratic energy is nonnegative, and summed body/obstacle action is zero within numerical tolerance. Closed gaps reject.

No existing explicit stepping or acceptance tolerance changed. Imported full contact animation still rejects frame 52. Next integrate these normal blocks with inertial/material terms, nonlinear residuals, a contact-safe line search and independently admitted work/thermal receipts; then qualify the actual imported clip.
