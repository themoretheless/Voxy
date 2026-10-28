"""Recover the exact unmodified upstream import acceptance asset, without overwriting work."""
import hashlib
import json
from pathlib import Path
import urllib.request
base=Path(__file__).resolve().parent
repo=base.parent.parent
record=json.loads((base/'locomotion-upstream-provenance.json').read_text())
target=repo/'artifacts'/'locomotion-2026-10-26'/'upstream-robot'
target.mkdir(parents=True,exist_ok=True)
for name,url,digest,limit in [
    ('RobotExpressive.glb',record['source'],record['model_sha256'],64*1024*1024),
    ('RobotExpressive.README.md',record['license_source'],record['license_sha256'],256*1024)]:
    path=target/name
    if path.exists():
        assert hashlib.sha256(path.read_bytes()).hexdigest()==digest,'preserve differing existing file'
        continue
    request=urllib.request.Request(url,headers={'User-Agent':'Voxy-asset-qualification'})
    with urllib.request.urlopen(request,timeout=60) as response:payload=response.read(limit+1)
    assert len(payload)<=limit and hashlib.sha256(payload).hexdigest()==digest,'upstream digest/size mismatch'
    with path.open('xb') as output:output.write(payload)
print('Pinned upstream model and license verified; no geometry, skins or morph channels removed')
