# Facial rendering source scan: aelzeiny/Animoji

Repository: https://github.com/aelzeiny/Animoji

Pinned source revision: 89b64705b8c565f55cbdb0a6fb23d2d58c5d18ef

Discovery topic: blendshape

Description: Unreal Engine Livelink App + Apple ARkit Blendshapes = Animoji Clone

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: animoji.py

{
  "path": "animoji.py",
  "blob_sha": "cbcbc4dd7f2abb4318cc2b3fb696d186e52a0424",
  "sha256": "f588a20a666c3a8305eb11ce8f567e21b5fc4393fbdf1f7bebe86aeaff59033b",
  "lines": 89,
  "bytes": 2722,
  "selection_score": 0,
  "keyword_line_counts": {
    "blendshape": 2,
    "blend_shape": 2,
    "face": 2
  },
  "url": "https://github.com/aelzeiny/Animoji/blob/89b64705b8c565f55cbdb0a6fb23d2d58c5d18ef/animoji.py",
  "function_signature_candidates": [
    {
      "line": 17,
      "text": "def render_morphs(v, scene, buf):"
    },
    {
      "line": 43,
      "text": "def livelink_udp_spammer(v, buf):"
    },
    {
      "line": 56,
      "text": "def livelink_file_reader(v, buf):"
    }
  ]
}

```text

import time
from threading import Thread

import pyrender
import numpy as np
import tqdm
import gltflib
import socket
from multiprocessing import Array

import renderer
from livelink import BlendShapes, LivelinkBuffer

FPS = 1/60


def render_morphs(v, scene, buf):
    last_rendered = None
    with tqdm.tqdm() as pbar:
        while v.is_active:
            if not hasattr(buf, 'value'):
                time.sleep(FPS)
                continue
            livelink_data = buf.value
            if last_rendered == livelink_data:
                time.sleep(FPS)
                continue
            last_rendered = livelink_data
            livelink = LivelinkBuffer(livelink_data).parse_livelink()
            with v:
                if livelink is not None:
                    for mesh_idx, mesh in enumerate(scene.meshes):
                        if mesh.weights is not None:
                            mesh_mapping = mesh_mappings[mesh_idx]
                            if mesh_mapping:
                                mesh.weights = livelink.face_blend_shapes_values[mesh_mapping]
                            else:
                                mesh.weights = livelink.face_blend_shapes_values[:len(mesh.weights)]
            pbar.update()
            time.sleep(FPS)


def livelink_udp_spammer(v, buf):
    """Spams UDP socket reads and updates the multithreaded array"""
    UDP_IP = '0.0.0.0'
    UDP_PORT = 11111

    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.bind((UDP_IP, UDP_PORT))
    while v.is_active():
        livelink_data, _ = sock.recvfrom(500)
        buf.value = livelink_data
        # time.sleep(FPS)


def livelink_file_reader(v, buf):
    with open('./experimentation/livelink.udp', 'rb') as f:
        livelink = LivelinkBuffer(f.read())
    prev_offset = 0
    while v.is_active():
        livelink.parse_livelink()
        curr_offset = livelink.offset
        buf.value = livelink.buffer[prev_offset: curr_offset]
        prev_offset = curr_offset
        time.sleep(1/20)


if __name__ == '__main__':
    gltf = gltflib.GLTF.load('./model_me.glb', load_file_resources=True)
    mesh_mappings = [
        [
            int(BlendShapes[target_name[0].upper() + target_name[1:]])
            for target_name in mesh.extras['targetNames']
        ] if 'targetNames' in mesh.extras else None
        for mesh in gltf.model.meshes
    ]

    scene = pyrender.Scene.from_gltflib_scene(gltf)

    buf = Array('b', 500)
    v = renderer.FakeWebcamRenderer(scene)
    t = Thread(target=render_morphs, args=(v, scene, buf))
    u = Thread(target=livelink_udp_spammer, args=(v, buf))
    u.start()
    t.start()
    v.start()

    # t.join(timeout=3)
    # u.join(timeout=3)


```

## Source file: livelink.py

{
  "path": "livelink.py",
  "blob_sha": "c79715631fa351686aa136c1815871e0c4feee3a",
  "sha256": "45ba8cf7eae2857d09551b5cd94a6135e695c6ed5a471a7d965d35e8ba3878b4",
  "lines": 158,
  "bytes": 4389,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 23,
    "blendshape": 8,
    "blend_shape": 4,
    "jaw": 5,
    "mouth": 24,
    "tongue": 1,
    "rig": 24,
    "face": 3,
    "lip": 1
  },
  "url": "https://github.com/aelzeiny/Animoji/blob/89b64705b8c565f55cbdb0a6fb23d2d58c5d18ef/livelink.py",
  "function_signature_candidates": [
    {
      "line": 106,
      "text": "def __init__(self, buffer, offset=0):"
    },
    {
      "line": 110,
      "text": "def parse(self, unpack_format: str):"
    },
    {
      "line": 117,
      "text": "def parse_numpy(self, dtype, count):"
    },
    {
      "line": 123,
      "text": "def parse_str(self):"
    },
    {
      "line": 131,
      "text": "def parse_frametime(self):"
    },
    {
      "line": 134,
      "text": "def parse_livelink(self):"
    }
  ]
}

