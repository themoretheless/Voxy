# Facial rendering source scan: danukim/Miko

Repository: https://github.com/danukim/Miko

Pinned source revision: 78bd305f5c3f2e458f93923a0b67f9f3af2e76a1

Discovery topic: lip sync 3D

Description: Embodied 3D AI desktop companion powered by Ollama, dual voice synthesis (Fish Audio S2.1 & local GPT-SoVITS), real-time lip-sync, and custom VRM avatar support.

Reported license: GPL-3.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE", "Miko_desktop/Assets/UniGLTF/LICENSE.md", "Miko_desktop/Assets/UniGLTF/LICENSE.md.meta", "Miko_desktop/Assets/VRM/LICENSE.md", "Miko_desktop/Assets/VRM/LICENSE.md.meta", "Miko_desktop/Assets/VRMShaders/LICENSE.md", "Miko_desktop/Assets/VRMShaders/LICENSE.md.meta", "Miko_desktop/Assets/VRMShaders/VRM/MToon/LICENSE", "Miko_desktop/Assets/VRMShaders/VRM/MToon/LICENSE.meta"]

Topic paths: {"eye": ["Miko_desktop/Assets/5325421668752063897.Materials/N00_000_00_EyeHighlight_00_EYE (Instance).asset", "Miko_desktop/Assets/5325421668752063897.Materials/N00_000_00_EyeHighlight_00_EYE (Instance).asset.meta", "Miko_desktop/Assets/5325421668752063897.Materials/N00_000_00_EyeIris_00_EYE (Instance).asset", "Miko_desktop/Assets/5325421668752063897.Materials/N00_000_00_EyeIris_00_EYE (Instance).asset.meta", "Miko_desktop/Assets/5325421668752063897.Materials/N00_000_00_EyeWhite_00_EYE (Instance).asset", "Miko_desktop/Assets/5325421668752063897.Materials/N00_000_00_EyeWhite_00_EYE (Instance).asset.meta", "Miko_desktop/Assets/5325421668752063897.Materials/N00_000_00_FaceEyeline_00_FACE (Instance).asset", "Miko_desktop/Assets/5325421668752063897.Materials/N00_000_00_FaceEyeline_00_FACE (Instance).asset.meta", "Miko_desktop/Assets/7090716841735475770.Materials/N00_000_00_EyeHighlight_00_EYE (Instance).asset", "Miko_desktop/Assets/7090716841735475770.Materials/N00_000_00_EyeHighlight_00_EYE (Instance).asset.meta", "Miko_desktop/Assets/7090716841735475770.Materials/N00_000_00_EyeIris_00_EYE (Instance).asset", "Miko_desktop/Assets/7090716841735475770.Materials/N00_000_00_EyeIris_00_EYE (Instance).asset.meta", "Miko_desktop/Assets/7090716841735475770.Materials/N00_000_00_EyeWhite_00_EYE (Instance).asset", "Miko_desktop/Assets/7090716841735475770.Materials/N00_000_00_EyeWhite_00_EYE (Instance).asset.meta", "Miko_desktop/Assets/7090716841735475770.Materials/N00_000_00_FaceEyelash_00_FACE (Instance).asset", "Miko_desktop/Assets/7090716841735475770.Materials/N00_000_00_FaceEyelash_00_FACE (Instance).asset.meta", "Miko_desktop/Assets/7090716841735475770.Materials/N00_000_00_FaceEyeline_00_FACE (Instance).asset", "Miko_desktop/Assets/7090716841735475770.Materials/N00_000_00_FaceEyeline_00_FACE (Instance).asset.meta", "Miko_desktop/Assets/É¦ïeâ~âï_ÄäòP.Materials/Eye_mtl.asset", "Miko_desktop/Assets/É¦ïeâ~âï_ÄäòP.Materials/Eye_mtl.asset.meta", "Miko_desktop/Assets/É¦ïeâ~âï_ÄäòP.Textures/Eye_bcm.png", "Miko_desktop/Assets/É¦ïeâ~âï_ÄäòP.Textures/Eye_bcm.png.meta"], "skin": ["Miko_desktop/Assets/5325421668752063897.Materials/N00_000_00_Body_00_SKIN (Instance).asset", "Miko_desktop/Assets/5325421668752063897.Materials/N00_000_00_Body_00_SKIN (Instance).asset.meta", "Miko_desktop/Assets/5325421668752063897.Materials/N00_000_00_Face_00_SKIN (Instance).asset", "Miko_desktop/Assets/5325421668752063897.Materials/N00_000_00_Face_00_SKIN (Instance).asset.meta", "Miko_desktop/Assets/7090716841735475770.Materials/N00_000_00_Body_00_SKIN (Instance).asset", "Miko_desktop/Assets/7090716841735475770.Materials/N00_000_00_Body_00_SKIN (Instance).asset.meta", "Miko_desktop/Assets/7090716841735475770.Materials/N00_000_00_Face_00_SKIN (Instance).asset", "Miko_desktop/Assets/7090716841735475770.Materials/N00_000_00_Face_00_SKIN (Instance).asset.meta", "Miko_desktop/Assets/TextMesh Pro/Shaders/TMP_SDF-Mobile Masking.shader", "Miko_desktop/Assets/TextMesh Pro/Shaders/TMP_SDF-Mobile Masking.shader.meta", "Miko_desktop/Assets/UniGLTF/Runtime/UniGLTF/Format/glTFSkin.cs", "Miko_desktop/Assets/UniGLTF/Runtime/UniGLTF/Format/glTFSkin.cs.meta", "Miko_desktop/Assets/UniGLTF/Runtime/UniGLTF/IO/MeshIO/SkinningInfo.cs", "Miko_desktop/Assets/UniGLTF/Runtime/UniGLTF/IO/MeshIO/SkinningInfo.cs.meta", "Miko_desktop/Assets/UniGLTF/Runtime/UniGLTF/IO/SkinJoint.cs", "Miko_desktop/Assets/UniGLTF/Runtime/UniGLTF/IO/SkinJoint.cs.meta", "Miko_desktop/Assets/VRM/Editor/SkinnedMeshUtility.meta", "Miko_desktop/Assets/VRM/Editor/SkinnedMeshUtility/VrmBlendShapeUpdater.cs", "Miko_desktop/Assets/VRM/Editor/SkinnedMeshUtility/VrmBlendShapeUpdater.cs.meta", "Miko_desktop/Assets/VRM/Editor/SkinnedMeshUtility/VrmMeshIntegratorWizard.cs", "Miko_desktop/Assets/VRM/Editor/SkinnedMeshUtility/VrmMeshIntegratorWizard.cs.meta", "Miko_desktop/Assets/VRM/Editor/SkinnedMeshUtility/VrmPrefabUtility.cs", "Miko_desktop/Assets/VRM/Editor/SkinnedMeshUtility/VrmPrefabUtility.cs.meta", "Miko_desktop/Assets/VRM/Runtime/SkinnedMeshUtility.meta", "Miko_desktop/Assets/VRM/Runtime/SkinnedMeshUtility/VRMBoneNormalizer.cs", "Miko_desktop/Assets/VRM/Runtime/SkinnedMeshUtility/VRMBoneNormalizer.cs.meta", "Miko_desktop/Assets/VRM/Runtime/SkinnedMeshUtility/VrmMeshUtility.cs", "Miko_desktop/Assets/VRM/Runtime/SkinnedMeshUtility/VrmMeshUtility.cs.meta", "Miko_desktop/Assets/É¦ïeâ~âï_ÄäòP.Materials/BodySkin_CostumeA_mtl.asset", "Miko_desktop/Assets/É¦ïeâ~âï_ÄäòP.Materials/BodySkin_CostumeA_mtl.asset.meta"], "blendshape": ["Miko_desktop/Assets/5325421668752063897.BlendShapes.meta", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.A.asset", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.A.asset.meta", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.Angry.asset", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.Angry.asset.meta", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.Blink.asset", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.Blink.asset.meta", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.Blink_L.asset", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.Blink_L.asset.meta", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.Blink_R.asset", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.Blink_R.asset.meta", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.E.asset", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.E.asset.meta", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.Fun.asset", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.Fun.asset.meta", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.I.asset", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.I.asset.meta", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.Joy.asset", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.Joy.asset.meta", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.Neutral.asset", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.Neutral.asset.meta", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.O.asset", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.O.asset.meta", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.Sorrow.asset", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.Sorrow.asset.meta", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.Surprised.asset", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.Surprised.asset.meta", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.U.asset", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.U.asset.meta", "Miko_desktop/Assets/5325421668752063897.BlendShapes/BlendShape.asset"], "mouth": ["Miko_desktop/Assets/5325421668752063897.Materials/N00_000_00_FaceMouth_00_FACE (Instance).asset", "Miko_desktop/Assets/5325421668752063897.Materials/N00_000_00_FaceMouth_00_FACE (Instance).asset.meta", "Miko_desktop/Assets/7090716841735475770.Materials/N00_000_00_FaceMouth_00_FACE (Instance).asset", "Miko_desktop/Assets/7090716841735475770.Materials/N00_000_00_FaceMouth_00_FACE (Instance).asset.meta"], "lbs": ["Miko_desktop/Assets/UniGLTF/Editor/UniGLTF/ScriptedImporter/GlbScriptedImporter.cs", "Miko_desktop/Assets/UniGLTF/Editor/UniGLTF/ScriptedImporter/GlbScriptedImporter.cs.meta", "Miko_desktop/Assets/UniGLTF/Editor/UniGLTF/ScriptedImporter/GlbScriptedImporterEditor.cs", "Miko_desktop/Assets/UniGLTF/Editor/UniGLTF/ScriptedImporter/GlbScriptedImporterEditor.cs.meta"], "iris": ["Miko_desktop/Assets/5325421668752063897.Materials/N00_000_00_EyeIris_00_EYE (Instance).asset", "Miko_desktop/Assets/5325421668752063897.Materials/N00_000_00_EyeIris_00_EYE (Instance).asset.meta", "Miko_desktop/Assets/7090716841735475770.Materials/N00_000_00_EyeIris_00_EYE (Instance).asset", "Miko_desktop/Assets/7090716841735475770.Materials/N00_000_00_EyeIris_00_EYE (Instance).asset.meta"], "shader": ["Miko_desktop/Assets/5325421668752063897.Textures/Shader_NoneBlack.png", "Miko_desktop/Assets/5325421668752063897.Textures/Shader_NoneBlack.png.meta", "Miko_desktop/Assets/5325421668752063897.Textures/Shader_NoneNormal.normal.png", "Miko_desktop/Assets/5325421668752063897.Textures/Shader_NoneNormal.normal.png.meta", "Miko_desktop/Assets/7090716841735475770.Textures/Shader_NoneBlack.png", "Miko_desktop/Assets/7090716841735475770.Textures/Shader_NoneBlack.png.meta", "Miko_desktop/Assets/7090716841735475770.Textures/Shader_NoneNormal.normal.png", "Miko_desktop/Assets/7090716841735475770.Textures/Shader_NoneNormal.normal.png.meta", "Miko_desktop/Assets/TextMesh Pro/Shaders.meta", "Miko_desktop/Assets/TextMesh Pro/Shaders/TMP_Bitmap-Custom-Atlas.shader", "Miko_desktop/Assets/TextMesh Pro/Shaders/TMP_Bitmap-Custom-Atlas.shader.meta", "Miko_desktop/Assets/TextMesh Pro/Shaders/TMP_Bitmap-Mobile.shader", "Miko_desktop/Assets/TextMesh Pro/Shaders/TMP_Bitmap-Mobile.shader.meta", "Miko_desktop/Assets/TextMesh Pro/Shaders/TMP_Bitmap.shader", "Miko_desktop/Assets/TextMesh Pro/Shaders/TMP_Bitmap.shader.meta", "Miko_desktop/Assets/TextMesh Pro/Shaders/TMP_SDF Overlay.shader", "Miko_desktop/Assets/TextMesh Pro/Shaders/TMP_SDF Overlay.shader.meta", "Miko_desktop/Assets/TextMesh Pro/Shaders/TMP_SDF SSD.shader", "Miko_desktop/Assets/TextMesh Pro/Shaders/TMP_SDF SSD.shader.meta", "Miko_desktop/Assets/TextMesh Pro/Shaders/TMP_SDF-Mobile Masking.shader", "Miko_desktop/Assets/TextMesh Pro/Shaders/TMP_SDF-Mobile Masking.shader.meta", "Miko_desktop/Assets/TextMesh Pro/Shaders/TMP_SDF-Mobile Overlay.shader", "Miko_desktop/Assets/TextMesh Pro/Shaders/TMP_SDF-Mobile Overlay.shader.meta", "Miko_desktop/Assets/TextMesh Pro/Shaders/TMP_SDF-Mobile SSD.shader", "Miko_desktop/Assets/TextMesh Pro/Shaders/TMP_SDF-Mobile SSD.shader.meta", "Miko_desktop/Assets/TextMesh Pro/Shaders/TMP_SDF-Mobile.shader", "Miko_desktop/Assets/TextMesh Pro/Shaders/TMP_SDF-Mobile.shader.meta", "Miko_desktop/Assets/TextMesh Pro/Shaders/TMP_SDF-Surface-Mobile.shader", "Miko_desktop/Assets/TextMesh Pro/Shaders/TMP_SDF-Surface-Mobile.shader.meta", "Miko_desktop/Assets/TextMesh Pro/Shaders/TMP_SDF-Surface.shader"], "rig": ["Miko_desktop/Assets/5325421668752063897.BlendShapes/LookRight.asset", "Miko_desktop/Assets/5325421668752063897.BlendShapes/LookRight.asset.meta", "Miko_desktop/Assets/7090716841735475770.BlendShapes/LookRight.asset", "Miko_desktop/Assets/7090716841735475770.BlendShapes/LookRight.asset.meta", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Crawling_Baby_TurnRight_180.FBX", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Crawling_Baby_TurnRight_180.FBX.meta", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Crawling_Baby_TurnRight_90.FBX", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Crawling_Baby_TurnRight_90.FBX.meta", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Crawling_Baby_Walk_Right.FBX", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Crawling_Baby_Walk_Right.FBX.meta", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Crawling_Baby_Walk_Right_Pivot.FBX", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Crawling_Baby_Walk_Right_Pivot.FBX.meta", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Crawling_Baby_Walk_Right_Start.FBX", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Crawling_Baby_Walk_Right_Start.FBX.meta", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Crawling_Baby_Walk_Right_Stop.FBX", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Crawling_Baby_Walk_Right_Stop.FBX.meta", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Dash_Right.FBX", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Dash_Right.FBX.meta", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Idle02_LookLeftAndRight.FBX", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Idle02_LookLeftAndRight.FBX.meta", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Lantern_StickToTheWall_Walk_Right_Loop.FBX", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Lantern_StickToTheWall_Walk_Right_Loop.FBX.meta", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Lantern_StickToTheWall_Walk_Right_Pivot.FBX", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Lantern_StickToTheWall_Walk_Right_Pivot.FBX.meta", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Lantern_StickToTheWall_Walk_Right_Start.FBX", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Lantern_StickToTheWall_Walk_Right_Start.FBX.meta", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Lantern_StickToTheWall_Walk_Right_Stop.FBX", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Lantern_StickToTheWall_Walk_Right_Stop.FBX.meta", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Lantern_TurnRight_180.FBX", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Lantern_TurnRight_180.FBX.meta"], "face": ["Miko_desktop/Assets/5325421668752063897.Materials/N00_000_00_FaceBrow_00_FACE (Instance).asset", "Miko_desktop/Assets/5325421668752063897.Materials/N00_000_00_FaceBrow_00_FACE (Instance).asset.meta", "Miko_desktop/Assets/5325421668752063897.Materials/N00_000_00_FaceEyeline_00_FACE (Instance).asset", "Miko_desktop/Assets/5325421668752063897.Materials/N00_000_00_FaceEyeline_00_FACE (Instance).asset.meta", "Miko_desktop/Assets/5325421668752063897.Materials/N00_000_00_FaceMouth_00_FACE (Instance).asset", "Miko_desktop/Assets/5325421668752063897.Materials/N00_000_00_FaceMouth_00_FACE (Instance).asset.meta", "Miko_desktop/Assets/5325421668752063897.Materials/N00_000_00_Face_00_SKIN (Instance).asset", "Miko_desktop/Assets/5325421668752063897.Materials/N00_000_00_Face_00_SKIN (Instance).asset.meta", "Miko_desktop/Assets/5325421668752063897.Meshes/Face (merged)(Clone).baked.asset", "Miko_desktop/Assets/5325421668752063897.Meshes/Face (merged)(Clone).baked.asset.meta", "Miko_desktop/Assets/5325421668752063897.Textures/N00_000_00_Face_00_out.png", "Miko_desktop/Assets/5325421668752063897.Textures/N00_000_00_Face_00_out.png.meta", "Miko_desktop/Assets/7090716841735475770.Materials/N00_000_00_FaceBrow_00_FACE (Instance).asset", "Miko_desktop/Assets/7090716841735475770.Materials/N00_000_00_FaceBrow_00_FACE (Instance).asset.meta", "Miko_desktop/Assets/7090716841735475770.Materials/N00_000_00_FaceEyelash_00_FACE (Instance).asset", "Miko_desktop/Assets/7090716841735475770.Materials/N00_000_00_FaceEyelash_00_FACE (Instance).asset.meta", "Miko_desktop/Assets/7090716841735475770.Materials/N00_000_00_FaceEyeline_00_FACE (Instance).asset", "Miko_desktop/Assets/7090716841735475770.Materials/N00_000_00_FaceEyeline_00_FACE (Instance).asset.meta", "Miko_desktop/Assets/7090716841735475770.Materials/N00_000_00_FaceMouth_00_FACE (Instance).asset", "Miko_desktop/Assets/7090716841735475770.Materials/N00_000_00_FaceMouth_00_FACE (Instance).asset.meta", "Miko_desktop/Assets/7090716841735475770.Materials/N00_000_00_Face_00_SKIN (Instance).asset", "Miko_desktop/Assets/7090716841735475770.Materials/N00_000_00_Face_00_SKIN (Instance).asset.meta", "Miko_desktop/Assets/7090716841735475770.Meshes/Face (merged).baked.asset", "Miko_desktop/Assets/7090716841735475770.Meshes/Face (merged).baked.asset.meta", "Miko_desktop/Assets/7090716841735475770.Textures/N00_000_00_Face_00_out.png", "Miko_desktop/Assets/7090716841735475770.Textures/N00_000_00_Face_00_out.png.meta", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Sleep_FaceDown.FBX", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Sleep_FaceDown.FBX.meta", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Sleep_FaceDown_End.FBX", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Sleep_FaceDown_End.FBX.meta"], "lip": ["Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Idle55_Backflip.FBX", "Miko_desktop/Assets/KAWAII_ANIMATIOMS_100/Assets/Animations/@KA_Idle55_Backflip.FBX.meta", "Miko_desktop/Assets/LipSyncController.cs", "Miko_desktop/Assets/LipSyncController.cs.meta", "Miko_desktop/Assets/UniGLTF/Runtime/UniHumanoid/AnimationClipUtility.cs", "Miko_desktop/Assets/UniGLTF/Runtime/UniHumanoid/AnimationClipUtility.cs.meta", "Miko_desktop/Assets/UniGLTF/Runtime/UniHumanoid/HumanPoseClip.cs", "Miko_desktop/Assets/UniGLTF/Runtime/UniHumanoid/HumanPoseClip.cs.meta", "Miko_desktop/Assets/UniGLTF/Runtime/UniHumanoid/IO/BvhAnimationClip.cs", "Miko_desktop/Assets/UniGLTF/Runtime/UniHumanoid/IO/BvhAnimationClip.cs.meta", "Miko_desktop/Assets/VRM/Editor/BlendShape/BlendShapeClipDrawer.cs", "Miko_desktop/Assets/VRM/Editor/BlendShape/BlendShapeClipDrawer.cs.meta", "Miko_desktop/Assets/VRM/Editor/BlendShape/BlendShapeClipEditor.cs", "Miko_desktop/Assets/VRM/Editor/BlendShape/BlendShapeClipEditor.cs.meta", "Miko_desktop/Assets/VRM/Editor/BlendShape/BlendShapeClipEditorHelper.cs", "Miko_desktop/Assets/VRM/Editor/BlendShape/BlendShapeClipEditorHelper.cs.meta", "Miko_desktop/Assets/VRM/Editor/BlendShape/BlendShapeClipSelector.cs", "Miko_desktop/Assets/VRM/Editor/BlendShape/BlendShapeClipSelector.cs.meta", "Miko_desktop/Assets/VRM/Editor/BlendShape/SerializedBlendShapeClipEditor.cs", "Miko_desktop/Assets/VRM/Editor/BlendShape/SerializedBlendShapeClipEditor.cs.meta", "Miko_desktop/Assets/VRM/Runtime/BlendShape/BlendShapeClip.cs", "Miko_desktop/Assets/VRM/Runtime/BlendShape/BlendShapeClip.cs.meta", "Miko_desktop/Assets/VRM/Runtime/BlendShape/BlendShapeClipHandler.cs", "Miko_desktop/Assets/VRM/Runtime/BlendShape/BlendShapeClipHandler.cs.meta", "Miko_desktop/Assets/VRMShaders/GLTF/IO/Runtime/Animation/Importer/AnimationClipFactory.cs", "Miko_desktop/Assets/VRMShaders/GLTF/IO/Runtime/Animation/Importer/AnimationClipFactory.cs.meta", "Miko_desktop/Assets/uLipSync.meta", "Miko_desktop/Assets/uLipSync/Assets.meta", "Miko_desktop/Assets/uLipSync/Assets/Profiles.meta", "Miko_desktop/Assets/uLipSync/Assets/Profiles/uLipSync-Profile-Sample-Female.asset"]}

