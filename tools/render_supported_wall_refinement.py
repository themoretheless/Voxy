#!/usr/bin/env python3
"""Plot measured common-time nodal errors from comparator JSON reports."""
import argparse
import json
from pathlib import Path
from PIL import Image, ImageDraw


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--report', type=Path, action='append', required=True)
    p.add_argument('--label', action='append', required=True)
    p.add_argument('--output', type=Path, required=True)
    args = p.parse_args()
    if len(args.report) != len(args.label):
        raise ValueError('each report needs one label')
    reports = [json.loads(path.read_text()) for path in args.report]
    endpoint = min(r.get('final_time_s',r.get('duration_s')) for r in reports)
    # All curves use exactly the same committed times, without interpolation.
    common = set(round(r['time_s'], 9) for r in reports[0]['comparisons'])
    for report in reports[1:]:
        common &= set(round(r['time_s'], 9) for r in report['comparisons'])
    curves = [[r for r in report['comparisons'] if round(r['time_s'], 9) in common]
              for report in reports]
    if not common:
        raise ValueError('no shared times')
    ymax = max(r['max_node_difference_m']*1e6 for curve in curves for r in curve)*1.1
    im = Image.new('RGB', (1100, 590), '#f5f7fa')
    d = ImageDraw.Draw(im)
    d.text((30, 20), 'Temporal refinement: maximum full-field nodal difference', fill='#152638')
    prefix = any(r.get('explicit_prefix') for r in reports)
    d.text((30, 45), f"{'Committed prefix' if prefix else 'Completed protocol'} through {endpoint:.3f} s; identical load/material/reference checks", fill='#465466')
    x0, y0, w, h = 80, 470, 960, 350
    for k in range(6):
        value = ymax*k/5
        y = y0-h*k/5
        d.line((x0, y, x0+w, y), fill='#d9e0e8')
        d.text((30, y-5), f'{value:.1f}', fill='#465466')
    d.text((15, 95), 'um', fill='#152638')
    d.line((x0, y0-h, x0, y0, x0+w, y0), fill='#152638', width=2)
    for k in range(9):
        t = endpoint*k/8
        x = x0+w*k/8
        d.text((x-12, y0+10), f'{t:.2f}', fill='#465466')
    d.text((500, 510), 'Physical time (s)', fill='#152638')
    colors = ['#c74643', '#197ca4', '#6d49a5']
    for i, (curve, label) in enumerate(zip(curves, args.label)):
        color = colors[i % len(colors)]
        points = [(x0+w*r['time_s']/endpoint, y0-h*r['max_node_difference_m']*1e6/ymax) for r in curve]
        d.line(points, fill=color, width=3)
        for x, y in points:
            d.ellipse((x-3, y-3, x+3, y+3), fill=color)
        d.text((80+i*310, 78), label, fill=color)
    d.text((30, 552), 'Measured at shared committed times; no interpolation, formal convergence order or anatomical accuracy claim.', fill='#465466')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    im.save(args.output)


if __name__ == '__main__':
    main()