```text

"""
Parses Unreal Engine 5's LiveLink UDP Stream.


"""
from enum import IntEnum

import numpy as np
import struct
from typing import NamedTuple, List


class BlendShapes(IntEnum):
    # Left eye blend shapes
    EyeBlinkLeft = 0
    EyeLookDownLeft = 1
    EyeLookInLeft = 2
    EyeLookOutLeft = 3
    EyeLookUpLeft = 4
    EyeSquintLeft = 5
    EyeWideLeft = 6
    # Right eye blend shapes
    EyeBlinkRight = 7
    EyeLookDownRight = 8
    EyeLookInRight = 9
    EyeLookOutRight = 10
    EyeLookUpRight = 11
    EyeSquintRight = 12
    EyeWideRight = 13
    # Jaw blend shapes
    JawForward = 14
    JawLeft = 15
    JawRight = 16
    JawOpen = 17
    # Mouth blend shapes
    MouthClose = 18
    MouthFunnel = 19
    MouthPucker = 20
    MouthLeft = 21
    MouthRight = 22
    MouthSmileLeft = 23
    MouthSmileRight = 24
    MouthFrownLeft = 25
    MouthFrownRight = 26
    MouthDimpleLeft = 27
    MouthDimpleRight = 28
    MouthStretchLeft = 29
    MouthStretchRight = 30
    MouthRollLower = 31
    MouthRollUpper = 32
    MouthShrugLower = 33
    MouthShrugUpper = 34
    MouthPressLeft = 35
    MouthPressRight = 36
    MouthLowerDownLeft = 37
    MouthLowerDownRight = 38
    MouthUpperUpLeft = 39
    MouthUpperUpRight = 40
    # Brow blend shapes
    BrowDownLeft = 41
    BrowDownRight = 42
    BrowInnerUp = 43
    BrowOuterUpLeft = 44
    BrowOuterUpRight = 45
    # Cheek blend shapes
    CheekPuff = 46
    CheekSquintLeft = 47
    CheekSquintRight = 48
    # Nose blend shapes
    NoseSneerLeft = 49
    NoseSneerRight = 50
    TongueOut = 51
    # Treat the head rotation as curves for LiveLink support
    HeadYaw = 52
    HeadPitch = 53
    HeadRoll = 54
    # Treat eye rotation as curves for LiveLink support
    LeftEyeYaw = 55
    LeftEyePitch = 56
    LeftEyeRoll = 57
    RightEyeYaw = 58
    RightEyePitch = 59
    RightEyeRoll = 60


BLEND_SHAPE_ARR = list(BlendShapes)


class FrameTime(NamedTuple):
    framenum: int
    subframe: int
    fps_numerator: int
    fps_denominator: int


class Livelink(NamedTuple):
    packet_ver: int
    device_id: str
    subject_name: str
    frametime: FrameTime
    blendshape_count: int
    face_blend_shapes_values: List[float]


class LivelinkBuffer:
    def __init__(self, buffer, offset=0):
        self.buffer = buffer
        self.offset = offset

    def parse(self, unpack_format: str):
        if not unpack_format.startswith('>') and not unpack_format.startswith('<'):
            unpack_format = '>' + unpack_format
        retval = struct.unpack_from(unpack_format, self.buffer, self.offset)
        self.offset += struct.calcsize(unpack_format)
        return retval

    def parse_numpy(self, dtype, count):
        np_dtype = np.dtype(dtype).newbyteorder('>')
        retval = np.frombuffer(self.buffer, offset=self.offset, dtype=np_dtype, count=count)
        self.offset += struct.calcsize(np_dtype.kind) * count
        return retval

    def parse_str(self):
        length, *_ = self.parse('i')
        if length > 0:
            retval = self.buffer[self.offset:self.offset + length]
            self.offset += length
            return retval.decode('utf8')
        return ''

    def parse_frametime(self):
        return FrameTime(*self.parse('ifii'))

    def parse_livelink(self):
        packet_ver, *_ = self.parse('B')
        if packet_ver != 6:
            raise NotImplementedError("Cannot support packet version: " + str(packet_ver))
        try:
            device_id, *_ = self.parse_str()
            subject_name = self.parse_str()
            frametime = self.parse_frametime()
            blendshape_count, *_ = self.parse('B')
            # In Unreal Engine code you have a check condition before
            # blendshape_count is serialized. If the next byte is
            # equal to the packet version, this is the start of a new packet
            if blendshape_count != len(BlendShapes):
                self.offset -= struct.calcsize('B')
                return None
            face_blend_shapes_values = self.parse_numpy(np.float32, blendshape_count).clip(0, 1)
            return Livelink(
                packet_ver, device_id, subject_name,
                frametime,
                blendshape_count, face_blend_shapes_values
            )
        # Buffer overflows and dropped packets are handled in this way.
        except struct.error:
            return None



```

