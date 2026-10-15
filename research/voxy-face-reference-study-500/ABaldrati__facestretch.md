# Facial rendering source scan: ABaldrati/facestretch

Repository: https://github.com/ABaldrati/facestretch

Pinned source revision: fea4572d517b24322e15137e821fbe3f07f48612

Discovery topic: face deformation

Description: In facestretch we describe how we exploited the dlib’s facial landmarks in order to measure face deformation and perform an expression recognition task. We implemented multiple approaches, based metric learning, neural networks and geodesic distances

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"face": ["docs/facestretch.pdf", "shape_predictor_68_face_landmarks.dat"]}

## Source file: train.py

{
  "path": "train.py",
  "blob_sha": "d6632ad03204ed53e5f9f1d57fb2aaae6f9c8da3",
  "sha256": "e9c00d5fba70159d68af4125e90778cd6db35d8cae37f0e00cc6c3c79356a99c",
  "lines": 67,
  "bytes": 3238,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/ABaldrati/facestretch/blob/fea4572d517b24322e15137e821fbe3f07f48612/train.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def main():"
    }
  ]
}

```text

from datetime import datetime
from pathlib import Path

from joblib import dump
from metric_learn import ITML, LMNN, NCA, LFDA, SDML, MMC

from dataset import generate_weakly_supervised_interpolated_dataset, generate_training_supervised_dataset_categorical
from detect_landmarks import MODELS_PATH


def main():
    NAME_MODEL_TO_TRAIN = "NCA"
    # weakly-supervised possibilities "ITML" , "SDML" , "MMC"
    # supervised possibilities: "LMNN" , "NCA" , "LFDA"

    rates_list = [0.5, 1]

    training_start_time = datetime.now().isoformat()

    MODELS_PATH.mkdir(exist_ok=True)

    if NAME_MODEL_TO_TRAIN == "ITML":
        landmarks_matrix, training_pairs_indices, training_pairs_labels = generate_weakly_supervised_interpolated_dataset(
            Path("dataset_metric_learning").resolve(), rates_list)
        model = ITML(preprocessor=landmarks_matrix, max_iter=20_000, verbose=True)
        model.fit(training_pairs_indices, training_pairs_labels)
        dump(model, str(MODELS_PATH.joinpath(f'ITML_{training_start_time}.joblib')))

    elif NAME_MODEL_TO_TRAIN == "SDML":  # may not work due to graphical lasso solver
        landmarks_matrix, training_pairs_indices, training_pairs_labels = generate_weakly_supervised_interpolated_dataset(
            Path("dataset_metric_learning").resolve(), rates_list)
        model = SDML(preprocessor=landmarks_matrix, verbose=True, prior="identity", balance_param=0.2)
        model.fit(training_pairs_indices, training_pairs_labels)
        dump(model, str(MODELS_PATH.joinpath(f'SDML_{training_start_time}.joblib')))

    elif NAME_MODEL_TO_TRAIN == "MMC":
        landmarks_matrix, training_pairs_indices, training_pairs_labels = generate_weakly_supervised_interpolated_dataset(
            Path("dataset_metric_learning").resolve(), rates_list)
        model = MMC(preprocessor=landmarks_matrix, verbose=True, convergence_threshold=1e-5)
        model.fit(training_pairs_indices, training_pairs_labels)
        MODELS_PATH.mkdir(exist_ok=True)
        dump(model, str(MODELS_PATH.joinpath(f'MMC_{training_start_time}.joblib')))

    elif NAME_MODEL_TO_TRAIN == "LMNN":
        landmarks_matrix, training_labels = generate_training_supervised_dataset_categorical(
            Path("dataset_metric_learning").resolve())
        model = LMNN(max_iter=10000, convergence_tol=1e-5, verbose=True)
        model.fit(landmarks_matrix, training_labels)
        dump(model, str(MODELS_PATH.joinpath(f'LMNN_{training_start_time}.joblib')))

    elif NAME_MODEL_TO_TRAIN == "NCA":
        landmarks_matrix, training_labels = generate_training_supervised_dataset_categorical(
            Path("dataset_metric_learning").resolve())
        model = NCA(verbose=True)
        model.fit(landmarks_matrix, training_labels)
        dump(model, str(MODELS_PATH.joinpath(f'NCA_{training_start_time}.joblib')))

    elif NAME_MODEL_TO_TRAIN == "LFDA":
        landmarks_matrix, training_labels = generate_training_supervised_dataset_categorical(
            Path("dataset_metric_learning").resolve())
        model = LFDA()
        model.fit(landmarks_matrix, training_labels)
        dump(model, str(MODELS_PATH.joinpath(f'LFDA_{training_start_time}.joblib')))


if __name__ == '__main__':
    main()


```

## Source file: utils.py

{
  "path": "utils.py",
  "blob_sha": "e63bad3ecb535177313747fbecc0e0d5a400ef87",
  "sha256": "eeb64b3f0534e7e79d5c53ce49763b670ce082bec98dadbfc79cdab38b7d91f6",
  "lines": 150,
  "bytes": 5753,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 4,
    "face": 8,
    "lip": 1
  },
  "url": "https://github.com/ABaldrati/facestretch/blob/fea4572d517b24322e15137e821fbe3f07f48612/utils.py",
  "function_signature_candidates": [
    {
      "line": 16,
      "text": "def manifold(flattened_landmarks1, flattened_landmarks2):  # manifold distance"
    },
    {
      "line": 28,
      "text": "def get_rotation_matrix(angle):"
    },
    {
      "line": 36,
      "text": "def normalize_landmarks(landmarks):"
    },
    {
      "line": 55,
      "text": "def normalize_landmarks_eyes(landmarks):"
    },
    {
      "line": 78,
      "text": "def extract_landmarks(image) -> List[np.ndarray]:"
    },
    {
      "line": 96,
      "text": "def parse_image_path(path: Path):"
    },
    {
      "line": 100,
      "text": "def interpolate_landmarks(base_landmarks, target_landmarks, rate):"
    },
    {
      "line": 104,
      "text": "def add_images_pairings(paths_indices_mapping, similar_images, training_pairs_indices, training_pairs_labels,"
    },
    {
      "line": 121,
      "text": "def generate_images_from_video(src_path: Path, dst_folder_path: Path):"
    },
    {
      "line": 141,
      "text": "def rotate_image(image_array: np.ndarray, degree: int):"
    },
    {
      "line": 145,
      "text": "def horizontal_flip(image_array):"
    }
  ]
}

