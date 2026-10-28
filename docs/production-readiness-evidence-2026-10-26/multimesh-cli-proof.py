"""Qualify controller plumbing using a clearly synthetic regression rig, not a visual demo.
The original upstream RobotExpressive asset remains an unmodified import acceptance case.
"""
import hashlib
import json
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tempfile
binary=Path(sys.argv[1]).resolve(strict=True)
evidence=Path(__file__).resolve().parent
repo=evidence.parent.parent
runs=[]
def run(*args,success=True):
    result=subprocess.run([str(binary),*map(str,args)],cwd='/',text=True,
                          stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=180)
    runs.append(dict(args=list(map(str,args)),exit_code=result.returncode,stdout=result.stdout,stderr=result.stderr))
    assert (result.returncode==0)==success,runs[-1]
    return result.stdout
def outcome(output):
    return json.loads(next(line[len('LOCOMOTION RESULT '):] for line in output.splitlines() if line.startswith('LOCOMOTION RESULT ')))
def object(id,parent,translation,components):
    return dict(id=id,name=id,parent=parent,active=True,translation=translation,rotation=[0,0,0,1],scale=[1,1,1],components=components)
with tempfile.TemporaryDirectory(prefix='voxy-locomotion-cli-') as temporary:
    root=Path(temporary)/'source';root.mkdir()
    source=(repo/'crates/voxy_render/examples/assets/animated-triangle.glb').read_bytes()
    old_json_length=struct.unpack_from('<I',source,12)[0]
    document=json.loads(source[20:20+old_json_length])
    document['nodes'][0]['children']=[1,2]
    document['nodes'].append(dict(mesh=0,skin=1))
    document['skins'].append(dict(document['skins'][0]))
    original=document['animations'][0]
    document['animations']=[dict(original,name=name) for name in ['Idle','Walking','Jump']]
    encoded=json.dumps(document,separators=(',',':')).encode()
    encoded+=b' '*((-len(encoded))%4)
    remaining=source[20+old_json_length:]
    fixture=b'glTF'+struct.pack('<II',2,20+len(encoded)+len(remaining))+struct.pack('<I4s',len(encoded),b'JSON')+encoded+remaining
    (root/'rig.glb').write_bytes(fixture)
    (root/'project.voxy').write_text(json.dumps(dict(version=1,name='Synthetic locomotion qualification',asset_manifest='assets.json',startup_scene='main.scene.json')))
    (root/'assets.json').write_text(json.dumps(dict(version=1,assets=[dict(asset='rig',source='rig.glb')])))
    scene=dict(version=1,objects=[
        object('body',None,[0,.101,0],{'game.character.v1':dict(half_extents=[.1,.1,.1],speed=1.,gravity=-9.8,jump_speed=3.)}),
        object('visual','body',[0,0,0],{'editor.model.v1':'rig','editor.model-animation.v1':dict(clip=0,clip_name='Idle',speed=1.,transition_seconds=.05),'editor.model-locomotion.v1':dict(idle='Idle',moving='Walking',rising='Jump',falling='Jump',speed_threshold=.01,transition_seconds=.05)}),
        object('ground',None,[0,-.1,0],{'game.box.v1':dict(half_extents=[10,.1,10])})])
    (root/'main.scene.json').write_text(json.dumps(scene))
    before=outcome(run('--check',root/'project.voxy'))
    package=Path(temporary)/'locomotion.vpak'
    run('--build',root/'project.voxy',package)
    payload=package.read_bytes()
    shutil.rmtree(root)
    after=outcome(run('--package-check',package))
    assert before==after and after['ticks']==240
    assert len(after['animation']['skin_matrices'])==5
    assert {name for _,name in after['transitions']} >= {'Idle','Walking','Jump'}
    assert len(after['pose_hashes'])>=30
    artifact=repo/'artifacts'/'multimesh-2026-10-26'
    artifact.mkdir(parents=True,exist_ok=True)
    saved=artifact/'regression-only.vpak'
    if saved.exists():assert saved.read_bytes()==payload,'preserve differing artifact'
    else:
        with saved.open('xb') as output:output.write(payload)
robot=repo/'artifacts'/'locomotion-2026-10-26'/'upstream-robot'/'RobotExpressive.glb'
run('--asset-info',robot,success=False)
assert 'without morph targets' in runs[-1]['stderr']
proof=dict(status='multiple_mesh_skin_controller_cli_pass_morph_pending',runs=runs,
           binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),package_sha256=hashlib.sha256(payload).hexdigest(),
           outcome=after,synthetic_fixture=True,original_source_removed_before_package=True,
           accepted_animation_and_scene_equal=True,robot_import_passed=False,
           native_window_verified=False,fps_qualified=False)
(evidence/'multimesh-cli-proof.json').write_text(json.dumps(proof,indent=2)+'\n')
print('MULTIMESH CLI PASS: two meshes/skins, one clock, matching source-free package; original RobotExpressive now reaches morph gate')
