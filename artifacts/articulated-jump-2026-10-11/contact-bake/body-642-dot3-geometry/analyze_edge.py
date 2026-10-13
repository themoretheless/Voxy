import struct,json,math,pathlib
root=pathlib.Path(__file__).resolve().parent
meta=json.loads((root/'capture-642.json').read_text());m=meta['mvp_columns']
cpu=list(struct.iter_unpack('<9f',(root/'cpu-642.vertices').read_bytes()));gpu=list(struct.iter_unpack('<9f',(root/'gpu-642.vertices').read_bytes()));indices=list(struct.iter_unpack('<3I',(root/'mesh-642.indices').read_bytes()))
def project(v):
 p=list(v[:3])+[1.];c=[sum(m[k*4+j]*p[k] for k in range(4)) for j in range(4)]
 return [(c[0]/c[3]+1)*256,(1-c[1]/c[3])*256,c[2]/c[3]]
pc=[project(v) for v in cpu];pg=[project(v) for v in gpu]
def distance(p,a,b):
 d=[b[k]-a[k] for k in range(2)];l=sum(x*x for x in d)
 t=max(0,min(1,sum((p[k]-a[k])*d[k] for k in range(2))/l)) if l else 0
 return math.hypot(*(p[k]-a[k]-t*d[k] for k in range(2)))
samples=[(190+.375,441+.125),(190+.875,441+.375),(190+.125,441+.625),(190+.625,441+.875)]
rows=[]
for i,ids in enumerate(indices):
 pts=[pc[j] for j in ids]
 if min(p[0] for p in pts)>191 or max(p[0] for p in pts)<190 or min(p[1] for p in pts)>442 or max(p[1] for p in pts)<441:continue
 for k in range(3):
  a,b=ids[k],ids[(k+1)%3]
  for si,p in enumerate(samples):
   d=distance(p,pc[a],pc[b])
   rows.append({'triangle':i,'edge':[a,b],'assumed_sample':si,'sample':p,'cpu_distance_px':d,'gpu_distance_px':distance(p,pg[a],pg[b]),'cpu_projected':[pc[a],pc[b]],'gpu_projected':[pg[a],pg[b]],'cpu_positions':[cpu[a][:3],cpu[b][:3]],'gpu_positions':[gpu[a][:3],gpu[b][:3]]})
rows.sort(key=lambda x:x['cpu_distance_px'])
errors=[math.dist(a[:3],b[:3]) for a,b in zip(cpu,gpu)];worst=max(range(len(errors)),key=errors.__getitem__)
report={'projection':'offline f64 evaluation using captured f32 matrix; does not reproduce hardware clipping/quantization','samples':'assumed standard 4x positions; hardware sample positions not queried','max_world_error_m':errors[worst],'worst_vertex':worst,'nonposition_attribute_mismatches':sum(a[3:]!=b[3:] for a,b in zip(cpu,gpu)),'nearest_edges':rows[:12]}
(root/'edge-analysis.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({k:v for k,v in report.items() if k!='nearest_edges'}));print(json.dumps(rows[:2],indent=2))
