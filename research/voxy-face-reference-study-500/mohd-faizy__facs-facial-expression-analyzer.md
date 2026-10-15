# Facial rendering source scan: mohd-faizy/facs-facial-expression-analyzer

Repository: https://github.com/mohd-faizy/facs-facial-expression-analyzer

Pinned source revision: 41218d78cfc67b0ab69bb5058e3b62ec5ad0fc8a

Discovery topic: FACS face

Description: Real-time Facial Expression Classifier, FACS Muscle Action Unit Engine & Micro-Expression Detector using MediaPipe 3D Face Mesh and OpenCV.

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {}

## Source file: main.py

{
  "path": "main.py",
  "blob_sha": "228ebdf45791376b1c4935ba9bf52db89468dda5",
  "sha256": "2be74b246f575f7ffa0d9cdeb2c6dbafbbc697665035038327d85f6b271500f5",
  "lines": 379,
  "bytes": 15709,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 4,
    "facs": 12,
    "rig": 1,
    "face": 19,
    "lip": 1
  },
  "url": "https://github.com/mohd-faizy/facs-facial-expression-analyzer/blob/41218d78cfc67b0ab69bb5058e3b62ec5ad0fc8a/main.py",
  "function_signature_candidates": [
    {
      "line": 67,
      "text": "def resolve_video_path(raw_path: str) -> str:"
    },
    {
      "line": 92,
      "text": "def save_session_log(stats: dict, analyzer: ExpressionAnalyzer):"
    },
    {
      "line": 114,
      "text": "def main():"
    }
  ]
}

