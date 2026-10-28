"""Exercise the public Rust game example without a window or source fallback."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

binary = Path(sys.argv[1]).resolve(strict=True)
evidence = Path(__file__).resolve().parent
repo = evidence.parent.parent
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

with tempfile.TemporaryDirectory(prefix="voxy-host-cli-") as temporary:
    root = Path(temporary) / "source"
    shutil.copytree(repo / "examples" / "host-game", root)
    project = root / "project.voxy"
    before = run("--check", project)
    assert "HOST MOVE END fixed=120 x=1.999999" in before or "HOST MOVE END fixed=120 x=2.000000" in before
    first_state = state(before)
    assert abs(first_state["objects"][0]["translation"][0] - 2.0) < 0.0001
    package = Path(temporary) / "host.vpak"
    build = run("--build", project, package)
    assert "HOST MOVE END fixed=0 x=0.000000" in build
    package_bytes = package.read_bytes()
    # Invalid game setup must fail the build without publishing an output file.
    scene_path = root / "main.scene.json"
    valid_scene = scene_path.read_bytes()
    invalid_scene = json.loads(valid_scene)
    invalid_scene["objects"][0]["components"]["sample.move.v1"]["speed"] = 100
    scene_path.write_text(json.dumps(invalid_scene))
    rejected = Path(temporary) / "rejected.vpak"
    run("--build", project, rejected, success=False)
    assert not rejected.exists()
    scene_path.write_bytes(valid_scene)
    shutil.rmtree(root)
    assert not project.exists()
    after = run("--package-check", package)
    assert state(after) == first_state
    assert "HOST MOVE END fixed=120" in after
    artifact = repo / "artifacts" / "project-host-2026-10-26"
    artifact.mkdir(parents=True, exist_ok=True)
    saved = artifact / "host-game.vpak"
    if saved.exists():
        assert saved.read_bytes() == package_bytes, "preserve differing artifact"
    else:
        with saved.open("xb") as output:
            output.write(package_bytes)

proof = {"status": "public_game_host_cli_pass", "runs": runs,
    "binary": str(binary), "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
    "package_sha256": hashlib.sha256(package_bytes).hexdigest(),
    "source_tree_removed_before_package_launch": True,
    "project_and_package_state_equal": True, "simulation_fixed_ticks": 120,
    "invalid_game_setup_rejected_without_output": True,
    "native_window_verified": False, "fps_qualified": False}
(evidence / "game-host-cli-proof.json").write_text(json.dumps(proof, indent=2) + "\n")
print("GAME HOST CLI PASS: public example, identical motion, source-free package, rejected invalid setup")
