# Facial rendering source scan: polyworks-art/FaceBridge

Repository: https://github.com/polyworks-art/FaceBridge

Pinned source revision: 5ad75bd220cb31b107c8a4dee736996e8c44975a

Discovery topic: ARKit blendshape

Description: Convert Video to ARKit Blendshapes

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"eye": ["facebridge/utils/__pycache__/eye_coupling.cpython-311.pyc", "facebridge/utils/eye_coupling.py"], "face": ["FaceBridge.spec", "facebridge/__pycache__/app.cpython-311.pyc", "facebridge/app.py", "facebridge/models/README.txt", "facebridge/models/face_landmarker.task", "facebridge/profiles/default_profile.json", "facebridge/requirements.txt", "facebridge/streaming/__init__.py", "facebridge/streaming/__pycache__/__init__.cpython-311.pyc", "facebridge/streaming/__pycache__/ifacialmocap_sender.cpython-311.pyc", "facebridge/streaming/ifacialmocap_sender.py", "facebridge/takes/take_20260619_134808.jsonl", "facebridge/takes/take_20260619_151459.jsonl", "facebridge/tools/__pycache__/blender_shape_key_audit.cpython-311.pyc", "facebridge/tools/__pycache__/merge_shape_limits.cpython-311.pyc", "facebridge/tools/blender_shape_key_audit.py", "facebridge/tools/merge_shape_limits.py", "facebridge/tools/test_sweep.py", "facebridge/tools/udp_listen.py", "facebridge/tracker/__init__.py", "facebridge/tracker/__pycache__/__init__.cpython-311.pyc", "facebridge/tracker/__pycache__/mediapipe_tracker.cpython-311.pyc", "facebridge/tracker/mediapipe_tracker.py", "facebridge/ui/__init__.py", "facebridge/ui/__pycache__/__init__.cpython-311.pyc", "facebridge/ui/__pycache__/calibration_wizard.cpython-311.pyc", "facebridge/ui/__pycache__/face_wireframe.cpython-311.pyc", "facebridge/ui/__pycache__/main_window.cpython-311.pyc", "facebridge/ui/__pycache__/take_editor.cpython-311.pyc", "facebridge/ui/calibration_wizard.py"]}

## Source file: facebridge/utils/eye_coupling.py

{
  "path": "facebridge/utils/eye_coupling.py",
  "blob_sha": "29f4b2ea8f12dd0dbda0d69edda1d276ceb1a40a",
  "sha256": "8b059f1435a4b9abdb15533bcb83504a48faa1c6f1e42d718bd19294498114b4",
  "lines": 47,
  "bytes": 1444,
  "selection_score": 18,
  "keyword_line_counts": {
    "eye": 17,
    "rig": 13,
    "face": 1
  },
  "url": "https://github.com/polyworks-art/FaceBridge/blob/5ad75bd220cb31b107c8a4dee736996e8c44975a/facebridge/utils/eye_coupling.py",
  "function_signature_candidates": [
    {
      "line": 20,
      "text": "def average_eye_pairs(values: dict[str, float]) -> dict[str, float]:"
    }
  ]
}

```text

"""Helpers for coupling left/right eye output channels."""

from __future__ import annotations


EYE_PAIR_BASES = (
    "eyeBlink",
    "eyeLookDown",
    "eyeLookUp",
    "eyeSquint",
    "eyeWide",
)

HORIZONTAL_GAZE_PAIRS = (
    ("eyeLookInLeft", "eyeLookOutRight"),
    ("eyeLookOutLeft", "eyeLookInRight"),
)


def average_eye_pairs(values: dict[str, float]) -> dict[str, float]:
    """Return values with matching eye channels set to their mean.

    Tracking and calibration can still use both sides independently. This only
    couples the final output channels so Faceit receives matching movement for
    both eyes.

    Horizontal gaze is coupled as conjugate movement: left-eye inward movement
    matches right-eye outward movement, and vice versa. Averaging
    ``eyeLookInLeft`` with ``eyeLookInRight`` would drive both eyes inward at
    once and make the character look cross-eyed.
    """
    out = dict(values)
    for base in EYE_PAIR_BASES:
        left = f"{base}Left"
        right = f"{base}Right"
        if left not in out or right not in out:
            continue
        mean = (float(out[left]) + float(out[right])) * 0.5
        out[left] = mean
        out[right] = mean
    for left, right in HORIZONTAL_GAZE_PAIRS:
        if left not in out or right not in out:
            continue
        mean = (float(out[left]) + float(out[right])) * 0.5
        out[left] = mean
        out[right] = mean
    return out


```

