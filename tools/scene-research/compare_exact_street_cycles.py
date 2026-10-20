#!/usr/bin/env python3
"""Compare matched completed street cycles; never certify isolation or presented FPS."""
import argparse
import json
import math
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--control', type=Path, required=True)
parser.add_argument('--candidate', type=Path, action='append', required=True)
parser.add_argument('--optimization', choices=['paired_dda_tile_edge_cache', 'world_ray_source_edge_cache', 'paired_dda_interval_cache', 'ray_queue', 'ray_spatial_key', 'ray_hierarchy', 'ray_hierarchy_reciprocal_cache', 'ray_hierarchy_cached', 'ray_hierarchy_near_first', 'ray_persistent', 'paired_dda_lazy_cell_bounds', 'paired_dda_touch_guard', 'world_ray_consumer_sorted', 'metal_compute_max_threads_requested'], default='paired_dda_interval_cache')
parser.add_argument('--control-metal-audit', type=Path)
parser.add_argument('--candidate-metal-audit', type=Path, action='append', default=[])
args = parser.parse_args()
control = json.loads(args.control.read_text())
candidates = [json.loads(p.read_text()) for p in args.candidate]
world_ray = control.get('world_ray_frame', False)
assert all(c.get('world_ray_frame', False) == world_ray for c in candidates)
fixed = ['scene', 'measurement', 'resolution', 'internal_resolution', 'antialiasing',
         'full_collision_triangles', 'fx_off', 'paired_depth_dda', 'paired_dda_tiles',
         'paired_dda_cell_bounds', 'paired_dda_far_bound', 'paired_dda_hierarchy',
         'paired_dda_edge_cache', 'paired_dda_plane_cache', 'paired_dda_point_cache',
         'compute_gi', 'compute_ssr', 'ray_stages', 'ray_coherent', 'ray_force_fallback',
         'refinement_cache', 'early_gi', 'motion_period_seconds', 'frames',
         'measured_physics_steps', 'measured_simulation_seconds']
for report in [control, *candidates]:
    assert report['completed_offscreen_measurement_valid']
    assert report['measurement'] == 'completed_full_resolution_offscreen_frames'
    assert report['complete_motion_cycles'] >= 1
    assert report['measured_physics_steps'] >= 2400
    assert report['source_simplified'] is False
    assert report['ray_force_fallback'] is False
    assert report['fx_off'] == '0'
    assert all(report[key] is True for key in ['paired_depth_dda', 'paired_dda_tiles',
               'paired_dda_hierarchy', 'compute_gi', 'compute_ssr'])
    if world_ray:
        assert report['world_ray_regions'] > 0 and report['world_query_encoded_frames'] > 0
        assert report['frame_sample_clock'] == 'current_completed_frame'
        assert math.isfinite(report['mean_fps']) and report['mean_fps']>0 and report['seconds']>0
        assert math.isclose(report['frames']/report['seconds'], report['mean_fps'], rel_tol=1e-8), 'FPS and wall duration disagree'
        samples=report['completed_frame_samples_seconds'];renders=report['current_render_samples_seconds']
        assert len(samples)==len(renders)==report['frames']
        assert all(math.isfinite(s) and math.isfinite(r) and 0<r<=s for s,r in zip(samples,renders)), 'sample does not bound its current render'
        assert math.isclose(sum(samples),report['sampled_frame_seconds'],rel_tol=1e-12)
        assert sum(samples)<=report['seconds'], 'active-frame time exceeds total measurement time'
        assert math.isclose(len(samples)/sum(samples),report['sampled_mean_fps'],rel_tol=1e-12)
        assert math.isclose(max(samples)*1000,report['max_frame_ms'],rel_tol=1e-12)
        assert report['simulation_clock'] == 'fixed_30_step_batch'
        assert report['benchmark_stop_policy'] == 'fixed_simulation_duration' 
        assert report['world_ray_consumer_cache'] is True
        assert report['world_ray_consumer_profile'] is False
        assert report['world_ray_consumer_cache_check'] is False
    else:
        assert report['ray_stages'] is True
        assert report['gpu_gi_samples'] > 0 and report['gpu_ssr_samples'] > 0
        assert report['gpu_samples'] > 0
    assert report['sampled_mean_fps'] > 0
