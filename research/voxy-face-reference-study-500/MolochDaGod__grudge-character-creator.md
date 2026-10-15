# Facial rendering source scan: MolochDaGod/grudge-character-creator

Repository: https://github.com/MolochDaGod/grudge-character-creator

Pinned source revision: 5476ce64a791b45fddd59dfecee96d584502e71e

Discovery topic: character creator 3D

Description: Grudge Studio — 3D Character Creator + Souls-like Boss Arena. Three.js, GLTF, PostFX, animation blending. GDevelop integrated.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"shader": ["skills/threejs-shaders/SKILL.md"]}

## Source file: server.js

{
  "path": "server.js",
  "blob_sha": "8d0282cbd9c3efea93f31488ba3fb4cec0df9ab9",
  "sha256": "c4b56187dc1d2cfd5670e6b4783ffd8955a2364fc319035a17404bb4bcb7d508",
  "lines": 90,
  "bytes": 2829,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 1
  },
  "url": "https://github.com/MolochDaGod/grudge-character-creator/blob/5476ce64a791b45fddd59dfecee96d584502e71e/server.js",
  "function_signature_candidates": []
}

```text

#!/usr/bin/env node
/**
 * Grudge Character Creator — Local game server.
 *
 * Serves:
 *   /           → dist/ (Vite build output)
 *   /assets/*   → parent directory (factioncharacters, animationsweapons, environment)
 *
 * Usage:
 *   npm run serve            # port 4010
 *   node server.js --port 8080
 */

import express from 'express';
import compression from 'compression';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const PORT = parseInt(process.argv.find((_, i, a) => a[i - 1] === '--port') || '4010', 10);
const DIST = path.join(__dirname, 'dist');
const PARENT = path.resolve(__dirname, '..');

const app = express();

// ── Compression ──
app.use(compression());

// ── CORS for all routes ──
app.use((req, res, next) => {
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET, HEAD, OPTIONS');
  next();
});

// ── MIME types for 3D assets ──
const MIME_OVERRIDES = {
  '.gltf': 'model/gltf+json',
  '.glb':  'model/gltf-binary',
  '.fbx':  'application/octet-stream',
  '.bin':  'application/octet-stream',
  '.tga':  'application/octet-stream',
  '.hdr':  'application/octet-stream',
};

function setCustomMime(res, filePath) {
  const ext = path.extname(filePath).toLowerCase();
  if (MIME_OVERRIDES[ext]) {
    res.setHeader('Content-Type', MIME_OVERRIDES[ext]);
  }
}

// ── /assets/* → serve from parent directory (3D models, textures, animations) ──
app.use('/assets', express.static(PARENT, {
  setHeaders: setCustomMime,
  maxAge: '1h',
  index: false,
  dotfiles: 'ignore',
}));

// ── / → serve the built app from dist/ ──
app.use(express.static(DIST, {
  maxAge: '10m',
  index: 'index.html',
}));

// ── SPA fallback → index.html for client-side routing ──
app.use((req, res) => {
  // Don't fallback for asset requests that genuinely 404
  if (req.path.startsWith('/assets/')) {
    return res.status(404).json({ error: 'Asset not found', path: req.path });
  }
  res.sendFile(path.join(DIST, 'index.html'));
});

// ── Start ──
app.listen(PORT, '0.0.0.0', () => {
  console.log('');
  console.log('  ⚔️  Grudge Character Creator');
  console.log('  ────────────────────────────');
  console.log(`  Local:   http://localhost:${PORT}`);
  console.log(`  Assets:  http://localhost:${PORT}/assets/`);
  console.log('');
  console.log('  /assets/factioncharacters/ADDITIONAL_MODELS/  → GLTF character models');
  console.log('  /assets/animationsweapons/                    → animation packs');
  console.log('  /environment/boss_wukong/                     → boss fight arena');
  console.log('');
});


```

## Source file: src/main.js

{
  "path": "src/main.js",
  "blob_sha": "f1447fef97f1c0f81a5ec449f021bd399149bc6e",
  "sha256": "3e121cb2cbe3276dadd9a74fe545210db5fa1badbbd4a87746ef0a23ecdb820f",
  "lines": 1157,
  "bytes": 46579,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 1,
    "skin": 2,
    "sss": 6,
    "rig": 10,
    "lip": 15
  },
  "url": "https://github.com/MolochDaGod/grudge-character-creator/blob/5476ce64a791b45fddd59dfecee96d584502e71e/src/main.js",
  "function_signature_candidates": []
}

```text

import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { loadModel, loadModelFromFile, loadAnimationClips, prepareModel, fbxLoader, initKTX2, SUPPORTED_EXTENSIONS } from './modules/SmartLoader.js';

import { EquipmentManager } from './modules/EquipmentManager.js';
import { getAllRaces, loadManifest, getAnimationPacks, getWeaponModelPacks } from './modules/FactionRegistry.js';
import {
  ATTRIBUTES, ATTR_KEYS, MAX_POINTS,
  calculateDerivedStats, simulateCombat, createDefaultCharacter
} from './modules/StatsEngine.js';
import { grudgeAuth } from './modules/GrudgeAuth.js';
import { characterStore } from './modules/CharacterStore.js';
import { equippedFromModel3d } from './modules/CharacterBridge.js';
import {
  CLASSES, CLASS_SKILLS, PROFESSIONS, WORGE_FORMS, MAGIC_VFX,
  WEAPON_TYPES, WEAPON_SKILLS, MASTERY_TIERS, getMasteryProgress,
  WEAPON_ANIM_MAP, COMBAT_ACTIONS,
} from './modules/GameData.js';
import { WeaponAnimController } from './modules/WeaponAnimController.js';

// ── New modules from threejs-skills integration ──
import { assetCache } from './modules/AssetCache.js';
import { PostFX } from './modules/PostFX.js';
import { BoneAttachment } from './modules/BoneAttachment.js';
import { BossFight } from './modules/BossFight.js';
import { VFXManager } from './modules/VFXManager.js';
import { ForgePanel } from './modules/ForgePanel.js';
import { resolveTextures, classifyStyle } from './modules/TextureResolver.js';
import { grudge6RaceTextureUrl, normalizeRaceId } from './modules/Grudge6Paths.js';
import { ASSET_BASE, isProduction } from './modules/AssetConfig.js';
import { BoneWeaponEditor } from './modules/BoneWeaponEditor.js';

// ════════════════════════════════════════════════════════════
// State
// ════════════════════════════════════════════════════════════
let scene, camera, renderer, controls, clock;
let mixer = null;
let currentModel = null;
let currentActions = {};
let currentAction = null;
let equipMgr = null;
let skeletonHelper = null;
let wireframeMode = false;
let character = createDefaultCharacter(10);
const dummies = [];

// Track current editor state for persistence
let currentFactionId = null;
let currentRaceId = null;

// Weapon animation controller
let weaponCtrl = null;
let vfxMgr = null;

// Post-processing, bone attachment, boss fight
let postfx = null;
let boneAttach = new BoneAttachment();
let bossFight = null;
let forgePanel = null;
let boneEditor = null;

