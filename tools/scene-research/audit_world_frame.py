#!/usr/bin/env python3
"""Independent full-pixel source-query coverage, parameter and UV qualification."""
import argparse
import json
import os
import struct
import subprocess
import sys
from pathlib import Path
from make_world_nearest_gold import build

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('root', type=Path)
parser.add_argument('--output', type=Path, required=True)
parser.add_argument('--existence-witness', action='store_true', help='also prove explicit history existence results against original geometry')
args = parser.parse_args()
frame = json.loads((args.root / 'world-frame.json').read_text())
width, height = frame['size']
reports = []
os.environ['VOXY_NEAREST_MATERIALS'] = '1'
for kind in range(2):
    seen = set()
    cases = []
    records = []
    inactive = 0
    for index, (x, y, w, h) in enumerate(frame['tiles']):
        stem = args.root / f'frame-{kind}-tile-{index}'
        inputs = list(struct.iter_unpack('<20I', stem.with_name(stem.name + '-input.bin').read_bytes()))
        raw = stem.with_name(stem.name + '-hits.bin').read_bytes()
        words = len(raw) // (w * h * 4)
        assert words in [20, 28]
        hits = list(struct.iter_unpack('<' + 'I' * words, raw))
        assert len(inputs) == len(hits) == w * h
        counts = frame['branch_counts'][kind * len(frame['tiles']) + index]
        if frame.get('audit_counters_enabled', True):
            assert sum(counts) == w * h
        else:
            assert counts == [0] * 8
        misses = tails = 0
        for local, (ray, hit) in enumerate(zip(inputs, hits)):
            pixel = (y + local // w) * width + x + local % w
            assert ray[18:20] == (pixel, kind) and hit[4:6] == ray[18:20]
            assert pixel not in seen
            seen.add(pixel)
            if ray[3] == 0:
                assert hit[0] == 3
                inactive += 1
                continue
            assert ray[3] == 1
            cases.append({'global_ray': pixel | kind << 32, 'ray_words': list(ray)})
            records.append(hit)
            misses += int(hit[0] == 0)
            tails += int(hit[0] == 1 and hit[15] & 1 != 0)
        if frame.get('audit_counters_enabled', True):
            assert counts[1] == misses and counts[3] == tails
    assert seen == set(range(width * height))
    oracle = build(args.root, cases)
    gold = {row['global_ray']: row for row in oracle['rows']}
    witness_count = 0
    native_witness_count = 0
    for hit in records:
        expected = gold[hit[4] | hit[5] << 32]
        expected_state = expected.get('expected_state', int(expected['exact_hit']))
        if args.existence_witness and hit[0] == 2:
            assert hit[1] == 8192 and expected_state == 1
            assert frame['history_admission'] and frame['any_hit_admission_enabled']
            manifest = json.loads((args.root / 'manifest.json').read_text())
            assert manifest['materials'] and all(not m['double_sided'] for m in manifest['materials'])
            witness_count += 1
            if hit[18] & 65536:
                assert frame['native_source_witness_enabled']
                native_witness_count += 1
        else:
            assert hit[0] == expected_state
        if hit[0] == 1:
            assert list(hit[6:8]) == [expected['winner_slot'], expected['winner_primitive']]
            assert hit[17] == expected['winner_material']
            assert list(hit[8:14]) == expected['parameter_words']
            assert hit[15] == expected['residual_mask']
    gold_path = args.root / f'full-frame-{kind}-gold.json'
    gold_path.write_text(json.dumps(oracle, indent=2) + '\n')
    uv_report = None
    if records and len(records[0]) == 28:
        merged = args.root / f'full-frame-{kind}-active-hits.bin'
        merged.write_bytes(b''.join(struct.pack('<28I', *hit) for hit in records))
        output = args.root / f'full-frame-{kind}-uv-audit.json'
        subprocess.run([sys.executable, str(Path(__file__).with_name('audit_world_hit_uv.py')),
                        str(args.root), str(gold_path), str(merged), '--output', str(output)],
                       check=True, stdout=subprocess.DEVNULL)
        uv_report = json.loads(output.read_text())
    reports.append({'kind': kind, 'original_pixels_once': len(seen), 'inactive': inactive,
                    'independent_source_queries': len(records), 'parameters_and_residuals_exact': True, 'canonical_parameters_scope': 'state1_only',
                    'existence_witnesses_checked': witness_count, 'native_source_witnesses_checked': native_witness_count,
                    'uv_audit': uv_report})
result = {'size': frame['size'], 'kinds': reports, 'original_frame_pass_skipped': True,
          'gpu_timing_positive': frame['gpu_gi_ms'][1] > 0 and frame['gpu_ssr_ms'] > 0,
          'full_street_qualified': False, 'fps_qualified': False}
args.output.write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result))
