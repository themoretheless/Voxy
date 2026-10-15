# Facial rendering source scan: Lexpartizan/Go_MakeHuman_dot

Repository: https://github.com/Lexpartizan/Go_MakeHuman_dot

Pinned source revision: 40fa86fa70defcc23490835f8801e4499104d830

Discovery topic: MakeHuman

Description: ingame character generator for godot (based on the project Makehuman)

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"eye": ["Character Generator/.import/brown_eye.png-f57623220a2da8e76b791b84b5487d10.etc2.stex", "Character Generator/.import/brown_eye.png-f57623220a2da8e76b791b84b5487d10.md5", "Character Generator/.import/brown_eye.png-f57623220a2da8e76b791b84b5487d10.s3tc.stex", "Character Generator/.import/brown_eye.png-f57623220a2da8e76b791b84b5487d10.stex", "Character Generator/char_edit/meshs/eyes.mesh", "Character Generator/char_edit/meshs/eyes.tscn", "Character Generator/char_edit/meshs/mats/brown_eye.png", "Character Generator/char_edit/meshs/mats/brown_eye.png.import", "Character Generator/char_edit/meshs/mats/eyes.tres", "Character Generator/char_edit/meshs4gen/eyes.mesh"], "skin": ["Character Generator/.import/young_lightskinned_female_diffuse2.jpg-b4a30511445a47885fd35a789b782ae8.etc2.stex", "Character Generator/.import/young_lightskinned_female_diffuse2.jpg-b4a30511445a47885fd35a789b782ae8.md5", "Character Generator/.import/young_lightskinned_female_diffuse2.jpg-b4a30511445a47885fd35a789b782ae8.s3tc.stex", "Character Generator/.import/young_lightskinned_female_diffuse2.jpg-dc57e84e8e7b85207e88889b1be77411.etc2.stex", "Character Generator/.import/young_lightskinned_female_diffuse2.jpg-dc57e84e8e7b85207e88889b1be77411.md5", "Character Generator/.import/young_lightskinned_female_diffuse2.jpg-dc57e84e8e7b85207e88889b1be77411.s3tc.stex", "Character Generator/char_edit/meshs/mats/young_lightskinned_female_diffuse2.jpg", "Character Generator/char_edit/meshs/mats/young_lightskinned_female_diffuse2.jpg.import", "Character Generator/char_edit/meshs/skins/body.tres", "Character Generator/char_edit/meshs/skins/dress_mini_03.tres", "Character Generator/char_edit/meshs/skins/male_jeans_01.tres", "Character Generator/char_edit/meshs/skins/male_shirt_01.tres"], "blendshape": ["how create blendshapes for clothes.mp4"], "teeth": ["Character Generator/.import/teeth.png-910f38aa6042295e4ae74ae613911133.md5", "Character Generator/.import/teeth.png-910f38aa6042295e4ae74ae613911133.stex", "Character Generator/char_edit/meshs/mats/teeth.png", "Character Generator/char_edit/meshs/mats/teeth.png.import", "Character Generator/char_edit/meshs4gen/teeth.mesh"], "tongue": ["Character Generator/char_edit/meshs4gen/tongue.mesh"], "rig": ["simple IK rig.blend"]}

## Source file: Character Generator/char_edit_global.gd

{
  "path": "Character Generator/char_edit_global.gd",
  "blob_sha": "94046087fa0c81389399c580e3341f2792fcc39e",
  "sha256": "c896b0b2bf0b32e3255cf462be693445473c9189a8835167853a336535ba2e74",
  "lines": 10,
  "bytes": 178,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/Lexpartizan/Go_MakeHuman_dot/blob/40fa86fa70defcc23490835f8801e4499104d830/Character%20Generator/char_edit_global.gd",
  "function_signature_candidates": []
}

```text

extends Node

var meshs_shapes

func _ready():
	var file = File.new()
	file.open_compressed("res://char_edit/shapes.dat",File.READ)
	meshs_shapes = file.get_var()
	file.close()



```
