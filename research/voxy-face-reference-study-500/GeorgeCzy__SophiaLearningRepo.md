# Facial rendering source scan: GeorgeCzy/SophiaLearningRepo

Repository: https://github.com/GeorgeCzy/SophiaLearningRepo

Pinned source revision: 81e52a0be00a9a425fec780acfc5ea4236eb6325

Discovery topic: facial retargeting

Description: It's a mixed repo for learning Sophia, including learning how the facial control works, how to do arm retargeting.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"face": ["face_demo.py"]}

## Source file: face_demo.py

{
  "path": "face_demo.py",
  "blob_sha": "bc1d64a56ea56e7bb89bc55cb7c30ac5d8c71154",
  "sha256": "bbc111ea775fa9d73f4d11bab8d97e126c6973a5592207d0d319cd95b91bf29c",
  "lines": 7,
  "bytes": 185,
  "selection_score": 13,
  "keyword_line_counts": {},
  "url": "https://github.com/GeorgeCzy/SophiaLearningRepo/blob/81e52a0be00a9a425fec780acfc5ea4236eb6325/face_demo.py",
  "function_signature_candidates": []
}

```text

import rospy
from sophia_dev.sophia_control.llf_control import ActuatorControl

if __name__ == "__main__":
    rospy.init_node("actuator_control")
    ActuatorControl()
    rospy.spin()

```

## Source file: temp.py

{
  "path": "temp.py",
  "blob_sha": "67ff73fa4168df2af532658ae712c627ed169bed",
  "sha256": "0fd8203ee9dcf28e8b4141b34801ddf0cf1e069f24594df6d0519fc516ec0e20",
  "lines": 42,
  "bytes": 1563,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 15
  },
  "url": "https://github.com/GeorgeCzy/SophiaLearningRepo/blob/81e52a0be00a9a425fec780acfc5ea4236eb6325/temp.py",
  "function_signature_candidates": []
}

```text

LIMITS = {
    # =========================
    # RIGHT ARM
    # =========================
    "RightShoulderPitch": (deg(-145), deg(35)),
    "RightShoulderRoll":  (deg(-4),   deg(101)),
    "RightShoulderYaw":   (deg(-66),  deg(83)),
    "RightElbowPitch":    (deg(-127), deg(119)),
    "RightElbowYaw":      (deg(-123), deg(123)),

    # =========================
    # RIGHT HAND
    # =========================
    "RightWristPitch":   (deg(-35),  deg(35)),
    "RightWristRoll":    (deg(-35),  deg(35)),
    "RightThumbRoll":    (deg(-31),  deg(22)),
    "RightThumbFinger":  (deg(-75),  deg(44)),
    "RightIndexFinger":  (deg(-123), deg(18)),
    "RightMiddleFinger": (deg(-18),  deg(132)),
    "RightRingFinger":   (deg(-18),  deg(136)),
    "RightPinkyFinger":  (deg(-75),  deg(4)),
    # =========================
    # LEFT ARM
    # =========================
    "LeftShoulderPitch": (deg(-35),  deg(145)),
    "LeftShoulderRoll":  (deg(-101), deg(4)),
    "LeftShoulderYaw":   (deg(-83),  deg(66)),
    "LeftElbowPitch":    (deg(-119), deg(127)),
    "LeftElbowYaw":      (deg(-123), deg(123)),

    # =========================
    # LEFT HAND
    # =========================
    "LeftWristPitch":   (deg(-35),  deg(35)),
    "LeftWristRoll":    (deg(-35),  deg(35)),
    "LeftThumbRoll":    (deg(-31),  deg(22)),
    "LeftThumbFinger":  (deg(-44),  deg(75)),
    "LeftIndexFinger":  (deg(-18),  deg(123)),
    "LeftMiddleFinger": (deg(-18),  deg(132)),
    "LeftRingFinger":   (deg(-18),  deg(136)),
    "LeftPinkyFinger":  (deg(-4),   deg(75)),
}


```

## Source file: smpl_visualizer.py

{
  "path": "smpl_visualizer.py",
  "blob_sha": "1d0d491ef99bd3ddbebf7e182f31e6a740188e96",
  "sha256": "aed47751cd7d2d7585e14cd7c0a2af34973189e0362fb7cb4d67a08c334a8e9d",
  "lines": 780,
  "bytes": 33706,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 2,
    "skin": 1,
    "lbs": 1,
    "rig": 21,
    "face": 4
  },
  "url": "https://github.com/GeorgeCzy/SophiaLearningRepo/blob/81e52a0be00a9a425fec780acfc5ea4236eb6325/smpl_visualizer.py",
  "function_signature_candidates": [
    {
      "line": 41,
      "text": "def to_axisangle(val: tuple[float, ...] ,index = 0) -> np.ndarray:"
    },
    {
      "line": 110,
      "text": "def __init__(self, model_path: Path) -> None:"
    },
    {
      "line": 125,
      "text": "def get_outputs(self, betas: np.ndarray, joint_rotmats: np.ndarray) -> SmplOutputs:"
    },
    {
      "line": 159,
      "text": "def deg(x):"
    },
    {
      "line": 176,
      "text": "def main(model_path: Path) -> None:"
    },
    {
      "line": 287,
      "text": "def make_gui_elements("
    },
    {
      "line": 302,
      "text": "def set_changed(_):"
    },
    {
      "line": 317,
      "text": "def _(_):"
    },
    {
      "line": 330,
      "text": "def _(_):"
    },
    {
      "line": 335,
      "text": "def _(_):"
    },
    {
      "line": 358,
      "text": "def _(_):"
    },
    {
      "line": 363,
      "text": "def _(_):"
    },
    {
      "line": 516,
      "text": "def _sync_left_thumb(_):"
    },
    {
      "line": 544,
      "text": "def _sync_right_thumb(_):"
    },
    {
      "line": 572,
      "text": "def set_callback_in_closure(i: int) -> None:"
    },
    {
      "line": 574,
      "text": "def _(_):"
    },
    {
      "line": 725,
      "text": "def set_callback_in_closure(i: int) -> None:"
    },
    {
      "line": 727,
      "text": "def _(_) -> None:"
    },
    {
      "line": 771,
      "text": "def getindexandvalue(i, value):"
    }
  ]
}

