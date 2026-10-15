# Facial rendering source scan: Prohell/vtbfacap

Repository: https://github.com/Prohell/vtbfacap

Pinned source revision: eca81442ffba96a902a57f13111313839f52d62b

Discovery topic: facial mocap

Description: Facial mocap python to unity/unreal

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"eye": ["images/live2d_eye.gif"], "blendshape": ["python_vtbfacap/vtbfacap/face_data/blendshape.py"], "mouth": ["images/live2d_mouth.gif"], "iris": ["python_vtbfacap/data/iris_landmark.tflite", "python_vtbfacap/vtbfacap/face_tracking/face_and_iris_tracking.py", "python_vtbfacap/vtbfacap/face_tracking/iris_tracking.py"], "face": ["python_vtbfacap/vtbfacap/face_data/__init__.py", "python_vtbfacap/vtbfacap/face_data/blendshape.py", "python_vtbfacap/vtbfacap/face_data/landmarks.py", "python_vtbfacap/vtbfacap/face_data/stabilizer.py", "python_vtbfacap/vtbfacap/face_tracking/__init__.py", "python_vtbfacap/vtbfacap/face_tracking/face_and_iris_tracking.py", "python_vtbfacap/vtbfacap/face_tracking/face_tracking.py", "python_vtbfacap/vtbfacap/face_tracking/iris_tracking.py"]}

## Source file: python_vtbfacap/vtbfacap/face_tracking/face_and_iris_tracking.py

{
  "path": "python_vtbfacap/vtbfacap/face_tracking/face_and_iris_tracking.py",
  "blob_sha": "ca372f4bf880bd52f0ab06365338aa9b1dcedf73",
  "sha256": "36f5462028ee8b237586d002705aa211ebfb80e8acb9b938ee7bb0746c0f4c9a",
  "lines": 112,
  "bytes": 4369,
  "selection_score": 30,
  "keyword_line_counts": {
    "eye": 16,
    "iris": 12,
    "rig": 6,
    "face": 18
  },
  "url": "https://github.com/Prohell/vtbfacap/blob/eca81442ffba96a902a57f13111313839f52d62b/python_vtbfacap/vtbfacap/face_tracking/face_and_iris_tracking.py",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "def __init__(self):"
    },
    {
      "line": 18,
      "text": "def _get_bounding_box(self, landmarks: Vectors):"
    },
    {
      "line": 28,
      "text": "def _get_rotated_box(self, landmarks: Vectors, box_up_vector: Vectors):"
    },
    {
      "line": 41,
      "text": "def _crop_frame(self, frame, box):"
    },
    {
      "line": 56,
      "text": "def _mask_frame(self, frame, contour: Vectors):"
    },
    {
      "line": 62,
      "text": "def _valid_points(self, frame_shape, landmarks):"
    },
    {
      "line": 69,
      "text": "def process(self, frame) -> FaceAndIrisLandmarks:"
    },
    {
      "line": 90,
      "text": "def process_eye(self, frame, face_landmarks, left):"
    }
  ]
}

