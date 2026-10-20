#!/usr/bin/env python3
"""Check consumer diagnostic captures; never promote counts to an FPS claim."""
import argparse
import hashlib
import json
from pathlib import Path
import struct

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('report', type=Path)
p.add_argument('--raw', type=Path)
p.add_argument('--reference-stem', type=Path)
p.add_argument('--output', type=Path, required=True)
a = p.parse_args()
report = json.loads(a.report.read_text())
audit = report.get('consumer_zero_audit', report)
assert audit['diagnostic_only'] and audit['source_queries_skipped'] == 0
records = audit['region_counters']
assert records and all(len(row) == 12 for row in records)
if a.raw:
    raw = a.raw.read_bytes()
    assert len(raw) == len(records) * 48
    assert [list(row) for row in struct.iter_unpack('<12I', raw)] == records
per_kind = audit.get('tiles_per_kind', len(audit.get('tiles', [])))
assert per_kind > 0 and len(records) % per_kind == 0
sums = [[0] * 4 for _ in range(len(records) // per_kind)]
for i, row in enumerate(records):
    assert row[8] == sum(row[9:12])
    # The diagnostic is independent of the optional original branch counters.
    # Those eight words legitimately remain zero when RESOLVE_AUDIT is disabled.
    if any(sum(v[:8]) for v in records):
        assert row[8] <= sum(row[:8])
    for k in range(4):
        sums[i // per_kind][k] += row[8 + k]
if 'by_kind' in audit:
    assert sums == audit['by_kind']
if 'internal_resolution' in report:
    width, height = report['internal_resolution']
    assert all(v[0] <= width * height for v in sums)
images = []
if a.reference_stem:
    stem = a.report.with_suffix('')
    for ext in ['.png', '.pfm']:
        candidate = Path(str(stem) + ext).read_bytes()
        reference = Path(str(a.reference_stem) + ext).read_bytes()
        assert candidate == reference, ext
        images.append(dict(extension=ext, bit_identical=True,
                           sha256=hashlib.sha256(candidate).hexdigest()))
    ref = json.loads(Path(str(a.reference_stem) + '.json').read_text())
    assert report['rendered_state_blake3'] == ref['rendered_state_blake3']
    for key in ['frames', 'simulation_steps', 'full_collision_triangles',
                'internal_resolution', 'output_resolution', 'world_regions']:
        assert report[key] == ref[key]
result = dict(diagnostic_only=True, source_queries_skipped=0,
              original_branch_counters_enabled=any(sum(v[:8]) for v in records),
              by_kind=[dict(eligible=v[0], positive_zero=v[1], other_finite=v[2],
                            nonfinite=v[3], positive_zero_fraction=v[1]/v[0] if v[0] else None)
                       for v in sums], images=images,
              evidence_scope='GPU bit-classification counters; not independent per-ray colour evidence',
              timing_includes_duplicate_fallback_work=True,
              complete_motion_cycle_qualified=False, fps_qualified=False)
a.output.write_text(json.dumps(result, indent=2))
print(json.dumps(result))