## Source file: facebridge/ui/face_wireframe.py

{
  "path": "facebridge/ui/face_wireframe.py",
  "blob_sha": "7ec77931d25664e8ffba3d7461e3a4ef44fda906",
  "sha256": "68af72a9c5667414e4d4e428972e569cae3f00d14b6746e448c7cdc825fd3eba",
  "lines": 270,
  "bytes": 10410,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 3,
    "skin": 1,
    "blendshape": 3,
    "jaw": 1,
    "mouth": 1,
    "deform": 1,
    "iris": 11,
    "rig": 11,
    "face": 31,
    "lip": 3
  },
  "url": "https://github.com/polyworks-art/FaceBridge/blob/5ad75bd220cb31b107c8a4dee736996e8c44975a/facebridge/ui/face_wireframe.py",
  "function_signature_candidates": [
    {
      "line": 47,
      "text": "def draw_mesh_on_frame(frame_bgr, landmarks: list[tuple[float, float]]) -> None:"
    },
    {
      "line": 73,
      "text": "def mask_around_face(frame_bgr, landmarks: list[tuple[float, float]]):"
    },
    {
      "line": 103,
      "text": "def __init__(self, parent: QWidget | None = None):"
    },
    {
      "line": 116,
      "text": "def set_data("
    },
    {
      "line": 136,
      "text": "def paintEvent(self, event) -> None:  # noqa: N802 (Qt naming)"
    },
    {
      "line": 175,
      "text": "def _project_points(self) -> list[QPointF]:"
    },
    {
      "line": 203,
      "text": "def _make_front_view("
    },
    {
      "line": 236,
      "text": "def _symmetry_error(points: np.ndarray) -> float:"
    },
    {
      "line": 259,
      "text": "def _draw_connections(self, painter: QPainter, pts: list[QPointF], conns) -> None:"
    },
    {
      "line": 265,
      "text": "def _draw_active_shapes(self, painter: QPainter) -> None:"
    }
  ]
}