```text

import math

import cv2
import numpy as np

from ..face_data import FaceAndIrisLandmarks
from .face_tracking import FaceTracking
from .iris_tracking import IrisTrackingV2
from ..math_utils import Vectors, Transform2
from ..settings import settings


class FaceAndIrisTracking:
    def __init__(self):
        self.face_tracking = FaceTracking()
        self.iris_tracking = IrisTrackingV2()

    def _get_bounding_box(self, landmarks: Vectors):
        min_xy = np.amin(landmarks.no_z(), axis=0)
        max_xy = np.amax(landmarks.no_z(), axis=0)
        # fmt: off
        return Vectors.init([min_xy,  # left top
                            [min_xy[0], max_xy[1]],  # left bottom
                            max_xy,  # right bottom
                            [max_xy[0], min_xy[1]]])  # right top
        # fmt: on

    def _get_rotated_box(self, landmarks: Vectors, box_up_vector: Vectors):
        # rotation between box up vector and screen up vector
        rotation_angle = math.copysign(box_up_vector.no_z().angle(Vectors.init([0, -1])), box_up_vector[0])
        R = Transform2.rotation(rotation_angle)

        center = landmarks.center()
        landmarks_local = (landmarks - center).transform(R.T)

        box = self._get_bounding_box(landmarks_local)
        box = (box.pad_z(1).transform(R) + center).no_z()

        return box

    def _crop_frame(self, frame, box):
        width = (box[3] - box[0]).length()
        height = (box[1] - box[0]).length()

        src_box = box.astype("float32")
        # fmt: off
        dst_box = Vectors.init([[0, 0],
                                [0, height - 1],
                                [width - 1, height - 1],
                                [width - 1, 0]], dtype="float32")
        # fmt: on

        M = cv2.getPerspectiveTransform(src_box, dst_box)
        return cv2.warpPerspective(frame, M, (int(width), int(height)))

    def _mask_frame(self, frame, contour: Vectors):
        """make area outside contour white"""
        mask = np.full(frame.shape, 255, dtype=frame.dtype)
        cv2.fillPoly(mask, [contour.astype(int)], (0, 0, 0))
        return cv2.bitwise_or(frame, mask)

    def _valid_points(self, frame_shape, landmarks):
        """check if points within frame"""
        frame_height = frame_shape[0]
        frame_width = frame_shape[1]
        m = settings.normalize_multiplier
        return np.all(landmarks[:, :2] >= 0) and np.all(landmarks[:, 0] < frame_width / m) and np.all(landmarks[:, 1] < frame_height / m)

    def process(self, frame) -> FaceAndIrisLandmarks:
        face_landmarks = self.face_tracking.process(frame)

        if face_landmarks is None:
            return None

        is_left_eye_visible = face_landmarks.yaw() < 0.7 and self._valid_points(frame.shape, face_landmarks.left_eye_contour_2nd_innermost)
        is_right_eye_visible = face_landmarks.yaw() > -0.7 and self._valid_points(frame.shape, face_landmarks.right_eye_contour_2nd_innermost)

        if is_left_eye_visible:
            iris_landmarks = self.process_eye(frame, face_landmarks, left=True)
            if iris_landmarks is not None:
                face_landmarks.set_left_eye_landmarks(iris_landmarks)

        if is_right_eye_visible:
            iris_landmarks = self.process_eye(frame, face_landmarks, left=False)
            if iris_landmarks is not None:
                face_landmarks.set_right_eye_landmarks(iris_landmarks)

        return face_landmarks

    def process_eye(self, frame, face_landmarks, left):
        eye_contour = getattr(face_landmarks, "left_eye_contour_2nd_innermost" if left else "right_eye_contour_2nd_innermost")

        m = settings.normalize_multiplier
        box = self._get_rotated_box(eye_contour, face_landmarks.up())

        eye_frame = self._mask_frame(frame, eye_contour.no_z() * m)
        eye_frame = self._crop_frame(eye_frame, box * m)

        landmarks = self.iris_tracking.process(eye_frame)

        if landmarks is not None:
            # transform back onto face
            global_o = box[0]
            global_x = box[3] - box[0]
            global_y = box[1] - box[0]
            X = (landmarks[:, 0] / eye_frame.shape[1]).outer(global_x)
            Y = (landmarks[:, 1] / eye_frame.shape[0]).outer(global_y)
            landmarks = X + Y + global_o

            landmarks = landmarks.pad_z(1)

        return landmarks


```

## Source file: python_vtbfacap/vtbfacap/face_data/blendshape.py