## Source file: Miko_desktop/Assets/uLipSync/Runtime/uLipSyncBlendShape.cs

{
  "path": "Miko_desktop/Assets/uLipSync/Runtime/uLipSyncBlendShape.cs",
  "blob_sha": "3a9a2fe96b6ca8266c37be8f0b630ec5e34d53c5",
  "sha256": "1bd764d3c532c6b7cfecc457184188ae88cb41965a41b0ad61d1e9bed41f2b52",
  "lines": 256,
  "bytes": 6447,
  "selection_score": 30,
  "keyword_line_counts": {
    "skin": 9,
    "blendshape": 28,
    "lip": 16
  },
  "url": "https://github.com/danukim/Miko/blob/78bd305f5c3f2e458f93923a0b67f9f3af2e76a1/Miko_desktop/Assets/uLipSync/Runtime/uLipSyncBlendShape.cs",
  "function_signature_candidates": [
    {
      "line": 40,
      "text": "void UpdateLipSync()"
    },
    {
      "line": 47,
      "text": "public void OnLipSyncUpdate(LipSyncInfo info)"
    },
    {
      "line": 58,
      "text": "void Update()"
    },
    {
      "line": 74,
      "text": "void LateUpdate()"
    },
    {
      "line": 85,
      "text": "void FixedUpdate()"
    },
    {
      "line": 96,
      "text": "float SmoothDamp(float value, float target, ref float velocity)"
    },
    {
      "line": 107,
      "text": "void UpdateVolume()"
    },
    {
      "line": 119,
      "text": "void UpdateVowels()"
    },
    {
      "line": 150,
      "text": "public void ApplyBlendShapes()"
    },
    {
      "line": 158,
      "text": "protected virtual void OnApplyBlendShapes()"
    },
    {
      "line": 235,
      "text": "public override void OnAnimationBakeStart()"
    },
    {
      "line": 240,
      "text": "public override void OnAnimationBakeUpdate(LipSyncInfo info, float dt)"
    },
    {
      "line": 248,
      "text": "public override void OnAnimationBakeEnd()"
    }
  ]
}

