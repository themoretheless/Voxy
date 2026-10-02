#!/usr/bin/env python3
"""Extract auditable, topology-preserving organ groups from HRA united-female v1.10 GLB.
No downloaded code or Blender scripts are executed. Requires only Python stdlib.
Usage: python3 tools/export_female_organs.py source.glb output_directory
"""
import argparse
import collections
import hashlib
import json
import math
import struct
from pathlib import Path

SOURCE = 'https://cdn.humanatlas.io/digital-objects/ref-organ/united-female/v1.10/assets/3d-vh-f-united.glb'
ROOTS = {
    'heart': 'VH_F_heart', 'liver': 'VH_F_liver', 'lungs': 'VH_F_lungs',
    'brain': 'Allen_brain', 'small-intestine': 'VH_F_small_intestine',
    'colon': 'VH_F_colon', 'skeleton-partial': 'VH_F_skeletal_system',
}
ROOTS.update({
    'eyes': 'VH_F_eyes', 'optic-nerves': 'VH_F_nerves_of_eye',
    'spinal-cord': 'VH_F_spinal_cord', 'teeth': 'VH_F_tooth_row',
    'vasculature-partial': 'VH_F_blood_vasculature',
    'lymphatic-organs': 'VH_F_lymphatic_system',
    'muscles-partial': 'VH_F_muscular_system',
    'subcutaneous-fat-partial': 'VH_F_subcutaneous_abdominal_adipose_tissue',
    'visceral-fat-partial': 'VH_F_visceral_adipose',
})
ROOTS.update({
    'mammary-glands': 'VH_F_mammary_gland',
    'vagina': 'VH_F_vagina', 'fallopian-tubes': 'VH_F_fallopian_tube',
    'ovaries': 'VH_F_ovary', 'uterus': 'VH_F_uterus',
    'reproductive-ligaments': 'VH_F_ligaments_of_uterus_and_ovaries',
})
COLORS = {
    'heart': [0.70, 0.16, 0.20, 1], 'liver': [0.45, 0.16, 0.10, 1],
    'lungs': [0.82, 0.45, 0.50, 1], 'brain': [0.74, 0.64, 0.55, 1],
    'small-intestine': [0.77, 0.54, 0.32, 1], 'colon': [0.63, 0.43, 0.22, 1],
    'skeleton-partial': [0.90, 0.88, 0.76, 1],
}

COLORS.update({
    'eyes': [0.78, 0.85, 0.90, 1], 'optic-nerves': [0.92, 0.76, 0.30, 1],
    'spinal-cord': [0.90, 0.80, 0.55, 1], 'teeth': [0.95, 0.93, 0.82, 1],
    'vasculature-partial': [0.75, 0.10, 0.12, 1],
    'lymphatic-organs': [0.20, 0.60, 0.30, 1], 'muscles-partial': [0.70, 0.22, 0.20, 1],
    'subcutaneous-fat-partial': [0.92, 0.77, 0.30, 1], 'visceral-fat-partial': [0.86, 0.68, 0.24, 1],
})

COLORS.update({
    'mammary-glands': [0.82, 0.57, 0.48, 1], 'vagina': [0.70, 0.30, 0.35, 1],
    'fallopian-tubes': [0.83, 0.43, 0.45, 1], 'ovaries': [0.79, 0.59, 0.53, 1],
    'uterus': [0.72, 0.33, 0.35, 1], 'reproductive-ligaments': [0.85, 0.76, 0.59, 1],
})


def parse_glb(path):
    data = path.read_bytes()
    magic, version, length = struct.unpack_from('<4sII', data)
    if magic != b'glTF' or version != 2 or length != len(data):
        raise ValueError('invalid GLB header')
    chunks, cursor = {}, 12
    while cursor < length:
        size, kind = struct.unpack_from('<II', data, cursor)
        cursor += 8
        if cursor + size > length or kind in chunks:
            raise ValueError('invalid GLB chunks')
        chunks[kind] = data[cursor:cursor + size]
        cursor += size
    return json.loads(chunks[0x4E4F534A]), chunks[0x004E4942], hashlib.sha256(data).hexdigest()


