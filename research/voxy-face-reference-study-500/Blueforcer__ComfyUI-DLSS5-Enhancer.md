# Facial rendering source scan: Blueforcer/ComfyUI-DLSS5-Enhancer

Repository: https://github.com/Blueforcer/ComfyUI-DLSS5-Enhancer

Pinned source revision: 796ed5927a202ba50b5c929cd08e16b365041162

Discovery topic: DLSS

Description: NVIDIA DLSS 5 Neural Rendering (NGX feature 18) as ComfyUI nodes. Enhance image batches and video files with the real neural renderer, with optional upscaling.

Reported license: NOASSERTION

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"dlss": ["dlss5/__init__.py", "dlss5/diagnostics.py", "dlss5/imaging.py", "dlss5/media.py", "dlss5/motion.py", "dlss5/paths.py", "dlss5/protocol.py", "dlss5/selftest.py", "dlss5/session.py", "dlss5/settings.py"]}

## Source file: dlss5/media.py

{
  "path": "dlss5/media.py",
  "blob_sha": "d12fd9a467fb16d810d501b8730e0fe175505188",
  "sha256": "ce17b11e8ac13151c4c5ec7c8a5f8a989da7ccc78f449bc714662b5111d41cac",
  "lines": 519,
  "bytes": 17685,
  "selection_score": 3,
  "keyword_line_counts": {
    "gum": 6,
    "dlss": 1
  },
  "url": "https://github.com/Blueforcer/ComfyUI-DLSS5-Enhancer/blob/796ed5927a202ba50b5c929cd08e16b365041162/dlss5/media.py",
  "function_signature_candidates": [
    {
      "line": 31,
      "text": "def require_av() -> None:"
    },
    {
      "line": 49,
      "text": "def validate_codec_container(codec: str, container: str) -> None:"
    },
    {
      "line": 60,
      "text": "def _run_json(command: list[str], timeout: int = PROBE_TIMEOUT) -> dict:"
    },
    {
      "line": 85,
      "text": "def probe_video(layout: RuntimeLayout, path: Path, *, exact_frames: bool = False) -> dict[str, Any]:"
    },
    {
      "line": 146,
      "text": "def rotate_frame(frame: np.ndarray, rotation: int) -> np.ndarray:"
    },
    {
      "line": 157,
      "text": "def decode_frames(path: Path, rotation: int, limit: int = 0) -> Iterator[tuple[int, np.ndarray, int]]:"
    },
    {
      "line": 174,
      "text": "def auto_bitrate_kbps(width: int, height: int, fps: float, codec: str) -> int:"
    },
    {
      "line": 181,
      "text": "def resolve_quality(quality: str, codec: str, width: int, height: int, fps: float) -> dict:"
    },
    {
      "line": 194,
      "text": "def _encoder_available_cached(ffmpeg: str, encoder: str, width: int, height: int) -> bool:"
    },
    {
      "line": 198,
      "text": "def _encoder_available(ffmpeg: str, encoder: str, width: int, height: int) -> bool:"
    },
    {
      "line": 206,
      "text": "def _probe_encoder(ffmpeg: str, encoder: str, width: int, height: int) -> bool:"
    },
    {
      "line": 237,
      "text": "def _codec_arguments("
    },
    {
      "line": 277,
      "text": "def __init__("
    },
    {
      "line": 349,
      "text": "def _drain(self) -> None:"
    },
    {
      "line": 361,
      "text": "def write(self, rgba: np.ndarray, pts: int) -> None:"
    },
    {
      "line": 371,
      "text": "def close(self) -> None:"
    },
    {
      "line": 398,
      "text": "def _kill(self) -> None:"
    },
    {
      "line": 417,
      "text": "def abort(self) -> None:"
    },
    {
      "line": 435,
      "text": "def _rendered_duration(layout: RuntimeLayout, path: Path) -> float:"
    },
    {
      "line": 461,
      "text": "def mux("
    }
  ]
}

