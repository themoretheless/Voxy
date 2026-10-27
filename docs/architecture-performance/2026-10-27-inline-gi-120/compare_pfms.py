import array,json,math,sys
from pathlib import Path
root=Path(sys.argv[1]);out=[]
def load(path):
 with path.open('rb') as f:
  assert f.readline()==b'PF\n';size=f.readline();assert f.readline()==b'-1.0\n';a=array.array('f');a.frombytes(f.read());return a
for frame in [120,600,1200,1800,2520]:
 a=load(root/'A0'/f'frame-{frame}.pfm');aa=load(root/'A1'/f'frame-{frame}.pfm');b=load(root/'B'/f'frame-{frame}.pfm')
 row={'frame':frame,'components':len(a)}
 for name,v in [('AA',aa),('AB',b)]:
  errors=[abs(x-y) for x,y in zip(a,v) if x!=y]
  row[name]={'different_components':len(errors),'max_abs':max(errors,default=0.),'mean_abs':math.fsum(errors)/len(a),'rmse':math.sqrt(math.fsum(x*x for x in errors)/len(a))}
 out.append(row);print(row,flush=True)
(root/'numeric-comparison.json').write_text(json.dumps(out,indent=2)+'\n')
