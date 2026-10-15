# Facial rendering source scan: Yoosseph/dental-scope

Repository: https://github.com/Yoosseph/dental-scope

Pinned source revision: 1a0615f384c5d069cac2de5c214952d8a15b5133

Discovery topic: teeth 3D

Description: Explore teeth, jaws, nerves, tissues, root canals, cross-sections and layered dental structures in 3D.

Reported license: NOASSERTION

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"teeth": ["public/models/teeth/tooth-11.glb", "public/models/teeth/tooth-12.glb", "public/models/teeth/tooth-13.glb", "public/models/teeth/tooth-14.glb", "public/models/teeth/tooth-15.glb", "public/models/teeth/tooth-16.glb", "public/models/teeth/tooth-17.glb", "public/models/teeth/tooth-18.glb", "public/models/teeth/tooth-21.glb", "public/models/teeth/tooth-22.glb", "public/models/teeth/tooth-23.glb", "public/models/teeth/tooth-24.glb", "public/models/teeth/tooth-25.glb", "public/models/teeth/tooth-26.glb", "public/models/teeth/tooth-27.glb", "public/models/teeth/tooth-28.glb", "public/models/teeth/tooth-31.glb", "public/models/teeth/tooth-32.glb", "public/models/teeth/tooth-33.glb", "public/models/teeth/tooth-34.glb", "public/models/teeth/tooth-35.glb", "public/models/teeth/tooth-36.glb", "public/models/teeth/tooth-37.glb", "public/models/teeth/tooth-38.glb", "public/models/teeth/tooth-41.glb", "public/models/teeth/tooth-42.glb", "public/models/teeth/tooth-43.glb", "public/models/teeth/tooth-44.glb", "public/models/teeth/tooth-45.glb", "public/models/teeth/tooth-46.glb"], "tooth": ["docs/screenshots/tooth-canals.png", "public/models/teeth/tooth-11.glb", "public/models/teeth/tooth-12.glb", "public/models/teeth/tooth-13.glb", "public/models/teeth/tooth-14.glb", "public/models/teeth/tooth-15.glb", "public/models/teeth/tooth-16.glb", "public/models/teeth/tooth-17.glb", "public/models/teeth/tooth-18.glb", "public/models/teeth/tooth-21.glb", "public/models/teeth/tooth-22.glb", "public/models/teeth/tooth-23.glb", "public/models/teeth/tooth-24.glb", "public/models/teeth/tooth-25.glb", "public/models/teeth/tooth-26.glb", "public/models/teeth/tooth-27.glb", "public/models/teeth/tooth-28.glb", "public/models/teeth/tooth-31.glb", "public/models/teeth/tooth-32.glb", "public/models/teeth/tooth-33.glb", "public/models/teeth/tooth-34.glb", "public/models/teeth/tooth-35.glb", "public/models/teeth/tooth-36.glb", "public/models/teeth/tooth-37.glb", "public/models/teeth/tooth-38.glb", "public/models/teeth/tooth-41.glb", "public/models/teeth/tooth-42.glb", "public/models/teeth/tooth-43.glb", "public/models/teeth/tooth-44.glb", "public/models/teeth/tooth-45.glb"]}

## Source file: src/engine/toothShading.ts

{
  "path": "src/engine/toothShading.ts",
  "blob_sha": "89e67aa2248189501bb0c647315563a5d3c1ac2c",
  "sha256": "c35914a7864c3e0c38eda9e6eb675dd6af4823930f812c8fd1dce4f9c119b5d6",
  "lines": 82,
  "bytes": 3760,
  "selection_score": 15,
  "keyword_line_counts": {
    "teeth": 1,
    "tooth": 9,
    "face": 2
  },
  "url": "https://github.com/Yoosseph/dental-scope/blob/1a0615f384c5d069cac2de5c214952d8a15b5133/src/engine/toothShading.ts",
  "function_signature_candidates": []
}

```text

/**
 * Per-vertex colour for tooth surfaces, baked once when a tooth mesh is added:
 * a crown-to-root colour change at the modeled CEJ and darkened occlusal fissures.
 */
import * as THREE from 'three';
import type { Registry } from '../anatomy/registry';
import type { TissueMaterial } from './materials';

/** Give the intact tooth a subtle crown-to-root colour change at its modeled CEJ. */
export function colorToothShell(geo: THREE.BufferGeometry, mat: TissueMaterial, registry: Registry, fdi: number) {
  const tooth = registry.manifest.teeth[String(fdi)];
  const cervical = tooth?.landmarks?.['cervical-line'];
  const axis = tooth?.frame.axis;
  if (!cervical || !axis) return;
  mat.userData.fx.uCervical.value.set(...cervical);
  mat.userData.fx.uToothAxis.value.set(...axis);
  const positions = geo.getAttribute('position');
  const crevices = meshCrevices(geo, 0.0004, 0.004);
  const colors = new Float32Array(positions.count * 3);
  // height of each vertex along the tooth axis, measured from the cervical line
  const heights = new Float64Array(positions.count);
  let crownTop = 0;
  for (let i = 0; i < positions.count; i++) {
    heights[i] = (positions.getX(i) - cervical[0]) * axis[0]
      + (positions.getY(i) - cervical[1]) * axis[1]
      + (positions.getZ(i) - cervical[2]) * axis[2];
    crownTop = Math.max(crownTop, heights[i]);
  }
  const shade = 1 + (((fdi * 17) % 7) - 3) * 0.006;
  for (let i = 0; i < positions.count; i++) {
    const height = heights[i];
    const crown = THREE.MathUtils.smoothstep(height, -0.11, 0.07);
    const tip = THREE.MathUtils.smoothstep(height, crownTop * 0.65, crownTop * 0.92);
    const fissure = 1 - 0.25 * crevices[i] * crown;
    colors[i * 3] = THREE.MathUtils.lerp(0.84, 1, crown) * (1 - 0.08 * tip) * shade * fissure;
    colors[i * 3 + 1] = THREE.MathUtils.lerp(0.76, 0.985, crown) * (1 - 0.015 * tip) * shade * fissure;
    colors[i * 3 + 2] = THREE.MathUtils.lerp(0.66, 0.96, crown) * (1 + 0.035 * tip) * shade * fissure;
  }
  geo.setAttribute('color', new THREE.BufferAttribute(colors, 3));
}

/** Accentuate concave regions already present in the tooth surface, especially occlusal fissures. */
export function shadeEnamelCrevices(geo: THREE.BufferGeometry) {
  const crevices = meshCrevices(geo, 0.0015, 0.008);
  const colors = new Float32Array(crevices.length * 3);
  for (let i = 0; i < crevices.length; i++) {
    const shade = 1 - 0.19 * crevices[i];
    colors[i * 3] = shade;
    colors[i * 3 + 1] = shade;
    colors[i * 3 + 2] = shade;
  }
  geo.setAttribute('color', new THREE.BufferAttribute(colors, 3));
}

function meshCrevices(geo: THREE.BufferGeometry, start: number, end: number): Float32Array {
  const pos = geo.getAttribute('position');
  const normal = geo.getAttribute('normal');
  const index = geo.getIndex();
  const out = new Float32Array(pos.count);
  if (!index || !normal) return out;
  const sum = new Float32Array(pos.count * 3);
  const degree = new Uint16Array(pos.count);
  const add = (from: number, to: number) => {
    sum[from * 3] += pos.getX(to);
    sum[from * 3 + 1] += pos.getY(to);
    sum[from * 3 + 2] += pos.getZ(to);
    degree[from]++;
  };
  for (let i = 0; i < index.count; i += 3) {
    const a = index.getX(i), b = index.getX(i + 1), c = index.getX(i + 2);
    add(a, b); add(a, c); add(b, a); add(b, c); add(c, a); add(c, b);
  }
  for (let i = 0; i < pos.count; i++) {
    if (!degree[i]) continue;
    const x = sum[i * 3] / degree[i] - pos.getX(i);
    const y = sum[i * 3 + 1] / degree[i] - pos.getY(i);
    const z = sum[i * 3 + 2] / degree[i] - pos.getZ(i);
    const depth = x * normal.getX(i) + y * normal.getY(i) + z * normal.getZ(i);
    out[i] = THREE.MathUtils.smoothstep(depth, start, end);
  }
  return out;
}


```

