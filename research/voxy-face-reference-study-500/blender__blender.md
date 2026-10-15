# Facial rendering source scan: blender/blender

Repository: https://github.com/blender/blender

Pinned source revision: 64d181010e0dbc49d2faaaa057357aef8d9a809c

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["COPYING", "assets/LICENSE", "build_files/windows/license.cmd", "extern/audaspace/LICENSE", "extern/cuew/LICENSE", "extern/gflags/COPYING.txt", "extern/glog/COPYING", "extern/gmock/LICENSE", "extern/gtest/LICENSE", "extern/mantaflow/LICENSE", "extern/md4c/LICENSE", "extern/quadriflow/3rd/lemon-1.3.1/LICENSE", "extern/quadriflow/LICENSE.txt", "extern/toml11/LICENSE", "extern/ufbx/LICENSE", "extern/volk/LICENSE.md", "extern/vulkan_memory_allocator/LICENSE.txt", "extern/xxhash/LICENSE", "release/datafiles/studiolights/matcap/license.txt", "release/datafiles/studiolights/world/license.txt"]

Topic paths: {"eye": ["release/datafiles/cursors/cursor_eyedropper.svg", "release/datafiles/icons/ops.paint.eyedropper_add.dat", "release/datafiles/icons_svg/eyedropper.svg", "scripts/addons_core/rigify/rigs/face/skin_eye.py", "source/blender/editors/interface/eyedroppers/eyedropper_bone.cc", "source/blender/editors/interface/eyedroppers/eyedropper_color.cc", "source/blender/editors/interface/eyedroppers/eyedropper_colorband.cc", "source/blender/editors/interface/eyedroppers/eyedropper_datablock.cc", "source/blender/editors/interface/eyedroppers/eyedropper_depth.cc", "source/blender/editors/interface/eyedroppers/eyedropper_driver.cc", "source/blender/editors/interface/eyedroppers/eyedropper_grease_pencil_color.cc", "source/blender/editors/interface/eyedroppers/eyedropper_intern.hh", "source/blender/editors/interface/eyedroppers/interface_eyedropper.cc", "tests/files/animation/driver-object-eyes.blend", "tests/files/render/camera/camera_fisheye_equidistant.blend", "tests/files/render/camera/camera_fisheye_equisolid.blend", "tests/files/render/camera/camera_fisheye_polynomial.blend", "tests/files/render/camera/cycles_renders/camera_fisheye_equidistant.png", "tests/files/render/camera/cycles_renders/camera_fisheye_equisolid.png", "tests/files/render/camera/cycles_renders/camera_fisheye_polynomial.png", "tests/files/render/camera/eevee_next_renders/camera_fisheye_equidistant.png", "tests/files/render/camera/eevee_next_renders/camera_fisheye_equisolid.png", "tests/files/render/camera/eevee_next_renders/camera_fisheye_polynomial.png", "tests/files/render/camera/eevee_renders/camera_fisheye_equidistant.png", "tests/files/render/camera/eevee_renders/camera_fisheye_equisolid.png", "tests/files/render/camera/eevee_renders/camera_fisheye_polynomial.png", "tests/files/render/camera/storm_renders/camera_fisheye_equidistant.png", "tests/files/render/camera/storm_renders/camera_fisheye_equisolid.png", "tests/files/render/camera/storm_renders/camera_fisheye_polynomial.png", "tests/files/render/camera/workbench_renders/camera_fisheye_equidistant.png"], "skin": ["release/datafiles/icons_svg/mod_skin.svg", "release/datafiles/icons_svg/onionskin_off.svg", "release/datafiles/icons_svg/onionskin_on.svg", "scripts/addons_core/io_scene_gltf2/blender/exp/skins.py", "scripts/addons_core/rigify/rigs/face/skin_eye.py", "scripts/addons_core/rigify/rigs/face/skin_jaw.py", "scripts/addons_core/rigify/rigs/skin/anchor.py", "scripts/addons_core/rigify/rigs/skin/basic_chain.py", "scripts/addons_core/rigify/rigs/skin/glue.py", "scripts/addons_core/rigify/rigs/skin/skin_nodes.py", "scripts/addons_core/rigify/rigs/skin/skin_parents.py", "scripts/addons_core/rigify/rigs/skin/skin_rigs.py", "scripts/addons_core/rigify/rigs/skin/stretchy_chain.py", "scripts/addons_core/rigify/rigs/skin/transform/basic.py", "source/blender/draw/engines/overlay/shaders/overlay_edit_mesh_skin_root_vert.glsl", "source/blender/draw/intern/mesh_extractors/extract_mesh_vbo_skin_roots.cc", "source/blender/editors/armature/armature_skinning.cc", "source/blender/editors/sculpt_paint/mesh/sculpt_automasking.cc", "source/blender/editors/transform/transform_convert_mesh_skin.cc", "source/blender/editors/transform/transform_mode_skin_resize.cc", "source/blender/modifiers/intern/MOD_skin.cc", "tests/files/io_tests/fbx/bl43_skinned_cylinder.fbx", "tests/files/io_tests/fbx/bl_cube_skinned.fbx", "tests/files/io_tests/fbx/bl_cube_skinned_ignore_leaf_bones.fbx", "tests/files/io_tests/fbx/bl_cube_skinned_ignore_leaf_bones.json", "tests/files/io_tests/fbx/issue43141_skinned_rabbit.fbx_DISABLED", "tests/files/io_tests/fbx/max2025-skinposes-anim.fbx", "tests/files/io_tests/fbx/max2025-skinposes.fbx", "tests/files/io_tests/fbx/reference/bl43_skinned_cylinder.txt", "tests/files/io_tests/fbx/reference/bl_cube_skinned.txt"], "subsurface": ["intern/cycles/kernel/integrator/intersect_subsurface.h", "intern/cycles/kernel/integrator/subsurface.h", "intern/cycles/kernel/integrator/subsurface_disk.h", "intern/cycles/kernel/integrator/subsurface_random_walk.h", "intern/cycles/kernel/osl/shaders/node_subsurface_scattering.osl", "source/blender/draw/engines/eevee/eevee_subsurface.cc", "source/blender/draw/engines/eevee/eevee_subsurface.hh", "source/blender/draw/engines/eevee/eevee_subsurface_shared.hh", "source/blender/draw/engines/eevee/shaders/eevee_subsurface.bsl.hh", "source/blender/draw/engines/eevee/shaders/eevee_subsurface_lib.bsl.hh", "source/blender/gpu/shaders/material/gpu_shader_material_subsurface_scattering.bsl.hh", "source/blender/nodes/shader/nodes/node_shader_subsurface_scattering.cc", "tests/files/render/light_linking/cycles_renders/shadow_link_subsurface.png", "tests/files/render/light_linking/eevee_renders/shadow_link_subsurface.png", "tests/files/render/light_linking/shadow_link_subsurface.blend", "tests/files/render/light_linking/storm_renders/shadow_link_subsurface.png", "tests/files/render/principled_bsdf/cycles_renders/principled_bsdf_subsurface.png", "tests/files/render/principled_bsdf/cycles_renders/principled_bsdf_thin_subsurface.png", "tests/files/render/principled_bsdf/eevee_renders/principled_bsdf_subsurface.png", "tests/files/render/principled_bsdf/eevee_renders/principled_bsdf_thin_subsurface.png", "tests/files/render/principled_bsdf/principled_bsdf_subsurface.blend", "tests/files/render/principled_bsdf/principled_bsdf_thin_subsurface.blend", "tests/files/render/principled_bsdf/storm_hydra_renders/principled_bsdf_thin_subsurface.png", "tests/files/render/principled_bsdf/storm_renders/principled_bsdf_subsurface.png", "tests/files/render/principled_bsdf/storm_renders/principled_bsdf_thin_subsurface.png", "tests/files/render/sss/cycles_renders/subsurface_base_color_mix.png", "tests/files/render/sss/cycles_renders/subsurface_behind_glass.png", "tests/files/render/sss/cycles_renders/subsurface_burley.png", "tests/files/render/sss/cycles_renders/subsurface_cubic.png", "tests/files/render/sss/cycles_renders/subsurface_gaussian.png"], "sss": ["extern/bullet2/src/BulletDynamics/MLCPSolvers/btSolveProjectedGaussSeidel.h", "extern/bullet2/src/BulletSoftBody/btDeformableMassSpringForce.h", "source/blender/freestyle/intern/python/StrokeShader/BPy_ConstantThicknessShader.cpp", "source/blender/freestyle/intern/python/StrokeShader/BPy_ConstantThicknessShader.h", "source/blender/freestyle/intern/python/StrokeShader/BPy_ConstrainedIncreasingThicknessShader.cpp", "source/blender/freestyle/intern/python/StrokeShader/BPy_ConstrainedIncreasingThicknessShader.h", "source/blender/freestyle/intern/python/StrokeShader/BPy_IncreasingThicknessShader.cpp", "source/blender/freestyle/intern/python/StrokeShader/BPy_IncreasingThicknessShader.h", "tests/files/render/denoise/cycles_renders/denoise_sss.png", "tests/files/render/denoise/denoise_sss.blend", "tests/files/render/guiding/cycles_renders/guiding_sss.png", "tests/files/render/guiding/guiding_sss.blend", "tests/files/render/reports/cycles_renders/sss_and_volume_39823.png", "tests/files/render/reports/cycles_renders/sss_glowing_149904.png", "tests/files/render/reports/cycles_renders/sss_spatial_splits_48860.png", "tests/files/render/reports/cycles_renders/sss_spatial_splits_48860_camera_only.png", "tests/files/render/reports/cycles_renders/sss_spatial_splits_mblur_48860.png", "tests/files/render/reports/sss_and_volume_39823.blend", "tests/files/render/reports/sss_glowing_149904.blend", "tests/files/render/reports/sss_spatial_splits_48860.blend", "tests/files/render/reports/sss_spatial_splits_48860_camera_only.blend", "tests/files/render/reports/sss_spatial_splits_mblur_48860.blend", "tests/files/render/sss/cycles_renders/sss_concave_clamp.png", "tests/files/render/sss/cycles_renders/sss_diffuse_mix_clamp.png", "tests/files/render/sss/cycles_renders/sss_hair.png", "tests/files/render/sss/cycles_renders/sss_reflection_clamp.png", "tests/files/render/sss/cycles_renders/subsurface_base_color_mix.png", "tests/files/render/sss/cycles_renders/subsurface_behind_glass.png", "tests/files/render/sss/cycles_renders/subsurface_burley.png", "tests/files/render/sss/cycles_renders/subsurface_cubic.png"], "blendshape": ["tests/files/io_tests/fbx/reference/unity_basic_blendshapes_test.txt", "tests/files/io_tests/fbx/unity_basic_blendshapes_test.fbx", "tests/files/usd/compare/reference/usd_blendshapes_duplicate.txt", "tests/files/usd/compare/reference/usd_blendshapes_test.txt", "tests/files/usd/compare/usd_blendshapes_duplicate.usda", "tests/files/usd/compare/usd_blendshapes_test.usda"], "blend_shape": ["source/blender/io/usd/intern/usd_blend_shape_utils.cc", "source/blender/io/usd/intern/usd_blend_shape_utils.hh", "tests/files/io_tests/fbx/reference/ufbx_maya_blend_shape_cube_77b.txt", "tests/files/io_tests/fbx/ufbx_maya_blend_shape_cube_77b.fbx"], "jaw": ["scripts/addons_core/rigify/rigs/face/skin_jaw.py"], "tooth": ["extern/audaspace/include/generator/Sawtooth.h", "extern/audaspace/include/generator/SawtoothReader.h", "extern/audaspace/src/generator/Sawtooth.cpp", "extern/audaspace/src/generator/SawtoothReader.cpp"], "gum": ["build_files/windows/parse_arguments.cmd"], "tongue": ["scripts/addons_core/rigify/rigs/face/basic_tongue.py"], "flame": ["tests/files/render/openvdb/blendcache_fire/data/flame_0001.uni", "tests/files/render/openvdb/blendcache_fire/data/flame_0002.uni", "tests/files/render/openvdb/blendcache_fire/data/flame_0003.uni", "tests/files/render/openvdb/blendcache_principled_blackbody/data/flame_0001.uni", "tests/files/render/openvdb/blendcache_principled_blackbody/data/flame_0002.uni", "tests/files/render/openvdb/blendcache_principled_blackbody/data/flame_0003.uni", "tests/files/render/openvdb/blendcache_smoke_fire/data/flame_0001.uni", "tests/files/render/openvdb/blendcache_smoke_fire/data/flame_0002.uni", "tests/files/render/openvdb/blendcache_smoke_fire/data/flame_0003.uni"], "deform": ["extern/bullet2/src/BulletSoftBody/DeformableBodyInplaceSolverIslandCallback.h", "extern/bullet2/src/BulletSoftBody/btDeformableBackwardEulerObjective.cpp", "extern/bullet2/src/BulletSoftBody/btDeformableBackwardEulerObjective.h", "extern/bullet2/src/BulletSoftBody/btDeformableBodySolver.cpp", "extern/bullet2/src/BulletSoftBody/btDeformableBodySolver.h", "extern/bullet2/src/BulletSoftBody/btDeformableContactConstraint.cpp", "extern/bullet2/src/BulletSoftBody/btDeformableContactConstraint.h", "extern/bullet2/src/BulletSoftBody/btDeformableContactProjection.cpp", "extern/bullet2/src/BulletSoftBody/btDeformableContactProjection.h", "extern/bullet2/src/BulletSoftBody/btDeformableCorotatedForce.h", "extern/bullet2/src/BulletSoftBody/btDeformableGravityForce.h", "extern/bullet2/src/BulletSoftBody/btDeformableLagrangianForce.h", "extern/bullet2/src/BulletSoftBody/btDeformableLinearElasticityForce.h", "extern/bullet2/src/BulletSoftBody/btDeformableMassSpringForce.h", "extern/bullet2/src/BulletSoftBody/btDeformableMousePickingForce.h", "extern/bullet2/src/BulletSoftBody/btDeformableMultiBodyConstraintSolver.cpp", "extern/bullet2/src/BulletSoftBody/btDeformableMultiBodyConstraintSolver.h", "extern/bullet2/src/BulletSoftBody/btDeformableMultiBodyDynamicsWorld.cpp", "extern/bullet2/src/BulletSoftBody/btDeformableMultiBodyDynamicsWorld.h", "extern/bullet2/src/BulletSoftBody/btDeformableNeoHookeanForce.h", "release/datafiles/icons_svg/mod_meshdeform.svg", "release/datafiles/icons_svg/mod_simpledeform.svg", "source/blender/blenkernel/BKE_deform.hh", "source/blender/blenkernel/BKE_object_deform.h", "source/blender/blenkernel/BKE_subdiv_deform.hh", "source/blender/blenkernel/intern/armature_deform.cc", "source/blender/blenkernel/intern/armature_deform_test.cc", "source/blender/blenkernel/intern/curve_deform.cc", "source/blender/blenkernel/intern/deform.cc", "source/blender/blenkernel/intern/deform_test.cc"], "iris": ["source/blender/imbuf/intern/format_iris.cc", "tests/files/imbuf_io/reference/iris-bw__from__rgba08.rgb", "tests/files/imbuf_io/reference/iris-bw__from__rgba32.rgb", "tests/files/imbuf_io/reference/iris-rgb__from__rgba08.rgb", "tests/files/imbuf_io/reference/iris-rgb__from__rgba32.rgb", "tests/files/imbuf_io/reference/iris-rgba__from__rgba08.rgb", "tests/files/imbuf_io/reference/iris-rgba__from__rgba32.rgb", "tests/files/imbuf_io/reference_load/iris-bw__from__rgba08.exr", "tests/files/imbuf_io/reference_load/iris-bw__from__rgba08.txt", "tests/files/imbuf_io/reference_load/iris-bw__from__rgba32.exr", "tests/files/imbuf_io/reference_load/iris-bw__from__rgba32.txt", "tests/files/imbuf_io/reference_load/iris-rgb__from__rgba08.exr", "tests/files/imbuf_io/reference_load/iris-rgb__from__rgba08.txt", "tests/files/imbuf_io/reference_load/iris-rgb__from__rgba32.exr", "tests/files/imbuf_io/reference_load/iris-rgb__from__rgba32.txt", "tests/files/imbuf_io/reference_load/iris-rgba__from__rgba08.exr", "tests/files/imbuf_io/reference_load/iris-rgba__from__rgba08.txt", "tests/files/imbuf_io/reference_load/iris-rgba__from__rgba32.exr", "tests/files/imbuf_io/reference_load/iris-rgba__from__rgba32.txt"], "shader": ["build_files/build_environment/cmake/shaderc.cmake", "build_files/build_environment/cmake/shaderc_deps.cmake", "build_files/build_environment/windows/nuke_shaderc.cmd", "build_files/cmake/Modules/FindShaderC.cmake", "doc/python_api/examples/bpy.types.InlineShaderNodes.0.py", "intern/cycles/app/opengl/shader.cpp", "intern/cycles/app/opengl/shader.h", "intern/cycles/blender/shader.cpp", "intern/cycles/integrator/shader_eval.cpp", "intern/cycles/integrator/shader_eval.h", "intern/cycles/kernel/device/optix/kernel_osl_shader_raytrace.cu", "intern/cycles/kernel/device/optix/kernel_shader_raytrace.cu", "intern/cycles/kernel/geom/motion_triangle_shader.h", "intern/cycles/kernel/geom/shader_data.h", "intern/cycles/kernel/integrator/displacement_shader.h", "intern/cycles/kernel/integrator/surface_shader.h", "intern/cycles/kernel/integrator/volume_shader.h", "intern/cycles/kernel/osl/shaders/CMakeLists.txt", "intern/cycles/kernel/osl/shaders/int_vector_types.h", "intern/cycles/kernel/osl/shaders/node_absorption_volume.osl", "intern/cycles/kernel/osl/shaders/node_add_closure.osl", "intern/cycles/kernel/osl/shaders/node_ambient_occlusion.osl", "intern/cycles/kernel/osl/shaders/node_attribute.osl", "intern/cycles/kernel/osl/shaders/node_background.osl", "intern/cycles/kernel/osl/shaders/node_bevel.osl", "intern/cycles/kernel/osl/shaders/node_blackbody.osl", "intern/cycles/kernel/osl/shaders/node_boolean_math.osl", "intern/cycles/kernel/osl/shaders/node_brick_texture.osl", "intern/cycles/kernel/osl/shaders/node_brightness.osl", "intern/cycles/kernel/osl/shaders/node_bump.osl"], "dlss": ["build_files/cmake/Modules/FindDLSS.cmake", "intern/cycles/integrator/denoiser_dlss.cpp", "intern/cycles/integrator/denoiser_dlss.h", "intern/itasc/WDLSSolver.cpp", "intern/itasc/WDLSSolver.hpp", "intern/itasc/WSDLSSolver.cpp", "intern/itasc/WSDLSSolver.hpp"], "rig": ["extern/bullet2/src/BulletDynamics/Dynamics/btRigidBody.cpp", "extern/bullet2/src/BulletDynamics/Dynamics/btRigidBody.h", "extern/bullet2/src/BulletSoftBody/btSoftBodyRigidBodyCollisionConfiguration.cpp", "extern/bullet2/src/BulletSoftBody/btSoftBodyRigidBodyCollisionConfiguration.h", "extern/bullet2/src/BulletSoftBody/btSoftRigidCollisionAlgorithm.cpp", "extern/bullet2/src/BulletSoftBody/btSoftRigidCollisionAlgorithm.h", "extern/bullet2/src/BulletSoftBody/btSoftRigidDynamicsWorld.cpp", "extern/bullet2/src/BulletSoftBody/btSoftRigidDynamicsWorld.h", "intern/cycles/kernel/osl/shaders/node_brightness.osl", "intern/cycles/kernel/svm/brightness.h", "intern/rigidbody/CMakeLists.txt", "intern/rigidbody/RBI_api.h", "intern/rigidbody/RBI_hull_api.h", "intern/rigidbody/rb_bullet_api.cpp", "intern/rigidbody/rb_convex_hull_api.cpp", "release/datafiles/cursors/cursor_right_handle.svg", "release/datafiles/icons_svg/align_right.svg", "release/datafiles/icons_svg/anchor_right.svg", "release/datafiles/icons_svg/arrow_leftright.svg", "release/datafiles/icons_svg/disclosure_tri_right.svg", "release/datafiles/icons_svg/grip_corner_bottom_right.svg", "release/datafiles/icons_svg/mod_brightness_contrast.svg", "release/datafiles/icons_svg/object_origin.svg", "release/datafiles/icons_svg/rightarrow.svg", "release/datafiles/icons_svg/rightarrow_thin.svg", "release/datafiles/icons_svg/rigid_body.svg", "release/datafiles/icons_svg/rigid_body_constraint.svg", "release/datafiles/icons_svg/transform_origins.svg", "release/datafiles/icons_svg/tria_right.svg", "release/datafiles/icons_svg/tria_right_bar.svg"], "face": ["extern/bullet2/src/BulletCollision/BroadphaseCollision/btBroadphaseInterface.h", "extern/bullet2/src/BulletCollision/CollisionShapes/btStridingMeshInterface.cpp", "extern/bullet2/src/BulletCollision/CollisionShapes/btStridingMeshInterface.h", "extern/bullet2/src/BulletCollision/NarrowPhaseCollision/btDiscreteCollisionDetectorInterface.h", "extern/bullet2/src/BulletCollision/NarrowPhaseCollision/btSimplexSolverInterface.h", "extern/bullet2/src/BulletDynamics/Character/btCharacterControllerInterface.h", "extern/bullet2/src/BulletDynamics/Dynamics/btActionInterface.h", "extern/bullet2/src/BulletDynamics/MLCPSolvers/btMLCPSolverInterface.h", "extern/bullet2/src/LinearMath/TaskScheduler/btThreadSupportInterface.h", "extern/mantaflow/preprocessed/plugin/surfaceturbulence.cpp", "intern/cycles/kernel/closure/bsdf_microfacet.h", "intern/cycles/kernel/integrator/intersect_subsurface.h", "intern/cycles/kernel/integrator/shade_surface.h", "intern/cycles/kernel/integrator/subsurface.h", "intern/cycles/kernel/integrator/subsurface_disk.h", "intern/cycles/kernel/integrator/subsurface_random_walk.h", "intern/cycles/kernel/integrator/surface_shader.h", "intern/cycles/kernel/osl/shaders/node_output_surface.osl", "intern/cycles/kernel/osl/shaders/node_subsurface_scattering.osl", "release/datafiles/icons/brush.sculpt.draw_face_sets.dat", "release/datafiles/icons/ops.mesh.extrude_faces_move.dat", "release/datafiles/icons/ops.sculpt.border_face_set.dat", "release/datafiles/icons/ops.sculpt.face_set_edit.dat", "release/datafiles/icons/ops.sculpt.lasso_face_set.dat", "release/datafiles/icons/ops.sculpt.line_face_set.dat", "release/datafiles/icons/ops.sculpt.polyline_face_set.dat", "release/datafiles/icons_svg/face_corner.svg", "release/datafiles/icons_svg/face_maps.svg", "release/datafiles/icons_svg/facesel.svg", "release/datafiles/icons_svg/mod_scatter_on_surface.svg"], "lip": ["extern/bullet2/src/BulletCollision/Gimpact/btClipPolygon.h", "extern/bullet2/src/BulletCollision/Gimpact/gim_clip_polygon.h", "extern/bullet2/src/BulletCollision/NarrowPhaseCollision/btPolyhedralContactClipping.cpp", "extern/bullet2/src/BulletCollision/NarrowPhaseCollision/btPolyhedralContactClipping.h", "extern/mantaflow/preprocessed/plugin/flip.cpp", "extern/quadriflow/src/parametrizer-flip.cpp", "intern/slim/intern/flip_avoiding_line_search.cpp", "intern/slim/intern/flip_avoiding_line_search.h", "release/datafiles/cursors/cursor_slip.svg", "release/datafiles/icons/ops.sequencer.slip.dat", "release/datafiles/icons_svg/clipuv_dehlt.svg", "release/datafiles/icons_svg/clipuv_hlt.svg", "release/datafiles/icons_svg/meta_ellipsoid.svg", "release/datafiles/icons_svg/pasteflipdown.svg", "release/datafiles/icons_svg/pasteflipup.svg", "scripts/freestyle/styles/blueprint_ellipses.py", "scripts/startup/bl_operators/clip.py", "scripts/startup/bl_ui/space_clip.py", "source/blender/blenkernel/BKE_movieclip.hh", "source/blender/blenkernel/intern/mesh_flip_faces.cc", "source/blender/blenkernel/intern/movieclip.cc", "source/blender/compositor/shaders/compositor_ellipse_mask.glsl", "source/blender/compositor/shaders/compositor_flip.glsl", "source/blender/compositor/shaders/infos/compositor_ellipse_mask_infos.hh", "source/blender/compositor/shaders/infos/compositor_flip_infos.hh", "source/blender/depsgraph/intern/eval/deg_eval_runtime_backup_movieclip.cc", "source/blender/depsgraph/intern/eval/deg_eval_runtime_backup_movieclip.h", "source/blender/draw/engines/overlay/shaders/overlay_clipbound_vert.glsl", "source/blender/draw/intern/shaders/draw_view_clipping_lib.glsl", "source/blender/editors/include/ED_clip.hh"]}

