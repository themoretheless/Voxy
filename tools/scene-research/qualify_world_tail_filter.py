#!/usr/bin/env python3
"""CPU prototype: prove a non-dyadic hit parameter without computing its quotient.

Every finite f32 and every finite sum of f32 values is dyadic. Map exact source
coefficients into F3/F5: if D maps to zero but T does not, T/D has an odd factor
in its reduced denominator and cannot be fully represented by two f32 words.
This is an inconclusive-or-proved-tail filter, never a geometric hit/miss test.
It is not connected to GPU queries or rendering by this prototype.
"""
import argparse
import json
import struct
from fractions import Fraction as Q
from functools import lru_cache
from pathlib import Path
from audit_world_triangle_case import word, cross, dot, sub


@lru_cache(None)
def residue(raw, prime):
    biased = raw >> 23 & 255
    if biased == 255:
        raise ValueError('nonfinite coefficient')
    mantissa = raw & 0x7fffff
    power = -149
    if biased:
        mantissa |= 0x800000
        power = biased - 150
    # 300 is divisible by both prime-1 periods and makes all finite exponents positive.
    result = (mantissa % prime) * pow(2, (power + 300) % (prime - 1), prime) % prime
    return (-result if raw >> 31 else result) % prime


def qmod(value, prime):
    return value.numerator % prime * pow(value.denominator % prime, -1, prime) % prime


def modular_vectors(mesh, primitive, ray, prime):
    matrix, vertices, indices = mesh
    points = []
    for vertex in indices[primitive * 3:primitive * 3 + 3]:
        local = vertices[vertex]
        points.append(tuple((residue(matrix[12 + axis], prime) + sum(
            residue(matrix[column * 4 + axis], prime) * residue(local[column], prime)
            for column in range(3))) % prime for axis in range(3)))
    origin = tuple((residue(ray[k], prime) + residue(ray[4 + k], prime)) % prime for k in range(3))
    direction = tuple((residue(ray[8 + k], prime) + residue(ray[12 + k], prime)) % prime for k in range(3))
    e1, e2 = sub(points[1], points[0]), sub(points[2], points[0])
    return dot(e2, cross(sub(origin, points[0]), e1)) % prime, dot(e1, cross(direction, e2)) % prime


def exact_vectors(mesh, primitive, ray):
    matrix, vertices, indices = mesh
    points = []
    for vertex in indices[primitive * 3:primitive * 3 + 3]:
        local = vertices[vertex]
        points.append(tuple(word(matrix[12 + axis]) + sum((
            word(matrix[column * 4 + axis]) * word(local[column]) for column in range(3)), Q(0))
                            for axis in range(3)))
    origin = tuple(word(ray[k]) + word(ray[4 + k]) for k in range(3))
    direction = tuple(word(ray[8 + k]) + word(ray[12 + k]) for k in range(3))
    e1, e2 = sub(points[1], points[0]), sub(points[2], points[0])
    return dot(e2, cross(sub(origin, points[0]), e1)), dot(e1, cross(direction, e2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    directed = [0, 0x80000000, 1, 0x80000001, 0x7fffff, 0x800000,
                0x800001, 0x3f800000, 0x3eaaaaab, 0x7f7fffff, 0xff7fffff]
    seed = 0x564f5859
    for _ in range(10000):
        seed = (seed * 1664525 + 1013904223) & 0xffffffff
        if seed & 0x7f800000 != 0x7f800000:
            directed.append(seed)
    for raw in directed:
        for prime in [3, 5]:
            assert residue(raw, prime) == qmod(word(raw), prime)
    def bits(value):
        return struct.unpack('<I', struct.pack('<f', value))[0]
    directed_distances = 0
    identity = tuple(bits(v) for v in [1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1])
    for plane, dz, expected in [(0,60,False),(0.9375,60,False),(30,60,False),(60,60,False),(20,60,True),(12,60,True),(1,7,False)]:
        mesh = (identity, [tuple(bits(v) for v in point) for point in [(-1,-1,plane),(1,-1,plane),(-1,1,plane)]], [0,1,2])
        ray = [0]*20
        ray[10] = bits(dz)
        t,d = exact_vectors(mesh,0,ray)
        verdict = any((lambda v:v[1]==0 and v[0]!=0)(modular_vectors(mesh,0,ray,prime)) for prime in [3,5])
        assert verdict == expected
        if verdict:
            value=t/d
            assert value.denominator & (value.denominator-1) != 0
        directed_distances += 1
    manifest = json.loads((args.root / 'manifest.json').read_text())
    sources = []
    for item in manifest['meshes']:
        raw = (args.root / item['vertices']['file']).read_bytes()
        vertices = [struct.unpack_from('<3I', raw, i * 48) for i in range(item['vertex_count'])]
        indices = list(struct.unpack('<' + 'I' * item['index_count'], (args.root / item['indices']['file']).read_bytes()))
        instances = (args.root / item['instances']['file']).read_bytes()
        for i in range(item['instance_count']):
            sources.append((struct.unpack_from('<16I', instances, i * 112), vertices, indices))
    frame = json.loads((args.root / 'world-frame.json').read_text())
    reports = []
    for kind in range(2):
        hits = proved = non_dyadic = dyadic = 0
        for index, tile in enumerate(frame['tiles']):
            stem = args.root / f'frame-{kind}-tile-{index}'
            rays = list(struct.iter_unpack('<20I', stem.with_name(stem.name + '-input.bin').read_bytes()))
            data = stem.with_name(stem.name + '-hits.bin').read_bytes()
            width = len(data) // (len(rays) * 4)
            for ray, hit in zip(rays, struct.iter_unpack('<' + 'I' * width, data)):
                if hit[0] != 1:
                    continue
                t, d = exact_vectors(sources[hit[6]], hit[7], ray)
                assert d != 0
                value = t / d
                non_dyadic += int(value.denominator & (value.denominator - 1) != 0)
                dyadic += int(value.denominator & (value.denominator - 1) == 0)
                verdict = False
                for prime in [3, 5]:
                    a, b = modular_vectors(sources[hit[6]], hit[7], ray, prime)
                    assert (a, b) == (qmod(t, prime), qmod(d, prime))
                    verdict |= b == 0 and a != 0
                if verdict:
                    assert value.denominator & (value.denominator - 1) != 0
                    assert hit[15] & 1 != 0
                    proved += 1
                hits += 1
        reports.append({'kind': kind, 'hits': hits, 'non_dyadic': non_dyadic,
                        'dyadic': dyadic, 'proved_tail_without_division': proved})
    result = {'finite_coefficient_cases': len(directed), 'directed_distance_cases': directed_distances, 'primes': [3, 5],
              'modular_values_match_independent_fraction': True, 'queries': reports,
              'GPU_implemented': False, 'frame_speedup_proven': False}
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result))


if __name__ == '__main__':
    main()
