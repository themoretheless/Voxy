# Facial rendering source scan: Apiwit22/3D_TeethSegment

Repository: https://github.com/Apiwit22/3D_TeethSegment

Pinned source revision: d47d8faa19d30462caf5e58bb7c6dc32ad8b74a2

Discovery topic: teeth 3D

Description: โปรเจกต์นี้จัดทำขึ้นเพื่อพัฒนาระบบ การแยกส่วนฟันแบบ 3 มิติ (3D Tooth Segmentation)โดยใช้เทคนิค ปัญญาประดิษฐ์และ Deep Learning กับข้อมูลสแกนฟันในรูปแบบ 3 มิติ

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"jaw": ["3D_Project/mesh_label_colorizer/run_all_jaws_pipeline.py"], "face": ["3D_Project/checkfaces.py", "3D_Project/colorize_face.py", "3D_Project/colorize_ver2face.py", "3D_Project/src/dataloader/face_baseforimesh.py", "3D_Project/src/dataloader/face_baseforimesh2.py", "3D_Project/src/dataloader/faces_base.py", "3D_Project/src/dataloader/faces_twostream24.py"]}

## Source file: 3D_Project/checkfaces.py

{
  "path": "3D_Project/checkfaces.py",
  "blob_sha": "47f4c4c93564f5dc2692f1b54bd92410de90c06d",
  "sha256": "0120d39109b3a6c5862ca1ae15d29804e6f179dac4921f5ab0c9cd7e4d1a4749",
  "lines": 88,
  "bytes": 2694,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 6
  },
  "url": "https://github.com/Apiwit22/3D_TeethSegment/blob/d47d8faa19d30462caf5e58bb7c6dc32ad8b74a2/3D_Project/checkfaces.py",
  "function_signature_candidates": [
    {
      "line": 21,
      "text": "def count_faces(ply_path: Path) -> int:"
    },
    {
      "line": 28,
      "text": "def scan_subset(subset_dir: Path):"
    },
    {
      "line": 67,
      "text": "def main():"
    }
  ]
}

```text

# checkfaces.py
from __future__ import annotations

import sys
from pathlib import Path
import open3d as o3d

# =========================
# CONFIG
# =========================
COLORED_ROOT = Path(r"D:\Project_Gujabaa\3D_Project\mesh_label_colorizer\data\colored_ready")
TARGET_FACES = 16000
SUBSETS = ["data_part_1", "data_part_2", "data_part_3"]

# แสดงรายชื่อไฟล์ที่เจอ
SHOW_FOUND_FILES = True
PRINT_LIMIT = 50            # แสดงแค่ 50 ไฟล์แรกต่อ subset (กันยาวเกิน)
PRINT_RELATIVE = True       # แสดง path แบบ relative ต่อ COLORED_ROOT


def count_faces(ply_path: Path) -> int:
    mesh = o3d.io.read_triangle_mesh(str(ply_path))
    if mesh.is_empty():
        raise RuntimeError("empty or unreadable mesh")
    return int(len(mesh.triangles))


def scan_subset(subset_dir: Path):
    files = sorted([p for p in subset_dir.rglob("*.ply") if p.is_file()] +
                   [p for p in subset_dir.rglob("*.PLY") if p.is_file()])
    files = sorted({p.resolve() for p in files})  # กันซ้ำบน Windows

    print(f"\n[SCAN] {subset_dir}")
    print(f"[INFO] found ply = {len(files)}")

    if SHOW_FOUND_FILES:
        show = files[:PRINT_LIMIT]
        for p in show:
            if PRINT_RELATIVE:
                try:
                    rp = p.relative_to(COLORED_ROOT)
                    print(f"[FILE] {rp}")
                except Exception:
                    print(f"[FILE] {p}")
            else:
                print(f"[FILE] {p}")

        if len(files) > PRINT_LIMIT:
            print(f"[INFO] ... and {len(files) - PRINT_LIMIT} more files (limit={PRINT_LIMIT})")

    ok = ng = er = 0
    for p in files:
        try:
            n = count_faces(p)
            if n == TARGET_FACES:
                ok += 1
            else:
                ng += 1
                print(f"[NG] {p}  faces={n} (expected {TARGET_FACES})")
        except Exception as e:
            er += 1
            print(f"[ER] {p}  {e}")

    print(f"[DONE] ok={ok}  ng={ng}  err={er}  total={ok+ng+er}")


def main():
    if not COLORED_ROOT.exists():
        raise SystemExit(f"[ER] COLORED_ROOT not found: {COLORED_ROOT}")

    any_found = False
    for name in SUBSETS:
        subset_dir = COLORED_ROOT / name
        if not subset_dir.exists():
            print(f"[SKIP] missing: {subset_dir}")
            continue
        any_found = True
        scan_subset(subset_dir)

    if not any_found:
        print(f"[ER] none of SUBSETS exist under: {COLORED_ROOT}")


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        sys.exit(130)


```

## Source file: 3D_Project/colorize_face.py

