# Facial rendering source scan: Merserk/dlss5-visual-enhancer

Repository: https://github.com/Merserk/dlss5-visual-enhancer

Pinned source revision: 30d700c557c833f3ee937998d9698ce38ffd381f

Discovery topic: DLSS

Description: DLSS 5 Neural Rendering for Images & Video with Frame Generation, RTX Video Super Resolution, HDR, and Live Playback

Reported license: NOASSERTION

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"eye": ["src/desktop/qml/assets/icons/png/128/eye.png", "src/desktop/qml/assets/icons/png/128/eye_off.png", "src/desktop/qml/assets/icons/png/16/eye.png", "src/desktop/qml/assets/icons/png/16/eye_off.png", "src/desktop/qml/assets/icons/png/20/eye.png", "src/desktop/qml/assets/icons/png/20/eye_off.png", "src/desktop/qml/assets/icons/png/24/eye.png", "src/desktop/qml/assets/icons/png/24/eye_off.png", "src/desktop/qml/assets/icons/png/256/eye.png", "src/desktop/qml/assets/icons/png/256/eye_off.png", "src/desktop/qml/assets/icons/png/32/eye.png", "src/desktop/qml/assets/icons/png/32/eye_off.png", "src/desktop/qml/assets/icons/png/48/eye.png", "src/desktop/qml/assets/icons/png/48/eye_off.png", "src/desktop/qml/assets/icons/png/64/eye.png", "src/desktop/qml/assets/icons/png/64/eye_off.png"], "sss": ["src/desktop/qml/components/DlssSuperResolutionControls.qml"], "dlss": ["src/core/dlss_bridge.py", "src/core/dlss_modes.py", "src/desktop/qml/components/DlssSuperResolutionControls.qml", "src/upscale/video/dlss_pipeline.py"], "rig": ["src/desktop/qml/assets/icons/png/128/chevron_right.png", "src/desktop/qml/assets/icons/png/16/chevron_right.png", "src/desktop/qml/assets/icons/png/20/chevron_right.png", "src/desktop/qml/assets/icons/png/24/chevron_right.png", "src/desktop/qml/assets/icons/png/256/chevron_right.png", "src/desktop/qml/assets/icons/png/32/chevron_right.png", "src/desktop/qml/assets/icons/png/48/chevron_right.png", "src/desktop/qml/assets/icons/png/64/chevron_right.png"], "lip": ["src/desktop/qml/assets/icons/png/128/clipboard.png", "src/desktop/qml/assets/icons/png/16/clipboard.png", "src/desktop/qml/assets/icons/png/20/clipboard.png", "src/desktop/qml/assets/icons/png/24/clipboard.png", "src/desktop/qml/assets/icons/png/256/clipboard.png", "src/desktop/qml/assets/icons/png/32/clipboard.png", "src/desktop/qml/assets/icons/png/48/clipboard.png", "src/desktop/qml/assets/icons/png/64/clipboard.png"]}

## Source file: src/core/dlss_modes.py

{
  "path": "src/core/dlss_modes.py",
  "blob_sha": "6a638d69854bf84c5578a49aa66a1ad2a3cbfbec",
  "sha256": "f8d4dbe76ebe921c2eac92a223ca0a52e9cadd38694e054f29bce7510fc30035",
  "lines": 38,
  "bytes": 1400,
  "selection_score": 15,
  "keyword_line_counts": {
    "dlss": 16
  },
  "url": "https://github.com/Merserk/dlss5-visual-enhancer/blob/30d700c557c833f3ee937998d9698ce38ffd381f/src/core/dlss_modes.py",
  "function_signature_candidates": [
    {
      "line": 21,
      "text": "def validate_dlss(mode: str, preset: str) -> None:"
    },
    {
      "line": 28,
      "text": "def dlss_output_size(width: int, height: int, mode: str, *, even: bool = False) -> tuple[int, int]:"
    }
  ]
}

```text

"""User-facing DLSS Super Resolution modes and output sizing."""

from __future__ import annotations

import math

# Each media frame is evaluated once. Repeating a resolved still cannot
# reproduce the new geometry samples supplied by a jittered game renderer.
DLSS_MODES = {
    "DLAA": (1.0, 1),
    "Quality": (1.5, 1),
    "Balanced": (1.724, 1),
    "Performance": (2.0, 1),
    "Ultra Performance": (3.0, 1),
}
DLSS_PRESETS = ("Default", "J", "K", "L", "M")
DLSS_METHODS = ("Standard", "DLSS")
UPSCALE_ENGINES = ("RTX Video Super Resolution", "DLSS")


def validate_dlss(mode: str, preset: str) -> None:
    if mode not in DLSS_MODES:
        raise ValueError(f"Unknown DLSS mode: {mode!r}.")
    if preset not in DLSS_PRESETS:
        raise ValueError(f"Unknown DLSS preset: {preset!r}.")


def dlss_output_size(width: int, height: int, mode: str, *, even: bool = False) -> tuple[int, int]:
    if mode not in DLSS_MODES:
        raise ValueError(f"Unknown DLSS mode: {mode!r}.")
    if width < 32 or height < 32:
        raise ValueError("DLSS input must be at least 32×32 pixels.")
    factor = DLSS_MODES[mode][0]
    rounder = (lambda value: max(2, math.ceil(value / 2) * 2)) if even else math.ceil
    output = rounder(width * factor), rounder(height * factor)
    if max(output) > 16384:
        raise ValueError("DLSS output exceeds the 16384-pixel texture limit.")
    return output


```