```text

import itertools
from pathlib import Path
from typing import List

import cv2
import dlib
import numpy as np
import skimage as sk
from imutils import face_utils
from matplotlib import pyplot as plt
from more_itertools import windowed
from scipy import linalg as la
from skimage import transform


def manifold(flattened_landmarks1, flattened_landmarks2):  # manifold distance
    landmarks1 = flattened_landmarks1.reshape((-1, 2))
    landmarks2 = flattened_landmarks2.reshape((-1, 2))
    g1 = np.outer(landmarks1, landmarks1.T)
    g2 = np.outer(landmarks2, landmarks2.T)
    # print('Printing matrix shapes'), print(pose_descriptor_1.shape), print(g1.shape)

    distance = np.trace(g1) + np.trace(g2) - 2.0 * np.trace(la.fractional_matrix_power(
        np.dot(la.fractional_matrix_power(g1, 0.5), np.dot(g2, la.fractional_matrix_power(g1, 0.5))), 0.5))
    return distance


def get_rotation_matrix(angle):
    cost = np.cos(np.deg2rad(angle))
    sint = np.sin(np.deg2rad(angle))
    rotation_matrix = np.array([[cost, -sint],
                                [sint, cost]])
    return rotation_matrix


def normalize_landmarks(landmarks):
    (x, y), (w, h), angle = cv2.minAreaRect(landmarks)

    if abs(angle + 90) < abs(angle):
        angle += 90

    landmarks_matrix = np.hstack([np.array(landmarks), np.ones((len(landmarks), 1))])
    center_point = (x + w / 2, y + h / 2)
    rotation_matrix = cv2.getRotationMatrix2D(center_point, angle, scale=1)
    rotated_landmarks = np.dot(landmarks_matrix, rotation_matrix.T)[:, :2]
    rotated_landmarks_int = np.int0(rotated_landmarks)

    x_bounding_rect, y_bounding_rect, w_bounding_rect, h_bounding_rect = cv2.boundingRect(rotated_landmarks_int)
    rotated_landmarks[:, 0] = (rotated_landmarks[:, 0] - x_bounding_rect) / w_bounding_rect
    rotated_landmarks[:, 1] = (rotated_landmarks[:, 1] - y_bounding_rect) / h_bounding_rect

    return rotated_landmarks


def normalize_landmarks_eyes(landmarks):
    """Normalize landmarks with respect to eyes and the nose instead of the whole landmarks' bounding box"""

    eyes_landmarks = landmarks[36:48]

    (x, y), (w, h), angle = cv2.minAreaRect(eyes_landmarks)

    if abs(angle + 90) < abs(angle):
        angle += 90

    landmarks_matrix = np.hstack([np.array(landmarks), np.ones((len(landmarks), 1))])
    center_point = tuple(landmarks[30])
    rotation_matrix = cv2.getRotationMatrix2D(center_point, angle, scale=1)
    rotated_landmarks = np.dot(landmarks_matrix, rotation_matrix.T)[:, :2]
    rotated_landmarks_int = np.int0(rotated_landmarks)

    x_bounding_rect, y_bounding_rect, w_bounding_rect, h_bounding_rect = cv2.boundingRect(rotated_landmarks_int)
    rotated_landmarks[:, 0] = (rotated_landmarks[:, 0] - x_bounding_rect) / w_bounding_rect
    rotated_landmarks[:, 1] = (rotated_landmarks[:, 1] - y_bounding_rect) / h_bounding_rect

    return rotated_landmarks


def extract_landmarks(image) -> List[np.ndarray]:
    grayscale_image = cv2.cvtColor(image, cv2.COLOR_BGR2GRAY)
    rects = detector(grayscale_image, 0)

    landmarks_faces = []

    for (i, rect) in enumerate(rects):
        # determine the facial landmarks for the face region, then
        # convert the facial landmark (x, y)-coordinates to a NumPy
        # array
        landmarks = predictor(grayscale_image, rect)
        landmarks = face_utils.shape_to_np(landmarks)

        landmarks_faces.append(landmarks)

    return landmarks_faces


def parse_image_path(path: Path):
    return path.stem.split("_")


def interpolate_landmarks(base_landmarks, target_landmarks, rate):
    return (1 - rate) * base_landmarks + rate * target_landmarks


def add_images_pairings(paths_indices_mapping, similar_images, training_pairs_indices, training_pairs_labels,
                        similarity, similar_pairs, sliding_window_size=15):
    for window in windowed(similar_images, sliding_window_size):
        for path1, path2 in itertools.combinations(window, 2):
            if (path1, path2) not in similar_pairs:
                index1 = paths_indices_mapping[path1]
                index2 = paths_indices_mapping[path2]

                training_pairs_indices = np.vstack([training_pairs_indices, np.array([index1, index2])])
                training_pairs_labels = np.append(training_pairs_labels, similarity)

                if similarity != -1:
                    similar_pairs.add((path1, path2))

    return training_pairs_indices, training_pairs_labels


def generate_images_from_video(src_path: Path, dst_folder_path: Path):
    subject, category = parse_image_path(src_path)
    video = cv2.VideoCapture(str(src_path))
    num_frames = int(video.get(cv2.CAP_PROP_FRAME_COUNT))
    frame_density = 100 / (num_frames - 1)
    current_frame = 0
    while video.isOpened():
        success, frame = video.read()
        if not success:
            break
        frame = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB, frame)
        # cv2.imwrite(str(dst_folder_path.joinpath(f"{subject}_{category}_{int(current_frame)}.jpg")), frame)
        if category != "neutro":
            plt.imsave(str(dst_folder_path.joinpath(f"{subject}_{category}_{int(round(current_frame))}.png")), frame)
        else:
            plt.imsave(str(dst_folder_path.joinpath(f"{subject}_{category}{int(current_frame)}_{0}.png")), frame)
        # current_frame = min(current_frame + frame_density, 100.)
        current_frame += frame_density


def rotate_image(image_array: np.ndarray, degree: int):
    return sk.transform.rotate(image_array, degree, preserve_range=True).astype(np.uint8)


def horizontal_flip(image_array):
    return image_array[:, ::-1]


detector = dlib.get_frontal_face_detector()
predictor = dlib.shape_predictor('shape_predictor_68_face_landmarks.dat')


```

## Source file: dataset.py

