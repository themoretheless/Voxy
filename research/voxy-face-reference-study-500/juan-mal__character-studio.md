# Facial rendering source scan: juan-mal/character-studio

Repository: https://github.com/juan-mal/character-studio

Pinned source revision: 64b60ea9b28294d4068f312b00d53d3bb03b7e81

Discovery topic: character creator 3D

Description: A local, data-driven 3D character creator built with React, TypeScript and Three.js.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["src/components/Notice.tsx"]

Topic paths: {"skin": ["src/components/SkinControls.tsx"], "rig": ["playwright.config.ts", "src/generated/meshes/girl-standard-body002-original-head.obj", "src/generated/meshes/girl-standard-body002-original-head.provenance.json"], "face": ["tools/inspection/face-evidence.ts"]}

## Source file: playwright.config.ts

{
  "path": "playwright.config.ts",
  "blob_sha": "9e2e0c97641149a502383058f5e59f05a5595355",
  "sha256": "f031555d121288fa1e3dfdf83da1cf7ab5094ab094d1e1a5ebefa88e7fbc6f3c",
  "lines": 18,
  "bytes": 416,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 1
  },
  "url": "https://github.com/juan-mal/character-studio/blob/64b60ea9b28294d4068f312b00d53d3bb03b7e81/playwright.config.ts",
  "function_signature_candidates": []
}

```text

import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests/e2e",
  timeout: 60_000,
  workers: 1,
  use: {
    baseURL: "http://127.0.0.1:5173",
    viewport: { width: 1440, height: 900 },
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
  },
  webServer: {
    command: "pnpm dev",
    url: "http://127.0.0.1:5173",
    reuseExistingServer: true,
  },
});


```

## Source file: tools/inspection/face-evidence.ts

{
  "path": "tools/inspection/face-evidence.ts",
  "blob_sha": "816cf990a395baa3759002b86e10dd1164034f70",
  "sha256": "e8d961467733e78351e24e01acfadc5625337f8889d5efbf3f78d48d0916b5f7",
  "lines": 37,
  "bytes": 3703,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 8
  },
  "url": "https://github.com/juan-mal/character-studio/blob/64b60ea9b28294d4068f312b00d53d3bb03b7e81/tools/inspection/face-evidence.ts",
  "function_signature_candidates": []
}

```text

import fs from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import { parseObj, pointKey, getBounds, digest } from '../assets/obj.ts';
import { surfaceCoverage } from '../assets/compatibility.ts';
import type { ParsedObj } from '../assets/obj.ts';
import type { AssetCatalog, InspectionReport, MeshRecord } from '../../src/assets/types.ts';
const catalog = JSON.parse(await fs.readFile('src/generated/assets.generated.json', 'utf8')) as AssetCatalog;
const report = JSON.parse(await fs.readFile('reports/asset-inspection.json', 'utf8')) as InspectionReport;
const cache = new Map<string, ParsedObj>();
async function load(m: MeshRecord) { let p = cache.get(m.id); if (!p) { const src = catalog.sources.find(s=>s.id===m.sourceId)!; p=parseObj(await fs.readFile(path.resolve(src.path.replace(/^~/,os.homedir()),m.relativePath),'utf8'));cache.set(m.id,p); } return p; }
function tris(m: ParsedObj) { return m.polygons.flatMap(f=> f.slice(1,-1).map((_,i)=>[f[0]!,f[i+1]!,f[i+2]!])); }
function key(m: ParsedObj,f:number[]) { return f.map(i=>pointKey(m.positions[i]!,.0001)).sort().join('|'); }
const targetMeshes = report.meshes.filter(m=>m.category==='merged characters' && m.lod===0 && /Girl_Standard_Body002/.test(m.name));
const faces = catalog.assets.filter(a=> a.category==='face');
const output = [];
for (const m of targetMeshes) {
  const merged = await load(m), bodyAsset = catalog.assets.find(a=>a.name==='Body002_Standard')!, hairAsset=catalog.assets.find(a=>a.name===m.name.match(/Hair\d+/)![0]+'_Standard')!;
  const body=await load(bodyAsset.mesh), hair=await load(hairAsset.mesh);
  const known = new Set([...tris(body).map(f=>key(body,f)), ...tris(hair).map(f=>key(hair,f))]);
  const residualPolygons=tris(merged).filter(f=>!known.has(key(merged,f))), indices=[...new Set(residualPolygons.flat())];
  const residual={...merged,positions:merged.positions,polygons:residualPolygons};
  const comparisons=[];
  for(const f of faces) { const obj=await load(f.mesh); comparisons.push({name:f.name,id:f.id,faceTriangles:obj.stats.triangleCount,bounds:obj.stats.bounds,...surfaceCoverage(obj,merged),residualTriangleCoverage:surfaceCoverage(obj,residual).triangleCoverage}); }
  comparisons.sort((a,b)=>b.triangleCoverage-a.triangleCoverage);
  const source=catalog.sources.find(s=>s.id===m.sourceId)!;
  const raw=await fs.readFile(path.resolve(source.path.replace(/^~/,os.homedir()),m.relativePath),'utf8');
  const uv=raw.split(/\r?\n/).filter(l=>l.startsWith('vt ')).map(l=>l.slice(3).split(/\s+/).map(Number));
  const normal=raw.split(/\r?\n/).filter(l=>l.startsWith('vn ')).map(l=>l.slice(3).split(/\s+/).map(Number));
  const rawFaces=raw.split(/\r?\n/).filter(l=>l.startsWith('f ')).map(l=>l.slice(2).split(/\s+/).map(c=>c.split('/').map(v=>Number(v)-1)));
  const residualRaw=rawFaces.filter(f=>!known.has(key(merged,f.map(c=>c[0]!))));
  const layout=residualRaw.map(f=>f.map(c=>[...merged.positions[c[0]!]!,...uv[c[1]!]!,...normal[c[2]!]!].map(v=>Math.round(v*1e6)).join(',')).sort().join('|')).sort();
  const row={name:m.name,mergedTriangles:merged.stats.triangleCount,body:surfaceCoverage(body,merged),hair:surfaceCoverage(hair,merged),residualTriangles:residualPolygons.length,residualLayoutHash:digest(layout.join(';')),residualUVBounds:getBounds(residualRaw.flat().map(c=>[uv[c[1]!]![0]!,uv[c[1]!]![1]!,0])),residualBounds:getBounds(indices.map(i=>merged.positions[i]!)),faces:comparisons}; output.push(row);
}
await fs.mkdir('reports/inspection',{recursive:true});
await fs.writeFile('reports/inspection/face-evidence.json',JSON.stringify(output,null,2)+'\n');
console.log(JSON.stringify(output.map(r=>({...r,faces:r.faces.slice(0,6)})),null,2));


```

