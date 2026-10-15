# Facial rendering source scan: alisaboorigemeni-blip/LipSync-Pro

Repository: https://github.com/alisaboorigemeni-blip/LipSync-Pro

Pinned source revision: 6a7ab94167228eaf2804dc4d9fbb8ea3fdd9fd28

Discovery topic: lip sync 3D

Description: Automated AI-driven lip sync generator for 3D characters using shape keys.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: types.ts

{
  "path": "types.ts",
  "blob_sha": "efac16cb3489d772a0b8d1e6564dae5c118931ec",
  "sha256": "61b43a2d5448fc80b8a3df98cfca6d1799eca1bfb185b479d36fa6ae4219bd25",
  "lines": 34,
  "bytes": 842,
  "selection_score": 0,
  "keyword_line_counts": {
    "mouth": 1,
    "teeth": 3,
    "tongue": 2,
    "face": 3,
    "lip": 2
  },
  "url": "https://github.com/alisaboorigemeni-blip/LipSync-Pro/blob/6a7ab94167228eaf2804dc4d9fbb8ea3fdd9fd28/types.ts",
  "function_signature_candidates": []
}

```text

export enum Viseme {
  SIL = 'sil', // Silence/Neutral
  PP = 'pp',   // P, B, M (Lips closed)
  FF = 'ff',   // F, V (Teeth on lip)
  TH = 'th',   // TH (Tongue between teeth)
  DD = 'dd',   // T, D, S, Z (Tongue behind teeth)
  KK = 'kk',   // K, G, NG
  CH = 'ch',   // CH, J, SH
  SS = 'ss',   // S, Z
  NN = 'nn',   // N
  RR = 'rr',   // R
  AA = 'aa',   // Wide open (Ah)
  E = 'e',     // Eh
  I = 'i',     // Ih, Ee
  O = 'o',     // Oh
  U = 'u'      // Oo, W
}

export interface VisemeEvent {
  viseme: Viseme;
  start: number;   // seconds
  end: number;     // seconds
  strength: number; // 0.0 to 1.0
}

export interface ShapeKeyMapping {
  // Maps a Viseme ID (e.g., 'aa') to a Mesh Shape Key Name (e.g., 'mouth_open')
  [viseme: string]: string | null;
}

export interface MeshInfo {
  name: string;
  shapeKeys: string[];
}


```

## Source file: constants.ts

{
  "path": "constants.ts",
  "blob_sha": "1509c31e36a46c8ee51be2e4c94f44eb806b467d",
  "sha256": "b5006693eb79500d1b69ff1753b93dc33406b1778724248c9697c9134166e14e",
  "lines": 94,
  "bytes": 2254,
  "selection_score": 0,
  "keyword_line_counts": {
    "jaw": 2,
    "mouth": 6,
    "teeth": 4,
    "tongue": 4,
    "face": 1,
    "lip": 9
  },
  "url": "https://github.com/alisaboorigemeni-blip/LipSync-Pro/blob/6a7ab94167228eaf2804dc4d9fbb8ea3fdd9fd28/constants.ts",
  "function_signature_candidates": []
}

```text

import { Viseme } from './types';

export const VISEMES_LIST = [
  { 
    id: Viseme.SIL, 
    label: 'Neutral / Silence', 
    description: 'Resting face',
    aliases: ['basis', 'neutral', 'rest', 'idle', 'sil', 'silence']
  },
  { 
    id: Viseme.PP, 
    label: 'M / B / P', 
    description: 'Lips pressed together',
    aliases: ['mouthClose', 'm_b_p', 'viseme_pp', 'pp', 'mbp', 'lipsClosed']
  },
  { 
    id: Viseme.FF, 
    label: 'F / V', 
    description: 'Upper teeth touch lower lip',
    aliases: ['f_v', 'viseme_ff', 'ff', 'lipBite', 'fv']
  },
  { 
    id: Viseme.TH, 
    label: 'TH', 
    description: 'Tongue tip between teeth',
    aliases: ['viseme_th', 'th', 'tongueOut']
  },
  { 
    id: Viseme.DD, 
    label: 'T / D', 
    description: 'Tongue tip behind upper teeth',
    aliases: ['viseme_dd', 'dd', 'td', 't_d']
  },
  { 
    id: Viseme.KK, 
    label: 'K / G', 
    description: 'Back of tongue raises',
    aliases: ['viseme_kk', 'kk', 'kg', 'k_g']
  },
  { 
    id: Viseme.CH, 
    label: 'CH / SH / J', 
    description: 'Lips flared forward',
    aliases: ['viseme_ch', 'ch', 'sh', 'j', 'ch_j_sh']
  },
  { 
    id: Viseme.SS, 
    label: 'S / Z', 
    description: 'Teeth together, lips slightly open',
    aliases: ['viseme_ss', 'ss', 'sz', 's_z']
  },
  { 
    id: Viseme.NN, 
    label: 'N', 
    description: 'Nose sound',
    aliases: ['viseme_nn', 'nn']
  },
  { 
    id: Viseme.RR, 
    label: 'R', 
    description: 'Lips rounded slightly',
    aliases: ['viseme_rr', 'rr']
  },
  { 
    id: Viseme.AA, 
    label: 'AA / AH', 
    description: 'Jaw dropped open',
    aliases: ['jawOpen', 'mouthOpen', 'viseme_aa', 'aa', 'ah', 'a_ah']
  },
  { 
    id: Viseme.E, 
    label: 'EH / AE', 
    description: 'Medium open mouth',
    aliases: ['viseme_e', 'e', 'eh', 'ae']
  },
  { 
    id: Viseme.I, 
    label: 'IH / EE', 
    description: 'Wide mouth, narrow opening',
    aliases: ['viseme_i', 'i', 'ih', 'ee']
  },
  { 
    id: Viseme.O, 
    label: 'OH', 
    description: 'Lips rounded open',
    aliases: ['viseme_o', 'o', 'oh', 'mouthRound']
  },
  { 
    id: Viseme.U, 
    label: 'UW / W', 
    description: 'Lips puckered small',
    aliases: ['viseme_u', 'u', 'uw', 'w', 'pucker', 'mouthPucker']
  },
];

```

