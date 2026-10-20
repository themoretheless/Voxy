#!/usr/bin/env python3
"""Audit intentional tile batching changes while preserving the rendered workload."""
import argparse
import hashlib
import json
import re
import struct
from pathlib import Path

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('report', type=Path)
p.add_argument('--reference-stem', required=True, type=Path)
p.add_argument('--output', required=True, type=Path)
p.add_argument('--raw', type=Path)
a = p.parse_args()
stem = a.report.with_suffix('')
r = json.loads(a.report.read_text())
reference = json.loads(Path(str(a.reference_stem) + '.json').read_text())
provenance = json.loads(Path(str(stem) + '-provenance.json').read_text())
settings = provenance['settings']
width = int(settings.get('VOXY_STREET_WORLD_RAY_TILE_WIDTH', '512'))
height = int(settings.get('VOXY_STREET_WORLD_RAY_TILE_HEIGHT', '256'))
assert width > 0 and height > 0
image_width, image_height = r['internal_resolution']
per_kind = ((image_width + width - 1) // width) * ((image_height + height - 1) // height)
for key in ['frames', 'simulation_steps', 'rendered_state_blake3',
            'full_collision_triangles', 'internal_resolution', 'output_resolution',
            'world_query_encoded_frames']:
    assert r[key] == reference[key], key
assert not r['source_simplified'] and r['world_frame_enabled']
assert settings['VOXY_STREET_COMPUTE_GI'] == settings['VOXY_STREET_COMPUTE_SSR'] == '1'
assert settings['VOXY_STREET_RENDER_SCALE'] == '2' and settings['VOXY_FX_OFF'] == '0'
assert r['world_regions'] == 2 * per_kind
log = Path(str(stem) + '.log').read_text()
match = re.search(r'STREET WORLD FRAME connected=true regions=(\d+) tile=(\d+)x(\d+) ray_capacity=(\d+)', log)
assert match and list(map(int, match.groups())) == [2*per_kind, width, height, width*height]
assert 'meshes=137 instances=91635' in log
cache = r['consumer_cache_audit']
assert cache['consumer_cache'] and cache['tiles_per_kind'] == per_kind
records = cache['region_counters']
raw_path = a.raw or Path(str(stem) + '-consumer.bin')
raw = raw_path.read_bytes()
assert len(raw) == 48 * 2 * per_kind
assert [list(v) for v in struct.iter_unpack('<12I', raw)] == records
sums = [[sum(row[k] for row in records[start:start+per_kind]) for k in range(8, 12)]
        for start in [0, per_kind]]
assert sums == cache['by_kind']
checking = settings.get('VOXY_STREET_WORLD_RAY_CONSUMER_CACHE_CHECK') == '1'
for pruned, checked, mismatched, nonfinite in sums:
    assert mismatched == nonfinite == 0
    if checking:
        assert checked >= pruned > 0
images = []
for ext in ['.png', '.pfm']:
    data = Path(str(stem) + ext).read_bytes()
    assert data == Path(str(a.reference_stem) + ext).read_bytes(), ext
    images.append({'extension': ext, 'bit_identical': True,
                   'sha256': hashlib.sha256(data).hexdigest()})
result = {'tile': [width, height], 'query_capacity': width*height,
          'regions_per_kind': per_kind, 'ray_kinds': 2,
          'world_regions_intentionally_changed': r['world_regions'] != reference['world_regions'],
          'geometry_replay_and_resolution_identical': True, 'cache_checker_enabled': checking,
          'raw_region_counters_match': True, 'images': images,
          'coverage_scope': 'production partition bitmap test plus inspected encoder and live region/capacity metadata; not an all-ray full-street capture',
          'full_motion_cycle_qualified': False, 'presented_fps_qualified': False}
a.output.write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result))
