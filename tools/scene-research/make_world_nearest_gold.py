#!/usr/bin/env python3
"""Exhaustive original-source world nearest oracle. Gold never enters GPU inputs."""
import os
import math
import json
import struct
import sys
from pathlib import Path
from fractions import Fraction as Q
from audit_world_triangle_case import decode_geometry, word
from exact_closed_world_triangle import classify
from make_world_exact_parameter_gold import exact_parameters, pair


def material_policy(material):
    values, image = material
    alpha, transmission, cutoff = [word(values[k]) for k in (3,14,15)]
    if not 0 <= alpha <= 1 or transmission < 0:
        return None
    kind = 0
    if cutoff >= 0:
        if image is not None:
            return None
        if alpha < cutoff:
            return False, 0
        kind = 1
    if cutoff == -2:
        if image is not None and word(values[17]) <= Q(3, 2):
            return None
        if alpha == 0 and transmission == 0:
            return False, 0
        kind = 2
    if transmission > 0:
        kind = 2
    return True, kind


def build(root, cases):
    manifest = json.loads((root/'manifest.json').read_text())
    admission = os.environ.get('VOXY_NEAREST_MATERIALS') == '1'
    materials = []
    if admission:
        for material in manifest['materials']:
            values = struct.unpack('<20I', (root/material['parameters']['file']).read_bytes())
            materials.append((values, material['images'][0]))
    inactive = set(map(int, filter(None, os.environ.get("VOXY_NEAREST_ORACLE_INACTIVE", "").split(","))))
    sources = []
    slot = 0
    for mesh in manifest['meshes']:
        vertices = (root/mesh['vertices']['file']).read_bytes()
        indices = struct.unpack('<'+'I'*mesh['index_count'], (root/mesh['indices']['file']).read_bytes())
        instances = (root/mesh['instances']['file']).read_bytes()
        positions = [struct.unpack_from('<3f', vertices, i*48) for i in range(mesh['vertex_count'])]
        local_bounds = [(min(p[k] for p in positions), max(p[k] for p in positions)) for k in range(3)]
        local_triangles = [[positions[indices[primitive*3+k]] for k in range(3)] for primitive in range(len(indices)//3)]
        for instance in range(mesh['instance_count']):
            matrix = struct.unpack_from('<16f', instances, instance*112)
            world = [matrix[k:k+4] for k in range(0,16,4)]
            bounds = []
            for axis in range(3):
                lo = hi = world[3][axis]
                for k in range(3):
                    products = [world[k][axis]*x for x in local_bounds[k]]
                    lo = math.nextafter(lo+min(products), -math.inf)
                    hi = math.nextafter(hi+max(products), math.inf)
                bounds.append((lo, hi))
            if slot in inactive:
                slot += 1
                continue
            sources.append((slot, mesh['material'], world, local_triangles, bounds))
            slot += 1
    rows = []
    for case in cases:
        winner = None
        unknown = False
        words = case['ray_words']
        from audit_world_triangle_case import word
        origin_q = [word(words[k])+word(words[k+4]) for k in range(3)]
        direction_q = [word(words[k+8])+word(words[k+12]) for k in range(3)]
        intervals = lambda values: [(math.nextafter(float(x), -math.inf), math.nextafter(float(x), math.inf)) for x in values]
        origins, directions = intervals(origin_q), intervals(direction_q)
        tested = 0
        for slot, material, world, local_triangles, bounds in sources:
            tlo, thi = 0., 1.
            rejected = False
            for (blo, bhi), (olo, ohi), (dlo, dhi) in zip(bounds, origins, directions):
                # Enclose the complete closed segment, including exact zero directions.
                swept_lo = math.nextafter(olo+min(0., dlo), -math.inf)
                swept_hi = math.nextafter(ohi+max(0., dhi), math.inf)
                if swept_lo > bhi or swept_hi < blo:
                    rejected = True
                    break
                if dlo <= 0 <= dhi:
                    if olo > bhi and dhi <= 0 or ohi < blo and dlo >= 0:
                        rejected = True
                    continue
                nlo = math.nextafter(blo-ohi, -math.inf)
                nhi = math.nextafter(bhi-olo, math.inf)
                quotients = [n/d for n in (nlo,nhi) for d in (dlo,dhi)]
                tlo = max(tlo, math.nextafter(min(quotients), -math.inf))
                thi = min(thi, math.nextafter(max(quotients), math.inf))
                if tlo > thi:
                    rejected = True
                    break
            if rejected:
                continue
            for primitive, local in enumerate(local_triangles):
                tested += 1
                points, origin, direction = decode_geometry(dict(world=world, local_triangle=local, ray_words=words))
                if not classify(points, origin, direction)['exact_hit']:
                    continue
                kind = 0
                if admission:
                    policy = material_policy(materials[material])
                    if policy is None:
                        unknown = True
                        continue
                    accepted, kind = policy
                    if not accepted:
                        continue
                values = exact_parameters(points, origin, direction)
                key = (values[0], slot, primitive)
                if winner is None or key < winner[0]:
                    winner = key, material, values, kind
        if unknown:
            winner = None
        row = dict(global_ray=case['global_ray'], exact_hit=winner is not None,
                   winner_slot=0xffffffff, winner_primitive=0xffffffff, winner_material=0xffffffff,
                   parameter_words=None, residual_mask=0, source_triangles_tested=tested, source_triangles_total=sum(len(s[3]) for s in sources))
        if winner:
            key, material, values, kind = winner
            pairs = [pair(v) for v in values]
            row.update(winner_slot=key[1], winner_primitive=key[2], winner_material=material,
                       parameter_words=[word for words, _ in pairs for word in words],
                       residual_mask=sum(int(rem)<<k for k, (_, rem) in enumerate(pairs)),
                       exact_parameters=list(map(str, values)))
        if admission:
            row['expected_state'] = 2 if unknown else int(winner is not None)
            row['material_kind'] = winner[3] if winner else 0
        rows.append(row)
    return dict(rows=rows, oracle='outward interval instance pruning + exact source Fraction + RN-even twice', cases=len(rows))


if __name__ == '__main__':
    root, cases_path, output = map(Path, sys.argv[1:])
    cases = [json.loads(line) for line in cases_path.read_text().splitlines() if line]
    result = build(root, cases)
    with output.open('x') as stream:
        json.dump(result, stream, indent=2)
    print(json.dumps(dict(cases=len(cases), hits=sum(row['exact_hit'] for row in result['rows']))))