## Source file: tools/pipeline/tooth_layers.py

{
  "path": "tools/pipeline/tooth_layers.py",
  "blob_sha": "97aa5f377a4ec53ec442a59ae1f6d419ad82c340",
  "sha256": "8aaad343d442b136368911d27e2ebc16d337038e337fe3c1bba13a842c74faef",
  "lines": 511,
  "bytes": 22924,
  "selection_score": 15,
  "keyword_line_counts": {
    "jaw": 1,
    "tooth": 11,
    "rig": 4,
    "face": 24,
    "lip": 7
  },
  "url": "https://github.com/Yoosseph/dental-scope/blob/1a0615f384c5d069cac2de5c214952d8a15b5133/tools/pipeline/tooth_layers.py",
  "function_signature_candidates": [
    {
      "line": 122,
      "text": "def R(self) -> np.ndarray:"
    },
    {
      "line": 126,
      "text": "def to_local(self, p):"
    },
    {
      "line": 129,
      "text": "def to_world(self, p):"
    },
    {
      "line": 142,
      "text": "def make_frame(mesh: trimesh.Trimesh, crown_dir: np.ndarray, mesial_hint: np.ndarray, buccal_hint: np.ndarray) -> Frame:"
    },
    {
      "line": 162,
      "text": "def voxelize_parity(tri: np.ndarray, lo: np.ndarray, h: float, shape) -> np.ndarray:"
    },
    {
      "line": 207,
      "text": "def smin(a, b, k):"
    },
    {
      "line": 213,
      "text": "def smax(a, b, k):"
    },
    {
      "line": 217,
      "text": "def surface(field: np.ndarray, lo, h, frame: Frame, target_faces: int, smooth=4) -> trimesh.Trimesh | None:"
    },
    {
      "line": 234,
      "text": "def reduce(source: trimesh.Trimesh, aggressiveness: float) -> trimesh.Trimesh:"
    },
    {
      "line": 258,
      "text": "def track_roots(inside, lo, h, z_top, z_bottom):"
    },
    {
      "line": 294,
      "text": "def build_tooth_layers(mesh: trimesh.Trimesh, arch: str, ttype: str, frame: Frame, h=0.09,"
    },
    {
      "line": 416,
      "text": "def tube_field(path, r0=0.42, r1=0.14):"
    }
  ]
}

