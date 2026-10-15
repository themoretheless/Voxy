# Facial rendering source scan: Akashkandari7/LipSync-3d-Videos

Repository: https://github.com/Akashkandari7/LipSync-3d-Videos

Pinned source revision: edcca1c8ed37aa351853935652e4a474b9311d38

Discovery topic: lip sync 3D

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"face": ["whichface.py"], "lip": ["lipsync.py"]}

## Source file: lipsync.py

{
  "path": "lipsync.py",
  "blob_sha": "c45baaefe942637bb8676fef0bc5a486cb90f835",
  "sha256": "dca532bbf535f73b62c3bde31183cbf7427e8e6a6300477495fb489c02884b2f",
  "lines": 228,
  "bytes": 9573,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 11,
    "lip": 4
  },
  "url": "https://github.com/Akashkandari7/LipSync-3d-Videos/blob/edcca1c8ed37aa351853935652e4a474b9311d38/lipsync.py",
  "function_signature_candidates": [
    {
      "line": 35,
      "text": "def key():"
    },
    {
      "line": 44,
      "text": "def req(method, url, headers=None, body=None, raw=None):"
    },
    {
      "line": 59,
      "text": "def ffprobe(path):"
    },
    {
      "line": 68,
      "text": "def rate(k):"
    },
    {
      "line": 85,
      "text": "def extract(src, start_frame, end_frame, outdir, fps=30):"
    },
    {
      "line": 105,
      "text": "def upload(path):"
    },
    {
      "line": 118,
      "text": "def submit(video_url, audio_url, idem, speaker=None):"
    },
    {
      "line": 132,
      "text": "def poll(gid, timeout=1800):"
    },
    {
      "line": 146,
      "text": "def validate(original, produced):"
    },
    {
      "line": 176,
      "text": "def save_job(rec):"
    },
    {
      "line": 183,
      "text": "def main():"
    }
  ]
}

