# Facial rendering source scan: yunik1004/SAiD

Repository: https://github.com/yunik1004/SAiD

Pinned source revision: bb7b18018358d3156569d8051f386943744a5ea0

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"blendshape": ["data/ARKit_blendshapes.txt", "data/ARKit_reference_blendshapes.zip", "data/blendshape_coeffs.zip", "data/blendshape_residuals.pickle", "said/optimize/blendshape_coeffs.py", "said/util/blendshape.py", "script/optimize_blendshape_coeffs.py", "static/image/editing/FaceTalk_170731_00024_TA-sentence01-0-blendshape.png", "static/image/editing/FaceTalk_170809_00138_TA-sentence02-0-blendshape.png", "static/video/SAiD/editing/FaceTalk_170731_00024_TA-sentence01-0-blendshape.mp4", "static/video/SAiD/editing/FaceTalk_170809_00138_TA-sentence02-0-blendshape.mp4"], "flame": ["data/FLAME_head_idx.txt", "data/FLAME_head_landmarks.txt", "data/FLAME_landmarks.txt"], "face": ["static/image/editing/FaceTalk_170731_00024_TA-sentence01-0-blendshape.png", "static/image/editing/FaceTalk_170731_00024_TA-sentence01-0-inbetween.png", "static/image/editing/FaceTalk_170809_00138_TA-sentence02-0-blendshape.png", "static/image/editing/FaceTalk_170809_00138_TA-sentence02-0-inbetween.png", "static/video/CodeTalker/FaceTalk_170731_00024_TA-sentence01-0.mp4", "static/video/CodeTalker/FaceTalk_170809_00138_TA-sentence02-0.mp4", "static/video/FaceFormer/FaceTalk_170731_00024_TA-sentence01-0.mp4", "static/video/FaceFormer/FaceTalk_170809_00138_TA-sentence02-0.mp4", "static/video/GT/FaceTalk_170731_00024_TA-sentence01.mp4", "static/video/GT/FaceTalk_170809_00138_TA-sentence02.mp4", "static/video/MeshTalk/FaceTalk_170731_00024_TA-sentence01.mp4", "static/video/MeshTalk/FaceTalk_170809_00138_TA-sentence02.mp4", "static/video/SAiD/FaceTalk_170731_00024_TA-sentence01-0-vocaset.mp4", "static/video/SAiD/FaceTalk_170809_00138_TA-sentence02-0-vocaset.mp4", "static/video/SAiD/ablation/finetune_wav2vec/FaceTalk_170731_00024_TA-sentence01-0.mp4", "static/video/SAiD/ablation/finetune_wav2vec/FaceTalk_170809_00138_TA-sentence02-0.mp4", "static/video/SAiD/ablation/no_alignment_bias/FaceTalk_170731_00024_TA-sentence01-0.mp4", "static/video/SAiD/ablation/no_alignment_bias/FaceTalk_170809_00138_TA-sentence02-0.mp4", "static/video/SAiD/ablation/squared_error/FaceTalk_170731_00024_TA-sentence01-0.mp4", "static/video/SAiD/ablation/squared_error/FaceTalk_170809_00138_TA-sentence02-0.mp4", "static/video/SAiD/ablation/velocity_loss/FaceTalk_170731_00024_TA-sentence01-0.mp4", "static/video/SAiD/ablation/velocity_loss/FaceTalk_170809_00138_TA-sentence02-0.mp4", "static/video/SAiD/diversity/FaceTalk_170731_00024_TA-sentence01-0.mp4", "static/video/SAiD/diversity/FaceTalk_170731_00024_TA-sentence01-1.mp4", "static/video/SAiD/diversity/FaceTalk_170731_00024_TA-sentence01-2.mp4", "static/video/SAiD/diversity/FaceTalk_170731_00024_TA-sentence01-3.mp4", "static/video/SAiD/diversity/FaceTalk_170731_00024_TA-sentence01-4.mp4", "static/video/SAiD/diversity/FaceTalk_170809_00138_TA-sentence02-0.mp4", "static/video/SAiD/diversity/FaceTalk_170809_00138_TA-sentence02-1.mp4", "static/video/SAiD/diversity/FaceTalk_170809_00138_TA-sentence02-2.mp4"], "lip": ["blender-addon/lipsync.py"]}

## Source file: said/util/blendshape.py

{
  "path": "said/util/blendshape.py",
  "blob_sha": "645354707c94151bfbb6c249d0bf9796096c1c49",
  "sha256": "46941a2834ddaabe373d7174516b23d62a350b84abe2f9ed42873de8720f1509",
  "lines": 84,
  "bytes": 2087,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 20,
    "rig": 2
  },
  "url": "https://github.com/yunik1004/SAiD/blob/bb7b18018358d3156569d8051f386943744a5ea0/said/util/blendshape.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def load_blendshape_deltas("
    },
    {
      "line": 36,
      "text": "def load_blendshape_coeffs(coeffs_path: str) -> torch.FloatTensor:"
    },
    {
      "line": 54,
      "text": "def save_blendshape_coeffs("
    },
    {
      "line": 72,
      "text": "def save_blendshape_coeffs_image(coeffs: np.ndarray, output_path: str) -> None:"
    }
  ]
}

