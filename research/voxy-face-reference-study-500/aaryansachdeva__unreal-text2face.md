# Facial rendering source scan: aaryansachdeva/unreal-text2face

Repository: https://github.com/aaryansachdeva/unreal-text2face

Pinned source revision: fd38209ba1ff2e03818b0bb6db783c79647706f4

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"face": ["unreal/Source/TextToFace/Private/AnimNode_TextToFace.cpp", "unreal/Source/TextToFace/Private/TextToFaceClient.cpp", "unreal/Source/TextToFace/Private/TextToFaceLiveLinkSource.cpp", "unreal/Source/TextToFace/Public/AnimNode_TextToFace.h", "unreal/Source/TextToFace/Public/TextToFaceClient.h", "unreal/Source/TextToFace/Public/TextToFaceLiveLinkSource.h", "unreal/Source/TextToFace/TextToFace.Build.cs"]}

## Source file: unreal/Source/TextToFace/TextToFace.Build.cs

{
  "path": "unreal/Source/TextToFace/TextToFace.Build.cs",
  "blob_sha": "b98a839a28a340883cdb7ed7023677e35fc0b4b7",
  "sha256": "b4ba7b0b1ec6a79949d376db31be2dc47503dcc1b660935d0f38b4b69149c0b4",
  "lines": 34,
  "bytes": 813,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 1
  },
  "url": "https://github.com/aaryansachdeva/unreal-text2face/blob/fd38209ba1ff2e03818b0bb6db783c79647706f4/unreal/Source/TextToFace/TextToFace.Build.cs",
  "function_signature_candidates": []
}

```text

using UnrealBuildTool;

// NOTE: Rename this class to match YOUR project's module name.
// E.g. if your project is "MyProject", rename to: public class MyProject : ModuleRules
public class MH_Express_Test : ModuleRules
{
	public MH_Express_Test(ReadOnlyTargetRules Target) : base(Target)
	{
		PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;

		PublicDependencyModuleNames.AddRange(new string[]
		{
			"Core",
			"CoreUObject",
			"Engine",
			"InputCore",
			"HTTP",
			"Json",
			"AnimGraphRuntime",
			"LiveLinkInterface",  // ILiveLinkSource, ILiveLinkClient, FLiveLinkBaseStaticData
		});

		PrivateDependencyModuleNames.AddRange(new string[] { });

		if (Target.Type == TargetType.Editor)
		{
			PrivateDependencyModuleNames.AddRange(new string[]
			{
				"AnimGraph",
				"BlueprintGraph",
			});
		}
	}
}


```

## Source file: unreal/Source/TextToFace/Public/TextToFaceClient.h

{
  "path": "unreal/Source/TextToFace/Public/TextToFaceClient.h",
  "blob_sha": "a9db8ab72eb964c31b8c83de755c701a4e34b7a2",
  "sha256": "5eb9712a738dc0bec2b364768db947bc34748fe21adfe1c9f82d2ea63cf99e79",
  "lines": 143,
  "bytes": 5142,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 1,
    "blendshape": 1,
    "rig": 1,
    "face": 38
  },
  "url": "https://github.com/aaryansachdeva/unreal-text2face/blob/fd38209ba1ff2e03818b0bb6db783c79647706f4/unreal/Source/TextToFace/Public/TextToFaceClient.h",
  "function_signature_candidates": [
    {
      "line": 48,
      "text": "bool IsValid() const { return bSuccess && NumFrames > 0; }"
    },
    {
      "line": 111,
      "text": "void GenerateExpression(const FString& Prompt, int32 Frames = 0, int32 Fps = 0);"
    },
    {
      "line": 114,
      "text": "void PlayLastBlock();"
    },
    {
      "line": 117,
      "text": "void Stop();"
    },
    {
      "line": 120,
      "text": "bool IsPlaying() const;"
    },
    {
      "line": 124,
      "text": "bool SampleCurrentCurves(TMap<FName, float>& OutCurves, float& OutProgress01) const;"
    },
    {
      "line": 127,
      "text": "virtual void BeginPlay() override;"
    },
    {
      "line": 128,
      "text": "virtual void EndPlay(const EEndPlayReason::Type EndPlayReason) override;"
    },
    {
      "line": 129,
      "text": "virtual void TickComponent(float DeltaTime, ELevelTick TickType, FActorComponentTickFunction* ThisTickFunction) override;"
    },
    {
      "line": 132,
      "text": "void HandleHttpResponse(FHttpRequestPtr Request, FHttpResponsePtr Response, bool bConnectedSuccessfully, FString Prompt);"
    },
    {
      "line": 133,
      "text": "bool ParseResponseJson(const FString& JsonString, FTextToFaceCurveBlock& OutBlock) const;"
    },
    {
      "line": 137,
      "text": "void InitLiveLinkSource();"
    },
    {
      "line": 138,
      "text": "void PushLiveLinkFrame();"
    }
  ]
}

