# Facial rendering source scan: LumKitty/VRCFTnyan

Repository: https://github.com/LumKitty/VRCFTnyan

Pinned source revision: 2211b85a88558fcb4e4d69d54037c7451ff65170

Discovery topic: ARKit blendshape

Description: Allows VRCFaceTracking to work with VNyan (for VTubers) Supports most VR hardware and avatars with ARKit blendshapes

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: VRCFTnyanEXE/VRChat.cs

{
  "path": "VRCFTnyanEXE/VRChat.cs",
  "blob_sha": "3339e377a4834e70060a33370942b3812e22d3e7",
  "sha256": "5d3eb3659f3e11c378d6825cdd0eb6cedc6b5d9c63279fd235f8bcf8f1c376e3",
  "lines": 395,
  "bytes": 10241,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 16,
    "jaw": 6,
    "mouth": 36,
    "tongue": 2,
    "rig": 30,
    "lip": 8
  },
  "url": "https://github.com/LumKitty/VRCFTnyan/blob/2211b85a88558fcb4e4d69d54037c7451ff65170/VRCFTnyanEXE/VRChat.cs",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "public static void CreateVRCAvatarFile(string filename)"
    }
  ]
}

```text

﻿using Common.Logging.Configuration;
using System;
using System.IO;
using System.Reflection;
using System.Text;

namespace VRCFTnyan
{
    internal static class VRChat
    {
        public static readonly string VRCDataFolder = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData).Replace("Roaming", "LocalLow"), "VRChat", "VRChat");
        public static readonly string VRCOSCFolder = Path.Combine(VRCDataFolder, "OSC");

        public static void CreateVRCAvatarFile(string filename)
        {
            string jsonText = ResourceString(filename);
            // VRCFTnyan.Log("JSON: " + jsonText);
            string vrcAvatarsFolder = VRCAvatarsFolder();
            Console.WriteLine("VRC Avatar dir: "+vrcAvatarsFolder);
            if (jsonText.Length > 0 && vrcAvatarsFolder.Length > 0)
            {
                Console.WriteLine("Writing: JSON to: " + filename);
                using var writer = new StreamWriter(Path.Combine(vrcAvatarsFolder, filename), false, new UTF8Encoding(true));
                writer.Write(jsonText);
            }
        }

        public static string VRCAvatarsFolder()
        {
            foreach (var userFolder in Directory.GetDirectories(VRCOSCFolder))
            {
                string avatarsFolder = Path.Combine(userFolder, "Avatars");
                if (Directory.Exists(avatarsFolder))
                {
                    return avatarsFolder;
                }
            }
            return "";
        }

        /*public static string ResourceString(string filename)
        {
            string resource = $"VRCFTnyan.Resources.{filename}";
            Assembly assembly = Assembly.GetExecutingAssembly();
            using (Stream? stream = assembly.GetManifestResourceStream(resource))
            {
                if (stream != null)
                {
                    using StreamReader reader = new StreamReader(stream);
                    return reader.ReadToEnd();
                }
            }
            return "";
        }*/

        // TODO: This is really fucking ugly but it works. Maybe I'll fix it at some point

        public static string ResourceString(string filename) {
/*            switch (filename) {
                case "avtr_00000000-0000-0000-0000-000000000000.json" :
                    return @"
{
  ""id"": ""avtr_00000000-0000-0000-0000-000000000000"",
  ""name"": ""[VRCFTtoVNyan] Tracking is OFF"",
  ""parameters"": []
}
";
                    break;
                case "avtr_00000000-0000-0000-0000-000000000001.json":*/
                    return @"
{
  ""id"": ""avtr_00000000-0000-0000-0000-000000000000"",
  ""name"": ""[VRCFTtoVNyan] Tracking is ON"",
  ""parameters"": [
    {
      ""name"": ""v2/EyeLeftX"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/EyeLeftX"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/EyeLeftY"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/EyeLeftY"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/EyeRightX"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/EyeRightX"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/EyeRightY"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/EyeRightY"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/BrowInnerUp"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/BrowInnerUp"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/BrowDownLeft"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/BrowDownLeft"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/BrowDownRight"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/BrowDownRight"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/BrowOuterUpLeft"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/BrowOuterUpLeft"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/BrowOuterUpRight"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/BrowOuterUpRight"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/EyeLidLeft"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/EyeLidLeft"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/EyeLidRight"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/EyeLidRight"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/EyeSquintLeft"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/EyeSquintLeft"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/EyeSquintRight"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/EyeSquintRight"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/CheekPuffSuck"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/CheekPuffSuck"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/CheekSquintLeft"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/CheekSquintLeft"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/CheekSquintRight"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/CheekSquintRight"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/NoseSneerLeft"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/NoseSneerLeft"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/NoseSneerRight"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/NoseSneerRight"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/JawOpen"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/JawOpen"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/JawZ"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/JawZ"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/JawX"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/JawX"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/LipFunnel"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/LipFunnel"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/LipPucker"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/LipPucker"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthX"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthX"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/LipSuckUpper"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/LipSuckUpper"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/LipSuckLower"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/LipSuckLower"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthRaiserUpper"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthRaiserUpper"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthRaiserLower"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthRaiserLower"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthClosed"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthClosed"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthCornerPullLeft"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthCornerPullLeft"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthCornerPullRight"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthCornerPullRight"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthFrownLeft"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthFrownLeft"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthFrownRight"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthFrownRight"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthDimpleLeft"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthDimpleLeft"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthDimpleRight"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthDimpleRight"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthUpperUpLeft"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthUpperUpLeft"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthUpperUpRight"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthUpperUpRight"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthLowerDownLeft"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthLowerDownLeft"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthLowerDownRight"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthLowerDownRight"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthPressLeft"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthPressLeft"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthPressRight"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthPressRight"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthStretchLeft"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthStretchLeft"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthStretchRight"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthStretchRight"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/TongueOut"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/TongueOut"",
        ""type"": ""Float""
      }
    }
  ]
}
";/*
                    break;
                default:
                    return "";

            }*/

        }

    }
}


```

