# Facial rendering source scan: 2600th/dlss5-video-player

Repository: https://github.com/2600th/dlss5-video-player

Pinned source revision: c55d29f73ccaa41fcc4d8424694d7f2feb78104c

Discovery topic: DLSS

Description: Windows player that runs video, photos or GIFs through NVIDIA's DLSS 5 neural renderer, then compares the result with the original frame by frame. Local files or YouTube. Needs an RTX card and driver 610.47+. Community project, unsigned runtime. C++20, D3D12.

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE", "assets/tabler/LICENSE"]

Topic paths: {"shader": ["src/NvofResolveShader.h", "src/TemporalStabilityShader.h"], "dlss": ["THIRD_PARTY_LICENSES/dlss5-feeder-MIT.txt", "assets/DLSSVideoPlayer.ico", "docs/DLSS5_SETUP.md", "src/DLSSBackend.cpp", "src/DLSSBackend.h", "src/DLSSGBackend.cpp", "src/DLSSGBackend.h", "src/DLSSVideoPlayer.manifest", "tests/DlssgEvaluateSmoke.cpp", "tests/DlssgProbeSmoke.cpp"], "rig": ["docs/measurements/camera-original-20260914/REPORT.md", "docs/measurements/q1-gate-camera-original-20260915/REPORT.md", "docs/measurements/q1-gate-camera-original-20260915/tables.py", "docs/measurements/q1-gate-camera-original-20260915/tables.txt", "docs/screenshots/current/original-comparison.jpg", "site/src/assets/gallery/gta-player-original-1440.avif", "site/src/assets/gallery/gta-player-original-1440.webp", "site/src/assets/gallery/gta-player-original-960.avif", "site/src/assets/gallery/gta-player-original-960.webp", "site/src/assets/hero/hero-original-1280.avif", "site/src/assets/hero/hero-original-1280.webp", "site/src/assets/hero/hero-original-1920.avif", "site/src/assets/hero/hero-original-1920.webp", "site/src/assets/hero/hero-original-960.avif", "site/src/assets/hero/hero-original-960.webp", "site/src/assets/hero/hero-original.jpg", "tools/benchmark/camera-original.digests.json", "tools/benchmark/fetch_camera_original.ps1"], "face": [".impeccable/surfaces/site-src-index-html.md", "docs/measurements/runtime-comparison-20260907/face-comparison.png", "tools/demo-video/make-face-comparison.py"]}

## Source file: src/DLSSBackend.h

{
  "path": "src/DLSSBackend.h",
  "blob_sha": "9f424b4a70ec8710c97f9d881891f037a5acb3cc",
  "sha256": "3ba5d13de685fd946692dd050b15d9e5a728a2e8a307ec2d931387512de11a1b",
  "lines": 94,
  "bytes": 4568,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 1,
    "shader": 1,
    "dlss": 6
  },
  "url": "https://github.com/2600th/dlss5-video-player/blob/c55d29f73ccaa41fcc4d8424694d7f2feb78104c/src/DLSSBackend.h",
  "function_signature_candidates": [
    {
      "line": 24,
      "text": "bool Initialize(ID3D12Device* device, ID3D12GraphicsCommandList* cmd,"
    },
    {
      "line": 28,
      "text": "bool EnsureFeature(ID3D12GraphicsCommandList* cmd);"
    },
    {
      "line": 29,
      "text": "bool RecreateFeature(ID3D12GraphicsCommandList* cmd);"
    },
    {
      "line": 38,
      "text": "bool ReleaseFeatureFreeingMemory();"
    },
    {
      "line": 39,
      "text": "bool FeatureCreated() const { return m_handle != nullptr; }"
    },
    {
      "line": 41,
      "text": "bool Evaluate(ID3D12GraphicsCommandList* cmd,"
    },
    {
      "line": 48,
      "text": "void Shutdown();"
    },
    {
      "line": 53,
      "text": "void SetExposureTexture(ID3D12Resource* exposure) { m_exposure = exposure; }"
    },
    {
      "line": 55,
      "text": "bool Available() const { return m_available && m_initialized && m_params != nullptr; }"
    },
    {
      "line": 64,
      "text": "bool SourceOutsideSupportedRange() const { return m_sourceOutsideRange; }"
    },
    {
      "line": 68,
      "text": "bool CreateFeature(ID3D12GraphicsCommandList* cmd);"
    },
    {
      "line": 69,
      "text": "void FillCreateParameters();"
    },
    {
      "line": 70,
      "text": "void FillEvaluateParameters(ID3D12Resource* color,"
    }
  ]
}

```text

#pragma once
#include <d3d12.h>
#include <cstdint>
#include <nvsdk_ngx.h>
#include <nvsdk_ngx_helpers.h>
#include "NgxSession.h"

constexpr NVSDK_NGX_PerfQuality_Value DefaultNeuralCarrierQuality() noexcept
{
    // DLAA preserves the hook-visible NGX evaluation contract at 1:1 resolution,
    // so neural rendering can run without enabling spatial DLSS upscaling.
    return NVSDK_NGX_PerfQuality_Value_DLAA;
}

// Direct NGX DLSS-SR host.
// The player deliberately uses the raw NVSDK_NGX_D3D12_CreateFeature / EvaluateFeature_C
// entry points after explicitly populating the parameter block. This makes the
// full DLSS contract visible to ReShade/RenoDX hooks instead of hiding it behind
// only the inline helper wrappers.
class DLSSBackend {
public:
    ~DLSSBackend();

    bool Initialize(ID3D12Device* device, ID3D12GraphicsCommandList* cmd,
                    uint32_t sourceW, uint32_t sourceH,
                    uint32_t outputW, uint32_t outputH,
                    NVSDK_NGX_PerfQuality_Value quality, bool preserveSource = false);
    bool EnsureFeature(ID3D12GraphicsCommandList* cmd);
    bool RecreateFeature(ID3D12GraphicsCommandList* cmd);
    // Hands the feature back after asking the runtime to free its memory with
    // it (NVSDK_NGX_Parameter_FreeMemOnReleaseFeature). For an idle helper
    // only: everything else this backend holds - the NGX session, the
    // parameter block, the negotiated sizes - stays, so the next
    // EnsureFeature re-creates the feature without re-initializing NGX.
    // False when there was no feature to hand back. Whether the runtime
    // honours the request is not observable here; the caller measures the
    // adapter on both sides of the call instead.
    bool ReleaseFeatureFreeingMemory();
    bool FeatureCreated() const { return m_handle != nullptr; }
    uint64_t EvaluationCount() const { return m_evaluations; }
    bool Evaluate(ID3D12GraphicsCommandList* cmd,
                  ID3D12Resource* color,
                  ID3D12Resource* output,
                  ID3D12Resource* depth,
                  ID3D12Resource* motion,
                  bool reset,
                  float frameTimeMs);
    void Shutdown();
    // A 1x1 exposure the evaluate reads instead of metering its own (ExposurePolicy.h).
    // Set before the feature is created: with one, the feature is created without
    // AutoExposure and every evaluate binds it; without one - the default - nothing
    // changes. The resource must be in NON_PIXEL_SHADER_RESOURCE at evaluate time.
    void SetExposureTexture(ID3D12Resource* exposure) { m_exposure = exposure; }

    bool Available() const { return m_available && m_initialized && m_params != nullptr; }
    uint32_t RenderWidth() const { return m_renderW; }
    uint32_t RenderHeight() const { return m_renderH; }
    // The output the runtime accepted, which may be smaller than the one asked
    // for when the source could not reach it.
    uint32_t OutputWidth() const { return m_outputW; }
    uint32_t OutputHeight() const { return m_outputH; }
    // Set when no output this source can reach was found, so the caller can say
    // that rather than reporting DLSS as simply unavailable.
    bool SourceOutsideSupportedRange() const { return m_sourceOutsideRange; }
    NVSDK_NGX_Result LastResult() const { return m_lastResult; }

private:
    bool CreateFeature(ID3D12GraphicsCommandList* cmd);
    void FillCreateParameters();
    void FillEvaluateParameters(ID3D12Resource* color,
                                ID3D12Resource* output,
                                ID3D12Resource* depth,
                                ID3D12Resource* motion,
                                bool reset,
                                float frameTimeMs);

    ID3D12Device* m_device = nullptr;
    const void* m_sessionKey = nullptr;
    NVSDK_NGX_Parameter* m_params = nullptr;
    NVSDK_NGX_Handle* m_handle = nullptr;
    uint32_t m_renderW = 0, m_renderH = 0;
    uint32_t m_optimalW = 0, m_optimalH = 0;
    uint32_t m_minW = 0, m_minH = 0, m_maxW = 0, m_maxH = 0;
    uint32_t m_outputW = 0, m_outputH = 0;
    NVSDK_NGX_PerfQuality_Value m_quality = DefaultNeuralCarrierQuality();
    NVSDK_NGX_Result m_lastResult = NVSDK_NGX_Result_Fail;
    bool m_initialized = false;
    bool m_sessionLeaseAcquired = false;
    bool m_available = false;
    bool m_sourceOutsideRange = false;
    ngx_session_detail::FeatureCreateGate m_featureCreateGate;
    uint64_t m_evaluations = 0;
    ID3D12Resource* m_exposure = nullptr;  // owned by the renderer
};


```

## Source file: src/DLSSGBackend.h