// ════════════════════════════════════════════════════════════
// Scene Setup
// ════════════════════════════════════════════════════════════
function initScene() {
  const container = document.getElementById('viewport');
  clock = new THREE.Clock();

  scene = new THREE.Scene();
  scene.background = new THREE.Color(0x1a1a2e);
  scene.fog = new THREE.Fog(0x1a1a2e, 30, 80);

  // Camera
  camera = new THREE.PerspectiveCamera(45, container.clientWidth / container.clientHeight, 0.1, 200);
  camera.position.set(0, 2.5, 5);

  // Renderer — best-in-class settings
  renderer = new THREE.WebGLRenderer({ antialias: true, powerPreference: 'high-performance' });
  renderer.setSize(container.clientWidth, container.clientHeight);
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFSoftShadowMap;
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 1.2;
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  container.appendChild(renderer.domElement);

  // Initialize Draco-compressed GLTF + KTX2 GPU texture support
  initKTX2(renderer);

  // Controls
  controls = new OrbitControls(camera, renderer.domElement);
  controls.target.set(0, 1, 0);
  controls.enableDamping = true;
  controls.dampingFactor = 0.08;
  controls.maxPolarAngle = Math.PI * 0.85;
  controls.minDistance = 1.5;
  controls.maxDistance = 20;
  controls.update();

  // Lighting
  const ambientLight = new THREE.AmbientLight(0x404060, 0.6);
  scene.add(ambientLight);

  const dirLight = new THREE.DirectionalLight(0xfff0dd, 1.5);
  dirLight.position.set(5, 10, 5);
  dirLight.castShadow = true;
  dirLight.shadow.mapSize.set(2048, 2048);
  dirLight.shadow.camera.near = 0.5;
  dirLight.shadow.camera.far = 50;
  dirLight.shadow.camera.left = -10;
  dirLight.shadow.camera.right = 10;
  dirLight.shadow.camera.top = 10;
  dirLight.shadow.camera.bottom = -10;
  scene.add(dirLight);

  const fillLight = new THREE.DirectionalLight(0x8888ff, 0.3);
  fillLight.position.set(-3, 5, -5);
  scene.add(fillLight);

  // Ground
  const groundGeo = new THREE.PlaneGeometry(40, 40);
  const groundMat = new THREE.MeshStandardMaterial({
    color: 0x2a2a3e, roughness: 0.9, metalness: 0.1,
  });
  const ground = new THREE.Mesh(groundGeo, groundMat);
  ground.rotation.x = -Math.PI / 2;
  ground.receiveShadow = true;
  scene.add(ground);

  // Grid
  const grid = new THREE.GridHelper(40, 40, 0x3a3a5e, 0x2a2a4e);
  grid.position.y = 0.01;
  scene.add(grid);

  // Resize
  window.addEventListener('resize', () => {
    camera.aspect = container.clientWidth / container.clientHeight;
    camera.updateProjectionMatrix();
    renderer.setSize(container.clientWidth, container.clientHeight);
    if (postfx) postfx.resize(container.clientWidth, container.clientHeight);
  });

  // Init post-processing pipeline
  postfx = new PostFX(renderer, scene, camera);

  // Init boss fight controller
  bossFight = new BossFight(scene, camera, { postfx, updateStatus });

  // Init Forge panel (drag-drop model inspector)
  forgePanel = new ForgePanel(scene, camera, controls, updateStatus);
  forgePanel.initDropZone(container);

  // Init Bone Weapon Editor
  boneEditor = new BoneWeaponEditor(scene, boneAttach, updateStatus);
  const boneEditorEl = document.getElementById('boneEditorPanel');
  if (boneEditorEl) boneEditor.init(boneEditorEl);

  // Forge file input + export button
  document.getElementById('forgeFileInput')?.addEventListener('change', async (e) => {
    const file = e.target.files?.[0];
    if (file && forgePanel) await forgePanel.loadDroppedFile(file);
  });
  document.getElementById('forgeExport')?.addEventListener('click', () => {
    forgePanel?.exportJSON();
  });
}

// ════════════════════════════════════════════════════════════
// Model Loading
// ════════════════════════════════════════════════════════════
async function applyGrudge6Atlas(model, raceId) {
  const atlasUrl = grudge6RaceTextureUrl(normalizeRaceId(raceId), isProduction ? ASSET_BASE : 'https://assets.grudge-studio.com');
  if (!atlasUrl) return 0;

  const atlas = await new Promise((resolve) => {
    new THREE.TextureLoader().load(atlasUrl, resolve, undefined, () => resolve(null));
  });
  if (!atlas) return 0;

  atlas.flipY = false;
  atlas.colorSpace = THREE.SRGBColorSpace;
  let patched = 0;
  model.traverse((child) => {
    if (!child.isMesh || !child.material || !child.geometry?.attributes?.uv) return;
    const mats = Array.isArray(child.material) ? child.material : [child.material];
    for (const mat of mats) {
      if (!mat?.isMeshStandardMaterial) continue;
      mat.map = atlas;
      mat.color.set(0xffffff);
      mat.needsUpdate = true;
      patched++;
    }
  });
  return patched;
}

async function loadCharacterModel(raceConfig) {
  const overlay = document.getElementById('loadingOverlay');
  const loadingText = document.getElementById('loadingText');
  overlay.classList.remove('hidden');
  loadingText.textContent = `Loading ${raceConfig.factionName} ${raceConfig.name}...`;

  // Clean up previous
  if (currentModel) {
    scene.remove(currentModel);
    if (skeletonHelper) { scene.remove(skeletonHelper); skeletonHelper = null; }
    mixer?.stopAllAction();
    mixer = null;
    currentActions = {};
    currentAction = null;
  }

  try {
    const { scene: model, animations } = await loadModel(raceConfig.model, (e) => {
      if (e.total) loadingText.textContent = `Loading... ${Math.floor((e.loaded / e.total) * 100)}%`;
    });

    // Scale, center, shadows via SmartLoader helper
    prepareModel(model);

    // Grudge6 race atlas (production FBX) then sibling / procedural fallback
    const atlasPatched = await applyGrudge6Atlas(model, raceConfig.raceId);
    const texResult = await resolveTextures(model, raceConfig.model);
    if (atlasPatched > 0) {
      console.log(`[Grudge6] Applied race atlas to ${atlasPatched} material slots`);
    } else if (texResult.discovered || texResult.generated) {
      const style = classifyStyle(model);
      console.log(`[TextureResolver] ${texResult.discovered} discovered, ${texResult.generated} generated (${style} style)`);
    }

    scene.add(model);
    currentModel = model;

    // Setup AnimationMixer
    mixer = new THREE.AnimationMixer(model);
    if (animations.length > 0) {
      animations.forEach(clip => {
        currentActions[clip.name] = mixer.clipAction(clip);
      });
    }

    // Equipment Manager
    equipMgr = new EquipmentManager(raceConfig.prefix);
    const slots = equipMgr.catalog(model);

    // VFX Manager — per-model because bones change with the loaded model
    vfxMgr = new VFXManager(scene, equipMgr.bones);

    // Weapon Animation Controller — binds weapon equips to animation packs
    weaponCtrl = new WeaponAnimController(mixer, fbxLoader, updateStatus, vfxMgr);
    weaponCtrl.setModel(model);
    weaponCtrl.onChange(() => buildHotbarUI());

    updateStatus(`Loaded ${raceConfig.name}: ${equipMgr.meshCount} equipment meshes found`);
    buildEquipmentUI(slots);
    buildHotbarUI();

    // Update Bone Weapon Editor with new model
    if (boneEditor) {
      const mId = currentFactionId && currentRaceId ? `${currentFactionId}_${currentRaceId}` : null;
      boneEditor.setModel(model, mId);
    }

    overlay.classList.add('hidden');

  } catch (err) {
    loadingText.textContent = `Error: ${err.message}`;
    updateStatus(`Failed to load model: ${err.message}`);
    console.error(err);
    // Hide overlay after 3s so user can retry
    setTimeout(() => overlay.classList.add('hidden'), 3000);
  }
}