```text

"""Decoding, encoding and muxing for the file-to-file video node."""

from __future__ import annotations

import json
import math
import subprocess
import threading
from fractions import Fraction
from functools import lru_cache
from pathlib import Path
from typing import Any, Iterator

import numpy as np

from .paths import RuntimeLayout

try:
    import av
except ImportError:  # reported when a node runs, not by hiding every node
    av = None

# Windows: keep console windows from flashing for every probe and encode.
NO_WINDOW = getattr(subprocess, "CREATE_NO_WINDOW", 0)

PROBE_TIMEOUT = 300
MUX_TIMEOUT = 3600
FLUSH_TIMEOUT = 120


def require_av() -> None:
    if av is None:
        raise RuntimeError(
            "The PyAV package is required for video decoding and encoding. "
            "Install it with: python -m pip install av"
        )

CODECS = ("H.264", "HEVC", "AV1", "ProRes Proxy")
CONTAINERS = ("MP4", "MKV", "MOV")
QUALITIES = ("Auto", "Good", "Best", "Max")

CONTAINER_EXTENSIONS = {"MP4": ".mp4", "MKV": ".mkv", "MOV": ".mov"}

# Bits per pixel-second divisors that produce the upstream automatic bitrates.
_AUTO_BITRATE_DIVISORS = {"H.264": 165_888, "HEVC": 331_776, "AV1": 414_720}
_QUALITY_MULTIPLIERS = {"Auto": 1, "Good": 2, "Best": 4}


def validate_codec_container(codec: str, container: str) -> None:
    if codec not in CODECS:
        raise ValueError(f"Unknown codec {codec!r}. Choose one of: {', '.join(CODECS)}.")
    if container not in CONTAINERS:
        raise ValueError(
            f"Unknown container {container!r}. Choose one of: {', '.join(CONTAINERS)}."
        )
    if codec == "ProRes Proxy" and container == "MP4":
        raise ValueError("ProRes Proxy needs the MOV or MKV container.")


def _run_json(command: list[str], timeout: int = PROBE_TIMEOUT) -> dict:
    try:
        result = subprocess.run(
            command,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout,
            creationflags=NO_WINDOW,
        )
    except subprocess.TimeoutExpired as exc:
        raise RuntimeError(
            f"ffprobe did not answer within {timeout} seconds for {command[-1]}."
        ) from exc
    if result.returncode:
        raise RuntimeError(result.stderr.strip() or "Media probe failed.")
    try:
        return json.loads(result.stdout)
    except ValueError as exc:
        raise RuntimeError(
            f"ffprobe returned no usable data for {command[-1]}: {result.stdout[:200]!r}"
        ) from exc


def probe_video(layout: RuntimeLayout, path: Path, *, exact_frames: bool = False) -> dict[str, Any]:
    """Read stream geometry, timing and frame count, honouring display rotation."""
    data = _run_json(
        [
            str(layout.ffprobe),
            "-v",
            "error",
            *(["-count_frames"] if exact_frames else []),
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,avg_frame_rate,r_frame_rate,time_base,duration,"
            "nb_frames,nb_read_frames,codec_name,color_transfer:stream_tags=rotate:"
            "stream_side_data=rotation",
            "-show_entries",
            "format=duration",
            "-of",
            "json",
            str(path),
        ]
    )
    streams = data.get("streams") or []
    if not streams:
        raise ValueError(f"{path} contains no decodable video stream.")
    stream = streams[0]

    rotation = int((stream.get("tags") or {}).get("rotate", 0) or 0)
    for side in stream.get("side_data_list") or []:
        if "rotation" in side:
            rotation = int(side["rotation"] or 0)
    rotation %= 360

    width, height = int(stream["width"]), int(stream["height"])
    if rotation in (90, 270):
        width, height = height, width

    if exact_frames:
        frames = int(stream.get("nb_read_frames") or stream.get("nb_frames") or 0)
    else:
        frames = int(stream.get("nb_frames") or 0)

    rate_text = stream.get("avg_frame_rate") or stream.get("r_frame_rate") or "30/1"
    rate = Fraction(rate_text) if rate_text not in {"0/0", None} else Fraction(30, 1)
    transfer = stream.get("color_transfer") or "unknown"

    return {
        "width": width,
        "height": height,
        "rotation": rotation,
        "frames": frames,
        "fps": float(rate),
        "rate": rate,
        "time_base": Fraction(stream.get("time_base") or "1/1000"),
        "duration": float(
            (data.get("format") or {}).get("duration") or stream.get("duration") or 0
        ),
        "codec": stream.get("codec_name") or "unknown",
        "hdr": transfer in {"smpte2084", "arib-std-b67"},
    }


def rotate_frame(frame: np.ndarray, rotation: int) -> np.ndarray:
    """Apply container rotation metadata so the render matches what players show."""
    if rotation == 90:
        return np.ascontiguousarray(np.rot90(frame, 3))
    if rotation == 180:
        return np.ascontiguousarray(np.rot90(frame, 2))
    if rotation == 270:
        return np.ascontiguousarray(np.rot90(frame, 1))
    return frame


def decode_frames(path: Path, rotation: int, limit: int = 0) -> Iterator[tuple[int, np.ndarray, int]]:
    """Yield ``(index, rgba, pts)`` for up to ``limit`` frames (0 = all)."""
    require_av()
    container = av.open(str(path))
    try:
        stream = container.streams.video[0]
        stream.thread_type = "AUTO"
        for index, frame in enumerate(container.decode(stream)):
            if limit and index >= limit:
                break
            rgba = rotate_frame(frame.to_ndarray(format="rgba"), rotation)
            pts = int(frame.pts if frame.pts is not None else index)
            yield index, rgba, pts
    finally:
        container.close()


def auto_bitrate_kbps(width: int, height: int, fps: float, codec: str) -> int:
    divisor = _AUTO_BITRATE_DIVISORS[codec]
    if width <= 0 or height <= 0 or not math.isfinite(fps) or fps <= 0:
        raise ValueError("Automatic bitrate needs positive dimensions and frame rate.")
    return max(1, int(math.floor(width * height * fps * 8 * 2 / divisor + 0.5)))


def resolve_quality(quality: str, codec: str, width: int, height: int, fps: float) -> dict:
    """Translate the quality label into encoder rate-control settings."""
    if quality not in QUALITIES:
        raise ValueError(f"Unknown quality {quality!r}. Choose one of: {', '.join(QUALITIES)}.")
    if codec == "ProRes Proxy":
        return {"selection": quality, "mode": "fixed-prores-proxy-profile"}
    if quality == "Max":
        return {"selection": quality, "mode": "constant-quality"}
    bitrate = auto_bitrate_kbps(width, height, fps, codec) * _QUALITY_MULTIPLIERS[quality]
    return {"selection": quality, "mode": "target-bitrate", "target_bitrate_kbps": bitrate}


@lru_cache(maxsize=64)
def _encoder_available_cached(ffmpeg: str, encoder: str, width: int, height: int) -> bool:
    return _probe_encoder(ffmpeg, encoder, width, height)


def _encoder_available(ffmpeg: str, encoder: str, width: int, height: int) -> bool:
    """Re-probe once after a negative result, so a transient failure is not permanent."""
    if _encoder_available_cached(ffmpeg, encoder, width, height):
        return True
    _encoder_available_cached.cache_clear()
    return _probe_encoder(ffmpeg, encoder, width, height)


def _probe_encoder(ffmpeg: str, encoder: str, width: int, height: int) -> bool:
    """Check that this encoder can really take the requested size on this GPU."""
    command = [
        ffmpeg,
        "-v",
        "error",
        "-f",
        "lavfi",
        "-i",
        f"color=size={width}x{height}:rate=1",
        "-frames:v",
        "1",
        "-c:v",
        encoder,
        "-f",
        "null",
        "-",
    ]
    try:
        result = subprocess.run(
            command,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            timeout=120,
            creationflags=NO_WINDOW,
        )
    except (OSError, subprocess.TimeoutExpired):
        return False
    return result.returncode == 0


def _codec_arguments(
    layout: RuntimeLayout, codec: str, quality: dict, width: int, height: int
) -> tuple[list[str], str]:
    if codec == "ProRes Proxy":
        return ["-c:v", "prores_ks", "-profile:v", "0", "-pix_fmt", "yuv422p10le"], "prores_ks"

    if quality["mode"] == "constant-quality":
        nvenc_rate = ["-rc", "vbr", "-cq", "0", "-b:v", "0"]
        software_rate = ["-crf", "0"]
    else:
        bitrate = f"{quality['target_bitrate_kbps']}k"
        nvenc_rate = ["-rc", "vbr", "-b:v", bitrate]
        software_rate = ["-b:v", bitrate]

    ffmpeg = str(layout.ffmpeg)
    plans = {
        "H.264": ("h264_nvenc", "libx264"),
        "HEVC": ("hevc_nvenc", "libx265"),
        "AV1": ("av1_nvenc", None),
    }
    hardware, software = plans[codec]
    if _encoder_available(ffmpeg, hardware, width, height):
        arguments = ["-c:v", hardware, "-preset", "p6", *nvenc_rate, "-pix_fmt", "yuv420p"]
        if codec != "AV1":
            arguments[4:4] = ["-tune", "hq"]
        return arguments, hardware
    if software is None:
        raise RuntimeError(
            f"AV1 NVENC cannot encode {width}x{height} on this GPU/driver. "
            "Choose H.264 or HEVC, or a lower upscaling mode."
        )
    return (
        ["-c:v", software, "-preset", "slow", *software_rate, "-pix_fmt", "yuv420p"],
        software,
    )


class RawFrameEncoder:
    """ffmpeg fed with raw RGBA frames through a NUT stream on stdin."""

    def __init__(
        self,
        layout: RuntimeLayout,
        target: Path,
        *,
        codec: str,
        quality: str,
        width: int,
        height: int,
        rate: Fraction,
        time_base: Fraction,
    ) -> None:
        self.quality = resolve_quality(quality, codec, width, height, float(rate))
        arguments, self.encoder_name = _codec_arguments(
            layout, codec, self.quality, width, height
        )
        command = [
            str(layout.ffmpeg),
            "-hide_banner",
            "-loglevel",
            "warning",
            "-y",
            "-f",
            "nut",
            "-i",
            "pipe:0",
            "-map",
            "0:v:0",
            "-an",
            *arguments,
            "-fps_mode",
            "passthrough",
            str(target),
        ]
        require_av()
        try:
            self.process = subprocess.Popen(
                command,
                stdin=subprocess.PIPE,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.PIPE,
                creationflags=NO_WINDOW,
            )
        except OSError as exc:
            raise RuntimeError(
                f"ffmpeg at {layout.ffmpeg} could not be started: {exc}"
            ) from exc
        self.logs: list[str] = []
        # PyAV containers are not thread safe: the writer thread and a cancelling
        # main thread must never touch this one at the same time.
        self._lock = threading.Lock()
        self._finished = False
        self._log_thread = threading.Thread(
            target=self._drain, name="dlss5-encoder-log", daemon=True
        )
        self._log_thread.start()

        try:
            self._container = av.open(self.process.stdin, mode="w", format="nut")
            self._stream = self._container.add_stream("rawvideo", rate=rate)
            self._stream.width = width
            self._stream.height = height
            self._stream.pix_fmt = "rgba"
            self._stream.time_base = time_base
            self._time_base = time_base
        except BaseException:
            # The log thread keeps this object alive, which would keep stdin open
            # and leave ffmpeg waiting for input forever.
            self._kill()
            self._log_thread.join(timeout=5)
            raise

    def _drain(self) -> None:
        try:
            for raw in iter(self.process.stderr.readline, b""):
                self.logs.append(raw.decode("utf-8", errors="replace").rstrip())
        except (OSError, ValueError):
            pass
        finally:
            try:
                self.process.stderr.close()
            except OSError:
                pass

    def write(self, rgba: np.ndarray, pts: int) -> None:
        frame = av.VideoFrame.from_ndarray(rgba, format="rgba")
        frame.pts = pts
        frame.time_base = self._time_base
        with self._lock:
            if self._finished:
                raise RuntimeError("The encoder is already closed, so the render is incomplete.")
            for packet in self._stream.encode(frame):
                self._container.mux(packet)

    def close(self) -> None:
        """Flush the encoder and require a clean ffmpeg exit."""
        watchdog = threading.Timer(FLUSH_TIMEOUT, self._kill)
        watchdog.daemon = True
        watchdog.start()
        try:
            with self._lock:
                if self._finished:
                    return
                self._finished = True
                for packet in self._stream.encode(None):
                    self._container.mux(packet)
                self._container.close()
        finally:
            watchdog.cancel()
        if self.process.stdin and not self.process.stdin.closed:
            self.process.stdin.close()
        try:
            code = self.process.wait(timeout=120)
        except subprocess.TimeoutExpired as exc:
            self._kill()
            self._log_thread.join(timeout=5)
            raise RuntimeError("ffmpeg did not finish within 120 seconds.") from exc
        self._log_thread.join(timeout=5)
        if code:
            raise RuntimeError("ffmpeg encoding failed:\n" + "\n".join(self.logs[-40:]))

    def _kill(self) -> None:
        """Kill ffmpeg and wait for it, so its handle on the output is released."""
        if self.process.poll() is None:
            try:
                self.process.kill()
            except OSError:
                pass
        try:
            self.process.wait(timeout=10)
        except (OSError, subprocess.TimeoutExpired):
            pass
        # Drop the buffered pipe explicitly, otherwise its finaliser reports a
        # broken pipe long after the fact.
        if self.process.stdin is not None and not self.process.stdin.closed:
            try:
                self.process.stdin.close()
            except OSError:
                pass

    def abort(self) -> None:
        """Tear the encoder down without flushing.

        ffmpeg is killed BEFORE the lock is taken: a writer thread blocked in
        mux() on a full stdin pipe holds that lock, and killing ffmpeg is what
        releases it. Taking the lock first would deadlock instead.
        """
        self._kill()
        with self._lock:
            if not self._finished:
                self._finished = True
                try:
                    self._container.close()
                except Exception:
                    pass
        self._log_thread.join(timeout=5)


def _rendered_duration(layout: RuntimeLayout, path: Path) -> float:
    data = _run_json(
        [
            str(layout.ffprobe),
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=duration:format=duration",
            "-of",
            "json",
            str(path),
        ]
    )
    streams = data.get("streams") or []
    for raw in ((streams[0] if streams else {}).get("duration"), (data.get("format") or {}).get("duration")):
        try:
            duration = float(raw)
        except (TypeError, ValueError):
            continue
        if math.isfinite(duration) and duration > 0:
            return duration
    raise RuntimeError("Could not determine the rendered duration for the final mux.")


def mux(
    layout: RuntimeLayout,
    rendered: Path,
    source: Path,
    output: Path,
    container: str,
    *,
    copy_audio: bool = True,
) -> None:
    """Combine the rendered video with the source audio, chapters and metadata."""
    duration = _rendered_duration(layout, rendered)
    if not copy_audio:
        maps = ["-map", "0:v:0"]
        streams = ["-c:v", "copy", "-an"]
    elif container == "MKV":
        maps = ["-map", "0:v:0", "-map", "1:a?", "-map", "1:s?"]
        streams = ["-c:v", "copy", "-c:a", "copy", "-c:s", "copy"]
    else:
        maps = ["-map", "0:v:0", "-map", "1:a?"]
        streams = ["-c:v", "copy", "-c:a", "aac", "-b:a", "192k", "-movflags", "+faststart"]

    command = [
        str(layout.ffmpeg),
        "-hide_banner",
        "-loglevel",
        "warning",
        "-y",
        "-i",
        str(rendered),
        # Input option on the SOURCE below: trims its audio to the rendered
        # length, which is what makes a max_frames preview mux correctly.
        "-t",
        f"{duration:.9f}",
        "-i",
        str(source),
        *maps,
        "-map_metadata",
        "1",
        "-map_chapters",
        "1",
        *streams,
        str(output),
    ]
    try:
        result = subprocess.run(
            command,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=MUX_TIMEOUT,
            creationflags=NO_WINDOW,
        )
    except subprocess.TimeoutExpired as exc:
        raise RuntimeError(
            f"The final mux did not finish within {MUX_TIMEOUT} seconds."
        ) from exc
    if result.returncode:
        raise RuntimeError("The final audio/metadata mux failed:\n" + result.stderr[-4000:])


```

