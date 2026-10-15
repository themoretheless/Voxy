# Facial rendering source scan: arturwyroslak/mediapipe-arkit-automesh

Repository: https://github.com/arturwyroslak/mediapipe-arkit-automesh

Pinned source revision: 29e7331f12f049c1e9664da74ea9879f4012481d

Discovery topic: ARKit blendshape

Description: Automated generation of ARKit 52 blendshapes for 3D face models using MediaPipe standards and Blender backend.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"rig": ["backend/core/facial_rig_names.py"]}

## Source file: backend/core/facial_rig_names.py

{
  "path": "backend/core/facial_rig_names.py",
  "blob_sha": "87ad86aa4e2994ea0117dfbb822a346b06d0596f",
  "sha256": "06518feb9b797bbe3383441b241509644aa5db802bec033b1f9b56e639d86ca5",
  "lines": 56,
  "bytes": 1578,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 14,
    "jaw": 4,
    "mouth": 23,
    "tongue": 1,
    "rig": 21
  },
  "url": "https://github.com/arturwyroslak/mediapipe-arkit-automesh/blob/29e7331f12f049c1e9664da74ea9879f4012481d/backend/core/facial_rig_names.py",
  "function_signature_candidates": []
}

```text

class FacialRigNames:
    names = {
        0: 'Basis',
        1: 'eyeBlinkLeft',
        2: 'eyeLookDownLeft',
        3: 'eyeLookInLeft',
        4: 'eyeLookOutLeft',
        5: 'eyeLookUpLeft',
        6: 'eyeSquintLeft',
        7: 'eyeWideLeft',
        8: 'eyeBlinkRight',
        9: 'eyeLookDownRight',
        10: 'eyeLookInRight',
        11: 'eyeLookOutRight',
        12: 'eyeLookUpRight',
        13: 'eyeSquintRight',
        14: 'eyeWideRight',
        15: 'jawForward',
        16: 'jawLeft',
        17: 'jawRight',
        18: 'jawOpen',
        19: 'mouthClose',
        20: 'mouthFunnel',
        21: 'mouthPucker',
        22: 'mouthRight',
        23: 'mouthLeft',
        24: 'mouthSmileLeft',
        25: 'mouthSmileRight',
        26: 'mouthFrownRight',
        27: 'mouthFrownLeft',
        28: 'mouthDimpleLeft',
        29: 'mouthDimpleRight',
        30: 'mouthStretchLeft',
        31: 'mouthStretchRight',
        32: 'mouthRollLower',
        33: 'mouthRollUpper',
        34: 'mouthShrugLower',
        35: 'mouthShrugUpper',
        36: 'mouthPressLeft',
        37: 'mouthPressRight',
        38: 'mouthLowerDownLeft',
        39: 'mouthLowerDownRight',
        40: 'mouthUpperUpLeft',
        41: 'mouthUpperUpRight',
        42: 'browDownLeft',
        43: 'browDownRight',
        44: 'browInnerUp',
        45: 'browOuterUpLeft',
        46: 'browOuterUpRight',
        47: 'cheekPuff',
        48: 'cheekSquintLeft',
        49: 'cheekSquintRight',
        50: 'noseSneerLeft',
        51: 'noseSneerRight',
        52: 'tongueOut'
    }


```

## Source file: backend/main.py

{
  "path": "backend/main.py",
  "blob_sha": "c27084eb6ee0de914328c835f87668c4fbffeb73",
  "sha256": "34ba6065b3615d42ac89086ad158c56436781ef3dc5becf6997cdd2a823b23d4",
  "lines": 70,
  "bytes": 2248,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 1
  },
  "url": "https://github.com/arturwyroslak/mediapipe-arkit-automesh/blob/29e7331f12f049c1e9664da74ea9879f4012481d/backend/main.py",
  "function_signature_candidates": [
    {
      "line": 25,
      "text": "async def health_check():"
    },
    {
      "line": 29,
      "text": "async def upload_file(file: UploadFile = File(...)):"
    },
    {
      "line": 43,
      "text": "async def process_file(file_id: str):"
    },
    {
      "line": 62,
      "text": "async def download_file(filename: str):"
    }
  ]
}