{
  "path": "python_vtbfacap/vtbfacap/face_data/blendshape.py",
  "blob_sha": "86eff6906a7bd489e1a10c511f4e9c6de3b1ea03",
  "sha256": "25ef4cbb9bf5659e9b209a283e63d194aa26792878a377a2cfb61cdf33423c96",
  "lines": 128,
  "bytes": 5061,
  "selection_score": 18,
  "keyword_line_counts": {
    "eye": 13,
    "blendshape": 2,
    "mouth": 5,
    "iris": 24,
    "rig": 34,
    "face": 7,
    "lip": 1
  },
  "url": "https://github.com/Prohell/vtbfacap/blob/eca81442ffba96a902a57f13111313839f52d62b/python_vtbfacap/vtbfacap/face_data/blendshape.py",
  "function_signature_candidates": [
    {
      "line": 21,
      "text": "def __post_init__(self):"
    },
    {
      "line": 28,
      "text": "def __setattr__(self, name, value):"
    },
    {
      "line": 44,
      "text": "def update(self, landmarks: FaceAndIrisLandmarks):"
    },
    {
      "line": 56,
      "text": "def compute_eye_open(self, landmarks):"
    },
    {
      "line": 81,
      "text": "def compute_iris(self, landmarks):"
    },
    {
      "line": 90,
      "text": "def valid(value):"
    },
    {
      "line": 126,
      "text": "def bytes(self):"
    }
  ]
}

```text

from dataclasses import dataclass

from .stabilizer import Stabilizer
from .landmarks import FaceAndIrisLandmarks
from ..math_utils import Mathf
from ..settings import face_settings


@dataclass
class Live2dShape:
    yaw: float = 0.5
    pitch: float = 0.5
    roll: float = 0.5
    mouth_open: float = 0
    mouth_form: float = 0.5
    left_eye_open: float = 1
    right_eye_open: float = 1
    iris_x: float = 0.5
    iris_y: float = 0.5

    def __post_init__(self):
        """create stabilizers"""
        for field_name in self.__dataclass_fields__:
            stabilize_param = getattr(face_settings.blendshape_adjust, f"stablize_{field_name}", [])
            if stabilize_param is not None:
                setattr(self, f"_{field_name}_stabilizer", Stabilizer(*stabilize_param))

    def __setattr__(self, name, value):
        """adjust value before assign (map range & stabilize)"""
        if name in self.__dataclass_fields__:
            if value is None:
                return

            minmax_range = getattr(face_settings.blendshape_adjust, f"{name}_range", None)
            stabilizer = getattr(self, f"_{name}_stabilizer", None)

            if minmax_range is not None:
                value = Mathf.map(value, *minmax_range)
            if stabilizer is not None:
                value = stabilizer.correct(value)

        super().__setattr__(name, value)

    def update(self, landmarks: FaceAndIrisLandmarks):
        face_width = (landmarks.right_edge3 - landmarks.left_edge3).length()
        mouth_width = (landmarks.mouth_corner_left - landmarks.mouth_corner_right).length()

        self.yaw = landmarks.yaw() + 0.5  # shift center to 0.5
        self.pitch = landmarks.pitch() + 0.5
        self.roll = landmarks.roll() + 0.5
        self.mouth_open = (landmarks.upper_lip_inner_edge_samples - landmarks.lower_lip_inner_edge_samples).center().length() / mouth_width
        self.mouth_form = mouth_width / face_width
        self.left_eye_open, self.right_eye_open = self.compute_eye_open(landmarks)
        self.iris_x, self.iris_y = self.compute_iris(landmarks)

    def compute_eye_open(self, landmarks):
        left_open = None
        right_open = None

        use_left = self.yaw < 0.6 and landmarks.left_iris_landmarks is not None
        use_right = self.yaw > 0.4 and landmarks.right_iris_landmarks is not None

        right = landmarks.right()
        up = landmarks.up()
        reference = (landmarks.eyes_middle1 - landmarks.eyes_middle2).project(right, up).length()

        if use_left:
            left_open = (landmarks.left_iris_up - landmarks.left_iris_down).project(right, up).length()
            left_open = left_open / reference

        if use_right:
            right_open = (landmarks.right_iris_up - landmarks.right_iris_down).project(right, up).length()
            right_open = right_open / reference

        if left_open is None:
            return right_open, right_open
        elif right_open is None:
            return left_open, left_open
        return left_open, right_open

    def compute_iris(self, landmarks):
        right = landmarks.right()
        up = landmarks.up()
        left_iris = landmarks.left_iris_center
        right_iris = landmarks.right_iris_center

        is_left_valid = False
        is_right_valid = False

        def valid(value):
            return value is not None and value >= 0 and value <= 1

        # left x y
        if left_iris is not None:
            X1 = (left_iris - landmarks.left_eye_right_corner).project(right)
            X2 = (landmarks.left_eye_left_corner - landmarks.left_eye_right_corner).project(right)
            Y1 = (left_iris - landmarks.left_eye_down_reference).project(up)
            Y2 = (landmarks.eyes_middle1 - landmarks.left_eye_down_reference).project(up)
            left_x = X1.length() / X2.length()
            left_y = Y1.length() / Y2.length()
            is_left_valid = valid(left_x) and valid(left_y)

        # right x y
        if right_iris is not None:
            X1 = (right_iris - landmarks.right_eye_right_corner).project(right)
            X2 = (landmarks.right_eye_left_corner - landmarks.right_eye_right_corner).project(right)
            Y1 = (right_iris - landmarks.right_eye_down_reference).project(up)
            Y2 = (landmarks.eyes_middle1 - landmarks.right_eye_down_reference).project(up)
            right_x = X1.length() / X2.length()
            right_y = Y1.length() / Y2.length()
            is_right_valid = valid(right_x) and valid(right_y)

        # final x y
        if is_left_valid and is_right_valid:
            iris_x = (left_x + right_x) / 2
            iris_y = (left_y + right_y) / 2
        elif is_left_valid:
            iris_x, iris_y = left_x, left_y
        elif is_right_valid:
            iris_x, iris_y = right_x, right_y
        else:
            iris_x, iris_y = None, None

        return iris_x, iris_y

    def bytes(self):
        msg = " ".join([f"{getattr(self, name):.4f}" for name in self.__dataclass_fields__])
        return msg.encode("ascii")


```

