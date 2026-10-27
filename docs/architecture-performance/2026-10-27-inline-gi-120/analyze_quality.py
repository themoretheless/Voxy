"""Declared native quantization bounds, never an assertion of bit identity."""
import json,sys
from pathlib import Path
import numpy as np
COLOUR={'scene-before-taa','taa-current','taa-previous','temporal-bloom','display-before-downsample','final-output'}
def analyze(root):
 policy=json.loads((root.parent/'quality-policy.json').read_text()); rows=[]; passed=True; exact=True
 for frame in policy['frames']:
  roots=[root/name/f'frame-{frame}' for name in ['A0','A1','B']]
  proofs=[json.loads((p/'stages.json').read_text()) for p in roots]
  uniforms=[p['temporal_uniform']['blake3'] for p in proofs]
  passed &= len(set(uniforms))==1
  specs=[{s['stage']:s for s in p['stages']} for p in proofs]
  if any(set(s)!=set(specs[0]) for s in specs):raise ValueError('different stage sets')
  for stage in specs[0]:
   entries=[s[stage] for s in specs]; colour=stage in COLOUR
   if len({(s['source_format'],tuple(s['size']),s.get('captured_channels')) for s in entries})!=1:raise ValueError('stage format changed')
   def read(p, entry):
    native=p/f'{stage}.native.bin'
    if native.exists():
     dtype='<f2' if entry['source_format'] in {'Rgba16Float','Rg16Float'} else '<f4'
     return np.fromfile(native,dtype=dtype).astype(np.float64)
    suffix='r32.bin' if entry['captured_channels']==1 else 'rgba32.bin'
    return np.fromfile(p/f'{stage}.{suffix}',dtype='<f4').astype(np.float64)
   arrays=[read(p,e) for p,e in zip(roots,entries)]
   a,aa,b=arrays
   expected=entries[0]['size'][0]*entries[0]['size'][1]*({'Rgba16Float':4,'Rg16Float':2,'R32Float':1,'Rg32Float':2,'Depth32Float':1}.get(entries[0]['source_format'],4) if (roots[0]/f'{stage}.native.bin').exists() else entries[0]['captured_channels'])
   if any(len(v)!=expected for v in arrays):raise ValueError('wrong native channel count')
   finite=all(np.isfinite(v).all() for v in arrays)
   lo=np.minimum(a,aa);hi=np.maximum(a,aa);excess=np.maximum(np.maximum(lo-b,b-hi),0.)
   allowance=np.zeros_like(a)
   if colour:
    if entries[0]['source_format']!='Rgba16Float':raise ValueError('policy requires half float colour source')
    scale=np.maximum(np.abs(a),np.abs(aa));exponent=np.frexp(scale)[1]
    ulp=np.where(scale==0.,2.**-24,np.exp2(np.maximum(exponent-11,-24)))
    limit=policy['scene_before_taa_max_excess_over_pixelwise_aa_envelope_source_half_ulps'] if stage=='scene-before-taa' else policy['postprocess_max_excess_over_pixelwise_aa_envelope_source_half_ulps']
    allowance=ulp*limit
   outside=int(np.count_nonzero(excess>allowance)); aa_changed=int(np.count_nonzero(a!=aa));ab_changed=int(np.count_nonzero(a!=b))
   exact &= aa_changed==0 and ab_changed==0
   ok=bool(finite and outside==0)
   if stage=='scene-before-taa':ok &= aa_changed/len(a)<=policy['max_scene_aa_changed_channel_fraction']
   passed &= ok
   rows.append(dict(frame=frame,stage=stage,source_format=entries[0]['source_format'],channels=len(a),aa_changed=aa_changed,ab_changed=ab_changed,aa_max_abs=float(np.max(np.abs(a-aa))),ab_max_abs=float(np.max(np.abs(a-b))),outside_envelope_and_quantization=outside,max_excess=float(np.max(excess)),finite=bool(finite),passed=bool(ok),uniforms_equal=len(set(uniforms))==1))
 return dict(scope='native source-stage quantization bound plus per-channel A/A envelope; NOT bit-exact parity',native_quality_bound_passed=bool(passed),strict_bit_parity_passed=bool(exact),rows=rows,policy=policy)
if __name__=='__main__':print(json.dumps(analyze(Path(sys.argv[1])),indent=2))
