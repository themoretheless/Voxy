# Facial rendering source scan: xanguera/aicompanion

Repository: https://github.com/xanguera/aicompanion

Pinned source revision: 34104ccddb29a38a44afadb9bf09c11598601eae

Discovery topic: lip sync 3D

Description: Local-first, voice-driven AI companion — 3D avatar with lip-sync, streaming voice, and persistent memory. Runs on your Mac; only the LLM call is remote.

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"lip": ["frontend/lipsync.js", "tests/lipsync_check.mjs"]}

## Source file: frontend/lipsync.js

{
  "path": "frontend/lipsync.js",
  "blob_sha": "68ed2063d7162674bfef4925c1f318f2cd39b4e9",
  "sha256": "8b7e1358944e5c3ae7341c971e2deaef1f9e0cc5b6a3b6d90e68938037e48ebf",
  "lines": 127,
  "bytes": 5233,
  "selection_score": 15,
  "keyword_line_counts": {
    "jaw": 1,
    "mouth": 3
  },
  "url": "https://github.com/xanguera/aicompanion/blob/34104ccddb29a38a44afadb9bf09c11598601eae/frontend/lipsync.js",
  "function_signature_candidates": []
}

```text

// Phoneme → Oculus viseme conversion for TalkingHead's provided-viseme path
// (head.speakAudio({ visemes, vtimes, vdurations })).
//
// WHY THIS IS NOT 1 phoneme = 1 viseme:
// Feeding one viseme per phoneme makes the mouth flutter (open/close mush) because each event
// is ~40-80ms and TalkingHead's provided-viseme path does NOT clamp/hold like its word path.
// The field's fix is many-to-one REDUCTION + frame-HOLDING + VOWEL DOMINANCE (coarticulation):
// keep the visually salient shapes, hold them long enough to read, and let vowels carry the
// motion while weak consonants are merged or dropped.
//
// Reduced salient set we emit (Disney/Preston-Blair-ish ~7 shapes):
//   vowels:  aa E I O U   | closures: PP (p/b/m), FF (f/v)   | sil (rest)
// Every other consonant is "weak" → one neutral shape, and dropped when too short.
//
// NOTE: the library hardcodes provided-viseme peak weight at 0.6 (0.9 for PP/FF), so vowels
// can't open as wide as the word-level path (which overdrives to 1.0). Holding + reduction
// still removes the flutter; for wider opening you'd additionally drive a jaw/mouthOpen morph.

const VOWELS = new Set(['aa', 'E', 'I', 'O', 'U']);
const CLOSURES = new Set(['PP', 'FF']);     // high-salience, never dropped
const NEUTRAL = 'nn';                        // single shape for all "weak" consonants

// IPA / Kokoro-phoneme → viseme. Low-salience consonants all fold to NEUTRAL on purpose.
export const PHONEME_TO_VISEME = {
  // silence / pauses — punctuation tokens must rest the mouth, not hold a neutral shape
  ' ': 'sil', '': 'sil', ',': 'sil', '.': 'sil', '!': 'sil', '?': 'sil',
  ';': 'sil', ':': 'sil', '…': 'sil', '"': 'sil', "'": 'sil', '—': 'sil',
  // closures (keep distinct — most readable)
  'p': 'PP', 'b': 'PP', 'm': 'PP',
  'f': 'FF', 'v': 'FF',
  // vowels
  'a': 'aa', 'ɑ': 'aa', 'æ': 'aa', 'ʌ': 'aa', 'ɐ': 'aa',
  'ə': 'E', 'ᵊ': 'E', 'e': 'E', 'ɛ': 'E', 'ɜ': 'E',
  'i': 'I', 'ɪ': 'I', 'j': 'I', 'y': 'I',
  'o': 'O', 'ɔ': 'O', 'ɒ': 'O',
  'u': 'U', 'ʊ': 'U', 'w': 'U',
  // weak consonants → one neutral shape (visually near-identical, low salience)
  't': NEUTRAL, 'd': NEUTRAL, 'k': NEUTRAL, 'g': NEUTRAL, 'n': NEUTRAL, 'l': NEUTRAL,
  'ŋ': NEUTRAL, 'h': NEUTRAL, 's': NEUTRAL, 'z': NEUTRAL, 'θ': NEUTRAL, 'ð': NEUTRAL,
  'ʃ': NEUTRAL, 'ʒ': NEUTRAL, 'tʃ': NEUTRAL, 'dʒ': NEUTRAL, 'r': NEUTRAL, 'ɹ': NEUTRAL,
  'ɾ': NEUTRAL,
};

const MULTI = Object.keys(PHONEME_TO_VISEME).filter((k) => k.length > 1);

// Tunables (ms). Raise minViseme/dropWeakBelow if it still looks busy; lower if it looks lazy.
export const DEFAULTS = { minViseme: 90, dropWeakBelow: 70, mergeGap: 30 };

function mapUnit(u) {
  return PHONEME_TO_VISEME[u] ?? NEUTRAL;
}

// Convert backend phoneme entries [{ph,start,dur}] (ms) → reduced/held viseme timeline.
export function phonemesToVisemes(phonemes, opts = {}) {
  const { minViseme, dropWeakBelow, mergeGap } = { ...DEFAULTS, ...opts };

  // 1. Expand tokens into per-unit viseme events (skip silence).
  const raw = [];
  for (const { ph, start, dur } of phonemes) {
    const units = splitIpa(ph);
    if (!units.length) continue;
    const per = dur / units.length;
    units.forEach((u, i) => {
      const v = mapUnit(u);
      if (v === 'sil') return;
      raw.push({ v, start: start + i * per, end: start + (i + 1) * per });
    });
  }
  if (!raw.length) return { visemes: [], vtimes: [], vdurations: [] };
  raw.sort((a, b) => a.start - b.start);

  // 2. Merge consecutive same-viseme events (and near-adjacent within mergeGap).
  let events = mergeSame(raw, mergeGap);

  // 3. Vowel dominance: drop weak (neutral) consonants that are too brief to read.
  //    Closures (PP/FF) and vowels are always kept.
  events = events.filter((e) => {
    const weak = !VOWELS.has(e.v) && !CLOSURES.has(e.v);
    return !(weak && e.end - e.start < dropWeakBelow);
  });

  // 4. Re-merge: dropping a weak consonant may leave two identical vowels adjacent.
  events = mergeSame(events, mergeGap);

  // 5. Enforce a minimum hold; clamp so we only slightly overlap the next event.
  const visemes = [];
  const vtimes = [];
  const vdurations = [];
  for (let i = 0; i < events.length; i++) {
    const e = events[i];
    let d = Math.max(e.end - e.start, minViseme);
    const next = events[i + 1];
    if (next) d = Math.min(d, next.start - e.start + minViseme / 2);
    visemes.push(e.v);
    vtimes.push(Math.round(e.start));
    vdurations.push(Math.max(1, Math.round(d)));
  }
  return { visemes, vtimes, vdurations };
}

function mergeSame(events, mergeGap) {
  const out = [];
  for (const e of events) {
    const last = out[out.length - 1];
    if (last && last.v === e.v && e.start - last.end <= mergeGap) {
      last.end = Math.max(last.end, e.end);
    } else {
      out.push({ ...e });
    }
  }
  return out;
}

function splitIpa(s) {
  const out = [];
  let i = 0;
  const str = String(s);
  while (i < str.length) {
    const two = str.slice(i, i + 2);
    if (MULTI.includes(two)) { out.push(two); i += 2; continue; }
    const one = str[i];
    if ('ˈˌːˑ'.includes(one)) { i += 1; continue; } // skip stress / length marks
    out.push(one);
    i += 1;
  }
  return out;
}


```

