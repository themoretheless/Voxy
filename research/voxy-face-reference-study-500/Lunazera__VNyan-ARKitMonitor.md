# Facial rendering source scan: Lunazera/VNyan-ARKitMonitor

Repository: https://github.com/Lunazera/VNyan-ARKitMonitor

Pinned source revision: 9cfd63d70144fa845506bdb1d8200d2535ca2101

Discovery topic: ARKit blendshape

Description: Displays your 52 ARKit blendshape tracking and lets you pause and save current values

Reported license: GPL-3.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE.txt"]

Topic paths: {"blendshape": ["project_files/src/UI/LZ_BlendshapeList.cs", "release_history/BlendshapeSnap_v1.0.zip"]}

## Source file: project_files/src/UI/LZ_BlendshapeList.cs

{
  "path": "project_files/src/UI/LZ_BlendshapeList.cs",
  "blob_sha": "7f0fbbd5636760959086c62fc8979947536d509e",
  "sha256": "fd1d26220cd52635fc303a6c1a42086524a75d35ece21eb50b5b38af67d5e240",
  "lines": 52,
  "bytes": 1995,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 18,
    "face": 6
  },
  "url": "https://github.com/Lunazera/VNyan-ARKitMonitor/blob/9cfd63d70144fa845506bdb1d8200d2535ca2101/project_files/src/UI/LZ_BlendshapeList.cs",
  "function_signature_candidates": [
    {
      "line": 26,
      "text": "void Start()"
    },
    {
      "line": 32,
      "text": "void Update()"
    },
    {
      "line": 44,
      "text": "public void changeThemeSettings()"
    }
  ]
}

```text

﻿using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using Newtonsoft.Json;
using UnityEngine;
using UnityEngine.EventSystems;
using UnityEngine.UI;
using VNyanInterface;

namespace LZUI
{
    class LZ_BlendshapeList : MonoBehaviour
    {
        // Text for Unity
        public Text Blendshapes_Col1;
        public Text Blendshapes_Col2;
        public Text Blendshapes_Col3;
        public Text Blendshapes_Col4;

        public InputField Blendshapes_Selectable;

        private List<StringBuilder> listBlendshapes = new List<StringBuilder>();
        private StringBuilder stringlargeBlendshapes = new StringBuilder();

        void Start()
        {
            changeThemeSettings();
            VNyanInterface.VNyanInterface.VNyanUI.colorThemeChanged += changeThemeSettings;
        }

        void Update()
        {
            listBlendshapes = ARKitMonitor.ARKitMonitorPlugin.listBlendshapes();

            Blendshapes_Col1.text = listBlendshapes[0].ToString();
            Blendshapes_Col2.text = listBlendshapes[1].ToString();
            Blendshapes_Col3.text = listBlendshapes[2].ToString();
            Blendshapes_Col4.text = listBlendshapes[3].ToString();

            Blendshapes_Selectable.text = listBlendshapes[4].ToString();
        }

        public void changeThemeSettings()
        {
            Blendshapes_Col1.GetComponent<Text>().color = LZ_UI.hexToColor(VNyanInterface.VNyanInterface.VNyanUI.getCurrentThemeColor(ThemeComponent.Text));
            Blendshapes_Col2.GetComponent<Text>().color = LZ_UI.hexToColor(VNyanInterface.VNyanInterface.VNyanUI.getCurrentThemeColor(ThemeComponent.Text));
            Blendshapes_Col3.GetComponent<Text>().color = LZ_UI.hexToColor(VNyanInterface.VNyanInterface.VNyanUI.getCurrentThemeColor(ThemeComponent.Text));
            Blendshapes_Col4.GetComponent<Text>().color = LZ_UI.hexToColor(VNyanInterface.VNyanInterface.VNyanUI.getCurrentThemeColor(ThemeComponent.Text));
        }
    }
}


```

## Source file: project_files/src/UI/LZ_UI.cs

{
  "path": "project_files/src/UI/LZ_UI.cs",
  "blob_sha": "13953b0d4a07ad20ca779326d70c4e7ce61381ab",
  "sha256": "44825582b5bf353138a9a69f6ff6aa46d7e39dfd8de903fb2f0dddbb847746f3",
  "lines": 84,
  "bytes": 2829,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 4
  },
  "url": "https://github.com/Lunazera/VNyan-ARKitMonitor/blob/9cfd63d70144fa845506bdb1d8200d2535ca2101/project_files/src/UI/LZ_UI.cs",
  "function_signature_candidates": [
    {
      "line": 25,
      "text": "void Awake()"
    },
    {
      "line": 56,
      "text": "public void pluginButtonClicked()"
    }
  ]
}

