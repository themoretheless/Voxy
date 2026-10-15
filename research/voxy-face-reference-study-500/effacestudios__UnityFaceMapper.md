# Facial rendering source scan: effacestudios/UnityFaceMapper

Repository: https://github.com/effacestudios/UnityFaceMapper

Pinned source revision: 7aa4953b726e01126bc49c0843eecaf82bd28f06

Discovery topic: character creator 3D

Description: Image to 3D Character Creator for Unity in C#

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"shader": ["Library/ShaderCache.db", "Library/ShaderCache/0/002b58be7c3e7374f19e6d487d762980.bin", "Library/ShaderCache/0/01af25d4589f5ada85837230535242c0.bin", "Library/ShaderCache/0/020102cc819f0cf3369f617fee78de3f.bin", "Library/ShaderCache/0/04fd28a04786100854c7c14292884eff.bin", "Library/ShaderCache/0/056678377624cfd4d05a55c1dd9b994d.bin", "Library/ShaderCache/0/075a5cc4bc7847dddee287e189b24ef8.bin", "Library/ShaderCache/0/08c1153d3dbe1d4b35d3c72184c4efd5.bin", "Library/ShaderCache/0/0a7b9d78acf7777694439441ed6086db.bin", "Library/ShaderCache/0/0cc47be46d6b758a73f73602105b6514.bin", "Library/ShaderCache/0/0ef9b56f73396b90777f1e1d0af1e2de.bin", "Library/ShaderCache/0/0f9f21d487d227a8e2f1ff48dd8a1830.bin", "Library/ShaderCache/1/11582d3b3379c4a37a7c311f74d2a4a1.bin", "Library/ShaderCache/1/12ebcc134f8bbfaa1d94f981ad73e2e3.bin", "Library/ShaderCache/1/12f06620ea3440614cfbf239f1a3fe16.bin", "Library/ShaderCache/1/16aa283808e8e9f6ed916590a5a71da7.bin", "Library/ShaderCache/1/180ce9f7ac9a79481f789f20a805af33.bin", "Library/ShaderCache/1/1fe06a7478c78f21d60ee6af96f4eefe.bin", "Library/ShaderCache/2/20f403ea78f14b9daff9263ea9f24d0f.bin", "Library/ShaderCache/2/26830d927e68c5dfbd37721f89b4ac66.bin", "Library/ShaderCache/2/26bcb1e1b017d92365734edb8979fa25.bin", "Library/ShaderCache/2/27f4fea3b144b3a102bf618b1353bee7.bin", "Library/ShaderCache/3/310d530238e510583761c2c04e42c498.bin", "Library/ShaderCache/3/3a6703434c8e75dfba7bb4cfc82ef059.bin", "Library/ShaderCache/3/3d3e3546bbc62e96334376e82ea4396e.bin", "Library/ShaderCache/4/40b16c69ba22885613a92436bbbf2bed.bin", "Library/ShaderCache/4/42dd4a63c6431128266970e20b000b9d.bin", "Library/ShaderCache/4/44b66cb2a478e383b4991c2dea1d24b9.bin", "Library/ShaderCache/4/49938184027e3262ff982afde1670621.bin", "Library/ShaderCache/5/56e28cc59d9566a19ebed9cc63ae4392.bin"], "face": [".vs/UnityFaceMapper-master/v15/.suo", ".vs/UnityFaceMapper-master/v15/Server/sqlite3/db.lock", ".vs/UnityFaceMapper-master/v15/Server/sqlite3/storage.ide", ".vs/UnityFaceMapper-master/v15/Server/sqlite3/storage.ide-shm", ".vs/UnityFaceMapper-master/v15/Server/sqlite3/storage.ide-wal", ".vs/UnityFaceMapper/v14/.suo", ".vs/UnityFaceMapper/v15/.suo", ".vs/UnityFaceMapper/v15/Server/sqlite3/db.lock", ".vs/UnityFaceMapper/v15/Server/sqlite3/storage.ide", ".vs/UnityFaceMapper/v15/Server/sqlite3/storage.ide-shm", ".vs/UnityFaceMapper/v15/Server/sqlite3/storage.ide-wal", "Assets/FaceContour.cs", "Assets/FaceContour.cs.meta", "Assets/FaceElement.cs", "Assets/FaceElement.cs.meta", "Assets/Prefabs/FaceContour.prefab", "Assets/Prefabs/FaceContour.prefab.meta", "Assets/Resources/Face.png", "Assets/Resources/Face.png.meta", "Assets/Resources/ManFace.jpg", "Assets/Resources/ManFace.jpg.meta", "Assets/Resources/ManFace2.jpg", "Assets/Resources/ManFace2.jpg.meta", "Assets/Resources/ManFace3.jpg", "Assets/Resources/ManFace3.jpg.meta", "Assets/Resources/face_PNG11760.png", "Assets/Resources/face_PNG11760.png.meta", "Assets/Resources/textures/Face.png", "Assets/Resources/textures/Face.png.meta", "Assets/faceMask.cs"], "lip": ["Assets/Ellipse.cs", "Assets/Ellipse.cs.meta", "Assets/EllipseContour.cs", "Assets/EllipseContour.cs.meta", "Assets/Lips.prefab", "Assets/Lips.prefab.meta", "Assets/Prefabs/EllipseContour.prefab", "Assets/Prefabs/EllipseContour.prefab.meta"]}

