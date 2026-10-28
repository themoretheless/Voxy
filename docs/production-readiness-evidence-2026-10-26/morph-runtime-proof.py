#!/usr/bin/env python3
"""Ordinary project import/player/build/package on the unchanged upstream GLB."""
from pathlib import Path
import hashlib
import json
import shutil
import subprocess
import tempfile

ROOT=Path(__file__).resolve().parents[2]
EVIDENCE=Path(__file__).resolve().parent
BINARY=ROOT/'target/research-ray-precision/debug/examples/locomotion_project'
MODEL=ROOT/'artifacts/locomotion-2026-10-26/upstream-robot/RobotExpressive.glb'
EXPECTED='047f5e5fb3bb6d378bd1df16ca6137f2a596c99b3a1b5690b4020c05aaf6f319'
FILES=['crates/voxy_model/src/lib.rs','crates/voxy_model/src/morph.rs',
       'crates/voxy_animation/src/property_playback.rs','crates/voxy_animation/src/property_track.rs',
       'crates/voxy_editor/src/import.rs','crates/voxy_editor/src/model_playback.rs',
       'crates/voxy_editor/src/animation_runtime.rs','crates/voxy_editor/src/animated_models.rs',
       'crates/voxy_editor/src/animation_preview.rs','crates/voxy_editor/src/headless_game.rs',
       'crates/voxy_editor/src/gpu_model.rs','crates/voxy_editor/src/retarget_profile.rs',
       'crates/voxy_editor/examples/locomotion_project.rs',
       'crates/voxy_render/src/skinned.rs','crates/voxy_render/src/skinned_morph.rs',
       'crates/voxy_render/src/scene/skinning.rs']
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def preserve_evidence(names):
 history=EVIDENCE/'evidence-history';history.mkdir(exist_ok=True)
 for name in names:
  source=EVIDENCE/name
  if source.exists():
   destination=history/(digest(source)+'-'+name)
   if destination.exists():assert destination.read_bytes()==source.read_bytes()
   else:
    with destination.open('xb') as output:output.write(source.read_bytes())
preserve_evidence(['morph-runtime-proof.json','morph-runtime-build.log'])
assert digest(MODEL)==EXPECTED
before={p:digest(ROOT/p) for p in FILES}
with (EVIDENCE/'morph-runtime-build.log').open('w') as log:
 subprocess.run(['cargo','+nightly-2026-05-27','build','-p','voxy_editor','--example','locomotion_project','--target-dir','target/research-ray-precision'],cwd=ROOT,stdout=log,stderr=subprocess.STDOUT,check=True)
runs=[]
def run(*args):
 result=subprocess.run([str(BINARY),*map(str,args)],cwd='/',capture_output=True,text=True,timeout=180)
 runs.append({'args':list(map(str,args)),'exit_code':result.returncode,'stdout':result.stdout,'stderr':result.stderr})
 assert result.returncode==0,runs[-1]
 return result.stdout
def result(output):return json.loads(next(line.removeprefix('LOCOMOTION RESULT ') for line in output.splitlines() if line.startswith('LOCOMOTION RESULT ')))
def obj(id,parent,position,components):return dict(id=id,name=id,parent=parent,active=True,translation=position,rotation=[0,0,0,1],scale=[1,1,1],components=components)
info=json.loads(next(line.removeprefix('ASSET INFO ') for line in run('--asset-info',MODEL).splitlines() if line.startswith('ASSET INFO ')))
assert info['joints']==160 and info['primitives']==19 and info['morph_primitives']==3
with tempfile.TemporaryDirectory(prefix='voxy-original-morph-project-') as temp:
 root=Path(temp)/'source';root.mkdir();shutil.copyfile(MODEL,root/'rig.glb')
 assert digest(root/'rig.glb')==EXPECTED
 (root/'project.voxy').write_text(json.dumps(dict(version=1,name='Original RobotExpressive import qualification',asset_manifest='assets.json',startup_scene='main.scene.json')))
 (root/'assets.json').write_text(json.dumps(dict(version=1,assets=[dict(asset='rig',source='rig.glb')])))
 scene=dict(version=1,objects=[
  obj('body',None,[0,.101,0],{'game.character.v1':dict(half_extents=[.1,.1,.1],speed=1.,gravity=-9.8,jump_speed=3.)}),
  obj('visual','body',[0,0,0],{'editor.model.v1':'rig','editor.model-animation.v1':dict(clip=0,clip_name='Idle',speed=1.,transition_seconds=.05),'editor.model-locomotion.v1':dict(idle='Idle',moving='Walking',rising='Jump',falling='Jump',speed_threshold=.01,transition_seconds=.05)}),
  obj('ground',None,[0,-.1,0],{'game.box.v1':dict(half_extents=[10,.1,10])})])
 (root/'main.scene.json').write_text(json.dumps(scene))
 expected=result(run('--check',root/'project.voxy'))
 package=Path(temp)/'game.vpak';run('--build',root/'project.voxy',package)
 payload=package.read_bytes();shutil.rmtree(root)
 actual=result(run('--package-check',package))
 assert expected==actual and actual['ticks']==240
 assert len(actual['animation']['skin_matrices'])==160
 weights=actual['animation']['morph_weights']
 assert len(weights)==19 and sum(bool(w) for w in weights)==3
 assert all(not w or len(w)==3 for w in weights)
 assert {name for _,name in actual['transitions']} >= {'Idle','Walking','Jump'}
 assert len(actual['pose_hashes'])>=30
 artifact=ROOT/'artifacts/morph-runtime-2026-10-26';artifact.mkdir(parents=True,exist_ok=True)
 legacy=artifact/'original-robot-regression.vpak'
 legacy_digest=digest(legacy) if legacy.exists() else None
 package_digest=hashlib.sha256(payload).hexdigest()
 saved=artifact/('original-robot-'+package_digest+'.vpak')
 if saved.exists():assert saved.read_bytes()==payload,'content-addressed package mismatch'
 else:
  with saved.open('xb') as output:output.write(payload)
 assert (digest(legacy) if legacy.exists() else None)==legacy_digest,'legacy artifact changed'
assert digest(MODEL)==EXPECTED
assert before=={p:digest(ROOT/p) for p in FILES},'source changed during runtime proof'
record={'status':'ordinary_original_morph_project_and_source_free_package_pass','source_sha256':EXPECTED,'source_bytes_unchanged':True,'code_sha256':before,'binary_sha256':digest(BINARY),'package_sha256':hashlib.sha256(payload).hexdigest(),'artifact':str(saved.relative_to(ROOT)),'legacy_artifact_sha256':legacy_digest,'legacy_artifact_preserved':True,'asset_info':info,'runs':runs,'outcome':actual,'original_source_removed_before_package_run':True,'accepted_scene_and_animation_equal':True,'source_weight_channels_constant_zero':True,'synthetic_character_physics_setup':True,'visual_locomotion_or_foot_contact_verified':False,'native_window_verified':False,'fps_qualified':False,'engine_ready':False}
(EVIDENCE/'morph-runtime-proof.json').write_text(json.dumps(record,indent=2)+'\n')
print('MORPH RUNTIME PASS: unchanged upstream asset, ordinary controller/project build and matching source-free package')
