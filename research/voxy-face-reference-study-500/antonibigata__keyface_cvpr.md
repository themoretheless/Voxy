# Facial rendering source scan: antonibigata/keyface_cvpr

Repository: https://github.com/antonibigata/keyface_cvpr

Pinned source revision: 9226167d0bd59791da079b98f25a60c204ca3bd7

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["evaluation/auto_avsr/LICENSE", "sgm/modules/autoencoding/lpips/loss/LICENSE", "sgm/modules/autoencoding/lpips/model/LICENSE"]

Topic paths: {"face": ["evaluation/auto_avsr/espnet/nets/scorer_interface.py", "evaluation/auto_avsr/preparation/detectors/mediapipe/20words_mean_face.npy", "evaluation/auto_avsr/preparation/detectors/retinaface/20words_mean_face.npy", "evaluation/auto_avsr/preparation/detectors/retinaface/detector.py", "evaluation/auto_avsr/preparation/detectors/retinaface/video_process.py"], "lip": ["evaluation/lipscore.py"]}

## Source file: evaluation/lipscore.py

{
  "path": "evaluation/lipscore.py",
  "blob_sha": "f1985fd15f9ed8749fba322e2269e8f3700d4740",
  "sha256": "ae4617f2685b71227c2a2e1c0f35f3efbda58071dd972de51324cd79f8b682a6",
  "lines": 164,
  "bytes": 5400,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 2,
    "lip": 5
  },
  "url": "https://github.com/antonibigata/keyface_cvpr/blob/9226167d0bd59791da079b98f25a60c204ca3bd7/evaluation/lipscore.py",
  "function_signature_candidates": [
    {
      "line": 25,
      "text": "def __init__("
    },
    {
      "line": 81,
      "text": "def update(self, videos_pred: List[torch.Tensor], videos_gt: List[torch.Tensor]):"
    },
    {
      "line": 116,
      "text": "def compute(self):"
    },
    {
      "line": 123,
      "text": "def get_feature(self, video: torch.Tensor):"
    }
  ]
}

