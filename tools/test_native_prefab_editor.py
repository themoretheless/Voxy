#!/usr/bin/env python3
"""Temporary nested prefab project -> actual presented native editor frames."""
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    editor = ROOT / 'target/debug/voxy_editor'
    with tempfile.TemporaryDirectory(prefix='voxy-native-prefab-') as directory:
        root = Path(directory)
        (root / 'mesh.obj').write_text('v -0.5 -0.5 0\nv 0.5 -0.5 0\nv 0 0.5 0\nf 1 2 3\n')
        object_ = {'id': 'root', 'parent': None, 'name': 'Prefab model', 'active': True,
                   'translation': [0, 0, 0.5], 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1],
                   'components': {'editor.model.v1': 'mesh'}}
        leaf = {'version': 1, 'objects': [object_], 'instances': []}
        instance = lambda id_, asset: {'id': id_, 'asset': asset, 'parent': None, 'overrides': {}}
        (root / 'leaf.prefab').write_text(json.dumps(leaf))
        (root / 'nested.prefab').write_text(json.dumps({'version': 1, 'objects': [], 'instances': [instance('child', 'leaf')]}))
        manifest = {'version': 1, 'assets': [{'asset': 'mesh', 'source': 'mesh.obj'},
                    {'asset': 'leaf', 'source': 'leaf.prefab'}, {'asset': 'nested', 'source': 'nested.prefab'}]}
        (root / 'assets.json').write_text(json.dumps(manifest))
        scene = {'version': 1, 'objects': [], 'instances': [instance('first', 'nested'), instance('second', 'nested')]}
        (root / 'scene.json').write_text(json.dumps(scene))
        result = subprocess.run([str(editor), '--manifest', str(root / 'assets.json'), 'mesh', '--scene', str(root / 'scene.json'), '--prefab-smoke'], capture_output=True, text=True, timeout=90)
        log = result.stdout + result.stderr
        markers = ['PREFAB NATIVE PANEL CREATE PLACE SAVE UNDO PASS', 'PREFAB NATIVE PANEL PLACE SAVE UNDO REDO PASS', 'PREFAB NATIVE PANEL REVERT UNDO REDO PASS', 'PREFAB NATIVE STRUCTURAL SAVE UNDO PASS', 'PREFAB NATIVE SAVE UNDO AND LAST GOOD PASS', 'PREFAB NATIVE PRESENTED AFTER FAILURE PASS', 'PREFAB NATIVE PLAY STOP PASS']
        if result.returncode or any(marker not in log for marker in markers):
            raise RuntimeError(log)
        saved = json.loads((root / 'scene.json').read_text())
        if saved['objects'] or len(saved['instances']) != 2 or len(saved['instances'][0]['overrides']) != 1 or saved['instances'][1]['overrides']:
            raise RuntimeError('editor baked instances or lost override isolation')
        created = root / 'prefab-1.prefab'
        template = json.loads(created.read_text())
        if template['objects'] or len(template['instances']) != 1 or template['instances'][0]['asset'] != 'nested' or template['instances'][0]['parent'] is not None:
            raise RuntimeError('created template lost selected subtree')
        if not any(binding['asset'] == created.name for binding in json.loads((root / 'assets.json').read_text())['assets']):
            raise RuntimeError('created template missing manifest registration')
        if json.loads((root / 'leaf.prefab').read_text()) != leaf:
            raise RuntimeError('acceptance did not restore source dependency')
        print(json.dumps({'editor_sha256': hashlib.sha256(editor.read_bytes()).hexdigest(), 'saved_authoring': saved, 'native_log': log.strip(), 'scope': 'presented native editor frames, rendered panel hit-test, subtree template creation/manifest registration/reinstantiation, prefab placement/save/load/undo/redo and instance revert/save/load/undo/redo, nested prefab structural edits, last-good scene after damaged dependency, restored dependency and isolated Play/Stop; no image-equivalence claim'}, indent=2))


if __name__ == '__main__':
    main()
