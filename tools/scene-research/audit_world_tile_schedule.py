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
p.add_argument('--current-scene', action='store_true', help='Match source mesh/instance counts to the saved current-scene control log.')
p.add_argument('--require-water-time', action='store_true', help='Require no dropped ripple solver time or whole-tick debt.')
p.add_argument('--require-water', action='store_true', help='Require water submissions and matching simulation metadata.')
a = p.parse_args()
if a.require_water_time:a.require_water=True
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
gi_32=r.get('world_gi_consumer_group_32',False)
assert gi_32==(settings.get('VOXY_STREET_WORLD_GI_CONSUMER_32','0')=='1'), 'prepared GI launch differs from requested mode'
if 'world_gi_consumer_group_size'in r:
    assert r['world_gi_consumer_group_size']==([8,4] if gi_32 else [8,8]), 'unexpected GI group shape'
enabled=r.get('world_sparse_certificates',False)
certificate_group=r.get('world_sparse_certificate_group_size',64 if enabled else 0)
assert type(certificate_group)is int and (certificate_group in (1,2,4,8,16,32,64) if enabled else certificate_group==0), 'invalid prepared certificate launch size'
assert r.get('world_sparse_certificates',False)==(settings.get('VOXY_STREET_WORLD_SPARSE_CERTIFICATES','0')=='1'), 'prepared source schedule differs from requested flag'
assert r.get('world_scalar_interval_products',False)==(settings.get('VOXY_STREET_WORLD_SCALAR_INTERVAL_PRODUCTS','0')=='1'), 'prepared interval product setting differs from requested flag'
assert r.get('world_lazy_triangle_certificate',False)==(settings.get('VOXY_STREET_WORLD_LAZY_TRIANGLE_CERTIFICATE','0')=='1'), 'prepared triangle certificate setting differs from requested flag'
assert r['world_regions'] == 2 * per_kind
log = Path(str(stem) + '.log').read_text()
match = re.search(r'STREET WORLD FRAME connected=true regions=(\d+) tile=(\d+)x(\d+) ray_capacity=(\d+)', log)
assert match and list(map(int, match.groups())) == [2*per_kind, width, height, width*height]
if a.current_scene:
    source_pattern=r'STREET WORLD SOURCE ready meshes=(\d+) instances=(\d+)'
    source=re.search(source_pattern,log)
    saved=re.search(source_pattern,Path(str(a.reference_stem)+'.log').read_text())
    assert source and saved and source.groups()==saved.groups()
    assert all(int(v)>0 for v in source.groups())
else:
    assert 'meshes=137 instances=91635' in log
for key,default in [('spawn_pool',False),('replay_state_schema','legacy-camera-weather-v1'),('water_render_stats',None)]:
    assert r.get(key,default)==reference.get(key,default),key
if a.require_water:
    for report in [r,reference]:
        water=report.get('water_render_stats')
        assert water and water['registered_bodies']>0 and water['drawn_frames']>0 and water['body_draws']>0
        # Probe baking also submits water at time zero. Require a water draw
        # in the final actual audited frame, after its simulation advance.
        assert water['last_drawn_bodies']>0
        assert water['last_submitted_time_bits']==water['simulation_time_bits']
        assert water['simulation_seconds']>0
        if a.require_water_time:
            assert {'ripple_tick_rate','ripple_pending_seconds','ripple_encoded_seconds','ripple_requested_seconds'}<=water.keys(), 'ripple clock metadata required'
            import math
            assert 0<=water['ripple_pending_seconds']<1/water['ripple_tick_rate']+1e-10
            assert math.isclose(water['ripple_encoded_seconds']+water['ripple_pending_seconds'],water['ripple_requested_seconds'],rel_tol=1e-12,abs_tol=1e-10)
            assert math.isclose(water['ripple_requested_seconds'],water['simulation_seconds'],rel_tol=1e-5,abs_tol=1e-4)


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
result = {'gi_consumer_group_size':r.get('world_gi_consumer_group_size'),'sparse_certificate_group_size':certificate_group,'tile': [width, height], 'query_capacity': width*height,
          'regions_per_kind': per_kind, 'ray_kinds': 2,
          'world_regions_intentionally_changed': r['world_regions'] != reference['world_regions'],
          'geometry_replay_and_resolution_identical': True, 'cache_checker_enabled': checking,
          'raw_region_counters_match': True, 'water_submission_requirement': a.require_water, 'water_time_requirement': a.require_water_time, 'current_scene_counts_matched': a.current_scene, 'images': images,
          'coverage_scope': 'production partition bitmap test plus inspected encoder and live region/capacity metadata; not an all-ray full-street capture',
          'full_motion_cycle_qualified': False, 'presented_fps_qualified': False}
a.output.write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result))