## Source file: vite.config.ts

{
  "path": "vite.config.ts",
  "blob_sha": "b4196815fb038f0ef441e3fae9885dea4926fb18",
  "sha256": "c1fe9c7c51f7bdba6cf78d82d030a4f58185e6572fcf7bcc220ca47d89c89e51",
  "lines": 27,
  "bytes": 834,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/juan-mal/character-studio/blob/64b60ea9b28294d4068f312b00d53d3bb03b7e81/vite.config.ts",
  "function_signature_candidates": []
}

```text

import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { studioAssetsPlugin } from "./tools/studio/asset-server.ts";

export default defineConfig({
  plugins: [react(), studioAssetsPlugin()],
  server: {
    host: "127.0.0.1",
    port: 5173,
    strictPort: true,
    watch: { ignored: ["**/.tools/**", "**/reports/**"], awaitWriteFinish: { stabilityThreshold: 150, pollInterval: 25 } },
  },
  preview: { host: "127.0.0.1", port: 4173, strictPort: true },
  build: {
    target: "es2022",
    rolldownOptions: {
      output: {
        manualChunks(id: string) {
          if (id.includes("/three/") || id.includes("\\three\\"))
            return "three";
          if (/[/\\](react|react-dom|scheduler)[/\\]/.test(id)) return "react";
          return undefined;
        },
      },
    },
  },
});


```

## Source file: eslint.config.js

{
  "path": "eslint.config.js",
  "blob_sha": "20bfe5941cd0e90f9b569e95b936429c2545a109",
  "sha256": "8aed0032e5eaf0c8c577262098cda0caed36db3745e5ef941315fdd2f3c91949",
  "lines": 22,
  "bytes": 781,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/juan-mal/character-studio/blob/64b60ea9b28294d4068f312b00d53d3bb03b7e81/eslint.config.js",
  "function_signature_candidates": []
}

```text

import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import reactHooks from 'eslint-plugin-react-hooks';

export default tseslint.config(
  { ignores: ['src/generated/**', 'dist/**', 'node_modules/**', 'reports/**'] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ['**/*.ts', '**/*.tsx'],
    rules: {
      '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_', varsIgnorePattern: '^_' }],
      '@typescript-eslint/no-explicit-any': 'error',
      '@typescript-eslint/no-non-null-assertion': 'off',
    },
  },
  {
    files: ['src/**/*.tsx', 'src/hooks/**/*.ts'],
    plugins: { 'react-hooks': reactHooks },
    rules: { 'react-hooks/rules-of-hooks': 'error', 'react-hooks/exhaustive-deps': 'error' },
  },
);


```
