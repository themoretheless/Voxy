# Facial rendering source scan: Spencereese/phaser-idle-quest

Repository: https://github.com/Spencereese/phaser-idle-quest

Pinned source revision: 925318728985fec1b419f2ae1c54064ac8a9a172

Discovery topic: tongue animation

Description: Phaser-based idle game featuring a whimsical watercolor frog hero with tongue attack animations.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"tongue": ["public/assets/ui/items/Barbed Tongue.png", "public/assets/ui/items/Long-tongue.png", "public/assets/ui/items/Thick-tongue.png", "public/assets/ui/items/Tongue-scarf.png"]}

## Source file: src/main.ts

{
  "path": "src/main.ts",
  "blob_sha": "e228aedaeacfe806f111fc53a736f2fad80a93ec",
  "sha256": "ca8739b7128b35ee9cfa09892ff681fe3b48279723ab0542cf5bcfade6ac4247",
  "lines": 33,
  "bytes": 1354,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/Spencereese/phaser-idle-quest/blob/925318728985fec1b419f2ae1c54064ac8a9a172/src/main.ts",
  "function_signature_candidates": []
}

```text

import Phaser from 'phaser';

// Import the clean scene structure (Boot -> Preload -> Game) as specified in the plan.
// This gives a proper Phaser 3 lifecycle for an expandable idle/incremental game.
import { BootScene } from './scenes/BootScene';
import { PreloadScene } from './scenes/PreloadScene';
import { GameScene } from './scenes/GameScene';

/**
 * Phaser Game configuration.
 * - AUTO: WebGL if available, fallback Canvas (good for broad compatibility in idle tab games).
 * - Parent targets the container in index.html.
 * - Scale FIT + CENTER for responsive play (important for different window sizes when leaving tab open).
 * - No physics by default (we'll use manual timers/tweens for ally attacks and mob lifetime; keeps it lightweight for idle).
 *
 * All scenes are listed so Phaser knows the order and can transition between them.
 * See individual scene files for heavy comments on their responsibilities.
 */
const config: Phaser.Types.Core.GameConfig = {
  type: Phaser.AUTO,
  width: 800,
  height: 600,
  parent: 'game-container',
  backgroundColor: '#1a1a2e',
  scene: [BootScene, PreloadScene, GameScene],
  scale: {
    mode: Phaser.Scale.FIT,
    autoCenter: Phaser.Scale.CENTER_BOTH,
  },
  // Future: add physics: { default: 'arcade' } when we want sprite bodies for more advanced mob movement.
};

new Phaser.Game(config);

```

## Source file: vite.config.ts

{
  "path": "vite.config.ts",
  "blob_sha": "025327c5b1918f64473f146efccf95594002f91e",
  "sha256": "a87fb81268b6209bab34acaacae3dc0bdcc58bed7bfa2a0de7b6df6a45f60087",
  "lines": 16,
  "bytes": 526,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/Spencereese/phaser-idle-quest/blob/925318728985fec1b419f2ae1c54064ac8a9a172/vite.config.ts",
  "function_signature_candidates": []
}

```text

import { defineConfig } from 'vite';

// Basic Vite + Phaser 3 + TS configuration.
// Phaser works great out of the box with Vite.
// We use a relative base so the game can be previewed from a subfolder or GitHub Pages if desired.
export default defineConfig({
  base: './',
  server: {
    port: 5173,
    host: true, // allow access from other devices on the same network (e.g. phone or other computers on WiFi)
  },
  build: {
    // Helps with larger Phaser bundles in production.
    chunkSizeWarningLimit: 1000,
  },
});

```

## Source file: src/game/GameState.ts

{
  "path": "src/game/GameState.ts",
  "blob_sha": "f25c201ea5ef6fb4c625eed8bb951c9c654c3110",
  "sha256": "36206f0a5935510a10984fdca1af3397830d3cd985c6c37f511686d3b389e7a4",
  "lines": 134,
  "bytes": 3834,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 1
  },
  "url": "https://github.com/Spencereese/phaser-idle-quest/blob/925318728985fec1b419f2ae1c54064ac8a9a172/src/game/GameState.ts",
  "function_signature_candidates": []
}

