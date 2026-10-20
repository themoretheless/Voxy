#!/usr/bin/env python3
"""Run pinned original-street builds in ABBA order, preserving raw evidence."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import statistics
import subprocess

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('reference_provenance', type=Path)
p.add_argument('candidate_provenance', type=Path)
p.add_argument('--output', type=Path, required=True)
a = p.parse_args()
a.output.mkdir(parents=True, exist_ok=False)
provenance = [json.loads(x.read_text()) for x in
              [a.reference_provenance, a.candidate_provenance]]
# Ignore output destinations when checking otherwise identical experimental knobs.
def knobs(v):
    return {k: x for k, x in v['settings'].items()
            if k not in ['VOXY_STREET_WORLD_RAY_AUDIT_REPORT',
                         'VOXY_STREET_WORLD_RAY_STAGE_REPORT']}
assert knobs(provenance[0]) == knobs(provenance[1])
for v in provenance:
    binary = Path(v['command'][0])
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == v['binary_sha256']
records = []
baseline = None
for index, variant in enumerate([0, 1, 1, 0]):
    v = provenance[variant]
    stem = a.output.resolve() / f'{index}-{["reference", "candidate"][variant]}'
    command = list(v['command'])
    assert '--benchmark' not in command
    command[command.index('--snapshot') + 1] = str(stem) + '.png'
    settings = dict(v['settings'])
    settings['VOXY_STREET_WORLD_RAY_AUDIT_REPORT'] = str(stem) + '.json'
    settings['VOXY_STREET_WORLD_RAY_STAGE_REPORT'] = str(stem) + '-stages.json'
    env = {k: x for k, x in os.environ.items() if not k.startswith('VOXY_')}
    env.update(settings)
    run = dict(command=command, settings=settings, binary_sha256=v['binary_sha256'])
    Path(str(stem) + '-provenance.json').write_text(json.dumps(run, indent=2))
    print(f'BEGIN {index} variant={variant} log={stem}.log', flush=True)
    with Path(str(stem) + '.log').open('w') as log:
        subprocess.run(command, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
    report = json.loads(Path(str(stem) + '.json').read_text())
    identity = {k: report[k] for k in ['frames', 'full_collision_triangles',
                'internal_resolution', 'output_resolution', 'rendered_state_blake3',
                'simulation_steps', 'simulation_clock', 'source_simplified',
                'world_frame_enabled', 'world_query_encoded_frames', 'world_regions']}
    assert identity['source_simplified'] is False and identity['world_frame_enabled']
    assert identity['frames'] == 4 and identity['world_query_encoded_frames'] == 3
    images = {ext: hashlib.sha256(Path(str(stem) + ext).read_bytes()).hexdigest()
              for ext in ['.png', '.pfm']}
    signature = dict(identity=identity, images=images)
    if baseline is None:
        baseline = signature
    assert signature == baseline, 'workload/replay/output changed'
    text = Path(str(stem) + '.log').read_text()
    times = [(int(frame), float(ms)) for frame, ms in re.findall(
        r'STREET completed_frame=(\d+) render_ms=([\d.]+)', text)]
    assert [f for f, _ in times] == [1, 2, 3, 4]
    active = [ms for frame, ms in times if frame > 1]
    record = dict(index=index, variant=variant, active_completed_frame_ms=active,
                  median_ms=statistics.median(active), signature=signature)
    records.append(record)
    Path(str(stem) + '-measurement.json').write_text(json.dumps(record, indent=2))
    print(f'END {index} active_ms={active}', flush=True)
pairs = [(records[0], records[1]), (records[3], records[2])]
ratios = [ref['median_ms'] / cand['median_ms'] for ref, cand in pairs]
summary = dict(order='ABBA', records=records, paired_reference_over_candidate=ratios,
               both_pairs_faster=all(r > 1 for r in ratios),
               geometric_mean_ratio=(ratios[0] * ratios[1]) ** 0.5,
               images_and_workload_identical=True,
               scope='four fixed-clock integration frames per process; two paired comparisons',
               complete_motion_cycle_qualified=False, presented_120_fps_qualified=False)
(a.output / 'summary.json').write_text(json.dumps(summary, indent=2))
print(json.dumps({k: v for k, v in summary.items() if k != 'records'}), flush=True)
