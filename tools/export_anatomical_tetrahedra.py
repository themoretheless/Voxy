#!/usr/bin/env python3
"""Constrained tetrahedralization of a closed HRA structure, preserving its GLB metre frame.
Requires numpy and tetgen==0.8.3. No geometry repair/decimation/convex-hull replacement.
Output VXTM v1: magic, version, point/tet/boundary counts (u32 LE), xyz f64,
positive-oriented tet indices u32, outward boundary triangles u32. JSON audit/provenance.
"""
import argparse
import collections
import hashlib
import json
import struct
from pathlib import Path
import numpy as np
import tetgen
from export_female_organs import parse_glb, accessor_values


def boundary(cells):
    faces = {}
    # For a positively oriented tetrahedron these faces point outward.
    for a, b, c, d in cells.tolist():
        for face in [(b, c, d), (a, d, c), (a, b, d), (a, c, b)]:
            key = tuple(sorted(face))
            faces.setdefault(key, []).append(face)
    if any(len(v) > 2 for v in faces.values()):
        raise ValueError('nonmanifold tetrahedral face')
    for pair in faces.values():
        if len(pair) == 2:
            a, b = pair
            if any(all(a[i] == b[(j+i) % 3] for i in range(3)) for j in range(3)):
                raise ValueError('same-direction interior faces')
    return np.asarray([v[0] for v in faces.values() if len(v) == 1], dtype=np.uint32)


def surface_volume(points, faces):
    origin = points[0]
    a, b, c = (points[faces[:, i]] - origin for i in range(3))
    return float(np.einsum('ij,ij->i', a, np.cross(b, c)).sum() / 6.)


def surface_area(points, faces):
    return float(np.linalg.norm(np.cross(points[faces[:,1]]-points[faces[:,0]],
                                        points[faces[:,2]]-points[faces[:,0]]), axis=1).sum()/2)


def boundary_distance(point, triangles):
    a, b, c = triangles[:,0], triangles[:,1], triangles[:,2]
    ab, ac = b-a, c-a
    n = np.cross(ab,ac); nn = np.einsum('ij,ij->i',n,n)
    if (nn <= 0).any(): raise ValueError('zero-area source triangle')
    signed = np.einsum('ij,ij->i',point-a,n)
    projected = point-n*(signed/nn)[:,None]
    v = projected-a
    aa = np.einsum('ij,ij->i',ab,ab); bb = np.einsum('ij,ij->i',ac,ac)
    cc = np.einsum('ij,ij->i',ab,ac)
    av = np.einsum('ij,ij->i',ab,v); bv = np.einsum('ij,ij->i',ac,v)
    denom = nn
    u, w = (av*bb-bv*cc)/denom, (bv*aa-av*cc)/denom
    inside = (u>=-1e-10)&(w>=-1e-10)&(u+w<=1+1e-10)
    distances = np.where(inside,signed*signed/nn,np.inf)
    for x,y in [(a,b),(b,c),(c,a)]:
        edge=y-x; t=np.clip(np.einsum('ij,ij->i',point-x,edge)/np.einsum('ij,ij->i',edge,edge),0,1)
        delta=point-(x+t[:,None]*edge)
        distances=np.minimum(distances,np.einsum('ij,ij->i',delta,delta))
    return float(np.sqrt(distances.min()))