# Composite mode comparisons explicitly change both algorithm flags. They
# establish end-to-end differences, not the isolated effect of either flag.
changed = ['ray_hierarchy', 'ray_hierarchy_reciprocal_cache'] if args.optimization == 'ray_hierarchy_cached' else [args.optimization]
if world_ray:
    fixed.extend(['frame_sample_clock', 'world_ray_frame', 'world_ray_regions', 'world_query_encoded_frames',
                  'world_ray_consumer_cache', 'world_ray_consumer_profile',
                  'world_ray_consumer_cache_check'])
compiler_mode = args.optimization == 'metal_compute_max_threads_requested'
for report in [control, *candidates]:
    report.setdefault('metal_compute_max_threads_requested', 0)
if not compiler_mode:
    fixed.append('metal_compute_max_threads_requested')
flags = ['paired_dda_tile_edge_cache', 'world_ray_source_edge_cache', 'world_ray_consumer_sorted', 'paired_dda_lazy_cell_bounds', 'paired_dda_touch_guard', 'paired_dda_interval_cache', 'ray_queue', 'ray_spatial_key', 'ray_hierarchy', 'ray_hierarchy_reciprocal_cache', 'ray_hierarchy_near_first', 'ray_persistent']
for report in [control, *candidates]:
    for flag in flags:
        if flag not in changed and flag != 'paired_dda_interval_cache':
            report.setdefault(flag, False)
for flag in flags:
    if flag not in changed:
        fixed.append(flag)
if compiler_mode:
    from collections import Counter
    assert world_ray
    assert control[args.optimization] == 0
    assert args.control_metal_audit and len(args.candidate_metal_audit) == len(candidates)
    audits = [json.loads(path.read_text()) for path in [args.control_metal_audit, *args.candidate_metal_audit]]
    fixed.append('metal_compute_audit_requested')
    assert control['metal_compute_audit_requested'] is True
    for report, audit in zip([control, *candidates], audits):
        assert audit['requested'] == report[args.optimization]
        assert audit['world_consumers_verified'] > 0
        records = audit['records']
        assert records and audit['native_pipelines_verified'] == len(records)
        for row in records:
            request=report[args.optimization]
            expected=request if request and row['local']<=request else 0
            assert row['requested']==request and row['applied']==expected
            assert row['native_max']>=row['local']>0 and row['execution_width']>0
            if expected:
                assert row['native_max']<=expected
        assert sum(row['entry']=='cs_world_consumer_cache' for row in records)==audit['world_consumers_verified']
        assert all(row['local']==64 for row in records if row['entry']=='cs_world_consumer_cache')
    shapes = [Counter((row['entry'], row['local']) for row in audit['records']) for audit in audits]
    assert all(shape == shapes[0] for shape in shapes[1:]), 'compiled pipeline dispatch shapes differ'
else:
    assert all(control[flag] is False for flag in changed)
for candidate in candidates:
    if compiler_mode:
        assert candidate[args.optimization] in [64, 128, 256, 512, 768, 1024]
    else:
        assert all(candidate[flag] is True for flag in changed)
    for field in fixed:
        assert candidate[field] == control[field], f'incomparable setting: {field}'
trajectory_match = control.get('replay_state_blake3') is not None and all(
    c.get('replay_state_blake3') == control['replay_state_blake3']
    and c.get('replay_render_frames') == control.get('replay_render_frames')
    and c['feet'] == control['feet']
    and c.get('measurement_start_simulation_steps') == control.get('measurement_start_simulation_steps')
    and c.get('simulation_clock') == control.get('simulation_clock') for c in candidates)
if world_ray:
    assert trajectory_match, 'world-ray cycles must have identical replay states'
fps_field='mean_fps' if world_ray else 'sampled_mean_fps'
result = {'optimization': args.optimization, 'world_ray_mode': world_ray, 'fps_clock': 'whole_measurement_wall_time' if world_ray else 'legacy_sample_sum',
          'isolation_established': False,
          'control_offscreen_fps': control[fps_field],
          'candidate_offscreen_fps': [c[fps_field] for c in candidates],
          'observed_relative_fps_gain': [c[fps_field] / control[fps_field] - 1
                                         for c in candidates],
          'quality_settings_match': True, 'trajectory_matches': trajectory_match,
          'qualified_speedup': False, 'complete_motion_cycles': True,
          'statistical_confidence_established': False,
          'minimum_120_presented_fps_passed': False}
print(json.dumps(result, indent=2))