{
  "path": "3D_Project/colorize_face.py",
  "blob_sha": "d385737792152f105a13cc5f8fb73bbadc741fbd",
  "sha256": "ed4cb8b3758414fd357083f6dd1f0035f7244eb57d2e9c82c858df3adeef1b1b",
  "lines": 276,
  "bytes": 9873,
  "selection_score": 15,
  "keyword_line_counts": {
    "jaw": 1,
    "gum": 1,
    "rig": 2,
    "face": 18
  },
  "url": "https://github.com/Apiwit22/3D_TeethSegment/blob/d47d8faa19d30462caf5e58bb7c6dc32ad8b74a2/3D_Project/colorize_face.py",
  "function_signature_candidates": [
    {
      "line": 48,
      "text": "def parse_fdi_from_name(stem_lower: str) -> int | None:"
    },
    {
      "line": 59,
      "text": "def is_gingiva_name(stem_lower: str) -> bool:"
    },
    {
      "line": 62,
      "text": "def infer_color_from_file(p: Path) -> tuple[int,int,int]:"
    },
    {
      "line": 71,
      "text": "def infer_arch_from_path_or_name(p: Path) -> str | None:"
    },
    {
      "line": 105,
      "text": "def get_vertex_xyz_normals(ply: PlyData) -> tuple[np.ndarray, np.ndarray]:"
    },
    {
      "line": 119,
      "text": "def get_face_indices(ply: PlyData) -> np.ndarray | None:"
    },
    {
      "line": 155,
      "text": "def build_colored_elements(in_path: Path) -> tuple[np.ndarray, np.ndarray | None]:"
    },
    {
      "line": 177,
      "text": "def write_ply(path: Path, v: np.ndarray, f: np.ndarray | None):"
    },
    {
      "line": 188,
      "text": "def merge_parts(parts: list[tuple[np.ndarray, np.ndarray | None]]) -> tuple[np.ndarray, np.ndarray | None]:"
    },
    {
      "line": 213,
      "text": "def list_case_dirs(root: Path) -> list[Path]:"
    },
    {
      "line": 218,
      "text": "def collect_arch_files(case_dir: Path) -> dict[str, list[Path]]:"
    },
    {
      "line": 229,
      "text": "def main():"
    }
  ]
}

