#!/usr/bin/env python3
"""CPU scheduling oracle; does not claim GPU arithmetic equivalence or speed."""
import argparse
import json
import random
from pathlib import Path
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--output', required=True, type=Path)
a = p.parse_args()
rng = random.Random(0x564f5859)
cases = 0
for width in range(1, 9):
    for height in range(1, 9):
        for trial in range(32):
            values = [((rng.randrange(66) if trial else 0) << 6 | lane)
                      if lane % 8 < width and lane // 8 < height else 0xffffff00 | lane
                      for lane in range(64)]
            expected = sorted(values)
            size = 2
            while size <= 64:
                stride = size // 2
                while stride:
                    before = values[:]
                    values = [(min(before[lane], before[lane ^ stride])
                               if ((lane & stride) == 0) == ((lane & size) == 0)
                               else max(before[lane], before[lane ^ stride]))
                              for lane in range(64)]
                    stride //= 2
                size *= 2
            assert values == expected
            assert sorted(v & 63 for v in values) == list(range(64))
            valid = [v & 63 for v in values if (v & 63) % 8 < width and (v & 63) // 8 < height]
            assert len(valid) == len(set(valid)) == width * height
            assert set(valid) == {y * 8 + x for y in range(height) for x in range(width)}
            cases += 1
result = dict(cpu_network_cases=cases, all_partial_shapes=[8,8],
              lanes_preserved=True, each_valid_pixel_exactly_once=True,
              gpu_equivalence_qualified=False, performance_qualified=False)
a.output.write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result))
