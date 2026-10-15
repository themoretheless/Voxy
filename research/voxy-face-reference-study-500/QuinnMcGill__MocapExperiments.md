# Facial rendering source scan: QuinnMcGill/MocapExperiments

Repository: https://github.com/QuinnMcGill/MocapExperiments

Pinned source revision: 09422f6a673192df1ae0e82a803d6dc06ce06d93

Discovery topic: facial mocap

Description: This repo is created as part of my thesis research into machine learning-enhanced facial motion capture.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"blendshape": ["MediaPipe/models/face_landmarker_v2_with_blendshapes.task"], "face": ["MediaPipe/models/face_landmarker_v2_with_blendshapes.task", "OpenFace2.0/lip_polygon_output/tc11_editted_polygons.csv", "OpenFace2.0/lip_polygon_output/tc12_editted_polygons.csv", "OpenFace2.0/lip_polygon_output/tc13_1_editted_polygons.csv", "OpenFace2.0/lip_polygon_output/tc13_editted_polygons.csv", "OpenFace2.0/lip_polygon_output/tc14_1_editted_polygons.csv", "OpenFace2.0/lip_polygon_output/tc14_editted_polygons.csv", "OpenFace3.0/__init__.py", "OpenFace3.0/create_layout_image.py", "OpenFace3.0/csv_files/face_analysis_1.csv", "OpenFace3.0/csv_files/face_analysis_2.csv", "OpenFace3.0/csv_files/face_analysis_3.csv", "OpenFace3.0/csv_files/face_analysis_4.csv", "OpenFace3.0/csv_files/face_analysis_5.csv", "OpenFace3.0/csv_files/face_analysis_6.csv", "OpenFace3.0/detect_faces.py", "OpenFace3.0/detected_faces/1.jpg", "OpenFace3.0/detected_faces/2.jpg", "OpenFace3.0/detected_faces/3.jpg", "OpenFace3.0/detected_faces/4.jpg", "OpenFace3.0/detected_faces/5.jpg", "OpenFace3.0/detected_faces/6.jpg", "OpenFace3.0/openface_utils.py", "OpenFace3.0/predict_landmarks.py", "OpenFace3.0/weights/.gitattributes", "OpenFace3.0/weights/Alignment_RetinaFace.pth", "OpenFace3.0/weights/Landmark_68.pkl", "OpenFace3.0/weights/Landmark_98.pkl", "OpenFace3.0/weights/MTL_backbone.pth", "OpenFace3.0/weights/mobilenetV1X0.25_pretrain.tar"], "lip": ["DLib/lip_polygon_output/tc13_1_editted_polygons.csv", "DLib/lip_polygon_output/tc14_1_editted_polygons.csv", "MediaPipe/lip_polygon_output/tc11_editted_polygons.csv", "MediaPipe/lip_polygon_output/tc12_editted_polygons.csv", "MediaPipe/lip_polygon_output/tc13_1_editted_polygons.csv", "MediaPipe/lip_polygon_output/tc13_editted_polygons.csv", "MediaPipe/lip_polygon_output/tc14_1_editted_polygons.csv", "MediaPipe/lip_polygon_output/tc14_editted_polygons.csv", "OpenFace2.0/lip_polygon_output/tc11_editted_polygons.csv", "OpenFace2.0/lip_polygon_output/tc12_editted_polygons.csv", "OpenFace2.0/lip_polygon_output/tc13_1_editted_polygons.csv", "OpenFace2.0/lip_polygon_output/tc13_editted_polygons.csv", "OpenFace2.0/lip_polygon_output/tc14_1_editted_polygons.csv", "OpenFace2.0/lip_polygon_output/tc14_editted_polygons.csv", "recolour_lips.py"]}

## Source file: recolour_lips.py

