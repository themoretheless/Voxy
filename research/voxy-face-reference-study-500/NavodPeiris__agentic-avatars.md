# Facial rendering source scan: NavodPeiris/agentic-avatars

Repository: https://github.com/NavodPeiris/agentic-avatars

Pinned source revision: fbc1bd4a270516f00f535fc9ebeaf8429e1ba57c

Discovery topic: lip sync 3D

Description: Zero-Infrastructure Lip-Synced 3D avatar components for AI voice agents. This library is built with React Three Fiber and AI providers

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE", "agentic-avatars/LICENSE"]

Topic paths: {"lip": ["agentic-avatars/src/audio/useAvatarLipsync.ts", "agentic-avatars/src/audio/usePushAudioLipsync.ts", "agentic-avatars/src/audio/useVolumeFallbackLipsync.ts", "agentic-avatars/src/audio/wav2arkit/liveLipsync.ts"]}

## Source file: agentic-avatars/src/audio/useAvatarLipsync.ts

{
  "path": "agentic-avatars/src/audio/useAvatarLipsync.ts",
  "blob_sha": "b10f2e9a68df3a252401a8f7b4bbeb6112c26d01",
  "sha256": "0c09b066aa88c23bb210be15d31e89fecf99238435a46cbfbba1ace14eb93686",
  "lines": 45,
  "bytes": 1841,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 3,
    "face": 1,
    "lip": 8
  },
  "url": "https://github.com/NavodPeiris/agentic-avatars/blob/fbc1bd4a270516f00f535fc9ebeaf8429e1ba57c/agentic-avatars/src/audio/useAvatarLipsync.ts",
  "function_signature_candidates": []
}

```text

import { useEffect } from 'react';
import type { RefObject } from 'react';
import { Wav2ArkitLipsync } from './wav2arkit/liveLipsync';
import { AgentLevelMeter, setActiveAgentLevelMeter } from './wav2arkit/agentLevelMeter';
import type { IAvatarController } from '../avatar/GaussianAvatarController';

interface UseAvatarLipsyncOptions {
  /** The remote agent audio stream. When this transitions from null to a stream,
   *  the lipsync pipeline is set up. Must be reactive state (not a ref). */
  remoteStream: MediaStream | null;
  /** Ref to the mounted avatar controller — decoded blendshape frames are pushed here. */
  controllerRef: RefObject<IAvatarController | null>;
  onStartRecording?: (stream: MediaStream) => void;
  onStopRecording?: () => void;
}

/**
 * Wires the remote audio stream into the wav2arkit lipsync engine whenever
 * the stream becomes available, streaming decoded ARKit blendshapes into
 * the avatar controller, and tears down cleanly when it goes away.
 */
export function useAvatarLipsync({ remoteStream, controllerRef, onStartRecording, onStopRecording }: UseAvatarLipsyncOptions) {
  useEffect(() => {
    if (!remoteStream || remoteStream.getAudioTracks().length === 0) return;

    const lipsync = new Wav2ArkitLipsync({
      onFrame: (weights) => controllerRef.current?.updateBlendshapes(weights),
    });
    const levelMeter = new AgentLevelMeter();

    levelMeter.start(remoteStream);
    setActiveAgentLevelMeter(levelMeter);
    lipsync.start(remoteStream);

    onStartRecording?.(remoteStream);

    return () => {
      lipsync.stop();
      levelMeter.stop();
      setActiveAgentLevelMeter(null);
      onStopRecording?.();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [remoteStream]); // re-runs when the stream reference changes (null → stream → null)
}


```

## Source file: agentic-avatars/src/audio/usePushAudioLipsync.ts

{
  "path": "agentic-avatars/src/audio/usePushAudioLipsync.ts",
  "blob_sha": "f33f04a097dd46538b1c558c3d0a3609738f2bc9",
  "sha256": "cfcabd47b8bb26d2eef585509f51854e299eee0657b2fa505d9d088743cfee83",
  "lines": 46,
  "bytes": 1753,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 1,
    "face": 1,
    "lip": 7
  },
  "url": "https://github.com/NavodPeiris/agentic-avatars/blob/fbc1bd4a270516f00f535fc9ebeaf8429e1ba57c/agentic-avatars/src/audio/usePushAudioLipsync.ts",
  "function_signature_candidates": []
}

