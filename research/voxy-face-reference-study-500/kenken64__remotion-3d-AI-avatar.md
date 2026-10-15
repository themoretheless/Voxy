# Facial rendering source scan: kenken64/remotion-3d-AI-avatar

Repository: https://github.com/kenken64/remotion-3d-AI-avatar

Pinned source revision: 8bc42f51cb56d4fff3fc415b298c07e1273d52d0

Discovery topic: mouth animation

Description: A 3D AI-powered talking avatar with real-time lip-sync. Chat with the avatar through a web interface — it responds using OpenClaw and speaks with synchronized mouth animation driven by OpenAI TTS audio.

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"lip": ["src/app/useAudioLipSync.ts", "src/lipSync.ts"]}

## Source file: src/lipSync.ts

{
  "path": "src/lipSync.ts",
  "blob_sha": "faf331df9462c95a11b30e593d990f780f93948a",
  "sha256": "f732e42b79b1370dfa2bb1a39a6659c125addfea581867a6e5598a294b6e4240",
  "lines": 209,
  "bytes": 5275,
  "selection_score": 15,
  "keyword_line_counts": {
    "mouth": 16,
    "face": 1
  },
  "url": "https://github.com/kenken64/remotion-3d-AI-avatar/blob/8bc42f51cb56d4fff3fc415b298c07e1273d52d0/src/lipSync.ts",
  "function_signature_candidates": []
}

