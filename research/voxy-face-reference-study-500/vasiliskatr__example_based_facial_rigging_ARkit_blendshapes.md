# Facial rendering source scan: vasiliskatr/example_based_facial_rigging_ARkit_blendshapes

Repository: https://github.com/vasiliskatr/example_based_facial_rigging_ARkit_blendshapes

Pinned source revision: 39db32431f3bab2ebb91a74560a61da1ab40af05

Discovery topic: blendshape

Description: 

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"eye": ["data/generic_blendShapes/eyeBlink_L.obj", "data/generic_blendShapes/eyeBlink_R.obj", "data/generic_blendShapes/eyeLookDown_L.obj", "data/generic_blendShapes/eyeLookDown_R.obj", "data/generic_blendShapes/eyeLookIn_L.obj", "data/generic_blendShapes/eyeLookIn_R.obj", "data/generic_blendShapes/eyeLookOut_L.obj", "data/generic_blendShapes/eyeLookOut_R.obj", "data/generic_blendShapes/eyeLookUp_L.obj", "data/generic_blendShapes/eyeLookUp_R.obj", "data/generic_blendShapes/eyeSquint_L.obj", "data/generic_blendShapes/eyeSquint_R.obj", "data/generic_blendShapes/eyeWide_L.obj", "data/generic_blendShapes/eyeWide_R.obj"], "blendshape": ["data/generic_blendShapes/Neutral.obj", "data/generic_blendShapes/browDown_L.obj", "data/generic_blendShapes/browDown_R.obj", "data/generic_blendShapes/browInnerUp.obj", "data/generic_blendShapes/browOuterUp_L.obj", "data/generic_blendShapes/browOuterUp_R.obj", "data/generic_blendShapes/cheekPuff.obj", "data/generic_blendShapes/cheekSquint_L.obj", "data/generic_blendShapes/cheekSquint_R.obj", "data/generic_blendShapes/eyeBlink_L.obj", "data/generic_blendShapes/eyeBlink_R.obj", "data/generic_blendShapes/eyeLookDown_L.obj", "data/generic_blendShapes/eyeLookDown_R.obj", "data/generic_blendShapes/eyeLookIn_L.obj", "data/generic_blendShapes/eyeLookIn_R.obj", "data/generic_blendShapes/eyeLookOut_L.obj", "data/generic_blendShapes/eyeLookOut_R.obj", "data/generic_blendShapes/eyeLookUp_L.obj", "data/generic_blendShapes/eyeLookUp_R.obj", "data/generic_blendShapes/eyeSquint_L.obj", "data/generic_blendShapes/eyeSquint_R.obj", "data/generic_blendShapes/eyeWide_L.obj", "data/generic_blendShapes/eyeWide_R.obj", "data/generic_blendShapes/jawForward.obj", "data/generic_blendShapes/jawLeft.obj", "data/generic_blendShapes/jawOpen.obj", "data/generic_blendShapes/jawRight.obj", "data/generic_blendShapes/mouthClose.obj", "data/generic_blendShapes/mouthDimple_L.obj", "data/generic_blendShapes/mouthDimple_R.obj"], "jaw": ["data/generic_blendShapes/jawForward.obj", "data/generic_blendShapes/jawLeft.obj", "data/generic_blendShapes/jawOpen.obj", "data/generic_blendShapes/jawRight.obj"], "mouth": ["data/generic_blendShapes/mouthClose.obj", "data/generic_blendShapes/mouthDimple_L.obj", "data/generic_blendShapes/mouthDimple_R.obj", "data/generic_blendShapes/mouthFrown_L.obj", "data/generic_blendShapes/mouthFrown_R.obj", "data/generic_blendShapes/mouthFunnel.obj", "data/generic_blendShapes/mouthLeft.obj", "data/generic_blendShapes/mouthLowerDown_L.obj", "data/generic_blendShapes/mouthLowerDown_R.obj", "data/generic_blendShapes/mouthPress_L.obj", "data/generic_blendShapes/mouthPress_R.obj", "data/generic_blendShapes/mouthPucker.obj", "data/generic_blendShapes/mouthRight.obj", "data/generic_blendShapes/mouthRollLower.obj", "data/generic_blendShapes/mouthRollUpper.obj", "data/generic_blendShapes/mouthShrugLower.obj", "data/generic_blendShapes/mouthShrugUpper.obj", "data/generic_blendShapes/mouthSmile_L.obj", "data/generic_blendShapes/mouthSmile_R.obj", "data/generic_blendShapes/mouthStretch_L.obj", "data/generic_blendShapes/mouthStretch_R.obj", "data/generic_blendShapes/mouthUpperUp_L.obj", "data/generic_blendShapes/mouthUpperUp_R.obj"], "rig": ["data/generic_blendShapes/jawRight.obj", "data/generic_blendShapes/mouthRight.obj", "local_packages/ExampleBasedRigging.py", "local_packages/__pycache__/ExampleBasedRigging.cpython-37.pyc"]}

