# Facial rendering source scan: ghif/FLAME_mlx

Repository: https://github.com/ghif/FLAME_mlx

Pinned source revision: 160449cdbd8a0e62ba1acabd102ef29bd79bdcf3

Discovery topic: FLAME 3D

Description: High-performance, differentiable FLAME 3D head model implementation in Apple MLX. Optimized for Apple Silicon with real-time interactive 3D viewer.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"flame": ["resources/flame_demo.gif", "resources/flame_demo.webp", "resources/flame_screenshot.png"]}

## Source file: main.py

{
  "path": "main.py",
  "blob_sha": "2e7dae9e8c9fead7f72a9f5d8912723b25333c14",
  "sha256": "28ae33d2815067e5c08312f73998e10dcffb9b46641cc1901488998f5d2b62f5",
  "lines": 60,
  "bytes": 2023,
  "selection_score": 0,
  "keyword_line_counts": {
    "jaw": 1,
    "flame": 12
  },
  "url": "https://github.com/ghif/FLAME_mlx/blob/160449cdbd8a0e62ba1acabd102ef29bd79bdcf3/main.py",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "def main():"
    }
  ]
}

```text


import os
import mlx.core as mx
import numpy as np
from core import FLAME, get_config

def main():
    """
    Main entry point for verifying the FLAME MLX implementation.
    
    This script:
    1. Loads the configuration.
    2. Checks for required model and embedding files.
    3. Initializes the FLAME model.
    4. Executes a forward pass with zeroed parameters.
    5. Prints the resulting tensor shapes for verification.
    """
    config = get_config()
    
    # Check if model files exist
    missing_files = []
    for path in [config.flame_model_path, config.static_landmark_embedding_path, config.dynamic_landmark_embedding_path]:
        if not os.path.exists(path):
            missing_files.append(path)
            
    if not os.path.exists(config.flame_model_path):
        print(f"Error: FLAME model file '{config.flame_model_path}' is missing.")
        print("Please download the FLAME model from http://flame.is.tue.mpg.de/ and place it in the correct directory.")
        return
    elif missing_files:
        print("Warning: Some landmark embedding files are missing. Landmarks will not be computed.")
        for f in missing_files:
            if f != config.flame_model_path:
                print(f"  - {f}")

    # Initialize FLAME model
    flame = FLAME(config)
    
    # Create batch of parameters
    batch_size = config.batch_size
    shape_params = mx.zeros((batch_size, 100))
    expression_params = mx.zeros((batch_size, 50))
    pose_params = mx.zeros((batch_size, 6)) # [global_rot, jaw_rot]
    
    # Forward pass
    vertices, landmarks = flame(
        shape_params=shape_params,
        expression_params=expression_params,
        pose_params=pose_params
    )
    
    print(f"Successfully ran FLAME forward pass in MLX!")
    print(f"Vertices shape: {vertices.shape}")
    if landmarks is not None:
        print(f"Landmarks shape: {landmarks.shape}")
    else:
        print("Landmarks not computed (missing embedding files).")

if __name__ == "__main__":
    main()


```

## Source file: viewer.py

{
  "path": "viewer.py",
  "blob_sha": "3fea665eac547205864804134440c2a5e19f8e9f",
  "sha256": "967451cbf39286c4dced67d9757e4b8f88ae3ae394d2f17872362decc8fe9d1c",
  "lines": 189,
  "bytes": 6252,
  "selection_score": 0,
  "keyword_line_counts": {
    "jaw": 9,
    "flame": 17,
    "face": 2
  },
  "url": "https://github.com/ghif/FLAME_mlx/blob/160449cdbd8a0e62ba1acabd102ef29bd79bdcf3/viewer.py",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "def main():"
    },
    {
      "line": 41,
      "text": "def load_model(path):"
    },
    {
      "line": 113,
      "text": "def update_mesh():"
    },
    {
      "line": 161,
      "text": "def _(_):"
    },
    {
      "line": 178,
      "text": "def _(client: viser.ClientHandle) -> None:"
    }
  ]
}