```text

"""
Tooth layer modelling for Dental Scope.

Takes one closed external tooth surface (BodyParts3D) and derives schematic
internal anatomy as separate closed solids:

    enamel, coronal dentin, radicular dentin, cementum, periodontal ligament,
    pulp chamber, one tube per root canal

Method
------
1. Build a local frame: long axis (PCA, oriented toward the crown), mesial and
   buccal directions supplied by the caller (from the arch layout).
2. Voxelise the tooth in that frame and compute a signed distance field (EDT).
3. Place the cervical line using a typical crown/total length ratio for the
   tooth type, curved toward the crown on proximal surfaces.
4. Every tissue is a scalar field whose zero level set is its surface:
       enamel   = inside ∧ depth < e(h) ∧ above cervical line
       dentin   = inside ∧ depth > outer layer ∧ outside pulp
       cementum = inside ∧ depth < c(h) ∧ below cervical line
       PDL      = thin shell just outside the root below the alveolar crest
       pulp     = (depth > w(h) above chamber floor) ∪ canal tubes
   Canals follow each root's medial axis, found by tracking connected
   components of cross-sections from the furcation to the apex.
5. Marching cubes → Taubin smoothing → quadric decimation.

Thickness parameters are *modelling* parameters chosen to give a readable
schematic at screen scale; they are not measurements. The UI labels these
structures as `modeled`.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from itertools import permutations

import numpy as np
import trimesh
from scipy import ndimage
from skimage import measure

from geometry import orient_outward, smooth_taubin

try:
    import fast_simplification
except ImportError:  # pragma: no cover
    fast_simplification = None


# --------------------------------------------------------------------------
# Per-type modelling parameters
# crown_ratio : typical anatomical crown length / total tooth length
# cej_curve   : how far (mm) the cervical line curves crownward on proximal faces
# enamel      : (axial wall, cusp/incisal) max enamel thickness in mm
# canals      : list of (root position label, canal count) in expected order
# --------------------------------------------------------------------------
TYPE_PARAMS = {
    ("maxillary", "central-incisor"): dict(crown_ratio=0.45, cej_curve=2.5, enamel=(0.9, 1.3)),
    ("maxillary", "lateral-incisor"): dict(crown_ratio=0.41, cej_curve=2.3, enamel=(0.8, 1.2)),
    ("maxillary", "canine"): dict(crown_ratio=0.37, cej_curve=2.0, enamel=(1.0, 1.6)),
    ("maxillary", "first-premolar"): dict(crown_ratio=0.38, cej_curve=1.0, enamel=(1.1, 1.8)),
    ("maxillary", "second-premolar"): dict(crown_ratio=0.38, cej_curve=1.0, enamel=(1.1, 1.8)),
    ("maxillary", "first-molar"): dict(crown_ratio=0.37, cej_curve=0.6, enamel=(1.2, 2.2)),
    ("maxillary", "second-molar"): dict(crown_ratio=0.38, cej_curve=0.5, enamel=(1.2, 2.1)),
    ("maxillary", "third-molar"): dict(crown_ratio=0.39, cej_curve=0.5, enamel=(1.2, 2.0)),
    ("mandibular", "central-incisor"): dict(crown_ratio=0.42, cej_curve=2.2, enamel=(0.7, 1.0)),
    ("mandibular", "lateral-incisor"): dict(crown_ratio=0.40, cej_curve=2.2, enamel=(0.7, 1.0)),
    ("mandibular", "canine"): dict(crown_ratio=0.41, cej_curve=1.8, enamel=(1.0, 1.5)),
    ("mandibular", "first-premolar"): dict(crown_ratio=0.38, cej_curve=0.9, enamel=(1.0, 1.7)),
    ("mandibular", "second-premolar"): dict(crown_ratio=0.36, cej_curve=0.9, enamel=(1.0, 1.7)),
    ("mandibular", "first-molar"): dict(crown_ratio=0.35, cej_curve=0.5, enamel=(1.2, 2.2)),
    ("mandibular", "second-molar"): dict(crown_ratio=0.35, cej_curve=0.5, enamel=(1.2, 2.1)),
    ("mandibular", "third-molar"): dict(crown_ratio=0.38, cej_curve=0.5, enamel=(1.2, 2.0)),
}

# Typical canal configuration per root (textbook "most common" case).
# Keys are root position labels resolved from geometry; a single-rooted tooth
# uses "single".
CANALS = {
    ("maxillary", "central-incisor"): {"single": 1},
    ("maxillary", "lateral-incisor"): {"single": 1},
    ("maxillary", "canine"): {"single": 1},
    ("maxillary", "first-premolar"): {"buccal": 1, "palatal": 1, "single": 2},
    ("maxillary", "second-premolar"): {"single": 1},
    ("maxillary", "first-molar"): {"mesiobuccal": 2, "distobuccal": 1, "palatal": 1},
    ("maxillary", "second-molar"): {"mesiobuccal": 1, "distobuccal": 1, "palatal": 1},
    ("maxillary", "third-molar"): {"mesiobuccal": 1, "distobuccal": 1, "palatal": 1},
    ("mandibular", "central-incisor"): {"single": 1},
    ("mandibular", "lateral-incisor"): {"single": 1},
    ("mandibular", "canine"): {"single": 1},
    ("mandibular", "first-premolar"): {"single": 1},
    ("mandibular", "second-premolar"): {"single": 1},
    ("mandibular", "first-molar"): {"mesial": 2, "distal": 1},
    ("mandibular", "second-molar"): {"mesial": 2, "distal": 1},
    ("mandibular", "third-molar"): {"mesial": 1, "distal": 1},
}

# Expected root layout used when geometry shows fused roots.
EXPECTED_ROOTS = {
    ("maxillary", "first-premolar"): ["buccal", "palatal"],
    ("maxillary", "first-molar"): ["mesiobuccal", "distobuccal", "palatal"],
    ("maxillary", "second-molar"): ["mesiobuccal", "distobuccal", "palatal"],
    ("maxillary", "third-molar"): ["mesiobuccal", "distobuccal", "palatal"],
    ("mandibular", "first-molar"): ["mesial", "distal"],
    ("mandibular", "second-molar"): ["mesial", "distal"],
    ("mandibular", "third-molar"): ["mesial", "distal"],
}

ROOT_DIRS = {  # unit direction in (mesial, buccal) plane
    "mesial": (1, 0), "distal": (-1, 0), "buccal": (0, 1), "palatal": (0, -1), "lingual": (0, -1),
    "mesiobuccal": (0.75, 0.66), "distobuccal": (-0.75, 0.66),
}


@dataclass
class Frame:
    origin: np.ndarray  # world point
    axis: np.ndarray    # long axis toward crown
    mesial: np.ndarray
    buccal: np.ndarray

    @property
    def R(self) -> np.ndarray:
        # rows = local basis (x=mesial, y=buccal, z=axis)
        return np.stack([self.mesial, self.buccal, self.axis])

    def to_local(self, p):
        return (p - self.origin) @ self.R.T

    def to_world(self, p):
        return p @ self.R + self.origin


@dataclass
class ToothLayers:
    meshes: dict = field(default_factory=dict)       # name -> trimesh.Trimesh (world)
    landmarks: dict = field(default_factory=dict)    # name -> world point
    roots: list = field(default_factory=list)        # [{label, canals:[names]}]
    cej_height: float = 0.0
    frame: Frame | None = None


def make_frame(mesh: trimesh.Trimesh, crown_dir: np.ndarray, mesial_hint: np.ndarray, buccal_hint: np.ndarray) -> Frame:
    v = mesh.vertices
    c = v.mean(0)
    cov = (v - c).T @ (v - c)
    w, vec = np.linalg.eigh(cov)
    axis = vec[:, 2]
    if axis @ crown_dir < 0:
        axis = -axis
    # Blend PCA axis with the jaw's crown direction; PCA alone is unstable for
    # wide molars.
    axis = axis * 0.7 + crown_dir * 0.3
    axis /= np.linalg.norm(axis)
    m = mesial_hint - axis * (mesial_hint @ axis)
    m /= np.linalg.norm(m)
    b = np.cross(axis, m)
    if b @ buccal_hint < 0:
        b = -b
    return Frame(origin=c, axis=axis, mesial=m, buccal=b)


def voxelize_parity(tri: np.ndarray, lo: np.ndarray, h: float, shape) -> np.ndarray:
    """Inside/outside by ray parity along +z for each (x, y) column."""
    nx, ny, nz = shape
    crossings = [[[] for _ in range(ny)] for _ in range(nx)]
    xs = lo[0] + (np.arange(nx) + 0.5) * h
    ys = lo[1] + (np.arange(ny) + 0.5) * h
    for t in tri:
        a, b, c = t
        minx, maxx = min(a[0], b[0], c[0]), max(a[0], b[0], c[0])
        miny, maxy = min(a[1], b[1], c[1]), max(a[1], b[1], c[1])
        i0 = max(int(np.ceil((minx - lo[0]) / h - 0.5)), 0)
        i1 = min(int(np.floor((maxx - lo[0]) / h - 0.5)), nx - 1)
        j0 = max(int(np.ceil((miny - lo[1]) / h - 0.5)), 0)
        j1 = min(int(np.floor((maxy - lo[1]) / h - 0.5)), ny - 1)
        if i0 > i1 or j0 > j1:
            continue
        d = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])
        if abs(d) < 1e-12:
            continue
        X, Y = np.meshgrid(xs[i0:i1 + 1], ys[j0:j1 + 1], indexing="ij")
        l1 = ((X - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (Y - a[1])) / d
        l2 = ((b[0] - a[0]) * (Y - a[1]) - (X - a[0]) * (b[1] - a[1])) / d
        l0 = 1 - l1 - l2
        ok = (l0 >= 0) & (l1 >= 0) & (l2 >= 0)
        if not ok.any():
            continue
        Z = l0 * a[2] + l1 * b[2] + l2 * c[2]
        for (ii, jj) in zip(*np.nonzero(ok)):
            crossings[i0 + ii][j0 + jj].append(Z[ii, jj])
    inside = np.zeros(shape, bool)
    zs = lo[2] + (np.arange(nz) + 0.5) * h
    for i in range(nx):
        for j in range(ny):
            cz = crossings[i][j]
            if len(cz) < 2:
                continue
            cz = np.sort(cz)
            # remove near-duplicates from shared edges
            keep = np.concatenate([[True], np.diff(cz) > 1e-6])
            cz = cz[keep]
            for k in range(0, len(cz) - 1, 2):
                inside[i, j, (zs > cz[k]) & (zs < cz[k + 1])] = True
    return inside


def smin(a, b, k):
    """Polynomial smooth minimum (k in field units)."""
    hh = np.clip(0.5 + 0.5 * (b - a) / k, 0, 1)
    return b * (1 - hh) + a * hh - k * hh * (1 - hh)


def smax(a, b, k):
    return -smin(-a, -b, k)


def surface(field: np.ndarray, lo, h, frame: Frame, target_faces: int, smooth=4) -> trimesh.Trimesh | None:
    if field.max() <= 0:
        return None
    # Suppress small voxel islands before meshing, avoiding detached specks.
    components, count = ndimage.label(field > 0)
    if count > 1:
        sizes = np.bincount(components.ravel())[1:]
        keep = np.flatnonzero(sizes >= sizes.max() * 0.02) + 1
        field = np.where(np.isin(components, keep), field, -h)
    padded = np.pad(field, 1, constant_values=-1.0)
    verts, faces, _, _ = measure.marching_cubes(padded, level=0.0, spacing=(h, h, h))
    verts = verts - h + lo
    # marching_cubes on a positive-inside field gives inward normals; flip.
    faces = faces[:, ::-1]
    m = trimesh.Trimesh(verts, faces, process=False)
    smooth_taubin(m, iterations=smooth)
    if fast_simplification is not None and len(m.faces) > target_faces:
        def reduce(source: trimesh.Trimesh, aggressiveness: float) -> trimesh.Trimesh:
            v2, f2 = fast_simplification.simplify(
                source.vertices.astype(np.float32), source.faces.astype(np.int32),
                target_count=target_faces, agg=aggressiveness, preserve_border=True,
            )
            return trimesh.Trimesh(v2, f2, process=False)

        # A very gentle pass removes marching-cubes pinches before the final
        # reduction; otherwise thin nested shells can acquire nonmanifold edges.
        clean = reduce(m, 2.0)
        simplified = reduce(clean, 7.0) if len(clean.faces) > target_faces else clean
        if not simplified.is_watertight:
            # Thin closed shells need gentler reduction. Accept extra detail up
            # to a bounded limit; never keep a huge unsimplified volume.
            for aggressiveness in (5.0, 4.0, 3.0):
                trial = reduce(clean, aggressiveness)
                if trial.is_watertight and len(trial.faces) <= target_faces * 2.5:
                    simplified = trial
                    break
        m = simplified
    m.vertices = frame.to_world(np.asarray(m.vertices))
    return orient_outward(m)


def track_roots(inside, lo, h, z_top, z_bottom):
    """Connected components of cross-sections from z_top down to z_bottom.
    Returns list of roots: dict(label_xy=[...], path=[(x,y,z)], area=[...])"""
    nz = inside.shape[2]
    zs = lo[2] + (np.arange(nz) + 0.5) * h
    ks = [k for k in range(nz) if z_bottom <= zs[k] <= z_top][::-1]
    roots = []
    for k in ks:
        lab, n = ndimage.label(inside[:, :, k])
        comps = []
        for ci in range(1, n + 1):
            ij = np.argwhere(lab == ci)
            if len(ij) < 6:
                continue
            xy = lo[:2] + (ij + 0.5) * h
            comps.append((xy.mean(0), len(ij) * h * h, xy))
        for cxy, area, pts in comps:
            # attach to nearest active root
            best, bd = None, 1e9
            for r in roots:
                if r["last_k"] - k > 3:
                    continue
                d = np.linalg.norm(r["path"][-1][:2] - cxy)
                if d < bd:
                    best, bd = r, d
            if best is not None and bd < 2.2 and best["last_k"] != k:
                best["path"].append(np.array([cxy[0], cxy[1], zs[k]]))
                best["area"].append(area)
                best["pts"].append(pts)
                best["last_k"] = k
            else:
                roots.append(dict(path=[np.array([cxy[0], cxy[1], zs[k]])], area=[area], pts=[pts], last_k=k))
    roots = [r for r in roots if len(r["path"]) >= 8]
    return roots


def build_tooth_layers(mesh: trimesh.Trimesh, arch: str, ttype: str, frame: Frame, h=0.09,
                       faces_budget=None) -> ToothLayers:
    key = (arch, ttype)
    P = TYPE_PARAMS[key]
    faces_budget = faces_budget or dict(enamel=12000, dentin=11000, cementum=7000, pdl=5000, chamber=4500, canal=1800)

    tri_local = frame.to_local(mesh.vertices)[mesh.faces]
    vl = tri_local.reshape(-1, 3)
    pad = 1.2
    lo = vl.min(0) - pad
    hi = vl.max(0) + pad
    shape = tuple(np.ceil((hi - lo) / h).astype(int))
    inside = voxelize_parity(tri_local, lo, h, shape)
    inside = ndimage.binary_closing(inside, iterations=1)

    din = ndimage.distance_transform_edt(inside) * h
    dout = ndimage.distance_transform_edt(~inside) * h
    sd = np.where(inside, din - 0.5 * h, -(dout - 0.5 * h))  # + inside

    X = lo[0] + (np.arange(shape[0]) + 0.5) * h
    Y = lo[1] + (np.arange(shape[1]) + 0.5) * h
    Z = lo[2] + (np.arange(shape[2]) + 0.5) * h
    XX, YY, ZZ = np.meshgrid(X, Y, Z, indexing="ij")

    z_top = vl[:, 2].max()
    z_apex = vl[:, 2].min()
    H = z_top - z_apex
    crown_len = P["crown_ratio"] * H
    z_cej = z_top - crown_len
    # cervical line curves crownward on the proximal (mesial/distal) surfaces
    r_xy = np.sqrt(XX ** 2 + YY ** 2) + 1e-6
    prox = (XX / r_xy) ** 2
    cej = z_cej + P["cej_curve"] * prox
    hc = ZZ - cej  # height above the cervical line (mm)

    # ---- enamel ------------------------------------------------------------
    e_ax, e_tip = P["enamel"]
    up = np.clip(hc / max(crown_len, 1e-3), 0, 1)
    e = (np.clip(hc / 1.8, 0, 1) ** 0.8) * (e_ax + (e_tip - e_ax) * np.clip((up - 0.55) / 0.45, 0, 1))
    # keep a minimum (slightly exaggerated) thickness so the enamel margin meets the
    # cementum cleanly at the cervical line instead of breaking up below voxel size
    e = np.maximum(e, 0.3)
    f_enamel = np.minimum.reduce([sd, e - sd, hc])

    # ---- pulp chamber -----------------------------------------------------
    multi = key in EXPECTED_ROOTS
    root_len = H - crown_len
    floor = -0.18 * root_len if multi else -0.05 * root_len
    half = np.percentile(np.abs(vl[np.abs(vl[:, 2] - z_cej) < 0.6][:, :2]), 90, axis=0).min()
    # dentin wall at the cervix ~ 60 % of the smaller half-width, thicker toward the roof
    wall = 0.6 * half
    # the wall thickens toward the occlusal surface; because `sd` is the distance to the
    # outer surface, the roof of the chamber follows the cusps and forms pulp horns
    roof_depth = wall + (0.9 if multi else 0.5)
    t = np.clip((hc / max(crown_len, 1e-3) - 0.1) / 0.45, 0, 1)
    w = wall + (roof_depth - wall) * t * t * (3 - 2 * t)
    f_chamber = smin(sd - w, (ZZ - (z_cej + floor)) * 1.0, 1.2)

    # ---- canals ----------------------------------------------------------
    z_furc = z_cej + floor - 0.3
    roots = track_roots(inside, lo, h, z_furc, z_apex + 0.2)
    labels = CANALS[key]
    expected = EXPECTED_ROOTS.get(key)
    canal_paths = []  # (name, rootLabel, [points local])
    root_infos = []

    if roots:
        centre = np.mean([r["path"][0][:2] for r in roots], axis=0)
    root_specs = []
    if expected and len(roots) >= len(expected):
        # Match the root configuration as a set. A greedy assignment can swap
        # adjacent molar roots when the first component is slightly off axis.
        roots = sorted(roots, key=lambda r: -len(r["path"]))[: len(expected)]
        centre = np.mean([r["path"][0][:2] for r in roots], axis=0)
        directions = [(r["path"][0][:2] - centre) / (np.linalg.norm(r["path"][0][:2] - centre) + 1e-9) for r in roots]
        assignment = max(permutations(expected), key=lambda labs: sum(np.dot(d, ROOT_DIRS[lab]) for d, lab in zip(directions, labs)))
        root_specs = list(zip(assignment, roots))
    elif roots:
        # fused / single root: one geometric root carries all expected canals
        r = max(roots, key=lambda r: len(r["path"]))
        if expected:
            for lab in expected:
                root_specs.append((lab, dict(r, virtual=True, dir=np.array(ROOT_DIRS[lab], float))))
        else:
            root_specs.append(("single", r))

    for lab, r in root_specs:
        path = np.array(r["path"])
        # smooth path
        if len(path) > 5:
            path[:, 0] = ndimage.gaussian_filter1d(path[:, 0], 2.5, mode="nearest")
            path[:, 1] = ndimage.gaussian_filter1d(path[:, 1], 2.5, mode="nearest")
        if r.get("virtual"):
            path = path.copy()
            t = np.linspace(0, 1, len(path))[:, None]
            path[:, :2] += r["dir"][None, :] * (1.2 * (1 - 0.6 * t))
        n = labels.get(lab, 1)
        names = []
        if n == 1:
            pts = [path]
        else:
            # split along the cross-section's major axis
            allp = np.concatenate(r["pts"][: max(3, len(r["pts"]) // 2)])
            cov = np.cov((allp - allp.mean(0)).T)
            ev, evec = np.linalg.eigh(cov)
            major = evec[:, 1]
            spread = np.sqrt(ev[1]) * 1.0
            t = np.linspace(0, 1, len(path))[:, None]
            off = np.concatenate([major, [0]])[None, :] * spread * (1 - 0.35 * t)
            pts = [path + off, path - off]
        for ci, pth in enumerate(pts):
            # start the canal inside the chamber, end just through the apex
            top = pth[0].copy()
            top[2] = z_cej + floor + 1.2
            ext = pth[-1] + (pth[-1] - pth[-4]) / max(np.linalg.norm(pth[-1] - pth[-4]), 1e-6) * 0.9
            full = np.vstack([top, pth, ext])
            suffix = "" if n == 1 else ("-1", "-2")[ci]
            canal_name = f"canal-{lab}{suffix}"
            names.append(canal_name)
            canal_paths.append((canal_name, lab, full))
        root_infos.append(dict(label=lab, canals=names))

    def tube_field(path, r0=0.42, r1=0.14):
        # distance from grid points to polyline, sampled radius along path
        seg_len = np.linalg.norm(np.diff(path, axis=0), axis=1)
        s = np.concatenate([[0], np.cumsum(seg_len)])
        s /= s[-1]
        best = np.full(ZZ.shape, 1e9)
        rad = np.full(ZZ.shape, r1)
        P3 = np.stack([XX, YY, ZZ], -1)
        for i in range(len(path) - 1):
            a, b = path[i], path[i + 1]
            ab = b - a
            L2 = ab @ ab
            if L2 < 1e-12:
                continue
            t = np.clip(((P3 - a) @ ab) / L2, 0, 1)
            d = np.linalg.norm(P3 - (a + t[..., None] * ab), axis=-1)
            m = d < best
            best = np.where(m, d, best)
            ss = s[i] + t * (s[i + 1] - s[i])
            rad = np.where(m, r0 + (r1 - r0) * ss, rad)
        return rad - best

    # subsample paths for speed
    canal_fields = []
    for name, lab, pth in canal_paths:
        step = max(1, len(pth) // 14)
        sp = np.vstack([pth[::step], pth[-1:]])
        canal_fields.append((name, lab, tube_field(sp)))

    f_pulp_canals = np.max([cf for _, _, cf in canal_fields], axis=0) if canal_fields else np.full(ZZ.shape, -1.0)
    f_pulp = smax(f_chamber, f_pulp_canals, 0.35)
    f_pulp = np.minimum(f_pulp, sd + 0.02)  # stays within tooth, exits at foramen

    # ---- cementum / dentin / pdl -----------------------------------------
    below = -hc
    depth_frac = np.clip(below / max(root_len, 1e-3), 0, 1)
    c = 0.22 + 0.35 * depth_frac
    # Adjacent tissues are separated by a hair-thin gap (EPS). Coincident surfaces
    # would z-fight when a section exposes them; with the gap, the inner tissue's
    # cut face always wins and sections read as clean coloured bands.
    EPS = 0.06
    f_cementum = np.minimum.reduce([sd, c - sd, below, -f_pulp - EPS])
    outer = np.where(hc > 0, e, c)
    f_dentin = np.minimum(sd - outer - EPS, -f_pulp - EPS)
    f_dentin_cor = np.minimum(f_dentin, hc)
    f_dentin_rad = np.minimum(f_dentin, below - EPS * 0.5)
    crest = 1.6
    f_pdl = np.minimum.reduce([-sd - EPS, 0.28 + sd, below - crest])

    L = ToothLayers(frame=frame)
    fb = faces_budget
    for name, fld, budget in [
        ("enamel", f_enamel, fb["enamel"]),
        ("dentin-coronal", f_dentin_cor, fb["dentin"]),
        ("dentin-radicular", f_dentin_rad, fb["dentin"]),
        ("cementum", f_cementum, fb["cementum"]),
        ("pdl", f_pdl, fb["pdl"]),
        ("pulp-chamber", np.minimum(f_pulp, (ZZ - (z_cej + floor + 0.4))) if multi else np.minimum(f_pulp, ZZ - (z_cej - 0.8)), fb["chamber"]),
    ]:
        m = surface(fld, lo, h, frame, budget)
        if m is not None:
            L.meshes[name] = m

    # canals: below the chamber region
    zcut = (z_cej + floor + 0.4) if multi else (z_cej - 0.8)
    for name, lab, cf in canal_fields:
        fld = np.minimum.reduce([smax(cf, np.full_like(cf, -1), 0.01), sd + 0.02, zcut + 0.3 - ZZ])
        m = surface(fld, lo, h, frame, fb["canal"], smooth=4)
        if m is not None:
            L.meshes[name] = m
        # apical foramen landmark: last point of the canal path
        pth = [p for n, _, p in canal_paths if n == name][0]
        L.landmarks[f"apical-foramen-{name[6:]}"] = frame.to_world(pth[-2])

    # pulp horns: local peaks of the chamber
    ch = L.meshes.get("pulp-chamber")
    if ch is not None:
        vloc = frame.to_local(ch.vertices)
        top = vloc[vloc[:, 2] > vloc[:, 2].max() - 1.6]
        order = np.argsort(-top[:, 2])
        peaks = []
        for p in top[order]:
            if all(np.linalg.norm(p[:2] - q[:2]) > 1.3 for q in peaks):
                peaks.append(p)
            if len(peaks) >= 5:
                break
        for i, p in enumerate(peaks):
            # direction label
            ang = np.degrees(np.arctan2(p[1], p[0]))
            L.landmarks[f"pulp-horn-{i + 1}"] = frame.to_world(p)

    L.roots = root_infos
    L.cej_height = float(z_cej)
    L.landmarks["cervical-line"] = frame.to_world(np.array([0, 0, z_cej]))
    L.landmarks["apex"] = frame.to_world(np.array([0, 0, z_apex]))
    return L


```