## Source file: Assets/Ellipse.cs

{
  "path": "Assets/Ellipse.cs",
  "blob_sha": "7b69889271cb280d22b2aac678a6624f945635ea",
  "sha256": "d2ed8e9d270aa125b34c91b77e072201c3d598b53a5fb7a77fd833b862d24268",
  "lines": 45,
  "bytes": 1219,
  "selection_score": 15,
  "keyword_line_counts": {
    "lip": 3
  },
  "url": "https://github.com/effacestudios/UnityFaceMapper/blob/7aa4953b726e01126bc49c0843eecaf82bd28f06/Assets/Ellipse.cs",
  "function_signature_candidates": [
    {
      "line": 17,
      "text": "void Update()"
    }
  ]
}

```text

﻿using UnityEngine;
using System.Collections;

[RequireComponent(typeof(LineRenderer))]
public class Ellipse : MonoBehaviour
{

    public float a = 5;
    public float b = 3;
    public float h = 1;
    public float k = 1;
    public float theta = 45;
    public int resolution = 1000;

    private Vector3[] positions;

    void Update()
    {
        positions = CreateEllipse(a, b, h, k, theta, resolution);
        LineRenderer lr = GetComponent<LineRenderer>();
        lr.SetVertexCount(resolution + 1);
        for (int i = 0; i <= resolution; i++)
        {
            lr.SetPosition(i, positions[i]);
        }
    }


    Vector3[] CreateEllipse(float a, float b, float h, float k, float theta, int resolution)
    {

        positions = new Vector3[resolution + 1];
        Quaternion q = Quaternion.AngleAxis(theta, Vector3.forward);
        Vector3 center = new Vector3(h, k, 0.0f);

        for (int i = 0; i <= resolution; i++)
        {
            float angle = (float)i / (float)resolution * 2.0f * Mathf.PI;
            positions[i] = new Vector3(a * Mathf.Cos(angle), b * Mathf.Sin(angle), 0.0f);
            positions[i] = q * positions[i] + center;
        }

        return positions;
    }
}

```

## Source file: Assets/faceMask.cs

{
  "path": "Assets/faceMask.cs",
  "blob_sha": "b9c9e730ff2a32aa1778db1f571bb52398cdea2c",
  "sha256": "d029434ca8a37f5020045b4f2b0d4de48f335558f28698be10143758617bde26",
  "lines": 53,
  "bytes": 1501,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 1,
    "face": 1
  },
  "url": "https://github.com/effacestudios/UnityFaceMapper/blob/7aa4953b726e01126bc49c0843eecaf82bd28f06/Assets/faceMask.cs",
  "function_signature_candidates": []
}

