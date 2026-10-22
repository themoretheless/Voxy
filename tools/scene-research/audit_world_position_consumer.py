#!/usr/bin/env python3
"""Independent rational geometry/position oracle for live source-to-shading controls."""
import argparse
import json
import struct
from fractions import Fraction as Q
from pathlib import Path
from audit_world_triangle_case import word, sub, dot, cross
from exact_closed_world_triangle import classify
from make_world_exact_parameter_gold import exact_parameters, pair


def audit(root):
    reports = []
    tail_prefix='position-tail-image-hit' if (root/'position-tail-image-hit-fixture.json').exists() else 'position-residual-image-hit'
    for prefix in ['position-exact-image-hit', 'position-optical-image-hit',
                   'position-one-sided-image-hit', tail_prefix]:
        fixture = json.loads((root / (prefix+'-fixture.json')).read_text())
        assert fixture['position_consumer'] and not fixture['history_admission']
        rays = list(struct.iter_unpack('<20I', (root/(prefix+'-input.bin')).read_bytes()))
        hits = list(struct.iter_unpack('<20I', (root/(prefix+'-output.bin')).read_bytes()))
        positions = list(struct.iter_unpack('<8I', (root/(prefix+'-position.bin')).read_bytes()))
        assert len(rays) == len(hits) == len(positions) == 16
        triangles = []
        for slot, mesh in enumerate(fixture['source_geometry']):
            matrix = [[word(v) for v in column] for column in mesh['world']]
            points = []
            for raw in mesh['vertices']:
                local = list(map(word, raw)) + [Q(1)]
                points.append([sum((matrix[c][axis]*local[c] for c in range(4)), Q(0)) for axis in range(3)])
            for offset in range(0, len(mesh['indices']), 3):
                triangles.append((slot, offset//3, mesh['material'], [points[i] for i in mesh['indices'][offset:offset+3]]))
        found = admitted = 0
        for index, (ray, hit, position) in enumerate(zip(rays, hits, positions)):
            assert hit[4:6] == ray[18:20]
            if ray[3] == 0:
                assert hit[0] == 3 and position == (0,)*8
                continue
            assert ray[3] == 1
            origin = [word(ray[k])+word(ray[k+4]) for k in range(3)]
            direction = [word(ray[k+8])+word(ray[k+12]) for k in range(3)]
            candidates = []
            unknown = False
            for slot, primitive, material, points in triangles:
                if not classify(points, origin, direction)['exact_hit']:
                    continue
                determinant = dot(sub(points[1],points[0]),cross(direction,sub(points[2],points[0])))
                if fixture['one_sided']:
                    if determinant < 0:
                        continue
                    if determinant == 0:
                        unknown = True
                        continue
                parameters = exact_parameters(points, origin, direction)
                candidates.append((parameters[0],slot,primitive,material,parameters))
            if unknown:
                assert hit[0] == 2 and position == (0,)*8
                continue
            if not candidates:
                assert hit[0] == 0 and position == (0,)*8
                continue
            t,slot,primitive,material,parameters = min(candidates)
            if hit[0] == 2 and hit[1] == 16384:
                assert fixture.get('history_position_admission')
                assert hit[18]&131072 and position == (0,)*8
                assert hit[6:8] == (0xffffffff,)*2 and hit[8:16] == (0,)*8
                actual_points = [origin[k]+direction[k]*t for k in range(3)]
                assert any(p.denominator&(p.denominator-1) for p in actual_points)
                admitted += 1
                continue
            encoded = [pair(value) for value in parameters]
            assert hit[0] == 1 and (hit[6],hit[7],hit[17]) == (slot,primitive,material)
            assert list(hit[8:14]) == [v for words,_ in encoded for v in words]
            assert hit[15] == sum(int(tail)<<k for k,(_,tail) in enumerate(encoded))
            expected = []
            residual = 0
            for axis in range(3):
                components,tail = pair(origin[axis]+direction[axis]*t)
                expected.extend(components)
                residual |= int(tail)<<axis
            assert list(position) == expected+[7,residual]
            if index == 0:
                if not fixture.get('residual_position'):
                    assert t == (Q(1,40) if fixture['tail'] else Q(1,64))
                assert bool(residual) == fixture.get('residual_position',False)
                assert bool(hit[15]&1) == fixture['tail']
            found += 1
        counts = fixture['branch_counts']
        assert sum(counts) == 16
        if fixture.get('residual_position'):
            assert counts[3]+counts[5] > 0 and counts[2] == 0
        elif fixture['optical']:
            assert counts[4] > 0 and counts[2] == 0
        else:
            assert counts[2] > 0
        # Captured actual outputs must remain finite; these checks do not assert
        # photorealism or pixel equivalence to a different sidedness policy.
        for suffix in ['gi','ssr']:
            values = struct.unpack('<'+'f'*4096, (root/(prefix+'-'+suffix+'.bin')).read_bytes())
            assert all(abs(v) <= 3.4028234663852886e38 for v in values)
        reports.append(dict(fixture=prefix,queries=16,exact_source_hits=found,proved_history_position_fallbacks=admitted,
                            original_tuv_and_positions_verified=True,branch_counts=counts))
    return dict(status='live_exact_position_consumer_passed',fixtures=reports,
                original_gpu_rays=True,source_geometry_unsimplified=True,
                one_sided_front_consumed=True,non_dyadic_t_exact_point_consumed=tail_prefix=='position-tail-image-hit',
                world_position_residual_fallback_checked=tail_prefix=='position-residual-image-hit',
                optical_fallback_retained=True,full_street_qualified=False,fps_qualified=False)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root',type=Path)
    parser.add_argument('--output',type=Path,required=True)
    args = parser.parse_args()
    result = audit(args.root)
    args.output.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result))