## Source file: src/engine/toothShading.test.ts

{
  "path": "src/engine/toothShading.test.ts",
  "blob_sha": "9acca5bbadec6db7eb75806ec0663d38b2e14d51",
  "sha256": "8fb349f6db4ff15c1f24ecb4990ec4d397a3548174cc8989a8304c74ac82582d",
  "lines": 94,
  "bytes": 3435,
  "selection_score": 13,
  "keyword_line_counts": {
    "teeth": 1,
    "tooth": 11,
    "rig": 1,
    "face": 2,
    "lip": 1
  },
  "url": "https://github.com/Yoosseph/dental-scope/blob/1a0615f384c5d069cac2de5c214952d8a15b5133/src/engine/toothShading.test.ts",
  "function_signature_candidates": []
}

```text

import * as THREE from 'three';
import { describe, expect, it } from 'vitest';
import type { Registry } from '../anatomy/registry';
import type { ManifestTooth } from '../anatomy/types';
import { createTissueMaterial } from './materials';
import { colorToothShell, shadeEnamelCrevices } from './toothShading';

/** An indexed, bumpy test surface: concave and convex vertices, like a real crown. */
function bumpyGeometry(): THREE.BufferGeometry {
  const geo = new THREE.IcosahedronGeometry(0.5, 3);
  const merged = mergeVertices(geo);
  const pos = merged.getAttribute('position');
  const v = new THREE.Vector3();
  for (let i = 0; i < pos.count; i++) {
    v.fromBufferAttribute(pos, i);
    v.multiplyScalar(1 + 0.08 * Math.sin(v.x * 23) * Math.cos(v.z * 17));
    pos.setXYZ(i, v.x, v.y, v.z);
  }
  merged.computeVertexNormals();
  return merged;
}

/** Minimal vertex welding (the test surface must be indexed). */
function mergeVertices(geo: THREE.BufferGeometry): THREE.BufferGeometry {
  const pos = geo.getAttribute('position');
  const map = new Map<string, number>();
  const verts: number[] = [];
  const index: number[] = [];
  for (let i = 0; i < pos.count; i++) {
    const key = [pos.getX(i), pos.getY(i), pos.getZ(i)].map((n) => n.toFixed(5)).join(',');
    let id = map.get(key);
    if (id === undefined) {
      id = verts.length / 3;
      map.set(key, id);
      verts.push(pos.getX(i), pos.getY(i), pos.getZ(i));
    }
    index.push(id);
  }
  const out = new THREE.BufferGeometry();
  out.setAttribute('position', new THREE.Float32BufferAttribute(verts, 3));
  out.setIndex(index);
  return out;
}

/** Order-sensitive digest of a colour attribute (rounded so float noise cannot flip it). */
function digest(attr: THREE.BufferAttribute | THREE.InterleavedBufferAttribute): { sum: number; weighted: number } {
  let sum = 0;
  let weighted = 0;
  for (let i = 0; i < attr.count * 3; i++) {
    const v = (attr.array as Float32Array)[i];
    sum += v;
    weighted += v * ((i % 97) + 1);
  }
  return { sum: Math.round(sum * 1e4) / 1e4, weighted: Math.round(weighted * 1e3) / 1e3 };
}

const tooth = {
  frame: { origin: [0, 0, 0], axis: [0, 1, 0], mesial: [1, 0, 0], buccal: [0, 0, 1] },
  landmarks: { 'cervical-line': [0, -0.1, 0] },
} as unknown as ManifestTooth;
const registry = { manifest: { teeth: { '36': tooth } } } as unknown as Registry;

describe('tooth shading', () => {
  it('colours a tooth shell from root to crown (unchanged output)', () => {
    const geo = bumpyGeometry();
    const mat = createTissueMaterial('shell');
    colorToothShell(geo, mat, registry, 36);
    expect(mat.userData.fx.uCervical.value.toArray()).toEqual([0, -0.1, 0]);
    expect(mat.userData.fx.uToothAxis.value.toArray()).toEqual([0, 1, 0]);
    expect(digest(geo.getAttribute('color'))).toMatchInlineSnapshot(`
      {
        "sum": 422.0161,
        "weighted": 20694.652,
      }
    `);
  });

  it('leaves a tooth without a cervical line uncoloured', () => {
    const geo = bumpyGeometry();
    colorToothShell(geo, createTissueMaterial('shell'), registry, 11);
    expect(geo.getAttribute('color')).toBeUndefined();
  });

  it('darkens enamel crevices (unchanged output)', () => {
    const geo = bumpyGeometry();
    shadeEnamelCrevices(geo);
    expect(digest(geo.getAttribute('color'))).toMatchInlineSnapshot(`
      {
        "sum": 475.9891,
        "weighted": 23279.674,
      }
    `);
  });
});


```

