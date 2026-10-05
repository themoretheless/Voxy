# Rejected contact secant restart experiment

After a line search with zero CCD-admissible trials, clear L-BFGS history and retry the current contact metric within the original 96-iteration budget. Energy, residual, CCD and force-law thresholds were unchanged.

138 physics tests passed (3 manual tests ignored). The full imported contact clip rejected at step 65 with nonlinear nonconvergence in 102.84 seconds. This regresses the qualified baseline and the experiment is excluded from the working solver. Source and trace are archived here. The trace includes 129 or more history restarts and subsequent CCD-rejected searches with empty history; history reset alone is insufficient.

The previous goal turn qualified the restored working solver with 138 physics checks, 20 integration checks and 68 imported contact steps. Full 480-step contact remains unqualified. Neither real-time performance nor anatomical skin integration is proven by this experiment.

Research pointer: RAG wiki `voxy-archive-01a0f45f-3339-71f3-b432-ae35a0dede51`, document `9e5b2366-0bd9-4377-87d2-3106a063296c`. Historical summary only; runtime conclusions above come from current logs. Semantic wiki search timed out, direct page retrieval succeeded.
