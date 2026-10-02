#!/usr/bin/env python3
"""Static SI diagnostics from a captured numerical film state, not a body render."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import numpy as np


def fields(points, triangles, volumes, density):
    points=np.asarray(points,dtype=float)
    raw=np.asarray(triangles)
    volumes=np.asarray(volumes,dtype=float)
    if points.ndim!=2 or points.shape[1]!=3 or not np.all(np.isfinite(points)):
        raise ValueError('invalid film coordinates')
    if raw.ndim!=2 or raw.shape[1]!=3 or not np.issubdtype(raw.dtype,np.integer):
        raise ValueError('triangle indices must be integer triples')
    if raw.size==0 or np.any(raw<0) or np.any(raw>=len(points)):
        raise ValueError('invalid film triangle indices')
    if volumes.shape!=(len(raw),) or not np.all(np.isfinite(volumes)) or np.any(volumes<0):
        raise ValueError('invalid cell volumes')
    if not math.isfinite(density) or density<=0:
        raise ValueError('invalid film density')
    geometry=points[raw]
    areas=np.linalg.norm(np.cross(geometry[:,1]-geometry[:,0],geometry[:,2]-geometry[:,0]),axis=1)*.5
    if not np.all(np.isfinite(areas)) or np.any(areas<=0):
        raise ValueError('degenerate film geometry')
    thickness=volumes/areas
    mass=float(np.sum(volumes)*density)
    if not math.isfinite(mass) or not np.all(np.isfinite(thickness)):
        raise ValueError('nonfinite film diagnostics')
    return areas,thickness,mass


def analyze(snapshot):
    if snapshot.get('capture')!='numericalFilmState' or snapshot.get('filmEnabled') is not True:
        raise ValueError('enabled numerical film capture required')
    state=snapshot['state']
    if state['format']!='voxy.surface-film-state.v1' or state['units']!='SI':
        raise ValueError('unsupported film format or units')
    physics=state['physics']
    areas,thickness,mass=fields(physics['pointsM'],physics['triangles'],physics['cellVolumesM3'],physics['material']['density'])
    measured=state['measurements']['massKg']
    if not math.isfinite(measured) or measured<0 or abs(mass-measured)>max(1e-18,measured*1e-12):
        raise ValueError('captured mass does not match cell volumes')
    return areas,thickness,mass


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source',type=Path)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--report',type=Path,required=True)
    parser.add_argument('--display-min-um',type=float,default=.01)
    args=parser.parse_args()
    if not math.isfinite(args.display_min_um) or args.display_min_um<0:parser.error('display cutoff must be finite and nonnegative')
    snapshot=json.loads(args.source.read_text())
    areas,thickness,mass=analyze(snapshot)
    data=snapshot['state']['physics'];points=np.asarray(data['pointsM']);triangles=np.asarray(data['triangles']);volumes=np.asarray(data['cellVolumesM3'])
    selected=thickness>args.display_min_um*1e-6
    if not np.any(selected):parser.error('no cells above display cutoff')
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    from matplotlib.collections import PolyCollection
    fig,axes=plt.subplots(1,2,figsize=(11,4.8),layout='constrained')
    polygons=points[triangles[selected]][:,:,[0,1]]*1000
    collection=PolyCollection(polygons,array=thickness[selected]*1e6,cmap='viridis',edgecolors='none')
    axes[0].add_collection(collection);axes[0].autoscale_view();axes[0].set_aspect('equal')
    axes[0].set_xlabel('x (mm)');axes[0].set_ylabel('y (mm)');axes[0].set_title('Front projection: thickness = cell volume / 3D area')
    fig.colorbar(collection,ax=axes[0],label='Film thickness (µm)')
    fraction=float(np.sum(volumes[selected])/np.sum(volumes))
    axes[1].hist(thickness[selected]*1e6,bins=25,weights=volumes[selected]/np.sum(volumes),color='#337ca0',edgecolor='white')
    axes[1].set_xlabel('Film thickness (µm)');axes[1].set_ylabel('Fraction of total liquid volume')
    axes[1].set_title(f'Display cutoff > {args.display_min_um:g} µm; {fraction:.6%} of volume')
    fig.suptitle(f'Captured numerical state: frame {snapshot["frame"]}, t = {snapshot["simulationTime"]:.6f} s\nMass {mass*1000:.6f} g; static projection, no renderer image or penetration proof')
    fig.savefig(args.output,dpi=160)
    report={'source_sha256':hashlib.sha256(args.source.read_bytes()).hexdigest(),'cells':len(volumes),'mass_kg':mass,
        'volume_m3':float(np.sum(volumes)),'maximum_thickness_m':float(np.max(thickness)),
        'display_cutoff_um':args.display_min_um,'displayed_cells':int(np.sum(selected)),
        'displayed_cell_area_range_m2':[float(np.min(areas[selected])),float(np.max(areas[selected]))],
        'displayed_max_edge_m':float(np.max(np.linalg.norm(points[triangles[selected]][:,[1,2,0]]-points[triangles[selected]],axis=2))),
        'displayed_volume_fraction':fraction,'frame':snapshot['frame'],'simulation_time_s':snapshot['simulationTime'],
        'output':str(args.output),'scope':'3D cell area thickness; front projection may overlap. Mass agreement is not flow/physiology calibration.'}
    args.report.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))
