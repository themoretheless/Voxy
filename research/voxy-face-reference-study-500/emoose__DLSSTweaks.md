# Facial rendering source scan: emoose/DLSSTweaks

Repository: https://github.com/emoose/DLSSTweaks

Pinned source revision: 1d2fddbe3d1e8f403f795afc17e7db239a83f6a2

Discovery topic: DLSS

Description: Tweak DLL for NVIDIA DLSS, force DLAA on DLSS-supported titles, tweak scaling ratios & DLSS 3.1 presets, override DLSS versions without overwriting game files.

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE.md", "external/DLSS/LICENSE.dxvk-remix.txt", "external/DLSS/LICENSE.txt", "external/DLSS/license.streamline.txt"]

Topic paths: {"dlss": ["ConfigTool/DLSSTweaks.ConfigTool.csproj", "ConfigTool/DLSSTweaks.ConfigTool.sln", "dlsstweaks.ini", "external/DLSS/LICENSE.dxvk-remix.txt", "external/DLSS/LICENSE.txt", "external/DLSS/include/nvsdk_ngx.h", "external/DLSS/include/nvsdk_ngx_defs.h", "external/DLSS/include/nvsdk_ngx_defs_deepdvc.h", "external/DLSS/include/nvsdk_ngx_defs_dlssd.h", "external/DLSS/include/nvsdk_ngx_dlssg.h", "external/DLSS/include/nvsdk_ngx_dlssg_vk.h", "external/DLSS/include/nvsdk_ngx_helpers.h", "external/DLSS/include/nvsdk_ngx_helpers_deepdvc.h", "external/DLSS/include/nvsdk_ngx_helpers_deepdvc_vk.h", "external/DLSS/include/nvsdk_ngx_helpers_dlssd.h", "external/DLSS/include/nvsdk_ngx_helpers_dlssd_vk.h", "external/DLSS/include/nvsdk_ngx_helpers_vk.h", "external/DLSS/include/nvsdk_ngx_params.h", "external/DLSS/include/nvsdk_ngx_params_dlssd.h", "external/DLSS/include/nvsdk_ngx_vk.h", "external/DLSS/license.streamline.txt", "src/DLSSTweaks.hpp", "src/module_hooks/nvngx_dlss.cpp", "src/module_hooks/nvngx_dlssg.cpp"]}

## Source file: src/DLSSTweaks.hpp

{
  "path": "src/DLSSTweaks.hpp",
  "blob_sha": "b6832d4141eb2fbc79e52dfccd7b6154f7322903",
  "sha256": "7c4ee12b8653649a4385f9c42b3cda7f8e2ac70b541d25bab603120091c326f9",
  "lines": 175,
  "bytes": 5402,
  "selection_score": 15,
  "keyword_line_counts": {
    "sss": 2,
    "dlss": 18,
    "rig": 4
  },
  "url": "https://github.com/emoose/DLSSTweaks/blob/1d2fddbe3d1e8f403f795afc17e7db239a83f6a2/src/DLSSTweaks.hpp",
  "function_signature_candidates": [
    {
      "line": 28,
      "text": "void zero_customized_values();"
    },
    {
      "line": 101,
      "text": "bool read(const std::filesystem::path& iniPath, int numInisRead = 0);"
    },
    {
      "line": 102,
      "text": "void print_to_log();"
    },
    {
      "line": 103,
      "text": "void watch_for_changes(const std::filesystem::path& iniPath);"
    },
    {
      "line": 109,
      "text": "void WaitForInitThread();"
    },
    {
      "line": 114,
      "text": "void hook_params(NVSDK_NGX_Parameter* params);"
    },
    {
      "line": 115,
      "text": "void init_from_proxy();"
    },
    {
      "line": 116,
      "text": "void init(HMODULE ngx_module);"
    },
    {
      "line": 122,
      "text": "void init(HMODULE ngx_module);"
    },
    {
      "line": 126,
      "text": "void settings_changed();"
    },
    {
      "line": 127,
      "text": "void init(HMODULE ngx_module);"
    },
    {
      "line": 131,
      "text": "void init(HMODULE ngx_module);"
    },
    {
      "line": 144,
      "text": "void reset()"
    }
  ]
}

```text

#pragma once
#include <SafetyHook.hpp>
#include "Utility.hpp"
#include <filesystem>
#include <unordered_map>
#include <nvsdk_ngx_defs.h>
#include <nvsdk_ngx_params.h>

// Certain settings which aren't currently included in DLSS SDK, but do seem checked by DLSS 3.1+ DLL files.
#ifndef NVSDK_NGX_Parameter_Disable_Watermark
#define NVSDK_NGX_Parameter_Disable_Watermark "Disable.Watermark"
#endif
#ifndef NVSDK_NGX_Parameter_DLSS_Get_Dynamic
#define NVSDK_NGX_Parameter_DLSS_Get_Dynamic "DLSS.Get.Dynamic."
#endif

constexpr float DLSS_MinScale = 0.0f;
constexpr float DLSS_MaxScale = 1.0f;

struct DlssNvidiaPresetOverrides
{
	uint32_t overrideDLAA;
	uint32_t overrideQuality;
	uint32_t overrideBalanced;
	uint32_t overridePerformance;
	uint32_t overrideUltraPerformance;
	
	void zero_customized_values();
};

struct QualityLevel
{
	std::string name;
	float scalingRatio;
	std::pair<int, int> resolution = { 0,0 };

	// The last resolution we told game about for the this level, so we can check against it later on
	// (based on either `scalingRatio` or `resolution` set by the user)
	std::pair<int, int> currentResolution = { 0,0 };

	unsigned int preset = NVSDK_NGX_DLSS_Hint_Render_Preset_Default;

	std::string lastUserValue = "";
};

struct DlssSettings
{
	NVSDK_NGX_PerfQuality_Value prevQualityLevel; // the last quality level setting that game requested
	std::optional<ID3D12Resource*> prevExposureTexture;

	int featureCreateFlags = 0;
	std::optional<DlssNvidiaPresetOverrides> nvidiaOverrides;
	unsigned long long appId = 0;
	std::string projectId;

	// app ID displayed by DLSS overlay is xored for some reason
	unsigned long long appIdDlss() const
	{
		return appId ^ 0xE658703;
	}
};

struct UserSettings
{
	bool disableAllTweaks = false; // not exposed in INI, is set if a serious error is detected (eg. two versions loaded at once)

	std::unordered_map<NVSDK_NGX_PerfQuality_Value, QualityLevel> qualities =
	{
		{NVSDK_NGX_PerfQuality_Value_UltraPerformance, {"UltraPerformance", 0.33333334f}},
		{NVSDK_NGX_PerfQuality_Value_MaxPerf, {"Performance", 0.5f}},
		{NVSDK_NGX_PerfQuality_Value_Balanced, {"Balanced", 0.58f}},
		{NVSDK_NGX_PerfQuality_Value_MaxQuality, {"Quality", 0.66666667f}},
		{NVSDK_NGX_PerfQuality_Value_DLAA, {"DLAA", 1.0f}},

		// note: if NVSDK_NGX_PerfQuality_Value_UltraQuality is non-zero, some games may detect that we're passing a valid resolution and show an Ultra Quality option as a result
		// very few games support this though, and right now DLSS seems to refuse to render if UltraQuality gets passed to it
		// our SetI hook in HooksNvngx can override the quality passed to DLSS if this gets used by the game, letting it think this is MaxQuality instead
		// but we'll only do that if user has overridden this in the INI to a non-zero value
		{NVSDK_NGX_PerfQuality_Value_UltraQuality, {"UltraQuality", 0.f}},
	};

	bool forceDLAA = false;
	int overrideAutoExposure = 0;
	int overrideAlphaUpscaling = 0;
	std::optional<float> overrideSharpening{};
	std::string overrideSharpeningString = "ignore";
	bool overrideSharpeningForceDisable = false;
	bool overrideAppId = false;
	int overrideDlssHud = 0;
	int overrideHDR = 0;
	bool disableDevWatermark = false;
	bool verboseLogging = false;
	std::unordered_map<std::string, std::filesystem::path> dllPathOverrides;
	bool overrideQualityLevels = false;

	int resolutionOffset = 0; // user-defined offset to apply to DLAA / full-res rendering (some titles don't like DLAA rendering at full res, so small offset is needed)
	bool dynamicResolutionOverride = true;
	int dynamicResolutionMinOffset = -1;
	bool disableIniMonitoring = false;

	bool read(const std::filesystem::path& iniPath, int numInisRead = 0);
	void print_to_log();
	void watch_for_changes(const std::filesystem::path& iniPath);
};

// DllMain.cpp / UserSettings.cpp
extern UserSettings settings;
extern DlssSettings dlss;
void WaitForInitThread();

// HooksNvngx.cpp
namespace nvngx
{
void hook_params(NVSDK_NGX_Parameter* params);
void init_from_proxy();
void init(HMODULE ngx_module);
};

// module_hooks/*
namespace nvngx_dlss
{
void init(HMODULE ngx_module);
};
namespace nvngx_dlssg
{
void settings_changed();
void init(HMODULE ngx_module);
};
namespace nvngx_dlssd
{
void init(HMODULE ngx_module);
};

// wrapper struct on top of SafetyHookInline
// can either call the hook trampoline via SafetyHookInline, or call a specified function if dest_proc is set
struct HookOrigFn
{
	SafetyHookInline hook{};
	FARPROC dest_proc = nullptr;

	std::recursive_mutex m_mutex{};

	// only resets inline hook, as resetting both hook & dest_proc would leave this with no function to call, causing issues
	void reset()
	{
		hook.reset();
	}

	HookOrigFn& operator=(SafetyHookInline other) noexcept
	{
		reset();
		dest_proc = nullptr;
		std::swap(hook, other);
		return *this;
	}

	HookOrigFn& operator=(FARPROC other) noexcept
	{
		reset();
		std::swap(dest_proc, other);
		return *this;
	}

	template <typename RetT = void, typename... Args> auto call(Args... args) {
		std::scoped_lock lock{m_mutex};

		uintptr_t dest = dest_proc ? uintptr_t(dest_proc) : hook.trampoline().address();
		return ((RetT(*)(Args...))dest)(args...);
	}

	template <typename RetT = void, typename... Args> auto unsafe_call(Args... args) {
		uintptr_t dest = dest_proc ? uintptr_t(dest_proc) : hook.trampoline().address();
		return ((RetT(*)(Args...))dest)(args...);
	}
};


```

## Source file: src/module_hooks/nvngx_dlss.cpp

{
  "path": "src/module_hooks/nvngx_dlss.cpp",
  "blob_sha": "3f83c6f56758cc86b0127876a07789646d7dd598",
  "sha256": "c878d51b18a005fbe2267196feb255cd37b2075b334c1e7d2b87fb2a0333fbf2",
  "lines": 521,
  "bytes": 21925,
  "selection_score": 15,
  "keyword_line_counts": {
    "sss": 9,
    "dlss": 146,
    "rig": 17
  },
  "url": "https://github.com/emoose/DLSSTweaks/blob/1d2fddbe3d1e8f403f795afc17e7db239a83f6a2/src/module_hooks/nvngx_dlss.cpp",
  "function_signature_candidates": [
    {
      "line": 78,
      "text": "void DlssPresetOverrideFunc_LaterVersion(SafetyHookContext& ctx)"
    },
    {
      "line": 91,
      "text": "void DlssPresetOverrideFunc_Version3_1_2(SafetyHookContext& ctx)"
    },
    {
      "line": 100,
      "text": "void DlssPresetOverrideFunc_Version3_1_1(SafetyHookContext& ctx)"
    },
    {
      "line": 123,
      "text": "void CreateDlssInstance_PresetSelection(SafetyHookContext& ctx)"
    },
    {
      "line": 214,
      "text": "bool hook(HMODULE ngx_module)"
    },
    {
      "line": 377,
      "text": "void unhook(HMODULE ngx_module)"
    },
    {
      "line": 399,
      "text": "void init(HMODULE ngx_module)"
    },
    {
      "line": 433,
      "text": "bool hook(HMODULE ngx_module)"
    },
    {
      "line": 491,
      "text": "void unhook(HMODULE ngx_module)"
    },
    {
      "line": 513,
      "text": "void init(HMODULE ngx_module)"
    }
  ]
}