```text

#---- ใส่สีให้ฟันแต่ละซี่และเหงือก RGB (หรือ RGBA) "เฉพาะ face" และ merge รวมกันเป็นกรามเดียว ----#
from __future__ import annotations
from pathlib import Path
import re
import numpy as np
from plyfile import PlyData, PlyElement

# ============================================================
# CONFIG (EDIT)
# ============================================================
IN_ROOT  = Path(r"D:\Project_Gujabaa\3D_Project\320_fix_resampled")
OUT_ROOT = Path(r"D:\Project_Gujabaa\3D_Project\320_fix_colored")   # output root for colored+merged parts (will create case subfolders)

SAVE_COLORED_PARTS = True
WRITE_BINARY = False

UPPER_PARTS_DIRNAME = "upper_parts"
LOWER_PARTS_DIRNAME = "lower_parts"

# --- Face color format ---
WRITE_ALPHA = True          # ถ้า True => face จะมี alpha ด้วย (เหมือน 007_U.ply)
DEFAULT_ALPHA = 255         # ค่า alpha ที่จะเขียน

# ============================================================
# COLOR MAP (RGB 0-255)  >>> RGB ONLY (NO ALPHA)
# ============================================================
FDI_RGB = {
    # Q1 upper right 11-18
    11:(255,0,0), 12:(255,128,0), 13:(255,200,0), 14:(255,255,0),
    15:(170,255,0), 16:(0,255,0), 17:(0,255,170), 18:(0,255,255),
    # Q2 upper left 21-28
    21:(255,60,60), 22:(255,150,60), 23:(255,200,60), 24:(240,240,0),
    25:(170,255,60), 26:(0,200,0), 27:(0,200,200), 28:(60,240,240),
    # Q3 lower left 31-38
    31:(255,100,140), 32:(255,150,170), 33:(255,190,170), 34:(255,230,150),
    35:(180,255,150), 36:(120,255,120), 37:(120,255,200), 38:(150,255,255),
    # Q4 lower right 41-48
    41:(255,120,160), 42:(255,170,170), 43:(255,210,170), 44:(255,240,170),
    45:(200,255,170), 46:(150,255,150), 47:(150,255,220), 48:(170,240,255),
}
GINGIVA_RGB = (255, 180, 200)

# ============================================================
# Filename parsing
# ============================================================
_RE_INTS = re.compile(r"(\d{2,})")

def parse_fdi_from_name(stem_lower: str) -> int | None:
    matches = _RE_INTS.findall(stem_lower)
    for m in matches:
        try:
            n = int(m)
            if 11 <= n <= 48 and n in FDI_RGB:
                return n
        except:
            pass
    return None

def is_gingiva_name(stem_lower: str) -> bool:
    return any(k in stem_lower for k in ("jaw", "gingiva", "gum", "lowerjaw", "upperjaw", "gingival"))

def infer_color_from_file(p: Path) -> tuple[int,int,int]:
    s = p.stem.lower()
    if is_gingiva_name(s):
        return GINGIVA_RGB
    fdi = parse_fdi_from_name(s)
    if fdi is not None:
        return FDI_RGB[fdi]
    return GINGIVA_RGB

def infer_arch_from_path_or_name(p: Path) -> str | None:
    parts_lower = [x.lower() for x in p.parts]
    if any(UPPER_PARTS_DIRNAME.lower() == x for x in parts_lower) or "upper" in p.as_posix().lower():
        return "upper"
    if any(LOWER_PARTS_DIRNAME.lower() == x for x in parts_lower) or "lower" in p.as_posix().lower():
        return "lower"
    s = p.stem.lower()
    fdi = parse_fdi_from_name(s)
    if fdi is not None:
        if 11 <= fdi <= 28: return "upper"
        if 31 <= fdi <= 48: return "lower"
    return None

# ============================================================
# DTYPE: vertex keeps xyz + normals ONLY (no colors)
#        face stores vertex_indices + (RGB or RGBA)
# ============================================================
VERT_DTYPE = np.dtype([
    ("x", "f4"), ("y", "f4"), ("z", "f4"),
    ("nx", "f4"), ("ny", "f4"), ("nz", "f4"),
])

if WRITE_ALPHA:
    FACE_DTYPE = np.dtype([
        ("vertex_indices", "i4", (3,)),
        ("red", "u1"), ("green", "u1"), ("blue", "u1"),
        ("alpha", "u1"),
    ])
else:
    FACE_DTYPE = np.dtype([
        ("vertex_indices", "i4", (3,)),
        ("red", "u1"), ("green", "u1"), ("blue", "u1"),
    ])

def get_vertex_xyz_normals(ply: PlyData) -> tuple[np.ndarray, np.ndarray]:
    v = ply["vertex"].data
    names = v.dtype.names or ()
    if not all(k in names for k in ("x","y","z")):
        raise ValueError("vertex missing xyz")

    xyz = np.stack([v["x"], v["y"], v["z"]], axis=1).astype(np.float32)

    if all(k in names for k in ("nx","ny","nz")):
        nrm = np.stack([v["nx"], v["ny"], v["nz"]], axis=1).astype(np.float32)
    else:
        nrm = np.zeros_like(xyz, dtype=np.float32)
    return xyz, nrm

def get_face_indices(ply: PlyData) -> np.ndarray | None:
    if "face" not in ply:
        return None
    f = ply["face"].data
    fnames = f.dtype.names or ()
    idx_field = None
    for cand in ("vertex_indices", "vertex_index", "indices"):
        if cand in fnames:
            idx_field = cand
            break
    if idx_field is None:
        raise ValueError("face element exists but no vertex_indices field")

    idx = f[idx_field]
    tris = []
    if idx.dtype == object:
        for poly in idx:
            poly = np.asarray(poly, dtype=np.int64).tolist()
            if len(poly) < 3:
                continue
            if len(poly) == 3:
                tris.append(poly)
            else:
                for i in range(1, len(poly)-1):
                    tris.append([poly[0], poly[i], poly[i+1]])
    else:
        idx = np.asarray(idx)
        if idx.ndim == 2 and idx.shape[1] >= 3:
            tris = idx[:, :3].astype(np.int64).tolist()
        else:
            raise ValueError("Unsupported face indices shape")

    if not tris:
        return None
    return np.asarray(tris, dtype=np.int64)

def build_colored_elements(in_path: Path) -> tuple[np.ndarray, np.ndarray | None]:
    ply = PlyData.read(str(in_path))
    xyz, nrm = get_vertex_xyz_normals(ply)
    r, g, b = infer_color_from_file(in_path)

    # vertex output: keep xyz + normals only
    vout = np.empty((xyz.shape[0],), dtype=VERT_DTYPE)
    vout["x"], vout["y"], vout["z"] = xyz[:,0], xyz[:,1], xyz[:,2]
    vout["nx"], vout["ny"], vout["nz"] = nrm[:,0], nrm[:,1], nrm[:,2]

    # face output: write color ONLY on face
    tris = get_face_indices(ply)
    if tris is None:
        return vout, None

    fout = np.empty((tris.shape[0],), dtype=FACE_DTYPE)
    fout["vertex_indices"] = tris.astype(np.int32)
    fout["red"], fout["green"], fout["blue"] = r, g, b
    if WRITE_ALPHA:
        fout["alpha"] = np.uint8(DEFAULT_ALPHA)
    return vout, fout

def write_ply(path: Path, v: np.ndarray, f: np.ndarray | None):
    elems = [PlyElement.describe(v, "vertex")]
    if f is not None:
        elems.append(PlyElement.describe(f, "face"))
    out = PlyData(elems, text=(not WRITE_BINARY))
    path.parent.mkdir(parents=True, exist_ok=True)
    out.write(str(path))

# ============================================================
# Merge (concat only, no weld)
# ============================================================
def merge_parts(parts: list[tuple[np.ndarray, np.ndarray | None]]) -> tuple[np.ndarray, np.ndarray | None]:
    v_all = []
    f_all = []
    v_offset = 0
    any_face = False

    for v, f in parts:
        v_all.append(v)
        if f is not None:
            any_face = True
            f2 = f.copy()
            f2["vertex_indices"] = (f2["vertex_indices"].astype(np.int64) + v_offset).astype(np.int32)
            f_all.append(f2)
        v_offset += v.shape[0]

    V = np.concatenate(v_all, axis=0) if v_all else np.empty((0,), dtype=VERT_DTYPE)
    if any_face and f_all:
        F = np.concatenate(f_all, axis=0)
    else:
        F = None
    return V, F

# ============================================================
# Walk cases
# ============================================================
def list_case_dirs(root: Path) -> list[Path]:
    if (root / UPPER_PARTS_DIRNAME).exists() or (root / LOWER_PARTS_DIRNAME).exists():
        return [root]
    return [p for p in sorted(root.iterdir()) if p.is_dir()]

def collect_arch_files(case_dir: Path) -> dict[str, list[Path]]:
    files = list(case_dir.rglob("*.ply"))
    upper, lower = [], []
    for p in files:
        arch = infer_arch_from_path_or_name(p)
        if arch == "upper":
            upper.append(p)
        elif arch == "lower":
            lower.append(p)
    return {"upper": sorted(upper), "lower": sorted(lower)}

def main():
    if not IN_ROOT.exists():
        raise FileNotFoundError(f"IN_ROOT not found: {IN_ROOT}")

    case_dirs = list_case_dirs(IN_ROOT)
    if not case_dirs:
        print(f"[WARN] no case dirs under {IN_ROOT}")
        return

    for case_dir in case_dirs:
        case_id = case_dir.name
        groups = collect_arch_files(case_dir)

        for arch in ("upper", "lower"):
            arch_files = groups[arch]
            if not arch_files:
                continue

            parts_colored = []
            for p in arch_files:
                try:
                    v, f = build_colored_elements(p)
                    parts_colored.append((v, f))

                    if SAVE_COLORED_PARTS:
                        rel = p.relative_to(case_dir)
                        out_part = OUT_ROOT / case_id / f"{arch}_colored_parts" / rel.name
                        write_ply(out_part, v, f)

                except Exception as e:
                    print(f"[FAIL] {case_id} {arch} {p.name}: {e}")

            if not parts_colored:
                continue

            V, F = merge_parts(parts_colored)

            suffix = "_U" if arch == "upper" else "_L"
            out_merged = OUT_ROOT / case_id / f"{case_id}{suffix}.ply"
            write_ply(out_merged, V, F)

            nf = 0 if F is None else len(F)
            print(f"[OK] MERGED {case_id} {arch}: parts={len(parts_colored)}  V={len(V)}  F={nf}  -> {out_merged}")

    print(f"[DONE] colored+merged saved to: {OUT_ROOT}")

if __name__ == "__main__":
    main()


```