def accessor_values(source, binary, index):
    a = source['accessors'][index]
    if 'sparse' in a or a.get('normalized'):
        raise ValueError('sparse/normalized audit accessor unsupported')
    v = source['bufferViews'][a['bufferView']]
    fmt = {5123: 'H', 5125: 'I', 5126: 'f'}[a['componentType']]
    components = {'SCALAR': 1, 'VEC3': 3}[a['type']]
    decoder = struct.Struct('<' + fmt * components)
    offset = v.get('byteOffset', 0) + a.get('byteOffset', 0)
    stride = v.get('byteStride', decoder.size)
    return [decoder.unpack_from(binary, offset + i * stride) for i in range(a['count'])]


def topology(source, binary, mesh):
    """Exact-position welding audits triangle edge incidence, without altering geometry."""
    result = []
    for primitive in mesh['primitives']:
        if primitive.get('mode', 4) != 4:
            raise ValueError('non-triangle primitive')
        positions = accessor_values(source, binary, primitive['attributes']['POSITION'])
        if any(not math.isfinite(x) for p in positions for x in p):
            raise ValueError('non-finite geometry')
        indices = [v[0] for v in accessor_values(source, binary, primitive['indices'])]
        if len(indices) % 3 or any(i >= len(positions) for i in indices):
            raise ValueError('invalid triangle indices')
        welded, mapping = {}, []
        for p in positions:
            mapping.append(welded.setdefault(p, len(welded)))
        edges, directed, degenerate = collections.Counter(), collections.Counter(), 0
        for k in range(0, len(indices), 3):
            a, b, c = [mapping[i] for i in indices[k:k + 3]]
            if len({a, b, c}) != 3:
                degenerate += 1
            for u, v in [(a, b), (b, c), (c, a)]:
                edges[tuple(sorted((u, v)))] += 1
                directed[u, v] += 1
        boundary = sum(n == 1 for n in edges.values())
        nonmanifold = sum(n > 2 for n in edges.values())
        winding = sum(n == 2 and directed[u, v] != directed[v, u] for (u, v), n in edges.items())
        result.append(dict(vertices=len(positions), triangles=len(indices) // 3,
                           welded_vertices=len(welded), boundary_edges=boundary,
                           nonmanifold_edges=nonmanifold, inconsistent_winding_edges=winding,
                           repeated_vertex_triangles=degenerate,
                           closed_oriented_edge_manifold=not (boundary or nonmanifold or winding or degenerate)))
    return result


def multiply(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(4)) for j in range(4)] for i in range(4)]


def local_matrix(node):
    if 'matrix' in node:
        return [[node['matrix'][j * 4 + i] for j in range(4)] for i in range(4)]
    x, y, z, w = node.get('rotation', [0, 0, 0, 1])
    rotation = [[1 - 2*(y*y+z*z), 2*(x*y-z*w), 2*(x*z+y*w)],
                [2*(x*y+z*w), 1-2*(x*x+z*z), 2*(y*z-x*w)],
                [2*(x*z-y*w), 2*(y*z+x*w), 1-2*(x*x+y*y)]]
    scale = node.get('scale', [1, 1, 1])
    translation = node.get('translation', [0, 0, 0])
    return [[rotation[i][j] * scale[j] for j in range(3)] + [translation[i]] for i in range(3)] + [[0, 0, 0, 1]]


