# Facial rendering source scan: thinhvd/Facial-Animation-Retargeting-to-Virtual-Characters

Repository: https://github.com/thinhvd/Facial-Animation-Retargeting-to-Virtual-Characters

Pinned source revision: 7befceb678c253e22259c2bb58da8b37e2d0b9ca

Discovery topic: facial retargeting

Description: 

Reported license: GPL-3.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"eye": ["NodeServer/models/HumanModel/FemaleModel/textures/brown_eye.png", "NodeServer/models/HumanModel/FemaleModel/textures/eyebrow002.png", "NodeServer/models/HumanModel/MaleModel/brown_eye.png", "NodeServer/models/HumanModel/MaleModel/eyebrow002.png", "NodeServer/models/HumanModel/MaleModel/textures/brown_eye.png", "NodeServer/models/HumanModel/MaleModel/textures/eyebrow002.png", "NodeServer/models/model1/eye.png", "NodeServer/models/model1/eye2.png", "NodeServer/models/model2/PostAlphatEye.fx", "NodeServer/models/model2/PostAlphatEye.x", "NodeServer/models/model2/tmeye.fx", "NodeServer/models/model4/PostAlphaEye.fx", "NodeServer/models/model4/PostAlphaEye.x", "NodeServer/models/model4/aliceeye.fx", "NodeServer/models/model5/eyelight.PNG"], "iris": ["service/TFLiteIrisLocalization.py", "weights/iris_localization.tflite"], "face": ["NodeServer/models/headmodel/Tex/face1.png", "NodeServer/models/model1/face.png", "NodeServer/models/model3/facebb.png", "NodeServer/models/model4/facea.png", "NodeServer/models/test_model/face.png", "NodeServer/models/test_model/facebb.png", "service/TFLiteFaceAlignment.py", "service/TFLiteFaceDetector.py"]}

## Source file: service/TFLiteFaceDetector.py

{
  "path": "service/TFLiteFaceDetector.py",
  "blob_sha": "d5281c98fe345e5342a5e362dc39cfff8cc82bfa",
  "sha256": "1d1e3345df3af60e9f30a83983afae37346e004684e1c3a147edd08be0b85808",
  "lines": 154,
  "bytes": 5203,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 2,
    "lip": 2
  },
  "url": "https://github.com/thinhvd/Facial-Animation-Retargeting-to-Virtual-Characters/blob/7befceb678c253e22259c2bb58da8b37e2d0b9ca/service/TFLiteFaceDetector.py",
  "function_signature_candidates": [
    {
      "line": 8,
      "text": "def __init__(self, filepath, input_size=(320, 240), conf_threshold=0.6,"
    },
    {
      "line": 42,
      "text": "def _generate_anchors(self):"
    },
    {
      "line": 67,
      "text": "def _pre_processing(self, img):"
    },
    {
      "line": 75,
      "text": "def inference(self, img):"
    },
    {
      "line": 93,
      "text": "def _post_processing(self, boxes, scores):"
    },
    {
      "line": 108,
      "text": "def _decode_regression(self, reg):"
    }
  ]
}

