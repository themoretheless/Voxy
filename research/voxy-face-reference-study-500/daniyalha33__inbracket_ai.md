# Facial rendering source scan: daniyalha33/inbracket_ai

Repository: https://github.com/daniyalha33/inbracket_ai

Pinned source revision: deef4f0f348d79d468195c38e4aacd2f3d23d8dc

Discovery topic: teeth 3D

Description: 3d teeth segmentation

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: runner.py

{
  "path": "runner.py",
  "blob_sha": "878ecf26eabc56bb885750b6da25e47a94248bfa",
  "sha256": "37b5243ae264c254d8867547df17766842cbd31875163beeebb4f5fede1be286",
  "lines": 57,
  "bytes": 1945,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/daniyalha33/inbracket_ai/blob/deef4f0f348d79d468195c38e4aacd2f3d23d8dc/runner.py",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "def collate_fn(batch):"
    },
    {
      "line": 21,
      "text": "def get_mesh_path(basename):"
    },
    {
      "line": 26,
      "text": "def get_generator_set(config, is_test=False):"
    },
    {
      "line": 52,
      "text": "def runner(config, model):"
    }
  ]
}

```text

from trainer import Trainer
from generator import DentalModelGenerator
from torch.utils.data import DataLoader
import os
import torch

def collate_fn(batch):
    output = {}

    for batch_item in batch:
        for key in batch_item.keys():
            if key not in output:
                output[key] = []
            output[key].append(batch_item[key])
    
    for output_key in output.keys():
        if output_key in ["feat", "gt_seg_label", "uniform_feat", "uniform_gt_seg_label"]:
            output[output_key] = torch.stack(output[output_key])
    return output

def get_mesh_path(basename):
    case_name = basename.split("_")[0]
    file_name = basename.split("_")[0]+"_"+basename.split("_")[1]+".obj"
    return os.path.join("all_datas", "chl", "3D_scans_per_patient_obj_files", f"{case_name}", file_name)

def get_generator_set(config, is_test=False):
    if not is_test:
        point_loader = DataLoader(
            DentalModelGenerator(
                config["input_data_dir_path"], 
                aug_obj_str=config["aug_obj_str"],
                split_with_txt_path=config["train_data_split_txt_path"]
            ), 
            shuffle=True,
            batch_size=config["train_batch_size"],
            collate_fn=collate_fn
        )

        val_point_loader = DataLoader(
            DentalModelGenerator(
                config["input_data_dir_path"], 
                aug_obj_str=None,
                split_with_txt_path=config["val_data_split_txt_path"]

            ), 
            shuffle=False,
            batch_size=config["val_batch_size"],
            collate_fn= collate_fn
        )
        return [point_loader, val_point_loader]

def runner(config, model):
    gen_set = [get_generator_set(config["generator"], False)]
    print("train_set", len(gen_set[0][0]))
    print("validation_set", len(gen_set[0][1]))
    trainner = Trainer(config=config, model = model, gen_set=gen_set)
    trainner.run()

```

## Source file: server.py

{
  "path": "server.py",
  "blob_sha": "712994129f697d7a932c2fb6a9ef54f0f4871f8f",
  "sha256": "0ae9fca68ee312d4d155f35c6a027ad557c6ffe0a63db412c2819ae0696fbc5a",
  "lines": 293,
  "bytes": 10628,
  "selection_score": 0,
  "keyword_line_counts": {
    "jaw": 10,
    "teeth": 3,
    "rig": 3,
    "face": 3,
    "lip": 9
  },
  "url": "https://github.com/daniyalha33/inbracket_ai/blob/deef4f0f348d79d468195c38e4aacd2f3d23d8dc/server.py",
  "function_signature_candidates": [
    {
      "line": 44,
      "text": "def prepare_output_files(cache_dir: Path, output_dir: Path):"
    },
    {
      "line": 53,
      "text": "def dedup_and_save(input_path: Path, output_path: Path, flip: bool = False):"
    },
    {
      "line": 105,
      "text": "async def root():"
    },
    {
      "line": 118,
      "text": "async def health_check():"
    },
    {
      "line": 143,
      "text": "async def segment("
    },
    {
      "line": 257,
      "text": "async def get_output(filename: str):"
    },
    {
      "line": 280,
      "text": "async def clear_cache():"
    }
  ]
}