{
  "path": "dataset.py",
  "blob_sha": "8491f8d1b7df116939bf2551ca04167114c18c88",
  "sha256": "810e43cbcf1e4c8c71c576a2220bf4ccd579ae6c6347f6823dc1a9a516231904",
  "lines": 369,
  "bytes": 20045,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 19,
    "deform": 4,
    "face": 3
  },
  "url": "https://github.com/ABaldrati/facestretch/blob/fea4572d517b24322e15137e821fbe3f07f48612/dataset.py",
  "function_signature_candidates": [
    {
      "line": 16,
      "text": "def generate_weakly_supervised_interpolated_dataset(src_path: Path, rates: List[int], normalize_eyes=True):"
    },
    {
      "line": 140,
      "text": "def generate_training_weakly_supervised(src_path: Path, normalize_eyes=True):"
    },
    {
      "line": 262,
      "text": "def generate_training_supervised_dataset_categorical(src_path: Path, normalize_eyes=True):"
    },
    {
      "line": 325,
      "text": "def generate_training_supervised_dataset_regression(path: Path, normalize_eyes=True):"
    },
    {
      "line": 366,
      "text": "def generate_video_dataset(src_path: Path):"
    }
  ]
}

```text

import itertools
import operator
import sys
from pathlib import Path
from random import sample
from typing import List

import cv2
import numpy as np
from more_itertools import windowed

from utils import parse_image_path, extract_landmarks, normalize_landmarks, interpolate_landmarks, add_images_pairings, \
    generate_images_from_video, normalize_landmarks_eyes


def generate_weakly_supervised_interpolated_dataset(src_path: Path, rates: List[int], normalize_eyes=True):
    """ This function generate a dataset intended to be used in a weakly supervised algorithm where label `+1` is
        assigned to a pair with similar landmarks and label `-1` is assigned to a pair with dissimilar landmarks.
        In order to augment the pair considered we interpolate the sample between the neuter images and the action images
        with rate specified by `rates` parameter.
        Each sample is normalized subtracting to each action image the corresponding neuter image of the subject.
        Similar pair considered:
            * same rate, same action, different subjects
        Different pair considered:
            * different action, same rate, different subjects

        In order to work this function need the dataset image files in format: `name_action.ext`

        :param src_path: input folder
        :param rates: rates used to interpolate landmarks
        :param normalize_eyes: if True normalize landmarks with respect to eyes and the nose instead of the whole landmarks' bounding box
        :return:
            * landmarks_matrix: matrix which contains all the landmarks normalized, dim: [num_samples, 68 * 2]
            * training_pairs_indices: each row of this matrix contains the indices of the landmarks that form a pair taken
                into account, dim: [num_pairs, 2]
            * training_pairs_labels: vector which contains the similarity (+1, -1) for each pair
        """
    if 0 in rates:
        rates.remove(0)
    if 1 not in rates:
        rates.append(1)

    action_images = filter(lambda i: "neutro" not in i.name, src_path.iterdir())

    landmarks_matrix = np.empty((0, 68 * 2))
    landmark_indices_mapping = {}
    training_pairs_indices = np.empty((0, 2), dtype=np.int16)
    training_pairs_labels = np.empty((0, 1), dtype=np.int16)

    neuter_landmarks = {}

    subjects = set(map(lambda i: parse_image_path(i)[0], src_path.iterdir()))
    actions = set(map(lambda i: parse_image_path(i)[1], src_path.iterdir()))
    actions.remove("neutro")

    for subject in subjects:
        print(f"Importing subject {subject}")
        subject_neuter_image_path = list(src_path.glob(f"{subject}_neutro.*"))[0]
        subject_neuter_image = cv2.imread(str(subject_neuter_image_path), cv2.IMREAD_UNCHANGED)
        landmarks_found = extract_landmarks(subject_neuter_image)

        if not landmarks_found:
            print(f"Neuter landmarks not found for subject {subject}. Skipping...", file=sys.stderr)
            action_images = list(filter(lambda i: subject not in i.name, list(action_images)))

            continue

        if normalize_eyes:
            neuter_subject_landmarks = normalize_landmarks_eyes(landmarks_found[0]).flatten()
        else:
            neuter_subject_landmarks = normalize_landmarks(landmarks_found[0]).flatten()

        print(f"Extracted landmarks from {subject_neuter_image_path}")

        neuter_landmarks[subject] = neuter_subject_landmarks
        landmarks_matrix = np.vstack([landmarks_matrix, np.zeros(len(neuter_subject_landmarks))])
        landmark_indices_mapping[f"{subject}_neutro"] = landmarks_matrix.shape[0] - 1

    for image_path in action_images:
        print(f"Processing {image_path}")
        subject, action = parse_image_path(image_path)
        subject_action_image = cv2.imread(str(image_path), cv2.IMREAD_UNCHANGED)
        landmarks_found = extract_landmarks(subject_action_image)

        if not landmarks_found:
            print(f"Action landmarks ({action}) not found for subject {subject}. Skipping...", file=sys.stderr)
            continue

        action_subject_landmarks = normalize_landmarks(landmarks_found[0]).flatten()
        neuter_subject_landmarks = neuter_landmarks[subject]

        for rate in rates:
            interpolated_subject_landmarks = interpolate_landmarks(neuter_subject_landmarks, action_subject_landmarks,
                                                                   rate) - neuter_subject_landmarks
            landmarks_matrix = np.vstack([landmarks_matrix, interpolated_subject_landmarks])
            landmark_indices_mapping[f"{subject}_{action}_{rate}"] = landmarks_matrix.shape[0] - 1

    for action in actions:
        for rate in rates:
            for subj1, subj2 in itertools.combinations(subjects, 2):
                subj1_landmarks_index = landmark_indices_mapping.get(f"{subj1}_{action}_{rate}")
                subj2_landmarks_index = landmark_indices_mapping.get(f"{subj2}_{action}_{rate}")

                if not (subj1_landmarks_index and subj2_landmarks_index):
                    continue

                training_pairs_indices = np.vstack([training_pairs_indices,
                                                    np.array([subj1_landmarks_index, subj2_landmarks_index])])
                training_pairs_labels = np.append(training_pairs_labels, 1)

    for act1, act2 in itertools.combinations(actions, 2):
        for rate in rates:
            for subj1, subj2 in itertools.combinations(subjects, 2):
                subj1_landmarks_index = landmark_indices_mapping.get(f"{subj1}_{act1}_{rate}")
                subj2_landmarks_index = landmark_indices_mapping.get(f"{subj2}_{act2}_{rate}")

                if not (subj1_landmarks_index and subj2_landmarks_index):
                    continue

                training_pairs_indices = np.vstack([training_pairs_indices,
                                                    np.array([subj1_landmarks_index, subj2_landmarks_index])])
                training_pairs_labels = np.append(training_pairs_labels, -1)

    for action in actions:
        for rate in rates:
            for subj1, subj2 in itertools.combinations(subjects, 2):
                subj1_landmarks_index = landmark_indices_mapping.get(f"{subj1}_{action}_{rate}")
                subj2_landmarks_index = landmark_indices_mapping.get(f"{subj2}_neutro")

                if not (subj1_landmarks_index and subj2_landmarks_index):
                    continue

                training_pairs_indices = np.vstack([training_pairs_indices,
                                                    np.array([subj1_landmarks_index, subj2_landmarks_index])])
                training_pairs_labels = np.append(training_pairs_labels, -1)

    return landmarks_matrix, training_pairs_indices, training_pairs_labels


def generate_training_weakly_supervised(src_path: Path, normalize_eyes=True):
    """ NB: Use `generate_weakly_supervised_interpolated_dataset` which is proven to work better.

        This function generate a dataset intended to be used in a weakly supervised algorithm where label `+1` is
           assigned to a pair with similar landmarks and label `-1` is assigned to a pair with dissimilar landmarks.
           The `levels` variable manage the possible range of the action considered
           Similar pair considered:
               * same subject, same action, similar levels (+-15)
               * neuter images and first (5) action images of same subject
               * different subject, same action, similar levels
           Different pair considered:
               * otherwise

            In order to work this function need the dataset image files in format: `name_action_level.ext`

           :param src_path: input folder
           :param normalize_eyes: if True normalize landmarks with respect to eyes and the nose instead of the whole landmarks' bounding box
           :return:
               * landmarks_matrix: matrix which contains all the landmarks normalized, dim [num_samples, 68 * 2]
               * training_pairs_indices: each row of this matrix contains the indices of the landmarks that form a pair taken
                   into account, dim: [num_pairs, 2]
               * training_pairs_labels: vector which contains the similarity (+1, -1) for each pair
           """
    # This code assumes that each image in the training path has only one face in it

    paths_indices_mapping = {}
    landmarks_matrix = np.empty((0, 68 * 2))

    training_pairs_indices = np.empty((0, 2), dtype=np.int16)
    training_pairs_labels = np.empty((0, 1), dtype=np.int16)

    for image_path in sorted(src_path.iterdir(), key=lambda p: (
            p.name.split("_")[:-1], 101 if "neutro" in p.name else int(p.stem.split("_")[-1]))):
        image = cv2.imread(str(image_path), cv2.IMREAD_UNCHANGED)

        landmarks_list = extract_landmarks(image)

        if not landmarks_list:
            raise Exception(f"landmarks not found in image ({image_path})")

        for landmarks in landmarks_list:  # `landmarks_list` should contain only one element to properly populate `paths_indices_mapping`
            if normalize_eyes:
                normalized_landmarks = normalize_landmarks_eyes(landmarks)
            else:
                normalized_landmarks = normalize_landmarks(landmarks)

            landmarks_matrix = np.vstack([landmarks_matrix, normalized_landmarks.flatten()])
            paths_indices_mapping[image_path] = landmarks_matrix.shape[0] - 1

    neuter_images = list(filter(lambda f: "neutro" in f.name, src_path.iterdir()))
    occhiolinodx_images = list(filter(lambda f: "occhiolinodx" in f.name, src_path.iterdir()))
    occhiolinosx_images = list(filter(lambda f: "occhiolinosx" in f.name, src_path.iterdir()))
    cruccio_images = list(filter(lambda f: "cruccio" in f.name, src_path.iterdir()))
    sorriso_images = list(filter(lambda f: "sorriso" in f.name, src_path.iterdir()))
    sorrisino_images = list(filter(lambda f: "sorrisino" in f.name, src_path.iterdir()))
    gengive_images = list(filter(lambda f: "gengive" in f.name, src_path.iterdir()))
    bacio_images = list(filter(lambda f: "bacio" in f.name, src_path.iterdir()))

    all_images = [occhiolinodx_images,
                  occhiolinosx_images,
                  cruccio_images,
                  sorriso_images,
                  sorrisino_images,
                  gengive_images,
                  bacio_images]

    similar_pairs = set()

    for category_images in all_images:
        image_subjects = set(map(operator.itemgetter(0), map(parse_image_path, category_images)))

        for subject in image_subjects:
            subject_images = list(
                sorted(filter(lambda i: subject in i.name and "neutro" not in i.name, category_images),
                       key=lambda p: (
                           p.name.split("_")[:-1], 101 if "neutro" in p.name else int(p.stem.split("_")[-1]))))
            training_pairs_indices, training_pairs_labels = add_images_pairings(paths_indices_mapping, subject_images,
                                                                                training_pairs_indices,
                                                                                training_pairs_labels, 1,
                                                                                similar_pairs)

            lows = range(5)
            images_low = list(filter(lambda i: any(str(level) in i.name for level in lows), subject_images))
            for neuter_image in filter(lambda i: subject in i.name, neuter_images):
                for image_low in images_low:
                    training_pairs_indices, training_pairs_labels = add_images_pairings(paths_indices_mapping,
                                                                                        [neuter_image, image_low],
                                                                                        training_pairs_indices,
                                                                                        training_pairs_labels, 1,
                                                                                        similar_pairs,
                                                                                        sliding_window_size=2)

        levels = range(101)

        for window_levels in windowed(levels, 5):
            level_images = list(filter(lambda i: any(str(level) in i.name for level in window_levels), category_images))
            training_pairs_indices, training_pairs_labels = add_images_pairings(paths_indices_mapping, level_images,
                                                                                training_pairs_indices,
                                                                                training_pairs_labels, 1, similar_pairs,
                                                                                sliding_window_size=len(level_images))

    for neuter_image_1, neuter_image_2 in itertools.combinations(neuter_images, 2):
        training_pairs_indices, training_pairs_labels = add_images_pairings(paths_indices_mapping,
                                                                            [neuter_image_1, neuter_image_2],
                                                                            training_pairs_indices,
                                                                            training_pairs_labels, 1, similar_pairs,
                                                                            sliding_window_size=2)

    all_images_flattened = list(itertools.chain.from_iterable(all_images))
    combinations = list(sample(list(itertools.combinations(all_images_flattened, 2)), len(similar_pairs)))
    for path1, path2 in combinations:
        if ((path1, path2) not in similar_pairs) and ((path2, path1) not in similar_pairs):
            training_pairs_indices, training_pairs_labels = add_images_pairings(paths_indices_mapping,
                                                                                [path1, path2],
                                                                                training_pairs_indices,
                                                                                training_pairs_labels, -1,
                                                                                similar_pairs,
                                                                                sliding_window_size=2)

    return landmarks_matrix, training_pairs_indices, training_pairs_labels


def generate_training_supervised_dataset_categorical(src_path: Path, normalize_eyes=True):
    """ This function generate a dataset intended to be used in a supervised algorithm.
        Each sample is normalized subtracting to each action image the corresponding neuter image of the subject.
        In order to work this function need the dataset image files in format: `name_action.ext`

        :param src_path: input folder
        :param normalize_eyes: if True normalize landmarks with respect to eyes and the nose instead of the whole landmarks' bounding box
        :return:
            * landmarks_matrix: matrix which contains all the normalized landmarks, dim: [num_samples, 68 * 2]
            * training_pairs_labels: vector which contains the action for each landmark
        """
    # This code assumes that each image in the training path has only one face in it

    landmarks_matrix = np.empty((0, 68 * 2))
    training_paris_labels = []
    training_paris_labels = np.array(training_paris_labels, dtype="str")

    neuter_landmarks = {}
    subjects = set(map(lambda i: parse_image_path(i)[0], src_path.iterdir()))

    path_images = [i for i in src_path.iterdir()]

    for subject in subjects:
        print(f"Importing subject {subject}")
        subject_neuter_image_path = list(src_path.glob(f"{subject}_neutro.*"))[0]
        subject_neuter_image = cv2.imread(str(subject_neuter_image_path), cv2.IMREAD_UNCHANGED)
        landmarks_found = extract_landmarks(subject_neuter_image)

        if not landmarks_found:
            print(f"Neuter landmarks not found for subject {subject}. Skipping...", file=sys.stderr)
            path_images = list(filter(lambda i: subject not in i.name, list(path_images)))
            continue

        if normalize_eyes:
            neuter_subject_landmarks = normalize_landmarks_eyes(landmarks_found[0]).flatten()
        else:
            neuter_subject_landmarks = normalize_landmarks(landmarks_found[0]).flatten()
        print(f"Extracted landmarks from {subject_neuter_image_path}")
        neuter_landmarks[subject] = neuter_subject_landmarks

    for image_path in sorted(path_images):
        image = cv2.imread(str(image_path), cv2.IMREAD_UNCHANGED)

        landmarks_list = extract_landmarks(image)

        if not landmarks_list:
            print(f"landmarks not found in image ({image_path})", file=sys.stderr)
            continue

        for landmarks in landmarks_list:  # `landmarks_list` should contain only one element to properly populate `paths_indices_mapping`
            if normalize_eyes:
                normalized_landmarks = normalize_landmarks_eyes(landmarks)
            else:
                normalized_landmarks = normalize_landmarks(landmarks)

            normalized_landmarks = normalized_landmarks.flatten() - neuter_landmarks[parse_image_path(image_path)[0]]
            landmarks_matrix = np.vstack([landmarks_matrix, normalized_landmarks])
            category = parse_image_path(image_path)[1]
            training_paris_labels = np.append(training_paris_labels, category)

    return landmarks_matrix, training_paris_labels


def generate_training_supervised_dataset_regression(path: Path, normalize_eyes=True):
    """ NB: In supervised training use `generate_training_supervised_dataset_categorical` which works better
        This function generate a dataset intended to be used in a supervised algorithm.
        In order to work this function need the dataset image files in format: `name_action_level.ext`

       :param src_path: input folder
       :param normalize_eyes: if True normalize landmarks with respect to eyes and the nose instead of the whole landmarks' bounding box
       :return:
           * landmarks_matrix: matrix which contains all the normalized landmarks, dim: [num_samples, 68 * 2]
           * training_pairs_labels: vector which contains the progress of the action
       """
    # This code assumes that each image in the training path has only one face in it

    landmarks_matrix = np.empty((0, 68 * 2))
    training_paris_labels = []
    training_paris_labels = np.array(training_paris_labels, dtype=np.int8)

    for image_path in sorted(path.iterdir(), key=lambda p: (
            p.name.split("_")[:-1], 101 if "neutro" in p.name else int(p.stem.split("_")[-1]))):
        image = cv2.imread(str(image_path), cv2.IMREAD_UNCHANGED)

        landmarks_list = extract_landmarks(image)

        if not landmarks_list:
            raise Exception(f"landmarks not found in image ({image_path})")

        for landmarks in landmarks_list:  # `landmarks_list` should contain only one element to properly populate `paths_indices_mapping`
            if normalize_eyes:
                normalized_landmarks = normalize_landmarks_eyes(landmarks)
            else:
                normalized_landmarks = normalize_landmarks(landmarks)

            landmarks_matrix = np.vstack([landmarks_matrix, normalized_landmarks.flatten()])
            deformation_index = parse_image_path(image_path)[2]
            if type(deformation_index) is str:
                deformation_index = 0
            training_paris_labels = np.append(training_paris_labels, deformation_index / 100)

    return landmarks_matrix, training_paris_labels


def generate_video_dataset(src_path: Path):
    dst_folder_path = src_path.parent.joinpath("dataset_video_images")
    for video in src_path.iterdir():
        generate_images_from_video(video, dst_folder_path)


```

