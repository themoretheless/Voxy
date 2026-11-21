#!/usr/bin/env python3
"""End-to-end qualification of voxy_mcp over both stdio and Streamable HTTP transports."""
import argparse
import base64
import http.client
import json
import shutil
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path
from urllib.parse import urlparse

ROOT = Path(__file__).resolve().parents[1]


def write_sample_project_assets(proj_dir: Path):
    (proj_dir / "quad.obj").write_text(
        "v -0.5 -0.5 0\nv 0.5 -0.5 0\nv 0.5 0.5 0\nv -0.5 0.5 0\nf 1 2 3\nf 1 3 4\n"
    )
    prop_scene = {
        "version": 1,
        "objects": [
            {
                "id": "prop-root",
                "parent": None,
                "name": "Prop Root",
                "active": True,
                "translation": [0.0, 0.0, 0.0],
                "rotation": [0.0, 0.0, 0.0, 1.0],
                "scale": [1.0, 1.0, 1.0],
                "components": {"editor.model.v1": "quad"},
            }
        ],
    }
    (proj_dir / "prop.scene.json").write_text(json.dumps(prop_scene))
    arena_scene = {
        "version": 1,
        "objects": [
            {
                "id": "arena-floor",
                "parent": None,
                "name": "Arena Floor",
                "active": True,
                "translation": [0.0, 0.0, 0.0],
                "rotation": [0.0, 0.0, 0.0, 1.0],
                "scale": [1.0, 1.0, 1.0],
                "components": {"editor.model.v1": "quad"},
            }
        ],
    }
    (proj_dir / "arena.scene.json").write_text(json.dumps(arena_scene))


class StdioClient:
    def __init__(self, binary: Path, extra_args=None):
        cmd = [str(binary)] + (extra_args or [])
        self.proc = subprocess.Popen(
            cmd,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
        )
        self.seq = 0
        self.notifications = []
        self.client_root_dir = None

    def send_raw(self, obj):
        self.proc.stdin.write(json.dumps(obj) + "\n")
        self.proc.stdin.flush()

    def _answer_server_request(self, msg):
        method = msg["method"]
        rid = msg["id"]
        if method == "ping":
            self.send_raw({"jsonrpc": "2.0", "id": rid, "result": {}})
        elif method == "roots/list":
            roots = []
            if self.client_root_dir:
                roots.append({"uri": f"file://{self.client_root_dir}", "name": "test-root"})
            self.send_raw({"jsonrpc": "2.0", "id": rid, "result": {"roots": roots}})
        elif method == "sampling/createMessage":
            self.send_raw({
                "jsonrpc": "2.0",
                "id": rid,
                "result": {
                    "role": "assistant",
                    "content": {"type": "text", "text": "ok-sample"},
                    "model": "test-stub",
                    "stopReason": "endTurn",
                },
            })
        elif method == "elicitation/create":
            self.send_raw({
                "jsonrpc": "2.0",
                "id": rid,
                "result": {"action": "accept", "content": {"confirm": True}},
            })
        else:
            self.send_raw({"jsonrpc": "2.0", "id": rid, "error": {"code": -32601, "message": "unsupported"}})

    def read_until_id(self, target_id, cancel_after_progress=False):
        sent_cancel = False
        while True:
            line = self.proc.stdout.readline()
            assert line, f"EOF from voxy_mcp; stderr={self.proc.stderr.read()}"
            msg = json.loads(line)
            if "method" in msg and "id" in msg:
                self._answer_server_request(msg)
                continue
            if "method" in msg:
                self.notifications.append(msg)
                if cancel_after_progress and not sent_cancel and msg["method"] == "notifications/progress":
                    self.send_raw({"jsonrpc": "2.0", "method": "notifications/cancelled", "params": {"requestId": target_id}})
                    sent_cancel = True
                    return None
                continue
            if msg.get("id") == target_id:
                return msg

    def request(self, method, params=None):
        self.seq += 1
        rid = self.seq
        payload = {"jsonrpc": "2.0", "id": rid, "method": method}
        if params is not None:
            payload["params"] = params
        self.send_raw(payload)
        return self.read_until_id(rid)

    def call(self, name, arguments=None, meta=None, expect_error=False):
        params = {"name": name, "arguments": arguments or {}}
        if meta is not None:
            params["_meta"] = meta
        resp = self.request("tools/call", params)
        assert "error" not in resp, resp
        res = resp["result"]
        assert bool(res["isError"]) == expect_error, res
        return res

    def close(self):
        self.proc.stdin.close()
        rc = self.proc.wait(timeout=15)
        assert rc == 0, f"voxy_mcp exited {rc}: {self.proc.stderr.read()}"