## Source file: VRCFTnyanDLL/VRCInfo.cs

{
  "path": "VRCFTnyanDLL/VRCInfo.cs",
  "blob_sha": "7cb86a9ff213cd07379e5c00c4eeb12d2fbef785",
  "sha256": "2189c5837886bf7d1b6aa69c80abc3f3fd9327e654c95187ba4ed4a245cee9b4",
  "lines": 339,
  "bytes": 8559,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 16,
    "jaw": 6,
    "mouth": 36,
    "tongue": 2,
    "rig": 30,
    "lip": 8
  },
  "url": "https://github.com/LumKitty/VRCFTnyan/blob/2211b85a88558fcb4e4d69d54037c7451ff65170/VRCFTnyanDLL/VRCInfo.cs",
  "function_signature_candidates": []
}

```text

﻿using System;
using System.Collections.Generic;
using System.IO;
using System.Text;

namespace VRCFTnyan {
    internal static class VRCInfo {
        internal static readonly string VRCDataFolder = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData).Replace("Roaming", "LocalLow"), "VRChat", "VRChat");
        internal static readonly string VRCOSCFolder = Path.Combine(VRCDataFolder, "OSC");
        
        internal static string VRCAvatarsFolder() {
            foreach (var userFolder in Directory.GetDirectories(VRCOSCFolder)) {
                string avatarsFolder = Path.Combine(userFolder, "Avatars");
                if (Directory.Exists(avatarsFolder)) {
                    return avatarsFolder;
                }
            }
            return "";
        }
        internal const string AvatarID = "avtr_00000000-0000-0000-0000-000000000000";
        internal const string AvatarFilename = "avtr_00000000-0000-0000-0000-000000000000.json";
        internal const string AvatarJSON = @"
{
  ""id"": ""avtr_00000000-0000-0000-0000-000000000000"",
  ""name"": ""[VRCFTtoVNyan] Tracking is ON"",
  ""parameters"": [
    {
      ""name"": ""v2/EyeLeftX"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/EyeLeftX"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/EyeLeftY"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/EyeLeftY"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/EyeRightX"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/EyeRightX"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/EyeRightY"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/EyeRightY"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/BrowInnerUp"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/BrowInnerUp"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/BrowDownLeft"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/BrowDownLeft"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/BrowDownRight"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/BrowDownRight"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/BrowOuterUpLeft"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/BrowOuterUpLeft"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/BrowOuterUpRight"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/BrowOuterUpRight"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/EyeLidLeft"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/EyeLidLeft"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/EyeLidRight"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/EyeLidRight"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/EyeSquintLeft"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/EyeSquintLeft"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/EyeSquintRight"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/EyeSquintRight"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/CheekPuffSuck"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/CheekPuffSuck"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/CheekSquintLeft"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/CheekSquintLeft"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/CheekSquintRight"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/CheekSquintRight"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/NoseSneerLeft"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/NoseSneerLeft"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/NoseSneerRight"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/NoseSneerRight"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/JawOpen"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/JawOpen"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/JawZ"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/JawZ"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/JawX"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/JawX"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/LipFunnel"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/LipFunnel"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/LipPucker"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/LipPucker"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthX"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthX"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/LipSuckUpper"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/LipSuckUpper"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/LipSuckLower"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/LipSuckLower"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthRaiserUpper"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthRaiserUpper"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthRaiserLower"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthRaiserLower"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthClosed"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthClosed"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthCornerPullLeft"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthCornerPullLeft"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthCornerPullRight"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthCornerPullRight"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthFrownLeft"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthFrownLeft"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthFrownRight"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthFrownRight"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthDimpleLeft"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthDimpleLeft"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthDimpleRight"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthDimpleRight"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthUpperUpLeft"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthUpperUpLeft"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthUpperUpRight"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthUpperUpRight"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthLowerDownLeft"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthLowerDownLeft"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthLowerDownRight"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthLowerDownRight"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthPressLeft"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthPressLeft"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthPressRight"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthPressRight"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthStretchLeft"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthStretchLeft"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/MouthStretchRight"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/MouthStretchRight"",
        ""type"": ""Float""
      }
    },
    {
      ""name"": ""v2/TongueOut"",
      ""input"": {
        ""address"": ""/avatar/parameters/v2/TongueOut"",
        ""type"": ""Float""
      }
    }
  ]
}
";
    }
}


```

