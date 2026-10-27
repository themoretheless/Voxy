#!/usr/bin/env python3
"""Fail-closed full-frame qualification; no trimming, retries or load correction."""
import json
import math
from pathlib import Path
import random
import sys


def analyze(root):
    manifest = json.loads((root / 'manifest.json').read_text())
    schedule = [('aa-start-0', 'A'), ('aa-start-1', 'A')]
    for pair in range(5):
        schedule.extend((f'pair-{pair}-{arm}', arm) for arm in ('AB' if pair % 2 == 0 else 'BA'))
    schedule += [('aa-end-0', 'A'), ('aa-end-1', 'A')]
    errors, runs = [], {}
    if manifest['schedule'] != [list(x) for x in schedule]:
        errors.append('wrong schedule')
    if [(r['name'], r['arm']) for r in manifest['runs']] != schedule:
        errors.append('incomplete/duplicate runs')
    for name, arm in schedule:
        p = root / f'{name}.json'
        if not p.exists():
            errors.append(f'{name}: missing report')
            continue
        d = json.loads(p.read_text())
        samples = d.get('completed_frame_samples_seconds', [])
        valid = (d.get('completed_offscreen_measurement_valid') is True
                 and d.get('frame_sample_clock') == 'current_completed_frame'
                 and d.get('simulation_clock') == 'fixed_one_120Hz_step_per_render'
                 and d.get('resolution') == [1920, 1080]
                 and d.get('internal_resolution') == [1920, 1080]
                 and d.get('source_simplified') is False
                 and d.get('antialiasing') == 'Temporal'
                 and d.get('frames', 0) >= 2400 and d.get('complete_motion_cycles', 0) >= 1
                 and d.get('measured_physics_steps') == d.get('frames')
                 and len(samples) == d.get('frames')
                 and all(math.isfinite(x) and x > 0 for x in samples)
                 and d.get('gpu_samples', 0) > 0
                 and isinstance(d.get('gpu_mean_ms'), (int, float))
                 and math.isfinite(d['gpu_mean_ms'])
                 and d.get('gpu_mean_ms', 0) > 0
                 and not d.get('fx_off')
                 and not d.get('compute_gi')
                 and not d.get('original_gi_queue_requested', False)
                 and not d.get('original_gi_queue_direct_scatter_requested', False)
                 and d.get('gi_input_diagnostic', 0) == 0
                 and not d.get('original_gi_queue_proof')
                 and d.get('correctness_capture_active') is False
                 and (arm!='B' or d.get('gi_quad_cache') is True)
                 and len(d.get('host_completion_intervals_seconds', [])) == len(samples)
                 and all(math.isfinite(x) and x > 0 for x in d.get('host_completion_intervals_seconds', []))
                 and not d.get('paired_depth_dda') and not d.get('ray_capture_enabled'))
        if not valid:
            errors.append(f'{name}: invalid full-cycle workload')
        elif not all(math.isclose(d[k], sorted(samples)[index] * 1000., rel_tol=1e-9)
                     for k, index in [('p50_ms', len(samples) // 2),
                                      ('p99_ms', len(samples) * 99 // 100)]):
            errors.append(f'{name}: quantiles disagree with raw samples')
        runs[name] = d
    if errors:
        return {'qualified_120_fps': False, 'errors': errors}
    if len({d['replay_state_blake3'] for d in runs.values()}) != 1:
        errors.append('physics/weather/water replay differs')
    for run in manifest['runs']:
        if run['exit_code'] != 0:
            errors.append(f"{run['name']}: process failed")
    cadence_p99 = {name: sorted(d["host_completion_intervals_seconds"])[len(d["host_completion_intervals_seconds"])*99//100]*1000. for name,d in runs.items()}
    for name,d in runs.items(): d["host_completion_p99_ms"] = cadence_p99[name]
    controls = []
    for prefix in ['aa-start', 'aa-end']:
        a, b = runs[f'{prefix}-0'], runs[f'{prefix}-1']
        ratios = {k: b[k] / a[k] for k in ['p50_ms', 'p99_ms', 'gpu_mean_ms', 'host_completion_p99_ms']}
        passed = all(1 - limit <= ratios[k] <= 1 + limit
                     for k, limit in [('p50_ms', .10), ('p99_ms', .10), ('gpu_mean_ms', .05), ('host_completion_p99_ms', .10)])
        controls.append({'control': prefix, 'ratios': ratios, 'passed': passed})
    pairs = []
    for i in range(5):
        a, b = runs[f'pair-{i}-A'], runs[f'pair-{i}-B']
        pairs.append({'pair': i, 'before_p99_ms': a['p99_ms'], 'after_p99_ms': b['p99_ms'],
                      'p99_ratio': b['p99_ms'] / a['p99_ms'],
                      'median_ratio': b['p50_ms'] / a['p50_ms'],
                      'gpu_ratio': b['gpu_mean_ms'] / a['gpu_mean_ms']})
    rng = random.Random(20261026)
    bootstrap = sorted(math.exp(sum(math.log(rng.choice(pairs)['p99_ratio'])
                                    for _ in pairs) / len(pairs)) for _ in range(10000))
    ci = [bootstrap[250], bootstrap[9749]]
    cadence_p99 = {name: sorted(d['host_completion_intervals_seconds'])[len(d['host_completion_intervals_seconds'])*99//100]*1000. for name,d in runs.items()}
    gpu_bootstrap = sorted(math.exp(sum(math.log(rng.choice(pairs)['gpu_ratio']) for _ in pairs) / len(pairs)) for _ in range(10000))
    gpu_ci = [gpu_bootstrap[250], gpu_bootstrap[9749]]
    absolute = all(p['after_p99_ms'] <= 8.33 and cadence_p99[f'pair-{p["pair"]}-B'] <= 8.33 for p in pairs)
    proof = json.loads((root.parent / 'quality-route' / 'quality.json').read_text())
    quality = (proof.get('passed') is True and proof.get('native_quality_bound_passed') is True and proof.get('binary_sha256', {}).get('A0') == manifest['binaries']['A']['sha256'] and proof.get('binary_sha256', {}).get('A1') == manifest['binaries']['A']['sha256'] and proof.get('binary_sha256', {}).get('B') == manifest['binaries']['B']['sha256'])
    controls_ok = all(c['passed'] for c in controls)
    return {'qualified_120_fps': not errors and controls_ok and ci[1] <= 1.05 and absolute and quality,
            'candidate_promotion_passed':not errors and controls_ok and ci[1] <= 1.05 and quality and gpu_ci[1] < 1.,
            'paired_gpu_ratio_ci95':gpu_ci,
            'absolute_p99_passed': absolute, 'native_quality_bound_passed':quality, 'strict_bit_parity_passed':proof.get('strict_bit_parity_passed'), 'host_completion_p99_ms':cadence_p99, 'aa_passed': controls_ok,
            'relative_p99_nonregression_passed': not errors and controls_ok and ci[1] <= 1.05,
            'errors': errors, 'controls': controls, 'pairs': pairs,
            'paired_p99_ratio_ci95': ci,
            'scope': 'GPU-completed full-resolution frames; no display scanout qualification'}


if __name__ == '__main__':
    print(json.dumps(analyze(Path(sys.argv[1])), indent=2))