## Source file: vite.config.ts

{
  "path": "vite.config.ts",
  "blob_sha": "ee5fb8d2dfb261530fb407cb2f28599d50fcae26",
  "sha256": "63e5a78aeea5894837a62ad0b90869c602bad713f93ae44e61c436a7b7689a19",
  "lines": 23,
  "bytes": 580,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/alisaboorigemeni-blip/LipSync-Pro/blob/6a7ab94167228eaf2804dc4d9fbb8ea3fdd9fd28/vite.config.ts",
  "function_signature_candidates": []
}

```text

import path from 'path';
import { defineConfig, loadEnv } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig(({ mode }) => {
    const env = loadEnv(mode, '.', '');
    return {
      server: {
        port: 3000,
        host: '0.0.0.0',
      },
      plugins: [react()],
      define: {
        'process.env.API_KEY': JSON.stringify(env.GEMINI_API_KEY),
        'process.env.GEMINI_API_KEY': JSON.stringify(env.GEMINI_API_KEY)
      },
      resolve: {
        alias: {
          '@': path.resolve(__dirname, '.'),
        }
      }
    };
});


```

## Source file: workers/jsonWorker.ts

{
  "path": "workers/jsonWorker.ts",
  "blob_sha": "d6ef6b71dfb30510f5cb05726d1b2de3904e93cf",
  "sha256": "c5c2a3bf6920040a5552bedff1036952dcd9a97ca0ccc98977cd1da988fbeb56",
  "lines": 36,
  "bytes": 1392,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/alisaboorigemeni-blip/LipSync-Pro/blob/6a7ab94167228eaf2804dc4d9fbb8ea3fdd9fd28/workers/jsonWorker.ts",
  "function_signature_candidates": []
}

```text

self.onmessage = (e: MessageEvent) => {
  const { type, payload } = e.data;
  
  if (type === 'PARSE_VISEMES') {
    try {
      const { jsonString } = payload;
      const data = JSON.parse(jsonString);
      
      const sanitizeVisemes = (rawVisemes: any[]) => {
          return rawVisemes.map((v: any, i: number) => {
              const start = v.start ?? v.time ?? 0;
              let end = v.end;
              if (end === undefined) {
                  end = (i < rawVisemes.length - 1) 
                      ? (rawVisemes[i+1].start ?? rawVisemes[i+1].time ?? start + 0.1)
                      : start + 0.1;
              }
              return { start, end, viseme: v.viseme || v.value || 'X', strength: v.strength ?? 1.0 };
          });
      };

      let resultVisemes = [];
      if (Array.isArray(data) && data.length > 0 && (data[0].viseme || data[0].value)) {
          resultVisemes = sanitizeVisemes(data);
      } else if (data.visemes && Array.isArray(data.visemes)) {
          resultVisemes = sanitizeVisemes(data.visemes);
      } else {
          throw new Error("Invalid visemes format. Must contain an array of visemes.");
      }

      self.postMessage({ type: 'PARSE_VISEMES_SUCCESS', payload: resultVisemes });
    } catch (err: any) {
      self.postMessage({ type: 'PARSE_VISEMES_ERROR', error: err.message || "Error parsing JSON file" });
    }
  }
};


```
