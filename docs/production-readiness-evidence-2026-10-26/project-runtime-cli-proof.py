"""Run headless project/package acceptance against the supplied native binary.
Uses a private temporary project, removes its source tree before packaged launch,
and saves outputs and binary identity beside this script. Does not open a window.
"""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

binary = Path(sys.argv[1]).resolve(strict=True)
evidence = Path(__file__).resolve().parent
runs = []

def run(*args, success=True):
    result = subprocess.run([str(binary), *map(str, args)], cwd="/", text=True,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=180)
    runs.append({"args": list(map(str, args)), "exit_code": result.returncode,
                 "stdout": result.stdout, "stderr": result.stderr})
    assert (result.returncode == 0) == success, runs[-1]
    return result.stdout

def state(output):
    return json.loads(next(line[len("GAME STATE "):] for line in output.splitlines()
                           if line.startswith("GAME STATE ")))

with tempfile.TemporaryDirectory(prefix="voxy-project-cli-") as temporary:
    root = Path(temporary) / "source"
    descriptor = root / "project.voxy"
    run("--create-project", root, "Project motion CLI")
    (root / "triangle.obj").write_text("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n")
    (root / "assets.json").write_text(json.dumps({"version": 1, "assets": [
        {"asset": "triangle", "source": "triangle.obj"}]}))
    scene = {"version": 1, "objects": [{"id": "triangle", "parent": None,
        "name": "Rotating triangle", "active": True, "translation": [0, 0, 0],
        "rotation": [0, 0, 0, 1], "scale": [1, 1, 1], "components": {
            "editor.model.v1": "triangle", "game.angular-motion.v1": {
                "axis": [0, 0, 1], "radians_per_second": 2.0}}}]}
    (root / "main.scene.json").write_text(json.dumps(scene))
    run("--validate-project", descriptor)
    before = run("--project", descriptor, "--game-check")
    assert "GAME CHECK PASS ticks=120 objects=1" in before
    game_state = state(before)
    assert abs(abs(game_state["objects"][0]["rotation"][2]) - 0.9092974268) < 0.001
    package = Path(temporary) / "game.vpak"
    run("--build-project", descriptor, package)
    original = package.read_bytes()
    run("--build-project", descriptor, package, success=False)
    assert package.read_bytes() == original
    # Retain the exact authored sample and built package for review.
    sample = evidence.parent.parent / "examples" / "project-motion"
    if sample.exists():
        for file in root.iterdir():
            assert (sample / file.name).read_bytes() == file.read_bytes(), "sample already differs"
    else:
        shutil.copytree(root, sample)
    artifact = evidence.parent.parent / "artifacts" / "project-runtime-2026-10-26"
    artifact.mkdir(parents=True, exist_ok=True)
    (artifact / "motion.vpak").write_bytes(original)
    shutil.rmtree(root)
    assert not descriptor.exists()
    after = run("--package", package, "--game-check")
    assert "GAME CHECK PASS ticks=120 objects=1" in after
    assert state(after) == game_state
    package_sha256 = hashlib.sha256(original).hexdigest()

proof = {"status": "project_and_package_cli_headless_pass", "runs": runs,
    "binary": str(binary), "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
    "package_sha256": package_sha256, "source_tree_removed_before_package_launch": True,
    "project_and_package_state_equal": True, "simulation_ticks": 120,
    "native_window_verified": False, "fps_qualified": False}
(evidence / "project-runtime-cli-proof.json").write_text(json.dumps(proof, indent=2) + "\n")
print("PROJECT/PACKAGE CLI PASS: source tree removed; 120 ticks; identical motion; output preserved")
