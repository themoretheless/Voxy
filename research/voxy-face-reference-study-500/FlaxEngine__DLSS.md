# Facial rendering source scan: FlaxEngine/DLSS

Repository: https://github.com/FlaxEngine/DLSS

Pinned source revision: 91ec2bcecd109e8729c2aa137a7a5d9bafcdb521

Discovery topic: DLSS

Description: NVIDIA DLSS plugin for Flax Engine

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"sss": ["Source/DLSS/DLSSSettings.h"], "dlss": ["Content/DLSS.Icon.flax", "DLSS.flaxproj", "Source/DLSS/DLSS.Build.cs", "Source/DLSS/DLSS.cpp", "Source/DLSS/DLSS.cs", "Source/DLSS/DLSS.h", "Source/DLSS/DLSSPostFx.cpp", "Source/DLSS/DLSSPostFx.h", "Source/DLSS/DLSSSettings.h", "Source/DLSS/NGXWrapper.cpp", "Source/DLSS/NGXWrapper.h", "Source/DLSS/Types.h", "Source/DLSSEditorTarget.Build.cs", "Source/DLSSTarget.Build.cs"]}

## Source file: Source/DLSS/DLSSSettings.h

{
  "path": "Source/DLSS/DLSSSettings.h",
  "blob_sha": "afdfeff041eb22d5b6097ed4f8321224cb96b910",
  "sha256": "728328391dd59725530cb2ec090933373d68823eab0c02214bb95a07ca888d0f",
  "lines": 27,
  "bytes": 819,
  "selection_score": 30,
  "keyword_line_counts": {
    "sss": 3,
    "dlss": 7
  },
  "url": "https://github.com/FlaxEngine/DLSS/blob/91ec2bcecd109e8729c2aa137a7a5d9bafcdb521/Source/DLSS/DLSSSettings.h",
  "function_signature_candidates": []
}

```text

﻿#pragma once

#include "Engine/Core/Config/Settings.h"
#include "Engine/Scripting/ScriptingObject.h"

/// <summary>
/// The settings for NVIDIA DLSS plugin.
/// </summary>
API_CLASS(Namespace="NVIDIA") class DLSS_API DLSSSettings : public SettingsBase
{
    API_AUTO_SERIALIZATION();
    DECLARE_SCRIPTING_TYPE_NO_SPAWN(DLSSSettings);
    DECLARE_SETTINGS_GETTER(DLSSSettings);

public:
    // App ID to pass for DLSS to identify the app. 0 if unused.
    API_FIELD(Attributes="EditorOrder(0)")
    uint32 AppId = 0;

    // Project ID to pass for DLSS to identify the project. Empty if unused.
    API_FIELD(Attributes="EditorOrder(10)")
    StringAnsi ProjectId;

    // If checked, DLSS initialization will be delayed until actually used.
    API_FIELD(Attributes="EditorOrder(100)")
    bool LazyInit = true;
};


```

## Source file: Source/DLSS/DLSS.h

{
  "path": "Source/DLSS/DLSS.h",
  "blob_sha": "88946ac960ea073d6a2315cb40d108a1052b18fc",
  "sha256": "dfabae90d9c307584269b963a65ccf8446ab4a19c1241a9cca88d63cfe94cc02",
  "lines": 64,
  "bytes": 2026,
  "selection_score": 15,
  "keyword_line_counts": {
    "sss": 2,
    "dlss": 17
  },
  "url": "https://github.com/FlaxEngine/DLSS/blob/91ec2bcecd109e8729c2aa137a7a5d9bafcdb521/Source/DLSS/DLSS.h",
  "function_signature_candidates": [
    {
      "line": 47,
      "text": "API_FUNCTION() void ApplyRecommendedSettings(DLSSQuality quality = DLSSQuality::MAX);"
    },
    {
      "line": 55,
      "text": "API_FUNCTION() void QueryRecommendedSettings(API_PARAM(ref) const Int2& displaySize, API_PARAM(Out) DLSSRecommendedSettings& result, DLSSQuality quality = DLSSQuality::MAX);"
    },
    {
      "line": 58,
      "text": "void DelayInit();"
    },
    {
      "line": 62,
      "text": "void Initialize() override;"
    },
    {
      "line": 63,
      "text": "void Deinitialize() override;"
    }
  ]
}