## Source file: dlss5/paths.py

{
  "path": "dlss5/paths.py",
  "blob_sha": "be5dcda4ed77c2ada1a7b773e1d38cdc39fd4491",
  "sha256": "f8d0e893e188b89fdb003421de1103dbeccfdb4c22dce2a7133691b5d3e49fa6",
  "lines": 177,
  "bytes": 5967,
  "selection_score": 3,
  "keyword_line_counts": {
    "dlss": 16
  },
  "url": "https://github.com/Blueforcer/ComfyUI-DLSS5-Enhancer/blob/796ed5927a202ba50b5c929cd08e16b365041162/dlss5/paths.py",
  "function_signature_candidates": [
    {
      "line": 58,
      "text": "def ffmpeg(self) -> Path:"
    },
    {
      "line": 62,
      "text": "def ffprobe(self) -> Path:"
    },
    {
      "line": 66,
      "text": "def worker(self) -> Path:"
    },
    {
      "line": 70,
      "text": "def addon(self) -> Path:"
    },
    {
      "line": 74,
      "text": "def neural_runtime(self) -> Path:"
    },
    {
      "line": 78,
      "text": "def reshade_log(self) -> Path:"
    },
    {
      "line": 81,
      "text": "def require_ffmpeg(self) -> \"RuntimeLayout\":"
    },
    {
      "line": 86,
      "text": "def validate(self) -> \"RuntimeLayout\":"
    },
    {
      "line": 103,
      "text": "def _read_config() -> dict:"
    },
    {
      "line": 113,
      "text": "def write_config(runtime_dir: Path, ffmpeg_dir: Path | None = None) -> Path:"
    },
    {
      "line": 122,
      "text": "def _runtime_candidates(config: dict) -> list[Path]:"
    },
    {
      "line": 134,
      "text": "def _ffmpeg_candidates(config: dict, runtime_root: Path) -> list[Path]:"
    },
    {
      "line": 148,
      "text": "def _resolve_ffmpeg(config: dict, runtime_root: Path) -> tuple[Path, Path]:"
    },
    {
      "line": 164,
      "text": "def find_runtime(override: str | os.PathLike[str] | None = None) -> RuntimeLayout:"
    }
  ]
}

