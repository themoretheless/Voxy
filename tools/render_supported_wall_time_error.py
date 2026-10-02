#!/usr/bin/env python3
"""Render measured nodal timestep differences on the raw benchmark mesh."""
import argparse
import json
import math
from pathlib import Path
from PIL import Image, ImageDraw
from compare_supported_wall_time import mesh


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--comparison', type=Path, required=True)
    p.add_argument('--coarse-meshes', type=Path, required=True)
    p.add_argument('--fine-meshes', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    args = p.parse_args()
    report = json.loads(args.comparison.read_text())
    row = max(report['comparisons'], key=lambda r: r['max_node_difference_m'])
    a, faces = mesh(args.coarse_meshes / (row['stage'] + '.obj'))
    b, other_faces = mesh(args.fine_meshes / (row['fine_stage'] + '.obj'))
    if len(a) != len(b) or faces != other_faces:
        raise ValueError('meshes differ in topology')
    error = [math.dist(x, y) for x, y in zip(a, b)]
    peak = max(error)
    if abs(peak-row['max_node_difference_m']) > 1e-12:
        raise ValueError('geometry does not match measured report')
    im = Image.new('RGB', (1100, 670), '#f5f7fa')
    d = ImageDraw.Draw(im)
    d.text((30, 20), 'Measured timestep sensitivity: raw committed FEM nodes', fill='#152638')
    d.text((30, 45), f"t={row['time_s']:.3f} s; pressure={row['support_pressure_pa']:.0f} Pa; maximum={peak*1e6:.3f} um", fill='#152638')
    scope = f"Committed prefix only, through {report['final_time_s']:.3f} s. " if report.get('explicit_prefix') else ''
    d.text((30, 68), scope+'Colors: coordinate differences on finer-step geometry.', fill='#465466')
    def color(value):
        t = min(1., value/peak) if peak else 0.
        return (int(35+220*t), int(130-80*t), int(220-185*t))
    for left, axes, title in [(30, (0, 2), 'X-Z projection'), (560, (1, 2), 'Y-Z projection')]:
        lo = [min(v[k] for v in b) for k in axes]
        hi = [max(v[k] for v in b) for k in axes]
        scale = min(460/(hi[0]-lo[0]), 440/(hi[1]-lo[1]))
        def project(v):
            return (left+250+(v[axes[0]]-(hi[0]+lo[0])/2)*scale,
                    555-(v[axes[1]]-lo[1])*scale)
        d.text((left+170, 96), title, fill='#152638')
        for face in faces:
            d.line([project(b[i]) for i in (*face, face[0])], fill='#c7d1dc', width=1)
        for i in sorted(range(len(b)), key=lambda i: error[i]):
            x, y = project(b[i])
            d.ellipse((x-3, y-3, x+3, y+3), fill=color(error[i]))
        node = row['maximum_difference_node']
        x, y = project(b[node])
        d.ellipse((x-8, y-8, x+8, y+8), outline='#111111', width=2)
        d.text((x+11, y-15), f'node {node}', fill='#111111')
    for i in range(300):
        d.line((30+i, 596, 30+i, 610), fill=color(peak*i/299))
    d.text((30, 615), f'0                                    {peak*1e6:.3f} um', fill='#152638')
    d.text((385, 598), 'No interpolation, welding or displacement magnification.', fill='#465466')
    d.text((30, 644), 'Synthetic tissues; this measures numerical time sensitivity, not anatomical accuracy.', fill='#465466')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    im.save(args.output)


if __name__ == '__main__':
    main()