## Source file: local_packages/ExampleBasedRigging.py

{
  "path": "local_packages/ExampleBasedRigging.py",
  "blob_sha": "fb6f739e3672089aa6f282ef746f22b2dd24c72b",
  "sha256": "4e1c4d6510067688bcc374ce0ee6eff13731e5b776ce8d61e322c1ffb33ab703",
  "lines": 256,
  "bytes": 8778,
  "selection_score": 13,
  "keyword_line_counts": {
    "blend_shape": 1,
    "deform": 1,
    "face": 2
  },
  "url": "https://github.com/vasiliskatr/example_based_facial_rigging_ARkit_blendshapes/blob/39db32431f3bab2ebb91a74560a61da1ab40af05/local_packages/ExampleBasedRigging.py",
  "function_signature_candidates": [
    {
      "line": 24,
      "text": "def reading_generic_bs (objpath_generic_bs):"
    },
    {
      "line": 49,
      "text": "def reading_training_data(objpath_training_poses):"
    },
    {
      "line": 67,
      "text": "def blend_shape_weights(A_0, B_0, A_BS_model, S_training_poses):"
    },
    {
      "line": 94,
      "text": "def columnise(model):"
    },
    {
      "line": 102,
      "text": "def local_tri_frame_fast(vertices, triangles, tri_index):"
    },
    {
      "line": 120,
      "text": "def compute_lf_fast (vertices, triangles):"
    },
    {
      "line": 130,
      "text": "def compute_lf_inverse_fast(vertices, triangles):"
    },
    {
      "line": 139,
      "text": "def make_M_S_minus_M_B_0_fast(S_training_poses, B_0, triangles):"
    },
    {
      "line": 155,
      "text": "def make_W_seed_fast(triangles, A_BS_model, kappa, theta):"
    },
    {
      "line": 169,
      "text": "def make_M_A_star_fast(triangles, A_0, B_0, A_BS_model):"
    },
    {
      "line": 189,
      "text": "def lf_optimisation (num_triangles, A, M_S_minus_M_B_0, M_B, M_A_star, beta, gamma, W_seed, opt_iteration, n, m):"
    },
    {
      "line": 209,
      "text": "def make_A_sparse_reconstruction (triangles, n_vertices):"
    },
    {
      "line": 243,
      "text": "def recon(M_B, A_sparse_recon, n_vertices, num_triangles, i):"
    }
  ]
}

