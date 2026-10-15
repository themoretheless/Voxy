# Facial rendering source scan: needle-mirror/com.unity.xr.arkit-face-tracking

Repository: https://github.com/needle-mirror/com.unity.xr.arkit-face-tracking

Pinned source revision: 1b81f6d4c4ae3a1eedf39f5f5017f4497586d0f9

Discovery topic: ARKit blendshape

Description: Support for face tracking on ARKit.Includes support for: ▪ Face pose tracking ▪ Blendshapes Note: Only supported on devices with a forward-facing depth camera, like the iPhone X 📦 [Mirrored from UPM, not affiliated with Unity Technologies.]

Reported license: NOASSERTION

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE.md", "LICENSE.md.meta"]

Topic paths: {"face": ["Editor/Unity.XR.ARKit.FaceTracking.Editor.asmdef", "Editor/Unity.XR.ARKit.FaceTracking.Editor.asmdef.meta", "Tests/Editor/Unity.XR.ARKit.FaceTracking.EditorTests.asmdef", "Tests/Editor/Unity.XR.ARKit.FaceTracking.EditorTests.asmdef.meta", "Tests/Runtime/Unity.XR.ARKit.FaceTracking.Tests.asmdef", "Tests/Runtime/Unity.XR.ARKit.FaceTracking.Tests.asmdef.meta"]}

## Source file: Editor/DeprecratedWarning.cs

{
  "path": "Editor/DeprecratedWarning.cs",
  "blob_sha": "7fb606167d6148c99359b46323468706ac1c4c91",
  "sha256": "f3f489d24d64e5e3c170d3d343e3da37b1d24e361d96dee46171593fbf2334ad",
  "lines": 36,
  "bytes": 1356,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 4
  },
  "url": "https://github.com/needle-mirror/com.unity.xr.arkit-face-tracking/blob/1b81f6d4c4ae3a1eedf39f5f5017f4497586d0f9/Editor/DeprecratedWarning.cs",
  "function_signature_candidates": [
    {
      "line": 15,
      "text": "static void ShowWarning()"
    }
  ]
}

```text

using UnityEngine;

namespace UnityEditor.XR.ARKit
{
    /// <summary>
    /// This class displays a warning that com.unity.xr.arkit-face-tracking is now deprecated.
    /// </summary>
    class DeprecatedWarning
    {
        const string k_Title = "Deprecated: Apple ARKit Face Tracking XR Plug-in Package";
        const string k_Message = "Apple ARKit Face Tracking XR Plug-in package has been deprecated. It is now a part of Apple ARKit XR Plug-in package. If you have an explicit dependency on Apple ARKit Face Tracking XR Plug-in package then replace it with Apple ARKit XR Plug-in package in the manifest.json file.";
        const string k_HideWarningKey = "HideARKitFaceTrackingDeprecatedWarning";

        [InitializeOnLoadMethod]
        static void ShowWarning()
        {
            if (Application.isBatchMode)
            {
                return;
            }

            if (EditorUserSettings.GetConfigValue(k_HideWarningKey)?.Equals("true") ?? false)
            {
                return;
            }

            var hideWarning = !EditorUtility.DisplayDialog(
                k_Title,
                k_Message,
                "Understood",
                "Don't warn me again for this project"
            );
            EditorUserSettings.SetConfigValue(k_HideWarningKey, hideWarning.ToString().ToLower());
        }
    }
}


```

## Source file: Tests/Editor/EditorExampleTest.cs

{
  "path": "Tests/Editor/EditorExampleTest.cs",
  "blob_sha": "06dd0a117fcb68f17d17b9bf98c2bdaeda6e2b37",
  "sha256": "95c2a87dd6b6123f1127fbe98e65d32b6c8a8872db4d4c19ae049fe8030987f8",
  "lines": 22,
  "bytes": 538,
  "selection_score": -2,
  "keyword_line_counts": {},
  "url": "https://github.com/needle-mirror/com.unity.xr.arkit-face-tracking/blob/1b81f6d4c4ae3a1eedf39f5f5017f4497586d0f9/Tests/Editor/EditorExampleTest.cs",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "public void EditorSampleTestSimplePasses() {"
    }
  ]
}

```text

﻿using UnityEngine;
using UnityEditor;
using UnityEngine.TestTools;
using NUnit.Framework;
using System.Collections;

class EditorExampleTest {

	[Test]
	public void EditorSampleTestSimplePasses() {
		// Use the Assert class to test conditions.
	}

	// A UnityTest behaves like a coroutine in PlayMode
	// and allows you to yield null to skip a frame in EditMode
	[UnityTest]
	public IEnumerator EditorSampleTestWithEnumeratorPasses() {
		// Use the Assert class to test conditions.
		// yield to skip a frame
		yield return null;
	}
}


```

## Source file: Tests/Runtime/RuntimeExampleTest.cs

{
  "path": "Tests/Runtime/RuntimeExampleTest.cs",
  "blob_sha": "a37f6982a62e50aaf36ca84c35aa46d025dd61ed",
  "sha256": "b3e5be81b0212edaaf81e0499f845b612db68915e502d5d0db9256c4e7ec8c93",
  "lines": 28,
  "bytes": 760,
  "selection_score": -2,
  "keyword_line_counts": {
    "face": 1
  },
  "url": "https://github.com/needle-mirror/com.unity.xr.arkit-face-tracking/blob/1b81f6d4c4ae3a1eedf39f5f5017f4497586d0f9/Tests/Runtime/RuntimeExampleTest.cs",
  "function_signature_candidates": [
    {
      "line": 13,
      "text": "public void PlayModeSampleTestSimplePasses()"
    }
  ]
}

```text

﻿using UnityEngine.TestTools;
using NUnit.Framework;
using System.Collections;
using System.Collections.Generic;
using System.Linq;

namespace UnityEngine.XR.ARKit.FaceTracking.Tests
{
    [TestFixture]
    public class RuntimeExampleTest
    {
        [Test]
        public void PlayModeSampleTestSimplePasses()
        {
            // Use the Assert class to test conditions.
        }

        // A UnityTest behaves like a coroutine in PlayMode
        // and allows you to yield null to skip a frame in EditMode
        [UnityTest]
        public IEnumerator PlayModeSampleTestWithEnumeratorPasses()
        {
            // Use the Assert class to test conditions.
            // yield to skip a frame
            yield return null;
        }
    }
}


```