## Source file: 3D_Project/colorize_ver2face.py

{
  "path": "3D_Project/colorize_ver2face.py",
  "blob_sha": "a8b701271d51d5172281d52d66af7807309a0fef",
  "sha256": "21f5f47ef34cd097aea84ee3ad7711e4b0e2a3e68607f1677a48f0333fefe224",
  "lines": 147,
  "bytes": 4689,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 7,
    "lip": 2
  },
  "url": "https://github.com/Apiwit22/3D_TeethSegment/blob/d47d8faa19d30462caf5e58bb7c6dc32ad8b74a2/3D_Project/colorize_ver2face.py",
  "function_signature_candidates": [
    {
      "line": 21,
      "text": "def load_mesh(p: Path) -> o3d.geometry.TriangleMesh:"
    },
    {
      "line": 34,
      "text": "def face_colors_from_vertex(VC_u8: np.ndarray, F: np.ndarray, method: str) -> np.ndarray:"
    },
    {
      "line": 60,
      "text": "def write_ply_like_sample(out_path: Path, V: np.ndarray, VN: np.ndarray, F: np.ndarray, FC: np.ndarray, alpha: int = 255):"
    },
    {
      "line": 71,
      "text": "F = F.astype(np.int32, copy=False)  # int indices (not uint)"
    },
    {
      "line": 100,
      "text": "def process_one(src: Path, dst: Path):"
    },
    {
      "line": 115,
      "text": "def main():"
    }
  ]
}

