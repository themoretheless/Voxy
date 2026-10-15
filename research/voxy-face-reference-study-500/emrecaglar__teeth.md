# Facial rendering source scan: emrecaglar/teeth

Repository: https://github.com/emrecaglar/teeth

Pinned source revision: e0f1417a1e9f7d58480bde9a69f9833d87f4d08f

Discovery topic: teeth 3D

Description: Teeth 3D

Reported license: Apache-2.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"jaw": ["assets/preview/07-jaw-open.png"], "tooth": ["src/core/ToothGeometry.js"]}

## Source file: src/core/ToothGeometry.js

{
  "path": "src/core/ToothGeometry.js",
  "blob_sha": "795a2d49334773439a11f421f1d4058d57ec9b80",
  "sha256": "905e67b5251a62042be1403415dad5653d51278aac25dcae5d5c494feca7b3d7",
  "lines": 559,
  "bytes": 19531,
  "selection_score": 15,
  "keyword_line_counts": {
    "teeth": 1,
    "tooth": 10,
    "rig": 1,
    "face": 10,
    "lip": 5
  },
  "url": "https://github.com/emrecaglar/teeth/blob/e0f1417a1e9f7d58480bde9a69f9833d87f4d08f/src/core/ToothGeometry.js",
  "function_signature_candidates": []
}