def test_stdio_full_suite(binary: Path, scratch: Path):
    client = StdioClient(binary, ["--page-size", "5"])
    try:
        # 1. Pre-init, initialize, ping
        assert client.request("tools/list")["error"]["code"] == -32000
        init = client.request("initialize", {
            "protocolVersion": "2025-06-18",
            "capabilities": {"roots": {"listChanged": True}, "sampling": {}, "elicitation": {}},
            "clientInfo": {"name": "voxy-e2e", "version": "1.0"},
        })["result"]
        assert init["protocolVersion"] == "2025-06-18"
        client.send_raw({"jsonrpc": "2.0", "method": "notifications/initialized"})
        assert client.request("ping")["result"] == {}

        # 2. Pagination across tools/list + invalid cursor
        tool_names = []
        cursor = None
        pages = 0
        while True:
            page = client.request("tools/list", {"cursor": cursor} if cursor else None)["result"]
            tool_names.extend(t["name"] for t in page["tools"])
            pages += 1
            cursor = page.get("nextCursor")
            if not cursor:
                break
        assert pages >= 4 and len(tool_names) >= 24, (pages, len(tool_names))
        assert client.request("tools/list", {"cursor": "bad-cursor"})["error"]["code"] == -32602

        # 3. Logging level + subscriptions
        client.request("logging/setLevel", {"level": "info"})
        proj_parent = scratch / "workspace"
        proj_parent.mkdir(parents=True, exist_ok=True)
        proj_dir = proj_parent / "demo_proj"
        client.client_root_dir = proj_dir
        res_create = client.call("project_create", {"directory": str(proj_dir), "name": "E2E Project"})
        assert res_create["structuredContent"]["opened"] is True
        assert any(n["method"] == "notifications/message" for n in client.notifications)

        write_sample_project_assets(proj_dir)
        client.call("asset_register", {"asset": "quad", "source": "quad.obj"})
        client.call("asset_register", {"asset": "prop", "source": "prop.scene.json"})
        listed = client.call("asset_list")["structuredContent"]["assets"]
        assert {a["asset"] for a in listed} == {"quad", "prop"}
        client.call("asset_register", {"asset": "arena", "source": "arena.scene.json"})
        inspected = client.call("asset_inspect", {"asset": "quad"})["structuredContent"]
        assert inspected["exists"] and inspected["model"]["vertices"] == 4 and inspected["model"]["indices"] == 6, inspected
        client.call("project_update_descriptor", {"name": "E2E Updated"})
        assert client.call("project_validate")["structuredContent"]["valid"] is True

        # 4. Component schemas (all 30), resources, templates, completions, prompts
        schemas = client.call("component_schemas")["structuredContent"]
        assert schemas["count"] == 30, schemas
        tpl_read = client.request("resources/read", {"uri": "voxy://schemas/component/game.character.v1"})["result"]
        assert "game.character.v1" in tpl_read["contents"][0]["text"]
        comp = client.request("completion/complete", {
            "ref": {"type": "ref/resource", "uri": "voxy://schemas/component/{schema}"},
            "argument": {"name": "schema", "value": "character"},
        })["result"]["completion"]
        assert "game.character.v1" in comp["values"]
        prompts = client.request("prompts/list")["result"]["prompts"]
        assert any(p["name"] == "author_component" for p in prompts)
        p_get = client.request("prompts/get", {"name": "author_component", "arguments": {"schema": "scene.fog.v1", "node_id": "fog"}})["result"]
        assert "scene.fog.v1" in p_get["messages"][0]["content"]["text"]

        # 5. Subscription notifications + scene CRUD + component_set/patch/remove + undo/redo/save
        client.request("resources/subscribe", {"uri": "voxy://project/scene"})
        client.notifications.clear()
        rev = client.call("scene_get")["structuredContent"]["revision"]
        client.call("scene_apply", {
            "expected_revision": rev,
            "edits": [
                {"op": "create", "object": {
                    "id": "camera", "parent": None, "name": "Main Cam", "active": True,
                    "translation": [0, 0, 0], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1],
                    "components": {"editor.camera.v1": {"legacy": False, "target": [0, 0, 0], "distance": 2.5, "yaw": 0.4, "pitch": 0.25, "perspective": True}},
                }},
                {"op": "create", "object": {
                    "id": "hero", "parent": None, "name": "Hero Quad", "active": True,
                    "translation": [0, 0, 0], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1],
                    "components": {"editor.model.v1": "quad", "editor.material.v1": {"tint": [0.2, 0.7, 0.3, 1.0], "lit": False}},
                }},
                {"op": "component_set", "node_id": "hero", "schema": "game.character.v1",
                 "value": {"half_extents": [0.1, 0.2, 0.1], "speed": 1.5, "gravity": -9.8, "jump_speed": 3.0}},
                {"op": "component_patch", "node_id": "hero", "schema": "game.character.v1", "patch": {"speed": 2.2}},
                {"op": "component_set", "node_id": "hero", "schema": "game.audio-bus.v1", "value": {"bus": 0, "gain": 0.6}},
                {"op": "component_remove", "node_id": "hero", "schema": "game.audio-bus.v1"},
            ],
        })
        assert any(n["method"] == "notifications/resources/updated" and n["params"]["uri"] == "voxy://project/scene" for n in client.notifications)
        queried = client.call("scene_query", {"schema": "game.character.v1"})["structuredContent"]
        assert queried["count"] == 1 and queried["objects"][0]["id"] == "hero"
        rev = client.call("scene_undo", {"expected_revision": rev + 1})["structuredContent"]["revision"]
        rev = client.call("scene_redo", {"expected_revision": rev})["structuredContent"]["revision"]
        dup_res = client.call("scene_apply", {
            "expected_revision": rev,
            "edits": [{"op": "duplicate", "node_ids": ["hero"], "offset": [2.0, 0.0, 0.0]}],
        })["structuredContent"]
        rev = dup_res["revision"]
        assert any(o["translation"] == [2.0, 0.0, 0.0] and o["id"] != "hero" for o in dup_res["scene"]["objects"]), dup_res
        rev = client.call("scene_undo", {"expected_revision": rev})["structuredContent"]["revision"]
        saved = client.call("scene_save", {"expected_revision": rev})["structuredContent"]
        assert saved["dirty"] is False

        # Crash recovery via MCP autosave sidecar across project reopen
        client.call("scene_apply", {
            "expected_revision": rev,
            "edits": [{"op": "rename", "node_id": "hero", "name": "Unsaved Hero Crash"}],
        })
        reopened = client.call("project_open", {"path": str(proj_dir / "project.voxy")})["structuredContent"]
        assert reopened["hasAutosaveRecovery"] is True, reopened
        assert client.call("scene_get")["structuredContent"]["hasAutosaveRecovery"] is True
        recovered = client.call("scene_recover_autosave", {"action": "recover"})["structuredContent"]
        assert any(o["name"] == "Unsaved Hero Crash" for o in recovered["scene"]["objects"]), recovered
        rev = client.call("scene_undo", {"expected_revision": recovered["revision"]})["structuredContent"]["revision"]
        saved = client.call("scene_save", {"expected_revision": rev})["structuredContent"]
        assert saved["dirty"] is False and saved["hasAutosaveRecovery"] is False

        # 6. Non-destructive Prefabs authoring in separate prefab test scene
        (proj_dir / "prefab_host.scene.json").write_text('{"version":1,"objects":[]}')
        client.call("scene_open_file", {"scene_path": "prefab_host.scene.json"})
        client.call("prefab_instance_add", {"id": "inst-1", "asset": "prop"})
        client.call("prefab_override_set", {"instance_id": "inst-1", "target_id": "prop-root", "override": {"translation": [3.0, 0.0, 0.0]}})
        prefab_state = client.call("prefab_scene_get")["structuredContent"]
        assert len(prefab_state["authored"]["instances"]) == 1
        assert len(prefab_state["expanded"]["objects"]) == 1
        client.call("prefab_override_clear", {"instance_id": "inst-1", "target_id": "prop-root"})
        client.call("prefab_instance_remove", {"id": "inst-1"})
        # Switch back to main.scene.json for Play / Export / Render
        client.call("scene_open_file", {"scene_path": "main.scene.json"})

        # 7. HeadlessGame Play + Progress + Cancellation + Package Export
        client.call("play_start")
        client.call("play_input", {"key": "ArrowRight", "pressed": True})
        client.notifications.clear()
        stepped = client.call("play_step", {"ticks": 16}, meta={"progressToken": "p-step"})["structuredContent"]
        assert stepped["stepped"] == 16 and stepped["fixedTicks"] == 16
        assert stepped["frameAttribution"]["framesRecorded"] >= 16 and stepped["frameAttribution"]["dominantDomain"] != "idle"
        assert any(n["method"] == "notifications/progress" and n["params"]["progressToken"] == "p-step" for n in client.notifications)

        # Cancel a slow multi-tick step mid-flight
        client.seq += 1
        cancel_id = client.seq
        client.send_raw({
            "jsonrpc": "2.0",
            "id": cancel_id,
            "method": "tools/call",
            "params": {"_meta": {"progressToken": "p-cancel"}, "name": "play_step", "arguments": {"ticks": 200, "tick_delay_ms": 10}},
        })
        assert client.read_until_id(cancel_id, cancel_after_progress=True) is None
        assert client.request("ping")["result"] == {}
        snap = client.call("play_snapshot")["structuredContent"]
        assert 16 < snap["fixedTicks"] < 216, snap["fixedTicks"]
        assert snap["frameAttribution"]["framesRecorded"] == snap["fixedTicks"]
        tr = client.call("play_transition_scene", {"scene_path": "arena.scene.json"})["structuredContent"]
        assert tr["transitioned"] is True and tr["activeScenePath"] == "arena.scene.json", tr
        snap_tr = client.call("play_snapshot")["structuredContent"]
        assert snap_tr["activeScenePath"] == "arena.scene.json"
        assert any(o["id"] == "arena-floor" for o in snap_tr["scene"]["objects"]), snap_tr
        client.call("play_stop")

        pkg_path = scratch / "demo.voxypkg"
        pkg_meta = client.call("package_export_project", {"output_path": str(pkg_path)})["structuredContent"]
        assert pkg_path.is_file() and pkg_meta["bytes"] > 0
        client.call("play_start", {"package_path": str(pkg_path)})
        client.call("play_step", {"ticks": 4})
        tr_pkg = client.call("play_transition_scene", {"scene_path": "arena.scene.json"})["structuredContent"]
        assert tr_pkg["transitioned"] is True and tr_pkg["activeScenePath"] == "arena.scene.json", tr_pkg
        client.call("play_stop")

        # 8. Bidirectional server->client probes (roots, sampling, elicitation, ping)
        roots_res = client.call("client_bridge_probe", {"action": "roots"})["structuredContent"]
        assert any("project.voxy" in p for p in roots_res["discoveredProjects"]), roots_res
        assert client.call("client_bridge_probe", {"action": "sampling"})["structuredContent"]["reply"]["model"] == "test-stub"
        assert client.call("client_bridge_probe", {"action": "elicitation"})["structuredContent"]["reply"]["action"] == "accept"
        assert client.call("client_bridge_probe", {"action": "ping"})["structuredContent"]["ok"] is True

        # 9. Offscreen render
        offscreen = client.call("viewport_render_offscreen", {"width": 256, "height": 256})
        assert offscreen["structuredContent"]["capture"] == "offscreen"
        png_block = next(c for c in offscreen["content"] if c["type"] == "image")
        png_bytes = base64.b64decode(png_block["data"])
        assert png_bytes[:8] == b"\x89PNG\r\n\x1a\n"
        assert int.from_bytes(png_bytes[16:20], "big") == 256
        assert int.from_bytes(png_bytes[20:24], "big") == 256
        blob_res = client.request("resources/read", {"uri": "voxy://renders/latest"})["result"]["contents"][0]
        assert base64.b64decode(blob_res["blob"]) == png_bytes

        # 10. Live voxy_editor window capture bridge test
        editor_bin = binary.with_name("voxy_editor.exe" if sys.platform == "win32" else "voxy_editor")
        if editor_bin.is_file():
            ed_proc = subprocess.Popen(
                [str(editor_bin), "--project", str(proj_dir / "project.voxy")],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.PIPE,
                text=True,
            )
            try:
                deadline = time.time() + 12.0
                connected = False
                while time.time() < deadline:
                    st = client.call("editor_window_status")["structuredContent"]
                    if st.get("liveWindowConnected"):
                        connected = True
                        break
                    if ed_proc.poll() is not None:
                        break
                    time.sleep(0.15)
                if connected:
                    live = client.call("editor_window_capture", {"timeout_ms": 5000})
                    assert live["structuredContent"]["capture"] == "live_window"
                    live_png = base64.b64decode(next(c for c in live["content"] if c["type"] == "image")["data"])
                    assert live_png[:8] == b"\x89PNG\r\n\x1a\n"
                    print("live_window capture PASS", live["structuredContent"]["blake3"][:12])
                else:
                    raise RuntimeError(f"voxy_editor live window did not connect in 12s: {ed_proc.stderr.read()}")
            finally:
                ed_proc.terminate()
                try:
                    ed_proc.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    ed_proc.kill()
                    ed_proc.wait()
    finally:
        client.close()


