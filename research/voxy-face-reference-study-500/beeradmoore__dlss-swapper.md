# Facial rendering source scan: beeradmoore/dlss-swapper

Repository: https://github.com/beeradmoore/dlss-swapper

Pinned source revision: ab9b1e2d4bb04187c48fe58eeea06c2b0ea71fbb

Discovery topic: DLSS

Description: 

Reported license: GPL-3.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE", "src/Acknowledgements/AsyncAwaitBestPractices/license.txt", "src/Acknowledgements/ByteSize/license.txt", "src/Acknowledgements/CommunityToolkit.Labs/license.txt", "src/Acknowledgements/CommunityToolkit.Mvvm/license.txt", "src/Acknowledgements/CommunityToolkit.WinUI/license.txt", "src/Acknowledgements/CsvHelper/license.txt", "src/Acknowledgements/DLSS/license.txt", "src/Acknowledgements/FidelityFX-SDK/license.txt", "src/Acknowledgements/Microsoft.Web.WebView2/license.txt", "src/Acknowledgements/Microsoft.Win32.Registry/license.txt", "src/Acknowledgements/Microsoft.WindowsAppSDK/license.txt", "src/Acknowledgements/Nito.AsyncEx/license.txt", "src/Acknowledgements/NvAPIWrapper/license.txt", "src/Acknowledgements/SQLite-net/license.txt", "src/Acknowledgements/SQLitePCLRaw/license.txt", "src/Acknowledgements/Serilog.Sinks.Debug/license.txt", "src/Acknowledgements/Serilog.Sinks.File/license.txt", "src/Acknowledgements/Serilog/license.txt", "src/Acknowledgements/SixLabors.ImageSharp/license.txt"]

Topic paths: {"sss": ["src/Converters/DLSSStateVisibilityConverter.cs", "src/Helpers/DLSSSettingsManager.cs"], "dlss": ["DLSS Swapper.sln", "src/Acknowledgements/DLSS/license.txt", "src/Acknowledgements/DLSS/notes.md", "src/Assets/dlss_d_presets.json", "src/Assets/dlss_g_presets.json", "src/Assets/dlss_presets.json", "src/Converters/DLSSStateVisibilityConverter.cs", "src/DLSS Swapper.csproj", "src/Data/DLSS/PresetOption.cs", "src/Helpers/DLSSSettingsManager.cs"], "face": ["src/Interfaces/IGameLibrary.cs", "src/Interfaces/LocalizedViewModelBase.cs"]}

## Source file: src/Helpers/DLSSSettingsManager.cs

{
  "path": "src/Helpers/DLSSSettingsManager.cs",
  "blob_sha": "1ca55bab5f0289cdbcc90c20291876900465451d",
  "sha256": "01246f99839669a2edeecafb0d89f00bf663d7d3e1dd4dfacebe53760fc318c8",
  "lines": 116,
  "bytes": 2788,
  "selection_score": 30,
  "keyword_line_counts": {
    "sss": 3,
    "gum": 2,
    "dlss": 8
  },
  "url": "https://github.com/beeradmoore/dlss-swapper/blob/ab9b1e2d4bb04187c48fe58eeea06c2b0ea71fbb/src/Helpers/DLSSSettingsManager.cs",
  "function_signature_candidates": [
    {
      "line": 20,
      "text": "bool RunRegAdd(string key, string name, string type, string value)"
    },
    {
      "line": 56,
      "text": "public bool SetShowDlssIndicator(int value)"
    },
    {
      "line": 61,
      "text": "public int GetShowDlssIndicator()"
    },
    {
      "line": 72,
      "text": "public bool SetLogLevel(int logLevel)"
    },
    {
      "line": 82,
      "text": "public int GetLogLevel()"
    },
    {
      "line": 93,
      "text": "public bool SetLoggingWindow(bool enabled)"
    },
    {
      "line": 105,
      "text": "public bool GetLoggingWindow()"
    }
  ]
}

