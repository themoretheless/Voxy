#!/usr/bin/env python3
"""Exact analytic plane cases; no model simplification or scene-performance claim."""
import json, struct, sys
from pathlib import Path
root = Path(sys.argv[1])
root.mkdir(parents=True, exist_ok=False)
# Full 16x16 native depth, with a hole at the shared four-cell corner.
depth = [.5] * 256
depth[8 * 16 + 8] = 1.
(root / 'depth.bin').write_bytes(struct.pack('<256f', *depth))
# begin, end-high, end-low, state, exact pixel, exact parameter, original
# uncached DDA event count (not geometric acceptance tolerance).
cases = [
    ((2.25, 1.25, 0.), (6.75, 9.75, 1.), (0.,0.,0.), 1, (4,5), .5, 7),
    ((.5,.5,.125), (15.5,.5,.25), (0.,0.,0.), 1, (-1,-1), 1e30, 16),
    ((.5,.5,.5), (15.5,15.5,.5), (0.,0.,0.), 1, (0,0), 0., 1),
    ((8.,0.,0.), (8.,16.,1.), (0.,0.,0.), 1, (7,7), .5, 8),
    # Hardware high-part direction is zero; low endpoint moves through the plane.
    ((.5,.5,.25), (.5,.5,.25), (0.,0.,.5), 1, (0,0), .5, 1),
    ((.5,.5,.5), (15.5,10.5,.75), (0.,0.,0.), 1, (0,0), 0., 1),
    ((20.,20.,.25), (24.,24.,.75), (0.,0.,0.), 1, (-1,-1), 1e30, 0),
    ((.5,.5,0.), (3.5,.5,.5), (0.,0.,0.), 1, (3,0), 1., 4),
    ((0.,0.,0.), (0.,0.,0.), (0.,0.,0.), 0, (-1,-1), 1e30, 0),
    ((0.,0.,0.), (0.,0.,0.), (0.,0.,0.), 2, (-1,-1), 1e30, 0),
]
segments, hits = bytearray(), bytearray()
for begin, end, end_low, state, pixel, parameter, visited in cases:
    segment = bytearray(176)
    struct.pack_into('<4f',segment,0,*begin,0.)
    struct.pack_into('<4f',segment,16,*end,0.)
    struct.pack_into('<4f',segment,112,*end_low,0.)
    struct.pack_into('<I',segment,160,state)
    segments.extend(segment)
    hits.extend(struct.pack('<2i2f4I',*pixel,parameter,0.,visited,0,0,0))
for kind in ['gi','ssr']:
    (root/(kind+'.segments.bin')).write_bytes(segments)
    (root/(kind+'.hits.bin')).write_bytes(hits)
    (root/(kind+'.order.bin')).write_bytes(struct.pack('<'+str(len(cases))+'I',*range(len(cases))))
(root/'camera.bin').write_bytes(bytes(448))
(root/'tile.bin').write_bytes(struct.pack('<4I',0,0,len(cases),1))
keys=['INTERVAL_CACHE','PLANE_CACHE','POINT_CACHE','EDGE_CACHE','HIERARCHY','CELL_BOUNDS','TILES','FAR_BOUND']
meta={'schema':'voxy-ray-chunk-v1','byte_order':'little','viewport':[16,16],'tile':[0,0,len(cases),1],'tile_index':0,'staged_frame':1,'ray_count_per_kind':len(cases),'ray_kinds':['gi','ssr'],'segment_stride':176,'hit_stride':32,'camera_bytes':448,'constants':[['ARCHIVE_DDA_'+key,0.] for key in keys],'scope':'synthetic projected closed-plane contracts, not rendered motion','full_scene_fps_proven':False,'reference':'analytic binary-exact plane intersections; uncached DDA event counts'}
meta['files']=[{'name':p.name,'bytes':p.stat().st_size} for p in root.iterdir()]
(root/'metadata.json').write_text(json.dumps(meta,indent=2)+'\n')
print(root)