## Source file: python_vtbfacap/vtbfacap/face_tracking/iris_tracking.py

{
  "path": "python_vtbfacap/vtbfacap/face_tracking/iris_tracking.py",
  "blob_sha": "ce684dc947c6fa09055e29f67ac614906ba15aca",
  "sha256": "cbe3ab7cb41de79afdda2f5bc5bda40da47a16e0c8cc567b2db72ff2ba8b71b6",
  "lines": 143,
  "bytes": 5596,
  "selection_score": 18,
  "keyword_line_counts": {
    "eye": 9,
    "iris": 34,
    "rig": 1
  },
  "url": "https://github.com/Prohell/vtbfacap/blob/eca81442ffba96a902a57f13111313839f52d62b/python_vtbfacap/vtbfacap/face_tracking/iris_tracking.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def _binarize_image(self, frame, threshold):"
    },
    {
      "line": 20,
      "text": "def _get_2nd_biggest_contour(self, contours):"
    },
    {
      "line": 24,
      "text": "def _get_contour_center(self, contour):"
    },
    {
      "line": 33,
      "text": "def _find_iris_contour(self, binary_frame):"
    },
    {
      "line": 44,
      "text": "def _draw_contour_edge(self, shape, contour):"
    },
    {
      "line": 48,
      "text": "def _get_iris_vertices(self, iris_frame, iris_center: Vectors):"
    },
    {
      "line": 66,
      "text": "def _find_iris(self, frame, threshold=50):"
    },
    {
      "line": 77,
      "text": "def process(self, frame):"
    }
  ]
}

