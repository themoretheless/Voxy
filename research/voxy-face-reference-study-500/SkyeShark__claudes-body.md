# Facial rendering source scan: SkyeShark/claudes-body

Repository: https://github.com/SkyeShark/claudes-body

Pinned source revision: b0e42cad0e27cbc1b5aefb4d1d527251912afaf6

Discovery topic: lip sync 3D

Description: Claude's very own body! A fully 3d VRM Claude that reacts to and reads Claude Code's responses with lip sync and keyword-driven anims and expressions.

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"mouth": ["tools/symmetrize-mouth-morphs.js"], "rig": ["tools/rig-tail.py"], "face": ["tools/build-face-atlases.html"]}

## Source file: tools/rig-tail.py

{
  "path": "tools/rig-tail.py",
  "blob_sha": "b6210fc82158d36531440efc5d937e40f8d47158",
  "sha256": "ddb7bec35e679ef05eba1092f165b42a43c30528fb5a42b8a21f586d5ad14ea2",
  "lines": 167,
  "bytes": 6412,
  "selection_score": 15,
  "keyword_line_counts": {
    "deform": 1,
    "rig": 18
  },
  "url": "https://github.com/SkyeShark/claudes-body/blob/b0e42cad0e27cbc1b5aefb4d1d527251912afaf6/tools/rig-tail.py",
  "function_signature_candidates": [
    {
      "line": 29,
      "text": "def closest_point_distance(p, head, tail):"
    },
    {
      "line": 40,
      "text": "def main():"
    }
  ]
}

```text

"""
rig-tail.py — Surgical tail-bone weight fixer.

Reads claude.fbx, finds the tail mesh and the tail bone chain
(tailbase / tail / tailtip), recomputes ONLY those bone weights on the
tail mesh's vertices using inverse-distance to each bone's head-to-tail
segment, and writes the FBX back. Every other bone on every other vertex
is left untouched.

Run:
    "C:\\Program Files\\Blender Foundation\\Blender 4.3\\blender.exe" \\
        --background --python tools/rig-tail.py
"""

import bpy
import re
import sys
from mathutils import Vector

INPUT_FBX  = r"C:\Users\sdn52\OneDrive\Desktop\claudethinking\claude.fbx"
OUTPUT_FBX = r"C:\Users\sdn52\OneDrive\Desktop\claudethinking\claude_rigged.fbx"

# Recognise tail bones by name. Includes singular and chain-style names.
TAIL_BONE_RE = re.compile(r'^(tail|tailbase|tailtip|tail[._-]?\d+)$', re.IGNORECASE)
TAIL_MESH_RE = re.compile(r'tail', re.IGNORECASE)

# ---------- helpers ----------

def closest_point_distance(p, head, tail):
    """Shortest distance from point p to the line segment between head and tail."""
    seg = tail - head
    seg_len_sq = seg.length_squared
    if seg_len_sq < 1e-9:
        return (p - head).length
    t = max(0.0, min(1.0, (p - head).dot(seg) / seg_len_sq))
    closest = head + seg * t
    return (p - closest).length


def main():
    # --- start clean ---
    bpy.ops.wm.read_factory_settings(use_empty=True)

    # --- import FBX ---
    print(f"[rig-tail] importing {INPUT_FBX}")
    bpy.ops.import_scene.fbx(filepath=INPUT_FBX)

    # --- find armature ---
    armature = next((o for o in bpy.data.objects if o.type == 'ARMATURE'), None)
    if not armature:
        print("[rig-tail] ERROR: no armature in file")
        sys.exit(1)
    print(f"[rig-tail] armature: {armature.name}")
    print(f"[rig-tail] all bones: {[b.name for b in armature.data.bones]}")

    # --- find tail bones (only true tail-chain bones, skip e.g. 'tailbone' on a different rig) ---
    tail_bones = [b for b in armature.data.bones if TAIL_BONE_RE.match(b.name)]
    if not tail_bones:
        # fallback: any bone whose name contains "tail"
        tail_bones = [b for b in armature.data.bones if 'tail' in b.name.lower()]
    print(f"[rig-tail] tail bones found: {[b.name for b in tail_bones]}")
    if not tail_bones:
        print("[rig-tail] ERROR: no tail bones found")
        sys.exit(1)
    tail_bone_names = {b.name for b in tail_bones}

    # --- find the body mesh (the rig's combined char1) ---
    body_mesh = None
    for obj in bpy.data.objects:
        if obj.type != 'MESH':
            continue
        # Prefer a mesh named 'tail' if it exists; fall back to the largest mesh.
        if TAIL_MESH_RE.search(obj.name) or TAIL_MESH_RE.search(obj.data.name):
            body_mesh = obj
            break
    if not body_mesh:
        meshes = [o for o in bpy.data.objects if o.type == 'MESH']
        if not meshes:
            print("[rig-tail] ERROR: no meshes found")
            sys.exit(1)
        body_mesh = max(meshes, key=lambda o: len(o.data.vertices))
        print(f"[rig-tail] no tail-named mesh; using largest mesh: {body_mesh.name}")
    tail_mesh = body_mesh
    print(f"[rig-tail] working on mesh: {tail_mesh.name}, {len(tail_mesh.data.vertices)} verts")

    # --- compute bone segments in WORLD space ---
    armature_world = armature.matrix_world
    bone_segments = []
    for bone in tail_bones:
        head_world = armature_world @ bone.head_local
        tail_world = armature_world @ bone.tail_local
        bone_segments.append((bone.name, head_world, tail_world))
        print(f"[rig-tail]   {bone.name}: head={tuple(round(x,3) for x in head_world)} tail={tuple(round(x,3) for x in tail_world)}")

    # --- ensure each tail bone has a vertex group on the tail mesh ---
    for bone_name in tail_bone_names:
        if bone_name not in tail_mesh.vertex_groups:
            tail_mesh.vertex_groups.new(name=bone_name)

    # --- per-vertex tail-only weight recompute ---
    # Process ONLY vertices that currently have any weight to any tail bone.
    # These are the actual tail vertices; everything else is body and gets
    # left strictly alone.
    mesh_world = tail_mesh.matrix_world
    mesh_data = tail_mesh.data

    # Map vertex_group_index → name once, for fast lookups.
    vg_idx_to_name = {vg.index: vg.name for vg in tail_mesh.vertex_groups}

    n_processed = 0
    n_skipped   = 0
    for v in mesh_data.vertices:
        # Existing tail-bone influence on this vertex (sum of current
        # weights on the tail bone groups). If zero, this vertex isn't
        # a tail vertex — skip it entirely.
        existing_tail_weight = 0.0
        for g in v.groups:
            if vg_idx_to_name[g.group] in tail_bone_names:
                existing_tail_weight += g.weight

        if existing_tail_weight <= 1e-5:
            n_skipped += 1
            continue

        p_world = mesh_world @ v.co

        # Inverse-distance² to each tail bone segment.
        raw = []
        for name, h, t in bone_segments:
            d = closest_point_distance(p_world, h, t)
            raw.append((name, 1.0 / (d * d + 1e-4)))
        raw_sum = sum(w for _, w in raw)

        # Redistribute the existing_tail_weight across tail bones by
        # inverse-distance. The total tail-bone influence on this vertex
        # therefore stays the same; only the *split* between tail bones
        # changes. Non-tail weights remain untouched.
        for name, w in raw:
            share = (w / raw_sum) * existing_tail_weight if raw_sum > 0 else 0.0
            vg = tail_mesh.vertex_groups[name]
            vg.add([v.index], share, 'REPLACE')

        n_processed += 1

    print(f"[rig-tail] reweighted {n_processed} tail vertices across {len(tail_bone_names)} tail bones; skipped {n_skipped} non-tail vertices")

    # --- export ---
    print(f"[rig-tail] exporting {OUTPUT_FBX}")
    bpy.ops.export_scene.fbx(
        filepath=OUTPUT_FBX,
        use_selection=False,
        apply_unit_scale=True,
        bake_space_transform=False,
        object_types={'ARMATURE', 'MESH', 'EMPTY'},
        use_armature_deform_only=False,
        bake_anim=False,
        path_mode='COPY',
        embed_textures=True,
        add_leaf_bones=False,
        primary_bone_axis='Y',
        secondary_bone_axis='X',
    )
    print("[rig-tail] done.")


if __name__ == '__main__':
    main()


```