```text

#!/usr/bin/env python3
"""Blender lip-sync automation - segment pipeline.

Extracts a frame range from a master render, sends only that chunk to
sync.so, validates the result, and reports whether it is safe to splice
back into the timeline.

Design rule: never let an AI result damage a valid original. Any check
that fails leaves the original segment in place.
"""

import json
import os
import subprocess
import sys
import time
import uuid
from pathlib import Path

ROOT = Path(__file__).parent
META = ROOT / "metadata"
JOBS = META / "jobs.json"
API = "https://api.sync.so/v2"

# Settings copied from the team's existing generations, not invented.
MODEL = "sync-3"
OPTIONS = {
    "pads": [0, 5, 0, 0],
    "sync_mode": "bounce",
    "output_format": "mp4",
    "active_speaker_detection": {"auto_detect": False},
}


def key():
    for line in (ROOT / ".env").read_text().splitlines():
        if line.startswith("SYNC_API_KEY="):
            k = line.split("=", 1)[1].strip().strip("\"'")
            if k and k != "yahan_apni_key_paste_karo":
                return k
    sys.exit("SYNC_API_KEY missing in .env")


def req(method, url, headers=None, body=None, raw=None):
    import urllib.request

    # A default Python-urllib User-Agent gets a 403 from sync.so's WAF.
    h = {"x-api-key": key(), "user-agent": "blender-lipsync/1.0"}
    h.update(headers or {})
    data = raw if raw is not None else (json.dumps(body).encode() if body else None)
    if body is not None:
        h["content-type"] = "application/json"
    r = urllib.request.Request(url, data=data, headers=h, method=method)
    with urllib.request.urlopen(r) as resp:
        payload = resp.read()
    return json.loads(payload) if payload[:1] in b"{[" else payload


def ffprobe(path):
    """Return the facts that matter for splicing: frames, fps, size, duration."""
    out = subprocess.run(
        ["ffprobe", "-v", "error", "-select_streams", "v:0", "-count_frames",
         "-show_entries", "stream=nb_read_frames,r_frame_rate,avg_frame_rate,width,height,duration",
         "-of", "json", str(path)],
        capture_output=True, text=True, check=True).stdout
    s = json.loads(out)["stream"][0] if "stream" in json.loads(out) else json.loads(out)["streams"][0]

    def rate(k):
        n, d = s.get(k, "0/1").split("/")
        return round(int(n) / int(d), 4) if int(d) else 0.0

    return {
        "frames": int(s.get("nb_read_frames", 0)),
        "fps": rate("r_frame_rate"),
        # sync.so has returned a clip with the right frame count but a 29fps
        # average, which concat then padded with a duplicate frame. Nominal
        # fps alone does not catch that; the average does.
        "avg_fps": rate("avg_frame_rate"),
        "width": int(s["width"]),
        "height": int(s["height"]),
        "duration": float(s.get("duration", 0)),
    }


def extract(src, start_frame, end_frame, outdir, fps=30):
    """Cut [start_frame, end_frame] inclusive, frame-exact, no timing drift."""
    outdir = Path(outdir)
    outdir.mkdir(parents=True, exist_ok=True)
    t0, t1 = start_frame / fps, (end_frame + 1) / fps
    vid, wav = outdir / "seg.mp4", outdir / "seg.wav"
    subprocess.run(
        ["ffmpeg", "-v", "error", "-i", str(src),
         "-vf", f"select='between(n,{start_frame},{end_frame})',setpts=PTS-STARTPTS",
         "-af", f"aselect='between(t,{t0},{t1})',asetpts=PTS-STARTPTS",
         "-r", str(fps), "-c:v", "libx264", "-crf", "12", "-preset", "medium",
         "-pix_fmt", "yuv420p", "-c:a", "aac", "-b:a", "192k", "-ar", "48000",
         "-movflags", "+faststart", "-y", str(vid)], check=True)
    subprocess.run(
        ["ffmpeg", "-v", "error", "-i", str(src),
         "-af", f"aselect='between(t,{t0},{t1})',asetpts=PTS-STARTPTS",
         "-vn", "-c:a", "pcm_s16le", "-ar", "48000", "-y", str(wav)], check=True)
    return vid, wav


def upload(path):
    """Two-step: ask for a presigned target, PUT the bytes, return the asset url."""
    path = Path(path)
    ctype = "video/mp4" if path.suffix == ".mp4" else "audio/wav"
    pre = req("POST", f"{API}/assets/upload", body={
        "fileName": path.name, "contentType": ctype, "size": path.stat().st_size})
    # curl, not urllib: the presigned URL signs content-length;host and urllib's
    # extra headers make S3 reject the PUT with 403.
    subprocess.run(["curl", "-sf", "-X", "PUT", "--upload-file", str(path),
                    pre["uploadUrl"]], check=True)
    return pre["url"]


def submit(video_url, audio_url, idem, speaker=None):
    """speaker: {"auto_detect": False, "frame_number": n, "coordinates": [x, y]}
    in the master's native pixel space. The API is snake_case here - the docs
    show camelCase, which it rejects outright."""
    opts = dict(OPTIONS)
    if speaker:
        opts["active_speaker_detection"] = speaker
    return req("POST", f"{API}/generate",
               headers={"x-idempotency-key": idem},
               body={"model": MODEL, "options": opts,
                     "input": [{"type": "video", "url": video_url},
                               {"type": "audio", "url": audio_url}]})


def poll(gid, timeout=1800):
    t0 = time.time()
    last = None
    while time.time() - t0 < timeout:
        g = req("GET", f"{API}/generations/{gid}")
        if g["status"] != last:
            last = g["status"]
            print(f"  [{int(time.time()-t0):4d}s] {last}", flush=True)
        if last in ("COMPLETED", "FAILED", "REJECTED", "CANCELLED"):
            return g
        time.sleep(10)
    raise TimeoutError(gid)


def validate(original, produced):
    """Gate before splicing. Every failure means: keep the original."""
    a, b = ffprobe(original), ffprobe(produced)
    # Frame count and geometry are the invariants: the compositor reads
    # frames sequentially and the splice rebuilds every timestamp from the
    # frame index, so a clip that carries the right frames with wrong timing
    # metadata is still usable. sync.so returns those regularly - 29fps on
    # one video, 32.3fps on the next - so they are reported, not fatal.
    timing_ok = (a["fps"] == b["fps"]
                 and abs(a["avg_fps"] - b["avg_fps"]) < 0.5
                 and abs(a["duration"] - b["duration"]) < 0.05)
    checks = {
        "frame_count": (a["frames"], b["frames"], a["frames"] == b["frames"]),
        "width": (a["width"], b["width"], a["width"] == b["width"]),
        "height": (a["height"], b["height"], a["height"] == b["height"]),
    }
    black = subprocess.run(
        ["ffmpeg", "-v", "info", "-i", str(produced),
         "-vf", "blackdetect=d=0.03:pic_th=0.98", "-an", "-f", "null", "-"],
        capture_output=True, text=True).stderr
    n_black = black.count("black_start")
    checks["no_black_frames"] = (0, n_black, n_black == 0)
    if not timing_ok:
        print(f"      note: timing metadata differs "
              f"(fps {a['fps']}->{b['fps']}, avg {a['avg_fps']}->{b['avg_fps']}, "
              f"dur {a['duration']:.3f}->{b['duration']:.3f}); frames match so "
              f"the splice will rebuild it", flush=True)
    return checks, a, b


def save_job(rec):
    META.mkdir(exist_ok=True)
    jobs = json.loads(JOBS.read_text()) if JOBS.exists() else {}
    jobs[rec["segment_id"]] = rec
    JOBS.write_text(json.dumps(jobs, indent=1))


def main():
    cmd = sys.argv[1] if len(sys.argv) > 1 else "help"
    if cmd == "probe":
        print(json.dumps(ffprobe(sys.argv[2]), indent=1))
    elif cmd == "extract":
        src, sf, ef, out = sys.argv[2], int(sys.argv[3]), int(sys.argv[4]), sys.argv[5]
        v, w = extract(src, sf, ef, out)
        print(json.dumps(ffprobe(v), indent=1))
    elif cmd == "upload":
        print(upload(sys.argv[2]))
    elif cmd == "run":
        vid, wav, seg_id = sys.argv[2], sys.argv[3], sys.argv[4]
        idem = str(uuid.uuid5(uuid.NAMESPACE_URL, f"{seg_id}:{vid}"))
        print("uploading video..."); v = upload(vid)
        print("uploading audio..."); a = upload(wav)
        print(f"submitting {MODEL} (idempotency {idem[:8]})...")
        g = submit(v, a, idem)
        gid = g["id"]
        print("generation:", gid)
        save_job({"segment_id": seg_id, "generation_id": gid, "status": g["status"],
                  "idempotency_key": idem, "video_url": v, "audio_url": a})
        g = poll(gid)
        save_job({"segment_id": seg_id, "generation_id": gid, "status": g["status"],
                  "idempotency_key": idem, "video_url": v, "audio_url": a,
                  "output_duration": g.get("outputDuration"),
                  "error": g.get("error"), "errorCode": g.get("errorCode")})
        if g["status"] != "COMPLETED":
            sys.exit(f"{g['status']}: {g.get('error')} / {g.get('errorCode')} -> KEEP ORIGINAL")
        out = Path(vid).parent.parent.parent / "output" / seg_id
        out.mkdir(parents=True, exist_ok=True)
        dst = out / "synced.mp4"
        subprocess.run(["curl", "-sfL", "-o", str(dst), g.get("outputMediaUrl") or g["outputUrl"]], check=True)
        print("downloaded:", dst)
    elif cmd == "validate":
        checks, a, b = validate(sys.argv[2], sys.argv[3])
        for k, (exp, got, ok) in checks.items():
            print(f"  {'PASS' if ok else 'FAIL'}  {k:16} expected={exp} got={got}")
        print("\nVERDICT:", "SAFE TO SPLICE" if all(c[2] for c in checks.values())
              else "REJECT - KEEP ORIGINAL")
    else:
        print(__doc__)
        print("commands: probe <f> | extract <src> <s> <e> <out> | upload <f> | validate <orig> <new>")


if __name__ == "__main__":
    main()


```

## Source file: whichface.py

{
  "path": "whichface.py",
  "blob_sha": "ee6327ffc4de8c92dda15e5f6ce7b6a9f0e86c5c",
  "sha256": "2e9ff00cae4194cb24a94eaa945754e8b1dbec46a8356bedd1c7347f7bdfb474",
  "lines": 32,
  "bytes": 1633,
  "selection_score": 15,
  "keyword_line_counts": {
    "mouth": 1,
    "face": 5
  },
  "url": "https://github.com/Akashkandari7/LipSync-3d-Videos/blob/edcca1c8ed37aa351853935652e4a474b9311d38/whichface.py",
  "function_signature_candidates": [
    {
      "line": 4,
      "text": "def rd(p):"
    }
  ]
}