```text

/**
 * Procedural tooth geometry generator.
 *
 * A tooth is built from three pieces that share one BufferGeometry:
 *   - the crown side wall  (a stack of super-elliptical rings)
 *   - the occlusal cap     (concentric rings displaced by a cusp height field)
 *   - one root per spec    (tapered tube swept along a quadratic bezier)
 *
 * The index buffer is split into two draw groups so the crown and the root can
 * carry different materials (needed for crowns, veneers, root canal therapy...).
 *   group 0 -> crown   group 1 -> root
 *
 * @module core/ToothGeometry
 */
import * as THREE from 'three';

const TAU = Math.PI * 2;

const lerp = (a, b, t) => a + (b - a) * t;
const clamp01 = (v) => (v < 0 ? 0 : v > 1 ? 1 : v);
const smoothstep = (t) => { const x = clamp01(t); return x * x * (3 - 2 * x); };

/**
 * Radius of a superellipse |x/a|^n + |z/b|^n = 1 at a given polar angle.
 * n = 2 is an ellipse, larger n squares off the corners (molars).
 */
function sectionRadius(theta, a, b, n) {
  const c = Math.abs(Math.cos(theta) / a);
  const s = Math.abs(Math.sin(theta) / b);
  return Math.pow(Math.pow(c, n) + Math.pow(s, n), -1 / n);
}

/** Crown semi-axes (mesiodistal, buccolingual) at height fraction t. */
function crownSection(spec, t) {
  const [ca, cb] = spec.cervical;
  const [oa, ob] = spec.occlusal;
  const w = spec.widestAt;
  let sa, sb;
  if (t <= w) {
    const u = smoothstep(t / w);
    sa = lerp(ca, 1, u); sb = lerp(cb, 1, u);
  } else {
    const u = smoothstep((t - w) / (1 - w));
    sa = lerp(1, oa, u); sb = lerp(1, ob, u);
  }
  return [sa * spec.md * 0.5, sb * spec.bl * 0.5];
}

/**
 * Smooth maximum of the cusp gaussians, evaluated in normalised occlusal
 * coordinates (nx, nz in -1..1). Produces cusp tips with natural grooves and
 * fossae in between.
 */
function cuspField(nx, nz, cusps) {
  if (!cusps || !cusps.length) return 0;
  const p = 6;
  let acc = 0;
  for (let i = 0; i < cusps.length; i++) {
    const c = cusps[i];
    const dx = (nx - c[0]) / c[3];
    const dz = (nz - c[1]) / c[4];
    const v = c[2] * Math.exp(-0.5 * (dx * dx + dz * dz));
    if (v > 1e-6) acc += Math.pow(v, p);
  }
  return acc > 0 ? Math.pow(acc, 1 / p) : 0;
}

function maxCuspHeight(cusps) {
  let m = 0;
  if (cusps) for (const c of cusps) m = Math.max(m, c[2]);
  return m;
}

/** Deterministic micro-relief so the enamel does not read as a plastic blob. */
function surfaceNoise(theta, t) {
  return 1 + 0.010 * Math.sin(theta * 9 + t * 7.3) + 0.006 * Math.sin(theta * 21 - t * 15.1);
}

/**
 * Flip triangle winding when a closed solid comes out inside-out.
 * Uses the divergence theorem, so it is independent of where the solid sits.
 */
export function ensureOutward(geometry) {
  const pos = geometry.getAttribute('position');
  const idx = geometry.getIndex().array;
  let vol = 0;
  const a = new THREE.Vector3(), b = new THREE.Vector3(), c = new THREE.Vector3();
  for (let i = 0; i < idx.length; i += 3) {
    a.fromBufferAttribute(pos, idx[i]);
    b.fromBufferAttribute(pos, idx[i + 1]);
    c.fromBufferAttribute(pos, idx[i + 2]);
    vol += a.dot(c.clone().cross(b));
  }
  if (vol > 0) {
    for (let i = 0; i < idx.length; i += 3) {
      const t = idx[i + 1]; idx[i + 1] = idx[i + 2]; idx[i + 2] = t;
    }
    geometry.getIndex().needsUpdate = true;
    geometry.computeVertexNormals();
  }
  return geometry;
}

// ---------------------------------------------------------------------------

class MeshBuilder {
  constructor() {
    this.pos = [];
    this.col = [];
    this.idx = [];
    this.groupStart = 0;
    this.groups = [];
  }

  vertex(x, y, z, c) {
    const i = this.pos.length / 3;
    this.pos.push(x, y, z);
    this.col.push(c.r, c.g, c.b);
    return i;
  }

  /** Bridge two closed rings of equal length; `low` must be the lower ring. */
  bridge(low, high) {
    const n = low.length;
    for (let k = 0; k < n; k++) {
      const k2 = (k + 1) % n;
      this.idx.push(low[k], high[k], high[k2]);
      this.idx.push(low[k], high[k2], low[k2]);
    }
  }

  /** Triangle fan from a centre vertex to a ring. `up` flips the winding. */
  fan(center, ring, up) {
    const n = ring.length;
    for (let k = 0; k < n; k++) {
      const k2 = (k + 1) % n;
      if (up) this.idx.push(center, ring[k2], ring[k]);
      else this.idx.push(center, ring[k], ring[k2]);
    }
  }

  closeGroup(materialIndex) {
    const count = this.idx.length - this.groupStart;
    if (count > 0) this.groups.push({ start: this.groupStart, count, materialIndex });
    this.groupStart = this.idx.length;
  }

  toGeometry() {
    const g = new THREE.BufferGeometry();
    g.setAttribute('position', new THREE.Float32BufferAttribute(this.pos, 3));
    g.setAttribute('color', new THREE.Float32BufferAttribute(this.col, 3));
    g.setIndex(this.idx);
    for (const gr of this.groups) g.addGroup(gr.start, gr.count, gr.materialIndex);
    g.computeVertexNormals();
    g.computeBoundingBox();
    g.computeBoundingSphere();
    return g;
  }
}

// ---------------------------------------------------------------------------

const ENAMEL_INCISAL = new THREE.Color(0xf3f6f8);
const ENAMEL_BODY = new THREE.Color(0xfbf7ee);
const ENAMEL_CERVICAL = new THREE.Color(0xf0e5cf);
const ROOT_CERVICAL = new THREE.Color(0xe8d9b4);
const ROOT_APICAL = new THREE.Color(0xd2bd90);

const _c = new THREE.Color();
function enamelColor(t) {
  // t: 0 at the cervix, 1 at the incisal / occlusal edge
  if (t < 0.45) return _c.copy(ENAMEL_CERVICAL).lerp(ENAMEL_BODY, smoothstep(t / 0.45));
  return _c.copy(ENAMEL_BODY).lerp(ENAMEL_INCISAL, smoothstep((t - 0.45) / 0.55) * 0.75);
}
function rootColor(s) {
  return _c.copy(ROOT_CERVICAL).lerp(ROOT_APICAL, smoothstep(s));
}

// ---------------------------------------------------------------------------

/**
 * Build a single tooth.
 *
 * The result is centred on the crown axis with y = 0 at the cervical line,
 * the crown growing towards +Y and the roots towards -Y.
 *
 * @param {object} spec  entry from {@link module:core/Anatomy.TOOTH_SPECS}
 * @param {object} [options]
 * @param {number} [options.segments=44]    radial resolution
 * @param {number} [options.crownRings=16]  vertical resolution of the crown wall
 * @param {number} [options.capRings=7]     concentric rings on the occlusal cap
 * @param {number} [options.rootRings=14]   rings per root
 * @param {number} [options.detail=1]       0 disables the micro relief
 * @returns {{geometry: THREE.BufferGeometry, meta: object}}
 */
export function buildToothGeometry(spec, options = {}) {
  const S = Math.max(12, options.segments ?? 44);
  const CR = Math.max(4, options.crownRings ?? 16);
  const CAP = Math.max(2, options.capRings ?? 7);
  const RR = Math.max(4, options.rootRings ?? 14);
  const detail = options.detail ?? 1;

  const b = new MeshBuilder();
  const n = spec.squareness;
  const cusps = spec.cusps || [];
  const maxCusp = maxCuspHeight(cusps);
  const capBaseY = spec.crownH - maxCusp;

  const [occA, occB] = crownSection(spec, 1);

  const angles = new Array(S);
  for (let k = 0; k < S; k++) angles[k] = (k / S) * TAU;

  // ---- crown side wall ----------------------------------------------------
  const wallRings = [];
  for (let i = 0; i <= CR; i++) {
    const t = i / CR;
    const [a, bb] = crownSection(spec, t);
    const col = enamelColor(t).clone();
    const ring = new Array(S);
    for (let k = 0; k < S; k++) {
      const th = angles[k];
      const r = sectionRadius(th, a, bb, n) * (detail ? surfaceNoise(th, t) : 1);
      const x = r * Math.cos(th);
      const z = r * Math.sin(th);
      let y = t * capBaseY;
      if (t > 0) {
        const nx = x / occA;
        const nz = z / occB;
        y += Math.pow(t, 3) * cuspField(nx, nz, cusps);
      }
      ring[k] = b.vertex(x, y, z, col);
    }
    wallRings.push(ring);
  }
  for (let i = 0; i < CR; i++) b.bridge(wallRings[i], wallRings[i + 1]);

  // ---- occlusal / incisal cap --------------------------------------------
  const rim = wallRings[CR];
  const rimPts = [];
  for (let k = 0; k < S; k++) {
    const th = angles[k];
    const r = sectionRadius(th, occA, occB, n);
    rimPts.push([r * Math.cos(th), r * Math.sin(th)]);
  }
  let prev = rim;
  for (let j = 1; j <= CAP; j++) {
    const rn = 1 - j / CAP;
    const col = enamelColor(1).clone();
    if (rn <= 1e-4) {
      const y = capBaseY + cuspField(0, 0, cusps);
      const c = b.vertex(0, y, 0, col);
      b.fan(c, prev, true);
      prev = null;
      break;
    }
    const ring = new Array(S);
    for (let k = 0; k < S; k++) {
      const x = rimPts[k][0] * rn;
      const z = rimPts[k][1] * rn;
      const y = capBaseY + cuspField(x / occA, z / occB, cusps);
      ring[k] = b.vertex(x, y, z, col);
    }
    // cap faces up: outer ring first keeps the +Y winding
    for (let k = 0; k < S; k++) {
      const k2 = (k + 1) % S;
      b.idx.push(prev[k], ring[k], ring[k2]);
      b.idx.push(prev[k], ring[k2], prev[k2]);
    }
    prev = ring;
  }

  // ---- cervical closing disc (hidden inside the root trunk) ---------------
  const [cvA, cvB] = crownSection(spec, 0);
  const cervicalCenter = b.vertex(0, 0, 0, ROOT_CERVICAL);
  b.fan(cervicalCenter, wallRings[0], false);

  b.closeGroup(0); // crown

  // ---- roots --------------------------------------------------------------
  const trunkFrac = 0.30;
  const TRUNK_INFLATE = 1.006;  // avoids z-fighting with the crown's cervical disc
  const overlap = Math.min(1.6, spec.crownH * 0.22);

  for (const root of spec.roots) {
    const ox = root.x * cvA;
    const oz = root.z * cvB;
    const ax = (root.x + root.tx) * cvA;
    const az = (root.z + root.tz) * cvB;
    // quadratic bezier: leaves the cervix vertically, then bends to the apex
    const p0 = [ox, 0, oz];
    const p1 = [ox, -root.len * 0.55, oz];
    const p2 = [ax, -root.len, az];

    const bezier = (s) => {
      const iu = 1 - s;
      return [
        iu * iu * p0[0] + 2 * iu * s * p1[0] + s * s * p2[0],
        iu * iu * p0[1] + 2 * iu * s * p1[1] + s * s * p2[1],
        iu * iu * p0[2] + 2 * iu * s * p1[2] + s * s * p2[2]
      ];
    };

    const rings = [];
    const S_END = 0.96;                       // last ring stops short of the apex
    for (let i = 0; i <= RR; i++) {
      const s = (i / RR) * S_END;
      let cx, cy, cz;
      if (i === 0) {
        cx = ox; cy = overlap; cz = oz;
      } else {
        [cx, cy, cz] = bezier(s);
      }
      // full through the middle third, then a short conical apex
      const taper = (1 - 0.45 * s * s) * Math.sqrt(Math.max(0, 1 - Math.pow(s, 8)));
      const blend = smoothstep(s / trunkFrac);
      const aTrunk = cvA * taper * TRUNK_INFLATE;
      const bTrunk = cvB * taper * TRUNK_INFLATE;
      const aRoot = root.rx * cvA * taper;
      const bRoot = root.rz * cvB * taper;
      const a = lerp(aTrunk, aRoot, blend);
      const bb = lerp(bTrunk, bRoot, blend);
      const px = lerp(0, cx, blend);
      const pz = lerp(0, cz, blend);
      const col = rootColor(s).clone();

      const ring = new Array(S);
      for (let k = 0; k < S; k++) {
        const th = angles[k];
        const rr = Math.max(1e-3, sectionRadius(th, Math.max(a, 1e-3), Math.max(bb, 1e-3), 2.2));
        ring[k] = b.vertex(px + rr * Math.cos(th), cy, pz + rr * Math.sin(th), col);
      }
      rings.push({ ring, y: cy, cx: px, cz: pz, col });
    }

    // top disc, sealed inside the crown
    const topCenter = b.vertex(rings[0].cx, rings[0].y, rings[0].cz, rings[0].col);
    b.fan(topCenter, rings[0].ring, true);

    for (let i = 0; i < RR; i++) b.bridge(rings[i + 1].ring, rings[i].ring);

    const last = rings[RR];
    const tip = bezier(1);
    const apex = b.vertex(tip[0], tip[1], tip[2], last.col);
    b.fan(apex, last.ring, false);
  }

  b.closeGroup(1); // root

  const geometry = b.toGeometry();
  const meta = {
    crownHeight: spec.crownH,
    occlusalY: spec.crownH,
    cervixY: 0,
    rootLength: Math.max(...spec.roots.map((r) => r.len)),
    crownHalfMd: spec.md * 0.5,
    crownHalfBl: spec.bl * 0.5,
    cervicalHalfMd: cvA,
    cervicalHalfBl: cvB,
    rootCount: spec.roots.length,
    roots: spec.roots.map((r) => ({
      origin: new THREE.Vector3(r.x * cvA, 0, r.z * cvB),
      apex: new THREE.Vector3((r.x + r.tx) * cvA, -r.len, (r.z + r.tz) * cvB),
      len: r.len
    }))
  };
  return { geometry, meta };
}

export { crownSection, cuspField, sectionRadius };

/**
 * Mirror a tooth geometry across the sagittal plane (local X).
 * Face winding is reversed so the surface still points outwards.
 * @param {THREE.BufferGeometry} geometry
 * @returns {THREE.BufferGeometry} a new geometry; the input is untouched
 */
export function mirrorGeometryX(geometry) {
  const g = geometry.clone();
  const pos = g.getAttribute('position');
  for (let i = 0; i < pos.count; i++) pos.setX(i, -pos.getX(i));
  pos.needsUpdate = true;
  const idx = g.getIndex();
  const arr = idx.array;
  for (let i = 0; i < arr.length; i += 3) {
    const t = arr[i + 1]; arr[i + 1] = arr[i + 2]; arr[i + 2] = t;
  }
  idx.needsUpdate = true;
  g.computeVertexNormals();
  g.computeBoundingBox();
  g.computeBoundingSphere();
  return g;
}

// ---------------------------------------------------------------------------
// surface patches (fillings, caries, veneers, sealants...)
// ---------------------------------------------------------------------------

const PATCH_ANGLE = { M: 0, B: Math.PI / 2, D: Math.PI, L: -Math.PI / 2 };

function gridNormals(pts, nu, nv) {
  const n = new Array(nu * nv);
  const a = new THREE.Vector3(), b = new THREE.Vector3();
  for (let j = 0; j < nv; j++) {
    for (let i = 0; i < nu; i++) {
      const k = j * nu + i;
      const iu = i < nu - 1 ? k + 1 : k - 1;
      const iv = j < nv - 1 ? k + nu : k - nu;
      a.subVectors(pts[iu], pts[k]); if (i === nu - 1) a.negate();
      b.subVectors(pts[iv], pts[k]); if (j === nv - 1) b.negate();
      n[k] = new THREE.Vector3().crossVectors(b, a).normalize();
    }
  }
  return n;
}

/**
 * Build a shell that follows the real tooth surface, so restorations wrap
 * around the crown instead of floating on it as a flat disc.
 *
 * Returned geometry is in the same local frame as {@link buildToothGeometry};
 * mirror it with {@link mirrorGeometryX} for mirrored teeth.
 *
 * @param {object} spec  entry from TOOTH_SPECS
 * @param {'M'|'D'|'B'|'L'|'O'} code
 * @param {object} [options]
 * @param {number} [options.scale=0.62]        fraction of the surface covered
 * @param {number} [options.longitudinal=0.62] fraction of the crown height (axial faces)
 * @param {number} [options.thickness=0.35]    shell thickness in mm
 * @param {number} [options.lift=0.05]         offset above the enamel in mm
 * @param {number} [options.centre=0.55]       crown-height centre of an axial patch
 * @returns {THREE.BufferGeometry}
 */
export function buildSurfacePatch(spec, code, options = {}) {
  const scale = THREE.MathUtils.clamp(options.scale ?? 0.62, 0.08, 1);
  const lon = THREE.MathUtils.clamp(options.longitudinal ?? 0.62, 0.08, 1);
  const thickness = options.thickness ?? 0.35;
  const lift = options.lift ?? 0.05;
  const n = spec.squareness;
  const cusps = spec.cusps || [];
  const capBaseY = spec.crownH - maxCuspHeight(cusps);
  const [occA, occB] = crownSection(spec, 1);

  const base = [];
  let nu, nv;

  if (code === 'O') {
    nu = 28; nv = 6;
    const rMax = scale;
    for (let j = 0; j < nv; j++) {
      const rho = rMax * ((j + 0.35) / (nv - 0.3));
      for (let i = 0; i < nu; i++) {
        const phi = (i / nu) * TAU;
        const r = sectionRadius(phi, occA, occB, n) * rho;
        const x = r * Math.cos(phi);
        const z = r * Math.sin(phi);
        base.push(new THREE.Vector3(x, capBaseY + cuspField(x / occA, z / occB, cusps), z));
      }
    }
    // closed in u (a ring), open in v
    const pts = base;
    const normals = [];
    for (let j = 0; j < nv; j++) {
      for (let i = 0; i < nu; i++) {
        const k = j * nu + i;
        const a = pts[j * nu + ((i + 1) % nu)].clone().sub(pts[k]);
        const jv = j < nv - 1 ? j + 1 : j - 1;
        const b = pts[jv * nu + i].clone().sub(pts[k]);
        if (j === nv - 1) b.negate();
        normals.push(new THREE.Vector3().crossVectors(b, a).normalize());
      }
    }
    orientNormals(normals, new THREE.Vector3(0, 1, 0));
    return assembleShell(pts, normals, nu, nv, thickness, lift, true);
  }

  nu = 18; nv = 14;
  const theta0 = PATCH_ANGLE[code];
  const halfAngle = Math.PI * 0.5 * scale * 0.66;
  const tc = THREE.MathUtils.clamp(options.centre ?? 0.55, 0.1, 0.9);
  const tHalf = lon * 0.45;
  const t0 = Math.max(0.02, tc - tHalf);
  const t1 = Math.min(0.985, tc + tHalf);

  for (let j = 0; j < nv; j++) {
    const v = lerp(-0.985, 0.985, j / (nv - 1));
    const t = lerp(t0, t1, (v + 1) / 2);
    // squeeze the angular span towards the ends so the outline reads as an
    // oval restoration instead of a rectangular sticker
    const squeeze = Math.pow(Math.max(0, 1 - Math.pow(Math.abs(v), 2.2)), 1 / 2.2);
    const [a, bb] = crownSection(spec, t);
    for (let i = 0; i < nu; i++) {
      const th = theta0 + lerp(-halfAngle, halfAngle, i / (nu - 1)) * squeeze;
      const r = sectionRadius(th, a, bb, n);
      const x = r * Math.cos(th);
      const z = r * Math.sin(th);
      let y = t * capBaseY;
      y += Math.pow(t, 3) * cuspField(x / occA, z / occB, cusps);
      base.push(new THREE.Vector3(x, y, z));
    }
  }
  const normals = gridNormals(base, nu, nv);
  orientNormals(normals, new THREE.Vector3(Math.cos(theta0), 0, Math.sin(theta0)));
  return assembleShell(base, normals, nu, nv, thickness, lift, false);
}

/** Force every grid normal onto the outward side of the surface. */
function orientNormals(normals, reference) {
  for (const n of normals) if (n.dot(reference) < 0) n.negate();
}

/** Turn a point grid into a closed two-sided shell with a rim. */
function assembleShell(pts, normals, nu, nv, thickness, lift, closedU) {
  const pos = [];
  const push = (p) => pos.push(p.x, p.y, p.z);
  const count = pts.length;

  for (let k = 0; k < count; k++) push(pts[k].clone().addScaledVector(normals[k], lift + thickness));
  for (let k = 0; k < count; k++) push(pts[k].clone().addScaledVector(normals[k], lift - thickness * 0.35));

  const idx = [];
  const quad = (a, b, c, d) => { idx.push(a, b, c, a, c, d); };
  const uMax = closedU ? nu : nu - 1;

  for (let j = 0; j < nv - 1; j++) {
    for (let i = 0; i < uMax; i++) {
      const i2 = (i + 1) % nu;
      const a = j * nu + i, b = j * nu + i2, c = (j + 1) * nu + i2, d = (j + 1) * nu + i;
      quad(a, b, c, d);                                       // outer
      quad(count + a, count + d, count + c, count + b);       // inner
    }
  }

  // rim
  if (!closedU) {
    for (let j = 0; j < nv - 1; j++) {
      const l0 = j * nu, l1 = (j + 1) * nu;
      quad(l0, count + l0, count + l1, l1);
      const r0 = j * nu + nu - 1, r1 = (j + 1) * nu + nu - 1;
      quad(r1, count + r1, count + r0, r0);
    }
  }
  for (let i = 0; i < uMax; i++) {
    const i2 = (i + 1) % nu;
    const b0 = i, b1 = i2;
    quad(b1, count + b1, count + b0, b0);
    const t0 = (nv - 1) * nu + i, t1 = (nv - 1) * nu + i2;
    quad(t0, count + t0, count + t1, t1);
  }

  const g = new THREE.BufferGeometry();
  g.setAttribute('position', new THREE.Float32BufferAttribute(pos, 3));
  g.setIndex(idx);
  g.computeVertexNormals();
  ensureOutward(g);
  g.computeBoundingBox();
  g.computeBoundingSphere();
  return g;
}


```

