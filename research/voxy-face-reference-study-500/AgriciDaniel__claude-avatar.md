# Facial rendering source scan: AgriciDaniel/claude-avatar

Repository: https://github.com/AgriciDaniel/claude-avatar

Pinned source revision: 651b45d543c1dd6541e2df8155ad12a210b20e14

Discovery topic: lip sync 3D

Description: 3D talking avatar connected to Claude Code CLI - voice conversation with lip-sync

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {}

## Source file: claude/tts.py

{
  "path": "claude/tts.py",
  "blob_sha": "0a237f7112904e234dde1f0bcc038aeb004485ec",
  "sha256": "087ae3ed68df46829a69b48de2adc065be4bc874e371218670f489cb29641a65",
  "lines": 45,
  "bytes": 1296,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/AgriciDaniel/claude-avatar/blob/651b45d543c1dd6541e2df8155ad12a210b20e14/claude/tts.py",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "async def main():"
    }
  ]
}

```text

#!/usr/bin/env python3
"""Generate TTS audio with word-level timing using edge-tts.
Outputs JSON: {"audio": "<base64 mp3>", "words": [...], "wtimes": [...], "wdurations": [...]}
"""
import sys
import json
import asyncio
import base64
import io
import edge_tts

VOICE = sys.argv[2] if len(sys.argv) > 2 else "en-US-AvaMultilingualNeural"

async def main():
    text = sys.argv[1]
    communicate = edge_tts.Communicate(text, VOICE, boundary="WordBoundary")

    audio_chunks = []
    words = []
    wtimes = []
    wdurations = []

    async for chunk in communicate.stream():
        if chunk["type"] == "audio":
            audio_chunks.append(chunk["data"])
        elif chunk["type"] == "WordBoundary":
            words.append(chunk["text"])
            # edge-tts times are in 100-nanosecond units, convert to milliseconds
            offset_ms = chunk["offset"] / 10000
            duration_ms = chunk["duration"] / 10000
            wtimes.append(offset_ms)
            wdurations.append(duration_ms)

    audio_bytes = b"".join(audio_chunks)
    audio_b64 = base64.b64encode(audio_bytes).decode("ascii")

    result = {
        "audio": audio_b64,
        "words": words,
        "wtimes": wtimes,
        "wdurations": wdurations
    }
    print(json.dumps(result))

asyncio.run(main())


```

## Source file: claude/transcribe.py

{
  "path": "claude/transcribe.py",
  "blob_sha": "bf469c3a682ad7f0b96ed1ead374adbc66d22086",
  "sha256": "46f33c9636e8a543e68cc2e21e11a5532beaddde501b3938e1251ab9f097a8af",
  "lines": 11,
  "bytes": 397,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/AgriciDaniel/claude-avatar/blob/651b45d543c1dd6541e2df8155ad12a210b20e14/claude/transcribe.py",
  "function_signature_candidates": []
}

```text

#!/usr/bin/env python3
"""Fast GPU transcription using faster-whisper. Outputs plain text to stdout."""
import sys
from faster_whisper import WhisperModel

model = WhisperModel("large-v3-turbo", device="cuda", compute_type="float16")

audio_file = sys.argv[1]
segments, _ = model.transcribe(audio_file, language="en", beam_size=5)
text = " ".join(seg.text.strip() for seg in segments)
print(text)


```

## Source file: claude/transcribe_server.py

{
  "path": "claude/transcribe_server.py",
  "blob_sha": "f40f27ccbdff05cd9a679fc81f94b9ff5342d3f2",
  "sha256": "141e1272033cac3f26b2b1336705e57409fb2c4b1e132854aeceb9e2e8527db4",
  "lines": 21,
  "bytes": 707,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/AgriciDaniel/claude-avatar/blob/651b45d543c1dd6541e2df8155ad12a210b20e14/claude/transcribe_server.py",
  "function_signature_candidates": []
}

```text

#!/usr/bin/env python3
"""Persistent faster-whisper server. Model stays loaded in VRAM.
Reads WAV file paths from stdin, outputs transcripts to stdout.
"""
import sys
from faster_whisper import WhisperModel

print("LOADING_MODEL", flush=True)
model = WhisperModel("large-v3-turbo", device="cuda", compute_type="float16")
print("MODEL_READY", flush=True)

for line in sys.stdin:
    wav_path = line.strip()
    if not wav_path:
        continue
    try:
        segments, _ = model.transcribe(wav_path, language="en", beam_size=5)
        text = " ".join(seg.text.strip() for seg in segments)
        print(f"TRANSCRIPT:{text}", flush=True)
    except Exception as e:
        print(f"ERROR:{e}", flush=True)


```