```text


import time
import os
import mlx.core as mx
import numpy as np
import viser
import viser.transforms as tf
from core import FLAME, get_config

def main():
    """
    Launches an interactive 3D web viewer for the FLAME model using Viser.
    
    The viewer provides:
    - Real-time sliders for shape, expression, neck, and jaw parameters.
    - Model switching between Generic, Male, and Female templates.
    - Live 3D mesh rendering on a local web server (port 8081).
    """
    server = viser.ViserServer(port=8081,open_browser=False)
    print(f"VISER_PORT: {server.get_port()}", flush=True)
    server.scene.set_up_direction("+y")
    config = get_config()

    # Predefined model paths
    MODEL_PATHS = {
        "Generic": "./model/flame2023_Open.pkl",
        "Male": "./model/male_model.pkl",
        "Female": "./model/female_model.pkl",
    }

    # Initial state
    state = {
        "model_type": "Generic",
        "shape": np.zeros(100, dtype=np.float32),
        "expr": np.zeros(50, dtype=np.float32),
        "neck": np.zeros(3, dtype=np.float32),
        "jaw": np.zeros(3, dtype=np.float32),
    }

    # Load initial model
    def load_model(path):
        """
        Helper to load a FLAME model from a pickle file.
        
        Args:
            path (str): Absolute or relative path to the .pkl model file.
            
        Returns:
            FLAME: Initialized FLAME model instance or None if loading fails.
        """
        print(f"Attempting to load model from: {path}")
        if not os.path.exists(path):
            print(f"File not found: {path}")
            return None
        config.flame_model_path = path
        try:
            model = FLAME(config)
            print("Model loaded successfully.")
            return model
        except Exception as e:
            print(f"Error loading model: {e}")
            return None

    flame_model = load_model(MODEL_PATHS["Generic"])
    if flame_model is None:
        print(f"Error: Default model {MODEL_PATHS['Generic']} not found.")
    
    # UI Components
    with server.gui.add_folder("Model Selection"):
        model_dropdown = server.gui.add_dropdown(
            "Model Type",
            options=["Generic"],
            initial_value="Generic",
        )

    shape_sliders = []
    with server.gui.add_folder("Shape Parameters"):
        for i in range(10):
            shape_sliders.append(
                server.gui.add_slider(
                    f"Shape {i}",
                    min=-3.0,
                    max=3.0,
                    step=0.1,
                    initial_value=0.0,
                )
            )

    expr_sliders = []
    with server.gui.add_folder("Expression Parameters"):
        for i in range(10):
            expr_sliders.append(
                server.gui.add_slider(
                    f"Expression {i}",
                    min=-3.0,
                    max=3.0,
                    step=0.1,
                    initial_value=0.0,
                )
            )

    with server.gui.add_folder("Jaw Pose"):
        jaw_x = server.gui.add_slider("Jaw Pitch", min=-0.5, max=0.5, step=0.01, initial_value=0.0)
        jaw_y = server.gui.add_slider("Jaw Yaw", min=-0.5, max=0.5, step=0.01, initial_value=0.0)
        jaw_z = server.gui.add_slider("Jaw Roll", min=-0.5, max=0.5, step=0.01, initial_value=0.0)

    with server.gui.add_folder("Neck Pose"):
        neck_x = server.gui.add_slider("Neck Pitch", min=-0.5, max=0.5, step=0.01, initial_value=0.0)
        neck_y = server.gui.add_slider("Neck Yaw", min=-0.5, max=0.5, step=0.01, initial_value=0.0)
        neck_z = server.gui.add_slider("Neck Roll", min=-0.5, max=0.5, step=0.01, initial_value=0.0)

    # Update Function
    def update_mesh():
        """
        Collects current parameter values from UI sliders, runs the FLAME 
        forward pass, and updates the 3D mesh in the Viser scene.
        """
        nonlocal flame_model
        if flame_model is None:
            return

        # Prepare parameters
        shape_mx = mx.zeros((1, 100))
        shape_vals = np.array([s.value for s in shape_sliders], dtype=np.float32)
        shape_mx[0, :10] = mx.array(shape_vals)

        expr_mx = mx.zeros((1, 50))
        expr_vals = np.array([e.value for e in expr_sliders], dtype=np.float32)
        expr_mx[0, :10] = mx.array(expr_vals)
        
        jaw_vals = np.array([jaw_x.value, jaw_y.value, jaw_z.value], dtype=np.float32)
        global_rot = mx.zeros((1, 3)) 
        pose_params = mx.concatenate([global_rot, mx.array(jaw_vals)[None]], axis=1)

        neck_vals = np.array([neck_x.value, neck_y.value, neck_z.value], dtype=np.float32)
        neck_pose = mx.array(neck_vals)[None]

        # Forward Pass
        vertices, _ = flame_model(
            shape_params=shape_mx,
            expression_params=expr_mx,
            pose_params=pose_params,
            neck_pose=neck_pose
        )

        # Convert to numpy for Viser
        verts_np = np.array(vertices[0])
        faces_np = np.array(flame_model.faces)

        # Update Viser Scene
        server.scene.add_mesh_simple(
            "/flame_mesh",
            vertices=verts_np,
            faces=faces_np,
            color=(200, 180, 150),
            wireframe=False,
        )

    # Listeners
    @model_dropdown.on_update
    def _(_):
        nonlocal flame_model
        path = MODEL_PATHS[model_dropdown.value]
        new_model = load_model(path)
        if new_model is not None:
            flame_model = new_model
            update_mesh()
        else:
            print(f"Warning: Model file {path} not found.")

    for s in shape_sliders + expr_sliders + [jaw_x, jaw_y, jaw_z, neck_x, neck_y, neck_z]:
        s.on_update(lambda _: update_mesh())

    # Initial update
    update_mesh()

    @server.on_client_connect
    def _(client: viser.ClientHandle) -> None:
        # Set initial camera position for new clients
        # Position: [x, y, z], Look at: [x, y, z]
        client.camera.position = (0.0, 0.02, 0.25)
        client.camera.look_at = (0.0, 0.0, 0.0)

    print(f"Viser server started at {server.get_host()}:{server.get_port()}")
    while True:
        time.sleep(0.1)

if __name__ == "__main__":
    main()


```