```text

// Fill out your copyright notice in the Description page of Project Settings.

#pragma once

#include "CoreMinimal.h"
#include "Components/ActorComponent.h"
#include "Components/SkeletalMeshComponent.h"
#include "Interfaces/IHttpRequest.h"
#include "TextToFaceClient.generated.h"

class FTextToFaceLiveLinkSource;

/**
 * One generated expression block — metadata plus per-channel per-frame values.
 */
USTRUCT(BlueprintType)
struct MH_EXPRESS_TEST_API FTextToFaceCurveBlock
{
	GENERATED_BODY()

	UPROPERTY(BlueprintReadOnly, Category = "TextToFace")
	FString Prompt;

	UPROPERTY(BlueprintReadOnly, Category = "TextToFace")
	float Duration = 0.f;

	UPROPERTY(BlueprintReadOnly, Category = "TextToFace")
	int32 Fps = 60;

	UPROPERTY(BlueprintReadOnly, Category = "TextToFace")
	int32 NumFrames = 0;

	UPROPERTY(BlueprintReadOnly, Category = "TextToFace")
	float GenerationMs = 0.f;

	UPROPERTY(BlueprintReadOnly, Category = "TextToFace")
	bool bSuccess = false;

	// --- LiveLink path: raw ARKit channel data ---
	// Property names (61 ARKit channels: 52 blendshapes + 9 rotations)
	TArray<FName> ARKitPropertyNames;
	// Per-frame values: outer = frames, inner = 61 values per frame
	TArray<TArray<float>> ARKitFrames;

	// --- Legacy AnimNode path: ctrl_expressions_* mapped curves ---
	TMap<FName, TArray<float>> Curves;

	bool IsValid() const { return bSuccess && NumFrames > 0; }
};

DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(
	FTextToFaceGenerationComplete, const FTextToFaceCurveBlock&, CurveBlock);

/**
 * Drives MetaHuman facial animation from text prompts via a Python sidecar.
 *
 * Uses LiveLink to publish ARKit-format face data that the MetaHuman
 * consumes through its standard LiveLink face pipeline (ABP_MH_LiveLink).
 * This handles expressions, head rotation, AND eye gaze natively —
 * the same path the iPhone Live Link Face app uses.
 *
 * Setup:
 *   1. Add this component to a MetaHuman actor
 *   2. Copy ABP_MH_LiveLink to your project and set it as the Face's Anim Class
 *      (Animation Mode = Use Animation Blueprint)
 *   3. In ABP_MH_LiveLink, set the LiveLink Subject Name to match
 *      this component's LiveLinkSubjectName (default: "TextToFace")
 *   4. Call GenerateExpression("A person looks surprised")
 */
UCLASS(ClassGroup = (TextToFace), meta = (BlueprintSpawnableComponent))
class MH_EXPRESS_TEST_API UTextToFaceClient : public UActorComponent
{
	GENERATED_BODY()

public:
	UTextToFaceClient();

	// --- Config ------------------------------------------------------------

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "TextToFace|Server")
	FString ServerUrl = TEXT("http://127.0.0.1:8765");

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "TextToFace|Server",
		meta = (ClampMin = "1", ClampMax = "480"))
	int32 DefaultFrames = 240;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "TextToFace|Server",
		meta = (ClampMin = "1", ClampMax = "120"))
	int32 DefaultFps = 60;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "TextToFace|Server",
		meta = (ClampMin = "0.5", ClampMax = "30.0"))
	float RequestTimeoutSeconds = 10.f;

	/** When true, PlayLastBlock() is called automatically on a successful generation. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "TextToFace|Playback")
	bool bAutoPlayOnReceive = true;

	/** LiveLink subject name. Must match what the Face AnimBP listens for. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "TextToFace|LiveLink")
	FName LiveLinkSubjectName = FName(TEXT("TextToFace"));

	// --- Events ------------------------------------------------------------

	UPROPERTY(BlueprintAssignable, Category = "TextToFace")
	FTextToFaceGenerationComplete OnGenerationComplete;

	// --- API ---------------------------------------------------------------

	UFUNCTION(BlueprintCallable, Category = "TextToFace")
	void GenerateExpression(const FString& Prompt, int32 Frames = 0, int32 Fps = 0);

	UFUNCTION(BlueprintCallable, Category = "TextToFace")
	void PlayLastBlock();

	UFUNCTION(BlueprintCallable, Category = "TextToFace")
	void Stop();

	UFUNCTION(BlueprintCallable, BlueprintPure, Category = "TextToFace")
	bool IsPlaying() const;

	/** Legacy AnimNode path — sample mapped ctrl_expressions_* curves. */
	UFUNCTION(BlueprintCallable, Category = "TextToFace")
	bool SampleCurrentCurves(TMap<FName, float>& OutCurves, float& OutProgress01) const;

protected:
	virtual void BeginPlay() override;
	virtual void EndPlay(const EEndPlayReason::Type EndPlayReason) override;
	virtual void TickComponent(float DeltaTime, ELevelTick TickType, FActorComponentTickFunction* ThisTickFunction) override;

private:
	void HandleHttpResponse(FHttpRequestPtr Request, FHttpResponsePtr Response, bool bConnectedSuccessfully, FString Prompt);
	bool ParseResponseJson(const FString& JsonString, FTextToFaceCurveBlock& OutBlock) const;

	// LiveLink source (publishes ARKit face data each tick)
	TSharedPtr<FTextToFaceLiveLinkSource> LiveLinkSource;
	void InitLiveLinkSource();
	void PushLiveLinkFrame();

	mutable FCriticalSection BlockMutex;
	FTextToFaceCurveBlock CurrentBlock;
	double StartTimeSeconds = -1.0;
};


```

