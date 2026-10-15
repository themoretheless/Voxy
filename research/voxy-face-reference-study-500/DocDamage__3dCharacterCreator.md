# Facial rendering source scan: DocDamage/3dCharacterCreator

Repository: https://github.com/DocDamage/3dCharacterCreator

Pinned source revision: 62a09e23c4c54c39ff07c9f0b9a7fe89ca051df1

Discovery topic: character creator 3D

Description: 3dCharacterCreator

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"skin": ["UE_5_7_1_Character_Creator_ALL_UI_UX_Screens/02_Additional_Screens/05_hair_grooming_skin.png"], "rig": ["UE_5_7_1_Character_Creator_ALL_UI_UX_Screens/01_Original_Concept_Screens/01_initial_character_creator_preview.png", "UE_5_7_1_Character_Creator_ALL_UI_UX_Screens/01_Original_Concept_Screens/02_character_creator_animation_workspace.png", "UE_5_7_1_Character_Creator_ALL_UI_UX_Screens/01_Original_Concept_Screens/03_ui_ux_sitemap_navigation_architecture.png", "UE_5_7_1_Character_Creator_ALL_UI_UX_Screens/01_Original_Concept_Screens/04_project_character_browser.png", "UE_5_7_1_Character_Creator_ALL_UI_UX_Screens/01_Original_Concept_Screens/05_body_face_advanced_creator.png", "UE_5_7_1_Character_Creator_ALL_UI_UX_Screens/01_Original_Concept_Screens/06_weapon_setup_and_ik.png", "UE_5_7_1_Character_Creator_ALL_UI_UX_Screens/01_Original_Concept_Screens/07_validation_and_export.png", "UE_5_7_1_Character_Creator_ALL_UI_UX_Screens/01_Original_Concept_Screens/README.txt", "UE_5_7_1_Character_Creator_ALL_UI_UX_Screens/02_Additional_Screens/13_skeleton_rig_socket_inspector.png"], "face": ["UE_5_7_1_Character_Creator_ALL_UI_UX_Screens/01_Original_Concept_Screens/05_body_face_advanced_creator.png", "UE_5_7_1_Character_Creator_ALL_UI_UX_Screens/02_Additional_Screens/03_face_sculpting.png"]}

## Source file: Source/threedcharacter.Target.cs

{
  "path": "Source/threedcharacter.Target.cs",
  "blob_sha": "6b0725154d4c8db4a02f0db096c7b07954ea3b94",
  "sha256": "d5b555f8521915bfa168f98080e3bef95718fbb6172a1866c68ffb5f4a6f288b",
  "lines": 12,
  "bytes": 360,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/DocDamage/3dCharacterCreator/blob/62a09e23c4c54c39ff07c9f0b9a7fe89ca051df1/Source/threedcharacter.Target.cs",
  "function_signature_candidates": []
}

```text

using UnrealBuildTool;

public class threedcharacterTarget : TargetRules
{
    public threedcharacterTarget(TargetInfo Target) : base(Target)
    {
        Type = TargetType.Game;
        DefaultBuildSettings = BuildSettingsVersion.V6;
        IncludeOrderVersion = EngineIncludeOrderVersion.Unreal5_7;
        ExtraModuleNames.Add("threedcharacter");
    }
}


```

## Source file: Source/threedcharacterEditor.Target.cs

{
  "path": "Source/threedcharacterEditor.Target.cs",
  "blob_sha": "7676c88b23fa84660b15b606d0a0ecc4c4982be0",
  "sha256": "1062b78fe1963211facf1a98dfb58a258d272eca0384e2a0549e38f64352350d",
  "lines": 12,
  "bytes": 374,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/DocDamage/3dCharacterCreator/blob/62a09e23c4c54c39ff07c9f0b9a7fe89ca051df1/Source/threedcharacterEditor.Target.cs",
  "function_signature_candidates": []
}

```text

using UnrealBuildTool;

public class threedcharacterEditorTarget : TargetRules
{
    public threedcharacterEditorTarget(TargetInfo Target) : base(Target)
    {
        Type = TargetType.Editor;
        DefaultBuildSettings = BuildSettingsVersion.V6;
        IncludeOrderVersion = EngineIncludeOrderVersion.Unreal5_7;
        ExtraModuleNames.Add("threedcharacter");
    }
}


```

## Source file: Source/threedcharacter/threedcharacter.h

{
  "path": "Source/threedcharacter/threedcharacter.h",
  "blob_sha": "fe82abe038ecee3f8bfa5dd7306707c14b18ee2b",
  "sha256": "0e65207f9e50b28203e68dede9e5749b8d16ab6ea3593af5e3891ed72c3844b2",
  "lines": 4,
  "bytes": 40,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/DocDamage/3dCharacterCreator/blob/62a09e23c4c54c39ff07c9f0b9a7fe89ca051df1/Source/threedcharacter/threedcharacter.h",
  "function_signature_candidates": []
}

```text

#pragma once

#include "CoreMinimal.h"



```

## Source file: Source/threedcharacter/threedcharacter.cpp

{
  "path": "Source/threedcharacter/threedcharacter.cpp",
  "blob_sha": "9ead78a3fefc683a69e9bbba6d857efd6dcb4c0d",
  "sha256": "409a98b08869b16a6feb47c04281ffae3900de7191ec64be302c02b643a2ec8d",
  "lines": 4,
  "bytes": 156,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/DocDamage/3dCharacterCreator/blob/62a09e23c4c54c39ff07c9f0b9a7fe89ca051df1/Source/threedcharacter/threedcharacter.cpp",
  "function_signature_candidates": []
}

```text

#include "threedcharacter.h"
#include "Modules/ModuleManager.h"

IMPLEMENT_PRIMARY_GAME_MODULE(FDefaultGameModuleImpl, threedcharacter, "threedcharacter");


```