```text

﻿#pragma once

#include "Engine/Scripting/Plugins/GamePlugin.h"
#include "Types.h"
#include "NGXWrapper.h"

class DLSSPostFx;

/// <summary>
/// DLSS plugin.
/// </summary>
API_CLASS(Namespace="NVIDIA") class DLSS_API DLSS : public GamePlugin
{
    friend DLSSPostFx;
    DECLARE_SCRIPTING_TYPE(DLSS);

private:
    NGXWrapper _ngx;
    DLSSSupport _support = DLSSSupport::NotSupported;
    bool _delayInit = false;

public:
    /// <summary>
    /// DLSS post process effect.
    /// </summary>
    API_FIELD(ReadOnly) DLSSPostFx* PostFx = nullptr;

    /// <summary>
    /// DLSS support information.
    /// </summary>
    API_PROPERTY() DLSSSupport GetSupport() const;

    /// <summary>
    /// DLSS upscaling quality.
    /// </summary>
    API_FIELD() DLSSQuality Quality = DLSSQuality::Balanced;

    /// <summary>
    /// Softening or sharpening factor to apply during the DLSS pass. Negative values soften the image, positive values sharpen. In range [-1; 1].
    /// </summary>
    API_FIELD() float Sharpness = 0.0f;

    /// <summary>
    /// Calculates the optimal settings for the rendering into the certain display resolution at given quality.
    /// </summary>
    /// <param name="quality">DLSS quality, MAX to use current setting.</param>
    API_FUNCTION() void ApplyRecommendedSettings(DLSSQuality quality = DLSSQuality::MAX);

    /// <summary>
    /// Calculates the optimal settings for the rendering into the certain display resolution at given quality.
    /// </summary>
    /// <param name="displaySize">Display (output) resolution (in pixels).</param>
    /// <param name="result">Output settings.</param>
    /// <param name="quality">DLSS quality, MAX to use current setting.</param>
    API_FUNCTION() void QueryRecommendedSettings(API_PARAM(ref) const Int2& displaySize, API_PARAM(Out) DLSSRecommendedSettings& result, DLSSQuality quality = DLSSQuality::MAX);

private:
    void DelayInit();

public:
    // [GamePlugin]
    void Initialize() override;
    void Deinitialize() override;
};


```

## Source file: Source/DLSS/DLSS.cs

{
  "path": "Source/DLSS/DLSS.cs",
  "blob_sha": "e2eb6226da3e85ac088151a5088d4304c37c2e93",
  "sha256": "b19810fead75962eadf786af31e687e087fed780980b51a790cbd83b4974f798",
  "lines": 56,
  "bytes": 1500,
  "selection_score": 15,
  "keyword_line_counts": {
    "sss": 1,
    "dlss": 9
  },
  "url": "https://github.com/FlaxEngine/DLSS/blob/91ec2bcecd109e8729c2aa137a7a5d9bafcdb521/Source/DLSS/DLSS.cs",
  "function_signature_candidates": [
    {
      "line": 37,
      "text": "public override void InitializeEditor()"
    },
    {
      "line": 46,
      "text": "public override void Deinitialize()"
    }
  ]
}

```text

﻿#if FLAX_EDITOR

using System;
using FlaxEngine;
using FlaxEditor;
using FlaxEditor.Content;

namespace NVIDIA
{
    /// <summary>
    /// DLSS plugin for Editor.
    /// </summary>
    public sealed class DLSSEditor : EditorPlugin
    {
        private AssetProxy _assetProxy;

        /// <summary>
        /// Initializes a new instance of the <see cref="DLSSEditor"/> class.
        /// </summary>
        public DLSSEditor()
        {
            _description = new PluginDescription
            {
                Name = "DLSS",
                Category = "Rendering",
                Description = "DLSS is a revolutionary breakthrough in AI-powered graphics upscaling technology that massively boosts performance.",
                Author = "NVIDIA",
                RepositoryUrl = "https://github.com/FlaxEngine/DLSS",
                Version = new Version(2, 4),
            };
        }

        /// <inheritdoc />
        public override Type GamePluginType => typeof(DLSS);

        /// <inheritdoc />
        public override void InitializeEditor()
        {
            base.InitializeEditor();

            _assetProxy = new CustomSettingsProxy(typeof(DLSSSettings), "DLSS");
            Editor.ContentDatabase.Proxy.Add(_assetProxy);
        }

        /// <inheritdoc />
        public override void Deinitialize()
        {
            Editor.ContentDatabase.Proxy.Remove(_assetProxy);
            _assetProxy = null;

            base.Deinitialize();
        }
    }
}

#endif


```

