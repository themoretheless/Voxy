#!/usr/bin/env python3
"""Build and inspect the unchanged upstream model through explicit CPU morph import."""
from pathlib import Path
import hashlib
import json
import subprocess

ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = Path(__file__).resolve().parent
SOURCE = ROOT / 'artifacts/locomotion-2026-10-26/upstream-robot/RobotExpressive.glb'
EXPECTED_SOURCE = '047f5e5fb3bb6d378bd1df16ca6137f2a596c99b3a1b5690b4020c05aaf6f319'
BINARY = ROOT / 'target/research-ray-precision/debug/examples/morph_import'
FILES = ['crates/voxy_model/src/lib.rs', 'crates/voxy_model/src/morph.rs',
         'crates/voxy_model/examples/morph_import.rs',
         'crates/voxy_animation/src/property_track.rs',
         'crates/voxy_animation/src/property_playback.rs',
         'crates/voxy_animation/src/lib.rs',
         'crates/voxy_animation/src/rigid_fade.rs',
         'crates/voxy_animation/src/retarget.rs',
         'crates/voxy_render/src/scene/geometry.rs']

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

assert digest(SOURCE) == EXPECTED_SOURCE, 'original asset bytes differ'
source_before = {name: digest(ROOT / name) for name in FILES}
with (EVIDENCE / 'morph-frame-build.log').open('w') as log:
    subprocess.run(['cargo', '+nightly-2026-05-27', 'build', '-p', 'voxy_model',
                    '--example', 'morph_import', '--target-dir', 'target/research-ray-precision'],
                   cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
result = subprocess.run([str(BINARY), str(SOURCE)], cwd='/', capture_output=True, text=True)
(EVIDENCE / 'morph-frame-upstream.log').write_text(result.stderr)
assert result.returncode == 0, result.stderr
report = json.loads(result.stdout)
assert report['mesh_instances'] == 14 and report['primitives'] == 19
assert report['morph_primitives'] == 3 and report['joints'] == 160
assert len(report['clips']) == 14
assert sum(clip['samples'] for clip in report['clips']) == 854
assert all(clip['distinct_geometry_samples'] > 1 for clip in report['clips'])
assert all(clip['distinct_weight_samples'] == 1 for clip in report['clips']), 'reinspect original constant weight channels'
assert len(report['explicit_shape_probes']) == 3
assert all(probe['changed_vertices'] > 0 and probe['max_displacement'] > 0
           for probe in report['explicit_shape_probes'])
assert report['animator_property_frames'] == 112
assert report['default_import_error'] == 'only triangle primitives without morph targets are supported'
assert digest(SOURCE) == EXPECTED_SOURCE
assert source_before == {name: digest(ROOT / name) for name in FILES}, 'source changed during proof'
record = {'status': 'original_asset_animator_property_frame_pass_editor_gpu_pending',
          'source_sha256': EXPECTED_SOURCE, 'source_bytes_unchanged': True,
          'code_sha256': source_before, 'binary_sha256': digest(BINARY),
          'exit_code': result.returncode, 'cwd': '/', 'report': report,
          'weight_channel_note': 'Original source channels are constant zero; nonzero deformation is tested by explicit weights without modifying the GLB.',
          'animator_property_frames_verified': True,
          'editor_accepted_morph_integrated': False, 'gpu_morph_integrated': False,
          'native_visuals_verified': False, 'fps_qualified': False, 'engine_ready': False}
(EVIDENCE / 'morph-frame-proof.json').write_text(json.dumps(record, indent=2) + '\n')
print('MORPH FRAME PASS: original asset, 112 Animator property frames with interrupted fades; editor/GPU pending')
