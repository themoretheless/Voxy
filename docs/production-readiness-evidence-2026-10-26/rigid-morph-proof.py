#!/usr/bin/env python3
"""Verify unskinned morph node binding and unchanged upstream regression."""
from pathlib import Path
import hashlib
import json
import subprocess
import sys
ROOT=Path(__file__).resolve().parents[2]
EVIDENCE=Path(__file__).resolve().parent
FILES=['crates/voxy_model/src/lib.rs','crates/voxy_model/src/morph.rs','crates/voxy_editor/src/locomotion_tests.rs','crates/voxy_editor/src/animated_models/tests.rs']
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
before={p:sha(ROOT/p) for p in FILES}
base=['cargo','+nightly-2026-05-27'];target=['--target-dir','target/research-ray-precision']
checks=[('rigid-morph-core.log','voxy_model','tests::rigid_morph_without_source_skin_keeps_node_animation_mirror_and_normals',False),
        ('rigid-morph-package.log','voxy_editor','locomotion_tests::unskinned_rigid_morph_project_and_source_free_package_share_full_frames',False),
        ('rigid-morph-no-clips.log','voxy_editor','locomotion_tests::morph_without_skins_or_clips_keeps_defaults_through_project_and_package',False),
        ('rigid-morph-gpu.log','voxy_editor','animated_models::tests::gpu_rigid_morph_without_source_skin_uses_common_owner',True)]
for name,package,test,gpu in checks:
 command=base+['test','-p',package,'--lib',test]+target+['--']+(['--ignored'] if gpu else [])+['--exact','--nocapture']
 with (EVIDENCE/name).open('w') as log:subprocess.run(command,cwd=ROOT,stdout=log,stderr=subprocess.STDOUT,check=True)
assert 'source_skins=0 gpu=true mirrored_nonuniform_node=true cpu_pixels_equal=true last_good_retained=true' in (EVIDENCE/'rigid-morph-gpu.log').read_text()
subprocess.run([sys.executable,str(EVIDENCE/'morph-ordinary-owner-proof.py')],cwd=ROOT,check=True)
upstream=json.loads((EVIDENCE/'morph-ordinary-owner-proof.json').read_text())
assert before=={p:sha(ROOT/p) for p in FILES},'source changed during proof'
record={'status':'unskinned_morph_common_gpu_binding_project_package_pass','code_sha256':before,
        'synthetic_unskinned_fixture':True,'new_nodes_or_bones_added':False,'extra_playback_clock':False,
        'source_node_transform_and_animation_preserved':True,'mirrored_nonuniform_scale_tested':True,
        'authored_normals_and_flat_normal_tag_tested':True,'no_skin_no_clips_no_playback_component_defaults_tested':True,
        'source_free_package_frames_equal':True,'physical_gpu_owner_and_cpu_pixels_equal':True,
        'last_good_gpu_resources_retained_on_tested_failures':True,
        'upstream_unchanged_regression':'morph-ordinary-owner-proof.json','original_source_sha256':upstream['source_sha256'],
        'full_native_image_verified':False,'native_editor_controls_verified':False,'fps_qualified':False,'engine_ready':False}
(EVIDENCE/'rigid-morph-proof.json').write_text(json.dumps(record,indent=2)+'\n')
print('RIGID MORPH PASS: no source skin or added bones, common GPU owner, node transforms/defaults/package retained; upstream regression passed')