```text

import cv2 as cv
import numpy as np
# import tensorflow as tf

from ..math_utils import Vectors


class IrisTrackingV2:
    # TODO differentiate blink and looking down

    def _binarize_image(self, frame, threshold):
        kernel = np.ones((3, 3), np.uint8)
        new_frame = cv.bilateralFilter(frame, 10, 15, 15)
        new_frame = cv.erode(new_frame, kernel, iterations=3)
        new_frame = cv.equalizeHist(new_frame)
        new_frame = cv.threshold(new_frame, threshold, 255, cv.THRESH_BINARY)[1]
        new_frame = cv.dilate(new_frame, kernel, iterations=3)
        return new_frame

    def _get_2nd_biggest_contour(self, contours):
        contours = sorted(contours, key=cv.contourArea)
        return contours[-2] if len(contours) >= 2 else None

    def _get_contour_center(self, contour):
        moments = cv.moments(contour)
        if moments["m00"] != 0 and moments["m00"] != 0:
            iris_x = moments["m10"] / moments["m00"]
            iris_y = moments["m01"] / moments["m00"]
            return Vectors.init([iris_x, iris_y])

        return None

    def _find_iris_contour(self, binary_frame):
        binary_frame = binary_frame.astype(np.uint8)
        padded_frame = np.pad(binary_frame, 1, constant_values=1)  # pad to avoid contour touching edge
        contours, _ = cv.findContours(padded_frame, cv.RETR_TREE, cv.CHAIN_APPROX_NONE)[-2:]
        iris_contour = self._get_2nd_biggest_contour(contours)  # 1st biggest contour is frame edge

        if iris_contour is not None:
            iris_contour -= (1, 1)  # cancel pad border

        return iris_contour

    def _draw_contour_edge(self, shape, contour):
        white_frame = np.full(shape[:2], 0, dtype=np.uint8)
        return cv.drawContours(white_frame, [contour], -1, (255), thickness=0)

    def _get_iris_vertices(self, iris_frame, iris_center: Vectors):
        # find intersect with x position
        iris_x = int(iris_center[0])
        intersect = np.copy(iris_frame)
        intersect[:,:iris_x] *= 0
        intersect[:,iris_x+1:] *= 0
        intersect_coordinates = list(zip(*(np.nonzero(intersect.T))))

        if len(intersect_coordinates) == 0:
            return iris_center, iris_center

        top = min(intersect_coordinates, key=lambda p: p[1])
        bottom = max(intersect_coordinates, key=lambda p: p[1])

        eye_up = Vectors.init(top)
        eye_down = Vectors.init(bottom)
        return eye_up, eye_down

    def _find_iris(self, frame, threshold=50):
        gray_frame = cv.cvtColor(frame, cv.COLOR_BGR2GRAY)
        binary_frame = self._binarize_image(gray_frame, threshold)
        iris_contour = self._find_iris_contour(binary_frame)

        if iris_contour is None:
            return None, None

        iris_frame = self._draw_contour_edge(frame.shape, iris_contour)
        return iris_frame, iris_contour

    def process(self, frame):
        iris_frame, iris_contour = self._find_iris(frame)
        iris_center = self._get_contour_center(iris_contour) if iris_contour is not None else None

        if iris_center is None:
            return None

        iris_up, iris_down = self._get_iris_vertices(iris_frame, iris_center)

        if iris_up is None or iris_down is None:
            return None

        return Vectors.init([iris_center, iris_up, iris_down], dtype=float)


# # input: right eye frame with no rotation
# # using mediapipe model
# # https://github.com/Kazuhito00/iris-detection-using-py-mediapipe
# class IrisTracking:
#     def __init__(
#         self,
#         model_path=settings.iris_model_path,
#         num_threads=1,
#     ):
#         self._interpreter = tf.lite.Interpreter(model_path=model_path, num_threads=num_threads)
#         self._interpreter.allocate_tensors()
#         self._input_details = self._interpreter.get_input_details()
#         self._output_details = self._interpreter.get_output_details()

#     def process(self, image):
#         input_shape = self._input_details[0]["shape"]

#         # preprocess
#         img = cv.cvtColor(image, cv.COLOR_BGR2RGB)
#         img = img / 255.0
#         img_resized = tf.image.resize(img, [input_shape[1], input_shape[2]], method="bicubic", preserve_aspect_ratio=False)
#         img_input = img_resized.numpy()
#         img_input = (img_input - 0.5) / 0.5

#         reshape_img = img_input.reshape(1, input_shape[1], input_shape[2], input_shape[3])
#         tensor = tf.convert_to_tensor(reshape_img, dtype=tf.float32)

#         # compute
#         input_details_tensor_index = self._input_details[0]["index"]
#         self._interpreter.set_tensor(input_details_tensor_index, tensor)
#         self._interpreter.invoke()

#         # get result
#         output_details_tensor_index0 = self._output_details[0]["index"]
#         output_details_tensor_index1 = self._output_details[1]["index"]
#         eye_contour = self._interpreter.get_tensor(output_details_tensor_index0)
#         iris = self._interpreter.get_tensor(output_details_tensor_index1)

#         # postprocess
#         eye_contour = np.reshape(np.squeeze(eye_contour), (-1, 3))
#         iris = np.reshape(np.squeeze(iris), (-1, 3))
#         # normalize  # Q: what to do with z?
#         eye_contour[:, 0] /= input_shape[1]
#         eye_contour[:, 1] /= input_shape[2]
#         iris[:, 0] /= input_shape[1]
#         iris[:, 1] /= input_shape[2]

#         return eye_contour.view(Vectors), iris.view(Vectors)

#     def get_input_shape(self):
#         input_shape = self._input_details[0]["shape"]
#         return [input_shape[1], input_shape[2]]  # 64 x 64 model input


```

