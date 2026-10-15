# Facial rendering source scan: sh-akira/VirtualMotionCaptureProtocol

Repository: https://github.com/sh-akira/VirtualMotionCaptureProtocol

Pinned source revision: f41f42777a3ef425d19f87a346f8682e70a2ec55

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {}

## Source file: sample/SampleTrackerSend.cs

{
  "path": "sample/SampleTrackerSend.cs",
  "blob_sha": "b6f93aa8771032535d1433bb0b709941b7fbca47",
  "sha256": "9e2de3ee6784c39c52e47f5df9123551e45e75358656b027fa7a5ab9f3aa2607",
  "lines": 85,
  "bytes": 2361,
  "selection_score": 0,
  "keyword_line_counts": {
    "blendshape": 4
  },
  "url": "https://github.com/sh-akira/VirtualMotionCaptureProtocol/blob/f41f42777a3ef425d19f87a346f8682e70a2ec55/sample/SampleTrackerSend.cs",
  "function_signature_candidates": [
    {
      "line": 38,
      "text": "void Start()"
    },
    {
      "line": 42,
      "text": "void LateUpdate()"
    }
  ]
}

```text

﻿/*
 * SampleTrackerSend
 * gpsnmeajp
 * https://sh-akira.github.io/VirtualMotionCaptureProtocol/
 *
 * These codes are licensed under CC0.
 * http://creativecommons.org/publicdomain/zero/1.0/deed.ja
 */
using System;
using System.IO;
using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.Events;
using UnityEngine.Profiling;

[RequireComponent(typeof(uOSC.uOscClient))]
public class SampleTrackerSend : MonoBehaviour
{
    public enum VirtualDevice
    {
        HMD = 0,
        Controller = 1,
        Tracker = 2,
    }

    [Header("Virtual Device")]
    public VirtualDevice DeviceMode = VirtualDevice.Tracker;
    public Transform DeviceTransform = null;
    public String DeviceSerial = "VIRTUAL_DEVICE";

    [Header("BlendShapeProxy")]
    public string BlendShapeName = "";
    public float BlendShapeValue = 0f;

    uOSC.uOscClient client = null;

    void Start()
    {
        client = GetComponent<uOSC.uOscClient>();
    }
    void LateUpdate()
    {
        if (client == null)
        {
            return;
        }

        {
            string name = null;
            switch (DeviceMode)
            {
                case VirtualDevice.HMD:
                    name = "/VMC/Ext/Hmd/Pos";
                    break;
                case VirtualDevice.Controller:
                    name = "/VMC/Ext/Con/Pos";
                    break;
                case VirtualDevice.Tracker:
                    name = "/VMC/Ext/Tra/Pos";
                    break;
                default:
                    name = null;
                    break;
            }
            if (name != null && DeviceTransform != null && DeviceSerial != null)
            {
                client.Send(name,
                    (string)DeviceSerial,
                    (float)DeviceTransform.position.x,
                    (float)DeviceTransform.position.y,
                    (float)DeviceTransform.position.z,
                    (float)DeviceTransform.rotation.x,
                    (float)DeviceTransform.rotation.y,
                    (float)DeviceTransform.rotation.z,
                    (float)DeviceTransform.rotation.w);
            }
        }

        {
            client.Send("/VMC/Ext/Blend/Val", BlendShapeName, BlendShapeValue);
            client.Send("/VMC/Ext/Blend/Apply");
        }
    }
}


```

## Source file: sample/CameraPositionSend.cs

{
  "path": "sample/CameraPositionSend.cs",
  "blob_sha": "1b4147c95148a0d945586ea2b7af0a3a4ecbf883",
  "sha256": "bf14cd2e64fbca35fe47792990a01c1992b316e9a11755e30036e9a1b74fd458",
  "lines": 30,
  "bytes": 827,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/sh-akira/VirtualMotionCaptureProtocol/blob/f41f42777a3ef425d19f87a346f8682e70a2ec55/sample/CameraPositionSend.cs",
  "function_signature_candidates": [
    {
      "line": 18,
      "text": "void Start()"
    },
    {
      "line": 23,
      "text": "void LateUpdate()"
    }
  ]
}

