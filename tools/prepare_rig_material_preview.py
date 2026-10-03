#!/usr/bin/env python3
"""Split the pinned Fox into two material primitives without changing its rig."""
import json
from pathlib import Path
import struct
import sys
import zlib

ROOT = Path(__file__).resolve().parents[1]

def png(color):
    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', 1, 1, 8, 6, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(bytes([0, *color]))) + chunk(b'IEND', b'')

def main(output):
    output.mkdir(parents=True, exist_ok=True)
    data = (ROOT / 'crates/voxy_render/examples/assets/fox/Fox.glb').read_bytes()
    size = struct.unpack_from('<I', data, 12)[0]
    document = json.loads(data[20:20 + size])
    binary_start = 20 + size + 8
    binary = data[binary_start:]
    document['buffers'][0]['uri'] = 'fox.bin'
    primitive = document['meshes'][0]['primitives'][0]
    assert 'indices' not in primitive and primitive.get('mode', 4) == 4
    count = document['accessors'][primitive['attributes']['POSITION']]['count']
    assert count % 6 == 0
    split = count // 2
    result = []
    for part in range(2):
        attributes = {}
        for name, index in primitive['attributes'].items():
            accessor = dict(document['accessors'][index])
            assert accessor['count'] == count and 'sparse' not in accessor
            view = document['bufferViews'][accessor['bufferView']]
            width = {'SCALAR': 1, 'VEC2': 2, 'VEC3': 3, 'VEC4': 4}[accessor['type']]
            component = {5121: 1, 5123: 2, 5126: 4}[accessor['componentType']]
            stride = view.get('byteStride', width * component)
            accessor['byteOffset'] = accessor.get('byteOffset', 0) + part * split * stride
            accessor['count'] = split
            attributes[name] = len(document['accessors'])
            document['accessors'].append(accessor)
        result.append({'attributes': attributes, 'material': part, 'mode': 4})
    document['meshes'][0]['primitives'] = result
    # Fit the diagnostic model into the editor's initial identity viewport.
    scene = document['scenes'][document.get('scene', 0)]
    parent = len(document['nodes'])
    document['nodes'].append({'name': 'Diagnostic framing', 'children': scene['nodes'], 'scale': [0.006] * 3, 'translation': [0, -0.3, 0], 'rotation': [0, 0.7071067811865476, 0, 0.7071067811865476]})
    scene['nodes'] = [parent]
    document['images'] = [{'uri': 'magenta.png'}, {'uri': 'cyan.png'}]
    document['textures'] = [{'source': i} for i in range(2)]
    document['materials'] = [{'pbrMetallicRoughness': {'baseColorFactor': [1, 1, 1, 1], 'baseColorTexture': {'index': i}}} for i in range(2)]
    (output / 'fox.bin').write_bytes(binary)
    (output / 'magenta.png').write_bytes(png((255, 0, 255, 255)))
    (output / 'cyan.png').write_bytes(png((0, 255, 255, 255)))
    (output / 'materials.gltf').write_text(json.dumps(document))
    print(output / 'materials.gltf')

if __name__ == '__main__':
    main(Path(sys.argv[1]).resolve())