## Source file: src/core/dlss_bridge.py

{
  "path": "src/core/dlss_bridge.py",
  "blob_sha": "003b7fc8f4bfe641ddbf3e518f2b700afbaaa26e",
  "sha256": "81472181db2d09f052a952d02e8ab0aab2b7b2931c65dfa2dbcdf813197cb25d",
  "lines": 420,
  "bytes": 20204,
  "selection_score": 15,
  "keyword_line_counts": {
    "sss": 51,
    "dlss": 74,
    "rig": 1,
    "face": 23,
    "lip": 3
  },
  "url": "https://github.com/Merserk/dlss5-visual-enhancer/blob/30d700c557c833f3ee937998d9698ce38ffd381f/src/core/dlss_bridge.py",
  "function_signature_candidates": [
    {
      "line": 64,
      "text": "def __init__(self):"
    },
    {
      "line": 72,
      "text": "def guard(self):"
    },
    {
      "line": 76,
      "text": "def run(self):"
    },
    {
      "line": 95,
      "text": "def submit(self, function, references=()):"
    },
    {
      "line": 108,
      "text": "def call(self, function, references=(), timeout=180.0):"
    },
    {
      "line": 118,
      "text": "def release(self, library, handle, owner):"
    },
    {
      "line": 123,
      "text": "def shutdown(self):"
    },
    {
      "line": 138,
      "text": "def _error(buffer: C.Array) -> str:"
    },
    {
      "line": 142,
      "text": "def _raise_native(buffer, references=()):"
    },
    {
      "line": 151,
      "text": "def _load(ordinal: int):"
    },
    {
      "line": 202,
      "text": "def initialize_bridge(ordinal: int) -> None:"
    },
    {
      "line": 208,
      "text": "def __init__(self, width: int, height: int, mode: str, preset: str = \"Default\", *,"
    },
    {
      "line": 232,
      "text": "def process(self, rgba: np.ndarray, *, reset: bool = False, phase: int = 0,"
    },
    {
      "line": 237,
      "text": "def _process(self, rgba: np.ndarray, *, reset: bool, phase: int, output_dtype) -> np.ndarray:"
    },
    {
      "line": 281,
      "text": "def process_cuda_frame(self, frame, *, color_matrix: int = 1,"
    },
    {
      "line": 292,
      "text": "def _process_cuda_frame(self, frame, *, color_matrix, color_range, rotation, reset, phase,"
    },
    {
      "line": 329,
      "text": "def retain():"
    },
    {
      "line": 332,
      "text": "def release():"
    },
    {
      "line": 363,
      "text": "def diagnostics(self):"
    },
    {
      "line": 376,
      "text": "def close(self):"
    },
    {
      "line": 383,
      "text": "def __enter__(self):"
    },
    {
      "line": 386,
      "text": "def __exit__(self, *_):"
    },
    {
      "line": 390,
      "text": "def process_image(rgba: np.ndarray, mode: str, preset: str = \"Default\", *,"
    }
  ]
}