```text

"""Live face-mesh wireframe widget.

Replaces the old text list of blendshape values with a "rig preview": the
MediaPipe 468/478-point face mesh drawn as a wireframe that deforms in real time
with the user's face (similar in spirit to a MetaHuman face-rig preview).

The wireframe is just the 2D projection of the normalised face landmarks. The
mesh topology (which points connect to which) comes from MediaPipe's
`solutions.face_mesh` connection sets, so we don't have to hard-code it.
"""

from __future__ import annotations

import cv2
import numpy as np
from PySide6.QtCore import Qt, QPointF
from PySide6.QtGui import QColor, QPainter, QPen
from PySide6.QtWidgets import QWidget

# Connection topology from MediaPipe. These are frozensets of (i, j) index
# pairs into the landmark list. We import them once and convert to plain
# tuples (iterating a frozenset every frame is wasteful).
#
# The legacy `solutions` module may be absent in some MediaPipe builds, so we
# fall back gracefully: without the topology the widget just draws points.
try:
    from mediapipe.python.solutions.face_mesh_connections import (
        FACEMESH_TESSELATION,
        FACEMESH_CONTOURS,
        FACEMESH_IRISES,
    )
    _TESSELATION = tuple(FACEMESH_TESSELATION)
    _CONTOURS = tuple(FACEMESH_CONTOURS)
    _IRISES = tuple(FACEMESH_IRISES)
except Exception:  # noqa: BLE001 - fall back to dots-only rendering
    _TESSELATION = ()
    _CONTOURS = ()
    _IRISES = ()


# Colors are BGR (OpenCV order).
_MESH_BGR = (130, 150, 70)      # dim teal tessellation
_CONTOUR_BGR = (190, 230, 120)  # brighter feature contours
_IRIS_BGR = (255, 200, 120)     # blue-ish irises


def draw_mesh_on_frame(frame_bgr, landmarks: list[tuple[float, float]]) -> None:
    """Draw the face mesh directly onto a BGR OpenCV frame (in place).

    `landmarks` are normalised (x, y) in 0..1, matching the frame that was
    tracked. We map them to pixel coords and draw the same topology the side
    panel uses, so the overlay lines up 1:1 with the video.
    """
    if len(landmarks) < 468:
        return
    h, w = frame_bgr.shape[:2]
    pts = [(int(x * w), int(y * h)) for x, y in landmarks]
    n = len(pts)

    # Tessellation without anti-aliasing (it's ~2500 edges; keep it cheap).
    for a, b in _TESSELATION:
        if a < n and b < n:
            cv2.line(frame_bgr, pts[a], pts[b], _MESH_BGR, 1)
    # Contours + irises anti-aliased so features read clearly.
    for a, b in _CONTOURS:
        if a < n and b < n:
            cv2.line(frame_bgr, pts[a], pts[b], _CONTOUR_BGR, 1, cv2.LINE_AA)
    for a, b in _IRISES:
        if a < n and b < n:
            cv2.line(frame_bgr, pts[a], pts[b], _IRIS_BGR, 1, cv2.LINE_AA)


def mask_around_face(frame_bgr, landmarks: list[tuple[float, float]]):
    """Black out everything outside the detected face (returns a new frame).

    Builds a convex hull from the face-mesh landmarks, expands it slightly so the
    silhouette isn't clipped, and zeroes every pixel outside it. With no face
    detected the whole frame goes black (nothing to keep visible).
    """
    h, w = frame_bgr.shape[:2]
    if len(landmarks) < 3:
        return np.zeros_like(frame_bgr)

    pts = np.array([(int(x * w), int(y * h)) for x, y in landmarks], dtype=np.int32)
    hull = cv2.convexHull(pts)
    mask = np.zeros((h, w), dtype=np.uint8)
    cv2.fillConvexPoly(mask, hull, 255)

    # Grow the mask by a small margin relative to face size so we don't shave the
    # edge of the face, and soften the border so it doesn't look hard-cut.
    face_w = int(pts[:, 0].max() - pts[:, 0].min())
    margin = max(3, int(face_w * 0.05))
    kernel = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (margin, margin))
    mask = cv2.dilate(mask, kernel)
    mask = cv2.GaussianBlur(mask, (0, 0), margin * 0.5)

    return (frame_bgr * (mask[:, :, None] / 255.0)).astype(frame_bgr.dtype)


class FaceWireframeWidget(QWidget):
    """Draws the live face mesh and a small list of the most active shapes."""

    def __init__(self, parent: QWidget | None = None):
        super().__init__(parent)
        self.setMinimumSize(280, 320)
        self._landmarks: list[tuple[float, float]] = []
        self._front_landmarks: list[tuple[float, float]] = []
        self._face_detected = False
        self._active_shapes: list[tuple[str, float]] = []

        # Pens are reused across frames.
        self._pen_mesh = QPen(QColor(70, 150, 130), 1)        # dim teal mesh
        self._pen_contour = QPen(QColor(120, 230, 190), 1.6)  # brighter outline
        self._pen_iris = QPen(QColor(120, 200, 255), 1.6)     # blue irises

    def set_data(
        self,
        landmarks: list[tuple[float, float]],
        blendshapes: dict[str, float],
        face_detected: bool,
        landmarks_3d: list[tuple[float, float, float]] | None = None,
        head_rotation_matrix=None,
    ) -> None:
        """Feed one frame of data and trigger a repaint."""
        self._landmarks = landmarks
        self._front_landmarks = self._make_front_view(
            landmarks, landmarks_3d, head_rotation_matrix
        )
        self._face_detected = face_detected
        # Top active shapes (value already in 0..100) for the side readout.
        active = sorted(blendshapes.items(), key=lambda kv: kv[1], reverse=True)
        self._active_shapes = [(n, v) for n, v in active[:8] if v >= 1.0]
        self.update()

    # ------------------------------------------------------------------ paint
    def paintEvent(self, event) -> None:  # noqa: N802 (Qt naming)
        painter = QPainter(self)
        painter.fillRect(self.rect(), QColor(24, 26, 30))

        if not self._face_detected or len(self._landmarks) < 468:
            painter.setPen(QColor(150, 150, 150))
            painter.drawText(self.rect(), Qt.AlignCenter, "No face")
            painter.end()
            return

        painter.setRenderHint(QPainter.Antialiasing, True)

        # Fit the normalised landmarks into the widget while keeping aspect
        # ratio and leaving a small margin.
        pts = self._project_points()

        if _TESSELATION:
            # 1) Dense tessellation (the "skin" wireframe).
            painter.setPen(self._pen_mesh)
            self._draw_connections(painter, pts, _TESSELATION)

            # 2) Feature contours (eyes, lips, brows, face oval), brighter.
            painter.setPen(self._pen_contour)
            self._draw_connections(painter, pts, _CONTOURS)

            # 3) Irises.
            painter.setPen(self._pen_iris)
            self._draw_connections(painter, pts, _IRISES)
        else:
            # Fallback: no topology available, just plot the landmark cloud.
            painter.setPen(self._pen_contour)
            for p in pts:
                painter.drawPoint(p)

        # 4) Active-shape readout in the corner.
        self._draw_active_shapes(painter)

        painter.end()

    def _project_points(self) -> list[QPointF]:
        margin = 12
        w = self.width() - 2 * margin
        h = self.height() - 2 * margin

        display_landmarks = self._front_landmarks or self._landmarks

        xs = [p[0] for p in display_landmarks]
        ys = [p[1] for p in display_landmarks]
        min_x, max_x = min(xs), max(xs)
        min_y, max_y = min(ys), max(ys)
        span_x = max(max_x - min_x, 1e-6)
        span_y = max(max_y - min_y, 1e-6)

        # Uniform scale so the face isn't stretched; centre it.
        scale = min(w / span_x, h / span_y)
        draw_w = span_x * scale
        draw_h = span_y * scale
        off_x = margin + (w - draw_w) / 2
        off_y = margin + (h - draw_h) / 2

        pts = []
        for x, y in display_landmarks:
            px = off_x + (x - min_x) * scale
            py = off_y + (y - min_y) * scale
            pts.append(QPointF(px, py))
        return pts

    def _make_front_view(
        self,
        landmarks: list[tuple[float, float]],
        landmarks_3d: list[tuple[float, float, float]] | None,
        head_rotation_matrix,
    ) -> list[tuple[float, float]]:
        """Return landmarks projected as a fixed front-facing rig preview."""
        if (
            not landmarks_3d
            or len(landmarks_3d) != len(landmarks)
            or head_rotation_matrix is None
        ):
            return landmarks

        try:
            points = np.asarray(landmarks_3d, dtype=float)
            rotation = np.asarray(head_rotation_matrix, dtype=float)
            if points.ndim != 2 or points.shape[1] != 3 or rotation.shape != (3, 3):
                return landmarks

            centered = points - points.mean(axis=0)
            candidates = (
                centered @ rotation,
                centered @ rotation.T,
            )
            front_points = min(candidates, key=self._symmetry_error)

            # Keep MediaPipe's screen-space orientation: +x right, +y down.
            return [(float(p[0]), float(p[1])) for p in front_points]
        except Exception:  # noqa: BLE001 - preview should never break tracking
            return landmarks

    @staticmethod
    def _symmetry_error(points: np.ndarray) -> float:
        """Score how close projected face landmarks are to a front view."""
        pairs = (
            (33, 263),    # outer eyes
            (133, 362),   # inner eyes
            (61, 291),    # mouth corners
            (234, 454),   # cheeks
            (127, 356),   # temples
            (172, 397),   # lower jaw
        )
        center_x = float(np.median(points[:, 0]))
        error = 0.0
        used = 0
        for left, right in pairs:
            if left >= len(points) or right >= len(points):
                continue
            lx = abs(float(points[left, 0]) - center_x)
            rx = abs(float(points[right, 0]) - center_x)
            denom = max(lx + rx, 1e-6)
            error += abs(lx - rx) / denom
            used += 1
        return error / max(used, 1)

    def _draw_connections(self, painter: QPainter, pts: list[QPointF], conns) -> None:
        n = len(pts)
        for a, b in conns:
            if a < n and b < n:
                painter.drawLine(pts[a], pts[b])

    def _draw_active_shapes(self, painter: QPainter) -> None:
        painter.setPen(QColor(180, 220, 200))
        y = 16
        for name, value in self._active_shapes:
            painter.drawText(8, y, f"{name}: {int(round(value))}")
            y += 15


```

