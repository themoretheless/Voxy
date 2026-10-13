#!/usr/bin/env python3
"""Audit nested, opt-in recovery timings. Partial traces never imply convergence."""
import argparse
import collections
import json
import hashlib
import math
import re
from pathlib import Path

EVENT = re.compile(r'^HAIR RECOVERY TIMING event=(begin|end) phase=(\w+) iteration=(None|Some\(\d+\)) backtrack=(None|Some\(\d+\))(?: elapsed_ms=([0-9.eE+-]+) admission=unclassified)?$')


def analyze(text):
    stack = []
    phases = collections.defaultdict(lambda: {'completed_scopes': 0, 'inclusive_ms': 0., 'exclusive_ms': 0., 'max_ms': 0.})
    closed_root_phases = {}
    pending_root_phases = {}
    roots = 0
    root_ms = 0.
    for number, line in enumerate(text.splitlines(), 1):
        if not line.startswith('HAIR RECOVERY TIMING '):
            continue
        match = EVENT.fullmatch(line)
        if not match:
            raise ValueError(f'malformed timing event at line {number}')
        event, phase, iteration, backtrack, elapsed = match.groups()
        key = (phase, iteration, backtrack)
        if event == 'begin':
            if elapsed is not None:
                raise ValueError(f'begin event has duration at line {number}')
            if not stack:
                pending_root_phases = {}
            stack.append({'key': key, 'children_ms': 0., 'line': number})
            continue
        if not stack or stack[-1]['key'] != key:
            raise ValueError(f'unmatched or interleaved scope at line {number}')
        if elapsed is None or not math.isfinite(float(elapsed)) or float(elapsed) < 0.:
            raise ValueError(f'invalid duration at line {number}')
        current = stack.pop()
        elapsed = float(elapsed)
        exclusive = elapsed - current['children_ms']
        if exclusive < -1e-6:
            raise ValueError(f'children exceed parent duration at line {number}')
        row = phases[phase]
        row['completed_scopes'] += 1
        row['inclusive_ms'] += elapsed
        row['exclusive_ms'] += max(0., exclusive)
        row['max_ms'] = max(row['max_ms'], elapsed)
        pending = pending_root_phases.setdefault(phase, {'completed_scopes': 0, 'inclusive_ms': 0., 'exclusive_ms': 0., 'max_ms': 0.})
        pending['completed_scopes'] += 1
        pending['inclusive_ms'] += elapsed
        pending['exclusive_ms'] += max(0., exclusive)
        pending['max_ms'] = max(pending['max_ms'], elapsed)
        if stack:
            stack[-1]['children_ms'] += elapsed
        else:
            roots += 1
            root_ms += elapsed
            for name, values in pending_root_phases.items():
                target = closed_root_phases.setdefault(name, {'completed_scopes': 0, 'inclusive_ms': 0., 'exclusive_ms': 0., 'max_ms': 0.})
                for field in ('completed_scopes', 'inclusive_ms', 'exclusive_ms'):
                    target[field] += values[field]
                target['max_ms'] = max(target['max_ms'], values['max_ms'])
    return {'scope': 'completed recovery timing scopes only; no convergence, full frame or FPS claim',
            'all_observed_scopes_closed': not stack, 'simulation_completion': 'not inferred; inspect terminal test result and launch receipt', 'completed_root_scopes': roots,
            'completed_root_ms': root_ms,
            'open_scopes': [{'phase': s['key'][0], 'iteration': s['key'][1], 'backtrack': s['key'][2], 'begin_line': s['line']} for s in stack],
            'completed_root_phases': closed_root_phases,
            'phases': dict(sorted(phases.items(), key=lambda item: -item[1]['exclusive_ms']))}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('log', type=Path)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    source = args.log.read_text()
    result = analyze(source)
    result['source_text_sha256'] = hashlib.sha256(source.encode()).hexdigest()
    result['source_log'] = str(args.log.resolve())
    output = json.dumps(result, indent=2) + '\n'
    if args.output:
        args.output.write_text(output)
    else:
        print(output, end='')


if __name__ == '__main__':
    main()
