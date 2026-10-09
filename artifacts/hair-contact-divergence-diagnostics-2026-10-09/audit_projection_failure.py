"""Independent numerical audit of opt-in VQP1 diagnostic exports.

Requires numpy/scipy. This is evidence, never a runtime solver fallback.
Effective bounds are reconstructed from the final floating-point response.
"""
import json
import struct
import sys
from pathlib import Path

import numpy as np
from scipy.optimize import linprog, minimize

data = Path(sys.argv[1]).read_bytes()
offset = 0


def read(fmt):
    global offset
    value = struct.unpack_from(fmt, data, offset)
    offset += struct.calcsize(fmt)
    return value


assert read("<4s")[0] == b"VQP1"
n = read("<I")[0]
tolerance = read("<d")[0]
assert 0 < n <= 4096
gram = np.array(read(f"<{n*n}d")).reshape(n, n)
rhs = np.array(read(f"<{n}d"))
multiplier = np.array(read(f"<{n}d"))
shape = read(f"<{read('<I')[0]}I")
entries = []
keys = set()
for row in range(n):
    row_entries = []
    for _ in range(4):
        rod, point, x, y, z = read("<II3d")
        assert rod < len(shape) and point < shape[rod]
        gradient = np.array([x, y, z])
        row_entries.append(((rod, point), gradient))
        if np.any(gradient):
            keys.add((rod, point))
    entries.append(row_entries)
assert offset == len(data)
columns = {key: i for i, key in enumerate(sorted(keys))}
jacobian = np.zeros((n, len(columns) * 3))
for i, row in enumerate(entries):
    for key, gradient in row:
        if key in columns:
            col = columns[key] * 3
            jacobian[i, col:col+3] += gradient
assert np.isfinite(gram).all() and np.isfinite(rhs).all()
rhs_scale = max(np.max(np.abs(rhs)), 1e-30)
# Primal linear feasibility, with metre residuals checked separately below.
primal = linprog(
    np.zeros(jacobian.shape[1]), A_ub=-jacobian, b_ub=-rhs/rhs_scale,
    bounds=[(None, None)]*jacobian.shape[1], method="highs",
    options={"primal_feasibility_tolerance": 1e-10, "dual_feasibility_tolerance": 1e-10},
)
# Farkas witness: nonnegative y, J^T y=0, sum(y)=1, b^T y>0.
farkas = linprog(
    -rhs/rhs_scale, A_eq=np.vstack([jacobian.T, np.ones(n)]),
    b_eq=np.r_[np.zeros(jacobian.shape[1]), 1.], bounds=(0., None),
    method="highs",
    options={"primal_feasibility_tolerance": 1e-10, "dual_feasibility_tolerance": 1e-10},
)
diagonal_scale = np.sqrt(np.diag(gram))
normalized = gram / np.outer(diagonal_scale, diagonal_scale)
normalized_rhs = rhs / diagonal_scale
dual_scale = max(np.max(np.abs(normalized_rhs)), 1e-30)
bound = normalized_rhs / dual_scale
dual = minimize(
    lambda x: (0.5*x@normalized@x-bound@x, normalized@x-bound),
    multiplier*diagonal_scale/dual_scale, jac=True, method="L-BFGS-B",
    bounds=[(0., None)]*n,
    options={"maxiter": 50000, "ftol": 0., "gtol": 1e-13, "maxls": 100},
)
lambda_candidate = dual.x*dual_scale/diagonal_scale
gap = gram@lambda_candidate-rhs
residual = np.where(lambda_candidate > 0., np.abs(gap), np.maximum(-gap, 0.))
result = {
    "scope": "independent numerical audit; reconstructed effective bounds; not a formal proof or runtime fallback",
    "constraints": n, "columns": jacobian.shape[1], "tolerance_m": tolerance,
    "gram_relative_asymmetry": float(np.max(np.abs(normalized-normalized.T))),
    "normalized_eigenvalue_min": float(np.linalg.eigvalsh((normalized+normalized.T)*0.5)[0]),
    "jacobian_rank": int(np.linalg.matrix_rank(jacobian)),
    "primal_status": primal.message,
    "primal_max_violation_m": float(np.max(np.maximum(rhs-jacobian@(primal.x*rhs_scale), 0.))) if primal.success else None,
    "farkas_status": farkas.message,
    "farkas_bound_dot_m": float(rhs@farkas.x) if farkas.success else None,
    "farkas_transpose_residual": float(np.max(np.abs(jacobian.T@farkas.x))) if farkas.success else None,
    "dual_status": dual.message, "dual_iterations": int(dual.nit),
    "dual_max_kkt_residual_m": float(np.max(residual)),
    "dual_passes_original_gate": bool(np.max(residual) <= tolerance),
}
Path(sys.argv[2]).write_text(json.dumps(result, indent=2)+"\n")
print(json.dumps(result, indent=2))