```text

﻿using UnityEngine;
using System.Collections;
using System.Collections.Generic;
using System;
using System.Linq;

public class faceMask : MonoBehaviour {
    public List<Vector2> positions = new List<Vector2> { 
        new Vector2(319, 110),
        new Vector2(398, 141),
        new Vector2(483, 203),
        new Vector2(519, 260),
        new Vector2(542, 338),
        new Vector2(548, 566),
        new Vector2(521, 649),
        new Vector2(480, 677),
        new Vector2(413, 693),
        new Vector2(234, 694),
        new Vector2(161, 679),
        new Vector2(117, 647),
        new Vector2(93, 567),
        new Vector2(97, 340),
        new Vector2(118, 266),
        new Vector2(158, 199),
        new Vector2(257, 131),
        };

    public List<Vector2> GetScreenCoordinates()
	{
		var worldPoints = new List<Vector2> ();
		var texture = GetComponent<Renderer> ().material.mainTexture;
		var width = texture.width;
		var height = texture.height;

		foreach (var point in positions) {

			var newPoint = new Vector3();

			newPoint.x = (float)(point.x/width - 0.5);
			newPoint.y = (float)(point.y / height - 0.5);
            newPoint.z = GetComponent<Transform>().position.z;

			Vector2 finalPoint = Camera.main.WorldToScreenPoint(GetComponent<Transform>().localToWorldMatrix.MultiplyPoint3x4(newPoint));

			worldPoints.Add(finalPoint);

			Debug.Log("Original point: " + point + ", local coord: " + newPoint + ", finalPoint: " + finalPoint);

		}

		return worldPoints;
	}
}


```

## Source file: Assets/FaceContour.cs

{
  "path": "Assets/FaceContour.cs",
  "blob_sha": "29d546dc14b21ddabd1495514aa6c58293d0fa37",
  "sha256": "f9e1423407ee6188f5049fa3f7f1d1e2a61d6cda8b376494b722da6135855860",
  "lines": 23,
  "bytes": 693,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 1
  },
  "url": "https://github.com/effacestudios/UnityFaceMapper/blob/7aa4953b726e01126bc49c0843eecaf82bd28f06/Assets/FaceContour.cs",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "void Update() {"
    }
  ]
}

```text

﻿using System.Collections;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;

public class FaceContour : FaceElement {
    public LineRenderer lineRenderer;
    public List<ControlPoint> points = new List<ControlPoint>();

    // Update is called once per frame
    void Update() {
        lineRenderer.positionCount = points.Count + 1;
        var cps = points.Select(p => p.gameObject.transform.position).ToList();
        cps.Add(cps.First());
        lineRenderer.SetPositions(cps.ToArray());
    }

    public override List<Point> LockAndReturnPoints()
    {
        foreach (var p in points) p.Lock();
        return points.Select(p => p.Point).ToList();
    }
}


```

## Source file: Assets/FaceElement.cs

{
  "path": "Assets/FaceElement.cs",
  "blob_sha": "9019679ac54dba606f589874c6a025e029641715",
  "sha256": "5bbc943f38e011d710c49424b717f4a96dd1c3aaa10f026f6719b23f09c4ebb9",
  "lines": 7,
  "bytes": 192,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 1
  },
  "url": "https://github.com/effacestudios/UnityFaceMapper/blob/7aa4953b726e01126bc49c0843eecaf82bd28f06/Assets/FaceElement.cs",
  "function_signature_candidates": []
}

```text

﻿using System.Collections;
using System.Collections.Generic;
using UnityEngine;

public abstract class FaceElement : MonoBehaviour {
    public abstract List<Point> LockAndReturnPoints();
}


```