{
  "path": "src/DLSSGBackend.h",
  "blob_sha": "8aa3b7c222238815fbe9f5e987471fdb764b517c",
  "sha256": "a422994a2619b8d924593e276a37b0d2f449d4d123467739edd6df7ca5a4d739",
  "lines": 182,
  "bytes": 10862,
  "selection_score": 15,
  "keyword_line_counts": {
    "dlss": 23
  },
  "url": "https://github.com/2600th/dlss5-video-player/blob/c55d29f73ccaa41fcc4d8424694d7f2feb78104c/src/DLSSGBackend.h",
  "function_signature_candidates": [
    {
      "line": 90,
      "text": "bool Initialize(ID3D12Device* device, ID3D12GraphicsCommandList* cmd,"
    },
    {
      "line": 108,
      "text": "bool Evaluate(ID3D12GraphicsCommandList* cmd,"
    },
    {
      "line": 118,
      "text": "bool FeatureCreated() const { return m_handle != nullptr; }"
    },
    {
      "line": 142,
      "text": "void SetMotionVectorUnits(MotionVectorUnits units) { m_mvecUnits = units; }"
    },
    {
      "line": 144,
      "text": "void Shutdown();"
    },
    {
      "line": 149,
      "text": "void Abandon();"
    },
    {
      "line": 155,
      "text": "bool Available() const { return m_available; }"
    },
    {
      "line": 159,
      "text": "bool SessionEstablished() const { return m_sessionLeaseAcquired && m_params != nullptr; }"
    },
    {
      "line": 163,
      "text": "bool AcquireSession(ID3D12Device* device);"
    }
  ]
}

```text

#pragma once
#include <d3d12.h>
#include <dxgiformat.h>
#include <cstdint>
#include <string>
#include <nvsdk_ngx.h>
#include <nvsdk_ngx_helpers.h>
#include <nvsdk_ngx_helpers_dlssg.h>
#include "NgxSession.h"

// What the runtime answered when asked for DLSS Frame Generation on this
// machine. Every field is measured: nothing here is inferred from the GPU
// name, the driver version or the presence of nvngx_dlssg.dll.
struct DLSSGCapability {
    bool available = false;          // feature created (the caller releases it: ReleaseProbedFeature)
    uint32_t multiFrameCountMax = 0; // generated frames per source pair the runtime admits (1 => 2x)
    bool hagsEnabled = false;        // HwSchMode == 2
    NVSDK_NGX_Result createResult = NVSDK_NGX_Result_Fail;
    std::wstring detail;             // human-readable reason when !available
};

// Direct NGX DLSS-G (Frame Generation) host.
//
// Frame Generation is normally reached through Streamline, whose sl.dlss_g.dll
// owns the swapchain and drives presentation and pacing. This player holds its
// own swapchain and calls NGX directly, so whether the raw
// NVSDK_NGX_D3D12_CreateFeature path admits NVSDK_NGX_Feature_FrameGeneration
// at all was an open question that only hardware could answer, and whether a
// feature it admits actually produces an intermediate image is a second one:
// NVSDK_NGX_Result_Success with an untouched output texture is a negative
// result, not a working feature. So there are two entry points here, in the
// order the two measurements had to be made. Probe creates the feature and
// records the NVSDK_NGX_Result, answering admission; the caller releases that
// feature once the create work has retired. Initialize creates the same
// feature and keeps it, so Evaluate can run on it and a caller can read the
// interpolated frame back.
//
// What Probe measured on 2026-09-17 (RTX 5090, driver 616.64, Windows 11 26200,
// no Streamline module loaded): the create returns NVSDK_NGX_Result_Success
// at 1920x1080 B8G8R8A8_UNORM and DLSSG.MultiFrameCountMax reads 5 - but only
// with nvngx_dlssg.dll resolvable beside the executable. Without it the same
// create answers 0xbad0000b and the capability block reports
// FrameGeneration.Available=0 with FeatureInitResult=0xbad00004, even though
// NGX locates and logs the driver-store fallback snippet. So raw-NGX DLSS-G
// admission is real on this machine and it is conditional on shipping the
// snippet, exactly as DLSS-SR ships nvngx_dlss.dll.
//
// What Evaluate measured on the same machine, same day, through
// tests/DlssgEvaluateSmoke.cpp - a synthetic pair whose only content is a
// 200x200 square translated exactly +200 px between two 1920x1080 frames, so
// the true intermediate frame has one correct answer: the raw evaluate path
// returns Success and writes a real intermediate frame. All 2073600 output
// pixels were written over a sentinel fill, and the square landed with its
// horizontal centroid at 812.7 against 699.5 in frame A and 899.5 in frame B -
// between the two inputs, differing from both (mean channel difference 5.4 and
// 4.4 of 255), soft-edged over a 736..894 span, and 13.2 px past the 799.5
// midpoint toward the newer frame. Production is therefore real, not just
// admission.
//
// It is not produced from the motion buffer. The same pair was evaluated three
// times - motion written in backbuffer pixels, motion written in normalized
// screen units, and motion deliberately zeroed, which claims nothing moved
// while the colour pair shows a 200 px jump - and the output was the same all
// three times (centroid 812.73 / 812.73 / 812.79, mean channel difference
// between runs 0.00 and 0.01 of 255). Whatever this build interpolates from,
// the tagged DLSSG.MVecs are not it, so nothing may claim that a better motion
// estimate buys a better generated frame until a runtime is measured that
// reads them.
class DLSSGBackend {
public:
    ~DLSSGBackend();

    // Creates the FrameGeneration feature at the given backbuffer geometry to
    // answer whether the runtime admits it. The create records work on `cmd`
    // that references the feature, so the feature is NOT released here: it is
    // held apart from the one Initialize keeps (Evaluate never sees it) until
    // the caller has submitted `cmd`, waited for it to retire and called
    // ReleaseProbedFeature - or Shutdown, which releases it too. Releasing it
    // here, as Probe did, freed the feature before the GPU ran its create.
    DLSSGCapability Probe(ID3D12Device* device, ID3D12GraphicsCommandList* cmd,
                          uint32_t width, uint32_t height, DXGI_FORMAT backbufferFormat);
    // Releases the feature Probe created and answers with the release's own
    // result, Success when there is none. Only after the command list Probe
    // recorded into has retired; for one that never did, Abandon forgets it.
    NVSDK_NGX_Result ReleaseProbedFeature();

    // Geometry and format the feature is created for; backbuffer-resolution inputs.
    // Unlike Probe the feature is kept, so the create work this records on `cmd`
    // has to be submitted and retired before the first Evaluate.
    bool Initialize(ID3D12Device* device, ID3D12GraphicsCommandList* cmd,
                    uint32_t width, uint32_t height, DXGI_FORMAT backbufferFormat);

    // Generates intermediate frame `multiFrameIndex` (1-based, <= multiFrameCount)
    // between the PREVIOUS evaluated backbuffer and `backbuffer`.
    // `motion` points from the current frame to the previous one, at backbuffer
    // resolution. `reset` breaks temporal continuity (first frame, scene cut).
    //
    // `backbufferFrameId` is DLSSG.BackbufferFrameID: the runtime's optional
    // declaration of frame progression, documented in nvsdk_ngx_defs_dlssg.h as
    // a counter that increments by one per fully rendered backbuffer frame and
    // keeps incrementing while generation is off. A caller that presents real
    // frames has one already; an offline caller has to state it, because the
    // alternative is a runtime deriving progression from nothing. 0 leaves the
    // key unset, which is what the capability probe wants - it creates a
    // feature and never evaluates. Every index generated between the same pair
    // carries the id of the newer frame of that pair: the pair is one rendered
    // frame's worth of progression however many frames are generated inside it.
    bool Evaluate(ID3D12GraphicsCommandList* cmd,
                  ID3D12Resource* backbuffer,
                  ID3D12Resource* motion,
                  ID3D12Resource* depth,
                  ID3D12Resource* outputInterpolated,
                  uint32_t multiFrameCount,
                  uint32_t multiFrameIndex,
                  bool reset,
                  uint64_t backbufferFrameId = 0);
    uint64_t EvaluationCount() const { return m_evaluations; }
    bool FeatureCreated() const { return m_handle != nullptr; }
    // DLSSG.MultiFrameCountMax as the runtime reported it to Initialize: the
    // most frames it will generate between one source pair, so the largest
    // multiplier is one more. 0 until Initialize has run, which is the same
    // "no answer" every other measured field here uses.
    uint32_t MultiFrameCountMax() const { return m_multiFrameCountMax; }

    // Units the motion-vector texture handed to Evaluate is written in. DLSS-G
    // takes no pixel-space option: DLSSG.MvecScale{X,Y} exist to normalize the
    // buffer into [-1,1], so the units are declared here and the scale is
    // derived from them rather than left to a caller to get consistent.
    // BackbufferPixels is the default because that is what this project's
    // motion producers emit - TemporalGuides writes "current -> previous,
    // already in DLSS input pixels" and the DLSS-SR path consumes it with
    // MV_Scale 1.0 - so the pixel buffer is the one that already exists.
    // NormalizedScreen exists because whether the runtime applies MvecScale at
    // all on the raw path could not be read out of the headers, and handing it
    // the same displacement under both conventions was the way to find out.
    // The answer measured on 2026-09-17 was that it cannot be told from the
    // output at all: both conventions, and a zeroed buffer, produced the same
    // interpolated frame. The declaration is still made correctly here, because
    // the option that is silently wrong the day a runtime does start reading
    // DLSSG.MVecs is the one that costs a debugging session.
    enum class MotionVectorUnits { BackbufferPixels, NormalizedScreen };
    void SetMotionVectorUnits(MotionVectorUnits units) { m_mvecUnits = units; }

    void Shutdown();
    // For a GPU that never retired the work this feature was recorded into:
    // forgets the feature, its parameters and the NGX session lease WITHOUT
    // releasing any of them, because releasing them under work that may still
    // be executing is the one thing worse than leaking them. Logged.
    void Abandon();

    // Whether the runtime admitted the feature: set by a Probe whose create
    // succeeded (and cleared again if its release is refused), and by an
    // Initialize that is holding a live feature. False until one of them succeeds and false again after
    // Shutdown, so no caller may read admission out of a call that did not run.
    bool Available() const { return m_available; }
    // True once NGX itself came up on the probe device and handed over a
    // parameter block, so a caller can tell a runtime refusal - which is a
    // verdict - from NGX never initializing, which is not one.
    bool SessionEstablished() const { return m_sessionLeaseAcquired && m_params != nullptr; }
    NVSDK_NGX_Result LastResult() const { return m_lastResult; }

private:
    bool AcquireSession(ID3D12Device* device);
    NVSDK_NGX_Result CreateFeature(ID3D12GraphicsCommandList* cmd,
                                   uint32_t width, uint32_t height, DXGI_FORMAT backbufferFormat);
    NVSDK_NGX_DLSSG_Opt_Eval_Params VideoEvalConstants(uint32_t multiFrameCount, uint32_t multiFrameIndex,
                                                       bool reset) const;

    ID3D12Device* m_device = nullptr;
    const void* m_sessionKey = nullptr;
    NVSDK_NGX_Parameter* m_params = nullptr;
    NVSDK_NGX_Handle* m_handle = nullptr;
    NVSDK_NGX_Handle* m_probeHandle = nullptr;  // Probe's, until ReleaseProbedFeature
    uint32_t m_width = 0, m_height = 0;
    DXGI_FORMAT m_backbufferFormat = DXGI_FORMAT_UNKNOWN;
    uint32_t m_multiFrameCountMax = 0;
    uint64_t m_evaluations = 0;
    MotionVectorUnits m_mvecUnits = MotionVectorUnits::BackbufferPixels;
    NVSDK_NGX_Result m_lastResult = NVSDK_NGX_Result_Fail;
    bool m_sessionLeaseAcquired = false;
    bool m_available = false;
};


```