// ════════════════════════════════════════════════════════════
// Animation Loading
// ════════════════════════════════════════════════════════════
async function loadAnimation(packKey, fileName) {
  const pack = getAnimationPacks()[packKey];
  if (!pack || !currentModel || !mixer) return;

  const url = pack.path + fileName;
  updateStatus(`Loading animation: ${fileName}`);

  try {
    const clips = await loadAnimationClips(url);

    if (clips.length > 0) {
      const clip = clips[0];
      clip.name = fileName.replace(/\.(fbx|FBX|gltf|glb)$/i, '');

      // Store and play
      if (currentActions[clip.name]) {
        currentActions[clip.name].stop();
      }
      const action = mixer.clipAction(clip, currentModel);
      currentActions[clip.name] = action;

      fadeToAction(clip.name);
      updateStatus(`Playing: ${clip.name}`);
      highlightAnimButton(clip.name);
    }
  } catch (err) {
    updateStatus(`Failed to load animation: ${err.message}`);
    console.error(err);
  }
}

function fadeToAction(name, duration = 0.2) {
  const nextAction = currentActions[name];
  if (!nextAction) return;

  nextAction.reset().play();
  nextAction.timeScale = parseFloat(document.getElementById('animSpeed').value) || 1.0;

  if (currentAction && currentAction !== nextAction) {
    currentAction.crossFadeTo(nextAction, duration, false);
  }
  currentAction = nextAction;
}

// ════════════════════════════════════════════════════════════
// UI Builders
// ════════════════════════════════════════════════════════════
function buildRaceSelector() {
  const container = document.getElementById('raceSelector');
  const races = getAllRaces();

  container.innerHTML = races.map(r => `
    <button class="faction-btn" data-faction="${r.factionId}" data-race="${r.raceId}">
      <span class="faction-dot" style="background:${r.factionColor}"></span>
      <span>${r.factionName} — ${r.name}</span>
    </button>
  `).join('');

  container.addEventListener('click', (e) => {
    const btn = e.target.closest('.faction-btn');
    if (!btn) return;
    container.querySelectorAll('.faction-btn').forEach(b => b.classList.remove('active'));
    btn.classList.add('active');

    const race = races.find(r => r.factionId === btn.dataset.faction && r.raceId === btn.dataset.race);
    if (race) {
      currentFactionId = race.factionId;
      currentRaceId = race.raceId;
      loadCharacterModel(race);
    }
  });
}

function buildEquipmentUI(slots) {
  const container = document.getElementById('equipmentPanel');
  const grouped = equipMgr.getGroupedSlots();

  let html = '';
  for (const [groupName, groupSlots] of Object.entries(grouped)) {
    const slotEntries = Object.entries(groupSlots);
    if (slotEntries.length === 0) continue;

    html += `<div class="slot-group"><h4>${groupName}</h4>`;
    for (const [slot, info] of slotEntries) {
      html += `<div class="slot-row"><span class="slot-label">${slot}</span>`;
      for (const variant of info.variants) {
        const label = variant === '_default' ? '●' : variant;
        const isEquipped = info.equipped === variant;
        html += `<button class="slot-btn ${isEquipped ? 'equipped' : ''}"
          data-slot="${slot}" data-variant="${variant}">${label}</button>`;
      }
      html += `<button class="slot-btn" data-slot="${slot}" data-variant="_none" title="Unequip">✕</button>`;
      html += `</div>`;
    }
    html += `</div>`;
  }
  container.innerHTML = html;

  container.addEventListener('click', async (e) => {
    const btn = e.target.closest('.slot-btn');
    if (!btn) return;
    const { slot, variant } = btn.dataset;

    if (variant === '_none') {
      equipMgr.unequip(slot);
      // If unequipping a weapon, sheath animations
      if (WEAPON_ANIM_MAP[slot] && weaponCtrl) {
        weaponCtrl.sheathWeapon();
      }
    } else {
      // Use equipWeapon for weapon/shield slots for mutual exclusion
      const isWeaponSlot = ['axe','hammer','sword','pick','spear','bow','staff','shield'].includes(slot);
      if (isWeaponSlot) {
        equipMgr.equipWeapon(slot, variant);
        // Auto-switch to weapon's animation pack
        if (WEAPON_ANIM_MAP[slot] && weaponCtrl && currentModel && mixer) {
          await weaponCtrl.equipWeapon(slot, getAnimationPacks(), currentModel);
          // Auto-select the animation pack in the dropdown
          const packSelect = document.getElementById('weaponPackSelect');
          if (packSelect) {
            packSelect.value = WEAPON_ANIM_MAP[slot].animPack;
            packSelect.dispatchEvent(new Event('change'));
          }
        }
      } else {
        equipMgr.equip(slot, variant);
      }
    }
    // Refresh UI
    buildEquipmentUI(equipMgr.getSlotSummary());
  });
}

function buildAnimationUI() {
  const select = document.getElementById('weaponPackSelect');
  for (const [key, pack] of Object.entries(getAnimationPacks())) {
    const opt = document.createElement('option');
    opt.value = key;
    opt.textContent = pack.name;
    select.appendChild(opt);
  }

  select.addEventListener('change', () => {
    const pack = getAnimationPacks()[select.value];
    const list = document.getElementById('animList');
    if (!pack) { list.innerHTML = ''; return; }

    list.innerHTML = pack.files.map(f => {
      const label = f.replace('.fbx', '').replace('.FBX', '');
      return `<button class="anim-btn" data-pack="${select.value}" data-file="${f}" title="${label}">${label}</button>`;
    }).join('');
  });

  document.getElementById('animList').addEventListener('click', (e) => {
    const btn = e.target.closest('.anim-btn');
    if (!btn) return;
    loadAnimation(btn.dataset.pack, btn.dataset.file);
  });

  // Speed control
  document.getElementById('animSpeed').addEventListener('input', (e) => {
    const val = parseFloat(e.target.value);
    document.getElementById('animSpeedVal').textContent = val.toFixed(1) + 'x';
    if (currentAction) currentAction.timeScale = val;
  });

  // Pause/Stop
  document.getElementById('animPause').addEventListener('click', () => {
    if (currentAction) {
      currentAction.paused = !currentAction.paused;
      document.getElementById('animPause').textContent = currentAction.paused ? '▶ Play' : '⏸ Pause';
    }
  });
  document.getElementById('animStop').addEventListener('click', () => {
    if (currentAction) { currentAction.stop(); currentAction = null; }
    document.querySelectorAll('.anim-btn').forEach(b => b.classList.remove('playing'));
  });
}

function highlightAnimButton(name) {
  document.querySelectorAll('.anim-btn').forEach(b => {
    b.classList.toggle('playing', b.textContent === name);
  });
}

// ════════════════════════════════════════════════════════════
// Stats Panel
// ════════════════════════════════════════════════════════════
function buildStatsPanel() {
  const container = document.getElementById('attributeSliders');
  container.innerHTML = ATTR_KEYS.map(key => {
    const attr = ATTRIBUTES[key];
    return `<div class="attr-row">
      <label style="color:${attr.color}" title="${attr.desc}">${attr.icon} ${key}</label>
      <input type="range" min="0" max="80" value="${character.attrs[key]}" data-attr="${key}">
      <span class="val" data-val="${key}">${character.attrs[key]}</span>
    </div>`;
  }).join('');

  container.addEventListener('input', (e) => {
    const slider = e.target.closest('input[data-attr]');
    if (!slider) return;
    const key = slider.dataset.attr;
    character.attrs[key] = parseInt(slider.value);
    document.querySelector(`[data-val="${key}"]`).textContent = slider.value;
    recalcStats();
  });

  recalcStats();
}