```text

# vertex2face_to_sample_format.py
from __future__ import annotations

import sys
from pathlib import Path
import numpy as np
import open3d as o3d

# =========================
# CONFIG (EDIT HERE)
# =========================
IN_ROOT  = Path(r"D:\Project_Gujabaa\3D_Project\mesh_label_colorizer\data\colored_ready")  # vertex-colored PLY
OUT_ROOT = Path(r"D:\Project_Gujabaa\3D_Project\data_part_colored_face")  # output

SUBSETS = ["data_part_1", "data_part_2", "data_part_3"]

ALPHA = 255
METHOD = "mode"  # "mode" (คม) หรือ "mean" (เนียน)


def load_mesh(p: Path) -> o3d.geometry.TriangleMesh:
    m = o3d.io.read_triangle_mesh(str(p))
    if m.is_empty():
        raise RuntimeError("empty/unreadable mesh")
    if len(m.triangles) == 0:
        raise RuntimeError("mesh has no faces")
    if not m.has_vertex_colors():
        raise RuntimeError("mesh has no vertex colors")
    if not m.has_vertex_normals():
        m.compute_vertex_normals()
    return m


def face_colors_from_vertex(VC_u8: np.ndarray, F: np.ndarray, method: str) -> np.ndarray:
    c0 = VC_u8[F[:, 0]]
    c1 = VC_u8[F[:, 1]]
    c2 = VC_u8[F[:, 2]]

    if method == "mean":
        fc = (c0.astype(np.float32) + c1.astype(np.float32) + c2.astype(np.float32)) / 3.0
        return np.clip(np.rint(fc), 0, 255).astype(np.uint8)

    if method == "mode":
        eq01 = np.all(c0 == c1, axis=1)
        eq02 = np.all(c0 == c2, axis=1)
        eq12 = np.all(c1 == c2, axis=1)

        out = np.empty_like(c0)
        m0 = eq01 | eq02
        out[m0] = c0[m0]
        m1 = (~m0) & eq12
        out[m1] = c1[m1]
        m2 = (~m0) & (~m1)
        out[m2] = c0[m2]
        return out.astype(np.uint8)

    raise ValueError("METHOD must be 'mode' or 'mean'")


def write_ply_like_sample(out_path: Path, V: np.ndarray, VN: np.ndarray, F: np.ndarray, FC: np.ndarray, alpha: int = 255):
    """
    Match sample header:
      format ascii 1.0
      vertex: float x y z nx ny nz
      face: list uchar int vertex_indices + uchar r g b a
    """
    out_path.parent.mkdir(parents=True, exist_ok=True)

    V = V.astype(np.float32, copy=False)
    VN = VN.astype(np.float32, copy=False)
    F = F.astype(np.int32, copy=False)  # int indices (not uint)
    FC = FC.astype(np.uint8, copy=False)
    a = int(alpha)

    with out_path.open("w", newline="\n") as f:
        f.write("ply\n")
        f.write("format ascii 1.0\n")
        f.write(f"element vertex {len(V)}\n")
        f.write("property float x\n")
        f.write("property float y\n")
        f.write("property float z\n")
        f.write("property float nx\n")
        f.write("property float ny\n")
        f.write("property float nz\n")
        f.write(f"element face {len(F)}\n")
        f.write("property list uchar int vertex_indices\n")
        f.write("property uchar red\n")
        f.write("property uchar green\n")
        f.write("property uchar blue\n")
        f.write("property uchar alpha\n")
        f.write("end_header\n")

        for (x, y, z), (nx, ny, nz) in zip(V, VN):
            f.write(f"{x:.6f} {y:.6f} {z:.6f} {nx:.6f} {ny:.6f} {nz:.6f}\n")

        for (i0, i1, i2), (r, g, b) in zip(F, FC):
            f.write(f"3 {int(i0)} {int(i1)} {int(i2)} {int(r)} {int(g)} {int(b)} {a}\n")


def process_one(src: Path, dst: Path):
    m = load_mesh(src)

    V = np.asarray(m.vertices, dtype=np.float64)
    VN = np.asarray(m.vertex_normals, dtype=np.float64)
    F = np.asarray(m.triangles, dtype=np.int64)

    VC = np.asarray(m.vertex_colors, dtype=np.float64)  # [0..1]
    VC_u8 = np.clip(np.rint(VC * 255.0), 0, 255).astype(np.uint8)

    FC = face_colors_from_vertex(VC_u8, F, METHOD)

    write_ply_like_sample(dst, V, VN, F, FC, alpha=ALPHA)


def main():
    OUT_ROOT.mkdir(parents=True, exist_ok=True)

    total = ok = fail = 0
    for subset in SUBSETS:
        in_subset = IN_ROOT / subset
        if not in_subset.exists():
            print(f"[SKIP] missing subset: {in_subset}")
            continue

        files = sorted({p.resolve() for p in in_subset.rglob("*.ply") if p.is_file()})
        print(f"\n[SUBSET] {subset} files={len(files)}")

        for src in files:
            rel = src.relative_to(IN_ROOT)      # keep same structure
            dst = OUT_ROOT / rel                # same filename
            total += 1
            try:
                process_one(src, dst)
                ok += 1
            except Exception as e:
                fail += 1
                print(f"[ER] {rel}: {e}")

    print(f"\n[DONE] total={total} ok={ok} fail={fail}")
    print(f"[OUT] {OUT_ROOT}")


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        sys.exit(130)


```

## Source file: 3D_Project/src/dataloader/faces_base.py

{
  "path": "3D_Project/src/dataloader/faces_base.py",
  "blob_sha": "9262d9341ef6b6b8f1fe8031d95f6c26b6d3a216",
  "sha256": "2c0cb61ad033f236261f07544ea7f19a727dabc4db6ea84f628849a14a9247a8",
  "lines": 393,
  "bytes": 16342,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 84,
    "lip": 1
  },
  "url": "https://github.com/Apiwit22/3D_TeethSegment/blob/d47d8faa19d30462caf5e58bb7c6dc32ad8b74a2/3D_Project/src/dataloader/faces_base.py",
  "function_signature_candidates": [
    {
      "line": 23,
      "text": "def face_centers(pos: np.ndarray, faces: np.ndarray) -> np.ndarray:"
    },
    {
      "line": 30,
      "text": "def face_normals(pos: np.ndarray, faces: np.ndarray) -> np.ndarray:"
    },
    {
      "line": 39,
      "text": "def face_area(pos: np.ndarray, faces: np.ndarray) -> np.ndarray:"
    },
    {
      "line": 47,
      "text": "def face_edge_len_mean(pos: np.ndarray, faces: np.ndarray) -> np.ndarray:"
    },
    {
      "line": 61,
      "text": "def build_face_neighbors(faces: np.ndarray, k: int) -> np.ndarray:"
    },
    {
      "line": 109,
      "text": "def neighbor_normal_variation(fn: np.ndarray, nbr: np.ndarray) -> np.ndarray:"
    },
    {
      "line": 164,
      "text": "def __init__(self, files: List[str], cfg: FaceConfig):"
    },
    {
      "line": 169,
      "text": "def from_config(cls, files: List[str], full_cfg: dict, arch: str):"
    },
    {
      "line": 195,
      "text": "def _needs_nbr(self, feat_name: str) -> bool:"
    },
    {
      "line": 206,
      "text": "def __getitem__(self, idx: int) -> Dict[str, Any]:"
    }
  ]
}