## Source file: VRCFTnyanDLL/OSC/Util.cs

{
  "path": "VRCFTnyanDLL/OSC/Util.cs",
  "blob_sha": "37acafb194b3a6fed3edf93cacf6c7c2c28199a6",
  "sha256": "ec7185778d579108b1f77eb73c3704fd59716cf0a284b02056502da81dcda061",
  "lines": 40,
  "bytes": 1020,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/LumKitty/VRCFTnyan/blob/2211b85a88558fcb4e4d69d54037c7451ff65170/VRCFTnyanDLL/OSC/Util.cs",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "public static bool IsMultipleOfFour(int num)"
    },
    {
      "line": 12,
      "text": "public static int GetStringAlignedSize(int size)"
    },
    {
      "line": 17,
      "text": "public static int GetBufferAlignedSize(int size)"
    }
  ]
}

```text

﻿using System.IO;

namespace VRCFTnyan.OSC
{
    internal static class Util
    {
        public static bool IsMultipleOfFour(int num)
        {
            return num == (num & ~0x3);
        }

        public static int GetStringAlignedSize(int size)
        {
            return (size + 4) & ~0x3;
        }

        public static int GetBufferAlignedSize(int size)
        {
            var offset = size & ~0x3;
            return (offset == size) ? size : (offset + 4);
        }

        public static string GetString(this object value)
        {
            switch (value)
            {
                case int i: return i.ToString();
                case float f: return f.ToString();
                case string s: return s;
                case byte[] bytes: return "Byte[" + bytes.Length + "]";
                default: return value.ToString() ?? "";
            };
        }

        public static byte[] GetBuffer(MemoryStream stream)
        {
            return stream.GetBuffer();
        }
    }
}


```

## Source file: VRCFTnyanEXE/OSC/Util.cs

{
  "path": "VRCFTnyanEXE/OSC/Util.cs",
  "blob_sha": "7c05273b57dee30558935c584aebe7a40acbf927",
  "sha256": "c11449f4454f6e78f4226e1b454549ef6a77f0b7cb630b51034c8e7014a23b4d",
  "lines": 40,
  "bytes": 1020,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/LumKitty/VRCFTnyan/blob/2211b85a88558fcb4e4d69d54037c7451ff65170/VRCFTnyanEXE/OSC/Util.cs",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "public static bool IsMultipleOfFour(int num)"
    },
    {
      "line": 12,
      "text": "public static int GetStringAlignedSize(int size)"
    },
    {
      "line": 17,
      "text": "public static int GetBufferAlignedSize(int size)"
    }
  ]
}

```text

﻿using System.IO;

namespace VRCFTnyan.Osc
{
    internal static class Util
    {
        public static bool IsMultipleOfFour(int num)
        {
            return num == (num & ~0x3);
        }

        public static int GetStringAlignedSize(int size)
        {
            return (size + 4) & ~0x3;
        }

        public static int GetBufferAlignedSize(int size)
        {
            var offset = size & ~0x3;
            return (offset == size) ? size : (offset + 4);
        }

        public static string GetString(this object value)
        {
            switch (value)
            {
                case int i: return i.ToString();
                case float f: return f.ToString();
                case string s: return s;
                case byte[] bytes: return "Byte[" + bytes.Length + "]";
                default: return value.ToString() ?? "";
            };
        }

        public static byte[] GetBuffer(MemoryStream stream)
        {
            return stream.GetBuffer();
        }
    }
}


```
