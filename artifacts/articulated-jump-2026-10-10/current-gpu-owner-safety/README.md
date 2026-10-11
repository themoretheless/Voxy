# GPU storage and changing-operator safety follow-up

Current frozen app test binary: /tmp/voxy-contact-owner-gpu-tests. Original binary and source hashes are recorded in provenance.json.

Actual GPU workspace reset regression passes 24 changed operators. Reused and fresh GPU coordinates/reactions and complete storage snapshots are bitwise equal; equal-size payloads with different layouts also match. Invalid nonfinite bounds preserve the existing pool. One creation and 23 reuses, no staging quarantine, scoped memory charge released on completion.

Actual GPU capacity regression passes 65 independent owners repeated twice under ordinary and 180-byte readback capacity. Results match serial GPU exactly, bounded chunks preserve order, no quarantine. This verifies the existing scheduling/storage contract, not full-density performance or the entire jump.

The QR prefix append/release/changed-scale/changed-mapping/invalid-status test is being rebuilt from current renderer source separately. The full-density owner-isolation run is still live; no restart or changes to its binary, physical tolerance or solver flags were made.

The existing renderer test binary also passes actual GPU prefix append, suffix release and complete reuse with bitwise fresh-workspace equality; changed scale/mapping and invalid status are rejected. Its binary hash is stored separately. This is explicitly existing-binary evidence; the current-source Cargo rebuild remains live in session 28416, so it is not reported as a successful current rebuild.

Current-source renderer rebuild has now terminated successfully: the actual GPU prefix append/release/changed-scale/changed-mapping/invalid-status test passes. Its complete build/test log is current-source-prefix.log. A later change adds separate repeated-pass timestamp instrumentation and disabled-feature preflight; that change is under its own validation and does not alter the frozen full-model candidate.
