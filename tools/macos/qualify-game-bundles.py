#!/usr/bin/env python3
"""Build, relocate and launch three Voxy project bundles on macOS."""
import argparse
import hashlib
import json
import os
import plistlib
import re
from pathlib import Path
import shutil
import subprocess
import tempfile

PACKAGER = Path(__file__).with_name("package-game.sh").resolve()


def run(*args, success=True, timeout=60, env=None):
    result = subprocess.run(args, cwd="/", text=True, capture_output=True, timeout=timeout, env=env)
    if (result.returncode == 0) != success:
        raise RuntimeError(f"{args[0]} exited with {result.returncode}\n{result.stdout}{result.stderr}")
    return result.stdout + result.stderr


def write_json(path, value):
    path.write_text(json.dumps(value), encoding="utf-8")


def object_data(identifier, components, translation=(0, 0, 0)):
    return dict(id=identifier, parent=None, name=identifier, active=True, translation=translation,
                rotation=[0, 0, 0, 1], scale=[1, 1, 1], components=components)


def configure(root, kind):
    motion = {"game.angular-motion.v1": {"axis": [0, 0, 1], "radians_per_second": 2}}
    if kind == "turntable":
        (root / "mesh.obj").write_text("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n")
        write_json(root / "assets.json", dict(version=1, assets=[dict(asset="mesh", source="mesh.obj")]))
        scene = dict(version=1, objects=[object_data("turntable", dict(motion, **{"editor.model.v1": "mesh"}))])
    elif kind == "character":
        scene = dict(version=1, objects=[
            object_data("player", {"game.character.v1": dict(
                half_extents=[0.2, 0.2, 0.2], speed=2, gravity=-9.8, jump_speed=3)}, (0, 1, 0)),
            object_data("floor", {"game.box.v1": dict(half_extents=[5, 0.25, 5])}, (0, -0.25, 0)),
        ])
    else:
        instance = lambda identifier, asset: dict(id=identifier, asset=asset, parent=None, overrides={})
        write_json(root / "leaf.prefab", dict(version=1, objects=[object_data("rotor", motion)], instances=[]))
        write_json(root / "nested.prefab", dict(version=1, objects=[], instances=[instance("leaf", "leaf")]))
        write_json(root / "assets.json", dict(version=1, assets=[
            dict(asset="leaf", source="leaf.prefab"), dict(asset="nested", source="nested.prefab"),
        ]))
        scene = dict(version=1, objects=[], instances=[instance("first", "nested"), instance("second", "nested")])
    write_json(root / "main.scene.json", scene)


def qualify(binary, player):
    digest = hashlib.sha256(binary.read_bytes()).hexdigest()
    player_digest = hashlib.sha256(player.read_bytes()).hexdigest()
    results = {}
    with tempfile.TemporaryDirectory(prefix="voxy-game-bundle-") as folder:
        temporary = Path(folder)
        relocated = temporary / "Relocated apps"
        relocated.mkdir()
        for kind in ("turntable", "character", "prefab"):
            source = temporary / f"{kind} source"
            project = source / "project.voxy"
            package = temporary / f"{kind}.vpak"
            bundle = temporary / f"{kind}.app"
            log_path = temporary / f"{kind}.log"
            error_path = temporary / f"{kind}.err"
            name = f"Voxy & {kind}"
            identifier = f"io.voxy.qualification.{kind}"
            run(binary, "--create-project", source, name)
            configure(source, kind)
            project_check = run(binary, "--project", project, "--game-check")
            run(binary, "--build-project", project, package)
            build_args = (PACKAGER, player, package, bundle, "--direct", "--sign", "-",
                          "--name", name, "--bundle-id", identifier)
            run(*build_args)
            run(*build_args, success=False)
            info = plistlib.loads((bundle / "Contents/Info.plist").read_bytes())
            assert info["CFBundleDisplayName"] == name and info["CFBundleIdentifier"] == identifier
            package_hash = hashlib.sha256(package.read_bytes()).hexdigest()
            shutil.rmtree(source)
            package.unlink()
            app = relocated / bundle.name
            bundle.rename(app)
            embedded = app / "Contents/Resources/game.vpak"
            assert hashlib.sha256(embedded.read_bytes()).hexdigest() == package_hash
            assert info["CFBundleExecutable"] == "voxy_game"
            run("/usr/bin/codesign", "--verify", "--strict", app)
            signature = run("/usr/bin/codesign", "-dv", "--verbose=4", app)
            assert "Signature=adhoc" in signature and "(adhoc,runtime)" in signature
            unavailable_temp = temporary / "unavailable-temp"
            check = run(app / "Contents/MacOS/voxy_game", "--embedded-game-check",
                        env=os.environ | {"TMPDIR": str(unavailable_temp)})
            assert not unavailable_temp.exists()
            strip_timing = lambda log: re.sub(r" publication_us=\d+", "", log)
            assert strip_timing(project_check) == strip_timing(check), (project_check, check)
            assert "GAME CHECK PASS ticks=120 " in check
            run("/usr/bin/open", "-W", "-n", app, "--stdout", log_path, "--stderr", error_path,
                "--args", "--embedded-game-native-smoke")
            log = log_path.read_text() + error_path.read_text()
            if "GAME NATIVE PASS" not in log:
                raise RuntimeError(f"{kind} did not present native game frames\n{log}")
            with embedded.open("ab") as file:
                file.write(b"tamper")
            run("/usr/bin/codesign", "--verify", "--strict", app, success=False)
            results[kind] = dict(bundle_id=identifier, name=name, package_sha256=package_hash,
                                 game_check=check, native_log=log, signature=signature,
                                 changed_resource_rejected=True)
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == digest
    assert hashlib.sha256(player.read_bytes()).hexdigest() == player_digest
    return dict(binary_sha256=digest, player_sha256=player_digest, bundles=results)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--player", type=Path)
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    player = (args.player or binary).resolve(strict=True)
    print(json.dumps(qualify(binary, player), indent=2))