```text

using UnityEngine;
using System.Collections.Generic;

namespace uLipSync
{

[ExecuteAlways]
public class uLipSyncBlendShape : AnimationBakableMonoBehaviour
{
    [System.Serializable]
    public class BlendShapeInfo
    {
        public string phoneme;
        public int index = -1;
        public float maxWeight = 1f;

        public float weight { get; set; } = 0f;
        public float weightVelocity { get; set; } = 0f;
    }

    public UpdateMethod updateMethod = UpdateMethod.LateUpdate;
    public SkinnedMeshRenderer skinnedMeshRenderer;
    public List<BlendShapeInfo> blendShapes = new List<BlendShapeInfo>();
    public float minVolume = -2.5f;
    public float maxVolume = -1.5f;
    [Range(0f, 0.3f)] public float smoothness = 0.05f;
    public bool usePhonemeBlend = false;

    LipSyncInfo _info = new LipSyncInfo();
    bool _lipSyncUpdated = false;
    float _volume = 0f;
    float _openCloseVelocity = 0f;
    protected float volume => _volume;

#if UNITY_EDITOR
    bool _isAnimationBaking = false;
    float _animBakeDeltaTime = 1f / 60;
#endif

    void UpdateLipSync()
    {
        UpdateVolume();
        UpdateVowels();
        _lipSyncUpdated = false;
    }

    public void OnLipSyncUpdate(LipSyncInfo info)
    {
        _info = info;
        _lipSyncUpdated = true;
        if (updateMethod == UpdateMethod.LipSyncUpdateEvent)
        {
            UpdateLipSync();
            OnApplyBlendShapes();
        }
    }

    void Update()
    {
#if UNITY_EDITOR
        if (_isAnimationBaking) return;
#endif
        if (updateMethod != UpdateMethod.LipSyncUpdateEvent)
        {
            UpdateLipSync();
        }

        if (updateMethod == UpdateMethod.Update)
        {
            OnApplyBlendShapes();
        }
    }

    void LateUpdate()
    {
#if UNITY_EDITOR
        if (_isAnimationBaking) return;
#endif
        if (updateMethod == UpdateMethod.LateUpdate)
        {
            OnApplyBlendShapes();
        }
    }

    void FixedUpdate()
    {
#if UNITY_EDITOR
        if (_isAnimationBaking) return;
#endif
        if (updateMethod == UpdateMethod.FixedUpdate)
        {
            OnApplyBlendShapes();
        }
    }

    float SmoothDamp(float value, float target, ref float velocity)
    {
#if UNITY_EDITOR
        if (_isAnimationBaking)
        {
            return Mathf.SmoothDamp(value, target, ref velocity, smoothness, Mathf.Infinity, _animBakeDeltaTime);
        }
#endif
        return Mathf.SmoothDamp(value, target, ref velocity, smoothness);
    }

    void UpdateVolume()
    {
        float normVol = 0f;
        if (_lipSyncUpdated && _info.rawVolume > 0f)
        {
            normVol = Mathf.Log10(_info.rawVolume);
            normVol = (normVol - minVolume) / Mathf.Max(maxVolume - minVolume, 1e-4f);
            normVol = Mathf.Clamp(normVol, 0f, 1f);
        }
        _volume = SmoothDamp(_volume, normVol, ref _openCloseVelocity);
    }

    void UpdateVowels()
    {
        float sum = 0f;
        var ratios = _info.phonemeRatios;

        foreach (var bs in blendShapes)
        {
            float targetWeight = 0f;
            if (usePhonemeBlend)
            {
                if (ratios != null && !string.IsNullOrEmpty(bs.phoneme))
                {
                    ratios.TryGetValue(bs.phoneme, out targetWeight);
                }
            }
            else
            {
                targetWeight = (bs.phoneme == _info.phoneme) ? 1f : 0f;
            }
            float weightVel = bs.weightVelocity;
            bs.weight = SmoothDamp(bs.weight, targetWeight, ref weightVel);
            bs.weightVelocity = weightVel;
            sum += bs.weight;
        }

        foreach (var bs in blendShapes)
        {
            bs.weight = sum > 0f ? bs.weight / sum : 0f;
        }
    }

    public void ApplyBlendShapes()
    {
        if (updateMethod == UpdateMethod.External)
        {
            OnApplyBlendShapes();
        }
    }
    
    protected virtual void OnApplyBlendShapes()
    {
        if (!skinnedMeshRenderer) return;

        foreach (var bs in blendShapes)
        {
            if (bs.index < 0) continue;
            skinnedMeshRenderer.SetBlendShapeWeight(bs.index, 0f);
        }

        foreach (var bs in blendShapes)
        {
            if (bs.index < 0) continue;
            float weight = skinnedMeshRenderer.GetBlendShapeWeight(bs.index);
            weight += bs.weight * bs.maxWeight * volume * 100;
            skinnedMeshRenderer.SetBlendShapeWeight(bs.index, weight);
        }
    }

    public BlendShapeInfo GetBlendShapeInfo(string phoneme)
    {
        foreach (var info in blendShapes)
        {
            if (info.phoneme == phoneme) return info;
        }
        return null;
    }

    public BlendShapeInfo AddBlendShape(string phoneme, string blendShape)
    {
        var bs = GetBlendShapeInfo(phoneme);
        if (bs == null) bs = new BlendShapeInfo() { phoneme = phoneme };

        blendShapes.Add(bs);

        if (!skinnedMeshRenderer) return bs;
        bs.index = Util.GetBlendShapeIndex(skinnedMeshRenderer, blendShape);

        return bs;
    }

#if UNITY_EDITOR
    public override GameObject target => skinnedMeshRenderer?.gameObject;

    public override List<string> GetPropertyNames()
    {
        var names = new List<string>();
        var mesh = skinnedMeshRenderer.sharedMesh;

        foreach (var bs in blendShapes)
        {
            if (bs.index < 0) continue;
            var name = mesh.GetBlendShapeName(bs.index);
            name = "blendShape." + name;
            names.Add(name);
        }

        return names;
    }

    public override List<float> GetPropertyWeights()
    {
        var weights = new List<float>();

        foreach (var bs in blendShapes)
        {
            if (bs.index < 0) continue;
            var weight = bs.weight * bs.maxWeight * volume * 100f;
            weights.Add(weight);
        }

        return weights;
    }

    public override float maxWeight => 100f;
    public override float minWeight => 0f;

    public override void OnAnimationBakeStart()
    {
        _isAnimationBaking = true;
    }

    public override void OnAnimationBakeUpdate(LipSyncInfo info, float dt)
    {
        _info = info;
        _animBakeDeltaTime = dt;
        _lipSyncUpdated = true;
        UpdateLipSync();
    }

    public override void OnAnimationBakeEnd()
    {
        _isAnimationBaking = false;
    }
#endif
}

}



```