## Source file: core/ops.py

{
  "path": "core/ops.py",
  "blob_sha": "5b186e256fcf4659bf484c88d7277a2e106b5d7a",
  "sha256": "b63dab2494e2342886d0d07864fd9aae3dc703436bd4bdf706d688bff2e6552a",
  "lines": 210,
  "bytes": 8113,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 2,
    "skin": 5,
    "blend_shape": 2,
    "lbs": 4,
    "rig": 4,
    "face": 9
  },
  "url": "https://github.com/ghif/FLAME_mlx/blob/160449cdbd8a0e62ba1acabd102ef29bd79bdcf3/core/ops.py",
  "function_signature_candidates": [
    {
      "line": 6,
      "text": "def vertices2landmarks(vertices, faces, lmk_faces_idx, lmk_bary_coords):"
    },
    {
      "line": 36,
      "text": "def vertices2joints(J_regressor, vertices):"
    },
    {
      "line": 49,
      "text": "def blend_shapes(betas, shape_disps):"
    },
    {
      "line": 62,
      "text": "def batch_rodrigues(rot_vecs, epsilon=1e-8):"
    },
    {
      "line": 91,
      "text": "def transform_mat(R, t):"
    },
    {
      "line": 111,
      "text": "def batch_rigid_transform(rot_mats, joints, parents):"
    },
    {
      "line": 157,
      "text": "def lbs(betas, pose, v_template, shapedirs, posedirs, J_regressor, parents, lbs_weights, pose2rot=True):"
    }
  ]
}

