# Facial rendering source scan: intoro/MakeHumanForBlender

Repository: https://github.com/intoro/MakeHumanForBlender

Pinned source revision: 8b0c95acad9f53629513bdebea39ac84ee8780a7

Discovery topic: MakeHuman

Description: Hairnet program and blender Add-On

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["blender_plugins_for_1.1.1/import_runtime_mhx2/license.txt", "blender_plugins_for_1.1.1/makeclothes/license.txt", "blender_plugins_for_1.1.1/maketarget/license.txt", "blender_plugins_for_1.1.1/makewalk/license.txt", "makehuman-1.1.1-win32/blendertools/makeclothes/license.txt", "makehuman-1.1.1-win32/blendertools/maketarget/license.txt", "makehuman-1.1.1-win32/blendertools/makewalk/license.txt", "makehuman-1.1.1-win32/license.txt", "makehuman-1.1.1-win32/tk/license.terms"]

Topic paths: {"eye": ["blender_plugins_for_1.1.1/maketarget/data/eye.json", "blender_plugins_for_1.1.1/makewalk/source_rigs/eyes.src", "blender_plugins_for_1.1.1/makewalk/source_rigs/eyes_tpose.json", "makehuman-1.1.1-win32/blendertools/maketarget/data/eye.json", "makehuman-1.1.1-win32/blendertools/makewalk/source_rigs/eyes.src", "makehuman-1.1.1-win32/blendertools/makewalk/source_rigs/eyes_tpose.json", "makehuman-1.1.1-win32/data/eyebrows/clear.thumb", "makehuman-1.1.1-win32/data/eyebrows/eyebrow001/eyebrow001.mhmat", "makehuman-1.1.1-win32/data/eyebrows/eyebrow001/eyebrow001.mhpxy", "makehuman-1.1.1-win32/data/eyebrows/eyebrow001/eyebrow001.npz", "makehuman-1.1.1-win32/data/eyebrows/eyebrow001/eyebrow001.png", "makehuman-1.1.1-win32/data/eyebrows/eyebrow001/eyebrow001.thumb", "makehuman-1.1.1-win32/data/eyebrows/eyebrow002/eyebrow002.mhmat", "makehuman-1.1.1-win32/data/eyebrows/eyebrow002/eyebrow002.mhpxy", "makehuman-1.1.1-win32/data/eyebrows/eyebrow002/eyebrow002.npz", "makehuman-1.1.1-win32/data/eyebrows/eyebrow002/eyebrow002.png", "makehuman-1.1.1-win32/data/eyebrows/eyebrow002/eyebrow002.thumb", "makehuman-1.1.1-win32/data/eyebrows/eyebrow003/eyebrow003.mhmat", "makehuman-1.1.1-win32/data/eyebrows/eyebrow003/eyebrow003.mhpxy", "makehuman-1.1.1-win32/data/eyebrows/eyebrow003/eyebrow003.npz", "makehuman-1.1.1-win32/data/eyebrows/eyebrow003/eyebrow003.png", "makehuman-1.1.1-win32/data/eyebrows/eyebrow003/eyebrow003.thumb", "makehuman-1.1.1-win32/data/eyebrows/eyebrow004/eyebrow004.mhmat", "makehuman-1.1.1-win32/data/eyebrows/eyebrow004/eyebrow004.mhpxy", "makehuman-1.1.1-win32/data/eyebrows/eyebrow004/eyebrow004.npz", "makehuman-1.1.1-win32/data/eyebrows/eyebrow004/eyebrow004.png", "makehuman-1.1.1-win32/data/eyebrows/eyebrow004/eyebrow004.thumb", "makehuman-1.1.1-win32/data/eyebrows/eyebrow005/eyebrow005.mhmat", "makehuman-1.1.1-win32/data/eyebrows/eyebrow005/eyebrow005.mhpxy", "makehuman-1.1.1-win32/data/eyebrows/eyebrow005/eyebrow005.npz"], "skin": ["makehuman-1.1.1-win32/data/litspheres/lit_standard_skin.png", "makehuman-1.1.1-win32/data/litspheres/skinmat_african.png", "makehuman-1.1.1-win32/data/litspheres/skinmat_asian.png", "makehuman-1.1.1-win32/data/litspheres/skinmat_caucasian.png", "makehuman-1.1.1-win32/data/litspheres/skinmat_eye.png", "makehuman-1.1.1-win32/data/povray/skin.inc", "makehuman-1.1.1-win32/data/shaders/glsl/skin_fragment_shader.txt", "makehuman-1.1.1-win32/data/shaders/glsl/skin_vertex_shader.txt", "makehuman-1.1.1-win32/data/skins/default.mhmat", "makehuman-1.1.1-win32/data/skins/default.thumb", "makehuman-1.1.1-win32/data/skins/middleage_african_female/middleage_african_female.mhmat", "makehuman-1.1.1-win32/data/skins/middleage_african_female/middleage_african_female.thumb", "makehuman-1.1.1-win32/data/skins/middleage_african_male/middleage_african_male.mhmat", "makehuman-1.1.1-win32/data/skins/middleage_african_male/middleage_african_male.thumb", "makehuman-1.1.1-win32/data/skins/middleage_asian_female/middleage_asian_female.mhmat", "makehuman-1.1.1-win32/data/skins/middleage_asian_female/middleage_asian_female.thumb", "makehuman-1.1.1-win32/data/skins/middleage_asian_male/middleage_asian_male.mhmat", "makehuman-1.1.1-win32/data/skins/middleage_asian_male/middleage_asian_male.thumb", "makehuman-1.1.1-win32/data/skins/middleage_caucasian_female/middleage_caucasian_female.mhmat", "makehuman-1.1.1-win32/data/skins/middleage_caucasian_female/middleage_caucasian_female.thumb", "makehuman-1.1.1-win32/data/skins/middleage_caucasian_male/middleage_caucasian_male.mhmat", "makehuman-1.1.1-win32/data/skins/middleage_caucasian_male/middleage_caucasian_male.thumb", "makehuman-1.1.1-win32/data/skins/notfound.thumb", "makehuman-1.1.1-win32/data/skins/old_african_female/old_african_female.mhmat", "makehuman-1.1.1-win32/data/skins/old_african_female/old_african_female.thumb", "makehuman-1.1.1-win32/data/skins/old_african_male/old_african_male.mhmat", "makehuman-1.1.1-win32/data/skins/old_african_male/old_african_male.thumb", "makehuman-1.1.1-win32/data/skins/old_asian_female/old_asian_female.mhmat", "makehuman-1.1.1-win32/data/skins/old_asian_female/old_asian_female.thumb", "makehuman-1.1.1-win32/data/skins/old_asian_male/old_asian_male.mhmat"], "sss": ["makehuman-1.1.1-win32/data/povray/sss.inc"], "jaw": ["makehuman-1.1.1-win32/data/targets/chin/images/chin-jaw-drop-decr-incr.png"], "mouth": ["blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/mouth_corner_down.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/mouth_corner_in.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/mouth_corner_up.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/mouth_down.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/mouth_narrow.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/mouth_open.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/mouth_up.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/mouth_wide.target", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-angles-down-up.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-cupidsbow-decr-incr.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-cupidsbow-width-decr-incr.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-dimples-in-out.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-laugh-lines-in-out.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-lowerlip-ext-down-up.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-lowerlip-height-decr-incr.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-lowerlip-middle-down-up.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-lowerlip-volume-decr-incr.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-lowerlip-width-decr-incr.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-philtrum-volume-decr-incr.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-scale-depth-decr-incr.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-scale-horiz-decr-incr.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-scale-vert-decr-incr.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-trans-backward-forward.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-trans-down-up.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-trans-in-out.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-up-down.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-upperlip-ext-down-up.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-upperlip-height-decr-incr.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-upperlip-middle-down-up.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-upperlip-volume-decr-incr.png"], "teeth": ["makehuman-1.1.1-win32/data/teeth/materials/teeth.mhmat", "makehuman-1.1.1-win32/data/teeth/materials/teeth.png", "makehuman-1.1.1-win32/data/teeth/materials/teeth.thumb", "makehuman-1.1.1-win32/data/teeth/teeth_base/teeth_base.mhpxy", "makehuman-1.1.1-win32/data/teeth/teeth_base/teeth_base.npz", "makehuman-1.1.1-win32/data/teeth/teeth_base/teeth_base.thumb", "makehuman-1.1.1-win32/data/teeth/teeth_shape01/teeth_shape01.mhpxy", "makehuman-1.1.1-win32/data/teeth/teeth_shape01/teeth_shape01.npz", "makehuman-1.1.1-win32/data/teeth/teeth_shape01/teeth_shape01.thumb", "makehuman-1.1.1-win32/data/teeth/teeth_shape02/teeth_shape02.mhpxy", "makehuman-1.1.1-win32/data/teeth/teeth_shape02/teeth_shape02.npz", "makehuman-1.1.1-win32/data/teeth/teeth_shape02/teeth_shape02.thumb", "makehuman-1.1.1-win32/data/teeth/teeth_shape03/teeth_shape03.mhpxy", "makehuman-1.1.1-win32/data/teeth/teeth_shape03/teeth_shape03.npz", "makehuman-1.1.1-win32/data/teeth/teeth_shape03/teeth_shape03.thumb", "makehuman-1.1.1-win32/data/teeth/teeth_shape04/teeth_shape04.mhpxy", "makehuman-1.1.1-win32/data/teeth/teeth_shape04/teeth_shape04.npz", "makehuman-1.1.1-win32/data/teeth/teeth_shape04/teeth_shape04.thumb", "makehuman-1.1.1-win32/data/teeth/teeth_shape05/teeth_shape05.mhpxy", "makehuman-1.1.1-win32/data/teeth/teeth_shape05/teeth_shape05.npz", "makehuman-1.1.1-win32/data/teeth/teeth_shape05/teeth_shape05.thumb", "makehuman-1.1.1-win32/plugins/3_libraries_teeth.py"], "tongue": ["blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/tongue_back_up.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/tongue_out.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/tongue_up.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/tongue_wide.target", "makehuman-1.1.1-win32/data/tongue/tongue01/tongue01.mhmat", "makehuman-1.1.1-win32/data/tongue/tongue01/tongue01.mhpxy", "makehuman-1.1.1-win32/data/tongue/tongue01/tongue01.npz", "makehuman-1.1.1-win32/data/tongue/tongue01/tongue01.thumb", "makehuman-1.1.1-win32/data/tongue/tongue01/tongue01_diffuse.png", "makehuman-1.1.1-win32/plugins/3_libraries_tongue.py"], "wrinkle": ["blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/nose_wrinkle.target"], "deform": ["makehuman-1.1.1-win32/plugins/9_export_fbx/fbx_deformer.py"], "shader": ["blender_plugins_for_1.1.1/import_runtime_mhx2/shaders.py", "makehuman-1.1.1-win32/data/shaders/glsl/litsphere_fragment_shader.txt", "makehuman-1.1.1-win32/data/shaders/glsl/litsphere_vertex_shader.txt", "makehuman-1.1.1-win32/data/shaders/glsl/normalmap_fragment_shader.txt", "makehuman-1.1.1-win32/data/shaders/glsl/normalmap_vertex_shader.txt", "makehuman-1.1.1-win32/data/shaders/glsl/phong_fragment_shader.txt", "makehuman-1.1.1-win32/data/shaders/glsl/phong_vertex_shader.txt", "makehuman-1.1.1-win32/data/shaders/glsl/skin_fragment_shader.txt", "makehuman-1.1.1-win32/data/shaders/glsl/skin_vertex_shader.txt", "makehuman-1.1.1-win32/data/shaders/glsl/toon_fragment_shader.txt", "makehuman-1.1.1-win32/data/shaders/glsl/toon_vertex_shader.txt", "makehuman-1.1.1-win32/data/shaders/glsl/xray_fragment_shader.txt", "makehuman-1.1.1-win32/data/shaders/glsl/xray_vertex_shader.txt"], "rig": ["blender_plugins_for_1.1.1/import_runtime_mhx2/armature/data/rigs/base.json", "blender_plugins_for_1.1.1/import_runtime_mhx2/armature/data/rigs/daz.json", "blender_plugins_for_1.1.1/import_runtime_mhx2/armature/data/rigs/exported_mhx.json", "blender_plugins_for_1.1.1/import_runtime_mhx2/armature/data/rigs/exported_rigify.json", "blender_plugins_for_1.1.1/import_runtime_mhx2/armature/data/rigs/game.json", "blender_plugins_for_1.1.1/import_runtime_mhx2/armature/data/rigs/humanik.json", "blender_plugins_for_1.1.1/import_runtime_mhx2/armature/data/rigs/mhx.json", "blender_plugins_for_1.1.1/import_runtime_mhx2/armature/data/rigs/rigify.json", "blender_plugins_for_1.1.1/import_runtime_mhx2/armature/data/rigs/second_life.json", "blender_plugins_for_1.1.1/import_runtime_mhx2/armature/data/rigs/xonotic.json", "blender_plugins_for_1.1.1/import_runtime_mhx2/armature/data/vertexgroups/hm8/vgrp_leftright.json", "blender_plugins_for_1.1.1/import_runtime_mhx2/armature/rerig.py", "blender_plugins_for_1.1.1/import_runtime_mhx2/armature/rig_arm.py", "blender_plugins_for_1.1.1/import_runtime_mhx2/armature/rig_control.py", "blender_plugins_for_1.1.1/import_runtime_mhx2/armature/rig_face.py", "blender_plugins_for_1.1.1/import_runtime_mhx2/armature/rig_hand.py", "blender_plugins_for_1.1.1/import_runtime_mhx2/armature/rig_joints.py", "blender_plugins_for_1.1.1/import_runtime_mhx2/armature/rig_leg.py", "blender_plugins_for_1.1.1/import_runtime_mhx2/armature/rig_merge.py", "blender_plugins_for_1.1.1/import_runtime_mhx2/armature/rig_panel.py", "blender_plugins_for_1.1.1/import_runtime_mhx2/armature/rig_rigify.py", "blender_plugins_for_1.1.1/import_runtime_mhx2/armature/rig_spine.py", "blender_plugins_for_1.1.1/import_runtime_mhx2/armature/rigify.py", "blender_plugins_for_1.1.1/makewalk/source_rigs/accad.src", "blender_plugins_for_1.1.1/makewalk/source_rigs/accad_tpose.json", "blender_plugins_for_1.1.1/makewalk/source_rigs/cmu_tpose.json", "blender_plugins_for_1.1.1/makewalk/source_rigs/eyes.src", "blender_plugins_for_1.1.1/makewalk/source_rigs/eyes_tpose.json", "blender_plugins_for_1.1.1/makewalk/source_rigs/max.src", "blender_plugins_for_1.1.1/makewalk/source_rigs/mb.src"], "face": ["blender_plugins_for_1.1.1/import_runtime_mhx2/armature/data/mhx/gizmos-face.json", "blender_plugins_for_1.1.1/import_runtime_mhx2/armature/data/vertexgroups/hm8/vgrp_face.json", "blender_plugins_for_1.1.1/import_runtime_mhx2/armature/rig_face.py", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/faceshapes.mxa", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/brow_mid_down.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/brow_mid_up.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/brow_outer_down.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/brow_outer_up.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/brow_squeeze.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/cheek_balloon.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/cheek_narrow.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/cheek_squint.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/cheek_up.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/lips_lower_in.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/lips_lower_out.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/lips_mid_lower_down.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/lips_mid_lower_up.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/lips_mid_upper_down.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/lips_mid_upper_up.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/lips_part.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/lips_upper_in.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/lips_upper_out.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/mouth_corner_down.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/mouth_corner_in.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/mouth_corner_up.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/mouth_down.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/mouth_narrow.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/mouth_open.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/mouth_up.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/mouth_wide.target"], "lip": ["blender_plugins_for_1.1.1/import_runtime_mhx2/armature/data/vertexgroups/hm8/vgrp_upper_lower_lip.json", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/lips_lower_in.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/lips_lower_out.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/lips_mid_lower_down.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/lips_mid_lower_up.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/lips_mid_upper_down.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/lips_mid_upper_up.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/lips_part.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/lips_upper_in.target", "blender_plugins_for_1.1.1/import_runtime_mhx2/data/hm8/faceshapes/raw/lips_upper_out.target", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-lowerlip-ext-down-up.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-lowerlip-height-decr-incr.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-lowerlip-middle-down-up.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-lowerlip-volume-decr-incr.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-lowerlip-width-decr-incr.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-upperlip-ext-down-up.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-upperlip-height-decr-incr.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-upperlip-middle-down-up.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-upperlip-volume-decr-incr.png", "makehuman-1.1.1-win32/data/targets/mouth/images/mouth-upperlip-width-decr-incr.png"]}

## Source file: blender_plugins_for_1.1.1/import_runtime_mhx2/armature/rig_face.py

{
  "path": "blender_plugins_for_1.1.1/import_runtime_mhx2/armature/rig_face.py",
  "blob_sha": "ada35dcd91933a2c955bbf6341d3702b0a9502db",
  "sha256": "3fffcd46864f343aa2dcda8c937199eb0438c2491c53cd2a8c39fe33fc155f4a",
  "lines": 152,
  "bytes": 5075,
  "selection_score": 30,
  "keyword_line_counts": {
    "eye": 19,
    "jaw": 6,
    "mouth": 1,
    "tongue": 9,
    "deform": 2,
    "rig": 1
  },
  "url": "https://github.com/intoro/MakeHumanForBlender/blob/8b0c95acad9f53629513bdebea39ac84ee8780a7/blender_plugins_for_1.1.1/import_runtime_mhx2/armature/rig_face.py",
  "function_signature_candidates": [
    {
      "line": 138,
      "text": "def DeformDrivers(fp, amt):"
    }
  ]
}

```text

# ##### BEGIN GPL LICENSE BLOCK #####
#
#  Authors:             Thomas Larsson
#  Script copyright (C) Thomas Larsson 2014
#
#  This program is free software; you can redistribute it and/or
#  modify it under the terms of the GNU General Public License
#  as published by the Free Software Foundation; either version 2
#  of the License, or (at your option) any later version.
#
#  This program is distributed in the hope that it will be useful,
#  but WITHOUT ANY WARRANTY; without even the implied warranty of
#  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
#  GNU General Public License for more details.
#
#  You should have received a copy of the GNU General Public License
#  along with this program; if not, write to the Free Software Foundation,
#  Inc., 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301, USA.
#
# ##### END GPL LICENSE BLOCK #####

from .flags import *

UseTranslationBones = False
LidPct = 0.5
eyeOffs = (0,0,0.3)
dy = (0,1,0)

Joints = [
    ('head-end',        'l', ((2.0, 'head'), (-1.0, 'neck'))),

    ('l-pupil',         'vl', ((0.5, 14625), (0.5, 14661))),
    ('r-pupil',         'vl', ((0.5, 14670), (0.5, 14706))),

    ('l-eye-end',        'l', ((1.5, 'l-pupil'), (-0.5, 'l-eye'))),
    ('r-eye-end',        'l', ((1.5, 'r-pupil'), (-0.5, 'r-eye'))),

    ('l-eye-top',       'o', ('l-eye-end', dy)),
    ('r-eye-top',       'o', ('r-eye-end', dy)),

    ('l-uplid1',       'v', 6785),
    ('l-lolid1',       'v', 11479),
    ('r-uplid1',       'v', 4847),
    ('r-lolid1',       'v', 4861),

    ('l-uplid0',        'p', ('l-uplid1', 'l-eye', 'l-eye')),
    ('l-lolid0',        'p', ('l-lolid1', 'l-eye', 'l-eye')),
    ('r-uplid0',        'p', ('r-uplid1', 'r-eye', 'r-eye')),
    ('r-lolid0',        'p', ('r-lolid1', 'r-eye', 'r-eye')),

    ('l-uplid',        'l', ((1.5, 'l-uplid1'), (-0.5, 'l-uplid0'))),
    ('l-lolid',        'l', ((1.5, 'l-lolid1'), (-0.5, 'l-lolid0'))),
    ('r-uplid',        'l', ((1.5, 'r-uplid1'), (-0.5, 'r-uplid0'))),
    ('r-lolid',        'l', ((1.5, 'r-lolid1'), (-0.5, 'r-lolid0'))),

    ('l-uplid-top',       'o', ('l-uplid', dy)),
    ('l-lolid-top',       'o', ('l-lolid', dy)),
    ('r-uplid-top',       'o', ('r-uplid', dy)),
    ('r-lolid-top',       'o', ('r-lolid', dy)),

]

Planes = {
    "PlaneEye.L" :         ('l-eye', 'l-eye-end', 'l-eye-top'),
    "PlaneEye.R" :         ('r-eye', 'r-eye-end', 'r-eye-top'),
}

HeadsTails = {
    'jaw' :                 ('mouth', 'jaw'),
    'tongue_base' :         ('tongue-1', 'tongue-2'),
    'tongue_mid' :          ('tongue-2', 'tongue-3'),
    'tongue_tip' :          ('tongue-3', 'tongue-4'),

    'eye.R' :               ('r-eye', 'r-eye-end'),
    'eye_parent.R' :        ('r-eye', 'r-eye-end'),
    'uplid.R' :             ('r-uplid0', 'r-uplid'),
    'lolid.R' :             ('r-lolid0', 'r-lolid'),

    'eye.L' :               ('l-eye', 'l-eye-end'),
    'eye_parent.L' :        ('l-eye', 'l-eye-end'),
    'uplid.L' :             ('l-uplid0', 'l-uplid'),
    'lolid.L' :             ('l-lolid0', 'l-lolid'),
}

Armature = {
    'jaw' :                 (0, 'head', F_DEF|F_NOLOCK, L_HEAD),
    'tongue_base' :         (0, 'jaw', F_DEF|F_SCALE, L_HEAD),
    'tongue_mid' :          (0, 'tongue_base', F_DEF|F_SCALE, L_HEAD),
    'tongue_tip' :          (0, 'tongue_mid', F_DEF|F_SCALE, L_HEAD),
    'eye.R' :               ('PlaneEye.L', 'head', F_DEF, L_HEAD),
    'eye.L' :               ('PlaneEye.R', 'head', F_DEF, L_HEAD),
    'uplid.R' :             (0, 'head', F_DEF|F_LOCKY, L_HEAD),
    'lolid.R' :             (0, 'head', F_DEF|F_LOCKY, L_HEAD),
    'uplid.L' :             (0, 'head', F_DEF|F_LOCKY, L_HEAD),
    'lolid.L' :             (0, 'head', F_DEF|F_LOCKY, L_HEAD),
}

Constraints = {}

CustomShapes = {
    'jaw' :             'GZM_Jaw',
    'eye.R' :           'GZM_Circle025',
    'eye.L' :           'GZM_Circle025',

    'uplid.L' :         'GZM_UpLid',
    'uplid.R' :         'GZM_UpLid',
    'lolid.L' :         'GZM_LoLid',
    'lolid.R' :         'GZM_LoLid',

    'tongue_base' :     'GZM_Tongue',
    'tongue_mid' :      'GZM_Tongue',
    'tongue_tip' :      'GZM_Tongue',
}

LocationLimits = {
    'jaw' :         (-0.2,0.2, -0.2,0.2, -0.2,0.2),
}

RotationLimits = {
    'jaw' :     (-5,45, 0,0, -20,20),
    'uplid.L':  (-10,45, 0,0, 0,0),
    'uplid.R':  (-10,45, 0,0, 0,0),
    'lolid.L':  (-45,10, 0,0, 0,0),
    'lolid.R':  (-45,10, 0,0, 0,0),
}

Locks = {
    "uplid.L" :  (0,1,1),
    "uplid.R" :  (0,1,1),
    "lolid.L" :  (0,1,1),
    "lolid.R" :  (0,1,1),
}

#
#    DeformDrivers(fp, amt):
#

def DeformDrivers(fp, amt):
    return []
    lidBones = [
    ('DEF_uplid.L', 'PUpLid_L', (0, 40*D)),
    ('DEF_lolid.L', 'PLoLid_L', (0, 20*D)),
    ('DEF_uplid.R', 'PUpLid_R', (0, 40*D)),
    ('DEF_lolid.R', 'PLoLid_R', (0, 20*D)),
    ]

    drivers = []
    for (driven, driver, coeff) in lidBones:
        drivers.append(    (driven, 'ROTQ', 'AVERAGE', None, 1, coeff,
         [("var", 'TRANSFORMS', [('OBJECT', amt.name, driver, 'LOC_Z', C_LOC)])]) )
    return drivers



```

## Source file: makehuman-1.1.1-win32/plugins/3_libraries_teeth.py

{
  "path": "makehuman-1.1.1-win32/plugins/3_libraries_teeth.py",
  "blob_sha": "dfdf6db1ec499ee18dc977b1e7b8998a07cfeb31",
  "sha256": "b2b262013fd930c6e83f85616f8def29b2e30ebc409dbd1d740488d4763755c3",
  "lines": 86,
  "bytes": 2342,
  "selection_score": 15,
  "keyword_line_counts": {
    "sss": 1,
    "teeth": 7,
    "rig": 1,
    "face": 1
  },
  "url": "https://github.com/intoro/MakeHumanForBlender/blob/8b0c95acad9f53629513bdebea39ac84ee8780a7/makehuman-1.1.1-win32/plugins/3_libraries_teeth.py",
  "function_signature_candidates": [
    {
      "line": 45,
      "text": "def __init__(self, category):"
    },
    {
      "line": 48,
      "text": "def getObjectLayer(self):"
    },
    {
      "line": 51,
      "text": "def proxySelected(self, proxy):"
    },
    {
      "line": 54,
      "text": "def proxyDeselected(self, proxy, suppressSignal = False):"
    },
    {
      "line": 57,
      "text": "def onShow(self, event):"
    },
    {
      "line": 69,
      "text": "def load(app):"
    },
    {
      "line": 84,
      "text": "def unload(app):"
    }
  ]
}

```text

#!/usr/bin/python2.7
# -*- coding: utf-8 -*-

"""
**Project Name:**      MakeHuman

**Product Home Page:** http://www.makehuman.org/

**Code Home Page:**    https://bitbucket.org/MakeHuman/makehuman/

**Authors:**           Jonas Hauquier

**Copyright(c):**      MakeHuman Team 2001-2017

**Licensing:**         AGPL3

    This file is part of MakeHuman (www.makehuman.org).

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as
    published by the Free Software Foundation, either version 3 of the
    License, or (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.


Abstract
--------

Teeth proxy library.
"""

import gui3d
import mh
import proxychooser

class TeethTaskView(proxychooser.ProxyChooserTaskView):

    def __init__(self, category):
        super(TeethTaskView, self).__init__(category, 'teeth', tagFilter = True, descriptionWidget = False)

    def getObjectLayer(self):
        return 5

    def proxySelected(self, proxy):
        self.human.teethProxy = proxy

    def proxyDeselected(self, proxy, suppressSignal = False):
        self.human.teethProxy = None

    def onShow(self, event):
        super(TeethTaskView, self).onShow(event)
        if gui3d.app.getSetting('cameraAutoZoom'):
            gui3d.app.setFaceCamera()


# This method is called when the plugin is loaded into makehuman
# The app reference is passed so that a plugin can attach a new category, task, or other GUI elements


taskview = None

def load(app):
    global taskview

    category = app.getCategory('Geometries')
    taskview = TeethTaskView(category)
    taskview.sortOrder = 1.3
    category.addTask(taskview)

    taskview.registerLoadSaveHandlers()


# This method is called when the plugin is unloaded from makehuman
# At the moment this is not used, but in the future it will remove the added GUI elements


def unload(app):
    taskview.onUnload()



```

## Source file: makehuman-1.1.1-win32/plugins/3_libraries_tongue.py

{
  "path": "makehuman-1.1.1-win32/plugins/3_libraries_tongue.py",
  "blob_sha": "45e1f75091e7949cd6f0fd302ff1833eb56bb0ff",
  "sha256": "e820601ea941a7fe312b93c16e685e8733778bd8f44bae649b42b580325d52be",
  "lines": 86,
  "bytes": 2348,
  "selection_score": 15,
  "keyword_line_counts": {
    "sss": 1,
    "tongue": 7,
    "rig": 1,
    "face": 1
  },
  "url": "https://github.com/intoro/MakeHumanForBlender/blob/8b0c95acad9f53629513bdebea39ac84ee8780a7/makehuman-1.1.1-win32/plugins/3_libraries_tongue.py",
  "function_signature_candidates": [
    {
      "line": 45,
      "text": "def __init__(self, category):"
    },
    {
      "line": 48,
      "text": "def getObjectLayer(self):"
    },
    {
      "line": 51,
      "text": "def proxySelected(self, proxy):"
    },
    {
      "line": 54,
      "text": "def proxyDeselected(self, proxy, suppressSignal = False):"
    },
    {
      "line": 57,
      "text": "def onShow(self, event):"
    },
    {
      "line": 69,
      "text": "def load(app):"
    },
    {
      "line": 84,
      "text": "def unload(app):"
    }
  ]
}

```text

#!/usr/bin/python2.7
# -*- coding: utf-8 -*-

"""
**Project Name:**      MakeHuman

**Product Home Page:** http://www.makehuman.org/

**Code Home Page:**    https://bitbucket.org/MakeHuman/makehuman/

**Authors:**           Jonas Hauquier

**Copyright(c):**      MakeHuman Team 2001-2017

**Licensing:**         AGPL3

    This file is part of MakeHuman (www.makehuman.org).

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as
    published by the Free Software Foundation, either version 3 of the
    License, or (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.


Abstract
--------

Tongue proxy library.
"""

import gui3d
import mh
import proxychooser

class TongueTaskView(proxychooser.ProxyChooserTaskView):

    def __init__(self, category):
        super(TongueTaskView, self).__init__(category, 'tongue', tagFilter = True, descriptionWidget = False)

    def getObjectLayer(self):
        return 5

    def proxySelected(self, proxy):
        self.human.tongueProxy = proxy

    def proxyDeselected(self, proxy, suppressSignal = False):
        self.human.tongueProxy = None

    def onShow(self, event):
        super(TongueTaskView, self).onShow(event)
        if gui3d.app.getSetting('cameraAutoZoom'):
            gui3d.app.setFaceCamera()


# This method is called when the plugin is loaded into makehuman
# The app reference is passed so that a plugin can attach a new category, task, or other GUI elements


taskview = None

def load(app):
    global taskview

    category = app.getCategory('Geometries')
    taskview = TongueTaskView(category)
    taskview.sortOrder = 4
    category.addTask(taskview)

    taskview.registerLoadSaveHandlers()


# This method is called when the plugin is unloaded from makehuman
# At the moment this is not used, but in the future it will remove the added GUI elements


def unload(app):
    taskview.onUnload()



```

## Source file: makehuman-1.1.1-win32/plugins/3_libraries_eyebrows.py

{
  "path": "makehuman-1.1.1-win32/plugins/3_libraries_eyebrows.py",
  "blob_sha": "34c5b06f94bdfb293eb28893fcff7c434a353be3",
  "sha256": "dafadf604712af3c4e1de803d106b2650aff4329a51fab1643f5d6177b04f9d7",
  "lines": 86,
  "bytes": 2364,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 7,
    "sss": 1,
    "rig": 1,
    "face": 1
  },
  "url": "https://github.com/intoro/MakeHumanForBlender/blob/8b0c95acad9f53629513bdebea39ac84ee8780a7/makehuman-1.1.1-win32/plugins/3_libraries_eyebrows.py",
  "function_signature_candidates": [
    {
      "line": 45,
      "text": "def __init__(self, category):"
    },
    {
      "line": 48,
      "text": "def getObjectLayer(self):"
    },
    {
      "line": 51,
      "text": "def proxySelected(self, proxy):"
    },
    {
      "line": 54,
      "text": "def proxyDeselected(self, proxy, suppressSignal = False):"
    },
    {
      "line": 57,
      "text": "def onShow(self, event):"
    },
    {
      "line": 69,
      "text": "def load(app):"
    },
    {
      "line": 84,
      "text": "def unload(app):"
    }
  ]
}

```text

#!/usr/bin/python2.7
# -*- coding: utf-8 -*-

"""
**Project Name:**      MakeHuman

**Product Home Page:** http://www.makehuman.org/

**Code Home Page:**    https://bitbucket.org/MakeHuman/makehuman/

**Authors:**           Jonas Hauquier

**Copyright(c):**      MakeHuman Team 2001-2017

**Licensing:**         AGPL3

    This file is part of MakeHuman (www.makehuman.org).

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as
    published by the Free Software Foundation, either version 3 of the
    License, or (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.


Abstract
--------

Eyebrows proxy library.
"""

import gui3d
import mh
import proxychooser

class EyebrowsTaskView(proxychooser.ProxyChooserTaskView):

    def __init__(self, category):
        super(EyebrowsTaskView, self).__init__(category, 'eyebrows', tagFilter = True, descriptionWidget = False)

    def getObjectLayer(self):
        return 5

    def proxySelected(self, proxy):
        self.human.eyebrowsProxy = proxy

    def proxyDeselected(self, proxy, suppressSignal = False):
        self.human.eyebrowsProxy = None

    def onShow(self, event):
        super(EyebrowsTaskView, self).onShow(event)
        if gui3d.app.getSetting('cameraAutoZoom'):
            gui3d.app.setFaceCamera()


# This method is called when the plugin is loaded into makehuman
# The app reference is passed so that a plugin can attach a new category, task, or other GUI elements


taskview = None

def load(app):
    global taskview

    category = app.getCategory('Geometries')
    taskview = EyebrowsTaskView(category)
    taskview.sortOrder = 4
    category.addTask(taskview)

    taskview.registerLoadSaveHandlers()


# This method is called when the plugin is unloaded from makehuman
# At the moment this is not used, but in the future it will remove the added GUI elements


def unload(app):
    taskview.onUnload()



```