function recalcStats() {
  const total = ATTR_KEYS.reduce((sum, k) => sum + (character.attrs[k] || 0), 0);
  document.getElementById('pointsDisplay').textContent = `${total} / ${MAX_POINTS}`;
  document.getElementById('pointsDisplay').style.color = total > MAX_POINTS ? '#ef4444' : total === MAX_POINTS ? '#6ee7b7' : '#e8eaf6';

  character.stats = calculateDerivedStats(character.attrs, character.level);

  const statsEl = document.getElementById('derivedStats');
  const statSections = [
    { title: '⚔️ Offense', css: 'offense', stats: {
      meleeAttack: 'Melee ATK', rangedAttack: 'Ranged ATK', spellPower: 'Spell Power',
      attackSpeed: 'ATK Speed', critChance: 'Crit %', critDamage: 'Crit DMG %',
      defenseBreak: 'Def Break', armorPenetration: 'Armor Pen', accuracy: 'Accuracy',
    }},
    { title: '🛡️ Defense', css: 'defense', stats: {
      maxHP: 'Max HP', maxMana: 'Max Mana', maxStamina: 'Max Stamina',
      defense: 'Defense', magicResist: 'Magic Res', blockChance: 'Block %',
      blockFactor: 'Block Factor', dodgeChance: 'Dodge %', critEvasion: 'Crit Evasion',
      ccResistance: 'CC Resist', absorbFactor: 'Absorb %',
    }},
    { title: '💚 Regen', css: 'regen', stats: {
      hpRegen: 'HP / sec', manaRegen: 'Mana / sec', staminaRegen: 'Stam / sec',
    }},
    { title: '⚡ Utility', css: 'utility', stats: {
      moveSpeed: 'Move Speed', cooldownReduction: 'CDR %', abilityCostRed: 'Cost Reduce %',
      comboCooldownRed: 'Combo CDR %', carryWeight: 'Carry Weight',
      miningBonus: 'Mining +', craftingBonus: 'Crafting +', harvestBonus: 'Harvest +',
    }},
    { title: '💀 Summary', css: 'offense', stats: {
      combatPower: 'Combat Power',
    }},
  ];

  const fmt = (v) => typeof v === 'number' ? (Number.isInteger(v) ? v : v.toFixed(1)) : v;
  statsEl.innerHTML = statSections.map(sec => {
    const rows = Object.entries(sec.stats).map(([key, label]) => {
      const val = character.stats[key] ?? 0;
      return `<div class="stat-item stat-item--${sec.css}"><span class="label">${label}</span><span class="value">${fmt(val)}</span></div>`;
    }).join('');
    return `<div style="grid-column:1/-1;font-size:.65rem;color:var(--muted);padding:4px 0 2px;border-top:1px solid var(--border);margin-top:4px;">${sec.title}</div>${rows}`;
  }).join('');

  // Header HP/MP/SP resource bars (character sheet: current = max).
  const setResLbl = (id, label, val) => {
    const el = document.getElementById(id);
    if (el) el.textContent = `${label} ${Math.round(val)}`;
  };
  setResLbl('resHPLbl', 'HP', character.stats.maxHP ?? 0);
  setResLbl('resMPLbl', 'MP', character.stats.maxMana ?? 0);
  setResLbl('resSPLbl', 'SP', character.stats.maxStamina ?? 0);
}

// Combat test
function setupCombatTest() {
  const dummy = createDefaultCharacter(10);
  // Dummy has even stats but lower
  ATTR_KEYS.forEach(k => dummy.attrs[k] = 15);
  dummy.stats = calculateDerivedStats(dummy.attrs, 10);

  document.getElementById('combatTestBtn').addEventListener('click', () => {
    const result = simulateCombat(character, dummy);
    const logEl = document.getElementById('combatLog');
    logEl.innerHTML = result.log.map(line => {
      if (line.includes('CRITICAL')) return `<div class="crit">${line}</div>`;
      if (line.includes('BLOCKED')) return `<div class="block">${line}</div>`;
      return `<div>${line}</div>`;
    }).join('');
    logEl.scrollTop = logEl.scrollHeight;
  });
}

// ════════════════════════════════════════════════════════════
// Admin Panel
// ════════════════════════════════════════════════════════════
function setupAdminPanel() {
  // Toggle wireframe
  document.getElementById('toggleWireframe').addEventListener('click', () => {
    wireframeMode = !wireframeMode;
    currentModel?.traverse(child => {
      if (child.isMesh || child.isSkinnedMesh) {
        const mats = Array.isArray(child.material) ? child.material : [child.material];
        mats.forEach(m => { m.wireframe = wireframeMode; });
      }
    });
    updateStatus(`Wireframe: ${wireframeMode ? 'ON' : 'OFF'}`);
  });

  // Toggle skeleton helper
  document.getElementById('toggleSkeleton').addEventListener('click', () => {
    if (skeletonHelper) {
      scene.remove(skeletonHelper);
      skeletonHelper = null;
    } else if (currentModel) {
      let skeleton = null;
      currentModel.traverse(c => { if (c.isSkinnedMesh && !skeleton) skeleton = c; });
      if (skeleton) {
      skeletonHelper = new THREE.SkeletonHelper(currentModel);
        scene.add(skeletonHelper);
      }
    }
  });

  // Show all equipment
  document.getElementById('showAllParts').addEventListener('click', () => {
    if (equipMgr) {
      equipMgr.showAll();
      buildEquipmentUI(equipMgr.getSlotSummary());
      updateStatus('All equipment visible');
    }
  });

  // Reset camera
  document.getElementById('resetCamera').addEventListener('click', () => {
    camera.position.set(0, 2.5, 5);
    controls.target.set(0, 1, 0);
    controls.update();
  });

  // Spawn test dummy
  document.getElementById('spawnDummy').addEventListener('click', () => {
    const geo = new THREE.CapsuleGeometry(0.3, 1.2, 4, 8);
    const mat = new THREE.MeshStandardMaterial({ color: 0xcc4444, roughness: 0.6 });
    const dummy = new THREE.Mesh(geo, mat);
    const angle = Math.random() * Math.PI * 2;
    const dist = 3 + Math.random() * 3;
    dummy.position.set(Math.cos(angle) * dist, 0.9, Math.sin(angle) * dist);
    dummy.castShadow = true;
    scene.add(dummy);
    dummies.push(dummy);
    updateStatus(`Spawned dummy (${dummies.length} total)`);
  });

  // Toggle PostFX
  const pfxBtn = document.getElementById('togglePostFX');
  if (pfxBtn) pfxBtn.addEventListener("click", () => {
    if (postfx) {
      const on = postfx.toggle();
      updateStatus("Post-processing: " + (on ? "ON" : "OFF"));
    }
  });

  // Boss Fight button
  const bossBtn = document.getElementById('startBossFight');
  if (bossBtn) bossBtn.addEventListener("click", async () => {
    if (!bossFight) return;
    if (bossFight.isActive) {
      bossFight.exit();
    } else {
      if (!currentModel) { updateStatus("Load a character first!"); return; }
      await bossFight.enter(currentModel, mixer);
    }
  });

}

// ════════════════════════════════════════════════════════════
// Utility
// ════════════════════════════════════════════════════════════
function updateStatus(msg) {
  document.getElementById('statusBar').textContent = msg;
}

// ════════════════════════════════════════════════════════════
// Render Loop
// ════════════════════════════════════════════════════════════
function animate() {
  const dt = clock.getDelta();
  controls.update();
  if (mixer) mixer.update(dt);

  // VFX update — ticks particle lifetimes and normalized-time watchers
  if (vfxMgr) vfxMgr.update(dt);

  // Boss fight update
  if (bossFight && bossFight.isActive) {
    bossFight.update(dt, currentModel);
  }

  // Render via PostFX pipeline (falls back to direct render when disabled)
  if (postfx) {
    postfx.update(dt);
  } else {
    renderer.render(scene, camera);
  }
}