```text

from functools import partial
import cv2
import tensorflow as tf
import numpy as np


class UltraLightFaceDetecion():
    def __init__(self, filepath, input_size=(320, 240), conf_threshold=0.6,
                 center_variance=0.1, size_variance=0.2,
                 nms_max_output_size=200, nms_iou_threshold=0.3) -> None:

        self._feature_maps = np.array([[40, 30], [20, 15], [10, 8], [5, 4]])
        self._min_boxes = [[10, 16, 24], [32, 48], [64, 96], [128, 192, 256]]

        self._resize = partial(cv2.resize, dsize=input_size)
        self._input_size = np.array(input_size)[:, None]

        self._anchors_xy, self._anchors_wh = self._generate_anchors()
        self._conf_threshold = conf_threshold
        self._center_variance = center_variance
        self._size_variance = size_variance
        self._nms = partial(tf.image.non_max_suppression,
                            max_output_size=nms_max_output_size,
                            iou_threshold=nms_iou_threshold)

        # tflite model init
        self._interpreter = tf.lite.Interpreter(model_path=filepath)
        self._interpreter.allocate_tensors()

        # model details
        input_details = self._interpreter.get_input_details()
        output_details = self._interpreter.get_output_details()

        # inference helper
        self._set_input_tensor = partial(self._interpreter.set_tensor,
                                         input_details[0]["index"])
        self._get_boxes_tensor = partial(self._interpreter.get_tensor,
                                         output_details[0]["index"])
        self._get_scores_tensor = partial(self._interpreter.get_tensor,
                                          output_details[1]["index"])

    def _generate_anchors(self):
        anchors = []
        for feature_map_w_h, min_box in zip(self._feature_maps, self._min_boxes):

            wh_grid = min_box / self._input_size
            wh_grid = np.tile(wh_grid.T, (np.prod(feature_map_w_h), 1))

            xy_grid = np.meshgrid(range(feature_map_w_h[0]),
                                  range(feature_map_w_h[1]))
            xy_grid = np.add(xy_grid, 0.5)

            xy_grid /= feature_map_w_h[..., None, None]

            xy_grid = np.stack(xy_grid, axis=-1)
            xy_grid = np.tile(xy_grid, [1, 1, len(min_box)])
            xy_grid = xy_grid.reshape(-1, 2)

            prior = np.concatenate((xy_grid, wh_grid), axis=-1)
            anchors.append(prior)

        anchors = np.concatenate(anchors, axis=0)
        anchors = np.clip(anchors, 0.0, 1.0)

        return anchors[:, :2], anchors[:, 2:]

    def _pre_processing(self, img):
        resized = self._resize(img)
        image_rgb = resized[..., ::-1]
        image_norm = image_rgb.astype(np.float32)
        cv2.normalize(image_norm, image_norm,
                      alpha=-1, beta=1, norm_type=cv2.NORM_MINMAX)
        return image_norm[None, ...]

    def inference(self, img):
        # BGR image to tensor
        input_tensor = self._pre_processing(img)

        # set tensor and invoke
        self._set_input_tensor(input_tensor)
        self._interpreter.invoke()

        # get results
        boxes = self._get_boxes_tensor()[0]
        scores = self._get_scores_tensor()[0]

        # decode boxes to corner format
        boxes, scores = self._post_processing(boxes, scores)
        boxes *= np.tile(img.shape[1::-1], 2)

        return boxes, scores

    def _post_processing(self, boxes, scores):
        # bounding box regression
        boxes = self._decode_regression(boxes)
        scores = scores[:, 1]

        # confidence threshold filter
        conf_mask = self._conf_threshold < scores
        boxes, scores = boxes[conf_mask], scores[conf_mask]

        # non-maximum suppression
        nms_mask = self._nms(boxes=boxes, scores=scores)
        boxes = np.take(boxes, nms_mask, axis=0)

        return boxes, scores

    def _decode_regression(self, reg):
        # bounding box regression
        center_xy = reg[:, :2] * self._center_variance * \
            self._anchors_wh + self._anchors_xy
        center_wh = np.exp(
            reg[:, 2:] * self._size_variance) * self._anchors_wh / 2

        # center to corner
        start_xy = center_xy - center_wh
        end_xy = center_xy + center_wh

        boxes = np.concatenate((start_xy, end_xy), axis=-1)
        boxes = np.clip(boxes, 0.0, 1.0)

        return boxes


if __name__ == '__main__':
    import sys
    import time

    fd = UltraLightFaceDetecion("weights/RFB-320.tflite",
                                conf_threshold=0.88)

    cap = cv2.VideoCapture(sys.argv[1])
    color = (125, 255, 125)

    while True:
        ret, frame = cap.read()


        if not ret:
            break

        start_time = time.perf_counter()
        boxes, scores = fd.inference(frame)
        print(time.perf_counter() - start_time)

        for det in boxes.astype(np.int32):
            cv2.rectangle(frame, (det[0], det[1]),
                          (det[2], det[3]), (2, 255, 0), 5)
            
        frame = cv2.resize(frame, (430, 360))

        cv2.imshow("result", frame)
        if cv2.waitKey(1) == ord('q'):
            break


```

## Source file: service/TFLiteFaceAlignment.py

