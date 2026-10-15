# Facial rendering source scan: michaelStettler/FacialRetargeting

Repository: https://github.com/michaelStettler/FacialRetargeting

Pinned source revision: ef65d185d365c68881c0c352c3b502775f6ee123

Discovery topic: facial retargeting

Description: Automatic Facial Retargeting

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"blendshape": ["06_build_actor_blendshapes.py", "maya_scripts/02_create_blendshapes_nodes.py", "utils/remove_neutral_blendshape.py"]}

## Source file: 06_build_actor_blendshapes.py

{
  "path": "06_build_actor_blendshapes.py",
  "blob_sha": "e0fd507ad594d2a7735ce5aa051ee427af116c11",
  "sha256": "bb9626e3d94f6a77b35a4acaba6ab449b9516c0d22f15827549f1844eaaa1b6d",
  "lines": 275,
  "bytes": 11682,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 22
  },
  "url": "https://github.com/michaelStettler/FacialRetargeting/blob/ef65d185d365c68881c0c352c3b502775f6ee123/06_build_actor_blendshapes.py",
  "function_signature_candidates": []
}

```text

import numpy as np
import time as time
import os
import json
import matplotlib.pyplot as plt

from utils.load_data import load_training_frames
from utils.compute_delta import compute_delta
from utils.remove_neutral_blendshape import remove_neutral_blendshape
from utils.normalize_positions import normalize_positions
from utils.align_to_head_markers import align_to_head_markers
from utils.modify_axis import modify_axis
from utils.re_order_delta import re_order_delta
from utils.get_key_expressions import get_key_expressions
from src.compute_corr_coef import compute_corr_coef
from src.compute_corr_coef import compute_tilda_corr_coef
from src.compute_trust_values import compute_trust_values
from src.get_soft_mask import get_soft_mask
from src.EAlign import EAlign
from src.RBF_warp import get_initial_actor_blendshapes
from utils.plotting import plot_similarities


"""
run: python -m blendshape_transfer
"""
np.set_printoptions(precision=4, linewidth=200, suppress=True)

# load and define parameters
with open("C:/Users/Michael/PycharmProjects/FacialRetargeting/configs/David_to_Louise_v2.json") as f:
    config = json.load(f)

alpha = int(config['alpha'])
beta = int(config['beta'])
print("[PARAMS] alpha:", alpha)
print("[PARAMS] beta:", beta)
print("[PARAMS] sk loaded:", config['vertices_pos_name'])

max_num_seq = None  # set to None if we want to use all the sequences
do_plot = True
save = True
load_pre_processed = True

# load data
mesh_list = np.load(os.path.join(config['python_data_path'], config['maya_bs_mesh_list']+'.npy')).astype(str)
sk = np.load(os.path.join(config['python_data_path'], config['vertices_pos_name']+'npy'))  # sparse representation of the blendshapes (vk)
# get Neutral ref index and new cleaned mesh list
cleaned_mesh_list, bs_index, ref_index = remove_neutral_blendshape(mesh_list, config['neutral_pose'])

# get neutral (reference) blendshape and normalize it
ref_sk, min_sk, max_sk = normalize_positions(np.copy(sk[ref_index]), return_min=True, return_max=True)

# normalize sk
sk = normalize_positions(sk, min_pos=min_sk, max_pos=max_sk)

if do_plot:
    fig = plt.figure()
    ax = fig.add_subplot(1, 1, 1, projection='3d')
    ax.plot_trisurf(ref_sk[:, 0], ref_sk[:, 1], ref_sk[:, 2])
    ax.set_title("Ref sk")
    ax.set_xlabel('X Label')
    ax.set_ylabel('Y Label')
    ax.set_zlabel('Z Label')

# compute delta sparse blendshape
delta_sk = compute_delta(sk[bs_index, :, :], ref_sk)

# test if delta_sk has no none-unique entry
test_unique = np.unique(delta_sk, axis=1)
if np.shape(test_unique)[0] != np.shape(delta_sk)[0]:
    raise ValueError("delta_sk contains non unique entry! Check your index dictionary to build the sparse blendshape "
                     "(maya_scripts.03_extract_blendshapes_pos_and_obj)")

# ----------------------------------------------------------------------------------------------------------------------
# get Actor Animation
# ----------------------------------------------------------------------------------------------------------------------

# load ref pose
ref_actor_pose = np.load(os.path.join(config['python_data_path'], config['neutral_pose_positions']+'.npy'))
# align sequence with the head markers
head_markers = range(np.shape(ref_actor_pose)[0] - 4, np.shape(ref_actor_pose)[0] - 1)  # use only 3 markers
ref_actor_pose = align_to_head_markers(ref_actor_pose, ref_idx=head_markers)

if do_plot:
    fig = plt.figure()
    ax = fig.add_subplot(1, 1, 1, projection='3d')
    ax.scatter(ref_actor_pose[:, 0], ref_actor_pose[:, 1], ref_actor_pose[:, 2])
    ax.set_title("ref pose A0")
    ax.set_xlabel('X Label')
    ax.set_ylabel('Y Label')
    ax.set_zlabel('Z Label')

ref_actor_pose = ref_actor_pose[:-4, :]  # remove HEAD markers
# modify axis from xzy to xyz to match the scatter blendshape axis orders
ref_actor_pose = modify_axis(ref_actor_pose, order='xzy2xyz', inverse_z=True)
# normalize reference (neutral) actor positions
ref_actor_pose, min_af, max_af = normalize_positions(ref_actor_pose, return_min=True, return_max=True)

if do_plot:
    fig = plt.figure()
    ax = fig.add_subplot(1, 1, 1, projection='3d')
    ax.scatter(ref_actor_pose[:, 0], ref_actor_pose[:, 1], ref_actor_pose[:, 2])
    ax.set_title("ref pose A0 normalized")
    ax.set_xlabel('X Label')
    ax.set_ylabel('Y Label')
    ax.set_zlabel('Z Label')

if load_pre_processed:
    delta_af = np.load("data/training_delta_af_v2.npy")
    tilda_ckf = np.load("data/training_tilda_ckf_v2.npy")
else:
    # load sequence
    af = load_training_frames(config['mocap_folder'],
                              num_markers=int(config['num_markers']),
                              template_labels=config['template_labels'],
                              max_num_seq=max_num_seq,
                              down_sample_factor=5)
    af = align_to_head_markers(af, ref_idx=head_markers)
    af = af[:, :-4, :]  # remove HEAD markers
    # modify axis from xyz to xzy to match the scatter blendshape axis orders
    af = modify_axis(af, order='xzy2xyz', inverse_z=True)
    af = normalize_positions(af, min_pos=min_af, max_pos=max_af)

    if do_plot:
        fig = plt.figure()
        ax = fig.add_subplot(1, 1, 1, projection='3d')
        ax.scatter(ref_actor_pose[:, 0], ref_actor_pose[:, 1], ref_actor_pose[:, 2])
        ax.scatter(af[0, :, 0], af[0, :, 1], af[0, :, 2], c='RED')
        # ax.scatter(af[5, :, 0], af[5, :, 1], af[5, :, 2], c='RED')
        # ax.scatter(af[10, :, 0], af[10, :, 1], af[10, :, 2], c='RED')
        # ax.scatter(af[2575, :, 0], af[2575, :, 1], af[2575, :, 2], c='YELLOW')
        ax.set_title("ref_pose A0 vs. af[0]")
        ax.set_xlabel('X Label')
        ax.set_ylabel('Y Label')
        ax.set_zlabel('Z Label')

    delta_af = compute_delta(af, ref_actor_pose, norm_thresh=2)

    print("[data] shape af:", np.shape(af))

print("[data] Finished loading data")
print("[data] Neutral Blendshape index:", ref_index)
print("[data] shape ref_actor_pose", np.shape(ref_actor_pose))
print("[data] shape delta af:", np.shape(delta_af))
print("[data] shape sk", np.shape(sk))
print("[data] shape delta_sk", np.shape(delta_sk))
print("[data] cleaned_mesh_list:", len(cleaned_mesh_list))

# get dimensions
K, M, n_dim = np.shape(delta_sk)
F = np.shape(delta_af)[0]
print("[data] num_blendshapes:", K)
print("[data] num_markers:", M)
print("[data] num_features (M*3):", M*n_dim)
print("[data] num_frames", F)
print()

# 1) Facial Motion Similarity
# reorder delta blendshapes
sorted_delta_sk, sorted_index = re_order_delta(delta_sk)
sorted_mesh_list = np.array(cleaned_mesh_list)[sorted_index]
print("[Pre-processing] shape sorted_delta_sk", np.shape(sorted_delta_sk))
print("[Pre-processing] len sorted_mesh_list", len(sorted_mesh_list))

if not load_pre_processed:
    # measure similarity between character blendshapes and actor's capture performance
    ckf = compute_corr_coef(np.reshape(delta_af, (np.shape(delta_af)[0], -1)),
                            np.reshape(sorted_delta_sk, (np.shape(sorted_delta_sk)[0], -1)))

    if do_plot:
        plot_similarities(ckf, "Fig. 7: Motion space similarity")

    # contrast enhancement
    tk = compute_trust_values(np.reshape(sorted_delta_sk, (np.shape(sorted_delta_sk)[0], -1)), do_plot=do_plot)
    tilda_ckf = compute_tilda_corr_coef(ckf, tk)
    print("[Pre-processing] shape ckf", np.shape(ckf))
    print("[Pre-processing] shape tk", np.shape(tk))
    print("[Pre-processing] shape tilda_ckf", np.shape(tilda_ckf))
    print()

    # 2) Key Expression Extraction
    key_expressions_idx = get_key_expressions(tilda_ckf, ksize=3, theta=2, do_plot=do_plot)
    F = len(key_expressions_idx)
    delta_af = delta_af[key_expressions_idx, :, :]
    tilda_ckf = tilda_ckf[:, key_expressions_idx]
    print("[Key Expr. Extract.] Keep", F, "frames")
    print("[Key Expr. Extract.] shape key_expressions", np.shape(key_expressions_idx))
    print("[Key Expr. Extract.] shape delta_af", np.shape(delta_af))
    print("[Key Expr. Extract.] shape tilda_ckf", np.shape(tilda_ckf))
    print()
    np.save("data/training_delta_af", delta_af)
    np.save("data/training_tilda_ckf", tilda_ckf)

# 3) Manifold Alignment
# built soft max vector
uk = get_soft_mask(sorted_delta_sk)
print("[SoftMax] shape uk", np.shape(uk))
print()

# 4) Geometric Constraint
# build initial guess blendshape using RBF wrap (in delta space)
delta_gk = get_initial_actor_blendshapes(ref_sk, ref_actor_pose, sorted_delta_sk)
print("[RBF Wrap] shape delta_gk", np.shape(delta_gk))
print()

if do_plot:
    fig = plt.figure()
    ax = fig.add_subplot(1, 1, 1, projection='3d')
    bs_idx = 0
    sk0 = ref_sk + sorted_delta_sk[bs_idx]
    ax.plot_trisurf(sk0[:, 0], sk0[:, 1], sk0[:, 2], alpha=0.6)
    gk0 = ref_sk + delta_gk[bs_idx]
    ax.plot_trisurf(gk0[:, 0], gk0[:, 1], gk0[:, 2], alpha=0.6)
    ax.set_title("delta sk[{}] vs. initial actor blendshape gk[{}]".format(bs_idx, bs_idx))
    ax.set_xlabel('X Label')
    ax.set_ylabel('Y Label')
    ax.set_zlabel('Z Label')

# ----------------------------------------------------------------------------
# ----------------------------------------------------------------------------
# ----------------------------------------------------------------------------
# 5) build personalized actor-specific blendshapes (delta_p)
# reshape to match required dimensions
delta_af = np.reshape(delta_af, (F, M*n_dim))
sorted_delta_sk = np.reshape(sorted_delta_sk, (K, M*n_dim))
# print control of all shapes
print("[dp] shape tilda_ckf:", np.shape(tilda_ckf))
print("[dp] shape uk:", np.shape(uk))
print("[dp] shape delta_af:", np.shape(delta_af))
print("[dp] shape delta_gk:", np.shape(delta_gk))
print("[dp] shape delta_sk", np.shape(sorted_delta_sk))
# declare E_Align
e_align = EAlign(tilda_ckf, uk, delta_af, delta_gk, ref_sk, sorted_delta_sk, alpha=alpha, beta=beta)
# compute personalized actor-specific blendshapes
start = time.time()
delta_p = e_align.compute_actor_specific_blendshapes(vectorized=False)
print("[dp] Solved in:", time.time() - start)
print("[dp] shape delta_p", np.shape(delta_p))
print()
# ----------------------------------------------------------------------------
# ----------------------------------------------------------------------------
# ----------------------------------------------------------------------------

# 6) save delta_p ans sorted_mesh_list
if save:
    saving_name = config['dp_name']+'_alpha_'+config['alpha']+'_beta_'+config['beta']
    np.save(os.path.join(config['python_data_path'], saving_name), delta_p)
    np.save(os.path.join(config['python_data_path'], config['sorted_maya_bs_mesh_list']), sorted_mesh_list)
    print("[save] saved delta_pk (actor specifik blendshapes), shape:", np.shape(delta_p))
    print("[save] saved sorted_mesh_list, shape:", np.shape(delta_p))

if do_plot:
    fig = plt.figure()
    ax = fig.add_subplot(1, 1, 1, projection='3d')
    gk0 = ref_sk + delta_gk[bs_idx]
    ax.plot_trisurf(gk0[:, 0], gk0[:, 1], gk0[:, 2])
    delta_p = np.reshape(delta_p, (K, M, n_dim))
    pk0 = ref_sk + delta_p[bs_idx]
    ax.plot_trisurf(pk0[:, 0], pk0[:, 1], pk0[:, 2], alpha=0.6)
    ax.set_title("initial dgk[{}] vs. optimized dpk[{}]".format(bs_idx, bs_idx))
    ax.set_xlabel('X Label')
    ax.set_ylabel('Y Label')
    ax.set_zlabel('Z Label')

    fig = plt.figure()
    ax = fig.add_subplot(1, 1, 1, projection='3d')
    # ax.plot_trisurf(sk0[:, 0], sk0[:, 1], sk0[:, 2], alpha=0.6)
    ax.plot_trisurf(pk0[:, 0], pk0[:, 1], pk0[:, 2], alpha=1.0)
    ax.set_title("sk[{}] vs. optimized dpk[{}]".format(bs_idx, bs_idx))
    ax.set_xlabel('X Label')
    ax.set_ylabel('Y Label')
    ax.set_zlabel('Z Label')

if do_plot:
    plt.show()


```