```text

import subprocess,numpy as np,cv2,sys,json
W,H=540,960
DET=cv2.FaceDetectorYN.create('models/yunet.onnx','',(W,H),0.6,0.3,5000)
def rd(p):
    r=subprocess.run(['ffmpeg','-v','error','-i',p,'-fps_mode','passthrough','-vf',f'scale={W}:{H}',
        '-pix_fmt','bgr24','-f','rawvideo','-'],capture_output=True).stdout
    return np.frombuffer(r,np.uint8).reshape(-1,H,W,3).astype(np.int16)
sid=sys.argv[1]
i=rd(f'segments2/input/{sid}/seg.mp4')
o=rd(f'segments2/output/{sid}/synced_OLD.mp4')
n=rd(f'segments2/output/{sid}/synced.mp4')
m=min(len(i),len(o),len(n))
# collect face mouth boxes from a few sample frames of the INPUT
boxes={}
for f in range(0,m,max(1,m//8)):
    _,fs=DET.detect(np.ascontiguousarray(i[f].astype(np.uint8)))
    if fs is None: continue
    for fa in fs:
        rx,ry,lx,ly=fa[10],fa[11],fa[12],fa[13]
        cx,cy=(rx+lx)/2,(ry+ly)/2; half=max(abs(lx-rx),8.)
        key=(round(cx/60),round(cy/60))
        boxes.setdefault(key,[]).append((int(cx-half),int(cy-half*.7),int(cx+half),int(cy+half*.7)))
print(f'{sid}: {len(boxes)} face(s) found')
print(f"{'face @(x,y)':>16}{'OLD changed':>14}{'NEW changed':>14}  verdict")
for key,bs in sorted(boxes.items(), key=lambda kv:-len(kv[1])):
    b=np.median(np.array(bs),axis=0).astype(int)
    x0,y0,x1,y1=max(0,b[0]),max(0,b[1]),min(W,b[2]),min(H,b[3])
    if x1<=x0 or y1<=y0: continue
    do=np.abs(i[:m,y0:y1,x0:x1]-o[:m,y0:y1,x0:x1]).mean()
    dn=np.abs(i[:m,y0:y1,x0:x1]-n[:m,y0:y1,x0:x1]).mean()
    v='NEW syncs this face' if dn>do*1.25 else ('OLD synced this one' if do>dn*1.25 else 'similar')
    print(f"{str((x0,y0)):>16}{do:14.2f}{dn:14.2f}  {v}")


```

## Source file: run.py

{
  "path": "run.py",
  "blob_sha": "d7b597828232965ba1b6214f2beeac9a02cbc6ee",
  "sha256": "dddb4e15510b4c6fe453f9ad0baffaf70338290f7e05134a9da9b3206ce88e86",
  "lines": 664,
  "bytes": 28779,
  "selection_score": 0,
  "keyword_line_counts": {
    "mouth": 11,
    "gum": 6,
    "rig": 7,
    "face": 36,
    "lip": 8
  },
  "url": "https://github.com/Akashkandari7/LipSync-3d-Videos/blob/edcca1c8ed37aa351853935652e4a474b9311d38/run.py",
  "function_signature_candidates": [
    {
      "line": 40,
      "text": "def probe(src):"
    },
    {
      "line": 64,
      "text": "def work_dirs(cfg):"
    },
    {
      "line": 79,
      "text": "def transcribe(cfg):"
    },
    {
      "line": 100,
      "text": "def cuts_of(cfg, th=0.10):"
    },
    {
      "line": 117,
      "text": "def scan(cfg):"
    },
    {
      "line": 176,
      "text": "def build_plan(cfg, words, cuts, sc, min_face_pct, min_run, merge_gap):"
    },
    {
      "line": 203,
      "text": "def shot(f):"
    },
    {
      "line": 225,
      "text": "def speakers(cfg, plan):"
    },
    {
      "line": 273,
      "text": "def score(t):"
    },
    {
      "line": 310,
      "text": "def extract_all(cfg, plan):"
    },
    {
      "line": 337,
      "text": "def face_image(cfg, seg, frame_number, box):"
    },
    {
      "line": 351,
      "text": "def generate(cfg, plan):"
    },
    {
      "line": 352,
      "text": "def one(p):"
    },
    {
      "line": 391,
      "text": "def composite(cfg, plan, usable):"
    },
    {
      "line": 470,
      "text": "def splice(cfg, plan, usable):"
    },
    {
      "line": 508,
      "text": "def verify(cfg, plan, usable):"
    },
    {
      "line": 511,
      "text": "def y(path):"
    },
    {
      "line": 591,
      "text": "def count(path, filt, tok):"
    },
    {
      "line": 605,
      "text": "def main():"
    }
  ]
}