## Source file: Miko_desktop/Assets/TextMesh Pro/Shaders/TMPro_Surface.cginc

{
  "path": "Miko_desktop/Assets/TextMesh Pro/Shaders/TMPro_Surface.cginc",
  "blob_sha": "622ae8758361df57d669eb6f693c1c70ba0e21b0",
  "sha256": "434adb97dfcd9d5772fc11bb3a76f36af13614a664aca840ebcbf635e6e2df24",
  "lines": 101,
  "bytes": 3563,
  "selection_score": 30,
  "keyword_line_counts": {
    "shader": 2,
    "face": 19,
    "lip": 1
  },
  "url": "https://github.com/danukim/Miko/blob/78bd305f5c3f2e458f93923a0b67f9f3af2e76a1/Miko_desktop/Assets/TextMesh%20Pro/Shaders/TMPro_Surface.cginc",
  "function_signature_candidates": [
    {
      "line": 1,
      "text": "void VertShader(inout appdata_full v, out Input data)"
    },
    {
      "line": 34,
      "text": "void PixShader(Input input, inout SurfaceOutput o)"
    }
  ]
}

```text

void VertShader(inout appdata_full v, out Input data)
{
	v.vertex.x += _VertexOffsetX;
	v.vertex.y += _VertexOffsetY;

	UNITY_INITIALIZE_OUTPUT(Input, data);

	float bold = step(v.texcoord1.y, 0);

	// Generate normal for backface
	float3 view = ObjSpaceViewDir(v.vertex);
	v.normal *= sign(dot(v.normal, view));

#if USE_DERIVATIVE
	data.param.y = 1;
#else
	float4 vert = v.vertex;
	float4 vPosition = UnityObjectToClipPos(vert);
	float2 pixelSize = vPosition.w;

	pixelSize /= float2(_ScaleX, _ScaleY) * mul((float2x2)UNITY_MATRIX_P, _ScreenParams.xy);
	float scale = rsqrt(dot(pixelSize, pixelSize));
	scale *= abs(v.texcoord1.y) * _GradientScale * (_Sharpness + 1);
	scale = lerp(scale * (1 - _PerspectiveFilter), scale, abs(dot(UnityObjectToWorldNormal(v.normal.xyz), normalize(WorldSpaceViewDir(vert)))));
	data.param.y = scale;
#endif

	data.param.x = (lerp(_WeightNormal, _WeightBold, bold) / 4.0 + _FaceDilate) * _ScaleRatioA * 0.5; // 

	v.texcoord1.xy = UnpackUV(v.texcoord1.x);
	data.viewDirEnv = mul((float3x3)_EnvMatrix, WorldSpaceViewDir(v.vertex));
}

void PixShader(Input input, inout SurfaceOutput o)
{

#if USE_DERIVATIVE
	float2 pixelSize = float2(ddx(input.uv_MainTex.y), ddy(input.uv_MainTex.y));
	pixelSize *= _TextureWidth * .75;
	float scale = rsqrt(dot(pixelSize, pixelSize)) * _GradientScale * (_Sharpness + 1);
#else
	float scale = input.param.y;
#endif

	// Signed distance
	float c = tex2D(_MainTex, input.uv_MainTex).a;
	float sd = (.5 - c - input.param.x) * scale + .5;
	float outline = _OutlineWidth*_ScaleRatioA * scale;
	float softness = _OutlineSoftness*_ScaleRatioA * scale;

	// Color & Alpha
	float4 faceColor = _FaceColor;
	float4 outlineColor = _OutlineColor;
	faceColor *= input.color;
	outlineColor.a *= input.color.a;
	faceColor *= tex2D(_FaceTex, float2(input.uv2_FaceTex.x + _FaceUVSpeedX * _Time.y, input.uv2_FaceTex.y + _FaceUVSpeedY * _Time.y));
	outlineColor *= tex2D(_OutlineTex, float2(input.uv2_OutlineTex.x + _OutlineUVSpeedX * _Time.y, input.uv2_OutlineTex.y + _OutlineUVSpeedY * _Time.y));
	faceColor = GetColor(sd, faceColor, outlineColor, outline, softness);
	faceColor.rgb /= max(faceColor.a, 0.0001);

#if BEVEL_ON
	float3 delta = float3(1.0 / _TextureWidth, 1.0 / _TextureHeight, 0.0);

	float4 smp4x = {tex2D(_MainTex, input.uv_MainTex - delta.xz).a,
					tex2D(_MainTex, input.uv_MainTex + delta.xz).a,
					tex2D(_MainTex, input.uv_MainTex - delta.zy).a,
					tex2D(_MainTex, input.uv_MainTex + delta.zy).a };

	// Face Normal
	float3 n = GetSurfaceNormal(smp4x, input.param.x);

	// Bumpmap
	float3 bump = UnpackNormal(tex2D(_BumpMap, input.uv2_FaceTex.xy)).xyz;
	bump *= lerp(_BumpFace, _BumpOutline, saturate(sd + outline * 0.5));
	bump = lerp(float3(0, 0, 1), bump, faceColor.a);
	n = normalize(n - bump);

	// Cubemap reflection
	fixed4 reflcol = texCUBE(_Cube, reflect(input.viewDirEnv, mul((float3x3)unity_ObjectToWorld, n)));
	float3 emission = reflcol.rgb * lerp(_ReflectFaceColor.rgb, _ReflectOutlineColor.rgb, saturate(sd + outline * 0.5)) * faceColor.a;
#else
	float3 n = float3(0, 0, -1);
	float3 emission = float3(0, 0, 0);
#endif
	
#if GLOW_ON
	float4 glowColor = GetGlowColor(sd, scale);
	glowColor.a *= input.color.a;
	emission += glowColor.rgb*glowColor.a;
	faceColor = BlendARGB(glowColor, faceColor);
	faceColor.rgb /= max(faceColor.a, 0.0001);
#endif

	// Set Standard output structure
	o.Albedo = faceColor.rgb;
	o.Normal = -n;
	o.Emission = emission;
	o.Specular = lerp(_FaceShininess, _OutlineShininess, saturate(sd + outline * 0.5));
	o.Gloss = 1;
	o.Alpha = faceColor.a;
}


```

