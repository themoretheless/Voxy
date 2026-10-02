#!/usr/bin/env python3
"""Render actual first-step states at a common scale, without exaggeration."""
import sys,math,json
from pathlib import Path
from PIL import Image,ImageDraw
from compare_supported_wall_time import mesh
roots=[Path(f'/tmp/voxy-anatomy-hgo-inherited-refine-{n}') for n in range(3)]
sets=[(mesh(r/'rest.obj')[0],*mesh(r/'loaded-sub-1.obj')) for r in roots]
ref=sets[0][0];axes=sorted(range(3),key=lambda k:max(p[k] for p in ref)-min(p[k] for p in ref),reverse=True);h,v,z=axes
allpts=[p for _,pts,_ in sets for p in pts];lo=[min(p[k] for p in allpts) for k in range(3)];hi=[max(p[k] for p in allpts) for k in range(3)];scale=min(330/(hi[h]-lo[h]),220/(hi[v]-lo[v]));maxu=max(math.dist(p,q) for rest,pts,_ in sets for p,q in zip(rest,pts))
im=Image.new('RGB',(1200,430),'#f5f7fa');d=ImageDraw.Draw(im)
d.text((25,20),'First committed step at 0.025 s / actual geometry / same scale / no displacement magnification',fill='#162638')
for n,(rest,pts,faces) in enumerate(sets):
    cx=200+400*n;cy=180
    def project(p):return (cx+(p[h]-(lo[h]+hi[h])/2)*scale,cy-(p[v]-(lo[v]+hi[v])/2)*scale)
    for f in sorted(faces,key=lambda f:sum(pts[i][z] for i in f)):
        t=sum(math.dist(pts[i],rest[i]) for i in f)/3/maxu;d.polygon([project(pts[i]) for i in f],fill=(int(50+190*t),int(145-85*t),int(200-130*t)))
    d.text((cx-110,310),f'{len(pts)} nodes / max displacement {max(math.dist(p,q) for p,q in zip(rest,pts))*1e3:.3f} mm',fill='#162638')
r=json.load(open('docs/anatomy-hgo-first-step-three-levels.json'));a,b=[p['max_difference_m']*1e6 for p in r['pairs']]
d.text((25,350),f'Fine vs prolonged coarse field: 0->1 = {a:.1f} um; 1->2 = {b:.1f} um',fill='#162638')
d.text((25,380),f'Color: displacement 0 .. {maxu*1e3:.3f} mm. Synthetic material and supports.',fill='#465466')
d.text((25,405),'Only one physical increment compared; the level-2 complete protocol remains in progress.',fill='#465466')
im.save('docs/anatomy-hgo-first-step-three-levels.png')
