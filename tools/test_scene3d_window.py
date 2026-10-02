#!/usr/bin/env python3
"""Native static-3D integration; requires presented frames on a real GPU/window."""
import argparse
import copy
import struct
import json
import pathlib
import shutil
import subprocess
import tempfile


def add_material_parts(fixture):
    path = fixture / 'assembly.glb'
    data = path.read_bytes()
    size = struct.unpack_from('<I', data, 12)[0]
    spec = json.loads(data[20:20 + size])
    binary = data[28 + size:]
    spec['accessors'][2]['count'] = 18
    second = copy.deepcopy(spec['accessors'][2])
    second['byteOffset'] = 36
    spec['accessors'].append(second)
    material = copy.deepcopy(spec['materials'][0])
    material['pbrMetallicRoughness']['baseColorFactor'] = [0.1, 0.5, 1, 1]
    sampler = copy.deepcopy(spec.get('samplers', [{}])[0])
    sampler.update(minFilter=9728, magFilter=9728)
    spec.setdefault('samplers', []).append(sampler)
    texture = copy.deepcopy(spec['textures'][0])
    texture['sampler'] = len(spec['samplers']) - 1
    spec['textures'].append(texture)
    material['pbrMetallicRoughness']['baseColorTexture']['index'] = len(spec['textures']) - 1
    spec['materials'].append(material)
    primitive = copy.deepcopy(spec['meshes'][0]['primitives'][0])
    primitive.update(indices=3, material=1)
    spec['meshes'][0]['primitives'].append(primitive)
    encoded = json.dumps(spec).encode()
    encoded += b' ' * (-len(encoded) % 4)
    path.write_bytes(struct.pack('<III', 0x46546c67, 2, 28 + len(encoded) + len(binary))
        + struct.pack('<II', len(encoded), 0x4e4f534a) + encoded
        + struct.pack('<II', len(binary), 0x004e4942) + binary)
    scene_path = fixture / 'scene.json'
    scene = json.loads(scene_path.read_text())
    for parent_index, parent in enumerate(scene['objects'][4:6]):
        for primitive_index in range(2):
            part = copy.deepcopy(parent)
            node = 3 + parent_index * 2 + primitive_index
            part.update(id=f'model-{node + 3}', parent=parent['id'],
                name=parent['name'] + f'/primitive-{primitive_index}',
                translation=[0, 0, 0], rotation=[0, 0, 0, 1], scale=[1, 1, 1])
            part['components'] = {'editor.model.v1': 'assembly',
                'editor.model-part.v1': {'node': node}}
            scene['objects'].append(part)
    scene_path.write_text(json.dumps(scene))


def main():
    root = pathlib.Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', default=str(root / 'target/debug/voxy_editor'))
    parser.add_argument('--multi-material', action='store_true')
    parser.add_argument('--shared-images', action='store_true')
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='voxy-scene3d-') as temporary:
        fixture = pathlib.Path(temporary) / 'scene3d'
        shutil.copytree(root / 'crates/voxy_editor/examples/scene3d', fixture)
        if args.multi_material or args.shared_images:
            add_material_parts(fixture)
        if args.shared_images:
            shutil.copyfile(fixture / 'assembly.glb', fixture / 'assembly-copy.glb')
            manifest = json.loads((fixture / 'assets.json').read_text())
            manifest['assets'].append({'asset': 'assembly-copy', 'source': 'assembly-copy.glb'})
            (fixture / 'assets.json').write_text(json.dumps(manifest))
            document = json.loads((fixture / 'scene.json').read_text())
            part = copy.deepcopy(document['objects'][6])
            part.update(id='shared-image-copy', parent=None, name='Shared image copy',
                translation=[-0.35, -0.35, 0.5], scale=[0.05, 0.05, 0.05])
            part['components']['editor.model.v1'] = 'assembly-copy'
            document['objects'].append(part)
            (fixture / 'scene.json').write_text(json.dumps(document))
        scene = fixture / 'scene.json'
        before = json.loads(scene.read_text())
        try:
            result = subprocess.run([str(pathlib.Path(args.binary).resolve()), '--manifest',
                str(fixture / 'assets.json'), 'assembly', '--scene', str(scene), '--scene3d-smoke'],
                capture_output=True, text=True, timeout=180, check=False)
        except subprocess.TimeoutExpired as error:
            for output in [error.stdout, error.stderr]:
                if output:
                    print(output.decode() if isinstance(output, bytes) else output, end='')
            raise SystemExit('native 3D launch/run exceeded 180 seconds') from error
        print(result.stdout, end='')
        print(result.stderr, end='')
        if result.returncode:
            raise SystemExit(result.returncode)
        for marker in ['SCENE3D GPU GEOMETRY BUDGET PASS', 'SCENE3D GPU IMAGE BUDGET PASS', 'SCENE3D GPU SHARING MIPS PASS', 'SCENE3D CAMERA PICK TEXTURE PERSIST PASS', 'SCENE3D PLAY STOP RELOAD PASS', 'SCENE3D NATIVE PASS']:
            if marker not in result.stdout:
                raise SystemExit('missing native marker: ' + marker)
        if args.shared_images and any(marker not in result.stdout for marker in
                ['SCENE3D CROSS RESOURCE IMAGE PASS', 'SCENE3D GPU EVICT RESTORE PASS', 'SCENE3D GPU ACTIVITY EVICTION PASS']):
            raise SystemExit('missing cross-resource GPU identity check')
        after = json.loads(scene.read_text())
        for old, new in zip(before['objects'], after['objects'], strict=True):
            for field in ['id', 'parent', 'translation', 'rotation', 'scale']:
                # Saving floats may round f64 source constants to authored f32.
                if field == 'rotation':
                    assert all(abs(a-b) < 1e-6 for a,b in zip(old[field], new[field], strict=True))
                else:
                    assert old[field] == new[field], (field, old[field], new[field])
        print('SCENE3D TEST PASS: native camera rays, texture publication, physics, persistence, Play/Stop/reload')


if __name__ == '__main__':
    main()
