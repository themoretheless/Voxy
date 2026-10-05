# Rejected imported contact step: measured refinement

The actual frame-52 failure is reproducible at 0.216666667 seconds. Optional VOXY_CONTACT_REJECTION_TRACE evaluates rejected trials on isolated diagnostic clones; these never publish state. The normal acceptance tolerance is unchanged.

At depth 12, dt=2.5431315104166666e-7 s, mechanical defect=-3.0552394902126585e-10 J against a half-budget of 3.0517578125e-10 J. Diagnostic subdivisions of that same interval give total defects -7.636233209789813e-11 J (2) and -1.9081347494794507e-11 J (4). Maximum leaf defects are 3.8452145450104073e-11 and 4.8183895111784996e-12 J, both below their unchanged leaf budgets. Local error converges approximately quadratically.

An experimental depth limit of 14 still rejects frame 52, later within the staged frame. At dt=6.3578287760416666e-8 s, mechanical defect=-7.641541368231863e-11 J exceeds 7.62939453125e-11 J. Further diagnostic subdivisions again converge (total -1.9142125391290606e-11 and -4.8431581821409124e-12 J). The depth-14 run produced only five prefix frames. The original limit of 12 was restored because this experiment did not solve the full-frame failure. No full clip is qualified.

This evidence rules out treating a simple depth-14 increase as the fix. It does not identify the physical/geometric root cause. Next investigate the closest active source triangles, attachment/pinned feature participation and gap evolution during the rejected frame; determine whether prescribed support motion conflicts with admissible contact or whether the explicit integrator encounters increasing contact stiffness.

The existing adaptive contact rollback test passed during the depth-14 experiment. Diagnostic trace preserves a normal rejected result and never commits its relaxed-admission clone.