// ════════════════════════════════════════════════════════════
// Tab System
// ════════════════════════════════════════════════════════════
function setupTabs() {
  const tabBar = document.getElementById('mainTabBar');
  tabBar.addEventListener('click', (e) => {
    const btn = e.target.closest('.tab-btn');
    if (!btn) return;
    const tabId = btn.dataset.tab;
    tabBar.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
    btn.classList.add('active');
    document.querySelectorAll('#rightPanel .tab-content').forEach(c => c.classList.remove('active'));
    document.getElementById(tabId).classList.add('active');
  });
}

// ════════════════════════════════════════════════════════════
// Class Skills Tab
// ════════════════════════════════════════════════════════════
let selectedClass = null;

function buildClassSelector() {
  const container = document.getElementById('classSelector');
  container.innerHTML = Object.entries(CLASSES).map(([id, cls]) => `
    <div class="class-card" data-class="${id}">
      <div class="class-header">
        <span style="color:${cls.color}">${cls.icon}</span>
        <span style="color:${cls.color}">${cls.name}</span>
        <span style="font-size:.6rem;color:var(--muted);margin-left:auto;">${cls.primaryAttr}/${cls.secondaryAttr}</span>
      </div>
      <div class="class-desc">${cls.desc}</div>
    </div>
  `).join('');

  container.addEventListener('click', (e) => {
    const card = e.target.closest('.class-card');
    if (!card) return;
    selectedClass = card.dataset.class;
    container.querySelectorAll('.class-card').forEach(c => c.classList.remove('selected'));
    card.classList.add('selected');
    buildClassSkillTrees(selectedClass);
  });
}

function buildClassSkillTrees(classId) {
  const container = document.getElementById('classSkillTrees');
  const classData = CLASS_SKILLS[classId];
  if (!classData) { container.innerHTML = ''; return; }

  const cls = CLASSES[classId];
  let html = `<p style="font-size:.7rem;color:var(--muted);margin-bottom:6px;">${cls.passive}</p>`;
  html += `<p style="font-size:.65rem;color:var(--muted);margin-bottom:8px;">Weapons: ${cls.weaponTypes.map(w => WEAPON_TYPES[w]?.name || w).join(', ')}</p>`;

  for (const [treeId, tree] of Object.entries(classData.trees)) {
    html += `<div class="tree-section">`;
    html += `<div class="tree-header">${tree.icon} ${tree.name} <span style="font-size:.6rem;color:var(--muted);margin-left:auto;">${tree.desc}</span></div>`;
    for (const skill of tree.skills) {
      const modelBadge = skill.model ? ' <span style="color:#fbbf24;font-size:.6rem;">[3D]</span>' : '';
      html += `<div class="skill-node">
        <span class="skill-lvl">Lv${skill.level}</span>
        <span class="skill-name">${skill.name}${modelBadge}</span>
        <span class="skill-cost">${skill.cost}pt</span>
        <div class="skill-desc">${skill.desc}${skill.model ? '<br><em style="color:#fbbf24;">Has 3D form model</em>' : ''}</div>
      </div>`;
    }
    html += `</div>`;
  }

  // Show Worge forms preview if Worge class
  if (classId === 'worge') {
    html += `<h3 style="margin-top:10px;">Form Models</h3>`;
    for (const [fid, form] of Object.entries(WORGE_FORMS)) {
      const hasModel = form.model ? '\u2705' : '\u274c';
      html += `<div class="skill-node">
        <span class="skill-lvl">${form.icon}</span>
        <span class="skill-name">${form.name} ${hasModel}</span>
        <div class="skill-desc">${form.desc}</div>
      </div>`;
    }
  }

  container.innerHTML = html;
}

// ════════════════════════════════════════════════════════════
// Weapon Skills Tab
// ════════════════════════════════════════════════════════════
function buildWeaponTypeGrid() {
  const grid = document.getElementById('weaponTypeGrid');
  grid.innerHTML = Object.entries(WEAPON_TYPES).map(([id, wep]) =>
    `<button class="wep-type-btn" data-wep="${id}" title="${wep.name}">${wep.icon}<br>${wep.name.split(' ').pop()}</button>`
  ).join('');

  grid.addEventListener('click', (e) => {
    const btn = e.target.closest('.wep-type-btn');
    if (!btn) return;
    grid.querySelectorAll('.wep-type-btn').forEach(b => b.classList.remove('active'));
    btn.classList.add('active');
    buildWeaponSkillTree(btn.dataset.wep);
  });
}

function buildWeaponSkillTree(weaponId) {
  const container = document.getElementById('weaponSkillTree');
  const wepData = WEAPON_SKILLS[weaponId];
  const wepType = WEAPON_TYPES[weaponId];
  if (!wepData) { container.innerHTML = ''; return; }

  let html = `<p style="font-size:.65rem;color:var(--muted);margin-bottom:6px;">Classes: ${wepType.classes.map(c => CLASSES[c]?.name || c).join(', ')} • ${wepType.hand.toUpperCase()}</p>`;
  html += `<div class="tree-section">`;
  html += `<div class="tree-header">${wepData.icon} ${wepData.name} Skill Tree</div>`;
  for (const skill of wepData.skills) {
    html += `<div class="skill-node">
      <span class="skill-lvl">Lv${skill.level}</span>
      <span class="skill-name">${skill.name}</span>
      <span class="skill-cost" style="color:var(--muted);">${skill.bonus}</span>
      <div class="skill-desc">${skill.desc}</div>
    </div>`;
  }
  html += `</div>`;
  container.innerHTML = html;
}

// ════════════════════════════════════════════════════════════
// Professions Tab
// ════════════════════════════════════════════════════════════
function buildProfessionsPanel() {
  const container = document.getElementById('professionsList');
  container.innerHTML = Object.entries(PROFESSIONS).map(([id, prof]) => {
    const tiers = prof.tiers.map(t =>
      `<div class="prof-tier">
        <span class="tier-lvl">Lv${t.level}</span>
        <span class="tier-name">${t.name}</span>
        <span class="tier-res">${t.resources.join(', ')}</span>
      </div>`
    ).join('');
    return `<div class="prof-card">
      <div class="prof-header">
        <span>${prof.icon}</span>
        <span style="color:${prof.color}">${prof.name}</span>
        <span style="font-size:.6rem;color:var(--muted);margin-left:auto;">Bonus: ${prof.attr}</span>
      </div>
      <p style="font-size:.65rem;color:var(--muted);margin-bottom:4px;">${prof.desc}</p>
      ${tiers}
    </div>`;
  }).join('');
}

// ════════════════════════════════════════════════════════════
// Weapon Mastery Tab
// ════════════════════════════════════════════════════════════
function buildMasteryPanel() {
  const container = document.getElementById('masteryList');
  // Simulated mastery XP per weapon (in real game, this comes from server)
  const simulatedXP = {};
  for (const id of Object.keys(WEAPON_TYPES)) {
    simulatedXP[id] = Math.floor(Math.random() * 15000);
  }

  container.innerHTML = Object.entries(WEAPON_TYPES).map(([id, wep]) => {
    const xp = simulatedXP[id];
    const { current, next, progress } = getMasteryProgress(xp);
    const pct = Math.floor(progress * 100);
    const nextLabel = next ? `${next.name} (${next.xp} XP)` : 'MAX';
    return `<div class="mastery-row">
      <span class="mastery-icon">${wep.icon}</span>
      <div class="mastery-info">
        <div class="mastery-name">${wep.name}</div>
        <div class="mastery-tier">${current.name} — ${current.bonus}</div>
        <div class="mastery-bar"><div class="mastery-fill" style="width:${pct}%"></div></div>
      </div>
      <span class="mastery-xp">${xp} XP</span>
    </div>`;
  }).join('');
}