```text

"""SMPL model visualizer

Visualizer for SMPL human body models. Requires a .npz model file.

See here for download instructions:
    https://github.com/vchoutas/smplx?tab=readme-ov-file#downloading-the-model
"""

from __future__ import annotations

import time
from dataclasses import dataclass
from pathlib import Path

import numpy as np
import trimesh
import tyro

import viser
import viser.transforms as tf

import Sophia_control
from scipy.spatial.transform import Rotation as R

import math

####################################################################################################
# ‼️  UI-RELATED LOGIC OVERVIEW                                                                    #
# The visualizer has two layers of UI:                                                             #
#   1. *Viewer widgets* shown in the browser (tabs, sliders, colour picker, check-boxes, etc.).     #
#      They are created **inside the make_gui_elements(..) function**.                             #
#   2. *Scene gizmos* (TransformControls) that appear as draggable handles on top of joints.        #
#      They are also spawned in make_gui_elements(..) and synchronised through callbacks.           #
#                                                                                                  #
#  ────────────────────────────────────────────────────────────────────────────────────────────────  #
#  Where to look:                                                                                  #
#    • make_gui_elements(..)  ← builds everything UI-related.                                       #
#    • main(..)                 ← calls make_gui_elements and drives updates every frame.           #
####################################################################################################

def to_axisangle(val: tuple[float, ...] ,index = 0) -> np.ndarray:
    if isinstance(val, (int, float)):
        if index in (25,26,27,28,29,30):
            return np.array([0.0, 0.0, float(val)], dtype=np.float32)
        if index in (31,32,33):
            return np.array([float(val), 0.0, float(val)], dtype=np.float32)
        if index in (34,35,36):
            return np.array([float(val)*0.3, 0.0, float(val)], dtype=np.float32)
        if index in (20,21):
            return np.array([float(val), 0.0, 0.0], dtype=np.float32)
        if index in (38,39):
            return np.array([float(val), 0.2 * float(val),   -float(val)], dtype=np.float32)
        if index in (40,41,42,43,44,45):
            return np.array([0.0, 0.0, -float(val)], dtype=np.float32)
        if index in (46,47,48):
            return np.array([float(val), 0.0, -float(val)], dtype=np.float32)
        if index in (49,50,51):
            return np.array([float(val)*0.3, 0.0, -float(val)], dtype=np.float32)
        if index in (53,54):
            return np.array([float(val), 0.2 * float(val),   -float(val)], dtype=np.float32)
            # dk, copied from 38,39. wait to be checked
        return np.array([0.0, float(val), 0.0], dtype=np.float32)
    
    if len(val) == 2:
      if index == 16:
          x,z = val
          y_linked = x        
          return np.array([x, 0.2 * y_linked, z], dtype=np.float32)

      if index == 17:
          x,z = val
          y_linked = x        
          return np.array([ x, -0.2 * y_linked, z], dtype=np.float32)
      
      if index == 18:
          x,y=val
          z_linked = -x
          return np.array([0.1 *x, y, z_linked], dtype=np.float32)
      
      if index == 19:
          x,y=val
          z_linked = x
          return np.array([x, y, z_linked], dtype=np.float32)
      
      if index == 37:
          x,y = val
          z_linked = y
          return np.array([x, y, z_linked], dtype=np.float32)
      
      if index == 52:
          x,y = val
          z_linked = y
          return np.array([x, y, z_linked], dtype=np.float32)
      

    return np.asarray(val, dtype=np.float32)


@dataclass(frozen=True)
class SmplOutputs:
    vertices: np.ndarray
    faces: np.ndarray
    T_world_joint: np.ndarray  # (num_joints, 4, 4)
    T_parent_joint: np.ndarray  # (num_joints, 4, 4)


class SmplHelper:
    """Helper for models in the SMPL family, implemented in numpy."""

    def __init__(self, model_path: Path) -> None:
        assert model_path.suffix.lower() == ".npz", "Model should be an .npz file!"
        body_dict = dict(**np.load(model_path, allow_pickle=True))

        self.J_regressor = body_dict["J_regressor"]
        self.weights = body_dict["weights"]
        self.v_template = body_dict["v_template"]
        self.posedirs = body_dict["posedirs"]
        self.shapedirs = body_dict["shapedirs"]
        self.faces = body_dict["f"]

        self.num_joints: int = self.weights.shape[-1]
        self.num_betas: int = self.shapedirs.shape[-1]
        self.parent_idx: np.ndarray = body_dict["kintree_table"][0]

    def get_outputs(self, betas: np.ndarray, joint_rotmats: np.ndarray) -> SmplOutputs:
        """Run the SMPL forward pass and return posed mesh & FK transforms."""
        # Shape blend-shapes
        v_tpose = self.v_template + np.einsum("vxb,b->vx", self.shapedirs, betas)
        j_tpose = np.einsum("jv,vx->jx", self.J_regressor, v_tpose)

        # Build local joint transforms (SE(3))
        T_parent_joint = np.zeros((self.num_joints, 4, 4)) + np.eye(4)
        T_parent_joint[:, :3, :3] = joint_rotmats
        T_parent_joint[0, :3, 3] = j_tpose[0]
        T_parent_joint[1:, :3, 3] = j_tpose[1:] - j_tpose[self.parent_idx[1:]]

        # Forward kinematics
        T_world_joint = T_parent_joint.copy()
        for i in range(1, self.num_joints):
            T_world_joint[i] = T_world_joint[self.parent_idx[i]] @ T_parent_joint[i]

        # Linear blend skinning (LBS)
        pose_delta = (joint_rotmats[1:, ...] - np.eye(3)).flatten()
        v_blend = v_tpose + np.einsum("byn,n->by", self.posedirs, pose_delta)
        v_delta = np.ones((v_blend.shape[0], self.num_joints, 4))
        v_delta[:, :, :3] = v_blend[:, None, :] - j_tpose[None, :, :]
        v_posed = np.einsum(
            "jxy,vj,vjy->vx", T_world_joint[:, :3, :], self.weights, v_delta
        )
        return SmplOutputs(v_posed, self.faces, T_world_joint, T_parent_joint)


########################################
#      APPLICATION ENTRY-POINT (main)  #
########################################



def deg(x):
    return x * math.pi / 180.0

VISUAL_OFFSET = {
    # web-pose startup for default motor = 0
    # 16: to_axisangle((0.0, -1.32), 16), # left shoulder roll
    # 18: to_axisangle((0.0, -0.96), 18), # left elbow pitch
    # 17: to_axisangle((0.0, 1.32), 17), # right shoulder roll
    # 19: to_axisangle((0.0, 0.96), 19), # right elbow pitch

    #-----------
    
    # web-pose startup for A-pose
    16: to_axisangle((0.0, -0.78), 16), # left shoulder roll
    17: to_axisangle((0.0, 0.78), 17), # right shoulder roll

}
def main(model_path: Path) -> None:
    # ————————————————————————————————————————
    # 1)  Spin-up TCP/WebSocket server (Viser)
    # ————————————————————————————————————————
    server = viser.ViserServer()
    server.scene.set_up_direction("+y")
    server.scene.add_grid("/grid", position=(0.0, -1.3, 0.0), plane="xz")

    # 2)  Initialise SMPL helper & GUI
    #     └─ make_gui_elements(..) **creates all widgets & gizmos**
    model = SmplHelper(model_path)

    gui_elements = make_gui_elements(
        server,
        num_betas=model.num_betas,
        num_joints=model.num_joints,
        parent_idx=model.parent_idx,
    )

    # 3)  Add mesh to scene (updated each frame)
    body_handle = server.scene.add_mesh_simple(
        "/human",
        model.v_template,
        model.faces,
        wireframe=gui_elements.gui_wireframe.value,
        color=gui_elements.gui_rgb.value,
    )

    # 4)  Main render/update loop – recompute SMPL when GUI changed
    red_sphere = trimesh.creation.icosphere(radius=0.001, subdivisions=1)
    red_sphere.visual.vertex_colors = (255, 0, 0, 255)  # type: ignore

    # print("render loop started")



    while True:
        time.sleep(0.02)  # crude throttling
        if not gui_elements.changed:
            continue

        gui_elements.changed = False

        

        # (Re)-evaluate SMPL with current GUI values

        # modify web pose
        # axes = []
        # for i, g in enumerate(gui_elements.gui_joints):
        #     a = to_axisangle(g.value, i)
        #     a = a + VISUAL_OFFSET.get(i, np.array([0.0, 0.0, 0.0]))
        #     axes.append(a)

        axes = []
        wxyz_vis = []
        for i, g in enumerate(gui_elements.gui_joints):
            a = to_axisangle(g.value, i)
            R_gui = tf.SO3.exp(a)
            off = VISUAL_OFFSET.get(i, None)
            if off is None:
                R_vis = R_gui
            else:
                R_off = tf.SO3.exp(off)
                R_vis = R_off @ R_gui
            axes.append(R_vis.log())
            wxyz_vis.append(R_vis.wxyz)

        smpl_outputs = model.get_outputs(
            betas=np.array([x.value for x in gui_elements.gui_betas]),
            joint_rotmats= tf.SO3.exp(np.array(axes)).as_matrix(),
        )

        # modified version
        # smpl_outputs = model.get_outputs(
        #     betas=np.array([x.value for x in gui_elements.gui_betas]),
        #     joint_rotmats= tf.SO3.exp(np.array(axes)).as_matrix(),
        # )

        # print("model has output")

        for i, control in enumerate(gui_elements.transform_controls):
            control.wxyz = wxyz_vis[i]
            control.position = smpl_outputs.T_parent_joint[i, :3, 3]

        # smpl_outputs = model.get_outputs(
        #     betas=np.array([x.value for x in gui_elements.gui_betas]),
        #     joint_rotmats= tf.SO3.exp(np.array([to_axisangle(g.value, i) for i,g in enumerate(gui_elements.gui_joints)])).as_matrix(),
        # )



        # Reflect into scene
        body_handle.vertices = smpl_outputs.vertices
        body_handle.wireframe = gui_elements.gui_wireframe.value
        body_handle.color = gui_elements.gui_rgb.value

        # Update gizmo positions so they stick to joints
        for i, control in enumerate(gui_elements.transform_controls):
            control.position = smpl_outputs.T_parent_joint[i, :3, 3]
        # updated gizmo
        # for i, control in enumerate(gui_elements.transform_controls):
        #     control.position = smpl_outputs.T_world_joint[i, :3, 3]


##############################################
#      GUI FACTORY – builds all user widgets  #
##############################################

hidden_indices = {26,27,29,30,32,33,35,36,38,39,41,42,44,45,47,48,50,51,53,54}

def make_gui_elements(
    server: viser.ViserServer,
    num_betas: int,
    num_joints: int,
    parent_idx: np.ndarray,
) -> "GuiElements":
    """Create every GUI widget and scene handle used by the app."""

    # ──────────────────────────────────────────
    # Tab layout container (left side of viewer)
    # ──────────────────────────────────────────
    tab_group = server.gui.add_tab_group()

    # Internal helper toggling a global "dirty" flag so the main loop knows
    # when to recompute the mesh after *any* widget changes.
    def set_changed(_):
        out.changed = True  # 'out' will be defined later after widgets exist.

    # ==============================================================================================
    # 1. VIEW TAB  — general render settings
    # ==============================================================================================
    with tab_group.add_tab("View", viser.Icon.VIEWFINDER):
        gui_rgb = server.gui.add_rgb("Color", initial_value=(90, 200, 255))
        gui_wireframe = server.gui.add_checkbox("Wireframe", initial_value=False)
        gui_show_controls = server.gui.add_checkbox("Handles", initial_value=True)

        gui_rgb.on_update(set_changed)
        gui_wireframe.on_update(set_changed)

        @gui_show_controls.on_update
        def _(_):
            # Show / hide scene gizmos (TransformControls)
            for control in transform_controls:
                control.visible = gui_show_controls.value

    # ==============================================================================================
    # 2. SHAPE TAB  — β-shape sliders (body thickness, height, …)
    # ==============================================================================================
    with tab_group.add_tab("Shape", viser.Icon.BOX):
        gui_reset_shape = server.gui.add_button("Reset Shape")
        gui_random_shape = server.gui.add_button("Random Shape")

        @gui_reset_shape.on_click
        def _(_):
            for beta in gui_betas:
                beta.value = 0.0

        @gui_random_shape.on_click
        def _(_):
            for beta in gui_betas:
                beta.value = np.random.normal(loc=0.0, scale=1.0)

        gui_betas = []
        for i in range(num_betas):
            beta = server.gui.add_slider(
                f"beta{i}", min=-5.0, max=5.0, step=0.01, initial_value=0.0
            )
            gui_betas.append(beta)
            beta.on_update(set_changed)

    # ==============================================================================================
    # 3. JOINTS TAB  — per-joint axis-angle controls
    # ==============================================================================================


    with tab_group.add_tab("Joints", viser.Icon.ANGLE):
      
        gui_reset_joints = server.gui.add_button("Reset Joints")
        gui_random_joints = server.gui.add_button("Random Joints")

        @gui_reset_joints.on_click
        def _(_):
            for joint in gui_joints:
                joint.value = (0.0, 0.0, 0.0)

        @gui_random_joints.on_click
        def _(_):
            rng = np.random.default_rng()
            for joint in gui_joints:
                joint.value = tf.SO3.sample_uniform(rng).log()

        gui_joints: list[viser.GuiInputHandle[tuple[float, float, float]]] = []
        for i in range(num_joints):
            if i == 16:
                gui_joint = server.gui.add_vector2(
                    label= f"Left Shoulder Pitch & Left Shoulder Roll",
                     initial_value=(0.0,0.0),
                     step=0.05,
                )
            elif i == 17:
              gui_joint = server.gui.add_vector2(
                    label= f"Right Shoulder Pitch & Right Shoulder Roll",
                     initial_value=(0.0,0.0),
                     step=0.05,
                )
            elif i == 18:
                gui_joint = server.gui.add_vector2(
                    label= f"Left Shoulder Yaw & Left Elbow Pitch",
                     initial_value=(0.0,0.0),
                     step=0.05,
                )
            elif i == 19:
              gui_joint = server.gui.add_vector2(
                    label= f"Right Shoulder Yaw & Right Elbow Pitch",
                     initial_value=(0.0,0.0),
                     step=0.05,
                )
            elif i == 20:
                gui_joint = server.gui.add_slider(
                      label = f"Left Elbow Yaw",
                      initial_value= 0.0,
                      step= 0.05,
                      min = deg(-123),
                      max = deg(123)
                  )
            elif i == 21:
                gui_joint = server.gui.add_slider(
                      label = f"Right Elbow Yaw",
                      initial_value= 0.0,
                      step= 0.05,
                      # min = -1.2,
                      # max = 1.2
                      min = deg(-123),
                      max = deg(123)
                  )
                
            elif i in (25,28,31,34,40,43,46,49):
                if i == 25: 
                  gui_joint = server.gui.add_slider(
                      label = f"Left Index Finger",
                      initial_value= 0.0,
                      step= 0.05,
                      # min = -1.2,
                      # max = 0.1
                      min = deg(-123),
                      max = deg(18)
                  )
                elif i == 28:
                    gui_joint = server.gui.add_slider(
                      label = f"Left Middle Finger",
                      initial_value= 0.0,
                      step= 0.05,
                      # min = -1.2,
                      # max = 0.1
                      min = deg(-132),
                      max = deg(18)
                  )
                elif i == 31:
                    gui_joint = server.gui.add_slider(
                      label = f"Left Pinkie Finger",
                      initial_value= 0.0,
                      step= 0.05,
                      # min = -1.2,
                      # max = 0.1
                      min = deg(-75),
                      max = deg(4)
                  )
                elif i == 34:
                    gui_joint = server.gui.add_slider(
                      label = f"Left Ring Finger",
                      initial_value= 0.0,
                      step= 0.05,
                      # min = -1.2,
                      # max = 0.1
                      min = deg(-136),
                      max = deg(18)
                  )
                elif i == 40: 
                  gui_joint = server.gui.add_slider(
                      label = f"Right Index Finger",
                      initial_value= 0.0,
                      step= 0.05,
                      # min = -1.2,
                      # max = 0.1
                      min = deg(-123),
                      max = deg(18)
                  )
                elif i == 43:
                    gui_joint = server.gui.add_slider(
                      label = f"Right Middle Finger",
                      initial_value= 0.0,
                      step= 0.05,
                      # min = -1.2,
                      # max = 0.1
                      min = deg(-132),
                      max = deg(18)
                  )
                elif i == 46:
                    gui_joint = server.gui.add_slider(
                      label = f"Right Pinkie Finger",
                      initial_value= 0.0,
                      step= 0.05,
                      # min = -1.2,
                      # max = 0.1
                      min = deg(-75),
                      max = deg(4)
                  )
                elif i == 49:
                    gui_joint = server.gui.add_slider(
                      label = f"Right Ring Finger",
                      initial_value= 0.0,
                      step= 0.05,
                      # min = -1.2,
                      # max = 0.1
                      min = deg(-136),
                      max = deg(18)
                  )
                    
            elif i == 37:
                left_thumb = server.gui.add_vector2(
                    label= f"Left Thumb Roll & Left Thumb Finger",
                     initial_value=(0.0,0.0),
                     step=0.05,
                     visible = False,
                )
                LeftThumb_roll = server.gui.add_slider(
                    label = "Left Thumb Roll",
                    initial_value= 0.0,
                    step = 0.05,
                    min = deg(-31),
                    max = deg(22),
                )
                LeftThumb_finger = server.gui.add_slider(
                    label = "Left Thumb Finger",
                    initial_value= 0.0,
                    step = 0.05,
                    min = deg(-44),
                    max = deg(75),
                )
                def _sync_left_thumb(_):
                    left_thumb.value = (float(LeftThumb_roll.value), float(-1.0 * LeftThumb_finger.value)) # magic number -1.0 used to match the moving direction between robot and web pose
                    print("read slider number")
                    out.changed = True
                LeftThumb_roll.on_update(_sync_left_thumb)
                LeftThumb_finger.on_update(_sync_left_thumb)
                gui_joint = left_thumb
            elif i == 52:
                right_thumb = server.gui.add_vector2(
                    label = "Right Thumb Roll & Right Thumb Finger",
                     initial_value=(0.0,0.0),
                     step=0.05,
                     visible = False,
                )
                RightThumb_roll = server.gui.add_slider(
                    label = "Right Thumb Roll",
                    initial_value= 0.0,
                    step = 0.05,
                    min = deg(-31),
                    max = deg(22),
                )
                RightThumb_finger = server.gui.add_slider(
                    label = "Right Thumb Finger",
                    initial_value= 0.0,
                    step = 0.05,
                    min = deg(-44),
                    max = deg(75),
                )
                def _sync_right_thumb(_):
                    right_thumb.value = (float(RightThumb_roll.value), float(RightThumb_finger.value))
                    out.changed = True
                RightThumb_roll.on_update(_sync_right_thumb)
                RightThumb_finger.on_update(_sync_right_thumb)
                gui_joint = right_thumb
            elif i in (26,27,29,30,32,33,35,36,38,39,41,42,44,45,47,48,50,51,53,54):
                gui_joint = server.gui.add_slider(
                    label = f"Joint {i}(1-DOF)",
                    initial_value= 0.0,
                    step= 0.05,
                    min = 0,
                    max = 4,
                    visible = False
                )
            else:
                # Each vector3 widget holds axis-angle (x,y,z) for one joint
                gui_joint = server.gui.add_vector3(
                    label=f"Joint {i}",
                    initial_value=(0.0, 0.0, 0.0),
                    step=0.05,
                )

            gui_joints.append(gui_joint)

            

            # <callback> When user drags a joint slider we update the gizmo rotation & mark dirty
            def set_callback_in_closure(i: int) -> None:
                @gui_joint.on_update
                def _(_):
                    if i == 25:
                      gui_joints[26].value = gui_joints[25].value
                      gui_joints[27].value = gui_joints[25].value
                      axis = to_axisangle(gui_joints[i].value, i)
                      # transform_controls[i].wxyz = tf.SO3.exp(axis).wxyz
                      R_gui = tf.SO3.exp(axis)
                      off = VISUAL_OFFSET.get(i, None)
                      R_vis = (tf.SO3.exp(off) @ R_gui) if off is not None else R_gui
                      transform_controls[i].wxyz = R_vis.wxyz
                      out.changed = True
                    elif i == 28:
                        gui_joints[29].value = gui_joints[28].value
                        gui_joints[30].value = gui_joints[28].value
                        axis = to_axisangle(gui_joints[i].value, i)
                        # transform_controls[i].wxyz = tf.SO3.exp(axis).wxyz
                        R_gui = tf.SO3.exp(axis)
                        off = VISUAL_OFFSET.get(i, None)
                        R_vis = (tf.SO3.exp(off) @ R_gui) if off is not None else R_gui
                        transform_controls[i].wxyz = R_vis.wxyz
                        out.changed = True
                    elif i == 31:
                        gui_joints[32].value = gui_joints[31].value
                        gui_joints[33].value = gui_joints[31].value
                        axis = to_axisangle(gui_joints[i].value, i)
                        # transform_controls[i].wxyz = tf.SO3.exp(axis).wxyz
                        R_gui = tf.SO3.exp(axis)
                        off = VISUAL_OFFSET.get(i, None)
                        R_vis = (tf.SO3.exp(off) @ R_gui) if off is not None else R_gui
                        transform_controls[i].wxyz = R_vis.wxyz
                        out.changed = True
                    elif i == 34:
                        gui_joints[35].value = gui_joints[34].value
                        gui_joints[36].value = gui_joints[34].value
                        axis = to_axisangle(gui_joints[i].value, i)
                        # transform_controls[i].wxyz = tf.SO3.exp(axis).wxyz
                        R_gui = tf.SO3.exp(axis)
                        off = VISUAL_OFFSET.get(i, None)
                        R_vis = (tf.SO3.exp(off) @ R_gui) if off is not None else R_gui
                        transform_controls[i].wxyz = R_vis.wxyz
                        out.changed = True
                    elif i == 37:
                        gui_joints[38].value = gui_joints[37].value[1]
                        gui_joints[39].value = gui_joints[37].value[1]
                        axis = to_axisangle(gui_joints[i].value, i)
                        # transform_controls[i].wxyz = tf.SO3.exp(axis).wxyz
                        R_gui = tf.SO3.exp(axis)
                        off = VISUAL_OFFSET.get(i, None)
                        R_vis = (tf.SO3.exp(off) @ R_gui) if off is not None else R_gui
                        transform_controls[i].wxyz = R_vis.wxyz
                        out.changed = True
                    elif i == 52:
                        gui_joints[53].value = gui_joints[52].value[1]
                        gui_joints[54].value = gui_joints[52].value[1]
                        axis = to_axisangle(gui_joints[i].value, i)
                        # transform_controls[i].wxyz = tf.SO3.exp(axis).wxyz
                        R_gui = tf.SO3.exp(axis)
                        off = VISUAL_OFFSET.get(i, None)
                        R_vis = (tf.SO3.exp(off) @ R_gui) if off is not None else R_gui
                        transform_controls[i].wxyz = R_vis.wxyz
                        out.changed = True
                    elif i == 40:
                      gui_joints[41].value = gui_joints[40].value
                      gui_joints[42].value = gui_joints[40].value
                      axis = to_axisangle(gui_joints[i].value, i)
                      # transform_controls[i].wxyz = tf.SO3.exp(axis).wxyz
                      R_gui = tf.SO3.exp(axis)
                      off = VISUAL_OFFSET.get(i, None)
                      R_vis = (tf.SO3.exp(off) @ R_gui) if off is not None else R_gui
                      transform_controls[i].wxyz = R_vis.wxyz
                      out.changed = True
                    elif i == 43:
                        gui_joints[44].value = gui_joints[43].value
                        gui_joints[45].value = gui_joints[43].value
                        axis = to_axisangle(gui_joints[i].value, i)
                        # transform_controls[i].wxyz = tf.SO3.exp(axis).wxyz
                        R_gui = tf.SO3.exp(axis)
                        off = VISUAL_OFFSET.get(i, None)
                        R_vis = (tf.SO3.exp(off) @ R_gui) if off is not None else R_gui
                        transform_controls[i].wxyz = R_vis.wxyz
                        out.changed = True
                    elif i == 46:
                        gui_joints[47].value = gui_joints[46].value
                        gui_joints[48].value = gui_joints[46].value
                        axis = to_axisangle(gui_joints[i].value, i)
                        # transform_controls[i].wxyz = tf.SO3.exp(axis).wxyz
                        R_gui = tf.SO3.exp(axis)
                        off = VISUAL_OFFSET.get(i, None)
                        R_vis = (tf.SO3.exp(off) @ R_gui) if off is not None else R_gui
                        transform_controls[i].wxyz = R_vis.wxyz
                        out.changed = True
                    elif i == 49:
                        gui_joints[50].value = gui_joints[49].value
                        gui_joints[51].value = gui_joints[49].value
                        axis = to_axisangle(gui_joints[i].value, i)
                        # transform_controls[i].wxyz = tf.SO3.exp(axis).wxyz
                        R_gui = tf.SO3.exp(axis)
                        off = VISUAL_OFFSET.get(i, None)
                        R_vis = (tf.SO3.exp(off) @ R_gui) if off is not None else R_gui
                        transform_controls[i].wxyz = R_vis.wxyz
                        out.changed = True
                        
                        
                    else:
                      axis = to_axisangle(gui_joints[i].value,i) 
                      # transform_controls[i].wxyz = tf.SO3.exp(axis).wxyz
                      R_gui = tf.SO3.exp(axis)
                      off = VISUAL_OFFSET.get(i, None)
                      R_vis = (tf.SO3.exp(off) @ R_gui) if off is not None else R_gui
                      transform_controls[i].wxyz = R_vis.wxyz
                      out.changed = True

                    value = tuple(to_axisangle(gui_joints[i].value,i))

                    if isinstance(value, np.ndarray):
                        value = value.tolist()
                    else:
                        value = tuple(float(x) for x in value)

                    print(value)
                    if i not in hidden_indices:
                        Sophia_control.call_remote(index = i,value = value)
                    

            set_callback_in_closure(i)

    # =====================================================================
    # 4. TRANSFORM CONTROLS (Scene gizmos attached to joints)              
    # =====================================================================
    # These are the coloured draggable arrows in the 3-D viewport.
    transform_controls: list[viser.TransformControlsHandle] = []
    prefixed_joint_names = []  # e.g. "root/hip_left/knee_left/..."
    for i in range(num_joints):
        prefixed_joint_name = f"joint_{i}"
        if i > 0:
            prefixed_joint_name = (
                prefixed_joint_names[parent_idx[i]] + "/" + prefixed_joint_name
            )
        prefixed_joint_names.append(prefixed_joint_name)

        controls = server.scene.add_transform_controls(
            f"/smpl/{prefixed_joint_name}",
            depth_test=False,
            scale=0.2 * (0.75 ** prefixed_joint_name.count("/")),
            disable_axes=True,
            disable_sliders=True,
            visible=True,  # sync later
        )
        transform_controls.append(controls)

        # <callback> Scene gizmo → UI synchronisation (inverse of earlier)
        def set_callback_in_closure(i: int) -> None:
            @controls.on_update
            def _(_) -> None:
                # axisangle = tf.SO3(controls.wxyz).log()
                R_vis = tf.SO3(controls.wxyz)
                off = VISUAL_OFFSET.get(i, None)
                if off is None:
                    R_gui = R_vis
                else:
                    R_off = tf.SO3.exp(off)
                    R_gui = R_off.inv() @ R_vis
                axisangle = R_gui.log()
                if len(gui_joints[i].value) == 2:
                    gui_joints[i].value = (axisangle[1], axisangle[2])
                else:
                    gui_joints[i].value = tuple(axisangle)

        set_callback_in_closure(i)

    # Bundle everything into a convenient struct that the main loop polls
    out = GuiElements(
        gui_rgb,
        gui_wireframe,
        gui_betas,
        gui_joints,
        transform_controls=transform_controls,
        changed=True,  # Force first recompute
    )
    return out

# ——————————————————————————————————————————————————————————————————————————
# Data-holder returned by make_gui_elements(..)
# ——————————————————————————————————————————————————————————————————————————
@dataclass
class GuiElements:
    """Container for easiest passing of GUI handles & state."""

    gui_rgb: viser.GuiInputHandle[tuple[int, int, int]]
    gui_wireframe: viser.GuiInputHandle[bool]
    gui_betas: list[viser.GuiInputHandle[float]]
    gui_joints: list[viser.GuiInputHandle[tuple[float, float, float]]]
    transform_controls: list[viser.TransformControlsHandle]
    changed: bool  # set to True whenever mesh needs recompute

# Dummy helper used somewhere else

def getindexandvalue(i, value):
    return i, value


# ——————————————————————————————————————————————————————————————————————————
#  CLI Entry – allow running with `python smpl_visualizer_annotated.py --model_path model.npz`
# ——————————————————————————————————————————————————————————————————————————
if __name__ == "__main__":
    tyro.cli(main, description=__doc__)



```