```text

﻿using System.Collections.Generic;
using System.ComponentModel.Design;
using System.IO;
using Newtonsoft.Json;
using UnityEngine;
using UnityEngine.UI;
using VNyanInterface;
using System;
using System.Security;

// UI Core
// modified from Sjatar's UI code example for VNyan Plugin UI's: https://github.com/Sjatar/Screen-Light

namespace LZUI
{
    // VNyanInterface.IButtonClickHandler gives access to pluginButtonClicked
    public class LZ_UI : MonoBehaviour, IButtonClickedHandler
    {
        public GameObject windowPrefab;
        private GameObject window;

        static IVNyanPluginManifest manifest = new ARKitMonitor.ARKitMonitorManifest();


        void Awake()
        {
            VNyanInterface.VNyanInterface.VNyanUI.registerPluginButton(pluginName(), (IButtonClickedHandler)this);
            this.window = (GameObject)VNyanInterface.VNyanInterface.VNyanUI.instantiateUIPrefab((object)this.windowPrefab);
            if ((UnityEngine.Object)this.window != (UnityEngine.Object)null)
            {
                this.window.GetComponent<RectTransform>().anchoredPosition = new Vector2(0.0f, 0.0f);
                this.window.SetActive(false);
            }
        }

        public static string pluginName()
        {
            return manifest.PluginName;
        }

        public static string pluginTitle()
        {
            return manifest.Title;
        }

        public static string pluginVersion()
        {
            return manifest.Version;
        }

        public static string pluginAuthor()
        {
            return manifest.Author;
        }

        public void pluginButtonClicked()
        {
            if ((UnityEngine.Object)this.window == (UnityEngine.Object)null)
                return;
            this.window.SetActive(!this.window.activeSelf);
            if ( !this.window.activeSelf )
                return;
            this.window.transform.SetAsLastSibling();
        }

        public static Color hexToColor(string hex)
        {
            hex = hex.Replace("0x", ""); //in case the string is formatted 0xFFFFFF
            hex = hex.Replace("#", ""); //in case the string is formatted #FFFFFF

            byte a = 255; //assume fully visible unless specified in hex
            byte r = byte.Parse(hex.Substring(0, 2), System.Globalization.NumberStyles.HexNumber);
            byte g = byte.Parse(hex.Substring(2, 2), System.Globalization.NumberStyles.HexNumber);
            byte b = byte.Parse(hex.Substring(4, 2), System.Globalization.NumberStyles.HexNumber);

            //Only use alpha if the string has enough characters
            if (hex.Length == 8)
            {
                a = byte.Parse(hex.Substring(6, 2), System.Globalization.NumberStyles.HexNumber);
            }
            return new Color32(r, g, b, a);
        }
    }
}


```

## Source file: project_files/src/ARKitMonitor.cs

{
  "path": "project_files/src/ARKitMonitor.cs",
  "blob_sha": "282f54b34357ffdf2e9419dd721aa4d397220b14",
  "sha256": "af36ed421b19790cf015bb104c6eaea884b017f8214e90b1d9ecb33beea56599",
  "lines": 110,
  "bytes": 5225,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 1,
    "blendshape": 23,
    "jaw": 1,
    "mouth": 1,
    "tongue": 1,
    "rig": 1,
    "face": 11
  },
  "url": "https://github.com/Lunazera/VNyan-ARKitMonitor/blob/9cfd63d70144fa845506bdb1d8200d2535ca2101/project_files/src/ARKitMonitor.cs",
  "function_signature_candidates": [
    {
      "line": 19,
      "text": "public void InitializePlugin()"
    },
    {
      "line": 85,
      "text": "void Awake()"
    },
    {
      "line": 93,
      "text": "void Update()"
    }
  ]
}