```text

#define WIN32_LEAN_AND_MEAN
#include <ntstatus.h>
#define WIN32_NO_STATUS
#include <Windows.h>
#include <winternl.h>

#include <spdlog/spdlog.h>
#include <Patterns.h>

#include "DLSSTweaks.hpp"

namespace nvngx_dlss
{
	// Hooks shared between both dlss & dlssd
	namespace shared
	{
		// Allow force enabling/disabling the DLSS debug display via RegQueryValueExW hook
		// (fallback method if we couldn't find the indicator value check pattern in the DLL)
		inline LSTATUS RegQueryValueExW_Hook(LSTATUS origRetValue, HKEY hKey, LPCWSTR lpValueName, LPDWORD lpReserved, LPDWORD lpType, LPBYTE lpData, LPDWORD lpcbData)
		{
			const LSTATUS ret = origRetValue;
			if (settings.overrideDlssHud == 0 || _wcsicmp(lpValueName, L"ShowDlssIndicator") != 0)
				return ret;

			if (lpcbData && *lpcbData >= 4 && lpData)
			{
				DWORD* outData = (DWORD*)lpData;
				if (settings.overrideDlssHud >= 1)
					*outData = 0x400;
				else if (settings.overrideDlssHud < 0)
					*outData = 0;
				return ERROR_SUCCESS;
			}

			return ret;
		}

		inline uint32_t DLSS_GetIndicatorValue_Hook(uint32_t origRetValue, void* thisptr, uint32_t* OutValue)
		{
			const auto ret = origRetValue;
			if (settings.overrideDlssHud == 0)
				return ret;
			*OutValue = settings.overrideDlssHud > 0 ? 0x400 : 0;
			return ret;
		}
	};

LSTATUS(__stdcall* RegQueryValueExW_Orig)(HKEY hKey, LPCWSTR lpValueName, LPDWORD lpReserved, LPDWORD lpType, LPBYTE lpData, LPDWORD lpcbData);
LSTATUS __stdcall RegQueryValueExW_Hook(HKEY hKey, LPCWSTR lpValueName, LPDWORD lpReserved, LPDWORD lpType, LPBYTE lpData, LPDWORD lpcbData)
{
	LSTATUS ret = RegQueryValueExW_Orig(hKey, lpValueName, lpReserved, lpType, lpData, lpcbData);
	return nvngx_dlss::shared::RegQueryValueExW_Hook(ret, hKey, lpValueName, lpReserved, lpType, lpData, lpcbData);
}

SafetyHookInline DLSS_GetIndicatorValue;
uint32_t __fastcall DLSS_GetIndicatorValue_Hook(void* thisptr, uint32_t* OutValue)
{
	const auto ret = DLSS_GetIndicatorValue.thiscall<uint32_t>(thisptr, OutValue);
	return nvngx_dlss::shared::DLSS_GetIndicatorValue_Hook(ret, thisptr, OutValue);
}

// Hooks that allow us to override the NV-provided DLSS3.1 presets, without needing us to override the whole AppID for the game
// I'd imagine leaving games AppID might help DLSS keep using certain game-specific things, and also probably let any DLSS telemetry be more accurate too
// @NVIDIA, please consider adding a parameter to tell DLSS to ignore the NV / DRS provided ones instead
// (or a parameter which lets it always use the game provided NVSDK_NGX_Parameter_DLSS_Hint_Render_Preset_XXX param)
SafetyHookInline DlssPresetSetupFunc_3_1_30_Hook;
uint32_t __fastcall DlssPresetSetupFunc_3_1_30(void* a1, void* a2, void* a3, DlssNvidiaPresetOverrides* nvidiaPresets)
{
	nvidiaPresets->zero_customized_values();

	return DlssPresetSetupFunc_3_1_30_Hook.thiscall<uint32_t>(a1, a2, a3, nvidiaPresets);
}

// Don't really like needing to inline hook NV's code for this, but it's necessary atm...
SafetyHookMid DlssPresetOverrideFunc_LaterVersion_Hook;
int8_t DlssPresetOverrideFunc_MovOffset1 = -0x38;
int8_t DlssPresetOverrideFunc_MovOffset2 = -0x3C;
void DlssPresetOverrideFunc_LaterVersion(SafetyHookContext& ctx)
{
	// Code that our hook overwrote
	*(uint64_t*)(ctx.rbp + DlssPresetOverrideFunc_MovOffset1) = ctx.rdi;
	*(uint32_t*)(ctx.rbp + DlssPresetOverrideFunc_MovOffset2) = uint32_t(ctx.rcx);

	auto* nv_settings = (DlssNvidiaPresetOverrides*)ctx.rax;
	nv_settings->zero_customized_values();
}

// Seems earlier DLSS 3.1 builds (3.1.2 and older?) don't use the func above, but some kind of inlined version
// So instead we'll hook the code that would call into it instead
SafetyHookMid DlssPresetOverrideFunc_EarlyVersion_Hook;
void DlssPresetOverrideFunc_Version3_1_2(SafetyHookContext& ctx)
{
	// Code that our hook overwrote
	*(uint32_t*)(ctx.rbx + 0x1C) = uint32_t(ctx.rcx);
	ctx.rax = *(uint64_t*)ctx.rdi;

	auto* nv_settings = (DlssNvidiaPresetOverrides*)ctx.rax;
	nv_settings->zero_customized_values();
}
void DlssPresetOverrideFunc_Version3_1_1(SafetyHookContext& ctx)
{
	// Code that our hook overwrote
	*(uint32_t*)(ctx.rdi + 0x1C) = uint32_t(ctx.rcx);
	//ctx.rax = *(uint64_t*)ctx.rsi;

	auto* nv_settings = (DlssNvidiaPresetOverrides*)ctx.rax;
	nv_settings->zero_customized_values();
}

// Hook that overrides the method DLSS uses to find the preset from the rendering resolution
// Seems DLSS just checks what the ratio is against display res, then maps certain ratio ranges to specific quality level
// (eg. if you had UltraPerf set to 0.666, DLSS is hardcoded to treat 0.666 as Quality, so would use the preset assigned to Quality instead)
// Instead we'll hook that code so we can check against the users customized ratios, and fetch the correct preset for each level
// 
// NOTE: this currently only works on 3.1.11 dev/release
//   some reason earlier versions duplicated the code that picks (and inits?) the preset, across all 5 blocks that check each ratio range
//   making it a lot harder for us to be able to hook it and change the preset (since we'd also have to copy the code that inits it all too..)
//   for now I'm fine leaving this as a 3.1.11+ only hook
SafetyHookMid CreateDlssInstance_PresetSelection_Hook;
uint8_t CreateDlssInstance_PresetSelection_Register = 0x83; // register used to access DLSS struct in this func, unfortunately changes between dev/release
uint32_t CreateDlssInstance_PresetSelection_OrigInsnOffset = 0x1F8;
int CreateDlssInstance_PresetSelection_Offset = 3;
void CreateDlssInstance_PresetSelection(SafetyHookContext& ctx)
{
	uint8_t* dlssStruct = 0;
	struct
	{
		uint32_t RenderResolution = 0x48;
		uint32_t DisplayResolution = 0x60;
	} offsets;

	if (CreateDlssInstance_PresetSelection_Register == 0x83) // 0x83 = rbx
		dlssStruct = (uint8_t*)ctx.rbx;
	else if (CreateDlssInstance_PresetSelection_Register == 0x86) // 0x86 = rsi
		dlssStruct = (uint8_t*)ctx.rsi;
	else if (CreateDlssInstance_PresetSelection_Register == 0x87) // 0x87 = rdi
	{
		// 3.1.30, offsets of resolutions were changed by 8 bytes, darn...
		dlssStruct = (uint8_t*)ctx.rdi;
		offsets.RenderResolution = 0x50;
		offsets.DisplayResolution = 0x68;
	}

	if (!dlssStruct)
		return;

	if (!settings.overrideQualityLevels)
		return;

	int dlssWidth = *(int*)(dlssStruct + offsets.RenderResolution);
	int dlssHeight = *(int*)(dlssStruct + offsets.RenderResolution + 4);
	int displayWidth = *(int*)(dlssStruct + offsets.DisplayResolution);
	int displayHeight = *(int*)(dlssStruct + offsets.DisplayResolution + 4);

	spdlog::debug("CreateDlssInstance_PresetSelection: DLSS res {}x{}, display {}x{}", dlssWidth, dlssHeight, displayWidth, displayHeight);

	std::optional<unsigned int> presetValue;
	presetValue.reset();

	unsigned int presetDLAA = 0;
	if (settings.qualities.contains(NVSDK_NGX_PerfQuality_Value_DLAA))
		presetDLAA = settings.qualities[NVSDK_NGX_PerfQuality_Value_DLAA].preset;

	for (const auto& [level, quality] : settings.qualities)
	{
		// Check that both height & width are within 1 pixel of each other either way
		bool widthIsClose = abs(quality.currentResolution.first - dlssWidth) <= 1;
		bool heightIsClose = abs(quality.currentResolution.second - dlssHeight) <= 1;
		if (widthIsClose && heightIsClose)
		{
			if (quality.preset != NVSDK_NGX_DLSS_Hint_Render_Preset_Default)
			{
				spdlog::debug("CreateDlssInstance_PresetSelection: using preset {} for quality {} with resolution {}x{}", char('A' + quality.preset - 1), quality.name, quality.currentResolution.first, quality.currentResolution.second);
				presetValue = quality.preset;
			}

			break;
		}
	}

	if (!presetValue.has_value())
	{
		// No match found for DLSS presets, check whether this could be DLAA
		if (presetDLAA)
		{
			bool widthIsClose = abs(displayWidth - dlssWidth) <= 1;
			bool heightIsClose = abs(displayHeight - dlssHeight) <= 1;
			if (widthIsClose && heightIsClose)
			{
				spdlog::debug("CreateDlssInstance_PresetSelection: using preset {} for DLAA with resolution {}x{}", char('A' + presetDLAA - 1), displayWidth, displayHeight);
				presetValue = presetDLAA;
			}
		}

		// If still no value override set, return now so DLSS will use whatever it was going to originally
		if (!presetValue.has_value())
			return;
	}

	if (CreateDlssInstance_PresetSelection_Register == 0x83) // release DLL, preset stored in rdx
		ctx.rdx = *presetValue;
	else if (CreateDlssInstance_PresetSelection_Register == 0x86) // dev DLL, preset stored in r15
		ctx.r15 = *presetValue;
	else if (CreateDlssInstance_PresetSelection_Register == 0x87)
	{
		if (CreateDlssInstance_PresetSelection_Offset == 0x11)
			ctx.rcx = *presetValue; // 3.6.0 / 3.7.0, preset stored in rcx
		else
			ctx.rdx = *presetValue; // 3.1.30, preset stored in rdx
	}
}

SafetyHookMid dlssIndicatorHudHook{};
bool hook(HMODULE ngx_module)
{
	// Search for & hook the function that overrides the DLSS presets with ones set by NV
	// So that users can set custom DLSS presets without needing to override the whole app ID
	// (if OverrideAppId is set there shouldn't be any need for this)
	if (!settings.overrideAppId)
	{
		auto pattern = hook::pattern(ngx_module, "41 0F 45 CE 48 89 7D ? 89 4D ? 48 8D 0D");
		if (pattern.size())
		{
			uint8_t* match = pattern.count(1).get_first<uint8_t>();
			DlssPresetOverrideFunc_MovOffset1 = int8_t(match[7]);
			DlssPresetOverrideFunc_MovOffset2 = int8_t(match[10]);

			DlssPresetOverrideFunc_LaterVersion_Hook = safetyhook::create_mid(match + 4, DlssPresetOverrideFunc_LaterVersion);

			spdlog::info("nvngx_dlss: applied DLSS preset override hook (> v3.1.2)");
			spdlog::debug("nvngx_dlss: DlssPresetOverrideFunc_LaterVersion MovOffset1 = 0x{:X}, MovOffset2 = 0x{:X}", DlssPresetOverrideFunc_MovOffset1, DlssPresetOverrideFunc_MovOffset2);
		}
		else
		{
			// 3.1.30 hook
			pattern = hook::pattern(ngx_module, "49 8B CA 48 8D 15 ? ? ? ? 49 8B F9");
			if (pattern.size())
			{
				uint8_t* func_3_1_30 = pattern.get(0).get<uint8_t>(-0x23);
				uint8_t func_known_start[] = { 0x48, 0x89, 0x5C, 0x24 };
				if (!memcmp(func_3_1_30, func_known_start, 4))
				{
					DlssPresetSetupFunc_3_1_30_Hook = safetyhook::create_inline(func_3_1_30, DlssPresetSetupFunc_3_1_30);
					spdlog::info("nvngx_dlss: applied DLSS preset override hook (> v3.1.30)");
				}
			}
			else
			{
				// Couldn't find the preset override func, seems it might be inlined inside earlier DLLs...
				// Search for & hook the inlined code instead
				// (unfortunately registers changed between 3.1.1 & 3.1.2, and probably the ones between 3.1.2 and 3.1.11 too, ugh)
				pattern = hook::pattern(ngx_module, "89 4D ? 8B 08 89 ? 1C 48 8B ?");
				if (!pattern.size())
					spdlog::warn("nvngx_dlss: failed to apply DLSS preset override hooks, recommend enabling OverrideAppId instead");
				else
				{
					uint8_t* match = pattern.count(1).get_first<uint8_t>();
					uint8_t* midhookAddress = match + 5;
					uint8_t destReg = match[6];
					if (destReg == 0x4B) // rbx, 3.1.2
					{
						DlssPresetOverrideFunc_EarlyVersion_Hook = safetyhook::create_mid(midhookAddress, DlssPresetOverrideFunc_Version3_1_2);
						spdlog::info("nvngx_dlss: applied DLSS preset override hook (v3.1.2)");
					}
					else if (destReg == 0x4F) // rdi, 3.1.1
					{
						DlssPresetOverrideFunc_EarlyVersion_Hook = safetyhook::create_mid(midhookAddress, DlssPresetOverrideFunc_Version3_1_1);
						spdlog::info("nvngx_dlss: applied DLSS preset override hook (v3.1.1)");
					}
					else
					{
						spdlog::warn("nvngx_dlss: failed to find DLSS preset override code, recommend enabling OverrideAppId instead");
					}
				}
			}
		}
	}

	// Hook to override the preset DLSS picks based on ratio, so we can check against users customized ratios/resolutions instead
	bool presetSelectPatternSuccess = false;
	auto presetSelectPattern = hook::pattern(ngx_module, "8B ? ? C7 ? ? ? 00 00 03 00 00 00");
	CreateDlssInstance_PresetSelection_Offset = 3;
	if (!presetSelectPattern.size())
	{
		// 3.6.0 / 3.7.0
		presetSelectPattern = hook::pattern(ngx_module, "8B ? ? F6 47 ? 80");
		CreateDlssInstance_PresetSelection_Offset = 0x11;
	}
	if (presetSelectPattern.size())
	{
		uint8_t* match = presetSelectPattern.count(1).get_first<uint8_t>(CreateDlssInstance_PresetSelection_Offset);
		CreateDlssInstance_PresetSelection_Register = match[1];
		CreateDlssInstance_PresetSelection_OrigInsnOffset = *(uint32_t*)&match[2];

		// TODO: better way of handling CreateDlssInstance_PresetSelection_Register

		if (CreateDlssInstance_PresetSelection_Register != 0x83 &&
			CreateDlssInstance_PresetSelection_Register != 0x86 &&
			CreateDlssInstance_PresetSelection_Register != 0x87)
		{
			spdlog::error("nvngx_dlss: DLSS preset selection hook failed (unknown register 0x{:X})", CreateDlssInstance_PresetSelection_Register);
		}
		else
		{
			CreateDlssInstance_PresetSelection_Hook = safetyhook::create_mid(match, CreateDlssInstance_PresetSelection);

			spdlog::info("nvngx_dlss: applied DLSS resolution-to-preset selection hook");

			presetSelectPatternSuccess = true;
		}

		spdlog::debug("nvngx_dlss: CreateDlssInstance_PresetSelection Register = 0x{:X}, OrigInsnOffset = 0x{:X}",
			CreateDlssInstance_PresetSelection_Register, CreateDlssInstance_PresetSelection_OrigInsnOffset);
	}
	
	if (!presetSelectPatternSuccess)
	{
		spdlog::warn("nvngx_dlss: failed to locate DLSS3.1.11+ preset selection code, customized [DLSSPresets] may act strange if used with customized [DLSSQualityLevels] - recommend using DLSS 3.7!");
	}

	// OverrideDlssHud hooks
	// This pattern finds 2 matches in latest DLSS, but only 1 in older ones
	// The one we're trying to find seems to always be first match
	// TODO: scan for RegQueryValue call and grab the offset from that, use offset instead of wildcards below
	auto indicatorValueCheck = hook::pattern(ngx_module, "8B 81 ? ? ? ? 89 02 33 C0 C3");

	// If INI monitoring is enabled, or OverrideDlssHud is 2, we'll try setting up the realtime vftable hook
	// allowing the HUD overlay to be toggled at runtime
	if (indicatorValueCheck.size() && (!settings.disableIniMonitoring || settings.overrideDlssHud == 2))
	{
		if (!settings.disableIniMonitoring)
			spdlog::debug("nvngx_dlss: applying hud hook via vftable hook...");
		else if (settings.overrideDlssHud == 2)
			spdlog::debug("nvngx_dlss: OverrideDlssHud == 2, applying hud hook via vftable hook...");

		auto indicatorValueCheck_addr = indicatorValueCheck.get(0).get<void>();
		DLSS_GetIndicatorValue = safetyhook::create_inline(indicatorValueCheck_addr, DLSS_GetIndicatorValue_Hook);

		// Unfortunately it's not enough to just hook the function, HUD render code seems to have an optimization where it checks funcptr and inlines code if it matches
		// So we also need to search for the address of the function, find vftable that holds it, and overwrite entry to point at our hook

		// Ugly way of converting our address to a pattern string...
		uint8_t* p = (uint8_t*)&indicatorValueCheck_addr;

		std::stringstream ss;
		ss << std::hex << std::setw(2) << std::setfill('0') << (int)p[0];
		for (int i = 1; i < 8; ++i)
			ss << " " << std::setw(2) << std::setfill('0') << (int)p[i];

		auto pattern = ss.str();

		auto indicatorValueCheckMemberVf = hook::pattern(ngx_module, pattern);
		for (int i = 0; i < indicatorValueCheckMemberVf.size(); i++)
		{
			auto vfAddr = indicatorValueCheckMemberVf.get(i).get<uintptr_t>(0);
			UnprotectMemory unprotect{ (uintptr_t)vfAddr, sizeof(uintptr_t) };
			*vfAddr = (uintptr_t)&DLSS_GetIndicatorValue_Hook;
		}

		spdlog::info("nvngx_dlss: applied debug hud overlay hook via vftable hook");
	}
	else
	{
		// Failed to locate indicator value check inside DLSS
		// Fallback to RegQueryValueExW hook so we can override the ShowDlssIndicator value
		HMODULE advapi = GetModuleHandleA("advapi32.dll");
		RegQueryValueExW_Orig = advapi ? (decltype(RegQueryValueExW_Orig))GetProcAddress(advapi, "RegQueryValueExW") : nullptr;
		if (!RegQueryValueExW_Orig || !utility::HookIAT(ngx_module, "advapi32.dll", RegQueryValueExW_Orig, RegQueryValueExW_Hook))
			spdlog::warn("nvngx_dlss: failed to hook DLSS HUD functions, OverrideDlssHud might not be available.");
		else
			spdlog::info("nvngx_dlss: applied debug hud overlay hook via registry");
	}

	return true;
}

void unhook(HMODULE ngx_module)
{
	dlssIndicatorHudHook.reset();

	spdlog::debug("nvngx_dlss: finished unhook");
}

SafetyHookInline dllmain;
BOOL APIENTRY hooked_dllmain(HMODULE hModule, int ul_reason_for_call, LPVOID lpReserved)
{
	BOOL res = dllmain.stdcall<BOOL>(hModule, ul_reason_for_call, lpReserved);

	if (ul_reason_for_call == DLL_PROCESS_DETACH)
	{
		unhook(hModule);
		dllmain.reset();
	}

	return res;
}

// Installs DllMain hook onto nvngx_dlss
void init(HMODULE ngx_module)
{
	if (settings.disableAllTweaks)
		return;

	hook(ngx_module);
	dllmain = safetyhook::create_inline(utility::ModuleEntryPoint(ngx_module), hooked_dllmain);
}
};

// NOTE: nvngx_dlssd namespace is pretty much a copy of nvngx_dlss atm...
// Ideally we would reuse the same code between them, but since both DLLs are usually loaded at the same time it wouldn't be possible to use the same SafetyHookInline instance between them
// our hook code also has no way to know which DLL we were called from, so wouldn't be able to add separate instances for each DLL...
// only solution I can see requires us to include separate functions for both dlss & dlssd
namespace nvngx_dlssd
{
// Allow force enabling/disabling the DLSS debug display via RegQueryValueExW hook
// (fallback method if we couldn't find the indicator value check pattern in the DLL)
LSTATUS(__stdcall* RegQueryValueExW_Orig)(HKEY hKey, LPCWSTR lpValueName, LPDWORD lpReserved, LPDWORD lpType, LPBYTE lpData, LPDWORD lpcbData);
LSTATUS __stdcall RegQueryValueExW_Hook(HKEY hKey, LPCWSTR lpValueName, LPDWORD lpReserved, LPDWORD lpType, LPBYTE lpData, LPDWORD lpcbData)
{
	const auto ret = RegQueryValueExW_Orig(hKey, lpValueName, lpReserved, lpType, lpData, lpcbData);
	return nvngx_dlss::shared::RegQueryValueExW_Hook(ret, hKey, lpValueName, lpReserved, lpType, lpData, lpcbData);
}

SafetyHookInline DLSS_GetIndicatorValue;
uint32_t __fastcall DLSS_GetIndicatorValue_Hook(void* thisptr, uint32_t* OutValue)
{
	const auto ret = DLSS_GetIndicatorValue.thiscall<uint32_t>(thisptr, OutValue);
	return nvngx_dlss::shared::DLSS_GetIndicatorValue_Hook(ret, thisptr, OutValue);
}

SafetyHookMid dlssIndicatorHudHook{};
// NOTE: copy any changes to the nvngx_dlss::hook above!
bool hook(HMODULE ngx_module)
{
	// OverrideDlssHud hooks
	// This pattern finds 2 matches in latest DLSS, but only 1 in older ones
	// The one we're trying to find seems to always be first match
	// TODO: scan for RegQueryValue call and grab the offset from that, use offset instead of wildcards below
	auto indicatorValueCheck = hook::pattern(ngx_module, "8B 81 ? ? ? ? 89 02 33 C0 C3");

	// If INI monitoring is enabled, or OverrideDlssHud is 2, we'll try setting up the realtime vftable hook
	// allowing the HUD overlay to be toggled at runtime
	if (indicatorValueCheck.size() && (!settings.disableIniMonitoring || settings.overrideDlssHud == 2))
	{
		if (!settings.disableIniMonitoring)
			spdlog::debug("nvngx_dlssd: applying hud hook via vftable hook...");
		else if (settings.overrideDlssHud == 2)
			spdlog::debug("nvngx_dlssd: OverrideDlssHud == 2, applying hud hook via vftable hook...");

		auto indicatorValueCheck_addr = indicatorValueCheck.get(0).get<void>();
		DLSS_GetIndicatorValue = safetyhook::create_inline(indicatorValueCheck_addr, DLSS_GetIndicatorValue_Hook);

		// Unfortunately it's not enough to just hook the function, HUD render code seems to have an optimization where it checks funcptr and inlines code if it matches
		// So we also need to search for the address of the function, find vftable that holds it, and overwrite entry to point at our hook

		// Ugly way of converting our address to a pattern string...
		auto* p = (uint8_t*)&indicatorValueCheck_addr;

		std::stringstream ss;
		ss << std::hex << std::setw(2) << std::setfill('0') << (int)p[0];
		for (int i = 1; i < 8; ++i)
			ss << " " << std::setw(2) << std::setfill('0') << (int)p[i];

		auto pattern = ss.str();

		auto indicatorValueCheckMemberVf = hook::pattern(ngx_module, pattern);
		for (int i = 0; i < indicatorValueCheckMemberVf.size(); i++)
		{
			auto vfAddr = indicatorValueCheckMemberVf.get(i).get<uintptr_t>(0);
			UnprotectMemory unprotect{ (uintptr_t)vfAddr, sizeof(uintptr_t) };
			*vfAddr = (uintptr_t)&DLSS_GetIndicatorValue_Hook;
		}

		spdlog::info("nvngx_dlssd: applied debug hud overlay hook via vftable hook");
	}
	else
	{
		// Failed to locate indicator value check inside DLSS
		// Fallback to RegQueryValueExW hook so we can override the ShowDlssIndicator value
		HMODULE advapi = GetModuleHandleA("advapi32.dll");
		RegQueryValueExW_Orig = advapi ? (decltype(RegQueryValueExW_Orig))GetProcAddress(advapi, "RegQueryValueExW") : nullptr;
		if (!RegQueryValueExW_Orig || !utility::HookIAT(ngx_module, "advapi32.dll", RegQueryValueExW_Orig, RegQueryValueExW_Hook))
			spdlog::warn("nvngx_dlssd: failed to hook DLSS HUD functions, OverrideDlssHud might not be available.");
		else
			spdlog::info("nvngx_dlssd: applied debug hud overlay hook via registry");
	}

	return true;
}

void unhook(HMODULE ngx_module)
{
	dlssIndicatorHudHook.reset();

	spdlog::debug("nvngx_dlssd: finished unhook");
}

SafetyHookInline dllmain;
BOOL APIENTRY hooked_dllmain(HMODULE hModule, int ul_reason_for_call, LPVOID lpReserved)
{
	BOOL res = dllmain.stdcall<BOOL>(hModule, ul_reason_for_call, lpReserved);

	if (ul_reason_for_call == DLL_PROCESS_DETACH)
	{
		unhook(hModule);
		dllmain.reset();
	}

	return res;
}

// Installs DllMain hook onto nvngx_dlssd
void init(HMODULE ngx_module)
{
	if (settings.disableAllTweaks)
		return;

	hook(ngx_module);
	dllmain = safetyhook::create_inline(utility::ModuleEntryPoint(ngx_module), hooked_dllmain);
}
};


```