```text

"""Define the utility functions related to the blendshape
"""
import pickle
from typing import Dict, List
import numpy as np
import pandas as pd
from PIL import Image
import torch


def load_blendshape_deltas(
    blendshape_deltas_path: str,
) -> Dict[str, Dict[str, np.ndarray]]:
    """Load the blendshape deltas

    Parameters
    ----------
    blendshape_deltas_path : str
        Path of the blendshape deltas file

    Returns
    -------
    Dict[str, Dict[str, np.ndarray]]
        {
            "<Person ID>": {
                "<Blendshape name>": np.ndarray, (|V|, 3)
            }
        }
    """
    with open(blendshape_deltas_path, "rb") as f:
        blendshape_deltas = pickle.load(f)

    return blendshape_deltas


def load_blendshape_coeffs(coeffs_path: str) -> torch.FloatTensor:
    """Load the blendshape coefficients file

    Parameters
    ----------
    coeffs_path : str
        Path of the blendshape coefficients file (csv format)

    Returns
    -------
    torch.FloatTensor
        (T_b, num_classes), Blendshape coefficients
    """
    df = pd.read_csv(coeffs_path)
    coeffs = torch.FloatTensor(df.values)
    return coeffs


def save_blendshape_coeffs(
    coeffs: np.ndarray, classes: List[str], output_path: str
) -> None:
    """Save the blendshape coefficients into the file

    Parameters
    ----------
    coeffs : np.ndarray
        (T_b, num_classes), Blendshape coefficients
    classes : List[str]
        List of the class names of the coefficients
    output_path : str
        Path of the output file
    """
    pout = pd.DataFrame(coeffs, columns=classes)
    pout.to_csv(output_path, index=False)


def save_blendshape_coeffs_image(coeffs: np.ndarray, output_path: str) -> None:
    """Save the blendshape coefficients into the image file

    Parameters
    ----------
    coeffs : np.ndarray
        (T_b, num_classes), Blendshape coefficients
    output_path : str
        Path of the output file
    """
    orig = (255 * coeffs.transpose()).round()
    img = Image.fromarray(orig).convert("L")
    img.save(output_path)


```

## Source file: blender-addon/lipsync.py

{
  "path": "blender-addon/lipsync.py",
  "blob_sha": "1a1816d5ed0da779b33b275116fb5bd61df4c863",
  "sha256": "9d9507b6fac54574395379c3ae6c185fd681e1200eb8179cb97060b4c462326c",
  "lines": 787,
  "bytes": 24048,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 71,
    "shader": 1,
    "lip": 65
  },
  "url": "https://github.com/yunik1004/SAiD/blob/bb7b18018358d3156569d8051f386943744a5ea0/blender-addon/lipsync.py",
  "function_signature_candidates": [
    {
      "line": 127,
      "text": "def draw(self, context: bpy.types.Context):"
    },
    {
      "line": 163,
      "text": "def draw(self, context: bpy.types.Context):"
    },
    {
      "line": 255,
      "text": "def load_obj(context: bpy.types.Context, path: str) -> Optional[bpy.types.Object]:"
    },
    {
      "line": 287,
      "text": "def load_speaker("
    },
    {
      "line": 326,
      "text": "def execute(self, context: bpy.types.Context) -> Set[str]:"
    },
    {
      "line": 436,
      "text": "def execute(self, context: bpy.types.Context) -> Set[str]:"
    },
    {
      "line": 495,
      "text": "def execute(self, context: bpy.types.Context) -> Set[str]:"
    },
    {
      "line": 593,
      "text": "def execute(self, context: bpy.types.Context) -> Set[str]:"
    },
    {
      "line": 648,
      "text": "def execute(self, context: bpy.types.Context) -> Set[str]:"
    },
    {
      "line": 772,
      "text": "def register():"
    },
    {
      "line": 779,
      "text": "def unregister():"
    }
  ]
}

