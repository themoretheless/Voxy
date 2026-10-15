"""Join independently computed positive-view and exact-grazing references."""
import argparse,json,struct,subprocess
from pathlib import Path
from ggx_grazing_reference import grazing_moments

def main():
 p=argparse.ArgumentParser();p.add_argument('gpu',type=Path);p.add_argument('native');p.add_argument('output',type=Path);a=p.parse_args()
 rows=[json.loads(l) for l in a.gpu.read_text().splitlines()];points=[r for r in rows if 'nv' in r]
 positive=[r for r in points if r['nv']>0];grazing=[r for r in points if r['nv']==0]
 stem=a.output.with_suffix('');source=Path(str(stem)+'-positive-input.jsonl');result=Path(str(stem)+'-positive.jsonl')
 source.write_text(''.join(json.dumps(r)+'\n' for r in positive))
 with result.open('w') as out,Path(str(stem)+'-native.stderr').open('w') as err:
  status=subprocess.run([a.native,str(source)],stdout=out,stderr=err).returncode
 assert status in (0,1),'native reference failed before comparison'
 refs=[json.loads(l) for l in result.read_text().splitlines() if '"nv"' in l]
 for r in grazing:
  roughness=struct.unpack('<f',struct.pack('<f',r['roughness']))[0];g=grazing_moments(roughness)
  errors=g.get('estimated_errors',[0.,0.]);ref={'A':g['A'],'B':g['B'],'loss':0.,'radial_estimated_error':0.,'bias_radial_estimated_error':errors[1]}
  refs.append(dict(nv=0.,roughness=roughness,samples=r['samples'],reference=ref,azimuth_refinement_difference=0.,bias_azimuth_refinement_difference=0.))
 assert len(refs)==len(points),'reference coverage incomplete'
 a.output.write_text(''.join(json.dumps(r)+'\n' for r in refs))
 print(json.dumps(dict(cases=len(refs),positive_cases=len(positive),grazing_cases=len(grazing),scope='native adaptive positive-view reference plus independent collapsed-azimuth grazing reference')))

if __name__=='__main__':main()
