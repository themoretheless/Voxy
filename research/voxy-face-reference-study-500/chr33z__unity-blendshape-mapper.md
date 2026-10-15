# Facial rendering source scan: chr33z/unity-blendshape-mapper

Repository: https://github.com/chr33z/unity-blendshape-mapper

Pinned source revision: c0471b88fc5da419fc429735413fef3620c27ba1

Discovery topic: blendshape

Description: A unity tool that helps to map blendshape animations to characters

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"blendshape": ["Editor/AnimationBlendShapeMappingEditor.cs"]}

## Source file: Editor/AnimationBlendShapeMappingEditor.cs

{
  "path": "Editor/AnimationBlendShapeMappingEditor.cs",
  "blob_sha": "d767396ea51404b3d1330fae5eef44265cf374f5",
  "sha256": "cdb1dd4eece4ef1d595569912dcb50c904cea000d7fa718de7b9fe5d526cf44b",
  "lines": 429,
  "bytes": 15893,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 32,
    "blendshape": 56,
    "lip": 18
  },
  "url": "https://github.com/chr33z/unity-blendshape-mapper/blob/c0471b88fc5da419fc429735413fef3620c27ba1/Editor/AnimationBlendShapeMappingEditor.cs",
  "function_signature_candidates": [
    {
      "line": 34,
      "text": "private static void ShowWindow()"
    },
    {
      "line": 39,
      "text": "private void OnSelectionChange()"
    },
    {
      "line": 53,
      "text": "private void OnInspectorUpdate()"
    },
    {
      "line": 59,
      "text": "private void OnGUI()"
    },
    {
      "line": 115,
      "text": "private void DrawCredits()"
    },
    {
      "line": 127,
      "text": "private void DrawMappingsTable()"
    },
    {
      "line": 220,
      "text": "private bool CanApplyBlendShapeMapping()"
    },
    {
      "line": 225,
      "text": "private void ApplyBlendShapeMapping()"
    },
    {
      "line": 258,
      "text": "private bool ShowConfirmApplyBlendShapeMapping(int changedBlendshapes, int totalBlendshapes)"
    },
    {
      "line": 265,
      "text": "private void CreateAutomaticMapping()"
    },
    {
      "line": 309,
      "text": "private void ClearMapping()"
    }
  ]
}

