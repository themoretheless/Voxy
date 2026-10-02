"""Audit exported wall states independently of the Rust contact solver."""
import argparse,collections,hashlib,json
from pathlib import Path
import prepare_body_geometry as geometry
from classify_body_intersections import classify

parser=argparse.ArgumentParser()
parser.add_argument('--meshes',required=True,type=Path)
parser.add_argument('--specimen',required=True,choices=['urethra','vagina'])
parser.add_argument('--sectors',required=True,type=int)
parser.add_argument('--stages',nargs='+',default=['rest','barrier','released'])
parser.add_argument('--output',required=True,type=Path)
args=parser.parse_args()
states=[]
for stage in args.stages:
    path=args.meshes/f'{args.specimen}-{stage}.obj'
    points,faces=geometry.load(path)
    result=geometry.intersections(points,faces,limit=100000)
    counts=collections.Counter()
    for a,b in result['intersection_pairs']:
        counts[classify([points[i] for i in faces[a]],[points[i] for i in faces[b]])]+=1
    states.append(dict(specimen=args.specimen,sectors=args.sectors,stage=stage,
        source=str(path.resolve()),sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
        vertices=len(points),triangles=len(faces),classification_counts=dict(counts),
        candidate_pairs=result['intersection_pairs'],limit_reached=result['intersection_limit_reached'],
        narrowphase_pairs=result['narrowphase_pairs']))
report=dict(scope='Static floating-point classification; shared-vertex pairs excluded; no CCD or anatomy certification.',states=states)
args.output.parent.mkdir(parents=True,exist_ok=True)
args.output.write_text(json.dumps(report,indent=2)+'\n')
for state in states:
    print(state['stage'],state['classification_counts'],'limit_reached=',state['limit_reached'])