```text

"""Discovery and validation of the native DLSS 5 runtime.

The runtime binaries are NVIDIA/ReShade/RenoDX components that this repository
neither ships nor redistributes; ``install_runtime.py`` fetches them, or the
user points at an existing DLSS 5 Visual Enhancer installation.
"""

from __future__ import annotations

import json
import os
import shutil
from dataclasses import dataclass, field
from pathlib import Path

PACKAGE_ROOT = Path(__file__).resolve().parents[1]
CONFIG_FILE = PACKAGE_ROOT / "config.json"

RUNTIME_ENV = "DLSS5_RUNTIME_DIR"
FFMPEG_ENV = "DLSS5_FFMPEG_DIR"

# The worker keeps NVIDIA's image name because the signed-snippet caller
# contract checks it; it is an executable, not a library.
WORKER_NAME = "nvngx.dll"

REQUIRED_RUNTIME_FILES = (
    WORKER_NAME,
    "dxgi.dll",
    "renodx-dlss5.addon64",
    "nvngx_dlss.dll",
    "nvngx_dlssnr.dll",
)

SETUP_HINT = """Install it with:
    python_embeded/python.exe ComfyUI/custom_nodes/ComfyUI-DLSS5-Enhancer/install_runtime.py
or point the node at an existing DLSS 5 Visual Enhancer install:
    python ... install_runtime.py --runtime-dir <path>/bin/runtime
or set the DLSS5_RUNTIME_DIR environment variable."""


class RuntimeMissing(RuntimeError):
    """Raised when the native runtime is absent or incomplete."""


@dataclass(frozen=True, slots=True)
class RuntimeLayout:
    """Resolved locations of every binary a render session needs.

    ffmpeg is resolved on first use: the image node never needs it, and failing
    its lookup there would block a perfectly usable install.
    """

    root: Path
    # Excluded from equality so the layout stays hashable and comparable.
    config: dict = field(default_factory=dict, compare=False, repr=False)

    @property
    def ffmpeg(self) -> Path:
        return _resolve_ffmpeg(self.config, self.root)[0]

    @property
    def ffprobe(self) -> Path:
        return _resolve_ffmpeg(self.config, self.root)[1]

    @property
    def worker(self) -> Path:
        return self.root / WORKER_NAME

    @property
    def addon(self) -> Path:
        return self.root / "renodx-dlss5.addon64"

    @property
    def neural_runtime(self) -> Path:
        return self.root / "nvngx_dlssnr.dll"

    @property
    def reshade_log(self) -> Path:
        return self.root / "ReShade.log"

    def require_ffmpeg(self) -> "RuntimeLayout":
        """Resolve ffmpeg now, so a missing binary fails before a long render."""
        _resolve_ffmpeg(self.config, self.root)
        return self

    def validate(self) -> "RuntimeLayout":
        missing = [
            str(self.root / name)
            for name in REQUIRED_RUNTIME_FILES
            if not (self.root / name).is_file()
        ]
        if missing:
            raise RuntimeMissing(
                "The DLSS 5 runtime in "
                f"{self.root} is incomplete. Missing:\n  "
                + "\n  ".join(missing)
                + "\n\n"
                + SETUP_HINT
            )
        return self


def _read_config() -> dict:
    if not CONFIG_FILE.is_file():
        return {}
    try:
        data = json.loads(CONFIG_FILE.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        raise RuntimeMissing(f"{CONFIG_FILE} is not readable JSON: {exc}") from exc
    return data if isinstance(data, dict) else {}


def write_config(runtime_dir: Path, ffmpeg_dir: Path | None = None) -> Path:
    """Persist the chosen runtime location for later ComfyUI sessions."""
    payload = {"runtime_dir": str(Path(runtime_dir).resolve())}
    if ffmpeg_dir is not None:
        payload["ffmpeg_dir"] = str(Path(ffmpeg_dir).resolve())
    CONFIG_FILE.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    return CONFIG_FILE


def _runtime_candidates(config: dict) -> list[Path]:
    candidates: list[Path] = []
    env_value = os.environ.get(RUNTIME_ENV)
    if env_value:
        candidates.append(Path(env_value))
    configured = config.get("runtime_dir")
    if configured:
        candidates.append(Path(configured))
    candidates.append(PACKAGE_ROOT / "runtime")
    return candidates


def _ffmpeg_candidates(config: dict, runtime_root: Path) -> list[Path]:
    candidates: list[Path] = []
    env_value = os.environ.get(FFMPEG_ENV)
    if env_value:
        candidates.append(Path(env_value))
    configured = config.get("ffmpeg_dir")
    if configured:
        candidates.append(Path(configured))
    candidates.append(PACKAGE_ROOT / "ffmpeg" / "bin")
    # Layout of a DLSS 5 Visual Enhancer install: bin/runtime and bin/ffmpeg/bin.
    candidates.append(runtime_root.parent / "ffmpeg" / "bin")
    return candidates


def _resolve_ffmpeg(config: dict, runtime_root: Path) -> tuple[Path, Path]:
    for directory in _ffmpeg_candidates(config, runtime_root):
        ffmpeg = directory / "ffmpeg.exe"
        ffprobe = directory / "ffprobe.exe"
        if ffmpeg.is_file() and ffprobe.is_file():
            return ffmpeg, ffprobe
    on_path = shutil.which("ffmpeg"), shutil.which("ffprobe")
    if all(on_path):
        return Path(on_path[0]), Path(on_path[1])
    raise RuntimeMissing(
        "ffmpeg/ffprobe were not found. They ship with the DLSS 5 Visual Enhancer "
        f"download, or set {FFMPEG_ENV} to a directory containing them, or put "
        "them on PATH."
    )


def find_runtime(override: str | os.PathLike[str] | None = None) -> RuntimeLayout:
    """Locate the runtime; ``override`` wins, then env var, config, bundled dir."""
    config = _read_config()
    candidates = [Path(override)] if override else _runtime_candidates(config)

    for candidate in candidates:
        root = candidate.expanduser()
        if (root / WORKER_NAME).is_file():
            return RuntimeLayout(root=root.resolve(), config=config).validate()

    searched = "\n  ".join(str(path) for path in candidates)
    raise RuntimeMissing(
        "No DLSS 5 runtime was found. Searched:\n  " + searched + "\n\n" + SETUP_HINT
    )


```

