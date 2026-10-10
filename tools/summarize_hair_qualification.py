#!/usr/bin/env python3
"""Summarize paired trajectory evidence, never infer FPS from solver logs."""
import argparse
import json
import math
import re
from pathlib import Path


def summarize(path, expected, require_coordinate_batches=False, expected_guides=469):
    text = path.read_text(errors="replace")
    pattern = r"HYBRID HAIR FRAME frame=(\d+) max_position_error_m=(\S+) max_quaternion_component_error=(\S+)"
    frames = [(int(n), float(p), float(q)) for n, p, q in re.findall(pattern, text)]
    counts = re.findall(r"HYBRID JOINT FRAME COUNTERS frame=(\d+) coordinate_calls=(\d+) equality_dispatches=(\d+) cache_hits=(\d+) admitted=(\d+) native_fallbacks=(\d+)", text)
    counters = {int(row[0]): list(map(int, row[1:])) for row in counts}
    ids = [n for n, _, _ in frames]
    complete = ids == list(range(1, expected + 1))
    numerical = bool(frames) and all(math.isfinite(p) and math.isfinite(q) and 0 <= p < 1e-6 and 0 <= q < 5e-5 for _, p, q in frames)
    gpu = bool(frames) and len(counts)==len(counters) and set(counters)==set(ids) and any(row[0]>0 for row in counters.values()) and all(n in counters and counters[n][-1] == 0 and counters[n][0] == counters[n][-2] for n in ids)
    terminal = re.findall(r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;", text)
    status = terminal[-1] if terminal else None
    passed = bool(len(terminal)==1 and status[0] == "ok" and int(status[1]) == 1 and int(status[2]) == 0 and int(status[3]) == 0)
    modes = re.findall(r"^HYBRID JOINT COORDINATE BATCHES (true|false)$", text, re.MULTILINE)
    batching = modes[0] == "true" if len(modes)==1 else None
    mode_matches = not require_coordinate_batches or batching is True
    totals = re.findall(r"^HYBRID FULL HAIR guides=(\d+) frames=(\d+) calls=\d+ ", text, re.MULTILINE)
    model_matches = len(totals)==1 and tuple(map(int,totals[0]))==(expected_guides,expected)
    return {
        "log": str(path.resolve()), "expected_frames": expected,
        "completed_frame_count": len(frames), "last_completed_frame": ids[-1] if ids else None,
        "complete_ordered_frame_sequence": complete,
        "observed_frames_within_original_error_limits": numerical,
        "observed_frames_have_gpu_admission_without_native_fallback": gpu,
        "terminal_test_result": status,
        "expected_guides": expected_guides,
        "terminal_model_and_frame_counts_match": model_matches,
        "coordinate_batches_required": require_coordinate_batches,
        "coordinate_batches_runtime": batching,
        "runtime_mode_matches_requirement": mode_matches,
        "complete_paired_trajectory_log_checks_pass": complete and numerical and gpu and passed and model_matches and mode_matches,
        "maximum_observed_position_error_m": max((p for _, p, _ in frames if math.isfinite(p)), default=None),
        "maximum_observed_quaternion_error": max((q for _, _, q in frames if math.isfinite(q)), default=None),
        "scope": "Log evidence only; verify launch/source/model and live process separately. Does not establish physical hardware coverage or rendered FPS.",
        "rendered_fps_measured": False,
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("logs", nargs="+", type=Path)
    parser.add_argument("--expected-frames", type=int, default=720)
    parser.add_argument("--expected-guides", type=int, default=469)
    parser.add_argument("--require-coordinate-batches", action="store_true")
    args = parser.parse_args()
    if args.expected_frames <= 0:
        parser.error("expected frames must be positive")
    if args.expected_guides <= 0:
        parser.error("expected guides must be positive")
    print(json.dumps([summarize(path, args.expected_frames, args.require_coordinate_batches, args.expected_guides) for path in args.logs], indent=2, allow_nan=False))