```text

# src/dataloader/faces_base.py
from __future__ import annotations

from dataclasses import dataclass
from typing import Dict, Any, List, Tuple, Optional

import numpy as np
import torch

from src.dataloader.base import BaseDentalDataset, BaseDentalDatasetConfig

# ============================================================
# NOTE:
# - ไฟล์นี้ "ตัดฟีเจอร์ 24D (two-stream / tgcn24)" ออกแล้ว
# - ถ้าต้องการฟีเจอร์ 24D สำหรับ TSGCNet ให้ใช้ไฟล์ใหม่:
#     src/dataloader/faces_twostream24.py
# ============================================================


# ============================================================
# Geometry helpers (NO color features)
# ============================================================
def face_centers(pos: np.ndarray, faces: np.ndarray) -> np.ndarray:
    v0 = pos[faces[:, 0]]
    v1 = pos[faces[:, 1]]
    v2 = pos[faces[:, 2]]
    return (v0 + v1 + v2) / 3.0


def face_normals(pos: np.ndarray, faces: np.ndarray) -> np.ndarray:
    v0 = pos[faces[:, 0]]
    v1 = pos[faces[:, 1]]
    v2 = pos[faces[:, 2]]
    n = np.cross(v1 - v0, v2 - v0)
    norm = np.linalg.norm(n, axis=1, keepdims=True) + 1e-12
    return (n / norm).astype(np.float32)


def face_area(pos: np.ndarray, faces: np.ndarray) -> np.ndarray:
    v0 = pos[faces[:, 0]]
    v1 = pos[faces[:, 1]]
    v2 = pos[faces[:, 2]]
    a = 0.5 * np.linalg.norm(np.cross(v1 - v0, v2 - v0), axis=1)
    return a.astype(np.float32)


def face_edge_len_mean(pos: np.ndarray, faces: np.ndarray) -> np.ndarray:
    v0 = pos[faces[:, 0]]
    v1 = pos[faces[:, 1]]
    v2 = pos[faces[:, 2]]
    e01 = np.linalg.norm(v1 - v0, axis=1)
    e12 = np.linalg.norm(v2 - v1, axis=1)
    e20 = np.linalg.norm(v0 - v2, axis=1)
    e = (e01 + e12 + e20) / 3.0
    return e.astype(np.float32)


# ============================================================
# Face-neighbor helpers (share-edge adjacency)
# ============================================================
def build_face_neighbors(faces: np.ndarray, k: int) -> np.ndarray:
    """
    สร้างเพื่อนบ้าน K ตัวต่อ 1 face โดยใช้ "แชร์ขอบ (edge)" เป็นเงื่อนไข
    - เรียงลำดับแบบ deterministic (กันผลลัพธ์สวิง)
    - ถ้าไม่มีเพื่อนบ้าน -> เติมเป็นตัวเอง (self)
    - ถ้ามีน้อยกว่า k -> pad ด้วย self

    Returns: (F, k) int64
    """
    faces = np.asarray(faces, dtype=np.int64)
    F = int(faces.shape[0])
    k = int(k)
    if F <= 0:
        return np.zeros((0, k), dtype=np.int64)

    edge_map: Dict[Tuple[int, int], List[int]] = {}
    for fi, (a, b, c) in enumerate(faces):
        a, b, c = int(a), int(b), int(c)
        for u, v in ((a, b), (b, c), (c, a)):
            if u > v:
                u, v = v, u
            edge_map.setdefault((u, v), []).append(fi)

    nbr = np.full((F, k), -1, dtype=np.int64)
    for fi, (a, b, c) in enumerate(faces):
        neigh = set()
        for u, v in ((int(a), int(b)), (int(b), int(c)), (int(c), int(a))):
            if u > v:
                u, v = v, u
            for fj in edge_map.get((u, v), []):
                if fj != fi:
                    neigh.add(int(fj))

        if not neigh:
            nbr[fi, :] = fi
            continue

        neigh = sorted(neigh)
        if len(neigh) >= k:
            nbr[fi, :] = np.asarray(neigh[:k], dtype=np.int64)
        else:
            nbr[fi, : len(neigh)] = np.asarray(neigh, dtype=np.int64)
            nbr[fi, len(neigh) :] = fi

    nbr[nbr < 0] = 0
    return nbr


def neighbor_normal_variation(fn: np.ndarray, nbr: np.ndarray) -> np.ndarray:
    """
    วัดความ "เปลี่ยนแปลงของ normal" เมื่อเทียบกับเพื่อนบ้าน
    fn:  (F,3) normalized
    nbr: (F,K) neighbor indices (self-filled)

    return: (F,) float32 => 1 - mean(dot(fn, fn_neighbor))
    """
    neigh = fn[nbr]  # (F,K,3)
    dots = np.sum(neigh * fn[:, None, :], axis=-1)  # (F,K)
    dots = np.clip(dots, -1.0, 1.0)
    return (1.0 - dots.mean(axis=1)).astype(np.float32)


# ============================================================
# Config
# ============================================================
@dataclass
class FaceConfig(BaseDentalDatasetConfig):
    """
    num_faces: จำนวน face ที่ต้องการ fix ให้เท่ากันทุกไฟล์ (ปกติ 16000)

    face_feature:
      - "center_normal"      ->  6ch: center(3) + face_normal(3)
      - "cn_area_edge_nvar"  ->  9ch: center+normal + area_norm + edge_norm + nvar
                               (ต้องใช้เพื่อนบ้าน share-edge)
      - "meshsegnet15"       -> 15ch: center(3) + face_normal(3) + (v0-c)(3) + (v1-c)(3) + (v2-c)(3)
                               (MeshSegNet-style geometry)

    return_faces: คืน faces (F,3)
    return_nbr:   คืน neighbor indices (F,K) (ใช้กับ MeshSegNet/EdgeConv หรือ debug)
    """
    num_faces: int = 16000
    feature: str = "center_normal"
    return_faces: bool = True
    return_nbr: bool = False
    k_neighbors: int = 3


# ============================================================
# Dataset
# ============================================================
class FaceDataset(BaseDentalDataset):
    """
    Dataset แบบ face-wise (หนึ่ง sample = mesh หนึ่งชิ้น)

    จุดสำคัญ:
      - x จะเป็น "geometry only" ไม่เอาสี (กัน color leakage)
      - y ใช้ policy ใน BaseDentalDatasetConfig (label_source + face_fallback)
      - Fix จำนวน faces:
          * mode="face"  -> สุ่ม sample ได้ (augment)
          * mode="graph" -> ตัดหน้าแรกแบบ deterministic (ช่วยให้ topology คงที่)
          * ถ้า F0 < target_F -> PAD และมี valid_face mask บอกหน้าไหนจริง
    """

    def __init__(self, files: List[str], cfg: FaceConfig):
        super().__init__(files, cfg)
        self.fcfg = cfg

    @classmethod
    def from_config(cls, files: List[str], full_cfg: dict, arch: str):
        data = full_cfg["data"]
        num_classes = int(full_cfg["model"]["kwargs"].get("num_classes", 16))

        base = FaceConfig(
            mode="face",
            arch=arch,
            normalize=bool(data.get("normalize", True)),
            align_pca=bool(data.get("align_pca", False)),
            ignore_index=int(data.get("ignore_index", -1)),
            unknown_policy=str(data.get("unknown_policy", "raise")),
            require_labels=bool(data.get("require_labels", True)),
            num_classes=num_classes,

            # label policy (สำคัญมาก)
            label_source=str(data.get("label_source", "auto")),
            face_fallback=str(data.get("face_fallback", "majority")),

            num_faces=int(data.get("num_faces", 16000)),
            feature=str(data.get("face_feature", "center_normal")),
            return_faces=bool(data.get("return_faces", True)),
            return_nbr=bool(data.get("return_nbr", False)),
            k_neighbors=int(data.get("k_neighbors", 3)),
        )
        return cls(files, base)

    def _needs_nbr(self, feat_name: str) -> bool:
        """
        ตัดสินใจว่าจะต้องสร้าง neighbor indices (nbr) ไหม
        - cn9 ต้องใช้ nbr เพื่อคำนวณ nvar
        - ที่เหลือจะสร้างก็ต่อเมื่อผู้ใช้ตั้ง return_nbr=True
        """
        feat_name = str(feat_name).lower()
        if feat_name in ("cn_area_edge_nvar", "cn9", "center_normal_9ch"):
            return True
        return bool(self.fcfg.return_nbr)

    def __getitem__(self, idx: int) -> Dict[str, Any]:
        path = self.files[idx]
        arch = self.get_arch(path)
        mesh = self.load_mesh(path)

        if mesh.faces is None:
            raise ValueError(f"PLY has no triangular faces but mode=face: {path}")

        pos_v = mesh.pos.astype(np.float32, copy=False)         # (Nv,3)
        faces_all = mesh.faces.astype(np.int64, copy=False)     # (F0,3)

        F0 = int(faces_all.shape[0])
        target_F = int(self.fcfg.num_faces)
        if F0 <= 0:
            raise ValueError(f"Empty faces in: {path}")

        # ------------------------------------------------------------
        # Fix จำนวน face ให้เท่ากันทุกไฟล์
        # ------------------------------------------------------------
        cropped = False
        if F0 >= target_F:
            mode = str(self.cfg.mode).lower()
            if mode == "graph":
                # graph-mode: ตัดหน้าแรกแบบ deterministic
                fidx = np.arange(target_F, dtype=np.int64)
                cropped = (F0 > target_F)
            else:
                # face-mode: สุ่มเพื่อ augmentation
                fidx = self.sample_indices(F0, target_F)
                cropped = (F0 > target_F)

            faces = faces_all[fidx]
            valid = np.ones((target_F,), dtype=np.bool_)
            padded = False
            F_used = target_F
        else:
            # face น้อยกว่าเป้าหมาย -> pad
            fidx = None
            faces = faces_all
            valid = np.zeros((target_F,), dtype=np.bool_)
            valid[:F0] = True
            padded = True
            F_used = F0

        # ------------------------------------------------------------
        # labels (ใช้ policy ใน base.py: label_source + face_fallback)
        # ------------------------------------------------------------
        if not self.cfg.require_labels:
            y_all = np.full((F0,), int(self.cfg.ignore_index), dtype=np.int64)
            gt_source = "disabled(require_labels=false)"
        else:
            if mesh.face_rgb is not None and str(self.cfg.label_source).lower() != "vertex":
                gt_source = "face_rgb"
            elif mesh.rgb is not None:
                gt_source = "vertex_fallback"
            else:
                gt_source = "missing_colors"
            y_all = self.get_labels_for_mode(mesh, arch)  # (F0,)

        y = y_all[fidx] if fidx is not None else y_all
        y = y.astype(np.int64, copy=False)

        # ------------------------------------------------------------
        # features (NO color)
        # ------------------------------------------------------------
        feat_name = str(self.fcfg.feature).lower()

        centers = face_centers(pos_v, faces).astype(np.float32, copy=False)   # (F,3)
        fn_face = face_normals(pos_v, faces).astype(np.float32, copy=False)  # (F,3)

        # ---- สร้าง nbr เมื่อจำเป็น
        need_nbr = self._needs_nbr(feat_name)
        if need_nbr:
            nbr: Optional[np.ndarray] = build_face_neighbors(faces, k=int(self.fcfg.k_neighbors)).astype(np.int64, copy=False)
        else:
            nbr = None

        if feat_name in ("meshsegnet15", "msn15", "mesh15"):
            # MeshSegNet-style 15D:
            # [center(3), face_normal(3), (v0-center)(3), (v1-center)(3), (v2-center)(3)]
            v0 = pos_v[faces[:, 0]]
            v1 = pos_v[faces[:, 1]]
            v2 = pos_v[faces[:, 2]]

            dv0 = (v0 - centers).astype(np.float32, copy=False)
            dv1 = (v1 - centers).astype(np.float32, copy=False)
            dv2 = (v2 - centers).astype(np.float32, copy=False)

            x = np.concatenate([centers, fn_face, dv0, dv1, dv2], axis=1).astype(np.float32, copy=False)  # (F,15)

        elif feat_name in ("center_normal", "cn", "cn6"):
            x = np.concatenate([centers, fn_face], axis=1).astype(np.float32, copy=False)  # (F,6)

        elif feat_name in ("cn_area_edge_nvar", "cn9", "center_normal_9ch"):
            # cn9 ต้องมี nbr เพื่อคำนวณ nvar
            if nbr is None:
                nbr = build_face_neighbors(faces, k=int(self.fcfg.k_neighbors)).astype(np.int64, copy=False)

            area = face_area(pos_v, faces)
            edge = face_edge_len_mean(pos_v, faces)
            area_n = area / (float(area.mean()) + 1e-9)
            edge_n = edge / (float(edge.mean()) + 1e-9)
            nvar = neighbor_normal_variation(fn_face, nbr)

            x = np.concatenate(
                [centers, fn_face, area_n[:, None], edge_n[:, None], nvar[:, None]],
                axis=1,
            ).astype(np.float32, copy=False)  # (F,9)

        else:
            raise ValueError(
                f"Unknown face_feature='{self.fcfg.feature}'. "
                f"Use 'center_normal' or 'cn_area_edge_nvar' or 'meshsegnet15'. "
                f"(ฟีเจอร์ 24D ถูกย้ายไป faces_twostream24.py แล้ว)"
            )

        # ------------------------------------------------------------
        # PAD to fixed target_F (if needed)
        # ------------------------------------------------------------
        if padded:
            C = int(x.shape[1])

            x_pad = np.zeros((target_F, C), dtype=np.float32)
            y_pad = np.full((target_F,), int(self.cfg.ignore_index), dtype=np.int64)
            centers_pad = np.zeros((target_F, 3), dtype=np.float32)
            faces_pad = np.zeros((target_F, 3), dtype=np.int64)

            x_pad[:F_used] = x
            y_pad[:F_used] = y
            centers_pad[:F_used] = centers
            faces_pad[:F_used] = faces

            x = x_pad
            y = y_pad
            centers = centers_pad
            faces = faces_pad

            if nbr is not None:
                K = int(self.fcfg.k_neighbors)
                nbr_pad = np.zeros((target_F, K), dtype=np.int64)
                nbr_pad[:F_used] = nbr
                # pad ส่วนที่เหลือให้ชี้ตัวเอง (กัน index หลุด)
                for i in range(F_used, target_F):
                    nbr_pad[i, :] = i
                nbr = nbr_pad

        # ------------------------------------------------------------
        # masks
        # ------------------------------------------------------------
        ign = int(self.cfg.ignore_index)
        mask = valid & (y != ign)

        out: Dict[str, Any] = {
            "x": torch.from_numpy(x),                                   # (F, C)
            "y": torch.from_numpy(y),                                   # (F,)
            "mask": torch.from_numpy(mask.astype(np.bool_)),            # (F,)
            "valid_face": torch.from_numpy(valid.astype(np.bool_)),     # (F,)
            "F_used": int(F_used),
            "arch": arch,
            "path": path,
            "pos": torch.from_numpy(centers),                           # (F,3) centers
            "meta": {
                **mesh.meta,
                "mode": str(self.cfg.mode).lower(),
                "arch": arch,
                "gt_source": gt_source,
                "face_feature": feat_name,
                "F_raw": int(F0),
                "F_used": int(F_used),
                "F_target": int(target_F),
                "padded": bool(padded),
                "cropped": bool(cropped),
                "label_source": str(self.cfg.label_source),
                "face_fallback": str(self.cfg.face_fallback),
                "uses_vertex_normals": False,
                "has_nbr": bool(nbr is not None),
                "k_neighbors": int(self.fcfg.k_neighbors),
            },
        }

        if bool(self.fcfg.return_faces):
            out["faces"] = torch.from_numpy(faces.astype(np.int64))

        # คืน nbr เฉพาะตอนที่ผู้ใช้ร้องขอจริง ๆ และเราสร้างมาแล้ว
        if bool(self.fcfg.return_nbr) and nbr is not None:
            out["nbr"] = torch.from_numpy(nbr.astype(np.int64))

        return out

```
