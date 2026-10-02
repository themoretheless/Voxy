"""Run pinned meshoptimizer output through the actual Voxy OBJ certificate example.

Build first: cargo build -p voxy_render --example certify_obj_lod
The report identifies the supplied executable; it does not infer its source revision.
"""
import argparse
import csv
import hashlib
import io
import json
import math
from pathlib import Path
import subprocess
import sys
import tempfile


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def fields(line):
    return dict(item.split("=", 1) for item in line.split())


def main():
    root = Path(__file__).resolve().parents[1]
    research = root / "docs/engine-research/mechanisms/godot-lod"
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=root / "target/debug/examples/certify_obj_lod")
    args = parser.parse_args()
    binary = args.binary.resolve()
    require(binary.is_file(), "build the certify_obj_lod example first")
    manifest = json.loads((research / "sources.json").read_text())
    for source in manifest["sources"]:
        digest = hashlib.sha256((research / source["file"]).read_bytes()).hexdigest()
        require(digest == source["sha256"], "pinned source digest mismatch")
    results = []
    with tempfile.TemporaryDirectory(prefix="voxy-meshopt-cert-") as directory:
        work = Path(directory)
        generated = subprocess.run(
            [sys.executable, str(research / "run_importer_probe.py"), "--export-dir", str(work)],
            text=True, capture_output=True, check=True,
        )
        rows = list(csv.DictReader(io.StringIO(generated.stdout)))
        require(len(rows) > 1, "upstream did not produce a LOD chain")
        hashes = {path.name: hashlib.sha256(path.read_bytes()).hexdigest()
                  for path in sorted(work.glob("*.obj"))}
        for row in rows:
            level = int(row["level"])
            for depth in (1, 3):
                checked = subprocess.run(
                    [str(binary), str(work / "base.obj"), str(work / f"lod-{level}.obj"),
                     "1000000", str(depth), "indexed"],
                    text=True, capture_output=True, check=True,
                )
                geometry = fields(checked.stdout.strip())
                search = fields(checked.stderr.strip())
                require("archive_bytes" in search, "rebuild example with archive re-verification")
                archive_bytes = int(search.pop("archive_bytes"))
                require(0 < archive_bytes <= 64 * 1024 * 1024, "archive byte budget exceeded")
                bound = float(geometry["geometric_bound"])
                source_triangles = int(geometry["base_triangles"])
                triangles = int(geometry["variant_triangles"])
                require(source_triangles * 3 == int(rows[0]["input_indices"]), "original base count changed")
                require(triangles * 3 == int(row["output_indices"]), "imported index count changed")
                require(math.isfinite(bound) and bound >= 0, "invalid verified bound")
                require(int(search["triangle_tests"]) <= 1000000, "triangle-test budget exceeded")
                require(int(search["node_visits"]) <= 16000000, "node budget exceeded")
                require(int(search["output_cells"]) == (source_triangles + triangles) * 4**depth,
                        "incomplete subdivision output")
                results.append({
                    "level": level, "depth": depth, "base_triangles": source_triangles,
                    "variant_triangles": triangles, "verified_bound": bound,
                    "optimizer_metric": float(row["object_metric"]),
                    "logical_index_bytes": int(geometry["logical_index_bytes"]),
                    "archive_bytes": archive_bytes,
                    "archive_reverification": True,
                    "search": {key: int(value) for key, value in search.items()},
                })
        retry = subprocess.run(
            [sys.executable, str(research / "run_importer_probe.py"), "--export-dir", str(work)],
            text=True, capture_output=True,
        )
        require(retry.returncode != 0, "export unexpectedly overwrote existing meshes")
        require(hashes == {path.name: hashlib.sha256(path.read_bytes()).hexdigest()
                           for path in sorted(work.glob("*.obj"))}, "rejected export changed meshes")
    print(json.dumps({
        "repository": manifest["repository"], "commit": manifest["commit"],
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "generated_mesh_sha256": hashes, "results": results,
        "overwrite_rejected_without_changes": True,
        "scope": "pinned offline simplifier and actual OBJ certificate executable; no native presentation or universal metric equivalence",
    }, indent=2))


if __name__ == "__main__":
    main()