## Source file: Miko_desktop/Assets/VRM/Runtime/BlendShape/BlendShapeClip.cs

{
  "path": "Miko_desktop/Assets/VRM/Runtime/BlendShape/BlendShapeClip.cs",
  "blob_sha": "8eff7633fcb83055098133952d0c21730ee55023",
  "sha256": "d7c01676a7b8e50180038df04fac705650eebf8c0d6759091d12ebceed36a0a3",
  "lines": 191,
  "bytes": 6280,
  "selection_score": 30,
  "keyword_line_counts": {
    "blendshape": 15,
    "lip": 6
  },
  "url": "https://github.com/danukim/Miko/blob/78bd305f5c3f2e458f93923a0b67f9f3af2e76a1/Miko_desktop/Assets/VRM/Runtime/BlendShape/BlendShapeClip.cs",
  "function_signature_candidates": [
    {
      "line": 24,
      "text": "public bool Equals(BlendShapeBinding other)"
    },
    {
      "line": 29,
      "text": "public override bool Equals(object obj)"
    },
    {
      "line": 35,
      "text": "public override int GetHashCode()"
    },
    {
      "line": 55,
      "text": "public bool Equals(MaterialValueBinding other)"
    },
    {
      "line": 60,
      "text": "public override bool Equals(object obj)"
    },
    {
      "line": 66,
      "text": "public override int GetHashCode()"
    },
    {
      "line": 179,
      "text": "public void CopyFrom(BlendShapeClip src)"
    }
  ]
}

