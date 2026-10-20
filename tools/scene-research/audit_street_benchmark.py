#!/usr/bin/env python3
"""Report interference evidence; this tool never certifies speedup or FPS."""
import argparse
import json
import math
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--report', type=Path, required=True)
parser.add_argument('--build-log', type=Path, action='append', default=[])
parser.add_argument('--process-samples', type=Path)
parser.add_argument('--legacy-window-start', type=float)
parser.add_argument('--legacy-window-end', type=float)
args = parser.parse_args()
report = json.loads(args.report.read_text())
start = report.get('measurement_start_unix_seconds')
end = report.get('measurement_end_unix_seconds')
source = 'report_embedded_timestamps'
if start is None or end is None:
    start, end = args.legacy_window_start, args.legacy_window_end
    source = 'explicit_legacy_window' if start is not None and end is not None else 'unavailable'
valid = (isinstance(start, (int, float)) and isinstance(end, (int, float))
         and math.isfinite(start) and math.isfinite(end) and 0 < start < end)
builds = []
for path in args.build_log:
    stat = path.stat()
    begin = getattr(stat, 'st_birthtime', None)
    builds.append({'path': str(path), 'start_unix': begin, 'end_unix': stat.st_mtime,
                   'timestamp_assumption': 'original_uncopied_log_filesystem_times',
                   'overlaps_measurement': (begin < end and stat.st_mtime > start)
                   if valid and begin is not None else None})
inside, outside = [], []
if args.process_samples:
    for line in args.process_samples.read_text().splitlines():
        sample = json.loads(line)
        observations = [item for item in sample.get('observed_background_load', [])
                        if item.get('cpu_percent', 0) > 0]
        item = {'time_unix': sample['time_unix'], 'observations': observations}
        (inside if valid and start <= sample['time_unix'] <= end else outside).append(item)
result = {'measurement_window': {'start': start, 'end': end, 'source': source, 'available': valid},
          'build_logs': builds,
          'build_overlap_detected': any(b['overlaps_measurement'] is True for b in builds),
          'cpu_observations_inside_window': inside, 'cpu_observations_outside_window': outside,
          'absence_of_other_interference_proven': False,
          'this_audit_does_not_certify_speedup_or_presented_fps': True}
print(json.dumps(result, indent=2))