## Source file: tools/symmetrize-mouth-morphs.js

{
  "path": "tools/symmetrize-mouth-morphs.js",
  "blob_sha": "34d0474ee3d61fe17cee8c7f7a807d8369b979e9",
  "sha256": "6ebc74a48f5ba652f3f00b978880380dcc811387fbdd65bd1c74b803526077da",
  "lines": 118,
  "bytes": 4374,
  "selection_score": 15,
  "keyword_line_counts": {
    "mouth": 8
  },
  "url": "https://github.com/SkyeShark/claudes-body/blob/b0e42cad0e27cbc1b5aefb4d1d527251912afaf6/tools/symmetrize-mouth-morphs.js",
  "function_signature_candidates": []
}

```text

#!/usr/bin/env node
'use strict';

// symmetrize-mouth-morphs.js — Edit a VRM/glTF in place: for each mouth
// morph target whose name appears in DEFAULT_TARGETS, zero the X-component
// of every position delta. The morph then opens/widens the mouth vertically
// without translating it sideways.
//
// Why we need this: the source-authored mouth shape keys for `surprised`,
// `aa`, `ih`, `ou`, `oh` push every vertex in -X (mean ΔX ≈ -12 mm), so
// the mouth visibly shifts to one side at high weights. Visemes switch
// every ~120 ms during speech and the asymmetry isn't noticed there, but
// `surprised` holds at full weight and the lopsidedness shows.
//
// This tool fixes the asymmetry in the file itself, so the model is
// usable in any VRM consumer (not just our own pipeline output).
//
// Usage:
//   node tools/symmetrize-mouth-morphs.js <input.vrm> [output.vrm]
//   (in-place if output is omitted)

const fs = require('fs');
const path = require('path');

const DEFAULT_TARGETS = new Set(['surprised', 'aa', 'ih', 'ou', 'oh']);

const INPUT  = process.argv[2];
const OUTPUT = process.argv[3] || INPUT;
if (!INPUT) {
  console.error('usage: symmetrize-mouth-morphs.js <in.vrm> [out.vrm]');
  process.exit(1);
}

// ---------- read GLB ----------
const raw = fs.readFileSync(INPUT);
if (raw.toString('ascii', 0, 4) !== 'glTF') throw new Error('not a GLB');
const c0len = raw.readUInt32LE(12);
const json = JSON.parse(raw.toString('utf8', 20, 20 + c0len));
const c0pad = (4 - (c0len % 4)) % 4;
const binStart = 20 + c0len + c0pad;
const c1len = raw.readUInt32LE(binStart);
const bin = Buffer.from(raw.slice(binStart + 8, binStart + 8 + c1len));

function targetNamesOf(meshIdx) {
  const m = json.meshes[meshIdx];
  return (m.extras || {}).targetNames
      || ((m.primitives[0] || {}).extras || {}).targetNames
      || [];
}

function zeroXOnAccessor(accIdx) {
  const acc = json.accessors[accIdx];
  if (!acc || acc.type !== 'VEC3') return 0;
  let touched = 0;
  // Dense data
  if (acc.bufferView !== undefined) {
    const bv = json.bufferViews[acc.bufferView];
    const off = (bv.byteOffset || 0) + (acc.byteOffset || 0);
    const stride = bv.byteStride || 12;
    for (let i = 0; i < acc.count; i++) bin.writeFloatLE(0, off + i * stride);
    touched += acc.count;
  }
  // Sparse data
  if (acc.sparse) {
    const sv = acc.sparse.values;
    const svBv = json.bufferViews[sv.bufferView];
    const svOff = (svBv.byteOffset || 0) + (sv.byteOffset || 0);
    const stride = svBv.byteStride || 12;
    for (let i = 0; i < acc.sparse.count; i++) bin.writeFloatLE(0, svOff + i * stride);
    touched += acc.sparse.count;
  }
  if (acc.min) acc.min[0] = 0;
  if (acc.max) acc.max[0] = 0;
  return touched;
}

let nSymmetrized = 0;
for (const m of json.meshes) {
  if (!/mouth/i.test(m.name || '')) continue;
  const names = targetNamesOf(json.meshes.indexOf(m));
  for (const prim of (m.primitives || [])) {
    for (let t = 0; t < (prim.targets || []).length; t++) {
      const tgt = prim.targets[t];
      if (tgt.POSITION === undefined) continue;
      if (!DEFAULT_TARGETS.has(names[t])) continue;
      const n = zeroXOnAccessor(tgt.POSITION);
      if (n > 0) {
        nSymmetrized++;
        console.log(`[symmetrize] '${names[t]}' (${n} verts) on mesh '${m.name}'`);
      }
    }
  }
}
if (nSymmetrized === 0) {
  console.log('[symmetrize] no matching mouth targets — nothing to do');
  process.exit(0);
}

// ---------- repack GLB ----------
const newJsonBuf = Buffer.from(JSON.stringify(json), 'utf8');
const jsonPad = (4 - (newJsonBuf.length % 4)) % 4;
const paddedJson = jsonPad ? Buffer.concat([newJsonBuf, Buffer.alloc(jsonPad, 0x20)]) : newJsonBuf;
const binPad = (4 - (bin.length % 4)) % 4;
const paddedBin = binPad ? Buffer.concat([bin, Buffer.alloc(binPad, 0x00)]) : bin;
const total = 12 + 8 + paddedJson.length + 8 + paddedBin.length;
const out = Buffer.alloc(total);
out.write('glTF', 0, 4, 'ascii');
out.writeUInt32LE(2, 4);
out.writeUInt32LE(total, 8);
out.writeUInt32LE(paddedJson.length, 12);
out.write('JSON', 16, 4, 'ascii');
paddedJson.copy(out, 20);
const binChunkOff = 20 + paddedJson.length;
out.writeUInt32LE(paddedBin.length, binChunkOff);
out.write('BIN\0', binChunkOff + 4, 4, 'ascii');
paddedBin.copy(out, binChunkOff + 8);
fs.writeFileSync(OUTPUT, out);
console.log(`[out] ${OUTPUT} (${out.length} bytes)`);


```

