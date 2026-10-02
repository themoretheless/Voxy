#!/usr/bin/env python3
"""Check the standalone executable's real loader and fixed simulation path."""
import hashlib
import math
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import struct
import zlib

ROOT = Path(__file__).resolve().parents[1]


def main():
    binary = ROOT / 'target/debug/voxy_app'
    with tempfile.TemporaryDirectory(prefix='voxy-game-prefabs-') as directory:
        root = Path(directory)
        model = 'v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n'
        (root / 'mesh.obj').write_text(model)
        (root / 'buffers').mkdir()
        (root / 'textures').mkdir()
        (root / 'buffers/mesh.bin').write_bytes(struct.pack('<15f', 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 1))
        chunk = lambda type_, data: struct.pack('>I', len(data)) + type_ + data + struct.pack('>I', zlib.crc32(type_ + data))
        png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', 1, 1, 8, 6, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(b'\x00\xff\xff\xff\xff')) + chunk(b'IEND', b'')
        (root / 'textures/albedo.png').write_bytes(png)
        gltf = {'asset': {'version': '2.0'}, 'scene': 0, 'scenes': [{'nodes': [0]}], 'nodes': [{'mesh': 0}],
                'buffers': [{'uri': 'buffers/mesh.bin', 'byteLength': 60}],
                'bufferViews': [{'buffer': 0, 'byteOffset': 0, 'byteLength': 36}, {'buffer': 0, 'byteOffset': 36, 'byteLength': 24}],
                'accessors': [{'bufferView': 0, 'componentType': 5126, 'count': 3, 'type': 'VEC3', 'min': [0, 0, 0], 'max': [1, 1, 0]},
                              {'bufferView': 1, 'componentType': 5126, 'count': 3, 'type': 'VEC2'}],
                'images': [{'uri': 'textures/albedo.png'}], 'textures': [{'source': 0}],
                'materials': [{'pbrMetallicRoughness': {'baseColorTexture': {'index': 0}}}],
                'meshes': [{'primitives': [{'attributes': {'POSITION': 0, 'TEXCOORD_0': 1}, 'material': 0}]}]}
        (root / 'mesh.gltf').write_text(json.dumps(gltf))

        object_ = {'id': 'root', 'parent': None, 'name': 'Game model', 'active': True,
                   'translation': [0, 0, 0], 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1],
                   'components': {'editor.model.v1': 'mesh', 'game.angular-motion.v1': {'axis': [0, 0, 1], 'radians_per_second': 0.5}}}
        pcm = struct.pack('<h', 8192)
        wav = b'RIFF' + struct.pack('<I', 36 + len(pcm)) + b'WAVEfmt ' + struct.pack('<IHHIIHH', 16, 1, 1, 48000, 96000, 2, 16) + b'data' + struct.pack('<I', len(pcm)) + pcm
        (root / 'tone.wav').write_bytes(wav)
        object_['components']['game.ui-element.v1'] = {'origin': [0.1, 0.1], 'size': [0.25, 0.1], 'color': [0.2, 0.4, 0.8, 1], 'layer': 0, 'enabled': True, 'action': 'jump'}
        object_['components']['game.audio-source.v1'] = {'asset': 'tone', 'import_settings': 'tone-settings', 'bus': 0, 'gain': 0.25, 'looping': True, 'spatial': False, 'near': 1, 'far': 5}
        (root / 'tone.import.json').write_text(json.dumps({'version': 1, 'max_input_bytes': 1024, 'max_frames': 10, 'max_filter_evaluations': 650}))
        font_candidates = [Path('/System/Library/Fonts/Supplemental/Arial.ttf'), Path('/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'), Path('C:/Windows/Fonts/arial.ttf')]
        font_path = next((path for path in font_candidates if path.is_file()), None)
        if font_path is None:
            raise RuntimeError('standalone Unicode integration requires a local TrueType font')
        (root / 'fonts').mkdir()
        (root / 'fonts/ui.ttf').write_bytes(font_path.read_bytes())
        object_['components']['game.ui-element.v1']['text'] = {'font': 'interface', 'content': 'Привет', 'size': 20, 'color': [1, 1, 1, 1]}
        leaf = json.dumps({'version': 1, 'objects': [object_], 'instances': []})
        (root / 'leaf.prefab').write_text(leaf)
        instance = lambda id_, asset: {'id': id_, 'asset': asset, 'parent': None, 'overrides': {}}
        (root / 'nested.prefab').write_text(json.dumps({'version': 1, 'objects': [], 'instances': [instance('child', 'leaf')]}))
        (root / 'assets.json').write_text(json.dumps({'version': 1, 'assets': [
            {'asset': 'mesh', 'source': 'mesh.obj'}, {'asset': 'leaf', 'source': 'leaf.prefab'},
            {'asset': 'nested', 'source': 'nested.prefab'}, {'asset': 'textured', 'source': 'mesh.gltf'}, {'asset': 'tone', 'source': 'tone.wav'}, {'asset': 'tone-settings', 'source': 'tone.import.json'}, {'asset': 'interface', 'source': 'fonts/ui.ttf'}]}))
        scene = root / 'scene.json'
        textured = dict(object_, id='textured-root', components={'editor.model.v1': 'textured', 'game.audio-bus.v1': {'bus': 0, 'gain': 0.5}})
        scene.write_text(json.dumps({'version': 1, 'objects': [textured], 'instances': [instance('first', 'nested'), instance('second', 'nested')]}))
        before = scene.read_bytes()
        command = [str(binary), '--model-manifest', str(root / 'assets.json'), 'mesh', '--scene', str(scene), '--game-check']
        result = subprocess.run(command, capture_output=True, text=True, timeout=30)
        log = result.stdout + result.stderr
        if result.returncode or 'GAME CHECK PASS ticks=120 objects=3 prefab_instances=2' not in log:
            raise RuntimeError(log)
        if scene.read_bytes() != before:
            raise RuntimeError('standalone check modified authoring source')
        def verify_motion(output):
            state_line = next((line for line in output.splitlines() if line.startswith('GAME STATE ')), None)
            if state_line is None:
                raise RuntimeError('game check did not expose runtime state')
            state = json.loads(state_line[len('GAME STATE '):])
            moving = [obj for obj in state['objects'] if 'game.angular-motion.v1' in obj['components']]
            if len(moving) != 2:
                raise RuntimeError('authored prefab behavior was lost')
            expected_rotation = [0, 0, math.sin(0.5), math.cos(0.5)]
            for obj in moving:
                if abs(sum(a*b for a, b in zip(obj['rotation'], expected_rotation))) < 0.9999:
                    raise RuntimeError('authored behavior did not run in fixed ticks')
        def verify_ui(output):
            if 'GAME UI LAYOUT elements=2 buttons=2' not in output:
                raise RuntimeError('saved nested UI layout/input was not checked: ' + output)
            if 'GAME UI TEXT fonts=1 runs=2 glyphs=12' not in output:
                raise RuntimeError('saved nested Unicode font/runs were not prepared: ' + output)
            if 'GAME UI MESH draws=4 glyph_quads=12' not in output:
                raise RuntimeError('saved UI draw geometry was not prepared: ' + output)

        def verify_audio(output):
            if 'GAME AUDIO frames=96000 peak=0.0625' not in output:
                raise RuntimeError('packaged/source audio PCM mismatch: ' + output)
        verify_ui(log)
        verify_audio(log)
        verify_motion(log)

        package_path = root / 'game.vpak'
        exported = subprocess.run(command[:-1] + ['--export-game', str(package_path)], capture_output=True, text=True, timeout=30)
        if exported.returncode or 'GAME PACKAGE PASS' not in exported.stdout:
            raise RuntimeError('export failed: ' + exported.stdout + exported.stderr)
        package_bytes = package_path.read_bytes()
        package = json.loads(package_bytes)
        expected_paths = {'__voxy_game.json', 'scene.json', 'assets.json', 'leaf.prefab', 'nested.prefab', 'mesh.obj', 'mesh.gltf', 'buffers/mesh.bin', 'textures/albedo.png', 'tone.wav', 'tone.import.json', 'fonts/ui.ttf'}
        if set(package['entries']) != expected_paths:
            raise RuntimeError('package omitted dependency or included unrelated files')
        for path in expected_paths - {'__voxy_game.json'}:
            if bytes(package['entries'][path]['bytes']) != (root / path).read_bytes():
                raise RuntimeError('package changed source bytes: ' + path)
        launch = json.loads(bytes(package['entries']['__voxy_game.json']['bytes']))
        if launch != {'version': 1, 'scene': 'scene.json', 'model': 'mesh', 'manifest': 'assets.json'}:
            raise RuntimeError('package launch descriptor changed identities')
        repeated = subprocess.run(command[:-1] + ['--export-game', str(package_path)], capture_output=True, text=True, timeout=30)
        if not repeated.returncode or package_path.read_bytes() != package_bytes:
            raise RuntimeError('export overwrote existing package')
        font_bytes = (root / 'fonts/ui.ttf').read_bytes()
        (root / 'fonts/ui.ttf').write_bytes(b'broken font')
        bad_font = subprocess.run(command, capture_output=True, text=True, timeout=30)
        if not bad_font.returncode or 'UI font:' not in bad_font.stderr or 'GAME CHECK PASS' in bad_font.stdout:
            raise RuntimeError('standalone accepted invalid UI font: ' + bad_font.stdout + bad_font.stderr)
        (root / 'fonts/ui.ttf').write_bytes(font_bytes)
        invalid_ui = json.loads(leaf)
        invalid_ui['objects'][0]['components']['game.ui-element.v1']['size'][0] = 0
        (root / 'leaf.prefab').write_text(json.dumps(invalid_ui))
        bad_ui = subprocess.run(command, capture_output=True, text=True, timeout=30)
        if not bad_ui.returncode or 'invalid scene UI descriptor' not in bad_ui.stderr or 'GAME CHECK PASS' in bad_ui.stdout:
            raise RuntimeError('standalone accepted invalid saved UI: ' + bad_ui.stdout + bad_ui.stderr)
        invalid_ui_error = bad_ui.stderr.strip()
        (root / 'leaf.prefab').write_text('broken dependency')
        bad_prefab = subprocess.run(command, capture_output=True, text=True, timeout=30)
        if not bad_prefab.returncode or 'GAME CHECK PASS' in bad_prefab.stdout:
            raise RuntimeError('standalone accepted corrupt prefab')
        (root / 'leaf.prefab').write_text(leaf)
        (root / 'mesh.obj').write_text('broken model')
        bad_model = subprocess.run(command, capture_output=True, text=True, timeout=30)
        if not bad_model.returncode or 'GAME CHECK PASS' in bad_model.stdout or 'game asset mesh failed:' not in bad_model.stderr:
            raise RuntimeError('standalone accepted corrupt model: ' + bad_model.stdout + bad_model.stderr)
        if scene.read_bytes() != before:
            raise RuntimeError('failure modified authoring source')
        report = {'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
                  'standalone_log': log.strip(), 'export_log': exported.stdout.strip(), 'packaged_paths': sorted(expected_paths), 'corrupt_prefab_error': bad_prefab.stderr.strip(),
                  'corrupt_model_error': bad_model.stderr.strip(),
                  'scope': 'actual standalone executable, nested composition, OBJ and textured glTF with external BIN/PNG dependency closure, 120 fixed steps with authored behavior, saved nested UI layout/keyboard events and observed Unicode font/glyph preparation, clipped UI mesh staging and failure diagnostics; game-check alone makes no window/GPU presentation claim'}
        for path in root.rglob('*'):
            if path.is_file() and path != package_path:
                path.unlink()
        packaged_command = [str(binary), '--game-package', str(package_path), '--game-check']
        packaged = subprocess.run(packaged_command, capture_output=True, text=True, timeout=30)
        if packaged.returncode or 'GAME CHECK PASS ticks=120 objects=3 prefab_instances=2' not in packaged.stdout:
            raise RuntimeError('package required deleted project: ' + packaged.stdout + packaged.stderr)
        verify_ui(packaged.stdout)
        verify_audio(packaged.stdout)
        verify_motion(packaged.stdout)
        report['invalid_ui_error'] = invalid_ui_error
        report['invalid_font_error'] = bad_font.stderr.strip()
        report['packaged_run_log'] = (packaged.stdout + packaged.stderr).strip()
        damaged = json.loads(package_bytes)
        damaged['entries']['mesh.obj']['bytes'][0] ^= 1
        package_path.write_text(json.dumps(damaged))
        rejected = subprocess.run(packaged_command, capture_output=True, text=True, timeout=30)
        if not rejected.returncode or 'integrity failure' not in rejected.stderr:
            raise RuntimeError('packaged game accepted damaged source')
        package_path.write_bytes(package_bytes)
        report['damaged_package_error'] = rejected.stderr.strip()
        status = 0
        if '--native' in sys.argv:
            native = subprocess.run(packaged_command[:-1] + ['--game-native-smoke'], capture_output=True, text=True, timeout=45)
            native_log = native.stdout + native.stderr
            passed = native.returncode == 0 and 'GAME NATIVE PASS' in native_log and 'GAME UI GPU PUBLISHED draws=4' in native_log
            report['native'] = {'passed': passed, 'returncode': native.returncode, 'log': native_log.strip()}
            status = 0 if passed else 1
        print(json.dumps(report, indent=2))
        return status



if __name__ == '__main__':
    raise SystemExit(main())
