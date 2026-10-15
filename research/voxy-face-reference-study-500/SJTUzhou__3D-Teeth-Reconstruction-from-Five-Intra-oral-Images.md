# Facial rendering source scan: SJTUzhou/3D-Teeth-Reconstruction-from-Five-Intra-oral-Images

Repository: https://github.com/SJTUzhou/3D-Teeth-Reconstruction-from-Five-Intra-oral-Images

Pinned source revision: 4754279119ae871579b31877b2b24a308c73fc6d

Discovery topic: teeth 3D

Description: Smartee project: 3D Teeth Reconstruction from Orthodontic Photos

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"teeth": ["demo/assets/teeth_reconstruction_framework.png", "seg/weights/weights-teeth-boundary-model.h5"], "tooth": ["data/tooth-pcd-txt/11/0U.txt", "data/tooth-pcd-txt/11/100U.txt", "data/tooth-pcd-txt/11/101U.txt", "data/tooth-pcd-txt/11/102U.txt", "data/tooth-pcd-txt/11/103U.txt", "data/tooth-pcd-txt/11/105U.txt", "data/tooth-pcd-txt/11/106U.txt", "data/tooth-pcd-txt/11/107U.txt", "data/tooth-pcd-txt/11/108U.txt", "data/tooth-pcd-txt/11/109U.txt", "data/tooth-pcd-txt/11/10U.txt", "data/tooth-pcd-txt/11/110U.txt", "data/tooth-pcd-txt/11/111U.txt", "data/tooth-pcd-txt/11/113U.txt", "data/tooth-pcd-txt/11/114U.txt", "data/tooth-pcd-txt/11/115U.txt", "data/tooth-pcd-txt/11/116U.txt", "data/tooth-pcd-txt/11/117U.txt", "data/tooth-pcd-txt/11/119U.txt", "data/tooth-pcd-txt/11/11U.txt", "data/tooth-pcd-txt/11/120U.txt", "data/tooth-pcd-txt/11/121U.txt", "data/tooth-pcd-txt/11/122U.txt", "data/tooth-pcd-txt/11/123U.txt", "data/tooth-pcd-txt/11/124U.txt", "data/tooth-pcd-txt/11/125U.txt", "data/tooth-pcd-txt/11/126U.txt", "data/tooth-pcd-txt/11/127U.txt", "data/tooth-pcd-txt/11/128U.txt", "data/tooth-pcd-txt/11/129U.txt"], "rig": ["demo/visualization/mesh-tag=0-PHOTO.RIGHT.png", "demo/visualization/mesh-tag=1-PHOTO.RIGHT.png", "demo/visualization/overlay-tag=0-PHOTO.RIGHT.png", "demo/visualization/overlay-tag=1-PHOTO.RIGHT.png", "gp_non_rigid_registration.py"]}

## Source file: gp_non_rigid_registration.py

{
  "path": "gp_non_rigid_registration.py",
  "blob_sha": "1f3b36e3ee30f3ade63ad08fc2a965a3324248cd",
  "sha256": "f844bf5611d1e77fe88ad16b206f82ab24a8aea530a01b2bb1b0883626687a3e",
  "lines": 88,
  "bytes": 3446,
  "selection_score": 15,
  "keyword_line_counts": {
    "deform": 5,
    "rig": 1
  },
  "url": "https://github.com/SJTUzhou/3D-Teeth-Reconstruction-from-Five-Intra-oral-Images/blob/4754279119ae871579b31877b2b24a308c73fc6d/gp_non_rigid_registration.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def __init__(self, s, sigma, srcX, targetY=None, n=21) -> None:"
    },
    {
      "line": 27,
      "text": "def compute_EigVals_EigFuncs(self):"
    },
    {
      "line": 49,
      "text": "def setTargetPcl(self, targetY):"
    },
    {
      "line": 53,
      "text": "def chamferDistance(X1, X2):  # 倒角距离 量纲[m^2]"
    },
    {
      "line": 62,
      "text": "def loss(self, alpha, eta):"
    },
    {
      "line": 72,
      "text": "def register(self, eta):"
    }
  ]
}