## Source file: src/app/seo.ts

{
  "path": "src/app/seo.ts",
  "blob_sha": "d33c3b60b98ac71a1da38d4c2185e4b81e487ddc",
  "sha256": "e2ac1525d5126302408bbc1d3b67ce2d6cafafa1696b0c30b267a255ca5319af",
  "lines": 237,
  "bytes": 11684,
  "selection_score": 0,
  "keyword_line_counts": {
    "jaw": 2,
    "mouth": 1,
    "teeth": 4,
    "tooth": 13,
    "face": 2
  },
  "url": "https://github.com/Yoosseph/dental-scope/blob/1a0615f384c5d069cac2de5c214952d8a15b5133/src/app/seo.ts",
  "function_signature_candidates": []
}

```text

/**
 * Search and sharing metadata. Pure functions shared by the build (vite.config.ts
 * writes the head tags, robots.txt, sitemap.xml and one static entry page per
 * tooth) and the app (document title while navigating).
 */
import { PERMANENT_FDI, fdiToPalmer, fdiToUniversal, toothName } from '../anatomy/notation.ts';
import type { Structure } from '../anatomy/types.ts';
import type { Lang } from '../i18n/lang.ts';
import { ABOUT_TEXT } from './about-text.ts';

export const SITE_NAME = 'Dental Scope';
export const AUTHOR = 'Yoseph';
export const REPOSITORY = 'https://github.com/Yoosseph/dental-scope';

export interface PageMeta {
  /** path below the site root, '' for home; tooth pages end with '/' (GitHub Pages serves folder/index.html) */
  path: string;
  title: string;
  description: string;
  /** page language (default English) */
  lang?: Lang;
  /** the same page in other languages, for hreflang links: language → path */
  alternates?: Partial<Record<Lang, string>>;
  /** breadcrumb trail after the site root (name + path); the page itself is appended */
  crumbs?: { name: string; path: string }[];
  /** the anatomical structure the page is about, for structured data */
  about?: { name: string; alternateName?: string[] };
  /** an educational text page (schema.org LearningResource) */
  learning?: boolean;
}

export const HOME: PageMeta = {
  path: '',
  title: `${SITE_NAME} – Interactive 3D dental anatomy`,
  description:
    'Free interactive 3D dental anatomy: all 32 permanent teeth with FDI, Universal and Palmer numbers, enamel to root canals, jaws and nerves. Educational tool.',
};

const ABOUT_ALTERNATES: Record<Lang, string> = { en: 'about/', sv: 'about/sv/', de: 'about/de/', es: 'about/es/', la: 'about/la/' };
const aboutPage = (lang: Lang): PageMeta => ({
  path: ABOUT_ALTERNATES[lang],
  title: ABOUT_TEXT[lang].title,
  description: ABOUT_TEXT[lang].description,
  lang,
  alternates: ABOUT_ALTERNATES,
});

export const ABOUT: PageMeta = aboutPage('en');
/** The about page in every language: English, Swedish, German, Spanish, Latin. */
export const ABOUT_PAGES: PageMeta[] = [ABOUT, aboutPage('sv'), aboutPage('de'), aboutPage('es'), aboutPage('la')];
const LOCALE: Record<Lang, string> = { en: 'en_US', sv: 'sv_SE', de: 'de_DE', es: 'es_ES', la: 'la_VA' };
const ABOUT_CRUMB: Record<Lang, string> = { en: 'About', sv: 'Om', de: 'Über', es: 'Acerca de', la: 'De opere' };

export const OG_IMAGE = { path: 'og-image.png', width: 1200, height: 630, alt: 'Dental Scope: 3D model of the jaws and permanent teeth' };

export function toothPage(fdi: number): PageMeta {
  const name = toothName(fdi);
  const numbers = `FDI ${fdi} · Universal ${fdiToUniversal(fdi)}`;
  return {
    path: `tooth/${fdi}/`,
    title: `${name} (${numbers}) — ${SITE_NAME}`,
    description: `Interactive 3D model of the ${name.toLowerCase()} (${numbers} · Palmer ${fdiToPalmer(fdi)}): crown, roots, pulp and canals. Educational reference.`,
  };
}

export const TOOTH_PAGES: PageMeta[] = PERMANENT_FDI.map(toothPage);

/** Title for the current selection (kept in sync by the router). */
export function documentTitle(sel: Pick<Structure, 'name' | 'tooth'> | undefined): string {
  if (!sel) return HOME.title;
  if (sel.tooth) return toothPage(sel.tooth.fdi).title;
  return `${sel.name} — ${SITE_NAME}`;
}

/** Absolute site root (with trailing slash), or null when not configured. */
export function normalizeSiteUrl(raw: string | undefined): string | null {
  const url = raw?.trim();
  if (!url || !/^https?:\/\/[^\s/]+/.test(url)) return null;
  return url.endsWith('/') ? url : `${url}/`;
}

const esc = (s: string) => s.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

export interface HeadOptions {
  /** SPA fallback page: noindex, no canonical URL */
  notFound?: boolean;
  /** Google Search Console verification token (content of the google-site-verification meta tag) */
  googleVerification?: string | null;
  /** keep this page out of the index (links are still followed) */
  noindex?: boolean;
}

/** JSON for a <script type="application/ld+json">, safe inside HTML. */
export const jsonLd = (data: unknown) => `<script type="application/ld+json">${JSON.stringify(data).replace(/</g, '\\u003c')}</script>`;

/** schema.org structured data for a page (a single @graph). */
export function structuredData(page: PageMeta, siteUrl: string): unknown {
  const url = siteUrl + page.path;
  const website = { '@id': `${siteUrl}#website` };
  const person = { '@type': 'Person', '@id': `${siteUrl}#author`, name: AUTHOR, url: 'https://github.com/Yoosseph' };
  const graph: unknown[] = [];
  if (page.path === '') {
    graph.push(
      {
        '@type': 'WebSite',
        ...website,
        name: SITE_NAME,
        url: siteUrl,
        description: page.description,
        inLanguage: 'en',
        publisher: { '@id': person['@id'] },
      },
      {
        '@type': 'WebApplication',
        '@id': `${siteUrl}#app`,
        name: SITE_NAME,
        url: siteUrl,
        description: page.description,
        applicationCategory: 'EducationalApplication',
        applicationSubCategory: 'Dental anatomy',
        operatingSystem: 'Any (modern web browser)',
        browserRequirements: 'Requires JavaScript and WebGL',
        isAccessibleForFree: true,
        offers: { '@type': 'Offer', price: '0', priceCurrency: 'USD' },
        image: siteUrl + OG_IMAGE.path,
        inLanguage: 'en',
        audience: { '@type': 'EducationalAudience', educationalRole: 'student', audienceType: 'Dental students, dental hygiene and assisting students, educators' },
        keywords: 'dental anatomy, tooth anatomy, 3D teeth, tooth numbering, FDI, Universal numbering, Palmer notation, root canals, dental education',
        author: { '@id': person['@id'] },
        sameAs: [REPOSITORY],
      },
      person,
    );
  } else {
    const crumbs: { name: string; url: string }[] = [{ name: SITE_NAME, url: siteUrl }];
    const isAbout = page.path.startsWith('about/');
    if (page.crumbs) crumbs.push(...page.crumbs.map((c) => ({ name: c.name, url: siteUrl + c.path })));
    else if (page.path.startsWith('tooth/')) crumbs.push({ name: 'Teeth', url: `${siteUrl}${ABOUT.path}#teeth` });
    crumbs.push({ name: isAbout ? ABOUT_CRUMB[page.lang ?? 'en'] : (page.about?.name ?? page.title.replace(` — ${SITE_NAME}`, '')), url });
    const fdi = /^tooth\/(\d{2})\/$/.exec(page.path);
    graph.push({
      '@type': isAbout ? 'AboutPage' : page.learning ? ['WebPage', 'LearningResource'] : 'WebPage',
      ...(page.learning ? { learningResourceType: 'Reference', educationalUse: 'Self-study', isAccessibleForFree: true, audience: { '@type': 'EducationalAudience', educationalRole: 'student' } } : {}),
      '@id': url,
      url,
      name: page.title,
      description: page.description,
      inLanguage: page.lang ?? 'en',
      isPartOf: website,
      primaryImageOfPage: siteUrl + OG_IMAGE.path,
      breadcrumb: {
        '@type': 'BreadcrumbList',
        itemListElement: crumbs.map((c, i) => ({ '@type': 'ListItem', position: i + 1, name: c.name, item: c.url })),
      },
      ...(page.about
        ? { about: { '@type': 'AnatomicalStructure', name: page.about.name, ...(page.about.alternateName ? { alternateName: page.about.alternateName } : {}) } }
        : fdi
        ? {
            about: {
              '@type': 'AnatomicalStructure',
              name: toothName(Number(fdi[1])),
              alternateName: [`FDI ${fdi[1]}`, `Universal ${fdiToUniversal(Number(fdi[1]))}`, `Palmer ${fdiToPalmer(Number(fdi[1]))}`],
              bodyLocation: 'Mouth',
            },
          }
        : {}),
    });
  }
  return { '@context': 'https://schema.org', '@graph': graph };
}