## Source file: utils/remove_neutral_blendshape.py

{
  "path": "utils/remove_neutral_blendshape.py",
  "blob_sha": "871c221805fbe408079b77129e0942750eccded3",
  "sha256": "40607720221c6143b1a0150f3d014232a27127a89a5afceb32f41fc839ea4a9b",
  "lines": 15,
  "bytes": 459,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 2
  },
  "url": "https://github.com/michaelStettler/FacialRetargeting/blob/ef65d185d365c68881c0c352c3b502775f6ee123/utils/remove_neutral_blendshape.py",
  "function_signature_candidates": [
    {
      "line": 1,
      "text": "def remove_neutral_blendshape(mesh_list, neutral_pose_name):"
    }
  ]
}

```text

def remove_neutral_blendshape(mesh_list, neutral_pose_name):

    ref_index = None
    bs_index = []
    cleaned_mesh_list = []
    for m, mesh in enumerate(mesh_list):
        if mesh == neutral_pose_name:
            ref_index = m
        else:
            bs_index.append(m)
            cleaned_mesh_list.append(mesh)
    if ref_index is None:
        raise ValueError("No Neutral blendshape pose found!")

    return cleaned_mesh_list, bs_index, ref_index

```

## Source file: maya_scripts/02_create_blendshapes_nodes.py