```text


import mlx.core as mx
from typing import Tuple, List
from .utils import rot_mat_to_euler

def vertices2landmarks(vertices, faces, lmk_faces_idx, lmk_bary_coords):
    """
    Calculates 3D landmarks by barycentric interpolation over specific mesh faces.
    
    Args:
        vertices (mx.array): Batch of mesh vertices of shape (B, V, 3).
        faces (mx.array): Mesh face indices of shape (F, 3).
        lmk_faces_idx (mx.array): Indices of faces where landmarks are located (B, L).
        lmk_bary_coords (mx.array): Barycentric coordinates of landmarks on those faces (B, L, 3).
        
    Returns:
        mx.array: Calculated 3D landmarks of shape (B, L, 3).
    """
    batch_size, num_verts = vertices.shape[:2]
    
    # Extract the indices of the vertices for each face
    lmk_faces = faces[lmk_faces_idx]
    
    # Add batch offset to indices for flat indexing
    batch_offset = mx.arange(batch_size)[:, None, None] * num_verts
    lmk_faces_absolute = lmk_faces + batch_offset
    
    # Gather vertices
    vertices_flat = vertices.reshape(-1, 3)
    lmk_vertices = vertices_flat[lmk_faces_absolute] # (B, L, 3, 3)
    
    # Compute landmarks using barycentric interpolation
    landmarks = mx.einsum('blfi,blf->bli', lmk_vertices, lmk_bary_coords)
    return landmarks

def vertices2joints(J_regressor, vertices):
    """
    Calculates 3D joint locations from mesh vertices using a regressor matrix.
    
    Args:
        J_regressor (mx.array): Joint regressor matrix of shape (J, V).
        vertices (mx.array): Batch of mesh vertices of shape (B, V, 3).
        
    Returns:
        mx.array: Calculated 3D joint locations of shape (B, J, 3).
    """
    return mx.einsum('bik,ji->bjk', vertices, J_regressor)

def blend_shapes(betas, shape_disps):
    """
    Calculates per-vertex displacements from blend shapes (identities or expressions).
    
    Args:
        betas (mx.array): Coefficients for the blend shapes of shape (B, K).
        shape_disps (mx.array): Blend shape bases of shape (V, 3, K).
        
    Returns:
        mx.array: Vertex displacements of shape (B, V, 3).
    """
    return mx.einsum('bl,mkl->bmk', betas, shape_disps)

def batch_rodrigues(rot_vecs, epsilon=1e-8):
    """
    Converts a batch of rotation vectors (axis-angle) to 3x3 rotation matrices.
    
    Args:
        rot_vecs (mx.array): Batch of rotation vectors of shape (B, 3).
        epsilon (float): Small value to avoid division by zero.
        
    Returns:
        mx.array: Batch of 3x3 rotation matrices of shape (B, 3, 3).
    """
    batch_size = rot_vecs.shape[0]
    
    angle = mx.linalg.norm(rot_vecs + epsilon, axis=1, keepdims=True)
    rot_dir = rot_vecs / angle
    
    cos = mx.expand_dims(mx.cos(angle), axis=1) # (B, 1, 1)
    sin = mx.expand_dims(mx.sin(angle), axis=1) # (B, 1, 1)
    
    rx, ry, rz = mx.split(rot_dir, 3, axis=1)
    zeros = mx.zeros((batch_size, 1))
    
    # Skew-symmetric matrix K
    K = mx.concatenate([zeros, -rz, ry, rz, zeros, -rx, -ry, rx, zeros], axis=1).reshape(batch_size, 3, 3)
    
    ident = mx.eye(3)[None]
    rot_mat = ident + sin * K + (1 - cos) * mx.matmul(K, K)
    return rot_mat

def transform_mat(R, t):
    """
    Combines rotation and translation into 4x4 homogenous transformation matrices.
    
    Args:
        R (mx.array): Batch of rotation matrices of shape (B, 3, 3).
        t (mx.array): Batch of translation vectors of shape (B, 3, 1).
        
    Returns:
        mx.array: Compiled 4x4 transformation matrices of shape (B, 4, 4).
    """
    B = R.shape[0]
    bottom_R = mx.zeros((B, 1, 3))
    R_homo = mx.concatenate([R, bottom_R], axis=1)
    
    bottom_t = mx.ones((B, 1, 1))
    t_homo = mx.concatenate([t, bottom_t], axis=1)
    
    return mx.concatenate([R_homo, t_homo], axis=2)

def batch_rigid_transform(rot_mats, joints, parents):
    """
    Computes global joint transformations based on relative rotations and kinematic chain.
    
    Args:
        rot_mats (mx.array): Batch of joint rotation matrices (B, J, 3, 3).
        joints (mx.array): Batch of joint initial positions (B, J, 3).
        parents (mx.array): Indices of parent joints for each joint (J,).
        
    Returns:
        tuple: (posed_joints, rel_transforms)
            - posed_joints (mx.array): Global joint positions after rotation (B, J, 3).
            - rel_transforms (mx.array): Relative transformation matrices for skinning (B, J, 4, 4).
    """
    joints = joints[:, :, :, None] # (B, J, 3, 1)
    
    rel_joints = mx.array(joints)
    rel_joints[:, 1:] -= joints[:, parents[1:]]
    
    # Local transforms
    transforms_mat = transform_mat(
        rot_mats.reshape(-1, 3, 3),
        rel_joints.reshape(-1, 3, 1)
    ).reshape(-1, joints.shape[1], 4, 4)
    
    # Global transforms via kinematic chain
    transform_chain = [transforms_mat[:, 0]]
    for i in range(1, parents.shape[0]):
        parent_idx = int(parents[i])
        curr_res = mx.matmul(transform_chain[parent_idx], transforms_mat[:, i])
        transform_chain.append(curr_res)
    
    transforms = mx.stack(transform_chain, axis=1) # (B, J, 4, 4)
    posed_joints = transforms[:, :, :3, 3]
    
    # Compute relative transforms for skinning
    joints_homogen = mx.concatenate([joints, mx.zeros((joints.shape[0], joints.shape[1], 1, 1))], axis=2)
    transformed_joints = mx.matmul(transforms, joints_homogen) # (B, J, 4, 1)
    
    B, J = transforms.shape[:2]
    padding = mx.zeros((B, J, 4, 3))
    rel_transforms_padding = mx.concatenate([padding, transformed_joints], axis=3)
    rel_transforms = transforms - rel_transforms_padding
    
    return posed_joints, rel_transforms

def lbs(betas, pose, v_template, shapedirs, posedirs, J_regressor, parents, lbs_weights, pose2rot=True):
    """
    Linear Blend Skinning (LBS) implementation.
    
    Args:
        betas (mx.array): Shape and expression coefficients (B, K).
        pose (mx.array): Pose parameters as axis-angle vectors (B, J, 3) or rotation matrices (B, J, 3, 3).
        v_template (mx.array): Template mesh vertices (B, V, 3).
        shapedirs (mx.array): Shape and expression blend shape bases (V, 3, K).
        posedirs (mx.array): Pose corrective blend shape bases (V*3, P).
        J_regressor (mx.array): Joint regressor matrix (J, V).
        parents (mx.array): Kinematic tree parent indices (J,).
        lbs_weights (mx.array): Skinning weights (V, J).
        pose2rot (bool): If True, convert 'pose' from axis-angle to rotation matrices.
        
    Returns:
        tuple: (verts, J_transformed)
            - verts (mx.array): Final posed mesh vertices of shape (B, V, 3).
            - J_transformed (mx.array): Global joint positions after transformations (B, J, 3).
    """
    batch_size = max(betas.shape[0], pose.shape[0])
    
    # 1. Shape Blend Shapes
    v_shaped = v_template + blend_shapes(betas, shapedirs)
    
    # 2. Get the neutral joints
    J = vertices2joints(J_regressor, v_shaped)
    
    # 3. Pose Corrective Blend Shapes
    ident = mx.eye(3)
    if pose2rot:
        rot_mats = batch_rodrigues(pose.reshape(-1, 3)).reshape(batch_size, -1, 3, 3)
        pose_feature = (rot_mats[:, 1:] - ident).reshape(batch_size, -1)
        pose_offsets = mx.matmul(pose_feature, posedirs).reshape(batch_size, -1, 3)
    else:
        rot_mats = pose
        pose_feature = (rot_mats[:, 1:] - ident).reshape(batch_size, -1)
        pose_offsets = mx.matmul(pose_feature, posedirs).reshape(batch_size, -1, 3)
        
    v_posed = v_shaped + pose_offsets
    
    # 4. Global joint location and transformation matrices
    J_transformed, A = batch_rigid_transform(rot_mats, J, parents)
    
    # 5. Skinning
    W = mx.broadcast_to(lbs_weights[None], (batch_size, *lbs_weights.shape))
    A_flat = A.reshape(batch_size, -1, 16)
    T = mx.matmul(W, A_flat).reshape(batch_size, -1, 4, 4)
    
    v_posed_homo = mx.concatenate([v_posed, mx.ones((batch_size, v_posed.shape[1], 1))], axis=2)
    v_homo = mx.matmul(T, v_posed_homo[:, :, :, None])
    verts = v_homo[:, :, :3, 0]
    
    return verts, J_transformed


```