## Source file: src/DLSSBackend.cpp

{
  "path": "src/DLSSBackend.cpp",
  "blob_sha": "d7d172f8dbf803d602c131dfd9c705340bfd3a53",
  "sha256": "4d1fb37beea1875fbef161aa3b26d76ae3492e5cfec296ee04df30949a8101f0",
  "lines": 473,
  "bytes": 26605,
  "selection_score": 15,
  "keyword_line_counts": {
    "subsurface": 1,
    "dlss": 61,
    "rig": 1,
    "face": 3
  },
  "url": "https://github.com/2600th/dlss5-video-player/blob/c55d29f73ccaa41fcc4d8424694d7f2feb78104c/src/DLSSBackend.cpp",
  "function_signature_candidates": []
}

```text

#include "DLSSBackend.h"
#include "PlatformPaths.h"
#include "HexText.h"
#include "Log.h"
#include "UpscalingPolicy.h"
#include <windows.h>
#include <filesystem>
#include <algorithm>
#include <iomanip>
#include <cmath>
#include <cctype>
#include <string>
#include <string_view>

DLSSBackend::~DLSSBackend() { Shutdown(); }

bool DLSSBackend::Initialize(ID3D12Device* device, ID3D12GraphicsCommandList*,
                             uint32_t sourceW, uint32_t sourceH,
                             uint32_t outputW, uint32_t outputH,
                             NVSDK_NGX_PerfQuality_Value quality, bool preserveSource) {
    m_device = device;
    m_outputW = outputW;
    m_outputH = outputH;
    m_quality = quality;

    // A MAX_PATH buffer with no return check used to put ngx_logs in the wrong
    // place on any install deeper than 260 characters: GetModuleFileNameW fills
    // the buffer, returns the size it was given, and parent_path() of that
    // truncation is a different directory. ModuleDirectory refuses a truncated
    // answer, and NGX is given no log directory at all rather than a wrong one.
    const auto moduleDirectory = platform_paths::ModuleDirectory();
    std::error_code ec;
    std::filesystem::path logDir;
    if (moduleDirectory) {
        logDir = *moduleDirectory / L"ngx_logs";
        std::filesystem::create_directories(logDir, ec);
    }

    // Custom engine/project identity is the officially supported NGX route for
    // non-engine samples. It is intentionally stable across runs.
    // IUnknown identity is stable even if the same singleton device is exposed
    // through different D3D12 interface pointers during an atomic renderer handoff.
    IUnknown* deviceIdentity = nullptr;
    if (!device || FAILED(device->QueryInterface(IID_IUnknown,
                                                  reinterpret_cast<void**>(&deviceIdentity)))) {
        m_lastResult = NVSDK_NGX_Result_Fail;
        LOG("NGX device identity lookup failed.");
        return false;
    }
    m_sessionKey = deviceIdentity;
    deviceIdentity->Release();

    // NGX and its feature DLLs report through this callback. The full stream
    // goes to ngx_logs/ as before; only lines that name a problem are copied
    // beside the player's own, so a refused feature explains itself in one
    // file without the ~170 lines of startup chatter each worker emits.
    //
    // The core-load sequence needs both halves, and filtering on "error" alone
    // gave only the first. NGX probes for its core beside the executable before
    // resolving it from the driver store; the app-local probe always fails with
    // 126, because the core is a driver component no application ships. So the
    // failure matched the filter, the resolution did not, and the log read as a
    // fatal error every session. The annotation below applies only to that
    // app-local probe: a driver-store path failing to load is the case where the
    // line means what it says, and it is passed through unannotated.
    static NVSDK_NGX_FeatureCommonInfo commonInfo{};
    commonInfo.LoggingInfo.LoggingCallback = [](const char* message, NVSDK_NGX_Logging_Level, NVSDK_NGX_Feature component) {
        std::string_view text = message ? message : "";
        while (!text.empty() && (text.back() == '\n' || text.back() == '\r')) text.remove_suffix(1);
        if (text.empty()) return;
        std::string lowered(text);
        std::transform(lowered.begin(), lowered.end(), lowered.begin(), [](unsigned char c) { return char(std::tolower(c)); });
        const bool coreLoadFailed = lowered.find("failed to load ngxcore") != std::string::npos;
        const bool fromDriverStore = lowered.find("driverstore") != std::string::npos;
        if (coreLoadFailed && !fromDriverStore) {
            LOG("[NGX feature " << int(component) << "] " << text
                << "  <- routine: the NGX core ships with the driver, not beside this"
                   " executable; the driver-store load is logged separately");
            return;
        }
        // Either outcome of the load that decides it, annotated by nothing.
        if (lowered.find("ngxloadcorelibrary") != std::string::npos) {
            LOG("[NGX feature " << int(component) << "] " << text);
            return;
        }
        for (std::string_view needle : {"error", "fail", "warn", "unsupported"}) {
            if (lowered.find(needle) != std::string::npos) {
                LOG("[NGX feature " << int(component) << "] " << text);
                return;
            }
        }
    };
    commonInfo.LoggingInfo.MinimumLoggingLevel = NVSDK_NGX_LOGGING_LEVEL_ON;

    const bool sessionAcquired = ngx_session_detail::ProcessRegistry().Acquire(
        m_sessionKey,
        [&] {
            m_lastResult = NVSDK_NGX_D3D12_Init_with_ProjectID(
                "50f09991-2962-44db-bad7-4be06dbbd1d2",
                NVSDK_NGX_ENGINE_TYPE_CUSTOM,
                "DLSSVideoPlayer-10.0",
                logDir.c_str(), device, &commonInfo, NVSDK_NGX_Version_API);
            return !NVSDK_NGX_FAILED(m_lastResult);
        });
    if (!sessionAcquired) {
        LOG("NGX Init failed result=" << HexText(m_lastResult));
        m_sessionKey = nullptr;
        return false;
    }
    m_sessionLeaseAcquired = true;
    m_initialized = true;

    m_lastResult = NVSDK_NGX_D3D12_GetCapabilityParameters(&m_params);
    if (NVSDK_NGX_FAILED(m_lastResult) || !m_params) {
        LOG("NGX GetCapabilityParameters failed result=" << HexText(m_lastResult));
        return false;
    }

    int srAvailable = 0;
    NVSDK_NGX_Result availResult = NVSDK_NGX_Parameter_GetI(
        m_params, NVSDK_NGX_Parameter_SuperSampling_Available, &srAvailable);
    if (!NVSDK_NGX_FAILED(availResult) && !srAvailable) {
        LOG("NGX reports SuperSampling.Available=0; native DLSS SR is unavailable on this system/runtime.");
        return false;
    }
    if (!NVSDK_NGX_FAILED(availResult)) {
        LOG("NGX capability: SuperSampling.Available=" << srAvailable);
    }

    float sharpness=0.0f;
    if (preserveSource) {
        // Select a runtime-supported dynamic range; never resize the decoded input
        // to force a nominal Quality/Performance ratio.
        //
        // Which source sizes an output admits is the runtime's answer, not a
        // ratio table's: NGX_DLSS_GET_OPTIMAL_SETTINGS returns an inclusive
        // [min,max] render range per quality mode, that range collapses onto the
        // optimal size wherever dynamic resolution is unsupported, and an
        // out-of-range Evaluate fails outright with FAIL_InvalidParameter (DLSS
        // Programming Guide 310.6.0, section 3.2.2). NVIDIA publishes no maximum
        // upscaling ratio, no fixed fraction for that minimum and no guidance for
        // a source that falls below it, so when the requested output is too large
        // for this source, shrink it by exactly the shortfall the runtime reported
        // and ask again rather than abandoning upscaling altogether. A 573-line
        // photo asked to reach 1440 lines used to get no upscaling at all.
        constexpr uint32_t kOutputRangeSearchLimit=3;
        const auto adoptOutput=[&](uint32_t w,uint32_t h){outputW=w;outputH=h;m_outputW=w;m_outputH=h;};
        bool supported=false;
        for (uint32_t search=0;search<kOutputRangeSearchLimit&&!supported;++search) {
            uint32_t nearestMinW=0,nearestMinH=0;
            for (auto candidate : {NVSDK_NGX_PerfQuality_Value_MaxQuality,
                                   NVSDK_NGX_PerfQuality_Value_Balanced,
                                   NVSDK_NGX_PerfQuality_Value_MaxPerf,
                                   NVSDK_NGX_PerfQuality_Value_UltraPerformance}) {
                m_lastResult=NGX_DLSS_GET_OPTIMAL_SETTINGS(m_params,outputW,outputH,candidate,
                    &m_optimalW,&m_optimalH,&m_maxW,&m_maxH,&m_minW,&m_minH,&sharpness);
                LOG("SR range query quality="<<candidate<<" result="<<HexText(m_lastResult)
                    <<" optimal="<<m_optimalW<<"x"<<m_optimalH<<" min="<<m_minW<<"x"<<m_minH<<" max="<<m_maxW<<"x"<<m_maxH);
                if (NVSDK_NGX_FAILED(m_lastResult)) continue;
                if (SourceFitsDLSSRange(sourceW,sourceH,outputW,outputH,m_minW,m_minH,m_maxW,m_maxH)) {
                    m_quality=candidate; supported=true; break;
                }
                // Only a source that sits below an advertised minimum can be
                // helped by a smaller output, and the loosest such minimum is the
                // one that costs the least output to reach.
                if (m_minW&&m_minH&&(sourceW<m_minW||sourceH<m_minH)&&
                    (!nearestMinW||m_minW<nearestMinW)) {nearestMinW=m_minW;nearestMinH=m_minH;}
            }
            if (supported) break;
            const auto reduced=AdmissibleDLSSOutput(sourceW,sourceH,outputW,outputH,nearestMinW,nearestMinH);
            if (!reduced.grows) break;
            LOG("DLSS SR output reduced from "<<outputW<<"x"<<outputH<<" to "<<reduced.width<<"x"<<reduced.height
                <<" so the "<<sourceW<<"x"<<sourceH<<" source lands inside the runtime-supported input range.");
            adoptOutput(reduced.width,reduced.height);
        }
        if (!supported) {
            m_sourceOutsideRange=true;
            LOG("DLSS SR source dimensions are outside the runtime-supported input range.");
            return false;
        }
    }
    else {
    m_lastResult = NGX_DLSS_GET_OPTIMAL_SETTINGS(m_params, outputW, outputH, quality,
        &m_optimalW, &m_optimalH, &m_maxW, &m_maxH, &m_minW, &m_minH, &sharpness);
    if (NVSDK_NGX_FAILED(m_lastResult) || !m_optimalW || !m_optimalH) {
        LOG("NGX optimal settings failed result=" << HexText(m_lastResult)
            << "; using 2/3 output dimensions fallback.");
        m_optimalW = std::max(1u, outputW * 2u / 3u);
        m_optimalH = std::max(1u, outputH * 2u / 3u);
        m_minW = m_maxW = m_optimalW;
        m_minH = m_maxH = m_optimalH;
    }
    }

    // Do not pre-upscale a low-resolution movie to the nominal DLSS quality input
    // before invoking DLSS.  Pick the source-equivalent size when the runtime's
    // dynamic-resolution range permits it, otherwise clamp to the advertised range.
    // This preserves the actual reconstruction job for DLSS instead of hiding it
    // behind a bilinear resize first.
    if (preserveSource) {
        m_renderW=sourceW; m_renderH=sourceH;
    } else if (quality == NVSDK_NGX_PerfQuality_Value_DLAA) {
        m_renderW = outputW; m_renderH = outputH;
    } else {
        const double outAR = outputH ? double(outputW) / double(outputH) : 1.0;
        const double sourcePixels = std::max(1.0, double(sourceW) * double(sourceH));
        double nativeW = std::sqrt(sourcePixels * outAR);
        double candidateW = std::min(double(m_optimalW), nativeW);
        const double minScale = (m_minW && m_minH) ? std::max(double(m_minW) / outputW, double(m_minH) / outputH) : 0.0;
        const double maxScale = (m_maxW && m_maxH) ? std::min(double(m_maxW) / outputW, double(m_maxH) / outputH) : 1.0;
        double scale = candidateW / double(outputW);
        if (minScale > 0.0 && maxScale >= minScale) scale = std::clamp(scale, minScale, maxScale);
        m_renderW = std::max(2u, uint32_t(std::lround(double(outputW) * scale)) & ~1u);
        m_renderH = std::max(2u, uint32_t(std::lround(double(outputH) * scale)) & ~1u);
    }

    LOG("NGX input policy: source=" << sourceW << "x" << sourceH
        << " optimal=" << m_optimalW << "x" << m_optimalH
        << " range=" << m_minW << "x" << m_minH << ".." << m_maxW << "x" << m_maxH
        << " selected=" << m_renderW << "x" << m_renderH
        << " output=" << outputW << "x" << outputH);

    // Prefer the current transformer preset when the runtime supports it.
    // Unsupported hints are simply ignored by older NGX builds.
    NVSDK_NGX_Parameter_SetI(m_params, NVSDK_NGX_Parameter_DLSS_Hint_Render_Preset_Quality,
                             NVSDK_NGX_DLSS_Hint_Render_Preset_K);
    NVSDK_NGX_Parameter_SetI(m_params, NVSDK_NGX_Parameter_DLSS_Hint_Render_Preset_Balanced,
                             NVSDK_NGX_DLSS_Hint_Render_Preset_K);
    NVSDK_NGX_Parameter_SetI(m_params, NVSDK_NGX_Parameter_DLSS_Hint_Render_Preset_Performance,
                             NVSDK_NGX_DLSS_Hint_Render_Preset_K);
    NVSDK_NGX_Parameter_SetI(m_params, NVSDK_NGX_Parameter_DLSS_Hint_Render_Preset_DLAA,
                             NVSDK_NGX_DLSS_Hint_Render_Preset_K);

    // Feature creation is deliberately deferred until the first rendered frame.
    // At that point D3D12/DXGI and ReShade add-ons are fully initialized, so a
    // generic RenoDX DLSS5 add-on can observe the *actual* CreateFeature call.
    m_available = true;
    LOG("NGX initialized. Deferred raw CreateFeature armed: input=" << m_renderW << "x" << m_renderH
        << " output=" << m_outputW << "x" << m_outputH);
    return true;
}

void DLSSBackend::FillCreateParameters() {
    // Match NVIDIA's official D3D12 helper contract even though we call the raw
    // CreateFeature symbol so ReShade/RenoDX can see it. Single-GPU apps use node 1.
    NVSDK_NGX_Parameter_SetUI(m_params, NVSDK_NGX_Parameter_CreationNodeMask, 1);
    NVSDK_NGX_Parameter_SetUI(m_params, NVSDK_NGX_Parameter_VisibilityNodeMask, 1);
    NVSDK_NGX_Parameter_SetUI(m_params, NVSDK_NGX_Parameter_Width, m_renderW);
    NVSDK_NGX_Parameter_SetUI(m_params, NVSDK_NGX_Parameter_Height, m_renderH);
    NVSDK_NGX_Parameter_SetUI(m_params, NVSDK_NGX_Parameter_OutWidth, m_outputW);
    NVSDK_NGX_Parameter_SetUI(m_params, NVSDK_NGX_Parameter_OutHeight, m_outputH);
    NVSDK_NGX_Parameter_SetI(m_params, NVSDK_NGX_Parameter_PerfQualityValue, m_quality);

    // AutoExposure unless the renderer supplies the exposure itself; the two are
    // exclusive, and the supplied one is what the SetExposureTexture caller asked for.
    const int flags = NVSDK_NGX_DLSS_Feature_Flags_MVLowRes |
                      (m_exposure ? 0 : NVSDK_NGX_DLSS_Feature_Flags_AutoExposure);
    // Depth is conventional HW depth: 0 = near, 1 = far, so no DepthInverted.
    // Motion vectors are generated without camera-jitter baked in, so no MVJittered.
    NVSDK_NGX_Parameter_SetI(m_params, NVSDK_NGX_Parameter_DLSS_Feature_Create_Flags, flags);
    NVSDK_NGX_Parameter_SetI(m_params, NVSDK_NGX_Parameter_DLSS_Enable_Output_Subrects, 0);
}

bool DLSSBackend::CreateFeature(ID3D12GraphicsCommandList* cmd) {
    if (!m_params || !cmd || !m_featureCreateGate.ShouldAttempt()) return false;
    if (m_handle) return true;

    FillCreateParameters();
    m_lastResult = NVSDK_NGX_D3D12_CreateFeature(
        cmd, NVSDK_NGX_Feature_SuperSampling, m_params, &m_handle);
    if (NVSDK_NGX_FAILED(m_lastResult) || !m_handle) {
        LOG("RAW NGX D3D12 CreateFeature failed result=" << HexText(m_lastResult));
        m_handle = nullptr;
        m_featureCreateGate.RecordFailure();
        return false;
    }

    LOG("RAW NGX D3D12 CreateFeature SUCCESS: feature=SuperSampling input="
        << std::dec << m_renderW << "x" << m_renderH << " output="
        << m_outputW << "x" << m_outputH
        << (m_exposure ? " flags=MVLowRes, supplied exposure" : " flags=MVLowRes|AutoExposure")
        << "; direct hook-visible contract");
    return true;
}

bool DLSSBackend::EnsureFeature(ID3D12GraphicsCommandList* cmd) {
    if (!Available() || !cmd) return false;
    return m_handle ? true : CreateFeature(cmd);
}

bool DLSSBackend::RecreateFeature(ID3D12GraphicsCommandList* cmd) {
    if (!Available() || !cmd) return false;
    if (m_handle) {
        NVSDK_NGX_D3D12_ReleaseFeature(m_handle);
        m_handle = nullptr;
    }
    m_evaluations = 0;
    m_featureCreateGate.Reset();
    LOG("Recreating RAW NGX DLSS feature for ReShade/RenoDX hook capture.");
    return CreateFeature(cmd);
}

bool DLSSBackend::ReleaseFeatureFreeingMemory() {
    if (!m_handle) return false;
    // NGX pools a feature's memory across an ordinary ReleaseFeature so the
    // next create is cheap, which is exactly wrong for a helper that is about
    // to sit idle holding it. The parameter below is the documented way to ask
    // for it back; it is set on this release only, so the mid-job recreate
    // path above keeps the pooling it wants.
    if (m_params) {
        NVSDK_NGX_Parameter_SetI(m_params, NVSDK_NGX_Parameter_FreeMemOnReleaseFeature, 1);
    }
    m_lastResult = NVSDK_NGX_D3D12_ReleaseFeature(m_handle);
    m_handle = nullptr;
    m_evaluations = 0;
    m_featureCreateGate.Reset();
    if (m_params) {
        NVSDK_NGX_Parameter_SetI(m_params, NVSDK_NGX_Parameter_FreeMemOnReleaseFeature, 0);
    }
    LOG("NGX feature released for idle with FreeMemOnReleaseFeature result="
        << HexText(m_lastResult));
    return true;
}

void DLSSBackend::FillEvaluateParameters(ID3D12Resource* color,
                                         ID3D12Resource* output,
                                         ID3D12Resource* depth,
                                         ID3D12Resource* motion,
                                         bool reset,
                                         float frameTimeMs) {
    // Required DLSS-SR resources.
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_Color, color);
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_Output, output);
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_Depth, depth);
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_MotionVectors, motion);

    // Required/meaningful temporal constants. The jitter offset is pinned to zero: the
    // colour buffer is a decoded video frame on a fixed sample grid, so there is no
    // sub-pixel camera phase to declare. Reporting one that nothing applied would make
    // the reconstruction resolve its history against an offset that never happened, and
    // applying one by resampling the frame only convolves it with a per-frame tent.
    NVSDK_NGX_Parameter_SetF(m_params, NVSDK_NGX_Parameter_Jitter_Offset_X, 0.0f);
    NVSDK_NGX_Parameter_SetF(m_params, NVSDK_NGX_Parameter_Jitter_Offset_Y, 0.0f);
    NVSDK_NGX_Parameter_SetF(m_params, NVSDK_NGX_Parameter_Sharpness, 0.0f);
    NVSDK_NGX_Parameter_SetI(m_params, NVSDK_NGX_Parameter_Reset, reset ? 1 : 0);

    // Buffer stores current-pixel -> previous-frame location directly in input pixels.
    NVSDK_NGX_Parameter_SetF(m_params, NVSDK_NGX_Parameter_MV_Scale_X, 1.0f);
    NVSDK_NGX_Parameter_SetF(m_params, NVSDK_NGX_Parameter_MV_Scale_Y, 1.0f);

    // Explicit extents/subrects make the contract unambiguous to both NGX and
    // generic add-ons which inspect the parameter block.
    NVSDK_NGX_Parameter_SetUI(m_params, NVSDK_NGX_Parameter_DLSS_Render_Subrect_Dimensions_Width, m_renderW);
    NVSDK_NGX_Parameter_SetUI(m_params, NVSDK_NGX_Parameter_DLSS_Render_Subrect_Dimensions_Height, m_renderH);
    NVSDK_NGX_Parameter_SetUI(m_params, NVSDK_NGX_Parameter_DLSS_Input_Color_Subrect_Base_X, 0);
    NVSDK_NGX_Parameter_SetUI(m_params, NVSDK_NGX_Parameter_DLSS_Input_Color_Subrect_Base_Y, 0);
    NVSDK_NGX_Parameter_SetUI(m_params, NVSDK_NGX_Parameter_DLSS_Input_Depth_Subrect_Base_X, 0);
    NVSDK_NGX_Parameter_SetUI(m_params, NVSDK_NGX_Parameter_DLSS_Input_Depth_Subrect_Base_Y, 0);
    NVSDK_NGX_Parameter_SetUI(m_params, NVSDK_NGX_Parameter_DLSS_Input_MV_SubrectBase_X, 0);
    NVSDK_NGX_Parameter_SetUI(m_params, NVSDK_NGX_Parameter_DLSS_Input_MV_SubrectBase_Y, 0);
    NVSDK_NGX_Parameter_SetUI(m_params, NVSDK_NGX_Parameter_DLSS_Output_Subrect_Base_X, 0);
    NVSDK_NGX_Parameter_SetUI(m_params, NVSDK_NGX_Parameter_DLSS_Output_Subrect_Base_Y, 0);

    // SDR video is converted to linear FP16 before DLSS. By default AutoExposure is
    // enabled and no exposure texture is bound; with a supplied one (ExposurePolicy.h)
    // it is bound below. These values define the colour scale either way.
    NVSDK_NGX_Parameter_SetF(m_params, NVSDK_NGX_Parameter_DLSS_Pre_Exposure, 1.0f);
    NVSDK_NGX_Parameter_SetF(m_params, NVSDK_NGX_Parameter_DLSS_Exposure_Scale, 1.0f);
    NVSDK_NGX_Parameter_SetF(m_params, NVSDK_NGX_Parameter_FrameTimeDeltaInMsec, frameTimeMs);
    NVSDK_NGX_Parameter_SetI(m_params, NVSDK_NGX_Parameter_DLSS_Indicator_Invert_X_Axis, 0);
    NVSDK_NGX_Parameter_SetI(m_params, NVSDK_NGX_Parameter_DLSS_Indicator_Invert_Y_Axis, 0);

    // Explicitly clear every optional research/engine guide that is not meaningful for
    // a decoded movie. This mirrors NVIDIA's helper behavior and prevents stale capability
    // parameter values from being interpreted as live resources by NGX or an interceptor.
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_TransparencyMask, nullptr);
#ifdef NVSDK_NGX_Parameter_DLSS_TransparencyLayer
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_DLSS_TransparencyLayer, nullptr);
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_DLSS_TransparencyLayerOpacity, nullptr);
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_DLSS_TransparencyLayerMvecs, nullptr);
#endif
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_ExposureTexture, m_exposure);
    NVSDK_NGX_Parameter_SetUI(m_params, NVSDK_NGX_Parameter_DLSS_Input_Translucency_SubrectBase_X, 0);
    NVSDK_NGX_Parameter_SetUI(m_params, NVSDK_NGX_Parameter_DLSS_Input_Translucency_SubrectBase_Y, 0);
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_GBuffer_Albedo, nullptr);
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_GBuffer_Roughness, nullptr);
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_GBuffer_Metallic, nullptr);
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_GBuffer_Specular, nullptr);
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_GBuffer_Subsurface, nullptr);
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_GBuffer_Normals, nullptr);
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_GBuffer_ShadingModelId, nullptr);
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_GBuffer_MaterialId, nullptr);
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_GBuffer_Atrrib_8, nullptr);
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_GBuffer_Atrrib_9, nullptr);
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_GBuffer_Atrrib_10, nullptr);
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_GBuffer_Atrrib_11, nullptr);
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_GBuffer_Atrrib_12, nullptr);
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_GBuffer_Atrrib_13, nullptr);
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_GBuffer_Atrrib_14, nullptr);
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_GBuffer_Atrrib_15, nullptr);
    NVSDK_NGX_Parameter_SetUI(m_params, NVSDK_NGX_Parameter_TonemapperType, NVSDK_NGX_TONEMAPPER_STRING);
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_MotionVectors3D, nullptr);
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_IsParticleMask, nullptr);
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_AnimatedTextureMask, nullptr);
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_DepthHighRes, nullptr);
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_Position_ViewSpace, nullptr);
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_RayTracingHitDistance, nullptr);
    NVSDK_NGX_Parameter_SetD3d12Resource(m_params, NVSDK_NGX_Parameter_MotionVectorsReflection, nullptr);
}

bool DLSSBackend::Evaluate(ID3D12GraphicsCommandList* cmd,
                           ID3D12Resource* color,
                           ID3D12Resource* output,
                           ID3D12Resource* depth,
                           ID3D12Resource* motion,
                           bool reset,
                           float frameTimeMs) {
    if (!Available() || !m_handle || !cmd || !color || !output || !depth || !motion) return false;

    FillEvaluateParameters(color, output, depth, motion, reset, frameTimeMs);

    // NVIDIA's current nvsdk_ngx_helpers.h finishes the D3D12 DLSS helper with
    // NVSDK_NGX_D3D12_EvaluateFeature_C. The leaked/generic RenoDX NR add-on also
    // explicitly looks for this symbol in several games, so make _C the primary path.
    // If an unusual/older runtime rejects it, retry the legacy non-_C export once.
    m_lastResult = NVSDK_NGX_D3D12_EvaluateFeature_C(cmd, m_handle, m_params, nullptr);
    const char* evalPath = "EvaluateFeature_C";
    if (NVSDK_NGX_FAILED(m_lastResult)) {
        const NVSDK_NGX_Result cResult = m_lastResult;
        LOG("RAW NGX D3D12 EvaluateFeature_C failed result=" << HexText(cResult)
            << "; trying legacy EvaluateFeature fallback.");
        m_lastResult = NVSDK_NGX_D3D12_EvaluateFeature(cmd, m_handle, m_params, nullptr);
        evalPath = "EvaluateFeature";
    }
    if (NVSDK_NGX_FAILED(m_lastResult)) {
        LOG("RAW NGX D3D12 " << evalPath << " failed result=" << HexText(m_lastResult));
        return false;
    }

    ++m_evaluations;
    if (m_evaluations == 1 || (m_evaluations % 300) == 0) {
        LOG("RAW NGX " << evalPath << " SUCCESS #" << std::dec << m_evaluations
            << " color=" << m_renderW << "x" << m_renderH
            << " depth=" << m_renderW << "x" << m_renderH
            << " mv=" << m_renderW << "x" << m_renderH
            << " output=" << m_outputW << "x" << m_outputH
            << " jitter=(0,0) reset=" << (reset?1:0));
    }
    return true;
}

void DLSSBackend::Shutdown() {
    if (m_handle) {
        NVSDK_NGX_D3D12_ReleaseFeature(m_handle);
        m_handle = nullptr;
    }
    if (m_params) {
        NVSDK_NGX_D3D12_DestroyParameters(m_params);
        m_params = nullptr;
    }
    if (m_sessionLeaseAcquired) {
        ngx_session_detail::ProcessRegistry().Release(
            m_sessionKey,
            [&] { NVSDK_NGX_D3D12_Shutdown1(m_device); });
        m_sessionLeaseAcquired = false;
    }
    m_initialized = false;
    m_sessionKey = nullptr;
    m_device = nullptr;
    m_available = false;
    m_evaluations = 0;
    m_featureCreateGate.Reset();
}


```