## Source file: frontend/ui.js

{
  "path": "frontend/ui.js",
  "blob_sha": "36609458680a56d54f313b852b78241723624d85",
  "sha256": "9bd3671f603b534fc69578a44ef5e32c0604267263f3bc168a03cd232f344328",
  "lines": 43,
  "bytes": 1190,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/xanguera/aicompanion/blob/34104ccddb29a38a44afadb9bf09c11598601eae/frontend/ui.js",
  "function_signature_candidates": []
}

```text

// Tiny DOM helpers: status line, captions, and Start/Stop button state.

const $ = (id) => document.getElementById(id);

export function setStatus(msg, warn = false) {
  const el = $('status');
  if (el) {
    el.textContent = msg;
    el.className = warn ? 'mock' : '';
  }
  console.log('[app]', msg);
}

export function addCaption(role, text) {
  const box = $('captions');
  if (!box || !$('captionsToggle')?.checked) return;
  const line = document.createElement('div');
  line.className = `cap cap-${role}`;
  line.textContent = `${role === 'user' ? '🧑' : '🤖'} ${text}`;
  box.appendChild(line);
  box.scrollTop = box.scrollHeight;
}

export function setButton(state) {
  // state: 'loading' | 'idle' | 'connecting' | 'live'
  const btn = $('startStop');
  if (!btn) return;
  const map = {
    loading: ['Loading avatar…', true],
    idle: ['▶︎ Start', false],
    connecting: ['Connecting…', true],
    live: ['⏹ Stop', false],
  };
  const [label, disabled] = map[state] || map.idle;
  btn.textContent = label;
  btn.disabled = disabled;
  btn.dataset.state = state;
}

export function on(id, ev, fn) {
  const el = $(id);
  if (el) el.addEventListener(ev, fn);
}


```