```text

import os
import torch
from torchmetrics import Metric
from typing import List
import numpy as np

import hydra

from auto_avsr.lightning import ModelModule
from auto_avsr.datamodule.transforms import VideoTransform
from auto_avsr.preparation.detectors.retinaface.detector import LandmarksDetector
from auto_avsr.preparation.detectors.retinaface.video_process import VideoProcess
import torch.nn.functional as F


class LipScore(Metric):
    """
    LipScore Metric using the auto_avsr lip reading model.
    """

    is_differentiable = False
    higher_is_better = False
    full_state_update = False

    def __init__(
        self,
        modality: str = "video",
        pretrained_model_path: str = "PATH_TO_CHECKPOINTS/vsr_trlrs2lrs3vox2avsp_base_updated.pth",
        config_dir: str = "PATH_TO_EVALUATION_FOLDER/auto_avsr/configs",
        device: torch.device = torch.device(
            "cuda" if torch.cuda.is_available() else "cpu"
        ),
    ):
        super().__init__()

        if not os.path.exists(pretrained_model_path):
            raise FileNotFoundError(f"Model file not found at {pretrained_model_path}")

        if not os.path.exists(config_dir):
            raise FileNotFoundError(f"Config directory not found at {config_dir}")

        # Initialize Hydra configuration using absolute path
        with hydra.initialize_config_dir(config_dir=config_dir, version_base="1.3"):
            cfg = hydra.compose(
                config_name="config",
                overrides=[
                    f"data.modality={modality}",
                    f"pretrained_model_path={pretrained_model_path}",
                ],
            )
        self.cfg = cfg
        self.modality = cfg.data.modality

        self.my_device = device  # Renamed to avoid conflict

        # Initialize the model components
        self.video_transform = VideoTransform(subset="test")
        self.landmarks_detector = LandmarksDetector(device=str(device))
        self.video_process = VideoProcess(convert_gray=False)

        # Initialize the ModelModule
        self.modelmodule = ModelModule(self.cfg)
        self.modelmodule.model.load_state_dict(
            torch.load(
                self.cfg.pretrained_model_path,
                map_location=lambda storage, loc: storage,
            )
        )
        self.modelmodule.to(device)
        self.modelmodule.eval()

        # States for the metric
        self.add_state("errors", default=torch.tensor(0.0), dist_reduce_fx="sum")
        self.add_state("total_words", default=torch.tensor(0.0), dist_reduce_fx="sum")
        self.add_state(
            "cosine_similarities", default=torch.tensor(0.0), dist_reduce_fx="sum"
        )

        self.counter = 0

    def update(self, videos_pred: List[torch.Tensor], videos_gt: List[torch.Tensor]):
        """
        Update the cosine similarity metric with batches of videos.

        Args:
            videos_pred (List[str]): List of predicted video tensors.
            videos_gt (List[str]): List of ground truth video tensors.
        """
        assert len(videos_pred) == len(videos_gt), (
            "The number of predicted and ground truth videos must be the same."
        )

        landmarks_pred_list = []
        landmarks_gt_list = []

        for pred, gt in zip(videos_pred, videos_gt):
            pred = pred.permute(0, 2, 3, 1).numpy() * 255
            gt = gt.permute(0, 2, 3, 1).numpy() * 255

            # Get features
            feature_pred, landmarks_pred = self.get_feature(pred)
            feature_gt, landmarks_gt = self.get_feature(gt)

            landmarks_pred_list.append(np.array(landmarks_pred))
            landmarks_gt_list.append(np.array(landmarks_gt))

            # Compute cosine similarity
            similarity = F.cosine_similarity(feature_pred, feature_gt, dim=1)

            # Update states
            self.cosine_similarities += similarity.mean().cpu().item()
            self.counter += 1

        return landmarks_pred_list, landmarks_gt_list

    def compute(self):
        result = {}

        result["cosine_sim"] = self.cosine_similarities.item() / self.counter

        return result

    def get_feature(self, video: torch.Tensor):
        # Apply landmarks detector
        landmarks = self.landmarks_detector(video)

        # Process video
        video_processed = self.video_process(video, landmarks)
        video_tensor = torch.tensor(video_processed).permute(0, 3, 1, 2)  # (T, C, H, W)
        video_tensor = self.video_transform(video_tensor)

        # Move to device
        video_tensor = video_tensor.to(self.my_device)

        # Get transcript
        with torch.no_grad():
            feature = self.modelmodule.forward_no_beam(
                video_tensor
            )  # Add batch dimension

        return feature, landmarks


if __name__ == "__main__":
    import torchvision

    # Create sample videos for testing
    batch_size = 2
    frames = 10
    height = 512
    width = 512
    channels = 3

    video_path = "PATH_TO_VIDEO"
    video = torchvision.io.read_video(video_path, pts_unit="sec")[
        0
    ]  # Returns (T, H, W, C)
    video = video.permute(0, 3, 1, 2) / 255  # Convert to (T, C, H, W)
    videos_pred = [video]
    videos_gt = [video]  # Using same video for demo

    lipscore = LipScore()
    lipscore.update(videos_pred, videos_gt)
    print(lipscore.compute())


```

## Source file: evaluation/auto_avsr/espnet/nets/scorer_interface.py

{
  "path": "evaluation/auto_avsr/espnet/nets/scorer_interface.py",
  "blob_sha": "fb3a09fa676539b27e063cb567ebe3fd4d7ee509",
  "sha256": "bfb21dd5b99f892fa9880764a47032729735bce3dd3b143d396e13ce1f31fc27",
  "lines": 186,
  "bytes": 5902,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 9
  },
  "url": "https://github.com/antonibigata/keyface_cvpr/blob/9226167d0bd59791da079b98f25a60c204ca3bd7/evaluation/auto_avsr/espnet/nets/scorer_interface.py",
  "function_signature_candidates": [
    {
      "line": 27,
      "text": "def init_state(self, x: torch.Tensor) -> Any:"
    },
    {
      "line": 38,
      "text": "def select_state(self, state: Any, i: int, new_id: int = None) -> Any:"
    },
    {
      "line": 52,
      "text": "def score("
    },
    {
      "line": 70,
      "text": "def final_score(self, state: Any) -> float:"
    },
    {
      "line": 86,
      "text": "def batch_init_state(self, x: torch.Tensor) -> Any:"
    },
    {
      "line": 97,
      "text": "def batch_score("
    },
    {
      "line": 142,
      "text": "def score_partial("
    },
    {
      "line": 165,
      "text": "def batch_score_partial("
    }
  ]
}

