# Facial rendering source scan: czh-98/REALY

Repository: https://github.com/czh-98/REALY

Pinned source revision: 3f13e9adcac8cbcfd4ac7db23e2c7c4f63174e8a

Discovery topic: face reconstruction 3D

Description: REALY: Rethinking the Evaluation of 3D Face Reconstruction (ECCV 2022)

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"flame": ["data/FLAME.obj", "data/FLAME.txt", "data/FLAME_detail.obj", "data/FLAME_detail.txt"]}

## Source file: main.py

{
  "path": "main.py",
  "blob_sha": "227f65d023d0c052cd25ccbf0a96be501c0e6f4f",
  "sha256": "d6fce78146e30a594995485c6d5f28086be29c76704ee896a10dfe8b696f1127",
  "lines": 309,
  "bytes": 13196,
  "selection_score": 0,
  "keyword_line_counts": {
    "mouth": 18,
    "gum": 8,
    "deform": 3,
    "rig": 6,
    "face": 8
  },
  "url": "https://github.com/czh-98/REALY/blob/3f13e9adcac8cbcfd4ac7db23e2c7c4f63174e8a/main.py",
  "function_signature_candidates": [
    {
      "line": 43,
      "text": "def get_statistic_metric(args):"
    },
    {
      "line": 102,
      "text": "def REALY_eval_all(args):"
    }
  ]
}