## Source file: hook.js

{
  "path": "hook.js",
  "blob_sha": "7e7f4446ed14a198039f76800c0fcdaa523cb4e4",
  "sha256": "c6a08c20964d4f5a0a60f08c370fc51eb62533997cbab895fe63b81972ba49f9",
  "lines": 153,
  "bytes": 5607,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 1
  },
  "url": "https://github.com/SkyeShark/claudes-body/blob/b0e42cad0e27cbc1b5aefb4d1d527251912afaf6/hook.js",
  "function_signature_candidates": []
}

```text

#!/usr/bin/env node
'use strict';

// ============================================================================
// claude-says — Claude Code Stop hook
//
// Claude Code invokes this script every time the assistant finishes a turn.
// Hook payload arrives on stdin as JSON, e.g.:
//   { "session_id":"...", "transcript_path":"/path/to/transcript.jsonl",
//     "cwd":"...", "hook_event_name":"Stop" }
//
// We:
//   1. Read the transcript JSONL
//   2. Find the last assistant text content (defensively across shapes)
//   3. POST it to the running claude-says app on 127.0.0.1:7777
//
// If the app isn't running, we fail silently — never block Claude Code.
// ============================================================================

const fs   = require('fs');
const path = require('path');
const os   = require('os');

const HARD_TIMEOUT_MS = 5000;
// File the Electron app polls for new lines to speak. Local I/O is far more
// reliable than HTTP — Claude Code can't mark this hook as "slow" because
// we never wait on a network round-trip; we just write and exit.
const SPOOL_PATH = path.join(os.tmpdir(), 'claude-says-spool.json');

// Debug log so we can see what Claude Code is feeding the hook and what
// text we extracted. Comment out / remove after debugging.
const LOG_PATH = path.join(os.tmpdir(), 'claude-says-hook.log');
function debugLog(msg) {
  try {
    fs.appendFileSync(LOG_PATH, `[${new Date().toISOString()}] ${msg}\n`);
  } catch (_) {}
}

let stdinBuf = '';
process.stdin.setEncoding('utf8');
process.stdin.on('data', chunk => { stdinBuf += chunk; });
process.stdin.on('end', handlePayload);

// safety net — if stdin never closes for some reason, exit so we don't hang
setTimeout(() => process.exit(0), HARD_TIMEOUT_MS);

function handlePayload() {
  debugLog(`hook fired; stdin bytes=${stdinBuf.length}`);
  let payload;
  try { payload = JSON.parse(stdinBuf || '{}'); }
  catch (e) { debugLog(`bad payload json: ${e.message}`); return process.exit(0); }

  const tp = payload && payload.transcript_path;
  debugLog(`transcript_path=${tp}`);
  if (!tp || !fs.existsSync(tp)) {
    debugLog(`transcript missing — exiting`);
    return process.exit(0);
  }

  let raw;
  try { raw = fs.readFileSync(tp, 'utf8'); }
  catch (e) { debugLog(`read fail: ${e.message}`); return process.exit(0); }

  const lines = raw.split('\n').filter(Boolean);
  debugLog(`transcript lines=${lines.length}`);
  const text = extractLastAssistantText(lines);
  debugLog(`extracted text (first 200): ${(text || '').slice(0, 200)}`);
  if (!text) {
    debugLog(`no text in current turn — skipping`);
    return process.exit(0);
  }

  // Skip duplicates — Claude Code can fire Stop multiple times per turn.
  const last = getLastPosted();
  if (text === last) {
    debugLog(`duplicate of last posted — skipping`);
    return process.exit(0);
  }
  setLastPosted(text);

  writeSpool(text);
  debugLog(`spooled ${text.length} chars`);
  process.exit(0);
}

// Atomic write to the spool file: write to a temp path then rename, so the
// Electron-side watcher never reads a half-written file.
function writeSpool(text) {
  const tmp = SPOOL_PATH + '.tmp.' + process.pid;
  try {
    fs.writeFileSync(tmp, JSON.stringify({
      text,
      source: 'claude-code',
      ts: Date.now() + Math.random(),
    }));
    fs.renameSync(tmp, SPOOL_PATH);
  } catch (e) {
    debugLog(`spool write failed: ${e.message}`);
  }
}

// Walks the JSONL backwards looking for the most recent assistant TEXT
// content within the CURRENT TURN. Each model invocation becomes its own
// JSONL line — many are thinking-only or tool_use-only (no text). We need
// to stop at a user message or tool_result so we don't accidentally
// surface text from an OLD turn when the latest entries are silent.
function extractLastAssistantText(lines) {
  // Just walk back and find the most recent assistant entry that has a
  // non-empty TEXT block. Don't try to detect turn boundaries — Claude Code
  // spawns Stop hooks asynchronously, so by the time we run the user's NEXT
  // message may already be in the transcript and would falsely stop us.
  // The dedupe file (claude-says-last.txt) prevents re-speaking the same
  // text from a previous turn.
  for (let i = lines.length - 1; i >= 0; i--) {
    let entry;
    try { entry = JSON.parse(lines[i]); } catch (_) { continue; }

    const role = entry.role || (entry.message && entry.message.role) || entry.type;
    if (role !== 'assistant' && entry.type !== 'assistant') continue;

    const content = entry.content != null
      ? entry.content
      : (entry.message && entry.message.content);

    let text = '';
    if (typeof content === 'string') {
      text = content;
    } else if (Array.isArray(content)) {
      // Only explicit text blocks — exclude thinking, tool_use, etc.
      text = content
        .filter(c => c && c.type === 'text' && typeof c.text === 'string')
        .map(c => c.text || '')
        .join('\n');
    }

    text = text && text.trim();
    if (text) return text;
  }
  return '';
}

// Avoid re-speaking the same text on duplicate Stop fires.
const LAST_POST_PATH = path.join(os.tmpdir(), 'claude-says-last.txt');
function getLastPosted() {
  try { return fs.readFileSync(LAST_POST_PATH, 'utf8'); } catch (_) { return ''; }
}
function setLastPosted(text) {
  try { fs.writeFileSync(LAST_POST_PATH, text); } catch (_) {}
}

// (HTTP path removed — see writeSpool above. The Electron app's HTTP server
// is still kept for direct curl testing, but the production hook path is the
// fast local-file spool.)


```

