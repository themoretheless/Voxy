# Facial rendering source scan: badhope/ChatVRM-Agent

Repository: https://github.com/badhope/ChatVRM-Agent

Pinned source revision: ebf9f7f329d474c07e255cfa4979803efe9c6124

Discovery topic: lip sync 3D

Description: 3D digital-human conversation framework — VRM avatars, 13 LLM providers, 5 TTS engines, voice input, lip sync, and a glassmorphism UI. Built with Next.js 13 + three.js.

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"rig": [".dockerignore", ".prettierignore"], "lip": ["src/features/lipSync/lipSync.ts", "src/features/lipSync/lipSyncAnalyzeResult.ts"]}

## Source file: src/features/lipSync/lipSync.ts

{
  "path": "src/features/lipSync/lipSync.ts",
  "blob_sha": "8b7c2dad5c0af30a3362122f81953eea1b2fcf56",
  "sha256": "13847c74d52d654adb88ff096c07d0ac8efcff9523a44a9b8703e9aeb60dbdc7",
  "lines": 53,
  "bytes": 1480,
  "selection_score": 15,
  "keyword_line_counts": {
    "lip": 3
  },
  "url": "https://github.com/badhope/ChatVRM-Agent/blob/ebf9f7f329d474c07e255cfa4979803efe9c6124/src/features/lipSync/lipSync.ts",
  "function_signature_candidates": []
}

```text

import { LipSyncAnalyzeResult } from "./lipSyncAnalyzeResult";

const TIME_DOMAIN_DATA_LENGTH = 2048;

export class LipSync {
  public readonly audio: AudioContext;
  public readonly analyser: AnalyserNode;
  public readonly timeDomainData: Float32Array;

  public constructor(audio: AudioContext) {
    this.audio = audio;

    this.analyser = audio.createAnalyser();
    this.timeDomainData = new Float32Array(TIME_DOMAIN_DATA_LENGTH);
  }

  public update(): LipSyncAnalyzeResult {
    this.analyser.getFloatTimeDomainData(this.timeDomainData);

    let volume = 0.0;
    for (let i = 0; i < TIME_DOMAIN_DATA_LENGTH; i++) {
      volume = Math.max(volume, Math.abs(this.timeDomainData[i]));
    }

    // cook
    volume = 1 / (1 + Math.exp(-45 * volume + 5));
    if (volume < 0.1) volume = 0;

    return {
      volume,
    };
  }

  public async playFromArrayBuffer(buffer: ArrayBuffer, onEnded?: () => void) {
    const audioBuffer = await this.audio.decodeAudioData(buffer);

    const bufferSource = this.audio.createBufferSource();
    bufferSource.buffer = audioBuffer;

    bufferSource.connect(this.audio.destination);
    bufferSource.connect(this.analyser);
    bufferSource.start();
    if (onEnded) {
      bufferSource.addEventListener("ended", onEnded);
    }
  }

  public async playFromURL(url: string, onEnded?: () => void) {
    const res = await fetch(url);
    const buffer = await res.arrayBuffer();
    this.playFromArrayBuffer(buffer, onEnded);
  }
}


```

## Source file: src/features/lipSync/lipSyncAnalyzeResult.ts

{
  "path": "src/features/lipSync/lipSyncAnalyzeResult.ts",
  "blob_sha": "79781060b0ba7f6ec2bf0b68d59d81658f380392",
  "sha256": "ca60f81fcd65900cdb891c8590ec3520b45f4395c82c13315bb24f509f9e696e",
  "lines": 3,
  "bytes": 60,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 1,
    "lip": 1
  },
  "url": "https://github.com/badhope/ChatVRM-Agent/blob/ebf9f7f329d474c07e255cfa4979803efe9c6124/src/features/lipSync/lipSyncAnalyzeResult.ts",
  "function_signature_candidates": []
}

```text

export interface LipSyncAnalyzeResult {
  volume: number;
}


```

## Source file: next.config.js

{
  "path": "next.config.js",
  "blob_sha": "592641d00f061860dd919959f2c7ceef6d46a585",
  "sha256": "42b76727d0d53ee669a67ee98c80cba600069b9acec6bac8e42a92b26fa5c491",
  "lines": 13,
  "bytes": 321,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/badhope/ChatVRM-Agent/blob/ebf9f7f329d474c07e255cfa4979803efe9c6124/next.config.js",
  "function_signature_candidates": []
}

```text

/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  assetPrefix: process.env.BASE_PATH || "",
  basePath: process.env.BASE_PATH || "",
  trailingSlash: true,
  publicRuntimeConfig: {
    root: process.env.BASE_PATH || "",
  },
  optimizeFonts: false,
};

module.exports = nextConfig;


```

## Source file: postcss.config.js

{
  "path": "postcss.config.js",
  "blob_sha": "33ad091d26d8a9dc95ebdf616e217d985ec215b8",
  "sha256": "251ecddd4672c9cf467547e3dc535de00ad2129df26e7e7ae728fa4e5ac45fc5",
  "lines": 6,
  "bytes": 82,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/badhope/ChatVRM-Agent/blob/ebf9f7f329d474c07e255cfa4979803efe9c6124/postcss.config.js",
  "function_signature_candidates": []
}

```text

module.exports = {
  plugins: {
    tailwindcss: {},
    autoprefixer: {},
  },
}


```