## Source file: source/blender/nodes/shader/nodes/node_shader_subsurface_scattering.cc

{
  "path": "source/blender/nodes/shader/nodes/node_shader_subsurface_scattering.cc",
  "blob_sha": "3d466123b589a7bf653cee3785372b1cc0c59ee2",
  "sha256": "7855f48b907b5698e561fd67f39c685a879e09f672e010ab7933831a758cc5ad",
  "lines": 178,
  "bytes": 6412,
  "selection_score": 45,
  "keyword_line_counts": {
    "skin": 2,
    "subsurface": 29,
    "sss": 4,
    "shader": 22,
    "rig": 1,
    "face": 30
  },
  "url": "https://github.com/blender/blender/blob/64d181010e0dbc49d2faaaa057357aef8d9a809c/source/blender/nodes/shader/nodes/node_shader_subsurface_scattering.cc",
  "function_signature_candidates": [
    {
      "line": 22,
      "text": "static void node_declare(NodeDeclarationBuilder &b)"
    },
    {
      "line": 63,
      "text": "static void node_shader_buts_subsurface(ui::Layout &layout, bContext * /*C*/, PointerRNA *ptr)"
    },
    {
      "line": 68,
      "text": "static void node_shader_init_subsurface_scattering(bNodeTree * /*ntree*/, bNode *node)"
    },
    {
      "line": 74,
      "text": "static int node_shader_gpu_subsurface_scattering(GPUMaterial *mat,"
    },
    {
      "line": 104,
      "text": "static void node_shader_update_subsurface_scattering(bNodeTree *ntree, bNode *node)"
    },
    {
      "line": 152,
      "text": "void register_node_type_sh_subsurface_scattering()"
    }
  ]
}