```text

﻿/**
 * GameState — single source of truth for persistent idle data.
 * Save key stays phaser-idle-quest-save. New fields migrate safely.
 */
export const SAVE_VERSION = 1;

export interface GameStateData {
  version: number;
  gold: number;
  lifetimeGold: number;
  upgrades: Record<string, number>;
  allies: Record<string, number>;
  prestige: {
    shards: number;
    multiplier: number;
    meta: Record<string, number>;
  };
  lastSaveTime: number;
  /** Lifetime gold earned while offline (informational; migrate-safe). */
  totalOfflineGold: number;
}

export class GameState {
  public data: GameStateData;

  constructor() {
    this.data = this.getDefaultState();
  }

  private getDefaultState(): GameStateData {
    return {
      version: SAVE_VERSION,
      gold: 0,
      lifetimeGold: 0,
      upgrades: {},
      allies: { frog: 1 },
      prestige: {
        shards: 0,
        multiplier: 1,
        meta: {},
      },
      lastSaveTime: Date.now(),
      totalOfflineGold: 0,
    };
  }

  touchSave() {
    this.data.lastSaveTime = Date.now();
  }

  /**
   * Prestige reset: keep shards/multiplier/meta, wipe run progress.
   */
  resetForPrestige(newShards: number, newMultiplier: number) {
    this.data.gold = 0;
    this.data.lifetimeGold = 0;
    this.data.upgrades = {};
    this.data.allies = { frog: 1 };
    this.data.prestige.shards += newShards;
    this.data.prestige.multiplier = newMultiplier;
    this.touchSave();
  }

  save() {
    try {
      this.data.version = SAVE_VERSION;
      localStorage.setItem('phaser-idle-quest-save', JSON.stringify(this.data));
    } catch (e) {
      console.warn('Save failed', e);
    }
  }

  /**
   * Upgrade older saves in place. Never wipes gold/allies/prestige.
   */
  private migrateSave(fromVersion: number) {
    let v = fromVersion;
    if (v < 1) {
      // Pre-version saves: add version + totalOfflineGold defaults already merged.
      v = 1;
    }
    this.data.version = Math.max(v, SAVE_VERSION);
    if (typeof this.data.totalOfflineGold !== 'number' || Number.isNaN(this.data.totalOfflineGold)) {
      this.data.totalOfflineGold = 0;
    }
  }

  load(): boolean {
    try {
      const raw = localStorage.getItem('phaser-idle-quest-save');
      if (!raw) return false;
      const parsed = JSON.parse(raw) as Partial<GameStateData> & { version?: number };
      if (typeof parsed.gold !== 'number') return false;
      const base = this.getDefaultState();
      const fromVersion =
        typeof parsed.version === 'number' && Number.isFinite(parsed.version) ? parsed.version : 0;
      this.data = {
        ...base,
        ...parsed,
        version: fromVersion,
        upgrades: { ...base.upgrades, ...(parsed.upgrades || {}) },
        allies: { ...base.allies, ...(parsed.allies || {}) },
        prestige: {
          ...base.prestige,
          ...(parsed.prestige || {}),
          meta: {
            ...base.prestige.meta,
            ...((parsed.prestige && parsed.prestige.meta) || {}),
          },
        },
        totalOfflineGold:
          typeof parsed.totalOfflineGold === 'number' ? parsed.totalOfflineGold : 0,
      };
      if (!this.data.allies.frog) this.data.allies.frog = 1;
      this.migrateSave(fromVersion);
      return true;
    } catch (e) {
      console.warn('Load failed or corrupted save', e);
    }
    return false;
  }

  /**
   * Offline fast-forward: apply gold/sec for capped seconds.
   * Returns gold earned (caller shows modal).
   */
  fastForward(seconds: number, goldPerSec: number, offlineMult = 1): number {
    const earned = Math.max(0, seconds) * Math.max(0, goldPerSec) * Math.max(0, offlineMult);
    this.data.gold += earned;
    this.data.lifetimeGold += earned;
    this.data.totalOfflineGold = (this.data.totalOfflineGold || 0) + earned;
    return earned;
  }
}


```

## Source file: src/scenes/BootScene.ts

{
  "path": "src/scenes/BootScene.ts",
  "blob_sha": "697c4cb4679ae0751fc71a3f148c40fc2a1548eb",
  "sha256": "def1228744f1d5587da471cfb1840b133f5563960a652443f12fe81e7e28c449",
  "lines": 25,
  "bytes": 609,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/Spencereese/phaser-idle-quest/blob/925318728985fec1b419f2ae1c54064ac8a9a172/src/scenes/BootScene.ts",
  "function_signature_candidates": []
}

```text

﻿import Phaser from 'phaser';

/**
 * BootScene - first scene in the clean structure.
 * Handles resize and any engine init, then goes to Preload.
 */
export class BootScene extends Phaser.Scene {
  constructor() {
    super({ key: 'Boot' });
  }

  init() {
    this.scale.on('resize', this.handleResize, this);
    this.registry.set('gameVersion', '0.5.0-r5-offline-allies');
    console.log('[BootScene] ready');
  }

  create() {
    this.scene.start('Preload');
  }

  private handleResize(gameSize: Phaser.Structs.Size) {
    console.log('[BootScene] resized', gameSize.width, gameSize.height);
  }
}


```
