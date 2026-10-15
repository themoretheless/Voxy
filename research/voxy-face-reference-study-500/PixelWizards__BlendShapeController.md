# Facial rendering source scan: PixelWizards/BlendShapeController

Repository: https://github.com/PixelWizards/BlendShapeController

Pinned source revision: ee0e791e318124dae82e3748214310069c7b26f9

Discovery topic: blendshape

Description: BlendShapeController timeline track - pose & animate blendshapes on Unity's Timeline

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"blendshape": ["BlendShapeControl.meta", "BlendShapeControl/BlendShapeControlBehaviour.cs", "BlendShapeControl/BlendShapeControlBehaviour.cs.meta", "BlendShapeControl/BlendShapeControlClip.cs", "BlendShapeControl/BlendShapeControlClip.cs.meta", "BlendShapeControl/BlendShapeControlMixerBehaviour.cs", "BlendShapeControl/BlendShapeControlMixerBehaviour.cs.meta", "BlendShapeControl/BlendShapeControlTrack.cs", "BlendShapeControl/BlendShapeControlTrack.cs.meta", "BlendShapeControl/BlendShapeController.cs", "BlendShapeControl/BlendShapeController.cs.meta", "BlendShapeControl/BlendShapeKey.cs", "BlendShapeControl/BlendShapeKey.cs.meta", "BlendShapeControl/BlendShapeTarget.cs", "BlendShapeControl/BlendShapeTarget.cs.meta", "BlendShapeControl/Editor.meta", "BlendShapeControl/Editor/BlendShapeControlClipEditor.cs", "BlendShapeControl/Editor/BlendShapeControlClipEditor.cs.meta", "BlendShapeControl/Editor/BlendShapeControllerEditor.cs", "BlendShapeControl/Editor/BlendShapeControllerEditor.cs.meta"], "lip": ["BlendShapeControl/BlendShapeControlClip.cs", "BlendShapeControl/BlendShapeControlClip.cs.meta", "BlendShapeControl/Editor/BlendShapeControlClipEditor.cs", "BlendShapeControl/Editor/BlendShapeControlClipEditor.cs.meta"]}

## Source file: BlendShapeControl/BlendShapeControlClip.cs

{
  "path": "BlendShapeControl/BlendShapeControlClip.cs",
  "blob_sha": "2d12a310099e647e3011b2ef26f98f73d77854c7",
  "sha256": "dddfe8df8dd95e15d546f6e6a0d0fd8019ccd581884804585e5b2b7b542416db",
  "lines": 20,
  "bytes": 600,
  "selection_score": 30,
  "keyword_line_counts": {
    "blendshape": 3,
    "lip": 3
  },
  "url": "https://github.com/PixelWizards/BlendShapeController/blob/ee0e791e318124dae82e3748214310069c7b26f9/BlendShapeControl/BlendShapeControlClip.cs",
  "function_signature_candidates": []
}

```text

using System;
using UnityEngine;
using UnityEngine.Playables;
using UnityEngine.Timeline;
using System.Collections.Generic;
[Serializable]
public class BlendShapeControlClip : PlayableAsset, ITimelineClipAsset
{
    public BlendShapeControlBehaviour template = new BlendShapeControlBehaviour();
    public ClipCaps clipCaps
    {
        get { return ClipCaps.Blending; }
    }
    public override Playable CreatePlayable(PlayableGraph graph, GameObject owner)
    {
        var playable = ScriptPlayable<BlendShapeControlBehaviour>.Create(graph, template);
        
        return playable;
    }
}


```

## Source file: BlendShapeControl/Editor/BlendShapeControlClipEditor.cs

{
  "path": "BlendShapeControl/Editor/BlendShapeControlClipEditor.cs",
  "blob_sha": "cb6df991dc29b993a8fd3fc4f0c35234aa5891ee",
  "sha256": "0dec350e94516aa7c1be215d85d599deb2da573880a0fcb210cf84e998aca9a9",
  "lines": 58,
  "bytes": 2311,
  "selection_score": 30,
  "keyword_line_counts": {
    "skin": 1,
    "blendshape": 9,
    "lip": 2
  },
  "url": "https://github.com/PixelWizards/BlendShapeController/blob/ee0e791e318124dae82e3748214310069c7b26f9/BlendShapeControl/Editor/BlendShapeControlClipEditor.cs",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "public override void OnInspectorGUI()"
    }
  ]
}

