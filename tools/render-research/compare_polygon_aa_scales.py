#!/usr/bin/env python3
"""Pixel-exact paired crops; deliberately no automatic aesthetic ranking."""
import argparse
import json
from pathlib import Path
from PIL import Image, ImageDraw

p = argparse.ArgumentParser()
p.add_argument('root', type=Path)
a = p.parse_args()
box = (1000, 130, 1240, 390)
for mode in ('temporal', 'metalfx'):
    reports = [json.loads((a.root / f'scale{s}' / mode / 'report.json').read_text()) for s in (1, 2)]
    for field in ('scene', 'rendered_triangles', 'collision_triangles', 'resolution', 'adapter', 'frames', 'route'):
        assert reports[0][field] == reports[1][field], field
    for s, report in zip((1, 2), reports):
        assert report['render_scale'] == s
        assert report['internal_resolution'] == [1920*s, 1080*s]
    canvas = Image.new('RGB', (510, 605), '#202020')
    draw = ImageDraw.Draw(canvas)
    for row, frame in enumerate((129, 160)):
        y = 20 + row*295
        draw.text((10, y), f'{mode}: frame {frame} / ' + ('reset history' if frame == 129 else 'static warm history'), fill='white')
        for col, scale in enumerate((1, 2)):
            x = 10 + col*250
            draw.text((x, y+15), f'{scale*100}%  |  crop at 1:1', fill='white')
            with Image.open(a.root / f'scale{scale}' / mode / f'frame-{frame:03}.png') as im:
                canvas.paste(im.crop(box), (x, y+30))
    canvas.save(a.root / f'comparison-{mode}.png')
(a.root / 'comparison.json').write_text(json.dumps({'crop_xyxy': box, 'resized': False, 'frames': [129,160], 'timing_comparison_valid': False, 'timing_note': 'Concurrent debug runs; no performance conclusion.', 'quality_note': 'Visual paired crops and exact synthetic coverage test; no scalar image quality claim.'}, indent=2)+'\n')