## Source file: python_vtbfacap/vtbfacap/face_tracking/face_tracking.py

{
  "path": "python_vtbfacap/vtbfacap/face_tracking/face_tracking.py",
  "blob_sha": "a8971d7d0d36dcc476fdcbab9b4aa43e54dcf093",
  "sha256": "002ede185ebb316ca0de4b34279dff503f679eb0b80b649bd2803c17cb23fcd8",
  "lines": 43,
  "bytes": 1620,
  "selection_score": 15,
  "keyword_line_counts": {
    "iris": 3,
    "face": 15
  },
  "url": "https://github.com/Prohell/vtbfacap/blob/eca81442ffba96a902a57f13111313839f52d62b/python_vtbfacap/vtbfacap/face_tracking/face_tracking.py",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "def __init__("
    },
    {
      "line": 23,
      "text": "def process(self, frame) -> FaceAndIrisLandmarks:"
    }
  ]
}

```text

import cv2
import mediapipe as mp

from ..face_data import FaceAndIrisLandmarks
from ..math_utils import Vectors
from ..settings import settings, face_settings


class FaceTracking:
    def __init__(
        self,
        # max_num_faces=1,  # assume always 1 face
        min_detection_confidence=0.7,
        min_tracking_confidence=0.7,
    ):
        mp_face_mesh = mp.solutions.face_mesh
        self.face_mesh = mp_face_mesh.FaceMesh(
            max_num_faces=1,
            min_detection_confidence=min_detection_confidence,
            min_tracking_confidence=min_tracking_confidence,
        )

    def process(self, frame) -> FaceAndIrisLandmarks:
        rgb_frame = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
        result = self.face_mesh.process(rgb_frame)

        if result.multi_face_landmarks is None:
            return None

        normalized_landmarks = Vectors.empty(shape=(face_settings.landmark_num, 3), dtype=float)
        width_multiplier = float(settings.width) / settings.normalize_multiplier
        height_multiplier = float(settings.height) / settings.normalize_multiplier

        for landmarks in result.multi_face_landmarks:
            for i in range(face_settings.landmark_num):
                # fmt: off
                normalized_landmarks[i] = [landmarks.landmark[i].x * width_multiplier,
                                           landmarks.landmark[i].y * height_multiplier,
                                           landmarks.landmark[i].z]
                # fmt: on
            break  # assume 1 face

        return FaceAndIrisLandmarks(face_landmarks=normalized_landmarks)


```
