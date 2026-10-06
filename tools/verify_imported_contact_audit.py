#!/usr/bin/env python3
"""Validate a completed imported-contact audit, without manufacturing completion.

This checks published aggregate evidence, not the physics independently. Per-frame
balances and CCD are checked by the Rust qualification that emits the audit.
"""
import argparse
import json
import math
import re
from pathlib import Path


def number(value, name):
    if type(value) not in (int, float):
        raise ValueError(f"{name}: expected a finite number")
    try:
        result = float(value)
    except OverflowError as error:
        raise ValueError(f"{name}: number exceeds finite range") from error
    if not math.isfinite(result):
        raise ValueError(f"{name}: expected a finite number")
    return result


def verify(audit, steps=480, bodies=4):
    if not isinstance(audit, dict):
        raise ValueError("audit must be an object")
    if audit.get("scope") != "independent nominal-frame mechanical energy audit":
        raise ValueError("wrong audit scope")
    if type(audit.get("steps")) is not int or audit["steps"] != steps:
        raise ValueError("incomplete or unexpected step count")
    if audit.get("reported_defect_subtracted") is not False:
        raise ValueError("independent frame audit must not subtract reported defect")
    budget = number(audit.get("requested_budget_per_body_j"), "budget")
    requested = steps * 1e-5
    if abs(budget - requested) > 64 * math.ulp(requested):
        raise ValueError("budget differs from the original 1e-5 J per frame")
    arrays = [audit.get(k) for k in (
        "absolute_frame_error_j", "rounding_allowance_j", "final_energy_receipts")]
    if any(not isinstance(a, list) or len(a) != bodies for a in arrays):
        raise ValueError("body count or audit arrays changed")
    results = []
    for index, (error, rounding, receipt) in enumerate(zip(*arrays)):
        error = number(error, f"body {index} absolute error")
        rounding = number(rounding, f"body {index} rounding")
        if error < 0 or rounding < 0 or error > budget + rounding:
            raise ValueError(f"body {index}: aggregate independent error exceeds budget")
        # Per-frame rounding cannot be reconstructed from this aggregate file.
        # Preserve the emitter's allowance; do not invent a new physics tolerance.
        if not isinstance(receipt, list) or len(receipt) != 4:
            raise ValueError(f"body {index}: invalid receipt shape")
        receipt = [number(x, f"body {index} receipt") for x in receipt]
        try:
            raw = math.fsum([receipt[0], -receipt[1], receipt[2]])
        except OverflowError as error:
            raise ValueError(f"body {index}: final balance overflow") from error
        ledger = raw - receipt[3]
        if not math.isfinite(ledger) or abs(ledger) >= 1e-8:
            raise ValueError(f"body {index}: final reported ledger does not close")
        results.append({"body": index, "absolute_frame_error_j": error,
                        "requested_budget_j": budget, "rounding_allowance_j": rounding,
                        "final_unadjusted_balance_j": raw,
                        "reported_ledger_residual_j": ledger})
    return {"scope": "published aggregate audit validation", "steps": steps,
            "bodies": results, "independent_frame_reported_defect_subtracted": False}


def read_audit(path):
    content = path.read_text()
    if path.suffix == ".json":
        return json.loads(content)
    if "test result: FAILED" in content:
        raise ValueError("qualification test ended with failure")
    prefix = "WIDE_IMPORTED_ENERGY_AUDIT "
    records = [line[len(prefix):] for line in content.splitlines() if line.startswith(prefix)]
    if len(records) != 1:
        raise ValueError("expected exactly one completed audit; pending or ambiguous log")
    audit_position = content.index(prefix)
    tail = content[audit_position:]
    if "test result: FAILED" in content or not re.search(
            r"test result: ok\. [1-9][0-9]* passed; 0 failed;", tail):
        raise ValueError("audit emitted without confirmed successful test completion")
    return json.loads(records[0])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path)
    parser.add_argument("--steps", type=int, default=480)
    parser.add_argument("--bodies", type=int, default=4)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if args.steps <= 0 or args.bodies <= 0:
        parser.error("steps and bodies must be positive")
    try:
        report = verify(read_audit(args.input), args.steps, args.bodies)
    except (OSError, ValueError, TypeError) as error:
        parser.exit(2, f"Audit not verified: {error}\n")
    text = json.dumps(report, indent=2, allow_nan=False) + "\n"
    if args.output:
        args.output.write_text(text)
    else:
        print(text, end="")


if __name__ == "__main__":
    main()