```text

"""Independent NGX DLSS Super Resolution bridge (ABI 1).

This module deliberately never loads the Neural Rendering or RTX Video engines.
The native engine owns its D3D12 device, NGX feature, and estimated temporal guides.
"""

from __future__ import annotations

import ctypes as C
import atexit
import queue
import threading

import numpy as np
from PIL import Image

from .dlss_modes import DLSS_MODES, DLSS_PRESETS, dlss_output_size, validate_dlss
from .gpu_selection import detect_gpu
from .paths import DLSSSR_BRIDGE, DLSSSR_DIR, DLSSSR_RUNTIME
from .ngx_runtime import NGX_RUNTIME_LOCK


class _SessionDesc(C.Structure):
    _fields_ = [(name, C.c_uint32) for name in (
        "size", "abi", "input_width", "input_height", "output_width", "output_height", "mode", "preset", "still_image")]


class _FrameDesc(C.Structure):
    _fields_ = [
        ("size", C.c_uint32), ("abi", C.c_uint32), ("input_rgba16f", C.c_void_p),
        ("input_stride", C.c_uint32), ("output_rgba16f", C.c_void_p),
        ("output_stride", C.c_uint32), ("reset", C.c_uint32), ("phase", C.c_uint32),
        ("input_format", C.c_uint32), ("output_format", C.c_uint32),
    ]


class _FrameResult(C.Structure):
    _fields_ = [(name, C.c_uint32) for name in (
        "size", "abi", "render_width", "render_height", "guide_estimated", "ngx_result", "phase", "reserved")]
    _fields_.append(("milliseconds", C.c_double))


class _CudaFrameDesc(C.Structure):
    _fields_ = [(name, C.c_uint32) for name in ("size", "abi", "width", "height", "pixel_format")]
    _fields_ += [("y_pointer", C.c_uint64), ("uv_pointer", C.c_uint64)]
    _fields_ += [(name, C.c_uint32) for name in (
        "y_stride", "uv_stride", "color_matrix", "color_range", "rotation",
        "reset", "phase", "output_p010", "video_color", "chroma_location")]


class _CudaSurfaceDesc(C.Structure):
    _fields_ = [(name, C.c_uint32) for name in (
        "size", "abi", "width", "height", "pixel_format", "stride")]
    _fields_ += [("y_pointer", C.c_uint64), ("uv_pointer", C.c_uint64)]


_library = None
_ordinal = None
_lock = threading.RLock()


class _NativeWorker:
    """One ordered NGX worker; DLPack destructors enqueue nonblocking releases."""
    def __init__(self):
        self.calls = queue.SimpleQueue()
        self.lock = threading.Lock()
        self.thread = None
        self.stopping = False
        self.poisoned = ""
        self.retained = []

    def guard(self):
        if self.poisoned:
            raise RuntimeError(f"DLSS SR engine is poisoned: {self.poisoned}. Restart the application.")

    def run(self):
        while True:
            request = self.calls.get()
            if request is None:
                return
            function, references, done, result, failures = request
            try:
                with NGX_RUNTIME_LOCK:
                    self.guard()
                    result.append(function())
            except BaseException as exc:
                self.poisoned = f"native exception: {exc}"
                self.retained.append(references)
                failures.append(exc)
            finally:
                function = references = request = None
                done.set()
                done = result = failures = None

    def submit(self, function, references=()):
        self.guard()
        done, result, failures = threading.Event(), [], []
        with self.lock:
            if self.stopping:
                raise RuntimeError("DLSS SR is shutting down.")
            if self.thread is None:
                self.thread = threading.Thread(target=self.run, name="dlsssr-native", daemon=True)
                self.thread.start()
                atexit.register(self.shutdown)
            self.calls.put((function, references, done, result, failures))
        return done, result, failures

    def call(self, function, references=(), timeout=180.0):
        done, result, failures = self.submit(function, references)
        if not done.wait(timeout):
            self.retained.append((function, references))
            self.poisoned = f"native call exceeded {timeout:g} seconds"
        self.guard()
        if failures:
            raise failures[0]
        return result[0]

    def release(self, library, handle, owner):
        if self.poisoned or self.stopping:
            return
        self.submit(lambda: library.dlsssr_cuda_surface_release(handle), (owner,))

    def shutdown(self):
        with self.lock:
            if self.thread is None or self.stopping:
                return
            self.stopping = True
            self.calls.put(None)
            thread = self.thread
        thread.join(5.0)
        if thread.is_alive():
            self.poisoned = "native worker did not drain during shutdown"


_WORKER = _NativeWorker()


def _error(buffer: C.Array) -> str:
    return buffer.value.decode("utf-8", "replace") or "DLSS native engine failed without a diagnostic."


def _raise_native(buffer, references=()):
    detail = _error(buffer)
    if "poison" in detail.casefold():
        _WORKER.poisoned = detail
        _WORKER.retained.append(references)
        _WORKER.guard()
    raise RuntimeError(detail)


def _load(ordinal: int):
    global _library, _ordinal
    with _lock:
        if _library is not None:
            if _ordinal != ordinal:
                raise RuntimeError("DLSS is already bound to another GPU; restart the application to switch GPUs.")
            return _library
        if not DLSSSR_RUNTIME.is_file() or not DLSSSR_BRIDGE.is_file():
            raise RuntimeError(f"DLSS bridge or NVIDIA runtime is missing from {DLSSSR_DIR}.")
        lib = C.CDLL(str(DLSSSR_BRIDGE))
        lib.dlsssr_abi_version.argtypes = []
        lib.dlsssr_abi_version.restype = C.c_uint32
        if lib.dlsssr_abi_version() != 1:
            raise RuntimeError("DLSS bridge ABI does not match this application.")
        lib.dlsssr_init.argtypes = [C.c_int, C.c_wchar_p, C.c_void_p, C.c_int]
        lib.dlsssr_init.restype = C.c_int
        lib.dlsssr_session_create.argtypes = [C.POINTER(_SessionDesc), C.c_void_p, C.c_int]
        lib.dlsssr_session_create.restype = C.c_void_p
        lib.dlsssr_session_release.argtypes = [C.c_void_p]
        lib.dlsssr_session_release.restype = None
        lib.dlsssr_process_rgba16f.argtypes = [C.c_void_p, C.POINTER(_FrameDesc), C.POINTER(_FrameResult), C.c_void_p, C.c_int]
        lib.dlsssr_process_rgba16f.restype = C.c_int
        lib.dlsssr_process_cuda_yuv.argtypes = [
            C.c_void_p, C.POINTER(_CudaFrameDesc), C.POINTER(_FrameResult),
            C.POINTER(C.c_void_p), C.c_void_p, C.c_int]
        lib.dlsssr_process_cuda_yuv.restype = C.c_int
        lib.dlsssr_cuda_surface_desc.argtypes = [C.c_void_p, C.POINTER(_CudaSurfaceDesc)]
        lib.dlsssr_cuda_surface_desc.restype = C.c_int
        lib.dlsssr_cuda_surface_retain.argtypes = [C.c_void_p]
        lib.dlsssr_cuda_surface_retain.restype = None
        lib.dlsssr_cuda_surface_release.argtypes = [C.c_void_p]
        lib.dlsssr_cuda_surface_release.restype = None
        if hasattr(lib, "dlsssr_session_diagnostics"):
            lib.dlsssr_session_diagnostics.argtypes = [C.c_void_p, C.POINTER(C.c_uint64), C.c_uint32]
            lib.dlsssr_session_diagnostics.restype = C.c_int
            lib.dlsssr_get_diagnostics.argtypes = [C.POINTER(C.c_uint64), C.c_uint32]
            lib.dlsssr_get_diagnostics.restype = C.c_int
            lib.dlsssr_version.argtypes = []
            lib.dlsssr_version.restype = C.c_char_p
            lib.dlsssr_host_formats.argtypes = []
            lib.dlsssr_host_formats.restype = C.c_uint32
        if hasattr(lib, "dlsssr_host_output_formats"):
            lib.dlsssr_host_output_formats.argtypes = []
            lib.dlsssr_host_output_formats.restype = C.c_uint32
        error = C.create_string_buffer(1024)
        if not _WORKER.call(lambda: lib.dlsssr_init(ordinal, str(DLSSSR_DIR), error, len(error)), (error, lib)):
            _raise_native(error, (error, lib))
        _library, _ordinal = lib, ordinal
        return lib


def initialize_bridge(ordinal: int) -> None:
    """Register DLSS SR with NGX before another feature fixes search paths."""
    _load(ordinal)


class DLSSSession:
    def __init__(self, width: int, height: int, mode: str, preset: str = "Default", *,
                 gpu_uuid: str = "auto", even: bool = False, still_image: bool = False):
        validate_dlss(mode, preset)
        self.width, self.height = width, height
        self.output_width, self.output_height = dlss_output_size(width, height, mode, even=even)
        self.mode, self.preset = mode, preset
        gpu = detect_gpu(gpu_uuid)
        self.gpu_name = str(gpu.get("name", "NVIDIA GPU"))
        self.ordinal = int(gpu.get("cuda_ordinal", gpu.get("index", 0)))
        self.lib = _load(self.ordinal)
        self.bridge_version = self.lib.dlsssr_version().decode("ascii") if hasattr(self.lib, "dlsssr_version") else "DLSS ABI 1"
        desc = _SessionDesc(C.sizeof(_SessionDesc), 1, width, height,
                            self.output_width, self.output_height,
                            tuple(DLSS_MODES).index(mode), DLSS_PRESETS.index(preset), int(still_image))
        error = C.create_string_buffer(1024)
        self._session_lock = threading.RLock()
        self._host_source = None
        self.handle = _WORKER.call(lambda: self.lib.dlsssr_session_create(C.byref(desc), error, len(error)),
                                   (self, desc, error))
        if not self.handle:
            _raise_native(error, (self, desc, error))
        self.frames = 0
        self.last_result = None

    def process(self, rgba: np.ndarray, *, reset: bool = False, phase: int = 0,
                output_dtype=np.float16) -> np.ndarray:
        with self._session_lock:
            return self._process(rgba, reset=reset, phase=phase, output_dtype=output_dtype)

    def _process(self, rgba: np.ndarray, *, reset: bool, phase: int, output_dtype) -> np.ndarray:
        if not self.handle:
            raise RuntimeError("DLSS session is closed.")
        if rgba.shape != (self.height, self.width, 4):
            raise ValueError("DLSS input shape does not match the session.")
        output_dtype = np.dtype(output_dtype)
        if output_dtype not in (np.float16, np.uint8, np.uint16):
            raise ValueError("DLSS output must be float16, uint8, or uint16.")
        output_format = 0
        if hasattr(self.lib, "dlsssr_host_output_formats"):
            output_format = {np.dtype(np.uint8): 1, np.dtype(np.uint16): 2}.get(output_dtype, 0)
        input_format = 0
        if rgba.dtype in (np.uint8, np.uint16) and hasattr(self.lib, "dlsssr_host_formats"):
            source = np.ascontiguousarray(rgba)
            input_format = 1 if rgba.dtype == np.uint8 else 2
        elif rgba.dtype in (np.uint8, np.uint16):
            if self._host_source is None:
                self._host_source = np.empty(rgba.shape, dtype=np.float16)
            source = self._host_source
            # Float32 division, followed by half rounding, preserves the original
            # pixels without allocating an intermediate full-size float32 array.
            np.divide(rgba, np.float32(255 if rgba.dtype == np.uint8 else 65535),
                      out=source, dtype=np.float32, casting="unsafe")
        else:
            source = np.ascontiguousarray(rgba[..., :4], dtype=np.float16)
        output = np.empty((self.output_height, self.output_width, 4),
                          dtype=output_dtype if output_format else np.float16)
        frame = _FrameDesc(C.sizeof(_FrameDesc), 1, source.ctypes.data, source.strides[0],
                           output.ctypes.data, output.strides[0], int(reset), phase, input_format, output_format)
        result = _FrameResult()
        result.size, result.abi = C.sizeof(_FrameResult), 1
        error = C.create_string_buffer(1024)
        ok = _WORKER.call(lambda: self.lib.dlsssr_process_rgba16f(
            self.handle, C.byref(frame), C.byref(result), error, len(error)),
            (self, source, output, frame, result, error))
        if not ok:
            _raise_native(error, (self, source, output, frame, result, error))
        self.frames += 1
        self.last_result = result
        if not output_format and output_dtype != np.float16:
            levels = 65535 if output_dtype == np.uint16 else 255
            output = np.rint(np.clip(output.astype(np.float32), 0, 1) * levels).astype(output_dtype)
        return output

    def process_cuda_frame(self, frame, *, color_matrix: int = 1,
                           color_range: int = 0, rotation: int = 0,
                           reset: bool = False, phase: int = 0,
                           output_p010: bool = False, video_color: bool = True,
                           chroma_location: int = 1):
        with self._session_lock:
            return self._process_cuda_frame(frame, color_matrix=color_matrix, color_range=color_range,
                                            rotation=rotation, reset=reset, phase=phase,
                                            output_p010=output_p010, video_color=video_color,
                                            chroma_location=chroma_location)

    def _process_cuda_frame(self, frame, *, color_matrix, color_range, rotation, reset, phase,
                            output_p010, video_color, chroma_location):
        """Evaluate NVDEC NV12/P010 and return a CUDA AVFrame for NVENC/NR."""
        if not self.handle:
            raise RuntimeError("DLSS session is closed.")
        if getattr(getattr(frame, "format", None), "name", "") != "cuda":
            raise ValueError("DLSS CUDA input must be a CUDA decoded video frame.")
        pixel_format = {"nv12": 4, "p010": 5, "p010le": 5}.get(
            getattr(getattr(frame, "sw_format", None), "name", ""))
        if pixel_format is None or len(frame.planes) < 2:
            raise ValueError("DLSS CUDA input must expose NV12 or P010 planes.")
        source = _CudaFrameDesc(
            C.sizeof(_CudaFrameDesc), 1, frame.width, frame.height, pixel_format,
            int(frame.planes[0].buffer_ptr), int(frame.planes[1].buffer_ptr),
            int(frame.planes[0].line_size), int(frame.planes[1].line_size),
            int(color_matrix), int(color_range), int(rotation), int(reset),
            int(phase), int(output_p010), int(video_color), int(chroma_location))
        result = _FrameResult()
        result.size, result.abi = C.sizeof(_FrameResult), 1
        handle = C.c_void_p()
        error = C.create_string_buffer(1024)
        ok = _WORKER.call(lambda: self.lib.dlsssr_process_cuda_yuv(
            self.handle, C.byref(source), C.byref(result), C.byref(handle), error, len(error)),
            (self, frame, source, result, handle, error))
        if not ok:
            _raise_native(error, (self, frame, source, result, handle, error))
        if not handle.value:
            raise RuntimeError("DLSS returned no CUDA output surface.")
        surface = _CudaSurfaceDesc()
        surface.size, surface.abi = C.sizeof(_CudaSurfaceDesc), 1
        if not self.lib.dlsssr_cuda_surface_desc(handle, C.byref(surface)):
            _WORKER.release(self.lib, handle, self)
            raise RuntimeError("DLSS returned an invalid CUDA output surface.")
        from .neural_bridge import _DLPackPlane
        import av
        released = False

        def retain():
            self.lib.dlsssr_cuda_surface_retain(handle)

        def release():
            _WORKER.release(self.lib, handle, self)

        bits = 16 if surface.pixel_format == 5 else 8
        item_size = bits // 8
        y = _DLPackPlane(
            pointer=surface.y_pointer, shape=(surface.height, surface.width),
            strides=(surface.stride // item_size, 1), bits=bits,
            device_id=self.ordinal, retain=retain, release=release)
        uv = _DLPackPlane(
            pointer=surface.uv_pointer,
            shape=((surface.height + 1) // 2, (surface.width + 1) // 2, 2),
            strides=(surface.stride // item_size, 2, 1), bits=bits,
            device_id=self.ordinal, retain=retain, release=release)
        try:
            output = av.VideoFrame.from_dlpack(
                [y, uv], format="p010le" if bits == 16 else "nv12",
                width=surface.width, height=surface.height,
                device_id=self.ordinal, primary_ctx=True)
        finally:
            if not released:
                release()
                released = True
        self.frames += 1
        self.last_result = result
        return output, {
            "input_ms": 0.0, "ngx_ms": result.milliseconds,
            "output_ms": 0.0, "scene_cut": bool(result.reserved),
            "gpu_pre_resize": (result.render_width, result.render_height) != (self.width, self.height),
        }

    def diagnostics(self):
        if not hasattr(self.lib, "dlsssr_session_diagnostics"):
            return {}
        with self._session_lock:
            if not self.handle:
                return {}
            values = (C.c_uint64 * 9)()
            if not _WORKER.call(lambda: self.lib.dlsssr_session_diagnostics(self.handle, values, len(values)),
                                (self, values)):
                raise RuntimeError("DLSS SR session returned invalid diagnostics.")
            return dict(zip(("host_frames", "cuda_frames", "upload_bytes", "download_bytes", "surface_allocations",
                             "guide_descriptors", "still_image", "render_alias", "surface_slots"), values))

    def close(self):
        with self._session_lock:
            if self.handle:
                _WORKER.call(lambda: self.lib.dlsssr_session_release(self.handle), (self,))
                self.handle = None
                self._host_source = None

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()


def process_image(rgba: np.ndarray, mode: str, preset: str = "Default", *,
                  gpu_uuid: str = "auto", controller=None):
    """Evaluate a still once; duplicate frames contain no new spatial samples."""
    height, width = rgba.shape[:2]
    with DLSSSession(width, height, mode, preset, gpu_uuid=gpu_uuid, still_image=True) as session:
        if controller is not None and controller.cancel.is_set():
            from .jobs import Cancelled
            raise Cancelled("DLSS image processing stopped by user.")
        high_depth = rgba.dtype == np.uint16
        levels = 65535 if high_depth else 255
        processed = session.process(rgba, reset=True, output_dtype=np.uint16 if high_depth else np.uint8)
        if rgba.shape[2] == 4:
            alpha_source = rgba[..., 3]
            if alpha_source.dtype.kind == "f":
                alpha_source = np.rint(np.clip(alpha_source.astype(np.float32), 0, 1) * levels).astype(processed.dtype)
            alpha = Image.fromarray(alpha_source).resize(
                (session.output_width, session.output_height), Image.Resampling.LANCZOS)
            processed[..., 3] = np.clip(np.asarray(alpha), 0, levels).astype(processed.dtype)
        return processed, {
            "engine": "DLSS Super Resolution", "mode": mode, "preset": preset,
            "evaluations": 1, "guide_estimated": True,
            "media_pipeline": "source-anchored DLSS",
            "synthetic_jitter": False,
            "render_width": session.last_result.render_width,
            "render_height": session.last_result.render_height,
            "gpu_pre_resize": (session.last_result.render_width,
                               session.last_result.render_height) != (width, height),
            "ngx_result": f"0x{session.last_result.ngx_result:08X}",
            "gpu": session.gpu_name,
            "bridge_version": session.bridge_version,
        }


```