```text

"""
FastAPI Server for 3D Teeth Segmentation
"""

from fastapi import FastAPI, UploadFile, File, HTTPException
from fastapi.responses import FileResponse
from fastapi.middleware.cors import CORSMiddleware
import subprocess
import os
import sys
import shutil
from pathlib import Path
import logging
import numpy as np

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

app = FastAPI(title="3D Teeth Segmentation API")

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

PROJECT_ROOT = Path("/content/inbracket_ai")
CACHE_DIR = Path("/content/inbracket_ai/cache")
UPLOAD_DIR = Path("/content/inbracket_ai/uploads")
OUTPUT_DIR = Path("/content/inbracket_ai/output")
CHECKPOINT_PATH_FPS = Path(os.environ.get("CHECKPOINT_PATH_FPS", "/content/tgnet_fps"))
CHECKPOINT_PATH_BDL = Path(os.environ.get("CHECKPOINT_PATH_BDL", "/content/tgnet_bdl"))
START_TEST_SCRIPT = PROJECT_ROOT / "start_test.py"

CACHE_DIR.mkdir(parents=True, exist_ok=True)
UPLOAD_DIR.mkdir(parents=True, exist_ok=True)
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

sys.path.append(str(PROJECT_ROOT))


def prepare_output_files(cache_dir: Path, output_dir: Path):
    """
    Process cache files and save clean versions to output dir:
    - Lower: load → dedup → save
    - Upper: load → dedup → flip back (x,z negate) → save
    - JSON:  copy as-is (labels already match deduplicated vertices)
    """
    import gen_utils as gu

    def dedup_and_save(input_path: Path, output_path: Path, flip: bool = False):
        result = gu.read_txt_obj_ls(str(input_path), ret_mesh=True, use_tri_mesh=True)
        mesh = result[1]
        mesh = mesh.remove_duplicated_vertices()
        mesh.compute_vertex_normals()

        vertices = np.asarray(mesh.vertices).copy()
        faces = np.asarray(mesh.triangles)
        normals = np.asarray(mesh.vertex_normals)

        # Flip upper jaw back to original orientation
        if flip:
            rot_mat = np.array([[-1, 0, 0], [0, 1, 0], [0, 0, -1]])
            vertices = vertices @ rot_mat.T

        # Write clean OBJ manually to preserve exact vertex count
        with open(output_path, 'w') as f:
            f.write("# Processed by InBracket AI\n")
            for v in vertices:
                f.write(f"v {v[0]:.6f} {v[1]:.6f} {v[2]:.6f}\n")
            for vn in normals:
                f.write(f"vn {vn[0]:.6f} {vn[1]:.6f} {vn[2]:.6f}\n")
            for face in faces:
                f.write(f"f {face[0]+1} {face[1]+1} {face[2]+1}\n")

        v_count = len(vertices)
        logger.info(f"✅ Saved {output_path.name} ({v_count} vertices, flip={flip})")
        return v_count

    # Process lower jaw (dedup only)
    lower_v = dedup_and_save(
        cache_dir / "input_lower.obj",
        output_dir / "lower.obj",
        flip=False
    )

    # Process upper jaw (dedup + flip back to original orientation)
    upper_v = dedup_and_save(
        cache_dir / "input_upper.obj",
        output_dir / "upper.obj",
        flip=True
    )

    # Copy JSON files as-is (labels match deduplicated vertex count)
    shutil.copy(cache_dir / "input_lower.json", output_dir / "lower.json")
    shutil.copy(cache_dir / "input_upper.json", output_dir / "upper.json")
    logger.info("✅ Copied JSON label files")

    return lower_v, upper_v


@app.get("/")
async def root():
    return {
        "status": "running",
        "service": "3D Teeth Segmentation API",
        "endpoints": {
            "/segment": "POST - Upload lower and upper jaw OBJ files",
            "/outputs/{filename}": "GET - Download output files",
            "/health": "GET - Health check"
        }
    }


@app.get("/health")
async def health_check():
    checks = {
        "start_test_exists": START_TEST_SCRIPT.exists(),
        "checkpoint_fps_exists": (Path(str(CHECKPOINT_PATH_FPS) + ".h5")).exists(),
        "checkpoint_bdl_exists": (Path(str(CHECKPOINT_PATH_BDL) + ".h5")).exists(),
        "cache_dir_exists": CACHE_DIR.exists(),
        "upload_dir_exists": UPLOAD_DIR.exists(),
        "output_dir_exists": OUTPUT_DIR.exists(),
    }
    return {
        "status": "healthy" if all(checks.values()) else "unhealthy",
        "checks": checks,
        "resolved_paths": {
            "project_root": str(PROJECT_ROOT),
            "cache_dir": str(CACHE_DIR),
            "upload_dir": str(UPLOAD_DIR),
            "output_dir": str(OUTPUT_DIR),
            "checkpoint_fps": str(CHECKPOINT_PATH_FPS) + ".h5",
            "checkpoint_bdl": str(CHECKPOINT_PATH_BDL) + ".h5",
            "start_test": str(START_TEST_SCRIPT)
        }
    }


@app.post("/segment")
async def segment(
    lower: UploadFile = File(..., description="Lower jaw OBJ file"),
    upper: UploadFile = File(..., description="Upper jaw OBJ file")
):
    if not lower.filename.endswith('.obj'):
        raise HTTPException(status_code=400, detail="Lower jaw file must be .obj format")
    if not upper.filename.endswith('.obj'):
        raise HTTPException(status_code=400, detail="Upper jaw file must be .obj format")

    lower_path = UPLOAD_DIR / "lower.obj"
    upper_path = UPLOAD_DIR / "upper.obj"

    try:
        # Save uploaded files
        with open(lower_path, "wb") as f:
            content = await lower.read()
            f.write(content)
            logger.info(f"✅ Saved lower jaw: {len(content)} bytes → {lower_path}")

        with open(upper_path, "wb") as f:
            content = await upper.read()
            f.write(content)
            logger.info(f"✅ Saved upper jaw: {len(content)} bytes → {upper_path}")

        if not lower_path.exists():
            raise HTTPException(status_code=500, detail=f"lower.obj not found at {lower_path}")
        if not upper_path.exists():
            raise HTTPException(status_code=500, detail=f"upper.obj not found at {upper_path}")

        logger.info("Starting inference...")

        cmd = [
            "python", str(START_TEST_SCRIPT),
            "--input_lower_path", str(lower_path),
            "--input_upper_path", str(upper_path),
            "--cache_path", str(CACHE_DIR),
            "--output_path", str(OUTPUT_DIR),
            "--checkpoint_path", str(CHECKPOINT_PATH_FPS),
            "--checkpoint_path_bdl", str(CHECKPOINT_PATH_BDL)
        ]

        logger.info(f"Running: {' '.join(cmd)}")
        result = subprocess.run(cmd, check=True, capture_output=True, text=True)
        logger.info(f"Stdout: {result.stdout}")
        if result.stderr:
            logger.warning(f"Stderr: {result.stderr}")

        # Check inference outputs exist in cache
        required_cache_files = {
            "input_lower.obj":  CACHE_DIR / "input_lower.obj",
            "input_upper.obj":  CACHE_DIR / "input_upper.obj",
            "input_lower.json": CACHE_DIR / "input_lower.json",
            "input_upper.json": CACHE_DIR / "input_upper.json",
        }
        missing = [name for name, path in required_cache_files.items() if not path.exists()]
        if missing:
            logger.error(f"Missing cache files: {missing}")
            raise HTTPException(
                status_code=500,
                detail=f"Inference done but cache files missing: {missing}"
            )

        # Process and save clean output files
        logger.info("Processing output files...")
        lower_v, upper_v = prepare_output_files(CACHE_DIR, OUTPUT_DIR)

        # Verify output files exist
        required_output_files = {
            "lower.obj":  OUTPUT_DIR / "lower.obj",
            "upper.obj":  OUTPUT_DIR / "upper.obj",
            "lower.json": OUTPUT_DIR / "lower.json",
            "upper.json": OUTPUT_DIR / "upper.json",
        }
        missing_outputs = [name for name, path in required_output_files.items() if not path.exists()]
        if missing_outputs:
            raise HTTPException(
                status_code=500,
                detail=f"Output processing failed, missing: {missing_outputs}"
            )

        logger.info("✅ All output files ready")

        return {
            "status": "success",
            "message": "Segmentation completed successfully",
            "outputs": {
                "lower_obj":    "/outputs/lower.obj",
                "upper_obj":    "/outputs/upper.obj",
                "lower_labels": "/outputs/lower.json",
                "upper_labels": "/outputs/upper.json",
            },
            "vertex_counts": {
                "lower": lower_v,
                "upper": upper_v,
            },
            "notes": {
                "lower": "vertex count matches lower.json labels exactly",
                "upper": "vertex count matches upper.json labels exactly, orientation restored",
                "visualization": "map label index to color using FDI palette, 0=gingiva"
            },
            "download_instructions": "Use GET /outputs/{filename} to download each file"
        }

    except subprocess.CalledProcessError as e:
        logger.error(f"❌ Inference failed stdout: {e.stdout}")
        logger.error(f"❌ Inference failed stderr: {e.stderr}")
        raise HTTPException(status_code=500, detail=f"Segmentation failed: {e.stderr or str(e)}")

    except Exception as e:
        logger.error(f"❌ Unexpected error: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@app.get("/outputs/{filename}")
async def get_output(filename: str):
    allowed_files = [
        "lower.obj",   # clean deduplicated lower mesh
        "upper.obj",   # clean deduplicated upper mesh (flipped back)
        "lower.json",  # lower labels + instances
        "upper.json",  # upper labels + instances
    ]

    if filename not in allowed_files:
        raise HTTPException(status_code=400, detail=f"Invalid filename. Allowed: {allowed_files}")

    file_path = OUTPUT_DIR / filename
    if file_path.exists():
        return FileResponse(
            path=file_path,
            filename=filename,
            media_type="application/octet-stream"
        )

    raise HTTPException(status_code=404, detail=f"File not found: {filename}")


@app.delete("/cache")
async def clear_cache():
    try:
        for directory in [CACHE_DIR, UPLOAD_DIR, OUTPUT_DIR]:
            for file in directory.glob("*"):
                if file.is_file():
                    file.unlink()
        return {"status": "success", "message": "Cache cleared"}
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8000)


```