## Source file: unreal/Source/TextToFace/Private/TextToFaceClient.cpp

{
  "path": "unreal/Source/TextToFace/Private/TextToFaceClient.cpp",
  "blob_sha": "a82dea37406ca6684281a2360d3d6f726e680a71",
  "sha256": "e0fcc197fbc42e5ce23b0035fe8790f7d92eca2e2cb13830d3f49f9492c88f03",
  "lines": 359,
  "bytes": 10936,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 1,
    "face": 31,
    "lip": 1
  },
  "url": "https://github.com/aaryansachdeva/unreal-text2face/blob/fd38209ba1ff2e03818b0bb6db783c79647706f4/unreal/Source/TextToFace/Private/TextToFaceClient.cpp",
  "function_signature_candidates": []
}

```text

// Fill out your copyright notice in the Description page of Project Settings.

#include "TextToFaceClient.h"
#include "TextToFaceLiveLinkSource.h"

#include "HttpModule.h"
#include "Interfaces/IHttpResponse.h"
#include "Dom/JsonObject.h"
#include "Dom/JsonValue.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"
#include "Engine/World.h"
#include "Features/IModularFeatures.h"
#include "ILiveLinkClient.h"

DEFINE_LOG_CATEGORY_STATIC(LogTextToFace, Log, All);

UTextToFaceClient::UTextToFaceClient()
{
	PrimaryComponentTick.bCanEverTick = true;
}

// ---------------------------------------------------------------------------
// LiveLink source management
// ---------------------------------------------------------------------------

void UTextToFaceClient::InitLiveLinkSource()
{
	IModularFeatures& ModularFeatures = IModularFeatures::Get();
	if (!ModularFeatures.IsModularFeatureAvailable(ILiveLinkClient::ModularFeatureName))
	{
		UE_LOG(LogTextToFace, Warning, TEXT("LiveLink client not available — is the LiveLink plugin enabled?"));
		return;
	}

	LiveLinkSource = MakeShared<FTextToFaceLiveLinkSource>(LiveLinkSubjectName);

	ILiveLinkClient& Client = ModularFeatures.GetModularFeature<ILiveLinkClient>(
		ILiveLinkClient::ModularFeatureName);
	Client.AddSource(LiveLinkSource);

	UE_LOG(LogTextToFace, Log, TEXT("LiveLink source created, subject='%s'"),
		*LiveLinkSubjectName.ToString());
}

void UTextToFaceClient::PushLiveLinkFrame()
{
	if (!LiveLinkSource.IsValid() || !LiveLinkSource->IsReady()) return;

	FScopeLock Lock(&BlockMutex);
	if (!CurrentBlock.IsValid() || StartTimeSeconds < 0.0) return;
	if (CurrentBlock.ARKitFrames.Num() == 0) return;

	const UWorld* World = GetWorld();
	if (!World) return;

	const double Elapsed = World->GetTimeSeconds() - StartTimeSeconds;
	if (Elapsed < 0.0) return;

	const int32 LastFrame = CurrentBlock.NumFrames - 1;
	const double FrameF = Elapsed * (double)CurrentBlock.Fps;
	const int32 NumProps = CurrentBlock.ARKitPropertyNames.Num();

	TArray<float> Values;
	Values.SetNum(NumProps);

	if (FrameF >= (double)LastFrame)
	{
		// Clip finished — push zeros to clear additive layer, then stop
		StartTimeSeconds = -1.0;
		TArray<float> Zeros;
		Zeros.SetNumZeroed(NumProps);
		LiveLinkSource->PushFrameData(Zeros);
		return;
	}
	else
	{
		// Linearly interpolate between frames
		const int32 F0 = (int32)FrameF;
		const int32 F1 = F0 + 1;
		const float Alpha = (float)(FrameF - (double)F0);

		const TArray<float>& V0 = CurrentBlock.ARKitFrames[F0];
		const TArray<float>& V1 = CurrentBlock.ARKitFrames[F1];
		for (int32 i = 0; i < NumProps; ++i)
		{
			Values[i] = FMath::Lerp(V0[i], V1[i], Alpha);
		}
	}

	LiveLinkSource->PushFrameData(Values);
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

void UTextToFaceClient::BeginPlay()
{
	Super::BeginPlay();
	InitLiveLinkSource();
}

void UTextToFaceClient::EndPlay(const EEndPlayReason::Type EndPlayReason)
{
	if (LiveLinkSource.IsValid())
	{
		LiveLinkSource->RequestSourceShutdown();
		LiveLinkSource.Reset();
	}
	Super::EndPlay(EndPlayReason);
}

void UTextToFaceClient::TickComponent(float DeltaTime, ELevelTick TickType,
	FActorComponentTickFunction* ThisTickFunction)
{
	Super::TickComponent(DeltaTime, TickType, ThisTickFunction);
	PushLiveLinkFrame();
}

// ---------------------------------------------------------------------------
// Request
// ---------------------------------------------------------------------------

void UTextToFaceClient::GenerateExpression(const FString& Prompt, int32 Frames, int32 Fps)
{
	if (Prompt.IsEmpty())
	{
		UE_LOG(LogTextToFace, Warning, TEXT("GenerateExpression: empty prompt"));
		return;
	}

	const int32 UseFrames = (Frames > 0) ? Frames : DefaultFrames;
	const int32 UseFps    = (Fps    > 0) ? Fps    : DefaultFps;

	const TSharedRef<FJsonObject> Body = MakeShared<FJsonObject>();
	Body->SetStringField(TEXT("prompt"), Prompt);
	Body->SetNumberField(TEXT("frames"), UseFrames);
	Body->SetNumberField(TEXT("fps"),    UseFps);

	FString BodyString;
	const TSharedRef<TJsonWriter<>> Writer = TJsonWriterFactory<>::Create(&BodyString);
	FJsonSerializer::Serialize(Body, Writer);

	const TSharedRef<IHttpRequest> Req = FHttpModule::Get().CreateRequest();
	Req->SetVerb(TEXT("POST"));
	Req->SetURL(ServerUrl + TEXT("/generate"));
	Req->SetHeader(TEXT("Content-Type"), TEXT("application/json"));
	Req->SetContentAsString(BodyString);
	Req->SetTimeout(RequestTimeoutSeconds);

	Req->OnProcessRequestComplete().BindUObject(
		this, &UTextToFaceClient::HandleHttpResponse, Prompt);

	if (!Req->ProcessRequest())
	{
		UE_LOG(LogTextToFace, Warning, TEXT("ProcessRequest failed for \"%s\""), *Prompt);
		FTextToFaceCurveBlock Failed;
		Failed.Prompt = Prompt;
		OnGenerationComplete.Broadcast(Failed);
		return;
	}

	UE_LOG(LogTextToFace, Log, TEXT("sent: \"%s\" (%d frames @ %d fps)"), *Prompt, UseFrames, UseFps);
}

// ---------------------------------------------------------------------------
// Response
// ---------------------------------------------------------------------------

void UTextToFaceClient::HandleHttpResponse(
	FHttpRequestPtr Request, FHttpResponsePtr Response,
	bool bConnectedSuccessfully, FString Prompt)
{
	FTextToFaceCurveBlock Block;
	Block.Prompt = Prompt;

	if (!bConnectedSuccessfully || !Response.IsValid())
	{
		UE_LOG(LogTextToFace, Warning, TEXT("request failed for \"%s\""), *Prompt);
		OnGenerationComplete.Broadcast(Block);
		return;
	}
	if (Response->GetResponseCode() != 200)
	{
		UE_LOG(LogTextToFace, Warning, TEXT("server returned %d for \"%s\""),
			Response->GetResponseCode(), *Prompt);
		OnGenerationComplete.Broadcast(Block);
		return;
	}
	if (!ParseResponseJson(Response->GetContentAsString(), Block))
	{
		UE_LOG(LogTextToFace, Warning, TEXT("parse failed for \"%s\""), *Prompt);
		OnGenerationComplete.Broadcast(Block);
		return;
	}

	Block.bSuccess = true;

	{
		FScopeLock Lock(&BlockMutex);
		CurrentBlock = Block;
		if (bAutoPlayOnReceive)
		{
			if (const UWorld* World = GetWorld())
			{
				StartTimeSeconds = World->GetTimeSeconds();
			}
		}
	}

	// Push static data to LiveLink (property names, once per generation)
	if (LiveLinkSource.IsValid() && LiveLinkSource->IsReady() && Block.ARKitPropertyNames.Num() > 0)
	{
		LiveLinkSource->PushStaticData(Block.ARKitPropertyNames);
	}

	UE_LOG(LogTextToFace, Log,
		TEXT("received \"%s\": %d ARKit channels x %d frames, %.1fms"),
		*Prompt, Block.ARKitPropertyNames.Num(), Block.NumFrames, Block.GenerationMs);

	OnGenerationComplete.Broadcast(Block);
}

bool UTextToFaceClient::ParseResponseJson(
	const FString& JsonString, FTextToFaceCurveBlock& OutBlock) const
{
	TSharedPtr<FJsonObject> Root;
	const TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(JsonString);
	if (!FJsonSerializer::Deserialize(Reader, Root) || !Root.IsValid()) return false;

	double D = 0.0;
	Root->TryGetNumberField(TEXT("duration"),      D); OutBlock.Duration     = (float)D;
	Root->TryGetNumberField(TEXT("fps"),           D); OutBlock.Fps          = (int32)D;
	Root->TryGetNumberField(TEXT("n_frames"),      D); OutBlock.NumFrames    = (int32)D;
	Root->TryGetNumberField(TEXT("generation_ms"), D); OutBlock.GenerationMs = (float)D;

	// --- Parse arkit_raw (for LiveLink path) ---
	const TSharedPtr<FJsonObject>* ARKitObj = nullptr;
	if (Root->TryGetObjectField(TEXT("arkit_raw"), ARKitObj) && ARKitObj && ARKitObj->IsValid())
	{
		// Collect property names and per-property arrays
		TArray<FName> Names;
		TArray<TArray<float>> PerPropertyArrays;

		for (const auto& Pair : (*ARKitObj)->Values)
		{
			const TArray<TSharedPtr<FJsonValue>>* Arr = nullptr;
			if (!Pair.Value->TryGetArray(Arr) || !Arr) continue;

			TArray<float> Vals;
			Vals.Reserve(Arr->Num());
			for (const TSharedPtr<FJsonValue>& V : *Arr)
			{
				Vals.Add((float)V->AsNumber());
			}
			Names.Add(FName(*Pair.Key));
			PerPropertyArrays.Add(MoveTemp(Vals));
		}

		OutBlock.ARKitPropertyNames = MoveTemp(Names);

		// Transpose: per-property arrays → per-frame arrays
		const int32 NumProps = OutBlock.ARKitPropertyNames.Num();
		const int32 NumFrames = (NumProps > 0) ? PerPropertyArrays[0].Num() : 0;
		OutBlock.ARKitFrames.SetNum(NumFrames);
		for (int32 F = 0; F < NumFrames; ++F)
		{
			OutBlock.ARKitFrames[F].SetNum(NumProps);
			for (int32 P = 0; P < NumProps; ++P)
			{
				OutBlock.ARKitFrames[F][P] = PerPropertyArrays[P][F];
			}
		}
	}

	// --- Parse curves (for legacy AnimNode path) ---
	const TSharedPtr<FJsonObject>* CurvesObj = nullptr;
	if (Root->TryGetObjectField(TEXT("curves"), CurvesObj) && CurvesObj && CurvesObj->IsValid())
	{
		OutBlock.Curves.Empty();
		for (const auto& Pair : (*CurvesObj)->Values)
		{
			const TArray<TSharedPtr<FJsonValue>>* Arr = nullptr;
			if (!Pair.Value->TryGetArray(Arr) || !Arr) continue;
			TArray<float> Vals;
			Vals.Reserve(Arr->Num());
			for (const TSharedPtr<FJsonValue>& V : *Arr)
				Vals.Add((float)V->AsNumber());
			OutBlock.Curves.Add(FName(*Pair.Key), MoveTemp(Vals));
		}
	}

	return OutBlock.NumFrames > 0;
}

// ---------------------------------------------------------------------------
// Playback controls
// ---------------------------------------------------------------------------

void UTextToFaceClient::PlayLastBlock()
{
	FScopeLock Lock(&BlockMutex);
	if (!CurrentBlock.IsValid()) return;
	if (const UWorld* World = GetWorld())
		StartTimeSeconds = World->GetTimeSeconds();
}

void UTextToFaceClient::Stop()
{
	FScopeLock Lock(&BlockMutex);
	StartTimeSeconds = -1.0;
}

bool UTextToFaceClient::IsPlaying() const
{
	FScopeLock Lock(&BlockMutex);
	return CurrentBlock.IsValid() && StartTimeSeconds >= 0.0;
}

bool UTextToFaceClient::SampleCurrentCurves(
	TMap<FName, float>& OutCurves, float& OutProgress01) const
{
	OutCurves.Reset();
	OutProgress01 = 0.f;

	FScopeLock Lock(&BlockMutex);
	if (!CurrentBlock.IsValid() || StartTimeSeconds < 0.0) return false;
	if (CurrentBlock.Curves.Num() == 0) return false;

	const UWorld* World = GetWorld();
	if (!World) return false;

	const double Elapsed = World->GetTimeSeconds() - StartTimeSeconds;
	if (Elapsed < 0.0) return false;

	const int32 LastFrame = CurrentBlock.NumFrames - 1;
	const double FrameF = Elapsed * (double)CurrentBlock.Fps;

	OutCurves.Reserve(CurrentBlock.Curves.Num());

	if (FrameF >= (double)LastFrame)
	{
		for (const auto& Pair : CurrentBlock.Curves)
			OutCurves.Add(Pair.Key, Pair.Value[LastFrame]);
		OutProgress01 = 1.f;
		return true;
	}

	const int32 F0 = (int32)FrameF;
	const int32 F1 = F0 + 1;
	const float Alpha = (float)(FrameF - (double)F0);

	for (const auto& Pair : CurrentBlock.Curves)
		OutCurves.Add(Pair.Key, FMath::Lerp(Pair.Value[F0], Pair.Value[F1], Alpha));

	OutProgress01 = CurrentBlock.Duration > 0.f ? (float)(Elapsed / CurrentBlock.Duration) : 0.f;
	return true;
}


```