```text

import numpy as np
import os
import time
import local_packages.tools3d_ as t3d 
from scipy.optimize import lsq_linear
from IPython.display import clear_output
from scipy.sparse import csr_matrix
import scipy.sparse as sp
from scipy.sparse.linalg import spsolve
from qpsolvers import solve_qp
import numba
from numba import jit
from numba.core.extending import overload
from numba.np.linalg import norm_impl
from numba.core.errors import NumbaDeprecationWarning, NumbaPendingDeprecationWarning, NumbaPerformanceWarning
import warnings
warnings.simplefilter('ignore', category=NumbaDeprecationWarning)
warnings.simplefilter('ignore', category=NumbaPendingDeprecationWarning)
warnings.simplefilter('ignore', category=NumbaPerformanceWarning)
from tqdm import tqdm



def reading_generic_bs (objpath_generic_bs):
    A_bs_model = [] # variable to store all data with regards to the template 
    B_bs_model = [] # variable to store all data with regards to the actor
    bs_name_list = [] # variable to store the names of imported BS
    A_0, faces, _, _ = t3d.Read(objpath_generic_bs + 'Neutral.obj', QuadMode = True)
    n_vertices = A_0.shape[1]
    #A_0 = tools_3d.center(A_0)
    generic_bs_data = os.scandir(objpath_generic_bs)
    for generic_bs in tqdm(generic_bs_data, unit=' files', desc="Loading generic blend shapes"):
        name, ext = os.path.splitext(generic_bs)
        name_s = name.split("/")
       
        if ext == '.obj' and 'neutral' not in name:  # read only the .obj files from the source directory
            temp_vertices, _, _, _ = t3d.Read(name+ext, QuadMode = True)
            A_bs_vertices = temp_vertices - A_0
            A_bs_model.append(A_bs_vertices)
            B_bs_model.append(np.zeros((3, n_vertices)))
            bs_name_list.append(name_s[-1])
            
    n = len(A_bs_model)
    print ('Generic model of n = ' + str(len(A_bs_model)) + ' blend shapes imported (+1 neutral pose)') 
    return A_bs_model, B_bs_model, A_0, faces, n, bs_name_list



def reading_training_data(objpath_training_poses):    
    S_training_poses = [] # Variable to store training poses
    training_pose_data = os.scandir(objpath_training_poses)

    for training_pose in tqdm(training_pose_data, unit=' files', desc="Loading poses"):
        name, ext = os.path.splitext(training_pose)

        if ext == '.obj':  # read only the .obj files from the source directory    
                temp_vertices, _, _, _ = t3d.Read(name+ext, QuadMode = True)
                
                S_training_poses.append(temp_vertices)
                
    m = len(S_training_poses)  
    print ('m = ' + str(m)+' training poses in total (+ 1 neutral)')
    return S_training_poses, m



def blend_shape_weights(A_0, B_0, A_BS_model, S_training_poses):
    # initial blend shape weights guessing for each training pose

    start_time = time.time()
    print ('Computing intial blend-shape weight guess for the training poses')
    n = len(A_BS_model)
    m = len(S_training_poses)
    n_vertices = A_0.shape[1]
    
    Alpha_star = np.zeros((m, n))# Initial guess of blendhspae weights

    A_All = A_BS_model[0].T.flatten().reshape(n_vertices*3, 1)

    for i in  range(1,n):
        A_All = np.concatenate((A_All, A_BS_model[i].T.flatten().reshape(n_vertices*3, 1)), axis=1)

    for i in tqdm(range(m), unit=' pose', desc='Guessing weights'):        
        B_temp = (S_training_poses[i] - B_0).T.flatten().reshape(n_vertices*3)
        weights_temp = lsq_linear(A_All, B_temp, bounds = (0, 1), lsmr_tol='auto', verbose=0)
        Alpha_star[i, :] = weights_temp.x.reshape(1, n)
    
    return Alpha_star


    print ("done in ",(time.time() - start_time), "sec")      


def columnise(model):
    for i in range(0, len(model)):
        model[i] = model[i].T

    return model


@jit(nopython=True, parallel=True)
def local_tri_frame_fast(vertices, triangles, tri_index):
    
    tri_vertices = vertices[triangles[tri_index, :], :]
    
    LF = np.zeros((3,3))
    
    v1 = tri_vertices[0, :]
    v2 = tri_vertices[1, :]
    v3 = tri_vertices[2, :]
    
    LF[:,0] = (v3-v1) # v3-v1
    LF[:,1] = (v2-v1) # v2-v1
    LF[:,2] = (np.cross((v3-v1),(v2-v1))) # n
    
    return LF


@jit(nopython=True, parallel=True)
def compute_lf_fast (vertices, triangles):

    lf = np.zeros((len(triangles)*3, 3))
    for i in numba.prange(len(triangles)): 
        lf[i*3:i*3+3]= local_tri_frame_fast(vertices, triangles, i)
    
    return lf


@jit(nopython=True, parallel=True)
def compute_lf_inverse_fast(vertices, triangles):
  
    lf_inv = np.zeros((len(triangles)*3, 3))
    for i in numba.prange(len(triangles)):  
        lf_inv[i*3:i*3+3] = np.linalg.inv(local_tri_frame_fast(vertices, triangles, i))
    return lf_inv


@jit(nopython=True, parallel=True)
def make_M_S_minus_M_B_0_fast(S_training_poses, B_0, triangles):
    
    m = len(S_training_poses)
    M_B_0 = compute_lf_fast(B_0, triangles)    
    M_S_minus_M_B_0 = np.empty((m, len(triangles)*3, 3))
    M_S = np.empty((m, len(triangles)*3, 3))

    for s in numba.prange(m):
        M_S_temp = compute_lf_fast(S_training_poses[s], triangles)
        M_S_minus_M_B_0[s] = M_S_temp - M_B_0     
        M_S[s] = M_S_temp
      
    return M_S_minus_M_B_0 , M_B_0, M_S


@jit(nopython=True, parallel=True)
def make_W_seed_fast(triangles, A_BS_model, kappa, theta):  
    n = len(A_BS_model) 
    W_seed = np.empty((n, len(triangles)))
    for i in numba.prange(n):
        M_A_i = compute_lf_fast(A_BS_model[i], triangles)
        
        for j in numba.prange(len(triangles)):
            lf_tri_norm = np.linalg.norm(M_A_i[j*3:j*3+3,:])
            W_seed[i,j] = (1 + lf_tri_norm)/np.power((kappa + lf_tri_norm), theta)
            
    return W_seed


@jit(nopython=True, parallel=True)
def make_M_A_star_fast(triangles, A_0, B_0, A_BS_model):
    n = len(A_BS_model)
    M_A_star = np.empty((n, len(triangles)*3, 3))
    M_A_0_inv = compute_lf_inverse_fast(A_0, triangles) 
    M_A_0 = compute_lf_fast(A_0, triangles)
    M_B_0 = compute_lf_fast(B_0, triangles)

    for i in numba.prange(n):
        M_A_i = compute_lf_fast(A_BS_model[i], triangles)
        M_A_sum = M_A_0 + M_A_i 
        
        for j in numba.prange(len(triangles)):   
            M_A_star[i][j*3:j*3+3] = ((M_A_sum[j*3:j*3+3] @ M_A_0_inv[j*3:j*3+3]) @ M_B_0[j*3:j*3+3]) - M_B_0[j*3:j*3+3]
        
    return M_A_star
    

    
# Parallel version lf optimisation
@jit(nopython=True, parallel=True)
def lf_optimisation (num_triangles, A, M_S_minus_M_B_0, M_B, M_A_star, beta, gamma, W_seed, opt_iteration, n, m):
    
    for tri_index in numba.prange(num_triangles): 
         # Constructing Bfit
        B_fit = np.zeros((n*3,3))
        B_fit = A.T @ M_S_minus_M_B_0[:,tri_index*3:tri_index*3+3,:].copy().reshape(m*3,3)
         
        # Constructing W   
        dia = [[i,i,i] for i in W_seed[:,tri_index]]
        dia = np.asarray(dia)
        dia = dia.flatten()
        W = np.diag(dia, 0)
        M_A_starr = M_A_star[:,tri_index*3:tri_index*3+3,:].copy().reshape(n*3,3) 
        A_sum = A.T @ A + beta[opt_iteration] * (W.T @ W)
        B_sum = B_fit + beta[opt_iteration] * (W.T @ (W @ M_A_starr))                 
        M_B_tri = np.linalg.solve(A_sum, B_sum[:,0:2])    #.copy() 
        M_B[:, tri_index*2:tri_index*2+2] = M_B_tri #.copy()
        
    return  M_B 

def make_A_sparse_reconstruction (triangles, n_vertices):
    row = []
    col = []
    data = []
        
    for j in range(len(triangles)):
        tri_indices = triangles[j]

        row.append(j*2)
        col.append(tri_indices[2])
        data.append(1)

        row.append(j*2)
        col.append(tri_indices[0])
        data.append(-1)

        row.append(j*2+1)
        col.append(tri_indices[1])
        data.append(1)

        row.append(j*2+1)
        col.append(tri_indices[0])
        data.append(-1)
        
    row = np.asarray(row)
    col = np.asarray(col)
    data = np.asarray(data)
        
    ########### I removed the consideration of zero-deformation vertices.
    ########### There is no drifting in the reconstruction even without it.    
    A_sparse = csr_matrix((data, (row, col)), shape=(triangles.shape[0]*2, n_vertices))
    return A_sparse


def recon(M_B, A_sparse_recon, n_vertices, num_triangles, i):
      # reconstruction of vertices 
    
    B_temp_X = M_B[i*3,:].reshape(num_triangles*2, 1)
    X_vals = sp.linalg.lsqr(A_sparse_recon, B_temp_X)[0]
    B_temp_Y = M_B[i*3+1,:].reshape(num_triangles*2, 1)
    Y_vals = sp.linalg.lsqr(A_sparse_recon, B_temp_Y)[0]
    B_temp_Z = M_B[i*3+2,:].reshape(num_triangles*2, 1)
    Z_vals = sp.linalg.lsqr(A_sparse_recon, B_temp_Z)[0]
    
    return X_vals.reshape(1, n_vertices), Y_vals.reshape(1, n_vertices), Z_vals.reshape(1, n_vertices), i 





```