## Source file: neural_network.py

{
  "path": "neural_network.py",
  "blob_sha": "b3c108c72b5bf15794c7d63a414c271594ed07bc",
  "sha256": "c23dfec93972f74ee2ccce82d1611ebb829db8c96e94cd4112e20e6d7f7bd772",
  "lines": 429,
  "bytes": 21106,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 17,
    "face": 11,
    "lip": 21
  },
  "url": "https://github.com/ABaldrati/facestretch/blob/fea4572d517b24322e15137e821fbe3f07f48612/neural_network.py",
  "function_signature_candidates": [
    {
      "line": 15,
      "text": "def generate_neural_network_dataset(src_path: Path, rates: List[int], normalize_eyes=True):"
    },
    {
      "line": 178,
      "text": "def dataset_generator(src_path: Path, batch_size: int, normalize_eyes=True):"
    },
    {
      "line": 358,
      "text": "def main():"
    }
  ]
}

```text

import random
import sys
from pathlib import Path
from typing import List

import cv2
import matplotlib.pyplot as plt
import numpy as np
from tensorflow import keras

from utils import parse_image_path, extract_landmarks, normalize_landmarks, interpolate_landmarks, \
    normalize_landmarks_eyes, rotate_image, horizontal_flip


def generate_neural_network_dataset(src_path: Path, rates: List[int], normalize_eyes=True):
    """
        This function generate a dataset intended to be used by a neural network.
        Each sample is normalized subtracting to each action image the corresponding neuter image of the subject.
        In order to augment the pair considered we interpolate the sample between the neuter images and the action images
        with rates specified by `rates` parameter. Since a neural network need a lot of data to work properly we perform
        a data augmentation flipping, rotating and cropping the images.
        In order to work this function need the dataset image files in format: `name_action.ext`

        We have used this function to generate the validation dataset since we need the same dataset for each epoch in
        order to have a fair comparison among them.

        :param src_path: input folder
        :param rates: rates used to interpolate landmarks
        :param normalize_eyes: if True normalize landmarks with respect to eyes and the nose instead of the whole landmarks' bounding box
        :return:
            * landmarks_matrix: matrix which contains all the normalized landmarks, dim: [num_samples, 68 * 2]
            * training_pairs_labels: matrix with the labels for each action, dim: [num_samples, num_actions]
      """

    NUM_CROPPING_ITERATIONS = 5

    MIN_ROTATION_DEGREE = -14
    MAX_ROTATION_DEGREE = 16
    STEP_ROTATION_DEGREE = 2

    CROPPING_TOLERANCE = 0.7

    if 0 in rates:
        rates.remove(0)
    if 1 not in rates:
        rates.append(1)

    action_images = filter(lambda i: "neutro" not in i.name, src_path.iterdir())

    landmarks_matrix = np.empty((0, 68 * 2))
    labels = np.empty((0, 8), dtype=np.int16)

    neuter_landmarks = {}

    subjects = sorted(set(map(lambda i: parse_image_path(i)[0], src_path.iterdir())))
    actions = sorted(set(map(lambda i: parse_image_path(i)[1], src_path.iterdir())))

    face_to_label_matrix = np.identity(len(actions))
    face_row_mapping = {}
    for i, action in enumerate(sorted(list(actions))):
        face_row_mapping[action] = i

    for subject in subjects:
        print(f"Importing subject {subject}")
        subject_neuter_image_path = list(src_path.glob(f"{subject}_neutro.*"))[0]
        subject_neuter_image = cv2.imread(str(subject_neuter_image_path), cv2.IMREAD_UNCHANGED)
        landmarks_subject_image = extract_landmarks(subject_neuter_image)

        if not landmarks_subject_image:
            print(f"Neuter landmarks not found for subject {subject}. Skipping...", file=sys.stderr)
            action_images = list(filter(lambda i: subject not in i.name, list(action_images)))

            continue

        landmark_min_x = np.min(landmarks_subject_image[0][:, 0])
        landmark_max_x = np.max(landmarks_subject_image[0][:, 0])
        landmark_min_y = np.min(landmarks_subject_image[0][:, 1])
        landmark_max_y = np.max(landmarks_subject_image[0][:, 1])

        if normalize_eyes:
            neuter_subject_landmarks = normalize_landmarks_eyes(landmarks_subject_image[0]).flatten()
        else:
            neuter_subject_landmarks = normalize_landmarks(landmarks_subject_image[0]).flatten()

        print(f"Extracted landmarks from {subject_neuter_image_path}")

        neuter_landmarks[subject] = neuter_subject_landmarks
        # landmarks_matrix = np.vstack([landmarks_matrix, neuter_subject_landmarks])
        for ith_cropping in range(NUM_CROPPING_ITERATIONS):
            cropping_min_x_coordinate = random.randint(0, int(CROPPING_TOLERANCE * landmark_min_x))
            cropping_max_x_coordinate = random.randint(
                subject_neuter_image.shape[1] - int(
                    CROPPING_TOLERANCE * (subject_neuter_image.shape[1] - landmark_max_x)),
                subject_neuter_image.shape[1])
            cropping_min_y_coordinate = random.randint(0, int(CROPPING_TOLERANCE * landmark_min_y))
            cropping_max_y_coordinate = random.randint(
                subject_neuter_image.shape[0] - int(
                    CROPPING_TOLERANCE * (subject_neuter_image.shape[0] - landmark_max_y)),
                subject_neuter_image.shape[0])

            cropped_image = subject_neuter_image[cropping_min_y_coordinate:cropping_max_y_coordinate,
                            cropping_min_x_coordinate: cropping_max_x_coordinate, :]

            for degree in range(MIN_ROTATION_DEGREE, MAX_ROTATION_DEGREE, STEP_ROTATION_DEGREE):
                rotated_image = rotate_image(cropped_image, degree)
                landmarks_cropped_rotated_found = extract_landmarks(rotated_image)
                if not landmarks_cropped_rotated_found:
                    print(
                        f"Neuter landmarks not found for subject {subject}. At angle {degree} at {ith_cropping} Skipping...",
                        file=sys.stderr)
                    action_images = list(filter(lambda i: subject not in i.name, list(action_images)))

                else:
                    if normalize_eyes:
                        normalized_landmarks = normalize_landmarks_eyes(landmarks_cropped_rotated_found[0]).flatten()
                    else:
                        normalized_landmarks = normalize_landmarks(landmarks_cropped_rotated_found[0]).flatten()
                    rescaled_normalized_landmarks = normalized_landmarks - neuter_landmarks[subject]
                    landmarks_matrix = np.vstack([landmarks_matrix, rescaled_normalized_landmarks])
                    labels = np.vstack([labels, face_to_label_matrix[face_row_mapping["neutro"]]])

    for image_path in sorted(action_images):
        print(f"Processing {image_path}")
        subject, action = parse_image_path(image_path)
        subject_action_image = cv2.imread(str(image_path), cv2.IMREAD_UNCHANGED)
        landmarks_subject_action_image = extract_landmarks(subject_action_image)
        if not landmarks_subject_action_image:
            print(f"Action landmarks ({action}) not found for subject {subject} Skipping...",
                  file=sys.stderr)
            continue
        landmark_min_x = np.min(landmarks_subject_action_image[0][:, 0])
        landmark_max_x = np.max(landmarks_subject_action_image[0][:, 0])
        landmark_min_y = np.min(landmarks_subject_action_image[0][:, 1])
        landmark_max_y = np.max(landmarks_subject_action_image[0][:, 1])

        for ith_cropping in range(NUM_CROPPING_ITERATIONS):
            cropping_min_x_coordinate = random.randint(0, int(CROPPING_TOLERANCE * landmark_min_x))
            cropping_max_x_coordinate = random.randint(
                subject_action_image.shape[1] - int(
                    CROPPING_TOLERANCE * (subject_action_image.shape[1] - landmark_max_x)),
                subject_action_image.shape[1])
            cropping_min_y_coordinate = random.randint(0, int(CROPPING_TOLERANCE * landmark_min_y))
            cropping_max_y_coordinate = random.randint(
                subject_action_image.shape[0] - int(
                    CROPPING_TOLERANCE * (subject_action_image.shape[0] - landmark_max_y)),
                subject_action_image.shape[0])

            cropped_image = subject_action_image[cropping_min_y_coordinate:cropping_max_y_coordinate,
                            cropping_min_x_coordinate: cropping_max_x_coordinate, :]

            for degree in range(MIN_ROTATION_DEGREE, MAX_ROTATION_DEGREE, STEP_ROTATION_DEGREE):
                rotated_image = rotate_image(cropped_image, degree)
                landmarks_cropped_rotated_found = extract_landmarks(rotated_image)
                if not landmarks_cropped_rotated_found:
                    print(
                        f"Action landmarks ({action}) not found for subject {subject}. At angle {degree} at {ith_cropping} Skipping...",
                        file=sys.stderr)
                else:
                    if normalize_eyes:
                        action_subject_landmarks = normalize_landmarks_eyes(
                            landmarks_cropped_rotated_found[0]).flatten()
                    else:
                        action_subject_landmarks = normalize_landmarks(landmarks_cropped_rotated_found[0]).flatten()

                    neuter_subject_landmarks = neuter_landmarks[subject]

                    for rate in rates:
                        interpolated_subject_landmarks = interpolate_landmarks(neuter_subject_landmarks,
                                                                               action_subject_landmarks,
                                                                               rate) - neuter_subject_landmarks
                        landmarks_matrix = np.vstack([landmarks_matrix, interpolated_subject_landmarks])
                        labels = np.vstack([labels, rate * face_to_label_matrix[face_row_mapping[action]] +
                                            (1 - rate) * face_to_label_matrix[face_row_mapping["neutro"]]])

    return landmarks_matrix, labels


def dataset_generator(src_path: Path, batch_size: int, normalize_eyes=True):
    """
        This function is a generator which each step create a batch of data for a neural network training.
        Each sample is normalized subtracting to each action image the corresponding neuter image of the subject.
        In order to augment the pair considered we interpolate the sample between the neuter images and the action images
        with random rates (actually three random rates and the rate `1`) . Since a neural network need a lot of data to work properly we perform
        a data augmentation flipping, rotating and cropping the images.
        In order to work this function need the dataset image files in format: `name_action.ext`

        We use this generator for dynamically create the training dataset so that the neural network didn't see the same
        image twice.

        :param src_path: input folder
        :param batch_size: number of samples for each batch
        :param normalize_eyes: if True normalize landmarks with respect to eyes and the nose instead of the whole landmarks' bounding box
        :return:
            * batch_landmarks_matrix: matrix which contains all the normalized landmarks in batch, dim: [batch_size, 68 * 2]
            * training_pairs_labels: matrix with the labels for each action in batch, dim: [batch_size, num_actions]
      """

    MIN_ROTATION_DEGREE = -14
    MAX_ROTATION_DEGREE = 14

    CROPPING_TOLERANCE = 0.7

    neuter_landmarks = {}

    subjects = sorted(set(map(lambda i: parse_image_path(i)[0], src_path.iterdir())))
    actions = sorted(set(map(lambda i: parse_image_path(i)[1], src_path.iterdir())))

    face_to_label_matrix = np.identity(len(actions))
    face_row_mapping = {}
    for i, action in enumerate(sorted(list(actions))):
        face_row_mapping[action] = i

    for subject in subjects:
        print(f"Importing subject {subject}")
        subject_neuter_image_path = list(src_path.glob(f"{subject}_neutro.*"))[0]
        subject_neuter_image = cv2.imread(str(subject_neuter_image_path), cv2.IMREAD_UNCHANGED)
        landmarks_subject_image = extract_landmarks(subject_neuter_image)

        if not landmarks_subject_image:
            print(f"Neuter landmarks not found for subject {subject}. Skipping...", file=sys.stderr)
            subjects.remove(subject)

            continue

        if normalize_eyes:
            neuter_subject_landmarks = normalize_landmarks_eyes(landmarks_subject_image[0]).flatten()
        else:
            neuter_subject_landmarks = normalize_landmarks(landmarks_subject_image[0]).flatten()

        print(f"Extracted landmarks from {subject_neuter_image_path}")

        neuter_landmarks[subject] = neuter_subject_landmarks

        # flipping the image

        flipped_image = horizontal_flip(subject_neuter_image)
        landmarks_subject_flipped_image = extract_landmarks(flipped_image)
        if not landmarks_subject_flipped_image:
            print(f"Neuter landmarks not found for subject {subject}-flipped. Skipping...", file=sys.stderr)
            subjects.remove(subject)
            continue

        if normalize_eyes:
            neuter_subject_flipped_landmarks = normalize_landmarks_eyes(landmarks_subject_flipped_image[0]).flatten()
        else:
            neuter_subject_flipped_landmarks = normalize_landmarks(landmarks_subject_flipped_image[0]).flatten()

        print(f"Extracted landmarks flipped from {subject_neuter_image_path}")

        neuter_landmarks[f"{subject}_flipped"] = neuter_subject_flipped_landmarks

    array_subjects = np.array(list(subjects))
    array_actions = np.array(list(actions))
    flip_image = np.array([True, False])
    while True:
        batch_landmarks_matrix = np.empty((0, 68 * 2))
        batch_labels = np.empty((0, 8), dtype=np.int8)
        while len(batch_labels) < batch_size:
            current_action = np.random.choice(array_actions)
            current_subject = np.random.choice(array_subjects)
            do_flip_image = np.random.choice(flip_image)
            current_image_angle_rotation = random.randint(MIN_ROTATION_DEGREE, MAX_ROTATION_DEGREE)
            rates = np.random.rand(3)
            rates = np.append(rates, 1)

            image_name = f"{current_subject}_{current_action}"
            rough_image_path_list = list(src_path.glob(f"{image_name}.*"))
            if not rough_image_path_list:
                # print(f"file with name {image_name} not found", file=sys.stderr)
                continue

            rough_image_path = rough_image_path_list[0]
            image = cv2.imread(str(rough_image_path), cv2.IMREAD_UNCHANGED)
            if do_flip_image:
                image = horizontal_flip(image)
                current_neuter_landmark = neuter_landmarks.get(f"{current_subject}_flipped")
            else:
                current_neuter_landmark = neuter_landmarks.get(current_subject)

            if current_neuter_landmark is None:
                print(f"neuter image of subject {current_subject} not available", file=sys.stderr)
                continue

            image_landmarks = extract_landmarks(image)
            if not image_landmarks:
                print(f"No landmark found in {image_name} with flipping = {do_flip_image}", file=sys.stderr)
                continue

            # image cropping
            landmark_min_x = np.min(image_landmarks[0][:, 0])
            landmark_max_x = np.max(image_landmarks[0][:, 0])
            landmark_min_y = np.min(image_landmarks[0][:, 1])
            landmark_max_y = np.max(image_landmarks[0][:, 1])

            cropping_min_x_coordinate = random.randint(0, int(CROPPING_TOLERANCE * landmark_min_x))
            cropping_max_x_coordinate = random.randint(
                image.shape[1] - int(CROPPING_TOLERANCE * (image.shape[1] - landmark_max_x)),
                image.shape[1])
            cropping_min_y_coordinate = random.randint(0, int(CROPPING_TOLERANCE * landmark_min_y))
            cropping_max_y_coordinate = random.randint(
                image.shape[0] - int(CROPPING_TOLERANCE * (image.shape[0] - landmark_max_y)),
                image.shape[0])

            image = image[cropping_min_y_coordinate:cropping_max_y_coordinate,
                    cropping_min_x_coordinate: cropping_max_x_coordinate, :]

            # rotating image
            final_image = rotate_image(image, current_image_angle_rotation)

            final_image_landmarks = extract_landmarks(final_image)
            if not final_image_landmarks:
                print(
                    f"No landmark found in {image_name} rotate of {current_image_angle_rotation} cropped and with flip = {do_flip_image}",
                    file=sys.stderr)
                continue

            if normalize_eyes:
                normalized_landmarks = normalize_landmarks_eyes(final_image_landmarks[0]).flatten()
            else:
                normalized_landmarks = normalize_landmarks(final_image_landmarks[0]).flatten()

            for rate in rates:
                rescaled_interpolated_landmark = interpolate_landmarks(current_neuter_landmark,
                                                                       normalized_landmarks,
                                                                       rate) - current_neuter_landmark
                batch_landmarks_matrix = np.vstack([batch_landmarks_matrix, rescaled_interpolated_landmark])
                if "sx" == current_action[-2:] and do_flip_image:
                    current_action = current_action[:-2] + "dx"
                elif "dx" == current_action[-2:] and do_flip_image:
                    current_action = current_action[:-2] + "sx"

                batch_labels = np.vstack([batch_labels, rate * face_to_label_matrix[face_row_mapping[current_action]] +
                                          (1 - rate) * face_to_label_matrix[face_row_mapping["neutro"]]])

        yield batch_landmarks_matrix, batch_labels


callbacks_list = [
    keras.callbacks.EarlyStopping(
        monitor='val_loss',
        patience=25,
        mode='auto',
        #   min_delta=-0.0001
    ),
    keras.callbacks.ModelCheckpoint(
        filepath='./neural_model/saved-models-{epoch:06d}-{val_loss:.5f}.h5',
        monitor='val_loss',
        save_best_only=False
    ),
    keras.callbacks.CSVLogger(
        filename='./neural_model/my_model.csv',
        separator=',',
        append=True
    ),
]


def main():
    batch_size = 200
    neural_model_path = Path("neural_model")
    neural_model_path.mkdir(exist_ok=True)

    neural_validation_landmarks_path = Path("neural_validation_landmarks.npy")
    neural_validation_labels_path = Path("neural_validation_labels.npy")

    if not neural_validation_labels_path.exists() or not neural_validation_landmarks_path.exists():
        print("Generating validation dataset...")

        neural_validation_landmarks, neural_validation_labels = generate_neural_network_dataset(
            Path("dataset_neural_validation").resolve(), [0.2, 0.4, 0.6, 0.8, 1])
        np.save(str(neural_validation_landmarks_path), neural_validation_landmarks)
        np.save(str(neural_validation_labels_path), neural_validation_labels)
    else:
        neural_validation_landmarks = np.load(str(neural_validation_landmarks_path))
        neural_validation_labels = np.load(str(neural_validation_labels_path))

    training_generator = dataset_generator(Path("dataset_neural_training").resolve(), batch_size=batch_size)
    validation_data = tuple([neural_validation_landmarks, neural_validation_labels])

    model = keras.Sequential()
    model.add(keras.layers.Input(batch_input_shape=[None, 68 * 2]))
    model.add(keras.layers.BatchNormalization())
    model.add(keras.layers.Dense(64, kernel_initializer="he_normal", activation="elu"))
    model.add(keras.layers.BatchNormalization())
    model.add(keras.layers.Dropout(0.1))
    model.add(keras.layers.Dense(64, kernel_initializer="he_normal", activation="elu"))
    model.add(keras.layers.BatchNormalization())
    model.add(keras.layers.Dropout(0.1))
    model.add(keras.layers.Dense(32, kernel_initializer="he_normal", activation="elu"))
    model.add(keras.layers.BatchNormalization())
    model.add(keras.layers.Dropout(0.1))
    model.add(keras.layers.Dense(8, activation="sigmoid"))  # 8 = num_actions, change if you use more actions

    model.compile(optimizer=keras.optimizers.Adam(learning_rate=0.0005),
                  loss='binary_crossentropy',
                  metrics=['accuracy'])

    history = model.fit_generator(generator=training_generator,
                                  steps_per_epoch=50,
                                  epochs=100,
                                  verbose=1,
                                  validation_data=validation_data,
                                  callbacks=callbacks_list)

    loss = history.history['loss']
    val_loss = history.history['val_loss']
    epochs = range(1, len(loss) + 1)
    plt.plot(epochs, loss, 'bo', label='Training loss')
    plt.plot(epochs, val_loss, 'b', label='Validation loss')
    plt.title('Training and validation loss')
    plt.xlabel('Epochs')
    plt.ylabel('Loss')
    plt.legend()
    plt.savefig('./neural_model/training-validation-loss')

    plt.clf()
    accuracy = history.history['accuracy']
    val_accuracy = history.history['val_accuracy']
    plt.plot(epochs, accuracy, 'bo', label='Training accuracy')
    plt.plot(epochs, val_accuracy, 'b', label='Validation accuracy')
    plt.title('Training and validation accuracy')
    plt.xlabel('Epochs')
    plt.ylabel('acc')
    plt.legend()
    plt.savefig('./neural_model/training-validation-accuracy')


if __name__ == '__main__':
    main()


```