## Source file: llf_control(ref).py

{
  "path": "llf_control(ref).py",
  "blob_sha": "db11591f42fe0091a06316d391857ace102e0464",
  "sha256": "557bc13aadaf50b575cbcc73d387e93beb335112b66dd351f78e496f78070d05",
  "lines": 157,
  "bytes": 5308,
  "selection_score": 0,
  "keyword_line_counts": {
    "blendshape": 4,
    "rig": 1,
    "face": 2
  },
  "url": "https://github.com/GeorgeCzy/SophiaLearningRepo/blob/81e52a0be00a9a425fec780acfc5ea4236eb6325/llf_control%28ref%29.py",
  "function_signature_candidates": [
    {
      "line": 22,
      "text": "def get_bounds(limit_file_path):"
    },
    {
      "line": 27,
      "text": "def check_bounds(name, value, limits):"
    },
    {
      "line": 36,
      "text": "def check_contiguous(past_values, values):"
    },
    {
      "line": 47,
      "text": "def __init__(self, blendshape_used = \"control\", configs_path = \"./sophia_dev/sophia_control/configs\", map_path = \"./sophia_dev/bs_mapping/mapping_table/arkit_to_sophia.csv\"):"
    },
    {
      "line": 82,
      "text": "def set_manual_control(self):"
    },
    {
      "line": 96,
      "text": "def move(self):"
    },
    {
      "line": 127,
      "text": "def get_message(self, values):"
    }
  ]
}