```text

"""
This file is part of the repo: https://github.com/czh-98/REALY
If you find the code useful, please cite our paper:
"REALY: Rethinking the Evaluation of 3D Face Reconstruction"
European Conference on Computer Vision 2022
Code: https://github.com/czh-98/REALY
Copyright (c) [2021-2022] [Tencent AI Lab]
Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:
The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.
THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
"""

import numpy as np
import os
from glob import glob
from utils.io_obj import read, write
import trimesh
from utils.util import keypoints_region_map
from utils.util import get_barycentric_coordinates
import argparse
from tqdm import tqdm
from utils.rICP import region_icp_all as rICP_all
from utils.gICP import global_rigid_align_7_kpt as gicp
from utils.eval import bidirectional_evaluation_pipeline as bi_eval

import warnings

warnings.filterwarnings("ignore")


def get_statistic_metric(args):
    REALY_error_save_path = os.path.join(args.save, "REALY_error.txt")

    error_at_nose_list = []
    error_at_mouth_list = []
    error_at_forehead_list = []
    error_at_cheek_list = []

    with open(REALY_error_save_path, "r") as f:
        for line in f:
            subject_id, _, error_at_nose, error_at_mouth, error_at_forehead, error_at_cheek = line.replace(
                "\n", ""
            ).split("\t")
            error_at_nose_list.append(float(error_at_nose))
            error_at_mouth_list.append(float(error_at_mouth))
            error_at_forehead_list.append(float(error_at_forehead))
            error_at_cheek_list.append(float(error_at_cheek))

    error_at_nose_list = np.array(error_at_nose_list)
    error_at_mouth_list = np.array(error_at_mouth_list)
    error_at_forehead_list = np.array(error_at_forehead_list)
    error_at_cheek_list = np.array(error_at_cheek_list)

    # average
    print("average nmse@nose = %.3f" % error_at_nose_list.mean())
    print("average nmse@mouth = %.3f" % error_at_mouth_list.mean())
    print("average nmse@forehead = %.3f" % error_at_forehead_list.mean())
    print("average nmse@cheek = %.3f" % error_at_cheek_list.mean())

    # median
    print("median nmse@nose = %.3f" % np.median(error_at_nose_list))
    print("median nmse@mouth = %.3f" % np.median(error_at_mouth_list))
    print("median nmse@forehead = %.3f" % np.median(error_at_forehead_list))
    print("median nmse@cheek = %.3f" % np.median(error_at_cheek_list))

    # std
    std = lambda data: data.std() * np.sqrt(data.shape[0]) / np.sqrt(data.shape[0] - 1)
    print("standard deviation nmse@nose = %.3f" % std(error_at_nose_list))
    print("standard deviation nmse@mouth = %.3f" % std(error_at_mouth_list))
    print("standard deviation nmse@forehead = %.3f" % std(error_at_forehead_list))
    print("standard deviation nmse@cheek = %.3f" % std(error_at_cheek_list))

    # all
    print(
        "average nmse@all = %.3f"
        % (
            (
                error_at_nose_list.mean()
                + error_at_mouth_list.mean()
                + error_at_forehead_list.mean()
                + error_at_cheek_list.mean()
            )
            / 4.0
        )
    )

    return


def REALY_eval_all(args):
    REALY_keypoints_root = args.REALY_HIFI3D_keypoints
    REALY_scan_region_root = args.REALY_scan_region
    prediction_mesh_root = args.prediction
    template_topology = args.template_topology
    template_vertices_mask = args.template_mask

    aligned_predicted_mesh_save_root = os.path.join(args.save, "region_align_save")
    os.makedirs(aligned_predicted_mesh_save_root, exist_ok=True)

    REALY_error_save_path = os.path.join(args.save, "REALY_error.txt")

    metrical_scale = np.loadtxt(args.scale_path)

    if template_vertices_mask is not None:
        pred_mask_face = np.loadtxt(template_vertices_mask, delimiter=",", dtype=np.int32)
        pred_mask_face = pred_mask_face.reshape(-1, 1)
    else:
        pred_mask_face = None

    predicted_mesh_subjects = glob(os.path.join(prediction_mesh_root, "*.obj"))
    predicted_mesh_subjects.sort(key=lambda x: int(os.path.split(x)[1].replace("_", "").replace(".obj", "")))

    with tqdm(total=len(predicted_mesh_subjects)) as pbar:
        with open(REALY_error_save_path, "w") as f_w:
            for subject_path in predicted_mesh_subjects:
                pbar.update(1)
                _, subject = os.path.split(subject_path)
                subject = subject.replace(".obj", "")
                subject_id = int(subject.split("_")[0])

                # read 3D meshes for evaluation
                REALY_keypoints_path = os.path.join(REALY_keypoints_root, str(subject_id) + ".obj")
                predicted_mesh_path = os.path.join(prediction_mesh_root, subject + ".obj")
                predicted_mesh = read(predicted_mesh_path)
                REALY_HIFI3D_keypoints = read(REALY_keypoints_path)["v"]

                REALY_scan_region_path = os.path.join(REALY_scan_region_root, str(subject_id))
                REALY_scan_nose_mesh = read(os.path.join(REALY_scan_region_path, "nose.obj"))
                REALY_scan_mouth_mesh = read(os.path.join(REALY_scan_region_path, "mouth.obj"))
                REALY_scan_forehead_mesh = read(os.path.join(REALY_scan_region_path, "forehead.obj"))
                REALY_scan_cheek_mesh = read(os.path.join(REALY_scan_region_path, "cheek.obj"))

                REALY_scan_nose_vertices = REALY_scan_nose_mesh["v"]
                REALY_scan_mouth_vertices = REALY_scan_mouth_mesh["v"]
                REALY_scan_forehead_vertices = REALY_scan_forehead_mesh["v"]
                REALY_scan_cheek_vertices = REALY_scan_cheek_mesh["v"]

                REALY_scan_nose_triangles = REALY_scan_nose_mesh["fv"]
                REALY_scan_mouth_triangles = REALY_scan_mouth_mesh["fv"]
                REALY_scan_forehead_triangles = REALY_scan_forehead_mesh["fv"]
                REALY_scan_cheek_triangles = REALY_scan_cheek_mesh["fv"]

                # Step1: gICP, global-align the prediction mesh to the ground-truth scan
                # We use 7 keypoint for global alignment for efficient
                # Here is used to save the globally rigid aligned mesh
                predicted_vertices_global_aligned = gicp(
                    predicted_mesh["v"], REALY_HIFI3D_keypoints, template_topology=template_topology
                )

                # Step2: rICP, regional align the prediction mesh to the ground-truth scan region
                REALY_scan_region_vertices_dict = {
                    "nose": REALY_scan_nose_vertices,
                    "mouth": REALY_scan_mouth_vertices,
                    "forehead": REALY_scan_forehead_vertices,
                    "cheek": REALY_scan_cheek_vertices,
                }

                REALY_scan_region_triangles_dict = {
                    "nose": REALY_scan_nose_triangles,
                    "mouth": REALY_scan_mouth_triangles,
                    "forehead": REALY_scan_forehead_triangles,
                    "cheek": REALY_scan_cheek_triangles,
                }

                region_list = ["nose", "mouth", "forehead", "cheek"]

                # for each region, regional aligned
                regional_aligned_vertices_dict, regional_aligned_triangles_dict = rICP_all(
                    predicted_mesh=predicted_mesh,
                    REALY_scan_region_dict=REALY_scan_region_vertices_dict,
                    REALY_HIFI3D_keypoints=REALY_HIFI3D_keypoints,
                    template_topology=template_topology,
                    max_iteration=100,
                    pred_mask_face=pred_mask_face,
                )

                align_save_path = os.path.join(aligned_predicted_mesh_save_root, subject)

                os.makedirs(align_save_path, exist_ok=True)

                write(
                    os.path.join(align_save_path, "at_global.obj"),
                    predicted_vertices_global_aligned,
                    regional_aligned_triangles_dict["forehead"],
                )

                write(
                    os.path.join(align_save_path, "at_nose.obj"),
                    regional_aligned_vertices_dict["nose"],
                    regional_aligned_triangles_dict["nose"],
                )
                write(
                    os.path.join(align_save_path, "at_mouth.obj"),
                    regional_aligned_vertices_dict["mouth"],
                    regional_aligned_triangles_dict["mouth"],
                )
                write(
                    os.path.join(align_save_path, "at_forehead.obj"),
                    regional_aligned_vertices_dict["forehead"],
                    regional_aligned_triangles_dict["forehead"],
                )
                write(
                    os.path.join(align_save_path, "at_cheek.obj"),
                    regional_aligned_vertices_dict["cheek"],
                    regional_aligned_triangles_dict["cheek"],
                )

                keypoints_map = {
                    "forehead": keypoints_region_map["forehead"],
                    "cheek": None,
                    "nose": keypoints_region_map["nose"],
                    "mouth": keypoints_region_map["mouth"],
                }

                f_w.write(subject)
                f_w.write("\t")

                # bICP and calculate errors for each region
                for region in region_list:
                    predicted_keypoints = get_barycentric_coordinates(
                        regional_aligned_vertices_dict[region], template_topology
                    )
                    predicted_keypoints_region = predicted_keypoints[keypoints_map[region]]
                    REALY_HIFI3D_keypoints_region = REALY_HIFI3D_keypoints[keypoints_map[region]]

                    regional_aligned_pd = trimesh.Trimesh(
                        vertices=regional_aligned_vertices_dict[region],
                        faces=regional_aligned_triangles_dict[region] - 1,
                        process=False,
                    )
                    ground_truth_region = trimesh.Trimesh(
                        vertices=REALY_scan_region_vertices_dict[region],
                        faces=REALY_scan_region_triangles_dict[region] - 1,
                        process=True,
                    )

                    error, deformation, error_map = bi_eval(
                        regional_aligned_pd,
                        ground_truth_region,
                        predicted_keypoints_region,
                        REALY_HIFI3D_keypoints_region,
                        region,
                        visualize_error_map=True,
                    )

                    f_w.write("\t")
                    f_w.write(str(error * metrical_scale[subject_id - 1])[:9])  # id start from 1
                    deformation.export(os.path.join(align_save_path, "deform_%s.obj" % region))
                    error_map.export(os.path.join(align_save_path, "error_%s.obj" % region))

                f_w.write("\n")
                f_w.flush()


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--REALY_HIFI3D_keypoints",
        help="PATH to the ground-truth HIFI3D mesh",
        default="./data/REALY_HIFI3D_keypoints",
        type=str,
    )
    parser.add_argument(
        "--REALY_scan_region",
        help="PATH to the ground-truth scan region",
        default="./data/REALY_scan_region",
        type=str,
    )

    parser.add_argument(
        "--prediction",
        help="PATH to the predicted mesh",
        type=str,
        required=True,
    )

    parser.add_argument(
        "--template_mask",
        help='PATH to the face mask of template vertices, save as ".txt" format, values: 0 or 1',
        default=None,
    )
    parser.add_argument("--template_topology", help="template topology", default="HIFI3D++", type=str)

    parser.add_argument(
        "--scale_path", help="PATH to the metrical scale file", default="./data/metrical_scale.txt", type=str
    )
    parser.add_argument(
        "--save",
        help="PATH to save the aligned meshes, deformation meshes, error map, and NMSE results",
        type=str,
        required=True,
    )

    args = parser.parse_args()

    REALY_eval_all(args)
    get_statistic_metric(args)


```

