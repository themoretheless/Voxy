# Facial rendering source scan: prodzpod/SnekStudio-iFacialMocap

Repository: https://github.com/prodzpod/SnekStudio-iFacialMocap

Pinned source revision: 6e485e6183f1215a6765bda7d21cb4bdfba8d3fe

Discovery topic: facial mocap

Description: janky hack to make iFacialMocap work

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {}

## Source file: Parser.gd

{
  "path": "Parser.gd",
  "blob_sha": "21f36c45a56a03225dafb1c6578b67e049b16693",
  "sha256": "5a72576dfc00603cae9fb35441d90b3b8f12a4038b957b2472d0c8016b96d630",
  "lines": 46,
  "bytes": 1765,
  "selection_score": 0,
  "keyword_line_counts": {
    "blend_shape": 3,
    "rig": 1,
    "face": 1
  },
  "url": "https://github.com/prodzpod/SnekStudio-iFacialMocap/blob/6e485e6183f1215a6765bda7d21cb4bdfba8d3fe/Parser.gd",
  "function_signature_candidates": []
}

```text

extends Node
	
var blend_shape_data = {} # for storing all shape data in a name:value dict
var head_rotation = Vector3.ZERO
var head_position = Vector3.ZERO
func parse(message: String):
	for part in message.split("|"):
		part = part.strip_edges()
		if part.is_empty():
			continue
			
		# parse head rotation and position first
		if part.begins_with("=head#"):
			var head_data = part.trim_prefix("=head#").split(",")
			if head_data.size() >= 6:
				# some manual angles for clamping the head rotation I landed on
				head_rotation.x = clamp(float(head_data[0]), -75.0, 75.0)
				head_rotation.y = clamp(float(head_data[1]), -90.0, 90.0)
				head_rotation.z = clamp(float(head_data[2]), -90.0, 90.0)
				
				head_position.x = float(head_data[3])
				head_position.y = float(head_data[4])
				head_position.z = float(head_data[5])
				# note the head rotation seems to be 2x too much, so you may want to reduce it in the model script
				
		# then do the shape keys
		elif "-" in part and not part.begins_with("="):
			var tokens = part.split("-", true, 1)
			if tokens.size() == 2: # if valid size of data
				var blend_name = tokens[0]
				# name normalization - I think this was since meowface used different L/R formatting than ARKit
				if "_L" in blend_name:
					blend_name = blend_name.replace("_L", "Left")
				elif "_R" in blend_name:
					blend_name = blend_name.replace("_R", "Right")
				
				var blend_value = clamp(float(tokens[1]) / 100.0, -1.0, 1.0) # int to float value and clamp weird data received outside of -1 to 1
				
				blend_shape_data[blend_name] = blend_value # store all data in a name:value dict
	
	var ret = {
		"head_position": head_position,
		"head_rotation": head_rotation,
		"blend_shape_data": blend_shape_data
	}
	return ret


```

## Source file: Transformer.gd

{
  "path": "Transformer.gd",
  "blob_sha": "d549ad4f42c9d465caf8992f034d1a03f1f12011",
  "sha256": "a03b4f5a7274ab92059fff21ac6686a201055694aea1812b7288475bff259a5b",
  "lines": 9,
  "bytes": 435,
  "selection_score": 0,
  "keyword_line_counts": {
    "blendshape": 1,
    "blend_shape": 1,
    "rig": 1
  },
  "url": "https://github.com/prodzpod/SnekStudio-iFacialMocap/blob/6e485e6183f1215a6765bda7d21cb4bdfba8d3fe/Transformer.gd",
  "function_signature_candidates": []
}

```text

extends Node
static func transform(packet: Dictionary, previous: Dictionary):
	var ret = previous.duplicate(true)
	ret.head_origin = [packet.head_position.x, packet.head_position.y, packet.head_position.z]
	ret.blendshapes.assign(packet.blend_shape_data)
	# rotation: euler in degrees to quat
	var quat = Quaternion.from_euler((packet.head_rotation as Vector3) / 180 * PI)
	ret.head_quat = [quat.x, quat.y, quat.z, quat.w]
	return ret


```

## Source file: iFacialMocap.gd

{
  "path": "iFacialMocap.gd",
  "blob_sha": "9d74b4698861a019586fe6c432042a4ec48e95e7",
  "sha256": "2ccdef297cab3a5ac0321a7e0d8167259fbf8bea4b1477180f49e4d4898b4909",
  "lines": 65,
  "bytes": 3265,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 1,
    "blendshape": 1,
    "jaw": 1,
    "mouth": 1,
    "tongue": 1,
    "rig": 7,
    "face": 1
  },
  "url": "https://github.com/prodzpod/SnekStudio-iFacialMocap/blob/6e485e6183f1215a6765bda7d21cb4bdfba8d3fe/iFacialMocap.gd",
  "function_signature_candidates": []
}