```text

/* SPDX-FileCopyrightText: 2005 Blender Authors
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

/** \file
 * \ingroup shdnodes
 */

#include "node_shader_util.hh"

#include "BLI_string.hh"

#include "UI_interface_layout.hh"
#include "UI_resources.hh"

#include "BKE_node_runtime.hh"

namespace blender {

namespace nodes::node_shader_subsurface_scattering_cc {

static void node_declare(NodeDeclarationBuilder &b)
{
  const bNodeTree *ntree = b.tree_or_null();
  const bool is_gpu_internal = ntree && (ntree->flag & NTREE_IS_GPU_SHADER_INTERNAL);

  b.add_input<decl::Color>("Color"_ustr).default_value({0.8f, 0.8f, 0.8f, 1.0f});
  b.add_input<decl::Float>("Scale"_ustr)
      .default_value(0.005f)
      .min(0.0f)
      .max(1000.0f)
      .description("Scale factor of the subsurface scattering radius");
  b.add_input<decl::Vector>("Radius"_ustr)
      .default_value({1.0f, 0.2f, 0.1f})
      .min(0.0f)
      .max(100.0f)
      .description("Scattering radius per color channel (RGB), multiplied with Scale");
  b.add_input<decl::Float>("IOR"_ustr)
      .default_value(1.4f)
      .min(1.01f)
      .max(3.8f)
      .subtype(PROP_FACTOR);
  b.add_input<decl::Float>("Roughness"_ustr)
      .default_value(1.0f)
      .min(0.0f)
      .max(1.0f)
      .subtype(PROP_FACTOR);
  b.add_input<decl::Float>("Anisotropy"_ustr)
      .default_value(0.0f)
      .min(-1.0f)
      .max(1.0f)
      .subtype(PROP_FACTOR)
      .description(
          "Directionality of volume scattering within the subsurface medium. "
          "Zero scatters uniformly in all directions, positive values scatter more in the forward "
          "direction, and negative values scatter more backwards. "
          "For example, skin has been measured to have an anisotropy of 0.8");
  b.add_input<decl::Vector>("Normal"_ustr).hide_value();
  b.add_input<decl::Float>("Weight"_ustr).available(is_gpu_internal);
  b.add_output<decl::Shader>("BSSRDF"_ustr);
}

static void node_shader_buts_subsurface(ui::Layout &layout, bContext * /*C*/, PointerRNA *ptr)
{
  layout.prop(ptr, "falloff", ui::ITEM_R_SPLIT_EMPTY_NAME, "", ICON_NONE);
}

static void node_shader_init_subsurface_scattering(bNodeTree * /*ntree*/, bNode *node)
{
  node->custom1 = SHD_SUBSURFACE_RANDOM_WALK;
  node->custom2 = true;
}

static int node_shader_gpu_subsurface_scattering(GPUMaterial *mat,
                                                 bNode *node,
                                                 bNodeExecData * /*execdata*/,
                                                 GPUNodeStack *in,
                                                 GPUNodeStack *out)
{
  if (!in[6].link) {
    GPU_link(mat, "world_normals_get", GPU_shading_data(), &in[6].link);
  }

  GPU_material_flag_set(mat, GPU_MATFLAG_DIFFUSE | GPU_MATFLAG_SUBSURFACE);

  /* EEVEE's subsurface closure still uses the legacy approximation where the radius is scaled by
   * 1/(4pi) (e.g., SHD_SUBSURFACE_RANDOM_WALK_LEGACY) */
  GPUNodeLink *random_walk_radius_scale = nullptr;
  float random_walk_scale = 1.0f;
  if (node->custom1 == SHD_SUBSURFACE_RANDOM_WALK) {
    random_walk_scale = 4.0f * M_PI;
  }
  random_walk_radius_scale = GPU_constant(&random_walk_scale);

  return GPU_stack_link(mat,
                        node,
                        "node_subsurface_scattering",
                        in,
                        out,
                        random_walk_radius_scale,
                        GPU_shading_data());
}

static void node_shader_update_subsurface_scattering(bNodeTree *ntree, bNode *node)
{
  const int sss_method = node->custom1;

  for (bNodeSocket &sock : node->inputs) {
    if (STR_ELEM(sock.name, "IOR", "Anisotropy")) {
      bke::node_set_socket_availability(*ntree, sock, sss_method != SHD_SUBSURFACE_BURLEY);
    }
    if (STR_ELEM(sock.name, "Roughness")) {
      const bool is_random_walk = sss_method == SHD_SUBSURFACE_RANDOM_WALK ||
                                  sss_method == SHD_SUBSURFACE_RANDOM_WALK_LEGACY;
      bke::node_set_socket_availability(*ntree, sock, is_random_walk);
    }
  }
}

NODE_SHADER_MATERIALX_BEGIN
#ifdef WITH_MATERIALX
{
  /* NOTE: IOR and Subsurface Method isn't supported for this node in MaterialX. */
  if (to_type_ != NodeItem::Type::BSDF) {
    return empty();
  }

  NodeItem color = get_input_value("Color", NodeItem::Type::Color3);
  NodeItem scale = get_input_value("Scale", NodeItem::Type::Float);
#  if MATERIALX_MAJOR_VERSION <= 1 && MATERIALX_MINOR_VERSION <= 38
  NodeItem radius = get_input_value("Radius", NodeItem::Type::Vector3);
#  else
  NodeItem radius = get_input_value("Radius", NodeItem::Type::Color3);
#  endif
  NodeItem anisotropy = get_input_value("Anisotropy", NodeItem::Type::Float);
  NodeItem normal = get_input_link("Normal", NodeItem::Type::Vector3);

  return create_node("subsurface_bsdf",
                     NodeItem::Type::BSDF,
                     {{"weight", val(1.0f)},
                      {"color", color},
                      {"radius", radius * scale},
                      {"anisotropy", anisotropy},
                      {"normal", normal}});
}
#endif
NODE_SHADER_MATERIALX_END

}  // namespace nodes::node_shader_subsurface_scattering_cc

/* node type definition */
void register_node_type_sh_subsurface_scattering()
{
  namespace file_ns = nodes::node_shader_subsurface_scattering_cc;

  static bke::bNodeType ntype;

  sh_node_type_base(&ntype, "ShaderNodeSubsurfaceScattering"_ustr, SH_NODE_SUBSURFACE_SCATTERING);
  ntype.ui_name = "Subsurface Scattering";
  ntype.ui_description =
      "Subsurface multiple scattering shader to simulate light entering the surface and bouncing "
      "internally.\nTypically used for materials such as skin, wax, marble or milk";
  ntype.enum_name_legacy = "SUBSURFACE_SCATTERING";
  ntype.nclass = NODE_CLASS_SHADER;
  ntype.declare = file_ns::node_declare;
  ntype.gather_link_search_ops = search_link_ops_for_shader_bsdf_node;
  ntype.add_ui_poll = object_shader_nodes_poll;
  ntype.draw_buttons = file_ns::node_shader_buts_subsurface;
  ntype.default_width = bke::NodeWidth::_160;
  ntype.initfunc = file_ns::node_shader_init_subsurface_scattering;
  ntype.gpu_fn = file_ns::node_shader_gpu_subsurface_scattering;
  ntype.updatefunc = file_ns::node_shader_update_subsurface_scattering;
  ntype.materialx_fn = file_ns::node_shader_materialx;

  bke::node_register_type(ntype);
}

}  // namespace blender


```

## Source file: source/blender/compositor/shaders/library/gpu_shader_compositor_bright_contrast.glsl

{
  "path": "source/blender/compositor/shaders/library/gpu_shader_compositor_bright_contrast.glsl",
  "blob_sha": "89171df00f63ead7425b539f284bccdf72a81fdd",
  "sha256": "837fc810d262eab3265638a9be8e4c3bdab5e9aa307b68fb19e453b23489436e",
  "lines": 31,
  "bytes": 922,
  "selection_score": 42,
  "keyword_line_counts": {
    "shader": 1,
    "rig": 5
  },
  "url": "https://github.com/blender/blender/blob/64d181010e0dbc49d2faaaa057357aef8d9a809c/source/blender/compositor/shaders/library/gpu_shader_compositor_bright_contrast.glsl",
  "function_signature_candidates": [
    {
      "line": 13,
      "text": "void node_composite_bright_contrast(float4 color, float brightness, float contrast, float4 &result)"
    }
  ]
}

```text

/* SPDX-FileCopyrightText: 2022 Blender Authors
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

/* The algorithm is by Werner D. Streidt, extracted of OpenCV demhist.c:
 *   http://visca.com/ffactory/archives/5-99/msg00021.html */

#include "gpu_shader_common_color_utils.bsl.hh"

#define FLT_EPSILON 1.192092896e-07F

[[node]]
void node_composite_bright_contrast(float4 color, float brightness, float contrast, float4 &result)
{
  float scaled_brightness = brightness / 100.0f;
  float delta = contrast / 200.0f;

  float multiplier, offset;
  if (contrast > 0.0f) {
    multiplier = 1.0f - delta * 2.0f;
    multiplier = 1.0f / max(multiplier, FLT_EPSILON);
    offset = multiplier * (scaled_brightness - delta);
  }
  else {
    delta *= -1.0f;
    multiplier = max(1.0f - delta * 2.0f, 0.0f);
    offset = multiplier * scaled_brightness + delta;
  }

  result = float4(color.rgb * multiplier + offset, color.a);
}


```

## Source file: scripts/addons_core/rigify/rigs/face/skin_eye.py

{
  "path": "scripts/addons_core/rigify/rigs/face/skin_eye.py",
  "blob_sha": "e9a010d05fefdf5d3b2215f183f00497a55a04c2",
  "sha256": "6d070e7d99317a5dcb3a2857dcea21ac09fa5b8e44a083a83bfb2cd8d326f1d9",
  "lines": 843,
  "bytes": 30809,
  "selection_score": 36,
  "keyword_line_counts": {
    "eye": 104,
    "skin": 16,
    "deform": 26,
    "iris": 6,
    "rig": 92,
    "face": 9
  },
  "url": "https://github.com/blender/blender/blob/64d181010e0dbc49d2faaaa057357aef8d9a809c/scripts/addons_core/rigify/rigs/face/skin_eye.py",
  "function_signature_candidates": [
    {
      "line": 40,
      "text": "def find_org_bones(self, bone: PoseBone) -> str:"
    },
    {
      "line": 54,
      "text": "def initialize(self):"
    },
    {
      "line": 70,
      "text": "def create_cluster_control(self):"
    },
    {
      "line": 76,
      "text": "def is_eye_control_node(self, node: ControlBoneNode) -> bool:"
    },
    {
      "line": 80,
      "text": "def is_eye_corner_node(self, node: ControlBoneNode) -> bool:"
    },
    {
      "line": 85,
      "text": "def init_eye_corner_space(self):"
    },
    {
      "line": 109,
      "text": "def get_eye_corner_angle(self, node: ControlBoneNode) -> float:"
    },
    {
      "line": 114,
      "text": "def get_master_control_position(self) -> Vector:"
    },
    {
      "line": 126,
      "text": "def get_lid_follow_influence(self, node: ControlBoneNode):"
    },
    {
      "line": 167,
      "text": "def init_child_chains(self):"
    },
    {
      "line": 174,
      "text": "def patch_chain(self, child: BasicChainRig):"
    },
    {
      "line": 180,
      "text": "def extend_control_node_parent(self, parent, node: BaseSkinNode):"
    },
    {
      "line": 195,
      "text": "def extend_mid_node_parent(self, parent: ControlBoneParentBase, node: ControlBoneNode):"
    },
    {
      "line": 218,
      "text": "def extend_control_node_rig(self, node: ControlBoneNode):"
    },
    {
      "line": 237,
      "text": "def configure_script_panels(self):"
    },
    {
      "line": 246,
      "text": "def add_custom_properties(self):"
    },
    {
      "line": 262,
      "text": "def add_ui_sliders(self, panel: PanelLayout, *, add_name=False):"
    },
    {
      "line": 283,
      "text": "def make_master_control(self):"
    },
    {
      "line": 290,
      "text": "def configure_master_control(self):"
    },
    {
      "line": 294,
      "text": "def make_master_control_widget(self):"
    },
    {
      "line": 302,
      "text": "def make_mch_track_bones(self):"
    },
    {
      "line": 312,
      "text": "def parent_mch_track_bones(self):"
    },
    {
      "line": 319,
      "text": "def rig_mch_track_bones(self):"
    },
    {
      "line": 361,
      "text": "def parent_org_chain(self):"
    },
    {
      "line": 368,
      "text": "def make_deform_bone(self):"
    },
    {
      "line": 379,
      "text": "def parent_deform_chain(self):"
    },
    {
      "line": 388,
      "text": "def rig_deform_chain(self):"
    },
    {
      "line": 400,
      "text": "def add_parameters(cls, params):"
    },
    {
      "line": 427,
      "text": "def parameters_ui(cls, layout, params):"
    },
    {
      "line": 446,
      "text": "def __init__(self, owner: BasicChainRig, eye: Rig):"
    },
    {
      "line": 452,
      "text": "def align_bone(self, name):"
    },
    {
      "line": 456,
      "text": "def prepare_bones(self):"
    },
    {
      "line": 460,
      "text": "def generate_bones(self):"
    },
    {
      "line": 466,
      "text": "def rig_bones(self):"
    },
    {
      "line": 493,
      "text": "def __init__(self, owner: Rig):"
    },
    {
      "line": 498,
      "text": "def find_cluster_rigs(self):"
    },
    {
      "line": 518,
      "text": "def find_cluster_position(self):"
    },
    {
      "line": 544,
      "text": "def project_rig_control(self, rig: Rig):"
    },
    {
      "line": 554,
      "text": "def get_common_rig_name(self):"
    },
    {
      "line": 564,
      "text": "def get_rig_control_matrix(self, rig: Rig):"
    },
    {
      "line": 570,
      "text": "def get_master_control_layers(self):"
    },
    {
      "line": 574,
      "text": "def get_all_rig_control_bones(self):"
    },
    {
      "line": 581,
      "text": "def initialize(self):"
    },
    {
      "line": 585,
      "text": "def generate_bones(self):"
    },
    {
      "line": 600,
      "text": "def make_master_control(self):"
    },
    {
      "line": 608,
      "text": "def make_child_control(self, rig: Rig):"
    },
    {
      "line": 614,
      "text": "def build_parent_switch(self):"
    },
    {
      "line": 627,
      "text": "def parent_bones(self):"
    },
    {
      "line": 634,
      "text": "def configure_bones(self):"
    },
    {
      "line": 647,
      "text": "def generate_widgets(self):"
    }
  ]
}