## Source file: utils/NICP.py

{
  "path": "utils/NICP.py",
  "blob_sha": "c0458e65019c6e65798ce135f1342e4e285eb1cc",
  "sha256": "faf1eec69d024c49573976d8d72ef09fcd14ff1809d00bdac7b346efbe0ad826",
  "lines": 287,
  "bytes": 10694,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 4,
    "face": 1
  },
  "url": "https://github.com/czh-98/REALY/blob/3f13e9adcac8cbcfd4ac7db23e2c7c4f63174e8a/utils/NICP.py",
  "function_signature_candidates": [
    {
      "line": 32,
      "text": "def spsolve(sparse_A, dense_b):"
    },
    {
      "line": 37,
      "text": "def triangles_to_edge_vertex_adjacent_matrix(triangles):  # starting from 1"
    },
    {
      "line": 74,
      "text": "def sparse_matrix_from_vertices(cur_src):"
    },
    {
      "line": 92,
      "text": "def __init__(self, ver_dst, n=100):  # target vertices"
    },
    {
      "line": 95,
      "text": "def find_nearest_neighbors(self, ver_src):"
    },
    {
      "line": 105,
      "text": "def __init__(self, ver_src, ver_dst, tri_src, epsilon, gamma, alpha, beta):"
    },
    {
      "line": 130,
      "text": "def apply(self):"
    },
    {
      "line": 176,
      "text": "def __init__("
    },
    {
      "line": 211,
      "text": "def apply(self):"
    }
  ]
}

