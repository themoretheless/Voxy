#!/usr/bin/env python3
"""Render actual exported states and measured spatial differences."""
import argparse,json,math
from pathlib import Path
from PIL import Image,ImageDraw
from compare_supported_wall_time import mesh
p=argparse.ArgumentParser();p.add_argument('--coarse',type=Path,required=True);p.add_argument('--fine',type=Path,required=True);p.add_argument('--report',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
r=json.loads(a.report.read_text()); im=Image.new('RGB',(1200,700),'#f5f7fa');d=ImageDraw.Draw(im)
d.text((30,20),'Atlas ovary geometry / synthetic HGO material / inherited supports',fill='#162638')
d.text((30,45),'Actual hold-2 geometry at 0.3 s; same scale; displacement is not magnified',fill='#465466')
sets=[]
for root in [a.coarse,a.fine]:
    ref,_=mesh(root/'rest.obj');pts,faces=mesh(root/'hold-2.obj');sets.append((ref,pts,faces))
# Orthographic projection, same coordinate axes and pixel scale for both meshes.
axes=sorted(range(3),key=lambda k:max(v[k] for v in sets[0][0])-min(v[k] for v in sets[0][0]),reverse=True);h,v=axes[:2];depth=axes[2]
allpts=[p for _,pts,_ in sets for p in pts];lo=[min(p[k] for p in allpts) for k in range(3)];hi=[max(p[k] for p in allpts) for k in range(3)]
scale=min(420/(hi[h]-lo[h]),260/(hi[v]-lo[v]));umax=max(math.dist(p,q) for ref,pts,_ in sets for p,q in zip(ref,pts))
for panel,(ref,pts,faces) in enumerate(sets):
    cx=300+600*panel;cy=250
    project=lambda q:(cx+(q[h]-(hi[h]+lo[h])/2)*scale,cy-(q[v]-(hi[v]+lo[v])/2)*scale)
    for face in sorted(faces,key=lambda f:sum(pts[i][depth] for i in f)):
        t=sum(math.dist(pts[i],ref[i]) for i in face)/3/umax
        color=(int(50+190*t),int(145-85*t),int(200-130*t))
        d.polygon([project(pts[i]) for i in face],fill=color)
    d.text((cx-100,375),f'{len(pts)} FEM nodes',fill='#162638')
d.text((30,405),f'Color: displacement 0 .. {umax*1e3:.3f} mm; anatomical material parameters are not calibrated',fill='#465466')
x0,y0,w,ht=80,620,1040,150;maxerr=r['max_difference_m']*1e6*1.1
for k in range(4):
    y=y0-ht*k/3;d.line((x0,y,x0+w,y),fill='#d9e0e8');d.text((25,y-5),f'{maxerr*k/3:.0f}',fill='#465466')
d.text((25,445),'um',fill='#162638');points=[(x0+w*q['time_s']/0.6,y0-ht*q['max_difference_m']*1e6/maxerr) for q in r['comparisons']];d.line(points,fill='#c74643',width=3)
for x,y in points:d.ellipse((x-2,y-2,x+2,y+2),fill='#c74643')
d.text((80,650),'0              Full-field difference: fine solution vs prolonged coarse displacement              0.6 s',fill='#162638')
d.text((30,677),'Two meshes quantify spatial sensitivity; no formal convergence order or physiological accuracy claim.',fill='#465466');im.save(a.output)