```text

"""Scorer interface module."""

import warnings
from typing import Any, List, Tuple

import torch


class ScorerInterface:
    """Scorer interface for beam search.

    The scorer performs scoring of the all tokens in vocabulary.

    Examples:
        * Search heuristics
            * :class:`espnet.nets.scorers.length_bonus.LengthBonus`
        * Decoder networks of the sequence-to-sequence models
            * :class:`espnet.nets.pytorch_backend.nets.transformer.decoder.Decoder`
            * :class:`espnet.nets.pytorch_backend.nets.rnn.decoders.Decoder`
        * Neural language models
            * :class:`espnet.nets.pytorch_backend.lm.transformer.TransformerLM`
            * :class:`espnet.nets.pytorch_backend.lm.default.DefaultRNNLM`
            * :class:`espnet.nets.pytorch_backend.lm.seq_rnn.SequentialRNNLM`

    """

    def init_state(self, x: torch.Tensor) -> Any:
        """Get an initial state for decoding (optional).

        Args:
            x (torch.Tensor): The encoded feature tensor

        Returns: initial state

        """
        return None

    def select_state(self, state: Any, i: int, new_id: int = None) -> Any:
        """Select state with relative ids in the main beam search.

        Args:
            state: Decoder state for prefix tokens
            i (int): Index to select a state in the main beam search
            new_id (int): New label index to select a state if necessary

        Returns:
            state: pruned state

        """
        return None if state is None else state[i]

    def score(
        self, y: torch.Tensor, state: Any, x: torch.Tensor
    ) -> Tuple[torch.Tensor, Any]:
        """Score new token (required).

        Args:
            y (torch.Tensor): 1D torch.int64 prefix tokens.
            state: Scorer state for prefix tokens
            x (torch.Tensor): The encoder feature that generates ys.

        Returns:
            tuple[torch.Tensor, Any]: Tuple of
                scores for next token that has a shape of `(n_vocab)`
                and next state for ys

        """
        raise NotImplementedError

    def final_score(self, state: Any) -> float:
        """Score eos (optional).

        Args:
            state: Scorer state for prefix tokens

        Returns:
            float: final score

        """
        return 0.0


class BatchScorerInterface(ScorerInterface):
    """Batch scorer interface."""

    def batch_init_state(self, x: torch.Tensor) -> Any:
        """Get an initial state for decoding (optional).

        Args:
            x (torch.Tensor): The encoded feature tensor

        Returns: initial state

        """
        return self.init_state(x)

    def batch_score(
        self, ys: torch.Tensor, states: List[Any], xs: torch.Tensor
    ) -> Tuple[torch.Tensor, List[Any]]:
        """Score new token batch (required).

        Args:
            ys (torch.Tensor): torch.int64 prefix tokens (n_batch, ylen).
            states (List[Any]): Scorer states for prefix tokens.
            xs (torch.Tensor):
                The encoder feature that generates ys (n_batch, xlen, n_feat).

        Returns:
            tuple[torch.Tensor, List[Any]]: Tuple of
                batchfied scores for next token with shape of `(n_batch, n_vocab)`
                and next state list for ys.

        """
        warnings.warn(
            "{} batch score is implemented through for loop not parallelized".format(
                self.__class__.__name__
            )
        )
        scores = list()
        outstates = list()
        for i, (y, state, x) in enumerate(zip(ys, states, xs)):
            score, outstate = self.score(y, state, x)
            outstates.append(outstate)
            scores.append(score)
        scores = torch.cat(scores, 0).view(ys.shape[0], -1)
        return scores, outstates


class PartialScorerInterface(ScorerInterface):
    """Partial scorer interface for beam search.

    The partial scorer performs scoring when non-partial scorer finished scoring,
    and receives pre-pruned next tokens to score because it is too heavy to score
    all the tokens.

    Examples:
         * Prefix search for connectionist-temporal-classification models
             * :class:`espnet.nets.scorers.ctc.CTCPrefixScorer`

    """

    def score_partial(
        self, y: torch.Tensor, next_tokens: torch.Tensor, state: Any, x: torch.Tensor
    ) -> Tuple[torch.Tensor, Any]:
        """Score new token (required).

        Args:
            y (torch.Tensor): 1D prefix token
            next_tokens (torch.Tensor): torch.int64 next token to score
            state: decoder state for prefix tokens
            x (torch.Tensor): The encoder feature that generates ys

        Returns:
            tuple[torch.Tensor, Any]:
                Tuple of a score tensor for y that has a shape `(len(next_tokens),)`
                and next state for ys

        """
        raise NotImplementedError


class BatchPartialScorerInterface(BatchScorerInterface, PartialScorerInterface):
    """Batch partial scorer interface for beam search."""

    def batch_score_partial(
        self,
        ys: torch.Tensor,
        next_tokens: torch.Tensor,
        states: List[Any],
        xs: torch.Tensor,
    ) -> Tuple[torch.Tensor, Any]:
        """Score new token (required).

        Args:
            ys (torch.Tensor): torch.int64 prefix tokens (n_batch, ylen).
            next_tokens (torch.Tensor): torch.int64 tokens to score (n_batch, n_token).
            states (List[Any]): Scorer states for prefix tokens.
            xs (torch.Tensor):
                The encoder feature that generates ys (n_batch, xlen, n_feat).

        Returns:
            tuple[torch.Tensor, Any]:
                Tuple of a score tensor for ys that has a shape `(n_batch, n_vocab)`
                and next states for ys
        """
        raise NotImplementedError


```