```text

import numpy as np
import scipy
import scipy.optimize as opt

# from cyipopt import minimize_ipopt


class GP_Non_Rigid_Registration(object):
    def __init__(self, s, sigma, srcX, targetY=None, n=21) -> None:
        # use gaussian kernel g(x',x) = s * exp(-||x'-x||^2 / sigma^2)
        self.s = s
        self.sigma = sigma
        assert srcX.ndim == 2, "srcX.ndim != 2"
        self.X = srcX  # source pointcloud
        self.N = self.X.shape[0]
        self.d = self.X.shape[1]
        self.Y = targetY  # target pointcloud
        # self.GP_Mu = np.zeros((self.N*self.d,))

        # 待计算
        self.n = n
        self.lmbda_n = None  # the first n eigen values
        self.phi_n = None  # the first n eigen vectors (approx. of eigen functions)
        self.alpha = None
        self.X_deformed = None

    def compute_EigVals_EigFuncs(self):
        # compute GP K
        distMat = scipy.spatial.distance_matrix(
            self.X, self.X, p=2, threshold=int(1e8)
        )  # If M * N * K > threshold, algorithm uses a Python loop instead of large temporary arrays.
        GP_K = self.s * np.kron(
            np.exp(-(distMat**2) / self.sigma**2), np.identity(self.d)
        )
        # Nyström Method 计算矩阵GP_K的特征值，特征向量用于近似GP_K的特征值和特征函数 low-rank approx. of GP
        eigVal, u = scipy.linalg.eigh(GP_K)  # np.linalg.eig(self.GP_K)
        eigOrder = sorted(
            range(len(eigVal)), key=lambda x: eigVal[x], reverse=True
        )  # eigVal从大到小的索引排序
        eigVal[eigVal < 1e-4] = 0.0
        self.lmbda_n = eigVal[eigOrder[: self.n]]
        self.phi_n = u[:, eigOrder[: self.n]]  # * np.sqrt(self.N)
        print(
            "GP low-rank approx. cumulative variance: {:.4f}".format(
                np.sum(self.lmbda_n) / np.sum(eigVal)
            )
        )

    def setTargetPcl(self, targetY):
        self.Y = targetY

    @staticmethod
    def chamferDistance(X1, X2):  # 倒角距离 量纲[m^2]
        squaredDistMat = (
            scipy.spatial.distance_matrix(X1, X2, p=2, threshold=int(1e8)) ** 2
        )
        return (
            np.min(squaredDistMat, axis=0).mean()
            + np.min(squaredDistMat, axis=1).mean()
        )

    def loss(self, alpha, eta):
        normalization = eta * np.sum(alpha**2)
        GP = np.sum(
            alpha * self.lmbda_n * self.phi_n, axis=1
        )  # + self.GP_Mu # use GP to model deformation, ignoring GP_Mu = [0]
        X_deformed = self.X + GP.reshape(self.N, self.d)
        chamfer_dist = self.chamferDistance(X_deformed, self.Y)
        # print("{:.4f}".format(chamfer_dist))
        return chamfer_dist + normalization

    def register(self, eta):
        alpha = np.zeros((self.n,))
        # ret = opt.minimize(self.loss, alpha, args=(eta,), method="Nelder-Mead", options={"fatol":1e-3,"maxiter":100})
        # ret = minimize_ipopt(self.loss, alpha, args=(eta,), options={"maxiter":20,"print_info_string":"yes"})
        ret = opt.minimize(
            self.loss,
            alpha,
            args=(eta,),
            jac="2-point",
            method="SLSQP",
            options={"ftol": 1e-6, "maxiter": 30, "disp": False},
        )
        self.alpha = ret.x
        GP = np.sum(
            self.alpha * self.lmbda_n * self.phi_n, axis=1
        )  # + self.GP_Mu  # ignoring GP_Mu = [0]
        self.X_deformed = self.X + np.reshape(GP, self.X.shape)


```