```text

# SPDX-FileCopyrightText: 2021-2022 Blender Foundation
#
# SPDX-License-Identifier: GPL-2.0-or-later

import bpy
import math
import mathutils

from typing import Optional

from bpy.types import PoseBone
from bpy.app.translations import pgettext_iface as iface_
from mathutils import Vector, Matrix

from ...rig_ui_template import PanelLayout
from ...utils.layers import set_bone_layers, union_layer_lists
from ...utils.naming import make_derived_name, mirror_name, change_name_side, Side, SideZ
from ...utils.bones import align_bone_z_axis, put_bone, TypedBoneDict
from ...utils.widgets import (widget_generator, generate_circle_geometry,
                              generate_circle_hull_geometry)
from ...utils.widgets_basic import create_circle_widget
from ...utils.switch_parent import SwitchParentBuilder
from ...utils.misc import matrix_from_axis_pair, LazyRef

from ...base_rig import stage, RigComponent

from ..skin.skin_nodes import ControlBoneNode, BaseSkinNode
from ..skin.skin_parents import ControlBoneParentOffset, ControlBoneParentBase
from ..skin.skin_rigs import BaseSkinRig

from ..skin.basic_chain import Rig as BasicChainRig


class Rig(BaseSkinRig):
    """
    Eye rig that manages two child eyelid chains. The chains must
    connect at their ends using T/B symmetry.
    """

    def find_org_bones(self, bone: PoseBone) -> str:
        return bone.name

    cluster_control = None

    center: Vector
    axis: Vector

    eye_corner_nodes: list[ControlBoneNode]
    eye_corner_matrix: Optional[Matrix]
    eye_corner_range: tuple[float, float]

    child_chains: list[BasicChainRig]

    def initialize(self):
        super().initialize()

        bone = self.get_bone(self.base_bone)
        self.center = bone.head
        self.axis = bone.vector

        self.eye_corner_nodes = []
        self.eye_corner_matrix = None

        # Create the cluster control (it will assign self.cluster_control)
        if not self.cluster_control:
            self.create_cluster_control()

        self.init_child_chains()

    def create_cluster_control(self):
        return EyeClusterControl(self)

    ####################################################
    # UTILITIES

    def is_eye_control_node(self, node: ControlBoneNode) -> bool:
        return node.rig in self.child_chains and node.is_master_node

    # noinspection PyMethodMayBeStatic
    def is_eye_corner_node(self, node: ControlBoneNode) -> bool:
        # Corners are nodes where the two T and B chains merge
        sides = set(n.name_split.side_z for n in node.get_merged_siblings())
        return {SideZ.BOTTOM, SideZ.TOP}.issubset(sides)

    def init_eye_corner_space(self):
        """Initialize the coordinate space of the eye based on two corners."""
        if self.eye_corner_matrix:
            return

        if len(self.eye_corner_nodes) != 2:
            self.raise_error('Expected 2 eye corners, but found {}', len(self.eye_corner_nodes))

        # Build a coordinate space with XY plane based on eye axis and two corners
        corner_axis = self.eye_corner_nodes[1].point - self.eye_corner_nodes[0].point

        matrix = matrix_from_axis_pair(self.axis, corner_axis, 'x').to_4x4()
        matrix.translation = self.center
        self.eye_corner_matrix = matrix.inverted()

        # Compute signed angles from space_axis to the eye corners
        angle_min, angle_max = sorted(map(self.get_eye_corner_angle, self.eye_corner_nodes))

        self.eye_corner_range = (angle_min, angle_max)

        if not (angle_min <= 0 <= angle_max):
            self.raise_error('Bad relative angles of eye corners: {}..{}',
                             math.degrees(angle_min), math.degrees(angle_max))

    def get_eye_corner_angle(self, node: ControlBoneNode) -> float:
        """Compute a signed Z rotation angle from the eye axis to the node."""
        pt = self.eye_corner_matrix @ node.point
        return math.atan2(pt.x, pt.y)

    def get_master_control_position(self) -> Vector:
        """Compute suitable position for the master control."""
        self.init_eye_corner_space()

        # Place the control between the two corners on the eye axis
        corner_points = [node.point for node in self.eye_corner_nodes]

        point, _ = mathutils.geometry.intersect_line_line(
            self.center, self.center + self.axis, corner_points[0], corner_points[1]
        )
        return point

    def get_lid_follow_influence(self, node: ControlBoneNode):
        """Compute the influence factor of the eye movement on this eyelid control node."""
        self.init_eye_corner_space()

        # Interpolate from axis to corners based on Z angle
        angle = self.get_eye_corner_angle(node)
        angle_min, angle_max = self.eye_corner_range

        if angle_min < angle < 0:
            return 1 - min(1.0, angle / angle_min) ** 2
        elif 0 < angle < angle_max:
            return 1 - min(1.0, angle / angle_max) ** 2
        else:
            return 0

    ####################################################
    # BONES

    class CtrlBones(BaseSkinRig.CtrlBones):
        master: str                    # Parent control for moving the whole eye.
        target: str                    # Individual target this eye aims for.

    class MchBones(BaseSkinRig.MchBones):
        master: str                    # Bone that rotates to track ctrl.target.
        track: str                     # Bone that translates to follow mch.master tail.

    class DeformBones(TypedBoneDict):
        master: str                    # Deform mirror of ctrl.master.
        eye: str                       # Deform bone that rotates with mch.master.
        iris: str                      # Iris deform bone at master tail that scales with ctrl.target

    bones: BaseSkinRig.ToplevelBones[
        str,
        'Rig.CtrlBones',
        'Rig.MchBones',
        'Rig.DeformBones'
    ]

    ####################################################
    # CHILD CHAINS

    def init_child_chains(self):
        self.child_chains = [rig for rig in self.rigify_children if isinstance(rig, BasicChainRig)]

        # Inject a component twisting handles to the eye radius
        for child in self.child_chains:
            self.patch_chain(child)

    def patch_chain(self, child: BasicChainRig):
        return EyelidChainPatch(child, self)

    ####################################################
    # CONTROL NODES

    def extend_control_node_parent(self, parent, node: BaseSkinNode):
        if not isinstance(node, ControlBoneNode):
            return parent

        if self.is_eye_control_node(node):
            if self.is_eye_corner_node(node):
                # Remember corners for later computations
                assert not self.eye_corner_matrix
                self.eye_corner_nodes.append(node)
            else:
                # Non-corners get extra motion applied to them
                return self.extend_mid_node_parent(parent, node)

        return parent

    def extend_mid_node_parent(self, parent: ControlBoneParentBase, node: ControlBoneNode):
        parent = ControlBoneParentOffset(self, node, parent)

        # Add movement of the eye to the eyelid controls
        parent.add_copy_local_location(
            LazyRef(self.bones.mch, 'track'),
            influence=LazyRef(self.get_lid_follow_influence, node)
        )

        # If Limit Distance on the control can be disabled, add another one to the mch
        if self.params.eyelid_detach_option:
            parent.add_limit_distance(
                self.bones.org,
                distance=(node.point - self.center).length,
                limit_mode='LIMITDIST_ONSURFACE', use_transform_limit=True,
                # Use custom space to accommodate scaling
                space='CUSTOM', space_object=self.obj, space_subtarget=self.bones.org,
                # Don't allow reordering this limit and subsequent offsets
                ensure_order=True,
            )

        return parent

    def extend_control_node_rig(self, node: ControlBoneNode):
        if self.is_eye_control_node(node):
            # Add Limit Distance to enforce following the surface of the eye to the control
            con = self.make_constraint(
                node.control_bone, 'LIMIT_DISTANCE', self.bones.org,
                distance=(node.point - self.center).length,
                limit_mode='LIMITDIST_ONSURFACE', use_transform_limit=True,
                # Use custom space to accommodate scaling
                space='CUSTOM', space_object=self.obj, space_subtarget=self.bones.org,
            )

            if self.params.eyelid_detach_option:
                self.make_driver(con, 'influence',
                                 variables=[(self.bones.ctrl.target, 'lid_attach')])

    ####################################################
    # SCRIPT

    @stage.configure_bones
    def configure_script_panels(self):
        ctrl = self.bones.ctrl

        controls = sum((chain.get_all_controls() for chain in self.child_chains), ctrl.flatten())
        panel = self.script.panel_with_selected_check(self, controls)

        self.add_custom_properties()
        self.add_ui_sliders(panel)

    def add_custom_properties(self):
        target = self.bones.ctrl.target

        if self.params.eyelid_follow_split:
            self.make_property(
                target, 'lid_follow', list(self.params.eyelid_follow_default),
                description='Eyelids follow eye movement (X and Z)'
            )
        else:
            self.make_property(target, 'lid_follow', 1.0,
                               description='Eyelids follow eye movement')

        if self.params.eyelid_detach_option:
            self.make_property(target, 'lid_attach', 1.0,
                               description='Eyelids follow eye surface')

    def add_ui_sliders(self, panel: PanelLayout, *, add_name=False):
        target = self.bones.ctrl.target

        name_tail = f' ({target})' if add_name else ''
        follow_text = iface_("Eyelids Follow{}").format(name_tail)

        if self.params.eyelid_follow_split:
            row = panel.split(factor=0.66, align=True)
            row.custom_prop(target, 'lid_follow', index=0, text=follow_text, translate=False, slider=True)
            row.custom_prop(target, 'lid_follow', index=1, text='', slider=True)
        else:
            panel.custom_prop(target, 'lid_follow', text=follow_text, translate=False, slider=True)

        if self.params.eyelid_detach_option:
            text = iface_("Eyelids Attached{}").format(name_tail)
            panel.custom_prop(target, 'lid_attach', text=text, translate=False, slider=True)

    ####################################################
    # Master control

    @stage.generate_bones
    def make_master_control(self):
        org = self.bones.org
        name = self.copy_bone(org, make_derived_name(org, 'ctrl', '_master'), parent=True)
        put_bone(self.obj, name, self.get_master_control_position())
        self.bones.ctrl.master = name

    @stage.configure_bones
    def configure_master_control(self):
        self.copy_bone_properties(self.bones.org, self.bones.ctrl.master)

    @stage.generate_widgets
    def make_master_control_widget(self):
        ctrl = self.bones.ctrl.master
        create_circle_widget(self.obj, ctrl, radius=1, head_tail=0.25)

    ####################################################
    # Tracking MCH

    @stage.generate_bones
    def make_mch_track_bones(self):
        org = self.bones.org
        mch = self.bones.mch

        mch.master = self.copy_bone(org, make_derived_name(org, 'mch'))
        mch.track = self.copy_bone(org, make_derived_name(org, 'mch', '_track'), scale=1 / 4)

        put_bone(self.obj, mch.track, self.get_bone(org).tail)

    @stage.parent_bones
    def parent_mch_track_bones(self):
        mch = self.bones.mch
        ctrl = self.bones.ctrl
        self.set_bone_parent(mch.master, ctrl.master)
        self.set_bone_parent(mch.track, ctrl.master)

    @stage.rig_bones
    def rig_mch_track_bones(self):
        mch = self.bones.mch
        ctrl = self.bones.ctrl

        # Rotationally track the target bone in mch.master
        self.make_constraint(mch.master, 'DAMPED_TRACK', ctrl.target)

        # Translate to track the tail of mch.master in mch.track. Its local
        # location is then copied to the control nodes.
        # Two constraints are used to provide different X and Z influence values.
        con_x = self.make_constraint(
            mch.track, 'COPY_LOCATION', mch.master, head_tail=1, name='lid_follow_x',
            use_xyz=(True, False, False),
            space='CUSTOM', space_object=self.obj, space_subtarget=self.bones.org,
        )

        con_z = self.make_constraint(
            mch.track, 'COPY_LOCATION', mch.master, head_tail=1, name='lid_follow_z',
            use_xyz=(False, False, True),
            space='CUSTOM', space_object=self.obj, space_subtarget=self.bones.org,
        )

        # Apply follow slider influence(s)
        if self.params.eyelid_follow_split:
            self.make_driver(con_x, 'influence', variables=[(ctrl.target, 'lid_follow', 0)])
            self.make_driver(con_z, 'influence', variables=[(ctrl.target, 'lid_follow', 1)])
        else:
            factor = self.params.eyelid_follow_default

            self.make_driver(
                con_x, 'influence', expression=f'var*{factor[0]}',
                variables=[(ctrl.target, 'lid_follow')]
            )
            self.make_driver(
                con_z, 'influence', expression=f'var*{factor[1]}',
                variables=[(ctrl.target, 'lid_follow')]
            )

    ####################################################
    # ORG bone

    @stage.parent_bones
    def parent_org_chain(self):
        self.set_bone_parent(self.bones.org, self.bones.ctrl.master, inherit_scale='FULL')

    ####################################################
    # Deform bones

    @stage.generate_bones
    def make_deform_bone(self):
        org = self.bones.org
        deform = self.bones.deform
        deform.master = self.copy_bone(org, make_derived_name(org, 'def', '_master'), scale=3 / 2)

        if self.params.make_deform:
            deform.eye = self.copy_bone(org, make_derived_name(org, 'def'))
            deform.iris = self.copy_bone(org, make_derived_name(org, 'def', '_iris'), scale=1 / 2)
            put_bone(self.obj, deform.iris, self.get_bone(org).tail)

    @stage.parent_bones
    def parent_deform_chain(self):
        deform = self.bones.deform
        self.set_bone_parent(deform.master, self.bones.org)

        if self.params.make_deform:
            self.set_bone_parent(deform.eye, self.bones.mch.master)
            self.set_bone_parent(deform.iris, deform.eye)

    @stage.rig_bones
    def rig_deform_chain(self):
        if self.params.make_deform:
            # Copy XZ local scale from the eye target control
            self.make_constraint(
                self.bones.deform.iris, 'COPY_SCALE', self.bones.ctrl.target,
                owner_space='LOCAL', target_space='LOCAL_OWNER_ORIENT', use_y=False,
            )

    ####################################################
    # SETTINGS

    @classmethod
    def add_parameters(cls, params):
        params.make_deform = bpy.props.BoolProperty(
            name="Deform",
            default=True,
            description="Create a deform bone for the copy"
        )

        params.eyelid_detach_option = bpy.props.BoolProperty(
            name="Eyelid Detach Option",
            default=False,
            description="Create an option to detach eyelids from the eye surface"
        )

        params.eyelid_follow_split = bpy.props.BoolProperty(
            name="Split Eyelid Follow Slider",
            default=False,
            description="Create separate eyelid follow influence sliders for X and Z"
        )

        params.eyelid_follow_default = bpy.props.FloatVectorProperty(
            size=2,
            name="Eyelids Follow Default",
            default=(0.2, 0.7), min=0, max=1,
            description="Default setting for the Eyelids Follow sliders (X and Z)",
        )

    @classmethod
    def parameters_ui(cls, layout, params):
        col = layout.column()
        col.prop(params, "make_deform", text="Eyeball And Iris Deforms")
        col.prop(params, "eyelid_detach_option")

        col.prop(params, "eyelid_follow_split")

        row = col.row(align=True)
        row.prop(params, "eyelid_follow_default", index=0, text="Follow X", slider=True)
        row.prop(params, "eyelid_follow_default", index=1, text="Follow Z", slider=True)


class EyelidChainPatch(RigComponent):
    """Component injected into child chains to twist handles aiming Z axis at the eye center."""

    rigify_sub_object_run_late = True

    owner: BasicChainRig

    def __init__(self, owner: BasicChainRig, eye: Rig):
        super().__init__(owner)

        self.eye = eye
        self.owner.use_pre_handles = True

    def align_bone(self, name):
        """Align bone rest orientation to aim Z axis at the eye center."""
        align_bone_z_axis(self.obj, name, self.eye.center - self.get_bone(name).head)

    def prepare_bones(self):
        for org in self.owner.bones.org:
            self.align_bone(org)

    def generate_bones(self):
        if self.owner.use_bbones:
            mch = self.owner.bones.mch
            for pre in [*mch.handles_pre, *mch.handles]:
                self.align_bone(pre)

    def rig_bones(self):
        if self.owner.use_bbones:
            for pre, node in zip(self.owner.bones.mch.handles_pre, self.owner.control_nodes):
                self.make_constraint(pre, 'COPY_LOCATION', node.control_bone, name='locate_cur')
                self.make_constraint(
                    pre, 'LOCKED_TRACK', self.eye.bones.org, name='track_center',
                    track_axis='TRACK_Z', lock_axis='LOCK_Y',
                )


class EyeClusterControl(RigComponent):
    """Component generating a common control for an eye cluster."""

    owner: Rig

    rig_list: list[Rig]
    rig_count: int

    size: float
    matrix: Matrix           # Cluster plane matrix
    inv_matrix: Matrix       # World to cluster plane

    rig_points: dict[Rig, Vector]  # Eye projections in cluster plane space

    master_bone: str
    child_bones: list[str]

    def __init__(self, owner: Rig):
        super().__init__(owner)

        self.find_cluster_rigs()

    def find_cluster_rigs(self):
        """Find and register all other eyes that belong to this cluster."""
        owner = self.owner

        owner.cluster_control = self
        self.rig_list = [owner]

        # Collect all sibling eye rigs
        parent_rig = owner.rigify_parent
        if parent_rig:
            for rig in parent_rig.rigify_children:
                if isinstance(rig, Rig) and rig != owner:
                    rig.cluster_control = self
                    self.rig_list.append(rig)

        self.rig_count = len(self.rig_list)

    ####################################################
    # UTILITIES

    def find_cluster_position(self):
        """Compute the eye cluster control position and orientation."""

        # Average location and Y axis of all the eyes
        axis = Vector((0, 0, 0))
        center = Vector((0, 0, 0))
        length = 0

        for rig in self.rig_list:
            bone = self.get_bone(rig.base_bone)
            axis += bone.y_axis
            center += bone.head
            length += bone.length

        axis /= self.rig_count
        center /= self.rig_count
        length /= self.rig_count

        # Create the matrix from the average Y and world Z
        matrix = matrix_from_axis_pair((0, 0, 1), axis, 'z').to_4x4()
        matrix.translation = center + axis * length * 5

        self.size = length * 3 / 4
        self.matrix = matrix
        self.inv_matrix = matrix.inverted()

    def project_rig_control(self, rig: Rig):
        """Intersect the given eye Y axis with the cluster plane, returns (x,y,0)."""
        bone = self.get_bone(rig.base_bone)

        head = self.inv_matrix @ bone.head
        tail = self.inv_matrix @ bone.tail
        axis = tail - head

        return head + axis * (-head.z / axis.z)

    def get_common_rig_name(self):
        """Choose a name for the cluster control based on the members."""
        names = set(rig.base_bone for rig in self.rig_list)
        name = min(names)

        if mirror_name(name) in names:
            return change_name_side(name, side=Side.MIDDLE)

        return name

    def get_rig_control_matrix(self, rig: Rig):
        """Compute a matrix for an individual eye sub-control."""
        matrix = self.matrix.copy()
        matrix.translation = self.matrix @ self.rig_points[rig]
        return matrix

    def get_master_control_layers(self):
        """Combine layers of all eyes for the cluster control."""
        return union_layer_lists(list(self.get_bone(rig.base_bone).collections) for rig in self.rig_list)

    def get_all_rig_control_bones(self):
        """Make a list of all control bones of all clustered eyes."""
        return list(set(sum((rig.bones.ctrl.flatten() for rig in self.rig_list), [self.master_bone])))

    ####################################################
    # STAGES

    def initialize(self):
        self.find_cluster_position()
        self.rig_points = {rig: self.project_rig_control(rig) for rig in self.rig_list}

    def generate_bones(self):
        if self.rig_count > 1:
            self.master_bone = self.make_master_control()
            self.child_bones = []

            for rig in self.rig_list:
                rig.bones.ctrl.target = child = self.make_child_control(rig)
                self.child_bones.append(child)
        else:
            self.master_bone = self.make_child_control(self.rig_list[0])
            self.child_bones = [self.master_bone]
            self.owner.bones.ctrl.target = self.master_bone

        self.build_parent_switch()

    def make_master_control(self):
        name = self.new_bone(make_derived_name(self.get_common_rig_name(), 'ctrl', '_common'))
        bone = self.get_bone(name)
        bone.matrix = self.matrix
        bone.length = self.size
        set_bone_layers(bone, self.get_master_control_layers())
        return name

    def make_child_control(self, rig: Rig):
        name = rig.copy_bone(
            rig.base_bone, make_derived_name(rig.base_bone, 'ctrl'), length=self.size)
        self.get_bone(name).matrix = self.get_rig_control_matrix(rig)
        return name

    def build_parent_switch(self):
        pbuilder = SwitchParentBuilder(self.owner.generator)

        org_parent = self.owner.rig_parent_bone
        parents = [org_parent] if org_parent else []

        pbuilder.build_child(
            self.owner, self.master_bone,
            prop_name=f'Parent ({self.master_bone})',
            extra_parents=parents, select_parent=org_parent,
            controls=self.get_all_rig_control_bones
        )

    def parent_bones(self):
        if self.rig_count > 1:
            self.get_bone(self.master_bone).use_local_location = False

            for child in self.child_bones:
                self.set_bone_parent(child, self.master_bone)

    def configure_bones(self):
        for child in self.child_bones:
            bone = self.get_bone(child)
            bone.lock_rotation = (True, True, True)
            bone.lock_rotation_w = True

        # When the cluster master control is selected, show sliders for all eyes
        if self.rig_count > 1:
            panel = self.owner.script.panel_with_selected_check(self.owner, [self.master_bone])

            for rig in self.rig_list:
                rig.add_ui_sliders(panel, add_name=True)

    def generate_widgets(self):
        for child in self.child_bones:
            create_eye_widget(self.obj, child)

        if self.rig_count > 1:
            pt2d = [p.to_2d() / self.size for p in self.rig_points.values()]
            create_eye_cluster_widget(self.obj, self.master_bone, points=pt2d)


@widget_generator
def create_eye_widget(geom, *, size=1):
    generate_circle_geometry(geom, Vector((0, 0, 0)), size / 2)


@widget_generator
def create_eye_cluster_widget(geom, *, size=1, points):
    hull_points = [points[i] for i in mathutils.geometry.convex_hull_2d(points)]

    generate_circle_hull_geometry(geom, hull_points, size * 0.75, size * 0.6)
    generate_circle_hull_geometry(geom, hull_points, size, size * 0.85)


def create_sample(obj):
    # generated by rigify.utils.write_metarig
    bpy.ops.object.mode_set(mode='EDIT')
    arm = obj.data

    bones = {}

    bone = arm.edit_bones.new('eye.L')
    bone.head = 0.0000, 0.0000, 0.0000
    bone.tail = 0.0000, -0.0125, 0.0000
    bone.roll = 0.0000
    bone.use_connect = False
    bones['eye.L'] = bone.name
    bone = arm.edit_bones.new('lid1.T.L')
    bone.head = 0.0155, -0.0006, -0.0003
    bone.tail = 0.0114, -0.0099, 0.0029
    bone.roll = 2.9453
    bone.use_connect = False
    bone.parent = arm.edit_bones[bones['eye.L']]
    bones['lid1.T.L'] = bone.name
    bone = arm.edit_bones.new('lid1.B.L')
    bone.head = 0.0155, -0.0006, -0.0003
    bone.tail = 0.0112, -0.0095, -0.0039
    bone.roll = -0.0621
    bone.use_connect = False
    bone.parent = arm.edit_bones[bones['eye.L']]
    bones['lid1.B.L'] = bone.name
    bone = arm.edit_bones.new('lid2.T.L')
    bone.head = 0.0114, -0.0099, 0.0029
    bone.tail = 0.0034, -0.0149, 0.0040
    bone.roll = 2.1070
    bone.use_connect = True
    bone.parent = arm.edit_bones[bones['lid1.T.L']]
    bones['lid2.T.L'] = bone.name
    bone = arm.edit_bones.new('lid2.B.L')
    bone.head = 0.0112, -0.0095, -0.0039
    bone.tail = 0.0029, -0.0140, -0.0057
    bone.roll = 0.8337
    bone.use_connect = True
    bone.parent = arm.edit_bones[bones['lid1.B.L']]
    bones['lid2.B.L'] = bone.name
    bone = arm.edit_bones.new('lid3.T.L')
    bone.head = 0.0034, -0.0149, 0.0040
    bone.tail = -0.0046, -0.0157, 0.0026
    bone.roll = 1.7002
    bone.use_connect = True
    bone.parent = arm.edit_bones[bones['lid2.T.L']]
    bones['lid3.T.L'] = bone.name
    bone = arm.edit_bones.new('lid3.B.L')
    bone.head = 0.0029, -0.0140, -0.0057
    bone.tail = -0.0041, -0.0145, -0.0057
    bone.roll = 1.0671
    bone.use_connect = True
    bone.parent = arm.edit_bones[bones['lid2.B.L']]
    bones['lid3.B.L'] = bone.name
    bone = arm.edit_bones.new('lid4.T.L')
    bone.head = -0.0046, -0.0157, 0.0026
    bone.tail = -0.0123, -0.0140, -0.0049
    bone.roll = 1.0850
    bone.use_connect = True
    bone.parent = arm.edit_bones[bones['lid3.T.L']]
    bones['lid4.T.L'] = bone.name
    bone = arm.edit_bones.new('lid4.B.L')
    bone.head = -0.0041, -0.0145, -0.0057
    bone.tail = -0.0123, -0.0140, -0.0049
    bone.roll = 1.1667
    bone.use_connect = True
    bone.parent = arm.edit_bones[bones['lid3.B.L']]
    bones['lid4.B.L'] = bone.name

    bpy.ops.object.mode_set(mode='OBJECT')
    pbone = obj.pose.bones[bones['eye.L']]
    pbone.rigify_type = 'face.skin_eye'
    pbone.lock_location = (False, False, False)
    pbone.lock_rotation = (False, False, False)
    pbone.lock_rotation_w = False
    pbone.lock_scale = (False, False, False)
    pbone.rotation_mode = 'QUATERNION'
    pbone = obj.pose.bones[bones['lid1.T.L']]
    pbone.rigify_type = 'skin.stretchy_chain'
    pbone.lock_location = (False, False, False)
    pbone.lock_rotation = (False, False, False)
    pbone.lock_rotation_w = False
    pbone.lock_scale = (False, False, False)
    pbone.rotation_mode = 'QUATERNION'
    try:
        pbone.rigify_parameters.skin_chain_pivot_pos = 2
    except AttributeError:
        pass
    try:
        pbone.rigify_parameters.bbones = 5
    except AttributeError:
        pass
    try:
        pbone.rigify_parameters.skin_chain_connect_mirror = [False, False]
    except AttributeError:
        pass
    pbone = obj.pose.bones[bones['lid1.B.L']]
    pbone.rigify_type = 'skin.stretchy_chain'
    pbone.lock_location = (False, False, False)
    pbone.lock_rotation = (False, False, False)
    pbone.lock_rotation_w = False
    pbone.lock_scale = (False, False, False)
    pbone.rotation_mode = 'QUATERNION'
    try:
        pbone.rigify_parameters.skin_chain_pivot_pos = 2
    except AttributeError:
        pass
    try:
        pbone.rigify_parameters.bbones = 5
    except AttributeError:
        pass
    try:
        pbone.rigify_parameters.skin_chain_connect_mirror = [False, False]
    except AttributeError:
        pass
    pbone = obj.pose.bones[bones['lid2.T.L']]
    pbone.rigify_type = ''
    pbone.lock_location = (False, False, False)
    pbone.lock_rotation = (False, False, False)
    pbone.lock_rotation_w = False
    pbone.lock_scale = (False, False, False)
    pbone.rotation_mode = 'QUATERNION'
    pbone = obj.pose.bones[bones['lid2.B.L']]
    pbone.rigify_type = ''
    pbone.lock_location = (False, False, False)
    pbone.lock_rotation = (False, False, False)
    pbone.lock_rotation_w = False
    pbone.lock_scale = (False, False, False)
    pbone.rotation_mode = 'QUATERNION'
    pbone = obj.pose.bones[bones['lid3.T.L']]
    pbone.rigify_type = ''
    pbone.lock_location = (False, False, False)
    pbone.lock_rotation = (False, False, False)
    pbone.lock_rotation_w = False
    pbone.lock_scale = (False, False, False)
    pbone.rotation_mode = 'QUATERNION'
    pbone = obj.pose.bones[bones['lid3.B.L']]
    pbone.rigify_type = ''
    pbone.lock_location = (False, False, False)
    pbone.lock_rotation = (False, False, False)
    pbone.lock_rotation_w = False
    pbone.lock_scale = (False, False, False)
    pbone.rotation_mode = 'QUATERNION'
    pbone = obj.pose.bones[bones['lid4.T.L']]
    pbone.rigify_type = ''
    pbone.lock_location = (False, False, False)
    pbone.lock_rotation = (False, False, False)
    pbone.lock_rotation_w = False
    pbone.lock_scale = (False, False, False)
    pbone.rotation_mode = 'QUATERNION'
    pbone = obj.pose.bones[bones['lid4.B.L']]
    pbone.rigify_type = ''
    pbone.lock_location = (False, False, False)
    pbone.lock_rotation = (False, False, False)
    pbone.lock_rotation_w = False
    pbone.lock_scale = (False, False, False)
    pbone.rotation_mode = 'QUATERNION'

    bpy.ops.object.mode_set(mode='EDIT')
    for bone in arm.edit_bones:
        bone.select = False
        bone.select_head = False
        bone.select_tail = False
    for b in bones:
        bone = arm.edit_bones[bones[b]]
        bone.select = True
        bone.select_head = True
        bone.select_tail = True
        bone.bbone_x = bone.bbone_z = bone.length * 0.05
        arm.edit_bones.active = bone
        if bcoll := arm.collections.active:
            bcoll.assign(bone)

    return bones


```