## Source file: src/module_hooks/nvngx_dlssg.cpp

{
  "path": "src/module_hooks/nvngx_dlssg.cpp",
  "blob_sha": "744a8c03ccff61e3723f3746d98dd7d10878a6e1",
  "sha256": "56d3bc6e36c9a3b8ef804bd1a25fbb04c5bb6b93cb0b955db9f685ea4b021855",
  "lines": 89,
  "bytes": 2082,
  "selection_score": 15,
  "keyword_line_counts": {
    "dlss": 6
  },
  "url": "https://github.com/emoose/DLSSTweaks/blob/1d2fddbe3d1e8f403f795afc17e7db239a83f6a2/src/module_hooks/nvngx_dlssg.cpp",
  "function_signature_candidates": [
    {
      "line": 18,
      "text": "void settings_changed()"
    },
    {
      "line": 76,
      "text": "void init(HMODULE ngx_module)"
    }
  ]
}

```text

#define WIN32_LEAN_AND_MEAN
#include <ntstatus.h>
#define WIN32_NO_STATUS
#include <Windows.h>
#include <winternl.h>

#include <spdlog/spdlog.h>
#include <Patterns.h>

#include "DLSSTweaks.hpp"

namespace nvngx_dlssg
{
std::mutex module_handle_mtx;
HMODULE module_handle = nullptr;

// Currently just nulls the watermark text included in certain DLSSG builds
void settings_changed()
{
	std::scoped_lock lock{ module_handle_mtx };

	if (!module_handle)
		return;

	const char patch = settings.disableDevWatermark ? 0 : 0x4E;

	// Search for DLSSG watermark text and null it if found
	auto pattern = hook::pattern(module_handle,
		"56 49 44 49 41 20 43 4F 4E 46 49 44 45 4E 54 49 41 4C 20 2D 20");

	size_t numWatermarkStrings = pattern.size();
	if (!numWatermarkStrings)
	{
		spdlog::warn("nvngx_dlssg: DisableDevWatermark failed, couldn't locate watermark string inside module");
		return;
	}

	int changed = 0;
	for (size_t i = 0; i < numWatermarkStrings; i++)
	{
		char* result = pattern.get(i).get<char>(-1);

		if (result[0] != patch)
		{
			UnprotectMemory unprotect{ (uintptr_t)result, 1 };
			*result = patch;
			changed++;
		}
	}

	if (changed)
		spdlog::info("nvngx_dlssg: DisableDevWatermark patch {} ({}/{} strings patched)", settings.disableDevWatermark ? "applied" : "removed", changed, numWatermarkStrings);
}
	
SafetyHookInline dllmain;
BOOL APIENTRY hooked_dllmain(HMODULE hModule, int ul_reason_for_call, LPVOID lpReserved)
{
	if (ul_reason_for_call == DLL_PROCESS_ATTACH)
	{
		std::scoped_lock lock{module_handle_mtx};
		module_handle = hModule;
	}

	BOOL res = dllmain.stdcall<BOOL>(hModule, ul_reason_for_call, lpReserved);

	if (ul_reason_for_call == DLL_PROCESS_DETACH)
	{
		std::scoped_lock lock{module_handle_mtx};
		module_handle = nullptr;
		dllmain.reset();
	}

	return res;
}

void init(HMODULE ngx_module)
{
	if (settings.disableAllTweaks)
		return;

	{
		std::scoped_lock lock{ module_handle_mtx };
		module_handle = ngx_module;
	}
	settings_changed();

	dllmain = safetyhook::create_inline(utility::ModuleEntryPoint(ngx_module), hooked_dllmain);
}
};


```