```text

"""
This file is part of the repo: https://github.com/czh-98/REALY
If you find the code useful, please cite our paper:
"REALY: Rethinking the Evaluation of 3D Face Reconstruction"
European Conference on Computer Vision 2022
Code: https://github.com/czh-98/REALY
Copyright (c) [2021-2022] [Tencent AI Lab]
Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:
The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.
THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
"""

import numpy as np
import scipy.sparse
import scipy.sparse.linalg
import scipy.spatial
from sksparse.cholmod import cholesky_AAt


def spsolve(sparse_A, dense_b):
    factor = cholesky_AAt(sparse_A.T)
    return factor(sparse_A.T.dot(dense_b)).toarray()


def triangles_to_edge_vertex_adjacent_matrix(triangles):  # starting from 1
    v1, v2, v3 = np.split(triangles, 3, axis=0)
    p_list = []
    tri_t = triangles.T
    for v1, v2, v3 in tri_t:
        if v1 < v2:
            p_list.append([v1, v2])
        else:
            p_list.append([v2, v1])

        if v1 < v3:
            p_list.append([v1, v3])
        else:
            p_list.append([v3, v1])

        if v2 < v3:
            p_list.append([v2, v3])
        else:
            p_list.append([v3, v2])

    p = np.stack(p_list, axis=1)

    pair_list = np.split(p, p.shape[1], axis=1)
    pair_tuple_list = [(int(elem[0]), int(elem[1])) for elem in pair_list]
    pair_set = set(pair_tuple_list)
    pair_array = np.array(list(pair_set))
    edge_numbers = np.arange(len(pair_array))

    index_i = np.concatenate([edge_numbers] * 2, axis=0)
    index_j = np.concatenate([pair_array[:, 0], pair_array[:, 1]], axis=0)
    data = np.ones([len(pair_set)])
    data = np.concatenate([data, -data], axis=0)

    ev_adj = scipy.sparse.csc_matrix((data, (index_i, index_j)), shape=[len(edge_numbers), np.amax(triangles) + 1])
    return ev_adj


def sparse_matrix_from_vertices(cur_src):
    # cur_src: 3xN
    one_row = np.ones([1, cur_src.shape[1]], np.float32)
    cur_src_ = np.concatenate([cur_src, one_row], axis=0)

    Di = np.arange(cur_src.shape[1])
    Dj = np.stack([Di * 4, Di * 4 + 1, Di * 4 + 2, Di * 4 + 3], axis=0)
    Dj = np.reshape(Dj, [-1])
    Di = np.stack([Di] * 4, axis=0)
    Di = np.reshape(Di, [-1])
    cur_src_vector = np.reshape(cur_src_, [-1])
    D = scipy.sparse.csc_matrix(
        (cur_src_vector, (Di, Dj)), shape=[cur_src.shape[1], cur_src.shape[1] * 4], dtype=np.float32
    )
    return D


class NNSearch(object):
    def __init__(self, ver_dst, n=100):  # target vertices
        self.kd_tree = scipy.spatial.KDTree(ver_dst.T, n)

    def find_nearest_neighbors(self, ver_src):
        # kd-tree to find NN
        N_src = ver_src.shape[1]
        nn_distances, nn_indices = self.kd_tree.query(ver_src.T, 1, p=2)
        nn_indices = np.array(nn_indices, np.int32)
        nn_distances = np.array(nn_distances, np.float32)
        return nn_indices, nn_distances


class nICP_without_keypoints(object):
    def __init__(self, ver_src, ver_dst, tri_src, epsilon, gamma, alpha, beta):
        self.ver_src = ver_src  # 3 x N
        self.ver_dst = ver_dst  # 3 x N
        self.tri_src = tri_src  # 3 x NT
        self.epsilon = epsilon  # optimization ending threshold
        self.gamma = float(gamma)  # stiffness loss weight for translation
        self.alpha = float(alpha)  # exponential decay for each step
        self.beta = float(beta)  # landmark loss weight

        # G: weighting for balancing rotation and translation in stiffness loss
        self.G = np.diag(np.array([1.0, 1.0, 1.0, self.gamma], np.float32))

        # homogeneous ver_src
        # M: build a node-arc incidence matrix for M
        self.M = triangles_to_edge_vertex_adjacent_matrix(tri_src)  # E x N

        # A: large matrix for complete cost function
        self.A1 = scipy.sparse.kron(self.M, self.G)  # 4E x 4N

        # B: large matrix for complete cost function
        self.B1 = np.zeros([self.A1.shape[0], 3], np.float32)
        self.B1 = scipy.sparse.csc_matrix(self.B1)

        self.nn_search = NNSearch(ver_dst)

    def apply(self):
        cur_src = self.ver_src
        cur_X = np.zeros([cur_src.shape[1] * 4, 3])
        X = np.ones([cur_src.shape[1] * 4, 3])  # initialization

        for decay in range(3):
            cur_alpha = self.alpha * np.exp(-0.5 * decay)
            epsilon = self.epsilon * pow(0.5, decay)
            step = 0
            while True:
                delta = np.linalg.norm(X - cur_X)
                if delta <= epsilon:
                    break

                cur_X = X.copy()

                # find nn
                nn_indices, nn_distances = self.nn_search.find_nearest_neighbors(cur_src)
                cur_nn_ver_dst = self.ver_dst[:, nn_indices]

                cur_D = sparse_matrix_from_vertices(cur_src)  # N x 4N
                threshold = max(np.mean(nn_distances) * 2, 1)

                cur_weight = (nn_distances < threshold).astype(np.float32)

                A2 = cur_D.multiply(cur_weight[:, np.newaxis])
                B2 = cur_nn_ver_dst.T
                B2 = np.multiply(B2, cur_weight[:, np.newaxis])

                # convert to sparse matrix
                A2 = scipy.sparse.csc_matrix(A2)
                B2 = scipy.sparse.csc_matrix(B2)

                A = scipy.sparse.vstack([self.A1.multiply(cur_alpha), A2])
                B = scipy.sparse.vstack([self.B1, B2])

                X = spsolve(A, B)

                cur_src = cur_D * X
                cur_src = cur_src.transpose()
                step += 1

        self.ver_src = cur_src


class nICP_with_keypoints(object):
    def __init__(
        self, ver_src, ver_dst, tri_src, epsilon, gamma, alpha, beta, src_pts_KP_idx, tgt_pts_KP, kp_w1, kp_w2, alpha2
    ):
        self.ver_src = ver_src  # 3 x N
        self.ver_dst = ver_dst  # 3 x N
        self.tri_src = tri_src  # 3 x NT
        self.epsilon = epsilon  # optimization ending threshold
        self.gamma = float(gamma)  # stiffness loss weight for translation
        self.alpha = float(alpha)  # exponential decay for each step
        self.beta = float(beta)  # landmark loss weight

        self.src_pts_KP_idx = np.array(src_pts_KP_idx)
        self.tgt_pts_KP = tgt_pts_KP
        self.kp_w1 = float(kp_w1)
        self.kp_w2 = float(kp_w2)
        self.alpha2 = float(alpha2)

        # G: weighting for balancing rotation and translation in stiffness loss
        self.G = np.diag(np.array([1.0, 1.0, 1.0, self.gamma], np.float32))
        self.G_KP = np.diag(np.array([2.0, 2.0, 3.0, self.gamma * 2], np.float32))

        # homogeneous ver_src
        # M: build a node-arc incidence matrix for M
        self.M = triangles_to_edge_vertex_adjacent_matrix(tri_src)  # E x N

        # A: large matrix for complete cost function
        self.A1 = scipy.sparse.kron(self.M, self.G)  # 4E x 4N
        self.A1_KP = scipy.sparse.kron(self.M, self.G_KP)  # 4E x 4N

        # B: large matrix for complete cost function
        self.B1 = np.zeros([self.A1.shape[0], 3], np.float32)
        self.B1 = scipy.sparse.csc_matrix(self.B1)

        self.nn_search = NNSearch(ver_dst)

    def apply(self):

        cur_src = self.ver_src
        cur_X = np.zeros([cur_src.shape[1] * 4, 3])
        X = np.ones([cur_src.shape[1] * 4, 3])  # initialization

        count = 0
        for decay in range(4):
            if decay == 0:
                cur_alpha = self.alpha2
            else:
                cur_alpha = self.alpha * np.exp(-0.5 * (decay - 1))
            epsilon = self.epsilon * pow(0.5, (decay - 1))

            step = 0
            while True:
                delta = np.linalg.norm(X - cur_X)

                if (delta <= epsilon) or (step > 10):
                    break

                cur_X = X.copy()

                # find nn
                nn_indices, nn_distances = self.nn_search.find_nearest_neighbors(cur_src)
                cur_nn_ver_dst = self.ver_dst[:, nn_indices]

                cur_D = sparse_matrix_from_vertices(cur_src)  # N x 4N
                threshold = max(np.mean(nn_distances) * 2, 1)
                cur_weight = (nn_distances < threshold).astype(np.float32)

                A2 = cur_D.multiply(cur_weight[:, np.newaxis])
                B2 = cur_nn_ver_dst.T
                B2 = np.multiply(B2, cur_weight[:, np.newaxis])

                # convert to sparse matrix
                A2 = scipy.sparse.csc_matrix(A2)
                B2 = scipy.sparse.csc_matrix(B2)

                kp_weight = np.zeros([cur_D.shape[0], 1])
                if decay == 0:
                    kp_weight[self.src_pts_KP_idx, :] = self.kp_w1
                else:
                    kp_weight[self.src_pts_KP_idx, :] = self.kp_w1 * np.exp(-0.9 * count) / 10.0

                cur_D_KP = cur_D.copy()  # N x 4N
                A3 = cur_D_KP.multiply(kp_weight)

                tgt_pts_KP_full = cur_nn_ver_dst.copy()
                tgt_pts_KP_full[:, self.src_pts_KP_idx] = self.tgt_pts_KP

                B3 = tgt_pts_KP_full.T
                B3 = np.multiply(B3, kp_weight)

                # convert to sparse matrix
                A3 = scipy.sparse.csc_matrix(A3)
                B3 = scipy.sparse.csc_matrix(B3)

                if decay == 0:
                    A = scipy.sparse.vstack([self.A1_KP.multiply(cur_alpha), A3])
                    B = scipy.sparse.vstack([self.B1, B3])
                else:
                    A = scipy.sparse.vstack([self.A1.multiply(cur_alpha), A2, A3])
                    B = scipy.sparse.vstack([self.B1, B2, B3])

                X = spsolve(A, B)

                cur_src = cur_D * X
                cur_src = cur_src.transpose()

                step += 1
                count += 1

                if decay == 0:
                    break

        self.ver_src = cur_src


```