## Source file: main.js

{
  "path": "main.js",
  "blob_sha": "2c1ecfd487b890818e36b42b83ea22ec970af78f",
  "sha256": "062dbcd4380195b4dd272c173cf0fe329f0d9a5a05835aa08021b62545ba4296",
  "lines": 639,
  "bytes": 23928,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 1,
    "face": 1,
    "lip": 2
  },
  "url": "https://github.com/SkyeShark/claudes-body/blob/b0e42cad0e27cbc1b5aefb4d1d527251912afaf6/main.js",
  "function_signature_candidates": []
}

```text

'use strict';

// ============================================================================
// claude-says — Electron main process
//
// Responsibilities:
//   1. Create a transparent, frameless, always-on-top BrowserWindow
//   2. Run a tiny HTTP server on 127.0.0.1:7777 that the Stop hook POSTs to
//   3. Forward incoming text to the renderer via IPC
//   4. Handle window-drag IPC from the renderer (we drive drag manually so we
//      can fire start/end events for animations)
//   5. Persist window position + user preferences across sessions
// ============================================================================

const { app, BrowserWindow, ipcMain, screen, globalShortcut } = require('electron');
const path = require('path');
const http = require('http');
const fs   = require('fs');
const os   = require('os');

const SERVER_PORT = 7777;
const WIN_WIDTH   = 360;
const WIN_HEIGHT  = 420;

// File the Stop hook writes new lines into. We poll it 5×/sec and forward
// new content to the renderer. Polling instead of fs.watch because fs.watch
// is unreliable on Windows.
const SPOOL_PATH = path.join(os.tmpdir(), 'claude-says-spool.json');

// Claude Code's transcript files. The Stop hook is the preferred path, but
// Claude Code doesn't always invoke it (close-together turns can drop fires).
// We also tail the JSONL transcripts directly as a fallback so no assistant
// turn ever goes unspoken.
const PROJECTS_DIR = path.join(os.homedir(), '.claude', 'projects');

// Mutable size — updated when the renderer picks a new size in settings.
// Used by the move-window IPC so the size is enforced on every drag tick
// (Aero Snap / cross-DPI moves can otherwise change it).
let currentWidth  = WIN_WIDTH;
let currentHeight = WIN_HEIGHT;

// Single-instance: if user launches a second copy, focus the first.
if (!app.requestSingleInstanceLock()) {
  app.quit();
  process.exit(0);
}

// ---------- settings persistence ----------
function settingsFile() {
  const dir = app.getPath('userData');
  return path.join(dir, 'claude-says-state.json');
}
function loadState() {
  try { return JSON.parse(fs.readFileSync(settingsFile(), 'utf8')); }
  catch (_) { return {}; }
}
function saveState(state) {
  try {
    fs.mkdirSync(path.dirname(settingsFile()), { recursive: true });
    fs.writeFileSync(settingsFile(), JSON.stringify(state, null, 2));
  } catch (_) {}
}

// ---------- window ----------
let mainWindow = null;

function createWindow() {
  const state    = loadState();
  if (Number.isFinite(state.width))  currentWidth  = state.width;
  if (Number.isFinite(state.height)) currentHeight = state.height;
  const display  = screen.getPrimaryDisplay();
  const work     = display.workArea;
  const defaultX = work.x + work.width  - currentWidth  - 24;
  const defaultY = work.y + work.height - currentHeight - 24;

  // Icon-capture mode wants a square canvas so the mane fits without
  // horizontal clipping.
  const iconCapture = process.argv.includes('--capture-icon');
  if (iconCapture) { currentWidth = 512; currentHeight = 512; }
  mainWindow = new BrowserWindow({
    title: "Claude's Body",
    width:  currentWidth,
    height: currentHeight,
    x: Number.isFinite(state.x) ? state.x : defaultX,
    y: Number.isFinite(state.y) ? state.y : defaultY,
    frame: false,
    transparent: true,
    alwaysOnTop: true,
    icon: path.join(__dirname, 'build', 'icon.png'),
    resizable: false,
    hasShadow: false,
    skipTaskbar: false,
    minimizable: true,
    maximizable: false,
    fullscreenable: false,
    backgroundColor: '#00000000',
    // Don't grab keyboard focus. Otherwise keystrokes you mean for the
    // app underneath can route to this window when it briefly has focus
    // (after a click / alt-tab), and since nothing in the renderer
    // handles them Windows plays the system "ding". Mouse events still
    // work (focusable=false only denies keyboard focus), and the global
    // Ctrl+Shift+L lock toggle is registered via globalShortcut so it's
    // focus-agnostic.
    focusable: false,
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true
    }
  });

  // Float above fullscreen apps too (so we ride along with VS Code in fullscreen)
  mainWindow.setAlwaysOnTop(true, 'screen-saver');
  if (mainWindow.setVisibleOnAllWorkspaces) {
    mainWindow.setVisibleOnAllWorkspaces(true, { visibleOnFullScreen: true });
  }

  const indexPath = path.join(__dirname, 'renderer', 'index.html');
  const captureIcon = process.argv.includes('--capture-icon');
  console.log('[claude-says] loading', indexPath, captureIcon ? '(icon-capture mode)' : '');
  if (captureIcon) {
    mainWindow.loadFile(indexPath, { search: 'capture=icon' });
  } else {
    mainWindow.loadFile(indexPath);
  }

  // Surface renderer crashes / load failures
  mainWindow.webContents.on('did-fail-load', (_e, code, desc, url) => {
    console.error(`[claude-says] did-fail-load (${code}) ${desc} → ${url}`);
  });
  mainWindow.webContents.on('render-process-gone', (_e, details) => {
    console.error(`[claude-says] render-process-gone:`, details);
  });
  mainWindow.webContents.on('console-message', (_e, level, message, line, source) => {
    console.log(`[renderer ${level}] ${source}:${line} ${message}`);
  });

  // DevTools off by default. Set CLAUDE_SAYS_DEBUG=1 to turn back on.
  if (process.env.CLAUDE_SAYS_DEBUG === '1') {
    mainWindow.webContents.openDevTools({ mode: 'detach' });
  }

  mainWindow.on('moved', () => {
    if (!mainWindow || mainWindow.isDestroyed()) return;
    const [x, y] = mainWindow.getPosition();
    const s = loadState();
    s.x = x; s.y = y;
    saveState(s);
  });

  mainWindow.on('closed', () => { mainWindow = null; });
}

// ---------- spool watcher (preferred path; hook writes here, we poll) ----------
let spoolPollTimer  = null;
let spoolLastMtime  = 0;
let spoolLastTs     = 0;

function startSpoolWatcher() {
  // Initialize with the current mtime so we don't replay any stale event
  // that's already sitting in the spool from a previous session.
  try { spoolLastMtime = fs.statSync(SPOOL_PATH).mtimeMs; } catch (_) {}

  spoolPollTimer = setInterval(() => {
    let stat;
    try { stat = fs.statSync(SPOOL_PATH); }
    catch (_) { return; }                // file doesn't exist yet
    if (stat.mtimeMs <= spoolLastMtime) return;
    spoolLastMtime = stat.mtimeMs;

    let raw;
    try { raw = fs.readFileSync(SPOOL_PATH, 'utf8'); }
    catch (_) { return; }
    if (!raw || !raw.trim()) return;

    let data;
    try { data = JSON.parse(raw); }
    catch (_) { return; }

    if (!data || typeof data.text !== 'string' || !data.text.trim()) return;
    if (data.ts && data.ts === spoolLastTs) return;
    spoolLastTs = data.ts;

    sendSay(data.text.trim(), data.source || 'spool');
  }, 200);
}

// ---------- transcript watcher (fallback for missed Stop hook fires) ----------
// Walks ~/.claude/projects/**/*.jsonl, picks the most-recently-modified file,
// and extracts the latest assistant text. Sends to renderer via the same IPC
// path the spool uses. We dedupe against `lastSpokenText` so a single turn
// can't be spoken twice (whichever source — hook or transcript — wins).
let transcriptPollTimer = null;
let transcriptLastFile  = '';
let transcriptLastMtime = 0;
let transcriptLastUserIdx = -1;
let lastSpokenText      = '';

function listJsonlFiles(dir) {
  const out = [];
  let entries;
  try { entries = fs.readdirSync(dir, { withFileTypes: true }); }
  catch (_) { return out; }
  for (const e of entries) {
    const full = path.join(dir, e.name);
    if (e.isDirectory()) {
      out.push(...listJsonlFiles(full));
    } else if (e.isFile() && e.name.endsWith('.jsonl')) {
      out.push(full);
    }
  }
  return out;
}

function findActiveTranscript() {
  const files = listJsonlFiles(PROJECTS_DIR);
  let best = null;
  let bestMtime = 0;
  for (const f of files) {
    let stat;
    try { stat = fs.statSync(f); } catch (_) { continue; }
    if (stat.mtimeMs > bestMtime) {
      bestMtime = stat.mtimeMs;
      best = f;
    }
  }
  return best ? { file: best, mtime: bestMtime } : null;
}

// Returns { text, lastUserIdx } — text is the most recent assistant text block
// AFTER the last genuine user message (so we don't replay stale text from a
// previous turn while the user is waiting for the new one). lastUserIdx is the
// line index of the last user message; renderer uses it to know "user just
// spoke, hush whatever you were saying".
function scanTranscript(raw) {
  const lines = raw.split('\n');
  let lastUserIdx = -1;
  // First pass: find the last real user message (not a tool_result).
  for (let i = lines.length - 1; i >= 0; i--) {
    const ln = lines[i]; if (!ln) continue;
    let entry; try { entry = JSON.parse(ln); } catch (_) { continue; }
    const role = entry.role || (entry.message && entry.message.role) || entry.type;
    if (role !== 'user' && entry.type !== 'user') continue;
    const content = entry.content != null
      ? entry.content
      : (entry.message && entry.message.content);
    // tool_result entries are also "user" role; only count entries that have
    // a real text block from a human (or a string content).
    let isHumanMsg = false;
    if (typeof content === 'string' && content.trim()) isHumanMsg = true;
    else if (Array.isArray(content)) {
      isHumanMsg = content.some(c => c && c.type === 'text' && typeof c.text === 'string' && c.text.trim());
    }
    if (isHumanMsg) { lastUserIdx = i; break; }
  }
  // Second pass: walk back from end, but stop at lastUserIdx — only assistant
  // text strictly AFTER the user's latest message counts.
  let text = '';
  for (let i = lines.length - 1; i > lastUserIdx; i--) {
    const ln = lines[i]; if (!ln) continue;
    let entry; try { entry = JSON.parse(ln); } catch (_) { continue; }
    const role = entry.role || (entry.message && entry.message.role) || entry.type;
    if (role !== 'assistant' && entry.type !== 'assistant') continue;
    const content = entry.content != null
      ? entry.content
      : (entry.message && entry.message.content);
    let t = '';
    if (typeof content === 'string') t = content;
    else if (Array.isArray(content)) {
      t = content
        .filter(c => c && c.type === 'text' && typeof c.text === 'string')
        .map(c => c.text || '')
        .join('\n');
    }
    t = t && t.trim();
    if (t) { text = t; break; }
  }
  return { text, lastUserIdx };
}

// Ring buffer of recently-spoken texts. We dedupe across the whole window,
// not just the most recent — otherwise a late-firing Stop hook for an older
// turn can overwrite the current turn's audio. 8 entries is plenty: it covers
// several turns of pipeline lag without growing unbounded.
const RECENT_SIZE = 8;
const recentSpoken = [];

function sendSay(text, source) {
  if (!text) return;
  if (recentSpoken.includes(text)) return;
  recentSpoken.push(text);
  if (recentSpoken.length > RECENT_SIZE) recentSpoken.shift();
  lastSpokenText = text;
  if (mainWindow && !mainWindow.isDestroyed()) {
    mainWindow.webContents.send('say', { text, source, ts: Date.now() });
  }
}

function startTranscriptWatcher() {
  // Seed lastSpokenText with whatever's currently in the active transcript so
  // we don't replay the most recent message on launch.
  const active = findActiveTranscript();
  if (active) {
    transcriptLastFile  = active.file;
    transcriptLastMtime = active.mtime;
    try {
      const raw = fs.readFileSync(active.file, 'utf8');
      const scan = scanTranscript(raw);
      lastSpokenText        = scan.text;
      transcriptLastUserIdx = scan.lastUserIdx;
      if (scan.text) recentSpoken.push(scan.text);
    } catch (_) {}
  }

  transcriptPollTimer = setInterval(() => {
    const active = findActiveTranscript();
    if (!active) return;
    if (active.file === transcriptLastFile && active.mtime <= transcriptLastMtime) return;
    transcriptLastFile  = active.file;
    transcriptLastMtime = active.mtime;
    let raw;
    try { raw = fs.readFileSync(active.file, 'utf8'); }
    catch (_) { return; }
    const { text, lastUserIdx } = scanTranscript(raw);
    // New user message → hush whatever the character was saying. The previous
    // turn's audio is now stale conversational context.
    if (lastUserIdx > transcriptLastUserIdx) {
      transcriptLastUserIdx = lastUserIdx;
      if (mainWindow && !mainWindow.isDestroyed()) {
        mainWindow.webContents.send('hush');
      }
    }
    if (text) sendSay(text, 'transcript');
  }, 500);
}

// ---------- HTTP server (kept for direct curl testing / external integrations) ----------
let httpServer = null;

function startHttpServer() {
  httpServer = http.createServer((req, res) => {
    if (req.method === 'POST' && req.url === '/say') {
      let body = '';
      req.on('data', chunk => {
        body += chunk;
        if (body.length > 200000) { req.destroy(); }
      });
      req.on('end', () => {
        try {
          const payload = JSON.parse(body || '{}');
          if (mainWindow && !mainWindow.isDestroyed()) {
            mainWindow.webContents.send('say', payload);
          }
          res.writeHead(200, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ ok: true }));
        } catch (e) {
          res.writeHead(400, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ ok: false, error: String(e && e.message) }));
        }
      });
      return;
    }

    if (req.method === 'GET' && req.url === '/health') {
      res.writeHead(200, { 'Content-Type': 'text/plain' });
      res.end('ok');
      return;
    }

    // Returns a PNG screenshot of the current renderer state. Used for
    // iterating on the 3D rig from the assistant side without a human in
    // the loop — capture, inspect via image read, adjust, repeat.
    if (req.method === 'GET' && req.url === '/screenshot') {
      if (!mainWindow || mainWindow.isDestroyed()) {
        res.writeHead(503); res.end('no window'); return;
      }
      mainWindow.webContents.capturePage().then(image => {
        const png = image.toPNG();
        res.writeHead(200, { 'Content-Type': 'image/png', 'Content-Length': png.length });
        res.end(png);
      }).catch(err => {
        res.writeHead(500, { 'Content-Type': 'text/plain' });
        res.end(String(err && err.message));
      });
      return;
    }

    res.writeHead(404);
    res.end();
  });

  httpServer.on('error', (err) => {
    if (err && err.code === 'EADDRINUSE') {
      console.warn(`[claude-says] port ${SERVER_PORT} already in use; another instance may be running.`);
    } else {
      console.error('[claude-says] server error:', err);
    }
  });

  httpServer.listen(SERVER_PORT, '127.0.0.1', () => {
    console.log(`[claude-says] listening on http://127.0.0.1:${SERVER_PORT}`);
  });
}