/**
 * `<head>` tags for one page. URL-based tags (canonical, og:url, og:image,
 * structured data) need an absolute site URL and are left out without one.
 */
export function headTags(page: PageMeta, siteUrl: string | null, opts: HeadOptions = {}): string {
  const tags = [
    `<title>${esc(page.title)}</title>`,
    `<meta name="description" content="${esc(page.description)}" />`,
    `<meta name="author" content="${AUTHOR}" />`,
    `<meta property="og:type" content="website" />`,
    `<meta property="og:site_name" content="${SITE_NAME}" />`,
    `<meta property="og:locale" content="${LOCALE[page.lang ?? 'en']}" />`,
    `<meta property="og:title" content="${esc(page.title)}" />`,
    `<meta property="og:description" content="${esc(page.description)}" />`,
    `<meta name="twitter:title" content="${esc(page.title)}" />`,
    `<meta name="twitter:description" content="${esc(page.description)}" />`,
  ];
  if (opts.googleVerification) tags.push(`<meta name="google-site-verification" content="${esc(opts.googleVerification)}" />`);
  if (opts.notFound) {
    // SPA fallback for other deep links: served with HTTP 404, so keep it out of the index and don't claim a URL
    tags.push(`<meta name="robots" content="noindex" />`, `<meta name="twitter:card" content="summary" />`);
  } else if (siteUrl) {
    const url = siteUrl + page.path;
    tags.push(
      `<meta name="robots" content="${opts.noindex ? 'noindex, follow' : 'index, follow, max-image-preview:large'}" />`,
      `<link rel="canonical" href="${esc(url)}" />`,
      `<meta property="og:url" content="${esc(url)}" />`,
      ...Object.entries(opts.noindex ? {} : (page.alternates ?? {})).map(([l, p]) => `<link rel="alternate" hreflang="${l}" href="${esc(siteUrl + p)}" />`),
      ...(!opts.noindex && page.alternates?.en ? [`<link rel="alternate" hreflang="x-default" href="${esc(siteUrl + page.alternates.en)}" />`] : []),
      `<meta property="og:image" content="${esc(siteUrl + OG_IMAGE.path)}" />`,
      `<meta property="og:image:width" content="${OG_IMAGE.width}" />`,
      `<meta property="og:image:height" content="${OG_IMAGE.height}" />`,
      `<meta property="og:image:alt" content="${esc(OG_IMAGE.alt)}" />`,
      `<meta name="twitter:card" content="summary_large_image" />`,
      `<meta name="twitter:image" content="${esc(siteUrl + OG_IMAGE.path)}" />`,
      jsonLd(structuredData(page, siteUrl)),
    );
  } else {
    tags.push(`<meta name="twitter:card" content="summary" />`);
  }
  return tags.join('\n    ');
}

