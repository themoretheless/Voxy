# Facial rendering source scan: RogTV/facial_anim_fix

Repository: https://github.com/RogTV/facial_anim_fix

Pinned source revision: a8fa452e3c5c03abcdaf184bafab71a9e45cd6c3

Discovery topic: facial animation

Description: Fix facial animation, fivem 2944 gamebuild.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: rog_voicefix/fxmanifest.lua

{
  "path": "rog_voicefix/fxmanifest.lua",
  "blob_sha": "8211b5b707d2889e20f3676a154080952d272e29",
  "sha256": "cc959cd9ab6cd54bd8c780c4d988506bb44c794db5591c3116548b133d61f817",
  "lines": 7,
  "bytes": 84,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/RogTV/facial_anim_fix/blob/a8fa452e3c5c03abcdaf184bafab71a9e45cd6c3/rog_voicefix/fxmanifest.lua",
  "function_signature_candidates": []
}

```text

fx_version 'cerulean'
games {'gta5'}

client_scripts{
    'client/*.lua',
}



```

## Source file: rog_voicefix/client/client.lua

{
  "path": "rog_voicefix/client/client.lua",
  "blob_sha": "73fc5600fbc914d84bcb1487edd0e2f95f4440a4",
  "sha256": "d3a0a6a7120f6969f571bdbd01208240e6f746dd30f5bc6739097b02c30814c5",
  "lines": 27,
  "bytes": 1078,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 1
  },
  "url": "https://github.com/RogTV/facial_anim_fix/blob/a8fa452e3c5c03abcdaf184bafab71a9e45cd6c3/rog_voicefix/client/client.lua",
  "function_signature_candidates": []
}

```text

staparlando = false


RegisterNetEvent("SaltyChat_TalkStateChanged")
AddEventHandler("SaltyChat_TalkStateChanged", function(isTalking)   -- Change this if you don't use saltychat on your server. You need to detect when a player is talking, every voice system has the proper exports.
    staparlando = isTalking
end)

--RegisterNetEvent("rog:userTalking")
--AddEventHandler("rog:userTalking", function(lastTalkingStatus)   -- If You're using pma-voice use this instead of "SaltyChat_TalkStateChanged". You need to add TriggerEvent("rog:userTalking",lastTalkingStatus) in pma-voice/client/init/proximity.lua/129 and it should work 
    --staparlando = lastTalkingStatus
--end)


Citizen.CreateThread(function()
  RequestAnimDict('facials@gen_male@variations@normal')
  RequestAnimDict('mp_facial')
 
  while true do
    Citizen.Wait(1000)
    if staparlando then
        SetFacialIdleAnimOverride(PlayerPedId(), "mood_talking_1", 0)
    elseif not staparlando then
        SetFacialIdleAnimOverride(PlayerPedId(), "mood_normal_1", 0)
    end
  end
end)


```
