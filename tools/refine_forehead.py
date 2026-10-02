#!/usr/bin/env python3
"""Conforming forehead refinement; preserve the neutral surface and skin shell.

Requires NumPy. This preparer accepts the current one-normal-per-position OBJ,
without UV seams, and writes separate outputs rather than modifying its source.
"""
import argparse
import hashlib
import json
from pathlib import Path

import numpy as np
from prepare_body_geometry import load, audit
from refine_body_landmarks import split_edges


def local(p):
    return abs(p[0]) < .060 and .742 < p[1] < .795 and p[2] > .10


def project(point, triangles):
    u = triangles[:, 1] - triangles[:, 0]
    v = triangles[:, 2] - triangles[:, 0]
    w = point - triangles[:, 0]
    uu = np.sum(u*u, axis=1); uv = np.sum(u*v, axis=1)
    vv = np.sum(v*v, axis=1); den = uu*vv-uv*uv
    wu = np.sum(w*u, axis=1); wv = np.sum(w*v, axis=1)
    y = (vv*wu-uv*wv)/den; z = (uu*wv-uv*wu)/den
    weights = np.stack((1-y-z, y, z), axis=1)
    distance = np.sum((np.sum(triangles*weights[:, :, None], axis=1)-point)**2, axis=1)
    distance[np.any(weights < 0, axis=1)] = np.inf
    for i, j in [(0, 1), (1, 2), (2, 0)]:
        edge = triangles[:, j]-triangles[:, i]
        t = np.clip(np.sum((point-triangles[:, i])*edge, axis=1)/np.sum(edge*edge, axis=1), 0, 1)
        nearest = triangles[:, i]+t[:, None]*edge
        d = np.sum((nearest-point)**2, axis=1); take = d < distance
        ew = np.zeros_like(weights); ew[:, i] = 1-t; ew[:, j] = t
        weights[take] = ew[take]; distance[take] = d[take]
    face = int(np.argmin(distance))
    return face, weights[face].tolist(), float(np.sqrt(distance[face]))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path)
    parser.add_argument('skin', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('output_skin', type=Path)
    parser.add_argument('--edge', type=float, default=.0012)
    args = parser.parse_args()
    if not .0005 <= args.edge <= .002:
        raise ValueError('edge must be between 0.5 and 2 mm')
    text = args.source.read_text()
    points, faces = load(args.source)
    normals = [tuple(map(float, l.split()[1:4])) for l in text.splitlines() if l.startswith('vn ')]
    raw_points = [tuple(map(float, l.split()[1:4])) for l in text.splitlines() if l.startswith('v ')]
    if raw_points != points or len(normals) != len(points) or any(l.startswith('vt ') for l in text.splitlines()):
        raise ValueError('requires unique positions, matching normals, and no UV seams')
    for line in text.splitlines():
        if line.startswith('f ') and any(x.split('//') != [x.split('//')[0]]*2 for x in line.split()[1:]):
            raise ValueError('requires matching position/normal face indices')
    before = audit(points, faces); original_count = len(points)
    parents = list(range(len(faces))); stages = []
    for _ in range(8):
        marked = set()
        for face in faces:
            for a, b in zip(face, face[1:]+face[:1]):
                p = np.asarray(points[a]); q = np.asarray(points[b])
                if local((p+q)/2) and np.linalg.norm(p-q) > args.edge:
                    marked.add(tuple(sorted((a, b))))
        if not marked:
            break
        # split_edges appends midpoints in this same deterministic order.
        for a, b in sorted(marked):
            n = np.asarray(normals[a])+np.asarray(normals[b])
            n /= np.linalg.norm(n)
            normals.append(tuple(n))
        points, faces, parents = split_edges(points, faces, marked, 2_000_000, parents)
        stages.append(dict(split_edges=len(marked), vertices=len(points), triangles=len(faces)))
    after = audit(points, faces)
    for key in ('boundary_edges', 'nonmanifold_edges', 'inconsistent_winding_edges', 'duplicate_triangles'):
        if before[key] != after[key]:
            raise ValueError('topology changed: '+key)
    if after['degenerate_triangles'] or abs(after['area_m2']-before['area_m2']) > 1e-10:
        raise ValueError('neutral surface changed or became degenerate')
    maximum = max(np.linalg.norm(np.asarray(points[a])-points[b]) for f in faces
                  for a, b in zip(f, f[1:]+f[:1]) if local((np.asarray(points[a])+points[b])/2))
    if maximum > args.edge+1e-10:
        raise ValueError('refinement did not converge')
    data = json.loads(args.skin.read_text())
    old = {tuple(np.asarray(b['position'], dtype=np.float32)): b for b in data['bindings']}
    shell = np.asarray(data['positions'])[np.asarray(data['triangles'])]
    bindings = []; distances = []
    for index, point in enumerate(points):
        key = tuple(np.asarray(point, dtype=np.float32))
        if index < original_count:
            b = dict(old[key])
        else:
            face, weights, distance = project(np.asarray(point), shell)
            if not np.isfinite(distance) or min(weights) < 0 or abs(sum(weights)-1) > 1e-8:
                raise ValueError('invalid added skin binding')
            b = dict(triangle=face, weights=weights, fade=1.)
            distances.append(distance)
        b.update(vertex=index, position=list(point)); bindings.append(b)
    data.update(bindings=bindings, render_vertices=len(points),
                max_binding_distance_m=max(data['max_binding_distance_m'], max(distances, default=0)))
    args.output.write_text('# Conforming forehead refinement of the original neutral surface\n'
        + ''.join('v '+' '.join(format(x, '.17g') for x in p)+'\n' for p in points)
        + ''.join('vn '+' '.join(format(x, '.17g') for x in n)+'\n' for n in normals)
        + ''.join('f '+' '.join(f'{i+1}//{i+1}' for i in t)+'\n' for t in faces))
    args.output_skin.write_text(json.dumps(data, separators=(',', ':'))+'\n')
    sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
    report = dict(source_sha256=sha(args.source), source_skin_sha256=sha(args.skin),
                  mesh_sha256=sha(args.output), skin_sha256=sha(args.output_skin),
                  before=before, after=after, stages=stages, maximum_local_edge_m=maximum,
                  preserved_bindings=original_count, added_bindings=len(distances),
                  maximum_added_binding_distance_m=max(distances, default=0), shell_unchanged=True)
    args.output.with_suffix('.audit.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(report))


if __name__ == '__main__':
    main()