```text

"""
Real-Time Facial Expression, FACS Muscle Action Units & Micro-Expression Analyzer.
===================================================================================
Renders dual-quadrant multi-cam display with pristine raw footage, cyber green
wiremesh with glowing green connection dots, dynamic facial muscle tension lines,
10 Action Units (FACS), involuntary micro-expressions, and adaptive personal calibration.

Layout:
  [ Quadrant 1: Raw Footage ] [ Quadrant 2: 3D Green Wiremesh & FACS ] [ Quadrant 3: Detection HUD Dashboard ]

Usage:
  uv run python main.py --webcam                        # Live webcam input (default)
  uv run python main.py --video assets/videos/sample.mp4 # Specific video file
  uv run python main.py --video asset/videos/sample.mp4  # Handles 'asset/' automatically

Controls:
  [V] Toggle Dual-View (Raw + Mesh) vs Single View
  [C] Re-calibrate neutral resting face (Hold relaxed face for 1 sec)
  [M] Toggle cyber green wiremesh with green dots
  [F] Toggle muscle tension vectors & facial highlights
  [D] Toggle HUD sidebar dashboard
  [SPACE] Pause / Resume video playback (video mode)
  [L] Toggle video looping (video mode)
  [S] Save session CSV report to data/
  [R] Reset session counters / restart video
  [Q] or [ESC] Quit application
"""

import argparse
import csv
import ctypes
from datetime import datetime
import os
import sys
import time
from typing import Optional

import cv2
import mediapipe as mp
import numpy as np

# Enable High-DPI awareness on Windows so OpenCV windows render crystal-clear
if sys.platform.startswith("win"):
    try:
        ctypes.windll.shcore.SetProcessDpiAwareness(2)
    except Exception:
        try:
            ctypes.windll.user32.SetProcessDPIAware()
        except Exception:
            pass

from src.config import (
    CAMERA_INDEX,
    COLOR_BADGE_MESH,
    COLOR_BADGE_RAW,
    COLOR_BG_DARK,
    COLOR_PANEL_BORDER,
    DASHBOARD_WIDTH,
    DEFAULT_FRAME_HEIGHT,
    DEFAULT_FRAME_WIDTH,
    QUADRANT_WIDTH,
)
from src.dashboard import ExpressionDashboard
from src.expression_analyzer import ExpressionAnalyzer


def resolve_video_path(raw_path: str) -> str:
    """Resolve video path handling 'asset/' vs 'assets/' and relative paths."""
    if os.path.exists(raw_path):
        return os.path.abspath(raw_path)

    # Check if 'asset/' was used instead of 'assets/'
    norm = raw_path.replace("\\", "/")
    if norm.startswith("asset/"):
        alt = "assets/" + norm[6:]
        if os.path.exists(alt):
            return os.path.abspath(alt)

    # Check relative to project root
    proj_dir = os.path.dirname(os.path.abspath(__file__))
    cand1 = os.path.join(proj_dir, raw_path)
    if os.path.exists(cand1):
        return cand1

    cand2 = os.path.join(proj_dir, "assets", "videos", os.path.basename(raw_path))
    if os.path.exists(cand2):
        return cand2

    return os.path.abspath(raw_path)


def save_session_log(stats: dict, analyzer: ExpressionAnalyzer):
    """Save session summary to data/ folder."""
    data_dir = os.path.join(os.path.dirname(__file__), "data")
    os.makedirs(data_dir, exist_ok=True)
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    filepath = os.path.join(data_dir, f"expression_session_{timestamp}.csv")

    with open(filepath, "w", newline="", encoding="utf-8") as f:
        writer = csv.writer(f)
        writer.writerow(["Metric", "Value"])
        writer.writerow(["Session Duration (s)", stats.get("duration_s", 0)])
        writer.writerow(["Total Frames Analyzed", stats.get("total_frames", 0)])
        writer.writerow(["Micro-Expression Events Detected", stats.get("micro_events_total", 0)])
        writer.writerow([])
        writer.writerow(["Emotion", "Percentage (%)"])
        for emo, pct in stats.get("distribution", {}).items():
            writer.writerow([emo, pct])

    print(f"\n[INFO] Session report successfully saved to: {filepath}")
    return filepath


def main():
    parser = argparse.ArgumentParser(
        description="Real-Time Facial Expression, FACS Muscle Action Units & Micro-Expression Analyzer"
    )
    group = parser.add_mutually_exclusive_group()
    group.add_argument(
        "--webcam", "-w",
        nargs="?",
        const=0,
        type=int,
        help="Use live webcam feed (optional camera index, default: 0)",
    )
    group.add_argument(
        "--video", "-v",
        type=str,
        help="Path to video file to analyze (e.g. assets/videos/sample.mp4)",
    )
    parser.add_argument(
        "--no-loop",
        action="store_true",
        help="Disable continuous looping of video playback",
    )

    args = parser.parse_args()

    # Determine input mode
    is_video = bool(args.video)
    loop_video = not args.no_loop

    print("=" * 68)
    print("  FACIAL EXPRESSION & FACS MICRO-ACTION UNIT ANALYZER")
    print("=" * 68)

    if is_video:
        video_path = resolve_video_path(args.video)
        if not os.path.exists(video_path):
            print(f"[ERROR] Video file not found: {args.video}")
            print(f"        Tried resolved path: {video_path}")
            sys.exit(1)

        print(f"  Input Mode: VIDEO FILE -> {os.path.basename(video_path)}")
        cap = cv2.VideoCapture(video_path)
        video_fps = cap.get(cv2.CAP_PROP_FPS) or 25.0
        total_video_frames = int(cap.get(cv2.CAP_PROP_FRAME_COUNT))
        frame_delay_ms = max(1, int(1000.0 / video_fps))
        source_label = f"VIDEO: {os.path.basename(video_path)}"
    else:
        cam_id = args.webcam if args.webcam is not None else CAMERA_INDEX
        print(f"  Input Mode: LIVE WEBCAM (Index: {cam_id})")
        cap = cv2.VideoCapture(cam_id)
        cap.set(cv2.CAP_PROP_FRAME_WIDTH, DEFAULT_FRAME_WIDTH)
        cap.set(cv2.CAP_PROP_FRAME_HEIGHT, DEFAULT_FRAME_HEIGHT)
        frame_delay_ms = 1
        source_label = "WEBCAM"
        total_video_frames = 0

    if not cap.isOpened():
        print(f"[ERROR] Cannot open media source. Please verify your camera or video path.")
        sys.exit(1)

    print("  Initializing MediaPipe Face Mesh & Expression Engine...")
    mp_face_mesh = mp.solutions.face_mesh
    face_mesh = mp_face_mesh.FaceMesh(
        max_num_faces=1,
        refine_landmarks=True,
        min_detection_confidence=0.5,
        min_tracking_confidence=0.5,
    )

    # Auto-calibrate live webcam on startup; for pre-recorded videos use population baseline (press [C] anytime to calibrate)
    analyzer = ExpressionAnalyzer(auto_calibrate=not is_video)
    dashboard = ExpressionDashboard(width=DASHBOARD_WIDTH)

    # Display flags
    show_dual_view = True   # True: Dual-Quadrant (Raw + Mesh) + Dashboard, False: Single-View (Mesh) + Dashboard
    show_mesh = True
    show_facs_vectors = True
    show_dashboard = True
    is_paused = False
    smoothed_cx: Optional[float] = None

    print("\n[READY] Running pipeline:")
    print("       Layout: [Quadrant 1: Raw Footage] | [Quadrant 2: 3D Green Wiremesh] | [Quadrant 3: Detection HUD]")
    print("       Press [V] to toggle Dual-View / Single-View.")
    print("       Press [M] for Green Mesh & Dots, [F] for FACS vectors, [D] for Dashboard.")
    print("       Press [C] to recalibrate neutral face.")
    if is_video:
        print("       Press [SPACE] to pause/resume, [R] to restart video, [Q] to quit.\n")
    else:
        print("       Press [R] to reset stats, [S] to save CSV report, [Q] to quit.\n")

    window_name = f"Facial Expression Analyzer - [{source_label}]"
    cv2.namedWindow(window_name, cv2.WINDOW_NORMAL)
    window_initialized = False

    try:
        while cap.isOpened():
            if not is_paused:
                ret, frame = cap.read()
                if not ret or frame is None:
                    if is_video and loop_video:
                        cap.set(cv2.CAP_PROP_POS_FRAMES, 0)
                        ret, frame = cap.read()
                        if not ret:
                            break
                    else:
                        break

                # Only mirror webcam feeds, keep videos in native perspective
                if not is_video:
                    frame = cv2.flip(frame, 1)

                # Standardize canvas height to 720p for clean alignment
                target_h = 720
                cur_h, cur_w = frame.shape[:2]
                if cur_h != target_h:
                    aspect = cur_w / cur_h
                    new_w = int(target_h * aspect)
                    frame = cv2.resize(frame, (new_w, target_h), interpolation=cv2.INTER_LINEAR)

                h, w = frame.shape[:2]

                # Keep a clean, pristine copy of the unannotated raw input footage
                raw_frame = frame.copy()

                # Convert BGR to RGB for MediaPipe
                rgb_frame = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
                rgb_frame.flags.writeable = False
                results = face_mesh.process(rgb_frame)
                rgb_frame.flags.writeable = True

                # Analyze landmarks if face is detected
                analysis_data = None
                raw_landmarks = None
                face_cx = w // 2

                if results.multi_face_landmarks:
                    raw_landmarks = results.multi_face_landmarks[0].landmark
                    analysis_data = analyzer.analyze(raw_landmarks, w, h)
                    if analysis_data and "pts_2d" in analysis_data and 1 in analysis_data["pts_2d"]:
                        face_cx = int(analysis_data["pts_2d"][1][0])
                    else:
                        face_cx = int(np.mean([lm.x * w for lm in raw_landmarks]))

                # Smooth tracking center for steady, cinematic quadrant framing
                if smoothed_cx is None:
                    smoothed_cx = float(face_cx)
                else:
                    smoothed_cx = 0.85 * smoothed_cx + 0.15 * float(face_cx)

                dashboard.update_fps()

                # Render cyber green wiremesh with green dots, FACS vectors, and floating pills onto mesh_frame
                mesh_frame = frame.copy()
                dashboard.draw_on_face(
                    frame=mesh_frame,
                    results=analysis_data,
                    show_mesh=show_mesh,
                    show_facs_vectors=show_facs_vectors,
                    mesh_landmarks=raw_landmarks,
                )

                # Extract standardized video quadrants (centered around detected face)
                target_qw = QUADRANT_WIDTH
                crop_raw = dashboard.extract_quadrant(raw_frame, int(smoothed_cx), target_w=target_qw, target_h=target_h)
                crop_mesh = dashboard.extract_quadrant(mesh_frame, int(smoothed_cx), target_w=target_qw, target_h=target_h)

                # Render top-left badges for each quadrant
                dashboard.draw_quadrant_header(crop_raw, "RAW FOOTAGE", COLOR_BADGE_RAW)
                dashboard.draw_quadrant_header(crop_mesh, "3D WIREMESH & FACS", COLOR_BADGE_MESH)

                # Crisp 1px dividing border lines
                cv2.line(crop_raw, (crop_raw.shape[1] - 1, 0), (crop_raw.shape[1] - 1, target_h), COLOR_PANEL_BORDER, 1)
                cv2.line(crop_mesh, (crop_mesh.shape[1] - 1, 0), (crop_mesh.shape[1] - 1, target_h), COLOR_PANEL_BORDER, 1)

                # Choose active feeds based on view mode
                if show_dual_view:
                    active_feeds = [crop_raw, crop_mesh]
                else:
                    active_feeds = [crop_mesh]

                # Composite with HUD Dashboard Sidebar if enabled
                if show_dashboard:
                    if is_video and total_video_frames > 0:
                        cur_frame_num = int(cap.get(cv2.CAP_PROP_POS_FRAMES))
                        cur_src_info = f"VIDEO ({cur_frame_num}/{total_video_frames})"
                    else:
                        cur_src_info = source_label

                    panel = dashboard.render_panel(analysis_data, target_h, source_info=cur_src_info)
                    display_img = np.hstack(active_feeds + [panel])
                else:
                    display_img = np.hstack(active_feeds) if len(active_feeds) > 1 else active_feeds[0]

                cv2.imshow(window_name, display_img)

                if not window_initialized:
                    init_w = display_img.shape[1]
                    init_h = display_img.shape[0]
                    cv2.resizeWindow(window_name, init_w, init_h)
                    cv2.moveWindow(window_name, 40, 30)
                    window_initialized = True

            # Keyboard handler
            key = cv2.waitKey(frame_delay_ms if not is_paused else 30) & 0xFF

            if key in [ord("q"), ord("Q"), 27]:  # Q or ESC
                break
            elif key == ord(" "):  # Space to pause/resume
                is_paused = not is_paused
                print(f"[PAUSE] Video {'PAUSED' if is_paused else 'RESUMED'}")
            elif key in [ord("v"), ord("V")]:
                show_dual_view = not show_dual_view
                print(f"[TOGGLE] View Mode: {'DUAL QUADRANT (Raw + Mesh)' if show_dual_view else 'SINGLE QUADRANT (Mesh Only)'}")
                new_w = (QUADRANT_WIDTH * (2 if show_dual_view else 1)) + (dashboard.width if show_dashboard else 0)
                cv2.resizeWindow(window_name, new_w, target_h)
            elif key in [ord("c"), ord("C")]:
                analyzer.trigger_calibration()
                print("[INFO] Re-calibrating neutral baseline... Hold relaxed face for 1 second!")
            elif key in [ord("m"), ord("M")]:
                show_mesh = not show_mesh
                print(f"[TOGGLE] Cyber Green Mesh overlay: {'ON' if show_mesh else 'OFF'}")
            elif key in [ord("f"), ord("F")]:
                show_facs_vectors = not show_facs_vectors
                print(f"[TOGGLE] FACS Muscle vectors: {'ON' if show_facs_vectors else 'OFF'}")
            elif key in [ord("d"), ord("D")]:
                show_dashboard = not show_dashboard
                new_w = (QUADRANT_WIDTH * (2 if show_dual_view else 1)) + (dashboard.width if show_dashboard else 0)
                cv2.resizeWindow(window_name, new_w, target_h)
                print(f"[TOGGLE] Dashboard HUD: {'ON' if show_dashboard else 'OFF'}")
            elif key in [ord("l"), ord("L")]:
                if is_video:
                    loop_video = not loop_video
                    print(f"[TOGGLE] Video loop: {'ON' if loop_video else 'OFF'}")
            elif key in [ord("r"), ord("R")]:
                analyzer.reset_stats()
                if is_video:
                    cap.set(cv2.CAP_PROP_POS_FRAMES, 0)
                    print("[INFO] Video restarted & statistics reset.")
                else:
                    print("[INFO] Session statistics reset.")
            elif key in [ord("s"), ord("S")]:
                save_session_log(analyzer.get_stats(), analyzer)

    except KeyboardInterrupt:
        print("\n[INFO] Session interrupted by user.")
    finally:
        cap.release()
        face_mesh.close()
        cv2.destroyAllWindows()

        stats = analyzer.get_stats()
        print("\n" + "=" * 55)
        print("           FINAL SESSION SUMMARY")
        print("=" * 55)
        print(f"  Duration:           {stats['duration_s']} s")
        print(f"  Total Frames:       {stats['total_frames']}")
        print(f"  Micro-Expressions:  {stats['micro_events_total']} events detected")
        print("  Emotion Distribution:")
        for emo, pct in stats["distribution"].items():
            print(f"    • {emo:<12}: {pct}%")
        print("=" * 55)


if __name__ == "__main__":
    main()

```

