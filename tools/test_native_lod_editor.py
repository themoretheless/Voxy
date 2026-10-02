#!/usr/bin/env python3
"""Pinned meshoptimizer fixture -> certificate -> actual native editor frames."""
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    mechanism = ROOT / 'docs/engine-research/mechanisms/godot-lod'
    manifest = json.loads((mechanism / 'sources.json').read_text())
    for source in manifest['sources']:
        if hashlib.sha256((mechanism / source['file']).read_bytes()).hexdigest() != source['sha256']:
            raise RuntimeError('pinned source integrity failed: ' + source['file'])
    editor = ROOT / 'target/debug/examples/lod_viewport'
    verifier = ROOT / 'target/debug/examples/certify_obj_lod'
    with tempfile.TemporaryDirectory(prefix='voxy-native-lod-') as directory:
        root = Path(directory)
        subprocess.run(['python3', str(ROOT / 'docs/engine-research/mechanisms/godot-lod/run_importer_probe.py'), '--export-dir', str(root)], check=True, capture_output=True, text=True, timeout=90)
        certification = subprocess.run([str(verifier), str(root / 'base.obj'), str(root / 'lod-1.obj'), '1000000', '1', 'indexed', str(root / 'cache')], check=True, capture_output=True, text=True, timeout=60)
        entries = list((root / 'cache').glob('*.artifact'))
        if len(entries) != 1:
            raise RuntimeError('expected exactly one cache envelope')
        envelope = entries[0].read_bytes()
        if envelope[:8] != b'VOXYCA01' or envelope[72:80] != b'VOXYLCD1':
            raise RuntimeError('unsupported fixture envelope/archive version')
        archive = envelope[72:]
        (root / 'proof.lod').write_bytes(archive)
        (root / 'model.vmodel').write_text(json.dumps({'version': 1, 'base': 'base.obj', 'certificate': 'proof.lod'}))
        result = subprocess.run([str(editor), str(root / 'model.vmodel'), '--smoke'], capture_output=True, text=True, timeout=60)
        log = result.stdout + result.stderr
        required = ['LOD NATIVE SNAPSHOT PASS', 'LOD NATIVE FAR PASS', 'LOD NATIVE NEAR AND LAST GOOD PASS', 'LOD NATIVE PRESENTED AFTER FAILURE PASS', 'LOD NATIVE BUDGET RECOVERY PASS']
        if result.returncode != 0 or any(marker not in log for marker in required):
            raise RuntimeError(log)
        print(json.dumps({'upstream_commit': manifest['commit'], 'editor_sha256': hashlib.sha256(editor.read_bytes()).hexdigest(), 'verifier_sha256': hashlib.sha256(verifier.read_bytes()).hexdigest(), 'archive_sha256': hashlib.sha256(archive).hexdigest(), 'certification': certification.stdout.strip(), 'native_log': log.strip(), 'scope': 'presented native editor frames, near/far selection, resident-byte agreement and ordinary residency retry rejection preserving previous geometry and recovery after restoring budget; no image-equivalence claim'}, indent=2))


if __name__ == '__main__':
    try:
        main()
    except subprocess.CalledProcessError as error:
        raise RuntimeError((error.stdout or '') + (error.stderr or '')) from error