```text

﻿using System;
using System.IO;
using System.Linq;
using UnityEngine;
#if UNITY_EDITOR
using UnityEditor;
#endif


namespace VRM
{
    [Serializable]
    public struct BlendShapeBinding : IEquatable<BlendShapeBinding>
    {
        public String RelativePath;
        public int Index;
        public float Weight;

        public override string ToString()
        {
            return string.Format("{0}[{1}]=>{2}", RelativePath, Index, Weight);
        }

        public bool Equals(BlendShapeBinding other)
        {
            return string.Equals(RelativePath, other.RelativePath) && Index == other.Index && Weight.Equals(other.Weight);
        }

        public override bool Equals(object obj)
        {
            if (ReferenceEquals(null, obj)) return false;
            return obj is BlendShapeBinding && Equals((BlendShapeBinding)obj);
        }

        public override int GetHashCode()
        {
            unchecked
            {
                var hashCode = (RelativePath != null ? RelativePath.GetHashCode() : 0);
                hashCode = (hashCode * 397) ^ Index;
                hashCode = (hashCode * 397) ^ Weight.GetHashCode();
                return hashCode;
            }
        }
    }

    [Serializable]
    public struct MaterialValueBinding : IEquatable<MaterialValueBinding>
    {
        public String MaterialName;
        public String ValueName;
        public Vector4 TargetValue;
        public Vector4 BaseValue;

        public bool Equals(MaterialValueBinding other)
        {
            return string.Equals(MaterialName, other.MaterialName) && string.Equals(ValueName, other.ValueName) && TargetValue.Equals(other.TargetValue) && BaseValue.Equals(other.BaseValue);
        }

        public override bool Equals(object obj)
        {
            if (ReferenceEquals(null, obj)) return false;
            return obj is MaterialValueBinding && Equals((MaterialValueBinding)obj);
        }

        public override int GetHashCode()
        {
            unchecked
            {
                var hashCode = (MaterialName != null ? MaterialName.GetHashCode() : 0);
                hashCode = (hashCode * 397) ^ (ValueName != null ? ValueName.GetHashCode() : 0);
                hashCode = (hashCode * 397) ^ TargetValue.GetHashCode();
                hashCode = (hashCode * 397) ^ BaseValue.GetHashCode();
                return hashCode;
            }
        }
    }

    [CreateAssetMenu(menuName = "VRM/BlendShapeClip")]
    public class BlendShapeClip : ScriptableObject
    {
#if UNITY_EDITOR
        /// <summary>
        /// Inspector preview 用の prefab をがんばってサーチする
        /// </summary>
        /// <param name="target"></param>
        /// <returns></returns>
        public static GameObject VrmPrefabSearch(UnityEngine.Object target)
        {
            var assetPath = AssetDatabase.GetAssetPath(target);
            if (string.IsNullOrEmpty(assetPath))
            {
                return null;
            }

            var prefab = AssetDatabase.LoadAssetAtPath<GameObject>(assetPath);
            // once more, with string-based method
            if (prefab == null)
            {
                var parent = UniGLTF.UnityPath.FromUnityPath(assetPath).Parent;
                var prefabPath = parent.Parent.Child(parent.FileNameWithoutExtension + ".prefab");
                prefab = UnityEditor.AssetDatabase.LoadAssetAtPath<GameObject>(prefabPath.Value);
            }
            // once more, with string-based method. search same folder *.prefab
            if (prefab == null)
            {
                var parent = UniGLTF.UnityPath.FromUnityPath(assetPath).Parent;
                foreach (var file in Directory.EnumerateFiles(parent.FullPath))
                {
                    var ext = Path.GetExtension(file).ToLower();
                    if (ext == ".prefab")
                    {
                        var prefabPath = UniGLTF.UnityPath.FromFullpath(file);
                        prefab = UnityEditor.AssetDatabase.LoadAssetAtPath<GameObject>(prefabPath.Value);
                        break;
                    }
                }
            }
            return prefab;
        }

        /// <summary>
        /// Preview 用のObject参照
        /// </summary>
        [SerializeField]
        GameObject m_prefab;
        public GameObject Prefab
        {
            set { m_prefab = value; }
            get
            {
                if (m_prefab == null)
                {
                    m_prefab = VrmPrefabSearch(this);
                }
                return m_prefab;
            }
        }
#endif

        /// <summary>
        /// BlendShapePresetがUnknown場合の識別子
        /// </summary>
        [SerializeField]
        public string BlendShapeName = "";

        /// <summary>
        /// BlendShapePresetを識別する。Unknownの場合は、BlendShapeNameで識別する
        /// </summary>
        [SerializeField]
        public BlendShapePreset Preset;

        /// <summary>
        /// BlendShapeClipに対応するBlendShapeKey
        /// </summary>
        /// <returns></returns>
        public BlendShapeKey Key => BlendShapeKey.CreateFromClip(this);

        /// <summary>
        /// BlendShapeに対する参照(index ベース)
        /// </summary>
        /// <value></value>
        [SerializeField]
        public BlendShapeBinding[] Values = new BlendShapeBinding[] { };

        /// <summary>
        /// マテリアルに対する参照(名前ベース)
        /// </summary>
        /// <value></value>
        [SerializeField]
        public MaterialValueBinding[] MaterialValues = new MaterialValueBinding[] { };

        /// <summary>
        /// UniVRM-0.45: trueの場合、このBlendShapeClipは0と1の間の中間値を取らない。四捨五入する
        /// </summary>
        [SerializeField]
        public bool IsBinary;

        public void CopyFrom(BlendShapeClip src)
        {
            IsBinary = src.IsBinary;
            MaterialValues = src.MaterialValues.ToArray();
            Values = src.Values.ToArray();
            Preset = src.Preset;
            name = src.name;
#if UNITY_EDITOR
            Prefab = src.Prefab;
#endif
        }
    }
}


```

