#!/usr/bin/env python3
"""Bounded local candidate repair; always checks crossings globally."""
import argparse
import hashlib
import json
from pathlib import Path
import prepare_body_geometry as g

def support(faces, seeds, rings):
    active = set(seeds)
    for _ in range(rings):
        previous = set(active)
        active.update({v for t in faces if previous.intersection(t) for v in t})
    return active

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path)
    parser.add_argument('reference', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--region', required=True)
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists(): parser.error('candidate already exists')
    points, faces = g.load(args.source)
    original, _ = g.load(args.reference)
    before = g.intersections(points, faces, 10000)
    if before['intersection_limit_reached']: raise ValueError('uncapped audit required')
    seeds = set()
    for a, b in before['intersection_pairs']:
        center = tuple(sum(points[v][k] for t in (faces[a], faces[b]) for v in t)/6 for k in range(3))
        if g.region(center) == args.region:
            seeds.update(faces[a]); seeds.update(faces[b])
    if not seeds: raise ValueError('region has no detected crossings')
    active = support(faces, seeds, 2)
    fixed, report = g.repair_intersections(points, faces, iterations=8,
        max_displacement=.003, reference_points=original, movable_vertices=active,
        smoothing_steps=0, reduce_segment_length=True)
    assert all(p == q for i, (p, q) in enumerate(zip(points, fixed)) if i not in active)
    with args.output.open('x') as stream:
        for p in fixed: stream.write('v '+' '.join(format(x, '.17g') for x in p)+'\n')
        for t in faces: stream.write('f '+' '.join(str(i+1) for i in t)+'\n')
    exported, exported_faces = g.load(args.output)
    assert exported == fixed and exported_faces == faces
    report.update(g.audit(fixed, faces), region=args.region, movable_vertices=len(active),
        source_sha256=hashlib.sha256(args.source.read_bytes()).hexdigest(),
        candidate_sha256=hashlib.sha256(args.output.read_bytes()).hexdigest(),
        export_reload_verified=True, adopted=False)
    args.report.write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps({'before':len(before['intersection_pairs']),
        'after':len(report['repair_remaining']['intersection_pairs']),
        'movable_vertices':len(active), 'steps':report['repair_steps']}))