```text

export type MouthShape = 'closed' | 'A' | 'E' | 'I' | 'O' | 'U' | 'F' | 'L' | 'M';

// Map each English character to a viseme (mouth shape)
const charToViseme: Record<string, MouthShape> = {
  a: 'A',
  b: 'M',
  c: 'E',
  d: 'E',
  e: 'E',
  f: 'F',
  g: 'E',
  h: 'A',
  i: 'I',
  j: 'I',
  k: 'E',
  l: 'L',
  m: 'M',
  n: 'L',
  o: 'O',
  p: 'M',
  q: 'O',
  r: 'E',
  s: 'E',
  t: 'E',
  u: 'U',
  v: 'F',
  w: 'U',
  x: 'E',
  y: 'I',
  z: 'E',
};

// Chinese characters are CJK Unified Ideographs: U+4E00–U+9FFF
function isChinese(char: string): boolean {
  const code = char.charCodeAt(0);
  return code >= 0x4e00 && code <= 0x9fff;
}

// Map Chinese characters to visemes based on char code for varied mouth movement.
// Each Chinese syllable typically has an open vowel sound, so we rotate through
// speaking shapes to create natural-looking articulation.
const chineseVisemes: MouthShape[] = ['A', 'O', 'E', 'I', 'U', 'A', 'L', 'E', 'O', 'M'];

function getChineseViseme(char: string): MouthShape {
  const code = char.charCodeAt(0);
  return chineseVisemes[code % chineseVisemes.length];
}

// Tamil block: U+0B80–U+0BFF
function isTamil(char: string): boolean {
  const code = char.charCodeAt(0);
  return code >= 0x0b80 && code <= 0x0bff;
}

// Map each Tamil codepoint to a viseme. Vowels and matras (vowel signs) drive
// the mouth shape; consonants fall back to a shape based on place of
// articulation. The virama (், U+0BCD) suppresses the inherent vowel so we
// render it as a brief closed transition.
const tamilToViseme: Record<number, MouthShape> = {
  // Vowels
  0x0b85: 'A', // அ
  0x0b86: 'A', // ஆ
  0x0b87: 'I', // இ
  0x0b88: 'I', // ஈ
  0x0b89: 'U', // உ
  0x0b8a: 'U', // ஊ
  0x0b8e: 'E', // எ
  0x0b8f: 'E', // ஏ
  0x0b90: 'I', // ஐ
  0x0b92: 'O', // ஒ
  0x0b93: 'O', // ஓ
  0x0b94: 'U', // ஔ
  // Vowel signs (matras)
  0x0bbe: 'A', // ா
  0x0bbf: 'I', // ி
  0x0bc0: 'I', // ீ
  0x0bc1: 'U', // ு
  0x0bc2: 'U', // ூ
  0x0bc6: 'E', // ெ
  0x0bc7: 'E', // ே
  0x0bc8: 'I', // ை
  0x0bca: 'O', // ொ
  0x0bcb: 'O', // ோ
  0x0bcc: 'U', // ௌ
  0x0bcd: 'closed', // ் virama
  // Consonants
  0x0b95: 'A', // க
  0x0b99: 'A', // ங
  0x0b9a: 'E', // ச
  0x0b9c: 'E', // ஜ
  0x0b9e: 'I', // ஞ
  0x0b9f: 'E', // ட
  0x0ba3: 'L', // ண
  0x0ba4: 'E', // த
  0x0ba8: 'L', // ந
  0x0ba9: 'L', // ன
  0x0baa: 'M', // ப
  0x0bae: 'M', // ம
  0x0baf: 'I', // ய
  0x0bb0: 'E', // ர
  0x0bb1: 'E', // ற
  0x0bb2: 'L', // ல
  0x0bb3: 'L', // ள
  0x0bb4: 'L', // ழ
  0x0bb5: 'F', // வ
  0x0bb6: 'E', // ஶ
  0x0bb7: 'E', // ஷ
  0x0bb8: 'E', // ஸ
  0x0bb9: 'A', // ஹ
};

function getTamilViseme(char: string): MouthShape {
  return tamilToViseme[char.charCodeAt(0)] ?? 'A';
}

interface VisemeFrame {
  shape: MouthShape;
  startFrame: number;
  endFrame: number;
}

/**
 * Convert text into a sequence of viseme frames.
 * Each character gets a fixed number of frames.
 * Spaces and punctuation get 'closed' mouth.
 */
export function textToVisemes(
  text: string,
  fps: number,
  totalFrames: number,
): VisemeFrame[] {
  const chars = text.toLowerCase().split('');
  const framesPerChar = Math.max(2, Math.floor(totalFrames / Math.max(chars.length, 1)));
  const visemes: VisemeFrame[] = [];

  let currentFrame = 0;

  for (const char of chars) {
    if (currentFrame >= totalFrames) break;

    const isLetter = /[a-z]/.test(char);
    const isCJK = isChinese(char);
    const isTamilChar = isTamil(char);
    const shape: MouthShape = isLetter
      ? (charToViseme[char] || 'A')
      : isCJK
        ? getChineseViseme(char)
        : isTamilChar
          ? getTamilViseme(char)
          : 'closed';

    // Add a brief transition to closed between words and punctuation
    // (English + Chinese + Tamil danda). Tamil itself uses ASCII punctuation.
    if (char === ' ' || /[.,!?;:，。！？；：、।॥]/.test(char)) {
      visemes.push({
        shape: 'closed',
        startFrame: currentFrame,
        endFrame: Math.min(currentFrame + Math.ceil(framesPerChar * 1.2), totalFrames),
      });
      currentFrame += Math.ceil(framesPerChar * 1.2);
    } else {
      visemes.push({
        shape,
        startFrame: currentFrame,
        endFrame: Math.min(currentFrame + framesPerChar, totalFrames),
      });
      currentFrame += framesPerChar;
    }
  }

  // Fill remaining frames with closed mouth
  if (currentFrame < totalFrames) {
    visemes.push({
      shape: 'closed',
      startFrame: currentFrame,
      endFrame: totalFrames,
    });
  }

  return visemes;
}

/**
 * Get the mouth shape for a specific frame.
 */
export function getMouthShapeAtFrame(
  visemes: VisemeFrame[],
  frame: number,
): MouthShape {
  for (const v of visemes) {
    if (frame >= v.startFrame && frame < v.endFrame) {
      return v.shape;
    }
  }
  return 'closed';
}

/**
 * Get the current character index being spoken at a given frame.
 */
export function getCharIndexAtFrame(
  text: string,
  totalFrames: number,
  frame: number,
): number {
  const chars = text.split('');
  const framesPerChar = Math.max(2, Math.floor(totalFrames / Math.max(chars.length, 1)));
  return Math.min(Math.floor(frame / framesPerChar), chars.length);
}


```

## Source file: src/app/useAudioLipSync.ts

{
  "path": "src/app/useAudioLipSync.ts",
  "blob_sha": "4ffaf07d30f88c04f7216236005256bb43b37581",
  "sha256": "d568c76efff4e37155c80dbbf388ef1a1806a59030b81de948cb453e2fa16fd7",
  "lines": 227,
  "bytes": 7425,
  "selection_score": 15,
  "keyword_line_counts": {
    "mouth": 9,
    "face": 2,
    "lip": 9
  },
  "url": "https://github.com/kenken64/remotion-3d-AI-avatar/blob/8bc42f51cb56d4fff3fc415b298c07e1273d52d0/src/app/useAudioLipSync.ts",
  "function_signature_candidates": []
}