```text

"""Blender Add-on for the lipsync animation
"""
import csv
from math import pi, sqrt
import os
from pathlib import Path
from typing import Optional, Set, Tuple
import aud
import bpy
from bpy_extras.io_utils import ExportHelper


# Add-on information
bl_info = {
    "name": "Lipsync",
    "author": "Inkyu",
    "version": (0, 6, 0),
    "blender": (4, 0, 0),
    "location": "View3D > Sidebar > Lipsync",
    "description": "Tools for generating lipsync animation",
    "category": "Lipsync",
}

# Global variables
BASIS_KEY = "Basis"
DIFF_MATERIAL = "DiffMaterial"
ACTION_NAME = "LipsyncAction"
DIFF_ACTION_NAME = "DiffAction"


class LipsyncProperty(bpy.types.PropertyGroup):
    """Store the properties"""

    audio_path_mesh_sequence: bpy.props.StringProperty(
        name="Audio",
        default="",
        description="Path of the audio file",
        subtype="FILE_PATH",
    )

    mesh_sequence_dir: bpy.props.StringProperty(
        name="Mesh Sequence",
        default="",
        description="Directory of the mesh sequence",
        subtype="DIR_PATH",
    )

    neutral_path: bpy.props.StringProperty(
        name="Template Mesh",
        default="",
        description="Path of the template mesh",
        subtype="FILE_PATH",
    )

    blendshape_dir: bpy.props.StringProperty(
        name="Blendshape Meshes",
        default="",
        description="Directory of the blendshape meshes",
        subtype="DIR_PATH",
    )

    audio_path_blendshape: bpy.props.StringProperty(
        name="Speech",
        default="",
        description="Path of the audio file",
        subtype="FILE_PATH",
    )

    blendshape_weights_path: bpy.props.StringProperty(
        name="Blendshape Coefficient Sequence",
        default="",
        description="Path of the blendshape coefficient sequence (CSV)",
        subtype="FILE_PATH",
    )

    fps_mesh: bpy.props.IntProperty(
        name="FPS",
        default=60,
        description="Automatically set fps when -1",
    )

    fps_blendshape: bpy.props.IntProperty(
        name="FPS",
        default=60,
        description="Automatically set fps when -1",
    )

    rearrange_blendshape: bpy.props.BoolProperty(
        name="Rearrange Blendshapes",
        default=True,
        description="Rearrange the order of shape keys (Blendshapes)",
    )

    target_obj: bpy.props.PointerProperty(
        name="Target Object",
        type=bpy.types.Object,
        description="Target object to visualize the vertex differences",
    )

    color_multiplier: bpy.props.FloatProperty(
        name="Color Multiplier",
        default=1.0,
        description="Multiplier for the color which visualize the vertex differences",
    )

    vis_option: bpy.props.EnumProperty(
        name="Visualize Option",
        items=[
            ("Vector", "Vector", ""),
            ("Amplitude", "Amplitude", ""),
        ],
        default="Vector",
        description="Option for the visualization",
    )


class Lipsync_PT_MeshsequencePanel(bpy.types.Panel):
    """Mesh panel"""

    bl_idname = "LIPSYNC_PT_meshsequence_panel"
    bl_label = "Mesh Sequence Panel"
    bl_category = "Lipsync"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_options = {"DEFAULT_CLOSED"}

    def draw(self, context: bpy.types.Context):
        """Draw UI elements

        Parameters
        ----------
        context : bpy.types.Context
            Blender context
        """
        box_mesh = self.layout.box()

        row = box_mesh.row()
        row.prop(context.scene.lipsync_property, "audio_path_mesh_sequence")

        row = box_mesh.row()
        row.prop(context.scene.lipsync_property, "mesh_sequence_dir")

        row = box_mesh.row()
        row.prop(context.scene.lipsync_property, "fps_blendshape")

        # Button
        row = box_mesh.row()
        row.operator(
            "lipsync.generate_mesh_anime_operator",
            text="Import Facial Motion",
        )


class Lipsync_PT_BlendshapePanel(bpy.types.Panel):
    """Mesh panel"""

    bl_idname = "LIPSYNC_PT_blendshape_panel"
    bl_label = "Blendshape Panel"
    bl_category = "Lipsync"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"

    def draw(self, context: bpy.types.Context):
        """Draw UI elements

        Parameters
        ----------
        context : bpy.types.Context
            Blender context
        """
        # Box for blendshape
        box_blendshape = self.layout.box()

        row = box_blendshape.row()
        row.prop(context.scene.lipsync_property, "neutral_path")

        row = box_blendshape.row()
        row.prop(context.scene.lipsync_property, "blendshape_dir")

        row = box_blendshape.row()
        row.operator(
            "lipsync.import_blendshape_operator",
            text="Import Blendshape Facial Model",
        )

        # Box for generating anime
        box_anime = self.layout.box()

        # Visualize the selected object
        row = box_anime.row()
        row.label(text="Selected Object: ", icon="OBJECT_DATA")
        box = box_anime.box()
        selected_objects = context.selected_objects
        if len(selected_objects) == 0:
            box.label(text="", icon="KEYFRAME")
        else:
            box.label(text=selected_objects[0].name, icon="KEYFRAME")

        row = box_anime.row()
        row.prop(context.scene.lipsync_property, "audio_path_blendshape")

        row = box_anime.row()
        row.prop(context.scene.lipsync_property, "blendshape_weights_path")

        row = box_anime.row()
        row.prop(context.scene.lipsync_property, "fps_blendshape")

        row = box_anime.row()
        row.prop(context.scene.lipsync_property, "rearrange_blendshape")

        row = box_anime.row()
        row.operator(
            "lipsync.generate_blendshape_anime_operator",
            text="Import Facial Motion",
        )

        # Box for saving anime
        box_save = self.layout.box()

        row = box_save.row()
        row.label(text="Selected Object: ", icon="OBJECT_DATA")
        box = box_save.box()
        if len(selected_objects) == 0:
            box.label(text="", icon="KEYFRAME")
        else:
            box.label(text=selected_objects[0].name, icon="KEYFRAME")

        row = box_save.row()
        row.operator(
            "lipsync.save_blendshape_anime_operator",
            text="Save Facial Motion",
        )

        """
        # Deprecated
        box_visualize = self.layout.box()

        row = box_visualize.row()
        row.prop(context.scene.lipsync_property, "target_obj")

        row = box_visualize.row()
        row.prop(context.scene.lipsync_property, "vis_option")

        row = box_visualize.row()
        row.prop(context.scene.lipsync_property, "color_multiplier")

        row = box_visualize.row()
        row.operator(
            "lipsync.visualize_difference_operator",
            text="Visualize Difference",
        )
        """


def load_obj(context: bpy.types.Context, path: str) -> Optional[bpy.types.Object]:
    """Load object from the path. Currently support 'ply' and 'obj' file.

    Parameters
    ----------
    context : bpy.types.Context
        Blender context

    path: str
        Path of the object file

    Returns
    -------
    Optional[bpy.types.Object]
        Loaded object
    """
    ext = os.path.splitext(path)[-1]

    obj = None

    if ext == ".ply":
        #bpy.ops.import_mesh.ply(filepath=path)
        bpy.ops.wm.ply_import(filepath=path)
        obj = context.object
    elif ext == ".obj":
        #bpy.ops.import_scene.obj(filepath=path, split_mode="OFF")
        bpy.ops.wm.obj_import(filepath=path)
        obj = context.selected_objects[0]

    return obj


def load_speaker(
    context: bpy.types.Context, audio_path: str
) -> Tuple[bpy.types.Object, int]:
    """Load audio using speaker object

    Parameters
    ----------
    context : bpy.types.Context
        Blender context

    audio_path: str
        Path of the audio file

    Returns
    -------
    Tuple[bpy.types.Object, int]
        (Speaker object, length of the audio)
    """
    # Load the sound
    bpy.ops.object.speaker_add(rotation=(pi, 0.0, 0.0))
    speaker = context.object
    speaker.hide_set(True)
    speaker.data.sound = bpy.data.sounds.load(audio_path)
    speaker.data.attenuation = 0.0
    speaker.data.update_tag()

    sd = aud.Sound(audio_path)
    length = sd.length
    del sd

    return speaker, length


class LipsyncGenerateMeshAnimeOperator(bpy.types.Operator):
    """Operator for the 'Import Facial Motion' button in Meshsequence panel"""

    bl_idname = "lipsync.generate_mesh_anime_operator"
    bl_label = "lipsync.generate_mesh_anime_operator"

    def execute(self, context: bpy.types.Context) -> Set[str]:
        """Execute the operator

        Parameters
        ----------
        context : bpy.types.Context
            Blender context

        Returns
        -------
        Set[str]
            Set of status messages: https://docs.blender.org/api/current/bpy.types.Operator.html
        """
        audio_path = bpy.path.abspath(
            context.scene.lipsync_property.audio_path_mesh_sequence
        )
        mesh_sequence_dir = bpy.path.abspath(
            context.scene.lipsync_property.mesh_sequence_dir
        )

        # Reset the scene frame to 1
        context.scene.frame_set(1)

        # Load the sound
        speaker, length = load_speaker(context, audio_path)

        # List the mesh sequence
        sequence = []
        for path in sorted(os.listdir(mesh_sequence_dir)):
            abs_path = os.path.join(mesh_sequence_dir, path)
            if os.path.isfile(abs_path):
                sequence.append(abs_path)

        # Update frame rate of the animation
        num_sequence = len(sequence)

        fps = context.scene.lipsync_property.fps_mesh
        if fps < 0:
            samplerate = speaker.data.sound.samplerate
            context.scene.render.fps = num_sequence
            context.scene.render.fps_base = length / samplerate
        else:
            context.scene.render.fps = fps
            context.scene.render.fps_base = 1

        # Generate animation sequence
        obj = load_obj(context, sequence[0])
        if obj is None:
            self.report({"ERROR_INVALID_INPUT"}, "File format is not supported")
            return {"CANCELLED"}

        mesh = obj.data
        num_vertices = len(mesh.vertices)

        # Generate vertices coordinates lists
        coords_list = []

        # Default vertices coordinates
        coords = [None] * (num_vertices * 3)
        mesh.vertices.foreach_get("co", coords)
        coords_list.append(coords)

        # Next vertices coordinates
        for sdx in range(1, num_sequence):
            obj_tmp = load_obj(context, sequence[sdx])
            if obj_tmp is None:
                self.report({"ERROR_INVALID_INPUT"}, "File format is not supported")
                return {"CANCELLED"}

            coords = [None] * (num_vertices * 3)
            obj_tmp.data.vertices.foreach_get("co", coords)
            coords_list.append(coords)

            bpy.ops.object.delete()

        # Create animation_data, action
        mesh.animation_data_create()
        mesh.animation_data.action = bpy.data.actions.new(name=ACTION_NAME)

        frames = range(1, num_sequence + 1)

        # Insert keyframes
        for vdx in range(num_vertices):
            for idx in range(3):
                fcurve = mesh.animation_data.action.fcurves.new(
                    data_path=f"vertices[{vdx}].co",
                    index=idx,
                )

                samples = [
                    coords_list[fdx][3 * vdx + idx] for fdx in range(num_sequence)
                ]

                fcurve.keyframe_points.add(count=num_sequence)
                fcurve.keyframe_points.foreach_set(
                    "co", [x for co in zip(frames, samples) for x in co]
                )

        # Update the frame end
        context.scene.frame_end = max(context.scene.frame_end, num_sequence)

        return {"FINISHED"}


class LipsyncAddBlendshapeOperator(bpy.types.Operator):
    """Operator for the 'Add Blendshape' button in Blendshape panel"""

    bl_idname = "lipsync.import_blendshape_operator"
    bl_label = "lipsync.import_blendshape_operator"

    def execute(self, context: bpy.types.Context) -> Set[str]:
        """Execute the operator

        Parameters
        ----------
        context : bpy.types.Context
            Blender context

        Returns
        -------
        Set[str]
            Set of status messages: https://docs.blender.org/api/current/bpy.types.Operator.html
        """
        neutral_path = bpy.path.abspath(context.scene.lipsync_property.neutral_path)
        blendshape_dir = bpy.path.abspath(context.scene.lipsync_property.blendshape_dir)

        # Get blendshape names and absolute paths
        blendshape_names = []
        blendshape_paths = []
        for path in sorted(os.listdir(blendshape_dir)):
            abs_path = os.path.join(blendshape_dir, path)
            if os.path.isfile(abs_path):
                blendshape_names.append(Path(path).stem)
                blendshape_paths.append(abs_path)

        # Load neutral mesh
        obj = load_obj(context, neutral_path)
        if obj is None:
            self.report({"ERROR_INVALID_INPUT"}, "File format is not supported")
            return {"CANCELLED"}

        num_vertices = len(obj.data.vertices)

        # Create shape keys
        obj.shape_key_add(name=BASIS_KEY)

        for bdx in range(len(blendshape_names)):
            obj_bdx = load_obj(context, blendshape_paths[bdx])
            if obj_bdx is None:
                self.report({"ERROR_INVALID_INPUT"}, "File format is not supported")
                continue

            sk = obj.shape_key_add(name=blendshape_names[bdx])

            coords = [None] * (num_vertices * 3)
            obj_bdx.data.vertices.foreach_get("co", coords)
            sk.data.foreach_set("co", coords)

            bpy.ops.object.delete()

        return {"FINISHED"}


class LipsyncGenerateBlendshapeAnimeOperator(bpy.types.Operator):
    """Operator for the 'Generate Blendshape Anime' button in Blendshape panel"""

    bl_idname = "lipsync.generate_blendshape_anime_operator"
    bl_label = "lipsync.generate_blendshape_anime_operator"

    def execute(self, context: bpy.types.Context) -> Set[str]:
        """Execute the operator

        Parameters
        ----------
        context : bpy.types.Context
            Blender context

        Returns
        -------
        Set[str]
            Set of status messages: https://docs.blender.org/api/current/bpy.types.Operator.html
        """
        audio_path = bpy.path.abspath(
            context.scene.lipsync_property.audio_path_blendshape
        )
        blendshape_weights_path = bpy.path.abspath(
            context.scene.lipsync_property.blendshape_weights_path
        )

        selected_objects = context.selected_objects
        if len(selected_objects) == 0:
            self.report({"ERROR_INVALID_INPUT"}, "Should have to select the object")
            return {"CANCELLED"}
        obj = selected_objects[0]

        # Reset the scene frame to 1
        context.scene.frame_set(1)

        # Load the sound
        speaker, length = load_speaker(context, audio_path)

        # Reset the active object
        context.view_layer.objects.active = obj

        # Load blendshape weights
        weights = []
        with open(blendshape_weights_path, "r") as csvfile:
            rdr = csv.DictReader(csvfile)
            for row in rdr:
                for k, v in row.items():
                    row[k] = float(v)
                weights.append(row)

        # Update frame rate of the animation
        num_sequence = len(weights)
        samplerate = speaker.data.sound.samplerate

        fps = context.scene.lipsync_property.fps_blendshape
        if fps < 0:
            samplerate = speaker.data.sound.samplerate
            context.scene.render.fps = num_sequence
            context.scene.render.fps_base = length / samplerate
        else:
            context.scene.render.fps = fps
            context.scene.render.fps_base = 1

        key_blocks = obj.data.shape_keys.key_blocks

        # Change the order of shape_keys
        if context.scene.lipsync_property.rearrange_blendshape and num_sequence > 0:
            blendshape_keys = list(weights[0].keys())
            blendshape_keys.insert(0, BASIS_KEY)

            for key in reversed(blendshape_keys):
                kdx = key_blocks.find(key)
                if kdx != -1:
                    obj.active_shape_key_index = kdx
                    bpy.ops.object.shape_key_move(type="TOP")

        # Generate animation sequence
        for wdx, weight in enumerate(weights):
            for keyblock in key_blocks:
                if keyblock.name == BASIS_KEY:
                    continue

                if keyblock.name in weight:
                    keyblock.value = weight[keyblock.name]
                    keyblock.keyframe_insert("value", frame=wdx + 1)

        # Update the frame end
        context.scene.frame_end = max(context.scene.frame_end, num_sequence)

        return {"FINISHED"}


class LipsyncSaveBlendshapeAnimeOperator(bpy.types.Operator, ExportHelper):
    """Operator for the 'Save Blendshape Anime' button in Blendshape panel"""

    bl_idname = "lipsync.save_blendshape_anime_operator"
    bl_label = "Save Blendshape"

    filename_ext = ".csv"
    filter_glob: bpy.props.StringProperty(
        default="*.csv",
        options={"HIDDEN"},
    )

    def execute(self, context: bpy.types.Context) -> Set[str]:
        """Execute the operator

        Parameters
        ----------
        context : bpy.types.Context
            Blender context

        Returns
        -------
        Set[str]
            Set of status messages: https://docs.blender.org/api/current/bpy.types.Operator.html
        """
        selected_objects = context.selected_objects
        if len(selected_objects) == 0:
            self.report({"ERROR_INVALID_INPUT"}, "No selected object")
            return {"CANCELLED"}

        obj = selected_objects[0]
        key_blocks = obj.data.shape_keys.key_blocks

        blendshape_keys = key_blocks.keys()[1:]  # Remove BASIS_KEY

        vals = []

        fcurves = obj.data.shape_keys.animation_data.action.fcurves
        if fcurves:
            num_frame = len(fcurves[0].keyframe_points)
            vals = [[0 for _ in range(len(blendshape_keys))] for _ in range(num_frame)]

        for fcurve in fcurves:
            key = fcurve.data_path.split('"')[1]
            if key == BASIS_KEY:
                continue

            kdx = blendshape_keys.index(key)

            for fdx, point in enumerate(fcurve.keyframe_points):
                frame_val = point.co[1]
                vals[fdx][kdx] = frame_val

        with open(self.filepath, "w", newline="") as csvfile:
            writer = csv.writer(csvfile)
            writer.writerow(blendshape_keys)
            writer.writerows(vals)

        return {"FINISHED"}


class LipsyncVisualizeDifferenceOperator(bpy.types.Operator):
    """Operator for the 'Visualize Difference' button in Blendshape panel"""

    bl_idname = "lipsync.visualize_difference_operator"
    bl_label = "Visualize Difference"

    def execute(self, context: bpy.types.Context) -> Set[str]:
        """Execute the operator

        Parameters
        ----------
        context : bpy.types.Context
            Blender context

        Returns
        -------
        Set[str]
            Set of status messages: https://docs.blender.org/api/current/bpy.types.Operator.html
        """
        selected_objects = context.selected_objects
        if len(selected_objects) == 0:
            self.report({"ERROR_INVALID_INPUT"}, "No selected object")
            return {"CANCELLED"}

        obj = selected_objects[0]

        target_obj = context.scene.lipsync_property.target_obj
        if target_obj is None:
            self.report({"ERROR_INVALID_INPUT"}, "No target object")
            return {"CANCELLED"}

        ### Update vertex colors
        mesh = obj.data

        if len(mesh.vertex_colors) == 0:
            mesh.vertex_colors.new()

        # Set material
        if DIFF_MATERIAL not in bpy.data.materials:
            mat = bpy.data.materials.new(name=DIFF_MATERIAL)
        mat = bpy.data.materials[DIFF_MATERIAL]
        mat.use_nodes = True

        vcolor = mat.node_tree.nodes.new("ShaderNodeVertexColor")

        bsdf = mat.node_tree.nodes["Principled BSDF"]
        mat.node_tree.links.new(vcolor.outputs[0], bsdf.inputs[0])

        mesh.materials.append(mat)

        new_mat_idx = len(mesh.materials) - 1

        for poly in mesh.polygons:
            poly.material_index = new_mat_idx

        color_multiplier = context.scene.lipsync_property.color_multiplier
        vis_option = context.scene.lipsync_property.vis_option

        try:
            fcurves = obj.data.shape_keys.animation_data.action.fcurves
            num_frame = len(fcurves[0].keyframe_points)
        except:
            num_frame = 0

        # Compute the difference
        diff_dict = [{} for _ in range(num_frame)]
        for fdx in range(num_frame):
            context.scene.frame_set(fdx + 1)
            for poly in mesh.polygons:
                for i, vdx in enumerate(poly.vertices):
                    diff = mesh.vertices[vdx].co - target_obj.data.vertices[vdx].co
                    loop_idx = poly.loop_indices[i]
                    diff_dict[fdx][loop_idx] = diff

        context.scene.frame_set(1)

        # Create animation_data, action
        mesh.animation_data_create()
        mesh.animation_data.action = bpy.data.actions.new(name=DIFF_ACTION_NAME)

        frames = range(1, num_frame + 1)

        # Insert keyframes
        for poly in mesh.polygons:
            for i, vdx in enumerate(poly.vertices):
                loop_idx = poly.loop_indices[i]
                diff = [diff_dict[fdx][loop_idx] for fdx in range(num_frame)]

                diff_val = []
                if vis_option == "Vector":
                    diff_val = [(abs(df[0]), abs(df[1]), abs(df[2])) for df in diff]
                elif vis_option == "Amplitude":
                    amp = [sqrt(df[0] ** 2 + df[1] ** 2 + df[2] ** 2) for df in diff]
                    diff_val = [(ap, ap, ap) for ap in amp]
                else:
                    self.report({"ERROR_INVALID_INPUT"}, "Weird visualize option")
                    return {"CANCELLED"}

                for idx in range(3):
                    fcurve = mesh.animation_data.action.fcurves.new(
                        data_path=f"vertex_colors.active.data[{loop_idx}].color",
                        index=idx,
                    )

                    samples = [
                        diff_val[fdx][idx] * color_multiplier
                        for fdx in range(num_frame)
                    ]

                    fcurve.keyframe_points.add(count=num_frame)
                    fcurve.keyframe_points.foreach_set(
                        "co", [x for co in zip(frames, samples) for x in co]
                    )

        return {"FINISHED"}


# List of classes to be registered
classes = (
    LipsyncProperty,
    Lipsync_PT_MeshsequencePanel,
    Lipsync_PT_BlendshapePanel,
    LipsyncGenerateMeshAnimeOperator,
    LipsyncAddBlendshapeOperator,
    LipsyncGenerateBlendshapeAnimeOperator,
    LipsyncSaveBlendshapeAnimeOperator,
    LipsyncVisualizeDifferenceOperator,
)


def register():
    """Register the classes"""
    for cls in classes:
        bpy.utils.register_class(cls)
    bpy.types.Scene.lipsync_property = bpy.props.PointerProperty(type=LipsyncProperty)


def unregister():
    """Unregister the classes"""
    for cls in classes:
        bpy.utils.unregister_class(cls)
    del bpy.types.Scene.lipsync_property


if __name__ == "__main__":
    register()


```