```text

import { useEffect } from 'react';
import type { RefObject } from 'react';
import { Wav2ArkitLipsync } from './wav2arkit/liveLipsync';
import type { IAvatarController } from '../avatar/GaussianAvatarController';

const IDLE_AFTER_MS = 400;

interface UsePushAudioLipsyncOptions {
  /** Present only for adapters that decode raw PCM themselves and can push samples directly (e.g. Deepgram). */
  subscribeToRemoteAudio?: (handler: (chunk: Float32Array, sampleRate: number) => void) => () => void;
  controllerRef: RefObject<IAvatarController | null>;
}

/**
 * Full wav2arkit neural lipsync fed directly from an adapter's already-
 * decoded PCM audio, for adapters that never produce a browser MediaStream
 * (e.g. Deepgram, which decodes raw PCM16 frames from its own WebSocket).
 */
export function usePushAudioLipsync({ subscribeToRemoteAudio, controllerRef }: UsePushAudioLipsyncOptions) {
  useEffect(() => {
    if (!subscribeToRemoteAudio) return;

    const lipsync = new Wav2ArkitLipsync({
      onFrame: (weights) => controllerRef.current?.updateBlendshapes(weights),
    });

    let idleTimer: ReturnType<typeof setTimeout> | null = null;
    const scheduleIdle = () => {
      if (idleTimer) clearTimeout(idleTimer);
      idleTimer = setTimeout(() => controllerRef.current?.setChatState('Idle'), IDLE_AFTER_MS);
    };

    const unsubscribe = subscribeToRemoteAudio((chunk, sampleRate) => {
      controllerRef.current?.setChatState('Responding');
      scheduleIdle();
      lipsync.pushAudio(chunk, sampleRate);
    });

    return () => {
      if (idleTimer) clearTimeout(idleTimer);
      unsubscribe();
      lipsync.stop();
      controllerRef.current?.setChatState('Idle');
    };
  }, [subscribeToRemoteAudio, controllerRef]);
}


```

## Source file: agentic-avatars/src/audio/wav2arkit/liveLipsync.ts

{
  "path": "agentic-avatars/src/audio/wav2arkit/liveLipsync.ts",
  "blob_sha": "3dd3315570d43e46087c248a8f95a4db0ec3ae79",
  "sha256": "eac14bdce5992b6a3352facea7583539c959c99e09f8d3213284d82b38b09b70",
  "lines": 171,
  "bytes": 6545,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 4,
    "face": 2,
    "lip": 3
  },
  "url": "https://github.com/NavodPeiris/agentic-avatars/blob/fbc1bd4a270516f00f535fc9ebeaf8429e1ba57c/agentic-avatars/src/audio/wav2arkit/liveLipsync.ts",
  "function_signature_candidates": []
}

