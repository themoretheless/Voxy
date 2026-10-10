#!/usr/bin/env python3
"""Aggregate opt-in GPU equality host-stage logs; never qualify physics or FPS."""
import argparse
import json
import math
from pathlib import Path

PREFIX = "HAIR JOINT STAGE PROFILE "
SCHEMA = "voxy-joint-host-stage-profile-v1"
SCOPE = "equality_tasks_host_wall_times_not_gpu_timestamps_or_rendered_fps"
STAGES = (
    "prepare_wall_ms", "encode_wall_ms", "submit_map_wall_ms",
    "wait_wall_ms", "decode_refine_wall_ms",
)
COUNTS = (
    "owners", "dispatches", "submissions", "workspace_creations",
    "workspace_reuses", "errors",
)


def nonnegative(value, name, integer=False):
    valid = type(value) is int if integer else type(value) in (int, float)
    if not valid or value < 0 or (not integer and not math.isfinite(value)):
        raise ValueError(f"invalid nonnegative {'integer' if integer else 'number'}: {name}")
    return value


def summarize(paths):
    timings = dict.fromkeys((*STAGES, "total_wall_ms"), 0.0)
    counters = dict.fromkeys(COUNTS, 0)
    payload_bytes = calls = 0
    for path in paths:
        with path.open(encoding="utf-8") as source:
            for line_number, line in enumerate(source, 1):
                if not line.startswith(PREFIX):
                    continue
                try:
                    record = json.loads(line[len(PREFIX):])
                    if not isinstance(record, dict) or record.get("schema") != SCHEMA or record.get("scope") != SCOPE:
                        raise ValueError("unknown stage profile schema or scope")
                    stages = record.get("stages")
                    if not isinstance(stages, dict):
                        raise ValueError("missing stage timings")
                    values = {name: nonnegative(stages.get(name), name) for name in timings}
                    counts = {name: nonnegative(record.get(name), name, True) for name in COUNTS}
                    payload = nonnegative(stages.get("readback_payload_bytes"), "payload bytes", True)
                    total = values["total_wall_ms"]
                    if sum(values[name] for name in STAGES) > total + max(1e-6, total * 1e-9):
                        raise ValueError("stage times exceed total wall time")
                    if counts["submissions"] > counts["dispatches"] or counts["errors"] > counts["owners"]:
                        raise ValueError("inconsistent task counters")
                    if counts["workspace_creations"] + counts["workspace_reuses"] > counts["owners"]:
                        raise ValueError("more workspaces than ready owners")
                except (ValueError, OverflowError) as error:
                    raise ValueError(f"{path}:{line_number}: {error}") from error
                calls += 1
                payload_bytes += payload
                for name in timings:
                    timings[name] += values[name]
                for name in counters:
                    counters[name] += counts[name]
    if not calls:
        raise ValueError("no stage profile records; enable VOXY_HAIR_JOINT_STAGE_PROFILE=1")
    if not all(math.isfinite(value) for value in timings.values()):
        raise ValueError("aggregated wall time overflow")
    total = timings["total_wall_ms"]
    percentages = {name: 100.0 * (timings[name] / total) if total else 0.0 for name in STAGES}
    return {
        "schema": "voxy-joint-host-stage-summary-v1",
        "scope": SCOPE,
        "logs": [str(path) for path in paths],
        "profiled_calls": calls,
        "owner_instances_not_unique_operators": counters.pop("owners"),
        "counters": counters,
        "staging_payload_bytes_not_bus_traffic": payload_bytes,
        "wall_ms": timings,
        "stage_percent_of_total": percentages,
        "unattributed_wall_ms": max(0.0, total - sum(timings[name] for name in STAGES)),
        "limits": "Concurrent host wall times, including queue wait and callbacks; exclude profile log output, shader compilation, physical owner assembly/admission and rendering. Task errors can be recovered by active-set retry; they are not native fallback counts. This report does not qualify trajectory accuracy, GPU execution time or rendered FPS.",
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("logs", nargs="+", type=Path)
    args = parser.parse_args()
    try:
        result = summarize(args.logs)
    except (OSError, ValueError) as error:
        parser.exit(2, f"error: {error}\n")
    print(json.dumps(result, indent=2, allow_nan=False))


if __name__ == "__main__":
    main()