## Source file: said/optimize/blendshape_coeffs.py

{
  "path": "said/optimize/blendshape_coeffs.py",
  "blob_sha": "427fba2e3796696816a3dc9cc30f255b84c8fbff",
  "sha256": "094b176a6533fc829d9cf1b5c5038b534dc21aec9d5f8e3c9c7f4612e82cef2f",
  "lines": 186,
  "bytes": 5529,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 3,
    "blendshape": 27,
    "lip": 2
  },
  "url": "https://github.com/yunik1004/SAiD/blob/bb7b18018358d3156569d8051f386943744a5ea0/said/optimize/blendshape_coeffs.py",
  "function_signature_candidates": [
    {
      "line": 13,
      "text": "def __init__("
    },
    {
      "line": 39,
      "text": "def optimize("
    },
    {
      "line": 76,
      "text": "def __init__("
    },
    {
      "line": 103,
      "text": "def optimize("
    },
    {
      "line": 165,
      "text": "def compute_g(self, seq_len: int) -> sp.csc_matrix:"
    }
  ]
}

```text

"""Define the optimization problem to parse blendshape coefficients
"""
from typing import List, Optional
import numpy as np
from qpsolvers import solve_qp
from scipy import linalg as la
from scipy import sparse as sp


class OptimizationProblemSingle:
    """Autoregressive optimization of pseudo-gt coefficients"""

    def __init__(
        self,
        neutral_vector: np.ndarray,
        blendshapes_matrix: np.ndarray,
    ) -> None:
        """Constructor of OptimizationProblemSingle

        Parameters
        ----------
        neutral_vector : np.ndarray
            (3|V|, 1), neutral mesh's vertices vector
        blendshapes_matrix : np.ndarray
            (3|V|, num_blendshapes), [b1 | b2 | ... | b_N] blendshape mesh's vertices vectors
        """
        self.neutral_vector = neutral_vector

        self.num_blendshapes = blendshapes_matrix.shape[1]

        # B_delta
        self.blendshapes_matrix_delta = blendshapes_matrix - self.neutral_vector

        self.P = self.blendshapes_matrix_delta.T @ self.blendshapes_matrix_delta

        self.lbw = np.zeros(self.num_blendshapes)
        self.ubw = np.ones(self.num_blendshapes)

    def optimize(
        self, vertices_vector: np.ndarray, init_vals: Optional[np.ndarray]
    ) -> np.ndarray:
        """Solve the optimization problem

        Parameters
        ----------
        vertices_vector : np.ndarray
            (3|V|, 1), Target mesh's vertices vector to be optimized
        init_vals : Optional[np.ndarray]
            (num_blendshapes,), Initial point of the optimization

        Returns
        -------
        np.ndarray
            (num_blendshapes,), optimization solution
        """
        q = (
            self.blendshapes_matrix_delta.T @ (self.neutral_vector - vertices_vector)
        ).reshape(-1)

        w_sol = solve_qp(
            self.P,
            q,
            lb=self.lbw,
            ub=self.ubw,
            solver="cvxopt",
            initvals=init_vals,
        )
        w_sol = np.clip(w_sol, self.lbw, self.ubw)

        return w_sol


class OptimizationProblemFull:
    """Full optimization of pseudo-gt coefficients"""

    def __init__(
        self,
        neutral_vector: np.ndarray,
        blendshapes_matrix: np.ndarray,
    ) -> None:
        """Constructor of OptimizationProblemFull

        Parameters
        ----------
        neutral_vector : np.ndarray
            (3|V|, 1), neutral mesh's vertices vector
        blendshapes_matrix : np.ndarray
            (3|V|, num_blendshapes), [b1 | b2 | ... | b_N] blendshape mesh's vertices vectors
        """
        self.neutral_vector = neutral_vector
        self.num_blendshapes = blendshapes_matrix.shape[1]

        # B_delta
        self.blendshapes_matrix_delta = blendshapes_matrix - self.neutral_vector
        self.btb = self.blendshapes_matrix_delta.T @ self.blendshapes_matrix_delta

        # D
        eye = sp.identity(self.num_blendshapes, dtype="int", format="csc")
        self.dipole_eye = sp.bmat([[eye], [-eye]])

        self.g_offset = sp.csc_matrix((0, self.num_blendshapes), dtype="int")

    def optimize(
        self,
        vertices_vector_list: List[np.ndarray],
        init_vals: Optional[np.ndarray] = None,
        delta: float = 0.1,
    ) -> np.ndarray:
        """Solve the optimization problem

        Parameters
        ----------
        vertices_vector_list : List[np.ndarray]
            (3|V|, 1), List of the target mesh sequence's vertices vectors to be optimized
        init_vals: Optional[np.ndarray]
            (seq_len, num_blendshapes), initial value of the optimization
        delta : float, optional
            Bound of the |w_{t} - w_{t+1}|, by default 0.1

        Returns
        -------
        np.ndarray
            (seq_len, num_blendshapes), optimization solution
        """
        seq_len = len(vertices_vector_list)

        # Compute P
        p = la.block_diag(*[self.btb for _ in range(seq_len)])

        # Compute q
        q = np.vstack(
            [
                self.blendshapes_matrix_delta.T @ (self.neutral_vector - vvector)
                for vvector in vertices_vector_list
            ]
        ).reshape(-1)

        # Compute G
        g = self.compute_g(seq_len)

        # Set h
        h = np.full(g.shape[0], delta)

        # Uppler/lower bound
        lbw = np.zeros(self.num_blendshapes * seq_len)
        ubw = np.ones(self.num_blendshapes * seq_len)

        # Solve the problem
        w_sol = solve_qp(
            P=p,
            q=q,
            G=g,
            h=h,
            lb=lbw,
            ub=ubw,
            solver="cvxopt",
            initvals=None if init_vals is None else init_vals.reshape(-1),
        )
        w_sol = np.clip(w_sol, lbw, ubw)

        w_vectors_matrix = w_sol.reshape(seq_len, self.num_blendshapes)

        return w_vectors_matrix

    def compute_g(self, seq_len: int) -> sp.csc_matrix:
        """Compute G efficiently using sparse matrix

        Parameters
        ----------
        seq_len : int
            Length of the target mesh sequence

        Returns
        -------
        sp.csc_matrix
            ((seq_len - 1) * num_blendshapes, seq_len * num_blendshapes), computed G
        """
        diag_g = sp.block_diag(
            [self.dipole_eye for _ in range(seq_len - 1)], format="csc"
        )

        pos_g = sp.block_diag((diag_g, self.g_offset), format="csc")
        neg_g = sp.block_diag((self.g_offset, diag_g), format="csc")

        g = pos_g - neg_g
        return g


```