## Source file: utils/eval.py

{
  "path": "utils/eval.py",
  "blob_sha": "1fd0c2accffaaa29f2b799f03bdf35357350f05a",
  "sha256": "932918a414b6a57ab1cf69e92fd257f74846124f8e2d188d75bc355e7bde8736",
  "lines": 137,
  "bytes": 5655,
  "selection_score": 0,
  "keyword_line_counts": {
    "mouth": 1,
    "deform": 9,
    "rig": 7,
    "face": 7
  },
  "url": "https://github.com/czh-98/REALY/blob/3f13e9adcac8cbcfd4ac7db23e2c7c4f63174e8a/utils/eval.py",
  "function_signature_candidates": [
    {
      "line": 32,
      "text": "def bidirectional_evaluation_pipeline("
    }
  ]
}

```text

"""
This file is part of the repo: https://github.com/czh-98/REALY
If you find the code useful, please cite our paper:
"REALY: Rethinking the Evaluation of 3D Face Reconstruction"
European Conference on Computer Vision 2022
Code: https://github.com/czh-98/REALY
Copyright (c) [2021-2022] [Tencent AI Lab]
Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:
The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.
THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
"""

import numpy as np
import trimesh
from utils.NICP import nICP_with_keypoints
from utils.NICP import nICP_without_keypoints
import cv2


def bidirectional_evaluation_pipeline(
    predicted_mesh,
    REALY_scan_region_mesh,
    predicted_keypoints_region,
    REALY_HIFI3D_keypoints_region,
    region="mouth",
    visualize_error_map=True,
):
    """

    Args:
        predicted_mesh: vertices from the regional aligned predicted mesh, shape: nx3
        REALY_scan_region_mesh: mesh from the REALY scan of specific region, <V,F>, Trimesh
        predicted_keypoints_region: regional keypoints from the predicted mesh specific region, shape: n1x3
        REALY_HIFI3D_keypoints_region: regional keypoints from the REALY HIFI3D mesh of specific region, shape: n1x3
        region: selected face region for alignment, str
        visualize_error_map: whether return the error map on ground-truth scan regions, bool: True

    Returns: the reconstruction error NMSE of the predicted mesh@region and error map scan

    """

    # init hyper-parameters on non-rigid alignment
    epsilon = 1.0
    gamma = 1
    alpha = 50
    beta = 0.5

    # 2nd-stage
    alpha2 = 50 * 3
    kp_w1 = 50.0  # key point weight-forhead
    kp_w2 = 50.0  # key point weight-others

    # Step1: find the nn-map of the ground truth vertices on the regional-aligned predicted mesh
    # to ensure the correspondence share the same vertex number as the ground-truth

    REALY_scan_region_vertices = REALY_scan_region_mesh.vertices
    REALY_scan_region_triangles = REALY_scan_region_mesh.faces
    correspondence_on_prediction, _, _ = predicted_mesh.nearest.on_surface(REALY_scan_region_vertices)

    # use REALY HIFI3D keypoints to find the keypoint index on REALY scan
    _, REALY_scan_keypoints_index = REALY_scan_region_mesh.nearest.vertex(REALY_HIFI3D_keypoints_region)

    # Step2: non-rigid alignment that deform the REALY scan region into the predicted mesh
    # through the coarse correspondence built from the nn-map in Step1
    if region == "cheek":
        deformation_tool = nICP_without_keypoints(
            REALY_scan_region_vertices.T,
            correspondence_on_prediction.T,
            REALY_scan_region_triangles.T,
            epsilon,
            gamma,
            alpha,
            beta,
        )
    else:
        deformation_tool = nICP_with_keypoints(
            REALY_scan_region_vertices.T,
            correspondence_on_prediction.T,
            REALY_scan_region_triangles.T,
            epsilon,
            gamma,
            alpha,
            beta,
            REALY_scan_keypoints_index,
            predicted_keypoints_region.T,
            kp_w1,
            kp_w2,
            alpha2,
        )

    deformation_tool.apply()

    # Step3: update the correspondence between the ground truth scan and the predicted mesh
    REALY_scan_deformed = trimesh.Trimesh(
        vertices=deformation_tool.ver_src.T, faces=REALY_scan_region_triangles, process=False
    )

    correspondence_on_prediction_updated, _, _ = predicted_mesh.nearest.on_surface(REALY_scan_deformed.vertices)

    # Step4: calculate the NMSE error
    # between the regional aligned predicted mesh and the original ground truth scan region
    normalized_mean_square_error = np.sqrt(
        np.sum((correspondence_on_prediction_updated - REALY_scan_region_vertices) ** 2, axis=-1)
    ).mean()

    if visualize_error_map is True:
        mean_square_error_on_vertices = np.sqrt(
            np.sum((correspondence_on_prediction_updated - REALY_scan_region_vertices) ** 2, axis=-1)
        )
        mean_square_error_on_vertices = (mean_square_error_on_vertices - 0) / 0.5
        mean_square_error_on_vertices[mean_square_error_on_vertices > 1] = 1.0

        rgb_values_map = cv2.applyColorMap(np.arange(256, dtype=np.uint8), cv2.COLORMAP_JET).reshape(256, 3)[::-1]
        vertices_colors = rgb_values_map[(mean_square_error_on_vertices * 255).astype(np.int32)]

        rgb_gt_region = trimesh.Trimesh(
            vertices=REALY_scan_region_vertices,
            faces=REALY_scan_region_triangles,
            vertex_colors=vertices_colors,
            process=False,
        )

        return normalized_mean_square_error, REALY_scan_deformed, rgb_gt_region

    return normalized_mean_square_error, REALY_scan_deformed


```