## Source file: dlss5/motion.py

{
  "path": "dlss5/motion.py",
  "blob_sha": "2364d44f6f0285342f9554d37115feca8d4600d5",
  "sha256": "4aa1e8bcd8d7702ad917061681f5ea056e0350e6f58d6fd8f9e6db9667099665",
  "lines": 98,
  "bytes": 3474,
  "selection_score": 3,
  "keyword_line_counts": {
    "dlss": 3
  },
  "url": "https://github.com/Blueforcer/ComfyUI-DLSS5-Enhancer/blob/796ed5927a202ba50b5c929cd08e16b365041162/dlss5/motion.py",
  "function_signature_candidates": [
    {
      "line": 29,
      "text": "def __init__("
    },
    {
      "line": 57,
      "text": "def _downscaled_gray(self, rgba: np.ndarray) -> np.ndarray:"
    },
    {
      "line": 66,
      "text": "def process(self, rgba: np.ndarray) -> Guide:"
    }
  ]
}

```text

"""Temporal guide buffers for DLSS.

A decoded video has no motion vectors, so they are estimated with dense optical
flow.  DLSS expects backward motion (where a pixel came from), computed at the
render resolution and stored as FP16.
"""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np

from .imaging import cv2, require_cv2


@dataclass(frozen=True, slots=True)
class Guide:
    """Per-frame motion buffer plus the history-reset decision."""

    motion: np.ndarray
    reset: bool
    scene_score: float


class TemporalGuide:
    """Estimate motion between consecutive frames at the DLSS render size."""

    def __init__(
        self,
        width: int,
        height: int,
        *,
        flow_width: int = 640,
        scene_change_threshold: float = 0.24,
        enabled: bool = True,
    ) -> None:
        self.width = width
        self.height = height
        self.enabled = enabled
        self.scene_change_threshold = scene_change_threshold
        self.zero_motion = np.zeros((height, width, 2), dtype=np.float16)
        self._previous_gray: np.ndarray | None = None
        self._seen_first = False

        scale = min(1.0, flow_width / max(1, width))
        self.flow_width = max(64, int(round(width * scale / 2) * 2))
        self.flow_height = max(64, int(round(height * scale / 2) * 2))

        self._flow = None
        if enabled:
            require_cv2()
            self._flow = cv2.DISOpticalFlow_create(cv2.DISOPTICAL_FLOW_PRESET_MEDIUM)
            self._flow.setUseSpatialPropagation(True)
            self._flow.setFinestScale(1)

    def _downscaled_gray(self, rgba: np.ndarray) -> np.ndarray:
        require_cv2()
        gray = cv2.cvtColor(rgba, cv2.COLOR_RGBA2GRAY)
        return cv2.resize(
            gray,
            (self.flow_width, self.flow_height),
            interpolation=cv2.INTER_AREA,
        )

    def process(self, rgba: np.ndarray) -> Guide:
        """Return the guide for ``rgba``; the first frame always resets history."""
        if not self.enabled:
            # Zero motion, but reset only once: resetting every frame would throw
            # away the temporal history the neural pass builds up. This assumes a
            # coherent sequence, which is what motion="none" is for.
            first = not self._seen_first
            self._seen_first = True
            return Guide(motion=self.zero_motion, reset=first, scene_score=0.0)

        current = self._downscaled_gray(rgba)
        if self._previous_gray is None:
            self._previous_gray = current
            return Guide(motion=self.zero_motion, reset=True, scene_score=1.0)

        scene_score = float(np.mean(cv2.absdiff(current, self._previous_gray))) / 255.0
        reset = scene_score > self.scene_change_threshold
        if reset:
            motion = self.zero_motion
        else:
            # Flow from the current frame back to the previous one.
            flow = self._flow.calc(current, self._previous_gray, None)
            flow = cv2.resize(
                flow,
                (self.width, self.height),
                interpolation=cv2.INTER_LINEAR,
            )
            flow[..., 0] *= self.width / self.flow_width
            flow[..., 1] *= self.height / self.flow_height
            motion = np.ascontiguousarray(flow.astype(np.float16))

        self._previous_gray = current
        return Guide(motion=motion, reset=reset, scene_score=scene_score)


```