```text

import { ARKIT_BLENDSHAPE_NAMES } from '../../constants/arkit';
import { resampleTo16k } from './resample';
import { getSharedWav2ArkitEngine } from './inferenceEngine';
import type { Wav2ArkitEngineOptions } from './inferenceEngine';

const CHUNK_MS = 200; // audio window sent to the model per inference call
const FRAME_MS = 1000 / 30; // pacing clock the decoded frames are drawn out at

function nextPowerOfTwo(n: number): number {
  let p = 32;
  while (p < n && p < 32768) p *= 2;
  return p;
}

export interface Wav2ArkitLipsyncOptions extends Wav2ArkitEngineOptions {
  onFrame: (weights: Record<string, number>) => void;
}

/**
 * Runs the shared wav2arkit ONNX engine on rolling ~200ms audio windows and
 * paces the decoded ARKit blendshape frames out at ~30fps via `onFrame`.
 * Two input modes, both feeding the same inference/pacing pipeline:
 *  - `start(stream)` taps a `MediaStream` via an AnalyserNode (OpenAI, Vapi,
 *    LiveKit).
 *  - `pushAudio(samples, sampleRate)` accepts already-decoded PCM directly,
 *    for adapters that never produce a MediaStream (Deepgram).
 * One instance per active session — safe to run several concurrently (e.g.
 * multiple avatars on one page), since only the underlying model session is
 * shared.
 */
export class Wav2ArkitLipsync {
  private audioCtx: AudioContext | null = null;
  private sourceNode: MediaStreamAudioSourceNode | null = null;
  private analyserNode: AnalyserNode | null = null;
  private readonly engine: ReturnType<typeof getSharedWav2ArkitEngine>;

  private chunkSamples = 0;
  private timeDomainBuffer: Float32Array = new Float32Array(0);
  private frameQueue: Record<string, number>[] = [];
  private lastFrame: Record<string, number> | null = null;
  private captureTimer: ReturnType<typeof setInterval> | null = null;
  private paceTimer: ReturnType<typeof setInterval> | null = null;
  private inferInFlight = false;
  private warmedUp = false;

  // ── Push-mode state (see `pushAudio`) ──────────────────────────────────
  private pushBuffer: number[] = [];
  private pushSampleRate = 0;
  private pushChunkSamples = 0;

  constructor(private readonly options: Wav2ArkitLipsyncOptions) {
    this.engine = getSharedWav2ArkitEngine(options);
  }

  private ensureWarmup(): void {
    if (this.warmedUp) return;
    this.warmedUp = true;
    // Fire-and-forget warmup so the model is fetched/initialized as early
    // as possible instead of stalling the first real audio chunk.
    this.engine.warmup().catch(() => {
      // First real inference call will surface (and can retry) the failure.
    });
  }

  private ensurePaceTimer(): void {
    if (this.paceTimer) return;
    this.paceTimer = setInterval(() => this.tick(), FRAME_MS);
  }

  /** Taps a `MediaStream` via an AnalyserNode. */
  public start(stream: MediaStream): void {
    if (this.audioCtx) return; // already running

    const AudioContextCtor =
      window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
    this.audioCtx = new AudioContextCtor({ latencyHint: 'interactive' });
    if (this.audioCtx.state === 'suspended') this.audioCtx.resume();

    this.chunkSamples = Math.round((CHUNK_MS / 1000) * this.audioCtx.sampleRate);
    const fftSize = nextPowerOfTwo(this.chunkSamples);
    this.timeDomainBuffer = new Float32Array(fftSize);

    this.sourceNode = this.audioCtx.createMediaStreamSource(stream);
    this.analyserNode = this.audioCtx.createAnalyser();
    this.analyserNode.fftSize = fftSize;
    this.sourceNode.connect(this.analyserNode);

    this.captureTimer = setInterval(() => this.captureChunk(), CHUNK_MS);
    this.ensurePaceTimer();
    this.ensureWarmup();
  }

  private captureChunk(): void {
    if (!this.analyserNode || this.inferInFlight) return;
    this.analyserNode.getFloatTimeDomainData(this.timeDomainBuffer as unknown as Float32Array<ArrayBuffer>);
    const chunk = this.timeDomainBuffer.slice(this.timeDomainBuffer.length - this.chunkSamples);
    void this.runInference(chunk, this.audioCtx?.sampleRate ?? 48000);
  }

  /**
   * Feeds already-decoded PCM samples directly into the pipeline, for
   * adapters that decode raw audio themselves and never produce a
   * MediaStream. Safe to call repeatedly with irregularly-sized chunks.
   */
  public pushAudio(samples: Float32Array, sampleRate: number): void {
    if (this.pushSampleRate !== sampleRate) {
      this.pushSampleRate = sampleRate;
      this.pushChunkSamples = Math.max(1, Math.round((CHUNK_MS / 1000) * sampleRate));
      this.pushBuffer = [];
    }
    for (let i = 0; i < samples.length; i++) this.pushBuffer.push(samples[i]);

    while (this.pushBuffer.length >= this.pushChunkSamples && !this.inferInFlight) {
      const chunk = new Float32Array(this.pushBuffer.splice(0, this.pushChunkSamples));
      void this.runInference(chunk, sampleRate);
    }

    this.ensurePaceTimer();
    this.ensureWarmup();
  }

  private async runInference(chunk: Float32Array, sampleRate: number): Promise<void> {
    this.inferInFlight = true;
    try {
      const resampled = await resampleTo16k(chunk, sampleRate);
      const frames = await this.engine.run(resampled);
      for (const frame of frames) {
        const weights: Record<string, number> = {};
        for (let i = 0; i < ARKIT_BLENDSHAPE_NAMES.length; i++) {
          weights[ARKIT_BLENDSHAPE_NAMES[i]] = frame[i] ?? 0;
        }
        this.frameQueue.push(weights);
      }
    } catch {
      // Drop this window on inference failure; the pacing clock holds the
      // last good frame and the next chunk will retry.
    } finally {
      this.inferInFlight = false;
    }
  }

  private tick(): void {
    const next = this.frameQueue.shift();
    if (next) {
      this.lastFrame = next;
      this.options.onFrame(next);
    } else if (this.lastFrame) {
      this.options.onFrame(this.lastFrame);
    }
  }

  public stop(): void {
    if (this.captureTimer) clearInterval(this.captureTimer);
    if (this.paceTimer) clearInterval(this.paceTimer);
    this.captureTimer = null;
    this.paceTimer = null;

    this.sourceNode?.disconnect();
    this.analyserNode?.disconnect();
    if (this.audioCtx && this.audioCtx.state !== 'closed') {
      this.audioCtx.close();
    }
    this.audioCtx = null;
    this.sourceNode = null;
    this.analyserNode = null;
    this.frameQueue = [];
    this.lastFrame = null;
    this.pushBuffer = [];
    this.pushSampleRate = 0;
  }
}


```