```text

using System;
using System.Diagnostics;
using System.Globalization;
using Microsoft.Win32;

namespace DLSS_Swapper.Helpers;

// Reg values defined in this file are from:
// https://github.com/NVIDIA/DLSS/tree/main/utils

internal class DLSSSettingsManager
{
    const string NGXCORE_REG_KEY = @"HKEY_LOCAL_MACHINE\SOFTWARE\NVIDIA Corporation\Global\NGXCore";

    public DLSSSettingsManager()
    {

    }

    bool RunRegAdd(string key, string name, string type, string value)
    {
        var processInfo = new ProcessStartInfo()
        {
            FileName = "reg",
            Arguments = $"add \"{key}\" /f /v {name} /t {type} /d {value}",
            Verb = "runas",
            UseShellExecute = true,
            CreateNoWindow = true
        };

        try
        {
            using (var process = Process.Start(processInfo))
            {
                if (process is not null)
                {
                    process.WaitForExit();

                    if (process.ExitCode == 0)
                    {
                        return true;
                    }

                    throw new Exception("Process exit code was {process.ExitCode}");
                }
            }
        }
        catch (Exception err)
        {
            Logger.Error(err, $"Could not command \"{processInfo.FileName} {processInfo.Arguments}");
        }

        return false;
    }

    public bool SetShowDlssIndicator(int value)
    {
        return RunRegAdd(NGXCORE_REG_KEY, "ShowDlssIndicator", "REG_DWORD", value.ToString(CultureInfo.InvariantCulture));
    }

    public int GetShowDlssIndicator()
    {
        if (Registry.GetValue(NGXCORE_REG_KEY, "ShowDlssIndicator", 0) is int existingValue)
        {
            return existingValue;
        }

        return 0;
    }


    public bool SetLogLevel(int logLevel)
    {
        if (logLevel == 0 || logLevel == 1 || logLevel == 2)
        {
            return RunRegAdd(NGXCORE_REG_KEY, "LogLevel", "REG_DWORD", $"{logLevel}");
        }

        return false;
    }

    public int GetLogLevel()
    {
        if (Registry.GetValue(NGXCORE_REG_KEY, "LogLevel", 0) is int existingValue)
        {
            return existingValue;
        }

        return 0;
    }


    public bool SetLoggingWindow(bool enabled)
    {
        if (enabled)
        {
            return RunRegAdd(NGXCORE_REG_KEY, "EnableConsoleLogging", "REG_DWORD", "1");
        }
        else
        {
            return RunRegAdd(NGXCORE_REG_KEY, "EnableConsoleLogging", "REG_DWORD", "0");
        }
    }

    public bool GetLoggingWindow()
    {
        if (Registry.GetValue(NGXCORE_REG_KEY, "EnableConsoleLogging", 0) is int existingValue)
        {
            return (existingValue == 1);
        }

        return false;
    }


}


```

## Source file: src/Converters/DLSSStateVisibilityConverter.cs

{
  "path": "src/Converters/DLSSStateVisibilityConverter.cs",
  "blob_sha": "6b8bfd3d84c3671e87f1a80e03658ef407e7b7f1",
  "sha256": "808ded02a60bbba41c1bc34ed6e77297891b8783a46a94daad75ddd81fb308fa",
  "lines": 71,
  "bytes": 2075,
  "selection_score": 30,
  "keyword_line_counts": {
    "sss": 3,
    "dlss": 5
  },
  "url": "https://github.com/beeradmoore/dlss-swapper/blob/ab9b1e2d4bb04187c48fe58eeea06c2b0ea71fbb/src/Converters/DLSSStateVisibilityConverter.cs",
  "function_signature_candidates": []
}

```text

using DLSS_Swapper.Data;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Data;
using System;

namespace DLSS_Swapper.Converters;

class DLSSStateVisibilityConverter : DependencyObject, IValueConverter
{
    public static readonly DependencyProperty DesierdStateProperty = DependencyProperty.Register(nameof(DesierdState), typeof(string), typeof(DLSSStateVisibilityConverter), new PropertyMetadata(null));
    public string DesierdState
    {
        get { return (string)GetValue(DesierdStateProperty); }
        set { SetValue(DesierdStateProperty, value); }
    }


    public DLSSStateVisibilityConverter()
    {

    }

    public object Convert(object value, Type targetType, object parameter, string language)
    {
        if (value is null)
        {
            return Visibility.Collapsed;
        }

        if (value is LocalRecord localRecord)
        {
            if (DesierdState == "Downloading")
            {
                if (localRecord.FileDownloader is null)
                {
                    return Visibility.Collapsed;
                }

                return Visibility.Visible;
            }
            else if (DesierdState == "Downloaded")
            {
                return localRecord.IsDownloaded ? Visibility.Visible : Visibility.Collapsed;
            }
            else if (DesierdState == "NotFound")
            {
                if (localRecord.FileDownloader is not null)
                {
                    return Visibility.Collapsed;
                }

                if (localRecord.IsDownloaded)
                {
                    return Visibility.Collapsed;
                }

                return Visibility.Visible;
            }
            else if (DesierdState == "Imported")
            {
                return localRecord.IsImported ? Visibility.Visible : Visibility.Collapsed;
            }
        }
        return Visibility.Collapsed;
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language)
    {
        throw new NotImplementedException();
    }
}


```

