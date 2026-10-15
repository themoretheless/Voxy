#!/usr/bin/env python3
"""Check deterministic full-polygon AA captures; differences are not quality scores."""
import argparse
import json
from pathlib import Path
import numpy as np
from PIL import Image


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("root", type=Path, help="contains off, temporal and metalfx capture directories")
    args = parser.parse_args()
    results = {}
    images = {}
    for mode in ("off", "temporal", "metalfx"):
        directory = args.root / mode
        report = json.loads((directory / "report.json").read_text())
        assert report["frames"] == 160
        assert report["windowless"] is True
        assert report["rendered_triangles"] == report["collision_triangles"]
        assert report["resolution"] == [1920, 1080]
        assert report["render_headroom"] == 2.0
        assert report["source_simplified"] is False
        assert report["completed_frame_timings"] is True
        assert report["collision_triangles"] > 100_000
        assert "Metal" in report["adapter"]
        frames = {}
        for path in sorted(directory.glob("frame-*.png")):
            frame = int(path.stem.split("-")[1])
            rgb = np.asarray(Image.open(path).convert("RGB"), dtype=np.float32)
            assert rgb.shape == (1080, 1920, 3)
            assert np.isfinite(rgb).all()
            assert rgb.std() > 5, f"blank output: {path}"
            frames[frame] = rgb
            hdr_path = path.with_suffix(".hdr.json")
            if hdr_path.exists():
                hdr = json.loads(hdr_path.read_text())
                assert hdr["finite"] is True
                assert hdr["pixels"] == 1920 * 1080
        assert len(frames) == 19
        static = np.stack([frames[i] for i in range(24, 33)])
        timing = np.asarray(report["frame_ms"][16:])
        assert np.isfinite(timing).all() and (timing > 0).all()
        results[mode] = {
            "mode": report["antialiasing"],
            "collision_triangles": report["collision_triangles"],
            "frames": report["frames"],
            "captures": len(frames),
            "completed_frame_median_ms": float(np.median(timing)),
            "completed_frame_p95_ms": float(np.percentile(timing, 95)),
            "static_frame_mean_absolute_delta_8bit": float(np.abs(np.diff(static, axis=0)).mean()),
        }
        images[mode] = {i: frames[i] for i in (32, 64, 96, 97, 129, 160)}
    assert len({results[m]["collision_triangles"] for m in results}) == 1
    assert results["temporal"]["static_frame_mean_absolute_delta_8bit"] < 1.0, "TAA static grid must stabilize"
    for mode in ("temporal", "metalfx"):
        differences = {
            str(i): float(np.abs(images[mode][i] - images["off"][i]).mean())
            for i in images[mode]
        }
        results[mode]["mean_absolute_difference_from_off_8bit"] = differences
        # A broad regression guard against displaying stale history after a cut.
        # Subpixel coverage differs from Off; this does not certify all ghosting.
        assert differences["97"] < 8, f"stale/incorrect camera-cut output in {mode}"
        assert differences["129"] < 8, f"stale/incorrect explicit-reset output in {mode}"
    output = {
        "passed": True,
        "scope": "native-resolution immutable polygon scene; static, rotation, translation, cut, reset",
        "limits": "single-run timing; pixel differences are not quality scores; no animated-object/reactive-mask certification",
        "results": results,
    }
    (args.root / "qualification.json").write_text(json.dumps(output, indent=2) + "\n")
    print(json.dumps(output, indent=2))


if __name__ == "__main__":
    main()