```text

﻿using System;
using System.Linq;
using UnityEngine;
using UnityEngine.UI;
using VNyanInterface;
using Unity.Profiling;
using System.Collections.Generic;
using System.Text;

namespace ARKitMonitor
{
    class ARKitMonitorManifest : IVNyanPluginManifest
    {
        public string PluginName { get; } = "ARKit Monitor";
        public string Version { get; } = "1.2";
        public string Title { get; } = "ARKit Blendshape Monitor";
        public string Author { get; } = "lunazera";
        public string Website { get; } = "https://github.com/Lunazera/VNyan-Blendshape-Snapshot";
        public void InitializePlugin()
        {
        }
    }

    public class ARKitMonitorPlugin : MonoBehaviour
    {
        private static Dictionary<string, float> CurrentBlendshapes;

        public float threshold = 30;
        public static string parameterNameThreshold = "LZMonitor_Threshold";

        public float pauseFlag = 0;
        public static string parameterNamepauseFlag = "LZMonitor_pause";

        public float continuousFlag = 0;
        public static string parameterNamecontinuousFlag = "LZMonitor_runcontinuous";

        private string parameterNameOutputText = "LZMonitor_Blendshapes";

        public static List<StringBuilder> listBlendshapes()
        {
            List<StringBuilder> blendshapeStrings = new List<StringBuilder>();
            blendshapeStrings.Add(new StringBuilder());
            blendshapeStrings.Add(new StringBuilder());
            blendshapeStrings.Add(new StringBuilder());
            blendshapeStrings.Add(new StringBuilder());
            blendshapeStrings.Add(new StringBuilder());

            float currentThreshold = VNyanInterface.VNyanInterface.VNyanParameter.getVNyanParameterFloat(parameterNameThreshold);
            int blendIndex = 0;
            int blendIndexOffset = 0;
            
            string[] ARKit_Blendshapes = new string[] { "BrowDownLeft", "BrowDownRight", "BrowInnerUp", "BrowOuterUpLeft", "BrowOuterUpRight", "CheekPuff", "CheekSquintLeft", "CheekSquintRight", "EyeBlinkLeft", "EyeBlinkRight", "EyeLookDownLeft", "EyeLookDownRight", "EyeLookInLeft", "EyeLookInRight", "EyeLookOutLeft", "EyeLookOutRight", "EyeLookUpLeft", "EyeLookUpRight", "EyeSquintLeft", "EyeSquintRight", "EyeWideLeft", "EyeWideRight", "JawForward", "JawLeft", "JawOpen", "JawRight", "MouthClose", "MouthDimpleLeft", "MouthDimpleRight", "MouthFrownLeft", "MouthFrownRight", "MouthFunnel", "MouthLeft", "MouthLowerDownLeft", "MouthLowerDownRight", "MouthPressLeft", "MouthPressRight", "MouthPucker", "MouthRight", "MouthRollLower", "MouthRollUpper", "MouthShrugLower", "MouthShrugUpper", "MouthSmileLeft", "MouthSmileRight", "MouthStretchLeft", "MouthStretchRight", "MouthUpperUpLeft", "MouthUpperUpRight", "NoseSneerLeft", "NoseSneerRight", "TongueOut" };
            
            foreach (string blendshape in ARKit_Blendshapes)
            {
                if (CurrentBlendshapes.ContainsKey(blendshape))
                {
                    blendIndex++;
                    if (blendIndex <= 26)
                    {
                        blendIndexOffset = 0;
                    }
                    else
                    {
                        blendIndexOffset = 2;
                    }

                    float shapeValue = CurrentBlendshapes[blendshape] * 100;
                    if (shapeValue >= currentThreshold)
                    {
                        blendshapeStrings[0 + blendIndexOffset].AppendLine(">" + blendshape + ": ");
                        blendshapeStrings[4].Append(blendshape + ";");
                    }
                    else
                    {
                        blendshapeStrings[0 + blendIndexOffset].AppendLine(blendshape + ": ");
                    }
                    blendshapeStrings[1 + blendIndexOffset].AppendLine(shapeValue.ToString("0.0"));
                }
            }

            return blendshapeStrings;
        }

        void Awake()
        {
            CurrentBlendshapes = VNyanInterface.VNyanInterface.VNyanAvatar.getBlendshapesInstant();
            VNyanInterface.VNyanInterface.VNyanParameter.setVNyanParameterFloat(parameterNameThreshold, threshold);
            VNyanInterface.VNyanInterface.VNyanParameter.setVNyanParameterFloat(parameterNamepauseFlag, 0);
            VNyanInterface.VNyanInterface.VNyanParameter.setVNyanParameterFloat(parameterNamecontinuousFlag, 0);
        }

        void Update()
        {
            threshold = VNyanInterface.VNyanInterface.VNyanParameter.getVNyanParameterFloat(parameterNameThreshold);
            pauseFlag = VNyanInterface.VNyanInterface.VNyanParameter.getVNyanParameterFloat(parameterNamepauseFlag);
            continuousFlag = VNyanInterface.VNyanInterface.VNyanParameter.getVNyanParameterFloat(parameterNamecontinuousFlag);

            if (pauseFlag == 0)
            {
                CurrentBlendshapes = VNyanInterface.VNyanInterface.VNyanAvatar.getBlendshapesInstant();
            }

            if (continuousFlag == 1 )
            {
                VNyanInterface.VNyanInterface.VNyanParameter.setVNyanParameterString(parameterNameOutputText, listBlendshapes()[4].ToString());
            }
        }
    }
}


```

