# Facial rendering source scan: lyingtiger88/BDFR_CharacterBridge

Repository: https://github.com/lyingtiger88/BDFR_CharacterBridge

Pinned source revision: 76fdad8d6a41921dab467e815ceb04740cb7cc77

Discovery topic: facial retargeting

Description: Next-generation Unreal Engine 5.8 character bridge for Daz, Blender, MetaHuman and custom characters, with skeleton conversion, animation transfer, facial expressions, retargeting and extensible digital-human pipelines.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"rig": ["Docs/IKRigRetargeterGeneration.md"]}

## Source file: Plugins/BDFR_CharacterBridge/Source/BDFR_CharacterBridge/BDFR_CharacterBridge.Build.cs

{
  "path": "Plugins/BDFR_CharacterBridge/Source/BDFR_CharacterBridge/BDFR_CharacterBridge.Build.cs",
  "blob_sha": "38c8ed7581193b9a13600ae92fca60c17d318b33",
  "sha256": "065a114b071ff5f3c3dd0878af2d52c38571b6effad1639d6f0362b0abfc70ba",
  "lines": 25,
  "bytes": 546,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/lyingtiger88/BDFR_CharacterBridge/blob/76fdad8d6a41921dab467e815ceb04740cb7cc77/Plugins/BDFR_CharacterBridge/Source/BDFR_CharacterBridge/BDFR_CharacterBridge.Build.cs",
  "function_signature_candidates": []
}

```text

using UnrealBuildTool;

public class BDFR_CharacterBridge : ModuleRules
{
    public BDFR_CharacterBridge(ReadOnlyTargetRules Target) : base(Target)
    {
        PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;

        PublicDependencyModuleNames.AddRange(
            new string[]
            {
                "Core",
                "CoreUObject",
                "Engine"
            }
        );

        PrivateDependencyModuleNames.AddRange(
            new string[]
            {
                "Json"
            }
        );
    }
}


```

## Source file: Plugins/BDFR_CharacterBridge/Source/BDFR_CharacterBridge/Public/BDFR_CharacterBridge.h

{
  "path": "Plugins/BDFR_CharacterBridge/Source/BDFR_CharacterBridge/Public/BDFR_CharacterBridge.h",
  "blob_sha": "1f73f8bca32ce9861f3efc96c5ac016ad3d20f52",
  "sha256": "e17bb7f941ebf5343c41c69fe8130df6a33c67ff9f1b98b1c4dbcbaa2a204891",
  "lines": 10,
  "bytes": 216,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 1
  },
  "url": "https://github.com/lyingtiger88/BDFR_CharacterBridge/blob/76fdad8d6a41921dab467e815ceb04740cb7cc77/Plugins/BDFR_CharacterBridge/Source/BDFR_CharacterBridge/Public/BDFR_CharacterBridge.h",
  "function_signature_candidates": [
    {
      "line": 8,
      "text": "virtual void StartupModule() override;"
    },
    {
      "line": 9,
      "text": "virtual void ShutdownModule() override;"
    }
  ]
}

```text

#pragma once

#include "Modules/ModuleManager.h"

class FBDFR_CharacterBridgeModule final : public IModuleInterface
{
public:
    virtual void StartupModule() override;
    virtual void ShutdownModule() override;
};


```

## Source file: Plugins/BDFR_CharacterBridge/Source/BDFR_CharacterBridge/Public/Morph/BDFRMorphTypes.h

{
  "path": "Plugins/BDFR_CharacterBridge/Source/BDFR_CharacterBridge/Public/Morph/BDFRMorphTypes.h",
  "blob_sha": "5fcf9a0cfa42879c64a7561cdbcb44e5978704df",
  "sha256": "fa4dd3c39adbbaa0bb3190f92b10868af912dcea9897f321b1ec8355a72ff2d8",
  "lines": 102,
  "bytes": 2726,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/lyingtiger88/BDFR_CharacterBridge/blob/76fdad8d6a41921dab467e815ceb04740cb7cc77/Plugins/BDFR_CharacterBridge/Source/BDFR_CharacterBridge/Public/Morph/BDFRMorphTypes.h",
  "function_signature_candidates": [
    {
      "line": 98,
      "text": "bool IsUsable() const"
    }
  ]
}