## Source file: main.py

{
  "path": "main.py",
  "blob_sha": "c87757f7f33607853841df53a7507feabcc06f4e",
  "sha256": "e0649893484a0f5feb7037e2a387e4b59e677ed352c1c3ca3093ffb437cf137c",
  "lines": 348,
  "bytes": 12589,
  "selection_score": 0,
  "keyword_line_counts": {
    "teeth": 6,
    "tooth": 10,
    "deform": 2,
    "face": 4
  },
  "url": "https://github.com/SJTUzhou/3D-Teeth-Reconstruction-from-Five-Intra-oral-Images/blob/4754279119ae871579b31877b2b24a308c73fc6d/main.py",
  "function_signature_candidates": [
    {
      "line": 30,
      "text": "def getToothIndex(f):"
    },
    {
      "line": 34,
      "text": "def loadMuEigValSigma(ssmDir, numPC):"
    },
    {
      "line": 48,
      "text": "def run_emopt(emopt: EMOpt5Views, verbose: bool = False):"
    },
    {
      "line": 188,
      "text": "def evaluation(h5File, X_Ref_Upper, X_Ref_Lower):"
    },
    {
      "line": 232,
      "text": "def create_mesh_from_emopt_h5File(h5File, meshDir, save_name):"
    },
    {
      "line": 262,
      "text": "def read_demo_mesh_vertices_by_FDI(mesh_dir, tag, FDIs):"
    },
    {
      "line": 273,
      "text": "def main(tag=\"0\"):"
    }
  ]
}