```text

import math
import threading
import time

import os
import json
import pickle

import sys
sys.path.append(os.getcwd())

import rospy
from hr_msgs.msg import TargetPosture
from geometry_msgs.msg import Vector3
from hr_msgs.srv import SetActuatorsControl, SetActuatorsControlRequest

from sophia_dev.bs_mapping.bs_mapping import convert_list_to_dict, read_map
from sophia_dev.livelinkface.receive import bind, receive_data, decode_from_livelinkface

# help function
# value boundary 
def get_bounds(limit_file_path):
    with open(limit_file_path, 'r') as limit_file:
        limits = json.load(limit_file)
    return limits

def check_bounds(name, value, limits):
    bounds = limits[name]["value"]
    if value < bounds[0]:
        value = bounds[0]
    elif value > bounds[1]:
        value = bounds[1]
    return value

# limit the change between contiguous frames
def check_contiguous(past_values, values):
    for key in values:
        if values[key]-past_values[key]>0.2:
            values[key] = past_values[key]+0.2
        elif values[key] - past_values[key]<-0.2:
            values[key] = past_values[key]-0.2
    return values

class ActuatorControl(object):
    """An example showing how to control indivisual actuators.
    """
    def __init__(self, blendshape_used = "control", configs_path = "./sophia_dev/sophia_control/configs", map_path = "./sophia_dev/bs_mapping/mapping_table/arkit_to_sophia.csv"):
        rospy.wait_for_service("/hr/actuators/get_loggers")
        
        self.pose = rospy.Publisher("/hr/actuators/pose", TargetPosture, queue_size=10)
        self.set_control = rospy.ServiceProxy("/hr/actuators/set_control", SetActuatorsControl)
        self.set_state = rospy.Publisher("/hr/animation/set_state", TargetPosture, queue_size = 10)
        self.set_neck_rotation = rospy.Publisher("/hr/animation/set_neck_rotation", Vector3, queue_size=10)
        # expression_control
        # limit the change between contiguous frames
        self.past_values = None

        # blendshape used
        with open(os.path.join(configs_path, "blendshape_used.json"), "r") as bs_used_file:
            bs_used = json.load(bs_used_file)
        self.actuator_names = bs_used[blendshape_used]
        self.ani_names = ["HeadYaw", "HeadRoll", "HeadPitch", "SneerLeft", "SneerRight"]

        # boundary
        self.limits = get_bounds(os.path.join(configs_path, "limit.json"))

        # socket
        self.sever = bind()

        # bs map
        df, columns, arkit_index, seq = read_map(map_path)
        self.df = df
        self.columns = columns
        self.arkit_index = arkit_index
        self.seq = seq

        # ros spin
        job = threading.Timer(0, self.move)
        job.daemon = True
        job.start()

    def set_manual_control(self):
        """
        Set the actuators to manual control, so them can be controlled
        directly.

        CONTROL_DISABLE: disable actuator
        CONTROL_MANUAL: control actuator by hand (code)
        CONTROL_ANIMATION: control actuator by animation
        """
        request = SetActuatorsControlRequest()
        request.control = SetActuatorsControlRequest.CONTROL_MANUAL
        request.actuators = self.actuator_names
        self.set_control(request)
   
    def move(self):
        """Move the actuators"""
        self.set_manual_control()
        
        neck_rotation = Vector3()
        neck_rotation.x = 0
        neck_rotation.y = 0
        neck_rotation.z = 0
        self.set_neck_rotation.publish(neck_rotation)
      
        pose = TargetPosture()
        ani_pose = TargetPosture()
        
        while True:
            data = receive_data(self.sever)
            data = decode_from_livelinkface(data)
            # print(data)
            if data:
                self.get_message(values=data)
                pose.names = self.actuator_names[:]
                pose.values = self.values[:]
                    
                self.pose.publish(pose)
            
                ani_pose.names = self.ani_names
                ani_pose.values = self.ani_values
            
                self.set_state.publish(ani_pose)
                        
        # conn.close()

    def get_message(self, values):
        # values: arkit params
        values = [100.0*i for i in values]
        values = convert_list_to_dict(values, self.df, self.columns, self.arkit_index, self.seq)
        print(values["UpperLidLeft"])
        # set past_values if past_values is None
        if self.past_values is None:
            self.past_values = {}
            for key in values:
                self.past_values[key] = 0
        
        # limit the change between contiguous frames
        values = check_contiguous(self.past_values, values)

        # control by animation
        self.ani_values = []
        for ani_name in self.ani_names:
            self.ani_values.append(check_bounds(ani_name, values[ani_name], self.limits))
        
        # control by actuator
        self.values = []
        for actuator_name in self.actuator_names:
            self.values.append(check_bounds(actuator_name, values[actuator_name], self.limits))
        
        self.past_values = values
        

if __name__ == "__main__":
    rospy.init_node("actuator_control")
    ActuatorControl()
    rospy.spin()

```
