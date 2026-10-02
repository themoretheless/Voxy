"""Render static audit locations, with heuristic regions and explicit scope."""
import json
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from prepare_body_geometry import load
r=json.loads(Path('docs/body-intersection-scale-corrected-audit.json').read_text())
points,faces=load(r['source'])
ids=sorted({i for pair in r['intersection_pairs'] for i in pair})
centers=[tuple(sum(points[j][k] for j in faces[i])/3 for k in range(3)) for i in ids]
fig,axes=plt.subplots(1,2,figsize=(9,9),layout='constrained')
for ax,axis,title in zip(axes,(0,2),('Спереди','Сбоку')):
 ax.scatter([p[axis] for p in points[::4]],[p[1] for p in points[::4]],s=.3,c='#8693a4',alpha=.35,rasterized=True)
 ax.scatter([p[axis] for p in centers],[p[1] for p in centers],s=8,c='#d43f35',label='Треугольники с обнаруженными пересечениями')
 ax.set_aspect('equal');ax.set_title(title);ax.set_xlabel(('x' if axis==0 else 'z')+' (м)');ax.set_ylabel('y (м)');ax.grid(alpha=.15)
fig.suptitle(f"Подготовленная сетка: {len(r['intersection_pairs'])} пар, {len(ids)} треугольников\nСтатическая проверка; грани с общими вершинами исключены",fontsize=12)
fig.savefig('docs/body-intersection-locations.png',dpi=150)