## Source file: src/index.js

{
  "path": "src/index.js",
  "blob_sha": "53b06e05afe0cc59160a7358ceefc52809051e4b",
  "sha256": "550db797cb7870204251d0f1393b310ebbf8a24fb7ff5936f3fe238bbaa5bd82",
  "lines": 27,
  "bytes": 1001,
  "selection_score": 0,
  "keyword_line_counts": {
    "teeth": 2,
    "tooth": 3,
    "face": 3
  },
  "url": "https://github.com/emrecaglar/teeth/blob/e0f1417a1e9f7d58480bde9a69f9833d87f4d08f/src/index.js",
  "function_signature_candidates": []
}

```text

/**
 * dental3d - public entry point.
 * @module dental3d
 */
export { DentalChart } from './DentalChart.js';

export {
  MODEL_TYPES, FDI_TEETH, FDI_TEETH_PRIMARY, QUADRANTS, POSITIONS,
  TOOTH_SPECS, DENTITION_SPECS,
  FDI_TO_UNIVERSAL, FDI_TO_UNIVERSAL_PRIMARY, UNIVERSAL_TO_FDI,
  getTeeth, dentitionOf, describeTooth, formatToothNumber, toFdi, isValidFdi, getSpec
} from './core/Anatomy.js';

export {
  TREATMENTS, CATEGORIES, SURFACES,
  registerTreatment, getTreatment, listTreatments,
  normalizeTreatment, normalizeSurface
} from './core/Treatments.js';

export { MATERIAL_PRESETS, MaterialLibrary } from './core/Materials.js';
export { buildToothGeometry, mirrorGeometryX, buildSurfacePatch } from './core/ToothGeometry.js';
export { buildLayout, outwardNormal } from './core/ArchLayout.js';
export { buildGingiva } from './core/Gingiva.js';
export { EventEmitter } from './core/EventEmitter.js';
export { STRINGS, createTranslator } from './core/i18n.js';

export const VERSION = '1.0.0';


```

## Source file: src/core/i18n.js

{
  "path": "src/core/i18n.js",
  "blob_sha": "7eaf57df082a458fb770e4340710d72276c13e02",
  "sha256": "4d22b570daae94ef9159f67ac8ef4db1e99e799e8caaa07f99ae679be391a5ca",
  "lines": 54,
  "bytes": 1417,
  "selection_score": 0,
  "keyword_line_counts": {
    "jaw": 2,
    "tooth": 2,
    "face": 2
  },
  "url": "https://github.com/emrecaglar/teeth/blob/e0f1417a1e9f7d58480bde9a69f9833d87f4d08f/src/core/i18n.js",
  "function_signature_candidates": []
}

```text

/**
 * UI strings for the built-in context menu and tooltip.
 * Extend or replace with `new DentalChart(el, { locale: 'en', strings: {...} })`.
 * @module core/i18n
 */

export const STRINGS = {
  tr: {
    tooth: 'Diş',
    treatments: 'Tedaviler',
    surfaces: 'Yüzeyler',
    noTreatment: 'Tedavi yok',
    clear: 'Temizle',
    close: 'Kapat',
    status: 'Durum',
    statusExisting: 'Mevcut',
    statusPlanned: 'Planlanan',
    statusCompleted: 'Tamamlandı',
    focus: 'Yakınlaş',
    notes: 'Not',
    search: 'Tedavi ara...',
    applied: 'uygulandı',
    upper: 'Üst çene',
    lower: 'Alt çene'
  },
  en: {
    tooth: 'Tooth',
    treatments: 'Treatments',
    surfaces: 'Surfaces',
    noTreatment: 'No treatment',
    clear: 'Clear',
    close: 'Close',
    status: 'Status',
    statusExisting: 'Existing',
    statusPlanned: 'Planned',
    statusCompleted: 'Completed',
    focus: 'Focus',
    notes: 'Note',
    search: 'Search treatment...',
    applied: 'applied',
    upper: 'Upper jaw',
    lower: 'Lower jaw'
  }
};

/** Build a translator for a locale, with optional per-instance overrides. */
export function createTranslator(locale = 'tr', overrides = {}) {
  const base = STRINGS[locale] || STRINGS.en;
  const table = { ...STRINGS.en, ...base, ...overrides };
  const t = (key) => table[key] ?? key;
  t.locale = STRINGS[locale] ? locale : 'en';
  t.table = table;
  return t;
}


```

## Source file: src/DentalChart.js

{
  "path": "src/DentalChart.js",
  "blob_sha": "c0a5a7de338c4f983da21f3bcd476990c1c9cae4",
  "sha256": "bc377a6400b2d8cc5de38a934e3c8f8581d04d8898e03a722938c92d6d75a469",
  "lines": 1396,
  "bytes": 51323,
  "selection_score": 0,
  "keyword_line_counts": {
    "jaw": 12,
    "mouth": 2,
    "teeth": 34,
    "tooth": 62,
    "gum": 17,
    "rig": 10,
    "face": 17,
    "lip": 1
  },
  "url": "https://github.com/emrecaglar/teeth/blob/e0f1417a1e9f7d58480bde9a69f9833d87f4d08f/src/DentalChart.js",
  "function_signature_candidates": []
}