## Source file: Miko_desktop/Assets/TextMesh Pro/Shaders/TMP_SDF-Surface.shader

{
  "path": "Miko_desktop/Assets/TextMesh Pro/Shaders/TMP_SDF-Surface.shader",
  "blob_sha": "bcb2bb273ae9140abb2adeef54b6c5a0f4d3880a",
  "sha256": "a1ec7f5beb83b6c2f00570246f542cc3c9d12516e69b4a2c39d58675b5b76d15",
  "lines": 158,
  "bytes": 4355,
  "selection_score": 30,
  "keyword_line_counts": {
    "shader": 6,
    "face": 15,
    "lip": 3
  },
  "url": "https://github.com/danukim/Miko/blob/78bd305f5c3f2e458f93923a0b67f9f3af2e76a1/Miko_desktop/Assets/TextMesh%20Pro/Shaders/TMP_SDF-Surface.shader",
  "function_signature_candidates": [
    {
      "line": 146,
      "text": "float4 frag(v2f i) : COLOR"
    }
  ]
}

```text

Shader "TextMeshPro/Distance Field (Surface)" {

Properties {
	_FaceTex			("Fill Texture", 2D) = "white" {}
	_FaceUVSpeedX		("Face UV Speed X", Range(-5, 5)) = 0.0
	_FaceUVSpeedY		("Face UV Speed Y", Range(-5, 5)) = 0.0
	[HDR]_FaceColor		("Fill Color", Color) = (1,1,1,1)
	_FaceDilate			("Face Dilate", Range(-1,1)) = 0

	[HDR]_OutlineColor	("Outline Color", Color) = (0,0,0,1)
	_OutlineTex			("Outline Texture", 2D) = "white" {}
	_OutlineUVSpeedX	("Outline UV Speed X", Range(-5, 5)) = 0.0
	_OutlineUVSpeedY	("Outline UV Speed Y", Range(-5, 5)) = 0.0
	_OutlineWidth		("Outline Thickness", Range(0, 1)) = 0
	_OutlineSoftness	("Outline Softness", Range(0,1)) = 0

	_Bevel				("Bevel", Range(0,1)) = 0.5
	_BevelOffset		("Bevel Offset", Range(-0.5,0.5)) = 0
	_BevelWidth			("Bevel Width", Range(-.5,0.5)) = 0
	_BevelClamp			("Bevel Clamp", Range(0,1)) = 0
	_BevelRoundness		("Bevel Roundness", Range(0,1)) = 0

	_BumpMap 			("Normalmap", 2D) = "bump" {}
	_BumpOutline		("Bump Outline", Range(0,1)) = 0.5
	_BumpFace			("Bump Face", Range(0,1)) = 0.5

	_ReflectFaceColor	    ("Face Color", Color) = (0,0,0,1)
	_ReflectOutlineColor	("Outline Color", Color) = (0,0,0,1)
	_Cube 					("Reflection Cubemap", Cube) = "black" { /* TexGen CubeReflect */ }
	_EnvMatrixRotation  	("Texture Rotation", vector) = (0, 0, 0, 0)
	[HDR]_SpecColor		    ("Specular Color", Color) = (0,0,0,1)

	_FaceShininess		("Face Shininess", Range(0,1)) = 0
	_OutlineShininess	("Outline Shininess", Range(0,1)) = 0

	[HDR]_GlowColor		("Color", Color) = (0, 1, 0, 0.5)
	_GlowOffset			("Offset", Range(-1,1)) = 0
	_GlowInner			("Inner", Range(0,1)) = 0.05
	_GlowOuter			("Outer", Range(0,1)) = 0.05
	_GlowPower			("Falloff", Range(1, 0)) = 0.75

	_WeightNormal		("Weight Normal", float) = 0
	_WeightBold			("Weight Bold", float) = 0.5

	// Should not be directly exposed to the user
	_ShaderFlags		("Flags", float) = 0
	_ScaleRatioA		("Scale RatioA", float) = 1
	_ScaleRatioB		("Scale RatioB", float) = 1
	_ScaleRatioC		("Scale RatioC", float) = 1

	_MainTex			("Font Atlas", 2D) = "white" {}
	_TextureWidth		("Texture Width", float) = 512
	_TextureHeight		("Texture Height", float) = 512
	_GradientScale		("Gradient Scale", float) = 5.0
	_ScaleX				("Scale X", float) = 1.0
	_ScaleY				("Scale Y", float) = 1.0
	_PerspectiveFilter	("Perspective Correction", Range(0, 1)) = 0.875
	_Sharpness			("Sharpness", Range(-1,1)) = 0

	_VertexOffsetX		("Vertex OffsetX", float) = 0
	_VertexOffsetY		("Vertex OffsetY", float) = 0

	_CullMode			("Cull Mode", Float) = 0
	//_MaskCoord		("Mask Coords", vector) = (0,0,0,0)
	//_MaskSoftness		("Mask Softness", float) = 0
}

SubShader {

	Tags { "Queue"="Transparent" "IgnoreProjector"="True" "RenderType"="Transparent" }

	LOD 300
	Cull [_CullMode]

	CGPROGRAM
	#pragma surface PixShader BlinnPhong alpha:blend vertex:VertShader nolightmap nodirlightmap
	#pragma target 3.0
	#pragma shader_feature __ GLOW_ON
	#pragma glsl

	#include "TMPro_Properties.cginc"
	#include "TMPro.cginc"

	half _FaceShininess;
	half _OutlineShininess;

	struct Input
	{
		fixed4	color			: COLOR;
		float2	uv_MainTex;
		float2	uv2_FaceTex;
		float2  uv2_OutlineTex;
		float2	param;						// Weight, Scale
		float3	viewDirEnv;
	};


	#define BEVEL_ON 1
	#include "TMPro_Surface.cginc"

	ENDCG

	// Pass to render object as a shadow caster
	Pass
	{
		Name "Caster"
		Tags { "LightMode" = "ShadowCaster" }
		Offset 1, 1

		Fog {Mode Off}
		ZWrite On
		ZTest LEqual
		Cull Off

		CGPROGRAM
		#pragma vertex vert
		#pragma fragment frag
		#pragma multi_compile_shadowcaster
		#include "UnityCG.cginc"

		struct v2f {
			V2F_SHADOW_CASTER;
			float2	uv			: TEXCOORD1;
			float2	uv2			: TEXCOORD3;
			float	alphaClip	: TEXCOORD2;
		};

		uniform float4 _MainTex_ST;
		uniform float4 _OutlineTex_ST;
		float _OutlineWidth;
		float _FaceDilate;
		float _ScaleRatioA;

		v2f vert( appdata_base v )
		{
			v2f o;
			TRANSFER_SHADOW_CASTER(o)
			o.uv = TRANSFORM_TEX(v.texcoord, _MainTex);
			o.uv2 = TRANSFORM_TEX(v.texcoord, _OutlineTex);
			o.alphaClip = (1.0 - _OutlineWidth * _ScaleRatioA - _FaceDilate * _ScaleRatioA) / 2;
			return o;
		}

		uniform sampler2D _MainTex;

		float4 frag(v2f i) : COLOR
		{
			fixed4 texcol = tex2D(_MainTex, i.uv).a;
			clip(texcol.a - i.alphaClip);
			SHADOW_CASTER_FRAGMENT(i)
		}
		ENDCG
	}
}

CustomEditor "TMPro.EditorUtilities.TMP_SDFShaderGUI"
}



```
