#!/usr/bin/env python3
"""Validate cumulative software certificate work without interpreting it as FPS."""
import argparse
import json
import struct
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('report', type=Path)
parser.add_argument('--raw', required=True, type=Path)
parser.add_argument('--ray-kinds', required=True, type=int, choices=[1, 2])
parser.add_argument('--output', required=True, type=Path)
args = parser.parse_args()
report = json.loads(args.report.read_text())
work = report['software_traversal_work']
raw = args.raw.read_bytes()
assert len(raw) == 48
words = list(struct.unpack('<12I', raw))
assert words == work['raw_words']
assert words[10] < 2**32-1 and words[11] < 2**32-1
labels = ['sampled_records', 'visited_nodes', 'triangle_tests',
          'records_with_nodes', 'records_with_triangles']
values = {label: words[2*i] + (words[2*i+1] << 32)
          for i, label in enumerate(labels)}
assert all(work[key] == value for key, value in values.items())
assert work['diagnostic_only'] and work['timing_includes_diagnostic_overhead']
assert not work['fps_qualified']
width, height = report['internal_resolution']
regions = report['world_regions']
frames = report['world_query_encoded_frames']
# This audit is for the demo's shared owner, prepared once before its full-frame
# encodes; a different job lifetime must supply a different coverage authority.
expected = frames * (width * height * args.ray_kinds - regions)
assert values['sampled_records'] == expected, (values, expected)
assert work['maximum_nodes'] == words[10]
assert work['maximum_triangles'] == words[11]
for total, records, maximum in [('visited_nodes', 'records_with_nodes', words[10]),
                                ('triangle_tests', 'records_with_triangles', words[11])]:
    assert 0 <= values[records] <= values['sampled_records']
    assert values[records] <= values[total] <= values[records] * maximum
    assert (values[total] == 0) == (maximum == 0)
result = {
    'scope': 'combined cumulative software certificate samples only',
    'raw_counter_words_match': True,
    'expected_sampled_records': expected,
    'excluded_first_ray_records': frames * regions,
    **values,
    'maximum_nodes': words[10], 'maximum_triangles': words[11],
    'mean_nodes_per_walked_record': (values['visited_nodes'] / values['records_with_nodes']
                                   if values['records_with_nodes'] else None),
    'mean_triangles_per_testing_record': (values['triangle_tests'] / values['records_with_triangles']
                                         if values['records_with_triangles'] else None),
    'hardware_and_exact_fallback_work_included': False,
    'counts_do_not_establish_phase_cost': True,
    'full_cycle_qualified': False, 'presented_fps_qualified': False,
}
args.output.write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result))