def export(args):
    if not 1.0 < args.minratio <= 10.0: raise ValueError('invalid TetGen quality ratio')
    source,binary,sha = parse_glb(args.source)
    manifest=json.loads((args.source.parent/'manifest.json').read_text())
    matches=[g for g in manifest['groups'].values() if g['file']==args.source.name and g['sha256']==sha]
    if len(matches)!=1: raise ValueError('source GLB hash does not match atlas manifest')
    source_nodes = [args.node] + args.include_node
    if len(source_nodes) != len(set(source_nodes)): raise ValueError('duplicate source node')
    primitives = []
    for node in source_nodes:
        candidates=[p for mesh in source['meshes'] for p in mesh['primitives']
                    if p.get('extras',{}).get('source_node')==node]
        if len(candidates)!=1: raise ValueError('choose uniquely identified source primitives')
        primitives.append(candidates[0])
    lookup={}; points=[]; indices=[]
    for primitive in primitives:
        remap=[]
        for p in accessor_values(source,binary,primitive['attributes']['POSITION']):
            if p not in lookup: lookup[p]=len(points); points.append(p)
            remap.append(lookup[p])
        indices.extend(remap[v[0]] for v in accessor_values(source,binary,primitive['indices']))
    points=np.asarray(points,dtype=np.float64)
    faces=np.asarray(indices,dtype=np.int32).reshape(-1,3)
    edges=collections.defaultdict(list)
    for a,b,c in faces.tolist():
        if len({a,b,c}) != 3: raise ValueError('repeated source triangle vertex')
        for x,y in [(a,b),(b,c),(c,a)]: edges[tuple(sorted((x,y)))].append((x,y))
    audit=dict(welded_vertices=len(points),triangles=len(faces),
               boundary_edges=sum(len(v)==1 for v in edges.values()),
               nonmanifold_edges=sum(len(v)>2 for v in edges.values()),
               inconsistent_winding_edges=sum(len(v)==2 and v[0]==v[1] for v in edges.values()))
    audit['closed_oriented_edge_manifold']=all(len(v)==2 and v[0]!=v[1] for v in edges.values())
    if not audit['closed_oriented_edge_manifold']: raise ValueError('source assembly is not closed/oriented manifold')
    if len({tuple(sorted(f)) for f in faces.tolist()}) != len(faces): raise ValueError('duplicate source faces')
    # Reject disconnected/nested surfaces: this exporter has no anatomical hole specification.
    adjacent=collections.defaultdict(set)
    for a,b,c in faces.tolist():
        adjacent[a].update((b,c)); adjacent[b].update((a,c)); adjacent[c].update((a,b))
    seen={0}; stack=[0]
    while stack:
        for node in adjacent[stack.pop()]-seen: seen.add(node); stack.append(node)
    if len(seen)!=len(points): raise ValueError('disconnected source needs explicit region/hole semantics')
    volume=surface_volume(points,faces)
    flipped=volume<0
    if flipped: faces=faces[:,[0,2,1]];volume=-volume
    if not np.isfinite(volume) or volume<=0: raise ValueError('invalid source enclosed volume')
    generator=tetgen.TetGen(points,faces)
    output=generator.tetrahedralize(plc=True,quality=True,minratio=args.minratio,
                                   mindihedral=10,steinerleft=100_000,docheck=True,quiet=True,
                                   nomergefacet=True,nomergevertex=True)
    nodes=np.asarray(output[0],dtype=np.float64); cells=np.asarray(output[1],dtype=np.uint32)
    if cells.shape[1]!=4 or len(cells)>250_000: raise ValueError('invalid/unbounded linear tetrahedral output')
    a,b,c,d=(nodes[cells[:,i]] for i in range(4))
    signed=np.einsum('ij,ij->i',b-a,np.cross(c-a,d-a))/6
    negative=signed<0
    cells[negative]=cells[negative][:,[0,2,1,3]]
    volumes=np.abs(signed)
    if not np.isfinite(nodes).all() or not np.isfinite(volumes).all() or (volumes<=1e-15).any():
        raise ValueError('tetrahedra fail FEM volume threshold')
    exterior=boundary(cells)
    max_distance=max(boundary_distance(p,points[faces]) for p in nodes[np.unique(exterior)])
    area=surface_area(points,faces); out_area=surface_area(nodes,exterior)
    volume_error=abs(float(volumes.sum())-volume)/volume
    area_error=abs(out_area-area)/area
    print(json.dumps(dict(boundary_distance=max_distance,volume_error=volume_error,area_error=area_error,original_volume=volume,tetra_volume=float(volumes.sum()))),flush=True)
    if max_distance>1e-9 or volume_error>1e-7 or area_error>1e-7:
        raise ValueError('tetrahedralization did not preserve source boundary/volume')
    payload=struct.pack('<4sIIII',b'VXTM',1,len(nodes),len(cells),len(exterior))
    payload+=nodes.astype('<f8').tobytes()+cells.astype('<u4').tobytes()+exterior.astype('<u4').tobytes()
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_bytes(payload)
    metadata=dict(format='VXTM/1',units='metres',source_file=str(args.source),source_sha256=sha,
                  source_node=args.node,source_nodes=source_nodes,
                  source_anatomy=primitives[0]['extras'] if len(primitives)==1 else [p['extras'] for p in primitives],source_surface_audit=audit,
                  atlas_source=manifest['source'],atlas_source_sha256=manifest['source_sha256'],
                  atlas_metadata=manifest['metadata'],
                  license='CC BY 4.0',attribution=source['asset'].get('copyright'),
                  original_volume_m3=volume,tetrahedral_volume_m3=float(volumes.sum()),
                  minimum_tet_volume_m3=float(volumes.min()),maximum_tet_volume_m3=float(volumes.max()),
                  source_surface_flipped=bool(flipped),points=len(nodes),tetrahedra=len(cells),boundary_faces=len(exterior),
                  max_boundary_vertex_distance_m=max_distance,relative_volume_error=volume_error,
                  relative_surface_area_error=area_error,sha256=hashlib.sha256(payload).hexdigest(),
                  generator=dict(name='tetgen',version=tetgen.__version__,numpy=np.__version__,
                                 minratio=args.minratio,mindihedral=10,plc=True,docheck=True,
                                 nomergefacet=True,nomergevertex=True))
    args.output.with_suffix('.json').write_text(json.dumps(metadata,indent=2)+'\n')
    print(json.dumps({k:metadata[k] for k in ['source_node','points','tetrahedra','minimum_tet_volume_m3',
                                             'max_boundary_vertex_distance_m','relative_volume_error']}))

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source',type=Path);parser.add_argument('node',type=int);parser.add_argument('output',type=Path)
    parser.add_argument('--minratio',type=float,default=1.4)
    parser.add_argument('--include-node',type=int,action='append',default=[],
                        help='additional boundary patch; weld exact positions only, reject open/overlapping assembly')
    export(parser.parse_args())