## Source file: backend/main.py

{
  "path": "backend/main.py",
  "blob_sha": "2db70f875d9df831cbb44f90d10a498e4fc777ef",
  "sha256": "68f12647803883e770505bb1a866dc131f207ca77ec49c9db2c3585200f8268c",
  "lines": 136,
  "bytes": 5932,
  "selection_score": 0,
  "keyword_line_counts": {
    "lip": 3
  },
  "url": "https://github.com/xanguera/aicompanion/blob/34104ccddb29a38a44afadb9bf09c11598601eae/backend/main.py",
  "function_signature_candidates": [
    {
      "line": 45,
      "text": "def _llm_mock_enabled() -> bool:"
    },
    {
      "line": 49,
      "text": "async def run_pipeline(websocket) -> None:"
    },
    {
      "line": 100,
      "text": "async def on_client_connected(_transport, _client):"
    },
    {
      "line": 107,
      "text": "async def on_client_ready(rtvi):"
    },
    {
      "line": 131,
      "text": "async def on_client_disconnected(_transport, _client):"
    }
  ]
}

```text

"""Assemble and run one Pipecat pipeline per WebSocket connection.

Pipeline (streaming, end-to-end):

    transport.input()
      -> InterruptionBridge            (barge-in -> {type:'interrupt'} to client)
      -> STT (Whisper, segmented)
      -> user_aggregator               (universal context; holds the VAD analyzer)
      -> [memory]                      (Hindsight recall before LLM; optional)
      -> LLM (OpenAI, streams tokens)  (the only network call)
      -> KokoroLipsyncTTSService       (sentence -> RTVIServerMessageFrame{tts_sentence})
      -> assistant_aggregator

The LLM token stream is cut into sentences by the base TTSService's built-in aggregation, so
sentence N+1 synthesizes while sentence N plays in the browser.

Pipecat 1.4.0 notes (verified):
  - PipelineWorker (not the deprecated PipelineTask) + WorkerRunner is the run pattern.
  - enable_rtvi=True (default) auto-adds the RTVIProcessor + RTVIObserver, so we DON'T wire them
    manually. The RTVIServerMessageFrames our TTS service pushes are forwarded to the browser's
    onServerMessage by that default observer. ``worker.rtvi`` is the auto-created processor.
"""
from __future__ import annotations

import os

from loguru import logger
from pipecat.frames.frames import LLMRunFrame
from pipecat.pipeline.pipeline import Pipeline
from pipecat.pipeline.runner import WorkerRunner
from pipecat.pipeline.task import PipelineParams, PipelineWorker
from pipecat.processors.frameworks.rtvi import RTVIServerMessageFrame

from backend import protocol
from backend.pipeline.asr_whisper import build_stt
from backend.pipeline.interrupt import InterruptionBridge
from backend.pipeline.llm import build_context_aggregators, build_llm
from backend.pipeline.memory import build_memory
from backend.pipeline.timing import TimingObserver
from backend.pipeline.transport import build_transport
from backend.pipeline.tts_kokoro import KokoroLipsyncTTSService
from backend.pipeline.vad import build_vad


def _llm_mock_enabled() -> bool:
    return os.environ.get("LLM_MOCK", "0").strip().lower() in ("1", "true", "yes", "on")


async def run_pipeline(websocket) -> None:
    """Build the pipeline for one connection and run it until the client disconnects."""
    transport = build_transport(websocket)
    vad = build_vad()
    stt = build_stt()
    context, user_aggregator, assistant_aggregator = build_context_aggregators(vad)
    tts = KokoroLipsyncTTSService()
    memory = build_memory()
    interrupt_bridge = InterruptionBridge()

    if _llm_mock_enabled():
        from backend.pipeline.mock_services import MockLLMService

        logger.info("LLM: MockLLMService (LLM_MOCK=1)")
        llm = MockLLMService()
    else:
        llm = build_llm()

    pipeline = Pipeline(
        [
            transport.input(),
            interrupt_bridge,
            stt,
            user_aggregator,
            *([memory] if memory is not None else []),
            llm,
            tts,
            # transport.output() is the channel that sends RTVI messages (bot-ready, tts_sentence)
            # to the client. It's REQUIRED even though audio_out_enabled=False — that flag only
            # stops it carrying TTS *audio* (TalkingHead plays that). Omitting it means the client
            # connects but never receives bot-ready or any server message.
            transport.output(),
            assistant_aggregator,
        ]
    )

    worker = PipelineWorker(
        pipeline,
        params=PipelineParams(
            audio_in_sample_rate=16_000,
            enable_metrics=True,
            enable_usage_metrics=True,
        ),
        # Passive latency instrumentation (logs per-stage timings; changes nothing). RTVI is still
        # auto-added alongside since this isn't an RTVIObserver.
        observers=[TimingObserver()],
        # The bot's audio never enters transport.output(), so don't treat its absence as idle.
        idle_timeout_secs=None,
    )

    @transport.event_handler("on_client_connected")
    async def on_client_connected(_transport, _client):
        logger.info("Client connected (transport)")

    # bot-ready MUST be sent in response to the client's client-ready (not on transport-connect),
    # otherwise the browser's connect() — which waits for bot-ready after sending client-ready —
    # misses it and hangs. on_client_ready fires when client-ready arrives.
    @worker.rtvi.event_handler("on_client_ready")
    async def on_client_ready(rtvi):
        try:
            logger.info("client-ready received; sending bot-ready + greeting")
            await rtvi.set_bot_ready()
            await rtvi.push_frame(RTVIServerMessageFrame(data=protocol.ready()))
            # Open the conversation. Vary it and lean on memory so it isn't a rote "hi there".
            context.add_message(
                {
                    "role": "developer",
                    "content": (
                        "The conversation is starting. If you remember things about this person from "
                        "past conversations, open by warmly referencing something specific you recall "
                        "about them (and use the name they gave you, if any). If you don't remember "
                        "them yet, introduce yourself warmly as their companion and say you'd love to "
                        "get to know them. Either way, end with one friendly, open question. Two or "
                        "three spoken sentences."
                    ),
                }
            )
            await worker.queue_frames([LLMRunFrame()])
        except Exception:
            logger.exception("on_client_ready failed")

    @transport.event_handler("on_client_disconnected")
    async def on_client_disconnected(_transport, _client):
        logger.info("Client disconnected")
        await worker.cancel()

    runner = WorkerRunner(handle_sigint=False)
    await runner.run(worker)


```