// ════════════════════════════════════════════════════════════
// Persistence UI
// ════════════════════════════════════════════════════════════
function buildSavedCharactersList() {
  const container = document.getElementById('savedCharactersList');
  const countEl = document.getElementById('charCount');
  const chars = characterStore.characters;
  countEl.textContent = `(${chars.length})`;

  if (chars.length === 0) {
    container.innerHTML = '<p style="color:var(--muted);font-size:.8rem;">No saved characters yet</p>';
    return;
  }

  container.innerHTML = chars.map(c => `
    <div class="saved-char ${c.id === characterStore.activeId ? 'active' : ''}" data-id="${c.id}">
      <span class="char-name">${c.name}</span>
      <span class="char-race">${c.factionId}/${c.raceId}</span>
      <span class="char-delete" data-delete="${c.id}" title="Delete">✕</span>
    </div>
  `).join('');

  container.addEventListener('click', async (e) => {
    const deleteBtn = e.target.closest('[data-delete]');
    if (deleteBtn) {
      e.stopPropagation();
      const id = deleteBtn.dataset.delete;
      if (confirm('Delete this character?')) {
        await characterStore.remove(id);
        updateStatus('Character deleted');
      }
      return;
    }
    const row = e.target.closest('.saved-char');
    if (!row) return;
    const id = row.dataset.id;
    characterStore.setActive(id);
    loadSavedCharacter(id);
  });
}

function loadSavedCharacter(id) {
  const char = characterStore.characters.find(c => c.id === id);
  if (!char) return;

  const races = getAllRaces();
  const race = races.find(r => r.factionId === char.factionId && r.raceId === char.raceId);
  if (!race) {
    updateStatus(`Race ${char.factionId}/${char.raceId} not found`);
    return;
  }

  // Restore faction/race
  currentFactionId = char.factionId;
  currentRaceId = char.raceId;

  // Highlight the race button
  document.querySelectorAll('.faction-btn').forEach(b => {
    b.classList.toggle('active',
      b.dataset.faction === char.factionId && b.dataset.race === char.raceId);
  });

  // Restore attributes
  if (char.attrs) {
    for (const key of ATTR_KEYS) {
      if (char.attrs[key] !== undefined) {
        character.attrs[key] = char.attrs[key];
        const slider = document.querySelector(`input[data-attr="${key}"]`);
        if (slider) slider.value = char.attrs[key];
        const valEl = document.querySelector(`[data-val="${key}"]`);
        if (valEl) valEl.textContent = char.attrs[key];
      }
    }
    character.level = char.level || 1;
    recalcStats();
  }

  // Load model and restore equipment after load
  loadCharacterModel(race).then(() => {
    const equipped = char.equipped || (char.model3d ? equippedFromModel3d(char) : null);
    if (equipped && equipMgr) {
      for (const [slot, variant] of Object.entries(equipped)) {
        equipMgr.equip(slot, variant);
      }
      buildEquipmentUI(equipMgr.getSlotSummary());
    }
    updateStatus(`Loaded: ${char.name}`);
    document.getElementById('updateBuildBtn').style.display = '';
  });
}

function getEditorState() {
  return {
    factionId: currentFactionId,
    raceId: currentRaceId,
    equipped: equipMgr ? { ...equipMgr.equipped } : {},
    attrs: { ...character.attrs },
    level: character.level,
  };
}

function setupPersistence() {
  // Save button
  document.getElementById('saveBuildBtn').addEventListener('click', async () => {
    if (!currentFactionId || !currentRaceId) {
      updateStatus('Select a race first');
      return;
    }
    const name = prompt('Character name:', `Character ${characterStore.characters.length + 1}`);
    if (name === null) return;

    const state = getEditorState();
    state.name = name;
    await characterStore.save(state);
    document.getElementById('updateBuildBtn').style.display = '';
    updateStatus(`Saved: ${name}`);
  });

  // Update button
  document.getElementById('updateBuildBtn').addEventListener('click', async () => {
    if (!characterStore.activeId) return;
    const state = getEditorState();
    await characterStore.update(state);
    updateStatus('Build updated');
  });

  // React to store changes
  characterStore.addEventListener('change', () => buildSavedCharactersList());
  characterStore.addEventListener('error', (e) => updateStatus(`Error: ${e.detail?.message || 'Unknown'}`));
}

async function initPersistence() {
  const grudgeIdBtn = document.getElementById('grudgeIdBtn');
  const discordBtn = document.getElementById('discordBtn');
  const signOutBtn = document.getElementById('signOutBtn');
  const userDisplay = document.getElementById('userDisplay');

  async function onSignedIn(user) {
    try {
      userDisplay.textContent = user.displayName || user.grudgeId || '';
      if (grudgeIdBtn) grudgeIdBtn.style.display = 'none';
      if (discordBtn) discordBtn.style.display = 'none';
      if (signOutBtn) signOutBtn.style.display = '';
      await characterStore.load();
      buildSavedCharactersList();
      updateStatus('Signed in as ' + (user.displayName || user.grudgeId));
    } catch (err) {
      console.error('Auth load failed:', err);
      showOffline();
    }
  }

  function showOffline() {
    if (grudgeIdBtn) grudgeIdBtn.style.display = '';
    if (discordBtn) discordBtn.style.display = '';
    if (signOutBtn) signOutBtn.style.display = 'none';
    userDisplay.textContent = '';
    document.getElementById('savedCharactersList').innerHTML =
      '<p style="color:var(--muted);font-size:.8rem;">Sign in with Grudge ID to save characters</p>';
  }

  if (grudgeIdBtn) {
    grudgeIdBtn.addEventListener('click', () => {
      grudgeAuth.loginWithGrudgeId('/auth/callback?return=/game');
    });
  }

  if (discordBtn) {
    discordBtn.addEventListener('click', () => {
      grudgeAuth.loginWithDiscord('/auth/callback?return=/game');
    });
  }

  if (signOutBtn) {
    signOutBtn.addEventListener('click', () => {
      grudgeAuth.logout();
      characterStore.characters = [];
      characterStore.activeId = null;
      buildSavedCharactersList();
      showOffline();
      updateStatus('Signed out');
    });
  }

  // Handle OAuth callback (token in URL from Discord/Google redirect)
  const callbackUser = await grudgeAuth.handleOAuthCallback();
  if (callbackUser) {
    await onSignedIn(callbackUser);
    return;
  }

  // Try to restore existing session
  const existingUser = await grudgeAuth.init();
  if (existingUser) {
    await onSignedIn(existingUser);
  } else {
    showOffline();
  }
}

// ════════════════════════════════════════════════════════════
// Boot
// ════════════════════════════════════════════════════════════
// Boot sequence — load D1 manifest first, then build UI
async function boot() {
  initScene();

  // Fetch D1 manifest (overwrites bundled data if available)
  await loadManifest();

  buildRaceSelector();
  buildAnimationUI();
  buildStatsPanel();
  setupCombatTest();
  setupAdminPanel();
  setupTabs();
  buildClassSelector();
  buildWeaponTypeGrid();
  buildProfessionsPanel();
  buildMasteryPanel();
  setupCombatHotkeys();
  setupPersistence();
  renderer.setAnimationLoop(animate);
  updateStatus('Ready — select a faction race to load');
  initPersistence();
}
boot();

