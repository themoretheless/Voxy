#!/usr/bin/env python3
"""Independent OSQP proposal for a frozen VQC1; never admits a physical state.

Requires numpy, scipy and osqp in an isolated environment. All original rows
are retained. Numerical infeasibility status is not a rigorous certificate.
"""
import argparse
import json
import numpy as np
import scipy
from scipy import sparse
import osqp
from audit_hair_qr_input import audit

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('capture')
parser.add_argument('--max-iterations', type=int, default=20000)
args = parser.parse_args()
if args.max_iterations <= 0:
    parser.error('max-iterations must be positive')
metadata = audit(args.capture)
rows, width = metadata['rows'], metadata['coordinates']
bounds = np.memmap(args.capture, dtype='<f8', mode='r', offset=28, shape=(rows,))
columns = np.memmap(args.capture, dtype='<f8', mode='r', offset=28 + rows * 16, shape=(rows, width))
# Only identically zero coordinates are removed; no contact rows or nonzero
# coefficients are dropped. The Euclidean objective is unchanged.
matrix = sparse.csc_matrix(columns)
used = np.diff(matrix.indptr) != 0
matrix = matrix[:, used]
solver = osqp.OSQP()
solver.setup(P=sparse.eye(matrix.shape[1], format='csc'), q=np.zeros(matrix.shape[1]),
             A=matrix, l=np.asarray(bounds), u=np.full(rows, np.inf),
             eps_abs=metadata['tolerance'], eps_rel=0., max_iter=args.max_iterations,
             polishing=True, verbose=False)
result = solver.solve(raise_error=False)
report = dict(capture=metadata, osqp_version=osqp.__version__, numpy_version=np.__version__,
              scipy_version=scipy.__version__, status=result.info.status,
              iterations=result.info.iter, retained_coordinates=matrix.shape[1],
              maximum_iterations=args.max_iterations, requested_absolute_tolerance=metadata['tolerance'],
              requested_relative_tolerance=0., nonzero_coefficients=matrix.nnz,
              scope='independent whitened QP proposal only; no physical force-balance or continuous-motion admission')
if result.info.status_val in (1, 2) and result.x is not None:
    gap = matrix @ result.x - bounds
    reactions = -result.y
    active = reactions > 0
    report.update(max_inequality_violation=float(max(0., -gap.min())),
                  minimum_reaction=float(reactions.min()),
                  maximum_active_gap=float(np.max(np.abs(gap[active]))) if active.any() else 0.,
                  stationarity_inf=float(np.max(np.abs(result.x - matrix.T @ reactions))))
elif result.info.status_val in (3, 4):
    # For Cx >= b a candidate Farkas vector is -OSQP's lower-bound certificate.
    witness = -result.prim_inf_cert
    report.update(candidate_certificate_minimum=float(witness.min()),
                  candidate_certificate_transpose_inf=float(np.max(np.abs(matrix.T @ witness))),
                  candidate_certificate_bound_dot=float(bounds @ witness),
                  certificate_scope='approximate numerical witness; not proof of infeasibility')
print(json.dumps(report, indent=2, allow_nan=False))