```text

/**
 * DentalChart - interactive 3D dentition.
 *
 * Renders either the permanent dentition (32 teeth, FDI 11..48) or the primary
 * one (20 teeth, FDI 51..85), in the anatomical arch or as two straight rows.
 *
 * ```js
 * import { DentalChart } from './src/index.js';
 *
 * const chart = new DentalChart(document.getElementById('viewer'), {
 *   modelType: 'permanent',   // or 'primary'
 *   layoutMode: 'anatomic',   // or 'flat'
 *   teeth: { 16: ['crown_gold'], 36: ['implant'] }
 * });
 * chart.on('toothClick', ({ fdi }) => console.log(fdi));
 * ```
 *
 * @module DentalChart
 */
import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';

import { EventEmitter } from './core/EventEmitter.js';
import {
  getTeeth, describeTooth, isValidFdi, toFdi, formatToothNumber, QUADRANTS
} from './core/Anatomy.js';
import { buildToothGeometry, mirrorGeometryX, buildSurfacePatch } from './core/ToothGeometry.js';
import { buildLayout } from './core/ArchLayout.js';
import { MaterialLibrary } from './core/Materials.js';
import { buildGingiva } from './core/Gingiva.js';
import { createStudioEnvironment } from './core/Environment.js';
import { createTranslator } from './core/i18n.js';
import { ContextMenu } from './core/ContextMenu.js';
import {
  TREATMENTS, getTreatment as getTreatmentDef, normalizeTreatment,
  disposeTreatmentGeometries, SURFACES
} from './core/Treatments.js';

const STATE_VERSION = 1;

// `pos` is a *direction* from the target; it gets normalised and scaled by the
// requested distance.
const VIEW_PRESETS = {
  front:   { pos: [0, 0.18, 1], up: [0, 1, 0] },
  back:    { pos: [0, 0.18, -1], up: [0, 1, 0] },
  right:   { pos: [-1, 0.14, 0.10], up: [0, 1, 0] },
  left:    { pos: [1, 0.14, 0.10], up: [0, 1, 0] },
  top:     { pos: [0, 1, 0.04], up: [0, 0, -1] },
  bottom:  { pos: [0, -1, 0.04], up: [0, 0, 1] },
  upper:   { pos: [0, -1, 0.04], up: [0, 0, 1], arch: 'upper' },
  lower:   { pos: [0, 1, 0.04], up: [0, 0, -1], arch: 'lower' },
  default: { pos: [-0.38, 0.26, 0.89], up: [0, 1, 0] }
};

/**
 * Temporomandibular hinge, in world millimetres. The mandible rotates about an
 * X axis through this point, which is why opening the jaw swings the chin down
 * *and* back the way a real mouth does.
 */
const HINGE = {
  permanent: { y: 40, z: -75 },
  primary: { y: 27, z: -51 }
};

const MAX_JAW_OPENING = 35;

const DEFAULTS = {
  locale: 'tr',
  numbering: 'fdi',
  theme: 'dark',
  background: null,
  contextMenu: true,
  tooltip: true,
  modelType: 'permanent',   // 'permanent' | 'primary'
  layoutMode: 'anatomic',   // 'anatomic' | 'flat'
  showGums: false,
  showLabels: false,
  arch: 'both',
  archSeparation: 0,
  jawOpening: 0,            // degrees of hinge rotation, 0..35
  hinge: null,              // { y, z } override
  rowGap: 'auto',           // distance between the two biting planes in flat mode
  autoRotate: false,
  cameraDistance: 'auto',
  minDistance: 'auto',
  maxDistance: 'auto',
  pixelRatioCap: 2,
  quality: {},
  materials: {},
  strings: {},
  teeth: null
};

let _liveCharts = 0;

const _v1 = new THREE.Vector3();
const _v2 = new THREE.Vector3();

/**
 * @fires DentalChart#ready
 * @fires DentalChart#toothClick
 * @fires DentalChart#toothHover
 * @fires DentalChart#change
 * @fires DentalChart#selectionChange
 */
export class DentalChart extends EventEmitter {
  /**
   * @param {HTMLElement} container  a sized, non-static element
   * @param {object} [options] see {@link DEFAULTS}
   */
  constructor(container, options = {}) {
    super();
    if (!container || !container.appendChild) throw new Error('[dental3d] a DOM container is required');

    this.container = container;
    this.options = { ...DEFAULTS, ...options };
    this.numbering = this.options.numbering;
    this.modelType = this.options.modelType === 'primary' ? 'primary' : 'permanent';
    this.layoutMode = this.options.layoutMode === 'flat' ? 'flat' : 'anatomic';
    this.options.modelType = this.modelType;
    this.options.layoutMode = this.layoutMode;
    this.teethList = getTeeth(this.modelType);
    this.t = createTranslator(this.options.locale, this.options.strings);

    /** @type {Map<number, object>} */
    this.views = new Map();
    /** @type {Map<number, object>} */
    this.state = new Map();
    /** @type {Set<number>} */
    this.selection = new Set();

    /** Treatment state is kept per dentition so switching model type is lossless. */
    this._savedStates = {};

    this._disposed = false;
    this._needsRender = true;
    this._hovered = null;
    this._camAnim = null;
    this._suspend = 0;
    _liveCharts++;

    this._initRenderer();
    this._initScene();
    this._buildModel();
    this._initInteraction();

    if (this.options.contextMenu) this.menu = new ContextMenu(container, this, { theme: this.options.theme });
    if (this.options.tooltip) this._initTooltip();

    this.setArch(this.options.arch);
    this.setGumsVisible(this.options.showGums);
    this.setLabelsVisible(this.options.showLabels);
    this._updateJawTransform();
    if (this.options.teeth) this.setState(this.options.teeth, { silent: true });

    this.setView('default', { instant: true });
    this._loop = this._loop.bind(this);
    this._raf = requestAnimationFrame(this._loop);
    queueMicrotask(() => this.emit('ready', { chart: this }));
  }

  // -------------------------------------------------------------------------
  // setup
  // -------------------------------------------------------------------------

  _initRenderer() {
    const { clientWidth: w, clientHeight: h } = this.container;
    const renderer = new THREE.WebGLRenderer({
      antialias: true,
      alpha: this.options.background === null,
      powerPreference: 'high-performance'
    });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, this.options.pixelRatioCap));
    renderer.setSize(Math.max(1, w), Math.max(1, h), false);
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.02;
    renderer.outputColorSpace = THREE.SRGBColorSpace;

    const canvas = renderer.domElement;
    canvas.style.cssText = 'display:block;width:100%;height:100%;touch-action:none;outline:none';
    if (getComputedStyle(this.container).position === 'static') this.container.style.position = 'relative';
    this.container.appendChild(canvas);
    this.renderer = renderer;
    this.canvas = canvas;
  }

  _initScene() {
    const scene = new THREE.Scene();
    if (this.options.background !== null) scene.background = new THREE.Color(this.options.background);

    const env = createStudioEnvironment(this.renderer);
    scene.environment = env.texture;
    this._env = env;

    scene.add(new THREE.HemisphereLight(0xffffff, 0x39404a, 0.55));
    const key = new THREE.DirectionalLight(0xffffff, 1.5);
    key.position.set(-60, 90, 110);
    scene.add(key);
    const fill = new THREE.DirectionalLight(0xdce8f5, 0.55);
    fill.position.set(90, 20, -60);
    scene.add(fill);
    const rim = new THREE.DirectionalLight(0xffffff, 0.4);
    rim.position.set(0, -80, -70);
    scene.add(rim);

    const camera = new THREE.PerspectiveCamera(34, this._aspect(), 1, 2500);
    scene.add(camera);

    const controls = new OrbitControls(camera, this.canvas);
    controls.enableDamping = true;
    controls.dampingFactor = 0.085;
    controls.minDistance = this.options.minDistance;
    controls.maxDistance = this.options.maxDistance;
    controls.autoRotate = this.options.autoRotate;
    controls.autoRotateSpeed = 0.9;
    controls.addEventListener('change', () => { this._needsRender = true; });

    this.root = new THREE.Group();
    this.archGroups = { upper: new THREE.Group(), lower: new THREE.Group() };
    this.gumGroups = { upper: new THREE.Group(), lower: new THREE.Group() };
    this.labelGroups = { upper: new THREE.Group(), lower: new THREE.Group() };
    this.root.add(this.archGroups.upper, this.archGroups.lower);
    for (const arch of ['upper', 'lower']) {
      this.labelGroups[arch].visible = false;
      this.archGroups[arch].add(this.gumGroups[arch], this.labelGroups[arch]);
    }
    this.bridgeGroup = new THREE.Group();
    this.wireGroup = new THREE.Group();
    this.root.add(this.bridgeGroup, this.wireGroup);
    scene.add(this.root);

    this.scene = scene;
    this.camera = camera;
    this.controls = controls;
    this.materials = new MaterialLibrary(this.options.materials);

    this._patchCache = new Map();
    this._raycaster = new THREE.Raycaster();
    this._probe = new THREE.Mesh(undefined, new THREE.MeshBasicMaterial());
    this._probe.matrixAutoUpdate = false;
  }

  _aspect() {
    const w = Math.max(1, this.container.clientWidth);
    const h = Math.max(1, this.container.clientHeight);
    return w / h;
  }

  /** Build teeth + gingiva for the current model type and layout mode. */
  _buildModel() {
    this.teethList = getTeeth(this.modelType);
    this._buildTeeth();
    this._buildGums();
    this._updateControlLimits();
  }

  /** Release everything the current model owns, ready for a rebuild. */
  _teardownModel() {
    const seen = new Set();
    for (const view of this.views.values()) {
      if (view.group.parent) view.group.parent.remove(view.group);
      if (!seen.has(view.mesh.geometry)) { seen.add(view.mesh.geometry); view.mesh.geometry.dispose(); }
      this.materials.release(view.crownMaterial);
      this.materials.release(view.rootMaterial);
      for (const m of view.ownedMaterials) this.materials.release(m);
      view.ownedMaterials.clear();
      if (view.outline) view.outline.material.dispose();
      if (view.label) {
        view.label.texture.dispose();
        view.label.sprite.material.dispose();
        if (view.label.sprite.parent) view.label.sprite.parent.remove(view.label.sprite);
      }
    }
    this.views.clear();
    for (const arch of ['upper', 'lower']) {
      for (const c of [...this.gumGroups[arch].children]) {
        c.geometry.dispose();
        this.materials.release(c.material);
        this.gumGroups[arch].remove(c);
      }
    }
    for (const g of [this.bridgeGroup, this.wireGroup]) {
      for (const c of [...g.children]) { c.geometry.dispose(); g.remove(c); }
    }
    for (const g of this._patchCache.values()) g.dispose();
    this._patchCache.clear();
    this._hovered = null;
  }

  /** Tear the model down and build it again from the current options. */
  _rebuildModel({ keepCamera = false } = {}) {
    if (this.menu) this.menu.close();
    this._teardownModel();
    this._buildModel();
    this.setArch(this.options.arch);
    this.setGumsVisible(this.options.showGums);
    this.setLabelsVisible(this.options.showLabels);
    this._updateJawTransform();
    for (const fdi of this.teethList) this._refresh(fdi);
    this.root.updateMatrixWorld(true);
    this._rebuildLinks();
    if (keepCamera) this.fit({ instant: true }); else this.setView('default', { instant: true });
    this._needsRender = true;
  }

  _buildTeeth() {
    const layout = buildLayout(this.options);
    this.layout = layout;

    const geomCache = new Map();
    const q = this.options.quality;

    for (const fdi of this.teethList) {
      const info = layout.teeth.get(fdi);
      const key = `${this.modelType}:${info.info.arch}:${info.info.position}`;
      if (!geomCache.has(key)) geomCache.set(key, buildToothGeometry(info.spec, q));
      const base = geomCache.get(key);
      const geometry = info.mirrored ? mirrorGeometryX(base.geometry) : base.geometry;

      const crownMaterial = this.materials.create('enamel');
      const rootMaterial = this.materials.create('dentin');

      const mesh = new THREE.Mesh(geometry, [crownMaterial, rootMaterial]);
      mesh.userData.fdi = fdi;
      mesh.name = `tooth-${fdi}`;

      const inner = new THREE.Group();
      inner.add(mesh);
      const decor = new THREE.Group();
      inner.add(decor);

      const group = new THREE.Group();
      group.position.copy(info.position);
      group.quaternion.copy(info.quaternion);
      group.add(inner);
      this.archGroups[info.info.arch].add(group);

      const view = {
        fdi,
        info: info.info,
        spec: info.spec,
        meta: base.meta,
        layout: info,
        mirrored: info.mirrored,
        group, inner, mesh, decor,
        outline: null,
        crownMaterial, rootMaterial,
        wirePoints: [],
        ownedMaterials: new Set()
      };
      this.views.set(fdi, view);
      if (!this.state.has(fdi)) this.state.set(fdi, { treatments: [] });

      this._makeLabel(view);
    }

    this._toothMeshes = [...this.views.values()].map((v) => v.mesh);
  }

  _makeLabel(view) {
    const canvas = document.createElement('canvas');
    canvas.width = 128; canvas.height = 72;
    const ctx = canvas.getContext('2d');
    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    const sprite = new THREE.Sprite(new THREE.SpriteMaterial({
      map: texture, depthTest: false, transparent: true, sizeAttenuation: false
    }));
    sprite.scale.set(0.055, 0.031, 1);
    sprite.renderOrder = 10;

    const p = view.layout.position.clone()
      .addScaledVector(view.layout.buccal, view.spec.bl * 0.5 + 5.5)
      .addScaledVector(view.layout.occlusal, view.spec.crownH * 0.45);
    sprite.position.copy(p);
    this.labelGroups[view.info.arch].add(sprite);

    view.label = { sprite, canvas, ctx, texture };
    this._paintLabel(view);
  }

  _paintLabel(view) {
    const { ctx, canvas, texture } = view.label;
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.font = 'bold 46px system-ui, sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.lineWidth = 7;
    ctx.strokeStyle = 'rgba(0,0,0,.72)';
    ctx.fillStyle = '#fff';
    const text = formatToothNumber(view.fdi, this.numbering);
    ctx.strokeText(text, canvas.width / 2, canvas.height / 2);
    ctx.fillText(text, canvas.width / 2, canvas.height / 2);
    texture.needsUpdate = true;
  }

  _buildGums() {
    for (const arch of ['upper', 'lower']) {
      const teeth = [...this.layout.teeth.values()].filter((t) => t.info.arch === arch);
      const geom = buildGingiva(arch, this.layout.curves[arch], this.layout.lengths[arch], teeth,
        { ...(this.options.gingiva || {}), flat: this.layout.flat });
      const mesh = new THREE.Mesh(geom, this.materials.create('gum'));
      mesh.name = `gingiva-${arch}`;
      mesh.userData.gingiva = true;
      this.gumGroups[arch].add(mesh);
    }
  }

  // -------------------------------------------------------------------------
  // interaction
  // -------------------------------------------------------------------------

  _initInteraction() {
    const canvas = this.canvas;
    let down = null;

    this._onPointerDown = (e) => { down = { x: e.clientX, y: e.clientY, t: performance.now() }; };

    this._onPointerUp = (e) => {
      if (!down) return;
      const moved = Math.hypot(e.clientX - down.x, e.clientY - down.y);
      const quick = performance.now() - down.t < 700;
      down = null;
      if (moved > 5 || !quick) return;
      const hit = this._pick(e);
      const local = this._localPoint(e);
      if (hit) {
        const payload = { fdi: hit.fdi, tooth: this.getTooth(hit.fdi), point: hit.point, event: e, screen: local };
        this.emit('toothClick', payload);
        if (this.options.contextMenu) {
          this.setSelection([hit.fdi]);
          this.menu.open(hit.fdi, local);
        }
      } else {
        this.emit('backgroundClick', { event: e });
        if (this.menu) this.menu.close();
        if (this.options.contextMenu) this.setSelection([]);
      }
    };

    this._onPointerMove = (e) => {
      const hit = this._pick(e);
      const fdi = hit ? hit.fdi : null;
      if (fdi !== this._hovered) {
        const prev = this._hovered;
        this._hovered = fdi;
        if (prev != null) { this._applyOutline(this.views.get(prev)); this.emit('toothOut', { fdi: prev }); }
        if (fdi != null) { this._applyOutline(this.views.get(fdi)); this.emit('toothHover', { fdi, tooth: this.getTooth(fdi) }); }
        canvas.style.cursor = fdi != null ? 'pointer' : '';
        this._needsRender = true;
      }
      if (this._tooltip) this._updateTooltip(e);
    };

    this._onPointerLeave = () => {
      if (this._hovered != null) {
        const prev = this._hovered;
        this._hovered = null;
        this._applyOutline(this.views.get(prev));
        this.emit('toothOut', { fdi: prev });
        this._needsRender = true;
      }
      if (this._tooltip) this._tooltip.hidden = true;
    };

    canvas.addEventListener('pointerdown', this._onPointerDown);
    canvas.addEventListener('pointerup', this._onPointerUp);
    canvas.addEventListener('pointermove', this._onPointerMove);
    canvas.addEventListener('pointerleave', this._onPointerLeave);

    this._resizeObserver = new ResizeObserver(() => this.resize());
    this._resizeObserver.observe(this.container);
  }

  _localPoint(e) {
    const r = this.canvas.getBoundingClientRect();
    return { x: e.clientX - r.left, y: e.clientY - r.top };
  }

  _pick(e) {
    const r = this.canvas.getBoundingClientRect();
    const ndc = new THREE.Vector2(
      ((e.clientX - r.left) / r.width) * 2 - 1,
      -((e.clientY - r.top) / r.height) * 2 + 1
    );
    this._raycaster.setFromCamera(ndc, this.camera);
    const candidates = this._toothMeshes.filter(
      (m) => m.visible && this.archGroups[this.views.get(m.userData.fdi).info.arch].visible
    );
    const hits = this._raycaster.intersectObjects(candidates, false);
    for (const h of hits) {
      const fdi = h.object.userData.fdi;
      const st = this.state.get(fdi);
      if (st && st.interactive === false) continue;
      return { fdi, point: h.point.clone(), object: h.object };
    }
    return null;
  }

  _initTooltip() {
    const el = document.createElement('div');
    el.hidden = true;
    el.style.cssText = `position:absolute;pointer-events:none;z-index:15;padding:6px 9px;border-radius:8px;
      font:12px/1.4 system-ui,sans-serif;max-width:220px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;
      background:rgba(20,24,30,.92);color:#eef2f7;box-shadow:0 6px 20px rgba(0,0,0,.35)`;
    this.container.appendChild(el);
    this._tooltip = el;
  }

  _updateTooltip(e) {
    const el = this._tooltip;
    if (this._hovered == null) { el.hidden = true; return; }
    const fdi = this._hovered;
    const info = describeTooth(fdi, this.t.locale);
    const st = this.state.get(fdi);
    const names = (st.treatments || []).map((r) => {
      const def = getTreatmentDef(r.type);
      const label = def ? (def.label[this.t.locale] || def.label.en) : r.type;
      return r.surfaces && r.surfaces.length ? `${label} (${r.surfaces.join('')})` : label;
    });
    el.innerHTML = `<b>${formatToothNumber(fdi, this.numbering)}</b> &middot; ${info.name}` +
      (names.length ? `<br><span style="opacity:.75">${names.join(', ')}</span>` : '');
    const p = this._localPoint(e);
    el.hidden = false;
    el.style.left = `${Math.min(p.x + 14, this.container.clientWidth - el.offsetWidth - 6)}px`;
    el.style.top = `${Math.max(4, p.y - el.offsetHeight - 10)}px`;
  }

  // -------------------------------------------------------------------------
  // rendering
  // -------------------------------------------------------------------------

  _loop() {
    if (this._disposed) return;
    this._raf = requestAnimationFrame(this._loop);
    const moving = this.controls.update() || this.controls.autoRotate;
    if (this._camAnim) this._stepCameraAnimation();
    if (moving || this._needsRender || this._camAnim) {
      this._needsRender = false;
      this.renderer.render(this.scene, this.camera);
    }
  }

  /** Force a redraw on the next frame. */
  invalidate() { this._needsRender = true; return this; }

  /** Re-read the container size. Called automatically by a ResizeObserver. */
  resize() {
    const w = Math.max(1, this.container.clientWidth);
    const h = Math.max(1, this.container.clientHeight);
    this.renderer.setSize(w, h, false);
    this.camera.aspect = w / h;
    this.camera.updateProjectionMatrix();
    this._updateControlLimits();
    this._needsRender = true;
    return this;
  }

  // -------------------------------------------------------------------------
  // tooth presentation
  // -------------------------------------------------------------------------

  _releaseOwned(view) {
    for (const m of view.ownedMaterials) this.materials.release(m);
    view.ownedMaterials.clear();
  }

  /**
   * Rebuild one tooth from its state. Always full-replay, never incremental,
   * so add/remove/reorder can never drift out of sync.
   */
  _refresh(fdi) {
    const view = this.views.get(fdi);
    if (!view) return;
    const st = this.state.get(fdi);

    this._releaseOwned(view);
    view.mesh.visible = true;
    view.mesh.material = [view.crownMaterial, view.rootMaterial];
    view.inner.position.set(0, 0, 0);
    view.inner.rotation.set(0, 0, 0);
    view.decor.clear();
    view.wirePoints = [];
    view.bridge = false;

    const records = [...(st.treatments || [])].sort(
      (a, b) => (getTreatmentDef(a.type)?.order ?? 50) - (getTreatmentDef(b.type)?.order ?? 50)
    );

    for (const rec of records) {
      const def = getTreatmentDef(rec.type);
      if (!def) continue;
      if (def.bridge) view.bridge = true;
      try {
        def.apply(this._makeContext(view, rec, def));
      } catch (err) {
        console.error(`[dental3d] treatment "${rec.type}" failed on tooth ${fdi}:`, err);
      }
    }

    this._applyOutline(view);
    this._needsRender = true;
  }

  _makeContext(view, rec, def) {
    const planned = rec.status === 'planned';
    const chart = this;
    return {
      chart, THREE,
      fdi: view.fdi,
      info: view.info,
      spec: view.spec,
      meta: view.meta,
      mirrored: view.mirrored,
      record: rec,
      opt: rec,
      planned,
      definition: def,

      material(name, extra) {
        if (extra) {
          const m = chart.materials.create(name, extra);
          view.ownedMaterials.add(m);
          return m;
        }
        return planned ? chart.materials.planned(name) : chart.materials.get(name);
      },
      setCrownMaterial(m) { view.mesh.material[0] = m; },
      setRootMaterial(m) { view.mesh.material[1] = m; },
      hide() { view.mesh.visible = false; },
      show() { view.mesh.visible = true; },
      ghost() {
        const g = chart.materials.get('ghost');
        view.mesh.visible = true;
        view.mesh.material = [g, g];
      },
      add(obj) { view.decor.add(obj); },
      transform({ offset, tiltX = 0, tiltZ = 0, rotateY = 0 } = {}) {
        if (offset) view.inner.position.set(offset[0] || 0, offset[1] || 0, offset[2] || 0);
        view.inner.rotation.set(tiltX * Math.PI / 180, rotateY * Math.PI / 180, tiltZ * Math.PI / 180);
      },
      surfaceHit: (code) => chart._surfaceHit(view, code),
      patch: (code, params) => chart._patch(view, code, params),
      registerWirePoint(point, kind = 'arch') { view.wirePoints.push({ point, kind }); }
    };
  }

  /** Ray-cast the tooth surface from outside along a surface direction. */
  _surfaceHit(view, code) {
    const surf = SURFACES[code];
    if (!surf) return null;
    const dir = new THREE.Vector3(...surf.dir);
    if (view.mirrored && (code === 'M' || code === 'D')) dir.x *= -1;

    const base = code === 'O'
      ? new THREE.Vector3(0, 0, 0)
      : new THREE.Vector3(0, view.spec.crownH * 0.55, 0);
    const origin = base.clone().addScaledVector(dir, 80);

    this._probe.geometry = view.mesh.geometry;
    this._raycaster.set(origin, dir.clone().negate());
    this._raycaster.near = 0;
    this._raycaster.far = Infinity;
    const hits = this._raycaster.intersectObject(this._probe, false);
    if (!hits.length) return null;
    const hit = hits[0];
    const normal = hit.face ? hit.face.normal.clone() : dir.clone();
    return { point: hit.point.clone(), normal: normal.normalize(), direction: dir };
  }

  /**
   * Place a restoration on a tooth surface. The shell is generated from the
   * same parametric surface the tooth itself is built from, so it wraps the
   * crown exactly instead of floating on it. Results are cached per shape.
   */
  _patch(view, code, params = {}) {
    if (!SURFACES[code]) return null;
    const opts = {
      scale: params.scale ?? 0.5,
      longitudinal: params.longitudinal ?? 0.55,
      thickness: params.thickness ?? 0.26,
      lift: params.lift ?? 0.04,
      centre: params.centre ?? 0.55
    };
    const key = `${this.modelType}:${view.info.arch}:${view.info.position}:${code}:${view.mirrored}:` +
      `${opts.scale}:${opts.longitudinal}:${opts.thickness}:${opts.lift}:${opts.centre}`;

    let geom = this._patchCache.get(key);
    if (!geom) {
      geom = buildSurfacePatch(view.spec, code, opts);
      if (view.mirrored) {
        const mirrored = mirrorGeometryX(geom);
        geom.dispose();
        geom = mirrored;
      }
      this._patchCache.set(key, geom);
    }
    const mesh = new THREE.Mesh(geom, params.material || this.materials.get('composite'));
    view.decor.add(mesh);
    return mesh;
  }

  _applyOutline(view) {
    const selected = this.selection.has(view.fdi);
    const hovered = this._hovered === view.fdi;
    const custom = view.highlightColor;
    const want = custom || (selected ? 0xffb454 : hovered ? 0x5fb0ff : null);

    if (!want) { if (view.outline) view.outline.visible = false; return; }
    if (!view.outline) {
      const mat = new THREE.MeshBasicMaterial({ color: want, side: THREE.BackSide, transparent: true, opacity: 0.9 });
      const outline = new THREE.Mesh(view.mesh.geometry, mat);
      const box = view.mesh.geometry.boundingBox;
      const centre = box.getCenter(new THREE.Vector3());
      const s = 1.045;
      outline.scale.setScalar(s);
      outline.position.copy(centre).multiplyScalar(1 - s);
      outline.renderOrder = -1;
      view.inner.add(outline);
      view.outline = outline;
    }
    view.outline.material.color.set(want);
    view.outline.visible = view.mesh.visible;
  }

  /** Rebuild bridge connectors and orthodontic wires across teeth. */
  _rebuildLinks() {
    for (const g of [this.bridgeGroup, this.wireGroup]) {
      for (const c of [...g.children]) { c.geometry.dispose(); g.remove(c); }
    }

    for (const arch of ['upper', 'lower']) {
      const ordered = [...this.views.values()]
        .filter((v) => v.info.arch === arch)
        .map((v) => ({ v, sd: v.info.side === 'right' ? v.layout.archDistance : -v.layout.archDistance }))
        .sort((a, b) => a.sd - b.sd);

      // bridge connectors between neighbouring bridge units
      for (let i = 0; i < ordered.length - 1; i++) {
        const a = ordered[i].v, b = ordered[i + 1].v;
        if (!a.bridge || !b.bridge) continue;
        const pa = a.group.localToWorld(_v1.set(0, a.spec.crownH * 0.45, 0)).clone();
        const pb = b.group.localToWorld(_v2.set(0, b.spec.crownH * 0.45, 0)).clone();
        this.root.worldToLocal(pa); this.root.worldToLocal(pb);
        const dir = pb.clone().sub(pa);
        const len = dir.length();
        const bar = new THREE.Mesh(
          new THREE.CylinderGeometry(1.5, 1.5, len * 1.05, 12),
          this.materials.get('pfm')
        );
        bar.position.copy(pa).addScaledVector(dir, 0.5);
        bar.quaternion.setFromUnitVectors(_v1.set(0, 1, 0), dir.normalize());
        this.bridgeGroup.add(bar);
      }

      // orthodontic wires, one polyline per registered kind
      const kinds = new Map();
      for (const { v } of ordered) {
        if (!v.mesh.visible) continue;
        for (const wp of v.wirePoints) {
          if (!kinds.has(wp.kind)) kinds.set(wp.kind, []);
          const p = v.inner.localToWorld(wp.point.clone());
          kinds.get(wp.kind).push(this.root.worldToLocal(p));
        }
      }
      for (const [kind, pts] of kinds) {
        if (pts.length < 2) continue;
        const curve = new THREE.CatmullRomCurve3(pts, false, 'centripetal', 0.5);
        const tube = new THREE.TubeGeometry(curve, pts.length * 8, kind === 'retainer' ? 0.30 : 0.34, 8, false);
        this.wireGroup.add(new THREE.Mesh(tube, this.materials.get('wire')));
      }
    }
    this._needsRender = true;
  }

  _commit(changed) {
    if (this._suspend > 0) return;
    this.root.updateMatrixWorld(true);
    this._rebuildLinks();
    if (this.menu && this.menu.isOpen) this.menu.render();
    this.emit('change', { teeth: changed, state: this.getState() });
  }

  /**
   * Batch several mutations into a single re-link + `change` event.
   * @param {() => void} fn
   */
  batch(fn) {
    this._suspend++;
    try { fn(this); } finally { this._suspend--; }
    this._commit(null);
    return this;
  }

  // -------------------------------------------------------------------------
  // public state API
  // -------------------------------------------------------------------------

  /** Normalise any accepted tooth reference into a valid FDI number. */
  resolveTooth(ref) {
    const fdi = toFdi(ref, this.numbering === 'universal' ? 'universal' : 'fdi');
    if (!isValidFdi(fdi, this.modelType)) {
      throw new Error(`[dental3d] "${ref}" is not a ${this.modelType} tooth`);
    }
    return fdi;
  }

  /** Every FDI number of the active dentition, in anatomical order. */
  get teeth() { return [...this.teethList]; }

  /**
   * FDI numbers of one quadrant or arch.
   * @param {'upper'|'lower'|number} which  quadrant 1..4 also works while the
   *        primary dentition is active (it maps onto 5..8).
   */
  getGroup(which) {
    if (which === 'upper' || which === 'lower') {
      return this.teethList.filter((f) => QUADRANTS[Math.floor(f / 10)].arch === which);
    }
    let q = Number(which);
    if (this.modelType === 'primary' && q >= 1 && q <= 4) q += 4;
    return this.teethList.filter((f) => Math.floor(f / 10) === q);
  }

  /** Current state of one tooth (a deep copy - safe to keep). */
  getTooth(ref) {
    const fdi = this.resolveTooth(ref);
    const st = this.state.get(fdi);
    return {
      fdi,
      ...describeTooth(fdi, this.t.locale),
      treatments: st.treatments.map((r) => ({ ...r, surfaces: r.surfaces ? [...r.surfaces] : undefined })),
      note: st.note,
      interactive: st.interactive !== false,
      selected: this.selection.has(fdi)
    };
  }

  /** Replace the full treatment list of a tooth. */
  setTreatment(ref, treatments) {
    const fdi = this.resolveTooth(ref);
    const list = (Array.isArray(treatments) ? treatments : treatments == null ? [] : [treatments])
      .map(normalizeTreatment);
    this.state.get(fdi).treatments = list;
    this._refresh(fdi);
    this._commit([fdi]);
    this.emit('treatmentChange', { fdi, treatments: this.getTooth(fdi).treatments });
    return this;
  }

  /** Add one treatment, replacing any conflicting one in the same group. */
  addTreatment(ref, treatment) {
    const fdi = this.resolveTooth(ref);
    const rec = normalizeTreatment(treatment);
    const def = getTreatmentDef(rec.type);
    const list = this.state.get(fdi).treatments.filter((r) => {
      if (r.type === rec.type) return false;
      const other = getTreatmentDef(r.type);
      return !(def.group && other && other.group === def.group);
    });
    list.push(rec);
    this.state.get(fdi).treatments = list;
    this._refresh(fdi);
    this._commit([fdi]);
    this.emit('treatmentChange', { fdi, treatments: this.getTooth(fdi).treatments, added: rec });
    return this;
  }

  /** Remove one treatment by type. */
  removeTreatment(ref, type) {
    const fdi = this.resolveTooth(ref);
    const st = this.state.get(fdi);
    const before = st.treatments.length;
    st.treatments = st.treatments.filter((r) => r.type !== type);
    if (st.treatments.length === before) return this;
    this._refresh(fdi);
    this._commit([fdi]);
    this.emit('treatmentChange', { fdi, treatments: this.getTooth(fdi).treatments, removed: type });
    return this;
  }

  /** Shallow-merge new options into an existing treatment record. */
  updateTreatment(ref, type, patch) {
    const fdi = this.resolveTooth(ref);
    const st = this.state.get(fdi);
    const idx = st.treatments.findIndex((r) => r.type === type);
    if (idx < 0) return this.addTreatment(fdi, { type, ...patch });
    st.treatments[idx] = normalizeTreatment({ ...st.treatments[idx], ...patch });
    this._refresh(fdi);
    this._commit([fdi]);
    this.emit('treatmentChange', { fdi, treatments: this.getTooth(fdi).treatments, updated: type });
    return this;
  }

  /** True when the tooth carries the given treatment type. */
  hasTreatment(ref, type) {
    return this.state.get(this.resolveTooth(ref)).treatments.some((r) => r.type === type);
  }

  /** The stored record for one treatment type, or null. */
  getTreatment(ref, type) {
    const rec = this.state.get(this.resolveTooth(ref)).treatments.find((r) => r.type === type);
    return rec ? { ...rec, surfaces: rec.surfaces ? [...rec.surfaces] : undefined } : null;
  }

  /** Remove every treatment from a tooth. */
  clearTooth(ref) { return this.setTreatment(ref, []); }

  /** Attach a free-text note to a tooth (round-trips through getState). */
  setNote(ref, note) {
    const fdi = this.resolveTooth(ref);
    this.state.get(fdi).note = note || undefined;
    this._commit([fdi]);
    return this;
  }

  /** Enable or disable picking for a tooth. */
  setInteractive(ref, value) {
    this.state.get(this.resolveTooth(ref)).interactive = value !== false;
    return this;
  }

  /**
   * Apply the same treatment to many teeth at once.
   * @param {Array<number|string>} refs
   * @param {string|object} treatment
   */
  applyToTeeth(refs, treatment) {
    return this.batch(() => { for (const r of refs) this.addTreatment(r, treatment); });
  }

  /**
   * Load a full configuration.
   *
   * Accepts `{ teeth: { 16: {...} } }` or the bare `{ 16: [...] }` map. Each
   * entry may be a treatment id, an array of them, or `{ treatments, note }`.
   */
  setState(config, { silent = false } = {}) {
    const src = config && config.teeth ? config.teeth : (config || {});
    if (config && config.modelType) this.setModelType(config.modelType, { silent: true });
    if (config && config.numbering) this.setNumbering(config.numbering);

    this._suspend++;
    try {
      for (const fdi of this.teethList) {
        this.state.set(fdi, { treatments: [] });
      }
      for (const [key, value] of Object.entries(src)) {
        const fdi = this.resolveTooth(key);
        let list = value, note, interactive;
        if (value && !Array.isArray(value) && typeof value === 'object') {
          list = value.treatments ?? [];
          note = value.note;
          interactive = value.interactive;
        }
        const treatments = (Array.isArray(list) ? list : list == null ? [] : [list]).map(normalizeTreatment);
        this.state.set(fdi, { treatments, note, interactive });
      }
      for (const fdi of this.teethList) this._refresh(fdi);
    } finally {
      this._suspend--;
    }
    this.root.updateMatrixWorld(true);
    this._rebuildLinks();
    if (this.menu && this.menu.isOpen) this.menu.render();
    if (!silent) this.emit('change', { teeth: this.teethList, state: this.getState() });
    return this;
  }

  /** Serialisable snapshot; feed it straight back into {@link setState}. */
  getState() {
    const teeth = {};
    for (const fdi of this.teethList) {
      const st = this.state.get(fdi);
      if (!st.treatments.length && !st.note && st.interactive === undefined) continue;
      const entry = {};
      if (st.treatments.length) {
        entry.treatments = st.treatments.map((r) => ({ ...r, surfaces: r.surfaces ? [...r.surfaces] : undefined }));
      }
      if (st.note) entry.note = st.note;
      if (st.interactive === false) entry.interactive = false;
      teeth[fdi] = entry;
    }
    return { version: STATE_VERSION, modelType: this.modelType, numbering: this.numbering, teeth };
  }

  /** Alias of {@link getState}. */
  toJSON() { return this.getState(); }

  /** Clear every tooth and the selection. */
  reset() {
    this.setSelection([]);
    return this.setState({});
  }

  // -------------------------------------------------------------------------
  // selection & highlighting
  // -------------------------------------------------------------------------

  /** Replace the selection. */
  setSelection(refs) {
    const next = new Set((refs || []).map((r) => this.resolveTooth(r)));
    const touched = new Set([...this.selection, ...next]);
    this.selection = next;
    for (const fdi of touched) this._applyOutline(this.views.get(fdi));
    this._needsRender = true;
    this.emit('selectionChange', { selection: [...next] });
    return this;
  }

  /** Add one tooth to the selection. */
  select(ref) { return this.setSelection([...this.selection, this.resolveTooth(ref)]); }

  /** Remove one tooth from the selection. */
  deselect(ref) {
    const fdi = this.resolveTooth(ref);
    return this.setSelection([...this.selection].filter((f) => f !== fdi));
  }

  /** Currently selected FDI numbers. */
  getSelection() { return [...this.selection]; }

  /**
   * Paint a persistent outline on a tooth, independent of the selection.
   * @param {number|string} ref
   * @param {number|string|null} color  CSS/hex colour, or null to clear
   */
  highlight(ref, color) {
    const view = this.views.get(this.resolveTooth(ref));
    view.highlightColor = color == null ? null : new THREE.Color(color).getHex();
    this._applyOutline(view);
    this._needsRender = true;
    return this;
  }

  /** Clear every custom highlight. */
  clearHighlights() {
    for (const view of this.views.values()) {
      if (view.highlightColor != null) { view.highlightColor = null; this._applyOutline(view); }
    }
    this._needsRender = true;
    return this;
  }

  // -------------------------------------------------------------------------
  // view control
  // -------------------------------------------------------------------------

  /** Show one arch or both. */
  setArch(which) {
    const w = which || 'both';
    this.archGroups.upper.visible = w === 'both' || w === 'upper';
    this.archGroups.lower.visible = w === 'both' || w === 'lower';
    this.options.arch = w;
    this._needsRender = true;
    this.emit('viewChange', { arch: w });
    return this;
  }

  /**
   * Pull the arches straight apart so the occlusal surfaces become visible.
   * @param {number} mm
   * @param {{fit?: boolean}} [opts] refit the camera afterwards (default true)
   */
  setArchSeparation(mm, opts = {}) {
    this.options.archSeparation = mm || 0;
    this._updateJawTransform();
    if (opts.fit !== false) this.fit({ instant: true });
    return this;
  }

  /**
   * Swing the mandible open about the temporomandibular hinge, the way a real
   * mouth opens: the chin travels down *and* back, not straight down.
   *
   * Roughly 25 deg corresponds to a maximum physiological opening (~45 mm
   * between the incisal edges). Ignored in flat layout mode.
   *
   * @param {number} degrees 0..35
   * @param {{fit?: boolean}} [opts] refit the camera afterwards (default true)
   */
  setJawOpening(degrees, opts = {}) {
    this.options.jawOpening = THREE.MathUtils.clamp(Number(degrees) || 0, 0, MAX_JAW_OPENING);
    this._updateJawTransform();
    if (opts.fit !== false) this.fit({ instant: true });
    this.emit('viewChange', { jawOpening: this.options.jawOpening });
    return this;
  }

  /** Current hinge point in world space. */
  _hingePoint() {
    const h = this.options.hinge || HINGE[this.modelType];
    return new THREE.Vector3(0, h.y, h.z);
  }

  _updateJawTransform() {
    const sep = this.options.archSeparation || 0;
    const deg = this.layout && this.layout.flat ? 0 : (this.options.jawOpening || 0);

    this.archGroups.upper.quaternion.identity();
    this.archGroups.upper.position.set(0, sep / 2, 0);

    const q = new THREE.Quaternion().setFromAxisAngle(_v1.set(1, 0, 0), deg * Math.PI / 180);
    const pivot = this._hingePoint();
    const shifted = pivot.clone().applyQuaternion(q);
    this.archGroups.lower.quaternion.copy(q);
    this.archGroups.lower.position.copy(pivot).sub(shifted).add(_v2.set(0, -sep / 2, 0));

    this.root.updateMatrixWorld(true);
    this._rebuildLinks();
    this._needsRender = true;
  }

  /** Show or hide the gingiva. */
  setGumsVisible(visible) {
    this.gumGroups.upper.visible = !!visible;
    this.gumGroups.lower.visible = !!visible;
    this.options.showGums = !!visible;
    this._needsRender = true;
    return this;
  }

  /** Show or hide the floating tooth-number labels. */
  setLabelsVisible(visible) {
    this.labelGroups.upper.visible = !!visible;
    this.labelGroups.lower.visible = !!visible;
    this.options.showLabels = !!visible;
    this._needsRender = true;
    return this;
  }

  /**
   * Switch between the permanent (32 teeth) and primary (20 teeth) dentition.
   * Treatment state is remembered per dentition, so switching back restores it.
   * @param {'permanent'|'primary'} type
   */
  setModelType(type, { silent = false } = {}) {
    const next = type === 'primary' ? 'primary' : 'permanent';
    if (next === this.modelType) return this;
    this._savedStates[this.modelType] = this.state;
    this.modelType = next;
    this.options.modelType = next;
    this.state = this._savedStates[next] || new Map();
    this.selection.clear();
    this._rebuildModel();
    this.emit('modelTypeChange', { modelType: next });
    if (!silent) this.emit('change', { teeth: this.teethList, state: this.getState() });
    return this;
  }

  /**
   * Switch between the anatomical arch and the straight two-row chart layout.
   * @param {'anatomic'|'flat'} mode
   */
  setLayout(mode) {
    const next = mode === 'flat' ? 'flat' : 'anatomic';
    if (next === this.layoutMode) return this;
    this.layoutMode = next;
    this.options.layoutMode = next;
    this._rebuildModel();
    this.emit('viewChange', { layoutMode: next });
    return this;
  }

  /** Switch numbering system ('fdi' | 'universal' | 'palmer'). */
  setNumbering(system) {
    this.numbering = system;
    for (const view of this.views.values()) this._paintLabel(view);
    if (this.menu && this.menu.isOpen) this.menu.render();
    this._needsRender = true;
    return this;
  }

  /** Switch UI language ('tr' | 'en'). */
  setLocale(locale) {
    this.t = createTranslator(locale, this.options.strings);
    if (this.menu && this.menu.isOpen) this.menu.render();
    return this;
  }

  /**
   * Move the camera to a named viewpoint.
   * @param {'front'|'back'|'left'|'right'|'top'|'bottom'|'upper'|'lower'|'default'} name
   * @param {{instant?: boolean, distance?: number, isolate?: boolean}} [opts]
   */
  setView(name, opts = {}) {
    const preset = VIEW_PRESETS[name] || VIEW_PRESETS.default;
    const dir = new THREE.Vector3(...preset.pos).normalize();
    // 'upper' / 'lower' are occlusal views, so they isolate that arch unless
    // the caller opts out with { isolate: false }
    if (preset.arch && opts.isolate !== false) this.setArch(preset.arch);
    const box = this._visibleBox();
    const target = this._viewTarget(box);
    this.camera.up.set(...preset.up);
    const dist = opts.distance ?? this._cameraDistance(box, dir, this.camera.up);
    const position = target.clone().addScaledVector(dir, dist);
    this._moveCamera(position, target, opts.instant);
    this.emit('viewChange', { view: name });
    return this;
  }

  /** Keep the current viewing direction but frame everything that is visible. */
  fit(opts = {}) {
    const box = this._visibleBox();
    if (box.isEmpty()) return this;
    const target = this._viewTarget(box);
    const dir = this.camera.position.clone().sub(this.controls.target);
    if (dir.lengthSq() < 1e-6) dir.set(-0.38, 0.26, 0.89);
    dir.normalize();
    const dist = this._cameraDistance(box, dir, this.camera.up);
    this._moveCamera(target.clone().addScaledVector(dir, dist), target, opts.instant);
    return this;
  }

  /** World bounding box of every visible tooth (plus gingiva when shown). */
  _visibleBox() {
    this.root.updateMatrixWorld(true);
    const box = new THREE.Box3();
    const tmp = new THREE.Box3();
    for (const v of this.views.values()) {
      if (!v.mesh.visible || !this.archGroups[v.info.arch].visible) continue;
      if (!v.mesh.geometry.boundingBox) v.mesh.geometry.computeBoundingBox();
      box.union(tmp.copy(v.mesh.geometry.boundingBox).applyMatrix4(v.mesh.matrixWorld));
    }
    if (this.options.showGums) {
      for (const arch of ['upper', 'lower']) {
        if (!this.archGroups[arch].visible) continue;
        for (const m of this.gumGroups[arch].children) {
          box.union(tmp.copy(m.geometry.boundingBox).applyMatrix4(m.matrixWorld));
        }
      }
    }
    return box;
  }

  /**
   * Distance that fits `box` in frame, or the explicit option when given.
   *
   * A bounding *sphere* would badly over-frame the flat layout, which is very
   * wide and very short, so the box half-extents are projected onto the camera
   * axes and each is checked against the fov it actually competes with.
   */
  _cameraDistance(box = this._visibleBox(), direction, upHint) {
    const explicit = this.options.cameraDistance;
    if (typeof explicit === 'number') return explicit;
    if (box.isEmpty()) return 120;

    const half = box.getSize(new THREE.Vector3()).multiplyScalar(0.5);
    const forward = (direction ? direction.clone()
      : this.camera.position.clone().sub(this.controls.target)).normalize();
    if (forward.lengthSq() < 1e-6) forward.set(0, 0, 1);

    let up = (upHint || this.camera.up).clone();
    let right = new THREE.Vector3().crossVectors(up, forward);
    if (right.lengthSq() < 1e-8) right.set(1, 0, 0);
    right.normalize();
    up.crossVectors(forward, right).normalize();

    const extent = (a) => Math.abs(half.x * a.x) + Math.abs(half.y * a.y) + Math.abs(half.z * a.z);
    const vFov = this.camera.fov * Math.PI / 180;
    const hFov = 2 * Math.atan(Math.tan(vFov / 2) * this.camera.aspect);
    return Math.max(
      extent(right) / Math.tan(hFov / 2),
      extent(up) / Math.tan(vFov / 2)
    ) * 1.06 + extent(forward) * 0.45;
  }

  /** Keep the orbit limits sensible for the current model size. */
  _updateControlLimits() {
    const box = this._visibleBox();
    const radius = box.isEmpty() ? 60 : box.getBoundingSphere(new THREE.Sphere()).radius;  // eslint-disable-line
    const { minDistance, maxDistance } = this.options;
    this.controls.minDistance = typeof minDistance === 'number' ? minDistance : Math.max(6, radius * 0.35);
    this.controls.maxDistance = typeof maxDistance === 'number' ? maxDistance : radius * 9;
  }

  /** Frame a single tooth. */
  focusTooth(ref, opts = {}) {
    const view = this.views.get(this.resolveTooth(ref));
    this.root.updateMatrixWorld(true);
    const target = view.group.localToWorld(new THREE.Vector3(0, view.spec.crownH * 0.4, 0));
    const dist = opts.distance ?? 42;
    const dir = view.layout.buccal.clone().multiplyScalar(0.82)
      .add(view.layout.occlusal.clone().multiplyScalar(0.35))
      .normalize();
    this._moveCamera(target.clone().addScaledVector(dir, dist), target, opts.instant);
    return this;
  }

  /** Centre of whatever is currently visible, used as the orbit target. */
  _viewTarget(box = this._visibleBox()) {
    if (box.isEmpty()) return new THREE.Vector3(0, 0, -2);
    return box.getCenter(new THREE.Vector3());
  }

  _moveCamera(position, target, instant) {
    if (instant) {
      this.camera.position.copy(position);
      this.controls.target.copy(target);
      this.controls.update();
      this._needsRender = true;
      return;
    }
    this._camAnim = {
      from: this.camera.position.clone(), to: position.clone(),
      tFrom: this.controls.target.clone(), tTo: target.clone(),
      start: performance.now(), duration: 620
    };
  }

  _stepCameraAnimation() {
    const a = this._camAnim;
    const k = Math.min(1, (performance.now() - a.start) / a.duration);
    const e = k < 0.5 ? 4 * k * k * k : 1 - Math.pow(-2 * k + 2, 3) / 2;
    this.camera.position.lerpVectors(a.from, a.to, e);
    this.controls.target.lerpVectors(a.tFrom, a.tTo, e);
    this.controls.update();
    this._needsRender = true;
    if (k >= 1) this._camAnim = null;
  }

  /**
   * Turn the built-in treatment picker on or off at runtime. Useful when your
   * own UI takes over clicks (a "paint the selected treatment" mode).
   */
  setContextMenuEnabled(on) {
    this.options.contextMenu = !!on;
    if (!on && this.menu) this.menu.close();
    return this;
  }

  /** Enable/disable the idle turntable. */
  setAutoRotate(on) { this.controls.autoRotate = !!on; this._needsRender = true; return this; }

  /**
   * Render a PNG of the current view.
   * @param {{width?: number, height?: number, mime?: string}} [opts]
   * @returns {string} data URL
   */
  exportImage(opts = {}) {
    const { width, height, mime = 'image/png' } = opts;
    const old = new THREE.Vector2();
    this.renderer.getSize(old);
    if (width && height) {
      this.renderer.setSize(width, height, false);
      this.camera.aspect = width / height;
      this.camera.updateProjectionMatrix();
    }
    this.renderer.render(this.scene, this.camera);
    const url = this.renderer.domElement.toDataURL(mime);
    if (width && height) {
      this.renderer.setSize(old.x, old.y, false);
      this.camera.aspect = old.x / old.y;
      this.camera.updateProjectionMatrix();
      this.renderer.render(this.scene, this.camera);
    }
    return url;
  }

  // -------------------------------------------------------------------------

  /** Tear down WebGL resources, listeners and DOM nodes. */
  dispose() {
    if (this._disposed) return;
    this._disposed = true;
    cancelAnimationFrame(this._raf);
    this._resizeObserver.disconnect();
    this.canvas.removeEventListener('pointerdown', this._onPointerDown);
    this.canvas.removeEventListener('pointerup', this._onPointerUp);
    this.canvas.removeEventListener('pointermove', this._onPointerMove);
    this.canvas.removeEventListener('pointerleave', this._onPointerLeave);
    this.controls.dispose();
    if (this.menu) this.menu.dispose();
    if (this._tooltip) this._tooltip.remove();

    const seenGeom = new Set();
    this.scene.traverse((o) => {
      if (o.isMesh || o.isSprite) {
        if (o.geometry && !seenGeom.has(o.geometry)) { seenGeom.add(o.geometry); o.geometry.dispose(); }
        const mats = Array.isArray(o.material) ? o.material : [o.material];
        for (const m of mats) { if (m) { if (m.map) m.map.dispose(); m.dispose(); } }
      }
    });
    for (const g of this._patchCache.values()) g.dispose();
    this._patchCache.clear();
    if (--_liveCharts <= 0) disposeTreatmentGeometries();
    this.materials.dispose();
    this._env.dispose();
    this.renderer.dispose();
    this.canvas.remove();
    this.removeAllListeners();
    this.views.clear();
  }
}

export { TREATMENTS, SURFACES };


```
