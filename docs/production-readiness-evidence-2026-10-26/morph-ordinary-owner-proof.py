#!/usr/bin/env python3
"""Qualify ordinary editor GPU owners against current project/package evidence."""
from pathlib import Path
import hashlib
import json
import os
import subprocess
import sys

ROOT=Path(__file__).resolve().parents[2]
EVIDENCE=Path(__file__).resolve().parent
SOURCE=ROOT/'artifacts/locomotion-2026-10-26/upstream-robot/RobotExpressive.glb'
EXPECTED='047f5e5fb3bb6d378bd1df16ca6137f2a596c99b3a1b5690b4020c05aaf6f319'
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
history=EVIDENCE/'evidence-history';history.mkdir(exist_ok=True)
for name in ['morph-ordinary-owner-proof.json','morph-ordinary-weight-only-gpu.log','morph-ordinary-original-gpu.log']:
 source=EVIDENCE/name
 if source.exists():
  destination=history/(sha(source)+'-'+name)
  if destination.exists():assert destination.read_bytes()==source.read_bytes()
  else:
   with destination.open('xb') as output:output.write(source.read_bytes())
assert sha(SOURCE)==EXPECTED
runtime=EVIDENCE/'morph-runtime-proof.json'
if not runtime.exists() or not all(sha(ROOT/p)==digest for p,digest in json.loads(runtime.read_text())['code_sha256'].items()):
 subprocess.run([sys.executable,str(EVIDENCE/'morph-runtime-proof.py')],cwd=ROOT,check=True)
project=json.loads(runtime.read_text())
files=list(project['code_sha256'])+['crates/voxy_editor/src/animated_models/tests.rs','crates/voxy_editor/src/locomotion_tests.rs','crates/voxy_render/src/scene/morph.wgsl','crates/voxy_render/src/scene/skinning.wgsl','crates/voxy_render/src/world_exact_fused_affine.wgsl']
before={p:sha(ROOT/p) for p in files}
for name,test in [('morph-ordinary-weight-only-gpu.log','gpu_morph_weight_only_updates_and_failed_replacement_retains_last_good'),('morph-ordinary-original-gpu.log','gpu_original_morph_owner_updates_all_primitives')]:
 env=dict(os.environ,VOXY_MORPH_UPSTREAM_GLB=str(SOURCE))
 with (EVIDENCE/name).open('w') as log:
  subprocess.run(['cargo','+nightly-2026-05-27','test','-p','voxy_editor','--lib','animated_models::tests::'+test,'--target-dir','target/research-ray-precision','--','--ignored','--exact','--nocapture'],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
weights=(EVIDENCE/'morph-ordinary-weight-only-gpu.log').read_text()
assert 'weight_only_updates=true cpu_pixels_equal=true last_good_retained=true stop_owners=0' in weights
raw=(EVIDENCE/'morph-ordinary-original-gpu.log').read_text()
report=json.loads(next(line.removeprefix('VOXY_ORIGINAL_MORPH_OWNER_GPU ') for line in raw.splitlines() if line.startswith('VOXY_ORIGINAL_MORPH_OWNER_GPU ')))
assert report['primitives']==19 and report['morph_primitives']==3 and report['frames']==3
assert report['checked_vertices']==21642 and report['max_position_error']<=1e-5
assert report['ordinary_import_and_playback'] and report['all_primitives_gpu']
assert before=={p:sha(ROOT/p) for p in files},'source changed during owner proof'
assert sha(SOURCE)==EXPECTED
record={'status':'ordinary_morph_import_playback_gpu_owner_and_package_pass','source_sha256':EXPECTED,'source_bytes_unchanged':True,'code_sha256':before,'project_package_proof':'morph-runtime-proof.json','ordinary_original_gpu':report,'weight_only_gpu_cpu_pixels_equal':True,'failed_gpu_replacement_preserves_last_good':True,'default_editor_import_enabled':True,'legacy_pose_only_import_stays_strict':True,'native_editor_controls_verified':False,'full_scene_image_verified':False,'fps_qualified':False,'engine_ready':False}
(EVIDENCE/'morph-ordinary-owner-proof.json').write_text(json.dumps(record,indent=2)+'\n')
print('ORDINARY MORPH PASS: original full GPU owner, weight-only update/CPU parity/last-good retention, source-free package evidence current')
