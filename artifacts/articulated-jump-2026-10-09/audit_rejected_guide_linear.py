"""Independent exact-product audit of a rejected guide's diagnostic Newton step.

Linear admission is not proof of nonlinear equilibrium or collision clearance.
"""
import json
import math
import sys
from decimal import Decimal, localcontext
from pathlib import Path

data = json.loads(Path(sys.argv[1]).read_text())
system = data["candidate_linear_system"]
assert system and system["native_correction_error"] is None
matrix, rhs, correction = system["matrix"], system["rhs"], system["native_correction"]
lo, hi, band = system["active_start"], system["active_end"], system["band_width"]
assert len(matrix) == len(rhs) * band and len(rhs) == len(correction)
assert all(correction[i] == rhs[i] for i in list(range(lo)) + list(range(hi, len(rhs))))
with localcontext() as context:
    context.prec = 100
    residuals = []
    for i in range(lo, hi):
        entries = [(matrix[i * band], correction[i])]
        entries += [(matrix[i * band + i - j], correction[j]) for j in range(max(lo, i - band + 1), i)]
        entries += [(matrix[j * band + j - i], correction[j]) for j in range(i + 1, min(hi, i + band))]
        products = [Decimal.from_float(a) * Decimal.from_float(b) for a, b in entries]
        value = Decimal.from_float(rhs[i])
        scale = sum(map(abs, products), abs(value))
        residuals.append(float(abs(sum(products, Decimal(0)) - value) / max(scale, Decimal("1e-30"))))
result = {
    "guide": data["guide"], "dt": system["dt"],
    "max_scaled_exact_linear_residual": max(residuals),
    "original_linear_tolerance": 1e-8,
    "all_linear_rows_admitted": all(r <= 1e-8 for r in residuals),
    "max_linear_correction_m": max(math.sqrt(sum(x * x for x in correction[i:i + 3])) for i in range(lo, hi, 6)),
    "max_angular_correction_rad": max(math.sqrt(sum(x * x for x in correction[i + 3:i + 6])) for i in range(lo, hi, 6)),
}
Path(sys.argv[2]).write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps(result, indent=2))
assert result["all_linear_rows_admitted"]