{
  "path": "service/TFLiteFaceAlignment.py",
  "blob_sha": "88c773ddac950d52b73067b32f337766401943ec",
  "sha256": "ece15425f00b384f1493d495176d2518646e125da96c67a3f4e7336d7c7d047c",
  "lines": 163,
  "bytes": 5182,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 1,
    "gum": 4,
    "face": 9
  },
  "url": "https://github.com/thinhvd/Facial-Animation-Retargeting-to-Virtual-Characters/blob/7befceb678c253e22259c2bb58da8b37e2d0b9ca/service/TFLiteFaceAlignment.py",
  "function_signature_candidates": [
    {
      "line": 12,
      "text": "def __init__(self, filepath, marker_nums=106, input_size=(192, 192)):"
    },
    {
      "line": 36,
      "text": "def _calibrate(self, pred, thd, skip=6):"
    },
    {
      "line": 53,
      "text": "def _preprocessing(self, img, bbox, factor=3.0):"
    },
    {
      "line": 83,
      "text": "def _inference(self, input_tensor):"
    },
    {
      "line": 89,
      "text": "def _postprocessing(self, out, M):"
    },
    {
      "line": 102,
      "text": "def get_landmarks(self, image, detected_faces=None):"
    }
  ]
}

```text

#!/usr/bin/python3
# -*- coding:utf-8 -*-

import numpy as np
import cv2
import tensorflow as tf
from functools import partial
import time


class CoordinateAlignmentModel():
    def __init__(self, filepath, marker_nums=106, input_size=(192, 192)):
        self._marker_nums = marker_nums
        self._input_shape = input_size
        self._trans_distance = self._input_shape[-1] / 2.0

        self.eye_bound = ([35, 41, 40, 42, 39, 37, 33, 36],
                          [89, 95, 94, 96, 93, 91, 87, 90])

        # tflite model init
        self._interpreter = tf.lite.Interpreter(model_path=filepath)
        self._interpreter.allocate_tensors()

        # model details
        input_details = self._interpreter.get_input_details()
        output_details = self._interpreter.get_output_details()

        # inference helper
        self._set_input_tensor = partial(self._interpreter.set_tensor,
                                         input_details[0]["index"])
        self._get_output_tensor = partial(self._interpreter.get_tensor,
                                          output_details[0]["index"])

        self.pre_landmarks = None

    def _calibrate(self, pred, thd, skip=6):
        if self.pre_landmarks is not None:
            for i in range(pred.shape[0]):
                if abs(self.pre_landmarks[i, 0] - pred[i, 0]) > skip:
                    self.pre_landmarks[i, 0] = pred[i, 0]
                elif abs(self.pre_landmarks[i, 0] - pred[i, 0]) > thd:
                    self.pre_landmarks[i, 0] += pred[i, 0]
                    self.pre_landmarks[i, 0] /= 2

                if abs(self.pre_landmarks[i, 1] - pred[i, 1]) > skip:
                    self.pre_landmarks[i, 1] = pred[i, 1]
                elif abs(self.pre_landmarks[i, 1] - pred[i, 1]) > thd:
                    self.pre_landmarks[i, 1] += pred[i, 1]  
                    self.pre_landmarks[i, 1] /= 2
        else:
            self.pre_landmarks = pred

    def _preprocessing(self, img, bbox, factor=3.0):
        """Pre-processing of the BGR image. Adopting warp affine for face corp.

        Arguments
        ----------
        img {numpy.array} : the raw BGR image.
        bbox {numpy.array} : bounding box with format: {x1, y1, x2, y2, score}.

        Keyword Arguments
        ----------
        factor : max edge scale factor for bounding box cropping.

        Returns
        ----------
        inp : input tensor with NHWC format.
        M : warp affine matrix.
        """

        maximum_edge = max(bbox[2:4] - bbox[:2]) * factor
        scale = self._trans_distance * 4.0 / maximum_edge
        center = (bbox[2:4] + bbox[:2]) / 2.0
        cx, cy = self._trans_distance - scale * center

        M = np.array([[scale, 0, cx], [0, scale, cy]])

        cropped = cv2.warpAffine(img, M, self._input_shape, borderValue=0.0)
        inp = cropped[..., ::-1].astype(np.float32)

        return inp[None, ...], M

    def _inference(self, input_tensor):
        self._set_input_tensor(input_tensor)
        self._interpreter.invoke()

        return self._get_output_tensor()[0]

    def _postprocessing(self, out, M):
        iM = cv2.invertAffineTransform(M)
        col = np.ones((self._marker_nums, 1))

        out = out.reshape((self._marker_nums, 2))

        out += 1
        out *= self._trans_distance

        out = np.concatenate((out, col), axis=1)

        return out @ iM.T  # dot product

    def get_landmarks(self, image, detected_faces=None):
        """Predict the landmarks for each face present in the image.

        This function predicts a set of 68 2D or 3D images, one for each image present.
        If detect_faces is None the method will also run a face detector.

        Arguments
        ----------
        image {numpy.array} : The input image.

        Keyword Arguments
        ----------
        detected_faces {list of numpy.array} : list of bounding boxes, one for each
        face found in the image (default: {None}, format: {x1, y1, x2, y2, score})
        """

        for box in detected_faces:
            inp, M = self._preprocessing(image, box)
            out = self._inference(inp)
            pred = self._postprocessing(out, M)

            # self._calibrate(pred, 1, skip=6)
            # yield self.pre_landmarks

            yield pred


if __name__ == '__main__':

    from TFLiteFaceDetector import UltraLightFaceDetecion
    import sys

    fd = UltraLightFaceDetecion(
        "weights/RFB-320.tflite",
        conf_threshold=0.88)
    fa = CoordinateAlignmentModel(
        "weights/coor_2d106.tflite")

    cap = cv2.VideoCapture(sys.argv[1])
    color = (125, 255, 125)

    while True:
        ret, frame = cap.read()

        if not ret:
            break

        start_time = time.perf_counter()

        boxes, scores = fd.inference(frame)

        for pred in fa.get_landmarks(frame, boxes):
            for p in np.round(pred).astype(np.int):
                cv2.circle(frame, tuple(p), 1, color, 5, cv2.LINE_AA)

        print(time.perf_counter() - start_time)

        frame = cv2.resize(frame, (450, 380))

        cv2.imshow("result", frame)
        if cv2.waitKey(0) == ord('q'):
            break


```