```text

﻿#if UNITY_EDITOR

using UnityEngine;
using UnityEditor;
using System.Collections.Generic;
using System.Linq;
using Chreez.Editor.Tools.Util;

namespace Chreez.Editor.Tools
{
    public class BlendShapeMapping
    {
        public string Source { get; set; }
        public string Target { get; set; }
    }

    public class AnimationBlendShapeMappingEditor : EditorWindow
    {
        private static readonly int columnWidth = 200;
        private static readonly int buttonHeight = 22;
		
		private static readonly double automaticMappingSensitivity = 0.3;

        private Animator animatorReference;
        private SkinnedMeshRenderer skinnedMeshRendererReference;
        private AnimationClip animationClip;

        private Dictionary<string, int> _blendShapeMappings = new Dictionary<string, int>();
        private List<string> _blendShapeOptions = new List<string>();

        private Vector2 scrollPos = Vector2.zero;

        [MenuItem("Window/Animation BlendShape Mapping Editor")]
        private static void ShowWindow()
        {
            GetWindow<AnimationBlendShapeMappingEditor>(TextResource.Title);
        }

        private void OnSelectionChange()
        {
            if (Selection.activeObject is AnimationClip)
            {
                animationClip = (AnimationClip)Selection.activeObject;
            }
            else
            {
                animationClip = null;
                _blendShapeMappings.Clear();
            }
            Repaint();
        }

        private void OnInspectorUpdate()
        {
            _blendShapeOptions = GetBlendShapeOptions(skinnedMeshRendererReference);
            Repaint();
        }

        private void OnGUI()
        {
            scrollPos = GUILayout.BeginScrollView(scrollPos, Styles.ScrollViewStyle);

            // Title
            GUILayout.Label(TextResource.Label_Title, Styles.TitleStyle);
            EditorGUILayout.LabelField("", GUI.skin.horizontalSlider);

            // Information
            GUILayout.Box(TextResource.Text_Intro, Styles.TextBoxStyle);

            if (animationClip)
            {
                // Animation Clip
                EditorGUILayout.BeginHorizontal();
                GUILayout.Label("Animation Clip:", GUILayout.Width(columnWidth));
                animationClip = ((AnimationClip)EditorGUILayout.ObjectField(
                    animationClip,
                    typeof(AnimationClip),
                    true));
                EditorGUILayout.EndHorizontal();

                // Facial Skinned Mesh Renderer
                EditorGUILayout.BeginHorizontal();
                GUILayout.Label("Facial SkinnedMeshRenderer:", GUILayout.Width(columnWidth));
                skinnedMeshRendererReference = ((SkinnedMeshRenderer)EditorGUILayout.ObjectField(
                    skinnedMeshRendererReference,
                    typeof(SkinnedMeshRenderer),
                    true));
                EditorGUILayout.EndHorizontal();

                // Character Animator
                EditorGUILayout.BeginHorizontal();
                GUILayout.Label("Character Animator (optional):", GUILayout.Width(columnWidth));

                animatorReference = ((Animator)EditorGUILayout.ObjectField(
                    animatorReference,
                    typeof(Animator),
                    true));

                EditorGUILayout.EndHorizontal();

                // Table Blendshape Mapping
                DrawMappingsTable();
            }
            else
            {
                GUILayout.Label("Please select an AnimationClip from Assets...");
            }

            EditorGUILayout.LabelField("", GUI.skin.horizontalSlider);
            DrawCredits();

            GUILayout.EndScrollView();
        }

        private void DrawCredits()
        {
            if (GUILayout.Button(TextResource.Text_Credits, Styles.Credits, GUILayout.ExpandWidth(false)))
            {
                Application.OpenURL("www.github.com/chr33z");
            }
            var lastRect = GUILayoutUtility.GetLastRect();
            lastRect.y += lastRect.height - 1;
            lastRect.height = 1;
            GUI.Box(lastRect, "");
        }

        private void DrawMappingsTable()
        {
            if (!animationClip || !skinnedMeshRendererReference)
                return;

            GUILayout.Space(20);

            // Button Apply Blendshape Mapping
            EditorGUILayout.BeginHorizontal();
            GUI.enabled = CanApplyBlendShapeMapping();
            GUILayout.Label("", GUILayout.Width(columnWidth));
            if (GUILayout.Button(
                new GUIContent(
                    "Apply & Save Blendshape Mapping",
                    TextResource.Tooltip_ApplyMapping
                ),
                Styles.ApplyMappingButtonStyle,
                new GUILayoutOption[]
                {
                    GUILayout.Height((int)(buttonHeight * 1.5)),
                }
                ))
            {
                ApplyBlendShapeMapping();
            }
            GUI.enabled = true;
            EditorGUILayout.EndHorizontal();

            // Button Automatic Mapping
            GUILayout.Space(5);
            EditorGUILayout.BeginHorizontal();
            GUILayout.Label("", GUILayout.Width(columnWidth));
            if (GUILayout.Button(
                new GUIContent(
                    "Create Automatic Mapping",
                    TextResource.Tooltip_AutomaticMapping
                ),
                new GUILayoutOption[]
                {
                    GUILayout.Height(buttonHeight)
                }
                ))
            {
                CreateAutomaticMapping();
            }
            EditorGUILayout.EndHorizontal();

            // Button Clear Mapping
            EditorGUILayout.BeginHorizontal();
            GUILayout.Label("", GUILayout.Width(columnWidth));
            if (GUILayout.Button(
                new GUIContent(
                    "Clear Current Mapping",
                    TextResource.Tooltip_ClearMapping
                ), GUILayout.Height(buttonHeight)
                ))
            {
                ClearMapping();
            }
            EditorGUILayout.EndHorizontal();

            EditorGUILayout.LabelField("", GUI.skin.horizontalSlider);

            // Table Header
            EditorGUILayout.BeginHorizontal();
            GUILayout.Label("Animation Blendshapes", Styles.TableHeaderStyle, GUILayout.Width(columnWidth));
            GUILayout.Label("Mapped Blendshapes", Styles.TableHeaderStyle, GUILayout.Width(columnWidth));
            EditorGUILayout.EndHorizontal();

            EditorGUILayout.LabelField("", GUI.skin.horizontalSlider);

            // Mappings Table
            var curveBindings = AnimationUtility.GetCurveBindings(animationClip);
            foreach (var curveBinding in curveBindings)
            {
                if (!curveBinding.propertyName.Contains("blendShape"))
                    continue;

                var source = curveBinding.propertyName.Replace("blendShape.", "");

                if (!_blendShapeMappings.ContainsKey(source))
                {
                    _blendShapeMappings[source] = 0;
                }

                EditorGUILayout.BeginHorizontal();
                GUILayout.Label(source, GUILayout.Width(columnWidth));
                var selectedIndex = _blendShapeMappings[source];
                _blendShapeMappings[source] = EditorGUILayout.Popup(selectedIndex, _blendShapeOptions.ToArray());
                EditorGUILayout.EndHorizontal();
            }
        }

        private bool CanApplyBlendShapeMapping()
        {
            return _blendShapeMappings.Values.Any(val => val > 0);
        }

        private void ApplyBlendShapeMapping()
        {
            var availableBlendshapes = skinnedMeshRendererReference.sharedMesh.blendShapeCount;
            var changedBlendshapes = _blendShapeMappings.ToArray().Where(mapping => mapping.Value > 0).Count();

            if (!ShowConfirmApplyBlendShapeMapping(changedBlendshapes, availableBlendshapes))
            {
                return;
            }

            var curveBindings = AnimationUtility.GetCurveBindings(animationClip);
            var relativePath = GetPathFromSkinnedMeshToAnimator();
            for (var i = 0; i < curveBindings.Length; i++)
            {
                var curveBinding = curveBindings[i];
                var key = curveBinding.propertyName.Replace("blendShape.", "");
                var mappingName = GetBlendShapeMappingName(key);

                if (_blendShapeMappings.ContainsKey(key) && !string.IsNullOrEmpty(mappingName))
                {
                    AnimationCurve curve = AnimationUtility.GetEditorCurve(animationClip, curveBinding);

                    if (curve != null)
                    {
                        AnimationUtility.SetEditorCurve(animationClip, curveBinding, null);
                        curveBinding.propertyName = "blendShape." + mappingName;
                        curveBinding.path = relativePath;
                        AnimationUtility.SetEditorCurve(animationClip, curveBinding, curve);
                    }
                }
            }
        }

        private bool ShowConfirmApplyBlendShapeMapping(int changedBlendshapes, int totalBlendshapes)
        {
            var title = "Apply Blendshape Mapping";
            var message = string.Format(TextResource.Template_ApplyDialogMessage, changedBlendshapes, totalBlendshapes);
            return EditorUtility.DisplayDialog(title, message, "Apply Mapping", "Cancel");
        }

        private void CreateAutomaticMapping()
        {
            if (!animationClip || !skinnedMeshRendererReference)
                return;

            var blendShapesInAnimation = _blendShapeMappings.Keys.ToArray();
            var blendShapesInCharacter = GetBlendShapeNames(skinnedMeshRendererReference);

            foreach (var blendShapeInAnim in blendShapesInAnimation)
            {
                var bestMatch = MatchToNameInList(blendShapeInAnim, blendShapesInCharacter, automaticMappingSensitivity);
                if (!string.IsNullOrEmpty(bestMatch))
                {
                    _blendShapeMappings[blendShapeInAnim] = _blendShapeOptions.IndexOf(bestMatch);
                }
            }
        }

        private static string MatchToNameInList(string targetName, List<string> availableBlendShapeNames, double tolerance)
        {
            string bestMatch = "";
            double maxScore = double.MinValue;

            foreach (var blendShapeName in availableBlendShapeNames)
            {
                double score = targetName.DiceCoefficient(blendShapeName);

                if (score > maxScore)
                {
                    maxScore = score;
                    bestMatch = blendShapeName;
                }
            }

            if (maxScore > tolerance)
            {
                return bestMatch;
            }
            else
            {
                return "";
            }
        }

        private void ClearMapping()
        {
            _blendShapeMappings.Clear();
        }

        private string GetPathFromSkinnedMeshToAnimator()
        {
            if (!animatorReference || !skinnedMeshRendererReference)
                return skinnedMeshRendererReference.name;

            var obj = skinnedMeshRendererReference.gameObject;

            string path = obj.name;
            while (obj.transform.parent != null && !obj.transform.parent.name.Equals(animatorReference.gameObject.name))
            {
                obj = obj.transform.parent.gameObject;
                path = obj.name + "/" + path;
            }
            return path;
        }

        private string GetBlendShapeMappingName(string key)
        {
            return _blendShapeOptions[_blendShapeMappings[key]];
        }

        private List<string> GetBlendShapeOptions(SkinnedMeshRenderer skinnedMeshRenderer)
        {
            var options = new List<string> { "None" };
            options.AddRange(GetBlendShapeNames(skinnedMeshRenderer));
            return options;
        }

        private List<string> GetBlendShapeNames(SkinnedMeshRenderer skinnedMeshRenderer)
        {
            if (!skinnedMeshRendererReference)
            {
                return new List<string>();
            }

            var mesh = skinnedMeshRenderer.sharedMesh;
            return Enumerable.Range(0, mesh.blendShapeCount).Select(i => mesh.GetBlendShapeName(i)).ToList();
        }
    }

    internal class Styles
    {
        internal static GUIStyle TableHeaderStyle {
            get {
                var tableHeaderStyle = new GUIStyle(GUI.skin.label);
                tableHeaderStyle.fontStyle = FontStyle.Bold;
                return tableHeaderStyle;
            }
        }

        internal static GUIStyle TitleStyle {
            get {
                var titleStyle = new GUIStyle(GUI.skin.label);
                titleStyle.fontStyle = FontStyle.Bold;
                titleStyle.alignment = TextAnchor.MiddleCenter;
                titleStyle.margin = new RectOffset(10, 10, 10, 0);
                return titleStyle;
            }
        }

        internal static GUIStyle ScrollViewStyle {
            get {
                var scrollViewStyle = new GUIStyle(GUI.skin.scrollView);
                scrollViewStyle.padding = new RectOffset(10, 10, 10, 10);
                return scrollViewStyle;
            }
        }

        internal static GUIStyle TextBoxStyle {
            get {
                var textBoxStyle = new GUIStyle(GUI.skin.box);
                textBoxStyle.alignment = TextAnchor.MiddleLeft;
                textBoxStyle.margin = new RectOffset(10, 10, 0, 10);
                textBoxStyle.padding = new RectOffset(10, 10, 10, 10);
                return textBoxStyle;
            }
        }

        internal static GUIStyle ApplyMappingButtonStyle {
            get {
                var style = new GUIStyle(GUI.skin.button);
                style.fontStyle = FontStyle.Bold;
                style.normal.textColor = new Color(0.14f, 0.5f, 0.34f);
                style.hover.textColor = new Color(0.24f, 0.6f, 0.44f);
                return style;
            }
        }

        internal static GUIStyle Credits {
            get {
                var style = new GUIStyle(GUI.skin.label);
                style.fontSize = (int)(style.fontSize * 0.9);
                style.hover.textColor = new Color(0x00 / 255f, 0x78 / 255f, 0xDA / 255f, 1f);
                return style;
            }
        }
    }

    internal class TextResource
    {
        internal static string Title = "Animation Blendshape Mapping Editor";
        internal static string Label_Title = "Animation Blendshape Mapper";
        internal static string Tooltip_AutomaticMapping = "Try to automatically match animation blendshapes with the characters blendshapes using fuzzy name matching.";
        internal static string Tooltip_ApplyMapping = "Apply the blendshape mapping to the animation and save it. Overwrites the existing blendshape mapping.";
        internal static string Tooltip_ClearMapping = "Reset all mappings.";

        internal static string Text_Intro =
            @"This editor helps mapping blendshapes of animations to a character and corrects the animation hierarchy. After selecting the Animator and Skinned Mesh Renderer containing the blendshapes you can adjust the mappings below.";

        internal static string Text_Credits = "2020 @ Christopher Gebhardt | https://github.com/chr33z";

        internal static string Template_ApplyDialogMessage = "This action will change {0} of {1} available blendshapes in the selected character and overwrite this animation. Do you want to do proceed?";
    }
}

#endif


```

