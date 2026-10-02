#!/usr/bin/env python3
"""Plot the exact unadopted geometry associated with a completed repair report."""
import argparse,hashlib,json
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.collections import LineCollection
from prepare_body_geometry import load

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source',type=Path)
    parser.add_argument('report',type=Path)
    parser.add_argument('output',type=Path)
    parser.add_argument('--component',type=int,help='Show one connected residual patch instead of the entire body')
    args=parser.parse_args()
    report=json.loads(args.report.read_text())
    digest=report.get('candidate_sha256',report.get('candidateSha256'))
    if hashlib.sha256(args.source.read_bytes()).hexdigest()!=digest:
        parser.error('geometry does not match completed audit')
    points,faces=load(args.source)
    remaining=report['remaining']
    pairs={tuple(p) for p in remaining['intersection_pairs']}
    pairs.update(tuple(p) for p in remaining.get('adjacent',{}).get('forbidden_pairs',[]))
    count=report.get('after',report.get('finalCrossings'))
    if remaining['intersection_limit_reached'] or len(pairs)!=count:
        parser.error('incomplete or inconsistent intersection report')
    if not pairs:parser.error('no residual intersections to plot')
    if args.component is not None:
        from diagnose_body_residuals import components
        patches=components(points,faces,pairs)
        if not 0<=args.component<len(patches):parser.error('component index outside residual patches')
        pairs={tuple(p) for p in patches[args.component]['forbidden_pairs']}
    ids={i for pair in pairs for i in pair}
    residual=[points[v] for i in ids for v in faces[i]]
    low=[min(p[k] for p in residual)-.008 for k in range(3)]
    high=[max(p[k] for p in residual)+.008 for k in range(3)]
    fig,axes=plt.subplots(1,2,figsize=(11,5),layout='constrained')
    for ax,axis,title in zip(axes,(0,2),('Front: x / y','Side: z / y')):
        visible=[p for p in points if all(low[k]<=p[k]<=high[k] for k in range(3))]
        ax.scatter([p[axis]*1000 for p in visible],[p[1]*1000 for p in visible],s=2,c='#8c9aaa',alpha=.4)
        edges=[]
        for i in sorted(ids):
            tri=[points[v] for v in faces[i]]
            edges.extend([[(tri[k][axis]*1000,tri[k][1]*1000),(tri[(k+1)%3][axis]*1000,tri[(k+1)%3][1]*1000)] for k in range(3)])
        ax.add_collection(LineCollection(edges,colors='#c43128',linewidths=.8))
        ax.set_xlim(low[axis]*1000,high[axis]*1000)
        ax.set_ylim(low[1]*1000,high[1]*1000)
        ax.set_aspect('equal');ax.set_title(title);ax.set_xlabel(('x' if axis==0 else 'z')+' (mm)');ax.set_ylabel('y (mm)');ax.grid(alpha=.15)
    selection='' if args.component is None else f'Patch {args.component}: '
    fig.suptitle(f'{selection}{len(pairs)} forbidden pairs; {len(ids)} involved triangles\nStatic candidate audit, including shared-vertex pairs; not adopted')
    fig.savefig(args.output,dpi=160)
    print(json.dumps({'source_sha256':digest,'pairs':len(pairs),'triangles':len(ids),'output':str(args.output)}))

if __name__=='__main__':main()