## Source file: service/TFLiteIrisLocalization.py

{
  "path": "service/TFLiteIrisLocalization.py",
  "blob_sha": "0a5211486bb009a7c0b9442696d696239847a8a3",
  "sha256": "61451de6dc289e1f7ec6469025efa2ed69d7b0b73f07457a20da20a5f425029f",
  "lines": 207,
  "bytes": 6614,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 18,
    "iris": 14,
    "rig": 3,
    "face": 4
  },
  "url": "https://github.com/thinhvd/Facial-Animation-Retargeting-to-Virtual-Characters/blob/7befceb678c253e22259c2bb58da8b37e2d0b9ca/service/TFLiteIrisLocalization.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def __init__(self, filepath):"
    },
    {
      "line": 27,
      "text": "def _preprocess(self, img, length, center, name=None):"
    },
    {
      "line": 53,
      "text": "def get_mesh(self, image, length, center, name=None):"
    },
    {
      "line": 84,
      "text": "def draw_pupil(iris, frame, color=(0, 0, 255), thickness=2):"
    },
    {
      "line": 96,
      "text": "def draw_eye_markers(landmarks, frame, close=True, color=(0, 255, 255), thickness=2):"
    },
    {
      "line": 101,
      "text": "def calculate_3d_gaze(poi, scale=256):"
    }
  ]
}