// ════════════════════════════════════════════════════════════
// Combat Hotkeys & Hotbar
// ════════════════════════════════════════════════════════════
function setupCombatHotkeys() {
  window.addEventListener('keydown', (e) => {
    if (e.target !== document.body) return;
    if (!weaponCtrl) return;

    switch (e.code) {
      case 'Digit1': weaponCtrl.triggerAction('slot1'); break;
      case 'Digit2': weaponCtrl.triggerAction('slot2'); break;
      case 'Digit3': weaponCtrl.triggerAction('slot3'); break;
      case 'Digit4': weaponCtrl.triggerAction('slot4'); break;
      case 'KeyQ':   weaponCtrl.triggerAction('block'); break;
      case 'KeyE':   weaponCtrl.triggerAction('dodge'); break;
      case 'KeyZ':   // Z-key combat mechanic
        updateStatus('⚡ Battle Cry!');
        weaponCtrl.triggerAction('slot4');
        break;
      case 'Tab':
        e.preventDefault();
        weaponCtrl.toggleMode();
        buildHotbarUI();
        break;
      case 'KeyX':
        weaponCtrl.sheathWeapon();
        buildHotbarUI();
        break;
    }
  });

  window.addEventListener('keyup', (e) => {
    if (e.code === 'KeyQ' && weaponCtrl) {
      weaponCtrl.releaseBlock();
    }
  });
}

function buildHotbarUI() {
  const container = document.getElementById('hotbarDisplay');
  if (!container) return;

  if (!weaponCtrl || !weaponCtrl.equippedWeaponType) {
    container.innerHTML = '<p style="color:var(--muted);font-size:.7rem;">Equip a weapon to see hotbar</p>';
    return;
  }

  const hotbar = weaponCtrl.getHotbar();
  if (!hotbar) { container.innerHTML = ''; return; }

  const modeLabel = weaponCtrl.mode === 'combat' ? '⚔️ Combat' : '🪨 Harvest';
  const stateLabel = weaponCtrl.state;
  const wpnName = WEAPON_TYPES[weaponCtrl.equippedWeaponType]?.name || weaponCtrl.equippedWeaponType;

  const slot = (key, keyLabel, fallback) => {
    const a = hotbar[key];
    const filled = !!a?.name;
    const name = filled ? a.name : (fallback || '');
    return `<div class="hb-slot${filled ? '' : ' empty'}" onclick="window._wc?.triggerAction('${key}')" title="${name || keyLabel}">
      <span class="key">${keyLabel}</span><span>${name}</span>
    </div>`;
  };

  container.innerHTML = `
    <div style="display:flex;align-items:center;gap:8px;margin-bottom:6px;">
      <span style="font-size:.72rem;color:var(--accent);font-family:var(--font-serif);text-transform:uppercase;letter-spacing:.5px;">${wpnName}</span>
      <span style="font-size:.6rem;color:var(--muted);">${modeLabel} • ${stateLabel}</span>
      <span style="font-size:.58rem;color:var(--muted);margin-left:auto;">Tab=mode • X=sheath</span>
    </div>
    <div class="hotbar">
      ${slot('slot1', '1')}${slot('slot2', '2')}${slot('slot3', '3')}${slot('slot4', '4')}
      ${slot('block', 'Q', 'Block')}${slot('dodge', 'E', 'Dodge')}
    </div>
  `;

  // Expose controller for hotbar button onclick
  window._wc = weaponCtrl;
}


```

## Source file: vite.config.js

{
  "path": "vite.config.js",
  "blob_sha": "8b6a39d349e7d7fea0d600787d55cef558f99b23",
  "sha256": "aa7e06a95f230214b12a077dbe52090e70e4427ae9279399d8046348b029abed",
  "lines": 98,
  "bytes": 2618,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 1
  },
  "url": "https://github.com/MolochDaGod/grudge-character-creator/blob/5476ce64a791b45fddd59dfecee96d584502e71e/vite.config.js",
  "function_signature_candidates": []
}

```text

import { defineConfig } from 'vite';
import path from 'path';
import fs from 'fs';

/**
 * Asset roots — maps URL prefix → absolute filesystem directory.
 *
 * /assets/factioncharacters/ → F:\Documents\Toon_RTS\Toon_RTS  (race FBX models)
 * /assets/animationsweapons/ → D:\Games\Models\grudgeracecharacters\animationsweapons (if present)
 * /assets/*                  → D:\Games\Models\grudgeracecharacters  (everything else)
 *
 * Checked in order: first match wins.
 */
const ASSET_ROOTS = [
  {
    prefix: '/assets/factioncharacters/',
    dir:    'F:\\Documents\\Toon_RTS\\Toon_RTS',
  },
  {
    prefix: '/assets/',
    dir:    path.resolve(__dirname, '..'),
  },
];

const MIME = {
  '.fbx':  'application/octet-stream',
  '.gltf': 'model/gltf+json',
  '.glb':  'model/gltf-binary',
  '.bin':  'application/octet-stream',
  '.png':  'image/png',
  '.jpg':  'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.tga':  'application/octet-stream',
  '.hdr':  'application/octet-stream',
  '.tif':  'image/tiff',
  '.json': 'application/json',
};

function serveParentAssets() {
  return {
    name: 'serve-parent-assets',
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        if (!req.url || !req.url.startsWith('/assets/')) return next();
        const url = decodeURIComponent(req.url);

        // Find first matching root
        let filePath = null;
        for (const { prefix, dir } of ASSET_ROOTS) {
          if (url.startsWith(prefix)) {
            const rel = url.slice(prefix.length);
            filePath = path.join(dir, rel);
            if (fs.existsSync(filePath)) break;
            filePath = null; // try next root
          }
        }

        if (!filePath) {
          res.statusCode = 404;
          res.end('Asset not found: ' + req.url);
          return;
        }

        const stat = fs.statSync(filePath);
        const ext  = path.extname(filePath).toLowerCase();
        res.setHeader('Content-Type', MIME[ext] || 'application/octet-stream');
        res.setHeader('Content-Length', stat.size);
        res.setHeader('Access-Control-Allow-Origin', '*');
        fs.createReadStream(filePath).pipe(res);
      });
    },
  };
}

export default defineConfig({
  base: '/creator/',
  root: '.',
  publicDir: 'public',
  plugins: [serveParentAssets()],
  server: {
    port: 3000,
    fs: {
      allow: ['.', '..'],
      strict: false,
    },
  },
  build: {
    outDir: 'dist/creator',
    chunkSizeWarningLimit: 1200,
    rollupOptions: {
      output: {
        manualChunks: {
          three: ['three'],
        },
      },
    },
  },
});


```

## Source file: worker/index.js

{
  "path": "worker/index.js",
  "blob_sha": "45b8af00d884006fd39f243cda6bdf4aa91dd2b6",
  "sha256": "40461602d930f8a62cc45ad88be6f9cf7511e29044ab732fd66a0f0e0d85ab76",
  "lines": 232,
  "bytes": 8460,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 1
  },
  "url": "https://github.com/MolochDaGod/grudge-character-creator/blob/5476ce64a791b45fddd59dfecee96d584502e71e/worker/index.js",
  "function_signature_candidates": []
}

```text

/**
 * Cloudflare Worker — Model Manifest API
 *
 * Queries D1 (grudge-models) and returns model/equipment/animation data.
 * Deploy: npx wrangler deploy (uses wrangler.toml at project root)
 *
 * Endpoints:
 *   GET /api/manifest           — full manifest (all models + equipment + anims)
 *   GET /api/models             — list all race models
 *   GET /api/models/:id         — single model with equipment slots
 *   GET /api/models/:id/equip   — equipment slots for a model
 *   GET /api/animations         — all animation packs
 *   GET /api/weapons            — all weapon model packs
 */

const ASSET_VERSION = 'v2';  // Bump when re-uploading textures to bust CDN cache

const CORS_HEADERS = {
  'Access-Control-Allow-Origin': '*',
  'Access-Control-Allow-Methods': 'GET, POST, PUT, DELETE, OPTIONS',
  'Access-Control-Allow-Headers': 'Content-Type, Authorization',
};