// ---------- IPC handlers (renderer ↔ main) ----------
ipcMain.handle('get-window-position', () => {
  if (!mainWindow || mainWindow.isDestroyed()) return [0, 0];
  return mainWindow.getPosition();
});

// Force the window to its canonical size on every move so nothing (Aero Snap,
// DPI rescale crossing monitors, etc.) can make the character "grow" between drags.
ipcMain.on('move-window', (_event, x, y) => {
  if (!mainWindow || mainWindow.isDestroyed()) return;
  mainWindow.setBounds({
    x: Math.round(x),
    y: Math.round(y),
    width:  currentWidth,
    height: currentHeight,
  }, false);
});

// Settings panel size selector
ipcMain.on('set-window-size', (_event, w, h) => {
  if (!mainWindow || mainWindow.isDestroyed()) return;
  currentWidth  = Math.round(w);
  currentHeight = Math.round(h);
  const [x, y] = mainWindow.getPosition();
  mainWindow.setBounds({ x, y, width: currentWidth, height: currentHeight }, false);
  const s = loadState();
  s.width = currentWidth; s.height = currentHeight;
  saveState(s);
});

ipcMain.on('quit', () => app.quit());

// Click-through toggle. The renderer flips this based on whether the cursor is
// over the character body (or controls). When ignoring, mouse events fall
// through to whatever's underneath the floater on the desktop. `forward: true`
// keeps mousemove events flowing to the renderer so we can keep hit-testing.
ipcMain.on('set-ignore-mouse', (_event, ignore, options) => {
  if (!mainWindow || mainWindow.isDestroyed()) return;
  mainWindow.setIgnoreMouseEvents(!!ignore, options || { forward: true });
});

