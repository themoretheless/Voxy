#!/usr/bin/env python3
"""Index velocity-island exports within one paired trajectory log, without guessing physical substeps."""
import argparse
import json
from pathlib import Path
import re


def index(path):
    started = False
    completed = 0
    records = []
    owners = {}
    seen = set()
    capture = re.compile(r'HAIR QR INPUT EXPORT ("[^"\n]+") systems=(\d+) rows=(\d+) refinement=(\d+)')
    for line_number, line in enumerate(path.read_text(errors='replace').splitlines(), 1):
        if line.startswith('HYBRID TEST BACKEND '):
            if started:
                raise ValueError('multiple trajectory runs in one log')
            started = True
        timing = re.match(r'HYBRID HAIR FRAME TIMING frame=(\d+) ', line)
        if timing:
            frame = int(timing[1])
            if not started or frame != completed + 1:
                raise ValueError('missing or reordered frame completion markers')
            completed = frame
        match = capture.match(line)
        if not match or not started:
            continue
        filename = Path(json.loads(match[1]))
        owner = re.fullmatch(r'(native|external)-(\d+)\.vqc', filename.name)
        if not owner:
            continue
        key = str(filename)
        if key in seen:
            raise ValueError('duplicate immutable input export')
        seen.add(key)
        frame = completed + 1
        ordinal_key = (frame, owner[1])
        owners[ordinal_key] = owners.get(ordinal_key, 0) + 1
        records.append({'file': key, 'backend': owner[1], 'observation_index': int(owner[2]),
                        'frame_from_log_order': frame, 'observation_in_frame': owners[ordinal_key],
                        'systems': int(match[2]), 'rows': int(match[3]),
                        'refinement': int(match[4]), 'log_line': line_number})
    return {'schema': 'voxy-velocity-export-log-index-v1', 'log': str(path.resolve()),
            'last_frame_completion_marker': completed, 'exports': records,
            'limits': 'Frame inferred from ordered paired-test completion markers and source loop order. Observation ordinal is not physical substep. Caller must verify velocity-only capture filter and launch provenance. This index does not match row identities or certify physics/FPS.'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('log', type=Path)
    args = parser.parse_args()
    try:
        print(json.dumps(index(args.log), indent=2))
    except (OSError, ValueError) as error:
        parser.exit(2, f'error: {error}\n')


if __name__ == '__main__':
    main()