def http_req(host, port, method, path="/mcp", headers=None, body=None):
    conn = http.client.HTTPConnection(host, port, timeout=10)
    raw = json.dumps(body).encode() if body is not None else None
    hdrs = dict(headers or {})
    if raw is not None and "Content-Type" not in hdrs:
        hdrs["Content-Type"] = "application/json"
    conn.request(method, path, body=raw, headers=hdrs)
    resp = conn.getresponse()
    data = resp.read()
    return resp.status, dict(resp.getheaders()), data


def test_streamable_http_suite(binary: Path, scratch: Path):
    proc = subprocess.Popen(
        [str(binary), "--http", "127.0.0.1:0"],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
    )
    try:
        listen_url = None
        deadline = time.time() + 10.0
        while time.time() < deadline:
            line = proc.stderr.readline()
            if "VOXY_MCP_HTTP_LISTEN " in line:
                listen_url = line.split("VOXY_MCP_HTTP_LISTEN ", 1)[1].strip()
                break
        assert listen_url, "HTTP MCP server did not print VOXY_MCP_HTTP_LISTEN"
        parsed = urlparse(listen_url)
        host, port = parsed.hostname, parsed.port

        # Origin & protocol version security
        st, _, _ = http_req(host, port, "POST", headers={"Origin": "https://evil.example", "Accept": "application/json"}, body={"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18"}})
        assert st == 403
        st, _, _ = http_req(host, port, "POST", headers={"MCP-Protocol-Version": "1999-01-01", "Accept": "application/json"}, body={"jsonrpc": "2.0", "id": 1, "method": "ping"})
        assert st == 400

        # Initialize & session header
        st, hdrs, body = http_req(
            host, port, "POST",
            headers={"Origin": f"http://localhost:{port}", "Accept": "application/json, text/event-stream"},
            body={"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {"roots": {"listChanged": True}}}},
        )
        assert st == 200
        sid = {k.lower(): v for k, v in hdrs.items()}["mcp-session-id"]
        assert json.loads(body)["result"]["protocolVersion"] == "2025-06-18"
        st, _, _ = http_req(host, port, "POST", headers={"Mcp-Session-Id": sid, "Accept": "application/json"}, body={"jsonrpc": "2.0", "method": "notifications/initialized"})
        assert st == 202

        # Open background GET /mcp SSE listener & handle bidirectional roots/list
        sse_events = []
        stop_sse = threading.Event()
        def sse_reader():
            conn = http.client.HTTPConnection(host, port, timeout=10)
            conn.request("GET", "/mcp", headers={"Mcp-Session-Id": sid, "Accept": "text/event-stream", "Last-Event-ID": "0"})
            resp = conn.getresponse()
            assert resp.status == 200
            while not stop_sse.is_set():
                line = resp.fp.readline().decode("utf-8", errors="replace")
                if not line:
                    break
                if line.startswith("data: "):
                    msg = json.loads(line[6:].strip())
                    sse_events.append(msg)
                    if msg.get("method") == "roots/list" and "id" in msg:
                        http_req(host, port, "POST", headers={"Mcp-Session-Id": sid, "Accept": "application/json"}, body={"jsonrpc": "2.0", "id": msg["id"], "result": {"roots": []}})

        sse_thread = threading.Thread(target=sse_reader, daemon=True)
        sse_thread.start()
        time.sleep(0.15)

        # Create project + subscribe + mutate -> verify GET SSE notification
        proj_dir = scratch / "http_proj"
        proj_dir.parent.mkdir(parents=True, exist_ok=True)
        http_req(host, port, "POST", headers={"Mcp-Session-Id": sid, "Accept": "application/json"}, body={"jsonrpc": "2.0", "id": 2, "method": "resources/subscribe", "params": {"uri": "voxy://project/scene"}})
        http_req(host, port, "POST", headers={"Mcp-Session-Id": sid, "Accept": "application/json"}, body={"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "project_create", "arguments": {"directory": str(proj_dir), "name": "HTTP Proj"}}})

        # Bidirectional roots/list delivered via GET SSE and answered via POST 202
        st, _, body = http_req(host, port, "POST", headers={"Mcp-Session-Id": sid, "Accept": "application/json"}, body={"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "client_bridge_probe", "arguments": {"action": "roots"}}})
        assert st == 200 and json.loads(body)["result"]["isError"] is False

        # POST SSE progress streaming on play_step
        http_req(host, port, "POST", headers={"Mcp-Session-Id": sid, "Accept": "application/json"}, body={"jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": {"name": "play_start", "arguments": {}}})
        st, hdrs, sse_post = http_req(
            host, port, "POST",
            headers={"Mcp-Session-Id": sid, "MCP-Protocol-Version": "2025-06-18", "Accept": "application/json, text/event-stream"},
            body={"jsonrpc": "2.0", "id": 6, "method": "tools/call", "params": {"_meta": {"progressToken": "http-p"}, "name": "play_step", "arguments": {"ticks": 8}}},
        )
        assert st == 200
        ct = {k.lower(): v for k, v in hdrs.items()}["content-type"]
        assert "text/event-stream" in ct and "notifications/progress" in sse_post.decode()
        http_req(host, port, "POST", headers={"Mcp-Session-Id": sid, "Accept": "application/json"}, body={"jsonrpc": "2.0", "id": 7, "method": "tools/call", "params": {"name": "play_stop", "arguments": {}}})

        sse_deadline = time.time() + 2.0
        while time.time() < sse_deadline and not any(ev.get("method") == "notifications/resources/updated" for ev in sse_events):
            time.sleep(0.05)
        assert any(ev.get("method") == "notifications/resources/updated" for ev in sse_events), sse_events

        # DELETE session -> 404 afterwards
        stop_sse.set()
        st, _, _ = http_req(host, port, "DELETE", headers={"Mcp-Session-Id": sid})
        assert st == 200
        st, _, _ = http_req(host, port, "POST", headers={"Mcp-Session-Id": sid, "Accept": "application/json"}, body={"jsonrpc": "2.0", "id": 8, "method": "ping"})
        assert st == 404
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile", default="release")
    args = parser.parse_args()
    suffix = ".exe" if sys.platform == "win32" else ""
    binary = ROOT / "target" / args.profile / f"voxy_mcp{suffix}"
    assert binary.is_file(), f"missing {binary}"
    scratch = Path(tempfile.mkdtemp(prefix="voxy-mcp-e2e-"))
    try:
        test_stdio_full_suite(binary, scratch / "stdio")
        print("voxy_mcp stdio suite PASS")
        test_streamable_http_suite(binary, scratch / "http")
        print("voxy_mcp Streamable HTTP suite PASS")
    finally:
        shutil.rmtree(scratch, ignore_errors=True)


if __name__ == "__main__":
    main()