```text

import {useState, useRef, useCallback} from 'react';
import {
  MouthShape,
  textToVisemes,
  getMouthShapeAtFrame,
  getCharIndexAtFrame,
} from '../lipSync';

const FPS = 30;

interface AudioLipSyncState {
  mouthShape: MouthShape;
  isSpeaking: boolean;
  spokenText: string;
  currentSpeech: string;
}

// Each clip in the queue plays back-to-back. `ownsUrl` is true for streaming
// segments (we created the object URL from a Blob and must revoke it on end);
// false for replay (the URL is a stored reference on a chat message and must
// outlive the playback).
interface QueueItem {
  text: string;
  url: string;
  ownsUrl: boolean;
}

export function useAudioLipSync() {
  const [state, setState] = useState<AudioLipSyncState>({
    mouthShape: 'closed',
    isSpeaking: false,
    spokenText: '',
    currentSpeech: '',
  });

  const audioRef = useRef<HTMLAudioElement | null>(null);
  const rafRef = useRef<number>(0);
  const mutedRef = useRef<boolean>(false);

  // Queue of pending clips. Items are appended by streaming `enqueue` calls
  // and consumed in order by `playNext`. `playingRef` guards against double-
  // starting the player, `streamDoneRef` tells the player whether the queue
  // is closed so it can resolve the finish promise once it drains.
  const queueRef = useRef<QueueItem[]>([]);
  const playingRef = useRef(false);
  const streamDoneRef = useRef(true);
  // Concatenation of every clip's text that has finished playing in the
  // current session. Subtitles render as `completedPrefix + active-clip text`
  // so a multi-sentence streamed reply reads as one continuous transcript.
  const completedPrefixRef = useRef('');
  // Resolvers awaiting full drain (one per pending `finish()` / `speak()` /
  // `replay()` call). Fired once the queue is empty AND the stream is closed.
  const finishResolversRef = useRef<Array<() => void>>([]);

  const setMuted = useCallback((muted: boolean) => {
    mutedRef.current = muted;
    if (audioRef.current) audioRef.current.muted = muted;
  }, []);

  const stopAudio = useCallback(() => {
    if (rafRef.current) cancelAnimationFrame(rafRef.current);
    rafRef.current = 0;
    if (audioRef.current) {
      audioRef.current.pause();
      audioRef.current = null;
    }
  }, []);

  const cancel = useCallback(() => {
    stopAudio();
    for (const item of queueRef.current) {
      if (item.ownsUrl) URL.revokeObjectURL(item.url);
    }
    queueRef.current = [];
    playingRef.current = false;
    streamDoneRef.current = true;
    completedPrefixRef.current = '';
    const resolvers = finishResolversRef.current;
    finishResolversRef.current = [];
    for (const r of resolvers) r();
    setState({mouthShape: 'closed', isSpeaking: false, spokenText: '', currentSpeech: ''});
  }, [stopAudio]);

  const playNext = useCallback(() => {
    const next = queueRef.current.shift();
    if (!next) {
      playingRef.current = false;
      // Only finalize when the producer has explicitly closed the stream.
      // Otherwise we may just be momentarily ahead of TTS fetches.
      if (streamDoneRef.current) {
        const finalText = completedPrefixRef.current;
        setState({
          mouthShape: 'closed',
          isSpeaking: false,
          spokenText: finalText,
          currentSpeech: '',
        });
        const resolvers = finishResolversRef.current;
        finishResolversRef.current = [];
        for (const r of resolvers) r();
      }
      return;
    }

    playingRef.current = true;
    const audio = new Audio(next.url);
    audio.muted = mutedRef.current;
    audioRef.current = audio;

    const segPrefix = completedPrefixRef.current;
    const segText = next.text;
    const joined = segPrefix ? `${segPrefix} ${segText}` : segText;

    setState({
      mouthShape: 'closed',
      isSpeaking: true,
      spokenText: segPrefix,
      currentSpeech: joined,
    });

    audio.addEventListener('loadedmetadata', () => {
      const duration = audio.duration;
      const totalFrames = Math.ceil(duration * FPS);
      const visemes = textToVisemes(segText, FPS, totalFrames);

      const tick = () => {
        if (audioRef.current !== audio) return;
        const t = audio.currentTime;
        const frame = Math.floor(t * FPS);
        const shape = getMouthShapeAtFrame(visemes, frame);
        const charIdx = getCharIndexAtFrame(segText, totalFrames, frame);
        setState({
          mouthShape: shape,
          isSpeaking: true,
          spokenText: segPrefix ? `${segPrefix} ${segText.slice(0, charIdx)}` : segText.slice(0, charIdx),
          currentSpeech: joined,
        });
        if (!audio.paused && !audio.ended) {
          rafRef.current = requestAnimationFrame(tick);
        }
      };

      audio.play().then(() => {
        rafRef.current = requestAnimationFrame(tick);
      }).catch(() => {/* autoplay blocked etc. */});
    });

    const advance = () => {
      if (audioRef.current !== audio) return;
      if (rafRef.current) cancelAnimationFrame(rafRef.current);
      rafRef.current = 0;
      audioRef.current = null;
      completedPrefixRef.current = segPrefix ? `${segPrefix} ${segText}` : segText;
      if (next.ownsUrl) URL.revokeObjectURL(next.url);
      playNext();
    };

    audio.addEventListener('ended', advance);
    audio.addEventListener('error', advance);
  }, []);

  // Begin a streaming session. The caller pushes audio clips with `enqueue`
  // as TTS for each sentence resolves, then calls `finish()` once the chat
  // stream has closed and the last sentence has been enqueued. `finish()`
  // resolves only when the player has drained the queue.
  const beginStream = useCallback(() => {
    cancel();
    streamDoneRef.current = false;

    return {
      enqueue: (text: string, blob: Blob) => {
        if (streamDoneRef.current && !playingRef.current) {
          // Producer already called finish(); ignore late arrivals.
          return;
        }
        const url = URL.createObjectURL(blob);
        queueRef.current.push({text, url, ownsUrl: true});
        if (!playingRef.current) playNext();
      },
      finish: (): Promise<void> => {
        streamDoneRef.current = true;
        if (!playingRef.current && queueRef.current.length === 0) {
          return Promise.resolve();
        }
        return new Promise((resolve) => {
          finishResolversRef.current.push(resolve);
        });
      },
      cancel,
    };
  }, [cancel, playNext]);

  // One-shot helper: speak a fully-resolved reply (used by paths that don't
  // stream — vision, sketch, music, telegram inject).
  const speak = useCallback(
    (text: string, audioBlob: Blob): Promise<void> => {
      const session = beginStream();
      session.enqueue(text, audioBlob);
      return session.finish();
    },
    [beginStream],
  );

  // Replay a previously-stored reply (URL kept on the chat message). We don't
  // own the URL so it survives playback and can be replayed again.
  const replay = useCallback(
    (text: string, audioUrl: string): Promise<void> => {
      cancel();
      streamDoneRef.current = true;
      queueRef.current = [{text, url: audioUrl, ownsUrl: false}];
      return new Promise((resolve) => {
        finishResolversRef.current.push(resolve);
        playNext();
      });
    },
    [cancel, playNext],
  );

  return {
    ...state,
    speak,
    replay,
    setMuted,
    beginStream,
    cancel,
  };
}


```