## Source file: src/upscale/video/dlss_pipeline.py

{
  "path": "src/upscale/video/dlss_pipeline.py",
  "blob_sha": "c5bfb95ff373a4a642620cd9cb70f06cf3589b81",
  "sha256": "86c058883a0ef42e640c76444d759fb355ddae04b45e8ceec30102902a305aa8",
  "lines": 276,
  "bytes": 16067,
  "selection_score": 15,
  "keyword_line_counts": {
    "sss": 2,
    "dlss": 33,
    "rig": 1,
    "lip": 2
  },
  "url": "https://github.com/Merserk/dlss5-visual-enhancer/blob/30d700c557c833f3ee937998d9698ce38ffd381f/src/upscale/video/dlss_pipeline.py",
  "function_signature_candidates": [
    {
      "line": 30,
      "text": "def convert_video_dlss(source, options, *, controller, progress=None, output_dir=None,"
    },
    {
      "line": 57,
      "text": "def update(value, message):"
    }
  ]
}

```text

"""DLSS video route with separate optional RTX Video HDR evaluation."""

from __future__ import annotations

import math
import tempfile
import time
import uuid
from contextlib import suppress
from fractions import Fraction
from pathlib import Path

import av
from ...core.ffmpeg.frames import open_video_decoder
import cv2
import numpy as np

from ...core import app_log, ffmpeg
from ...core.disk_paths import OutputFile, prepare_output_dir
from ...core.dlss_bridge import DLSSSession
from ...core.jobs import Cancelled
from ...core.naming import output_filename, unique_output_path
from ...core.paths import JOBS
from ...neural_rendering.video.guides import TemporalGuideGenerator
from .media import inspect_video, result_frame
from .models import UpscaleResult, output_size
from .native import FORMAT_RGBA8, FORMAT_R10, RTXVideoSession, probe_capabilities


def convert_video_dlss(source, options, *, controller, progress=None, output_dir=None,
                       metadata=None, video_gpu=None, preview_start_seconds=0.0):
    started = time.perf_counter()
    metadata = metadata or inspect_video(source, controller, reject_hdr=True)
    width, height = int(metadata["width"]), int(metadata["height"])
    ow, oh, _ = output_size(width, height, options)
    source_high_depth = int(metadata["depth"]) > 8
    output_depth = ffmpeg.output_video_depth(metadata["depth"], options.codec, options.hdr_enabled)
    preview = options.preview_frames is not None or options.preview_seconds is not None
    extension = {"MP4": ".mp4", "MKV": ".mkv", "MOV": ".mov"}[options.container]
    stamp = time.strftime("%Y%m%d-%H%M%S") + "-" + uuid.uuid4().hex[:8]
    destination = prepare_output_dir(output_dir)
    output = unique_output_path(destination / output_filename(
        source, extension, "Auto" if preview else options.rename_mode,
        options.custom_suffix, f"{source.stem}_DLSS_{stamp}"))
    output_file = OutputFile(output)
    report_path = app_log.session_path()
    input_container = None
    encoder = None
    writer = None
    hdr_session = None
    dlss = None
    delivered = 0
    cuts = 0
    timings = {"decode_seconds": 0.0, "dlss_seconds": 0.0,
               "hdr_seconds": 0.0, "encode_seconds": 0.0}

    def update(value, message):
        if controller.cancel.is_set():
            raise Cancelled("DLSS upscale stopped by user.")
        if progress:
            progress(value, message)

    JOBS.mkdir(exist_ok=True)
    try:
        with tempfile.TemporaryDirectory(prefix="dlss-video-", dir=JOBS) as work:
            temp_video = Path(work) / "encoded.mkv"
            update(.01, "Starting DLSS Super Resolution")
            dlss = DLSSSession(width, height, options.dlss_mode, options.dlss_preset,
                               gpu_uuid=options.ai_gpu_uuid, even=True)
            if (dlss.output_width, dlss.output_height) != (ow, oh):
                raise RuntimeError("DLSS output sizing differs from the video encoder size.")
            if options.hdr_enabled:
                from dataclasses import replace
                caps = probe_capabilities(options.ai_gpu_uuid, controller=controller)
                hdr_session = RTXVideoSession(
                    ow, oh, ow, oh, replace(options, vsr_enabled=False),
                    FORMAT_R10 if source_high_depth else FORMAT_RGBA8, caps, controller)
            encoder, encoder_thread, encoder_logs, selected, quality = ffmpeg.start_encoder(
                temp_video, options.codec, options.quality, controller, ow, oh,
                float(metadata["rate"]),
                None if video_gpu is None else int(video_gpu["cuda_ordinal"]),
                video_gpu is not None, hdr_mode=options.hdr_enabled,
                hdr_metadata={
                    "color_space": "bt2020nc" if options.hdr_enabled else "bt709",
                    "color_primaries": "bt2020" if options.hdr_enabled else "bt709",
                    "color_transfer": "smpte2084" if options.hdr_enabled else "bt709",
                    "color_range": "tv", "hdr": options.hdr_enabled,
                }, preserve_timestamps=True, bounded_logs=True,
                output_depth=output_depth)
            input_container = open_video_decoder(
                source, controller,
                pixel_format="rgba64le" if source_high_depth else "rgba",
                start_seconds=preview_start_seconds)
            stream = input_container.streams.video[0]
            stream.thread_type = "AUTO"
            stream_tb = stream.time_base or Fraction(1, max(1, round(float(metadata["rate"]))))
            writer = ffmpeg.RawVideoPacketMuxer(
                encoder.stdin, width=ow, height=oh, rate=metadata["rate"],
                time_base=stream_tb, pix_fmt="gbrp10le" if hdr_session else "rgba64le" if output_depth > 8 else "rgba")
            guides = TemporalGuideGenerator(width, height, cut_threshold=0.10)
            processed_float = None
            first_pts = None
            last_pts = None
            default_duration = max(1, round(Fraction(1) / metadata["rate"] / stream_tb))
            estimated = int(metadata["frames"] or max(1, math.ceil(metadata["duration"] * float(metadata["rate"]))))
            timings["setup_seconds"] = time.perf_counter() - started
            pipeline_tick = time.perf_counter()
            decoded = iter(input_container.decode(stream))
            while True:
                decode_tick = time.perf_counter()
                try:
                    frame = next(decoded)
                except StopIteration:
                    break
                timings["decode_seconds"] += time.perf_counter() - decode_tick
                if frame.is_corrupt:
                    raise RuntimeError("The source decoder returned a corrupt frame.")
                decode_tick = time.perf_counter()
                pts = round(Fraction(frame.pts) * (frame.time_base or stream_tb) / stream_tb) if frame.pts is not None else (
                    0 if last_pts is None else last_pts + default_duration)
                if last_pts is not None and pts <= last_pts:
                    pts = last_pts + default_duration
                if first_pts is None:
                    first_pts = pts
                if ((options.preview_frames is not None and delivered >= int(options.preview_frames)) or
                    (options.preview_seconds is not None and delivered and
                     float(Fraction(pts - first_pts) * stream_tb) >= options.preview_seconds)):
                    break
                rgba = frame.to_ndarray(format="rgba64le" if source_high_depth else "rgba")
                rotation = int(metadata["rotation"])
                if rotation:
                    rgba = cv2.rotate(rgba, {90: cv2.ROTATE_90_CLOCKWISE,
                                             180: cv2.ROTATE_180,
                                             270: cv2.ROTATE_90_COUNTERCLOCKWISE}[rotation])
                if rgba.shape[:2] != (height, width):
                    from ...core.ffmpeg.filters import resize_fit
                    rgba = resize_fit(rgba, width, height, controller=controller)
                rgba = np.ascontiguousarray(rgba)
                timings["decode_seconds"] += time.perf_counter() - decode_tick
                guide = guides.process(rgba)
                cuts += int(delivered > 0 and guide.reset)
                dlss_tick = time.perf_counter()
                result = dlss.process(rgba, reset=guide.reset, phase=delivered,
                                      output_dtype=np.float16 if hdr_session else np.uint16 if output_depth > 8 else np.uint8)
                timings["dlss_seconds"] += time.perf_counter() - dlss_tick
                postprocess_tick = time.perf_counter()
                if hdr_session is not None:
                    if processed_float is None:
                        processed_float = np.empty(result.shape, dtype=np.float32)
                    # Avoid NumPy's software half-float clipping loop.
                    np.clip(result, 0, 1, out=processed_float, dtype=np.float32)
                    hdr_tick = time.perf_counter()
                    if source_high_depth:
                        rgb10 = np.rint(processed_float[..., :3] * 1023).astype(np.uint32)
                        hdr_input = np.ascontiguousarray(
                            rgb10[..., 0] | (rgb10[..., 1] << 10) |
                            (rgb10[..., 2] << 20) | np.uint32(3 << 30))
                    else:
                        hdr_input = np.rint(processed_float * 255).astype(np.uint8)
                        hdr_input[..., 3] = 255
                    hdr_data = hdr_session.process_frame(hdr_input)
                    hdr_frame = result_frame(hdr_data, ow, oh, 2)
                    from ...core.ffmpeg.frames import packed_frame
                    processed = packed_frame(hdr_frame)
                    timings["hdr_seconds"] += time.perf_counter() - hdr_tick
                else:
                    processed = result
                timings["postprocess_seconds"] = timings.get("postprocess_seconds", 0.) + time.perf_counter() - postprocess_tick
                encode_tick = time.perf_counter()
                duration = round(Fraction(frame.duration or default_duration) *
                                 (frame.time_base or stream_tb) / stream_tb)
                writer.write(processed, pts - first_pts if preview else pts, max(1, duration))
                timings["encode_seconds"] += time.perf_counter() - encode_tick
                delivered += 1
                last_pts = pts
                if delivered % 4 == 0:
                    update(min(.86, .04 + .82 * delivered / max(1, estimated)),
                           f"DLSS video processing: {delivered:,} / {estimated:,} frames")
            if not delivered:
                raise ValueError("The source contains no decodable video frames.")
            timings["pipeline_seconds"] = time.perf_counter() - pipeline_tick
            writer.close(); writer = None
            encoder.stdin.close()
            flush_tick = time.perf_counter()
            if encoder.wait(timeout=600) != 0:
                encoder_thread.join(timeout=2)
                raise RuntimeError("Video encoder failed: " + "\n".join(list(encoder_logs)[-20:]))
            encoder_thread.join(timeout=2)
            timings["encoder_flush_seconds"] = time.perf_counter() - flush_tick
            controller.unregister(encoder)
            encoder = None
            input_container.close(); input_container = None
            if hdr_session:
                if hdr_session.completed_frames != delivered:
                    raise RuntimeError("RTX Video HDR frame count does not match DLSS output.")
                hdr_session.close(); hdr_session = None
            if dlss.frames != delivered:
                raise RuntimeError("DLSS frame count does not match the source.")
            # Strict source EOF and one evaluation/packet per frame already
            # establish the count when the container omits nb_frames.
            if not preview and metadata["frames"] and delivered != metadata["frames"]:
                source_count = ffmpeg.probe_video(
                    source, count_mode="exact", strict_decode=True, controller=controller)
                if int(source_count["frames"]) != delivered:
                    raise RuntimeError(
                        f"Source has {source_count['frames']} frames but DLSS processed {delivered}.")
            render_w, render_h = dlss.last_result.render_width, dlss.last_result.render_height
            native_diagnostics = dlss.diagnostics()
            bridge_version = dlss.bridge_version
            dlss.close(); dlss = None
            update(.90, "Muxing audio and metadata")
            mux_tick = time.perf_counter()
            audio_info = {}
            ffmpeg.final_mux(temp_video, source, output_file.temporary, options.container,
                             controller, preserve_supported_subtitles=True,
                             source_time_origin=metadata["origin"], audio_diagnostics=audio_info,
                             video_only=preview_start_seconds > 0,
                             video_color_metadata={
                                 "color_space": "bt2020nc" if options.hdr_enabled else "bt709",
                                 "color_primaries": "bt2020" if options.hdr_enabled else "bt709",
                                 "color_transfer": "smpte2084" if options.hdr_enabled else "bt709",
                                 "color_range": "tv"})
            timings["final_mux_seconds"] = time.perf_counter() - mux_tick
            update(.96, "Verifying output")
            verification_tick = time.perf_counter()
            saved = ffmpeg.probe_video(output_file.temporary, count_mode="packets", controller=controller)
            if int(saved["frames"]) != delivered:
                saved = ffmpeg.probe_video(output_file.temporary, count_mode="exact",
                                           strict_decode=True, controller=controller)
            if (int(saved["frames"]) != delivered or
                (int(saved["width"]), int(saved["height"])) != (ow, oh)):
                raise RuntimeError("DLSS output has an incorrect frame count or size.")
            if options.hdr_enabled:
                verified = inspect_video(output_file.temporary, controller)
                if (not verified["hdr"] or verified["depth"] < 10 or
                    verified["stream"].get("color_primaries") != "bt2020" or
                    verified["stream"].get("color_space") != "bt2020nc"):
                    raise RuntimeError("DLSS → RTX Video HDR output lost HDR signaling.")
            timings["verification_seconds"] = time.perf_counter() - verification_tick
            elapsed = time.perf_counter() - started
            status = {
                "engine": "DLSS Super Resolution", "mode": options.dlss_mode,
                "preset": options.dlss_preset, "guide_estimated": True,
                "media_pipeline": "source-anchored DLSS", "synthetic_jitter": False,
                "render_width": render_w, "render_height": render_h,
                "gpu_pre_resize": (render_w, render_h) != (width, height),
                "scene_cuts": cuts, "frames": delivered,
                "rtx_video_hdr": bool(options.hdr_enabled),
                "audio_streams": audio_info.get("streams", []),
                "timings": timings,
                "motion_backend": "gpu_lucas_kanade", **native_diagnostics,
                "bridge_version": bridge_version,
            }
            output_file.publish()
            app_log.info("upscale-dlss", f"done src={source.name} out={output.name} frames={delivered} mode={options.dlss_mode} preset={options.dlss_preset} fps={delivered/max(elapsed, 1e-9):.2f}")
            update(1.0, "Complete — DLSS evaluated")
            return UpscaleResult(str(output), report_path, delivered, ow, oh,
                                 options.hdr_enabled, elapsed, bridge_version=bridge_version,
                                 memory_path="host_rgba_d3d12_host_encoder",
                                 decode_backend="FFmpeg Vulkan/software", encode_backend=selected,
                                 timings=timings, bridge_status=status)
    finally:
        if writer:
            with suppress(Exception): writer.close()
        if input_container:
            with suppress(Exception): input_container.close()
        if hdr_session:
            with suppress(Exception): hdr_session.close(abort=True)
        if dlss:
            dlss.close()
        if encoder:
            with suppress(Exception):
                if encoder.poll() is None: encoder.kill()
                encoder.wait(timeout=5)
            controller.unregister(encoder)
        output_file.cleanup()


```

## Source file: app.py

{
  "path": "app.py",
  "blob_sha": "156549da2454f3ca40c88d506329db80ff31e71a",
  "sha256": "2f2fd3ef8e8b715b2df57c9bc6a0029e1c0de8ef231326800891ecbb0fae4b36",
  "lines": 20,
  "bytes": 378,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/Merserk/dlss5-visual-enhancer/blob/30d700c557c833f3ee937998d9698ce38ffd381f/app.py",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "def main() -> int:"
    }
  ]
}

```text

from __future__ import annotations

import sys

sys.dont_write_bytecode = True

from src.portable import configure_portable_environment

configure_portable_environment()

from src.desktop.app import launch_desktop


def main() -> int:
    """Launch the PySide6 + Qt Quick/QML desktop application."""
    return launch_desktop()


if __name__ == "__main__":
    sys.exit(main())


```