## Source file: unreal/Source/TextToFace/Public/AnimNode_TextToFace.h

{
  "path": "unreal/Source/TextToFace/Public/AnimNode_TextToFace.h",
  "blob_sha": "67cb27132e53d907b312a4cde00af731bcbe4496",
  "sha256": "8987c07766457a66622630e6c0001fe02bddc9e6bbbb2a0e84195660279fe632",
  "lines": 55,
  "bytes": 1575,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 1,
    "face": 7
  },
  "url": "https://github.com/aaryansachdeva/unreal-text2face/blob/fd38209ba1ff2e03818b0bb6db783c79647706f4/unreal/Source/TextToFace/Public/AnimNode_TextToFace.h",
  "function_signature_candidates": [
    {
      "line": 26,
      "text": "virtual void Initialize_AnyThread(const FAnimationInitializeContext& Context) override;"
    },
    {
      "line": 27,
      "text": "virtual void CacheBones_AnyThread(const FAnimationCacheBonesContext& Context) override;"
    },
    {
      "line": 28,
      "text": "virtual void Update_AnyThread(const FAnimationUpdateContext& Context) override;"
    },
    {
      "line": 29,
      "text": "virtual void Evaluate_AnyThread(FPoseContext& Output) override;"
    }
  ]
}

```text

// Custom AnimNode that reads ctrl_expressions_* curves from a
// UTextToFaceClient component on the owning actor and writes them
// into the output pose each frame.

#pragma once

#include "CoreMinimal.h"
#include "Animation/AnimNodeBase.h"

#if WITH_EDITORONLY_DATA
#include "AnimGraphNode_Base.h"
#endif

#include "AnimNode_TextToFace.generated.h"

class UTextToFaceClient;

USTRUCT(BlueprintInternalUseOnly)
struct MH_EXPRESS_TEST_API FAnimNode_TextToFace : public FAnimNode_Base
{
	GENERATED_BODY()

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Links")
	FPoseLink SourcePose;

	virtual void Initialize_AnyThread(const FAnimationInitializeContext& Context) override;
	virtual void CacheBones_AnyThread(const FAnimationCacheBonesContext& Context) override;
	virtual void Update_AnyThread(const FAnimationUpdateContext& Context) override;
	virtual void Evaluate_AnyThread(FPoseContext& Output) override;

private:
	TWeakObjectPtr<UTextToFaceClient> CachedClient;
	TMap<FName, float> SampledCurves;
	float SampledProgress = 0.f;

	// Cached bone index for direct head bone rotation (bypasses ControlRig)
	FCompactPoseBoneIndex HeadBoneIndex = FCompactPoseBoneIndex(INDEX_NONE);
};

#if WITH_EDITORONLY_DATA

UCLASS()
class UAnimGraphNode_TextToFace : public UAnimGraphNode_Base
{
	GENERATED_BODY()

	UPROPERTY(EditAnywhere, Category = "Settings")
	FAnimNode_TextToFace Node;

	virtual FText GetNodeTitle(ENodeTitleType::Type TitleType) const override;
	virtual FText GetTooltipText() const override;
	virtual FString GetNodeCategory() const override;
};

#endif


```