/** JSON with caching for reads, no-cache for writes */
function corsHeaders(cacheable = true) {
  return cacheable
    ? { ...CORS_HEADERS, 'Cache-Control': 'public, max-age=300' }
    : CORS_HEADERS;
}

function json(data, status = 200) {
  return new Response(JSON.stringify(data), {
    status,
    headers: { 'Content-Type': 'application/json', ...CORS_HEADERS },
  });
}

export default {
  async fetch(request, env) {
    if (request.method === 'OPTIONS') {
      return new Response(null, { headers: CORS_HEADERS });
    }

    const url = new URL(request.url);
    const path = url.pathname;

    try {
      // ── Full manifest (primary endpoint for frontend) ──
      if (path === '/api/manifest') {
        return await getFullManifest(env.DB);
      }

      // ── Models ──
      if (path === '/api/models') {
        const { results } = await env.DB.prepare(
          'SELECT * FROM models ORDER BY faction_id, race_id'
        ).all();
        return json({ models: results });
      }

      // ── Single model with equipment ──
      const modelMatch = path.match(/^\/api\/models\/([^/]+)$/);
      if (modelMatch) {
        return await getModelWithEquipment(env.DB, modelMatch[1]);
      }

      // ── Equipment for model ──
      const equipMatch = path.match(/^\/api\/models\/([^/]+)\/equip$/);
      if (equipMatch) {
        const { results } = await env.DB.prepare(
          'SELECT * FROM equipment_slots WHERE model_id = ? ORDER BY sort_order'
        ).bind(equipMatch[1]).all();
        return json({ slots: results });
      }

      // ── Animation packs ──
      if (path === '/api/animations') {
        const { results } = await env.DB.prepare(
          'SELECT * FROM animation_packs ORDER BY name'
        ).all();
        return json({ packs: results.map(p => ({ ...p, files: JSON.parse(p.files || '[]') })) });
      }

      // ── Weapon model packs ──
      if (path === '/api/weapons') {
        const { results } = await env.DB.prepare(
          'SELECT * FROM weapon_model_packs ORDER BY name'
        ).all();
        return json({ packs: results });
      }

      // ── Bone weapon attachments ──
      const attachListMatch = path.match(/^\/api\/weapon-attachments\/([^/]+)$/);
      if (request.method === 'GET' && attachListMatch) {
        const { results } = await env.DB.prepare(
          'SELECT * FROM weapon_bone_attachments WHERE model_id = ? ORDER BY created_at DESC'
        ).bind(attachListMatch[1]).all();
        return json({ attachments: results });
      }

      if (request.method === 'POST' && path === '/api/weapon-attachments') {
        const body = await request.json();
        const id = crypto.randomUUID();
        const { model_id, bone_name, weapon_url, weapon_name, slot_label,
                pos_x, pos_y, pos_z, rot_x, rot_y, rot_z, scale } = body;
        if (!model_id || !bone_name || !weapon_url) {
          return json({ error: 'model_id, bone_name, weapon_url required' }, 400);
        }
        await env.DB.prepare(
          `INSERT INTO weapon_bone_attachments (id, model_id, bone_name, weapon_url, weapon_name, slot_label, pos_x, pos_y, pos_z, rot_x, rot_y, rot_z, scale)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`
        ).bind(id, model_id, bone_name, weapon_url, weapon_name || '', slot_label || 'custom',
          pos_x || 0, pos_y || 0, pos_z || 0, rot_x || 0, rot_y || 0, rot_z || 0, scale || 1.0
        ).run();
        return json({ id, ...body }, 201);
      }

      if (request.method === 'PUT' && path.startsWith('/api/weapon-attachments/')) {
        const attachId = path.split('/').pop();
        const body = await request.json();
        const fields = [];
        const values = [];
        for (const key of ['bone_name','weapon_url','weapon_name','slot_label','pos_x','pos_y','pos_z','rot_x','rot_y','rot_z','scale']) {
          if (body[key] !== undefined) { fields.push(`${key} = ?`); values.push(body[key]); }
        }
        if (fields.length === 0) return json({ error: 'No fields to update' }, 400);
        fields.push("updated_at = datetime('now')");
        values.push(attachId);
        await env.DB.prepare(`UPDATE weapon_bone_attachments SET ${fields.join(', ')} WHERE id = ?`).bind(...values).run();
        return json({ id: attachId, updated: true });
      }

      if (request.method === 'DELETE' && path.startsWith('/api/weapon-attachments/')) {
        const attachId = path.split('/').pop();
        await env.DB.prepare('DELETE FROM weapon_bone_attachments WHERE id = ?').bind(attachId).run();
        return json({ deleted: attachId });
      }

      // ── Health check ──
      if (path === '/health' || path === '/') {
        return json({ status: 'ok', service: 'grudge-models', version: '2.0.0' });
      }

      return json({ error: 'Not found' }, 404);

    } catch (err) {
      console.error('Worker error:', err);
      return json({ error: 'Internal server error', message: err.message }, 500);
    }
  },
};

/**
 * Build the full manifest — the single payload the frontend needs
 * to initialize FactionRegistry, EquipmentManager, and animation packs.
 */
async function getFullManifest(db) {
  const [modelsRes, slotsRes, animsRes, weaponsRes] = await Promise.all([
    db.prepare('SELECT * FROM models ORDER BY faction_id, race_id').all(),
    db.prepare('SELECT * FROM equipment_slots ORDER BY model_id, sort_order').all(),
    db.prepare('SELECT * FROM animation_packs ORDER BY name').all(),
    db.prepare('SELECT * FROM weapon_model_packs ORDER BY name').all(),
  ]);

  // Group equipment slots by model_id
  const slotsByModel = {};
  for (const slot of slotsRes.results) {
    if (!slotsByModel[slot.model_id]) slotsByModel[slot.model_id] = [];
    slotsByModel[slot.model_id].push(slot);
  }

  // Build faction-grouped model structure (mirrors FactionRegistry.FACTIONS shape)
  const factions = {};
  for (const model of modelsRes.results) {
    if (!factions[model.faction_id]) {
      factions[model.faction_id] = {
        name: model.faction_name,
        color: model.faction_color,
        races: {},
      };
    }
    factions[model.faction_id].races[model.race_id] = {
      name: model.name,
      prefix: model.prefix,
      model: model.r2_url + '?' + ASSET_VERSION,
      format: model.format,
      skeletonType: model.skeleton_type,
      equipment: slotsByModel[model.id] || [],
    };
  }

  // Parse animation pack files JSON
  const animationPacks = {};
  for (const pack of animsRes.results) {
    animationPacks[pack.pack_key] = {
      name: pack.name,
      path: pack.r2_base_url,
      files: JSON.parse(pack.files || '[]'),
      extra: JSON.parse(pack.extra || '{}'),
    };
  }

  // Weapon model packs
  const weaponModelPacks = {};
  for (const pack of weaponsRes.results) {
    weaponModelPacks[pack.pack_key] = {
      name: pack.name,
      path: pack.r2_base_url,
      count: pack.count,
      prefix: pack.prefix,
    };
  }

  return json({
    version: '1.0.0',
    generatedAt: new Date().toISOString(),
    factions,
    animationPacks,
    weaponModelPacks,
  });
}

async function getModelWithEquipment(db, modelId) {
  const model = await db.prepare('SELECT * FROM models WHERE id = ?').bind(modelId).first();
  if (!model) return json({ error: 'Model not found' }, 404);

  const { results: slots } = await db.prepare(
    'SELECT * FROM equipment_slots WHERE model_id = ? ORDER BY sort_order'
  ).bind(modelId).all();

  return json({ model, equipment: slots });
}


```
