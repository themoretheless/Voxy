"""Launch ordinary native Play/Stop with a clipless target and distinct source rig."""
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

def require_native_markers(text, markers):
    """Require exact marker tokens in execution order, not substring matches."""
    lines = text.splitlines()
    after = -1
    for marker in markers:
        matches = [i for i, line in enumerate(lines)
                   if i > after and line.startswith(marker + ' ')]
        if not matches:
            raise AssertionError(f'missing or out-of-order native marker: {marker}\n{text[-12000:]}')
        after = matches[0]

parser = argparse.ArgumentParser()
parser.add_argument('--binary', type=Path, required=True)
parser.add_argument('--fixture', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
parser.add_argument('--angular', action='store_true')
parser.add_argument('--permute', action='store_true')
args = parser.parse_args()
assert not args.permute or args.angular
args.output.mkdir(parents=True, exist_ok=True)
original = args.fixture.read_bytes()
json_size, json_type = struct.unpack_from('<II', original, 12)
assert json_type == 0x4E4F534A
metadata = json.loads(original[20:20+json_size])
position = 20+json_size
bin_size, bin_type = struct.unpack_from('<II', original, position)
assert bin_type == 0x004E4942
payload = bytearray(original[position+8:position+8+bin_size])
def pack_glb(document, binary):
    encoded = json.dumps(document, separators=(',', ':')).encode()
    encoded += b' ' * (-len(encoded) % 4)
    chunks = struct.pack('<II', len(encoded), json_type) + encoded
    chunks += struct.pack('<II', len(binary), bin_type) + binary
    return struct.pack('<III', 0x46546C67, 2, 12+len(chunks)) + chunks

target = copy.deepcopy(metadata)
target['animations'] = []
if args.angular:
    metadata['nodes'][0]['name'] = 'sourceRoot'
    target['nodes'][0]['name'] = 'targetRoot'
    target['nodes'][0]['translation'] = [0., .8, 0.] if args.permute else [.8, 0., 0.]
else:
    for index, name in enumerate(['sourceHip', 'sourceKnee', 'sourceFoot']):
        metadata['nodes'][index]['name'] = name
    metadata['nodes'][0]['translation'] = [0., .6, 0.]
    accessor = metadata['accessors'][metadata['animations'][0]['samplers'][0]['output']]
    assert accessor['count'] == 2 and accessor['componentType'] == 5126
    view = metadata['bufferViews'][accessor['bufferView']]
    offset = view.get('byteOffset', 0)+accessor.get('byteOffset', 0)
    for key in range(2):
        at = offset + key*12+4
        value = struct.unpack_from('<f', payload, at)[0]+.1+key*.05
        struct.pack_into('<f', payload, at, value)
with tempfile.TemporaryDirectory(prefix='voxy-native-retarget-') as directory:
    root = Path(directory)
    executable = root/'Voxy Retarget.app/Contents/MacOS/voxy_app'
    executable.parent.mkdir(parents=True)
    shutil.copy2(args.binary, executable)
    with executable.parent.parent.joinpath('Info.plist').open('wb') as stream:
        plistlib.dump({'CFBundleExecutable':'voxy_app','CFBundleIdentifier':'com.voxy.retarget',
                      'CFBundleName':'Voxy Retarget','CFBundlePackageType':'APPL',
                      'NSHighResolutionCapable':True}, stream)
    # Target binary must retain original geometry/bind data.
    original_bin = original[position+8:position+8+bin_size]
    fixture = root/'target.glb'
    fixture.write_bytes(pack_glb(target, original_bin))
    root.joinpath('source.glb').write_bytes(pack_glb(metadata, payload))
    env = os.environ.copy()
    env['VOXY_EDITOR_TRACE_INPUT'] = '1'
    if args.angular:
        env['VOXY_RETARGET_ROTATION_SMOKE'] = '1'
        if args.permute:
            env['VOXY_RETARGET_ROTATION_PERMUTE'] = '1'
    else:
        env.update(VOXY_FOOT_CONTACT_SMOKE='1', VOXY_FOOT_RETARGET_SMOKE='1')
    log = args.output/'native.log'
    with log.open('w') as stream:
        process = subprocess.Popen([str(executable),'--model',str(fixture),'--animation-native-smoke'],
                                   env=env,stdout=stream,stderr=subprocess.STDOUT)
        try:
            result = process.wait(timeout=45)
        except subprocess.TimeoutExpired:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
            raise RuntimeError('native retarget timed out')
    args.output.joinpath('native.exit').write_text(str(result)+'\n')
    text = log.read_text()
    assert result == 0, text[-12000:]
    markers = ['VOXY_NATIVE_RETARGET_BONE_PICK', 'VOXY_NATIVE_RETARGET_AUTHORING', 'VOXY_NATIVE_RETARGET_ROTATION', 'VOXY_NATIVE_RETARGET_ROTATION_STOP'] if args.angular else ['VOXY_NATIVE_RETARGET','VOXY_NATIVE_FOOT_CONTACT','VOXY_NATIVE_FOOT_STOP']
    require_native_markers(text, markers)
    print('\n'.join(line for line in text.splitlines() if line.startswith('VOXY_NATIVE_')))
