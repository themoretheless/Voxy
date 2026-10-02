#!/usr/bin/env python3
"""Locate connected residual patches from an exact, uncapped static mesh audit.

Components connect forbidden face pairs and involved faces sharing a vertex.
This reports triangle patches, not penetration depth or anatomical segmentation.
"""
import argparse
import collections
import hashlib
import json
from pathlib import Path
import prepare_body_geometry as geometry
import audit_body_adjacent_faces as adjacent


def components(points, faces, pairs):
    pairs = {tuple(sorted(pair)) for pair in pairs}
    if any(len(pair) != 2 or pair[0] < 0 or pair[1] >= len(faces)
           or pair[0] == pair[1] for pair in pairs):
        raise ValueError('invalid residual face pair')
    involved = {face for pair in pairs for face in pair}
    neighbors = {face: set() for face in involved}
    incidence = collections.defaultdict(set)
    for face in involved:
        for vertex in faces[face]:
            incidence[vertex].add(face)
    for ids in incidence.values():
        for face in ids:
            neighbors[face].update(ids - {face})
    for a, b in pairs:
        neighbors[a].add(b)
        neighbors[b].add(a)
    all_incidence = collections.defaultdict(set)
    for face, vertices in enumerate(faces):
        for vertex in vertices:
            all_incidence[vertex].add(face)
    remaining = set(involved)
    result = []
    while remaining:
        seed = min(remaining)
        selected = set()
        pending = [seed]
        while pending:
            face = pending.pop()
            if face in selected:
                continue
            selected.add(face)
            pending.extend(neighbors[face] - selected)
        remaining.difference_update(selected)
        vertices = sorted({vertex for face in selected for vertex in faces[face]})
        ring = sorted({face for vertex in vertices for face in all_incidence[vertex]})
        low = [min(points[v][axis] for v in vertices) for axis in range(3)]
        high = [max(points[v][axis] for v in vertices) for axis in range(3)]
        local_pairs = sorted(pair for pair in pairs if pair[0] in selected)
        result.append({'faces': sorted(selected), 'vertices': vertices,
                       'forbidden_pairs': local_pairs, 'one_ring_faces': ring,
                       'bounds_m': {'minimum': low, 'maximum': high},
                       'extent_mm': [(b-a)*1000 for a, b in zip(low, high)]})
    return result


def diagnose(source, audit):
    digest = hashlib.sha256(source.read_bytes()).hexdigest()
    if digest != audit.get('candidate_sha256', audit.get('candidateSha256')):
        raise ValueError('mesh hash does not match audit')
    residual = audit['remaining']
    if residual['intersection_limit_reached']:
        raise ValueError('capped intersection audit')
    pairs = {tuple(sorted(pair)) for pair in residual['intersection_pairs']}
    pairs.update(tuple(sorted(pair)) for pair in residual.get('adjacent', {}).get('forbidden_pairs', []))
    if len(pairs) != audit.get('after', audit.get('finalCrossings')):
        raise ValueError('inconsistent forbidden-pair union')
    points, faces = geometry.load(source)
    patches = components(points, faces, pairs)
    for a, b in pairs:
        shared = len(set(faces[a]).intersection(faces[b]))
        triangles = ([points[v] for v in faces[a]], [points[v] for v in faces[b]])
        intersects = adjacent.forbidden(*triangles, shared) if shared else geometry.crossing(*triangles)
        if not intersects:
            raise ValueError('reported pair fails exact geometry recheck')
    return {'source': str(source), 'source_sha256': digest, 'forbidden_pairs': len(pairs),
            'components': patches, 'adopted': False, 'reported_pairs_rechecked': True,
            'scope': 'Static face patches from a hash-matched uncapped audit. Listed pairs are rechecked; omitted pairs are not searched. Not penetration depth, internal anatomy or dynamic contact proof.'}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path)
    parser.add_argument('audit', type=Path)
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    report = diagnose(args.source, json.loads(args.audit.read_text()))
    args.report.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'forbidden_pairs': report['forbidden_pairs'], 'components': [
        {'faces': len(patch['faces']), 'pairs': len(patch['forbidden_pairs']),
         'support_faces': len(patch['one_ring_faces']), 'bounds_m': patch['bounds_m']}
        for patch in report['components']]}))