## Source file: project_files/src/UI/LZ_Button.cs

{
  "path": "project_files/src/UI/LZ_Button.cs",
  "blob_sha": "c60d42f7e81cb5314390af96e8bea6e4468b98ab",
  "sha256": "40a39618c5f64ebd5d177beb4cdf8fb2dcfd84afd27f5bae40413a4119ce1f54",
  "lines": 73,
  "bytes": 2420,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 3
  },
  "url": "https://github.com/Lunazera/VNyan-ARKitMonitor/blob/9cfd63d70144fa845506bdb1d8200d2535ca2101/project_files/src/UI/LZ_Button.cs",
  "function_signature_candidates": [
    {
      "line": 16,
      "text": "void Start()"
    },
    {
      "line": 27,
      "text": "public void ChangeButtonColor(bool boolbuttonState)"
    },
    {
      "line": 51,
      "text": "public void ButtonPressCheck()"
    }
  ]
}

```text

﻿using System;
using System.IO;
using Newtonsoft.Json;
using UnityEngine;
using UnityEngine.EventSystems;
using UnityEngine.UI;

namespace LZUI
{
    class LZ_Button : MonoBehaviour
    {
        private Button mainButton;
        private float buttonState;
        public string buttonName;

        void Start()
        {
            mainButton = GetComponent(typeof(Button)) as Button;
            mainButton.onClick.AddListener(delegate { ButtonPressCheck(); });

            // Start button off
            buttonState = 0f;
            VNyanInterface.VNyanInterface.VNyanParameter.setVNyanParameterFloat(buttonName, buttonState);
            ChangeButtonColor(false);
        }

        public void ChangeButtonColor(bool boolbuttonState)
        {
            // If the input is true make it green
            if (boolbuttonState)
            {
                ColorBlock cb = mainButton.colors;
                cb.normalColor = new Color(0.5f, 1f, 0.5f);
                cb.highlightedColor = new Color(0.5f, 0.9f, 0.5f);
                cb.pressedColor = new Color(0.4f, 0.8f, 0.4f);
                cb.selectedColor = new Color(0.5f, 1f, 0.5f);
                mainButton.colors = cb;
            }
            // Else make it red! 
            else
            {
                ColorBlock cb = mainButton.colors;
                cb.normalColor = new Color(1f, 0.5f, 0.5f);
                cb.highlightedColor = new Color(0.9f, 0.5f, 0.5f);
                cb.pressedColor = new Color(0.8f, 0.4f, 0.4f);
                cb.selectedColor = new Color(1f, 0.5f, 0.5f);
                mainButton.colors = cb;
            }
        }

        public void ButtonPressCheck()
        {
            {
                // If button state was off, run on part of script
                if (buttonState == 0f)
                {
                    buttonState = 1f;
                    VNyanInterface.VNyanInterface.VNyanParameter.setVNyanParameterFloat(buttonName, buttonState);
                    ChangeButtonColor(true);
                }
                // If the button was not off, run off part of the script!
                else
                {
                    buttonState = 0f;
                    VNyanInterface.VNyanInterface.VNyanParameter.setVNyanParameterFloat(buttonName, buttonState);
                    ChangeButtonColor(false);
                }
            }
            
        }

    }
}


```
