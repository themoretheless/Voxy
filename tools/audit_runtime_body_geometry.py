#!/usr/bin/env python3
"""Audit the adopted meshes without rewriting geometry or authored normals."""
import argparse
import re
import collections
import hashlib
import json
from pathlib import Path
from prepare_body_geometry import audit, intersections, load, region

ROOT = Path(__file__).resolve().parents[1]

def runtime_sources(source=ROOT / 'crates/voxy_app/src/female_demo.rs'):
    text = source.read_text()
    female = re.search(r'const BODY: &str = include_str!\("([^"\n]+)"\);', text)
    male = re.search(r'pub\(crate\) fn new_male\(\).*?Self::male_from_assets\(\s*include_str!\("([^"\n]+)"\)', text, re.S)
    if female is None or male is None:
        raise ValueError('unsupported runtime source declarations; cannot audit a guessed mesh')
    return {name: (source.parent / match.group(1)).resolve()
            for name, match in (('female', female), ('male', male))}

def inspect(path):
    points, faces = load(path)
    result = audit(points, faces)
    result.update(intersections(points, faces, limit=10000))
    counts = collections.Counter()
    for a, b in result['intersection_pairs']:
        labels = []
        for face in (faces[a], faces[b]):
            center = tuple(sum(points[i][axis] for i in face)/3 for axis in range(3))
            labels.append(region(center))
        counts[' / '.join(sorted(labels))] += 1
    result.update(source=str(path.relative_to(ROOT)),
                  source_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                  intersection_regions=dict(sorted(counts.items())),
                  region_labels='coordinate heuristics, not anatomical segmentation')
    return result

if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--report',type=Path,default=ROOT / 'docs/body-runtime-geometry-audit.json')
    args=parser.parse_args()
    report = {}
    for name, path in runtime_sources().items():
        report[name] = inspect(path)
        print(name, json.dumps({k:v for k,v in report[name].items()
                              if k not in ('intersection_pairs', 'degenerate_triangles')}), flush=True)
        args.report.write_text(json.dumps(report, indent=2)+'\n')