## Source file: evaluation/auto_avsr/preparation/detectors/retinaface/detector.py

{
  "path": "evaluation/auto_avsr/preparation/detectors/retinaface/detector.py",
  "blob_sha": "a77d886cba73148a929ff1418894716a09303f23",
  "sha256": "c21a98a19de9f88183efc51a4e1e8c674d621ad9692b743a2b92b7706125862f",
  "lines": 38,
  "bytes": 1353,
  "selection_score": 3,
  "keyword_line_counts": {
    "rig": 1,
    "face": 9
  },
  "url": "https://github.com/antonibigata/keyface_cvpr/blob/9226167d0bd59791da079b98f25a60c204ca3bd7/evaluation/auto_avsr/preparation/detectors/retinaface/detector.py",
  "function_signature_candidates": [
    {
      "line": 16,
      "text": "def __init__(self, device=\"cuda:0\", model_name=\"resnet50\"):"
    },
    {
      "line": 24,
      "text": "def __call__(self, video_frames):"
    }
  ]
}

```text

#! /usr/bin/env python
# -*- coding: utf-8 -*-

# Copyright 2021 Imperial College London (Pingchuan Ma)
# Apache 2.0  (http://www.apache.org/licenses/LICENSE-2.0)

import warnings

from ibug.face_alignment import FANPredictor
from ibug.face_detection import RetinaFacePredictor

warnings.filterwarnings("ignore")


class LandmarksDetector:
    def __init__(self, device="cuda:0", model_name="resnet50"):
        self.face_detector = RetinaFacePredictor(
            device=device,
            threshold=0.8,
            model=RetinaFacePredictor.get_model(model_name),
        )
        self.landmark_detector = FANPredictor(device=device, model=None)

    def __call__(self, video_frames):
        landmarks = []
        for frame in video_frames:
            detected_faces = self.face_detector(frame, rgb=True)
            face_points, _ = self.landmark_detector(frame, detected_faces, rgb=True)
            if len(detected_faces) == 0:
                landmarks.append(None)
            else:
                max_id, max_size = 0, 0
                for idx, bbox in enumerate(detected_faces):
                    bbox_size = (bbox[2] - bbox[0]) + (bbox[3] - bbox[1])
                    if bbox_size > max_size:
                        max_id, max_size = idx, bbox_size
                landmarks.append(face_points[max_id])
        return landmarks


```

## Source file: evaluation/auto_avsr/preparation/detectors/retinaface/video_process.py