```text

﻿/*
 * CameraPositionSend
 * gpsnmeajp
 * https://sh-akira.github.io/VirtualMotionCaptureProtocol/
 *
 * These codes are licensed under CC0.
 * http://creativecommons.org/publicdomain/zero/1.0/deed.ja
 */
using System.Collections;
using System.Collections.Generic;
using UnityEngine;

[RequireComponent(typeof(uOSC.uOscClient))]
public class CameraPositionSend : MonoBehaviour
{
    public float fov = 60;
    uOSC.uOscClient uClient = null;
    void Start()
    {
        uClient = GetComponent<uOSC.uOscClient>();
    }

    void LateUpdate()
    {
        uClient.Send("/VMC/Ext/Cam",
                    "camera",
                    transform.position.x, transform.position.y, transform.position.z,
                    transform.rotation.x, transform.rotation.y, transform.rotation.z, transform.rotation.w,fov);
    }
}


```

## Source file: sample/VRM0/SampleBonesSend.cs

{
  "path": "sample/VRM0/SampleBonesSend.cs",
  "blob_sha": "cbdeb44799ab2893c791cd9aa36821df9a047d87",
  "sha256": "68eecd4c96e576221bbc46302fe83505f9df463743a4882d44f450393ef89874",
  "lines": 135,
  "bytes": 4798,
  "selection_score": 0,
  "keyword_line_counts": {
    "blendshape": 5,
    "rig": 2
  },
  "url": "https://github.com/sh-akira/VirtualMotionCaptureProtocol/blob/f41f42777a3ef425d19f87a346f8682e70a2ec55/sample/VRM0/SampleBonesSend.cs",
  "function_signature_candidates": [
    {
      "line": 43,
      "text": "void Start()"
    },
    {
      "line": 48,
      "text": "void LateUpdate()"
    },
    {
      "line": 120,
      "text": "void SendBoneTransformForTracker(HumanBodyBones bone, string DeviceSerial)"
    }
  ]
}

```text

﻿/*
 * SampleBonesSend (VRM0.99)
 * gpsnmeajp
 * https://sh-akira.github.io/VirtualMotionCaptureProtocol/
 *
 * These codes are licensed under CC0.
 * http://creativecommons.org/publicdomain/zero/1.0/deed.ja
 */
 
// このスクリプトの利用は推奨しません！！
// このスクリプトは、学習用であり、必要な処理を欠いているため、通信バッファの詰まりを引き起こして、動作に異常な遅れを生じる場合があります。
// SampleBonesSendBundle.csの方を利用することを強く推奨します。

// NOT RECOMMENDED USE THIS SCRIPT!!
// This script is for learning purposes only and lacks necessary processing, which can cause communication buffers to clog up and cause unusual delays in operation.
// It is strongly recommended to use SampleBonesSendBundle.cs.

using System;
using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using VRM;

[RequireComponent(typeof(uOSC.uOscClient))]
public class SampleBonesSend : MonoBehaviour
{
    uOSC.uOscClient uClient = null;

    public GameObject Model = null;
    private GameObject OldModel = null;

    Animator animator = null;
    VRMBlendShapeProxy blendShapeProxy = null;

    public string filepath;
    public enum VirtualDevice
    {
        HMD = 0,
        Controller = 1,
        Tracker = 2,
    }

    void Start()
    {
        uClient = GetComponent<uOSC.uOscClient>();
    }

    void LateUpdate()
    {
        //モデルが更新されたときのみ読み込み
        if (Model != null && OldModel != Model)
        {
            animator = Model.GetComponent<Animator>();
            blendShapeProxy = Model.GetComponent<VRMBlendShapeProxy>();
            OldModel = Model;
        }

        if (Model != null && animator != null && uClient != null)
        {
            //Root
            var RootTransform = Model.transform;
            if (RootTransform != null)
            {
                uClient.Send("/VMC/Ext/Root/Pos",
                    "root",
                    RootTransform.position.x, RootTransform.position.y, RootTransform.position.z,
                    RootTransform.rotation.x, RootTransform.rotation.y, RootTransform.rotation.z, RootTransform.rotation.w);
            }

            //Bones
            foreach (HumanBodyBones bone in Enum.GetValues(typeof(HumanBodyBones)))
            {
                if (bone != HumanBodyBones.LastBone)
                {
                    var Transform = animator.GetBoneTransform(bone);
                    if (Transform != null)
                    {
                        uClient.Send("/VMC/Ext/Bone/Pos",
                            bone.ToString(),
                            Transform.localPosition.x, Transform.localPosition.y, Transform.localPosition.z,
                            Transform.localRotation.x, Transform.localRotation.y, Transform.localRotation.z, Transform.localRotation.w);
                    }
                }
            }

            //ボーン位置を仮想トラッカーとして送信
            SendBoneTransformForTracker(HumanBodyBones.Head, "Head");
            SendBoneTransformForTracker(HumanBodyBones.Spine, "Spine");
            SendBoneTransformForTracker(HumanBodyBones.LeftHand, "LeftHand");
            SendBoneTransformForTracker(HumanBodyBones.RightHand, "RightHand");
            SendBoneTransformForTracker(HumanBodyBones.LeftFoot, "LeftFoot");
            SendBoneTransformForTracker(HumanBodyBones.RightFoot, "RightFoot");

            //BlendShape
            if (blendShapeProxy != null)
            {
                foreach (var b in blendShapeProxy.GetValues())
                {
                    uClient.Send("/VMC/Ext/Blend/Val",
                        b.Key.ToString(),
                        (float)b.Value
                        );
                }
                uClient.Send("/VMC/Ext/Blend/Apply");
            }

            //Available
            uClient.Send("/VMC/Ext/OK", 1);
        }
        else
        {
            uClient.Send("/VMC/Ext/OK", 0);
        }
        uClient.Send("/VMC/Ext/T", Time.time);

        //Load request
        uClient.Send("/VMC/Ext/VRM", filepath, "");
    }

    void SendBoneTransformForTracker(HumanBodyBones bone, string DeviceSerial)
    {
        var DeviceTransform = animator.GetBoneTransform(bone);
        if (DeviceTransform != null) {
            uClient.Send("/VMC/Ext/Tra/Pos",
        (string)DeviceSerial,
        (float)DeviceTransform.position.x,
        (float)DeviceTransform.position.y,
        (float)DeviceTransform.position.z,
        (float)DeviceTransform.rotation.x,
        (float)DeviceTransform.rotation.y,
        (float)DeviceTransform.rotation.z,
        (float)DeviceTransform.rotation.w);
        }
    }
}


```

