#!/usr/bin/env python3
"""Summarize exact ray counters by collection time; the source frame may differ."""
import argparse
import ast
import json
import re
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('log', type=Path)
parser.add_argument('--report', type=Path)
args = parser.parse_args()
pending = None
samples = []
invalid = 0
for line in args.log.read_text().splitlines():
    if 'STREET GPU RAY PHASES ' in line:
        match = re.search(r'project_traverse_resolve_ms=Some\((.*)\)$', line)
        pending = ast.literal_eval(match[1]) if match else None
        if match is None:
            invalid += 1
    match = re.search(r'^STREET GPU (?:previous_frame|collected_at_frame)=(\d+) ', line)
    if match:
        if int(match[1]) > 180 and pending is not None:
            samples.append(pending)
        pending = None
result = {'samples': len(samples), 'invalid_samples': invalid,
          'source_frame_association_exact': False, 'collection_window_only': True,
          'stage_order': ['project_and_optional_scheduling', 'traverse', 'resolve'],
          'ray_type_order': ['GI', 'SSR'], 'intervals_may_overlap': True,
          'exclusive_frame_cost': False, 'minimum_120_presented_fps_passed': False}
if samples:
    kinds = len(samples[0])
    assert all(len(s) == kinds and all(len(v) == 3 for v in s) for s in samples)
    result['mean_ms'] = [[sum(s[k][j] for s in samples) / len(samples)
                          for j in range(3)] for k in range(kinds)]
if args.report and args.report.exists():
    report = json.loads(args.report.read_text())
    result['full_cycle_complete'] = report.get('complete_motion_cycles', 0) >= 1
    result['completed_offscreen_fps'] = report.get('sampled_mean_fps')
else:
    result['full_cycle_complete'] = False
print(json.dumps(result, indent=2))