{
  "path": "evaluation/auto_avsr/preparation/detectors/retinaface/video_process.py",
  "blob_sha": "6c5a1057c71222081c1849319c0274a88113b861",
  "sha256": "d3bbd6ac32ac5985d9e46484e2232449085f0dbeb204b2a96657b41018a577b2",
  "lines": 217,
  "bytes": 7593,
  "selection_score": 3,
  "keyword_line_counts": {
    "rig": 1,
    "face": 2,
    "lip": 4
  },
  "url": "https://github.com/antonibigata/keyface_cvpr/blob/9226167d0bd59791da079b98f25a60c204ca3bd7/evaluation/auto_avsr/preparation/detectors/retinaface/video_process.py",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "def linear_interpolate(landmarks, start_idx, stop_idx):"
    },
    {
      "line": 25,
      "text": "def warp_img(src, dst, img, std_size):"
    },
    {
      "line": 32,
      "text": "def apply_transform(transform, img, std_size):"
    },
    {
      "line": 38,
      "text": "def cut_patch(img, landmarks, height, width, threshold=5):"
    },
    {
      "line": 56,
      "text": "def __init__("
    },
    {
      "line": 76,
      "text": "def __call__(self, video, landmarks):"
    },
    {
      "line": 90,
      "text": "def crop_patch(self, video, landmarks):"
    },
    {
      "line": 120,
      "text": "def interpolate_landmarks(self, landmarks):"
    },
    {
      "line": 147,
      "text": "def affine_transform("
    },
    {
      "line": 180,
      "text": "def get_stable_reference("
    },
    {
      "line": 188,
      "text": "def estimate_affine_transform(self, landmarks, stable_points, stable_reference):"
    },
    {
      "line": 195,
      "text": "def apply_affine_transform("
    }
  ]
}

