# Facial rendering source scan: waleedsworld/ai-model-scultpor

Repository: https://github.com/waleedsworld/ai-model-scultpor

Pinned source revision: abf77ed5e11fdf4c406af51eb5aaad747242b3a6

Discovery topic: character creator 3D

Description: AI 3D Model Creator — turn a text prompt into a rotatable 3D character. Vite + React + TypeScript + Supabase.

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"rig": ["playwright.config.ts"]}

## Source file: playwright.config.ts

{
  "path": "playwright.config.ts",
  "blob_sha": "25b340f467190e7549a45b265713e0738312171c",
  "sha256": "6caa901ede1921578c4b8a92821f3f630c7079ac6186dd8b985ff12b060b8d9b",
  "lines": 32,
  "bytes": 936,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 2
  },
  "url": "https://github.com/waleedsworld/ai-model-scultpor/blob/abf77ed5e11fdf4c406af51eb5aaad747242b3a6/playwright.config.ts",
  "function_signature_candidates": []
}

```text

import { defineConfig, devices } from "@playwright/test";

// End-to-end smoke tests. Playwright builds the app once and serves the
// production bundle via `vite preview`, then drives it in a headless
// Chromium. The port is overridable so CI and local runs can differ.
const PORT = Number(process.env.E2E_PORT ?? 4173);
const HOST = "127.0.0.1";
const baseURL = `http://${HOST}:${PORT}`;

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? "line" : "list",
  use: {
    baseURL,
    trace: "on-first-retry",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
  webServer: {
    command: `npm run build && npm run preview -- --host ${HOST} --port ${PORT} --strictPort`,
    url: baseURL,
    reuseExistingServer: !process.env.CI,
    timeout: 180_000,
  },
});


```

## Source file: vite.config.ts

{
  "path": "vite.config.ts",
  "blob_sha": "7f0707e50306b5ccc50d51d56ffda8555d6ed49b",
  "sha256": "43e827cd1a83f23551e9c89f7333b792dd2035fcc0eb7dd6ebe39d93185206c8",
  "lines": 44,
  "bytes": 1321,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/waleedsworld/ai-model-scultpor/blob/abf77ed5e11fdf4c406af51eb5aaad747242b3a6/vite.config.ts",
  "function_signature_candidates": []
}

```text

import { defineConfig } from "vite";
import react from "@vitejs/plugin-react-swc";
import path from "path";

// https://vitejs.dev/config/
export default defineConfig(() => ({
  server: {
    host: "::",
    port: 8080,
  },
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  build: {
    // Raise the warning threshold now that vendor code is split into
    // dedicated, cacheable chunks below.
    chunkSizeWarningLimit: 700,
    rollupOptions: {
      output: {
        // Split large, rarely-changing dependencies into their own chunks so
        // the browser can cache them independently of the app code and the
        // initial route only pays for what it needs.
        manualChunks: {
          "react-vendor": ["react", "react-dom", "react-router-dom"],
          "radix-vendor": [
            "@radix-ui/react-dialog",
            "@radix-ui/react-dropdown-menu",
            "@radix-ui/react-tooltip",
            "@radix-ui/react-toast",
            "@radix-ui/react-select",
            "@radix-ui/react-tabs",
            "@radix-ui/react-slider",
            "@radix-ui/react-popover",
          ],
          "supabase-vendor": ["@supabase/supabase-js"],
          "query-vendor": ["@tanstack/react-query"],
        },
      },
    },
  },
}));


```

## Source file: eslint.config.js

{
  "path": "eslint.config.js",
  "blob_sha": "e67846f70fbbe40407fc84875913595ab31c4a47",
  "sha256": "5a016705c1d9758fd11687906bc6426983abc607d1462fd7915d5bbc5cec099f",
  "lines": 29,
  "bytes": 789,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/waleedsworld/ai-model-scultpor/blob/abf77ed5e11fdf4c406af51eb5aaad747242b3a6/eslint.config.js",
  "function_signature_candidates": []
}

```text

import js from "@eslint/js";
import globals from "globals";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";
import tseslint from "typescript-eslint";

export default tseslint.config(
  { ignores: ["dist"] },
  {
    extends: [js.configs.recommended, ...tseslint.configs.recommended],
    files: ["**/*.{ts,tsx}"],
    languageOptions: {
      ecmaVersion: 2020,
      globals: globals.browser,
    },
    plugins: {
      "react-hooks": reactHooks,
      "react-refresh": reactRefresh,
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      "react-refresh/only-export-components": [
        "warn",
        { allowConstantExport: true },
      ],
      "@typescript-eslint/no-unused-vars": "off",
    },
  }
);


```

## Source file: src/lib/utils.ts

{
  "path": "src/lib/utils.ts",
  "blob_sha": "bd0c391ddd1088e9067844c48835bf4abcd61783",
  "sha256": "7c8c3dfc0cdd370d44932828eb067ef771c8fe7996693221d5d4b90af6d54f2d",
  "lines": 6,
  "bytes": 166,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/waleedsworld/ai-model-scultpor/blob/abf77ed5e11fdf4c406af51eb5aaad747242b3a6/src/lib/utils.ts",
  "function_signature_candidates": []
}

```text

import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}


```
