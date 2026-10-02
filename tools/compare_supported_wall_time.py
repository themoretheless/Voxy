#!/usr/bin/env python3
"""Compare identical pressure/history protocols at common committed physical times.
Preserves raw FEM node identities; never welds, transforms or interpolates meshes.
"""
import argparse
import csv
import hashlib
import io
import json
import math
from pathlib import Path


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def mesh(path):
    points, faces = [], []
    for line in path.read_text().splitlines():
        fields = line.split()
        if fields[:1] == ['v']:
            if len(fields) != 4:
                raise ValueError(f'invalid vertex: {path}')
            points.append(tuple(map(float, fields[1:])))
        elif fields[:1] == ['f']:
            if len(fields) != 4:
                raise ValueError(f'nontriangular face: {path}')
            faces.append(tuple(int(i) - 1 for i in fields[1:]))
    if not points or not faces or any(not math.isfinite(v) for p in points for v in p):
        raise ValueError(f'missing/nonfinite geometry: {path}')
    if any(i < 0 or i >= len(points) for f in faces for i in f):
        raise ValueError(f'invalid topology: {path}')
    return points, faces


def rows(path, snapshot=None):
    data = list(csv.DictReader(io.StringIO((snapshot if snapshot is not None else path.read_bytes()).decode())))
    previous = 0.
    for row in data:
        time = float(row['physical_time_s'])
        if not math.isfinite(time) or time <= previous:
            raise ValueError(f'nonmonotonic physical time: {path}')
        previous = time
        pressure = float(row['applied_support_pressure_pa'])
        volume_ratio = float(row['min_j'])
        if not math.isfinite(pressure) or pressure < 0 or not math.isfinite(volume_ratio) or volume_ratio <= 0:
            raise ValueError(f'invalid pressure/element volume row: {path}')
        residual = float(row['residual_n'])
        if not math.isfinite(residual) or not 0 <= residual <= 1e-7:
            raise ValueError(f'uncertified equilibrium row: {path}')
    if not data:
        raise ValueError(f'no committed rows: {path}')
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--coarse', required=True, type=Path)
    parser.add_argument('--fine', required=True, type=Path)
    parser.add_argument('--coarse-meshes', required=True, type=Path)
    parser.add_argument('--fine-meshes', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--until-time', type=float,
                        help='Explicit committed prefix endpoint; never implies a completed full protocol')
    args = parser.parse_args()
    # A live producer may append after this read. Hash the bytes actually parsed.
    coarse_snapshot, fine_snapshot = args.coarse.read_bytes(), args.fine.read_bytes()
    coarse, fine = rows(args.coarse, coarse_snapshot), rows(args.fine, fine_snapshot)
    observed_ends = [float(coarse[-1]['physical_time_s']), float(fine[-1]['physical_time_s'])]
    if args.until_time is not None:
        end = args.until_time
        if not math.isfinite(end) or end <= 0 or any(t < end-1e-9 for t in observed_ends):
            raise ValueError('prefix endpoint must be positive and already committed in both runs')
        coarse = [r for r in coarse if float(r['physical_time_s']) <= end+1e-9]
        fine = [r for r in fine if float(r['physical_time_s']) <= end+1e-9]
        if not coarse or not fine or any(abs(float(data[-1]['physical_time_s'])-end)>1e-9 for data in [coarse, fine]):
            raise ValueError('prefix endpoint must match a committed time in both runs')
    ref, topology = mesh(args.coarse_meshes / 'rest.obj')
    other_ref, other_topology = mesh(args.fine_meshes / 'rest.obj')
    if ref != other_ref or topology != other_topology:
        raise ValueError('reference geometry/topology differs')
    material = 'material-profile.csv'
    if digest(args.coarse_meshes / material) != digest(args.fine_meshes / material):
        raise ValueError('material coefficients differ')
    if abs(float(coarse[-1]['physical_time_s'])-float(fine[-1]['physical_time_s']))>1e-9:
        raise ValueError('observed physical durations differ')
    comparisons = []
    previous_time=0.
    for row in coarse:
        time = float(row['physical_time_s'])
        matches = [r for r in fine if abs(float(r['physical_time_s']) - time) <= 1e-9]
        if len(matches) != 1:
            raise ValueError(f'missing/ambiguous fine physical time {time}; full matching schedule required')
        match = matches[0]
        interval=[r for r in fine if float(r['physical_time_s'])>previous_time+1e-9 and float(r['physical_time_s'])<=time+1e-9]
        if not interval or any(abs(float(r['applied_support_pressure_pa'])-float(row['applied_support_pressure_pa']))>1e-9 for r in interval):
            raise ValueError(f'pressure protocol differs inside interval ending {time}')
        if any(r['load_law']!=row['load_law'] or r['contact_law']!=row['contact_law'] for r in interval):
            raise ValueError(f'load/contact law differs inside interval ending {time}')
        previous_time=time
        for key in ['load_law', 'contact_law']:
            if row[key] != match[key]:
                raise ValueError(f'{key} differs at {time}')
        if abs(float(row['applied_support_pressure_pa']) - float(match['applied_support_pressure_pa'])) > 1e-9:
            raise ValueError(f'pressure differs at {time}')
        a_path = args.coarse_meshes / (row['stage'] + '.obj')
        b_path = args.fine_meshes / (match['stage'] + '.obj')
        a, af = mesh(a_path)
        b, bf = mesh(b_path)
        if len(a) != len(ref) or len(b) != len(ref) or af != topology or bf != topology:
            raise ValueError(f'committed topology differs at {time}')
        error = [math.dist(p, q) for p, q in zip(a, b)]
        peak = max(math.dist(p, q) for p, q in zip(a, ref))
        maximum_node=max(range(len(error)),key=error.__getitem__)
        comparisons.append(dict(time_s=time, stage=row['stage'], fine_stage=match['stage'],
            support_pressure_pa=float(row['applied_support_pressure_pa']),
            max_node_difference_m=max(error), maximum_difference_node=maximum_node,
            coarse_position_at_maximum_m=a[maximum_node], fine_position_at_maximum_m=b[maximum_node],
            rms_node_difference_m=math.sqrt(sum(e*e for e in error)/len(error)),
            relative_to_coarse_peak_displacement=max(error)/peak if peak else None,
            coarse_mesh_sha256=digest(a_path), fine_mesh_sha256=digest(b_path)))
    report = dict(scope='Temporal refinement at identical reference/materials and common committed times; no interpolation, anatomy or convergence-order certification',
        coarse_csv_sha256=hashlib.sha256(coarse_snapshot).hexdigest(), fine_csv_sha256=hashlib.sha256(fine_snapshot).hexdigest(),
        material_sha256=digest(args.coarse_meshes / material), coarse_steps=len(coarse), fine_steps=len(fine),
        final_time_s=float(coarse[-1]['physical_time_s']), comparisons=comparisons,
        explicit_prefix=args.until_time is not None, source_observed_end_times_s=observed_ends,
        max_node_difference_m=max(r['max_node_difference_m'] for r in comparisons))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'matched_times': len(comparisons), 'max_node_difference_m': report['max_node_difference_m']}))


if __name__ == '__main__':
    main()
