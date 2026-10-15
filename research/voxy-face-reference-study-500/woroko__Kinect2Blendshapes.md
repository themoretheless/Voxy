# Facial rendering source scan: woroko/Kinect2Blendshapes

Repository: https://github.com/woroko/Kinect2Blendshapes

Pinned source revision: 455af0088b711fad88bf4086cf508f435154641a

Discovery topic: blendshape

Description: Quick scripts for displaying Kinect V2 HD Face animation units as Unity skinned mesh blendshapes

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"blendshape": ["BlendshapeMapping.cs"], "face": ["HDFaceSourceManager.cs"]}

## Source file: BlendshapeMapping.cs

{
  "path": "BlendshapeMapping.cs",
  "blob_sha": "fec90d2affda98c528ae2f2cba68364a4ab28dda",
  "sha256": "95712e1e4e3b45a87d66923769e7b41adb2e4c4afcf5ae8d2909d3ba69fc8b20",
  "lines": 11,
  "bytes": 261,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 1
  },
  "url": "https://github.com/woroko/Kinect2Blendshapes/blob/455af0088b711fad88bf4086cf508f435154641a/BlendshapeMapping.cs",
  "function_signature_candidates": []
}

```text

﻿using System.Collections;
using System.Collections.Generic;
using UnityEngine;

[System.Serializable]
public class BlendshapeMapping : System.Object
{
    public string from = "";
    public float offset = 0.0f;
    public List<StringFloat> to;
}


```

## Source file: HDFaceSourceManager.cs

{
  "path": "HDFaceSourceManager.cs",
  "blob_sha": "28e8de66f54ae5eb847fb8f2f2c70eff4bd3a92b",
  "sha256": "2deaacc30b66b0b4dc83e428f3830ca95d499ebd8a3a60d56faa0a016d1bc1d5",
  "lines": 511,
  "bytes": 11676,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 1,
    "skin": 2,
    "blendshape": 13,
    "deform": 1,
    "face": 44
  },
  "url": "https://github.com/woroko/Kinect2Blendshapes/blob/455af0088b711fad88bf4086cf508f435154641a/HDFaceSourceManager.cs",
  "function_signature_candidates": [
    {
      "line": 187,
      "text": "void Start()"
    },
    {
      "line": 221,
      "text": "private void OnApplicationQuit()"
    },
    {
      "line": 230,
      "text": "private void InitializeMesh(MeshFilter faceMeshFilter, Mesh faceMesh)"
    },
    {
      "line": 314,
      "text": "private void UpdateMesh()"
    },
    {
      "line": 336,
      "text": "void Update()"
    },
    {
      "line": 343,
      "text": "private void BodyReader_FrameArrived(object sender, BodyFrameArrivedEventArgs e)"
    },
    {
      "line": 399,
      "text": "private void HdFaceSource_TrackingIdLost(object sender, TrackingIdLostEventArgs e)"
    },
    {
      "line": 433,
      "text": "private void UpdateActormeshBlendshapes(Dictionary<string, float> aus)"
    },
    {
      "line": 466,
      "text": "private void HdFaceReader_FrameArrived(object sender, HighDefinitionFaceFrameArrivedEventArgs e)"
    }
  ]
}