## Source file: src/Proxy.cpp

{
  "path": "src/Proxy.cpp",
  "blob_sha": "35e3691ae03eca71453426048907be8f90002865",
  "sha256": "c946f2abf924a655d53bbab9ecf37fb1b304e4ea17967f233a3f8d3ecb2409b2",
  "lines": 1413,
  "bytes": 42939,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 650,
    "face": 5
  },
  "url": "https://github.com/emoose/DLSSTweaks/blob/1d2fddbe3d1e8f403f795afc17e7db239a83f6a2/src/Proxy.cpp",
  "function_signature_candidates": [
    {
      "line": 53,
      "text": "PLUGIN_API void DllMain_stub()"
    },
    {
      "line": 376,
      "text": "PLUGIN_API void CloseDriver()"
    },
    {
      "line": 380,
      "text": "PLUGIN_API void DefDriverProc()"
    },
    {
      "line": 384,
      "text": "PLUGIN_API void DriverCallback()"
    },
    {
      "line": 388,
      "text": "PLUGIN_API void DrvGetModuleHandle()"
    },
    {
      "line": 392,
      "text": "PLUGIN_API void GetDriverModuleHandle()"
    },
    {
      "line": 396,
      "text": "PLUGIN_API void NotifyCallbackData()"
    },
    {
      "line": 400,
      "text": "PLUGIN_API void OpenDriver()"
    },
    {
      "line": 404,
      "text": "PLUGIN_API void PlaySound()"
    },
    {
      "line": 408,
      "text": "PLUGIN_API void PlaySoundA()"
    },
    {
      "line": 412,
      "text": "PLUGIN_API void PlaySoundW()"
    },
    {
      "line": 416,
      "text": "PLUGIN_API void SendDriverMessage()"
    },
    {
      "line": 420,
      "text": "PLUGIN_API void WOW32DriverCallback()"
    },
    {
      "line": 424,
      "text": "PLUGIN_API void WOW32ResolveMultiMediaHandle()"
    },
    {
      "line": 428,
      "text": "PLUGIN_API void WOWAppExit()"
    },
    {
      "line": 432,
      "text": "PLUGIN_API void aux32Message()"
    },
    {
      "line": 436,
      "text": "PLUGIN_API void auxGetDevCapsA()"
    },
    {
      "line": 440,
      "text": "PLUGIN_API void auxGetDevCapsW()"
    },
    {
      "line": 444,
      "text": "PLUGIN_API void auxGetNumDevs()"
    },
    {
      "line": 448,
      "text": "PLUGIN_API void auxGetVolume()"
    },
    {
      "line": 452,
      "text": "PLUGIN_API void auxOutMessage()"
    },
    {
      "line": 456,
      "text": "PLUGIN_API void auxSetVolume()"
    },
    {
      "line": 460,
      "text": "PLUGIN_API void joy32Message()"
    },
    {
      "line": 464,
      "text": "PLUGIN_API void joyConfigChanged()"
    },
    {
      "line": 468,
      "text": "PLUGIN_API void joyGetDevCapsA()"
    },
    {
      "line": 472,
      "text": "PLUGIN_API void joyGetDevCapsW()"
    },
    {
      "line": 476,
      "text": "PLUGIN_API void joyGetNumDevs()"
    },
    {
      "line": 480,
      "text": "PLUGIN_API void joyGetPos()"
    },
    {
      "line": 484,
      "text": "PLUGIN_API void joyGetPosEx()"
    },
    {
      "line": 488,
      "text": "PLUGIN_API void joyGetThreshold()"
    },
    {
      "line": 492,
      "text": "PLUGIN_API void joyReleaseCapture()"
    },
    {
      "line": 496,
      "text": "PLUGIN_API void joySetCapture()"
    },
    {
      "line": 500,
      "text": "PLUGIN_API void joySetThreshold()"
    },
    {
      "line": 504,
      "text": "PLUGIN_API void mci32Message()"
    },
    {
      "line": 508,
      "text": "PLUGIN_API void mciDriverNotify()"
    },
    {
      "line": 512,
      "text": "PLUGIN_API void mciDriverYield()"
    },
    {
      "line": 516,
      "text": "PLUGIN_API void mciExecute()"
    },
    {
      "line": 520,
      "text": "PLUGIN_API void mciFreeCommandResource()"
    },
    {
      "line": 524,
      "text": "PLUGIN_API void mciGetCreatorTask()"
    },
    {
      "line": 528,
      "text": "PLUGIN_API void mciGetDeviceIDA()"
    },
    {
      "line": 532,
      "text": "PLUGIN_API void mciGetDeviceIDFromElementIDA()"
    },
    {
      "line": 536,
      "text": "PLUGIN_API void mciGetDeviceIDFromElementIDW()"
    },
    {
      "line": 540,
      "text": "PLUGIN_API void mciGetDeviceIDW()"
    },
    {
      "line": 544,
      "text": "PLUGIN_API void mciGetDriverData()"
    },
    {
      "line": 548,
      "text": "PLUGIN_API void mciGetErrorStringA()"
    },
    {
      "line": 552,
      "text": "PLUGIN_API void mciGetErrorStringW()"
    },
    {
      "line": 556,
      "text": "PLUGIN_API void mciGetYieldProc()"
    },
    {
      "line": 560,
      "text": "PLUGIN_API void mciLoadCommandResource()"
    },
    {
      "line": 564,
      "text": "PLUGIN_API void mciSendCommandA()"
    },
    {
      "line": 568,
      "text": "PLUGIN_API void mciSendCommandW()"
    }
  ]
}

