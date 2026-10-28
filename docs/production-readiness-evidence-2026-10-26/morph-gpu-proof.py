#!/usr/bin/env python3
"""Headless physical-GPU morph correctness, including an independent rational oracle."""
from pathlib import Path
import hashlib
import json
import subprocess
import sys

ROOT=Path(__file__).resolve().parents[2]
EVIDENCE=Path(__file__).resolve().parent
SOURCE=ROOT/'artifacts/locomotion-2026-10-26/upstream-robot/RobotExpressive.glb'
SOURCE_HASH='047f5e5fb3bb6d378bd1df16ca6137f2a596c99b3a1b5690b4020c05aaf6f319'
FILES=['crates/voxy_render/src/skinned_morph.rs','crates/voxy_render/src/scene/skinning.rs',
       'crates/voxy_render/src/scene/skinning.wgsl','crates/voxy_render/src/scene/morph.wgsl',
       'crates/voxy_render/src/scene/skinning_morph_tests.rs','crates/voxy_render/src/world_exact_fused_affine.wgsl',
       'crates/voxy_render/src/lib.rs','crates/voxy_model/src/lib.rs','crates/voxy_model/src/morph.rs',
       'crates/voxy_model/examples/morph_gpu.rs']
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
assert sha(SOURCE)==SOURCE_HASH
before={p:sha(ROOT/p) for p in FILES}
base=['cargo','+nightly-2026-05-27']
target=['--target-dir','target/research-ray-precision']
for name,test in [('morph-gpu-exact-cases.log','gpu_exact_morph_component_edges_and_full_target_budget'),
                  ('morph-gpu-instance-readback.log','gpu_exact_morph_precedes_skinning_and_preserves_independent_instance_weights')]:
 with (EVIDENCE/name).open('w') as log:
  subprocess.run(base+['test','-p','voxy_render','--lib','scene::skinning::morph_tests::'+test]+target+['--','--ignored','--exact','--nocapture'],cwd=ROOT,stdout=log,stderr=subprocess.STDOUT,check=True)
subprocess.run([sys.executable,str(EVIDENCE/'morph-gpu-rational-oracle.py')],cwd='/',check=True)
with (EVIDENCE/'morph-gpu-build.log').open('w') as log:
 subprocess.run(base+['build','-p','voxy_model','--example','morph_gpu']+target,cwd=ROOT,stdout=log,stderr=subprocess.STDOUT,check=True)
binary=ROOT/'target/research-ray-precision/debug/examples/morph_gpu'
result=subprocess.run([str(binary),str(SOURCE)],cwd='/',capture_output=True,text=True)
(EVIDENCE/'morph-gpu-original.log').write_text(result.stderr)
assert result.returncode==0,result.stderr
report=json.loads(result.stdout)
assert report['source_primitives']==19 and report['gpu_morph_primitives']==3
assert report['frames']==12 and report['checked_vertices']==20904
assert report['max_world_position_error']<=1e-5
assert not report['full_model_image_verified'] and not report['editor_integrated'] and not report['fps_qualified']
assert sha(SOURCE)==SOURCE_HASH
assert before=={p:sha(ROOT/p) for p in FILES},'source changed during GPU proof'
record={'status':'exact_gpu_morph_correctness_pass_default_editor_pending','source_sha256':SOURCE_HASH,
        'source_bytes_unchanged':True,'code_sha256':before,'binary_sha256':sha(binary),'cwd':'/',
        'oracle':json.loads((EVIDENCE/'morph-gpu-rational-oracle.json').read_text()),'original_report':report,
        'default_editor_integrated':False,'native_image_verified':False,'fps_qualified':False,'engine_ready':False}
(EVIDENCE/'morph-gpu-proof.json').write_text(json.dumps(record,indent=2)+'\n')
print('GPU MORPH PASS: exact arithmetic oracle, independent instances, original imported morph primitives; editor/FPS pending')