## Source file: src/Data/DLSS/PresetOption.cs

{
  "path": "src/Data/DLSS/PresetOption.cs",
  "blob_sha": "5e88ff4cb858ab29fd4452c44fbd1aac980c4da9",
  "sha256": "a548fc1360e9782b27479cc604aca4910974a00fef02c048f03d18479555d332",
  "lines": 60,
  "bytes": 3100,
  "selection_score": 3,
  "keyword_line_counts": {
    "dlss": 18
  },
  "url": "https://github.com/beeradmoore/dlss-swapper/blob/ab9b1e2d4bb04187c48fe58eeea06c2b0ea71fbb/src/Data/DLSS/PresetOption.cs",
  "function_signature_candidates": [
    {
      "line": 32,
      "text": "public void UpdateNameFromTranslation()"
    }
  ]
}

```text


using System.ComponentModel;
using System.Text.Json.Serialization;
using DLSS_Swapper.Helpers;
using DLSS_Swapper.JsonConverters;

namespace DLSS_Swapper.Data.DLSS;

public class PresetOption : INotifyPropertyChanged
{
    [JsonPropertyName("name")]
    public string Name { get; set; }

    [JsonPropertyName("value")]
    [JsonConverter(typeof(HexStringToUintConverter))]
    public uint Value { get; init; }

    [JsonPropertyName("used")]
    public bool Used { get; set; }

    public bool Deprecated { get; set; }

    [JsonConstructor]
    public PresetOption(string name, uint value)
    {
        Name = name;
        Value = value;
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    public void UpdateNameFromTranslation()
    {
        var newName = Value switch
        {
            0x00000000 => ResourceHelper.GetString("DLSS_Preset_Default"),
            0x00000001 => ResourceHelper.GetFormattedResourceTemplate(Deprecated ? "DLSS_Preset_Letter_Deprecated" : "DLSS_Preset_Letter", "A"),
            0x00000002 => ResourceHelper.GetFormattedResourceTemplate(Deprecated ? "DLSS_Preset_Letter_Deprecated" : "DLSS_Preset_Letter", "B"),
            0x00000003 => ResourceHelper.GetFormattedResourceTemplate(Deprecated ? "DLSS_Preset_Letter_Deprecated" : "DLSS_Preset_Letter", "C"),
            0x00000004 => ResourceHelper.GetFormattedResourceTemplate(Deprecated ? "DLSS_Preset_Letter_Deprecated" : "DLSS_Preset_Letter", "D"),
            0x00000005 => ResourceHelper.GetFormattedResourceTemplate(Deprecated ? "DLSS_Preset_Letter_Deprecated" : "DLSS_Preset_Letter", "E"),
            0x00000006 => ResourceHelper.GetFormattedResourceTemplate(Deprecated ? "DLSS_Preset_Letter_Deprecated" : "DLSS_Preset_Letter", "F"),
            //0x00000007 => ResourceHelper.GetFormattedResourceTemplate(Deprecated ? "DLSS_Preset_Letter_Deprecated" : "DLSS_Preset_Letter", "G"),
            //0x00000008 => ResourceHelper.GetFormattedResourceTemplate(Deprecated ? "DLSS_Preset_Letter_Deprecated" : "DLSS_Preset_Letter", "H"),
            //0x00000009 => ResourceHelper.GetFormattedResourceTemplate(Deprecated ? "DLSS_Preset_Letter_Deprecated" : "DLSS_Preset_Letter", "I"),
            0x0000000A => ResourceHelper.GetFormattedResourceTemplate(Deprecated ? "DLSS_Preset_Letter_Deprecated" : "DLSS_Preset_Letter", "J"),
            0x0000000B => ResourceHelper.GetFormattedResourceTemplate(Deprecated ? "DLSS_Preset_Letter_Deprecated" : "DLSS_Preset_Letter", "K"),
            0x0000000C => ResourceHelper.GetFormattedResourceTemplate(Deprecated ? "DLSS_Preset_Letter_Deprecated" : "DLSS_Preset_Letter", "L"),
            0x0000000D => ResourceHelper.GetFormattedResourceTemplate(Deprecated ? "DLSS_Preset_Letter_Deprecated" : "DLSS_Preset_Letter", "M"),
            0x00FFFFFF => ResourceHelper.GetString("DLSS_Preset_AlwaysUseLatest"),
            _ => string.Empty
        };

        if (string.IsNullOrWhiteSpace(newName) == false)
        {
            Name = newName;
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(Name)));
        }
    }
}


```