```text

class_name IFacialMocap_Tracker extends Mod_Base
static var instance: IFacialMocap_Tracker
func _enter_tree() -> void: instance = self

var bind_port: int = 49983
var receiver_enabled : bool = false
var mediapipe_guy: Mod_MediaPipeController

func _ready():
	add_tracked_setting("bind_port", "Receiver port")
	add_tracked_setting("receiver_enabled", "Receiver enabled")
	var test_button : Button = Button.new()
	test_button.text = "Test Packet"
	get_settings_window().add_child(test_button)
	test_button.pressed.connect(test_packet)
	load_after({}, {})

func load_after(_settings_old: Dictionary, _settings_new: Dictionary):
	if get_parent().has_node("MediaPipeController"): 
		mediapipe_guy = $"../MediaPipeController"
		print_log(["Found Mediapipe Guy:", mediapipe_guy])
	start_server()

# shoutout xhiggy we love you xhiggy
var udp_socket: PacketPeerUDP
func start_server():
	if udp_socket and udp_socket.is_bound(): udp_socket.close()
	if not receiver_enabled: return
	udp_socket = PacketPeerUDP.new()
	var error = udp_socket.bind(bind_port)
	if error != OK:
		print_log("Failed to bind UDP socket on port %d: %s" % [bind_port, error_string(error)])
		return
	print_log("Face receiver bound to port %d" % bind_port)

func _physics_process(_delta: float) -> void:
	if udp_socket != null and udp_socket.get_available_packet_count() > 0:
		var packet
		while udp_socket.get_available_packet_count() > 0:
			packet = udp_socket.get_packet()
		on_message(_delta, packet.get_string_from_utf8())

func test_packet():
	on_message(1, "eyeLookIn_R-54|noseSneer_L-5|mouthPress_L-8|mouthSmile_R-4|mouthLowerDown_L-1|mouthSmile_L-1|eyeWide_L-26|mouthRollUpper-1|mouthPucker-3|browOuterUp_L-3|mouthDimple_R-3|mouthShrugLower-21|mouthLeft-0|eyeLookUp_R-0|mouthFunnel-1|mouthDimple_L-3|mouthUpperUp_R-2|noseSneer_R-6|eyeSquint_R-3|jawForward-2|mouthClose-2|mouthFrown_L-0|mouthShrugUpper-15|eyeSquint_L-3|cheekSquint_L-3|eyeLookDown_L-16|mouthLowerDown_R-1|eyeLookOut_R-0|jawLeft-0|mouthStretch_L-5|cheekPuff-3|eyeLookUp_L-0|eyeBlink_R-0|jawOpen-2|mouthRollLower-5|browInnerUp-4|browOuterUp_R-3|mouthFrown_R-0|mouthStretch_R-5|eyeLookIn_L-0|tongueOut-0|eyeBlink_L-0|browDown_L-0|eyeWide_R-26|eyeLookDown_R-16|mouthUpperUp_L-2|cheekSquint_R-3|mouthPress_R-8|browDown_R-0|jawRight-0|mouthRight-2|eyeLookOut_L-44|hapihapi-0|=head#-1.6704091,-7.3032465,2.886358,0.084120944,0.03458406,-0.4721467|rightEye#5.3555145,19.067966,1.8478252|leftEye#5.5607924,15.616646,1.5515244|")

func on_message(delta: float, message: String):
	#print_log(["Message Recieved:", message])
	var p = $Parser.parse(message)
	var previous_data = {
		"hand_left_origin": [0., 0., 0.],
		"hand_left_rotation": [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
		"hand_left_score": 1.,
		"hand_right_origin": [0.0, 0.0, 0.0],
		"hand_right_rotation": [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
		"hand_right_score": 1.,
		"head_origin": [0., 0., 0.],
		"head_quat": [0., 0., 0., 1.],
		"blendshapes": { "_neutral": 0. },
		"head_missing_time": 1.0,
		"hand_landmarks_left": [],
		"hand_landmarks_right": []
	}
	previous_data.merge(mediapipe_guy.last_parsed_data if mediapipe_guy else {})
	var t = $Transformer.transform(p, previous_data)
	if mediapipe_guy: mediapipe_guy._process_single_packet(delta, t)


```