## Source file: src/DLSSGBackend.cpp

{
  "path": "src/DLSSGBackend.cpp",
  "blob_sha": "d763ac555abeca2ced3aac38ef0b809782c42412",
  "sha256": "dda793b54bd6db42d1bc2032a618fa3dc329ad1dbe1feb21236fe9cc9b34dcf4",
  "lines": 579,
  "bytes": 28889,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 1,
    "dlss": 49,
    "face": 3,
    "lip": 8
  },
  "url": "https://github.com/2600th/dlss5-video-player/blob/c55d29f73ccaa41fcc4d8424694d7f2feb78104c/src/DLSSGBackend.cpp",
  "function_signature_candidates": [
    {
      "line": 47,
      "text": "void Append(std::wstring& detail, const std::wstring& sentence)"
    }
  ]
}

```text

#include "DLSSGBackend.h"
#include "PlatformPaths.h"
#include "HexText.h"
#include "Log.h"
#include "NeuralPreflight.h"
#include <windows.h>
#include <cfloat>
#include <filesystem>
#include <optional>
#include <string>
#include <string_view>

#pragma comment(lib, "advapi32.lib")

namespace {

// HKLM\SYSTEM\CurrentControlSet\Control\GraphicsDrivers\HwSchMode is what the
// Windows "Hardware-accelerated GPU scheduling" switch writes: 2 with it on, 1
// with it off, and nothing at all where the switch was never touched - which
// is this machine (Windows 11 26200, driver 616.64), where the value does not
// exist and the runtime admitted Frame Generation regardless. So the value is
// measured and reported, an absent one is reported as absent rather than as
// off, and neither is ever gated on: the runtime is the only thing entitled
// to refuse, and a refusal HAGS would explain is a different finding from one
// it would not.
std::optional<uint32_t> HardwareScheduledGpuMode()
{
    DWORD mode = 0;
    DWORD size = sizeof(mode);
    if (RegGetValueW(HKEY_LOCAL_MACHINE, L"SYSTEM\\CurrentControlSet\\Control\\GraphicsDrivers",
                     L"HwSchMode", RRF_RT_REG_DWORD, nullptr, &mode, &size) != ERROR_SUCCESS) {
        return std::nullopt;
    }
    return mode;
}

// The measured HAGS state as one clause of a capability detail.
std::wstring HardwareSchedulingSentence(std::optional<uint32_t> mode)
{
    if (!mode) {
        return L"the HwSchMode value is absent from HKLM\\SYSTEM\\CurrentControlSet\\Control\\"
               L"GraphicsDrivers, so hardware-accelerated GPU scheduling is unconfirmed";
    }
    return L"HwSchMode=" + std::to_wstring(*mode) + L" so hardware-accelerated GPU scheduling is off";
}

void Append(std::wstring& detail, const std::wstring& sentence)
{
    if (!detail.empty()) detail += L"; ";
    detail += sentence;
}

} // namespace

DLSSGBackend::~DLSSGBackend() { Shutdown(); }

bool DLSSGBackend::AcquireSession(ID3D12Device* device)
{
    if (m_sessionLeaseAcquired && m_device == device && m_params) return true;
    if (m_device != device) Shutdown();
    m_device = device;

    // A MAX_PATH buffer with no return check used to put ngx_logs in the wrong
    // place on any install deeper than 260 characters: GetModuleFileNameW fills
    // the buffer, returns the size it was given, and parent_path() of that
    // truncation is a different directory. ModuleDirectory refuses a truncated
    // answer, and NGX is given no log directory at all rather than a wrong one.
    const auto moduleDirectory = platform_paths::ModuleDirectory();
    std::error_code ec;
    std::filesystem::path logDir;
    if (moduleDirectory) {
        logDir = *moduleDirectory / L"ngx_logs";
        std::filesystem::create_directories(logDir, ec);
    }

    // IUnknown identity, as the SR backend does it, so the lease below keys on
    // the same value for the same device no matter which D3D12 interface
    // pointer each backend was handed.
    IUnknown* deviceIdentity = nullptr;
    if (FAILED(device->QueryInterface(IID_IUnknown, reinterpret_cast<void**>(&deviceIdentity)))) {
        m_lastResult = NVSDK_NGX_Result_Fail;
        LOG("NGX device identity lookup failed for the DLSS-G probe.");
        m_device = nullptr;
        return false;
    }
    m_sessionKey = deviceIdentity;
    deviceIdentity->Release();

    // The SR backend's callback copies only lines that name a problem, because
    // its ~170 lines of startup chatter drown the player's own log every
    // session. This one copies everything: the raw-NGX Frame Generation path is
    // undocumented, so when it refuses, the surrounding lines are the only
    // statement of why, and reading that statement is the entire purpose of
    // this class.
    static NVSDK_NGX_FeatureCommonInfo commonInfo{};
    commonInfo.LoggingInfo.LoggingCallback = [](const char* message, NVSDK_NGX_Logging_Level,
                                                NVSDK_NGX_Feature component) {
        std::string_view text = message ? message : "";
        while (!text.empty() && (text.back() == '\n' || text.back() == '\r')) text.remove_suffix(1);
        if (text.empty()) return;
        LOG("[NGX feature " << int(component) << "] " << text);
    };
    commonInfo.LoggingInfo.MinimumLoggingLevel = NVSDK_NGX_LOGGING_LEVEL_ON;

    // Same project identity the SR backend registers, so the driver sees one
    // application asking for both features rather than two. The lease means
    // whichever backend reaches a device second reuses the first one's init
    // instead of initializing NGX twice on it.
    const bool sessionAcquired = ngx_session_detail::ProcessRegistry().Acquire(
        m_sessionKey,
        [&] {
            m_lastResult = NVSDK_NGX_D3D12_Init_with_ProjectID(
                "50f09991-2962-44db-bad7-4be06dbbd1d2",
                NVSDK_NGX_ENGINE_TYPE_CUSTOM,
                "DLSSVideoPlayer-10.0",
                logDir.c_str(), device, &commonInfo, NVSDK_NGX_Version_API);
            return !NVSDK_NGX_FAILED(m_lastResult);
        });
    if (!sessionAcquired) {
        LOG("NGX Init failed for the DLSS-G probe result=" << HexText(m_lastResult));
        m_sessionKey = nullptr;
        m_device = nullptr;
        return false;
    }
    m_sessionLeaseAcquired = true;

    m_lastResult = NVSDK_NGX_D3D12_GetCapabilityParameters(&m_params);
    if (NVSDK_NGX_FAILED(m_lastResult) || !m_params) {
        LOG("NGX GetCapabilityParameters failed for the DLSS-G probe result=" << HexText(m_lastResult));
        m_params = nullptr;
        return false;
    }
    return true;
}

// The create both entry points share, so admission and production are answered
// about the same call: one differing parameter between them would make the
// probe's verdict say nothing about the feature Initialize goes on to hold.
NVSDK_NGX_Result DLSSGBackend::CreateFeature(ID3D12GraphicsCommandList* cmd,
                                             uint32_t width, uint32_t height, DXGI_FORMAT backbufferFormat)
{
    // The DLFG-specific geometry keys, which the runtime prefers over the
    // generic Width/Height pair whenever they are set
    // (nvsdk_ngx_defs_dlssg.h), are populated here as well as in the create
    // params below, so the create does not depend on which of the two pairs
    // NGX_D3D12_CREATE_DLSSG happens to write.
    NVSDK_NGX_Parameter_SetUI(m_params, NVSDK_NGX_DLSSG_Parameter_Width, width);
    NVSDK_NGX_Parameter_SetUI(m_params, NVSDK_NGX_DLSSG_Parameter_Height, height);

    NVSDK_NGX_DLSSG_Create_Params createParams{};
    createParams.Width = width;
    createParams.Height = height;
    createParams.NativeBackbufferFormat = static_cast<unsigned int>(backbufferFormat);
    // A player that owns its swapchain presents at one geometry, so the
    // internal (render) size is the backbuffer size and never moves: dynamic
    // resolution is off and the runtime is not asked to admit a range.
    createParams.RenderWidth = width;
    createParams.RenderHeight = height;
    createParams.DynamicResolutionScaling = false;

    // NGX_D3D12_CREATE_DLSSG sets the node masks and the DLSSG keys and then
    // calls the raw NVSDK_NGX_D3D12_CreateFeature with
    // NVSDK_NGX_Feature_FrameGeneration, which is the entry point in question.
    const NVSDK_NGX_Result result = NGX_D3D12_CREATE_DLSSG(cmd, 1, 1, &m_handle, m_params, &createParams);
    if (NVSDK_NGX_FAILED(result)) m_handle = nullptr;
    return result;
}

DLSSGCapability DLSSGBackend::Probe(ID3D12Device* device, ID3D12GraphicsCommandList* cmd,
                                    uint32_t width, uint32_t height, DXGI_FORMAT backbufferFormat)
{
    DLSSGCapability capability;
    m_available = false;

    const std::optional<uint32_t> hwSchMode = HardwareScheduledGpuMode();
    capability.hagsEnabled = hwSchMode == 2u;

    if (!device || !cmd || !width || !height) {
        m_lastResult = NVSDK_NGX_Result_FAIL_InvalidParameter;
        capability.createResult = m_lastResult;
        capability.detail = L"The probe needs a device, a command list open for recording and a non-empty "
                            L"backbuffer geometry.";
        return capability;
    }
    if (!AcquireSession(device)) {
        capability.createResult = m_lastResult;
        capability.detail = L"NGX D3D12 initialization refused: " + HexResultTextWide(uint32_t(m_lastResult));
        return capability;
    }

    int advertised = 0;
    const NVSDK_NGX_Result advertisedRead =
        NVSDK_NGX_Parameter_GetI(m_params, NVSDK_NGX_Parameter_FrameGeneration_Available, &advertised);
    int featureInitResult = 0;
    NVSDK_NGX_Parameter_GetI(m_params, NVSDK_NGX_Parameter_FrameGeneration_FeatureInitResult, &featureInitResult);
    int needsUpdatedDriver = 0;
    NVSDK_NGX_Parameter_GetI(m_params, NVSDK_NGX_Parameter_FrameGeneration_NeedsUpdatedDriver, &needsUpdatedDriver);
    int minDriverMajor = 0, minDriverMinor = 0;
    NVSDK_NGX_Parameter_GetI(m_params, NVSDK_NGX_Parameter_FrameGeneration_MinDriverVersionMajor, &minDriverMajor);
    NVSDK_NGX_Parameter_GetI(m_params, NVSDK_NGX_Parameter_FrameGeneration_MinDriverVersionMinor, &minDriverMinor);

    // DLSSG.MultiFrameCountMax is the runtime's own count and the only key in
    // the vendored headers that carries it: nvsdk_ngx_defs_dlssg.h publishes it
    // as an output capability read off the capability parameter block, where 1
    // or absent means no multiframe. A runtime that does not publish it has
    // still admitted one generated frame per real pair if the create below
    // succeeds, which is the 2x case, so that success fills in 1 and no larger
    // number is ever assumed.
    unsigned int multiFrameCountMax = 0;
    if (!NVSDK_NGX_FAILED(NVSDK_NGX_Parameter_GetUI(m_params, NVSDK_NGX_DLSSG_Parameter_MultiFrameCountMax,
                                                    &multiFrameCountMax))) {
        capability.multiFrameCountMax = multiFrameCountMax;
    }

    LOG("NGX DLSS-G capability keys: FrameGeneration.Available=" << std::dec << advertised
        << " (read=" << HexText(advertisedRead) << ") FeatureInitResult="
        << HexText(uint32_t(featureInitResult))
        << " NeedsUpdatedDriver=" << needsUpdatedDriver
        << " MinDriverVersion=" << minDriverMajor << "." << minDriverMinor
        << " MultiFrameCountMax=" << capability.multiFrameCountMax
        << " HwSchMode=" << (hwSchMode ? std::to_string(*hwSchMode) : std::string("absent")));

    // A probe whose feature is still held has create work that may not have
    // retired, and a second create would lose the handle that has to wait
    // for it.
    if (m_probeHandle) {
        m_lastResult = NVSDK_NGX_Result_FAIL_InvalidParameter;
        capability.createResult = m_lastResult;
        capability.detail = L"The previous probe's feature has not been released yet.";
        return capability;
    }
    // CreateFeature writes m_handle; the probe's feature is kept apart from
    // the one Initialize holds, so Evaluate can never run on it.
    NVSDK_NGX_Handle* const kept = m_handle;
    m_handle = nullptr;
    m_lastResult = CreateFeature(cmd, width, height, backbufferFormat);
    m_probeHandle = NVSDK_NGX_FAILED(m_lastResult) ? nullptr : m_handle;
    m_handle = kept;
    capability.createResult = m_lastResult;
    if (NVSDK_NGX_FAILED(m_lastResult) || !m_probeHandle) {
        m_probeHandle = nullptr;
        Append(capability.detail,
               L"CreateFeature(FrameGeneration) refused: " + HexResultTextWide(uint32_t(m_lastResult)));
        if (NVSDK_NGX_FAILED(advertisedRead)) {
            Append(capability.detail, L"the runtime published no FrameGeneration.Available key");
        } else if (!advertised) {
            Append(capability.detail, L"FrameGeneration.Available=0, FeatureInitResult=" +
                                          HexResultTextWide(uint32_t(featureInitResult)));
        }
        if (needsUpdatedDriver) {
            Append(capability.detail, L"FrameGeneration.NeedsUpdatedDriver=1, the runtime asks for driver " +
                                          std::to_wstring(minDriverMajor) + L"." +
                                          std::to_wstring(minDriverMinor) + L" or newer");
        }
        // Measured here on 2026-09-17 (RTX 5090, driver 616.64): this exact
        // pair - 0xbad0000b with FrameGeneration.Available=0 and
        // FeatureInitResult=0xbad00004 - is what the runtime answers when
        // nvngx_dlssg.dll is not resolvable beside the executable, and staging
        // the vendored one beside it turns the same create into Success. The
        // driver-store copy is found and logged as a fallback but was measured
        // not to serve this path, so the snippet's absence is named here
        // rather than left for a caller to guess at.
        if (!GetModuleHandleW(L"nvngx_dlssg.dll")) {
            Append(capability.detail,
                   L"no nvngx_dlssg.dll is loaded in this process; the DLSS-G snippet has to be resolvable beside "
                   L"the executable, the driver-store copy alone was measured not to serve this path");
        }
        if (!capability.hagsEnabled) Append(capability.detail, HardwareSchedulingSentence(hwSchMode));
        LOG("RAW NGX D3D12 CreateFeature(FrameGeneration) failed result=" << HexText(m_lastResult)
            << " at " << width << "x" << height << " format=" << int(backbufferFormat));
        return capability;
    }

    // Not released here. The create was recorded on `cmd`, which the caller
    // has yet to submit; releasing the feature first - as this used to - freed
    // it before the GPU ran the work that initializes it. The caller submits,
    // waits and calls ReleaseProbedFeature (or Shutdown).
    if (!capability.multiFrameCountMax) capability.multiFrameCountMax = 1;
    if (!capability.hagsEnabled) {
        Append(capability.detail, HardwareSchedulingSentence(hwSchMode) +
                                      L"; the runtime admitted the feature anyway");
    }
    capability.available = true;
    m_available = true;
    LOG("RAW NGX D3D12 CreateFeature(FrameGeneration) SUCCESS at " << std::dec << width << "x" << height
        << " format=" << int(backbufferFormat) << " multiFrameCountMax=" << capability.multiFrameCountMax
        << "; held until the create work on the caller's command list retires");
    return capability;
}

NVSDK_NGX_Result DLSSGBackend::ReleaseProbedFeature()
{
    if (!m_probeHandle) return NVSDK_NGX_Result_Success;
    const NVSDK_NGX_Result result = NVSDK_NGX_D3D12_ReleaseFeature(m_probeHandle);
    m_probeHandle = nullptr;
    if (NVSDK_NGX_FAILED(result)) {
        m_lastResult = result;
        if (!m_handle) m_available = false;
        LOG("RAW NGX D3D12 ReleaseFeature(FrameGeneration) of the probe failed result=" << HexText(result));
    }
    return result;
}

bool DLSSGBackend::Initialize(ID3D12Device* device, ID3D12GraphicsCommandList* cmd,
                              uint32_t width, uint32_t height, DXGI_FORMAT backbufferFormat)
{
    if (!device || !cmd || !width || !height) {
        m_lastResult = NVSDK_NGX_Result_FAIL_InvalidParameter;
        return false;
    }

    // A live feature at this geometry already is the feature this call would
    // create, and re-creating it would discard the temporal history it holds -
    // which is the whole reason it is kept - for nothing.
    if (m_handle && m_device == device && m_width == width && m_height == height &&
        m_backbufferFormat == backbufferFormat) {
        return true;
    }
    // Geometry and buffer format are the changes NVIDIA's DLSS Programming
    // Guide 310.6.0 (S3.2 step 6) does require a re-create for. Every command
    // list that referenced the outgoing feature in Evaluate must already have
    // retired (S5.5), which only the caller can know, so this release trusts
    // the caller the same way the SR backend's recreate path does.
    if (m_handle) {
        NVSDK_NGX_D3D12_ReleaseFeature(m_handle);
        m_handle = nullptr;
    }
    m_available = false;
    m_evaluations = 0;

    if (!AcquireSession(device)) return false;

    // The runtime's own ceiling on generated frames per real pair, read the way
    // Probe reads it. An absent key means no multiframe, which is still one
    // generated frame per pair once the create below succeeds, so it floors at
    // 1 and Evaluate refuses anything above whatever this says.
    unsigned int multiFrameCountMax = 0;
    m_multiFrameCountMax =
        NVSDK_NGX_FAILED(NVSDK_NGX_Parameter_GetUI(m_params, NVSDK_NGX_DLSSG_Parameter_MultiFrameCountMax,
                                                   &multiFrameCountMax)) || multiFrameCountMax == 0
            ? 1u
            : multiFrameCountMax;

    m_lastResult = CreateFeature(cmd, width, height, backbufferFormat);
    if (NVSDK_NGX_FAILED(m_lastResult) || !m_handle) {
        m_handle = nullptr;
        LOG("RAW NGX D3D12 CreateFeature(FrameGeneration) for evaluation failed result=" << HexText(m_lastResult)
            << " at " << width << "x" << height
            << " format=" << int(backbufferFormat)
            << "; DLSSGBackend::Probe reports why a create is refused");
        return false;
    }

    m_width = width;
    m_height = height;
    m_backbufferFormat = backbufferFormat;
    m_available = true;
    LOG("RAW NGX D3D12 CreateFeature(FrameGeneration) KEPT at " << std::dec << width << "x" << height
        << " format=" << int(backbufferFormat) << " multiFrameCountMax=" << m_multiFrameCountMax
        << "; the create work is on the caller's command list and has to retire before the first evaluate");
    return true;
}

// Everything in the optional evaluate block that a video source can honestly
// state. Each value is a claim about the input, and a claim the source cannot
// support - a projection it has no camera for, a jitter phase nothing applied -
// would be answered by the runtime resolving the frame against geometry that
// never existed.
NVSDK_NGX_DLSSG_Opt_Eval_Params DLSSGBackend::VideoEvalConstants(uint32_t multiFrameCount,
                                                                 uint32_t multiFrameIndex, bool reset) const
{
    NVSDK_NGX_DLSSG_Opt_Eval_Params constants{};
    constants.multiFrameCount = multiFrameCount;
    constants.multiFrameIndex = multiFrameIndex;
    constants.reset = reset;
    // Reset is the caller's statement, never the runtime's: automode is not in
    // play on the raw path, so nothing here overrode the flag.
    constants.automodeOverrideReset = false;

    // A decoded frame has no camera, so there is no view, projection or lens
    // transform to declare and every matrix is the identity. clipToPrevClip and
    // prevClipToClip identity says the camera did not move between the two
    // frames, which is exactly true of a video source: all apparent motion
    // belongs to the content and is already in the MV buffer, which is what
    // cameraMotionIncluded below declares. A made-up projection instead would
    // have the runtime unproject the flat depth proxy into a scene that does
    // not exist, and clipToLensClip identity says the colour is undistorted -
    // the same statement the null distortion field in Evaluate makes.
    const auto setIdentity = [](float matrix[4][4]) {
        for (int row = 0; row < 4; ++row) {
            for (int column = 0; column < 4; ++column) matrix[row][column] = row == column ? 1.0f : 0.0f;
        }
    };
    setIdentity(constants.cameraViewToClip);
    setIdentity(constants.clipToCameraView);
    setIdentity(constants.clipToLensClip);
    setIdentity(constants.clipToPrevClip);
    setIdentity(constants.prevClipToClip);

    // No temporal AA jitter: decoded frames arrive on a fixed sample grid, so
    // there is no sub-pixel phase to report. This is the position
    // DLSSBackend::FillEvaluateParameters takes for SR, for the same reason.
    constants.jitterOffset[0] = 0.0f;
    constants.jitterOffset[1] = 0.0f;

    // DLSSG.MvecScale{X,Y} exist to normalize the buffer into [-1,1]
    // (nvsdk_ngx_params_dlssg.h), so a buffer written in backbuffer pixels is
    // divided by the backbuffer extent and one already written in normalized
    // screen units passes through at 1.0. The units are the caller's
    // declaration via SetMotionVectorUnits; the scale is derived here so the
    // two can never disagree.
    const bool pixelUnits = m_mvecUnits == MotionVectorUnits::BackbufferPixels;
    constants.mvecScale[0] = pixelUnits && m_width ? 1.0f / float(m_width) : 1.0f;
    constants.mvecScale[1] = pixelUnits && m_height ? 1.0f / float(m_height) : 1.0f;

    // -FLT_MAX is unreachable by any displacement an R16G16_FLOAT buffer can
    // hold, so nothing in the buffer is treated as invalid. The sentinel cannot
    // be 0: a video producer writes zero for "this pixel did not move", which
    // is most of a static shot, and calling that invalid would throw away the
    // one thing the buffer is certain about.
    constants.motionVectorsInvalidValue = -FLT_MAX;
    // Per-pixel vectors straight from the estimator, not spread over silhouette
    // edges by a dilation pass the video path does not run.
    constants.motionVectorsDilated = false;

    constants.cameraMotionIncluded = true;
    // A frame is a flat image on a sample grid, not a perspective view of a
    // scene, so the projection it was produced under is orthographic.
    constants.orthoProjection = true;
    // The depth handed in is a flat proxy written near-to-far as a plain [0,1]
    // value, so it is not reversed-Z. depthInverted also selects the runtime's
    // linearization (nvsdk_ngx_defs_dlssg.h: lin = 1/(1-depth) when false), and
    // false is the branch that stays finite for a proxy written below 1.0.
    constants.depthInverted = false;
    // The backbuffer this path hands over is the player's SDR present surface.
    constants.colorBuffersHDR = false;
    // Every evaluate here is driven by a real decoded frame. There is no paused
    // menu and no cut-scene player to declare, and saying otherwise would ask
    // the runtime to stop generating exactly when the video is playing.
    constants.notRenderingGameFrames = false;
    // Fullscreen menu detection looks for a game menu that replaced the scene.
    // This player composites its own UI after this pass, so there is no such
    // menu in the backbuffer and the heuristic has only false positives to find
    // in ordinary video content.
    constants.menuDetectionEnabled = false;

    // Explicit full-frame extents on every resource this path actually hands
    // over, the way DLSSBackend states its subrects: with EvalFlags left at its
    // default the runtime writes interpolated pixels inside the backbuffer
    // extent and uninterpolated ones outside it, so declaring the extent as the
    // whole frame is what says there is no "outside" here. The extents of the
    // resources Evaluate leaves null stay zero.
    const NVSDK_NGX_Dimensions frame{m_width, m_height};
    constants.backbufferSubrectSize = frame;
    constants.mvecsSubrectSize = frame;
    constants.depthSubrectSize = frame;
    constants.outputInterpSubrectSize = frame;
    return constants;
}

bool DLSSGBackend::Evaluate(ID3D12GraphicsCommandList* cmd,
                            ID3D12Resource* backbuffer,
                            ID3D12Resource* motion,
                            ID3D12Resource* depth,
                            ID3D12Resource* outputInterpolated,
                            uint32_t multiFrameCount,
                            uint32_t multiFrameIndex,
                            bool reset,
                            uint64_t backbufferFrameId)
{
    if (!Available() || !m_handle || !cmd || !backbuffer || !motion || !depth || !outputInterpolated) return false;
    // The index is 1-based and bounded by the count, and the count by what the
    // runtime published. Out of range is a caller bug, refused here rather than
    // handed to the runtime to interpret.
    if (!multiFrameCount || !multiFrameIndex || multiFrameIndex > multiFrameCount ||
        multiFrameCount > m_multiFrameCountMax) {
        m_lastResult = NVSDK_NGX_Result_FAIL_InvalidParameter;
        LOG("DLSS-G evaluate refused: multiFrameIndex=" << std::dec << multiFrameIndex << " of count="
            << multiFrameCount << " is outside 1.." << m_multiFrameCountMax);
        return false;
    }

    NVSDK_NGX_D3D12_DLSSG_Eval_Params evalParams{};
    evalParams.pBackbuffer = backbuffer;
    evalParams.pMVecs = motion;
    evalParams.pDepth = depth;
    evalParams.pOutputInterpFrame = outputInterpolated;
    // Null because a decoded video frame has nothing to put in them, not
    // because they were forgotten: the frame is one flat composited layer, so
    // there is no HUD-less copy of it to hand over, no separate UI colour or
    // alpha plane, and no post-process lens distortion for a bidirectional
    // distortion field to undo. pOutputRealFrame is null because this path
    // presents the decoded frame itself and will not have the runtime draw
    // debug text into it, and pOutputDisableInterpolation is null because the
    // hint it carries only matters to a presentation path that could act on it,
    // which is a later measurement than this one.
    evalParams.pHudless = nullptr;
    evalParams.pUI = nullptr;
    evalParams.pUIAlpha = nullptr;
    evalParams.pBidirectionalDistortionField = nullptr;
    evalParams.pOutputRealFrame = nullptr;
    evalParams.pOutputDisableInterpolation = nullptr;

    // DLSSG.BackbufferFrameID, set before the helper writes the rest of the
    // block so it travels with this evaluate. It is not in
    // NVSDK_NGX_DLSSG_Opt_Eval_Params - the vendored struct has no field for it
    // - so it is written straight onto the parameter block the way the create
    // path writes DLSSG.Width/Height. Left unset at 0: a runtime that reads the
    // key is better off seeing it absent than seeing a counter that restarted.
    if (backbufferFrameId != 0) {
        NVSDK_NGX_Parameter_SetULL(m_params, NVSDK_NGX_DLSSG_Parameter_BackbufferFrameID,
                                   backbufferFrameId);
    }

    NVSDK_NGX_DLSSG_Opt_Eval_Params constants = VideoEvalConstants(multiFrameCount, multiFrameIndex, reset);

    // NGX_D3D12_EVALUATE_DLSSG writes every DLSSG.* key from the two structs -
    // including the optional resources above, which it sets to null explicitly
    // rather than leaving whatever the last evaluate put there - and finishes
    // with the raw NVSDK_NGX_D3D12_EvaluateFeature_C, which is the same entry
    // point the SR path uses.
    m_lastResult = NGX_D3D12_EVALUATE_DLSSG(cmd, m_handle, m_params, &evalParams, &constants);
    if (NVSDK_NGX_FAILED(m_lastResult)) {
        LOG("RAW NGX D3D12 EvaluateFeature(FrameGeneration) failed result=" << HexText(m_lastResult)
            << " frame " << multiFrameIndex << "/" << multiFrameCount
            << " reset=" << (reset ? 1 : 0));
        return false;
    }

    ++m_evaluations;
    if (m_evaluations <= 2 || (m_evaluations % 300) == 0) {
        LOG("RAW NGX D3D12 EvaluateFeature(FrameGeneration) SUCCESS #" << std::dec << m_evaluations
            << " at " << m_width << "x" << m_height << " frame " << multiFrameIndex << "/" << multiFrameCount
            << " mvecScale=(" << constants.mvecScale[0] << "," << constants.mvecScale[1] << ")"
            << " reset=" << (reset ? 1 : 0));
    }
    return true;
}

void DLSSGBackend::Abandon()
{
    if (m_handle || m_probeHandle || m_params || m_sessionLeaseAcquired) {
        LOG("DLSS-G feature abandoned without release: the GPU never retired the work it was recorded "
            "into, so the feature, its parameters and the NGX session are deliberately leaked.");
    }
    m_handle = nullptr;
    m_probeHandle = nullptr;
    m_params = nullptr;
    m_sessionLeaseAcquired = false;
    m_sessionKey = nullptr;
    Shutdown();   // with nothing left to release, this only resets the fields
}

void DLSSGBackend::Shutdown()
{
    ReleaseProbedFeature();
    if (m_handle) {
        NVSDK_NGX_D3D12_ReleaseFeature(m_handle);
        m_handle = nullptr;
    }
    if (m_params) {
        NVSDK_NGX_D3D12_DestroyParameters(m_params);
        m_params = nullptr;
    }
    if (m_sessionLeaseAcquired) {
        ngx_session_detail::ProcessRegistry().Release(
            m_sessionKey,
            [&] { NVSDK_NGX_D3D12_Shutdown1(m_device); });
        m_sessionLeaseAcquired = false;
    }
    m_sessionKey = nullptr;
    m_device = nullptr;
    m_width = 0;
    m_height = 0;
    m_backbufferFormat = DXGI_FORMAT_UNKNOWN;
    m_multiFrameCountMax = 0;
    m_evaluations = 0;
    m_available = false;
}


```