```text

import functools
import glob
import os

os.environ["CUDA_VISIBLE_DEVICES"] = "-1"  # run on CPU
import sys

import h5py
import matplotlib.pyplot as plt
import numpy as np
import open3d as o3d
import psutil
import ray

import pcd_mesh_utils as pm_util
import recons_eval_metric as metric
from const import *
from emopt5views import EMOpt5Views
from seg.seg_const import IMG_SHAPE
from seg.seg_model import ASPP_UNet
from seg.utils import predict_teeth_contour

TEMP_DIR = r"./demo/_temp/"
os.makedirs(TEMP_DIR, exist_ok=True)

NUM_CPUS = psutil.cpu_count(logical=False)
print = functools.partial(print, flush=True)


def getToothIndex(f):
    return int(os.path.basename(f).split(".")[0].split("_")[-1])


def loadMuEigValSigma(ssmDir, numPC):
    """Mu.shape=(28,1500,3), sqrtEigVals.shape=(28,1,100), Sigma.shape=(28,4500,100)"""
    muNpys = glob.glob(os.path.join(ssmDir, "meanAlignedPG_*.npy"))
    muNpys = sorted(muNpys, key=lambda x: getToothIndex(x))
    Mu = np.array([np.load(x) for x in muNpys])
    eigValNpys = glob.glob(os.path.join(ssmDir, "eigVal_*.npy"))
    eigValNpys = sorted(eigValNpys, key=lambda x: getToothIndex(x))
    sqrtEigVals = np.sqrt(np.array([np.load(x) for x in eigValNpys]))
    eigVecNpys = glob.glob(os.path.join(ssmDir, "eigVec_*.npy"))
    eigVecNpys = sorted(eigVecNpys, key=lambda x: getToothIndex(x))
    Sigma = np.array([np.load(x) for x in eigVecNpys])
    return Mu, sqrtEigVals[:, np.newaxis, :numPC], Sigma[..., :numPC]


def run_emopt(emopt: EMOpt5Views, verbose: bool = False):
    # 3d teeth reconstruction by optimization
    print("-" * 100)
    print("Start optimization.")

    # grid search parallelled by Ray
    print("-" * 100)
    print("Start Grid Search.")

    # parallel function supported by Ray
    emopt.searchDefaultRelativePoseParams()
    emopt.gridSearchExtrinsicParams()
    emopt.gridSearchRelativePoseParams()

    emopt.expectation_step_5Views(-1, verbose)
    min_e_loss = emopt.get_e_loss()
    optParamDict = emopt.get_current_e_step_result()

    # stage0initMatFile = os.path.join(TEMP_DIR, "E-step-result-stage0-init.mat")
    # stage0finalMatFile = os.path.join(TEMP_DIR, "E-step-result-stage0-final.mat")

    # emopt.save_expectation_step_result(stage0initMatFile) # save checkpoint

    maxiter = 20
    stageIter = [10, 5, 10]
    # stage 0 & 1 optimization

    print("-" * 100)
    print("Start Stage 0.")
    stage = 0

    # # Continue from checkpoint "E-step-result-stage0-init.mat"
    # emopt.load_expectation_step_result(stage0initMatFile, stage)
    # emopt.expectation_step_5Views(stage, verbose)

    E_loss = []
    for it in range(stageIter[0]):
        emopt.maximization_step_5Views(stage, step=-1, maxiter=maxiter, verbose=False)
        print("M-step loss: {:.4f}".format(emopt.loss_maximization_step))
        emopt.expectation_step_5Views(stage, verbose)
        e_loss = emopt.get_e_loss()
        if e_loss < min_e_loss:
            optParamDict = emopt.get_current_e_step_result()
            min_e_loss = e_loss
        print("Sum of expectation step loss: {:.4f}".format(e_loss))
        if len(E_loss) >= 2 and e_loss >= np.mean(E_loss[-2:]):
            print(
                "Early stop with last 3 e-step loss {:.4f}, {:.4f}, {:.4f}".format(
                    E_loss[-2], E_loss[-1], e_loss
                )
            )
            E_loss.append(e_loss)
            break
        else:
            E_loss.append(e_loss)

    # Load best result of stage 0
    emopt.load_e_step_result_from_dict(optParamDict)
    emopt.expectation_step_5Views(stage, verbose)
    E_loss.append(min_e_loss)

    # emopt.save_expectation_step_result(stage0finalMatFile)  # save checkpoint

    skipStage1Flag = False
    print("-" * 100)
    print("Start Stage 1.")

    stage = 1
    for it in range(stageIter[1]):
        emopt.maximization_step_5Views(stage, step=-1, maxiter=maxiter, verbose=False)
        print("M-step loss: {:.4f}".format(emopt.loss_maximization_step))
        emopt.expectation_step_5Views(stage, verbose)
        e_loss = emopt.get_e_loss()
        print("Sum of expectation step loss: {:.4f}".format(e_loss))
        if e_loss >= E_loss[-1]:
            if it == 0:
                skipStage1Flag = True  # first optimization with rowScaleXZ gets worse result compared with optimziaiton without rowScaleXZ
            print(
                "Early stop with last 3 e-step loss {:.4f}, {:.4f}, {:.4f}".format(
                    E_loss[-2], E_loss[-1], e_loss
                )
            )
            break
        else:
            E_loss.append(e_loss)

    # whether to skip stage1 to avoid extreme deformation
    if skipStage1Flag == True:
        print("Skip Stage 1; Reverse to Stage 0 final result.")
        emopt.rowScaleXZ = np.ones((2,))
        emopt.load_e_step_result_from_dict(optParamDict)
        # # Continue from checkpoint "E-step-result-stage0-final.mat"
        # emopt.load_expectation_step_result(stage0finalMatFile, stage=2)
    else:
        print("Accept Stage 1.")
        print("emopt.rowScaleXZ: ", emopt.rowScaleXZ)
        print("approx tooth scale: ", np.prod(emopt.rowScaleXZ) ** (1 / 3))
        emopt.anistropicRowScale2ScalesAndTransVecs()

    # Load best result of stage 1
    emopt.expectation_step_5Views(stage, verbose)
    e_loss = emopt.get_e_loss()
    optParamDict = emopt.get_current_e_step_result()

    # Stage = 2 and 3
    print("-" * 100)
    print("Start Stage 2 and 3.")
    stage = 2
    E_loss = [
        min_e_loss,
    ]
    for it in range(stageIter[2]):
        emopt.maximization_step_5Views(stage, step=2, maxiter=maxiter, verbose=False)
        emopt.maximization_step_5Views(stage, step=3, maxiter=maxiter, verbose=False)
        emopt.maximization_step_5Views(stage=3, step=-1, maxiter=maxiter, verbose=False)
        emopt.maximization_step_5Views(stage, step=1, maxiter=maxiter, verbose=False)
        print("M-step loss: {:.4f}".format(emopt.loss_maximization_step))
        emopt.expectation_step_5Views(stage=3, verbose=verbose)
        e_loss = emopt.get_e_loss()
        if e_loss < min_e_loss:
            optParamDict = emopt.get_current_e_step_result()
            min_e_loss = e_loss
        print("Sum of expectation step loss: {:.4f}".format(e_loss))
        if len(E_loss) >= 2 and (e_loss >= np.mean(E_loss[-2:])):
            print(
                "Early stop with last 3 e-step loss {:.4f}, {:.4f}, {:.4f}".format(
                    E_loss[-2], E_loss[-1], e_loss
                )
            )
            break
        else:
            E_loss.append(e_loss)

    # Load best result of stage 2 and 3
    emopt.load_e_step_result_from_dict(optParamDict)
    emopt.expectation_step_5Views(stage=3, verbose=verbose)

    return emopt


def evaluation(h5File, X_Ref_Upper, X_Ref_Lower):
    """
    h5file: emopt result saved in h5 format
    X_Ref_Upper, X_Ref_Lower: List of numpy arrays
    """
    with h5py.File(h5File, "r") as f:
        grp = f["EMOPT"]
        X_Pred_Upper = grp["UPPER_PRED"][:]
        X_Pred_Lower = grp["LOWER_PRED"][:]

    _X_Ref = X_Ref_Upper + X_Ref_Lower  # List concat
    print(
        "Compare prediction shape aligned by similarity registration with ground truth."
    )
    with_scale = True
    TX_Upper = pm_util.getAlignedSrcPointCloud(
        X_Pred_Upper.reshape(-1, 3), np.concatenate(X_Ref_Upper), with_scale=with_scale
    )
    TX_Lower = pm_util.getAlignedSrcPointCloud(
        X_Pred_Lower.reshape(-1, 3), np.concatenate(X_Ref_Lower), with_scale=with_scale
    )

    TX_Pred_Upper = TX_Upper.reshape(-1, NUM_POINT, 3)
    TX_Pred_Lower = TX_Lower.reshape(-1, NUM_POINT, 3)
    _TX_Pred = np.concatenate([TX_Pred_Upper, TX_Pred_Lower])

    RMSD_T_pred = metric.computeRMSD(_X_Ref, _TX_Pred)
    ASSD_T_pred = metric.computeASSD(_X_Ref, _TX_Pred)
    HD_T_pred = metric.computeHD(_X_Ref, _TX_Pred)
    CD_T_pred = metric.computeChamferDistance(_X_Ref, _TX_Pred)
    print("[RMSD] Root Mean Squared surface Distance (mm): {:.4f}".format(RMSD_T_pred))
    print("[ASSD] average symmetric surface distance (mm): {:.4f}".format(ASSD_T_pred))
    print("[HD] Hausdorff distance (mm): {:.4f}".format(HD_T_pred))
    print("[CD] Chamfer distance (mm^2): {:.4f}".format(CD_T_pred))

    Dice_VOE_lst = [
        metric.computeDiceAndVOE(_x_ref, _x_pred, pitch=0.2)
        for _x_ref, _x_pred in zip(_X_Ref, _TX_Pred)
    ]
    avg_Dice, avg_VOE = np.array(Dice_VOE_lst).mean(axis=0)
    print("[DC] Volume Dice Coefficient: {:.4f}".format(avg_Dice))
    print("[VOE] Volumetric Overlap Error: {:.2f} %".format(100.0 * avg_VOE))


def create_mesh_from_emopt_h5File(h5File, meshDir, save_name):
    with h5py.File(h5File, "r") as f:
        grp = f["EMOPT"]
        X_Pred_Upper = grp["UPPER_PRED"][:]
        X_Pred_Lower = grp["LOWER_PRED"][:]

    X_Pred_Upper_Meshes = [
        pm_util.surfaceVertices2WatertightO3dMesh(pg) for pg in X_Pred_Upper
    ]
    X_Pred_Lower_Meshes = [
        pm_util.surfaceVertices2WatertightO3dMesh(pg) for pg in X_Pred_Lower
    ]
    Pred_Upper_Mesh = pm_util.mergeO3dTriangleMeshes(X_Pred_Upper_Meshes)
    Pred_Lower_Mesh = pm_util.mergeO3dTriangleMeshes(X_Pred_Lower_Meshes)

    demoMeshDir = os.path.join(meshDir, "{}/".format(save_name))
    os.makedirs(demoMeshDir, exist_ok=True)

    pm_util.exportTriMeshObj(
        np.asarray(Pred_Upper_Mesh.vertices),
        np.asarray(Pred_Upper_Mesh.triangles),
        os.path.join(demoMeshDir, "Pred_Upper_Mesh_Tag={}.obj".format(save_name)),
    )
    pm_util.exportTriMeshObj(
        np.asarray(Pred_Lower_Mesh.vertices),
        np.asarray(Pred_Lower_Mesh.triangles),
        os.path.join(demoMeshDir, "Pred_Lower_Mesh_Tag={}.obj".format(save_name)),
    )


def read_demo_mesh_vertices_by_FDI(mesh_dir, tag, FDIs):
    mesh_vertices_by_FDI = []
    for fdi in FDIs:
        mshf = os.path.join(
            mesh_dir, str(tag), "byFDI", f"Ref_Mesh_Tag={tag}_FDI={fdi}.obj"
        )
        msh = o3d.io.read_triangle_mesh(mshf)
        mesh_vertices_by_FDI.append(np.asarray(msh.vertices, np.float64))
    return mesh_vertices_by_FDI


def main(tag="0"):
    Mu, SqrtEigVals, Sigma = loadMuEigValSigma(SSM_DIR, numPC=NUM_PC)
    Mu_normals = EMOpt5Views.computePointNormals(Mu)

    transVecStd = 1.1463183505325343  # obtained by SSM
    rotVecStd = 0.13909168140778128  # obtained by SSM
    PoseCovMats = np.load(
        os.path.join(REGIS_PARAM_DIR, "PoseCovMats.npy")
    )  # Covariance matrix of tooth pose for each tooth, shape=(28,6,6)
    ScaleCovMat = np.load(
        os.path.join(REGIS_PARAM_DIR, "ScaleCovMat.npy")
    )  # Covariance matrix of scales for each tooth, shape=(28,28)

    tooth_exist_mask = TOOTH_EXIST_MASK[tag]
    LogFile = os.path.join(TEMP_DIR, "Tag={}.log".format(tag))
    if os.path.exists(LogFile):
        os.remove(LogFile)
    log = open(LogFile, "a", encoding="utf-8")
    sys.stdout = log

    # teeth boundary segmentation model
    weight_ckpt = r".\seg\weights\weights-teeth-boundary-model.h5"
    model = ASPP_UNet(IMG_SHAPE, filters=[16, 32, 64, 128, 256])
    model.load_weights(weight_ckpt)

    # predcit teeth boundary in each photo
    edgeMasks = []
    for phtype in PHOTO_TYPES:
        imgfile = glob.glob(os.path.join(PHOTO_DIR, f"{tag}-{phtype.value}.png"))[0]
        edge_mask = predict_teeth_contour(
            model, imgfile, resized_width=RECONS_IMG_WIDTH
        )  # resize image to (800,~600)
        edgeMasks.append(edge_mask)
        # plt.imshow(edge_mask)
        # plt.show()

    # del model # to release memory

    mask_u, mask_l = np.split(tooth_exist_mask, 2)
    X_Ref_Upper = read_demo_mesh_vertices_by_FDI(
        mesh_dir=REF_MESH_DIR, tag=tag, FDIs=np.array(UPPER_INDICES)[mask_u]
    )
    X_Ref_Lower = read_demo_mesh_vertices_by_FDI(
        mesh_dir=REF_MESH_DIR, tag=tag, FDIs=np.array(LOWER_INDICES)[mask_l]
    )

    # run deformation-based 3d reconstruction
    emopt = EMOpt5Views(
        edgeMasks,
        PHOTO_TYPES,
        VISIBLE_MASKS,
        tooth_exist_mask,
        Mu,
        Mu_normals,
        SqrtEigVals,
        Sigma,
        PoseCovMats,
        ScaleCovMat,
        transVecStd,
        rotVecStd,
    )
    emopt = run_emopt(emopt)
    demoh5File = os.path.join(DEMO_H5_DIR, f"demo-tag={tag}.h5")
    emopt.saveDemo2H5(demoh5File)

    # evaluation(demoh5File, X_Ref_Upper, X_Ref_Lower)

    create_mesh_from_emopt_h5File(demoh5File, meshDir=DEMO_MESH_DIR, save_name=tag)

    log.close()


if __name__ == "__main__":
    ray.init(num_cpus=4, num_gpus=0)
    tag = "3"  # demo tag name: "0" or "1"
    main(tag)


```