## Source file: Editor/Utils/DiceCoefficientExtension.cs

{
  "path": "Editor/Utils/DiceCoefficientExtension.cs",
  "blob_sha": "68168df3292168e0923f0df7c043143ed8786c23",
  "sha256": "e8f2a7738ebb885f2db624b4edeec99efc10ea13b52a81f6c011bf0392fce2ab",
  "lines": 73,
  "bytes": 2637,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 1
  },
  "url": "https://github.com/chr33z/unity-blendshape-mapper/blob/c0471b88fc5da419fc429735413fef3620c27ba1/Editor/Utils/DiceCoefficientExtension.cs",
  "function_signature_candidates": []
}

```text

﻿using System.Linq;

namespace Chreez.Editor.Tools.Util
{
    public static class DiceCoefficientExtension
    {
        /// <summary>
        /// Dice Coefficient based on bigrams. <br />
        /// A good value would be 0.33 or above, a value under 0.2 is not a good match, from 0.2 to 0.33 is iffy.
        /// </summary>
        /// <param name="input"></param>
        /// <param name="comparedTo"></param>
        /// <returns></returns>
        public static double DiceCoefficient(this string input, string comparedTo)
        {
            var ngrams = input.ToBiGrams();
            var compareToNgrams = comparedTo.ToBiGrams();
            return ngrams.DiceCoefficient(compareToNgrams);
        }

        /// <summary>
        /// Dice Coefficient used to compare nGrams arrays produced in advance.
        /// </summary>
        /// <param name="nGrams"></param>
        /// <param name="compareToNGrams"></param>
        /// <returns></returns>
        public static double DiceCoefficient(this string[] nGrams, string[] compareToNGrams)
        {
            int matches = 0;
            foreach (var nGram in nGrams)
            {
                if (compareToNGrams.Any(x => x == nGram))
                    matches++;
            }
            if (matches == 0)
                return 0.0d;
            double totalBigrams = nGrams.Length + compareToNGrams.Length;
            return (2 * matches) / totalBigrams;
        }

        public static string[] ToBiGrams(this string input)
        {
            // nLength == 2
            //   from Jackson, return %j ja ac ck ks so on n#
            //   from Main, return #m ma ai in n#
            input = SinglePercent + input + SinglePound;
            return ToNGrams(input, 2);
        }

        public static string[] ToTriGrams(this string input)
        {
            // nLength == 3
            //   from Jackson, return %%j %ja jac ack cks kso son on# n##
            //   from Main, return ##m #ma mai ain in# n##
            input = DoublePercent + input + DoublePount;
            return ToNGrams(input, 3);
        }

        private static string[] ToNGrams(string input, int nLength)
        {
            int itemsCount = input.Length - 1;
            string[] ngrams = new string[input.Length - 1];
            for (int i = 0; i < itemsCount; i++)
                ngrams[i] = input.Substring(i, nLength);
            return ngrams;
        }

        private const string SinglePercent = "%";
        private const string SinglePound = "#";
        private const string DoublePercent = "&&";
        private const string DoublePount = "##";
    }
}

```
