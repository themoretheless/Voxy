import sys,json,tempfile,hashlib
from pathlib import Path
root=Path(__file__).resolve().parent
sys.path.insert(0,str(root.parent/'2026-10-27-inline-gi-120'))
from analyze_quality import analyze
candidate=Path(sys.argv[1]).resolve();output=Path(sys.argv[2]);policy=json.loads((root/'quality-policy.json').read_text());policy['frames']=[120]
with tempfile.TemporaryDirectory() as d:
 tmp=Path(d);(tmp/'quality-policy.json').write_text(json.dumps(policy));r=tmp/'route';r.mkdir()
 for name,source in [('A0',root/'quality-route/A0'),('A1',root/'quality-route/A1'),('B',candidate)]: (r/name).symlink_to(source.resolve(),target_is_directory=True)
 result=analyze(r)
result['full_route_qualified']=False;result['scope']='single phase rejection screen only';output.write_text(json.dumps(result,indent=2)+'\n');print(result['native_quality_bound_passed']);print([r for r in result['rows'] if r['stage']=='scene-before-taa'])