{
  "path": "recolour_lips.py",
  "blob_sha": "3e3a12d5ea6ba49ea5b606d10f213e7db697cb3b",
  "sha256": "f1fd6327e48626717e51a99b4ced91d52383b38fd4aa23fe82ee0300f7089138",
  "lines": 90,
  "bytes": 2379,
  "selection_score": 15,
  "keyword_line_counts": {
    "gum": 3,
    "rig": 1,
    "lip": 2
  },
  "url": "https://github.com/QuinnMcGill/MocapExperiments/blob/09422f6a673192df1ae0e82a803d6dc06ce06d93/recolour_lips.py",
  "function_signature_candidates": []
}

```text

import argparse
import cv2
import numpy as np
import os
from recolouring_utils import *

# ==== Parse command-line arguments ==== #
parser = argparse.ArgumentParser(description="Visualize ARKit mocap data")
parser.add_argument("--v", help="Video file name in data folder", default="tc6.mp4")

args = parser.parse_args()

# ==== Variable Initialization ==== #
tc_name = args.v.split(".")[0]
input_video_path = "data/" + args.v
print("Input path:", input_video_path)
output_video_path = "data/editted_vids/" + tc_name + "_1_editted.mp4"

cap = cv2.VideoCapture(input_video_path)

# Get input video properties
fps = cap.get(cv2.CAP_PROP_FPS)
frame_width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
frame_height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

# Create MP4 writer
fourcc = cv2.VideoWriter.fourcc(*"mp4v")
video_writer = cv2.VideoWriter(
    output_video_path,
    fourcc,
    fps,
    (frame_width, frame_height)
)

print("Output path:", output_video_path)
frame_idx = -1

while cap.isOpened():

    ret, frame = cap.read()
    if not ret:
        break

    frame_idx += 1

    hsv = cv2.cvtColor(frame, cv2.COLOR_BGR2HSV)

    green_mask, red_mask = get_masks(tc_name, hsv)

    green_mask = largest_connected_component(green_mask)
    red_mask = largest_connected_component(red_mask)

    # Close any holes in the masks
    green_mask = close_mask_holes(green_mask)
    red_mask = close_mask_holes(red_mask)

    # Change hue to red while preserving saturation/value
    hsv_modified = hsv.copy()

    # Change the lips to a more natural colour
    # hsv_modified[..., 0][green_mask > 0] = 178  
    # hsv_modified[..., 1][green_mask > 0] = 165

    # hsv_modified[..., 0][red_mask > 0] = 178  
    # hsv_modified[..., 1][red_mask > 0] = 165

    hsv_modified[..., 0][green_mask > 0] = 3  
    hsv_modified[..., 1][green_mask > 0] = 140

    hsv_modified[..., 0][red_mask > 0] = 3  
    hsv_modified[..., 1][red_mask > 0] = 140

    # Brighten without overflow
    value = hsv_modified[..., 2].astype(np.int16)
    value[red_mask > 0] += 5
    value[green_mask > 0] += 80
    hsv_modified[..., 2] = np.clip(value, 0, 255).astype(np.uint8)

    frame_modified = cv2.cvtColor(
        hsv_modified,
        cv2.COLOR_HSV2BGR
    )

    video_writer.write(frame_modified)
    # if frame_idx == 6:
    #     break

cap.release()
video_writer.release()
cv2.destroyAllWindows()


```

## Source file: OpenFace3.0/detect_faces.py

{
  "path": "OpenFace3.0/detect_faces.py",
  "blob_sha": "3ece0a98bda7b7f3ef33dc563d5b633bf149e37f",
  "sha256": "26cd7b8b6f6b11b3e5e6f80e105795a682617afb920c6225c1b262cb3e529051",
  "lines": 24,
  "bytes": 727,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 13
  },
  "url": "https://github.com/QuinnMcGill/MocapExperiments/blob/09422f6a673192df1ae0e82a803d6dc06ce06d93/OpenFace3.0/detect_faces.py",
  "function_signature_candidates": []
}

