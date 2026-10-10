#!/usr/bin/env python3
"""Convert cumulative qualification counters into per-frame solver workload.

Solver wall times exclude rendering. Counts and timings do not measure FPS.
Only completed timing/counter blocks are summarized; a trailing live block is
reported separately. Counter resets and inconsistent records are rejected.
"""
import argparse
import json
import math
import re
from pathlib import Path

TIMING = re.compile(r'^HYBRID HAIR FRAME TIMING frame=(\d+) native_ms=(\S+) external_ms=(\S+) native_only=(true|false) cpu_control=(true|false)$')
COUNTERS = re.compile(r'^HYBRID JOINT FRAME COUNTERS frame=(\d+) coordinate_calls=(\d+) equality_dispatches=(\d+) cache_hits=(\d+) admitted=(\d+) native_fallbacks=(\d+)$')
SUBMISSIONS = re.compile(r'^HYBRID JOINT SUBMISSION FRAME COUNTERS frame=(\d+) submissions=(\d+) equality_dispatches=(\d+)$')
NAMES = ('coordinate_calls', 'equality_dispatches', 'cache_hits', 'admitted', 'native_fallbacks')


def summarize(path):
    timings, counters, submissions = {}, {}, {}
    order = {name: [] for name in ('timing', 'counter', 'submission')}
    for number, line in enumerate(path.read_text().splitlines(), 1):
        for name, prefix, pattern, store in (
            ('timing', 'HYBRID HAIR FRAME TIMING ', TIMING, timings),
            ('counter', 'HYBRID JOINT FRAME COUNTERS ', COUNTERS, counters),
            ('submission', 'HYBRID JOINT SUBMISSION FRAME COUNTERS ', SUBMISSIONS, submissions),
        ):
            if not line.startswith(prefix):
                continue
            match = pattern.fullmatch(line)
            if not match:
                raise ValueError(f'{path}:{number}: malformed {name} record')
            frame = int(match[1])
            if frame < 1 or frame in store or (order[name] and frame != order[name][-1] + 1) or (not order[name] and frame != 1):
                raise ValueError(f'{path}:{number}: duplicate or noncontiguous {name} frame')
            values = match.groups()[1:]
            if name == 'timing':
                values = (float(values[0]), float(values[1]), values[2] == 'true', values[3] == 'true')
                if any(not math.isfinite(v) or v < 0 for v in values[:2]):
                    raise ValueError(f'{path}:{number}: invalid solver wall time')
            else:
                values = tuple(map(int, values))
            store[frame] = values
            order[name].append(frame)
    if not timings:
        raise ValueError('no completed solver frame timing records')
    frames = sorted(timings.keys() & counters.keys() & submissions.keys())
    if frames != list(range(1, len(frames) + 1)):
        raise ValueError('complete workload frames are not a contiguous prefix')
    all_frames = timings.keys() | counters.keys() | submissions.keys()
    pending = sorted(all_frames - set(frames))
    if len(pending) > 1 or (pending and pending[0] != len(frames) + 1):
        raise ValueError('missing interior frame workload records')
    previous = (0,) * len(NAMES)
    previous_submissions = 0
    result = []
    for frame in frames:
        current = counters[frame]
        submit, dispatch = submissions[frame]
        if dispatch != current[1] or submit > dispatch:
            raise ValueError(f'frame {frame}: inconsistent submission/dispatch counters')
        if any(a < b for a, b in zip(current, previous)) or submit < previous_submissions:
            raise ValueError(f'frame {frame}: cumulative counters decreased')
        delta = dict(zip(NAMES, (a-b for a, b in zip(current, previous))))
        delta['submissions'] = submit - previous_submissions
        if delta['admitted'] + delta['native_fallbacks'] != delta['coordinate_calls']:
            raise ValueError(f'frame {frame}: coordinate calls lack admission outcomes')
        if delta['submissions'] > delta['equality_dispatches']:
            raise ValueError(f'frame {frame}: per-frame submissions exceed dispatches')
        native, external, native_only, cpu_control = timings[frame]
        if native_only or cpu_control:
            raise ValueError(f'frame {frame}: workload is a CPU control, not GPU qualification')
        result.append({'frame': frame, 'native_solver_wall_ms': native,
                       'external_solver_wall_ms': external, **delta})
        previous, previous_submissions = current, submit
    return {'schema': 'voxy-hair-frame-work-v1', 'log': str(path.resolve()),
            'scope': 'completed per-frame solver wall times and counter deltas; excludes rendering; not GPU timestamps or FPS',
            'frames': result, 'pending_workload_frames': pending,
            'rendered_fps_measured': False,
            'qualification_pass_inferred': False}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('log', type=Path)
    args = parser.parse_args()
    print(json.dumps(summarize(args.log), indent=2, allow_nan=False))