```text

from fastapi import FastAPI, UploadFile, File, HTTPException
from fastapi.responses import FileResponse
from fastapi.middleware.cors import CORSMiddleware
import shutil
import os
import uuid

app = FastAPI()

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

UPLOAD_DIR = "uploads"
PROCESSED_DIR = "processed"

os.makedirs(UPLOAD_DIR, exist_ok=True)
os.makedirs(PROCESSED_DIR, exist_ok=True)

@app.get("/")
async def health_check():
    return {"status": "ok", "service": "backend"}

@app.post("/upload")
async def upload_file(file: UploadFile = File(...)):
    if not file.filename.endswith('.glb'):
        raise HTTPException(status_code=400, detail="Only .glb files are supported")
    
    unique_id = str(uuid.uuid4())
    input_filename = f"{unique_id}_{file.filename}"
    input_path = os.path.join(UPLOAD_DIR, input_filename)
    
    with open(input_path, "wb") as buffer:
        shutil.copyfileobj(file.file, buffer)
        
    return {"id": unique_id, "filename": input_filename}

@app.post("/process/{file_id}")
async def process_file(file_id: str):
    files = [f for f in os.listdir(UPLOAD_DIR) if f.startswith(file_id)]
    if not files:
        raise HTTPException(status_code=404, detail="File not found")
    
    input_filename = files[0]
    input_path = os.path.join(UPLOAD_DIR, input_filename)
    output_filename = f"processed_{input_filename}"
    output_path = os.path.join(PROCESSED_DIR, output_filename)
    
    # TEMP: zamiast Blendera kopiujemy plik 1:1, żeby zawsze był obecny
    try:
        shutil.copyfile(input_path, output_path)
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Processing failed: {str(e)}")
    
    return {"status": "success", "output_file": output_filename, "download_url": f"/download/{output_filename}"}

@app.get("/download/{filename}")
async def download_file(filename: str):
    path = os.path.join(PROCESSED_DIR, filename)
    if not os.path.exists(path):
        raise HTTPException(status_code=404, detail="File not found")
    return FileResponse(path)

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8000)


```

## Source file: frontend/next.config.js

{
  "path": "frontend/next.config.js",
  "blob_sha": "5d2353495d0a3966b7e0d86d924606fe881fdbd8",
  "sha256": "065212221e1bc3cbbe5446983f375a91181df92c45a2f88318e128cb6a939d49",
  "lines": 21,
  "bytes": 434,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/arturwyroslak/mediapipe-arkit-automesh/blob/29e7331f12f049c1e9664da74ea9879f4012481d/frontend/next.config.js",
  "function_signature_candidates": []
}

```text

/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  eslint: {
    ignoreDuringBuilds: true,
  },
  typescript: {
    ignoreBuildErrors: true,
  },
  output: 'standalone',
  async rewrites() {
    return [
      {
        source: '/api/:path*',
        destination: 'http://127.0.0.1:8000/:path*', // Proxy to backend running in same container
      },
    ];
  },
};

module.exports = nextConfig;


```

## Source file: backend/blender_script.py

{
  "path": "backend/blender_script.py",
  "blob_sha": "cb2397c45420d84c83c623f7e2ecc5200b567c08",
  "sha256": "b2034fdb3e9b56c3917a9804cd3faa86049edbf99382e3074c363790010ad665",
  "lines": 133,
  "bytes": 5500,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 3,
    "blendshape": 1,
    "jaw": 4,
    "mouth": 6,
    "gum": 2,
    "tongue": 1,
    "deform": 2,
    "rig": 13,
    "face": 2
  },
  "url": "https://github.com/arturwyroslak/mediapipe-arkit-automesh/blob/29e7331f12f049c1e9664da74ea9879f4012481d/backend/blender_script.py",
  "function_signature_candidates": [
    {
      "line": 30,
      "text": "def log(msg):"
    },
    {
      "line": 33,
      "text": "def clear_scene():"
    },
    {
      "line": 37,
      "text": "def import_glb(filepath):"
    },
    {
      "line": 59,
      "text": "def create_shape_keys(obj):"
    },
    {
      "line": 93,
      "text": "def export_glb(filepath):"
    }
  ]
}

