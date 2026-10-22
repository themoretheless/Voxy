#!/usr/bin/env python3
"""Validate full-scene workload preservation for the optional exact-point consumer.

Image differences are reported, never treated as a photorealism or quality proof.
This four-frame integration capture does not qualify a complete motion cycle/FPS.
"""
import argparse
import array
import hashlib
import json
import math
import re
import sys
from pathlib import Path


def pfm(path):
    with path.open('rb') as f:
        assert f.readline() == b'PF\n'
        size = tuple(map(int,f.readline().split()))
        scale = float(f.readline())
        values = array.array('f',f.read())
    if (scale < 0) != (sys.byteorder == 'little'):
        values.byteswap()
    assert len(values) == size[0]*size[1]*3
    assert all(math.isfinite(v) for v in values)
    return size, values


def check(root):
    control = json.loads((root/'scene-control/report.json').read_text())
    candidate = json.loads((root/'scene-position/report.json').read_text())
    for key in ['frames','simulation_clock','simulation_steps','rendered_state_blake3',
                'full_collision_triangles','internal_resolution','output_resolution',
                'world_query_encoded_frames','world_regions','spawn_pool','water_render_stats']:
        assert control[key] == candidate[key], key
    assert candidate['frames'] == 4 and candidate['world_query_encoded_frames'] == 3
    assert candidate['complete_motion_cycles'] == 0
    assert candidate['internal_resolution'] == [3840,2160]
    assert candidate['output_resolution'] == [1920,1080]
    assert candidate['spawn_pool'] and not candidate['source_simplified'] and candidate['world_frame_enabled']
    water = candidate['water_render_stats']
    assert water['last_drawn_bodies'] > 0 and water['registered_bodies'] > 0
    assert water['last_submitted_time_bits'] == water['simulation_time_bits']
    assert water['ripple_encoded_ticks'] == 240 and water['ripple_tick_rate'] == 240
    assert water['ripple_pending_seconds'] == 0 and water['ripple_requested_seconds'] == 1
    logs = [(root/d/'log.txt').read_text() for d in ['scene-control','scene-position']]
    pattern=r'ORIGINAL STREET seed=(\d+) source_geometry_assets=(\d+) materials=(\d+) mesh_batches=(\d+) triangles=(\d+) instances=(\d+)'
    sources=[re.search(pattern,t).groups() for t in logs]
    assert sources[0] == sources[1] and int(sources[1][4]) == 3472196
    assert 'exact_position_consumer=true material_surface_policy=true' in logs[1]
    for name in ['scene-control','scene-position']:
        p=json.loads((root/name/'provenance.json').read_text())
        s=p['settings']
        assert s['VOXY_STREET_RENDER_SCALE']=='2' and s['VOXY_FX_OFF']=='0'
        assert s['VOXY_STREET_COMPUTE_GI']==s['VOXY_STREET_COMPUTE_SSR']=='1'
        assert s['VOXY_STREET_WORLD_RAY_TILE_WIDTH']=='3840' and s['VOXY_STREET_WORLD_RAY_TILE_HEIGHT']=='432'
        assert not p['performance_measurement']
        assert hashlib.sha256(Path(p['command'][0]).read_bytes()).hexdigest()==p['binary_sha256']
        process=json.loads((root/name/'process.json').read_text())
        assert process['state']=='terminal' and process['returncode']==0
    size,a=pfm(root/'scene-control/report.pfm')
    size_b,b=pfm(root/'scene-position/report.pfm')
    assert size == size_b == (1920,1080)
    changed=sum(x!=y for x,y in zip(a,b))
    delta=max(abs(x-y) for x,y in zip(a,b))
    return dict(status='exact_position_full_scene_integration_passed',full_collision_triangles=3472196,
                internal_resolution=[3840,2160],output_resolution=[1920,1080],
                frames=4,query_frames=3,geometry_replay_resolution_and_water_matched=True,
                exact_position_consumer_prepared=True,source_coverage_scope='production partition and live owner/count metadata; no exhaustive full-scene ray dump',
                changed_hdr_channels=changed,max_hdr_channel_difference=delta,
                image_changes_are_not_quality_proof=True,
                last_completed_render_ms_control=control['last_completed_render_ms'],
                last_completed_render_ms_candidate=candidate['last_completed_render_ms'],
                timing_scope='integration diagnostic only; not paired steady-state or presented FPS',
                complete_motion_cycle_qualified=False,minimum_120_fps_qualified=False)


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('root',type=Path)
    p.add_argument('--output',type=Path,required=True)
    a=p.parse_args()
    result=check(a.root)
    a.output.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result))
