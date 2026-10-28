#!/usr/bin/env python3
"""Exercise real stdio MCP and Voxy GPU renders in a private project copy."""
import argparse
import base64
import hashlib
import json
from pathlib import Path
import selectors
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    project = output / "project"
    project.mkdir()
    source = ROOT / "examples/img2voxy-airpods-pro-3-final"
    for path in source.iterdir():
        if path.suffix == ".obj" or path.name in ("project.voxy", "assets.json", "main.scene.json"):
            shutil.copy2(path, project / path.name)
    transcript = []
    with (output / "server-stderr.log").open("w") as stderr:
        process = subprocess.Popen(
            [str(ROOT / "target/debug/examples/scene_mcp"), str(project / "project.voxy")],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=stderr,
        )
        sequence = 0

        def request(method, params=None):
            nonlocal sequence
            sequence += 1
            payload = dict(jsonrpc="2.0", id=sequence, method=method)
            if params is not None:
                payload["params"] = params
            process.stdin.write((json.dumps(payload) + "\n").encode())
            process.stdin.flush()
            with selectors.DefaultSelector() as selector:
                selector.register(process.stdout, selectors.EVENT_READ)
                if not selector.select(60):
                    raise TimeoutError(method)
            line = process.stdout.readline()
            response = json.loads(line)
            assert response["id"] == sequence, response
            sanitized = json.loads(line)
            for item in sanitized.get("result", {}).get("content", []):
                if item.get("type") == "image":
                    data = base64.b64decode(item.pop("data"), validate=True)
                    item.update(bytes=len(data), sha256=hashlib.sha256(data).hexdigest())
            transcript.append(dict(request=payload, response=sanitized))
            return response

        def call(name, arguments=None, error=False):
            response = request("tools/call", dict(name=name, arguments=arguments or {}))
            assert "error" not in response, response
            result = response["result"]
            assert bool(result.get("isError")) == error, result
            return result

        def state():
            return call("scene_get")["structuredContent"]

        def action(name, **extra):
            return call(name, dict(expected_revision=state()["revision"], **extra))

        def render(name):
            result = action("scene_render")
            data = base64.b64decode(next(c["data"] for c in result["content"] if c["type"] == "image"), validate=True)
            assert data[:8] == b"\x89PNG\r\n\x1a\n"
            assert int.from_bytes(data[16:20], "big") == 1024
            assert int.from_bytes(data[20:24], "big") == 1024
            (output / name).write_bytes(data)
            metadata = result["structuredContent"]
            assert metadata["sceneHash"] == state()["sceneHash"]
            assert metadata["meshHashes"]
            return data

        try:
            assert request("tools/list")["error"]["code"] == -32000
            assert request("initialize", dict(protocolVersion="2025-06-18", capabilities={}, clientInfo=dict(name="voxy-qualification", version="1")))["result"]["protocolVersion"] == "2025-06-18"
            process.stdin.write(b'{"jsonrpc":"2.0","method":"notifications/initialized"}\n')
            process.stdin.flush()
            assert len(request("tools/list")["result"]["tools"]) == 6
            initial = state()
            baseline = render("baseline.png")
            edits = [
                dict(op="material", node_id="case-body", tint=[0.18, 0.35, 0.9, 1], lit=True),
                dict(op="camera", node_id="reconstruction-camera", target=[0, .046, 0], distance=.16, yaw=.75, pitch=.23, perspective=True),
                dict(op="transform", node_id="right-earbud", translation=[.041, .080, 0]),
            ]
            action("scene_apply", edits=edits)
            assert state()["dirty"]
            edited = render("edited.png")
            assert baseline != edited
            action("scene_undo")
            assert state()["scene"] == initial["scene"]
            assert render("restored.png") == baseline
            action("scene_redo")
            assert render("redone.png") == edited
            action("scene_undo")
            before = state()
            call("scene_apply", dict(expected_revision=before["revision"], edits=[edits[2], dict(op="material", node_id="case-body", tint=[2, 0, 0, 1], lit=True)]), error=True)
            assert state() == before
            call("scene_apply", dict(expected_revision=0, edits=edits), error=True)
            group = dict(id="mcp-test-group", parent="airpods-pro-3", name="MCP group", active=True, translation=[0, 0, 0], rotation=[0, 0, 0, 1], scale=[1, 1, 1], components={})
            action("scene_apply", edits=[dict(op="create", object=group), dict(op="reparent", node_id="left-earbud", parent="mcp-test-group"), dict(op="active", node_id="right-earbud", active=False)])
            action("scene_apply", edits=[dict(op="remove", node_id="mcp-test-group")])
            assert not any(o["id"].startswith("left-") for o in state()["scene"]["objects"])
            action("scene_undo")
            action("scene_undo")
            assert state()["scene"] == initial["scene"]
            action("scene_apply", edits=edits)
            action("scene_save")
            assert not state()["dirty"]
            scene_path = project / "main.scene.json"
            saved = scene_path.read_bytes()
            assert next(o for o in json.loads(saved)["objects"] if o["id"] == "case-body")["components"]["editor.material.v1"]["tint"][0] < .2
            scene_path.write_bytes(saved + b"\n")
            call("scene_save", dict(expected_revision=state()["revision"]), error=True)
            assert scene_path.read_bytes() == saved + b"\n"
            scene_path.write_bytes(saved)
            action("scene_undo")
            action("scene_save")
            assert state()["scene"] == initial["scene"]
            assert not state()["dirty"]
            summary = dict(status="PASS", tools=6, checks=["initialization", "atomic batch rejection", "stale revision rejection", "scene edits", "material/camera/render", "inline PNG", "undo pixel equality", "redo pixel equality", "create/reparent/subtree removal", "save", "external edit refusal"], initial_scene_hash=initial["sceneHash"], final_scene_hash=state()["sceneHash"], baseline_sha256=hashlib.sha256(baseline).hexdigest(), edited_sha256=hashlib.sha256(edited).hexdigest(), live_editor_sync=False)
            (output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
            print(json.dumps(summary, indent=2))
        finally:
            (output / "transcript.json").write_text(json.dumps(transcript, indent=2) + "\n")
            process.stdin.close()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()


if __name__ == "__main__":
    main()