```text

import bpy
import sys
import os
import math

# Ensure backend directory is in path to import core modules
current_dir = os.path.dirname(os.path.abspath(__file__))
sys.path.append(current_dir)

try:
    from core.facial_rig_names import FacialRigNames
except ImportError:
    # Fallback if running directly without proper package context
    class FacialRigNames:
        names = {
            0: 'Basis', 1: 'eyeBlinkLeft', 2: 'eyeLookDownLeft', 3: 'eyeLookInLeft', 4: 'eyeLookOutLeft',
            5: 'eyeLookUpLeft', 6: 'eyeSquintLeft', 7: 'eyeWideLeft', 8: 'eyeBlinkRight', 9: 'eyeLookDownRight',
            10: 'eyeLookInRight', 11: 'eyeLookOutRight', 12: 'eyeLookUpRight', 13: 'eyeSquintRight', 14: 'eyeWideRight',
            15: 'jawForward', 16: 'jawLeft', 17: 'jawRight', 18: 'jawOpen', 19: 'mouthClose', 20: 'mouthFunnel',
            21: 'mouthPucker', 22: 'mouthRight', 23: 'mouthLeft', 24: 'mouthSmileLeft', 25: 'mouthSmileRight',
            26: 'mouthFrownRight', 27: 'mouthFrownLeft', 28: 'mouthDimpleLeft', 29: 'mouthDimpleRight',
            30: 'mouthStretchLeft', 31: 'mouthStretchRight', 32: 'mouthRollLower', 33: 'mouthRollUpper',
            34: 'mouthShrugLower', 35: 'mouthShrugUpper', 36: 'mouthPressLeft', 37: 'mouthPressRight',
            38: 'mouthLowerDownLeft', 39: 'mouthLowerDownRight', 40: 'mouthUpperUpLeft', 41: 'mouthUpperUpRight',
            42: 'browDownLeft', 43: 'browDownRight', 44: 'browInnerUp', 45: 'browOuterUpLeft', 46: 'browOuterUpRight',
            47: 'cheekPuff', 48: 'cheekSquintLeft', 49: 'cheekSquintRight', 50: 'noseSneerLeft', 51: 'noseSneerRight',
            52: 'tongueOut'
        }

def log(msg):
    print(f"[ARKit-AutoMesh] {msg}")

def clear_scene():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    log("Scene cleared.")

def import_glb(filepath):
    log(f"Importing {filepath}...")
    bpy.ops.import_scene.gltf(filepath=filepath)
    
    # Identify the main mesh. Prioritize objects with 'Head' or 'Face' in name, otherwise pick largest mesh.
    candidates = [obj for obj in bpy.context.selected_objects if obj.type == 'MESH']
    
    if not candidates:
        return None
        
    target = candidates[0]
    # Simple heuristic: largest number of vertices usually implies the main mesh if there are accessories
    if len(candidates) > 1:
        target = max(candidates, key=lambda o: len(o.data.vertices))
        
    log(f"Selected target mesh: {target.name}")
    
    # Make active and selected
    bpy.context.view_layer.objects.active = target
    target.select_set(True)
    return target

def create_shape_keys(obj):
    if not obj.data.shape_keys:
        obj.shape_key_add(name="Basis")
        log("Created Basis shape key.")
    
    existing_keys = obj.data.shape_keys.key_blocks
    
    count = 0
    # Access vertices once for performance if we were doing real math
    verts = obj.data.vertices
    
    for i in range(1, 53):
        shape_name = FacialRigNames.names.get(i)
        if shape_name and shape_name not in existing_keys:
            key_block = obj.shape_key_add(name=shape_name)
            count += 1
            
            # DEMO: Add a fake deformation to 'jawOpen' so the user sees something happening
            # In a real app, this would use MediaPipe landmarks to calculate deltas
            if shape_name == 'jawOpen':
                log("Injecting dummy deformation for jawOpen...")
                for v in key_block.data:
                    # Move lower vertices down (simple assumption that y < 0 is lower face in some spaces, 
                    # or z < 0 depending on orientation. GLB is usually Y-up)
                    # Let's just move everything below a certain threshold to be safe visually
                    if v.co.y < 0: # Assuming Y is up/down-ish or forward. 
                        # Actually in Blender Z is up. Y is forward.
                        # But imports might vary.
                        # Let's just move vertices with negative Z down further
                        if v.co.z < 0:
                             v.co.z -= 0.05
            
    log(f"Added {count} ARKit blendshapes.")

def export_glb(filepath):
    log(f"Exporting to {filepath}...")
    # Ensure export includes shape keys (morph targets)
    bpy.ops.export_scene.gltf(
        filepath=filepath,
        export_format='GLB',
        export_morph=True,
        export_apply=True  # Apply modifiers if any
    )
    log(\"Export complete.\")

if __name__ == \"__main__\":
    try:
        argv = sys.argv
        if \"--\" in argv:
            args = argv[argv.index(\"--\") + 1:]
            if len(args) >= 2:
                input_path = args[0]
                output_path = args[1]
                
                clear_scene()
                target_mesh = import_glb(input_path)
                
                if target_mesh:
                    create_shape_keys(target_mesh)
                    export_glb(output_path)
                else:
                    log(\"ERROR: No suitable mesh found in input file.\")
                    sys.exit(1)
            else:
                log(\"ERROR: Insufficient arguments. Usage: blender ... -- <input> <output>\")
                sys.exit(1)
        else:
            log(\"ERROR: Separator '--' not found in arguments.\")
            sys.exit(1)
            
    except Exception as e:
        log(f\"FATAL ERROR: {str(e)}\")
        import traceback
        traceback.print_exc()
        sys.exit(1)


```