## Source file: utils/gICP.py

{
  "path": "utils/gICP.py",
  "blob_sha": "0633d18d4f2c16e4ef0d352eae25f6adbbecb57f",
  "sha256": "9f7e1673759d4f06d5908957fea4be6b65537aa008b2f2eb85cc2ad009178b8c",
  "lines": 51,
  "bytes": 2302,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 7,
    "face": 1
  },
  "url": "https://github.com/czh-98/REALY/blob/3f13e9adcac8cbcfd4ac7db23e2c7c4f63174e8a/utils/gICP.py",
  "function_signature_candidates": [
    {
      "line": 31,
      "text": "def global_rigid_align_7_kpt(predicted_vertices=None, REALY_HIFI3D_keypoints=None, template_topology=\"HIFI3D\"):"
    }
  ]
}

```text

"""
This file is part of the repo: https://github.com/czh-98/REALY
If you find the code useful, please cite our paper:
"REALY: Rethinking the Evaluation of 3D Face Reconstruction"
European Conference on Computer Vision 2022
Code: https://github.com/czh-98/REALY
Copyright (c) [2021-2022] [Tencent AI Lab]
Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:
The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.
THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
"""

import numpy as np

from utils.util import keypoints_region_map
from utils.util import get_barycentric_coordinates, fit_icp_RT


def global_rigid_align_7_kpt(predicted_vertices=None, REALY_HIFI3D_keypoints=None, template_topology="HIFI3D"):
    """

    Args:
        predicted_vertices: vertices from the predicted mesh, shape: nx3
        REALY_HIFI3D_keypoints: keypoints from the REALY HIFI3D mesh, shape: 85x3
        template_topology: topology of the predicted mesh, default: HIFI3D

    Returns: gICP aligned vertices of the predicted mesh

    """
    assert REALY_HIFI3D_keypoints.shape[0] == 85

    keypoints = keypoints_region_map["seven_keypoints"]
    predicted_keypoints = get_barycentric_coordinates(predicted_vertices, template_topology)[keypoints]

    trans, scale, R, t = fit_icp_RT(predicted_keypoints.T, REALY_HIFI3D_keypoints[keypoints].T, with_scale=True)

    rigid_align = np.matmul(predicted_vertices * scale, R.T) + t

    return rigid_align


```