ipcMain.on('minimize', () => {
  if (mainWindow && !mainWindow.isDestroyed()) mainWindow.minimize();
});

ipcMain.handle('get-state',  ()       => loadState());
ipcMain.handle('set-state',  (_e, s)  => { saveState(s); return true; });

// ---------- Kokoro TTS (out-of-process) ----------
// onnxruntime-node's session.run blocks the Node event loop during
// inference. In the Electron main process that means mouse, IPC, and
// audio playback all stutter for several seconds per synth. Run it
// in a separate child process instead and JSON-RPC over stdio.
const KOKORO_VOICES = { male: 'am_michael', female: 'af_bella' };
let kokoroProc = null;
let kokoroReady = false;
const kokoroPending = new Map();   // id → { resolve, reject }
let kokoroNextId = 1;
let kokoroLineBuf = '';

function spawnKokoro() {
  if (kokoroProc) return;
  const { spawn } = require('child_process');
  const workerPath = path.join(__dirname, 'tools', 'kokoro-worker.mjs');
  console.log('[kokoro] spawning worker:', workerPath);
  // Cap ONNX runtime / OpenMP thread count so the synth doesn't peg
  // every core on the machine while the user is doing other work.
  // Four threads keeps RTF around ~0.3 — fast enough that even when
  // a long chunk follows a short one, the long one finishes synthing
  // before the short one's audio ends (no gap). Most modern desktops
  // have 8+ logical cores, so leaving 4+ free preserves system
  // responsiveness in other apps.
  const SYNTH_THREADS = '4';
  kokoroProc = spawn(process.execPath, [workerPath], {
    cwd: __dirname,
    stdio: ['pipe', 'pipe', 'pipe'],
    env: {
      ...process.env,
      ELECTRON_RUN_AS_NODE: '1',
      OMP_NUM_THREADS:        SYNTH_THREADS,
      ORT_NUM_THREADS:        SYNTH_THREADS,
      MKL_NUM_THREADS:        SYNTH_THREADS,
      OPENBLAS_NUM_THREADS:   SYNTH_THREADS,
    },
  });
  kokoroProc.stdout.on('data', (chunk) => {
    kokoroLineBuf += chunk.toString('utf8');
    let idx;
    while ((idx = kokoroLineBuf.indexOf('\n')) >= 0) {
      const line = kokoroLineBuf.slice(0, idx);
      kokoroLineBuf = kokoroLineBuf.slice(idx + 1);
      if (!line.trim()) continue;
      let msg;
      try { msg = JSON.parse(line); }
      catch (_) { continue; }
      const handler = kokoroPending.get(msg.id);
      if (!handler) continue;
      kokoroPending.delete(msg.id);
      if (msg.ok) handler.resolve(msg.path);
      else        handler.reject(new Error(msg.error || 'kokoro error'));
    }
  });
  kokoroProc.stderr.on('data', (d) => {
    const txt = d.toString();
    process.stderr.write('[kokoro-worker] ' + txt);
    if (txt.includes('[kokoro-worker] ready')) kokoroReady = true;
  });
  kokoroProc.on('exit', (code) => {
    console.log('[kokoro] worker exited', code);
    kokoroProc = null;
    kokoroReady = false;
    for (const [, h] of kokoroPending) h.reject(new Error('kokoro worker died'));
    kokoroPending.clear();
  });
}
function kokoroSynth(text, voice) {
  return new Promise((resolve, reject) => {
    if (!kokoroProc) spawnKokoro();
    const id = kokoroNextId++;
    kokoroPending.set(id, { resolve, reject });
    kokoroProc.stdin.write(JSON.stringify({ id, text, voice }) + '\n');
  });
}
// Warm the worker on app ready so the first synth doesn't pay the
// model-load penalty.
app.whenReady().then(() => { spawnKokoro(); });