```text

﻿using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using UnityEditor;

[CustomEditor(typeof(BlendShapeControlClip))]
public class BlendShapeControlClipEditor : Editor
{

    public override void OnInspectorGUI()
    {
        SerializedObject so = base.serializedObject;
        so.UpdateIfRequiredOrScript();


        SerializedProperty behaviourProperty = so.FindProperty("template");



        EditorGUILayout.LabelField(behaviourProperty.FindPropertyRelative("BCBGuid").stringValue);
        GUILayout.BeginVertical(EditorStyles.helpBox);
        //identify Skinned mesh with blend shapes
        GUILayout.Label(behaviourProperty.FindPropertyRelative("name").stringValue + " Blend Shapes", EditorStyles.boldLabel);

        // Only uncomment if problems are occurring and you need to visually examine the data being stored
       // EditorGUILayout.PropertyField(behaviourProperty.FindPropertyRelative("blendShapeKeyInfos"), true);

        //get array of BlendShapeKeys
        var BSTArray = behaviourProperty.FindPropertyRelative("blendShapeKeys");
        
        //iterate through BlendShapeTargets array if array isn't empty
        if (BSTArray != null && BSTArray.arraySize > 0)
        {
            for (int i = 0; i < BSTArray.arraySize; i++)
            {
                GUILayout.BeginVertical("Box");
                var target = BSTArray.GetArrayElementAtIndex(i);
                //not sure if retrieving multiple SerializedObjects is best practice, seems to work though
                var targetprop = new SerializedObject(target.objectReferenceValue);
                targetprop.UpdateIfRequiredOrScript();
                //informational label
                string blendShapeInfo = "Name: " + targetprop.FindProperty("BlendShapeName").stringValue + "   Index: " + targetprop.FindProperty("BlendShapeIndex").intValue.ToString();
                EditorGUILayout.LabelField(blendShapeInfo);


                //blemd shape value (normalized form 0 to 1)
                EditorGUILayout.PropertyField(targetprop.FindProperty("BlendShapeValue"), true);


                targetprop.ApplyModifiedProperties();
                GUILayout.EndVertical();
            }
        }
        GUILayout.EndVertical();
        so.ApplyModifiedProperties();
    }
}



```

## Source file: BlendShapeControl/BlendShapeKey.cs

{
  "path": "BlendShapeControl/BlendShapeKey.cs",
  "blob_sha": "f74577324f3d5b558eae887578cd7d57b4ad18e7",
  "sha256": "c2a347f1557aaeb1ff8c7b79aba9b1993c9ce3071ce0e3752ffd3c82dafeffd9",
  "lines": 58,
  "bytes": 1160,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 16
  },
  "url": "https://github.com/PixelWizards/BlendShapeController/blob/ee0e791e318124dae82e3748214310069c7b26f9/BlendShapeControl/BlendShapeKey.cs",
  "function_signature_candidates": [
    {
      "line": 28,
      "text": "public delegate void UpdateValue(int index, float value);"
    },
    {
      "line": 36,
      "text": "public void init(string name, int index, float value)"
    },
    {
      "line": 43,
      "text": "void OnValidate()"
    },
    {
      "line": 54,
      "text": "void OnDestroy()"
    }
  ]
}

```text

﻿using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using System;





[Serializable]
public struct BlendShapeKeyInfo
{

    public float BlendShapeValue;
    public string BlendShapeName;
    public int BlendShapeIndex;
    public BlendShapeKeyInfo(string name, int index, float value)
    {
        BlendShapeName = name;
        BlendShapeIndex = index;
        BlendShapeValue = value;
    }
}

[System.Serializable]
public class BlendShapeKey : ScriptableObject
{
    public delegate void UpdateValue(int index, float value);
    public UpdateValue updateBlendValue;
    public string BlendShapeName;
    public int BlendShapeIndex;

    [RangeAttribute(0, 1)]
    public float BlendShapeValue = 0;

    public void init(string name, int index, float value)
    {
        BlendShapeName = name;
        BlendShapeIndex = index;
        BlendShapeValue = value;
    }

    void OnValidate()
    {
   
        if (!Application.isPlaying && updateBlendValue!= null )
        {


            updateBlendValue(BlendShapeIndex, BlendShapeValue);
        }
    }

    void OnDestroy()
    {
        updateBlendValue = null;
    }
}


```

## Source file: BlendShapeControl/BlendShapeTarget.cs

{
  "path": "BlendShapeControl/BlendShapeTarget.cs",
  "blob_sha": "6941d907d618edd45faca65e14f6726dd709ae68",
  "sha256": "97054fd0290b63495ee3c6a0cbeb020990c4afae29663df0ea9ce226f3c928a1",
  "lines": 44,
  "bytes": 1043,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 2,
    "blendshape": 11,
    "face": 1
  },
  "url": "https://github.com/PixelWizards/BlendShapeController/blob/ee0e791e318124dae82e3748214310069c7b26f9/BlendShapeControl/BlendShapeTarget.cs",
  "function_signature_candidates": [
    {
      "line": 29,
      "text": "public void init(string name, int index, SkinnedMeshRenderer mesh = null)"
    },
    {
      "line": 36,
      "text": "void OnValidate()"
    }
  ]
}

```text

﻿using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using System;

public interface IBlendShapeOwner
{

}

[System.Serializable]
public class BlendShapeTarget : ScriptableObject
{
    [SerializeField]
    public string BlendShapeName;
    [SerializeField]
    public int BlendShapeIndex;
    [SerializeField]
    [RangeAttribute(0, 1)]
    public float BlendShapeValue = 0;
    [RangeAttribute(0, 1)]
    [SerializeField]
    public float BlendShapeDefaultValue = 0;
    [SerializeField]
    public IBlendShapeOwner owningBehaviour;
    [SerializeField]
    public SkinnedMeshRenderer TargetMesh;

    public void init(string name, int index, SkinnedMeshRenderer mesh = null)
    {
        BlendShapeName = name;
        BlendShapeIndex = index;
        TargetMesh = mesh;
    }

    void OnValidate()
    {
        if (TargetMesh != null)
        {
            BlendShapeValue = BlendShapeDefaultValue;
            TargetMesh.SetBlendShapeWeight(BlendShapeIndex, BlendShapeDefaultValue * 100);
        }
    }
}


```
