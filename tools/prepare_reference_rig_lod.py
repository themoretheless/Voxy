"""Produce a position-only LOD candidate with pinned research meshoptimizer.

The output still requires Voxy certification and pose/camera acceptance.
This offline probe does not certify skin attributes, normals or visual quality.
"""
import argparse
import ctypes
import hashlib
import json
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tempfile


def accessor(document, blob, index):
    value = document['accessors'][index]
    assert 'sparse' not in value
    view = document['bufferViews'][value['bufferView']]
    assert view['buffer'] == 0
    scalar = {5121: 'B', 5123: 'H', 5125: 'I', 5126: 'f'}[value['componentType']]
    width = {'SCALAR': 1, 'VEC3': 3}[value['type']]
    fmt = '<' + scalar * width
    size = struct.calcsize(fmt)
    stride = view.get('byteStride', size)
    offset = view.get('byteOffset', 0) + value.get('byteOffset', 0)
    assert stride >= size and value['count'] <= 1_572_864
    return [struct.unpack_from(fmt, blob, offset + i * stride) for i in range(value['count'])]


def main():
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    model = root / 'crates/voxy_render/examples/assets/rigged-figure/RiggedFigure.glb'
    raw = model.read_bytes()
    assert struct.unpack_from('<III', raw) == (0x46546c67, 2, len(raw))
    json_length, json_type = struct.unpack_from('<II', raw, 12)
    assert json_type == 0x4e4f534a
    document = json.loads(raw[20:20 + json_length])
    blob_length, blob_type = struct.unpack_from('<II', raw, 20 + json_length)
    assert blob_type == 0x004e4942
    blob = raw[28 + json_length:28 + json_length + blob_length]
    primitive = document['meshes'][0]['primitives'][0]
    positions = accessor(document, blob, primitive['attributes']['POSITION'])
    indices = [value[0] for value in accessor(document, blob, primitive['indices'])]
    research = root / 'docs/engine-research/mechanisms/godot-lod'
    manifest = json.loads((research / 'sources.json').read_text())
    names = ['meshoptimizer.h', 'simplifier.cpp', 'allocator.cpp']
    with tempfile.TemporaryDirectory(prefix='voxy-rig-lod-') as directory:
        work = Path(directory)
        for name in names:
            source = 'thirdparty__meshoptimizer__' + name
            entry = next(item for item in manifest['sources'] if item['file'] == source)
            assert hashlib.sha256((research / source).read_bytes()).hexdigest() == entry['sha256']
            shutil.copyfile(research / source, work / name)
        library = work / ('meshoptimizer.dylib' if sys.platform == 'darwin' else 'meshoptimizer.so')
        subprocess.run(['clang++', '-std=c++17', '-O2', '-dynamiclib' if sys.platform == 'darwin' else '-shared', '-fPIC',
                        str(work / 'simplifier.cpp'), str(work / 'allocator.cpp'), '-o', str(library)], check=True, timeout=60)
        function = ctypes.CDLL(str(library)).meshopt_simplify
        function.argtypes = [ctypes.POINTER(ctypes.c_uint), ctypes.POINTER(ctypes.c_uint), ctypes.c_size_t,
                             ctypes.POINTER(ctypes.c_float), ctypes.c_size_t, ctypes.c_size_t, ctypes.c_size_t,
                             ctypes.c_float, ctypes.c_uint, ctypes.POINTER(ctypes.c_float)]
        function.restype = ctypes.c_size_t
        source_indices = (ctypes.c_uint * len(indices))(*indices)
        target_indices = (ctypes.c_uint * len(indices))()
        vertices = (ctypes.c_float * (len(positions) * 3))(*(x for position in positions for x in position))
        error = ctypes.c_float()
        count = function(target_indices, source_indices, len(indices), vertices, len(positions), 12,
                         len(indices) // 6 * 3, 0.02, 0, ctypes.byref(error))
        result = list(target_indices[:count])
        assert 0 < count < len(indices) and count % 3 == 0 and max(result) < len(positions)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    for name in ['variant.txt', 'RiggedFigure.glb', 'candidate.json']:
        if (output / name).exists():
            raise RuntimeError('refusing to overwrite existing candidate: ' + name)
    (output / 'variant.txt').write_text(' '.join(map(str, result)) + '\n')
    shutil.copyfile(model, output / 'RiggedFigure.glb')
    report = {'meshoptimizer_source': manifest['commit'], 'input_sha256': hashlib.sha256(raw).hexdigest(),
              'vertices': len(positions), 'base_indices': len(indices), 'variant_indices': count,
              'upstream_relative_error_estimate': error.value,
              'scope': 'Position-only offline candidate; skin/pose/normal/visual acceptance required'}
    (output / 'candidate.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report))


if __name__ == '__main__':
    main()
