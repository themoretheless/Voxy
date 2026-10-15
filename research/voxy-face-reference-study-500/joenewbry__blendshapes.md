# Facial rendering source scan: joenewbry/blendshapes

Repository: https://github.com/joenewbry/blendshapes

Pinned source revision: fa8dc0f91b4933c5cd841c6498976346878d2aea

Discovery topic: ARKit blendshape

Description: Blendshapes collection from ARKit

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE/LICENSE.txt"]

Topic paths: {"blendshape": ["ARKitFaceExample/Face Overlay Content/BlendShapeCharacter.swift", "ARKitFaceExample/Resources/Assets.xcassets/blendShapes.imageset/Contents.json", "ARKitFaceExample/Resources/Assets.xcassets/blendShapes.imageset/blendShapes-1.png", "ARKitFaceExample/Resources/Assets.xcassets/blendShapes.imageset/blendShapes.png"], "shader": ["ARKitFaceExample/Resources/VideoTexturedFace.shader"], "rig": ["ARKitFaceExample/Resources/Models.scnassets/coordinateOrigin.scn"], "face": ["ARFaceGeometry.obj", "ARKitFaceExample.xcodeproj/.xcodesamplecode.plist", "ARKitFaceExample.xcodeproj/project.pbxproj", "ARKitFaceExample.xcodeproj/project.xcworkspace/xcshareddata/WorkspaceSettings.xcsettings", "ARKitFaceExample.xcodeproj/xcshareddata/xcschemes/ARKitFaceExample.xcscheme", "ARKitFaceExample/AppDelegate.swift", "ARKitFaceExample/Base.lproj/Main.storyboard", "ARKitFaceExample/Face Overlay Content/BlendShapeCharacter.swift", "ARKitFaceExample/Face Overlay Content/FaceOcclusionOverlay.swift", "ARKitFaceExample/Face Overlay Content/TexturedFace.swift", "ARKitFaceExample/Face Overlay Content/TransformVisualization.swift", "ARKitFaceExample/Face Overlay Content/VideoTexturedFace.swift", "ARKitFaceExample/Face Overlay Content/VirtualContent.swift", "ARKitFaceExample/Resources/Assets.xcassets/AppIcon.appiconset/Contents.json", "ARKitFaceExample/Resources/Assets.xcassets/AppIcon.appiconset/Icon-40.png", "ARKitFaceExample/Resources/Assets.xcassets/AppIcon.appiconset/Icon-40@2x.png", "ARKitFaceExample/Resources/Assets.xcassets/AppIcon.appiconset/Icon-40@3x.png", "ARKitFaceExample/Resources/Assets.xcassets/AppIcon.appiconset/Icon-60@2x.png", "ARKitFaceExample/Resources/Assets.xcassets/AppIcon.appiconset/Icon-60@3x.png", "ARKitFaceExample/Resources/Assets.xcassets/AppIcon.appiconset/Icon-72.png", "ARKitFaceExample/Resources/Assets.xcassets/AppIcon.appiconset/Icon-72@2x.png", "ARKitFaceExample/Resources/Assets.xcassets/AppIcon.appiconset/Icon-76.png", "ARKitFaceExample/Resources/Assets.xcassets/AppIcon.appiconset/Icon-76@2x.png", "ARKitFaceExample/Resources/Assets.xcassets/AppIcon.appiconset/Icon-83.5@2x.png", "ARKitFaceExample/Resources/Assets.xcassets/AppIcon.appiconset/Icon-Small-50.png", "ARKitFaceExample/Resources/Assets.xcassets/AppIcon.appiconset/Icon-Small-50@2x.png", "ARKitFaceExample/Resources/Assets.xcassets/AppIcon.appiconset/Icon-Small.png", "ARKitFaceExample/Resources/Assets.xcassets/AppIcon.appiconset/Icon-Small@2x.png", "ARKitFaceExample/Resources/Assets.xcassets/AppIcon.appiconset/Icon-Small@3x.png", "ARKitFaceExample/Resources/Assets.xcassets/AppIcon.appiconset/Icon.png"]}

## Source file: ARKitFaceExample/Resources/VideoTexturedFace.shader

{
  "path": "ARKitFaceExample/Resources/VideoTexturedFace.shader",
  "blob_sha": "a8c757adab091f1c47d28661f2b7f7cb0cf4c40f",
  "sha256": "c46387e6ac747a54c35333d5d869d1ed8259b075bb0c4f68d00b8a7c12654f16",
  "lines": 37,
  "bytes": 1295,
  "selection_score": 28,
  "keyword_line_counts": {
    "gum": 1,
    "shader": 1,
    "rig": 1,
    "face": 1,
    "lip": 7
  },
  "url": "https://github.com/joenewbry/blendshapes/blob/fa8dc0f91b4933c5cd841c6498976346878d2aea/ARKitFaceExample/Resources/VideoTexturedFace.shader",
  "function_signature_candidates": []
}

```text

/*
 <samplecode>
 <abstract>
 SceneKit shader (geometry) modifier for texture mapping ARKit camera video onto the face.
 </abstract>
 </samplecode>
 */

#pragma arguments
float4x4 displayTransform // from ARFrame.displayTransform(for:viewportSize:)

#pragma body

// Transform the vertex to the camera coordinate system.
float4 vertexCamera = scn_node.modelViewTransform * _geometry.position;

// Camera projection and perspective divide to get normalized viewport coordinates (clip space).
float4 vertexClipSpace = scn_frame.projectionTransform * vertexCamera;
vertexClipSpace /= vertexClipSpace.w;

// XY in clip space is [-1,1]x[-1,1], so adjust to UV texture coordinates: [0,1]x[0,1].
// Image coordinates are Y-flipped (upper-left origin).
float4 vertexImageSpace = float4(vertexClipSpace.xy * 0.5 + 0.5, 0.0, 1.0);
vertexImageSpace.y = 1.0 - vertexImageSpace.y;

// Apply ARKit's display transform (device orientation * front-facing camera flip).
float4 transformedVertex = displayTransform * vertexImageSpace;

// Output as texture coordinates for use in later rendering stages.
_geometry.texcoords[0] = transformedVertex.xy;

/**
 * MARK: Post-process special effects
 */

// Make head appear big. (You could also apply other geometry modifications here.)
_geometry.position.xyz *= 1.5;


```
