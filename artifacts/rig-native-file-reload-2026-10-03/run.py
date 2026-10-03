"""Drive the existing native acceptance mode with a real watched GLB revision."""
import argparse
import copy
import json
import os
from pathlib import Path
import plistlib
import shutil
import struct
import subprocess
import tempfile
import time

parser = argparse.ArgumentParser()
parser.add_argument('--binary', required=True, type=Path)
parser.add_argument('--fixture', required=True, type=Path)
parser.add_argument('--output', required=True, type=Path)
parser.add_argument('--motion', action='store_true')
parser.add_argument('--reorder', action='store_true')
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=True)
original = args.fixture.read_bytes()
json_size, json_type = struct.unpack_from('<II', original, 12)
assert json_type == 0x4E4F534A
metadata = json.loads(original[20:20 + json_size])
bin_header = 20 + json_size
bin_size, bin_type = struct.unpack_from('<II', original, bin_header)
assert bin_type == 0x004E4942
payload = bytearray(original[bin_header + 8:bin_header + 8 + bin_size])
def pack_glb(document, binary):
    encoded = json.dumps(document, separators=(',', ':')).encode()
    encoded += b' ' * (-len(encoded) % 4)
    chunks = struct.pack('<II', len(encoded), json_type) + encoded
    chunks += struct.pack('<II', len(binary), bin_type) + binary
    return struct.pack('<III', 0x46546C67, 2, 12 + len(chunks)) + chunks

if args.reorder:
    assert args.motion, '--reorder requires --motion'
    other = copy.deepcopy(metadata['animations'][0])
    other['name'] = 'other'
    view_index = len(metadata['bufferViews'])
    metadata['bufferViews'].append({'buffer': 0, 'byteOffset': len(payload), 'byteLength': 24})
    accessor_index = len(metadata['accessors'])
    metadata['accessors'].append({'bufferView': view_index, 'componentType': 5126, 'count': 2, 'type': 'VEC3'})
    other['samplers'][0]['output'] = accessor_index
    metadata['animations'].append(other)
    payload.extend(struct.pack('<6f', -.1, .5, 0., -.1, .5, 0.))
    metadata['buffers'][0]['byteLength'] = len(payload)
    original = pack_glb(metadata, payload)
accessor = metadata['accessors'][metadata['animations'][0]['samplers'][0]['input']]
assert accessor['componentType'] == 5126 and accessor['count'] == 2
view = metadata['bufferViews'][accessor['bufferView']]
offset = view.get('byteOffset', 0) + accessor.get('byteOffset', 0)
assert struct.unpack_from('<2f', payload, offset) == (0., 1.)
struct.pack_into('<f', payload, offset + 4, 2.)
accessor['max'] = [2.]
if args.motion:
    output = metadata['accessors'][metadata['animations'][0]['samplers'][0]['output']]
    assert output['componentType'] == 5126 and output['count'] == 2 and output['type'] == 'VEC3'
    output_view = metadata['bufferViews'][output['bufferView']]
    output_offset = output_view.get('byteOffset', 0) + output.get('byteOffset', 0)
    struct.pack_into('<3f', payload, output_offset + 12, 0.15, 0.55, 0.)
if args.reorder:
    metadata['animations'].reverse()
revised = pack_glb(metadata, payload)
with tempfile.TemporaryDirectory(prefix='voxy-native-revision-') as directory:
    root = Path(directory)
    executable = root / 'Voxy Clip Revision.app/Contents/MacOS/voxy_app'
    executable.parent.mkdir(parents=True)
    shutil.copy2(args.binary, executable)
    with executable.parent.parent.joinpath('Info.plist').open('wb') as stream:
        plistlib.dump({'CFBundleExecutable': 'voxy_app', 'CFBundleIdentifier': 'com.voxy.cliprevision',
                      'CFBundleName': 'Voxy Clip Revision', 'CFBundlePackageType': 'APPL',
                      'NSHighResolutionCapable': True}, stream)
    fixture = root / 'foot-contact.glb'
    fixture.write_bytes(original)
    env = os.environ.copy()
    env.update(VOXY_FOOT_CONTACT_SMOKE='1', VOXY_FOOT_RELOAD_SMOKE='1', VOXY_EDITOR_TRACE_INPUT='1')
    if args.motion:
        env['VOXY_FOOT_RELOAD_MOTION'] = '1'
    if args.reorder:
        env['VOXY_FOOT_REORDER_SMOKE'] = '1'
    log_path = args.output / 'native.log'
    mutated = False
    with log_path.open('w') as stream:
        process = subprocess.Popen([str(executable), '--model', str(fixture), '--animation-native-smoke'],
                                   env=env, stdout=stream, stderr=subprocess.STDOUT)
        deadline = time.monotonic() + 30
        while process.poll() is None:
            if not mutated and 'VOXY_NATIVE_FOOT_RELOAD_READY' in log_path.read_text():
                pending = root / 'revision.pending'
                pending.write_bytes(revised)
                pending.replace(fixture)
                mutated = True
            if time.monotonic() >= deadline:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
                raise RuntimeError('native clip revision timed out')
            time.sleep(0.01)
    result = process.returncode
    args.output.joinpath('native.exit').write_text(str(result) + '\n')
    text = log_path.read_text()
    assert mutated, text
    assert result == 0, text
    for marker in ['VOXY_NATIVE_FOOT_RELOAD_FADE', 'VOXY_NATIVE_FOOT_CONTACT', 'VOXY_NATIVE_FOOT_STOP']:
        assert marker in text, text
    if args.motion:
        assert 'VOXY_NATIVE_FOOT_RELOAD_MOTION' in text, text
    if args.reorder:
        assert 'VOXY_NATIVE_NAMED_CLIP_REORDER' in text, text
    print('\n'.join(line for line in text.splitlines() if line.startswith('VOXY_NATIVE_')))