## Source file: sample/VRM1/SampleBonesSend.cs

{
  "path": "sample/VRM1/SampleBonesSend.cs",
  "blob_sha": "3511b609c4793d7a276fbcf8ea8789761f210521",
  "sha256": "fbf39b66ad6be94bff6fc2dfeb4e33a2ebb19ae35d7f25a4f077c6d4c82088da",
  "lines": 160,
  "bytes": 5955,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 3
  },
  "url": "https://github.com/sh-akira/VirtualMotionCaptureProtocol/blob/f41f42777a3ef425d19f87a346f8682e70a2ec55/sample/VRM1/SampleBonesSend.cs",
  "function_signature_candidates": [
    {
      "line": 46,
      "text": "void Start()"
    },
    {
      "line": 51,
      "text": "void LateUpdate()"
    },
    {
      "line": 145,
      "text": "void SendBoneTransformForTracker(HumanBodyBones bone, string DeviceSerial)"
    }
  ]
}

```text

﻿/*
 * SampleBonesSend (VRM1)
 * gpsnmeajp
 * https://sh-akira.github.io/VirtualMotionCaptureProtocol/
 *
 * These codes are licensed under CC0.
 * http://creativecommons.org/publicdomain/zero/1.0/deed.ja
 */
 
// このスクリプトの利用は推奨しません！！
// このスクリプトは、学習用であり、必要な処理を欠いているため、通信バッファの詰まりを引き起こして、動作に異常な遅れを生じる場合があります。
// SampleBonesSendBundle.csの方を利用することを強く推奨します。

// NOT RECOMMENDED USE THIS SCRIPT!!
// This script is for learning purposes only and lacks necessary processing, which can cause communication buffers to clog up and cause unusual delays in operation.
// It is strongly recommended to use SampleBonesSendBundle.cs.
 
using System;
using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using UniVRM10;

[RequireComponent(typeof(uOSC.uOscClient))]
public class SampleBonesSend : MonoBehaviour
{
    uOSC.uOscClient uClient = null;

    public GameObject Model = null;
    private GameObject OldModel = null;

    Animator animator = null;
    Vrm10Instance vrmRoot = null;

    public bool vrm0styleExpression = true;

    public string filepath;

    public enum VirtualDevice
    {
        HMD = 0,
        Controller = 1,
        Tracker = 2,
    }

    void Start()
    {
        uClient = GetComponent<uOSC.uOscClient>();
    }

    void LateUpdate()
    {
        //Only model updated
        if (Model != null && OldModel != Model)
        {
            animator = Model.GetComponent<Animator>();
            vrmRoot = Model.GetComponent<Vrm10Instance>();
            OldModel = Model;
        }

        if (Model != null && animator != null && uClient != null)
        {
            //Root
            var RootTransform = Model.transform;
            if (RootTransform != null)
            {
                uClient.Send("/VMC/Ext/Root/Pos",
                    "root",
                    RootTransform.position.x, RootTransform.position.y, RootTransform.position.z,
                    RootTransform.rotation.x, RootTransform.rotation.y, RootTransform.rotation.z, RootTransform.rotation.w);
            }

            //Bones
            foreach (HumanBodyBones bone in Enum.GetValues(typeof(HumanBodyBones)))
            {
                if (bone != HumanBodyBones.LastBone)
                {
                    var Transform = animator.GetBoneTransform(bone);
                    if (Transform != null)
                    {
                        uClient.Send("/VMC/Ext/Bone/Pos",
                            bone.ToString(),
                            Transform.localPosition.x, Transform.localPosition.y, Transform.localPosition.z,
                            Transform.localRotation.x, Transform.localRotation.y, Transform.localRotation.z, Transform.localRotation.w);
                    }
                }
            }

            //Virtual Tracker send from bone position
            SendBoneTransformForTracker(HumanBodyBones.Head, "Head");
            SendBoneTransformForTracker(HumanBodyBones.Spine, "Spine");
            SendBoneTransformForTracker(HumanBodyBones.LeftHand, "LeftHand");
            SendBoneTransformForTracker(HumanBodyBones.RightHand, "RightHand");
            SendBoneTransformForTracker(HumanBodyBones.LeftFoot, "LeftFoot");
            SendBoneTransformForTracker(HumanBodyBones.RightFoot, "RightFoot");

            //Expression
            if (vrmRoot != null)
            {
                foreach (var b in vrmRoot.Runtime.Expression.GetWeights())
                {
                    string expressionName = b.Key.ToString();

                    if (vrm0styleExpression)
                    {
                        //VRM1 Preset -> VRM0 Preset 
                        switch (expressionName.ToLower())
                        {
                            case "happy": expressionName = "Joy"; break;
                            case "angry": expressionName = "Angry"; break;
                            case "sad": expressionName = "Sorrow"; break;
                            case "relaxed": expressionName = "Fun"; break;
                            case "aa": expressionName = "A"; break;
                            case "ih": expressionName = "I"; break;
                            case "ou": expressionName = "U"; break;
                            case "ee": expressionName = "E"; break;
                            case "oh": expressionName = "O"; break;
                            case "blinkleft": expressionName = "Blink_L"; break;
                            case "blinkright": expressionName = "Blink_R"; break;
                            default: break;
                        }
                    }

                    uClient.Send("/VMC/Ext/Blend/Val",
                        expressionName,
                        (float)b.Value
                        );
                }
                uClient.Send("/VMC/Ext/Blend/Apply");
            }

            //Available
            uClient.Send("/VMC/Ext/OK", 1);
        }
        else
        {
            uClient.Send("/VMC/Ext/OK", 0);
        }
        uClient.Send("/VMC/Ext/T", Time.time);

        //Load request
        uClient.Send("/VMC/Ext/VRM", filepath, vrmRoot.Vrm.Meta.Name);
    }

    void SendBoneTransformForTracker(HumanBodyBones bone, string DeviceSerial)
    {
        var DeviceTransform = animator.GetBoneTransform(bone);
        if (DeviceTransform != null) {
            uClient.Send("/VMC/Ext/Tra/Pos",
        (string)DeviceSerial,
        (float)DeviceTransform.position.x,
        (float)DeviceTransform.position.y,
        (float)DeviceTransform.position.z,
        (float)DeviceTransform.rotation.x,
        (float)DeviceTransform.rotation.y,
        (float)DeviceTransform.rotation.z,
        (float)DeviceTransform.rotation.w);
        }
    }
}


```
