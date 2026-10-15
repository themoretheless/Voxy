# Facial rendering source scan: BarbUnicorn/uniVRM-ARKit

Repository: https://github.com/BarbUnicorn/uniVRM-ARKit

Pinned source revision: 21634ddd8bcbcef9937f77989f13046a6ab52519

Discovery topic: ARKit blendshape

Description: A script to generate the ARKit necessary blendshape clips automatically

Reported license: BSD-3-Clause

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE.txt"]

Topic paths: {}

## Source file: Assets/ARKit/ARKit Create.cs

{
  "path": "Assets/ARKit/ARKit Create.cs",
  "blob_sha": "cdce08f20a8dfa17058e565995fcad9d865edb2a",
  "sha256": "c8ba052c7d7e92b0d9920fcc64d8b60a8c3337d5ecdd7ba66c6301f333e624ed",
  "lines": 150,
  "bytes": 5361,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 22,
    "skin": 5,
    "blendshape": 17,
    "jaw": 4,
    "mouth": 23,
    "tongue": 1,
    "rig": 28,
    "lip": 7
  },
  "url": "https://github.com/BarbUnicorn/uniVRM-ARKit/blob/21634ddd8bcbcef9937f77989f13046a6ab52519/Assets/ARKit/ARKit%20Create.cs",
  "function_signature_candidates": [
    {
      "line": 114,
      "text": "static void GenerateBlendshapeClips()"
    }
  ]
}

```text

﻿using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using UnityEditor;
using VRM;
using System.IO;
using UniGLTF;

namespace VRMARkit
{
    public class ARKitCreate : MonoBehaviour
    {
#if UNITY_EDITOR
        static readonly string[] blendShapes = {
            "browDownLeft",
            "browDownRight",
            "browInnerUpLeft",
            "browInnerUpRight",
            "browOuterUpLeft",
            "browOuterUpRight",
            "cheekPuff",
            "eyeBlinkLeft",
            "eyeBlinkRight",
            "eyeLookDownLeft",
            "eyeLookDownRight",
            "eyeLookInLeft",
            "eyeLookInRight",
            "eyeLookOutLeft",
            "eyeLookOutRight",
            "eyeLookUpLeft",
            "eyeLookUpRight",
            "eyeSquintLeft",
            "eyeSquintRight",
            "eyeWideLeft",
            "eyeWideRight",
            "jawForward",
            "jawLeft",
            "jawOpen",
            "jawRight",
            "mouthClose",
            "mouthDimpleLeft",
            "mouthDimpleRight",
            "mouthFrownLeft",
            "mouthFrownRight",
            "mouthFunnel",
            "mouthLeft",
            "mouthLowerDownLeft",
            "mouthLowerDownRight",
            "mouthPressLeft",
            "mouthPressRight",
            "mouthPucker",
            "mouthRight",
            "mouthRollLower",
            "mouthRollUpper",
            "mouthShrugLower",
            "mouthShrugUpper",
            "mouthSmileLeft",
            "mouthSmileRight",
            "mouthStretchLeft",
            "mouthStretchRight",
            "mouthUpperUpLeft",
            "mouthUpperUpRight",
            "noseSneerLeft",
            "noseSneerRight",
            "tongueOut"
        };
        static readonly Dictionary<string, string> bsConstrains = new Dictionary<string, string>() {
            { "eyeLookDownLeft",    "eyeLookDownRight"  },
            { "eyeLookDownRight",   "eyeLookDownLeft"   },
            { "eyeLookUpLeft",      "eyeLookUpRight"    },
            { "eyeLookUpRight",     "eyeLookUpLeft"     },
            { "eyeLookInLeft",      "eyeLookOutRight"   },
            { "eyeLookInRight",     "eyeLookOutLeft"    },
            { "eyeLookOutLeft",     "eyeLookInRight"    },
            { "eyeLookOutRight",    "eyeLookInLeft"     }
        };

        static List<BlendShapeBinding> CreateBlendShapeBinding(SkinnedMeshRenderer[] smrs, Transform root, string shapeName)
        {
            List<BlendShapeBinding> shapeBindings = new List<BlendShapeBinding>();
            string constrainShape = "";
            if (bsConstrains.ContainsKey(shapeName))
                constrainShape = bsConstrains[shapeName];

            foreach (SkinnedMeshRenderer smr in smrs)
            {
                Mesh mesh = smr.sharedMesh;
                if (mesh == null)
                {
                    Debug.LogErrorFormat("SkinnedMeshRenderer {1} has no mesh associated to it", smr.name);
                    return null;
                }
             
                for (int i = 0; i < mesh.blendShapeCount; i++)
                {
                    float weight = 0;
                    if (mesh.GetBlendShapeName(i).ToLower() == shapeName.ToLower())
                        weight = 100;
                    else if (mesh.GetBlendShapeName(i).ToLower() == constrainShape.ToLower())
                        weight = 100;

                    shapeBindings.Add(new BlendShapeBinding
                    {
                        Index = i,
                        RelativePath = smr.transform.RelativePathFrom(root),
                        Weight = weight,
                    });
                }
            }
            return shapeBindings;
        }

        [MenuItem("ARKit/Generate ARKit Blendshape clips")]
        static void GenerateBlendshapeClips()
        {
            Transform activeAvatar = Selection.activeTransform;
            if (activeAvatar == null )
            {
                Debug.LogError("No object has been selected in the scene!");
                return;
            }

            VRMBlendShapeProxy proxy = activeAvatar.GetComponent<VRMBlendShapeProxy>();
            if ( proxy == null )
            {
                Debug.LogErrorFormat("The object {0} has no BlenderShapes", activeAvatar);
                return;
            }

            SkinnedMeshRenderer[] skinnedMeshes = activeAvatar.GetComponentsInChildren<SkinnedMeshRenderer>();

            string blendShapesPath = Directory.GetParent(AssetDatabase.GetAssetPath( proxy.BlendShapeAvatar )).ToString();
            Debug.LogFormat("Path to the BlendShapes folder: {0}", blendShapesPath);

            foreach (string shapeName in blendShapes)
            {
                List<BlendShapeBinding> bindings = CreateBlendShapeBinding(skinnedMeshes, activeAvatar, shapeName);
                string path = string.Format("{0}/{1}.asset", blendShapesPath, shapeName);
                BlendShapeClip bsClip = BlendShapeAvatar.CreateBlendShapeClip(path);
                if (bsClip != null)
                {
                    bsClip.name = shapeName;
                    bsClip.Values = bindings.ToArray();
                    proxy.BlendShapeAvatar.Clips.Add(bsClip);
                }
            }
        }
#endif
    }
}


```
