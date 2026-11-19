#!/usr/bin/env python3
"""Qualify native project authoring and native source-free package startup."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/debug/voxy_editor')
    editor = parser.parse_args().binary.resolve(strict=True)
    digest = hashlib.sha256(editor.read_bytes()).hexdigest()
    with tempfile.TemporaryDirectory(prefix='voxy-native-prefab-') as directory:
        workspace = Path(directory)
        root = workspace / 'project'
        path = root / 'project.voxy'
        calls = {}

        def run(label, *arguments):
            result = subprocess.run(
                [str(editor), *map(str, arguments)], cwd=workspace,
                capture_output=True, text=True, timeout=90,
                env=dict(os.environ, VOXY_PREFAB_SMOKE_HIERARCHY_SEARCH='1', VOXY_PREFAB_SMOKE_JOINT='1'),
            )
            log = result.stdout + result.stderr
            if result.returncode:
                raise RuntimeError(f'{label} exited with {result.returncode}\n{log}')
            calls[label] = log.strip()
            return log

        run('create', '--create-project', root, 'Native Prefab Qualification')
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
        (root / 'main.scene.json').write_text(json.dumps(scene))
        previous_package = root / 'game.vpak'
        previous_package.write_bytes(b'previous package sentinel')
        log = run('authoring', '--project', path, '--prefab-smoke')
        markers = ['PREFAB NATIVE JOINT PICK KIND REMOVE PASS', 'PREFAB NATIVE HIERARCHY SEARCH PASS', 'PREFAB NATIVE PANEL CREATE PLACE SAVE UNDO PASS', 'PREFAB NATIVE PANEL PLACE SAVE UNDO REDO PASS', 'PREFAB NATIVE PANEL REVERT UNDO REDO PASS', 'PREFAB NATIVE STRUCTURAL SAVE UNDO PASS', 'PREFAB NATIVE SAVE UNDO AND LAST GOOD PASS', 'PREFAB NATIVE PRESENTED AFTER FAILURE PASS', 'PREFAB NATIVE PLAY STOP PASS']
        if any(marker not in log for marker in markers) or not re.search(
            r'PREFAB NATIVE PLAY STOP PASS frames=\d+ ticks=[1-9]\d*\b', log
        ):
            raise RuntimeError(log)
        saved = json.loads((root / 'main.scene.json').read_text())
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
        def game_state(log):
            marker = 'GAME STATE '
            if 'GAME CHECK PASS ticks=120 ' not in log:
                raise RuntimeError(log)
            return json.loads(next(line[len(marker):] for line in log.splitlines()
                                   if line.startswith(marker)))

        project_state = game_state(run('reopened', '--project', path, '--game-check'))
        package = workspace / 'game.vpak'
        built = root / 'game-2.vpak'
        if 'PREFAB NATIVE RUN BUILT GAME PASS' not in log or 'GAME NATIVE PASS' not in log:
            raise RuntimeError('the native run action did not present the built game')
        if 'PREFAB NATIVE BUILD PASS' not in log or not built.is_file():
            raise RuntimeError('the native build action did not publish a package')
        if previous_package.read_bytes() != b'previous package sentinel':
            raise RuntimeError('the native build action overwrote an existing package')
        built.rename(package)
        shutil.rmtree(root)
        package_state = game_state(run('package_state', '--package', package, '--game-check'))
        if project_state != package_state:
            raise RuntimeError('source-free package changed the reopened project state')
        native = run('package_native', '--package', package, '--game-native-smoke')
        if 'GAME NATIVE PASS' not in native:
            raise RuntimeError(native)
        if hashlib.sha256(editor.read_bytes()).hexdigest() != digest:
            raise RuntimeError('editor binary changed during qualification')
        print(json.dumps({
            'editor_sha256': digest,
            'saved_authoring': saved,
            'reopened_and_packaged_state': package_state,
            'calls': calls,
        }, indent=2))



if __name__ == '__main__':
    main()