```text

import cv2
import tensorflow as tf
import numpy as np
from functools import partial


class IrisLocalizationModel():

    def __init__(self, filepath):
        # Load the TFLite model and allocate tensors.
        self._interpreter = tf.lite.Interpreter(model_path=filepath)
        self._interpreter.allocate_tensors()

        # model details
        input_details = self._interpreter.get_input_details()
        output_details = self._interpreter.get_output_details()

        # inference helper
        self._set_input_tensor = partial(self._interpreter.set_tensor,
                                         input_details[0]["index"])
        self._get_output_tensor = partial(self._interpreter.get_tensor,
                                          output_details[0]["index"])

        self.trans_distance = 32
        self.input_shape = (64, 64)

    def _preprocess(self, img, length, center, name=None):
        """Preprocess the image to meet the model's input requirement.
        Args:
            img: An image in default BGR format.

        Returns:
            image_norm: The normalized image ready to be feeded.
        """

        scale = 23 / length
        cx, cy = self.trans_distance - scale * center

        M = np.array([[scale, 0, cx], [0, scale, cy]])

        resized = cv2.warpAffine(img, M, self.input_shape, borderValue=0.0)

        if name is not None:
            cv2.imshow(name, resized)

        image_rgb = cv2.cvtColor(resized, cv2.COLOR_BGR2RGB)
        image_norm = image_rgb.astype(np.float32)
        cv2.normalize(image_norm, image_norm, alpha=-1,
                      beta=1, norm_type=cv2.NORM_MINMAX)

        return image_norm, M

    def get_mesh(self, image, length, center, name=None):
        """Detect the face mesh from the image given.
        Args:
            image: An image in default BGR format.

        Returns:
            mesh: An eyebrow mesh, normalized.
            iris: Iris landmarks.
        """

        # Preprocess the image before sending to the network.
        image, M = self._preprocess(image, length, center, name)

        image = tf.image.convert_image_dtype(image, tf.float32)
        image = image[tf.newaxis, :]

        # The actual detection.
        self._set_input_tensor(image)
        self._interpreter.invoke()

        # Save the results.
        iris = self._get_output_tensor()[0]

        iris = iris.reshape(-1, 3)
        iris[:, 2] = 1

        iM = cv2.invertAffineTransform(M)

        return iris @ iM.T

    @staticmethod
    def draw_pupil(iris, frame, color=(0, 0, 255), thickness=2):
        pupil = iris[0]
        radius = np.linalg.norm(iris[1:] - iris[0], axis=1)

        pupil = pupil.astype(int)
        radius = int(max(radius))

        cv2.circle(frame, tuple(pupil), radius, color, thickness, cv2.LINE_AA)

        return pupil, radius

    @staticmethod
    def draw_eye_markers(landmarks, frame, close=True, color=(0, 255, 255), thickness=2):
        landmarks = landmarks.astype(np.int32)
        cv2.polylines(frame, landmarks, close, color, thickness, cv2.LINE_AA)

    @staticmethod
    def calculate_3d_gaze(poi, scale=256):
        SIN_LEFT_THETA = 2 * np.sin(np.pi / 4)
        SIN_UP_THETA = np.sin(np.pi / 6)

        starts, ends, pupils, centers = poi

        eye_length = np.linalg.norm(starts - ends, axis=1)
        ic_distance = np.linalg.norm(pupils - centers, axis=1)
        zc_distance = np.linalg.norm(pupils - starts, axis=1)

        s0 = (starts[:, 1] - ends[:, 1]) * pupils[:, 0]
        s1 = (starts[:, 0] - ends[:, 0]) * pupils[:, 1]
        s2 = starts[:, 0] * ends[:, 1]
        s3 = starts[:, 1] * ends[:, 0]

        delta_y = (s0 - s1 + s2 - s3) / eye_length / 2
        delta_x = np.sqrt(abs(ic_distance**2 - delta_y**2))

        delta = np.array((delta_x * SIN_LEFT_THETA,
                          delta_y * SIN_UP_THETA))
        delta /= eye_length
        theta, pha = np.arcsin(delta)

        inv_judge = zc_distance**2 - delta_y**2 < eye_length**2 / 4

        delta[0, inv_judge] *= -1
        theta[inv_judge] *= -1
        delta *= scale

        return theta, pha, delta.T


if __name__ == "__main__":
    import sys
    from SolvePnPHeadPoseEstimation import HeadPoseEstimator
    from TFLiteFaceAlignment import CoordinateAlignmentModel
    from TFLiteFaceDetector import UltraLightFaceDetecion

    gpu_ctx = -1
    video = sys.argv[1]
    YAW_THD = 45

    cap = cv2.VideoCapture(video)

    fd = UltraLightFaceDetecion("weights/RFB-320.tflite",
                                conf_threshold=0.9)
    fa = CoordinateAlignmentModel("weights/coor_2d106.tflite")
    hp = HeadPoseEstimator("weights/head_pose_object_points.npy",
                           cap.get(3), cap.get(4))
    gs = IrisLocalizationModel("weights/iris_localization.tflite")

    counter = 0

    while True:
        ret, frame = cap.read()

        if not ret:
            break

        # frame = frame[:480, 380:920, :]  # dress
        # frame = cv2.resize(frame, (960, 1080))

        bboxes, _ = fd.inference(frame)

        for landmarks in fa.get_landmarks(frame, bboxes):
            # calculate head pose
            euler_angle = hp.get_head_pose(landmarks)
            pitch, yaw, roll = euler_angle[:, 0]

            eye_markers = np.take(landmarks, fa.eye_bound, axis=0)

            # eye_centers = np.average(eye_markers, axis=1)
            eye_centers = landmarks[[34, 88]]

            # eye_lengths = np.linalg.norm(landmarks[[39, 93]] - landmarks[[35, 89]], axis=1)
            eye_start = landmarks[[35, 89]]
            eye_end = landmarks[[39, 93]]
            eye_lengths = (eye_end - eye_start)[:, 0]

            pupils = eye_centers.copy()

            if yaw > -YAW_THD:
                iris_left = gs.get_mesh(frame, eye_lengths[0], eye_centers[0])
                pupils[0], _ = gs.draw_pupil(iris_left, frame, thickness=2)

            if yaw < YAW_THD:
                iris_right = gs.get_mesh(frame, eye_lengths[1], eye_centers[1])
                pupils[1], _ = gs.draw_pupil(iris_right, frame, thickness=2)

            poi = landmarks[[35, 89]], landmarks[[39, 93]], pupils, eye_centers

            theta, pha, _ = gs.calculate_3d_gaze(poi)

            # print(theta.mean(), pha.mean())

            gs.draw_eye_markers(eye_markers, frame, thickness=2)

        # frame = cv2.resize(frame, (430, 360))
        cv2.imshow('res', frame)
        # cv2.imwrite(f'./asset/orign_dress/img{counter:0>3}.png', frame)

        counter += 1
        if cv2.waitKey(0) == ord('q'):
            break

    cap.release()
    cv2.destroyAllWindows()


```