## Source file: agentic-avatars/src/audio/useVolumeFallbackLipsync.ts

{
  "path": "agentic-avatars/src/audio/useVolumeFallbackLipsync.ts",
  "blob_sha": "a1bfb27c24739b70c2d72142535e5cb003c6c8f3",
  "sha256": "9a8f0f1366636a3bfdd09a152c68a5ae6e37b19f38ce0ec9157778a964450544",
  "lines": 42,
  "bytes": 1691,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 1,
    "jaw": 1,
    "mouth": 2,
    "face": 1,
    "lip": 3
  },
  "url": "https://github.com/NavodPeiris/agentic-avatars/blob/fbc1bd4a270516f00f535fc9ebeaf8429e1ba57c/agentic-avatars/src/audio/useVolumeFallbackLipsync.ts",
  "function_signature_candidates": []
}

```text

import { useEffect } from 'react';
import type { RefObject } from 'react';
import type { IAvatarController } from '../avatar/GaussianAvatarController';
import { createNeutralWeights } from '../constants/arkit';

const SPEAKING_THRESHOLD = 0.04;

interface UseVolumeFallbackLipsyncOptions {
  /** Present only for adapters that can't expose a MediaStream (e.g. ElevenLabs). */
  getRemoteAudioLevel?: () => number;
  /** Whether this fallback path should be running (e.g. connected and no remoteStream). */
  active: boolean;
  controllerRef: RefObject<IAvatarController | null>;
}

/**
 * Coarse, volume-only mouth animation for adapters whose SDK exposes only a
 * scalar output level (no raw audio) — currently just ElevenLabs. Other
 * providers use `useAvatarLipsync` (full wav2arkit neural lipsync) instead;
 * this is a deliberately lower-fidelity fallback, not a replacement.
 */
export function useVolumeFallbackLipsync({ getRemoteAudioLevel, active, controllerRef }: UseVolumeFallbackLipsyncOptions) {
  useEffect(() => {
    if (!getRemoteAudioLevel || !active) return;

    const neutral = createNeutralWeights();
    let frameId: number;

    const tick = () => {
      const level = Math.max(0, Math.min(1, getRemoteAudioLevel()));
      controllerRef.current?.setChatState(level > SPEAKING_THRESHOLD ? 'Responding' : 'Idle');
      controllerRef.current?.updateBlendshapes({ ...neutral, jawOpen: Math.min(0.6, level * 0.9), mouthClose: 0 });
      frameId = requestAnimationFrame(tick);
    };
    tick();

    return () => {
      cancelAnimationFrame(frameId);
      controllerRef.current?.setChatState('Idle');
    };
  }, [getRemoteAudioLevel, active, controllerRef]);
}


```