## Source file: const.py

{
  "path": "const.py",
  "blob_sha": "86441bc7a58b69eff1dd7aa0af1d4437fc308f37",
  "sha256": "a8f531a30b5937c32367f15065b14965b15bcc1974f0c0d69192643e4c85061b",
  "lines": 240,
  "bytes": 3701,
  "selection_score": 0,
  "keyword_line_counts": {
    "teeth": 3,
    "tooth": 5,
    "deform": 1,
    "rig": 4,
    "face": 1
  },
  "url": "https://github.com/SJTUzhou/3D-Teeth-Reconstruction-from-Five-Intra-oral-Images/blob/4754279119ae871579b31877b2b24a308c73fc6d/const.py",
  "function_signature_candidates": []
}

```text

import enum
import os

import numpy as np

# tooth existence of the patient: FDI numbering order 11-17, 21-27, 31-37, 41-47
TOOTH_EXIST_MASK = {"0": np.ones((28,), np.bool_), "1": np.ones((28,), np.bool_)}


# Mask used to project the contour of the selected teeth in photos of different views
MASK_UPPER = np.array(
    [
        True,
        True,
        True,
        True,
        True,
        True,
        True,
        True,
        True,
        True,
        True,
        True,
        True,
        True,
        False,
        False,
        False,
        False,
        False,
        False,
        False,
        False,
        False,
        False,
        False,
        False,
        False,
        False,
    ]
)

MASK_LOWER = np.array(
    [
        False,
        False,
        False,
        False,
        False,
        False,
        False,
        False,
        False,
        False,
        False,
        False,
        False,
        False,
        True,
        True,
        True,
        True,
        True,
        True,
        True,
        True,
        True,
        True,
        True,
        True,
        True,
        True,
    ]
)

MASK_LEFT = np.array(
    [
        True,
        False,
        False,
        False,
        False,
        False,
        False,
        True,
        True,
        True,
        True,
        True,
        True,
        False,
        True,
        True,
        True,
        True,
        True,
        True,
        False,
        True,
        False,
        False,
        False,
        False,
        False,
        False,
    ]
)

MASK_RIGHT = np.array(
    [
        True,
        True,
        True,
        True,
        True,
        True,
        False,
        True,
        False,
        False,
        False,
        False,
        False,
        False,
        True,
        False,
        False,
        False,
        False,
        False,
        False,
        True,
        True,
        True,
        True,
        True,
        True,
        False,
    ]
)

MASK_FRONTAL = np.array(
    [
        True,
        True,
        True,
        True,
        True,
        True,
        False,
        True,
        True,
        True,
        True,
        True,
        True,
        False,
        True,
        True,
        True,
        True,
        True,
        True,
        False,
        True,
        True,
        True,
        True,
        True,
        True,
        False,
    ]
)


@enum.unique
class PHOTO(enum.Enum):
    # Enum values must be 0,1,2,3,4
    UPPER = 0
    LOWER = 1
    LEFT = 2
    RIGHT = 3
    FRONTAL = 4


PHOTO_TYPES = [PHOTO.UPPER, PHOTO.LOWER, PHOTO.LEFT, PHOTO.RIGHT, PHOTO.FRONTAL]
VISIBLE_MASKS = [MASK_UPPER, MASK_LOWER, MASK_LEFT, MASK_RIGHT, MASK_FRONTAL]
RECONS_IMG_WIDTH = 800

PHOTO_DIR = r"./seg/valid/image"

NUM_PC = 10  # num of modes of deformation for each tooth used in reconstruction
NUM_POINT = 1500  # num of points to represent tooth surface used in SSM
PG_SHAPE = (NUM_POINT, 3)

# FDI TOOTH NUMEBRING
UPPER_INDICES = [
    11,
    12,
    13,
    14,
    15,
    16,
    17,
    21,
    22,
    23,
    24,
    25,
    26,
    27,
]  # ignore wisdom teeth
LOWER_INDICES = [
    31,
    32,
    33,
    34,
    35,
    36,
    37,
    41,
    42,
    43,
    44,
    45,
    46,
    47,
]  # ignore wisdom teeth


SSM_DIR = r"./ssm/eigValVec/"
REGIS_PARAM_DIR = r"./ssm/cpdGpParams/"
DEMO_H5_DIR = r"./demo/h5/"
DEMO_MESH_DIR = r"./demo/mesh/"
REF_MESH_DIR = r"./demo/ref_mesh/"
VIS_DIR = r"./demo/visualization"
os.makedirs(DEMO_H5_DIR, exist_ok=True)
os.makedirs(DEMO_MESH_DIR, exist_ok=True)
os.makedirs(VIS_DIR, exist_ok=True)


```

