# Implicit contact with Maxwell and thermal ownership

The existing viscoelastic transactional split now selects either the existing explicit mechanical step or implicit midpoint contact. step_viscoelastic_implicit_with_surface_motion uses the same first/second exact Maxwell relaxation and thermal deposition, unchanged quarter/half/quarter admission budgets and single candidate commit. The implicit internal mechanical solve sees frozen history; the public time-independent implicit API still rejects histories. No second thermal or relaxation system is introduced.

TissueDemo selects this implicit split when a prescribed mesh pose is supplied, preserving the existing no-surface path. Its adaptive retry, per-region domains, conduction halves and full-frame rollback remain shared. Diagnostic contact trials use the same implicit split.

Physics qualification: 17 prescribed-contact and 8 viscoelastic tests passed. The new history/thermal regression begins with presheared Maxwell material, rejects crossing atomically after relaxation is staged, then checks held-pose heat release and exact state equality against the existing split.

Actual imported Metal run still FAILED at frame 52 (0.216666667 seconds), now with `implicit contact work defect`. Five diagnostic prefix frames were written, no complete clip. Prefix step counts differ from explicit stepping, but no runtime speed claim is made. Replacing the explicit integrator alone has not fixed this fixture. Full work/path admission remains enforced.

Next diagnose the actual implicit rejected trial, physical contact gap trajectory and compatibility of the authored attachment domain with prescribed support motion. Do not silently drop the contact pair or widen the work tolerance. Full tissue regression is recorded in tissue-tests.log.