## Source file: src/config.py

{
  "path": "src/config.py",
  "blob_sha": "63347720c4ecc7604673bc07fd4fc1ffcf05b9bd",
  "sha256": "63d88d96e4c05a576492214692099eac50c757646b28350a83f02c8818c206b7",
  "lines": 184,
  "bytes": 6012,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 14,
    "jaw": 3,
    "mouth": 7,
    "wrinkle": 3,
    "facs": 2,
    "rig": 15,
    "face": 3,
    "lip": 11
  },
  "url": "https://github.com/mohd-faizy/facs-facial-expression-analyzer/blob/41218d78cfc67b0ab69bb5058e3b62ec5ad0fc8a/src/config.py",
  "function_signature_candidates": []
}

```text

"""
Configuration and constants for the Facial Expression & Micro-Detail Analyzer (FACS Engine).
Defines MediaPipe Face Mesh landmark indices, Action Units (AU), micro-expressions,
vibrant green cyber wiremesh colors, and multi-quadrant split-screen display parameters.
"""

# =====================================================================
# CAMERA & DISPLAY SETTINGS
# =====================================================================
CAMERA_INDEX = 0
DEFAULT_FRAME_WIDTH = 1280
DEFAULT_FRAME_HEIGHT = 720
QUADRANT_WIDTH = 600                  # Width of each video quadrant in dual-view
DASHBOARD_WIDTH = 440                 # Width of the right-hand HUD sidebar
TARGET_FPS = 30

# Number of frames to gather for baseline neutral calibration
CALIBRATION_FRAME_COUNT = 35

# =====================================================================
# UI COLOR PALETTE (BGR Format for OpenCV)
# =====================================================================
COLOR_BG_DARK = (16, 16, 22)          # Deep crisp charcoal
COLOR_PANEL_BG = (26, 26, 36)         # Glassmorphic dark slate
COLOR_PANEL_BORDER = (48, 48, 66)     # Subtle outline
COLOR_TEXT_PRIMARY = (255, 255, 255)  # Pure crisp white
COLOR_TEXT_SECONDARY = (185, 185, 205)# High-contrast soft gray
COLOR_TEXT_MUTED = (130, 130, 150)    # Readable medium gray

# Wiremesh & Dot Colors (Vibrant Cyber Green)
COLOR_MESH_LINE = (40, 240, 110)      # Vivid cyber neon green lines
COLOR_MESH_NODE = (50, 255, 130)      # Glowing neon green connection dots
COLOR_MESH_NODE_HALO = (10, 150, 60)  # Outer green glow halo
COLOR_MESH_NODE_ACCENT = (140, 255, 180)

# Quadrant Header Colors
COLOR_BADGE_RAW = (240, 175, 40)      # Ocean cyan/amber for raw feed
COLOR_BADGE_MESH = (50, 255, 130)     # Neon green for wiremesh feed

# Emotion Colors (BGR)
COLOR_EMOTION = {
    "HAPPY": (40, 225, 120),       # Vibrant emerald / mint
    "SURPRISED": (30, 220, 255),   # Neon bright yellow-gold
    "ANGRY": (50, 50, 245),        # Deep intense red
    "SAD": (230, 160, 60),         # Steel oceanic blue
    "DISGUSTED": (185, 50, 165),   # Deep magenta-purple
    "FEARFUL": (30, 140, 255),     # Bright warning amber/orange
    "SMIRK": (220, 100, 220),      # Electric orchid
    "NEUTRAL": (175, 185, 200),    # Balanced slate silver
}

COLOR_GAUGE_BG = (36, 36, 48)
COLOR_ACCENT_CYAN = (240, 200, 40)
COLOR_ALERT_FLASH = (40, 200, 255)
COLOR_CALIBRATING = (40, 210, 255)

# =====================================================================
# MEDIAPIPE FACE MESH LANDMARK INDICES (478 Keypoints)
# =====================================================================

# Eye outer / inner corners for scale & tilt normalization
LEFT_EYE_OUTER = 33
LEFT_EYE_INNER = 133
RIGHT_EYE_INNER = 362
RIGHT_EYE_OUTER = 263

# Eye contours (for Eye Aspect Ratio & Squint)
LEFT_EYE = [33, 160, 158, 133, 153, 144]
RIGHT_EYE = [362, 385, 387, 263, 373, 380]
LEFT_EYELID_UPPER = 159
LEFT_EYELID_LOWER = 145
RIGHT_EYELID_UPPER = 386
RIGHT_EYELID_LOWER = 374

# Eyebrows (Inner heads, Arch, Outer tails)
LEFT_BROW_INNER = 55
LEFT_BROW_MID = 65
LEFT_BROW_OUTER = 52
LEFT_BROW_TOP = 70

RIGHT_BROW_INNER = 285
RIGHT_BROW_MID = 295
RIGHT_BROW_OUTER = 282
RIGHT_BROW_TOP = 300

GLABELLA = 9  # Mid-point between eyebrows

# Mouth & Lips
MOUTH_CORNER_LEFT = 61
MOUTH_CORNER_RIGHT = 291
UPPER_LIP_TOP = 0
LOWER_LIP_BOTTOM = 17
UPPER_LIP_INNER = 13
LOWER_LIP_INNER = 14
MOUTH_CENTER_X = 14

# Cheeks
LEFT_CHEEK_UPPER = 117
RIGHT_CHEEK_UPPER = 346

# Nose
NOSE_BRIDGE = 168
NOSE_TIP = 1
NOSE_BOTTOM = 2
NOSE_ALA_LEFT = 102
NOSE_ALA_RIGHT = 331

# Jaw & Face bounds
CHIN_BOTTOM = 152
FOREHEAD_TOP = 10

# =====================================================================
# FACIAL ACTION UNITS (FACS) METADATA
# =====================================================================
ACTION_UNITS_META = {
    "AU04_brow_furrow": {
        "name": "Brow Furrow",
        "muscle": "Corrugator",
        "desc": "Brows knit together (Anger/Focus)",
        "color": (60, 60, 245)
    },
    "AU01_brow_raise": {
        "name": "Brow Raise",
        "muscle": "Frontalis",
        "desc": "Inner/mid brows elevated (Surprise/Awe)",
        "color": (40, 220, 245)
    },
    "AU12_lip_pull": {
        "name": "Smile Pull",
        "muscle": "Zygomaticus",
        "desc": "Mouth corners pulled up/back (Joy)",
        "color": (50, 225, 120)
    },
    "AU15_lip_droop": {
        "name": "Lip Droop",
        "muscle": "Depressor anguli",
        "desc": "Mouth corners dragged down (Sadness)",
        "color": (230, 155, 60)
    },
    "AU06_cheek_squint": {
        "name": "Cheek Squint",
        "muscle": "Orbicularis oculi",
        "desc": "Cheeks raised, genuine Duchenne crinkle",
        "color": (100, 240, 150)
    },
    "AU07_lid_tighten": {
        "name": "Lid Tighten",
        "muscle": "Orbicularis pars palpebralis",
        "desc": "Squint of suspicion / skepticism / scrutiny",
        "color": (240, 175, 50)
    },
    "AU09_nose_wrinkle": {
        "name": "Nose Wrinkle",
        "muscle": "Levator labii alaeque",
        "desc": "Bridge wrinkled & nostrils flared (Disgust)",
        "color": (190, 60, 185)
    },
    "AU24_lip_compress": {
        "name": "Lip Compress",
        "muscle": "Orbicularis oris",
        "desc": "Lips pressed tight (Suppressed Anger / Stress)",
        "color": (70, 135, 255)
    },
    "AU26_jaw_drop": {
        "name": "Mouth Open",
        "muscle": "Masseter relax",
        "desc": "Vertical jaw separation (Surprise/Shock)",
        "color": (40, 195, 255)
    },
    "AU14_asymmetry": {
        "name": "Asymmetry / Smirk",
        "muscle": "Buccinator unilateral",
        "desc": "One-sided smile / contempt twitch",
        "color": (235, 100, 225)
    },
}

# Smoothing constant for Action Units (0.0 to 1.0, higher = faster response)
SMOOTHING_ALPHA = 0.35

# Micro-expression spike detection parameters
MICRO_SPIKE_THRESHOLD = 0.22
MICRO_MIN_DURATION_S = 0.09
MICRO_MAX_DURATION_S = 0.52


```