## Source file: trainer.py

{
  "path": "trainer.py",
  "blob_sha": "d94bcdc8870536d3e499b1899b90e4990a287274",
  "sha256": "dc29ba825d93d50e1a74bafee3316660db2947d75127e51bdf3372e0ee15bbfb",
  "lines": 102,
  "bytes": 4187,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/daniyalha33/inbracket_ai/blob/deef4f0f348d79d468195c38e4aacd2f3d23d8dc/trainer.py",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "def __init__(self, config = None, model=None, gen_set=None):"
    },
    {
      "line": 27,
      "text": "def train(self, epoch, data_loader):"
    },
    {
      "line": 50,
      "text": "def test(self, epoch, data_loader, save_best_model):"
    },
    {
      "line": 66,
      "text": "def train_depr(self):"
    },
    {
      "line": 84,
      "text": "def test_depr(self):"
    },
    {
      "line": 97,
      "text": "def run(self):"
    }
  ]
}

```text

import torch
import wandb
from loss_meter import LossMeter
from math import inf

class Trainer:
    def __init__(self, config = None, model=None, gen_set=None):
        self.gen_set = gen_set
        self.config = config
        self.model = model

        self.val_count = 0
        self.train_count = 0
        self.step_count = 0
        if config["wandb"]["wandb_on"]:
            wandb.init(
            entity=self.config["wandb"]["entity"],
            project=self.config["wandb"]["project"],
            notes=self.config["wandb"]["notes"],
            tags=self.config["wandb"]["tags"],
            name=self.config["wandb"]["name"],
            config=self.config,
            )

        self.best_val_loss = inf

    def train(self, epoch, data_loader):
        total_loss_meter = LossMeter()
        step_loss_meter =  LossMeter()
        pre_step = self.step_count
        for batch_idx, batch_item in enumerate(data_loader):
            loss = self.model.step(batch_idx, batch_item, "train")
            torch.cuda.empty_cache()
            total_loss_meter.aggr(loss.get_loss_dict_for_print("train"))
            step_loss_meter.aggr(loss.get_loss_dict_for_print("step"))
            print(loss.get_loss_dict_for_print("step"))
            if ((batch_idx+1) % self.config["tr_set"]["scheduler"]["schedueler_step"] == 0) or (self.step_count == pre_step and batch_idx == len(data_loader)-1):
                if self.config["wandb"]["wandb_on"]:
                    wandb.log(step_loss_meter.get_avg_results(), step=self.step_count)
                    wandb.log({"step_lr": self.model.scheduler.get_last_lr()[0]}, step = self.step_count)
                self.step_count +=1
                self.model.scheduler.step(self.step_count)
                step_loss_meter.init()
                
        if self.config["wandb"]["wandb_on"]:
            wandb.log(total_loss_meter.get_avg_results(), step = self.step_count)
            self.train_count += 1
        self.model.save("train")

    def test(self, epoch, data_loader, save_best_model):
        total_loss_meter = LossMeter()
        for batch_idx, batch_item in enumerate(data_loader):
            loss = self.model.step(batch_idx, batch_item, "test")
            total_loss_meter.aggr(loss.get_loss_dict_for_print("val"))

        avg_total_loss = total_loss_meter.get_avg_results()
        if self.config["wandb"]["wandb_on"]:
            wandb.log(avg_total_loss, step = self.step_count)
            self.val_count+=1

        if save_best_model:
            if self.best_val_loss > avg_total_loss["total_val"]:
                self.best_val_loss = avg_total_loss["total_val"]
                self.model.save("val")

    def train_depr(self):
        total_loss = 0
        step_loss = 0
        for batch_idx, batch_item in enumerate(self.train_loader):
            loss = self.model.step(batch_idx, batch_item, "train")
            total_loss += loss
            step_loss += loss
            if (batch_idx+1) % self.config["tr_set"]["schedueler_step"] == 0:
                self.model.scheduler.step()
                step_loss /= self.config["tr_set"]["schedueler_step"]
                if self.config["wandb"]["wandb_on"]:
                    wandb.log({"step_train_loss":step_loss})
                step_loss = 0
        total_loss /= len(self.train_loader)
        if self.config["wandb"]["wandb_on"]:
            wandb.log({"train_loss": total_loss})
        self.model.save("train")

    def test_depr(self):
        total_loss = 0
        for batch_idx, batch_item in enumerate(self.val_loader):
            loss = self.model.step(batch_idx, batch_item, "test")
            total_loss += loss
        total_loss /= len(self.val_loader)
        if self.config["wandb"]["wandb_on"]:
            wandb.log({"val_loss": total_loss})

        if self.best_val_loss > total_loss:
            self.best_val_loss = total_loss
            self.model.save("val")
    
    def run(self):
        train_data_loader = self.gen_set[0][0]
        val_data_loader = self.gen_set[0][1]
        for epoch in range(100000):
            self.train(epoch, train_data_loader)
            self.test(epoch, val_data_loader, True)

```