## Source file: Source/DLSS/DLSS.cpp

{
  "path": "Source/DLSS/DLSS.cpp",
  "blob_sha": "604cf8200c706451a7d3a5988dc9f3c812b2e92c",
  "sha256": "9b57962aae8555cbd070b77c081cbf86d3fb08b4501b98af897d0bdfaed5bcbc",
  "lines": 92,
  "bytes": 2747,
  "selection_score": 15,
  "keyword_line_counts": {
    "sss": 6,
    "dlss": 23
  },
  "url": "https://github.com/FlaxEngine/DLSS/blob/91ec2bcecd109e8729c2aa137a7a5d9bafcdb521/Source/DLSS/DLSS.cpp",
  "function_signature_candidates": []
}

```text

﻿#include "DLSS.h"
#include "DLSSPostFx.h"
#include "DLSSSettings.h"
#include "Engine/Core/Log.h"
#include "Engine/Core/Config/GameSettings.h"
#include "Engine/Content/Content.h"
#include "Engine/Content/JsonAsset.h"
#include "Engine/Graphics/RenderTask.h"
#include "Engine/Profiler/ProfilerCPU.h"

IMPLEMENT_GAME_SETTINGS_GETTER(DLSSSettings, "DLSS");

DLSS::DLSS(const SpawnParams& params)
    : GamePlugin(params)
{
    _description.Name = TEXT("DLSS");
#if USE_EDITOR
    _description.Category = TEXT("Rendering");
    _description.Description = TEXT("DLSS is a revolutionary breakthrough in AI-powered graphics upscaling technology that massively boosts performance.");
    _description.Author = TEXT("NVIDIA");
    _description.RepositoryUrl = TEXT("https://github.com/FlaxEngine/DLSS");
#endif
    _description.Version = Version(3, 1, 0);
}

DLSSSupport DLSS::GetSupport() const
{
    if (_delayInit)
        const_cast<DLSS*>(this)->DelayInit();
    return _support;
}

void DLSS::ApplyRecommendedSettings(DLSSQuality quality)
{
    auto task = MainRenderTask::Instance;
    if (!task)
        return;
    DLSSRecommendedSettings settings;
    const Float2 outputSize = task->GetOutputViewport().Size;
    QueryRecommendedSettings(outputSize, settings, quality);
    task->RenderingPercentage = Math::Min((float)settings.ResolutionOptimal.X / outputSize.X, (float)settings.ResolutionOptimal.Y / outputSize.Y);
    Sharpness = settings.Sharpness;
}

void DLSS::QueryRecommendedSettings(const Int2& displaySize, DLSSRecommendedSettings& result, DLSSQuality quality)
{
    if (quality == DLSSQuality::MAX)
        quality = Quality;
    _ngx.QueryRecommendedSettings(displaySize, result, quality);
}

void DLSS::DelayInit()
{
    PROFILE_CPU();
    _delayInit = false;
    const auto settings = DLSSSettings::Get();
    LOG(Info, "Initializing DLSS with AppId={}, ProjectId={}", settings->AppId, String(settings->ProjectId));
    if (_ngx.Initialize(settings->AppId, settings->ProjectId, _support))
    {
        LOG(Warning, "DLSS is not supported on this platform.");
        return;
    }
}

void DLSS::Initialize()
{
    GamePlugin::Initialize();

    // TODO: apply global mip bias to texture groups samplers to increase texturing quality
    PostFx = New<DLSSPostFx>();
    SceneRenderTask::AddGlobalCustomPostFx(PostFx);

    const auto settings = DLSSSettings::Get();
    _support = DLSSSupport::NotSupported;
    _delayInit = settings->LazyInit;
    if (_delayInit)
        return;
    DelayInit();
}

void DLSS::Deinitialize()
{
    if (PostFx)
    {
        SceneRenderTask::RemoveGlobalCustomPostFx(PostFx);
        PostFx->DeleteObject();
        PostFx = nullptr;
    }
    _ngx.Shutdown();

    GamePlugin::Deinitialize();
}


```
