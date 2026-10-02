#!/usr/bin/env python3
"""Plot independently audited minimum signed-volume ratios."""
import json
from PIL import Image,ImageDraw
reports=[json.load(open(f'docs/anatomy-hgo-refine-{n}-volume.json')) for n in (0,1)]
im=Image.new('RGB',(1000,450),'#f5f7fa');d=ImageDraw.Draw(im)
d.text((25,20),'Measured minimum tetrahedral volume ratio J across committed physical states',fill='#162638')
x0,y0,w,h=80,350,860,250
for k in range(5):
    j=.965+k*.01;y=y0-h*(j-.965)/.04;d.line((x0,y,x0+w,y),fill='#d9e0e8');d.text((25,y-5),f'{j:.3f}',fill='#465466')
for n,r in enumerate(reports):
    color=['#197ca4','#c74643'][n];points=[(x0+w*s['time_s']/.6,y0-h*(s['minimum_j']-.965)/.04) for s in r['states']]
    d.line(points,fill=color,width=3);d.text((80+400*n,60),f"{r['nodes']} nodes: min J={r['minimum_j']:.6f}",fill=color)
for k in range(7):d.text((x0+w*k/6-10,370),f'{k/10:.1f}',fill='#162638')
d.text((400,390),'Physical time (s)',fill='#162638')
d.text((25,420),'J > 0 in all audited cells. Static volume check only; no self-contact or physiological accuracy claim.',fill='#465466')
im.save('docs/anatomy-hgo-volume-render.png')