{
  "path": "maya_scripts/02_create_blendshapes_nodes.py",
  "blob_sha": "75d14762d3569e9d8ffed49a983c2d450334130c",
  "sha256": "a32c83d5a89176dcfd0d5cdac4e23657e5c606e82fba871fd8735b10fdfbbdc1",
  "lines": 28,
  "bytes": 1046,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 4,
    "rig": 1
  },
  "url": "https://github.com/michaelStettler/FacialRetargeting/blob/ef65d185d365c68881c0c352c3b502775f6ee123/maya_scripts/02_create_blendshapes_nodes.py",
  "function_signature_candidates": []
}

```text

import maya.cmds as cmds
import numpy as np
import os
import json

with open("C:/Users/Michael/PycharmProjects/FacialRetargeting/configs/David_to_Louise_v2.json") as f:
    config = json.load(f)

# get all blendshapes' meshes
mesh_list = cmds.ls(config['maya_bs_group'], dag=1, type="mesh")  # get all blenshapes from the blenshape group folder

# remove names issue and make a list of string instead of that "maya" list [u"", u""]
mesh_list_tuple = []
for mesh in mesh_list:
    remove_letters = 5  # somehow maya adds "Shape" at the end of the mesh
    if 'ShapeOrig' in mesh:  # ... and sometimes "ShapeOrig"
        remove_letters = 9
    # create blendshape string list
    mesh_list_tuple.append(str(mesh[:-remove_letters]))

print("mesh_list_tuple")
print(mesh_list_tuple)

# create a blendshape nodes for every blendshape mesh
cmds.blendShape(mesh_list_tuple, config['maya_base_mesh_name'], name=config['blendshape_node'])

# save mesh names
np.save(os.path.join(config['python_data_path'], config['maya_bs_mesh_list']), mesh_list_tuple)


```