## Source file: src/index.ts

{
  "path": "src/index.ts",
  "blob_sha": "d831f7b0f12b8445b8a2d547eb2bd0c3559a5d36",
  "sha256": "5462d3268136731ce43bfd7c5072ea1ba48b51fec94fa30076f5044f4b39e4a2",
  "lines": 4,
  "bytes": 105,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/kenken64/remotion-3d-AI-avatar/blob/8bc42f51cb56d4fff3fc415b298c07e1273d52d0/src/index.ts",
  "function_signature_candidates": []
}

```text

import {registerRoot} from 'remotion';
import {RemotionRoot} from './Root';

registerRoot(RemotionRoot);


```

## Source file: src/app/api.ts

{
  "path": "src/app/api.ts",
  "blob_sha": "a66bd207cef48e7e952e632f7a6dd783b74ae877",
  "sha256": "509aebd18d053fe27046c96298739017f4a1f9f5a13cee89aee3cf4a984b7894",
  "lines": 169,
  "bytes": 4740,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 1,
    "face": 3
  },
  "url": "https://github.com/kenken64/remotion-3d-AI-avatar/blob/8bc42f51cb56d4fff3fc415b298c07e1273d52d0/src/app/api.ts",
  "function_signature_candidates": []
}

```text

export interface ChatMsg {
  role: 'user' | 'assistant';
  content: string;
}

export interface ChatReply {
  reply: string;
  imageUrl?: string;
}

export async function sendChatMessage(messages: ChatMsg[]): Promise<ChatReply> {
  const res = await fetch('/api/chat', {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({messages}),
  });

  if (!res.ok) {
    throw new Error(`Chat request failed: ${res.status}`);
  }

  const data = await res.json();
  return {reply: data.reply, imageUrl: data.imageUrl};
}

export async function transcribeAudio(audioBlob: Blob): Promise<string> {
  const reader = new FileReader();
  const base64 = await new Promise<string>((resolve) => {
    reader.onloadend = () => {
      const result = reader.result as string;
      resolve(result.split(',')[1]);
    };
    reader.readAsDataURL(audioBlob);
  });

  const res = await fetch('/api/transcribe', {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({audio: base64}),
  });

  if (!res.ok) {
    throw new Error(`Transcription failed: ${res.status}`);
  }

  const data = await res.json();
  return data.text;
}

export interface StreamHandlers {
  onDelta: (delta: string) => void;
  onDone: (info: {full: string; imageUrl?: string}) => void;
  onError?: (err: Error) => void;
}

// Reads the SSE body of /api/chat-stream and dispatches each event to the
// caller. Each `delta` event carries an incremental text fragment; a single
// `done` event carries the assembled full reply and any generated image URL.
export async function streamChatMessage(
  messages: ChatMsg[],
  handlers: StreamHandlers,
  signal?: AbortSignal,
): Promise<void> {
  const res = await fetch('/api/chat-stream', {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({messages}),
    signal,
  });

  if (!res.ok || !res.body) {
    throw new Error(`Chat stream failed: ${res.status}`);
  }

  const reader = res.body.getReader();
  const decoder = new TextDecoder();
  let buf = '';

  while (true) {
    const {value, done} = await reader.read();
    if (done) break;
    buf += decoder.decode(value, {stream: true});
    let sep;
    while ((sep = buf.indexOf('\n\n')) !== -1) {
      const block = buf.slice(0, sep);
      buf = buf.slice(sep + 2);
      let event = 'message';
      let data = '';
      for (const line of block.split('\n')) {
        if (line.startsWith('event: ')) event = line.slice(7).trim();
        else if (line.startsWith('data: ')) data += line.slice(6);
      }
      if (!data) continue;
      let parsed: any;
      try {
        parsed = JSON.parse(data);
      } catch {
        continue;
      }
      if (event === 'delta') handlers.onDelta(String(parsed.text || ''));
      else if (event === 'done') {
        handlers.onDone({full: String(parsed.full || ''), imageUrl: parsed.imageUrl});
        return;
      } else if (event === 'error') {
        handlers.onError?.(new Error(String(parsed.error || 'stream error')));
        return;
      }
    }
  }
}

// Buffers streamed text and flushes one chunk at a time at sentence
// boundaries. We require a minimum length before flushing so abbreviations
// (e.g. "Mr." or "1.5") don't trigger a premature split — the buffer keeps
// growing until a punctuation arrives that would yield a piece long enough
// to be worth speaking.
export function makeSentenceFlusher(
  onSentence: (sentence: string) => void,
  minLength = 20,
) {
  // Punctuation that ends a sentence, possibly followed by a closing quote.
  const BOUNDARY = /[.?!。！？]+["')\]」』]?(?=\s|$)/g;
  let buf = '';

  const tryFlush = () => {
    while (buf.trim().length >= minLength) {
      BOUNDARY.lastIndex = 0;
      let cut = -1;
      let m: RegExpExecArray | null;
      while ((m = BOUNDARY.exec(buf)) !== null) {
        const end = m.index + m[0].length;
        if (buf.slice(0, end).trim().length >= minLength) {
          cut = end;
          break;
        }
      }
      if (cut < 0) return;
      const piece = buf.slice(0, cut).trim();
      buf = buf.slice(cut).replace(/^\s+/, '');
      if (piece) onSentence(piece);
    }
  };

  return {
    push(delta: string) {
      buf += delta;
      tryFlush();
    },
    flush() {
      const tail = buf.trim();
      buf = '';
      if (tail) onSentence(tail);
    },
  };
}

export async function fetchSpeechAudio(text: string): Promise<Blob> {
  const res = await fetch('/api/tts', {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({text}),
  });

  if (!res.ok) {
    throw new Error(`TTS request failed: ${res.status}`);
  }

  return res.blob();
}


```