```text

#!/usr/bin/env python3
"""One command: video in, lip-synced video plus a QC report out.

    python run.py source/video4.mp4              # plan only, costs nothing
    python run.py source/video4.mp4 --go         # plan, then generate
    python run.py source/video4.mp4 --go --min-face 1.0

Everything this pipeline knows is enforced here rather than remembered.
The notes below each stage say which defect the code is guarding
against, because every one of them shipped at least once and passed the
checks that existed at the time.

What it does NOT do: retry on its own. A segment that fails QC is
reported with its credit cost and left alone until told otherwise.
"""

import argparse
import base64
import json
import subprocess
import sys
import uuid
import wave
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import cv2
import numpy as np

import lipsync as L

ROOT = Path(__file__).parent
RATE = 0.5333                     # credits per frame, sync-3, measured
DW, DH = 540, 960                 # face-detection scale
YUNET = ROOT / "models" / "yunet.onnx"


# ---------------------------------------------------------------- probe

def probe(src):
    """Read the master's real properties. Nothing downstream is hardcoded:
    one video was tagged iec61966-2-1 and the next bt709, and writing the
    wrong one back shifts the whole picture."""
    s = json.loads(subprocess.run(
        ["ffprobe", "-v", "error", "-select_streams", "v:0", "-count_frames",
         "-show_entries", "stream=width,height,r_frame_rate,nb_read_frames,"
         "color_range,color_space,color_transfer,color_primaries",
         "-of", "json", str(src)], capture_output=True, text=True).stdout)["streams"][0]
    num, den = s["r_frame_rate"].split("/")
    trc = s.get("color_transfer") or "bt709"
    prim = s.get("color_primaries") or "bt709"
    csp = s.get("color_space") or "bt709"
    rng = s.get("color_range") or "tv"
    h264_trc = {"bt709": 1, "iec61966-2-1": 13, "smpte170m": 6}.get(trc, 1)
    return {
        "src": Path(src), "w": int(s["width"]), "h": int(s["height"]),
        "fps": round(int(num) / int(den)), "total": int(s["nb_read_frames"]),
        "trc": trc, "prim": prim, "csp": csp, "range": rng, "h264_trc": h264_trc,
        "setparams": (f"setparams=color_primaries={prim}:color_trc={trc}"
                      f":colorspace={csp}:range={rng},"),
    }


def work_dirs(cfg):
    stem = cfg["src"].stem
    w = ROOT / f"work_{stem}"
    w.mkdir(exist_ok=True)
    (ROOT / f"segments_{stem}" / "input").mkdir(parents=True, exist_ok=True)
    (ROOT / f"segments_{stem}" / "output").mkdir(parents=True, exist_ok=True)
    cfg["work"] = w
    cfg["segin"] = ROOT / f"segments_{stem}" / "input"
    cfg["segout"] = ROOT / f"segments_{stem}" / "output"
    cfg["out"] = ROOT / "final" / f"FINAL_{stem}.mp4"
    return cfg


# ------------------------------------------------------------- analysis

def transcribe(cfg):
    wav = cfg["work"] / "audio16k.wav"
    words = cfg["work"] / "words.json"
    if words.exists():
        return json.loads(words.read_text())
    subprocess.run(["ffmpeg", "-v", "error", "-i", str(cfg["src"]), "-map", "0:a:0",
                    "-vn", "-ac", "1", "-ar", "16000", "-c:a", "pcm_s16le",
                    "-y", str(wav)], check=True)
    from faster_whisper import WhisperModel
    m = WhisperModel("small", device="cpu", compute_type="int8")
    segs, _ = m.transcribe(str(wav), word_timestamps=True, vad_filter=False,
                           language="en")
    out = [{"start": round(s.start, 2), "end": round(s.end, 2),
            "text": s.text.strip(),
            "words": [{"w": w.word.strip(), "s": round(w.start, 2),
                       "e": round(w.end, 2)} for w in (s.words or [])]}
           for s in segs]
    words.write_text(json.dumps(out, indent=1))
    return out


def cuts_of(cfg, th=0.10):
    f = cfg["work"] / "cuts.txt"
    if not f.exists():
        out = subprocess.run(
            ["ffmpeg", "-v", "error", "-i", str(cfg["src"]), "-map", "0:v:0",
             "-vf", f"select='gt(scene,{th})',metadata=print:file=-",
             "-an", "-f", "null", "-"], capture_output=True, text=True).stdout
        ts, t = [], None
        for line in out.splitlines():
            if "pts_time:" in line:
                t = float(line.split("pts_time:")[1].split()[0])
            elif "scene_score=" in line and t is not None:
                ts.append(f"{t:.2f}")
        f.write_text("\n".join(ts))
    return sorted(float(x) for x in f.read_text().split())


def scan(cfg):
    """Per frame: is a face on screen, how big, where, and is its mouth moving.

    Mouth movement is scored against whole-frame movement. Raw mouth
    difference just measures cuts and camera moves - a shot change scores
    higher than a spoken sentence."""
    npz = cfg["work"] / "scan.npz"
    if npz.exists():
        return dict(np.load(npz))
    det = cv2.FaceDetectorYN.create(str(YUNET), "", (DW, DH), 0.6, 0.3, 5000)
    p = subprocess.Popen(
        ["ffmpeg", "-v", "error", "-i", str(cfg["src"]), "-map", "0:v:0",
         "-fps_mode", "passthrough", "-vf", f"scale={DW}:{DH}",
         "-pix_fmt", "bgr24", "-f", "rawvideo", "-"], stdout=subprocess.PIPE)
    size = DW * DH * 3
    n = cfg["total"]
    act = np.full(n, np.nan)
    face = np.zeros(n, bool)
    area = np.zeros(n)
    cen = np.zeros((n, 2))
    mouth = np.zeros((n, 4), int)
    prev = None
    for i in range(n):
        b = p.stdout.read(size)
        if len(b) < size:
            break
        f = np.frombuffer(b, np.uint8).reshape(DH, DW, 3)
        _, faces = det.detect(np.ascontiguousarray(f))
        box = None
        if faces is not None and len(faces):
            fa = max(faces, key=lambda r: r[2] * r[3])
            face[i] = True
            area[i] = float(fa[2] * fa[3])
            cen[i] = [float(fa[0] + fa[2] / 2), float(fa[1] + fa[3] / 2)]
            rx, ry, lx, ly = fa[10], fa[11], fa[12], fa[13]
            cx, cy = (rx + lx) / 2, (ry + ly) / 2
            half = max(abs(lx - rx), 8.0)
            box = (max(0, int(cx - half)), max(0, int(cy - half * .7)),
                   min(DW, int(cx + half)), min(DH, int(cy + half * .7)))
            mouth[i] = box
        if prev is not None and box is not None:
            A, B = prev.astype(np.int16), f.astype(np.int16)
            g = float(np.abs(A - B).mean())
            if g <= 40:
                x0, y0, x1, y1 = box
                if x1 > x0 and y1 > y0:
                    act[i] = float(np.abs(A[y0:y1, x0:x1]
                                          - B[y0:y1, x0:x1]).mean()) / (g + 1e-3)
        prev = f
        if i % 600 == 0:
            print(f"    scanning {i}/{n}", flush=True)
    p.stdout.close(); p.wait()
    d = {"act": act, "face": face, "area": area, "cen": cen, "mouth": mouth}
    np.savez(npz, **d)
    return d


# ----------------------------------------------------------------- plan

def build_plan(cfg, words, cuts, sc, min_face_pct, min_run, merge_gap):
    """Segments come from the footage, not from the transcript's grammar.

    Reading narration as voice-over and skipping it was right on two videos
    and wrong on the third, where the characters mouth the narration too -
    that mistake left most of the video unsynced."""
    n = cfg["total"]
    fps = cfg["fps"]
    speech = np.zeros(n, bool)
    for s in words:
        for w in s["words"]:
            speech[int(w["s"] * fps):int(w["e"] * fps) + 1] = True
    big = sc["area"] > DW * DH * (min_face_pct / 100.0)
    cand = sc["face"] & big & speech

    runs, i = [], 0
    while i < n:
        if cand[i]:
            j = i
            while j + 1 < n and cand[j + 1]:
                j += 1
            if j - i + 1 >= min_run:
                runs.append([i, j])
            i = j + 1
        else:
            i += 1

    def shot(f):
        t = f / fps
        return (max([c for c in cuts if c <= t], default=0.0),
                min([c for c in cuts if c > t], default=n / fps))

    merged = []
    for r in runs:
        if merged and r[0] - merged[-1][1] <= merge_gap \
                and shot(merged[-1][1]) == shot(r[0]):
            merged[-1][1] = r[1]
        else:
            merged.append(r)

    plan = []
    for k, (a, b) in enumerate(merged, 1):
        said = " ".join(w["w"] for s in words for w in s["words"]
                        if w["e"] > a / fps and w["s"] < (b + 1) / fps)
        plan.append({"n": k, "final": [a, b], "frames": b - a + 1,
                     "text": said[:60]})
    return plan


def speakers(cfg, plan):
    """Name the face to sync and track it for the compositor.

    Left alone sync.so takes the most prominent face, and the compositor
    used to mask the biggest one - between them they twice synced a
    bystander and then threw the correct result away."""
    det = cv2.FaceDetectorYN.create(str(YUNET), "", (DW, DH), 0.6, 0.3, 5000)
    sx, sy = cfg["w"] / DW, cfg["h"] / DH
    out = []
    for p in plan:
        a, b = p["final"]
        raw = subprocess.run(
            ["ffmpeg", "-v", "error", "-i", str(cfg["src"]), "-map", "0:v:0",
             "-fps_mode", "passthrough",
             "-vf", f"select='between(n,{a},{b})',scale={DW}:{DH}",
             "-pix_fmt", "bgr24", "-f", "rawvideo", "-"], capture_output=True).stdout
        fr = np.frombuffer(raw, np.uint8).reshape(-1, DH, DW, 3)
        tracks = []
        for i, f in enumerate(fr):
            _, faces = det.detect(np.ascontiguousarray(f))
            if faces is None:
                continue
            for fa in faces:
                x, y, w, h = fa[:4]
                cx, cy = float(x + w / 2), float(y + h / 2)
                rx, ry, lx, ly = fa[10], fa[11], fa[12], fa[13]
                mx, my = (rx + lx) / 2, (ry + ly) / 2
                half = max(abs(lx - rx), 8.0)
                mb = (int(mx - half), int(my - half * .7),
                      int(mx + half), int(my + half * .7))
                best, bd = None, 60
                for t in tracks:
                    if t["last"] == i:
                        continue
                    dd = ((t["cx"] - cx) ** 2 + (t["cy"] - cy) ** 2) ** .5
                    if dd < bd:
                        best, bd = t, dd
                if best is None:
                    tracks.append({"cx": cx, "cy": cy, "last": i, "n": 1,
                                   "frames": {i: mb}})
                else:
                    best.update(cx=cx, cy=cy, last=i, n=best["n"] + 1)
                    best["frames"][i] = mb
        tracks = [t for t in tracks if t["n"] >= max(3, len(fr) * .2)]
        if not tracks:
            out.append({**p, "coords": None, "track": {}})
            continue

        def score(t):
            vals = []
            for i in sorted(t["frames"]):
                if i + 1 >= len(fr) or (i + 1) not in t["frames"]:
                    continue
                A, B = fr[i].astype(np.int16), fr[i + 1].astype(np.int16)
                g = float(np.abs(A - B).mean())
                if g > 40:
                    continue
                x0, y0, x1, y1 = t["frames"][i]
                x0, y0 = max(0, x0), max(0, y0)
                x1, y1 = min(DW, x1), min(DH, y1)
                if x1 > x0 and y1 > y0:
                    vals.append(float(np.abs(A[y0:y1, x0:x1]
                                             - B[y0:y1, x0:x1]).mean()) / (g + 1e-3))
            return float(np.mean(vals)) if vals else 0.0

        best = max(tracks, key=score)
        # the frame where this face is biggest - a small face at the wrong
        # moment got a whole generation rejected
        fnum = max(best["frames"], key=lambda i:
                   (best["frames"][i][2] - best["frames"][i][0])
                   * (best["frames"][i][3] - best["frames"][i][1]))
        mb = best["frames"][fnum]
        out.append({**p, "faces": len(tracks),
                    "coords": [round(((mb[0] + mb[2]) / 2) * sx),
                               round(((mb[1] + mb[3]) / 2) * sy)],
                    "frame_number": int(fnum),
                    "face_box_small": [int(v) for v in mb],
                    "track": {str(i): [round(((v[0] + v[2]) / 2) / DW, 5),
                                       round(((v[1] + v[3]) / 2) / DH, 5)]
                              for i, v in best["frames"].items()}})
    return out


# ------------------------------------------------------------- generate

def extract_all(cfg, plan):
    """One decode pass for every segment. Running one ffmpeg per segment
    scans the whole master each time and was the slowest stage by far."""
    fc, maps = [], []
    for i, p in enumerate(plan):
        a, b = p["final"]
        t0, t1 = a / cfg["fps"], (b + 1) / cfg["fps"]
        d = cfg["segin"] / f"s{p['n']:02d}"
        d.mkdir(parents=True, exist_ok=True)
        fc.append(f"[0:v:0]select='between(n,{a},{b})',setpts=PTS-STARTPTS[v{i}]")
        fc.append(f"[0:a:0]aselect='between(t,{t0},{t1})',asetpts=PTS-STARTPTS,"
                  f"asplit=2[a{i}][w{i}]")
        maps += ["-map", f"[v{i}]", "-map", f"[a{i}]",
                 "-c:v", "libx264", "-crf", "12", "-preset", "medium",
                 "-pix_fmt", "yuv420p", "-r", str(cfg["fps"]),
                 "-c:a", "aac", "-b:a", "192k", "-ar", "48000",
                 "-movflags", "+faststart", str(d / "seg.mp4")]
        maps += ["-map", f"[w{i}]", "-vn", "-c:a", "pcm_s16le", "-ar", "48000",
                 str(d / "seg.wav")]
    subprocess.run(["ffmpeg", "-v", "error", "-i", str(cfg["src"]),
                    "-filter_complex", ";".join(fc)] + maps + ["-y"], check=True)
    for p in plan:
        got = L.ffprobe(cfg["segin"] / f"s{p['n']:02d}" / "seg.mp4")
        assert got["frames"] == p["frames"], \
            f"s{p['n']:02d}: extracted {got['frames']} != planned {p['frames']}"


def face_image(cfg, seg, frame_number, box):
    sx = cfg["w"] / DW
    x0, y0, x1, y1 = [v * sx for v in box]
    cx, cy = int((x0 + x1) / 2), int((y0 + y1) / 2)
    half = int(max(x1 - x0, y1 - y0))
    out = subprocess.run(
        ["ffmpeg", "-v", "error", "-i", str(seg),
         "-vf", f"select='eq(n,{frame_number})',"
                f"crop={half*2}:{half*2}:{max(0,cx-half)}:{max(0,cy-half)},"
                f"scale=128:128", "-frames:v", "1", "-f", "webp", "-"],
        capture_output=True).stdout
    return "data:image/webp;base64," + base64.b64encode(out).decode() if out else None


def generate(cfg, plan):
    def one(p):
        sid = f"s{p['n']:02d}"
        d = cfg["segin"] / sid
        try:
            v, a = L.upload(d / "seg.mp4"), L.upload(d / "seg.wav")
            asd = None
            if p.get("coords"):
                asd = {"auto_detect": False, "frame_number": p["frame_number"],
                       "coordinates": p["coords"]}
                fi = face_image(cfg, d / "seg.mp4", p["frame_number"],
                                p["face_box_small"])
                if fi:
                    asd["face_image"] = fi
            idem = str(uuid.uuid5(uuid.NAMESPACE_URL,
                                  f"{cfg['src'].stem}:{sid}:{p['final'][0]}:"
                                  f"{p.get('coords')}"))
            g = L.poll(L.submit(v, a, idem, asd)["id"])
            if g["status"] != "COMPLETED":
                print(f"  {sid} {g['status']} -> keeping original", flush=True)
                return sid, False
            out = cfg["segout"] / sid
            out.mkdir(parents=True, exist_ok=True)
            subprocess.run(["curl", "-sfL", "-o", str(out / "synced.mp4"),
                            g["outputMediaUrl"]], check=True)
            checks, _, _ = L.validate(d / "seg.mp4", out / "synced.mp4")
            ok = all(c[2] for c in checks.values())
            print(f"  {sid} {'OK' if ok else 'FAILED ' + str([k for k, c in checks.items() if not c[2]])}",
                  flush=True)
            return sid, ok
        except Exception as e:
            print(f"  {sid} ERROR {e} -> keeping original", flush=True)
            return sid, False

    with ThreadPoolExecutor(max_workers=7) as ex:
        return [s for s, ok in ex.map(one, plan) if ok]


# ------------------------------------------------------------ composite

def composite(cfg, plan, usable):
    """Replace only the speaker's mouth, level-matched.

    sync.so returns frames darker than it was given, and more so outside
    the face than on it, so dropping them in whole makes every boundary
    flicker. All of this stays in yuv444p and writes no colour tags - a
    bgr24 roundtrip moved the picture +2.3 Y and tagging the encoder
    another -4.5 Y."""
    det = cv2.FaceDetectorYN.create(str(YUNET), "", (DW, DH), 0.6, 0.3, 5000)
    W, H = cfg["w"], cfg["h"]
    scx, scy = W / DW, H / DH
    plane, frame = W * H, W * H * 3
    for p in plan:
        sid = f"s{p['n']:02d}"
        if sid not in usable:
            continue
        src, syn = cfg["segin"] / sid / "seg.mp4", cfg["segout"] / sid / "synced.mp4"
        dst = cfg["segout"] / sid / "composited.mp4"
        trk = {int(k): (v[0] * DW, v[1] * DH) for k, v in (p.get("track") or {}).items()}
        rd = lambda f: subprocess.Popen(
            ["ffmpeg", "-v", "error", "-i", str(f), "-fps_mode", "passthrough",
             "-pix_fmt", "yuv444p", "-f", "rawvideo", "-"], stdout=subprocess.PIPE)
        A, B = rd(src), rd(syn)
        out = subprocess.Popen(
            ["ffmpeg", "-v", "error", "-f", "rawvideo", "-pix_fmt", "yuv444p",
             "-s", f"{W}x{H}", "-r", str(cfg["fps"]), "-i", "-", "-i", str(src),
             "-map", "0:v", "-map", "1:a?", "-c:v", "libx264", "-crf", "12",
             "-preset", "medium", "-pix_fmt", "yuv420p", "-c:a", "aac",
             "-b:a", "192k", "-fps_mode", "passthrough", "-y", str(dst)],
            stdin=subprocess.PIPE)
        last = np.zeros((H, W), np.float32)
        want, n = None, 0
        while True:
            ra, rb = A.stdout.read(frame), B.stdout.read(frame)
            if len(ra) < frame or len(rb) < frame:
                break
            po = [np.frombuffer(ra, np.uint8, plane, i * plane)
                  .reshape(H, W).astype(np.float32) for i in range(3)]
            ps = [np.frombuffer(rb, np.uint8, plane, i * plane)
                  .reshape(H, W).astype(np.float32) for i in range(3)]
            want = trk.get(n, want)
            small = cv2.cvtColor(np.stack([cv2.resize(x, (DW, DH)) for x in po],
                                          axis=-1).astype(np.uint8),
                                 cv2.COLOR_YUV2BGR)
            _, faces = det.detect(np.ascontiguousarray(small))
            if faces is not None and len(faces):
                if want is not None:
                    fa = min(faces, key=lambda r: (r[0] + r[2] / 2 - want[0]) ** 2
                                                  + (r[1] + r[3] / 2 - want[1]) ** 2)
                else:
                    fa = max(faces, key=lambda r: r[2] * r[3])
                x, y, w, h = [float(v) for v in fa[:4]]
                cx, cy = (x + w / 2) * scx, (y + h * .72) * scy
                ax, ay = w * .62 * scx, h * .42 * scy
                m = np.zeros((H, W), np.uint8)
                cv2.ellipse(m, (int(cx), int(cy)), (int(ax), int(ay)), 0, 0, 360, 255, -1)
                k = int(max(ax, ay) * .25) | 1
                last = cv2.GaussianBlur(m, (k, k), 0).astype(np.float32) / 255.0
            outside = last < 0.02
            keep = outside.sum() > 1000
            buf = bytearray()
            for c in range(3):
                s = ps[c]
                if keep:
                    oc, scc = po[c][outside], s[outside]
                    sd = scc.std()
                    g = float(np.clip((oc.std() / sd) if sd > 1e-3 else 1.0, .9, 1.1))
                    s = s * g + (oc.mean() - g * scc.mean())
                buf += np.clip(po[c] * (1 - last) + s * last, 0, 255).astype(np.uint8).tobytes()
            out.stdin.write(bytes(buf))
            n += 1
        for q in (A, B):
            q.stdout.close(); q.wait()
        out.stdin.close(); out.wait()
        print(f"  {sid} {n} frames {'OK' if n == p['frames'] else 'MISMATCH'}", flush=True)


# --------------------------------------------------------------- splice

def splice(cfg, plan, usable):
    segs = sorted((p["final"][0], p["final"][1], f"s{p['n']:02d}")
                  for p in plan if f"s{p['n']:02d}" in usable)
    inputs = ["-i", str(cfg["src"])]
    for _, _, sid in segs:
        c = cfg["segout"] / sid / "composited.mp4"
        inputs += ["-i", str(c if c.exists() else cfg["segout"] / sid / "synced.mp4")]
    sp = cfg["setparams"]
    fc, labels, cursor = [], [], 0
    for idx, (a, b, _) in enumerate(segs, start=1):
        if a > cursor:
            fc.append(f"[0:v:0]trim=start_frame={cursor}:end_frame={a},"
                      f"{sp}setpts=N/{cfg['fps']}/TB[o{idx}]")
            labels.append(f"o{idx}")
        fc.append(f"[{idx}:v]{sp}setpts=N/{cfg['fps']}/TB[s{idx}]")
        labels.append(f"s{idx}")
        cursor = b + 1
    if cursor < cfg["total"]:
        fc.append(f"[0:v:0]trim=start_frame={cursor}:end_frame={cfg['total']},"
                  f"{sp}setpts=N/{cfg['fps']}/TB[oz]")
        labels.append("oz")
    fc.append("".join(f"[{l}]" for l in labels) + f"concat=n={len(labels)}:v=1:a=0[out]")
    cfg["out"].parent.mkdir(exist_ok=True)
    subprocess.run(["ffmpeg", "-v", "error", *inputs,
                    "-filter_complex", ";".join(fc),
                    "-map", "[out]", "-map", "0:a:0",
                    "-c:v", "libx264", "-crf", "14", "-preset", "fast",
                    "-pix_fmt", "yuv420p", "-fps_mode", "passthrough",
                    "-c:a", "aac", "-b:a", "192k",
                    "-bsf:v", f"h264_metadata=colour_primaries=1:"
                              f"transfer_characteristics={cfg['h264_trc']}:"
                              f"matrix_coefficients=1:video_full_range_flag=0",
                    "-movflags", "+faststart", "-y", str(cfg["out"])], check=True)
    print(f"  spliced {len(segs)} segments into {cfg['out']}")


# --------------------------------------------------------------- verify

def verify(cfg, plan, usable):
    w, h = 270, int(270 * cfg["h"] / cfg["w"])

    def y(path):
        # passthrough and the Y plane: a raw pipe defaults to CFR and adds a
        # frame, and reading through RGB applies the transfer tag - both once
        # made this very check report a corrupted baseline
        r = subprocess.run(["ffmpeg", "-v", "error", "-i", str(path),
                            "-map", "0:v:0", "-fps_mode", "passthrough",
                            "-vf", f"scale={w}:{h}", "-pix_fmt", "yuv420p",
                            "-f", "rawvideo", "-"], capture_output=True).stdout
        fs = w * h * 3 // 2
        n = len(r) // fs
        return (np.frombuffer(r[:n * fs], np.uint8).reshape(n, fs)[:, :w * h]
                .reshape(n, h, w).astype(np.float32))

    new, old = y(cfg["out"]), y(cfg["src"])
    n = min(len(new), len(old))
    fails = []
    print(f"\n  frames: final={len(new)} original={len(old)} "
          f"{'MATCH' if len(new) == len(old) == cfg['total'] else 'MISMATCH'}")
    if not (len(new) == len(old) == cfg["total"]):
        fails.append("frame count")

    segs = [p for p in plan if f"s{p['n']:02d}" in usable]
    inside = np.zeros(n, bool)
    for p in segs:
        a, b = p["final"]
        inside[a:b + 1] = True
    d = np.abs(new[:n] - old[:n]).mean(axis=(1, 2))
    stray = int(((~inside) & (d > 3)).sum())
    print(f"  untouched frames altered: {stray}  (mean {d[~inside].mean():.3f})"
          f"  {'PASS' if stray == 0 else 'FAIL'}")
    if stray:
        fails.append("untouched frames changed")

    worst = max(abs(float((new[p["final"][0]:p["final"][1] + 1]
                           - old[p["final"][0]:p["final"][1] + 1]).mean()))
                for p in segs) if segs else 0.0
    print(f"  worst level shift: {worst:.2f} Y  {'PASS' if worst < 1.0 else 'FAIL'}")
    if worst >= 1.0:
        fails.append("level shift (flicker)")

    dn = np.abs(np.diff(new, axis=0)).mean(axis=(1, 2))
    do = np.abs(np.diff(old, axis=0)).mean(axis=(1, 2))
    ws = 0.0
    for p in segs:
        a, b = p["final"]
        for i in (a - 1, b):
            if 0 <= i < len(dn):
                ws = max(ws, abs(dn[i] - do[i]))
    print(f"  worst seam delta: {ws:.2f}  (original std {do.std():.2f})  "
          f"{'PASS' if ws < do.std() else 'CHECK'}")
    if ws >= do.std():
        fails.append("seam")

    print("\n  lipsync present on the speaker:")
    for p in segs:
        a, b = p["final"]
        trk = p.get("track") or {}
        rd = lambda src: np.frombuffer(subprocess.run(
            ["ffmpeg", "-v", "error", "-i", str(src), "-map", "0:v:0",
             "-fps_mode", "passthrough",
             "-vf", f"select='between(n,{a},{b})',scale={DW}:{DH}",
             "-pix_fmt", "bgr24", "-f", "rawvideo", "-"],
            capture_output=True).stdout, np.uint8).reshape(-1, DH, DW, 3).astype(np.float32)
        o, f = rd(cfg["src"]), rd(cfg["out"])
        m = min(len(o), len(f))
        vals = []
        for i in range(m):
            if str(i) not in trk:
                continue
            cx, cy = trk[str(i)][0] * DW, trk[str(i)][1] * DH
            x0, y0 = max(0, int(cx - 34)), max(0, int(cy - 26))
            x1, y1 = min(DW, int(cx + 34)), min(DH, int(cy + 26))
            if x1 > x0 and y1 > y0:
                vals.append(np.abs(f[i, y0:y1, x0:x1] - o[i, y0:y1, x0:x1]).mean())
        mouth = float(np.mean(vals)) if vals else 0.0
        bad = mouth < 3.0
        print(f"    s{p['n']:02d} mouth diff {mouth:6.2f}{'   NO LIPSYNC' if bad else ''}")
        if bad:
            fails.append(f"s{p['n']:02d} no lipsync")

    def count(path, filt, tok):
        e = subprocess.run(["ffmpeg", "-v", "info", "-i", str(path), "-vf", filt,
                            "-an", "-f", "null", "-"],
                           capture_output=True, text=True).stderr
        return e.count(tok)

    for lbl, p in (("original", cfg["src"]), ("final", cfg["out"])):
        print(f"  {lbl:9} black={count(p, 'blackdetect=d=0.03:pic_th=0.98', 'black_start')}"
              f" freeze={count(p, 'freezedetect=n=0.001:d=0.5', 'freeze_start')}")
    return fails


# ----------------------------------------------------------------- main

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("video")
    ap.add_argument("--go", action="store_true", help="spend credits and build")
    ap.add_argument("--min-face", type=float, default=2.0,
                    help="smallest face to sync, %% of frame area")
    ap.add_argument("--min-run", type=int, default=12)
    ap.add_argument("--merge-gap", type=int, default=10)
    a = ap.parse_args()

    cfg = work_dirs(probe(a.video))
    print(f"{cfg['src'].name}: {cfg['w']}x{cfg['h']} {cfg['fps']}fps "
          f"{cfg['total']} frames, transfer={cfg['trc']}")

    print("\n[1/7] transcribing")
    words = transcribe(cfg)
    print("[2/7] scene cuts")
    cuts = cuts_of(cfg)
    print(f"      {len(cuts)} cuts")
    print("[3/7] scanning faces and mouths")
    sc = scan(cfg)
    print("[4/7] planning from the footage")
    plan = build_plan(cfg, words, cuts, sc, a.min_face, a.min_run, a.merge_gap)
    plan = speakers(cfg, plan)
    (cfg["work"] / "plan.json").write_text(json.dumps(plan, indent=1))

    total = sum(p["frames"] for p in plan)
    print(f"\n{'seg':6}{'time':>16}{'sec':>6}{'cr':>6}  words")
    for p in plan:
        x, b = p["final"]
        print(f"s{p['n']:02d}  {x/cfg['fps']:7.2f}-{(b+1)/cfg['fps']:<8.2f}"
              f"{p['frames']/cfg['fps']:6.1f}{p['frames']*RATE:6.0f}  {p['text'][:44]}")
    print(f"\n{len(plan)} segments, {total} frames = {total/cfg['fps']:.1f}s "
          f"= {total*RATE:.0f} credits (${total*RATE/100:.2f})")
    print(f"whole video would be {cfg['total']*RATE:.0f} credits "
          f"- saving {(1-total/cfg['total'])*100:.0f}%")

    if not a.go:
        print("\nplan only. re-run with --go to spend credits.")
        return

    print("\n[5/7] extracting and generating")
    extract_all(cfg, plan)
    usable = set(generate(cfg, plan))
    (cfg["work"] / "usable.json").write_text(json.dumps(sorted(usable)))
    if not usable:
        sys.exit("nothing usable - original left untouched")

    print("\n[6/7] compositing")
    composite(cfg, plan, usable)
    splice(cfg, plan, usable)

    print("\n[7/7] verifying")
    fails = verify(cfg, plan, usable)
    print("\n" + ("ALL CHECKS PASSED" if not fails else "FAILED: " + ", ".join(fails)))
    print(f"\n{cfg['out']}")


if __name__ == "__main__":
    main()


```