## Source file: NodeServer/index.js

{
  "path": "NodeServer/index.js",
  "blob_sha": "b7c07cd82e82863b995569a51164dd94c46a60c7",
  "sha256": "f792b26abd140cfa0f4068e238a308babaa96edf35ef2560595fc5076691eba3",
  "lines": 49,
  "bytes": 1429,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 1
  },
  "url": "https://github.com/thinhvd/Facial-Animation-Retargeting-to-Virtual-Characters/blob/7befceb678c253e22259c2bb58da8b37e2d0b9ca/NodeServer/index.js",
  "function_signature_candidates": []
}

```text

var app = require('http').createServer(handler)
var io = require('socket.io')(app);
var fs = require('fs');

function handler(req, res) {
    res.setHeader('Access-Control-Allow-Origin', '*');
    
    let url = req.url;
    if (url === '/') {
        url = '/model.html';
    }
    
    fs.readFile(__dirname + url,
        function (err, data) {
            if (err) {
                if (err.code === 'ENOENT') {
                    res.writeHead(404);
                    return res.end('File not found');
                }
                res.writeHead(500);
                return res.end('Error loading ' + url);
            }

            res.writeHead(200);
            res.end(data);
        });
}

io.of('/model').on('connection', (socket) => {
    console.log('a model client connected');

    socket.on('result_data', (result) => {
        if (result != 0) {
            socket.broadcast.emit('result_download', result);
        }
    });

    socket.on('disconnect', () => { console.log('a model client disconnected') });
});

io.of('/human').on('connection', (socket) => {
    console.log('a human client connected');
    socket.on('result_data', (result) => {
        if (result != 0) socket.broadcast.emit('result_download', result);
    });
    socket.on('disconnect', () => console.log('a human client disconnected'));
});

app.listen(6789, () => console.log('listening on http://127.0.0.1:6789/model.html'));


```