## Source file: landmarks/__init__.py

{
  "path": "landmarks/__init__.py",
  "blob_sha": "8b137891791fe96927ad78e64b0aad7bded08bdc",
  "sha256": "01ba4719c80b6fe911b091a7c05124b64eeece964e09c058ef8f9805daca546b",
  "lines": 1,
  "bytes": 1,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/vasiliskatr/example_based_facial_rigging_ARkit_blendshapes/blob/39db32431f3bab2ebb91a74560a61da1ab40af05/landmarks/__init__.py",
  "function_signature_candidates": []
}

```text




```

## Source file: landmarks/LICT_narrow_r.py

{
  "path": "landmarks/LICT_narrow_r.py",
  "blob_sha": "d057f24c380ef285a042e20ae64db3e062a23a16",
  "sha256": "3121d297623addd7960b3df15beb986d7164c8dc963e361c391e4ece57b8b682",
  "lines": 32,
  "bytes": 913,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 8,
    "mouth": 6,
    "rig": 4
  },
  "url": "https://github.com/vasiliskatr/example_based_facial_rigging_ARkit_blendshapes/blob/39db32431f3bab2ebb91a74560a61da1ab40af05/landmarks/LICT_narrow_r.py",
  "function_signature_candidates": []
}

```text

import numpy as np
LM = np.zeros(18).astype(int)
### Indices for key landmarks
LM[0] = 5069 #lm_nose_tip 

LM[1] = 3888#lm_right_eye_outer 3888
LM[2] = 3621 #lm_right_eye_inner (tears) 3621

LM[3] = 1244  #lm_left_eye_inner (tears) 1244
LM[4] = 2023 #lm_left_eye_outer 2023

LM[5] = 6128#lm_mouth_R 
LM[6] = 5567  #lm_mouth_L 5567

LM[7] = 6323#lm_mouth_middle_Top_T 
LM[8] = 6248 #lm_mouth_middle_Top_B

LM[9] = 6399 #lm_mouth_middle_Bottom_T 
LM[10] = 6414 #lm_mouth_middle_Bottom_B 

LM[11] = 3138#center_chin

# Use edge vertices for reigid alignment (the less the better)
LM[12] =  2141 # top_of_mask_mid (edge vertice)
LM[13] =  1844 # mask_edge_Right_eyeheight (edge vertice)
LM[14] = 4063 # mask_edge_left_eyeheight (edge vertice)
LM[15] = 829# mask_edge_right_beloweyeheight (edge vertice)
LM[16] = 3083# mask_edge_left_beloweyeheight (edge vertice)
LM[17] = 2003# bottom_mask_neck)mid (edge vertice)





```

## Source file: local_packages/__init__.py

{
  "path": "local_packages/__init__.py",
  "blob_sha": "8b137891791fe96927ad78e64b0aad7bded08bdc",
  "sha256": "01ba4719c80b6fe911b091a7c05124b64eeece964e09c058ef8f9805daca546b",
  "lines": 1,
  "bytes": 1,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/vasiliskatr/example_based_facial_rigging_ARkit_blendshapes/blob/39db32431f3bab2ebb91a74560a61da1ab40af05/local_packages/__init__.py",
  "function_signature_candidates": []
}

```text




```