## Source file: renderer.py

{
  "path": "renderer.py",
  "blob_sha": "953a95b61824f276116af70040430b396030c37c",
  "sha256": "efe1abf4a7531e6581c0dc7abca8bfcb136ea5cb1cf5bcf4efb4ab33913e7eb6",
  "lines": 107,
  "bytes": 3368,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/aelzeiny/Animoji/blob/89b64705b8c565f55cbdb0a6fb23d2d58c5d18ef/renderer.py",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "def create_camera():"
    },
    {
      "line": 20,
      "text": "def create_lights():"
    },
    {
      "line": 25,
      "text": "def __init__(self, scene):"
    },
    {
      "line": 37,
      "text": "def start(self):"
    },
    {
      "line": 40,
      "text": "def is_active(self):"
    },
    {
      "line": 43,
      "text": "def __enter__(self):"
    },
    {
      "line": 46,
      "text": "def __exit__(self, *args, **kwargs):"
    },
    {
      "line": 51,
      "text": "def __init__(self, scene):"
    },
    {
      "line": 68,
      "text": "def start(self):"
    },
    {
      "line": 71,
      "text": "def is_active(self):"
    },
    {
      "line": 74,
      "text": "def __enter__(self):"
    },
    {
      "line": 77,
      "text": "def __exit__(self, *args, **kwargs):"
    },
    {
      "line": 82,
      "text": "def start(self):"
    },
    {
      "line": 99,
      "text": "def start(self):"
    }
  ]
}

```text

import abc
from threading import RLock
import time

import numpy as np
import pyrender
import pyvirtualcam


def create_camera():
    return pyrender.Node(
        camera=pyrender.PerspectiveCamera(yfov=1.0471975511965976, zfar=100, znear=0.0004926449574960696),
        matrix=np.array([[0.9999451448020004, 0.010473495475301227, -0.00011523643331192809, 2.632996104694026e-05], [-0.010456117703794985, 0.9988101239021526, 0.047634084363313206, 0.0021249022974223746], [0.0006139946832835854, -0.047630266460473435, 0.9988648460765015, 0.0058722449231459], [0.0, 0.0, 0.0, 1.0]]),
        scale=np.ones(3),
        translation=np.array([0.004293392833761634, 0.0018634976900897661, 0.004657140988373949]),
        rotation=np.array([0.27059805007309856, 0.27059805007309856, 0.6532814824381883, 0.6532814824381883]),
    )


def create_lights():
    return pyrender.Viewer._create_raymond_lights(None)


class PyrenderRenderer:
    def __init__(self, scene):
        # CREATE CAMERA + LIGHTS
        cam_node = create_camera()
        scene.add_node(cam_node)
        scene.main_camera_node = cam_node

        # CREATE LIGHTS
        for light in create_lights():
            scene.add_node(light, parent_node=cam_node)

        self.viewer = pyrender.Viewer(scene, use_raymond_lighting=True, auto_start=False)

    def start(self):
        self.viewer.start()

    def is_active(self):
        return self.viewer.is_active

    def __enter__(self):
        self.viewer.render_lock.acquire()

    def __exit__(self, *args, **kwargs):
        self.viewer.render_lock.release()


class BufferFrameRenderer(abc.ABC):
    def __init__(self, scene):
        self.scene = scene

        # CREATE CAMERA
        camera_node = create_camera()
        scene.add_node(camera_node)
        scene.main_camera_node = camera_node

        # CREATE LIGHTS
        for light in pyrender.Viewer._create_raymond_lights(None):
            scene.add_node(light, parent_node=camera_node)

        self._is_active = False
        self.render_lock = RLock()
        self.renderer = pyrender.OffscreenRenderer(640,480)

    @abc.abstractmethod
    def start(self):
        pass

    def is_active(self):
        return self._is_active

    def __enter__(self):
        self.render_lock.acquire()

    def __exit__(self, *args, **kwargs):
        self.render_lock.release()


class OpenCvRenderer(BufferFrameRenderer):
    def start(self):
        import cv2
        self._is_active = True

        while True:
            with self:
                color, _ = self.renderer.render(self.scene, pyrender.RenderFlags.ALL_SOLID)
                # RGB -> BGR
            color_bgr = cv2.cvtColor(color, cv2.COLOR_RGB2BGR)
            cv2.imshow('memoji', color_bgr)
            if cv2.waitKey(1) & 0xFF == ord('q'):
                break
        self._is_active = False
        cv2.destroyAllWindows()


class FakeWebcamRenderer(BufferFrameRenderer):
    def start(self):
        with pyvirtualcam.Camera(width=self.renderer.viewport_width, height=self.renderer.viewport_height, fps=30) as camera:
            self._is_active = True
            while True:
                with self:
                    color, _ = self.renderer.render(self.scene, pyrender.RenderFlags.ALL_SOLID)
                camera.send(color)
                camera.sleep_until_next_frame()
        self._is_active = False


```