ipcMain.handle('tts-available', () => kokoroReady);
ipcMain.handle('tts-synth', async (_event, text, gender) => {
  if (!text || !text.trim()) return null;
  if (!kokoroReady) return null;
  const voice = KOKORO_VOICES[gender] || KOKORO_VOICES.male;
  console.log('[kokoro] synth gender=', gender, 'voice=', voice, 'text=', JSON.stringify(text));
  try {
    const t0 = Date.now();
    const wavPath = await kokoroSynth(text, voice);
    if (!wavPath) return null;
    const sz = (() => { try { return fs.statSync(wavPath).size; } catch (_) { return 0; } })();
    console.log('[kokoro] synth ok:', sz, 'bytes,', (Date.now() - t0) + 'ms,', path.basename(wavPath));
    // Schedule deletion after the audio's almost certainly played out.
    // 60s is conservative for any sentence-length response.
    setTimeout(() => { try { fs.unlinkSync(wavPath); } catch (_) {} }, 60000);
    // Return as a file:// URL the renderer can use directly with <Audio>.
    // pathToFileURL handles the Windows backslash / drive-letter mangling.
    return require('url').pathToFileURL(wavPath).href;
  } catch (e) {
    console.error('[kokoro] synth failed:', e?.message);
    return null;
  }
});