## Source file: src/__init__.py

{
  "path": "src/__init__.py",
  "blob_sha": "0ef8e8630f0b2cee17445e65efaecac283daae8f",
  "sha256": "7d57a0eb6e42fd60c49e6c0d1266e97c67fd2eaff6ea625eaa8b250523665a3d",
  "lines": 1,
  "bytes": 76,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/mohd-faizy/facs-facial-expression-analyzer/blob/41218d78cfc67b0ab69bb5058e3b62ec5ad0fc8a/src/__init__.py",
  "function_signature_candidates": []
}

```text

"""Contactless Vital Signs & Cognitive Fatigue Monitor - Source Package."""


```

## Source file: src/dashboard.py

{
  "path": "src/dashboard.py",
  "blob_sha": "815a92a28770b54b3008a2f21e8809263c8b1382",
  "sha256": "78ee98965c04b2f0fed4572c82c652d676f90d0734679287567ee37362ab75b0",
  "lines": 561,
  "bytes": 27944,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 13,
    "jaw": 2,
    "mouth": 7,
    "wrinkle": 2,
    "facs": 7,
    "rig": 3,
    "face": 21,
    "lip": 14
  },
  "url": "https://github.com/mohd-faizy/facs-facial-expression-analyzer/blob/41218d78cfc67b0ab69bb5058e3b62ec5ad0fc8a/src/dashboard.py",
  "function_signature_candidates": [
    {
      "line": 50,
      "text": "def __init__(self, width: int = DASHBOARD_WIDTH):"
    },
    {
      "line": 56,
      "text": "def update_fps(self) -> float:"
    },
    {
      "line": 69,
      "text": "def extract_quadrant("
    },
    {
      "line": 98,
      "text": "def draw_on_face("
    },
    {
      "line": 142,
      "text": "def draw_quadrant_header(self, frame: np.ndarray, title: str, color: Tuple[int, int, int]):"
    },
    {
      "line": 160,
      "text": "def _draw_calibration_banner(self, frame: np.ndarray, progress: float, w: int, center_x: Optional[int] = None):"
    },
    {
      "line": 183,
      "text": "def _draw_green_mesh_and_dots(self, frame: np.ndarray, landmarks, w: int, h: int):"
    },
    {
      "line": 231,
      "text": "def _draw_muscle_details("
    },
    {
      "line": 340,
      "text": "def _draw_floating_pill(self, frame: np.ndarray, pt: Tuple[int, int], emotion: str, conf: float, arousal: float, color):"
    },
    {
      "line": 364,
      "text": "def render_panel(self, results: Optional[Dict[str, Any]], frame_h: int, source_info: str = \"WEBCAM\") -> np.ndarray:"
    },
    {
      "line": 553,
      "text": "def _render_shortcuts(self, panel: np.ndarray, frame_h: int):"
    }
  ]
}

