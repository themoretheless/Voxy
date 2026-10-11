"""Independent 100-digit physical gap audit of immutable VQA1 loads/responses."""
import json
import struct
import sys
from decimal import Decimal, localcontext
from pathlib import Path

raw = memoryview(Path(sys.argv[1]).read_bytes())
assert raw[:4] == b"VQA1"
rows, systems = struct.unpack_from("<2I", raw, 4)
tolerance = struct.unpack_from("<d", raw, 12)[0]
offset = 20

def scalars(count):
    global offset
    values = struct.unpack_from(f"<{count}d", raw, offset)
    offset += count * 8
    return values

bounds = scalars(rows)
reactions = scalars(rows)
with localcontext() as context:
    context.prec = 100
    actual = [Decimal(0)] * rows
    for _ in range(systems):
        width = struct.unpack_from("<I", raw, offset)[0]
        offset += 4
        response = [Decimal.from_float(x) for x in scalars(width)]
        for i in range(rows):
            actual[i] += sum((Decimal.from_float(a) * b for a, b in zip(scalars(width), response)), Decimal(0))
    assert offset == len(raw)
    gaps = [a - Decimal.from_float(b) for a, b in zip(actual, bounds)]
    limit = Decimal.from_float(tolerance)
    result = {
        "rows": rows, "systems": systems, "tolerance": tolerance,
        "max_active_exact_residual": float(max((abs(gaps[i]) for i in range(rows) if reactions[i] > 0), default=Decimal(0))),
        "minimum_inactive_exact_gap": float(min((gaps[i] for i in range(rows) if reactions[i] == 0), default=Decimal(0))),
        "all_original_physical_gaps_admitted": all(reactions[i] >= 0 and (abs(gaps[i]) <= limit if reactions[i] > 0 else gaps[i] >= -limit) for i in range(rows)),
    }
Path(sys.argv[2]).write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps(result, indent=2))
assert result["all_original_physical_gaps_admitted"]
