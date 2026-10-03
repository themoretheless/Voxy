# Named motion bones

ModelAnimation stores `root_motion_bone`, an exact authored glTF node name. An empty string retains the legacy `root_motion_joint` selection. Existing v1 scene documents remain readable through serde defaults.

ModelAsset retains original optional node names alongside its skeleton. Diagnostic skeleton names with `#<node index>` remain unchanged; selection never strips a suffix from an authored name. The resolver requires exactly one match in the accepted imported revision. Missing and duplicate names reject publication. Unnamed nodes still require numeric selection.

Scene validation resolves the name when an imported resource is available. Playback resolves again against the accepted model revision, including reimport. A settings update stages the selected joint on a cloned playback owner without resetting its clock. Import replacement retains the existing clock-reset policy. This is persistence across node ordering changes, not a rename or hierarchy retargeting system.

The generic inspector exposes Motion bone name and Motion bone index. Names take precedence when nonempty. Save/load and undo/redo use the existing component codec and document history.

Verification includes an actual glTF reimport fixture with inserted and reordered nodes, missing and ambiguous names (including the literal authored name `motion#1`), playback clock preservation, JSON round trip, and ordinary inspector history. The native Fox smoke selects `b_Hip_01` through the ordinary inspector and exercises two playback owners and Stop resource release.

Limits: no bone dropdown or hierarchy-path disambiguation yet. Renaming a bone requires an explicit authoring change. The name feature itself resolves a parent-local motion joint. Opt-in axis extraction and fixed-tick character application are now described in [fixed-animation-play](fixed-animation-play.md). This change does not establish CUDA support or broader physics completion.