## Source file: frontend/app.js

{
  "path": "frontend/app.js",
  "blob_sha": "51df0d0c04325c0f0b03d9dcf007fa350193fd09",
  "sha256": "aad690d3806851aede45ca48de89ce7384c525b47191c820a23715ecfb5e83c5",
  "lines": 125,
  "bytes": 3963,
  "selection_score": 0,
  "keyword_line_counts": {
    "mouth": 2,
    "face": 1
  },
  "url": "https://github.com/xanguera/aicompanion/blob/34104ccddb29a38a44afadb9bf09c11598601eae/frontend/app.js",
  "function_signature_candidates": []
}

```text

// App entry: boot the avatar, then wire the Pipecat client to the backend.
//
// Pipecat owns the brain (mic -> VAD -> STT -> memory -> LLM -> sentence-chunk -> Kokoro);
// the browser owns the mouth: each `tts_sentence` RTVI message is played via TalkingHead's
// speakAudio(), and `interrupt` calls stopSpeaking(). See the build plan for the full rationale.

import { PipecatClient } from '@pipecat-ai/client-js';
import { WebSocketTransport, ProtobufFrameSerializer } from '@pipecat-ai/websocket-transport';

import { initAvatar, bootAvatar, resumeAudio, speakSentence, stopSpeaking, noteSeq, toggleZoom } from './avatar.js';
import { setStatus, addCaption, setButton, on } from './ui.js';

window.__bootStarted = true; // tells the inline watchdog the module executed

const WS_URL = `${location.protocol === 'https:' ? 'wss' : 'ws'}://${location.host}/ws`;
// Mic is captured at 16 kHz — what Silero VAD and Whisper expect.
const MIC_SAMPLE_RATE = 16000;

