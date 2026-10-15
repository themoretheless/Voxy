# Facial rendering source scan: ruduba/FaceMeshDeformation

Repository: https://github.com/ruduba/FaceMeshDeformation

Pinned source revision: f7c9a762cbc271e93edcc202300979c89ee2bedb

Discovery topic: face deformation

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"face": ["face_landmarks.npy"]}

## Source file: editor.py

{
  "path": "editor.py",
  "blob_sha": "ae84b594025f38ae043b6b30c754ca737c7d51c3",
  "sha256": "b07b280567e1b445034edfce3814c0a93f806c5d2192932fb117facfe40437e1",
  "lines": 59,
  "bytes": 1603,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 1
  },
  "url": "https://github.com/ruduba/FaceMeshDeformation/blob/f7c9a762cbc271e93edcc202300979c89ee2bedb/editor.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def make_spheres(landmarks):"
    },
    {
      "line": 15,
      "text": "def update_landmark(index, axis, value):"
    },
    {
      "line": 20,
      "text": "def refresh_viewer():"
    },
    {
      "line": 45,
      "text": "def loop():"
    },
    {
      "line": 50,
      "text": "def update_open3d():"
    }
  ]
}

```text

import open3d as o3d
import numpy as np
import tkinter as tk
from tkinter import Scale, HORIZONTAL

# Load landmark points
landmarks = np.load("landmarks.npy")  # shape: (N, 3)
spheres = []

# Visualize as small spheres in Open3D
def make_spheres(landmarks):
    return [o3d.geometry.TriangleMesh.create_sphere(radius=0.005).translate(p) for p in landmarks]

# Callback: update landmark + Open3D viewer
def update_landmark(index, axis, value):
    landmarks[index][axis] = float(value)
    spheres[index] = o3d.geometry.TriangleMesh.create_sphere(radius=0.005).translate(landmarks[index])
    refresh_viewer()

def refresh_viewer():
    vis.clear_geometries()
    for s in spheres:
        vis.add_geometry(s)

# Initialize Open3D window
vis = o3d.visualization.Visualizer()
vis.create_window(window_name="Face Landmark Editor")
spheres = make_spheres(landmarks)
for s in spheres:
    vis.add_geometry(s)

# Tkinter UI
root = tk.Tk()
root.title("Landmark Control Panel")

for idx in range(len(landmarks)):
    for axis, name in enumerate(['X', 'Y', 'Z']):
        scale = Scale(root, from_=-0.5, to=0.5, resolution=0.01,
                      orient=HORIZONTAL,
                      label=f'Pt{idx} {name}',
                      command=lambda val, i=idx, a=axis: update_landmark(i, a, val))
        scale.set(landmarks[idx][axis])
        scale.pack()

def loop():
    vis.poll_events()
    vis.update_renderer()
    root.after(100, loop)

def update_open3d():
    vis.poll_events()
    vis.update_renderer()
    root.after(50, update_open3d)



update_open3d()
root.after(100, loop)
root.mainloop()


```

## Source file: facial_mesh.py

{
  "path": "facial_mesh.py",
  "blob_sha": "b8f76550948984233141c078f8b00599f2fd887f",
  "sha256": "ff6a4d82a41aea64ef0b34e3baff457d2b01c306fc68b6d03e96479152153ee7",
  "lines": 49,
  "bytes": 1411,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 14,
    "lip": 1
  },
  "url": "https://github.com/ruduba/FaceMeshDeformation/blob/f7c9a762cbc271e93edcc202300979c89ee2bedb/facial_mesh.py",
  "function_signature_candidates": []
}

