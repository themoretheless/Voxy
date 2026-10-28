"""Validate and replay a nested prefab project/package using actual CLI binaries."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
editor=Path(sys.argv[1]).resolve(strict=True)
game=Path(sys.argv[2]).resolve(strict=True)
evidence=Path(__file__).resolve().parent
repo=evidence.parent.parent
runs=[]
def run(binary,*args):
    result=subprocess.run([str(binary),*map(str,args)],cwd='/',text=True,
                          stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=180)
    runs.append(dict(binary=str(binary),args=list(map(str,args)),exit_code=result.returncode,
                     stdout=result.stdout,stderr=result.stderr))
    assert result.returncode==0,runs[-1]
    return result.stdout
def outcome(output):
    return json.loads(next(line[len('INTERACTIVE RESULT '):] for line in output.splitlines()
                           if line.startswith('INTERACTIVE RESULT ')))
with tempfile.TemporaryDirectory(prefix='voxy-prefab-cli-') as temporary:
    root=Path(temporary)/'source'
    shutil.copytree(repo/'examples'/'prefab-interactive-project',root,ignore=shutil.ignore_patterns('.voxy-cache'))
    source=json.loads((root/'main.scene.json').read_text())
    nested=json.loads((root/'level.prefab').read_text())
    assert not source['objects'] and source['instances'][0]['overrides']
    assert not nested['objects'] and nested['instances']
    moved=Path(temporary)/'moved'
    root.rename(moved)
    descriptor=moved/'project.voxy'
    assert '4 scene objects' in run(editor,'--validate-project',descriptor)
    before=outcome(run(game,'--check',descriptor))
    player=next(o for o in before['scene']['objects'] if o['name']=='player')
    assert player['id']!='player' and player['components']['game.character.v1']['speed']>1.19
    package=Path(temporary)/'prefab-interactive.vpak'
    run(game,'--build',descriptor,package)
    payload=package.read_bytes()
    shutil.rmtree(moved)
    assert not descriptor.exists()
    after=outcome(run(game,'--package-check',package))
    assert before==after and after['ticks']==180 and after['audio_frames']==144000
    artifact=repo/'artifacts'/'prefab-project-2026-10-26'
    artifact.mkdir(parents=True,exist_ok=True)
    saved=artifact/'prefab-interactive.vpak'
    if saved.exists(): assert saved.read_bytes()==payload,'preserve differing artifact'
    else:
        with saved.open('xb') as output: output.write(payload)
proof=dict(status='nested_prefab_project_and_package_cli_pass',runs=runs,outcome=after,
           editor_sha256=hashlib.sha256(editor.read_bytes()).hexdigest(),
           game_sha256=hashlib.sha256(game.read_bytes()).hexdigest(),
           package_sha256=hashlib.sha256(payload).hexdigest(),
           project_relocated_before_validation=True,source_deleted_before_package_launch=True,
           nested_override_applied=True,project_and_package_identical=True,
           native_window_verified=False,native_audio_device_verified=False,fps_qualified=False)
(evidence/'prefab-cli-proof.json').write_text(json.dumps(proof,indent=2)+'\n')
print('PREFAB CLI PASS: validate relocated nested project, apply override, source-free package, identical interactive replay')
