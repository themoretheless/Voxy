#!/usr/bin/env python3
"""Aggregate opt-in GPU equality host-stage logs; never qualify physics or FPS."""
import argparse
import json
import math
import re
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


SEED_COUNTS = ("incoming_valid_seeded_owners", "incoming_positive_seed_rows", "seeded_upload_owners")


def nonnegative(value, name, integer=False):
    valid = type(value) is int if integer else type(value) in (int, float)
    if not valid or value < 0 or (not integer and not math.isfinite(value)):
        raise ValueError(f"invalid nonnegative {'integer' if integer else 'number'}: {name}")
    return value


def summarize(paths):
    timings = dict.fromkeys((*STAGES, "total_wall_ms"), 0.0)
    counters = dict.fromkeys(COUNTS, 0)
    payload_bytes = calls = 0
    workspace_bytes = workspace_profiled_calls = 0
    seed_counts = {name: {"instances": 0, "reported_calls": 0} for name in SEED_COUNTS}
    shape_counts = {}
    slowest_batches = []
    frames = {}
    for path in paths:
        current_frame = None
        with path.open(encoding="utf-8") as source:
            for line_number, line in enumerate(source, 1):
                begin = re.match(r"(?:COUPLED HAIR GPU BEGIN frame=|AUTHORED FULL CYCLE BEGIN frame=)(\d+)\b", line)
                complete = re.match(r"(?:COUPLED HAIR GPU passed_frames=|AUTHORED FULL CYCLE frame=)(\d+)\b", line)
                if begin:
                    number = int(begin[1])
                    if number < 1:
                        raise ValueError(f"{path}:{line_number}: invalid frame number")
                    key = (str(path), number)
                    current_frame = frames.setdefault(key, {
                        "log": str(path), "frame": number, "status": "incomplete",
                        "profiled_calls": 0, "dispatches": 0, "errors": 0,
                        "profile_total_wall_ms": 0.0, "profile_wait_wall_ms": 0.0,
                        "staging_payload_bytes_not_bus_traffic": 0,
                        "complete_workspace_payload_bytes_over_reported_calls": 0,
                        "workspace_payload_reported_calls": 0,
                    })
                if complete:
                    completed = frames.get((str(path), int(complete[1])))
                    if completed is not None:
                        completed["status"] = "completed_as_reported_by_coupled_test"
                    current_frame = None
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
                    workspace = (nonnegative(record["complete_workspace_payload_bytes"], "complete workspace payload", True)
                                 if "complete_workspace_payload_bytes" in record else None)
                    seeds = {name: nonnegative(record[name], name, True)
                             for name in SEED_COUNTS if name in record}
                    for name in ("incoming_valid_seeded_owners", "seeded_upload_owners"):
                        if seeds.get(name, 0) > counts["owners"]:
                            raise ValueError("seeded owners exceed batch owners")
                    if ("seeded_upload_owners" in seeds and "incoming_valid_seeded_owners" in seeds
                            and seeds["seeded_upload_owners"] > seeds["incoming_valid_seeded_owners"]):
                        raise ValueError("seeded uploads exceed valid seed owners")
                    total = values["total_wall_ms"]
                    if sum(values[name] for name in STAGES) > total + max(1e-6, total * 1e-9):
                        raise ValueError("stage times exceed total wall time")
                    if counts["submissions"] > counts["dispatches"] or counts["errors"] > counts["owners"]:
                        raise ValueError("inconsistent task counters")
                    if counts["workspace_creations"] + counts["workspace_reuses"] > counts["owners"]:
                        raise ValueError("more workspaces than ready owners")
                    shapes = record.get("submitted_operator_shapes_rows_coordinates", [])
                    if not isinstance(shapes, list):
                        raise ValueError("invalid operator shapes")
                    for shape in shapes:
                        if not isinstance(shape, list) or len(shape) != 2:
                            raise ValueError("invalid operator shape")
                        rows, coordinates = (nonnegative(v, "operator dimension", True) for v in shape)
                        if not rows or not coordinates:
                            raise ValueError("empty operator shape")
                except (ValueError, OverflowError) as error:
                    raise ValueError(f"{path}:{line_number}: {error}") from error
                calls += 1
                if current_frame is not None:
                    current_frame["profiled_calls"] += 1
                    current_frame["dispatches"] += counts["dispatches"]
                    current_frame["errors"] += counts["errors"]
                    current_frame["profile_total_wall_ms"] += total
                    current_frame["profile_wait_wall_ms"] += values["wait_wall_ms"]
                    current_frame["staging_payload_bytes_not_bus_traffic"] += payload
                    if workspace is not None:
                        current_frame["complete_workspace_payload_bytes_over_reported_calls"] += workspace
                        current_frame["workspace_payload_reported_calls"] += 1
                for name, value in seeds.items():
                    seed_counts[name]["instances"] += value
                    seed_counts[name]["reported_calls"] += 1
                payload_bytes += payload
                if workspace is not None:
                    workspace_bytes += workspace
                    workspace_profiled_calls += 1
                for shape in shapes:
                    key = tuple(shape)
                    shape_counts[key] = shape_counts.get(key, 0) + 1
                slowest_batches.append({
                    "log": str(path), "line": line_number,
                    "owners": counts["owners"], "errors": counts["errors"],
                    "packed_operator_shapes_rows_coordinates": shapes,
                    "total_wall_ms": total, "wait_wall_ms": values["wait_wall_ms"],
                })
                slowest_batches.sort(key=lambda batch: batch["total_wall_ms"], reverse=True)
                del slowest_batches[10:]
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
        "coupled_frames": list(frames.values()),
        "owner_instances_not_unique_operators": counters.pop("owners"),
        "counters": counters,
        "staging_payload_bytes_not_bus_traffic": payload_bytes,
        "complete_workspace_payload": {
            "bytes_over_profiled_calls_not_peak_memory": workspace_bytes,
            "profiled_calls": workspace_profiled_calls,
            "unreported_calls": calls - workspace_profiled_calls,
            "scope": "Complete packed GPU workspace payload; legacy logs remain unknown, never inferred from readback size.",
        },
        "warm_start": {
            "counters": {name: {**value, "unreported_calls": calls - value["reported_calls"]}
                         for name, value in seed_counts.items()},
            "scope": "Input and uploaded seed instances, not unique operators or successful solver iterations. Missing legacy fields remain unknown.",
        },
        "packed_operator_shape_instances": [
            {"rows": rows, "coordinates": coordinates, "instances": count}
            for (rows, coordinates), count in sorted(shape_counts.items())
        ],
        "slowest_profiled_batches": slowest_batches,
        "batch_attribution_limit": "Batch wall time includes all listed operators and shared queue wait; it is not an individual operator timing.",
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