export function robotsTxt(siteUrl: string | null, preview = false): string {
  // preview deployments must never be indexed
  if (preview) return 'User-agent: *\nDisallow: /\n';
  return `User-agent: *\nAllow: /\n${siteUrl ? `\nSitemap: ${siteUrl}sitemap.xml\n` : ''}`;
}

/** A sitemap entry: a path, or a path with its language versions (written as xhtml:link hreflang). */
export type SitemapEntry = string | { path: string; alternates?: Partial<Record<Lang, string>> };

export function sitemapXml(siteUrl: string, entries: SitemapEntry[], lastmod?: string): string {
  const mod = lastmod ? `<lastmod>${lastmod}</lastmod>` : '';
  const urls = entries
    .map((entry) => {
      const { path, alternates } = typeof entry === 'string' ? { path: entry, alternates: undefined } : entry;
      const alt = Object.entries(alternates ?? {});
      const links = alt.length > 1
        ? [...alt, ...(alternates?.en ? [['x-default', alternates.en]] : [])].map(([l, p]) => `<xhtml:link rel="alternate" hreflang="${l}" href="${esc(siteUrl + p)}"/>`).join('')
        : '';
      return `  <url><loc>${esc(siteUrl + path)}</loc>${mod}${links}</url>`;
    })
    .join('\n');
  return `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">\n${urls}\n</urlset>\n`;
}


```
