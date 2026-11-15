#!/usr/bin/env python3
"""Exercise the voxy-street stdio MCP server.

Without --live only the protocol is checked (no GPU, no scene). With --live the scene is
started and camera, capture, motion and stats tools are run; images go to --output.
"""
import argparse
import base64
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile", default="release")
    parser.add_argument("--live", action="store_true")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    exe = ROOT / "target" / args.profile / "examples" / ("street_mcp.exe" if sys.platform == "win32" else "street_mcp")
    process = subprocess.Popen([str(exe)], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, encoding="utf-8")
    sequence = 0

    def request(method, params=None):
        nonlocal sequence
        sequence += 1
        payload = dict(jsonrpc="2.0", id=sequence, method=method)
        if params is not None:
            payload["params"] = params
        process.stdin.write(json.dumps(payload) + "\n")
        process.stdin.flush()
        response = json.loads(process.stdout.readline())
        assert response["id"] == sequence, response
        return response

    def call(name, **arguments):
        result = request("tools/call", dict(name=name, arguments=arguments))["result"]
        texts = [c["text"] for c in result["content"] if c["type"] == "text"]
        images = [base64.b64decode(c["data"]) for c in result["content"] if c["type"] == "image"]
        assert not result["isError"], texts
        return json.loads(texts[0]), images

    init = request("initialize", dict(protocolVersion="2025-06-18"))["result"]
    assert init["serverInfo"]["name"] == "voxy-street" and init["instructions"]
    process.stdin.write(json.dumps(dict(jsonrpc="2.0", method="notifications/initialized")) + "\n")
    names = {t["name"] for t in request("tools/list")["result"]["tools"]}
    assert names == {"street_open", "street_close", "street_set", "street_capture", "street_motion", "street_stats"}, names
    assert request("tools/call", dict(name="nope", arguments={}))["result"]["isError"]
    assert request("bogus")["error"]["code"] == -32601
    print("protocol ok")
    if args.live:
        output = args.output.resolve()
        output.mkdir(parents=True, exist_ok=True)
        tree = dict(eye=[-3.0, 1.7, -9.0], look=[-5.9, 3.6, -5.0])
        print("open", call("street_open")[0])
        meta, images = call("street_capture", frames=4, **tree)
        assert len(images) == 2 and meta["frames"] == 4, meta
        (output / "capture.png").write_bytes(images[0])
        (output / "heat.png").write_bytes(images[1])
        for speed in (0, 8, 15):
            meta, images = call("street_motion", wind_speed=speed, settle_ms=1500, distance_m=5, **tree)
            (output / f"motion-{speed}.png").write_bytes(images[0])
            keys = ("moving_fraction_of_textured", "amplitude_px_median", "amplitude_px_p90", "amplitude_m_p90",
                    "frequency_hz_median", "neighbour_coherence_mean", "seconds")
            print("wind", speed, {k: (round(meta[k], 3) if isinstance(meta.get(k), float) else meta.get(k)) for k in keys})
        stats, _ = call("street_stats", seconds=3)
        print("stats", {k: stats[k] for k in ("mean_fps", "p50_ms", "p99_ms", "max_ms", "over_120hz_frames", "gpu_scene_mean_ms")})
        call("street_close")
    process.stdin.close()
    assert process.wait(timeout=30) == 0
    print("ok")


if __name__ == "__main__":
    main()