## Source file: scripts/addons_core/rigify/rigs/face/skin_jaw.py

{
  "path": "scripts/addons_core/rigify/rigs/face/skin_jaw.py",
  "blob_sha": "8d519d7f8bdfd522d0c214ebcc71b3b31fcd1ff7",
  "sha256": "2309df2702996da9c7dd69079d274c6ac3ab0d699861e9fb8747180247508720",
  "lines": 880,
  "bytes": 32190,
  "selection_score": 36,
  "keyword_line_counts": {
    "skin": 27,
    "jaw": 38,
    "mouth": 76,
    "teeth": 8,
    "deform": 11,
    "rig": 86,
    "face": 1,
    "lip": 46
  },
  "url": "https://github.com/blender/blender/blob/64d181010e0dbc49d2faaaa057357aef8d9a809c/scripts/addons_core/rigify/rigs/face/skin_jaw.py",
  "function_signature_candidates": [
    {
      "line": 38,
      "text": "def find_org_bones(self, bone: PoseBone) -> str:"
    },
    {
      "line": 57,
      "text": "def initialize(self):"
    },
    {
      "line": 68,
      "text": "def get_mouth_orientation(self) -> Quaternion:"
    },
    {
      "line": 74,
      "text": "def is_corner_node(self, node: ControlBoneNode) -> Side | SideZ | None:"
    },
    {
      "line": 130,
      "text": "def init_child_chains(self):"
    },
    {
      "line": 139,
      "text": "def arrange_child_chains(self):"
    },
    {
      "line": 234,
      "text": "def position_mouth_bone(self, name: str, scale: float):"
    },
    {
      "line": 244,
      "text": "def get_node_parent_bones(self, node: ControlBoneNode"
    },
    {
      "line": 282,
      "text": "def get_parent_for_name(self, name: str, parent_bone: str) -> Lazy[str]:"
    },
    {
      "line": 293,
      "text": "def get_child_chain_parent(self, rig: BaseSkinRig, parent_bone: str) -> str:"
    },
    {
      "line": 296,
      "text": "def build_control_node_parent(self, node: BaseSkinNode, parent_bone: str):"
    },
    {
      "line": 313,
      "text": "def make_master_control(self):"
    },
    {
      "line": 319,
      "text": "def configure_master_control(self):"
    },
    {
      "line": 325,
      "text": "def make_master_control_widget(self):"
    },
    {
      "line": 333,
      "text": "def make_mouth_control(self):"
    },
    {
      "line": 340,
      "text": "def parent_mouth_control(self):"
    },
    {
      "line": 344,
      "text": "def configure_mouth_control(self):"
    },
    {
      "line": 348,
      "text": "def make_mouth_control_widget(self):"
    },
    {
      "line": 367,
      "text": "def make_mch_lock_bones(self):"
    },
    {
      "line": 382,
      "text": "def make_mch_top_bone(self, _i: int, org: str):"
    },
    {
      "line": 385,
      "text": "def make_mch_bottom_bone(self, _i: int, org: str):"
    },
    {
      "line": 388,
      "text": "def make_mch_middle_bone(self, _i: int, org: str):"
    },
    {
      "line": 392,
      "text": "def parent_mch_lock_bones(self):"
    },
    {
      "line": 403,
      "text": "def configure_mch_lock_bones(self):"
    },
    {
      "line": 412,
      "text": "def rig_mch_track_bones(self):"
    },
    {
      "line": 452,
      "text": "def make_mch_mouth_bones(self):"
    },
    {
      "line": 465,
      "text": "def make_mch_mouth_bone(self, _i: int, suffix: str, size: float):"
    },
    {
      "line": 470,
      "text": "def make_mch_mouth_inout_bone(self, _i: int, suffix: str, size: float):"
    },
    {
      "line": 474,
      "text": "def parent_mch_mouth_bones(self):"
    },
    {
      "line": 486,
      "text": "def rig_mch_mouth_bones(self):"
    },
    {
      "line": 505,
      "text": "def rig_mch_mouth_layer_bone(self, i: int, mch: str, ctrl: str):"
    },
    {
      "line": 521,
      "text": "def parent_org_chain(self):"
    },
    {
      "line": 528,
      "text": "def make_deform_bone(self):"
    },
    {
      "line": 533,
      "text": "def parent_deform_chain(self):"
    },
    {
      "line": 541,
      "text": "def add_parameters(cls, params):"
    },
    {
      "line": 561,
      "text": "def parameters_ui(cls, layout, params):"
    },
    {
      "line": 567,
      "text": "def create_sample(obj):"
    }
  ]
}