## Source file: core/model.py

{
  "path": "core/model.py",
  "blob_sha": "17658f44a15eec75d412c0b96871b220e43f54e2",
  "sha256": "7655f570fa9a7a97e998cfcf84da61317578f1c739307bec6532f55deb605dc8",
  "lines": 259,
  "bytes": 10868,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 8,
    "skin": 2,
    "blendshape": 1,
    "jaw": 2,
    "flame": 15,
    "lbs": 6,
    "rig": 2,
    "face": 24,
    "lip": 1
  },
  "url": "https://github.com/ghif/FLAME_mlx/blob/160449cdbd8a0e62ba1acabd102ef29bd79bdcf3/core/model.py",
  "function_signature_candidates": [
    {
      "line": 35,
      "text": "def __init__(self, config):"
    },
    {
      "line": 123,
      "text": "def _find_dynamic_lmk_idx_and_bcoords("
    },
    {
      "line": 171,
      "text": "def __call__("
    }
  ]
}

```text


import os
import pickle
import warnings
import numpy as np
import mlx.core as mx
import mlx.nn as nn

# Suppress NumPy 2.x VisibleDeprecationWarning from loading old pickle files
try:
    from numpy import VisibleDeprecationWarning
    warnings.filterwarnings("ignore", category=VisibleDeprecationWarning)
except ImportError:
    pass

# Additional robust filter for the specific message
warnings.filterwarnings("ignore", message=".*align should be passed as Python or NumPy boolean.*")
from .ops import lbs, vertices2landmarks, batch_rodrigues
from .utils import Struct, to_mx, rot_mat_to_euler

class FLAME(nn.Module):
    """
    Given FLAME parameters, this class generates a differentiable 3D head mesh 
    and associated facial landmarks.
    
    Attributes:
        faces (mx.array): Face indices of the mesh.
        v_template (mx.array): Mean shape template (neutral head).
        shapedirs (mx.array): PCA basis for identity and expression shapes.
        posedirs (mx.array): PCA basis for pose-corrective blendshapes.
        J_regressor (mx.array): Matrix to regress joint positions from vertices.
        parents (mx.array): Kinematic chain parent indices for joints.
        lbs_weights (mx.array): Skinning weights for Linear Blend Skinning.
    """
    def __init__(self, config):
        """
        Initializes the FLAME model with provided configuration and learned model files.
        
        Args:
            config: Configuration object containing model paths and hyperparameters.
        """
        super().__init__()
        print("creating the FLAME Decoder in MLX")
        with open(config.flame_model_path, "rb") as f:
            self.flame_model = Struct(**pickle.load(f, encoding="latin1"))
        
        self.NECK_IDX = 1
        self.batch_size = config.batch_size
        self.use_face_contour = config.use_face_contour
        self.faces = to_mx(self.flame_model.f, dtype=mx.int32)
        
        # Shape betas
        default_shape = mx.zeros((self.batch_size, 300 - config.shape_params))
        self.shape_betas = default_shape 
        
        # Expression betas
        default_exp = mx.zeros((self.batch_size, 100 - config.expression_params))
        self.expression_betas = default_exp
        
        # Eyeball and neck rotation
        self.eye_pose = mx.zeros((self.batch_size, 6))
        self.neck_pose = mx.zeros((self.batch_size, 3))
        self.transl = mx.zeros((self.batch_size, 3))
        
        self.use_3D_translation = config.use_3D_translation
        
        # Template vertices
        self.v_template = to_mx(self.flame_model.v_template)
        
        # Shape components
        self.shapedirs = to_mx(self.flame_model.shapedirs)
        
        # J regressor
        self.J_regressor = to_mx(self.flame_model.J_regressor)
        
        # Pose blend shape basis
        num_pose_basis = self.flame_model.posedirs.shape[-1]
        posedirs = np.reshape(self.flame_model.posedirs, [-1, num_pose_basis]).T
        self.posedirs = to_mx(posedirs)
        
        # Parent indices
        parents = to_mx(self.flame_model.kintree_table[0], dtype=mx.int32)
        self.parents = parents
        
        # LBS weights
        self.lbs_weights = to_mx(self.flame_model.weights)
        
        # Static landmarks
        if os.path.exists(config.static_landmark_embedding_path):
            with open(config.static_landmark_embedding_path, "rb") as f:
                static_embeddings = Struct(**pickle.load(f, encoding="latin1"))
            
            self.lmk_faces_idx = to_mx(static_embeddings.lmk_face_idx, dtype=mx.int32)
            self.lmk_bary_coords = to_mx(static_embeddings.lmk_b_coords)
        else:
            print(f"Warning: Static landmark embedding path {config.static_landmark_embedding_path} not found. Landmarks will not be computed.")
            self.lmk_faces_idx = None
            self.lmk_bary_coords = None
        
        if self.use_face_contour and os.path.exists(config.dynamic_landmark_embedding_path):
            contour_embeddings = np.load(
                config.dynamic_landmark_embedding_path,
                allow_pickle=True,
                encoding="latin1",
            )
            contour_embeddings = contour_embeddings[()]
            self.dynamic_lmk_faces_idx = to_mx(contour_embeddings["lmk_face_idx"], dtype=mx.int32)
            self.dynamic_lmk_bary_coords = to_mx(contour_embeddings["lmk_b_coords"])
            
            neck_kin_chain = []
            curr_idx = self.NECK_IDX
            while curr_idx != -1:
                neck_kin_chain.append(curr_idx)
                curr_idx = int(self.flame_model.kintree_table[0][curr_idx]) if curr_idx != 0 else -1
            self.neck_kin_chain = mx.array(neck_kin_chain, dtype=mx.int32)
        else:
            if self.use_face_contour:
                print(f"Warning: Dynamic landmark embedding path {config.dynamic_landmark_embedding_path} not found. Face contour will not be computed.")
            self.use_face_contour = False
            self.dynamic_lmk_faces_idx = None
            self.dynamic_lmk_bary_coords = None

    def _find_dynamic_lmk_idx_and_bcoords(
        self,
        vertices,
        pose,
        dynamic_lmk_faces_idx,
        dynamic_lmk_b_coords,
        neck_kin_chain,
    ):
        """
        Dynamically selects face contour landmark indices and barycentric coordinates 
        based on the global rotation of the head.
        
        Args:
            vertices (mx.array): Posed mesh vertices.
            pose (mx.array): Full joint poses (axis-angle).
            dynamic_lmk_faces_idx (mx.array): Precomputed contour face indices.
            dynamic_lmk_b_coords (mx.array): Precomputed contour barycentric coords.
            neck_kin_chain (mx.array): Chain of joint indices to compute head rotation.
            
        Returns:
            tuple: (dyn_lmk_faces_idx, dyn_lmk_b_coords)
        """
        batch_size = vertices.shape[0]
        
        # pose is (B, J, 3) where J is number of joints
        aa_pose = pose[:, neck_kin_chain] # (B, K, 3)
        
        rot_mats = batch_rodrigues(aa_pose.reshape(-1, 3)).reshape(batch_size, -1, 3, 3)
        
        rel_rot_mat = mx.broadcast_to(mx.eye(3)[None], (batch_size, 3, 3))
        for idx in range(len(neck_kin_chain)):
            rel_rot_mat = mx.matmul(rot_mats[:, idx], rel_rot_mat)
            
        # Compute head rotation angle around y-axis for contour selection
        y_rot_angle_rad = rot_mat_to_euler(rel_rot_mat)
        y_rot_angle = mx.round(mx.clip(-y_rot_angle_rad * 180.0 / np.pi, a_min=-100, a_max=39)).astype(mx.int32)
        
        # Handle negative angles for mirrored indexing
        neg_mask = y_rot_angle < 0
        mask = y_rot_angle < -39
        neg_vals = mask.astype(mx.int32) * 78 + (1 - mask.astype(mx.int32)) * (39 - y_rot_angle)
        y_rot_angle = mx.where(neg_mask, neg_vals, y_rot_angle)
        
        dyn_lmk_faces_idx = dynamic_lmk_faces_idx[y_rot_angle]
        dyn_lmk_b_coords = dynamic_lmk_b_coords[y_rot_angle]
        
        return dyn_lmk_faces_idx, dyn_lmk_b_coords

    def __call__(
        self,
        shape_params=None,
        expression_params=None,
        pose_params=None,
        neck_pose=None,
        eye_pose=None,
        transl=None,
    ):
        """
        Forward pass to generate the head mesh and landmarks.
        
        Args:
            shape_params (mx.array): Identity coefficients (B, 10-300).
            expression_params (mx.array): Expression coefficients (B, 10-100).
            pose_params (mx.array): Global rotation and jaw rotation (B, 6).
            neck_pose (mx.array): Neck rotation coefficients (B, 3). Defaults to zero.
            eye_pose (mx.array): Eyeball rotation coefficients (B, 6). Defaults to zero.
            transl (mx.array): 3D translation vector (B, 3). Defaults to zero.
            
        Returns:
            tuple: (vertices, landmarks)
                - vertices (mx.array): Posed 3D mesh vertices of shape (B, 5023, 3).
                - landmarks (mx.array): Calculated 3D facial landmarks of shape (B, 68, 3).
        """
        if shape_params is None:
            shape_params = mx.zeros((self.batch_size, 100)) # Default 100
        if expression_params is None:
            expression_params = mx.zeros((self.batch_size, 50)) # Default 50
        if pose_params is None:
            pose_params = mx.zeros((self.batch_size, 6))

        # Concatenate base and variation betas
        betas = mx.concatenate(
            [shape_params, self.shape_betas, expression_params, self.expression_betas],
            axis=1,
        )
        
        neck_pose = neck_pose if neck_pose is not None else self.neck_pose
        eye_pose = eye_pose if eye_pose is not None else self.eye_pose
        transl = transl if transl is not None else self.transl
        
        # full_pose: [global_rot, neck_pose, jaw_pose, eye_pose]
        full_pose = mx.concatenate(
            [pose_params[:, :3], neck_pose, pose_params[:, 3:6], eye_pose], axis=1
        ).reshape(self.batch_size, -1, 3)
        
        template_vertices = mx.broadcast_to(self.v_template[None], (self.batch_size, *self.v_template.shape))
        
        # Linear Blend Skinning
        vertices, _ = lbs(
            betas,
            full_pose,
            template_vertices,
            self.shapedirs,
            self.posedirs,
            self.J_regressor,
            self.parents,
            self.lbs_weights,
            pose2rot=True
        )
        
        # Landmark interpolation
        if self.lmk_faces_idx is not None:
            lmk_faces_idx = mx.broadcast_to(self.lmk_faces_idx[None], (self.batch_size, *self.lmk_faces_idx.shape))
            lmk_bary_coords = mx.broadcast_to(self.lmk_bary_coords[None], (self.batch_size, *self.lmk_bary_coords.shape))
            
            if self.use_face_contour:
                dyn_lmk_faces_idx, dyn_lmk_bary_coords = self._find_dynamic_lmk_idx_and_bcoords(
                    vertices,
                    full_pose,
                    self.dynamic_lmk_faces_idx,
                    self.dynamic_lmk_bary_coords,
                    self.neck_kin_chain
                )
                lmk_faces_idx = mx.concatenate([dyn_lmk_faces_idx, lmk_faces_idx], axis=1)
                lmk_bary_coords = mx.concatenate([dyn_lmk_bary_coords, lmk_bary_coords], axis=1)
                
            landmarks = vertices2landmarks(vertices, self.faces, lmk_faces_idx, lmk_bary_coords)
        else:
            landmarks = None
        
        # Apply 3D translation
        if self.use_3D_translation:
            if landmarks is not None:
                landmarks += transl[:, None, :]
            vertices += transl[:, None, :]
            
        return vertices, landmarks


```
