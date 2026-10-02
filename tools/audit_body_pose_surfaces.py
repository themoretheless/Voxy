#!/usr/bin/env python3
"""Audit exported actual render poses, without inferring continuous contact."""
import argparse,hashlib,json
from pathlib import Path
import prepare_body_geometry as g
from audit_body_adjacent_faces import audit as adjacent

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('samples',type=Path);parser.add_argument('output',type=Path)
    args=parser.parse_args();manifest=json.loads(args.samples.read_text());reports=[]
    for sample in manifest['samples']:
        path=Path(sample['path']);points,faces=g.load(path)
        report=dict(sample,sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
            topology=g.audit(points,faces),nonadjacent=g.intersections(points,faces,10000),adjacent=adjacent(points,faces))
        reports.append(report)
        args.output.write_text(json.dumps(dict(scope=manifest['scope'],samples=reports),indent=2)+'\n')
        print('time',sample['time_s'],'nonadjacent',len(report['nonadjacent']['intersection_pairs']),
              'adjacent',len(report['adjacent']['forbidden_pairs']),'boundaries',report['topology']['boundary_edges'],flush=True)
