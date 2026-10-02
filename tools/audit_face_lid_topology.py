"""Audit welded OBJ lid topology; no claim of anatomical segmentation."""
import argparse
from collections import Counter, defaultdict
import csv
import json
from pathlib import Path


def audit(path, output):
    vertices, faces = [], []
    for line in path.read_text().splitlines():
        fields = line.split()
        if not fields:
            continue
        if fields[0] == 'v':
            vertices.append(tuple(map(float, fields[1:4])))
        elif fields[0] == 'f':
            ids = [int(token.split('/')[0]) for token in fields[1:]]
            ids = [i - 1 if i > 0 else len(vertices) + i for i in ids]
            faces.extend((ids[0], ids[i], ids[i + 1]) for i in range(1, len(ids) - 1))
    keys, welded = {}, []
    for vertex in vertices:
        key = tuple(round(value * 10_000_000) for value in vertex)
        welded.append(keys.setdefault(key, len(keys)))
    positions = {welded[i]: vertex for i, vertex in enumerate(vertices)}
    edges, adjacency = Counter(), defaultdict(set)
    for face in faces:
        ids = [welded[i] for i in face]
        for a, b in zip(ids, ids[1:] + ids[:1]):
            if a == b:
                continue
            edges[tuple(sorted((a, b)))] += 1
            adjacency[a].add(b)
            adjacency[b].add(a)
    selected = {i for i, (x, y, z) in positions.items()
                if 0.018 < abs(x) < 0.048 and 0.700 < y < 0.726 and z > 0.120}
    local = [edge for edge in edges if all(i in selected for i in edge)]
    visited, components = set(), []
    for start in sorted(selected):
        if start in visited:
            continue
        pending, component = [start], []
        visited.add(start)
        while pending:
            node = pending.pop()
            component.append(node)
            for neighbor in adjacency[node] & selected:
                if neighbor not in visited:
                    visited.add(neighbor)
                    pending.append(neighbor)
        components.append(component)
    labels = {node: index for index, component in enumerate(components) for node in component}
    summary = dict(source=str(path), source_vertices=len(vertices), triangles=len(faces),
                   welded_vertices=len(keys), lid_vertices=len(selected), lid_edges=len(local),
                   boundary_edges=sum(edges[e] == 1 for e in local),
                   nonmanifold_edges=sum(edges[e] > 2 for e in local),
                   local_components=sorted(map(len, components), reverse=True),
                   region_meters=dict(abs_x=[0.018, 0.048], y=[0.700, 0.726], min_z=0.120),
                   welding_tolerance_meters=1e-7)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.with_suffix('.json').write_text(json.dumps(summary, indent=2) + '\n')
    with output.with_suffix('.csv').open('w', newline='') as stream:
        writer = csv.writer(stream)
        writer.writerow(['vertex', 'x', 'y', 'z', 'component', 'local_degree', 'boundary_degree'])
        for node in sorted(selected):
            writer.writerow([node, *positions[node], labels[node], len(adjacency[node] & selected),
                             sum(edges[tuple(sorted((node, other)))] == 1 for other in adjacency[node])])
    print(json.dumps(summary, indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path)
    parser.add_argument('--output', type=Path, default=Path('/tmp/voxy-lid-source-topology'))
    args = parser.parse_args()
    audit(args.source, args.output)
