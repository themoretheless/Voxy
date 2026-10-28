"""Public input/physics/UI/audio replay against a project and its isolated package."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
binary = Path(sys.argv[1]).resolve(strict=True)
evidence = Path(__file__).resolve().parent
repo = evidence.parent.parent
runs = []
def run(*args):
    result = subprocess.run([str(binary), *map(str,args)], cwd="/", text=True,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=180)
    runs.append({"args":list(map(str,args)),"exit_code":result.returncode,"stdout":result.stdout,"stderr":result.stderr})
    assert result.returncode == 0, runs[-1]
    return result.stdout
def outcome(output):
    return json.loads(next(line[len("INTERACTIVE RESULT "):] for line in output.splitlines()
                           if line.startswith("INTERACTIVE RESULT ")))
with tempfile.TemporaryDirectory(prefix="voxy-interactive-cli-") as temporary:
    root = Path(temporary)/"source"
    shutil.copytree(repo/"examples"/"interactive-project",root,ignore=shutil.ignore_patterns('.voxy-cache'))
    project = root/"project.voxy"
    before = outcome(run("--check",project))
    package = Path(temporary)/"interactive.vpak"
    run("--build",project,package)
    payload = package.read_bytes()
    shutil.rmtree(root)
    assert not project.exists()
    after = outcome(run("--package-check",package))
    assert before == after
    assert after["ticks"] == 180 and after["audio_frames"] == 144000 and after["audio_peak"] > 0
    player = next(o for o in after['scene']['objects'] if o['id']=='player')
    button = next(o for o in after['scene']['objects'] if o['id']=='dismiss')
    assert 0.7 < player['translation'][0] < 0.801
    assert 0.099 <= player['translation'][1] < 0.11 and after['max_height'] > 0.3
    assert not button['active']
    artifact = repo/'artifacts'/'interactive-project-2026-10-26'
    artifact.mkdir(parents=True,exist_ok=True)
    saved = artifact/'interactive.vpak'
    if saved.exists():
        assert saved.read_bytes() == payload, 'preserve differing artifact'
    else:
        with saved.open('xb') as output: output.write(payload)
proof = dict(status='public_interactive_project_package_headless_pass',runs=runs,
    binary=str(binary),binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
    package_sha256=hashlib.sha256(payload).hexdigest(),outcome=after,
    source_removed_before_package_launch=True,project_and_package_identical=True,
    native_window_verified=False,native_audio_device_verified=False,fps_qualified=False)
(evidence/'interactive-cli-proof.json').write_text(json.dumps(proof,indent=2)+'\n')
print('INTERACTIVE CLI PASS: movement, wall, jump/landing, UI callback and offline spatial audio; source-free package matches')