## Source file: src/ECEG.py

{
  "path": "src/ECEG.py",
  "blob_sha": "97ce8fab5df61c44b629a2c452f6958c63f57f93",
  "sha256": "ef9ead740e57cfead25a5e603e502c213c0403b7d8dd61cfecbea8c98491c463",
  "lines": 200,
  "bytes": 5486,
  "selection_score": 0,
  "keyword_line_counts": {
    "blendshape": 4
  },
  "url": "https://github.com/michaelStettler/FacialRetargeting/blob/ef65d185d365c68881c0c352c3b502775f6ee123/src/ECEG.py",
  "function_signature_candidates": [
    {
      "line": 15,
      "text": "def __init__(self, delta_sk):"
    },
    {
      "line": 23,
      "text": "def get_graph_laplacian(self, dsk):"
    },
    {
      "line": 50,
      "text": "def _eceg(self, dp):"
    },
    {
      "line": 68,
      "text": "def get_eCEG(self):"
    },
    {
      "line": 75,
      "text": "def get_dECEG(self):"
    }
  ]
}

```text

import numpy as np
from src.compute_corr_coef import compute_corr_coef

class ECEG:
    """
    Construct a class to compute E_CEG as in formula 13 using a function to pass directly the personalized blendshapes
    in delta space delta_p (dp)

    k:= num_of_blendshapes
    f:= num_frames
    m:= num_markers
    n:= num_features
    """

    def __init__(self, delta_sk):
        self.delta_sk = delta_sk
        self.K = np.shape(delta_sk)[0]
        self.N = np.shape(delta_sk)[1]
        self.M = int(self.N/3)

        self.L = self.get_graph_laplacian(self.delta_sk)

    def get_graph_laplacian(self, dsk):
        """
        Get the Laplacian linear decomposition af formula 12 L_op(dsk)


        k:= num_of_blendshapes
        n:= num_of_features

        :param dsk: (k, n)
        :return: L (k, n)
        """
        # compute coefficients
        ckl = compute_corr_coef(dsk, dsk)

        # compute laplacians
        L = []
        for k in range(np.shape(dsk)[0]):
            sum_coeff = 0
            sum_norm = 0
            for l in range(np.shape(dsk)[0]):
                if l != k:
                    sum_coeff += ckl[k, l] * (dsk[l] - dsk[k])
                    sum_norm += np.abs(ckl[k, l])
            L.append(sum_coeff/sum_norm)

        return np.array(L)

    def _eceg(self, dp):
        """
        Compute E_CEG as in formula 13

        :param dp: delta p (k, n)
        :return: e_mesh
        """
        if len(np.shape(dp)) < 2:
            dp = np.reshape(dp, (self.K, self.N))

        # compute w = Laplacian * diff
        w = self.L * (dp - self.delta_sk)

        # compute norm
        norm = np.linalg.norm(w, axis=1) ** 2

        return np.sum(norm) / self.M

    def get_eCEG(self):
        """
        return the function eceg
        :return:
        """
        return self._eceg

    def get_dECEG(self):
        # test if delta_sk are separable into xyz
        if self.N % 3 != 0:
            raise ValueError("Number of features ({}) is not a multiple of 3 (xyz)".format(self.N))

        # split data into xyz coordinates
        x_indices = np.arange(start=0, stop=self.N, step=3)
        y_indices = np.arange(start=1, stop=self.N, step=3)
        z_indices = np.arange(start=2, stop=self.N, step=3)

        # split L
        LX = self.L[:, x_indices]
        LY = self.L[:, y_indices]
        LZ = self.L[:, z_indices]

        # split Lsk
        LskX = self.delta_sk[:, x_indices]
        LskY = self.delta_sk[:, y_indices]
        LskZ = self.delta_sk[:, z_indices]

        # build A
        AX = (2/self.M) * np.diag(np.power(LX, 2).flatten())
        AY = (2/self.M) * np.diag(np.power(LY, 2).flatten())
        AZ = (2/self.M) * np.diag(np.power(LZ, 2).flatten())

        # build b
        bX = (2/self.M) * np.multiply(np.power(LX, 2), LskX).flatten()
        bY = (2/self.M) * np.multiply(np.power(LY, 2), LskY).flatten()
        bZ = (2/self.M) * np.multiply(np.power(LZ, 2), LskZ).flatten()

        return AX, AY, AZ, bX, bY, bZ


if __name__ == '__main__':
    """
    
    """

    np.random.seed(1)
    np.set_printoptions(precision=4, linewidth=200, suppress=True)

    # declare variables
    n_k = 4  # num_blendshapes
    n_m = 2  # num markers
    n_n = n_m * 3  # num_features (num_markers * 3)
    dsk = np.random.rand(n_k, n_n)
    dp = np.random.rand(n_k, n_n)
    print("shape dp", np.shape(dp))
    print(dp)
    print()

    # delcare ECEG
    e_CEG = ECEG(dsk)

    # compute Laplacian
    ckl = compute_corr_coef(dsk, dsk)
    L = []
    for k in range(np.shape(dsk)[0]):
        sum_coeff = 0
        sum_norm = 0
        for l in range(np.shape(dsk)[0]):
            if l != k:
                sum_coeff += ckl[k, l] * (dsk[l] - dsk[k])
                sum_norm += np.abs(ckl[k, l])
        L.append(sum_coeff/sum_norm)
    L = np.array(L)

    # compute eceg
    e_ceg = 0
    for k in range(n_k):
        ds = dsk[l] - dsk[k]
        w = L[k] * (dp[k] - dsk[k])

        norm = np.linalg.norm(w) ** 2
        e_ceg += norm

    eceg_ctrl = e_ceg / n_m
    print("eceg_ctrl = ", eceg_ctrl)

    # compute eCEG
    e_ceg_fn = e_CEG.get_eCEG()
    eceg = e_ceg_fn(dp)
    print("eceg =", eceg)

    assert np.around(eceg, 6) == np.around(eceg_ctrl, 6)
    print("emesh values are equal")
    print()

    # print("------------- test L@k decomposition --------------")
    # Lpk_test = e_CEG._compute_gl_operator(dp)
    # print("shape Lpk_test", np.shape(Lpk_test))
    # print(Lpk_test)
    # # build matrix L to form the L*dp
    # L = e_CEG.get_graph_laplacian(dp)
    # Lpk = L @ dp
    # print("shape Lpk", np.shape(Lpk))
    # print(Lpk)
    # assert Lpk.all() == Lpk_test.all()
    # print("L Decomposition works!")
    # print()

    print("----- Minimization ------")
    print("try minimize")
    import time as time
    print("try optimizer")
    from scipy import optimize
    start = time.time()
    opt = optimize.minimize(e_CEG.get_eCEG(), dsk, method="BFGS")
    print("solved in:", time.time() - start)
    print("shape opt.x", np.shape(opt.x))
    print(opt.x)

    print("try solve")
    from scipy.linalg import solve
    AX, AY, AZ, bX, bY, bZ = e_CEG.get_dECEG()
    start = time.time()
    solX = solve(AX, bX)
    solY = solve(AY, bY)
    solZ = solve(AZ, bZ)
    sol = np.vstack((solX, solY, solZ)).reshape(-1, order='F')
    print("solved in:", time.time() - start)
    print("shape sol", np.shape(sol))
    print(sol)
    print("dsk")
    print(dsk.flatten())



```