```text

#pragma once

#include "CoreMinimal.h"
#include "BDFRMorphTypes.generated.h"

UENUM(BlueprintType)
enum class EBDFRMorphSemantic : uint8
{
    Unknown,
    Body,
    Head,
    Expression,
    Viseme,
    Corrective,
    Control
};

USTRUCT(BlueprintType)
struct FBDFRMorphKey
{
    GENERATED_BODY()

    UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "BDFR|Morph")
    float Angle = 0.0f;

    UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "BDFR|Morph")
    float Value = 0.0f;
};

USTRUCT(BlueprintType)
struct FBDFRJointDrivenMorphLink
{
    GENERATED_BODY()

    UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "BDFR|Morph")
    FName Bone = NAME_None;

    UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "BDFR|Morph")
    FName Morph = NAME_None;

    UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "BDFR|Morph")
    FName Axis = NAME_None;

    UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "BDFR|Morph")
    float Scalar = 0.0f;

    UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "BDFR|Morph")
    float Alpha = 1.0f;

    UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "BDFR|Morph")
    TArray<FBDFRMorphKey> Keys;
};

USTRUCT(BlueprintType)
struct FBDFRMorphDescriptor
{
    GENERATED_BODY()

    UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "BDFR|Morph")
    FName SourceInternalName = NAME_None;

    UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "BDFR|Morph")
    FString SourceLabel;

    UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "BDFR|Morph")
    FName ImportedMorphName = NAME_None;

    UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "BDFR|Morph")
    FName TargetMorphName = NAME_None;

    UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "BDFR|Morph")
    EBDFRMorphSemantic Semantic = EBDFRMorphSemantic::Unknown;

    UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "BDFR|Morph")
    bool bJointDriven = false;

    UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "BDFR|Morph")
    bool bFoundOnImportedMesh = false;
};

USTRUCT(BlueprintType)
struct FBDFRMorphTransferPlan
{
    GENERATED_BODY()

    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "BDFR|Morph")
    TArray<FBDFRMorphDescriptor> Morphs;

    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "BDFR|Morph")
    TArray<FBDFRJointDrivenMorphLink> JointDrivenLinks;

    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "BDFR|Morph")
    TArray<FName> MissingRequestedMorphs;

    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "BDFR|Morph")
    TArray<FName> UnlistedImportedMorphs;

    bool IsUsable() const
    {
        return Morphs.Num() > 0 && MissingRequestedMorphs.Num() == 0;
    }
};


```

## Source file: Plugins/BDFR_CharacterBridge/Source/BDFR_CharacterBridge/Private/BDFR_CharacterBridge.cpp

{
  "path": "Plugins/BDFR_CharacterBridge/Source/BDFR_CharacterBridge/Private/BDFR_CharacterBridge.cpp",
  "blob_sha": "4f478cd2c0b5be7beec421f3a333d9cb79592a09",
  "sha256": "e8cb879d497afd92c49f86558d05224cffc9407efe5b03548b45ddc095beccab",
  "lines": 15,
  "bytes": 297,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/lyingtiger88/BDFR_CharacterBridge/blob/76fdad8d6a41921dab467e815ceb04740cb7cc77/Plugins/BDFR_CharacterBridge/Source/BDFR_CharacterBridge/Private/BDFR_CharacterBridge.cpp",
  "function_signature_candidates": []
}

```text

#include "BDFR_CharacterBridge.h"

#define LOCTEXT_NAMESPACE "FBDFR_CharacterBridgeModule"

void FBDFR_CharacterBridgeModule::StartupModule()
{
}

void FBDFR_CharacterBridgeModule::ShutdownModule()
{
}

#undef LOCTEXT_NAMESPACE

IMPLEMENT_MODULE(FBDFR_CharacterBridgeModule, BDFR_CharacterBridge)


```