## Source file: script/optimize_blendshape_coeffs.py

{
  "path": "script/optimize_blendshape_coeffs.py",
  "blob_sha": "8fee7a347f88187e9f63098b475c19620e8d155e",
  "sha256": "3a109e0ddceee61b700a08ede72f264cc89acbda92c51fc9f388c0ef1631edc5",
  "lines": 166,
  "bytes": 5342,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 29,
    "gum": 7,
    "flame": 1
  },
  "url": "https://github.com/yunik1004/SAiD/blob/bb7b18018358d3156569d8051f386943744a5ea0/script/optimize_blendshape_coeffs.py",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "def main():"
    },
    {
      "line": 68,
      "text": "def coeff_out_path(person_id: str, seq_id: int, exists_ok: bool = False) -> str:"
    }
  ]
}

```text

"""Generate the Pseudo-GT blendshape coefficients
"""
import argparse
import numpy as np
import os
import pathlib
from tqdm import tqdm
from said.util.blendshape import save_blendshape_coeffs
from said.util.parser import parse_list
from said.optimize.blendshape_coeffs import OptimizationProblemFull
from dataset.dataset_voca import BlendVOCADataset, BlendVOCAPseudoGTOptDataset


def main():
    """Main function"""
    default_data_dir = pathlib.Path(__file__).resolve().parent.parent / "data"

    parser = argparse.ArgumentParser(
        description="Generate the Pseudo-GT blendshape coefficients by solving the optimization problem"
    )
    parser.add_argument(
        "--neutrals_dir",
        type=str,
        default="../BlendVOCA/templates_head",
        help="Directory of the neutral meshes",
    )
    parser.add_argument(
        "--blendshapes_dir",
        type=str,
        default="../BlendVOCA/blendshapes_head",
        help="Directory of the blendshape meshes",
    )
    parser.add_argument(
        "--mesh_seqs_dir",
        type=str,
        default="../BlendVOCA/unposedcleaneddata",
        help="Directory of the mesh sequences",
    )
    parser.add_argument(
        "--blendshape_list_path",
        type=str,
        default=default_data_dir / "ARKit_blendshapes.txt",
        help="List of the blendshapes",
    )
    parser.add_argument(
        "--head_idx_path",
        type=str,
        default=default_data_dir / "FLAME_head_idx.txt",
        help="List of the head indices. Empty string will disable this option.",
    )
    parser.add_argument(
        "--blendshapes_coeffs_out_dir",
        type=str,
        default="../output_coeffs",
        help="Directory of the output coefficients",
    )
    args = parser.parse_args()

    neutrals_dir = args.neutrals_dir
    blendshapes_dir = args.blendshapes_dir
    mesh_seqs_dir = args.mesh_seqs_dir

    blendshape_list_path = args.blendshape_list_path
    head_idx_path = args.head_idx_path

    blendshapes_coeffs_out_dir = args.blendshapes_coeffs_out_dir

    def coeff_out_path(person_id: str, seq_id: int, exists_ok: bool = False) -> str:
        """Generate the output path of the coefficients.
        If you want to change the output file name, then change this function

        Parameters
        ----------
        person_id : str
            Person id
        seq_id : int
            Sequence id
        exists_ok : bool, optional
            If false, raise error when the file already exists, by default False

        Returns
        -------
        str
            Output path of the coefficients
        """
        out_dir = os.path.join(blendshapes_coeffs_out_dir, person_id)
        try:
            os.makedirs(out_dir)
        except OSError:
            if not exists_ok:
                raise "Directory already exists"

        out_path = os.path.join(out_dir, f"sentence{seq_id:02}.csv")

        return out_path

    # Parse blendshape name
    blendshape_name_list = parse_list(blendshape_list_path, str)

    # Parse head indices
    head_idx_list = None if head_idx_path == "" else parse_list(head_idx_path, int)

    dataset = BlendVOCAPseudoGTOptDataset(
        neutrals_dir, blendshapes_dir, mesh_seqs_dir, blendshape_name_list
    )

    person_id_list = (
        BlendVOCADataset.person_ids_train
        + BlendVOCADataset.person_ids_val
        + BlendVOCADataset.person_ids_test
    )
    seq_id_list = BlendVOCADataset.sentence_ids

    for person_id in tqdm(person_id_list):
        bl_out = dataset.get_blendshapes(person_id)

        neutral_mesh = bl_out.neutral
        blendshapes_meshes_dict = bl_out.blendshapes

        neutral_vertices = neutral_mesh.vertices
        blendshapes_vertices_list = [
            blendshapes_meshes_dict[name].vertices for name in blendshape_name_list
        ]

        neutral_vector = neutral_vertices.reshape((-1, 1))
        blendshape_vectors = []
        for v in blendshapes_vertices_list:
            blendshape_vectors.append(v.reshape((-1, 1)))

        blendshapes_matrix = np.concatenate(blendshape_vectors, axis=1)

        # Define the optimization problem
        opt_prob = OptimizationProblemFull(neutral_vector, blendshapes_matrix)

        for sdx, seq_id in enumerate(tqdm(seq_id_list, leave=False)):
            mesh_seq_list = dataset.get_mesh_seq(person_id, seq_id)
            if len(mesh_seq_list) == 0:
                continue

            mesh_seq_vertices_list = [mesh.vertices for mesh in mesh_seq_list]
            mesh_seq_vertices_vector_list = []
            if head_idx_list is None:
                mesh_seq_vertices_vector_list = [
                    vertices.reshape((-1, 1)) for vertices in mesh_seq_vertices_list
                ]
            else:
                mesh_seq_vertices_vector_list = [
                    vertices[head_idx_list].reshape((-1, 1))
                    for vertices in mesh_seq_vertices_list
                ]

            # Solve Optimization problem
            w_soln = opt_prob.optimize(mesh_seq_vertices_vector_list)

            # Save outputs
            out_path = coeff_out_path(person_id, seq_id, sdx > 0)

            save_blendshape_coeffs(
                w_soln,
                blendshape_name_list,
                out_path,
            )


if __name__ == "__main__":
    main()


```
