"""Native acceptance test. Build asset_window before running this script."""
import argparse
import json
import pathlib
import re
import selectors
import subprocess
import tempfile
import time


def main():
    repo = pathlib.Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=pathlib.Path, default=repo / "target/debug/examples/asset_window")
    parser.add_argument("--manifest", action="store_true", help="Verify stable logical identity across source rename")
    parser.add_argument("--app", action="store_true", help="Exercise the main voxy_app model modes")
    parser.add_argument("--editor-checks", action="store_true", help="Require native authoring deletion and instance acceptance checks")
    parser.add_argument("--scene-test", action="store_true", help="Verify standalone editor scene file round trip")
    args = parser.parse_args()
    assert not (args.scene_test and args.app), "scene-test uses the standalone editor"
    original = (repo / "crates/voxy_render/examples/assets/quad.obj").read_text()
    with tempfile.TemporaryDirectory(prefix="voxy-window-reload-") as directory:
        model = pathlib.Path(directory) / "quad.obj"
        model.write_text(original)
        manifest = pathlib.Path(directory) / "assets.json"
        if args.manifest:
            manifest.write_text(json.dumps({"version": 1, "assets": [{"asset": "logical-quad", "source": model.name}]}))
            command = [str(args.binary), "--model-manifest" if args.app else "--manifest", str(manifest), "logical-quad", "--smoke"]
        else:
            command = [str(args.binary), *(["--model"] if args.app else []), str(model), "--smoke"]
        if args.scene_test:
            command.extend(["--scene", str(pathlib.Path(directory) / "saved.scene.json")])
        process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1)
        selector = selectors.DefaultSelector()
        selector.register(process.stdout, selectors.EVENT_READ)
        stage = 0
        failed_frame = None
        recovered_frame = None
        transcript = []
        deadline = time.monotonic() + 25
        try:
            while process.poll() is None and time.monotonic() < deadline:
                if not selector.select(1):
                    continue
                line = process.stdout.readline()
                print(line, end="", flush=True)
                transcript.append(line)
                if "MODEL FAILED" in line and stage >= 1:
                    assert "last_good=true" in line, "relocation failure lost previous geometry"
                if args.manifest and "MODEL PUBLISHED" in line:
                    assert "asset=logical-quad" in line, "logical identity changed during relocation"
                if "MODEL PUBLISHED" in line and stage == 0:
                    model.write_text("not valid OBJ")
                    stage = 1
                elif "MODEL FAILED" in line and stage == 1:
                    assert "last_good=true" in line, "failed reload lost last-good model"
                    failed_frame = int(re.search(r"frames=(\d+)", line)[1])
                    time.sleep(0.3)  # Leave the window rendering while the source is invalid.
                    if args.manifest:
                        renamed = model.with_name("renamed-quad.obj")
                        model.rename(renamed)
                        model = renamed
                        replacement = manifest.with_suffix(".next.json")
                        replacement.write_text(json.dumps({"version": 1, "assets": [{"asset": "logical-quad", "source": model.name}]}))
                        replacement.replace(manifest)
                        time.sleep(0.35)  # The newly mapped source is still invalid during this scan.
                    for left, right in [(-0.25, 0.75), (-0.1, 0.9), (0.0, 1.0)]:
                        model.write_text(original.replace("v -0.5", f"v {left}").replace("v 0.5", f"v {right}"))
                        time.sleep(0.03)
                    stage = 2
                elif "MODEL PUBLISHED" in line and stage >= 2:
                    left = float(re.search(r"first_x=([^\s]+)", line)[1])
                    if stage == 3:
                        assert left == 0.0, "late completion replaced the final geometry"
                    if left == 0.0:
                        recovered_frame = int(re.search(r"frames=(\d+)", line)[1])
                        stage = 3
            if process.poll() is None:
                process.terminate()
            remaining = process.communicate(timeout=5)[0]
            print(remaining, end="")
            transcript.append(remaining)
            assert process.returncode == 0, f"native process exited {process.returncode}"
            assert stage == 3 and recovered_frame > failed_frame, "edit burst did not recover while native frames advanced"
            assert "ASSET WINDOW PASS" in "".join(transcript), "native acceptance checks did not complete"
            if args.scene_test:
                assert "MODEL SCENE PASS" in "".join(transcript), "scene persistence acceptance checks missing"
                saved = json.loads((pathlib.Path(directory) / "saved.scene.json").read_text())
                assert len(saved["objects"]) == 2, "saved instance count changed"
            if args.editor_checks:
                assert "MODEL PLAY PASS" in "".join(transcript), "native fixed-step Play/Stop checks missing"
                assert "MODEL DRAG PASS" in "".join(transcript), "drag transaction checks missing"
                assert "MODEL DELETE PASS" in "".join(transcript), "deletion/undo checks missing"
                assert "MODEL INSTANCES PASS: 2" in "".join(transcript), "instance checks missing"
            print("NATIVE RELOAD TEST PASS: corrupt source retained model, rapid edits converged to final geometry, rendering continued")
            if args.manifest:
                print("MANIFEST RELOCATION PASS: native resource identity stayed logical-quad after real rename and manifest replacement")
        finally:
            selector.close()
            if process.poll() is None:
                process.kill()
                process.wait(timeout=5)


if __name__ == "__main__":
    main()