```text

# SPDX-FileCopyrightText: 2021-2022 Blender Foundation
#
# SPDX-License-Identifier: GPL-2.0-or-later

from typing import Optional

import bpy
import math

from itertools import repeat

from bpy.types import PoseBone
from mathutils import Vector, Matrix, Quaternion
from bl_math import clamp

from ...utils.bones import TypedBoneDict
from ...utils.naming import make_derived_name, Side, SideZ, get_name_side_z
from ...utils.misc import map_list, matrix_from_axis_pair, LazyRef, Lazy
from ...utils.widgets_basic import create_circle_widget

from ...base_rig import stage

from ..skin.skin_nodes import ControlBoneNode, BaseSkinNode
from ..skin.skin_parents import ControlBoneParentOrg, ControlBoneParentArmature
from ..skin.skin_rigs import BaseSkinRig

from ..skin.basic_chain import Rig as BasicChainRig

from ..widgets import create_jaw_widget


class Rig(BaseSkinRig):
    """
    Jaw rig that manages loops of four mouth chains each. The chains
    must connect together at their ends using L/R and T/B symmetry.
    """

    def find_org_bones(self, bone: PoseBone) -> str:
        return bone.name

    mouth_orientation: Quaternion

    chain_to_layer: Optional[dict[BasicChainRig, int]]
    num_layers: int

    child_chains: list[BasicChainRig]
    corners: dict[Side | SideZ, list[ControlBoneNode]]
    chains_by_side: dict[Side | SideZ, set[BasicChainRig]]

    mouth_center: Vector
    mouth_space: Matrix
    to_mouth_space: Matrix

    left_sign: int
    layer_width: list[float]

    def initialize(self):
        super().initialize()

        self.mouth_orientation = self.get_mouth_orientation()
        self.chain_to_layer = None

        self.init_child_chains()

    ####################################################
    # UTILITIES

    def get_mouth_orientation(self) -> Quaternion:
        jaw_axis = self.get_bone(self.base_bone).y_axis.copy()
        jaw_axis[2] = 0

        return matrix_from_axis_pair(jaw_axis, (0, 0, 1), 'z').to_quaternion()

    def is_corner_node(self, node: ControlBoneNode) -> Side | SideZ | None:
        # Corners are nodes where two T/B or L/R chains meet.
        siblings = [n for n in node.get_merged_siblings() if n.rig in self.child_chains]

        sides_x = set(n.name_split.side for n in siblings)
        sides_z = set(n.name_split.side_z for n in siblings)

        if {SideZ.BOTTOM, SideZ.TOP}.issubset(sides_z):
            if Side.LEFT in sides_x:
                return Side.LEFT
            else:
                return Side.RIGHT

        if {Side.LEFT, Side.RIGHT}.issubset(sides_x):
            if SideZ.TOP in sides_z:
                return SideZ.TOP
            else:
                return SideZ.BOTTOM

        return None

    ####################################################
    # BONES

    class CtrlBones(BaseSkinRig.CtrlBones):
        master: str                    # Main jaw open control.
        mouth: str                     # Main control for adjusting mouth position and scale.

    class MchBones(BaseSkinRig.MchBones):
        lock: str                      # Jaw master mirror for the locked mouth.

        top: list[str]                 # Jaw master mirrors for the loop top.
        bottom: list[str]              # Jaw master mirrors for the loop bottom.
        middle: list[str]              # Middle position between top[] and bottom[].

        mouth_parent: str              # Parent for ctrl.mouth, mouth_layers and *_in (= middle[0])
        mouth_layers: list[str]        # Apply fade out of ctrl.mouth motion for outer loops.

        # Combine mouth and jaw motions via Copy Custom to Local.
        top_out: list[str]
        bottom_out: list[str]
        middle_out: list[str]

    class DeformBones(TypedBoneDict):
        master: str                    # Deform mirror of ctrl.master.

    bones: BaseSkinRig.ToplevelBones[
        str,
        'Rig.CtrlBones',
        'Rig.MchBones',
        'Rig.DeformBones'
    ]

    ####################################################
    # CHILD CHAINS

    def init_child_chains(self):
        self.child_chains = [
            rig
            for rig in self.rigify_children
            if isinstance(rig, BasicChainRig) and get_name_side_z(rig.base_bone) != SideZ.MIDDLE
        ]

        self.corners = {Side.LEFT: [], Side.RIGHT: [], SideZ.TOP: [], SideZ.BOTTOM: []}

    def arrange_child_chains(self):
        """Sort child chains into their corresponding mouth loops."""
        if self.chain_to_layer is not None:
            return

        # Index child node corners
        for child in self.child_chains:
            for node in child.control_nodes:
                corner = self.is_corner_node(node)
                if corner:
                    if node.merged_master not in self.corners[corner]:
                        self.corners[corner].append(node.merged_master)

        self.num_layers = len(self.corners[SideZ.TOP])

        for k, v in self.corners.items():
            if len(v) == 0:
                self.raise_error("Could not find all mouth corners")
            if len(v) != self.num_layers:
                self.raise_error(
                    "Mouth corner counts differ: {} vs {}",
                    [n.name for n in v], [n.name for n in self.corners[SideZ.TOP]]
                )

        # Find inner top/bottom corners
        anchor = self.corners[SideZ.BOTTOM][0].point
        inner_top = min(self.corners[SideZ.TOP], key=lambda p: (p.point - anchor).length)

        anchor = inner_top.point
        inner_bottom = min(self.corners[SideZ.BOTTOM], key=lambda p: (p.point - anchor).length)

        # Compute the mouth space
        self.mouth_center = center = (inner_top.point + inner_bottom.point) / 2

        matrix = self.mouth_orientation.to_matrix().to_4x4()
        matrix.translation = center
        self.mouth_space = matrix
        self.to_mouth_space = matrix.inverted()

        # Build a mapping of child chain to layer (i.e. sort multiple mouth loops)
        self.chain_to_layer = {}
        self.chains_by_side = {}

        for k, v in list(self.corners.items()):
            ordered: list[ControlBoneNode] = sorted(v, key=lambda p: (p.point - center).length)

            self.corners[k] = ordered

            chain_set: set[BasicChainRig] = set()

            for i, node in enumerate(ordered):
                for sibling in node.get_merged_siblings():
                    if sibling.rig in self.child_chains:
                        assert isinstance(sibling.rig, BasicChainRig)

                        cur_layer = self.chain_to_layer.get(sibling.rig)

                        if cur_layer is not None and cur_layer != i:
                            self.raise_error(
                                "Conflicting mouth chain layer on {}: {} and {}",
                                sibling.rig.base_bone, i, cur_layer)

                        self.chain_to_layer[sibling.rig] = i
                        chain_set.add(sibling.rig)

            self.chains_by_side[k] = chain_set

        for child in self.child_chains:
            if child not in self.chain_to_layer:
                self.raise_error("Could not determine chain layer on {}", child.base_bone)

        if not self.chains_by_side[Side.LEFT].isdisjoint(self.chains_by_side[Side.RIGHT]):
            self.raise_error("Left/right conflict in mouth")
        if not self.chains_by_side[SideZ.TOP].isdisjoint(self.chains_by_side[SideZ.BOTTOM]):
            self.raise_error("Top/bottom conflict in mouth")

        # Find left/right direction
        pt = self.to_mouth_space @ self.corners[Side.LEFT][0].point

        self.left_sign = 1 if pt.x > 0 else -1

        for node in self.corners[Side.LEFT]:
            if (self.to_mouth_space @ node.point).x * self.left_sign <= 0:
                self.raise_error("Bad left corner location: {}", node.name)

        for node in self.corners[Side.RIGHT]:
            if (self.to_mouth_space @ node.point).x * self.left_sign >= 0:
                self.raise_error("Bad right corner location: {}", node.name)

        # Find layer loop widths
        self.layer_width = [
            (self.corners[Side.LEFT][i].point - self.corners[Side.RIGHT][i].point).length
            for i in range(self.num_layers)
        ]

    def position_mouth_bone(self, name: str, scale: float):
        self.arrange_child_chains()

        bone = self.get_bone(name)
        bone.matrix = self.mouth_space
        bone.length = self.layer_width[0] * scale

    ####################################################
    # CONTROL NODES

    def get_node_parent_bones(self, node: ControlBoneNode
                              ) -> list[tuple[Lazy[str], float] | Lazy[str]]:
        """Get parent bones and their armature weights for the given control node."""
        self.arrange_child_chains()

        assert isinstance(node.rig, BasicChainRig)

        # Choose correct layer bones
        layer = self.chain_to_layer[node.rig]

        top_mch = LazyRef(self.bones.mch, 'top_out', layer)
        bottom_mch = LazyRef(self.bones.mch, 'bottom_out', layer)
        middle_mch = LazyRef(self.bones.mch, 'middle_out', layer)

        # Corners have one input
        corner = self.is_corner_node(node)
        if corner:
            if corner == SideZ.TOP:
                return [top_mch]
            elif corner == SideZ.BOTTOM:
                return [bottom_mch]
            else:
                return [middle_mch]

        # Otherwise blend two
        if node.rig in self.chains_by_side[SideZ.TOP]:
            side_mch = top_mch
        else:
            side_mch = bottom_mch

        pt_x = (self.to_mouth_space @ node.point).x
        side = Side.LEFT if pt_x * self.left_sign >= 0 else Side.RIGHT

        corner_x = (self.to_mouth_space @ self.corners[side][layer].point).x
        factor = math.sqrt(1 - clamp(pt_x / corner_x) ** 2)

        return [(side_mch, factor), (middle_mch, 1 - factor)]

    def get_parent_for_name(self, name: str, parent_bone: str) -> Lazy[str]:
        """Get single replacement parent for the given child bone."""
        if parent_bone == self.base_bone:
            side = get_name_side_z(name)
            if side == SideZ.TOP:
                return LazyRef(self.bones.mch, 'top', -1)
            if side == SideZ.BOTTOM:
                return LazyRef(self.bones.mch, 'bottom', -1)

        return parent_bone

    def get_child_chain_parent(self, rig: BaseSkinRig, parent_bone: str) -> str:
        return self.get_parent_for_name(rig.base_bone, parent_bone)

    def build_control_node_parent(self, node: BaseSkinNode, parent_bone: str):
        if node.rig in self.child_chains:
            assert isinstance(node, ControlBoneNode)

            return ControlBoneParentArmature(
                self, node,
                bones=self.get_node_parent_bones(node),
                orientation=self.mouth_orientation,
                copy_scale=LazyRef(self.bones.mch, 'mouth_parent'),
            )

        return ControlBoneParentOrg(self.get_parent_for_name(node.name, parent_bone))

    ####################################################
    # Master control

    @stage.generate_bones
    def make_master_control(self):
        org = self.bones.org
        name = self.copy_bone(org, make_derived_name(org, 'ctrl'), parent=True)
        self.bones.ctrl.master = name

    @stage.configure_bones
    def configure_master_control(self):
        self.copy_bone_properties(self.bones.org, self.bones.ctrl.master)

        self.get_bone(self.bones.ctrl.master).lock_scale = (True, True, True)

    @stage.generate_widgets
    def make_master_control_widget(self):
        ctrl = self.bones.ctrl.master
        create_jaw_widget(self.obj, ctrl)

    ####################################################
    # Mouth control

    @stage.generate_bones
    def make_mouth_control(self):
        org = self.bones.org
        name = self.copy_bone(org, make_derived_name(org, 'ctrl', '_mouth'))
        self.position_mouth_bone(name, 1)
        self.bones.ctrl.mouth = name

    @stage.parent_bones
    def parent_mouth_control(self):
        self.set_bone_parent(self.bones.ctrl.mouth, self.bones.mch.mouth_parent)

    @stage.configure_bones
    def configure_mouth_control(self):
        pass

    @stage.generate_widgets
    def make_mouth_control_widget(self):
        ctrl = self.bones.ctrl.mouth

        width = (self.corners[Side.LEFT][0].point - self.corners[Side.RIGHT][0].point).length
        height = (self.corners[SideZ.TOP][0].point - self.corners[SideZ.BOTTOM][0].point).length
        back = (self.corners[Side.LEFT][0].point + self.corners[Side.RIGHT][0].point) / 2
        front = (self.corners[SideZ.TOP][0].point + self.corners[SideZ.BOTTOM][0].point) / 2
        depth = (front - back).length

        create_circle_widget(
            self.obj, ctrl,
            radius=0.2 + 0.5 * (height / width), radius_x=0.7,
            head_tail=0.2, head_tail_x=0.2 - (depth / width)
        )

    ####################################################
    # Jaw Motion MCH

    @stage.generate_bones
    def make_mch_lock_bones(self):
        org = self.bones.org
        mch = self.bones.mch

        self.arrange_child_chains()

        mch.lock = self.copy_bone(
            org, make_derived_name(org, 'mch', '_lock'), scale=1 / 2, parent=True)

        mch.top = map_list(self.make_mch_top_bone, range(self.num_layers), repeat(org))
        mch.bottom = map_list(self.make_mch_bottom_bone, range(self.num_layers), repeat(org))
        mch.middle = map_list(self.make_mch_middle_bone, range(self.num_layers), repeat(org))

        mch.mouth_parent = mch.middle[0]

    def make_mch_top_bone(self, _i: int, org: str):
        return self.copy_bone(org, make_derived_name(org, 'mch', '_top'), scale=1 / 4, parent=True)

    def make_mch_bottom_bone(self, _i: int, org: str):
        return self.copy_bone(org, make_derived_name(org, 'mch', '_bottom'), scale=1 / 3, parent=True)

    def make_mch_middle_bone(self, _i: int, org: str):
        return self.copy_bone(org, make_derived_name(org, 'mch', '_middle'), scale=2 / 3, parent=True)

    @stage.parent_bones
    def parent_mch_lock_bones(self):
        mch = self.bones.mch
        ctrl = self.bones.ctrl

        for mid, top in zip(mch.middle, mch.top):
            self.set_bone_parent(mid, top)

        for bottom in mch.bottom[1:]:
            self.set_bone_parent(bottom, ctrl.master)

    @stage.configure_bones
    def configure_mch_lock_bones(self):
        ctrl = self.bones.ctrl

        panel = self.script.panel_with_selected_check(self, [ctrl.master, ctrl.mouth])

        self.make_property(ctrl.master, 'mouth_lock', 0.0, description='Mouth is locked closed')
        panel.custom_prop(ctrl.master, 'mouth_lock', text='Mouth Lock', slider=True)

    @stage.rig_bones
    def rig_mch_track_bones(self):
        mch = self.bones.mch
        ctrl = self.bones.ctrl

        # Lock position follows jaw master with configured influence
        self.make_constraint(
            mch.lock, 'COPY_TRANSFORMS', ctrl.master,
            influence=self.params.jaw_locked_influence,
        )

        # Innermost top bone follows lock position according to slider
        con = self.make_constraint(mch.top[0], 'COPY_TRANSFORMS', mch.lock)
        self.make_driver(con, 'influence', variables=[(ctrl.master, 'mouth_lock')])

        # Innermost bottom bone follows jaw master with configured influence, and then lock
        self.make_constraint(
            mch.bottom[0], 'COPY_TRANSFORMS', ctrl.master,
            influence=self.params.jaw_mouth_influence,
        )

        con = self.make_constraint(mch.bottom[0], 'COPY_TRANSFORMS', mch.lock)
        self.make_driver(con, 'influence', variables=[(ctrl.master, 'mouth_lock')])

        # Outer layer bones interpolate toward innermost based on influence decay
        fac = self.params.jaw_secondary_influence

        for i, name in enumerate(mch.top[1:]):
            self.make_constraint(name, 'COPY_TRANSFORMS', mch.top[0], influence=fac ** (1 + i))

        for i, name in enumerate(mch.bottom[1:]):
            self.make_constraint(name, 'COPY_TRANSFORMS', mch.bottom[0], influence=fac ** (1 + i))

        # Middle bones interpolate the middle between top and bottom
        for mid, bottom in zip(mch.middle, mch.bottom):
            self.make_constraint(mid, 'COPY_TRANSFORMS', bottom, influence=0.5)

    ####################################################
    # Mouth MCH

    @stage.generate_bones
    def make_mch_mouth_bones(self):
        mch = self.bones.mch

        mch.mouth_layers = map_list(self.make_mch_mouth_bone,
                                    range(1, self.num_layers), repeat('_mouth_layer'), repeat(0.6))

        mch.top_out = map_list(self.make_mch_mouth_inout_bone,
                               range(self.num_layers), repeat('_top_out'), repeat(0.4))
        mch.bottom_out = map_list(self.make_mch_mouth_inout_bone,
                                  range(self.num_layers), repeat('_bottom_out'), repeat(0.35))
        mch.middle_out = map_list(self.make_mch_mouth_inout_bone,
                                  range(self.num_layers), repeat('_middle_out'), repeat(0.3))

    def make_mch_mouth_bone(self, _i: int, suffix: str, size: float):
        name = self.copy_bone(self.bones.org, make_derived_name(self.bones.org, 'mch', suffix))
        self.position_mouth_bone(name, size)
        return name

    def make_mch_mouth_inout_bone(self, _i: int, suffix: str, size: float):
        return self.copy_bone(self.bones.org, make_derived_name(self.bones.org, 'mch', suffix), scale=size)

    @stage.parent_bones
    def parent_mch_mouth_bones(self):
        mch = self.bones.mch
        layers = [self.bones.ctrl.mouth, *mch.mouth_layers]

        for name in mch.mouth_layers:
            self.set_bone_parent(name, mch.mouth_parent)

        for name_list in [mch.top_out, mch.bottom_out, mch.middle_out]:
            for name, parent in zip(name_list, layers):
                self.set_bone_parent(name, parent)

    @stage.rig_bones
    def rig_mch_mouth_bones(self):
        mch = self.bones.mch
        ctrl = self.bones.ctrl.mouth

        # Mouth influence fade out
        for i, name in enumerate(mch.mouth_layers):
            self.rig_mch_mouth_layer_bone(i + 1, name, ctrl)

        # Transfer and combine jaw motion with mouth
        all_jaw = mch.top + mch.bottom + mch.middle
        all_out = mch.top_out + mch.bottom_out + mch.middle_out

        for dest, src in zip(all_out, all_jaw):
            self.make_constraint(
                dest, 'COPY_TRANSFORMS', src,
                owner_space='LOCAL', target_space='CUSTOM',
                space_object=self.obj, space_subtarget=mch.mouth_parent,
            )

    def rig_mch_mouth_layer_bone(self, i: int, mch: str, ctrl: str):
        # Fade location and rotation based on influence decay
        inf = self.params.jaw_secondary_influence ** i

        self.make_constraint(mch, 'COPY_LOCATION', ctrl, influence=inf)
        self.make_constraint(mch, 'COPY_ROTATION', ctrl, influence=inf)

        # For scale, additionally take radius into account
        inf_scale = inf * self.layer_width[0] / self.layer_width[i]

        self.make_constraint(mch, 'COPY_SCALE', ctrl, influence=inf_scale)

    ####################################################
    # ORG bone

    @stage.parent_bones
    def parent_org_chain(self):
        self.set_bone_parent(self.bones.org, self.bones.ctrl.master, inherit_scale='FULL')

    ####################################################
    # Deform bones

    @stage.generate_bones
    def make_deform_bone(self):
        org = self.bones.org
        self.bones.deform.master = self.copy_bone(org, make_derived_name(org, 'def'))

    @stage.parent_bones
    def parent_deform_chain(self):
        deform = self.bones.deform
        self.set_bone_parent(deform.master, self.bones.org)

    ####################################################
    # SETTINGS

    @classmethod
    def add_parameters(cls, params):
        params.jaw_mouth_influence = bpy.props.FloatProperty(
            name="Bottom Lip Influence",
            default=0.5, min=0, max=1,
            description="Influence of the jaw on the bottom lip chains"
        )

        params.jaw_locked_influence = bpy.props.FloatProperty(
            name="Locked Influence",
            default=0.2, min=0, max=1,
            description="Influence of the jaw on the locked mouth"
        )

        params.jaw_secondary_influence = bpy.props.FloatProperty(
            name="Secondary Influence Falloff",
            default=0.5, min=0, max=1,
            description="Reduction factor for each level of secondary mouth loops"
        )

    @classmethod
    def parameters_ui(cls, layout, params):
        layout.prop(params, "jaw_mouth_influence", slider=True)
        layout.prop(params, "jaw_locked_influence", slider=True)
        layout.prop(params, "jaw_secondary_influence", slider=True)


def create_sample(obj):
    # generated by rigify.utils.write_metarig
    bpy.ops.object.mode_set(mode='EDIT')
    arm = obj.data

    bones = {}

    bone = arm.edit_bones.new('jaw')
    bone.head = 0.0000, 0.0000, 0.0000
    bone.tail = 0.0000, -0.0585, -0.0489
    bone.roll = 0.0000
    bone.use_connect = False
    bones['jaw'] = bone.name
    bone = arm.edit_bones.new('teeth.T')
    bone.head = 0.0000, -0.0589, 0.0080
    bone.tail = 0.0000, -0.0283, 0.0080
    bone.roll = 0.0000
    bone.use_connect = False
    bones['teeth.T'] = bone.name
    bone = arm.edit_bones.new('lip.T.L')
    bone.head = -0.0000, -0.0684, 0.0030
    bone.tail = 0.0105, -0.0655, 0.0033
    bone.roll = -0.0000
    bone.use_connect = False
    bone.parent = arm.edit_bones[bones['jaw']]
    bones['lip.T.L'] = bone.name
    bone = arm.edit_bones.new('lip.B.L')
    bone.head = -0.0000, -0.0655, -0.0078
    bone.tail = 0.0107, -0.0625, -0.0053
    bone.roll = -0.0551
    bone.use_connect = False
    bone.parent = arm.edit_bones[bones['jaw']]
    bones['lip.B.L'] = bone.name
    bone = arm.edit_bones.new('lip.T.R')
    bone.head = 0.0000, -0.0684, 0.0030
    bone.tail = -0.0105, -0.0655, 0.0033
    bone.roll = 0.0000
    bone.use_connect = False
    bone.parent = arm.edit_bones[bones['jaw']]
    bones['lip.T.R'] = bone.name
    bone = arm.edit_bones.new('lip.B.R')
    bone.head = 0.0000, -0.0655, -0.0078
    bone.tail = -0.0107, -0.0625, -0.0053
    bone.roll = 0.0551
    bone.use_connect = False
    bone.parent = arm.edit_bones[bones['jaw']]
    bones['lip.B.R'] = bone.name
    bone = arm.edit_bones.new('teeth.B')
    bone.head = 0.0000, -0.0543, -0.0136
    bone.tail = 0.0000, -0.0237, -0.0136
    bone.roll = 0.0000
    bone.use_connect = False
    bone.parent = arm.edit_bones[bones['jaw']]
    bones['teeth.B'] = bone.name
    bone = arm.edit_bones.new('lip1.T.L')
    bone.head = 0.0105, -0.0655, 0.0033
    bone.tail = 0.0193, -0.0586, 0.0007
    bone.roll = -0.0257
    bone.use_connect = True
    bone.parent = arm.edit_bones[bones['lip.T.L']]
    bones['lip1.T.L'] = bone.name
    bone = arm.edit_bones.new('lip1.B.L')
    bone.head = 0.0107, -0.0625, -0.0053
    bone.tail = 0.0194, -0.0573, -0.0029
    bone.roll = 0.0716
    bone.use_connect = True
    bone.parent = arm.edit_bones[bones['lip.B.L']]
    bones['lip1.B.L'] = bone.name
    bone = arm.edit_bones.new('lip1.T.R')
    bone.head = -0.0105, -0.0655, 0.0033
    bone.tail = -0.0193, -0.0586, 0.0007
    bone.roll = 0.0257
    bone.use_connect = True
    bone.parent = arm.edit_bones[bones['lip.T.R']]
    bones['lip1.T.R'] = bone.name
    bone = arm.edit_bones.new('lip1.B.R')
    bone.head = -0.0107, -0.0625, -0.0053
    bone.tail = -0.0194, -0.0573, -0.0029
    bone.roll = -0.0716
    bone.use_connect = True
    bone.parent = arm.edit_bones[bones['lip.B.R']]
    bones['lip1.B.R'] = bone.name
    bone = arm.edit_bones.new('lip2.T.L')
    bone.head = 0.0193, -0.0586, 0.0007
    bone.tail = 0.0236, -0.0539, -0.0014
    bone.roll = 0.0324
    bone.use_connect = True
    bone.parent = arm.edit_bones[bones['lip1.T.L']]
    bones['lip2.T.L'] = bone.name
    bone = arm.edit_bones.new('lip2.B.L')
    bone.head = 0.0194, -0.0573, -0.0029
    bone.tail = 0.0236, -0.0539, -0.0014
    bone.roll = 0.0467
    bone.use_connect = True
    bone.parent = arm.edit_bones[bones['lip1.B.L']]
    bones['lip2.B.L'] = bone.name
    bone = arm.edit_bones.new('lip2.T.R')
    bone.head = -0.0193, -0.0586, 0.0007
    bone.tail = -0.0236, -0.0539, -0.0014
    bone.roll = -0.0324
    bone.use_connect = True
    bone.parent = arm.edit_bones[bones['lip1.T.R']]
    bones['lip2.T.R'] = bone.name
    bone = arm.edit_bones.new('lip2.B.R')
    bone.head = -0.0194, -0.0573, -0.0029
    bone.tail = -0.0236, -0.0539, -0.0014
    bone.roll = -0.0467
    bone.use_connect = True
    bone.parent = arm.edit_bones[bones['lip1.B.R']]
    bones['lip2.B.R'] = bone.name

    bpy.ops.object.mode_set(mode='OBJECT')
    pbone = obj.pose.bones[bones['jaw']]
    pbone.rigify_type = 'face.skin_jaw'
    pbone.lock_location = (False, False, False)
    pbone.lock_rotation = (False, False, False)
    pbone.lock_rotation_w = False
    pbone.lock_scale = (False, False, False)
    pbone.rotation_mode = 'QUATERNION'
    pbone = obj.pose.bones[bones['teeth.T']]
    pbone.rigify_type = 'basic.super_copy'
    pbone.lock_location = (False, False, False)
    pbone.lock_rotation = (False, False, False)
    pbone.lock_rotation_w = False
    pbone.lock_scale = (False, False, False)
    pbone.rotation_mode = 'QUATERNION'
    try:
        pbone.rigify_parameters.make_deform = False
    except AttributeError:
        pass
    try:
        pbone.rigify_parameters.super_copy_widget_type = "teeth"
    except AttributeError:
        pass
    pbone = obj.pose.bones[bones['lip.T.L']]
    pbone.rigify_type = 'skin.stretchy_chain'
    pbone.lock_location = (False, False, False)
    pbone.lock_rotation = (False, False, False)
    pbone.lock_rotation_w = False
    pbone.lock_scale = (False, False, False)
    pbone.rotation_mode = 'QUATERNION'
    try:
        pbone.rigify_parameters.bbones = 3
    except AttributeError:
        pass
    try:
        pbone.rigify_parameters.skin_chain_falloff_spherical = [True, False, True]
    except AttributeError:
        pass
    try:
        pbone.rigify_parameters.skin_chain_falloff = [0.5, 1.0, -0.5]
    except AttributeError:
        pass
    try:
        pbone.rigify_parameters.skin_chain_connect_mirror = [True, False]
    except AttributeError:
        pass
    pbone = obj.pose.bones[bones['lip.B.L']]
    pbone.rigify_type = 'skin.stretchy_chain'
    pbone.lock_location = (False, False, False)
    pbone.lock_rotation = (False, False, False)
    pbone.lock_rotation_w = False
    pbone.lock_scale = (False, False, False)
    pbone.rotation_mode = 'QUATERNION'
    try:
        pbone.rigify_parameters.bbones = 3
    except AttributeError:
        pass
    try:
        pbone.rigify_parameters.skin_chain_falloff_spherical = [True, False, True]
    except AttributeError:
        pass
    try:
        pbone.rigify_parameters.skin_chain_falloff = [0.5, 1.0, -0.5]
    except AttributeError:
        pass
    try:
        pbone.rigify_parameters.skin_chain_connect_mirror = [True, False]
    except AttributeError:
        pass
    pbone = obj.pose.bones[bones['lip.T.R']]
    pbone.rigify_type = 'skin.stretchy_chain'
    pbone.lock_location = (False, False, False)
    pbone.lock_rotation = (False, False, False)
    pbone.lock_rotation_w = False
    pbone.lock_scale = (False, False, False)
    pbone.rotation_mode = 'QUATERNION'
    try:
        pbone.rigify_parameters.bbones = 3
    except AttributeError:
        pass
    try:
        pbone.rigify_parameters.skin_chain_falloff_spherical = [True, False, True]
    except AttributeError:
        pass
    try:
        pbone.rigify_parameters.skin_chain_falloff = [0.5, 1.0, -0.5]
    except AttributeError:
        pass
    try:
        pbone.rigify_parameters.skin_chain_connect_mirror = [True, False]
    except AttributeError:
        pass
    pbone = obj.pose.bones[bones['lip.B.R']]
    pbone.rigify_type = 'skin.stretchy_chain'
    pbone.lock_location = (False, False, False)
    pbone.lock_rotation = (False, False, False)
    pbone.lock_rotation_w = False
    pbone.lock_scale = (False, False, False)
    pbone.rotation_mode = 'QUATERNION'
    try:
        pbone.rigify_parameters.bbones = 3
    except AttributeError:
        pass
    try:
        pbone.rigify_parameters.skin_chain_falloff_spherical = [True, False, True]
    except AttributeError:
        pass
    try:
        pbone.rigify_parameters.skin_chain_falloff = [0.5, 1.0, -0.5]
    except AttributeError:
        pass
    try:
        pbone.rigify_parameters.skin_chain_connect_mirror = [True, False]
    except AttributeError:
        pass
    pbone = obj.pose.bones[bones['teeth.B']]
    pbone.rigify_type = 'basic.super_copy'
    pbone.lock_location = (False, False, False)
    pbone.lock_rotation = (False, False, False)
    pbone.lock_rotation_w = False
    pbone.lock_scale = (False, False, False)
    pbone.rotation_mode = 'QUATERNION'
    try:
        pbone.rigify_parameters.super_copy_widget_type = "teeth"
    except AttributeError:
        pass
    try:
        pbone.rigify_parameters.make_deform = False
    except AttributeError:
        pass
    pbone = obj.pose.bones[bones['lip1.T.L']]
    pbone.rigify_type = ''
    pbone.lock_location = (False, False, False)
    pbone.lock_rotation = (False, False, False)
    pbone.lock_rotation_w = False
    pbone.lock_scale = (False, False, False)
    pbone.rotation_mode = 'QUATERNION'
    pbone = obj.pose.bones[bones['lip1.B.L']]
    pbone.rigify_type = ''
    pbone.lock_location = (False, False, False)
    pbone.lock_rotation = (False, False, False)
    pbone.lock_rotation_w = False
    pbone.lock_scale = (False, False, False)
    pbone.rotation_mode = 'QUATERNION'
    pbone = obj.pose.bones[bones['lip1.T.R']]
    pbone.rigify_type = ''
    pbone.lock_location = (False, False, False)
    pbone.lock_rotation = (False, False, False)
    pbone.lock_rotation_w = False
    pbone.lock_scale = (False, False, False)
    pbone.rotation_mode = 'QUATERNION'
    pbone = obj.pose.bones[bones['lip1.B.R']]
    pbone.rigify_type = ''
    pbone.lock_location = (False, False, False)
    pbone.lock_rotation = (False, False, False)
    pbone.lock_rotation_w = False
    pbone.lock_scale = (False, False, False)
    pbone.rotation_mode = 'QUATERNION'
    pbone = obj.pose.bones[bones['lip2.T.L']]
    pbone.rigify_type = ''
    pbone.lock_location = (False, False, False)
    pbone.lock_rotation = (False, False, False)
    pbone.lock_rotation_w = False
    pbone.lock_scale = (False, False, False)
    pbone.rotation_mode = 'QUATERNION'
    pbone = obj.pose.bones[bones['lip2.B.L']]
    pbone.rigify_type = ''
    pbone.lock_location = (False, False, False)
    pbone.lock_rotation = (False, False, False)
    pbone.lock_rotation_w = False
    pbone.lock_scale = (False, False, False)
    pbone.rotation_mode = 'QUATERNION'
    pbone = obj.pose.bones[bones['lip2.T.R']]
    pbone.rigify_type = ''
    pbone.lock_location = (False, False, False)
    pbone.lock_rotation = (False, False, False)
    pbone.lock_rotation_w = False
    pbone.lock_scale = (False, False, False)
    pbone.rotation_mode = 'QUATERNION'
    pbone = obj.pose.bones[bones['lip2.B.R']]
    pbone.rigify_type = ''
    pbone.lock_location = (False, False, False)
    pbone.lock_rotation = (False, False, False)
    pbone.lock_rotation_w = False
    pbone.lock_scale = (False, False, False)
    pbone.rotation_mode = 'QUATERNION'

    bpy.ops.object.mode_set(mode='EDIT')
    for bone in arm.edit_bones:
        bone.select = False
        bone.select_head = False
        bone.select_tail = False
    for b in bones:
        bone = arm.edit_bones[bones[b]]
        bone.select = True
        bone.select_head = True
        bone.select_tail = True
        bone.bbone_x = bone.bbone_z = bone.length * 0.05
        arm.edit_bones.active = bone
        if bcoll := arm.collections.active:
            bcoll.assign(bone)

    return bones


```