## Source file: src/Interfaces/IGameLibrary.cs

{
  "path": "src/Interfaces/IGameLibrary.cs",
  "blob_sha": "f91dc479173113954c7157d28217879aa8a3048b",
  "sha256": "9f16bb1a3f3989cf32d6ef4395e6e50c3734daa463827df6a1ede471d6c2f66a",
  "lines": 87,
  "bytes": 2345,
  "selection_score": 3,
  "keyword_line_counts": {
    "dlss": 10,
    "face": 2
  },
  "url": "https://github.com/beeradmoore/dlss-swapper/blob/ab9b1e2d4bb04187c48fe58eeea06c2b0ea71fbb/src/Interfaces/IGameLibrary.cs",
  "function_signature_candidates": [
    {
      "line": 38,
      "text": "bool IsInstalled();"
    },
    {
      "line": 64,
      "text": "public void Disable()"
    },
    {
      "line": 76,
      "text": "public void Enable()"
    }
  ]
}

```text

using DLSS_Swapper.Data;
using DLSS_Swapper.Data.EpicGamesStore;
using DLSS_Swapper.Data.GOG;
using DLSS_Swapper.Data.Steam;
using DLSS_Swapper.Data.UbisoftConnect;
using DLSS_Swapper.Data.Xbox;
using DLSS_Swapper.Data.ManuallyAdded;
using DLSS_Swapper.Data.BattleNet;
using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using DLSS_Swapper.Data.EAApp;

namespace DLSS_Swapper.Interfaces;

[Flags]
public enum GameLibrary : uint
{
    Steam = 1,
    GOG = 2,
    EpicGamesStore = 4,
    UbisoftConnect = 8,
    XboxApp = 16,
    ManuallyAdded = 32,
    BattleNet = 64,
    EAApp = 128,
};

public interface IGameLibrary
{
    GameLibrary GameLibrary { get; }
    GameLibrarySettings? GameLibrarySettings { get; }
    string Name { get; }
    Type GameType { get; }

    Task<List<Game>> ListGamesAsync(bool forceNeedsProcessing);
    Task LoadGamesFromCacheAsync();
    bool IsInstalled();

    static IGameLibrary GetGameLibrary(GameLibrary gameLibrary)
    {
        return gameLibrary switch
        {
            GameLibrary.Steam => SteamLibrary.Instance,
            GameLibrary.GOG => GOGLibrary.Instance,
            GameLibrary.EpicGamesStore => EpicGamesStoreLibrary.Instance,
            GameLibrary.UbisoftConnect => UbisoftConnectLibrary.Instance,
            GameLibrary.XboxApp => XboxLibrary.Instance,
            GameLibrary.BattleNet => BattleNetLibrary.Instance,
            GameLibrary.EAApp => EAAppLibrary.Instance,
            GameLibrary.ManuallyAdded => ManuallyAddedLibrary.Instance,
            _ => throw new Exception($"Could not load game library {gameLibrary}."),
        };
    }

    public bool IsEnabled
    {
        get
        {
            return GameLibrarySettings?.IsEnabled ?? false;
        }
    }

    public void Disable()
    {
        if (GameLibrarySettings is not null)
        {
            if (GameLibrarySettings.IsEnabled == true)
            {
                GameLibrarySettings.IsEnabled = false;
                Settings.Instance.SaveJson();
            }
        }
    }

    public void Enable()
    {
        if (GameLibrarySettings is not null)
        {
            if (GameLibrarySettings.IsEnabled == false)
            {
                GameLibrarySettings.IsEnabled = true;
                Settings.Instance.SaveJson();
            }
        }
    }
}


```