## Source file: facebridge/app.py

{
  "path": "facebridge/app.py",
  "blob_sha": "cb3adb687a69b1d9f243a1ab7bdff901e9f7f1fa",
  "sha256": "3c892b3f0449b40c7fc87ea8c0faf82ea74bd050bf89664528185bd7c49412d3",
  "lines": 32,
  "bytes": 891,
  "selection_score": 3,
  "keyword_line_counts": {
    "blendshape": 1,
    "face": 4
  },
  "url": "https://github.com/polyworks-art/FaceBridge/blob/5ad75bd220cb31b107c8a4dee736996e8c44975a/facebridge/app.py",
  "function_signature_candidates": [
    {
      "line": 24,
      "text": "def main() -> int:"
    }
  ]
}

```text

"""FaceBridge entry point.

Run with:  python app.py   (from inside the facebridge/ folder)

FaceBridge captures your webcam, estimates ARKit-style facial blendshapes with
MediaPipe, and streams them over UDP in an iFacialMocap-compatible format so
Blender's Faceit add-on can record your facial expressions.
"""

from __future__ import annotations

import os
import sys

from PySide6.QtWidgets import QApplication

# Make sibling packages (tracker/, streaming/, ui/, utils/) importable when
# running `python app.py` directly. This also keeps things PyInstaller-friendly.
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from ui.main_window import MainWindow  # noqa: E402  (after sys.path tweak)


def main() -> int:
    app = QApplication(sys.argv)
    window = MainWindow()
    window.show()
    return app.exec()


if __name__ == "__main__":
    raise SystemExit(main())


```

## Source file: facebridge/ui/__init__.py

{
  "path": "facebridge/ui/__init__.py",
  "blob_sha": "89eab422f8a23c45b7b6fe91bdac30dee68f3e6b",
  "sha256": "7fb8ef4bd4c2261297095d2519b22695e7bb9132ce4d650c011d6e63b77d7a43",
  "lines": 1,
  "bytes": 33,
  "selection_score": 3,
  "keyword_line_counts": {
    "face": 1
  },
  "url": "https://github.com/polyworks-art/FaceBridge/blob/5ad75bd220cb31b107c8a4dee736996e8c44975a/facebridge/ui/__init__.py",
  "function_signature_candidates": []
}

```text

"""PySide6 UI for FaceBridge."""


```