```text

import cv2
from openface.face_detection import FaceDetector    #type:ignore

# Initialize the FaceDetector
model_path = 'weights/Alignment_RetinaFace.pth'
detector = FaceDetector(model_path=model_path, device='cuda')

# Path to the input image
image_path = '../data/6.jpg'
img_id = image_path.split('/')[-1].split('.')[0]

# Detect and extract the face
cropped_face, dets = detector.get_face(image_path)

if cropped_face is not None:
    print("Face detected!")
    print(f"Detection results: {dets}")
    
    # Save the cropped face as an image
    output_path = f'detected_faces/{img_id}.jpg'
    cv2.imwrite(output_path, cropped_face)
    print(f"Detected face saved to: {output_path}")
else:
    print("No face detected.")

```

## Source file: OpenFace3.0/openface_utils.py

{
  "path": "OpenFace3.0/openface_utils.py",
  "blob_sha": "1e766da46dd530b1d12ab9795ae5de752dee79ce",
  "sha256": "63d2de6cc7f047e723078213b05238d349f61aa75ff58fb38de0dc2ba4687a2d",
  "lines": 76,
  "bytes": 1921,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 1,
    "face": 11
  },
  "url": "https://github.com/QuinnMcGill/MocapExperiments/blob/09422f6a673192df1ae0e82a803d6dc06ce06d93/OpenFace3.0/openface_utils.py",
  "function_signature_candidates": [
    {
      "line": 4,
      "text": "def visualize_landmarks(image, landmarks, radius=6, color=(0, 255, 0)):"
    },
    {
      "line": 49,
      "text": "def visualize_landmarks_with_ids(image, landmarks):"
    }
  ]
}

```text

import cv2
import numpy as np

def visualize_landmarks(image, landmarks, radius=6, color=(0, 255, 0)):
    """
    Display facial landmarks overlaid on an image.

    Parameters
    ----------
    image : np.ndarray
        Original BGR image loaded by cv2.imread().
    landmarks : list
        Output from landmark_detector.detect_landmarks().
        Expected format:
            landmarks[face_idx] -> (N,2) array of (x,y) coordinates.
    radius : int
        Radius of landmark points.
    color : tuple
        BGR color for landmark points.

    Returns
    -------
    vis : np.ndarray
        Copy of image with overlayed landmark locations.
    """

    vis = image.copy()

    for face_landmarks in landmarks:

        # Convert tensor to numpy if needed
        if hasattr(face_landmarks, "cpu"):
            face_landmarks = face_landmarks.cpu().numpy()

        face_landmarks = np.asarray(face_landmarks)

        for x, y in face_landmarks:
            cv2.circle(
                vis,
                (int(round(x)), int(round(y))),
                radius,
                color,
                -1
            )

    return vis


def visualize_landmarks_with_ids(image, landmarks):
    vis = image.copy()

    for face_landmarks in landmarks:

        if hasattr(face_landmarks, "cpu"):
            face_landmarks = face_landmarks.cpu().numpy()

        face_landmarks = np.asarray(face_landmarks)

        for idx, (x, y) in enumerate(face_landmarks):
            x, y = int(round(x)), int(round(y))

            cv2.circle(vis, (x, y), 2, (0, 255, 0), -1)
            cv2.putText(
                vis,
                str(idx),
                (x + 3, y - 3),
                cv2.FONT_HERSHEY_SIMPLEX,
                0.3,
                (0, 0, 255),
                1,
                cv2.LINE_AA
            )

    cv2.imshow("Facial Landmarks", vis)
    cv2.waitKey(0)
    cv2.destroyAllWindows()

```

## Source file: OpenFace3.0/__init__.py

{
  "path": "OpenFace3.0/__init__.py",
  "blob_sha": "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
  "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "lines": 0,
  "bytes": 0,
  "selection_score": 3,
  "keyword_line_counts": {},
  "url": "https://github.com/QuinnMcGill/MocapExperiments/blob/09422f6a673192df1ae0e82a803d6dc06ce06d93/OpenFace3.0/__init__.py",
  "function_signature_candidates": []
}

```text



```