```text

"""
Real-time high-fidelity OpenCV dashboard HUD for Facial Expression & FACS Analyzer.
Renders on-face cyber green wiremesh with glowing green connection dots,
dynamic muscle tension vectors, dual-quadrant header badges, and a high-DPI
glassmorphic sidebar HUD with crisp typography and anti-aliasing.
"""

import math
import time
from typing import Any, Dict, List, Optional, Tuple

import cv2
import mediapipe as mp
import numpy as np

from src.config import (
    ACTION_UNITS_META,
    COLOR_ALERT_FLASH,
    COLOR_BADGE_MESH,
    COLOR_BADGE_RAW,
    COLOR_BG_DARK,
    COLOR_CALIBRATING,
    COLOR_EMOTION,
    COLOR_GAUGE_BG,
    COLOR_MESH_LINE,
    COLOR_MESH_NODE,
    COLOR_MESH_NODE_ACCENT,
    COLOR_MESH_NODE_HALO,
    COLOR_PANEL_BG,
    COLOR_PANEL_BORDER,
    COLOR_TEXT_MUTED,
    COLOR_TEXT_PRIMARY,
    COLOR_TEXT_SECONDARY,
    DASHBOARD_WIDTH,
    QUADRANT_WIDTH,
)

# Canonical MediaPipe face mesh connections
mp_face_mesh = mp.solutions.face_mesh
FACEMESH_TESSELATION = list(mp_face_mesh.FACEMESH_TESSELATION)
FACEMESH_TESSELATION_ARR = np.array(FACEMESH_TESSELATION, dtype=np.int32)
FACEMESH_CONTOURS_ARR = np.array(list(mp_face_mesh.FACEMESH_CONTOURS), dtype=np.int32)


class ExpressionDashboard:
    """Renders real-time HUD dashboard, cyber green mesh with green dots,
    dual-quadrant split headers, and dynamic facial muscle tension vectors.
    """

    def __init__(self, width: int = DASHBOARD_WIDTH):
        self.width = width
        self.fps_history: List[float] = []
        self.last_frame_time = time.time()
        self.fps = 30.0

    def update_fps(self) -> float:
        now = time.time()
        dt = now - self.last_frame_time
        self.last_frame_time = now
        if dt > 0.001:
            inst_fps = 1.0 / dt
            self.fps_history.append(inst_fps)
            if len(self.fps_history) > 20:
                self.fps_history.pop(0)
            self.fps = sum(self.fps_history) / len(self.fps_history)
        return self.fps

    @staticmethod
    def extract_quadrant(
        img: np.ndarray,
        center_x: Optional[int],
        target_w: int = 600,
        target_h: int = 720,
    ) -> np.ndarray:
        """Extract a centered quadrant crop (target_w x target_h) around the detected face."""
        ih, iw = img.shape[:2]
        if ih != target_h:
            scale = target_h / ih
            new_w = int(iw * scale)
            img = cv2.resize(img, (new_w, target_h), interpolation=cv2.INTER_LINEAR)
            ih, iw = img.shape[:2]
            if center_x is not None:
                center_x = int(center_x * scale)

        if iw >= target_w:
            cx = iw // 2 if center_x is None else center_x
            x1 = max(0, min(cx - target_w // 2, iw - target_w))
            return img[:, x1 : x1 + target_w].copy()
        else:
            # Letterbox pad symmetrically with dark background if frame is narrower
            pad_left = (target_w - iw) // 2
            pad_right = target_w - iw - pad_left
            canvas = np.zeros((target_h, target_w, 3), dtype=np.uint8)
            canvas[:, :] = COLOR_BG_DARK
            canvas[:, pad_left : pad_left + iw] = img
            return canvas

    def draw_on_face(
        self,
        frame: np.ndarray,
        results: Optional[Dict[str, Any]],
        show_mesh: bool = True,
        show_facs_vectors: bool = True,
        mesh_landmarks=None,
    ):
        """Draw cyber green wiremesh with glowing green connection dots, muscle vectors,
        and floating emotion pill.
        """
        if mesh_landmarks is None:
            return

        h, w = frame.shape[:2]
        dominant = results["dominant"] if results else "NEUTRAL"
        conf = results["confidence"] if results else 1.0
        aus = results["action_units"] if results else {}
        pts_2d = results.get("pts_2d", {}) if results else {}
        is_calibrating = results.get("is_calibrating", False) if results else False
        calib_progress = results.get("calibration_progress", 1.0) if results else 1.0
        emo_color = COLOR_EMOTION.get(dominant, (180, 180, 180))

        # 1. Cyber Green Wiremesh with Glowing Green Connection Dots
        if show_mesh:
            self._draw_green_mesh_and_dots(frame, mesh_landmarks, w, h)

        # 2. Dynamic FACS Muscle Vectors & Expression Geometry
        if show_facs_vectors and pts_2d and not is_calibrating:
            self._draw_muscle_details(frame, aus, pts_2d, emo_color, dominant)

        # 3. Floating Pill or Calibration Banner
        if results is not None:
            face_cx = pts_2d.get(1, (w // 2, 0))[0] if pts_2d else w // 2
            if is_calibrating:
                self._draw_calibration_banner(frame, calib_progress, w, center_x=face_cx)
            else:
                forehead_pt = pts_2d.get(10, None)
                if forehead_pt is None:
                    forehead_pt = (face_cx, 60)
                else:
                    forehead_pt = (forehead_pt[0], max(35, forehead_pt[1] - 45))
                self._draw_floating_pill(frame, forehead_pt, dominant, conf, results["arousal"], emo_color)

    def draw_quadrant_header(self, frame: np.ndarray, title: str, color: Tuple[int, int, int]):
        """Render a sleek, semi-transparent quadrant header pill at the top of a video frame."""
        font = cv2.FONT_HERSHEY_SIMPLEX
        (tw, th), _ = cv2.getTextSize(title, font, 0.44, 1)

        pill_w = tw + 34
        pill_h = th + 18
        px, py = 16, 16

        overlay = frame.copy()
        cv2.rectangle(overlay, (px, py), (px + pill_w, py + pill_h), COLOR_PANEL_BG, -1)
        cv2.addWeighted(overlay, 0.85, frame, 0.15, 0, frame)

        # Subtle border and accent dot
        cv2.rectangle(frame, (px, py), (px + pill_w, py + pill_h), COLOR_PANEL_BORDER, 1, lineType=cv2.LINE_AA)
        cv2.circle(frame, (px + 12, py + pill_h // 2), 4, color, -1, lineType=cv2.LINE_AA)
        cv2.putText(frame, title, (px + 22, py + th + 5), font, 0.44, COLOR_TEXT_PRIMARY, 1, cv2.LINE_AA)

    def _draw_calibration_banner(self, frame: np.ndarray, progress: float, w: int, center_x: Optional[int] = None):
        """Draw prominent floating banner when calibrating neutral resting face."""
        bar_w = 340
        bar_h = 44
        if center_x is not None:
            x = int(center_x - bar_w // 2)
            x = max(16, min(frame.shape[1] - bar_w - 16, x))
        else:
            x = (w - bar_w) // 2
        y = 56

        overlay = frame.copy()
        cv2.rectangle(overlay, (x, y), (x + bar_w, y + bar_h), COLOR_PANEL_BG, -1)
        cv2.addWeighted(overlay, 0.85, frame, 0.15, 0, frame)

        cv2.rectangle(frame, (x, y), (x + bar_w, y + bar_h), COLOR_CALIBRATING, 1, lineType=cv2.LINE_AA)

        fill_w = int((bar_w - 20) * np.clip(progress, 0.0, 1.0))
        cv2.rectangle(frame, (x + 10, y + 26), (x + 10 + fill_w, y + 34), COLOR_CALIBRATING, -1)

        msg = f"CALIBRATING NEUTRAL FACE... {int(progress * 100)}%"
        cv2.putText(frame, msg, (x + 14, y + 19), cv2.FONT_HERSHEY_SIMPLEX, 0.42, COLOR_TEXT_PRIMARY, 1, cv2.LINE_AA)

    def _draw_green_mesh_and_dots(self, frame: np.ndarray, landmarks, w: int, h: int):
        """Draw vibrant cyber green wiremesh tessellation with glowing green connection dots."""
        coords = np.empty((len(landmarks), 2), dtype=np.int32)
        for i, lm in enumerate(landmarks):
            coords[i] = [int(lm.x * w), int(lm.y * h)]

        overlay = frame.copy()

        # 1. Dense 3D green wiremesh lines (all 2556 structural edges)
        lines = coords[FACEMESH_TESSELATION_ARR]
        cv2.polylines(overlay, lines, isClosed=False, color=COLOR_MESH_LINE, thickness=1)

        # 2. Enhanced bright green facial contours (eyes, lips, brows, face oval)
        contour_lines = coords[FACEMESH_CONTOURS_ARR]
        cv2.polylines(overlay, contour_lines, isClosed=False, color=COLOR_MESH_NODE_ACCENT, thickness=1, lineType=cv2.LINE_AA)

        # Blend vibrant green lines smoothly onto frame
        cv2.addWeighted(overlay, 0.65, frame, 0.35, 0, frame)

        # 3. Green connection dots on EVERY mesh landmark vertex
        for pt in coords:
            cv2.circle(frame, (pt[0], pt[1]), 1, COLOR_MESH_LINE, -1, lineType=cv2.LINE_AA)

        # 4. Glowing neon green anchor dots on primary topological nodes
        key_node_indices = [
            # Jawline
            10, 338, 297, 332, 284, 251, 389, 356, 454, 323, 361, 288,
            397, 365, 379, 378, 400, 377, 152, 148, 176, 149, 150, 136,
            172, 58, 132, 93, 234, 127, 162, 21, 54, 103, 67, 109,
            # Eyes & Brows
            33, 160, 158, 133, 153, 144, 55, 65, 52, 70,
            362, 385, 387, 263, 373, 380, 285, 295, 282, 300,
            # Nose
            168, 6, 1, 2, 102, 331,
            # Lips
            61, 185, 40, 39, 37, 0, 267, 269, 270, 409, 291,
            146, 91, 181, 84, 17, 314, 405, 321, 375,
            # Cheeks & Temples
            117, 346, 123, 352, 234, 454
        ]

        for idx in key_node_indices:
            p = coords[idx]
            # Outer darker green halo
            cv2.circle(frame, (p[0], p[1]), 3, COLOR_MESH_NODE_HALO, -1, lineType=cv2.LINE_AA)
            # Inner bright glowing neon green dot
            cv2.circle(frame, (p[0], p[1]), 2, COLOR_MESH_NODE, -1, lineType=cv2.LINE_AA)

    def _draw_muscle_details(
        self,
        frame: np.ndarray,
        aus: Dict[str, float],
        pts_2d: Dict[int, np.ndarray],
        color: Tuple[int, int, int],
        dominant: str,
    ):
        """Draw active muscle tension lines with crisp vector rendering."""
        # 1. Eyebrow Furrow Line (Between Brows)
        lb = pts_2d.get(55)
        rb = pts_2d.get(285)
        furrow_val = aus.get("AU04_brow_furrow", 0)

        if lb is not None and rb is not None:
            if furrow_val > 0.15:
                thickness = int(2 + furrow_val * 4)
                cv2.line(frame, tuple(lb), tuple(rb), (40, 40, 245), thickness, lineType=cv2.LINE_AA)
                cv2.circle(frame, tuple(lb), 5, (50, 60, 255), -1, lineType=cv2.LINE_AA)
                cv2.circle(frame, tuple(rb), 5, (50, 60, 255), -1, lineType=cv2.LINE_AA)

                mid_x = (lb[0] + rb[0]) // 2
                mid_y = (lb[1] + rb[1]) // 2 - 12
                cv2.putText(frame, f"FURROW {int(furrow_val * 100)}%", (mid_x - 42, mid_y),
                            cv2.FONT_HERSHEY_SIMPLEX, 0.40, (70, 90, 255), 1, cv2.LINE_AA)
            else:
                cv2.line(frame, tuple(lb), tuple(rb), (80, 80, 110), 1, lineType=cv2.LINE_AA)

        # 2. Eyebrow Raise Lift Vectors (AU01)
        raise_val = aus.get("AU01_brow_raise", 0)
        if raise_val > 0.20:
            lp = pts_2d.get(70)
            rp = pts_2d.get(300)
            lift_len = int(10 + raise_val * 24)
            if lp is not None:
                cv2.arrowedLine(frame, tuple(lp), (lp[0], lp[1] - lift_len), (30, 220, 250), 2, tipLength=0.35)
            if rp is not None:
                cv2.arrowedLine(frame, tuple(rp), (rp[0], rp[1] - lift_len), (30, 220, 250), 2, tipLength=0.35)

        # 3. Mouth Expression Triangle & Polygon
        lc = pts_2d.get(61)
        rc = pts_2d.get(291)
        top_lip = pts_2d.get(0)
        bot_lip = pts_2d.get(17)

        if lc is not None and rc is not None and top_lip is not None and bot_lip is not None:
            mouth_poly = np.array([lc, top_lip, rc, bot_lip], dtype=np.int32)
            smile_val = aus.get("AU12_lip_pull", 0)
            sad_val = aus.get("AU15_lip_droop", 0)
            open_val = aus.get("AU26_jaw_drop", 0)
            compress_val = aus.get("AU24_lip_compress", 0)

            if smile_val > 0.20 and open_val < 0.35:
                thickness = int(2 + smile_val * 3)
                cv2.polylines(frame, [mouth_poly], isClosed=True, color=(40, 225, 120), thickness=thickness, lineType=cv2.LINE_AA)
                cv2.circle(frame, tuple(lc), 5, (50, 240, 140), -1)
                cv2.circle(frame, tuple(rc), 5, (50, 240, 140), -1)
                cv2.line(frame, tuple(lc), (lc[0] - 14, lc[1] - 8), (40, 225, 120), 2, lineType=cv2.LINE_AA)
                cv2.line(frame, tuple(rc), (rc[0] + 14, rc[1] - 8), (40, 225, 120), 2, lineType=cv2.LINE_AA)

            elif open_val > 0.22:
                # Open mouth / Surprise
                cv2.polylines(frame, [mouth_poly], isClosed=True, color=(30, 220, 250), thickness=3, lineType=cv2.LINE_AA)
                cv2.circle(frame, tuple(bot_lip), 5, (30, 220, 250), -1)

            elif sad_val > 0.20:
                cv2.polylines(frame, [mouth_poly], isClosed=True, color=(230, 160, 60), thickness=2, lineType=cv2.LINE_AA)
                cv2.arrowedLine(frame, tuple(lc), (lc[0], lc[1] + 20), (230, 160, 60), 2, tipLength=0.35)
                cv2.arrowedLine(frame, tuple(rc), (rc[0], rc[1] + 20), (230, 160, 60), 2, tipLength=0.35)

            elif compress_val > 0.25:
                cv2.line(frame, tuple(lc), tuple(rc), (70, 135, 255), 3, lineType=cv2.LINE_AA)
                cv2.circle(frame, tuple(lc), 4, (70, 135, 255), -1)
                cv2.circle(frame, tuple(rc), 4, (70, 135, 255), -1)

            else:
                cv2.polylines(frame, [mouth_poly], isClosed=True, color=(120, 120, 145), thickness=1, lineType=cv2.LINE_AA)

        # 4. Cheek Apple Circles & Duchenne Eye Crinkle Rays
        cheek_val = aus.get("AU06_cheek_squint", 0)
        if cheek_val > 0.25:
            l_chk = pts_2d.get(117)
            r_chk = pts_2d.get(346)
            if l_chk is not None:
                cv2.circle(frame, tuple(l_chk), int(6 + cheek_val * 8), (100, 240, 150), 2, lineType=cv2.LINE_AA)
            if r_chk is not None:
                cv2.circle(frame, tuple(r_chk), int(6 + cheek_val * 8), (100, 240, 150), 2, lineType=cv2.LINE_AA)

            l_eye = pts_2d.get(33)
            r_eye = pts_2d.get(263)
            if l_eye is not None:
                cv2.line(frame, tuple(l_eye), (l_eye[0] - 12, l_eye[1] - 6), (100, 240, 150), 1, lineType=cv2.LINE_AA)
                cv2.line(frame, tuple(l_eye), (l_eye[0] - 12, l_eye[1] + 6), (100, 240, 150), 1, lineType=cv2.LINE_AA)
            if r_eye is not None:
                cv2.line(frame, tuple(r_eye), (r_eye[0] + 12, r_eye[1] - 6), (100, 240, 150), 1, lineType=cv2.LINE_AA)
                cv2.line(frame, tuple(r_eye), (r_eye[0] + 12, r_eye[1] + 6), (100, 240, 150), 1, lineType=cv2.LINE_AA)

        # 5. Nose Bridge Wrinkle Crossbar
        nose_val = aus.get("AU09_nose_wrinkle", 0)
        if nose_val > 0.25:
            nb = pts_2d.get(168)
            al = pts_2d.get(102)
            ar = pts_2d.get(331)
            if nb is not None:
                cv2.line(frame, (nb[0] - 20, nb[1]), (nb[0] + 20, nb[1]), (185, 50, 165), 3, lineType=cv2.LINE_AA)
            if al is not None and ar is not None:
                cv2.circle(frame, tuple(al), 4, (185, 50, 165), -1)
                cv2.circle(frame, tuple(ar), 4, (185, 50, 165), -1)

    def _draw_floating_pill(self, frame: np.ndarray, pt: Tuple[int, int], emotion: str, conf: float, arousal: float, color):
        """Draw crisp floating emotion badge with anti-aliasing."""
        badge = f"{emotion} {int(conf * 100)}%"
        sub_text = f"Intensity {int(arousal * 100)}%"

        font = cv2.FONT_HERSHEY_SIMPLEX
        (tw, th), _ = cv2.getTextSize(badge, font, 0.62, 2)
        (sw, sh), _ = cv2.getTextSize(sub_text, font, 0.40, 1)

        pill_w = max(tw, sw) + 38
        pill_h = th + sh + 22
        px = max(10, min(frame.shape[1] - pill_w - 10, pt[0] - pill_w // 2))
        py = max(10, pt[1] - pill_h)

        overlay = frame.copy()
        cv2.rectangle(overlay, (px, py), (px + pill_w, py + pill_h), COLOR_PANEL_BG, -1)
        cv2.addWeighted(overlay, 0.88, frame, 0.12, 0, frame)

        cv2.rectangle(frame, (px, py), (px + pill_w, py + pill_h), color, 2, lineType=cv2.LINE_AA)
        cv2.circle(frame, (px + 14, py + 16), 5, color, -1, lineType=cv2.LINE_AA)

        cv2.putText(frame, badge, (px + 26, py + th + 6), font, 0.60, COLOR_TEXT_PRIMARY, 2, cv2.LINE_AA)
        cv2.putText(frame, sub_text, (px + 26, py + th + sh + 14), font, 0.40, COLOR_TEXT_SECONDARY, 1, cv2.LINE_AA)

    def render_panel(self, results: Optional[Dict[str, Any]], frame_h: int, source_info: str = "WEBCAM") -> np.ndarray:
        """Render high-DPI sidebar HUD with crisp, legible typography and input source badge."""
        panel_h = max(frame_h, 720)
        panel = np.zeros((panel_h, self.width, 3), dtype=np.uint8)
        panel[:] = COLOR_BG_DARK

        cv2.line(panel, (0, 0), (self.width, 0), COLOR_PANEL_BORDER, 2)

        # -------------------------------------------------------------
        # 1. Header (0 - 70px)
        # -------------------------------------------------------------
        cv2.putText(panel, "EXPRESSION & FACS ENGINE", (22, 28), cv2.FONT_HERSHEY_SIMPLEX, 0.58, COLOR_TEXT_PRIMARY, 2, cv2.LINE_AA)
        cv2.putText(panel, "MICRO-ACTION UNIT ANALYZER", (22, 45), cv2.FONT_HERSHEY_SIMPLEX, 0.36, COLOR_TEXT_MUTED, 1, cv2.LINE_AA)

        fps_str = f"{self.fps:.1f} FPS"
        cv2.putText(panel, fps_str, (self.width - 92, 28), cv2.FONT_HERSHEY_SIMPLEX, 0.44, COLOR_TEXT_SECONDARY, 1, cv2.LINE_AA)

        has_face = results is not None
        status_dot_color = (40, 220, 120) if has_face else (50, 50, 240)
        src_label = f"[{source_info}]"
        cv2.circle(panel, (26, 61), 5, status_dot_color, -1, lineType=cv2.LINE_AA)
        cv2.putText(panel, src_label, (38, 65), cv2.FONT_HERSHEY_SIMPLEX, 0.38, status_dot_color, 1, cv2.LINE_AA)

        cv2.line(panel, (20, 75), (self.width - 20, 75), COLOR_PANEL_BORDER, 1)

        if not has_face:
            cv2.putText(panel, "Standby: Align face in view", (22, 120), cv2.FONT_HERSHEY_SIMPLEX, 0.48, COLOR_TEXT_MUTED, 1, cv2.LINE_AA)
            self._render_shortcuts(panel, panel_h)
            return panel

        is_calibrating = results.get("is_calibrating", False)
        calib_progress = results.get("calibration_progress", 1.0)
        y_cursor = 85

        # Calibration Status Banner (if active)
        if is_calibrating:
            cv2.rectangle(panel, (20, y_cursor), (self.width - 20, y_cursor + 54), (38, 32, 18), -1)
            cv2.rectangle(panel, (20, y_cursor), (self.width - 20, y_cursor + 54), COLOR_CALIBRATING, 2)
            cv2.putText(panel, "CALIBRATING RESTING BASELINE...", (30, y_cursor + 20), cv2.FONT_HERSHEY_SIMPLEX, 0.40, COLOR_CALIBRATING, 1, cv2.LINE_AA)
            cv2.putText(panel, "Keep face relaxed / neutral", (30, y_cursor + 35), cv2.FONT_HERSHEY_SIMPLEX, 0.35, COLOR_TEXT_MUTED, 1, cv2.LINE_AA)
            bar_fill = int((self.width - 60) * np.clip(calib_progress, 0.0, 1.0))
            cv2.rectangle(panel, (30, y_cursor + 42), (30 + bar_fill, y_cursor + 48), COLOR_CALIBRATING, -1)
            y_cursor += 62

        # -------------------------------------------------------------
        # 2. Dominant Expression Hero Card
        # -------------------------------------------------------------
        dominant = results["dominant"]
        conf = results["confidence"]
        emo_color = COLOR_EMOTION.get(dominant, (180, 180, 180))

        card_h = 58
        cv2.rectangle(panel, (20, y_cursor), (self.width - 20, y_cursor + card_h), COLOR_PANEL_BG, -1)
        cv2.rectangle(panel, (20, y_cursor), (self.width - 20, y_cursor + card_h), COLOR_PANEL_BORDER, 1)
        cv2.rectangle(panel, (20, y_cursor), (26, y_cursor + card_h), emo_color, -1)

        cv2.putText(panel, "CURRENT EXPRESSION", (36, y_cursor + 18), cv2.FONT_HERSHEY_SIMPLEX, 0.36, COLOR_TEXT_MUTED, 1, cv2.LINE_AA)
        cv2.putText(panel, dominant, (36, y_cursor + 44), cv2.FONT_HERSHEY_SIMPLEX, 0.85, emo_color, 2, cv2.LINE_AA)

        pct_label = f"{int(conf * 100)}%"
        cv2.putText(panel, pct_label, (self.width - 92, y_cursor + 42), cv2.FONT_HERSHEY_SIMPLEX, 0.80, COLOR_TEXT_PRIMARY, 2, cv2.LINE_AA)
        cv2.putText(panel, "CONFIDENCE", (self.width - 92, y_cursor + 53), cv2.FONT_HERSHEY_SIMPLEX, 0.30, COLOR_TEXT_MUTED, 1, cv2.LINE_AA)

        y_cursor += card_h + 10

        # -------------------------------------------------------------
        # 3. Emotional Valence Bar
        # -------------------------------------------------------------
        val = results["valence"]
        cv2.putText(panel, "EMOTIONAL VALENCE", (22, y_cursor + 10), cv2.FONT_HERSHEY_SIMPLEX, 0.38, COLOR_TEXT_SECONDARY, 1, cv2.LINE_AA)
        v_color = (40, 220, 120) if val >= 0 else (60, 60, 240)
        cv2.putText(panel, f"{val:+.2f}", (self.width - 70, y_cursor + 10), cv2.FONT_HERSHEY_SIMPLEX, 0.38, v_color, 1, cv2.LINE_AA)

        bar_x = 22
        bar_w = self.width - 44
        bar_y = y_cursor + 16
        bar_h = 8
        cv2.rectangle(panel, (bar_x, bar_y), (bar_x + bar_w, bar_y + bar_h), COLOR_GAUGE_BG, -1)
        center_x = bar_x + bar_w // 2
        cv2.line(panel, (center_x, bar_y - 2), (center_x, bar_y + bar_h + 2), COLOR_TEXT_MUTED, 1)

        if val >= 0:
            fill_w = int((val) * (bar_w // 2))
            cv2.rectangle(panel, (center_x, bar_y), (center_x + fill_w, bar_y + bar_h), (40, 220, 120), -1)
        else:
            fill_w = int((-val) * (bar_w // 2))
            cv2.rectangle(panel, (center_x - fill_w, bar_y), (center_x, bar_y + bar_h), (60, 60, 240), -1)

        cv2.putText(panel, "NEG (-)", (bar_x, bar_y + bar_h + 10), cv2.FONT_HERSHEY_SIMPLEX, 0.30, COLOR_TEXT_MUTED, 1, cv2.LINE_AA)
        cv2.putText(panel, "NEUTRAL", (center_x - 18, bar_y + bar_h + 10), cv2.FONT_HERSHEY_SIMPLEX, 0.30, COLOR_TEXT_MUTED, 1, cv2.LINE_AA)
        cv2.putText(panel, "POS (+)", (bar_x + bar_w - 36, bar_y + bar_h + 10), cv2.FONT_HERSHEY_SIMPLEX, 0.30, COLOR_TEXT_MUTED, 1, cv2.LINE_AA)

        y_cursor += 38

        # -------------------------------------------------------------
        # 4. Emotion Spectrum
        # -------------------------------------------------------------
        cv2.putText(panel, "EMOTION SPECTRUM", (22, y_cursor), cv2.FONT_HERSHEY_SIMPLEX, 0.42, COLOR_TEXT_PRIMARY, 1, cv2.LINE_AA)
        cv2.line(panel, (22, y_cursor + 4), (self.width - 22, y_cursor + 4), COLOR_PANEL_BORDER, 1)
        y_cursor += 12

        emotions = results["emotions"]
        for emo_name, score in sorted(emotions.items(), key=lambda x: x[1], reverse=True):
            e_color = COLOR_EMOTION.get(emo_name, (180, 180, 180))
            cv2.putText(panel, emo_name, (22, y_cursor + 9), cv2.FONT_HERSHEY_SIMPLEX, 0.36, COLOR_TEXT_SECONDARY, 1, cv2.LINE_AA)

            b_start = 125
            b_width = self.width - b_start - 60
            cv2.rectangle(panel, (b_start, y_cursor), (b_start + b_width, y_cursor + 8), COLOR_GAUGE_BG, -1)
            fill_len = int(b_width * np.clip(score, 0.0, 1.0))
            if fill_len > 0:
                cv2.rectangle(panel, (b_start, y_cursor), (b_start + fill_len, y_cursor + 8), e_color, -1)

            cv2.putText(panel, f"{int(score * 100)}%", (b_start + b_width + 10, y_cursor + 8), cv2.FONT_HERSHEY_SIMPLEX, 0.34, COLOR_TEXT_PRIMARY, 1, cv2.LINE_AA)
            y_cursor += 13

        y_cursor += 6

        # -------------------------------------------------------------
        # 5. Minute Action Units (FACS Muscular Detail)
        # -------------------------------------------------------------
        cv2.putText(panel, "MINUTE ACTION UNITS (FACS MUSCLE)", (22, y_cursor), cv2.FONT_HERSHEY_SIMPLEX, 0.42, COLOR_TEXT_PRIMARY, 1, cv2.LINE_AA)
        cv2.line(panel, (22, y_cursor + 4), (self.width - 22, y_cursor + 4), COLOR_PANEL_BORDER, 1)
        y_cursor += 12

        action_units = results["action_units"]
        for au_id, val in action_units.items():
            meta = ACTION_UNITS_META.get(au_id, {"name": au_id, "color": (180, 180, 180)})
            au_name = meta["name"]
            au_color = meta["color"]

            text_color = COLOR_TEXT_PRIMARY if val > 0.35 else COLOR_TEXT_SECONDARY
            cv2.putText(panel, au_name, (22, y_cursor + 8), cv2.FONT_HERSHEY_SIMPLEX, 0.35, text_color, 1, cv2.LINE_AA)

            b_start = 155
            b_width = self.width - b_start - 60
            cv2.rectangle(panel, (b_start, y_cursor), (b_start + b_width, y_cursor + 8), COLOR_GAUGE_BG, -1)
            fill_len = int(b_width * np.clip(val, 0.0, 1.0))
            if fill_len > 0:
                cv2.rectangle(panel, (b_start, y_cursor), (b_start + fill_len, y_cursor + 8), au_color, -1)

            cv2.putText(panel, f"{int(val * 100)}%", (b_start + b_width + 10, y_cursor + 8), cv2.FONT_HERSHEY_SIMPLEX, 0.32, COLOR_TEXT_MUTED, 1, cv2.LINE_AA)
            y_cursor += 13

        y_cursor += 6

        # -------------------------------------------------------------
        # 6. Micro-Expression Tracker
        # -------------------------------------------------------------
        cv2.putText(panel, "MICRO-EXPRESSION TRACKER", (22, y_cursor), cv2.FONT_HERSHEY_SIMPLEX, 0.42, COLOR_TEXT_PRIMARY, 1, cv2.LINE_AA)
        cv2.line(panel, (22, y_cursor + 4), (self.width - 22, y_cursor + 4), COLOR_PANEL_BORDER, 1)
        y_cursor += 10

        micro_event = results.get("micro_event")
        if micro_event is not None:
            cv2.rectangle(panel, (20, y_cursor), (self.width - 20, y_cursor + 40), (35, 30, 60), -1)
            cv2.rectangle(panel, (20, y_cursor), (self.width - 20, y_cursor + 40), COLOR_ALERT_FLASH, 2)

            flash_title = f"! MICRO-EXPRESSION DETECTED !"
            cv2.putText(panel, flash_title, (28, y_cursor + 16), cv2.FONT_HERSHEY_SIMPLEX, 0.38, COLOR_ALERT_FLASH, 1, cv2.LINE_AA)

            detail_str = f"{micro_event['title']} ({micro_event['duration_ms']}ms) - Int: {micro_event['intensity']}%"
            cv2.putText(panel, detail_str, (28, y_cursor + 32), cv2.FONT_HERSHEY_SIMPLEX, 0.35, COLOR_TEXT_PRIMARY, 1, cv2.LINE_AA)
            y_cursor += 46
        else:
            cv2.rectangle(panel, (20, y_cursor), (self.width - 20, y_cursor + 24), COLOR_PANEL_BG, -1)
            cv2.rectangle(panel, (20, y_cursor), (self.width - 20, y_cursor + 24), COLOR_PANEL_BORDER, 1)
            cv2.putText(panel, "Scanning 90-520ms involuntary twitches...", (26, y_cursor + 16), cv2.FONT_HERSHEY_SIMPLEX, 0.33, COLOR_TEXT_MUTED, 1, cv2.LINE_AA)
            y_cursor += 28

        # -------------------------------------------------------------
        # 7. Recent Micro-Events Log
        # -------------------------------------------------------------
        recent_events = results.get("recent_events", [])
        if recent_events:
            cv2.putText(panel, "RECENT MICRO-EVENTS:", (22, y_cursor + 10), cv2.FONT_HERSHEY_SIMPLEX, 0.32, COLOR_TEXT_MUTED, 1, cv2.LINE_AA)
            y_cursor += 16
            for ev in reversed(recent_events):
                ev_str = f"• {ev['title']} ({ev['duration_ms']}ms)"
                cv2.putText(panel, ev_str, (28, y_cursor), cv2.FONT_HERSHEY_SIMPLEX, 0.31, COLOR_TEXT_SECONDARY, 1, cv2.LINE_AA)
                y_cursor += 13

        # -------------------------------------------------------------
        # 8. Fixed Bottom Controls Footer
        # -------------------------------------------------------------
        self._render_shortcuts(panel, panel_h)

        return panel

    def _render_shortcuts(self, panel: np.ndarray, frame_h: int):
        """Render clean bottom controls footer without colliding with content."""
        y_bottom = frame_h - 48
        cv2.line(panel, (20, y_bottom), (self.width - 20, y_bottom), COLOR_PANEL_BORDER, 1)

        sc1 = "[C] Calibrate    [M] Green Mesh    [F] Muscle Vectors"
        sc2 = "[V] Dual-View    [D] Dashboard     [SPACE] Pause   [Q] Quit"
        cv2.putText(panel, sc1, (22, y_bottom + 18), cv2.FONT_HERSHEY_SIMPLEX, 0.33, COLOR_TEXT_SECONDARY, 1, cv2.LINE_AA)
        cv2.putText(panel, sc2, (22, y_bottom + 34), cv2.FONT_HERSHEY_SIMPLEX, 0.33, COLOR_TEXT_MUTED, 1, cv2.LINE_AA)


```