## Source file: seg/utils.py

{
  "path": "seg/utils.py",
  "blob_sha": "48ed92dcd7a191916d080793b2db2a2ef1b5d864",
  "sha256": "330e062f8b533026b8638e1d6de727b914fb7d51bf4b2a31783310fc40d1c96e",
  "lines": 25,
  "bytes": 826,
  "selection_score": 0,
  "keyword_line_counts": {
    "teeth": 1
  },
  "url": "https://github.com/SJTUzhou/3D-Teeth-Reconstruction-from-Five-Intra-oral-Images/blob/4754279119ae871579b31877b2b24a308c73fc6d/seg/utils.py",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "def get_contour_from_raw_pred(pred_label, mask_shape, thresh=0.5):"
    },
    {
      "line": 15,
      "text": "def predict_teeth_contour(model, imgfile, resized_width=800):"
    }
  ]
}

```text

import numpy as np
import skimage

from seg.seg_const import IMG_SHAPE


def get_contour_from_raw_pred(pred_label, mask_shape, thresh=0.5):
    pred_prob_map = skimage.transform.resize(pred_label, mask_shape)
    pred_mask = pred_prob_map > thresh
    pred_mask = skimage.morphology.skeletonize(pred_mask.astype(np.uint8))
    pred_edge_img = (255.0 * pred_mask).astype(np.uint8)
    return pred_edge_img


def predict_teeth_contour(model, imgfile, resized_width=800):
    img = skimage.io.imread(imgfile)
    h, w = img.shape[:2]
    scale = resized_width / w
    rimg = skimage.transform.resize(img, IMG_SHAPE)
    raw_pred = model.predict(rimg[None, :])
    raw_pred = np.squeeze(raw_pred)
    edge_pred = get_contour_from_raw_pred(
        raw_pred, (int(scale * h), int(scale * w)), thresh=0.5
    )
    return edge_pred


```