// Save the icon dataURL to build/icon.png and exit. Used by the
// `npm run capture-icon` flow.
ipcMain.on('save-icon-png', (_event, dataUrl) => {
  try {
    const m = /^data:image\/png;base64,(.+)$/.exec(dataUrl);
    if (!m) return;
    const dir = path.join(__dirname, 'build');
    fs.mkdirSync(dir, { recursive: true });
    const outPath = path.join(dir, 'icon.png');
    fs.writeFileSync(outPath, Buffer.from(m[1], 'base64'));
    console.log('[icon] saved', outPath, 'bytes=', m[1].length * 0.75 | 0);
    setTimeout(() => app.quit(), 500);
  } catch (e) {
    console.error('[icon] save failed:', e);
  }
});

// Debug-only: write a PNG dataURL from the renderer to disk.
ipcMain.on('save-debug-frame', (_event, name, dataUrl) => {
  try {
    const m = /^data:image\/png;base64,(.+)$/.exec(dataUrl);
    if (!m) return;
    const dir = path.join(os.tmpdir(), 'claude-says-debug');
    fs.mkdirSync(dir, { recursive: true });
    const safeName = String(name || 'frame').replace(/[^\w.-]/g, '_');
    fs.writeFileSync(path.join(dir, safeName + '.png'), Buffer.from(m[1], 'base64'));
  } catch (e) {
    console.error('[debug] saveDebugFrame failed:', e);
  }
});

// ---------- app lifecycle ----------
app.on('second-instance', () => {
  if (mainWindow && !mainWindow.isDestroyed()) {
    mainWindow.show();
    mainWindow.focus();
  }
});

app.setName("Claude's Body");
if (process.platform === 'win32') {
  app.setAppUserModelId('com.claudesbody.app');
}

app.whenReady().then(() => {
  startHttpServer();
  startSpoolWatcher();
  startTranscriptWatcher();
  createWindow();

  // Global hotkey to toggle "lock" mode (click-through everywhere). Lets the
  // user grab the character even when locked-and-passive, since clicks alone
  // can't reach a click-through window.
  const ok = globalShortcut.register('CommandOrControl+Shift+L', () => {
    if (mainWindow && !mainWindow.isDestroyed()) {
      mainWindow.webContents.send('toggle-lock');
    }
  });
  if (!ok) console.warn('[claude-says] could not register Ctrl/Cmd+Shift+L hotkey');

  // On Mac, hide from the dock — we're a floating utility, not an app the user
  // alt-tabs to. Comment this out if you want it to appear in the dock.
  if (process.platform === 'darwin' && app.dock && app.dock.hide) {
    try { app.dock.hide(); } catch (_) {}
  }
});

app.on('will-quit', () => {
  try { globalShortcut.unregisterAll(); } catch (_) {}
  if (spoolPollTimer)      try { clearInterval(spoolPollTimer); }      catch (_) {}
  if (transcriptPollTimer) try { clearInterval(transcriptPollTimer); } catch (_) {}
});

app.on('window-all-closed', () => {
  if (httpServer) {
    try { httpServer.close(); } catch (_) {}
  }
  if (process.platform !== 'darwin') app.quit();
});

app.on('activate', () => {
  if (BrowserWindow.getAllWindows().length === 0) createWindow();
});


```