## Source file: gen_utils.py

{
  "path": "gen_utils.py",
  "blob_sha": "61d71b3635a558fd3833bfb535aa74e21f02f4fd",
  "sha256": "e59b77404d265033f24bc20b756d1b35bf6d7d5e0129f13eeeaa3a8a6370bfa2",
  "lines": 241,
  "bytes": 7330,
  "selection_score": 0,
  "keyword_line_counts": {
    "tooth": 4,
    "rig": 1,
    "face": 2
  },
  "url": "https://github.com/daniyalha33/inbracket_ai/blob/deef4f0f348d79d468195c38e4aacd2f3d23d8dc/gen_utils.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def np_to_pcd(arr, color=[1,0,0]):"
    },
    {
      "line": 20,
      "text": "def save_pcd(path, arr):"
    },
    {
      "line": 23,
      "text": "def save_mesh(path, mesh):"
    },
    {
      "line": 26,
      "text": "def count_unique_by_row(a):"
    },
    {
      "line": 34,
      "text": "def load_mesh(mesh_path, only_tooth_crop = False):"
    },
    {
      "line": 48,
      "text": "def get_colored_mesh(mesh, label_arr):"
    },
    {
      "line": 79,
      "text": "def np_to_pcd_with_label(arr, label_arr=None, axis=3):"
    },
    {
      "line": 115,
      "text": "def sigmoid(x):"
    },
    {
      "line": 118,
      "text": "def get_number_from_name(path):"
    },
    {
      "line": 121,
      "text": "def get_up_from_name(path):"
    },
    {
      "line": 124,
      "text": "def resample_pcd(pcd_ls, n, method):"
    },
    {
      "line": 135,
      "text": "def fps(xyz, npoint):"
    },
    {
      "line": 142,
      "text": "def print_3d(*data_3d_ls):"
    },
    {
      "line": 149,
      "text": "def torch_to_numpy(cuda_arr):"
    },
    {
      "line": 152,
      "text": "def save_np(arr, path):"
    },
    {
      "line": 156,
      "text": "def load_np(path):"
    },
    {
      "line": 161,
      "text": "def axis_rotation(axis, angle):"
    },
    {
      "line": 178,
      "text": "def make_coord_frame(size=1):"
    },
    {
      "line": 181,
      "text": "def load_json(file_path):"
    },
    {
      "line": 185,
      "text": "def save_json(file_path, json_obj):"
    },
    {
      "line": 190,
      "text": "def read_txt(file_path):"
    },
    {
      "line": 201,
      "text": "def read_txt_obj_ls(path, ret_mesh=False, use_tri_mesh=False):"
    }
  ]
}