let client = null;

function handleServerMessage(data) {
  if (!data || !data.type) return;
  switch (data.type) {
    case 'ready':
      setStatus('Connected. Listening…');
      break;
    case 'tts_sentence':
      noteSeq(data.seq);
      speakSentence(data).catch((e) => console.error('[app] speakSentence failed', e));
      break;
    case 'assistant_text':
      addCaption('assistant', data.text);
      break;
    case 'user_transcript':
      if (data.final) addCaption('user', data.text);
      break;
    case 'interrupt':
      stopSpeaking();
      break;
    case 'turn_complete':
      break;
    case 'status':
      setStatus(data.message, data.warn);
      break;
    case 'error':
      setStatus(`Error: ${data.message}`, true);
      break;
    default:
      console.debug('[app] unhandled server message', data);
  }
}

function buildClient() {
  const transport = new WebSocketTransport({
    serializer: new ProtobufFrameSerializer(),
    recorderSampleRate: MIC_SAMPLE_RATE,
  });
  return new PipecatClient({
    transport,
    enableMic: true,
    enableCam: false,
    callbacks: {
      onConnected: () => { console.log('[rtvi] onConnected'); setStatus('Connected.'); },
      onBotReady: (d) => { console.log('[rtvi] onBotReady', JSON.stringify(d)); },
      onError: (e) => { console.error('[rtvi] onError', JSON.stringify(e)); },
      onDisconnected: () => {
        console.log('[rtvi] onDisconnected');
        setStatus('Disconnected.');
        setButton('idle');
      },
      onServerMessage: (data) => { console.debug('[rtvi] onServerMessage', data && data.type); handleServerMessage(data); },
      onUserStartedSpeaking: () => stopSpeaking(), // local barge-in feel; backend also confirms
      onUserTranscript: (t) => t && addCaption('user', t.text),
    },
  });
}

async function start() {
  try {
    setButton('connecting');
    await resumeAudio(); // unlock AudioContext on the user gesture
    client = buildClient();
    await client.connect({ wsUrl: WS_URL });
    setButton('live');
    setStatus('Live — say hello.');
  } catch (err) {
    console.error('[app] connect failed', err);
    setStatus(`Connect failed: ${err && err.message ? err.message : err}`, true);
    setButton('idle');
  }
}

async function stop() {
  try {
    stopSpeaking();
    if (client) await client.disconnect();
  } finally {
    client = null;
    setButton('idle');
    setStatus('Stopped.');
  }
}

async function main() {
  setButton('loading');
  initAvatar(document.getElementById('avatar'));
  try {
    await bootAvatar();
  } catch {
    return; // bootAvatar already surfaced the error
  }
  setButton('idle');

  on('startStop', 'click', () => {
    const live = document.getElementById('startStop').dataset.state === 'live';
    if (live) stop();
    else start();
  });
  on('zoom', 'click', () => {
    const z = toggleZoom();
    document.getElementById('zoom').textContent = z ? '🔍 Regular view' : '🔍 Zoom to mouth';
  });
}

main();


```