## Source file: cfg3.py

{
  "path": "cfg3.py",
  "blob_sha": "23546371689cbf254b3ed0b69dba3c0f68b571d2",
  "sha256": "97db7cd29d29a1153e96969b331e5f28a004aca40f9ee2a843acef0cb90c205e",
  "lines": 18,
  "bytes": 782,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 1
  },
  "url": "https://github.com/Akashkandari7/LipSync-3d-Videos/blob/edcca1c8ed37aa351853935652e4a474b9311d38/cfg3.py",
  "function_signature_candidates": []
}

```text

"""Per-video settings. Everything the pipeline needs to know that differs
between masters - kept in one place because the last run's worst bugs came
from a constant that was right for one video and wrong for the next."""
from pathlib import Path
ROOT = Path(__file__).parent
SRC = ROOT / "source" / "video3.mp4"
WORK = ROOT / "work3"
SEGIN = ROOT / "segments3" / "input"
SEGOUT = ROOT / "segments3" / "output"
OUT = ROOT / "final" / "FINAL_video3.mp4"
FPS = 30
TOTAL = 3330
NW, NH = 1080, 1920
# video3 is tagged bt709 transfer; video2 was iec61966-2-1. Writing the wrong
# one back shifts the whole picture.
TRC_FF = "bt709"
TRC_H264 = 1          # transfer_characteristics for h264_metadata
SETPARAMS = "setparams=color_primaries=bt709:color_trc=bt709:colorspace=bt709:range=tv,"


```