```text

import open3d as o3d
import numpy as np
import torch
import os
import matplotlib.pyplot as plt
from sklearn.neighbors import KDTree
import json
from external_libs.pointops import pointops
import trimesh

def np_to_pcd(arr, color=[1,0,0]):
    arr = np.array(arr)
    pcd = o3d.geometry.PointCloud()
    pcd.points = o3d.utility.Vector3dVector(arr[:,:3])
    if arr.shape[1] >= 6:
        pcd.normals = o3d.utility.Vector3dVector(arr[:,3:6])
    pcd.colors = o3d.utility.Vector3dVector([color]*len(pcd.points))
    return pcd

def save_pcd(path, arr):
    o3d.io.write_point_cloud(path, arr)

def save_mesh(path, mesh):
    o3d.io.write_triangle_mesh(path, mesh)

def count_unique_by_row(a):
    weight = 1j*np.linspace(0, a.shape[1], a.shape[0], endpoint=False)
    b = a + weight[:, np.newaxis]
    u, ind, cnt = np.unique(b, return_index=True, return_counts=True)
    b = np.zeros_like(a)
    np.put(b, ind, cnt)
    return b

def load_mesh(mesh_path, only_tooth_crop = False):
    mesh = o3d.io.read_triangle_mesh(mesh_path)
    mesh.remove_duplicated_vertices()
    mesh.remove_degenerate_triangles()
    mesh.remove_unreferenced_vertices()
    
    if only_tooth_crop:
        cluster_idxes, cluster_nums, _ = mesh.cluster_connected_triangles()
        cluster_idxes = np.asarray(cluster_idxes)
        cluster_nums = np.asarray(cluster_nums)
        tooth_cluster_num = np.argmax(cluster_nums)
        mesh.remove_triangles_by_mask(cluster_idxes!=tooth_cluster_num)
    return mesh

def get_colored_mesh(mesh, label_arr):
    palte = np.array([
        [255,153,153],

        [153,76,0],
        [153,153,0],
        [76,153,0],
        [0,153,153],
        [0,0,153],
        [153,0,153],
        [153,0,76],
        [64,64,64],

        [255,128,0],
        [153,153,0],
        [76,153,0],
        [0,153,153],
        [0,0,153],
        [153,0,153],
        [153,0,76],
        [64,64,64],
    ])/255
    palte[9:] *= 0.4
    label_arr = label_arr.copy()
    label_arr %= palte.shape[0]
    label_colors = np.zeros((label_arr.shape[0], 3))
    for idx, palte_color in enumerate(palte):
        label_colors[label_arr==idx] = palte[idx]
    mesh.vertex_colors = o3d.utility.Vector3dVector(label_colors)
    return mesh

def np_to_pcd_with_label(arr, label_arr=None, axis=3):
    if type(label_arr) == np.ndarray:
        arr = np.concatenate([arr[:,:3], label_arr.reshape(-1,1)],axis=1)
    pcd = o3d.geometry.PointCloud()
    pcd.points = o3d.utility.Vector3dVector(arr[:,:3])
    
    palte = np.array([
        [255,153,153],

        [153,76,0],
        [153,153,0],
        [76,153,0],
        [0,153,153],
        [0,0,153],
        [153,0,153],
        [153,0,76],
        [64,64,64],

        [255,128,0],
        [153,153,0],
        [76,153,0],
        [0,153,153],
        [0,0,153],
        [153,0,153],
        [153,0,76],
        [64,64,64],
    ])/255
    palte[9:] *= 0.4
    arr = arr.copy()
    arr[:,axis] %= palte.shape[0]
    label_colors = np.zeros((arr.shape[0], 3))
    for idx, palte_color in enumerate(palte):
        label_colors[arr[:,axis]==idx] = palte[idx]
    pcd.colors = o3d.utility.Vector3dVector(label_colors)
    return pcd

def sigmoid(x):
    return 1 / (1 +np.exp(-x))

def get_number_from_name(path):
    return int(os.path.basename(path).split("_")[-1].split(".")[0])

def get_up_from_name(path):
    return os.path.basename(path).split("_")[-1].split(".")[0]=="up"

def resample_pcd(pcd_ls, n, method):
    """Drop or duplicate points so that pcd has exactly n points"""
    if method=="uniformly":
        idx = np.random.permutation(pcd_ls[0].shape[0])
    elif method == "fps":
        idx = fps(pcd_ls[0][:,:3], n)
    pcd_resampled_ls = []
    for i in range(len(pcd_ls)):
        pcd_resampled_ls.append(pcd_ls[i][idx[:n]])
    return pcd_resampled_ls

def fps(xyz, npoint):
    if xyz.shape[0]<=npoint:
        raise "new fps error"
    xyz = torch.from_numpy(np.array(xyz)).type(torch.float).cuda()
    idx = pointops.furthestsampling_gpu(xyz, 1, npoint)
    return torch_to_numpy(idx).reshape(-1)

def print_3d(*data_3d_ls):
    data_3d_ls = [item for item in data_3d_ls]
    for idx, item in enumerate(data_3d_ls):
        if type(item) == np.ndarray:
            data_3d_ls[idx] = np_to_pcd(item)
    o3d.visualization.draw_geometries(data_3d_ls, mesh_show_wireframe = True, mesh_show_back_face = True)

def torch_to_numpy(cuda_arr):
    return cuda_arr.cpu().detach().numpy()

def save_np(arr, path):
    with open(path, 'wb') as f:
        np.save(f, arr)

def load_np(path):
    with open(path, 'rb') as f:
        arr = np.load(f)
    return arr

def axis_rotation(axis, angle):
    ang = np.radians(angle) 
    R=np.zeros((3,3))
    ux, uy, uz = axis
    cos = np.cos
    sin = np.sin
    R[0][0] = cos(ang)+ux*ux*(1-cos(ang))
    R[0][1] = ux*uy*(1-cos(ang)) - uz*sin(ang)
    R[0][2] = ux*uz*(1-cos(ang)) + uy*sin(ang)
    R[1][0] = uy*ux*(1-cos(ang)) + uz*sin(ang)
    R[1][1] = cos(ang) + uy*uy*(1-cos(ang))
    R[1][2] = uy*uz*(1-cos(ang))-ux*sin(ang)
    R[2][0] = uz*ux*(1-cos(ang))-uy*sin(ang)
    R[2][1] = uz*uy*(1-cos(ang))+ux*sin(ang)
    R[2][2] = cos(ang) + uz*uz*(1-cos(ang))
    return R

def make_coord_frame(size=1):
    return o3d.geometry.TriangleMesh.create_coordinate_frame(size=size, origin=[0, 0, 0])

def load_json(file_path):
    with open(file_path, "r") as st_json:
        return json.load(st_json)

def save_json(file_path, json_obj):
    with open(file_path, "w") as json_file:

        json.dump(json_obj, json_file)

def read_txt(file_path):
    f = open(file_path, 'r')
    path_ls = []
    while True:
        line = f.readline().split()
        if not line: break
        path_ls.append(os.path.join(os.path.dirname(file_path), line.split("\n")[0] + ".npy"))
    f.close()

    return path_ls

def read_txt_obj_ls(path, ret_mesh=False, use_tri_mesh=False):
    # In some cases, trimesh can change vertex order
    if use_tri_mesh:
        tri_mesh_loaded_mesh = trimesh.load_mesh(path, process=False)
        vertex_ls = np.array(tri_mesh_loaded_mesh.vertices)
        tri_ls = np.array(tri_mesh_loaded_mesh.faces)+1
    else:
        f = open(path, 'r')
        vertex_ls = []
        tri_ls = []
        #vertex_color_ls = []
        while True:
            line = f.readline().split()
            if not line: break
            if line[0]=='v':
                vertex_ls.append(list(map(float,line[1:4])))
                #vertex_color_ls.append(list(map(float,line[4:7])))
            elif line[0]=='f':
                tri_verts_idxes = list(map(str,line[1:4]))
                if "//" in tri_verts_idxes[0]:
                    for i in range(len(tri_verts_idxes)):
                        tri_verts_idxes[i] = tri_verts_idxes[i].split("//")[0]
                tri_verts_idxes = list(map(int, tri_verts_idxes))
                tri_ls.append(tri_verts_idxes)
            else:
                continue
        f.close()

    mesh = o3d.geometry.TriangleMesh()
    mesh.vertices = o3d.utility.Vector3dVector(vertex_ls)
    mesh.triangles = o3d.utility.Vector3iVector(np.array(tri_ls)-1)
    mesh.compute_vertex_normals()

    norms = np.array(mesh.vertex_normals)

    vertex_ls = np.array(vertex_ls)
    output = [np.concatenate([vertex_ls,norms], axis=1)]

    if ret_mesh:
        output.append(mesh)
    return output

```