def extract(source, binary, root, group):
    nodes = source['nodes']
    parent = {}
    for i, n in enumerate(nodes):
        for child in n.get('children', []):
            if child in parent:
                raise ValueError('multiple parents')
            parent[child] = i
    worlds = {}
    active = set()
    def world(i):
        if i in active:
            raise ValueError('node cycle')
        if i not in worlds:
            active.add(i)
            local = local_matrix(nodes[i])
            worlds[i] = multiply(world(parent[i]), local) if i in parent else local
            active.remove(i)
        return worlds[i]
    selected = set()
    def visit(i):
        if i in selected:
            raise ValueError('node cycle')
        selected.add(i)
        for c in nodes[i].get('children', []):
            visit(c)
    visit(root)
    accessors, views, output, primitives, audit = [], [], bytearray(), [], []
    def pack(values, fmt, kind, component):
        while len(output) % 4:
            output.append(0)
        offset = len(output)
        encoder = struct.Struct('<' + fmt)
        for v in values:
            output.extend(encoder.pack(*v))
        views.append(dict(buffer=0, byteOffset=offset, byteLength=len(output)-offset))
        a = dict(bufferView=len(views)-1, componentType=component, count=len(values), type=kind)
        if kind == 'VEC3':
            a['min'] = [min(v[k] for v in values) for k in range(3)]
            a['max'] = [max(v[k] for v in values) for k in range(3)]
        accessors.append(a)
        return len(accessors)-1
    for old in sorted(selected):
        original = nodes[old]
        if 'mesh' not in original:
            continue
        m = source['meshes'][original['mesh']]
        matrix = world(old)
        # glTF transformation into its original world frame. Normals are regenerated
        # by the runtime from transformed geometry, so no approximate normal bake.
        for primitive in m['primitives']:
            source_positions = accessor_values(source, binary, primitive['attributes']['POSITION'])
            positions = [tuple(sum(matrix[i][k]*v[k] for k in range(3)) + matrix[i][3] for i in range(3)) for v in source_positions]
            indices = accessor_values(source, binary, primitive['indices'])
            primitives.append(dict(attributes={'POSITION': pack(positions, 'fff', 'VEC3', 5126)},
                                   indices=pack(indices, 'I', 'SCALAR', 5125), material=0,
                                   extras={'source_node': old, 'name': original.get('name'), **original.get('extras', {})}))
        audit.append(dict(source_node=old, name=original.get('name'),
                          anatomy=original.get('extras', {}), primitives=topology(source, binary, m)))
    gltf = dict(asset={'version': '2.0', 'generator': 'Voxy HRA world-frame organ subset exporter',
                      'copyright': 'Kristen Browne and Heidi Schlehlein, HuBMAP / Human Reference Atlas. CC BY 4.0.'},
                scene=0, scenes=[{'nodes': [0]}], nodes=[{'name': group, 'mesh': 0}],
                meshes=[{'name': group, 'primitives': primitives}], accessors=accessors, bufferViews=views,
                buffers=[{'byteLength': len(output)}],
                materials=[{'name': group, 'pbrMetallicRoughness': {
                    'baseColorFactor': COLORS[group], 'metallicFactor': 0, 'roughnessFactor': 0.8}, 'doubleSided': True}])
    return gltf, output, audit


def write_glb(path, gltf, binary):
    js = json.dumps(gltf, separators=(',', ':')).encode()
    js += b' ' * (-len(js) % 4)
    binary += b'\0' * (-len(binary) % 4)
    path.write_bytes(struct.pack('<4sII', b'glTF', 2, 28 + len(js) + len(binary))
                     + struct.pack('<II', len(js), 0x4E4F534A) + js
                     + struct.pack('<II', len(binary), 0x004E4942) + binary)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('source', type=Path)
    p.add_argument('output', type=Path)
    p.add_argument('--groups', nargs='+', choices=list(ROOTS), help='Extract selected groups, preserving an existing compatible manifest')
    args = p.parse_args()
    source, binary, checksum = parse_glb(args.source)
    args.output.mkdir(parents=True, exist_ok=True)
    manifest = dict(source=SOURCE, source_sha256=checksum, license='CC BY 4.0',
                    metadata='https://cdn.humanatlas.io/digital-objects/ref-organ/united-female/v1.10/metadata.json',
                    groups={}, limitations=['Reference atlas assembly, not a single patient.',
                    'Surface anatomy only; topology audit does not certify a FEM volume.',
                    'Partial skeleton, muscle and vascular coverage; no anal sphincters. No physiological parameters.',
                    'Lymphatic organs are not a complete lymphatic vessel network; blood and lymph fluid are not surface assets.',
                    'Coordinates are the original atlas frame; not registered to the Blender mannequin.'])
    manifest_path = args.output / 'manifest.json'
    if args.groups and manifest_path.exists():
        existing = json.loads(manifest_path.read_text())
        if existing['source_sha256'] != checksum:
            raise ValueError('existing output has a different source checksum')
        manifest['groups'].update(existing['groups'])
    for group, name in ROOTS.items():
        if args.groups and group not in args.groups:
            continue
        matches = [i for i, n in enumerate(source['nodes']) if n.get('name') == name]
        if len(matches) != 1:
            raise ValueError(f'expected one anatomical root {name}: {matches}')
        gltf, data, audit = extract(source, binary, matches[0], group)
        path = args.output / (group + '.glb')
        write_glb(path, gltf, data)
        manifest['groups'][group] = dict(root_name=name, source_root=matches[0],
                                        file=path.name, sha256=hashlib.sha256(path.read_bytes()).hexdigest(), structures=audit)
        print(group, len(audit), 'structures', path.stat().st_size, 'bytes')
    (args.output / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')

if __name__ == '__main__':
    main()