```text

#define WIN32_LEAN_AND_MEAN
#include <Windows.h>

#include <cstdio>
#include <cstdlib>
#include "Proxy.hpp"

// XAPOFX1_5.dll
typedef DWORD(WINAPI* CreateFX_ptr)(REFCLSID clsid, void* pEffect);

CreateFX_ptr CreateFX_orig;

PLUGIN_API DWORD WINAPI CreateFX(REFCLSID clsid, void* pEffect)
{
    return CreateFX_orig(clsid, pEffect);
}

// X3DAudio1_7
typedef DWORD(WINAPI* X3DAudioInitialize_ptr)(UINT32 SpeakerChannelMask, float SpeedOfSound, void* Instance);
typedef DWORD(WINAPI* X3DAudioCalculate_ptr)(void* Instance, void* pListener, void* pEmitter, UINT32 Flags, void* pDSPSettings);

X3DAudioInitialize_ptr X3DAudioInitialize_orig;
X3DAudioCalculate_ptr X3DAudioCalculate_orig;

PLUGIN_API DWORD WINAPI X3DAudioInitialize(UINT32 SpeakerChannelMask, float SpeedOfSound, void* Instance)
{
    return X3DAudioInitialize_orig(SpeakerChannelMask, SpeedOfSound, Instance);
}

PLUGIN_API DWORD WINAPI X3DAudioCalculate(void* Instance, void* pListener, void* pEmitter, UINT32 Flags, void* pDSPSettings)
{
    return X3DAudioCalculate_orig(Instance, pListener, pEmitter, Flags, pDSPSettings);
}

// XInput1_3 and XInput9_1_0
typedef DWORD(WINAPI* XInputGetState_ptr)(DWORD dwUserIndex, void* pState);
typedef DWORD(WINAPI* XInputSetState_ptr)(DWORD dwUserIndex, void* pVibration);
typedef DWORD(WINAPI* XInputGetCapabilities_ptr)(DWORD dwUserIndex, DWORD dwFlags, void* pCapabilities);
typedef void(WINAPI* XInputEnable_ptr)(BOOL enable);
typedef DWORD(WINAPI* XInputGetDSoundAudioDeviceGuids_ptr)(DWORD dwUserIndex, GUID* pDSoundRenderGuid, GUID* pDSoundCaptureGuid);
typedef DWORD(WINAPI* XInputGetBatteryInformation_ptr)(DWORD dwUserIndex, BYTE devType, void* pBatteryInformation);
typedef DWORD(WINAPI* XInputGetKeystroke_ptr)(DWORD dwUserIndex, DWORD dwReserved, void* pKeystroke);

XInputGetState_ptr XInputGetState_orig;
XInputSetState_ptr XInputSetState_orig;
XInputGetCapabilities_ptr XInputGetCapabilities_orig;
XInputEnable_ptr XInputEnable_orig;
XInputGetDSoundAudioDeviceGuids_ptr XInputGetDSoundAudioDeviceGuids_orig;
XInputGetBatteryInformation_ptr XInputGetBatteryInformation_orig;
XInputGetKeystroke_ptr XInputGetKeystroke_orig;

// xinput1_3 needs everything at the proper ordinal, proxy.def handles that, but we need something for ordinal 1 so:
PLUGIN_API void DllMain_stub()
{
}

PLUGIN_API DWORD WINAPI XInputGetState(DWORD dwUserIndex, void* pState)
{
    return XInputGetState_orig(dwUserIndex, pState);
}

PLUGIN_API DWORD WINAPI XInputSetState(DWORD dwUserIndex, void* pVibration)
{
    return XInputSetState_orig(dwUserIndex, pVibration);
}

PLUGIN_API DWORD WINAPI XInputGetCapabilities(DWORD dwUserIndex, DWORD dwFlags, void* pCapabilities)
{
    return XInputGetCapabilities_orig(dwUserIndex, dwFlags, pCapabilities);
}

PLUGIN_API void WINAPI XInputEnable(BOOL enable)
{
    XInputEnable_orig(enable);
}

PLUGIN_API DWORD WINAPI XInputGetDSoundAudioDeviceGuids(DWORD dwUserIndex, GUID* pDSoundRenderGuid, GUID* pDSoundCaptureGuid)
{
    return XInputGetDSoundAudioDeviceGuids_orig(dwUserIndex, pDSoundRenderGuid, pDSoundCaptureGuid);
}

PLUGIN_API DWORD WINAPI XInputGetBatteryInformation(DWORD dwUserIndex, BYTE devType, void* pBatteryInformation)
{
    return XInputGetBatteryInformation_orig(dwUserIndex, devType, pBatteryInformation);
}

PLUGIN_API DWORD WINAPI XInputGetKeystroke(DWORD dwUserIndex, DWORD dwReserved, void* pKeystroke)
{
    return XInputGetKeystroke_orig(dwUserIndex, dwReserved, pKeystroke);
}

// dinput8.dll
typedef HRESULT(WINAPI* DirectInput8Create_ptr)(HINSTANCE hinst, DWORD dwVersion, REFIID riidltf, LPVOID* ppvOut, void* punkOuter);

DirectInput8Create_ptr DirectInput8Create_orig;

PLUGIN_API HRESULT WINAPI DirectInput8Create(HINSTANCE hinst, DWORD dwVersion, REFIID riidltf, LPVOID* ppvOut, void* punkOuter)
{
    return DirectInput8Create_orig(hinst, dwVersion, riidltf, ppvOut, punkOuter);
}

// dxgi.dll
struct UNKNOWN
{
    BYTE unknown[20];
};

typedef HRESULT(WINAPI* DXGIDumpJournal_ptr)(void* unk);
typedef HRESULT(WINAPI* CreateDXGIFactory_ptr)(REFIID riid, void** ppFactory);
typedef HRESULT(WINAPI* CreateDXGIFactory1_ptr)(REFIID riid, void** ppFactory);
typedef HRESULT(WINAPI* CreateDXGIFactory2_ptr)(UINT Flags, REFIID riid, void** ppFactory);
typedef HRESULT(WINAPI* DXGID3D10CreateDevice_ptr)(
    HMODULE hModule, void* pFactory, void* pAdapter, UINT Flags, void* unknown, void* ppDevice);
typedef HRESULT(WINAPI* DXGID3D10CreateLayeredDevice_ptr)(UNKNOWN unk);
typedef size_t(WINAPI* DXGID3D10GetLayeredDeviceSize_ptr)(const void* pLayers, UINT NumLayers);
typedef HRESULT(WINAPI* DXGID3D10RegisterLayers_ptr)(const void* pLayers, UINT NumLayers);
typedef HRESULT(WINAPI* DXGIGetDebugInterface1_ptr)(UINT Flags, REFIID riid, void** pDebug);
typedef HRESULT(WINAPI* DXGIReportAdapterConfiguration_ptr)(DWORD unk);

DXGIDumpJournal_ptr DXGIDumpJournal_orig;
CreateDXGIFactory_ptr CreateDXGIFactory_orig;
CreateDXGIFactory1_ptr CreateDXGIFactory1_orig;
CreateDXGIFactory2_ptr CreateDXGIFactory2_orig;
DXGID3D10CreateDevice_ptr DXGID3D10CreateDevice_orig;
DXGID3D10CreateLayeredDevice_ptr DXGID3D10CreateLayeredDevice_orig;
DXGID3D10GetLayeredDeviceSize_ptr DXGID3D10GetLayeredDeviceSize_orig;
DXGID3D10RegisterLayers_ptr DXGID3D10RegisterLayers_orig;
DXGIGetDebugInterface1_ptr DXGIGetDebugInterface1_orig;
DXGIReportAdapterConfiguration_ptr DXGIReportAdapterConfiguration_orig;

PLUGIN_API HRESULT WINAPI DXGIDumpJournal(void* unk)
{
    return DXGIDumpJournal_orig(unk);
}

PLUGIN_API HRESULT WINAPI CreateDXGIFactory(REFIID riid, _Out_ void** ppFactory)
{
    return CreateDXGIFactory_orig(riid, ppFactory);
}

PLUGIN_API HRESULT WINAPI CreateDXGIFactory1(REFIID riid, _Out_ void** ppFactory)
{
    return CreateDXGIFactory1_orig(riid, ppFactory);
}

PLUGIN_API HRESULT WINAPI CreateDXGIFactory2(UINT Flags, REFIID riid, _Out_ void** ppFactory)
{
    return CreateDXGIFactory2_orig(Flags, riid, ppFactory);
}

PLUGIN_API HRESULT WINAPI DXGID3D10CreateDevice(HMODULE hModule, void* pFactory, void* pAdapter, UINT Flags, void* unknown, void* ppDevice)
{
    return DXGID3D10CreateDevice_orig(hModule, pFactory, pAdapter, Flags, unknown, ppDevice);
}

PLUGIN_API HRESULT WINAPI DXGID3D10CreateLayeredDevice(UNKNOWN unk)
{
    return DXGID3D10CreateLayeredDevice_orig(unk);
}

PLUGIN_API size_t WINAPI DXGID3D10GetLayeredDeviceSize(const void* pLayers, UINT NumLayers)
{
    return DXGID3D10GetLayeredDeviceSize_orig(pLayers, NumLayers);
}

PLUGIN_API HRESULT WINAPI DXGID3D10RegisterLayers(const void* pLayers, UINT NumLayers)
{
    return DXGID3D10RegisterLayers_orig(pLayers, NumLayers);
}

PLUGIN_API HRESULT WINAPI DXGIGetDebugInterface1(UINT Flags, REFIID riid, void** pDebug)
{
    return DXGIGetDebugInterface1_orig(Flags, riid, pDebug);
}

PLUGIN_API HRESULT WINAPI DXGIReportAdapterConfiguration(DWORD unk)
{
    return DXGIReportAdapterConfiguration_orig(unk);
}

// winmm.dll
FARPROC PlaySoundW_orig;
FARPROC timeSetEvent_orig;
FARPROC timeKillEvent_orig;
FARPROC midiOutMessage_orig;
FARPROC timeBeginPeriod_orig;
FARPROC timeGetTime_orig;
FARPROC NotifyCallbackData_orig;
FARPROC WOW32DriverCallback_orig;
FARPROC WOW32ResolveMultiMediaHandle_orig;
FARPROC aux32Message_orig;
FARPROC joy32Message_orig;
FARPROC mid32Message_orig;
FARPROC mod32Message_orig;
FARPROC mxd32Message_orig;
FARPROC tid32Message_orig;
FARPROC wid32Message_orig;
FARPROC wod32Message_orig;
FARPROC mci32Message_orig;
FARPROC CloseDriver_orig;
FARPROC DefDriverProc_orig;
FARPROC DriverCallback_orig;
FARPROC DrvGetModuleHandle_orig;
FARPROC GetDriverModuleHandle_orig;
FARPROC OpenDriver_orig;
FARPROC PlaySound_orig;
FARPROC Ordinal2_orig;
FARPROC SendDriverMessage_orig;
FARPROC auxGetDevCapsA_orig;
FARPROC auxGetDevCapsW_orig;
FARPROC auxGetNumDevs_orig;
FARPROC auxGetVolume_orig;
FARPROC auxOutMessage_orig;
FARPROC auxSetVolume_orig;
FARPROC joyConfigChanged_orig;
FARPROC joyGetDevCapsA_orig;
FARPROC joyGetDevCapsW_orig;
FARPROC joyGetNumDevs_orig;
FARPROC joyGetPosEx_orig;
FARPROC joyGetPos_orig;
FARPROC joyGetThreshold_orig;
FARPROC joyReleaseCapture_orig;
FARPROC joySetCapture_orig;
FARPROC joySetThreshold_orig;
FARPROC midiConnect_orig;
FARPROC midiDisconnect_orig;
FARPROC midiInAddBuffer_orig;
FARPROC midiInClose_orig;
FARPROC midiInGetDevCapsA_orig;
FARPROC midiInGetDevCapsW_orig;
FARPROC midiInGetErrorTextA_orig;
FARPROC midiInGetErrorTextW_orig;
FARPROC midiInGetID_orig;
FARPROC midiInGetNumDevs_orig;
FARPROC midiInMessage_orig;
FARPROC midiInOpen_orig;
FARPROC midiInPrepareHeader_orig;
FARPROC midiInReset_orig;
FARPROC midiInStart_orig;
FARPROC midiInStop_orig;
FARPROC midiInUnprepareHeader_orig;
FARPROC midiOutCacheDrumPatches_orig;
FARPROC midiOutCachePatches_orig;
FARPROC midiOutClose_orig;
FARPROC midiOutGetDevCapsA_orig;
FARPROC midiOutGetDevCapsW_orig;
FARPROC midiOutGetErrorTextA_orig;
FARPROC midiOutGetErrorTextW_orig;
FARPROC midiOutGetID_orig;
FARPROC midiOutGetNumDevs_orig;
FARPROC midiOutGetVolume_orig;
FARPROC midiOutLongMsg_orig;
FARPROC midiOutOpen_orig;
FARPROC midiOutPrepareHeader_orig;
FARPROC midiOutReset_orig;
FARPROC midiOutSetVolume_orig;
FARPROC midiOutShortMsg_orig;
FARPROC midiOutUnprepareHeader_orig;
FARPROC midiStreamClose_orig;
FARPROC midiStreamOpen_orig;
FARPROC midiStreamOut_orig;
FARPROC midiStreamPause_orig;
FARPROC midiStreamPosition_orig;
FARPROC midiStreamProperty_orig;
FARPROC midiStreamRestart_orig;
FARPROC midiStreamStop_orig;
FARPROC mixerClose_orig;
FARPROC mixerGetControlDetailsA_orig;
FARPROC mixerGetControlDetailsW_orig;
FARPROC mixerGetDevCapsA_orig;
FARPROC mixerGetDevCapsW_orig;
FARPROC mixerGetID_orig;
FARPROC mixerGetLineControlsA_orig;
FARPROC mixerGetLineControlsW_orig;
FARPROC mixerGetLineInfoA_orig;
FARPROC mixerGetLineInfoW_orig;
FARPROC mixerGetNumDevs_orig;
FARPROC mixerMessage_orig;
FARPROC mixerOpen_orig;
FARPROC mixerSetControlDetails_orig;
FARPROC mmDrvInstall_orig;
FARPROC mmGetCurrentTask_orig;
FARPROC mmTaskBlock_orig;
FARPROC mmTaskCreate_orig;
FARPROC mmTaskSignal_orig;
FARPROC mmTaskYield_orig;
FARPROC mmioAdvance_orig;
FARPROC mmioAscend_orig;
FARPROC mmioClose_orig;
FARPROC mmioCreateChunk_orig;
FARPROC mmioDescend_orig;
FARPROC mmioFlush_orig;
FARPROC mmioGetInfo_orig;
FARPROC mmioInstallIOProcA_orig;
FARPROC mmioInstallIOProcW_orig;
FARPROC mmioOpenA_orig;
FARPROC mmioOpenW_orig;
FARPROC mmioRead_orig;
FARPROC mmioRenameA_orig;
FARPROC mmioRenameW_orig;
FARPROC mmioSeek_orig;
FARPROC mmioSendMessage_orig;
FARPROC mmioSetBuffer_orig;
FARPROC mmioSetInfo_orig;
FARPROC mmioStringToFOURCCA_orig;
FARPROC mmioStringToFOURCCW_orig;
FARPROC mmioWrite_orig;
FARPROC timeEndPeriod_orig;
FARPROC timeGetDevCaps_orig;
FARPROC timeGetSystemTime_orig;
FARPROC waveInAddBuffer_orig;
FARPROC waveInClose_orig;
FARPROC waveInGetDevCapsA_orig;
FARPROC waveInGetDevCapsW_orig;
FARPROC waveInGetErrorTextA_orig;
FARPROC waveInGetErrorTextW_orig;
FARPROC waveInGetID_orig;
FARPROC waveInGetNumDevs_orig;
FARPROC waveInGetPosition_orig;
FARPROC waveInMessage_orig;
FARPROC waveInOpen_orig;
FARPROC waveInPrepareHeader_orig;
FARPROC waveInReset_orig;
FARPROC waveInStart_orig;
FARPROC waveInStop_orig;
FARPROC waveInUnprepareHeader_orig;
FARPROC waveOutBreakLoop_orig;
FARPROC waveOutClose_orig;
FARPROC waveOutGetDevCapsA_orig;
FARPROC waveOutGetDevCapsW_orig;
FARPROC waveOutGetErrorTextA_orig;
FARPROC waveOutGetErrorTextW_orig;
FARPROC waveOutGetID_orig;
FARPROC waveOutGetNumDevs_orig;
FARPROC waveOutGetPitch_orig;
FARPROC waveOutGetPlaybackRate_orig;
FARPROC waveOutGetPosition_orig;
FARPROC waveOutGetVolume_orig;
FARPROC waveOutMessage_orig;
FARPROC waveOutOpen_orig;
FARPROC waveOutPause_orig;
FARPROC waveOutPrepareHeader_orig;
FARPROC waveOutReset_orig;
FARPROC waveOutRestart_orig;
FARPROC waveOutSetPitch_orig;
FARPROC waveOutSetPlaybackRate_orig;
FARPROC waveOutSetVolume_orig;
FARPROC waveOutUnprepareHeader_orig;
FARPROC waveOutWrite_orig;
FARPROC mciExecute_orig;
FARPROC mciGetErrorStringA_orig;
FARPROC mciGetErrorStringW_orig;
FARPROC mciSendCommandA_orig;
FARPROC mciSendCommandW_orig;
FARPROC mciSendStringA_orig;
FARPROC mciSendStringW_orig;
FARPROC mciFreeCommandResource_orig;
FARPROC mciLoadCommandResource_orig;
FARPROC mciDriverNotify_orig;
FARPROC mciDriverYield_orig;
FARPROC mciGetCreatorTask_orig;
FARPROC mciGetDeviceIDA_orig;
FARPROC mciGetDeviceIDFromElementIDA_orig;
FARPROC mciGetDeviceIDFromElementIDW_orig;
FARPROC mciGetDeviceIDW_orig;
FARPROC mciGetDriverData_orig;
FARPROC mciGetYieldProc_orig;
FARPROC mciSetDriverData_orig;
FARPROC mciSetYieldProc_orig;
FARPROC PlaySoundA_orig;
FARPROC sndPlaySoundA_orig;
FARPROC sndPlaySoundW_orig;
FARPROC WOWAppExit_orig;
FARPROC mmsystemGetVersion_orig;

PLUGIN_API void CloseDriver()
{
    CloseDriver_orig();
}
PLUGIN_API void DefDriverProc()
{
    DefDriverProc_orig();
}
PLUGIN_API void DriverCallback()
{
    DriverCallback_orig();
}
PLUGIN_API void DrvGetModuleHandle()
{
    DrvGetModuleHandle_orig();
}
PLUGIN_API void GetDriverModuleHandle()
{
    GetDriverModuleHandle_orig();
}
PLUGIN_API void NotifyCallbackData()
{
    NotifyCallbackData_orig();
}
PLUGIN_API void OpenDriver()
{
    OpenDriver_orig();
}
PLUGIN_API void PlaySound()
{
    PlaySound_orig();
}
PLUGIN_API void PlaySoundA()
{
    PlaySoundA_orig();
}
PLUGIN_API void PlaySoundW()
{
    PlaySoundW_orig();
}
PLUGIN_API void SendDriverMessage()
{
    SendDriverMessage_orig();
}
PLUGIN_API void WOW32DriverCallback()
{
    WOW32DriverCallback_orig();
}
PLUGIN_API void WOW32ResolveMultiMediaHandle()
{
    WOW32ResolveMultiMediaHandle_orig();
}
PLUGIN_API void WOWAppExit()
{
    WOWAppExit_orig();
}
PLUGIN_API void aux32Message()
{
    aux32Message_orig();
}
PLUGIN_API void auxGetDevCapsA()
{
    auxGetDevCapsA_orig();
}
PLUGIN_API void auxGetDevCapsW()
{
    auxGetDevCapsW_orig();
}
PLUGIN_API void auxGetNumDevs()
{
    auxGetNumDevs_orig();
}
PLUGIN_API void auxGetVolume()
{
    auxGetVolume_orig();
}
PLUGIN_API void auxOutMessage()
{
    auxOutMessage_orig();
}
PLUGIN_API void auxSetVolume()
{
    auxSetVolume_orig();
}
PLUGIN_API void joy32Message()
{
    joy32Message_orig();
}
PLUGIN_API void joyConfigChanged()
{
    joyConfigChanged_orig();
}
PLUGIN_API void joyGetDevCapsA()
{
    joyGetDevCapsA_orig();
}
PLUGIN_API void joyGetDevCapsW()
{
    joyGetDevCapsW_orig();
}
PLUGIN_API void joyGetNumDevs()
{
    joyGetNumDevs_orig();
}
PLUGIN_API void joyGetPos()
{
    joyGetPos_orig();
}
PLUGIN_API void joyGetPosEx()
{
    joyGetPosEx_orig();
}
PLUGIN_API void joyGetThreshold()
{
    joyGetThreshold_orig();
}
PLUGIN_API void joyReleaseCapture()
{
    joyReleaseCapture_orig();
}
PLUGIN_API void joySetCapture()
{
    joySetCapture_orig();
}
PLUGIN_API void joySetThreshold()
{
    joySetThreshold_orig();
}
PLUGIN_API void mci32Message()
{
    mci32Message_orig();
}
PLUGIN_API void mciDriverNotify()
{
    mciDriverNotify_orig();
}
PLUGIN_API void mciDriverYield()
{
    mciDriverYield_orig();
}
PLUGIN_API void mciExecute()
{
    mciExecute_orig();
}
PLUGIN_API void mciFreeCommandResource()
{
    mciFreeCommandResource_orig();
}
PLUGIN_API void mciGetCreatorTask()
{
    mciGetCreatorTask_orig();
}
PLUGIN_API void mciGetDeviceIDA()
{
    mciGetDeviceIDA_orig();
}
PLUGIN_API void mciGetDeviceIDFromElementIDA()
{
    mciGetDeviceIDFromElementIDA_orig();
}
PLUGIN_API void mciGetDeviceIDFromElementIDW()
{
    mciGetDeviceIDFromElementIDW_orig();
}
PLUGIN_API void mciGetDeviceIDW()
{
    mciGetDeviceIDW_orig();
}
PLUGIN_API void mciGetDriverData()
{
    mciGetDriverData_orig();
}
PLUGIN_API void mciGetErrorStringA()
{
    mciGetErrorStringA_orig();
}
PLUGIN_API void mciGetErrorStringW()
{
    mciGetErrorStringW_orig();
}
PLUGIN_API void mciGetYieldProc()
{
    mciGetYieldProc_orig();
}
PLUGIN_API void mciLoadCommandResource()
{
    mciLoadCommandResource_orig();
}
PLUGIN_API void mciSendCommandA()
{
    mciSendCommandA_orig();
}
PLUGIN_API void mciSendCommandW()
{
    mciSendCommandW_orig();
}
PLUGIN_API void mciSendStringA()
{
    mciSendStringA_orig();
}
PLUGIN_API void mciSendStringW()
{
    mciSendStringW_orig();
}
PLUGIN_API void mciSetDriverData()
{
    mciSetDriverData_orig();
}
PLUGIN_API void mciSetYieldProc()
{
    mciSetYieldProc_orig();
}
PLUGIN_API void mid32Message()
{
    mid32Message_orig();
}
PLUGIN_API void midiConnect()
{
    midiConnect_orig();
}
PLUGIN_API void midiDisconnect()
{
    midiDisconnect_orig();
}
PLUGIN_API void midiInAddBuffer()
{
    midiInAddBuffer_orig();
}
PLUGIN_API void midiInClose()
{
    midiInClose_orig();
}
PLUGIN_API void midiInGetDevCapsA()
{
    midiInGetDevCapsA_orig();
}
PLUGIN_API void midiInGetDevCapsW()
{
    midiInGetDevCapsW_orig();
}
PLUGIN_API void midiInGetErrorTextA()
{
    midiInGetErrorTextA_orig();
}
PLUGIN_API void midiInGetErrorTextW()
{
    midiInGetErrorTextW_orig();
}
PLUGIN_API void midiInGetID()
{
    midiInGetID_orig();
}
PLUGIN_API void midiInGetNumDevs()
{
    midiInGetNumDevs_orig();
}
PLUGIN_API void midiInMessage()
{
    midiInMessage_orig();
}
PLUGIN_API void midiInOpen()
{
    midiInOpen_orig();
}
PLUGIN_API void midiInPrepareHeader()
{
    midiInPrepareHeader_orig();
}
PLUGIN_API void midiInReset()
{
    midiInReset_orig();
}
PLUGIN_API void midiInStart()
{
    midiInStart_orig();
}
PLUGIN_API void midiInStop()
{
    midiInStop_orig();
}
PLUGIN_API void midiInUnprepareHeader()
{
    midiInUnprepareHeader_orig();
}
PLUGIN_API void midiOutCacheDrumPatches()
{
    midiOutCacheDrumPatches_orig();
}
PLUGIN_API void midiOutCachePatches()
{
    midiOutCachePatches_orig();
}
PLUGIN_API void midiOutClose()
{
    midiOutClose_orig();
}
PLUGIN_API void midiOutGetDevCapsA()
{
    midiOutGetDevCapsA_orig();
}
PLUGIN_API void midiOutGetDevCapsW()
{
    midiOutGetDevCapsW_orig();
}
PLUGIN_API void midiOutGetErrorTextA()
{
    midiOutGetErrorTextA_orig();
}
PLUGIN_API void midiOutGetErrorTextW()
{
    midiOutGetErrorTextW_orig();
}
PLUGIN_API void midiOutGetID()
{
    midiOutGetID_orig();
}
PLUGIN_API void midiOutGetNumDevs()
{
    midiOutGetNumDevs_orig();
}
PLUGIN_API void midiOutGetVolume()
{
    midiOutGetVolume_orig();
}
PLUGIN_API void midiOutLongMsg()
{
    midiOutLongMsg_orig();
}
PLUGIN_API void midiOutMessage()
{
    midiOutMessage_orig();
}
PLUGIN_API void midiOutOpen()
{
    midiOutOpen_orig();
}
PLUGIN_API void midiOutPrepareHeader()
{
    midiOutPrepareHeader_orig();
}
PLUGIN_API void midiOutReset()
{
    midiOutReset_orig();
}
PLUGIN_API void midiOutSetVolume()
{
    midiOutSetVolume_orig();
}
PLUGIN_API void midiOutShortMsg()
{
    midiOutShortMsg_orig();
}
PLUGIN_API void midiOutUnprepareHeader()
{
    midiOutUnprepareHeader_orig();
}
PLUGIN_API void midiStreamClose()
{
    midiStreamClose_orig();
}
PLUGIN_API void midiStreamOpen()
{
    midiStreamOpen_orig();
}
PLUGIN_API void midiStreamOut()
{
    midiStreamOut_orig();
}
PLUGIN_API void midiStreamPause()
{
    midiStreamPause_orig();
}
PLUGIN_API void midiStreamPosition()
{
    midiStreamPosition_orig();
}
PLUGIN_API void midiStreamProperty()
{
    midiStreamProperty_orig();
}
PLUGIN_API void midiStreamRestart()
{
    midiStreamRestart_orig();
}
PLUGIN_API void midiStreamStop()
{
    midiStreamStop_orig();
}
PLUGIN_API void mixerClose()
{
    mixerClose_orig();
}
PLUGIN_API void mixerGetControlDetailsA()
{
    mixerGetControlDetailsA_orig();
}
PLUGIN_API void mixerGetControlDetailsW()
{
    mixerGetControlDetailsW_orig();
}
PLUGIN_API void mixerGetDevCapsA()
{
    mixerGetDevCapsA_orig();
}
PLUGIN_API void mixerGetDevCapsW()
{
    mixerGetDevCapsW_orig();
}
PLUGIN_API void mixerGetID()
{
    mixerGetID_orig();
}
PLUGIN_API void mixerGetLineControlsA()
{
    mixerGetLineControlsA_orig();
}
PLUGIN_API void mixerGetLineControlsW()
{
    mixerGetLineControlsW_orig();
}
PLUGIN_API void mixerGetLineInfoA()
{
    mixerGetLineInfoA_orig();
}
PLUGIN_API void mixerGetLineInfoW()
{
    mixerGetLineInfoW_orig();
}
PLUGIN_API void mixerGetNumDevs()
{
    mixerGetNumDevs_orig();
}
PLUGIN_API void mixerMessage()
{
    mixerMessage_orig();
}
PLUGIN_API void mixerOpen()
{
    mixerOpen_orig();
}
PLUGIN_API void mixerSetControlDetails()
{
    mixerSetControlDetails_orig();
}
PLUGIN_API void mmDrvInstall()
{
    mmDrvInstall_orig();
}
PLUGIN_API void mmGetCurrentTask()
{
    mmGetCurrentTask_orig();
}
PLUGIN_API void mmTaskBlock()
{
    mmTaskBlock_orig();
}
PLUGIN_API void mmTaskCreate()
{
    mmTaskCreate_orig();
}
PLUGIN_API void mmTaskSignal()
{
    mmTaskSignal_orig();
}
PLUGIN_API void mmTaskYield()
{
    mmTaskYield_orig();
}
PLUGIN_API void mmioAdvance()
{
    mmioAdvance_orig();
}
PLUGIN_API void mmioAscend()
{
    mmioAscend_orig();
}
PLUGIN_API void mmioClose()
{
    mmioClose_orig();
}
PLUGIN_API void mmioCreateChunk()
{
    mmioCreateChunk_orig();
}
PLUGIN_API void mmioDescend()
{
    mmioDescend_orig();
}
PLUGIN_API void mmioFlush()
{
    mmioFlush_orig();
}
PLUGIN_API void mmioGetInfo()
{
    mmioGetInfo_orig();
}
PLUGIN_API void mmioInstallIOProcA()
{
    mmioInstallIOProcA_orig();
}
PLUGIN_API void mmioInstallIOProcW()
{
    mmioInstallIOProcW_orig();
}
PLUGIN_API void mmioOpenA()
{
    mmioOpenA_orig();
}
PLUGIN_API void mmioOpenW()
{
    mmioOpenW_orig();
}
PLUGIN_API void mmioRead()
{
    mmioRead_orig();
}
PLUGIN_API void mmioRenameA()
{
    mmioRenameA_orig();
}
PLUGIN_API void mmioRenameW()
{
    mmioRenameW_orig();
}
PLUGIN_API void mmioSeek()
{
    mmioSeek_orig();
}
PLUGIN_API void mmioSendMessage()
{
    mmioSendMessage_orig();
}
PLUGIN_API void mmioSetBuffer()
{
    mmioSetBuffer_orig();
}
PLUGIN_API void mmioSetInfo()
{
    mmioSetInfo_orig();
}
PLUGIN_API void mmioStringToFOURCCA()
{
    mmioStringToFOURCCA_orig();
}
PLUGIN_API void mmioStringToFOURCCW()
{
    mmioStringToFOURCCW_orig();
}
PLUGIN_API void mmioWrite()
{
    mmioWrite_orig();
}
PLUGIN_API void mmsystemGetVersion()
{
    mmsystemGetVersion_orig();
}
PLUGIN_API void mod32Message()
{
    mod32Message_orig();
}
PLUGIN_API void mxd32Message()
{
    mxd32Message_orig();
}
PLUGIN_API void sndPlaySoundA()
{
    sndPlaySoundA_orig();
}
PLUGIN_API void sndPlaySoundW()
{
    sndPlaySoundW_orig();
}
PLUGIN_API void tid32Message()
{
    tid32Message_orig();
}
PLUGIN_API void timeBeginPeriod()
{
    timeBeginPeriod_orig();
}
PLUGIN_API void timeEndPeriod()
{
    timeEndPeriod_orig();
}
PLUGIN_API void timeGetDevCaps()
{
    timeGetDevCaps_orig();
}
PLUGIN_API void timeGetSystemTime()
{
    timeGetSystemTime_orig();
}
PLUGIN_API void timeGetTime()
{
    timeGetTime_orig();
}
PLUGIN_API void timeKillEvent()
{
    timeKillEvent_orig();
}
PLUGIN_API void timeSetEvent()
{
    timeSetEvent_orig();
}
PLUGIN_API void waveInAddBuffer()
{
    waveInAddBuffer_orig();
}
PLUGIN_API void waveInClose()
{
    waveInClose_orig();
}
PLUGIN_API void waveInGetDevCapsA()
{
    waveInGetDevCapsA_orig();
}
PLUGIN_API void waveInGetDevCapsW()
{
    waveInGetDevCapsW_orig();
}
PLUGIN_API void waveInGetErrorTextA()
{
    waveInGetErrorTextA_orig();
}
PLUGIN_API void waveInGetErrorTextW()
{
    waveInGetErrorTextW_orig();
}
PLUGIN_API void waveInGetID()
{
    waveInGetID_orig();
}
PLUGIN_API void waveInGetNumDevs()
{
    waveInGetNumDevs_orig();
}
PLUGIN_API void waveInGetPosition()
{
    waveInGetPosition_orig();
}
PLUGIN_API void waveInMessage()
{
    waveInMessage_orig();
}
PLUGIN_API void waveInOpen()
{
    waveInOpen_orig();
}
PLUGIN_API void waveInPrepareHeader()
{
    waveInPrepareHeader_orig();
}
PLUGIN_API void waveInReset()
{
    waveInReset_orig();
}
PLUGIN_API void waveInStart()
{
    waveInStart_orig();
}
PLUGIN_API void waveInStop()
{
    waveInStop_orig();
}
PLUGIN_API void waveInUnprepareHeader()
{
    waveInUnprepareHeader_orig();
}
PLUGIN_API void waveOutBreakLoop()
{
    waveOutBreakLoop_orig();
}
PLUGIN_API void waveOutClose()
{
    waveOutClose_orig();
}
PLUGIN_API void waveOutGetDevCapsA()
{
    waveOutGetDevCapsA_orig();
}
PLUGIN_API void waveOutGetDevCapsW()
{
    waveOutGetDevCapsW_orig();
}
PLUGIN_API void waveOutGetErrorTextA()
{
    waveOutGetErrorTextA_orig();
}
PLUGIN_API void waveOutGetErrorTextW()
{
    waveOutGetErrorTextW_orig();
}
PLUGIN_API void waveOutGetID()
{
    waveOutGetID_orig();
}
PLUGIN_API void waveOutGetNumDevs()
{
    waveOutGetNumDevs_orig();
}
PLUGIN_API void waveOutGetPitch()
{
    waveOutGetPitch_orig();
}
PLUGIN_API void waveOutGetPlaybackRate()
{
    waveOutGetPlaybackRate_orig();
}
PLUGIN_API void waveOutGetPosition()
{
    waveOutGetPosition_orig();
}
PLUGIN_API void waveOutGetVolume()
{
    waveOutGetVolume_orig();
}
PLUGIN_API void waveOutMessage()
{
    waveOutMessage_orig();
}
PLUGIN_API void waveOutOpen()
{
    waveOutOpen_orig();
}
PLUGIN_API void waveOutPause()
{
    waveOutPause_orig();
}
PLUGIN_API void waveOutPrepareHeader()
{
    waveOutPrepareHeader_orig();
}
PLUGIN_API void waveOutReset()
{
    waveOutReset_orig();
}
PLUGIN_API void waveOutRestart()
{
    waveOutRestart_orig();
}
PLUGIN_API void waveOutSetPitch()
{
    waveOutSetPitch_orig();
}
PLUGIN_API void waveOutSetPlaybackRate()
{
    waveOutSetPlaybackRate_orig();
}
PLUGIN_API void waveOutSetVolume()
{
    waveOutSetVolume_orig();
}
PLUGIN_API void waveOutUnprepareHeader()
{
    waveOutUnprepareHeader_orig();
}
PLUGIN_API void waveOutWrite()
{
    waveOutWrite_orig();
}
PLUGIN_API void wid32Message()
{
    wid32Message_orig();
}
PLUGIN_API void wod32Message()
{
    wod32Message_orig();
}

namespace proxy
{
HMODULE origModule = NULL;
bool is_wrapping_nvngx = false;

bool on_attach(HMODULE ourModule)
{
    // get the filename of our DLL and try loading the DLL with the same name from system dir
    WCHAR modulePath[MAX_PATH] = {0};
    if (!GetSystemDirectoryW(modulePath, _countof(modulePath)))
        return false;

    WCHAR ourModulePath[MAX_PATH] = {0};
    GetModuleFileNameW(ourModule, ourModulePath, _countof(ourModulePath));

    WCHAR exeName[MAX_PATH] = {0};
    WCHAR extName[MAX_PATH] = {0};
    _wsplitpath_s(ourModulePath, NULL, NULL, NULL, NULL, exeName, MAX_PATH, extName, MAX_PATH);

    if (!_wcsicmp(exeName, L"nvngx"))
    {
        is_wrapping_nvngx = true;
        return proxy_nvngx::on_attach(ourModule);
    }

    WCHAR origModulePath[MAX_PATH] = {0};
    swprintf_s(origModulePath, MAX_PATH, L"%ws\\%ws%ws", modulePath, exeName, extName);

    origModule = LoadLibraryW(origModulePath);
    if (!origModule)
        return false;

    XInputGetCapabilities_orig = (XInputGetCapabilities_ptr)GetProcAddress(origModule, "XInputGetCapabilities");
    XInputGetDSoundAudioDeviceGuids_orig =
        (XInputGetDSoundAudioDeviceGuids_ptr)GetProcAddress(origModule, "XInputGetDSoundAudioDeviceGuids");
    XInputGetState_orig = (XInputGetState_ptr)GetProcAddress(origModule, "XInputGetState");
    XInputSetState_orig = (XInputSetState_ptr)GetProcAddress(origModule, "XInputSetState");
    XInputEnable_orig = (XInputEnable_ptr)GetProcAddress(origModule, "XInputEnable");
    XInputGetBatteryInformation_orig = (XInputGetBatteryInformation_ptr)GetProcAddress(origModule, "XInputGetBatteryInformation");
    XInputGetKeystroke_orig = (XInputGetKeystroke_ptr)GetProcAddress(origModule, "XInputGetKeystroke");

    DirectInput8Create_orig = (DirectInput8Create_ptr)GetProcAddress(origModule, "DirectInput8Create");

    DXGIDumpJournal_orig = (DXGIDumpJournal_ptr)GetProcAddress(origModule, "DXGIDumpJournal");
    CreateDXGIFactory_orig = (CreateDXGIFactory_ptr)GetProcAddress(origModule, "CreateDXGIFactory");
    CreateDXGIFactory1_orig = (CreateDXGIFactory1_ptr)GetProcAddress(origModule, "CreateDXGIFactory1");
    CreateDXGIFactory2_orig = (CreateDXGIFactory2_ptr)GetProcAddress(origModule, "CreateDXGIFactory2");
    DXGID3D10CreateDevice_orig = (DXGID3D10CreateDevice_ptr)GetProcAddress(origModule, "DXGID3D10CreateDevice");
    DXGID3D10CreateLayeredDevice_orig = (DXGID3D10CreateLayeredDevice_ptr)GetProcAddress(origModule, "DXGID3D10CreateLayeredDevice");
    DXGID3D10GetLayeredDeviceSize_orig = (DXGID3D10GetLayeredDeviceSize_ptr)GetProcAddress(origModule, "DXGID3D10GetLayeredDeviceSize");
    DXGID3D10RegisterLayers_orig = (DXGID3D10RegisterLayers_ptr)GetProcAddress(origModule, "DXGID3D10RegisterLayers");
    DXGIGetDebugInterface1_orig = (DXGIGetDebugInterface1_ptr)GetProcAddress(origModule, "DXGIGetDebugInterface1");
    DXGIReportAdapterConfiguration_orig = (DXGIReportAdapterConfiguration_ptr)GetProcAddress(origModule, "DXGIReportAdapterConfiguration");

    X3DAudioInitialize_orig = (X3DAudioInitialize_ptr)GetProcAddress(origModule, "X3DAudioInitialize");
    X3DAudioCalculate_orig = (X3DAudioCalculate_ptr)GetProcAddress(origModule, "X3DAudioCalculate");

    CreateFX_orig = (CreateFX_ptr)GetProcAddress(origModule, "CreateFX");

    PlaySoundW_orig = GetProcAddress(origModule, "PlaySoundW");
    timeSetEvent_orig = GetProcAddress(origModule, "timeSetEvent");
    timeKillEvent_orig = GetProcAddress(origModule, "timeKillEvent");
    midiOutMessage_orig = GetProcAddress(origModule, "midiOutMessage");
    timeBeginPeriod_orig = GetProcAddress(origModule, "timeBeginPeriod");
    timeGetTime_orig = GetProcAddress(origModule, "timeGetTime");
    NotifyCallbackData_orig = GetProcAddress(origModule, "NotifyCallbackData");
    WOW32DriverCallback_orig = GetProcAddress(origModule, "WOW32DriverCallback");
    WOW32ResolveMultiMediaHandle_orig = GetProcAddress(origModule, "WOW32ResolveMultiMediaHandle");
    aux32Message_orig = GetProcAddress(origModule, "aux32Message");
    joy32Message_orig = GetProcAddress(origModule, "joy32Message");
    mid32Message_orig = GetProcAddress(origModule, "mid32Message");
    mod32Message_orig = GetProcAddress(origModule, "mod32Message");
    mxd32Message_orig = GetProcAddress(origModule, "mxd32Message");
    tid32Message_orig = GetProcAddress(origModule, "tid32Message");
    wid32Message_orig = GetProcAddress(origModule, "wid32Message");
    wod32Message_orig = GetProcAddress(origModule, "wod32Message");
    mci32Message_orig = GetProcAddress(origModule, "mci32Message");
    CloseDriver_orig = GetProcAddress(origModule, "CloseDriver");
    DefDriverProc_orig = GetProcAddress(origModule, "DefDriverProc");
    DriverCallback_orig = GetProcAddress(origModule, "DriverCallback");
    DrvGetModuleHandle_orig = GetProcAddress(origModule, "DrvGetModuleHandle");
    GetDriverModuleHandle_orig = GetProcAddress(origModule, "GetDriverModuleHandle");
    OpenDriver_orig = GetProcAddress(origModule, "OpenDriver");
    PlaySound_orig = GetProcAddress(origModule, "PlaySound");
    Ordinal2_orig = GetProcAddress(origModule, "Ordinal2");
    SendDriverMessage_orig = GetProcAddress(origModule, "SendDriverMessage");
    auxGetDevCapsA_orig = GetProcAddress(origModule, "auxGetDevCapsA");
    auxGetDevCapsW_orig = GetProcAddress(origModule, "auxGetDevCapsW");
    auxGetNumDevs_orig = GetProcAddress(origModule, "auxGetNumDevs");
    auxGetVolume_orig = GetProcAddress(origModule, "auxGetVolume");
    auxOutMessage_orig = GetProcAddress(origModule, "auxOutMessage");
    auxSetVolume_orig = GetProcAddress(origModule, "auxSetVolume");
    joyConfigChanged_orig = GetProcAddress(origModule, "joyConfigChanged");
    joyGetDevCapsA_orig = GetProcAddress(origModule, "joyGetDevCapsA");
    joyGetDevCapsW_orig = GetProcAddress(origModule, "joyGetDevCapsW");
    joyGetNumDevs_orig = GetProcAddress(origModule, "joyGetNumDevs");
    joyGetPosEx_orig = GetProcAddress(origModule, "joyGetPosEx");
    joyGetPos_orig = GetProcAddress(origModule, "joyGetPos");
    joyGetThreshold_orig = GetProcAddress(origModule, "joyGetThreshold");
    joyReleaseCapture_orig = GetProcAddress(origModule, "joyReleaseCapture");
    joySetCapture_orig = GetProcAddress(origModule, "joySetCapture");
    joySetThreshold_orig = GetProcAddress(origModule, "joySetThreshold");
    midiConnect_orig = GetProcAddress(origModule, "midiConnect");
    midiDisconnect_orig = GetProcAddress(origModule, "midiDisconnect");
    midiInAddBuffer_orig = GetProcAddress(origModule, "midiInAddBuffer");
    midiInClose_orig = GetProcAddress(origModule, "midiInClose");
    midiInGetDevCapsA_orig = GetProcAddress(origModule, "midiInGetDevCapsA");
    midiInGetDevCapsW_orig = GetProcAddress(origModule, "midiInGetDevCapsW");
    midiInGetErrorTextA_orig = GetProcAddress(origModule, "midiInGetErrorTextA");
    midiInGetErrorTextW_orig = GetProcAddress(origModule, "midiInGetErrorTextW");
    midiInGetID_orig = GetProcAddress(origModule, "midiInGetID");
    midiInGetNumDevs_orig = GetProcAddress(origModule, "midiInGetNumDevs");
    midiInMessage_orig = GetProcAddress(origModule, "midiInMessage");
    midiInOpen_orig = GetProcAddress(origModule, "midiInOpen");
    midiInPrepareHeader_orig = GetProcAddress(origModule, "midiInPrepareHeader");
    midiInReset_orig = GetProcAddress(origModule, "midiInReset");
    midiInStart_orig = GetProcAddress(origModule, "midiInStart");
    midiInStop_orig = GetProcAddress(origModule, "midiInStop");
    midiInUnprepareHeader_orig = GetProcAddress(origModule, "midiInUnprepareHeader");
    midiOutCacheDrumPatches_orig = GetProcAddress(origModule, "midiOutCacheDrumPatches");
    midiOutCachePatches_orig = GetProcAddress(origModule, "midiOutCachePatches");
    midiOutClose_orig = GetProcAddress(origModule, "midiOutClose");
    midiOutGetDevCapsA_orig = GetProcAddress(origModule, "midiOutGetDevCapsA");
    midiOutGetDevCapsW_orig = GetProcAddress(origModule, "midiOutGetDevCapsW");
    midiOutGetErrorTextA_orig = GetProcAddress(origModule, "midiOutGetErrorTextA");
    midiOutGetErrorTextW_orig = GetProcAddress(origModule, "midiOutGetErrorTextW");
    midiOutGetID_orig = GetProcAddress(origModule, "midiOutGetID");
    midiOutGetNumDevs_orig = GetProcAddress(origModule, "midiOutGetNumDevs");
    midiOutGetVolume_orig = GetProcAddress(origModule, "midiOutGetVolume");
    midiOutLongMsg_orig = GetProcAddress(origModule, "midiOutLongMsg");
    midiOutOpen_orig = GetProcAddress(origModule, "midiOutOpen");
    midiOutPrepareHeader_orig = GetProcAddress(origModule, "midiOutPrepareHeader");
    midiOutReset_orig = GetProcAddress(origModule, "midiOutReset");
    midiOutSetVolume_orig = GetProcAddress(origModule, "midiOutSetVolume");
    midiOutShortMsg_orig = GetProcAddress(origModule, "midiOutShortMsg");
    midiOutUnprepareHeader_orig = GetProcAddress(origModule, "midiOutUnprepareHeader");
    midiStreamClose_orig = GetProcAddress(origModule, "midiStreamClose");
    midiStreamOpen_orig = GetProcAddress(origModule, "midiStreamOpen");
    midiStreamOut_orig = GetProcAddress(origModule, "midiStreamOut");
    midiStreamPause_orig = GetProcAddress(origModule, "midiStreamPause");
    midiStreamPosition_orig = GetProcAddress(origModule, "midiStreamPosition");
    midiStreamProperty_orig = GetProcAddress(origModule, "midiStreamProperty");
    midiStreamRestart_orig = GetProcAddress(origModule, "midiStreamRestart");
    midiStreamStop_orig = GetProcAddress(origModule, "midiStreamStop");
    mixerClose_orig = GetProcAddress(origModule, "mixerClose");
    mixerGetControlDetailsA_orig = GetProcAddress(origModule, "mixerGetControlDetailsA");
    mixerGetControlDetailsW_orig = GetProcAddress(origModule, "mixerGetControlDetailsW");
    mixerGetDevCapsA_orig = GetProcAddress(origModule, "mixerGetDevCapsA");
    mixerGetDevCapsW_orig = GetProcAddress(origModule, "mixerGetDevCapsW");
    mixerGetID_orig = GetProcAddress(origModule, "mixerGetID");
    mixerGetLineControlsA_orig = GetProcAddress(origModule, "mixerGetLineControlsA");
    mixerGetLineControlsW_orig = GetProcAddress(origModule, "mixerGetLineControlsW");
    mixerGetLineInfoA_orig = GetProcAddress(origModule, "mixerGetLineInfoA");
    mixerGetLineInfoW_orig = GetProcAddress(origModule, "mixerGetLineInfoW");
    mixerGetNumDevs_orig = GetProcAddress(origModule, "mixerGetNumDevs");
    mixerMessage_orig = GetProcAddress(origModule, "mixerMessage");
    mixerOpen_orig = GetProcAddress(origModule, "mixerOpen");
    mixerSetControlDetails_orig = GetProcAddress(origModule, "mixerSetControlDetails");
    mmDrvInstall_orig = GetProcAddress(origModule, "mmDrvInstall");
    mmGetCurrentTask_orig = GetProcAddress(origModule, "mmGetCurrentTask");
    mmTaskBlock_orig = GetProcAddress(origModule, "mmTaskBlock");
    mmTaskCreate_orig = GetProcAddress(origModule, "mmTaskCreate");
    mmTaskSignal_orig = GetProcAddress(origModule, "mmTaskSignal");
    mmTaskYield_orig = GetProcAddress(origModule, "mmTaskYield");
    mmioAdvance_orig = GetProcAddress(origModule, "mmioAdvance");
    mmioAscend_orig = GetProcAddress(origModule, "mmioAscend");
    mmioClose_orig = GetProcAddress(origModule, "mmioClose");
    mmioCreateChunk_orig = GetProcAddress(origModule, "mmioCreateChunk");
    mmioDescend_orig = GetProcAddress(origModule, "mmioDescend");
    mmioFlush_orig = GetProcAddress(origModule, "mmioFlush");
    mmioGetInfo_orig = GetProcAddress(origModule, "mmioGetInfo");
    mmioInstallIOProcA_orig = GetProcAddress(origModule, "mmioInstallIOProcA");
    mmioInstallIOProcW_orig = GetProcAddress(origModule, "mmioInstallIOProcW");
    mmioOpenA_orig = GetProcAddress(origModule, "mmioOpenA");
    mmioOpenW_orig = GetProcAddress(origModule, "mmioOpenW");
    mmioRead_orig = GetProcAddress(origModule, "mmioRead");
    mmioRenameA_orig = GetProcAddress(origModule, "mmioRenameA");
    mmioRenameW_orig = GetProcAddress(origModule, "mmioRenameW");
    mmioSeek_orig = GetProcAddress(origModule, "mmioSeek");
    mmioSendMessage_orig = GetProcAddress(origModule, "mmioSendMessage");
    mmioSetBuffer_orig = GetProcAddress(origModule, "mmioSetBuffer");
    mmioSetInfo_orig = GetProcAddress(origModule, "mmioSetInfo");
    mmioStringToFOURCCA_orig = GetProcAddress(origModule, "mmioStringToFOURCCA");
    mmioStringToFOURCCW_orig = GetProcAddress(origModule, "mmioStringToFOURCCW");
    mmioWrite_orig = GetProcAddress(origModule, "mmioWrite");
    timeEndPeriod_orig = GetProcAddress(origModule, "timeEndPeriod");
    timeGetDevCaps_orig = GetProcAddress(origModule, "timeGetDevCaps");
    timeGetSystemTime_orig = GetProcAddress(origModule, "timeGetSystemTime");
    waveInAddBuffer_orig = GetProcAddress(origModule, "waveInAddBuffer");
    waveInClose_orig = GetProcAddress(origModule, "waveInClose");
    waveInGetDevCapsA_orig = GetProcAddress(origModule, "waveInGetDevCapsA");
    waveInGetDevCapsW_orig = GetProcAddress(origModule, "waveInGetDevCapsW");
    waveInGetErrorTextA_orig = GetProcAddress(origModule, "waveInGetErrorTextA");
    waveInGetErrorTextW_orig = GetProcAddress(origModule, "waveInGetErrorTextW");
    waveInGetID_orig = GetProcAddress(origModule, "waveInGetID");
    waveInGetNumDevs_orig = GetProcAddress(origModule, "waveInGetNumDevs");
    waveInGetPosition_orig = GetProcAddress(origModule, "waveInGetPosition");
    waveInMessage_orig = GetProcAddress(origModule, "waveInMessage");
    waveInOpen_orig = GetProcAddress(origModule, "waveInOpen");
    waveInPrepareHeader_orig = GetProcAddress(origModule, "waveInPrepareHeader");
    waveInReset_orig = GetProcAddress(origModule, "waveInReset");
    waveInStart_orig = GetProcAddress(origModule, "waveInStart");
    waveInStop_orig = GetProcAddress(origModule, "waveInStop");
    waveInUnprepareHeader_orig = GetProcAddress(origModule, "waveInUnprepareHeader");
    waveOutBreakLoop_orig = GetProcAddress(origModule, "waveOutBreakLoop");
    waveOutClose_orig = GetProcAddress(origModule, "waveOutClose");
    waveOutGetDevCapsA_orig = GetProcAddress(origModule, "waveOutGetDevCapsA");
    waveOutGetDevCapsW_orig = GetProcAddress(origModule, "waveOutGetDevCapsW");
    waveOutGetErrorTextA_orig = GetProcAddress(origModule, "waveOutGetErrorTextA");
    waveOutGetErrorTextW_orig = GetProcAddress(origModule, "waveOutGetErrorTextW");
    waveOutGetID_orig = GetProcAddress(origModule, "waveOutGetID");
    waveOutGetNumDevs_orig = GetProcAddress(origModule, "waveOutGetNumDevs");
    waveOutGetPitch_orig = GetProcAddress(origModule, "waveOutGetPitch");
    waveOutGetPlaybackRate_orig = GetProcAddress(origModule, "waveOutGetPlaybackRate");
    waveOutGetPosition_orig = GetProcAddress(origModule, "waveOutGetPosition");
    waveOutGetVolume_orig = GetProcAddress(origModule, "waveOutGetVolume");
    waveOutMessage_orig = GetProcAddress(origModule, "waveOutMessage");
    waveOutOpen_orig = GetProcAddress(origModule, "waveOutOpen");
    waveOutPause_orig = GetProcAddress(origModule, "waveOutPause");
    waveOutPrepareHeader_orig = GetProcAddress(origModule, "waveOutPrepareHeader");
    waveOutReset_orig = GetProcAddress(origModule, "waveOutReset");
    waveOutRestart_orig = GetProcAddress(origModule, "waveOutRestart");
    waveOutSetPitch_orig = GetProcAddress(origModule, "waveOutSetPitch");
    waveOutSetPlaybackRate_orig = GetProcAddress(origModule, "waveOutSetPlaybackRate");
    waveOutSetVolume_orig = GetProcAddress(origModule, "waveOutSetVolume");
    waveOutUnprepareHeader_orig = GetProcAddress(origModule, "waveOutUnprepareHeader");
    waveOutWrite_orig = GetProcAddress(origModule, "waveOutWrite");
    mciExecute_orig = GetProcAddress(origModule, "mciExecute");
    mciGetErrorStringA_orig = GetProcAddress(origModule, "mciGetErrorStringA");
    mciGetErrorStringW_orig = GetProcAddress(origModule, "mciGetErrorStringW");
    mciSendCommandA_orig = GetProcAddress(origModule, "mciSendCommandA");
    mciSendCommandW_orig = GetProcAddress(origModule, "mciSendCommandW");
    mciSendStringA_orig = GetProcAddress(origModule, "mciSendStringA");
    mciSendStringW_orig = GetProcAddress(origModule, "mciSendStringW");
    mciFreeCommandResource_orig = GetProcAddress(origModule, "mciFreeCommandResource");
    mciLoadCommandResource_orig = GetProcAddress(origModule, "mciLoadCommandResource");
    mciDriverNotify_orig = GetProcAddress(origModule, "mciDriverNotify");
    mciDriverYield_orig = GetProcAddress(origModule, "mciDriverYield");
    mciGetCreatorTask_orig = GetProcAddress(origModule, "mciGetCreatorTask");
    mciGetDeviceIDA_orig = GetProcAddress(origModule, "mciGetDeviceIDA");
    mciGetDeviceIDFromElementIDA_orig = GetProcAddress(origModule, "mciGetDeviceIDFromElementIDA");
    mciGetDeviceIDFromElementIDW_orig = GetProcAddress(origModule, "mciGetDeviceIDFromElementIDW");
    mciGetDeviceIDW_orig = GetProcAddress(origModule, "mciGetDeviceIDW");
    mciGetDriverData_orig = GetProcAddress(origModule, "mciGetDriverData");
    mciGetYieldProc_orig = GetProcAddress(origModule, "mciGetYieldProc");
    mciSetDriverData_orig = GetProcAddress(origModule, "mciSetDriverData");
    mciSetYieldProc_orig = GetProcAddress(origModule, "mciSetYieldProc");
    PlaySoundA_orig = GetProcAddress(origModule, "PlaySoundA");
    sndPlaySoundA_orig = GetProcAddress(origModule, "sndPlaySoundA");
    sndPlaySoundW_orig = GetProcAddress(origModule, "sndPlaySoundW");
    WOWAppExit_orig = GetProcAddress(origModule, "WOWAppExit");
    mmsystemGetVersion_orig = GetProcAddress(origModule, "mmsystemGetVersion");

    return true;
}

void on_detach()
{
    if (is_wrapping_nvngx)
        proxy_nvngx::on_detach();

    if (!origModule)
        return;

    FreeLibrary(origModule);
    origModule = nullptr;
}

};


```