```text

import cv2
import mediapipe as mp
import numpy as np

cap = cv2.VideoCapture(0)

mp_face_mesh = mp.solutions.face_mesh
mp_drawing = mp.solutions.drawing_utils
mp_dstyles = mp.solutions.drawing_styles

my_drawing_sepc = mp_drawing.DrawingSpec(color = (194, 97, 190), thickness = 1)
with mp_face_mesh.FaceMesh( 
    max_num_faces = 1,
    refine_landmarks = True,
    min_detection_confidence = 0.5,
    min_tracking_confidence = 0.5
) as face_mesh:
    

    while cap.isOpened():
        success, image = cap.read()
        if not success:
            break
        results = face_mesh.process(image)

        for face_landmarks in results.multi_face_landmarks:
            mp_drawing.draw_landmarks(
                image = image,
                landmark_list = face_landmarks,
                connections = mp_face_mesh.FACEMESH_TESSELATION,
                landmark_drawing_spec = None,
                connection_drawing_spec = my_drawing_sepc
                #.get_default_face_mesh_contours_style()
            )


        for face_landmarks in results.multi_face_landmarks:
            print(face_landmarks)


        cv2.imshow("Face Mesh", cv2.flip(image, 1))

        landmarks = np.array([[lm.x, lm.y, lm.z] for lm in face_landmarks.landmark]) #mod
        np.save('face_landmarks.npy', landmarks)#mod

        if cv2.waitKey(100) == ord("q"):
            break
cap.release()
cv2.destroyAllWindows()

```

## Source file: facial_mesh_wsave.py

{
  "path": "facial_mesh_wsave.py",
  "blob_sha": "9d9361928b5ec1541bf5d9f121092c6b3d09f1da",
  "sha256": "40127494c52526538275a534355a49a7255409cf3d600c4be6bbc422d7c64127",
  "lines": 53,
  "bytes": 1572,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 11,
    "lip": 1
  },
  "url": "https://github.com/ruduba/FaceMeshDeformation/blob/f7c9a762cbc271e93edcc202300979c89ee2bedb/facial_mesh_wsave.py",
  "function_signature_candidates": []
}

```text

import os
import cv2
import mediapipe as mp
import numpy as np

cap = cv2.VideoCapture(0)

mp_face_mesh = mp.solutions.face_mesh
mp_drawing = mp.solutions.drawing_utils
mp_dstyles = mp.solutions.drawing_styles

my_drawing_spec = mp_drawing.DrawingSpec(color=(194, 97, 190), thickness=1)

with mp_face_mesh.FaceMesh( 
    max_num_faces=1,
    refine_landmarks=True,
    min_detection_confidence=0.5,
    min_tracking_confidence=0.5
) as face_mesh:

    while cap.isOpened():
        success, image = cap.read()
        if not success:
            break

        results = face_mesh.process(image)

        if results.multi_face_landmarks:
            for face_landmarks in results.multi_face_landmarks:
                mp_drawing.draw_landmarks(
                    image=image,
                    landmark_list=face_landmarks,
                    connections=mp_face_mesh.FACEMESH_TESSELATION,
                    landmark_drawing_spec=None,
                    connection_drawing_spec=my_drawing_spec
                )

                # Save landmarks safely
                landmarks = np.array([[lm.x, lm.y, lm.z] for lm in face_landmarks.landmark])
                tmp_file = 'landmarks_tmp.npy'
                final_file = 'landmarks.npy'
                np.save(tmp_file, landmarks)
                os.replace(tmp_file, final_file)  # atomic rename

                print(f"[Saved] {len(landmarks)} landmarks")

        cv2.imshow("Face Mesh", cv2.flip(image, 1))

        if cv2.waitKey(100) == ord("q"):
            break

cap.release()
cv2.destroyAllWindows()


```

## Source file: demo.py

{
  "path": "demo.py",
  "blob_sha": "f0ba50a39ec29a282f937059ee09b3247b763d07",
  "sha256": "e72185583b6f7c73c4fddde906ed04576dae3b00cd5cf64df1eb1f8f9dc16b89",
  "lines": 4,
  "bytes": 185,
  "selection_score": -2,
  "keyword_line_counts": {},
  "url": "https://github.com/ruduba/FaceMeshDeformation/blob/f7c9a762cbc271e93edcc202300979c89ee2bedb/demo.py",
  "function_signature_candidates": []
}

```text

import open3d as o3d; \
           mesh = o3d.geometry.TriangleMesh.create_sphere(); \
           mesh.compute_vertex_normals(); \
           o3d.visualization.draw(mesh, raw_mode=True)

```