## Source file: dlss5/imaging.py

{
  "path": "dlss5/imaging.py",
  "blob_sha": "8474701a734aaa1b3b18fd379796b002989039b8",
  "sha256": "8cad2d4a2c86b2291e16807991526b3227a0df817cadcbd4b654003c364ca5a5",
  "lines": 72,
  "bytes": 2749,
  "selection_score": 3,
  "keyword_line_counts": {
    "dlss": 1
  },
  "url": "https://github.com/Blueforcer/ComfyUI-DLSS5-Enhancer/blob/796ed5927a202ba50b5c929cd08e16b365041162/dlss5/imaging.py",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "def require_cv2() -> None:"
    },
    {
      "line": 22,
      "text": "def fit_frame(rgba: np.ndarray, width: int, height: int) -> np.ndarray:"
    },
    {
      "line": 42,
      "text": "def tensor_to_rgba(frame: torch.Tensor) -> np.ndarray:"
    },
    {
      "line": 60,
      "text": "def rgba_to_tensor(rgba: np.ndarray, *, keep_alpha: bool) -> torch.Tensor:"
    },
    {
      "line": 67,
      "text": "def resize_alpha(alpha: np.ndarray, width: int, height: int) -> np.ndarray:"
    }
  ]
}

```text

"""Conversions between ComfyUI IMAGE tensors and the worker's RGBA8 frames."""

from __future__ import annotations

import numpy as np
import torch

try:
    import cv2
except ImportError:  # reported when a node runs, not by hiding every node
    cv2 = None


def require_cv2() -> None:
    if cv2 is None:
        raise RuntimeError(
            "OpenCV is required for frame scaling and motion estimation. "
            "Install opencv-python, or opencv-contrib-python if you already use it."
        )


def fit_frame(rgba: np.ndarray, width: int, height: int) -> np.ndarray:
    """Letterbox an RGBA frame into ``width`` x ``height`` without distortion."""
    require_cv2()
    source_height, source_width = rgba.shape[:2]
    if (source_width, source_height) == (width, height):
        return np.ascontiguousarray(rgba, dtype=np.uint8)

    scale = min(width / source_width, height / source_height)
    fit_width = max(1, min(width, int(round(source_width * scale))))
    fit_height = max(1, min(height, int(round(source_height * scale))))
    interpolation = cv2.INTER_AREA if scale < 1.0 else cv2.INTER_LANCZOS4
    resized = cv2.resize(rgba, (fit_width, fit_height), interpolation=interpolation)

    canvas = np.zeros((height, width, 4), dtype=np.uint8)
    left = (width - fit_width) // 2
    top = (height - fit_height) // 2
    canvas[top : top + fit_height, left : left + fit_width] = resized
    return canvas


def tensor_to_rgba(frame: torch.Tensor) -> np.ndarray:
    """Convert one (H, W, C) float image in 0..1 to a contiguous RGBA8 array."""
    array = frame.detach().to(device="cpu", dtype=torch.float32).clamp(0.0, 1.0).numpy()
    array = np.rint(array * 255.0).astype(np.uint8)
    channels = array.shape[2]
    if channels == 4:
        return np.ascontiguousarray(array)
    if channels == 3:
        height, width = array.shape[:2]
        rgba = np.empty((height, width, 4), dtype=np.uint8)
        rgba[..., :3] = array
        rgba[..., 3] = 255
        return rgba
    if channels == 1:
        return tensor_to_rgba(frame.repeat(1, 1, 3))
    raise ValueError(f"Unsupported image with {channels} channels.")


def rgba_to_tensor(rgba: np.ndarray, *, keep_alpha: bool) -> torch.Tensor:
    """Convert an RGBA8 worker result back to a float image in 0..1."""
    channels = 4 if keep_alpha else 3
    array = np.ascontiguousarray(rgba[..., :channels], dtype=np.float32) / 255.0
    return torch.from_numpy(array)


def resize_alpha(alpha: np.ndarray, width: int, height: int) -> np.ndarray:
    """Scale a source alpha plane onto the DLSS output grid."""
    require_cv2()
    if alpha.shape[:2] == (height, width):
        return alpha
    return cv2.resize(alpha, (width, height), interpolation=cv2.INTER_LINEAR)


```