```text

#! /usr/bin/env python
# -*- coding: utf-8 -*-

# Copyright 2023 Imperial College London (Pingchuan Ma)
# Apache 2.0  (http://www.apache.org/licenses/LICENSE-2.0)

import os

import cv2
import numpy as np
from skimage import transform as tf


def linear_interpolate(landmarks, start_idx, stop_idx):
    start_landmarks = landmarks[start_idx]
    stop_landmarks = landmarks[stop_idx]
    delta = stop_landmarks - start_landmarks
    for idx in range(1, stop_idx - start_idx):
        landmarks[start_idx + idx] = (
            start_landmarks + idx / float(stop_idx - start_idx) * delta
        )
    return landmarks


def warp_img(src, dst, img, std_size):
    tform = tf.estimate_transform("similarity", src, dst)
    warped = tf.warp(img, inverse_map=tform.inverse, output_shape=std_size)
    warped = (warped * 255).astype("uint8")
    return warped, tform


def apply_transform(transform, img, std_size):
    warped = tf.warp(img, inverse_map=transform.inverse, output_shape=std_size)
    warped = (warped * 255).astype("uint8")
    return warped


def cut_patch(img, landmarks, height, width, threshold=5):
    center_x, center_y = np.mean(landmarks, axis=0)
    # Check for too much bias in height and width
    if abs(center_y - img.shape[0] / 2) > height + threshold:
        raise OverflowError("too much bias in height")
    if abs(center_x - img.shape[1] / 2) > width + threshold:
        raise OverflowError("too much bias in width")
    # Calculate bounding box coordinates
    y_min = int(round(np.clip(center_y - height, 0, img.shape[0])))
    y_max = int(round(np.clip(center_y + height, 0, img.shape[0])))
    x_min = int(round(np.clip(center_x - width, 0, img.shape[1])))
    x_max = int(round(np.clip(center_x + width, 0, img.shape[1])))
    # Cut the image
    cutted_img = np.copy(img[y_min:y_max, x_min:x_max])
    return cutted_img


class VideoProcess:
    def __init__(
        self,
        mean_face_path="20words_mean_face.npy",
        crop_width=96,
        crop_height=96,
        start_idx=48,
        stop_idx=68,
        window_margin=12,
        convert_gray=True,
    ):
        self.reference = np.load(
            os.path.join(os.path.dirname(__file__), mean_face_path)
        )
        self.crop_width = crop_width
        self.crop_height = crop_height
        self.start_idx = start_idx
        self.stop_idx = stop_idx
        self.window_margin = window_margin
        self.convert_gray = convert_gray

    def __call__(self, video, landmarks):
        # Pre-process landmarks: interpolate frames that are not detected
        preprocessed_landmarks = self.interpolate_landmarks(landmarks)
        # Exclude corner cases: no landmark in all frames or number of frames is less than window length
        if (
            not preprocessed_landmarks
            or len(preprocessed_landmarks) < self.window_margin
        ):
            return
        # Affine transformation and crop patch
        sequence = self.crop_patch(video, preprocessed_landmarks)
        assert sequence is not None, "crop an empty patch."
        return sequence

    def crop_patch(self, video, landmarks):
        sequence = []
        for frame_idx, frame in enumerate(video):
            window_margin = min(
                self.window_margin // 2, frame_idx, len(landmarks) - 1 - frame_idx
            )
            smoothed_landmarks = np.mean(
                [
                    landmarks[x]
                    for x in range(
                        frame_idx - window_margin, frame_idx + window_margin + 1
                    )
                ],
                axis=0,
            )
            smoothed_landmarks += landmarks[frame_idx].mean(
                axis=0
            ) - smoothed_landmarks.mean(axis=0)
            transformed_frame, transformed_landmarks = self.affine_transform(
                frame, smoothed_landmarks, self.reference, grayscale=self.convert_gray
            )
            patch = cut_patch(
                transformed_frame,
                transformed_landmarks[self.start_idx : self.stop_idx],
                self.crop_height // 2,
                self.crop_width // 2,
            )
            sequence.append(patch)
        return np.array(sequence)

    def interpolate_landmarks(self, landmarks):
        valid_frames_idx = [idx for idx, lm in enumerate(landmarks) if lm is not None]

        if not valid_frames_idx:
            return None

        for idx in range(1, len(valid_frames_idx)):
            if valid_frames_idx[idx] - valid_frames_idx[idx - 1] > 1:
                landmarks = linear_interpolate(
                    landmarks, valid_frames_idx[idx - 1], valid_frames_idx[idx]
                )

        valid_frames_idx = [idx for idx, lm in enumerate(landmarks) if lm is not None]

        # Handle corner case: keep frames at the beginning or at the end that failed to be detected
        if valid_frames_idx:
            landmarks[: valid_frames_idx[0]] = [
                landmarks[valid_frames_idx[0]]
            ] * valid_frames_idx[0]
            landmarks[valid_frames_idx[-1] :] = [landmarks[valid_frames_idx[-1]]] * (
                len(landmarks) - valid_frames_idx[-1]
            )

        assert all(lm is not None for lm in landmarks), "not every frame has landmark"

        return landmarks

    def affine_transform(
        self,
        frame,
        landmarks,
        reference,
        grayscale=True,
        target_size=(256, 256),
        reference_size=(256, 256),
        stable_points=(28, 33, 36, 39, 42, 45, 48, 54),
        interpolation=cv2.INTER_LINEAR,
        border_mode=cv2.BORDER_CONSTANT,
        border_value=0,
    ):
        if grayscale:
            frame = cv2.cvtColor(frame, cv2.COLOR_RGB2GRAY)
        stable_reference = self.get_stable_reference(
            reference, stable_points, reference_size, target_size
        )
        transform = self.estimate_affine_transform(
            landmarks, stable_points, stable_reference
        )
        transformed_frame, transformed_landmarks = self.apply_affine_transform(
            frame,
            landmarks,
            transform,
            target_size,
            interpolation,
            border_mode,
            border_value,
        )

        return transformed_frame, transformed_landmarks

    def get_stable_reference(
        self, reference, stable_points, reference_size, target_size
    ):
        stable_reference = np.vstack([reference[x] for x in stable_points])
        stable_reference[:, 0] -= (reference_size[0] - target_size[0]) / 2.0
        stable_reference[:, 1] -= (reference_size[1] - target_size[1]) / 2.0
        return stable_reference

    def estimate_affine_transform(self, landmarks, stable_points, stable_reference):
        return cv2.estimateAffinePartial2D(
            np.vstack([landmarks[x] for x in stable_points]),
            stable_reference,
            method=cv2.LMEDS,
        )[0]

    def apply_affine_transform(
        self,
        frame,
        landmarks,
        transform,
        target_size,
        interpolation,
        border_mode,
        border_value,
    ):
        transformed_frame = cv2.warpAffine(
            frame,
            transform,
            dsize=(target_size[0], target_size[1]),
            flags=interpolation,
            borderMode=border_mode,
            borderValue=border_value,
        )
        transformed_landmarks = (
            np.matmul(landmarks, transform[:, :2].transpose())
            + transform[:, 2].transpose()
        )
        return transformed_frame, transformed_landmarks


```