```text

// NOTE: contains parts from Sample Code, the rest is licensed under MIT license

using UnityEngine;

using System.Collections;

using System.Collections.Generic;

using Windows.Kinect;

using Microsoft.Kinect.Face;

public class HDFaceSourceManager : MonoBehaviour
{

    private KinectSensor sensor = null;

    private BodyFrameSource bodySource = null;

    private BodyFrameReader bodyReader = null;

    private HighDefinitionFaceFrameSource highDefinitionFaceFrameSource = null;

    private HighDefinitionFaceFrameReader highDefinitionFaceFrameReader = null;

    private FaceAlignment currentFaceAlignment = null;

    private FaceModel currentFaceModel = null;

    private Body currentTrackedBody = null;

    private ulong currentTrackingId = 0;

    public bool debugEnabled;
    public SkinnedMeshRenderer actorMesh;
    public List<BlendshapeMapping> blendshapeMapping;
    private Dictionary<string, float> tempAus;
    private string[] actorBlendshapeNames;


    private ulong CurrentTrackingId

    {

        get

        {

            return currentTrackingId;

        }

        set

        {

            currentTrackingId = value;

        }

    }

    private FaceModel CurrentFaceModel

    {

        get

        {

            return currentFaceModel;

        }

        set

        {

            if (currentFaceModel != null)

            {

                currentFaceModel.Dispose();

                currentFaceModel = null;

            }

            currentFaceModel = value;

        }

    }

    private static double VectorLength(CameraSpacePoint point)

    {

        var result = Mathf.Pow(point.X, 2) + Mathf.Pow(point.Y, 2) + Mathf.Pow(point.Z, 2);

        result = Mathf.Sqrt(result);

        return result;

    }

    private static Body FindClosestBody(BodyFrame bodyFrame)

    {

        Body result = null;

        double closestBodyDistance = double.MaxValue;

        Body[] bodies = new Body[bodyFrame.BodyCount];

        bodyFrame.GetAndRefreshBodyData(bodies);

        foreach (var body in bodies)

        {

            if (body.IsTracked)

            {

                var currentLocation = body.Joints[JointType.SpineBase].Position;

                var currentDistance = VectorLength(currentLocation);

                if (result == null || currentDistance < closestBodyDistance)

                {

                    result = body;

                    closestBodyDistance = currentDistance;

                }

            }

        }

        return result;

    }

    private static Body FindBodyWithTrackingId(BodyFrame bodyFrame, ulong trackingId)

    {

        Body result = null;

        Body[] bodies = new Body[bodyFrame.BodyCount];

        bodyFrame.GetAndRefreshBodyData(bodies);

        foreach (var body in bodies)

        {

            if (body.IsTracked)

            {

                if (body.TrackingId == trackingId)

                {

                    result = body;

                    break;

                }

            }

        }

        return result;

    }

    private Mesh theGeometry;

    void Start()
    {  //this like InitializeHDFace()

        theGeometry = new Mesh();

        //SetViewCollectionStatus();

        sensor = KinectSensor.GetDefault();

        bodySource = sensor.BodyFrameSource;

        bodyReader = bodySource.OpenReader();

        bodyReader.FrameArrived += BodyReader_FrameArrived;

        highDefinitionFaceFrameSource = HighDefinitionFaceFrameSource.Create(sensor);

        highDefinitionFaceFrameSource.TrackingIdLost += HdFaceSource_TrackingIdLost;

        highDefinitionFaceFrameReader = highDefinitionFaceFrameSource.OpenReader();

        highDefinitionFaceFrameReader.FrameArrived += HdFaceReader_FrameArrived;

        CurrentFaceModel = FaceModel.Create();

        currentFaceAlignment = FaceAlignment.Create();

        sensor.Open();

        tempAus = new Dictionary<string, float>();
        actorBlendshapeNames = getBlendShapeNames(actorMesh);

    }

    private void OnApplicationQuit()
    {
        sensor.Close();

        bodyReader.FrameArrived -= BodyReader_FrameArrived;
        highDefinitionFaceFrameSource.TrackingIdLost -= HdFaceSource_TrackingIdLost;
        highDefinitionFaceFrameReader.FrameArrived -= HdFaceReader_FrameArrived;
    }

    private void InitializeMesh(MeshFilter faceMeshFilter, Mesh faceMesh)

    {

        faceMeshFilter.mesh.Clear();

        var vertices = this.currentFaceModel.CalculateVerticesForAlignment(this.currentFaceAlignment);

        Debug.Log("FaceModel.TriangleCount " + FaceModel.TriangleCount);

        var triangleIndices = FaceModel.TriangleIndices;

        var indices = new int[triangleIndices.Count];

        Vector3[] newVerts = new Vector3[vertices.Count];

        Vector2[] newUV = new Vector2[vertices.Count];

        Vector3[] newNormals = new Vector3[vertices.Count];

        for (int i = 0; i < vertices.Count; ++i)

        {

            newVerts[i] = new Vector3(vertices[i].X, vertices[i].Y, vertices[i].Z);

            newNormals[i] = -Vector3.forward;

        }

        faceMesh.vertices = newVerts;

        faceMesh.uv = newUV;

        for (int i = 0; i < triangleIndices.Count; i += 3)

        {

            uint index01 = triangleIndices[i];

            uint index02 = triangleIndices[i + 1];

            uint index03 = triangleIndices[i + 2];

            indices[i] = (int)index01;

            indices[i + 1] = (int)index02;

            indices[i + 2] = (int)index03;

            if (indices[i] >= triangleIndices.Count || indices[i] < 0)

            {

                Debug.Log("Out of Range index i " + i + " = " + indices[i]);

            }

            if (indices[i + 1] >= triangleIndices.Count || indices[i + 1] < 0)

            {

                Debug.Log("Out of Range index i+1 " + (i + 1) + " = " + indices[i + 1]);

            }

            if (indices[i + 2] >= triangleIndices.Count || indices[i + 2] < 0)

            {

                Debug.Log("Out of Range index i+2 " + (i + 2) + " = " + indices[i + 2]);

            }

        }

        faceMesh.triangles = indices;

        faceMesh.RecalculateNormals();

        faceMeshFilter.mesh = faceMesh;

    }

    private void UpdateMesh()

    {

        var vertices = this.currentFaceModel.CalculateVerticesForAlignment(this.currentFaceAlignment);

        //currentFaceModel.FaceShapeDeformations[FaceShapeDeformations.Eyes00]

        Vector3[] newVerts = new Vector3[vertices.Count];

        for (int i = 0; i < vertices.Count; ++i)

        {

            newVerts[i] = new Vector3(vertices[i].X, vertices[i].Y, vertices[i].Z);

        }

        this.theGeometry.vertices = newVerts;

    }

    void Update()
    {

        UpdateMesh();

    }

    private void BodyReader_FrameArrived(object sender, BodyFrameArrivedEventArgs e)

    {

        var frameReference = e.FrameReference;

        using (var frame = frameReference.AcquireFrame())

        {

            if (frame == null)

            {

                // We might miss the chance to acquire the frame, it will be null if it's missed

                return;

            }

            if (currentTrackedBody != null)

            {

                currentTrackedBody = FindBodyWithTrackingId(frame, CurrentTrackingId);

                if (currentTrackedBody != null)

                {

                    return;

                }

            }

            Body selectedBody = FindClosestBody(frame);

            if (selectedBody == null)

            {

                return;

            }

            currentTrackedBody = selectedBody;

            CurrentTrackingId = selectedBody.TrackingId;

            highDefinitionFaceFrameSource.TrackingId = CurrentTrackingId;

        }

    }

    private void HdFaceSource_TrackingIdLost(object sender, TrackingIdLostEventArgs e)

    {

        var lostTrackingID = e.TrackingId;

        if (CurrentTrackingId == lostTrackingID)

        {

            CurrentTrackingId = 0;

            currentTrackedBody = null;

            highDefinitionFaceFrameSource.TrackingId = 0;

        }

    }

    private string[] getBlendShapeNames(SkinnedMeshRenderer head)
    {
        Mesh m = head.sharedMesh;
        string[] arr;
        arr = new string[m.blendShapeCount];
        for (int i = 0; i < m.blendShapeCount; i++)
        {
            string s = m.GetBlendShapeName(i);
            print("Blend Shape: " + i + " " + s);
            arr[i] = s;
        }
        return arr;
    }

    private void UpdateActormeshBlendshapes(Dictionary<string, float> aus)
    {
        foreach (string key in aus.Keys)
        {
            foreach (BlendshapeMapping map in blendshapeMapping)
            {
                if (map.from.ToLower() == key.ToLower())
                {
                    float sourceVal = aus[key] + map.offset;
                    foreach (StringFloat sf in map.to)
                    {
                        int destIndex = -1;
                        for (int i=0; i< actorBlendshapeNames.Length; i++)
                        {
                            if (actorBlendshapeNames[i].ToLower() == sf.shapeName.ToLower())
                            {
                                destIndex = i;
                                break;
                            }
                        }
                        if (destIndex >= 0)
                        {
                            float destVal = sourceVal * sf.multiplier * 100f; // map from 0-1 to 0-100
                            if (destVal < 0f)
                                destVal = 0f;
                            actorMesh.SetBlendShapeWeight(destIndex, destVal);
                        }
                    }
                }
            }
        }
    }

    private void HdFaceReader_FrameArrived(object sender, HighDefinitionFaceFrameArrivedEventArgs e)

    {

        using (var frame = e.FrameReference.AcquireFrame())

        {

            // We might miss the chance to acquire the frame; it will be null if it's missed.

            // Also ignore this frame if face tracking failed.

            if (frame == null || !frame.IsFaceTracked)

            {

                return;

            }

            frame.GetAndRefreshFaceAlignmentResult(currentFaceAlignment);
            if (debugEnabled)
            {
                string aus = "";
                foreach (FaceShapeAnimations key in currentFaceAlignment.AnimationUnits.Keys)
                {
                    aus += key.ToString();
                    aus += currentFaceAlignment.AnimationUnits[key].ToString("0.00") + "; ";
                    Debug.Log(aus);
                }
            }

            if (actorMesh != null)
            {
                tempAus.Clear();
                foreach (FaceShapeAnimations key in currentFaceAlignment.AnimationUnits.Keys)
                {
                    tempAus.Add(key.ToString(), currentFaceAlignment.AnimationUnits[key]);
                }
                UpdateActormeshBlendshapes(tempAus);
            }
        }

    }

}


```

## Source file: StringFloat.cs

{
  "path": "StringFloat.cs",
  "blob_sha": "2d7a91d2577a03f01c3491a1918a74f4188663bd",
  "sha256": "3231dc201d62140bef5a9c3a926189fa71412843186d486274c4789595fcd93c",
  "lines": 10,
  "bytes": 228,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/woroko/Kinect2Blendshapes/blob/455af0088b711fad88bf4086cf508f435154641a/StringFloat.cs",
  "function_signature_candidates": []
}

```text

﻿using System.Collections;
using System.Collections.Generic;
using UnityEngine;

[System.Serializable]
public class StringFloat : System.Object
{
    public string shapeName = "";
    public float multiplier = 1f;
}


```
