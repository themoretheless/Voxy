# Facial rendering source scan: LiyCg/DFL

Repository: https://github.com/LiyCg/DFL

Pinned source revision: 3bd74bdb45d25bb029be2eff7fb1816d46e52141

Discovery topic: facial retargeting

Description: Official Implementation of "Deep-Learning-Based Facial Retargeting Using Local Patches"

Reported license: GPL-3.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"face": ["src/bpNet/face_dataset.py", "src/reenactNet/face_dataset.py"]}

## Source file: src/bpNet/face_dataset.py

{
  "path": "src/bpNet/face_dataset.py",
  "blob_sha": "0b40dc4bd48c45a4e4d36759ad19cf4f9feb7c2c",
  "sha256": "70824eeeae116d5df9c68c082d4ad9648b970f9bfc6852215e77a60d983f4f30",
  "lines": 406,
  "bytes": 17381,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 38,
    "blendshape": 4,
    "rig": 2,
    "face": 4,
    "lip": 23
  },
  "url": "https://github.com/LiyCg/DFL/blob/3bd74bdb45d25bb029be2eff7fb1816d46e52141/src/bpNet/face_dataset.py",
  "function_signature_candidates": [
    {
      "line": 23,
      "text": "def __init__(self, root_dir, anim_dir, is_pca = False, is_test=False, aug_color=False, device=\"cuda\"):"
    },
    {
      "line": 114,
      "text": "def __len__(self):"
    },
    {
      "line": 117,
      "text": "def __getitem__(self, idx):"
    },
    {
      "line": 183,
      "text": "def __init__(self, root_dir, is_test=True, color=False, device=\"cuda\"):"
    },
    {
      "line": 222,
      "text": "def __len__(self):"
    },
    {
      "line": 225,
      "text": "def __getitem__(self, idx):"
    },
    {
      "line": 257,
      "text": "def random_warp(image):"
    },
    {
      "line": 308,
      "text": "def umeyama(src, dst, estimate_scale):"
    },
    {
      "line": 381,
      "text": "def __init__(self, root_dir, anim_dir, device=\"cuda\"):"
    },
    {
      "line": 397,
      "text": "def __len__(self):"
    },
    {
      "line": 400,
      "text": "def __getitem__(self, idx):"
    }
  ]
}

```text

import os
import torch
import torch.nn.functional as F
from torch.utils.data import Dataset, DataLoader
import torchvision.transforms as transforms
import glob
import numpy as np
import cv2
import sys
import numpy as np

from kornia.utils.helpers import _torch_inverse_cast, _torch_solve_cast
from kornia.geometry.conversions import (
    convert_affinematrix_to_homography,
    normalize_homography,
)
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), './../../dataset/data_preparation/renderer/')))
from animation import AnimationBlendShape, AnimationPCA


class FaceImgDataset_local(Dataset):

    def __init__(self, root_dir, anim_dir, is_pca = False, is_test=False, aug_color=False, device="cuda"):
        
        ### face image directory ###
        # val_dir = "../../dataset/image/child/normal_child" # child
        # val_dir = "../../dataset/image/girl/normal_girl" # girl
        val_dir = root_dir
        
        ### animation(that rendered image frames) directory ###
        if is_pca:
            ## PCA case, normalization = True as default
            self.animation = AnimationPCA(anim_dir)
            val_anim_dir = anim_dir
            self.animation_val = AnimationPCA(val_anim_dir)
            print("using pca param")
        else:
            ## Blendshape case
            self.animation = AnimationBlendShape(anim_dir)
            val_anim_dir = anim_dir
            self.animation_val = AnimationBlendShape(val_anim_dir)
            print("using bshp param")
        
        
        ### get full animation ###
        # import pdb;pdb.set_trace()
        self.full_anim = self.animation.get_full_animation()
        self.anim_val_full = self.animation_val.get_full_animation()

        ### image ### 
        load_list = ["img", "img_eyel", "img_eyer", "img_lips"]
        loaded_data = {}
        loaded_data_val = {}

        for load_element in load_list:
            # file_list = [glob.glob(root_dir+e, recursive=True) for e in ["/**/"+load_element+"/*.png", "/**/"+load_element+"/*.jpg"]]
            file_list = [glob.glob(root_dir+"/"+load_element+"/*.png", recursive=False)]
            file_list = sum(file_list, [])
            file_list.sort()
            loaded_data[load_element] = file_list # contains all the file list in each list element(img, img_eyel, img_eyer, img_lips)

            file_list_val = [glob.glob(val_dir+e, recursive=True) for e in ["/**/"+load_element+"/*.png", "/**/"+load_element+"/*.jpg"]]
            file_list_val = sum(file_list_val, [])
            file_list_val.sort()
            loaded_data_val[load_element] = file_list_val

        self.img_file_list = loaded_data[load_list[0]]
        self.img_eyel_file_list = loaded_data[load_list[1]]
        self.img_eyer_file_list = loaded_data[load_list[2]]
        self.img_lips_file_list = loaded_data[load_list[3]]
        print("img_len: ", len(self.img_eyel_file_list), "anim_len: ", self.full_anim.shape[0])
        # import pdb;pdb.set_trace()
        if is_test == False:
            assert len(self.img_eyel_file_list) == len(self.img_eyer_file_list) == len(self.img_lips_file_list) == self.full_anim.shape[0]
        self.index = len(self.img_eyel_file_list)

        self.img_file_list_val = loaded_data_val[load_list[0]]
        self.img_eyel_file_list_val = loaded_data_val[load_list[1]]
        self.img_eyer_file_list_val = loaded_data_val[load_list[2]]
        self.img_lips_file_list_val = loaded_data_val[load_list[3]]
        self.val_len = len(self.img_eyel_file_list_val)

        self.transform = {
            'base': transforms.Compose([
                                # INPUT : tensor (0~255)
                                transforms.Resize((128, 128)),
                                transforms.ToTensor(),
            ]),
            'color': transforms.Compose([
                                # INPUT : tensor (0~255)
                                transforms.ToPILImage(),
                                transforms.Resize((128, 128)),
                                transforms.ColorJitter(saturation=(0.1, 1)),
                                transforms.ToTensor()
            ])
#             'color': transforms.Compose([
#                                 # INPUT : tensor (0~255)
#                                 transforms.ToPILImage(),
#                                 transforms.Resize((128, 128)),
#                                 transforms.ColorJitter(brightness=(0.2, 2), 
#                                                         contrast=(0.3, 2), 
#                                                         saturation=(0.2, 2), 
#                                                         hue=(-0.3, 0.3)),
#                                 transforms.ToTensor(),
#             ])
        }

        self.is_test = is_test
        self.aug_color = aug_color
        self.device = device

        print("root_dir: ", root_dir, ", Img length: ", self.index) 

    def __len__(self):
        return self.index

    def __getitem__(self, idx):
        
        img_file_path = self.img_file_list[idx] 
        eyel_img_file_path = self.img_eyel_file_list[idx]
        eyer_img_file_path = self.img_eyer_file_list[idx]
        lips_img_file_path = self.img_lips_file_list[idx]
        
        idx_val = idx % self.val_len
        img_file_path_val = self.img_file_list_val[idx_val]
        eyel_img_file_path_val = self.img_eyel_file_list_val[idx_val]
        eyer_img_file_path_val = self.img_eyer_file_list_val[idx_val]
        lips_img_file_path_val = self.img_lips_file_list_val[idx_val]

        if self.is_test == False:
            parameter = self.full_anim[idx]
            parameter_val = self.anim_val_full[idx_val]

        file_path_list = [img_file_path, eyel_img_file_path, eyer_img_file_path, lips_img_file_path]
        file_path_list_val = [img_file_path_val, eyel_img_file_path_val, eyer_img_file_path_val, lips_img_file_path_val]
        out = {}

        for file_path, file_path_val in zip(file_path_list, file_path_list_val):
            name = file_path.split("/")[-2].replace("_file_path", "").replace("img_", "") # eyel, eyer, lips
            img = cv2.imread(file_path)
            img = cv2.cvtColor(img, cv2.COLOR_BGR2RGB)
            img = cv2.resize(img, dsize=(128,128), interpolation=cv2.INTER_LANCZOS4)
            val_img = cv2.imread(file_path_val)
            val_img = cv2.cvtColor(val_img, cv2.COLOR_BGR2RGB)
            val_img = cv2.resize(val_img, dsize=(128,128), interpolation=cv2.INTER_LANCZOS4)

            if self.is_test:
                ''' OUPUT : tensor (C,H,W), max value is 1 '''
                img = img.transpose(2,0,1).astype("float32")/255.0
                img = torch.from_numpy(img).to(self.device, dtype=torch.float)

                out[name] = {"gt": img}
            
            else: # train
                if self.aug_color:
                    img = self.transform['color'](img.astype("uint8"))
                    val_img = self.transform['color'](val_img.astype("uint8"))
                    img = img.permute(1, 2, 0).numpy() * 255 
                    val_img = val_img.permute(1, 2, 0).numpy() * 255
                    warped_img, target_img, mat = random_warp(img)
                    warped_val_img, _, _ = random_warp(val_img)
                else:
                    warped_img, target_img, mat = random_warp(img)
                    warped_val_img, _, _ = random_warp(val_img)

                img = img / 255.0
                val_img = val_img / 255.0

                out[name] = {"gt": img, "warped": warped_img, "target": target_img, "warped_val": warped_val_img, "mat": mat}

        if self.is_test:      
            return {'gt':out["img"]["gt"], 'gt_eyel':out["eyel"]["gt"], 'gt_eyer':out["eyer"]["gt"], 'gt_lips':out["lips"]["gt"], 'idx':idx}

        else:
            return {'gt_eyel':out["eyel"]["gt"], 's_eyel':out["eyel"]["warped"], 't_eyel':out["eyel"]["target"], 'v_s_eyel':out["eyel"]["warped_val"], 'm_eyel':out["eyel"]["mat"], \
                    'gt_eyer':out["eyer"]["gt"], 's_eyer':out["eyer"]["warped"], 't_eyer':out["eyer"]["target"], 'v_s_eyer':out["eyer"]["warped_val"], 'm_eyer':out["eyer"]["mat"], \
                    'gt_lips':out["lips"]["gt"], 's_lips':out["lips"]["warped"], 't_lips':out["lips"]["target"], 'v_s_lips':out["lips"]["warped_val"], 'm_lips':out["lips"]["mat"], \
                    'idx':idx, 'p':parameter, 'v_p':parameter_val}


class FaceImgDataset_local_test(Dataset):

    def __init__(self, root_dir, is_test=True, color=False, device="cuda"):
        
        ### image ### 
        load_list = ["img", "img_eyel", "img_eyer", "img_lips"]
        loaded_data = {}

        for load_element in load_list:
            file_list = [glob.glob(root_dir+e, recursive=True) for e in ["/**/"+load_element+"/*.png", "/**/"+load_element+"/*.jpg"]]
            file_list = sum(file_list, [])
            file_list.sort()
            loaded_data[load_element] = file_list # contains all the file list in each list element(img, img_eyel, img_eyer, img_lips)

        self.img_file_list = loaded_data[load_list[0]]
        self.img_eyel_file_list = loaded_data[load_list[1]]
        self.img_eyer_file_list = loaded_data[load_list[2]]
        self.img_lips_file_list = loaded_data[load_list[3]]
        assert len(self.img_eyel_file_list) == len(self.img_eyer_file_list) == len(self.img_lips_file_list)
        self.index = len(self.img_eyel_file_list)

        self.transform = {
            'color': transforms.Compose([
                                # INPUT : tensor (0~255)
                                transforms.ToPILImage(),
                                transforms.Resize((128, 128)),
                                transforms.ColorJitter(saturation=(0.1, 1.0)),
                                # transforms.ColorJitter(brightness=(0.2, 2), 
                                #                         contrast=(0.3, 2), 
                                #                         saturation=(0.2, 2), 
                                #                         hue=(-0.3, 0.3)),
                                transforms.ToTensor(), # automatically scales the pixel intensity values of images from a range of [0,255] (typical for images stored in the standard 8-bit format) to a floating point range of [0,1].
            ])
        }
        
        self.color = color
        self.is_test = is_test
        self.device = device

        print("root_dir: ", root_dir, ", Img length: ", self.index) 

    def __len__(self):
        return self.index

    def __getitem__(self, idx):
        
        img_file_path = self.img_file_list[idx] 
        eyel_img_file_path = self.img_eyel_file_list[idx]
        eyer_img_file_path = self.img_eyer_file_list[idx]
        lips_img_file_path = self.img_lips_file_list[idx]

        file_path_list = [img_file_path, eyel_img_file_path, eyer_img_file_path, lips_img_file_path]
        out = {}

        for file_path in (file_path_list):
            name = file_path.split("/")[-2].replace("_file_path", "").replace("img_", "") # eyel, eyer, lips
            img = cv2.imread(file_path)
            img = cv2.cvtColor(img, cv2.COLOR_BGR2RGB)
            img = cv2.resize(img, dsize=(128,128), interpolation=cv2.INTER_LANCZOS4)

            if self.is_test:
                if self.color:
                    img = self.transform['color'](img).to(self.device)
                    # img = img / 255.0
                else:
                    ''' OUPUT : tensor (C,H,W), max value is 1 '''
                    img = img.transpose(2,0,1).astype("float32")/255.0
                    img = torch.from_numpy(img).to(self.device, dtype=torch.float)

                out[name] = {"gt": img}

        if self.is_test:    
            return {'gt':out["img"]["gt"], 'gt_eyel':out["eyel"]["gt"], 'gt_eyer':out["eyer"]["gt"], 'gt_lips':out["lips"]["gt"], 'idx':idx}



def random_warp(image):
    # assert image.shape == (256, 256, 3)
    # range_ = np.zeros(7)
    w = 128
    cell_size = [ w // (2**i) for i in range(1,4) ] [ np.random.randint(3) ]
    # cell_size = 32
    cell_count = w // cell_size + 1
    # cell_count = 5
    range_ = np.linspace( 0, w, cell_count)
    # range_ = np.linspace(0, 256, 5)

    mapx = np.broadcast_to(range_, (cell_count, cell_count)).copy()
    mapy = mapx.T

    mapx = mapx + np.random.normal(size=(cell_count, cell_count)) * (cell_size*0.12)
    mapy = mapy + np.random.normal(size=(cell_count, cell_count)) * (cell_size*0.12)

    half_cell_size = cell_size // 2
    interp_mapx = cv2.resize(mapx, (w+cell_size,)*2 )[half_cell_size:-half_cell_size,half_cell_size:-half_cell_size].astype(np.float32)
    interp_mapy = cv2.resize(mapy, (w+cell_size,)*2 )[half_cell_size:-half_cell_size,half_cell_size:-half_cell_size].astype(np.float32)
    
    warped_image = cv2.remap(image, interp_mapx, interp_mapy, cv2.INTER_LINEAR)
    
    img_torch = torch.from_numpy(cv2.cvtColor(warped_image, cv2.COLOR_BGR2RGB)/255)
    img_torch = torch.permute(img_torch, (2, 0, 1)).unsqueeze(0).float()

    src_points = np.stack([mapx.ravel(), mapy.ravel()], axis=-1)
    dst_points = np.mgrid[0:w+1:cell_size, 0:w+1:cell_size].T.reshape(-1, 2)
    
    mat = umeyama(src_points, dst_points, True)[0:2]

    mat_torch = torch.from_numpy(mat).type(torch.FloatTensor).unsqueeze(0)   # [1,2,3]
    
    M_3x3: torch.Tensor = convert_affinematrix_to_homography(mat_torch)
    dst_norm_trans_src_norm: torch.Tensor = normalize_homography(M_3x3, (256, 256), (256, 256))
    mat_torch = _torch_inverse_cast(dst_norm_trans_src_norm)  # torch.Size([1, 3, 3])
     
    grid = F.affine_grid(mat_torch[:, :2, :].float(), img_torch.size(), align_corners=True)    # [1, 128, 128, 2]
    image = torch.from_numpy(cv2.cvtColor(image, cv2.COLOR_BGR2RGB)/255).type(torch.FloatTensor).permute(2,0,1).unsqueeze(0)
    target_image = F.grid_sample(image, grid, align_corners=True)    # [1, 3, 128, 128]
    target_image =  np.array(target_image.squeeze(0).permute(1, 2, 0))
    target_image = (cv2.cvtColor(target_image, cv2.COLOR_BGR2RGB))
    mat_torch_2X3 = mat_torch[:, :2, :].squeeze(0)
    
    # target_image = cv2.warpAffine(image, mat, (w, w))
    # mask = cv2.warpAffine(mask, mat, (w, w))
    
    return warped_image, target_image, mat_torch_2X3



def umeyama(src, dst, estimate_scale):
    """Estimate N-D similarity transformation with or without scaling.
    Parameters
    ----------
    src : (M, N) array
        Source coordinates.
    dst : (M, N) array
        Destination coordinates.
    estimate_scale : bool
        Whether to estimate scaling factor.
    Returns
    -------
    T : (N + 1, N + 1)
        The homogeneous similarity transformation matrix. The matrix contains
        NaN values only if the problem is not well-conditioned.
    References
    ----------
    .. [1] "Least-squares estimation of transformation parameters between two
            point patterns", Shinji Umeyama, PAMI 1991, :DOI:`10.1109/34.88573`
    """

    num = src.shape[0]
    dim = src.shape[1]

    # Compute mean of src and dst.
    src_mean = src.mean(axis=0)
    dst_mean = dst.mean(axis=0)

    # Subtract mean from src and dst.
    src_demean = src - src_mean
    dst_demean = dst - dst_mean

    # Eq. (38).
    A = dst_demean.T @ src_demean / num

    # Eq. (39).
    d = np.ones((dim,), dtype=np.double)
    if np.linalg.det(A) < 0:
        d[dim - 1] = -1

    T = np.eye(dim + 1, dtype=np.double)

    U, S, V = np.linalg.svd(A)

    # Eq. (40) and (43).
    rank = np.linalg.matrix_rank(A)
    if rank == 0:
        return np.nan * T
    elif rank == dim - 1:
        if np.linalg.det(U) * np.linalg.det(V) > 0:
            T[:dim, :dim] = U @ V
        else:
            s = d[dim - 1]
            d[dim - 1] = -1
            T[:dim, :dim] = U @ np.diag(d) @ V
            d[dim - 1] = s
    else:
        T[:dim, :dim] = U @ np.diag(d) @ V

    if estimate_scale:
        # Eq. (41) and (42).
        scale = 1.0 / src_demean.var(axis=0).sum() * (S @ d)
    else:
        scale = 1.0

    T[:dim, dim] = dst_mean - scale * (T[:dim, :dim] @ src_mean.T)
    T[:dim, :dim] *= scale

    return T


class FaceLatentDataset(Dataset):

    def __init__(self, root_dir, anim_dir, device="cuda"):
        
        self.animation = AnimationPCA(anim_dir)
        self.full_anim = self.animation.get_full_animation()

        ckpt_dir_reenactNet = "../checkpoint/ReenactNet/{0}".format(root_dir)
        reenactNet_network_dir = os.path.join(ckpt_dir_reenactNet, "train_result")

        self.eyel_latents = torch.load(os.path.join(reenactNet_network_dir, "eyel_latents.pth")).squeeze(dim=1)
        self.eyer_latents = torch.load(os.path.join(reenactNet_network_dir, "eyer_latents.pth")).squeeze(dim=1)
        self.lips_latents = torch.load(os.path.join(reenactNet_network_dir, "lips_latents.pth")).squeeze(dim=1)

        self.device = device

        print("root_dir: ", reenactNet_network_dir) 

    def __len__(self):
        return len(self.eyel_latents)

    def __getitem__(self, idx):
        eyel_latent = self.eyel_latents[idx]
        eyer_latent = self.eyer_latents[idx]
        lips_latent = self.lips_latents[idx]
        parameter = self.full_anim[idx]
        return {'eyel':eyel_latent, 'eyer':eyer_latent, 'lips':lips_latent, 'p':parameter}
        # return {'p':parameter}


```

## Source file: src/reenactNet/face_dataset.py

{
  "path": "src/reenactNet/face_dataset.py",
  "blob_sha": "d0154c29dc7ae85dba6949d54ff8b85e1b36635f",
  "sha256": "ddf8c53f6405ae7fce6347eaaa2220521bc16fcc1cbc10a4908c01a6f1248d2e",
  "lines": 485,
  "bytes": 21141,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 42,
    "blendshape": 4,
    "rig": 3,
    "face": 2,
    "lip": 24
  },
  "url": "https://github.com/LiyCg/DFL/blob/3bd74bdb45d25bb029be2eff7fb1816d46e52141/src/reenactNet/face_dataset.py",
  "function_signature_candidates": [
    {
      "line": 16,
      "text": "def __init__(self, root_dir, is_test=False, aug_rotation=False, aug_random=False, aug_color=False, aug_affine_color=False, device=\"cuda\", id=\"real\"):"
    },
    {
      "line": 118,
      "text": "def __len__(self):"
    },
    {
      "line": 121,
      "text": "def __getitem__(self, idx):"
    },
    {
      "line": 219,
      "text": "def __init__(self, root_dir, anim_dir, is_test=False, aug_color=False, device=\"cuda\"):"
    },
    {
      "line": 294,
      "text": "def __len__(self):"
    },
    {
      "line": 297,
      "text": "def __getitem__(self, idx):"
    },
    {
      "line": 364,
      "text": "def random_warp(image):"
    },
    {
      "line": 415,
      "text": "def umeyama(src, dst, estimate_scale):"
    }
  ]
}

```text

import os
import torch
import torch.nn.functional as F
from torch.utils.data import Dataset, DataLoader
import torchvision.transforms as transforms
import torchvision.transforms.functional as TF
import glob
import numpy as np
import cv2
import sys
import numpy as np


class FaceImgDataset(Dataset):

    def __init__(self, root_dir, is_test=False, aug_rotation=False, aug_random=False, aug_color=False, aug_affine_color=False, device="cuda", id="real"):
        '''
        local - eyel_img, eyer_img, lips_img
        '''
        # if id == "real":
        #     val_dir = "../../dataset/image/real/sihun_brow_test"
        # elif id == "child":
        #     val_dir = "../../dataset/image/child/normal_child"
        # elif id == "girl":
        #     val_dir = "../../dataset/image/girl/normal_girl"
        # elif id == "piers":
        #     val_dir = "../../dataset/image/piers/normal_piers"
        # elif id == "malcom":
        #     val_dir = "../../dataset/image/malcom/normal_malcom"
        val_dir = root_dir
        

        load_list = ["img_eyel", "img_eyer", "img_lips"]
        loaded_data = {}
        loaded_data_val = {}
        for load_element in load_list:
            # file_list = [glob.glob(root_dir+e, recursive=False) for e in ["/**/"+load_element+"/*.png", "/**/"+load_element+"/*.jpg"]] # match any directories, nested to any level, from the root directory
            file_list = [glob.glob(root_dir+"/"+load_element+"/*.png", recursive=False)]
            
            file_list = sum(file_list, [])
            file_list.sort()
            loaded_data[load_element] = file_list

            # file_list_val = [glob.glob(val_dir+e, recursive=False) for e in ["/**/"+load_element+"/*.png", "/**/"+load_element+"/*.jpg"]]
            file_list_val = [glob.glob(val_dir+"/"+load_element+"/*.png", recursive=False)]
            file_list_val = sum(file_list_val, [])
            file_list_val.sort()
            loaded_data_val[load_element] = file_list_val

        self.img_eyel_file_list     = loaded_data[load_list[0]]
        self.img_eyer_file_list     = loaded_data[load_list[1]]
        self.img_lips_file_list     = loaded_data[load_list[2]]

        self.img_eyel_file_list_val = loaded_data_val[load_list[0]]
        self.img_eyer_file_list_val = loaded_data_val[load_list[1]]
        self.img_lips_file_list_val = loaded_data_val[load_list[2]]
        

        assert len(self.img_eyel_file_list) == len(self.img_eyer_file_list) == len(self.img_lips_file_list)

        self.index = len(self.img_eyel_file_list)
        self.val_len = len(self.img_eyel_file_list_val)

        self.transform = {
            'base': transforms.Compose([
                                # INPUT : tensor (0~255)
                                transforms.ToPILImage(),
                                transforms.Resize((128, 128)),
                                transforms.ToTensor(),
            ]),
            'random': transforms.Compose([
                                # INPUT : tensor (0~255)
                                transforms.ToPILImage(),
                                transforms.Resize((128, 128)),
                                transforms.RandomAffine(degrees=(-10, 10), translate=(0.1, 0.1), scale=(0.9, 1.1)),
                                transforms.ToTensor(),
                    
            ]),
            'color': transforms.Compose([
                                # INPUT : tensor (0~255)
                                transforms.ToPILImage(),
                                transforms.Resize((128, 128)),
                                transforms.ColorJitter(brightness=(0.5, 2), 
                                                        contrast=(0.5, 2), 
                                                        saturation=(0.5, 2), 
                                                        hue=(-0.2, 0.2)),
                                transforms.ToTensor(),
                    
            ]),
            'random_color': transforms.Compose([
                                # INPUT : tensor (0~255)
                                transforms.ToPILImage(),
                                transforms.Resize((128, 128)),
                                transforms.RandomAffine(degrees=(-10, 10), translate=(0.1, 0.1), scale=(0.9, 1.1)),
                                transforms.ColorJitter(brightness=(0.5, 2), 
                                                        contrast=(0.5, 2), 
                                                        saturation=(0.5, 2), 
                                                        hue=(-0.2, 0.2)),
                                transforms.ToTensor(),
            ]),

            'valid': transforms.Compose([
                transforms.Resize((224, 224)),
                transforms.ToTensor(),
                transforms.Normalize([0.485, 0.456, 0.406], [0.229, 0.224, 0.225])
        ]),
    }

        self.is_test = is_test
        self.aug_rotation = aug_rotation
        self.aug_random = aug_random
        self.aug_color = aug_color
        self.aug_affine_color = aug_affine_color
        self.device = device
        self.id = id
        print("root_dir: ", root_dir, ", Img length: ", self.index ) 

    def __len__(self):
        return self.index

    def __getitem__(self, idx):
        
        eyel_img_file_path = self.img_eyel_file_list[idx]
        eyer_img_file_path = self.img_eyer_file_list[idx]
        lips_img_file_path = self.img_lips_file_list[idx]

        idx_val = idx % self.val_len
        eyel_img_file_path_val = self.img_eyel_file_list_val[idx_val]
        eyer_img_file_path_val = self.img_eyer_file_list_val[idx_val]
        lips_img_file_path_val = self.img_lips_file_list_val[idx_val]

        file_path_list = [eyel_img_file_path, eyer_img_file_path, lips_img_file_path]
        file_path_list_val = [eyel_img_file_path_val, eyer_img_file_path_val, lips_img_file_path_val]

        out = {}

        for file_path, file_path_val in zip(file_path_list, file_path_list_val):
            name = file_path.split("/")[-2].replace("_file_path", "").replace("img_", "")
            img = cv2.imread(file_path)
            img = cv2.cvtColor(img, cv2.COLOR_BGR2RGB)
            img = cv2.resize(img, dsize=(128,128), interpolation=cv2.INTER_LANCZOS4) # 0~255 numpy

            val_img = cv2.imread(file_path_val)
            val_img = cv2.cvtColor(val_img, cv2.COLOR_BGR2RGB)
            val_img = cv2.resize(val_img, dsize=(128,128), interpolation=cv2.INTER_LANCZOS4)

            if self.is_test:
                ''' OUPUT : tensor (C,H,W), max value is 1 '''
                img = img.transpose(2,0,1).astype("float32")/255.0
                img = torch.from_numpy(img).to(self.device, dtype=torch.float)
                
                out[name] = {"gt": img}
            
            elif self.is_test == False: 
                ''' is_train '''
                if self.id == "mery":
                    if self.aug_rotation:
                        if name == "eyer":
                            img = self.transform['base'](img) # 0~1 tensor
                            img = TF.rotate(img, angle=-20.0) # 0~1 tensor 
                            img = img.permute(1, 2, 0).numpy() * 255.0
                    
                        if name == "eyel":
                            img = self.transform['base'](img)
                            img = TF.rotate(img, angle=20.0)
                            img = img.permute(1, 2, 0).numpy() * 255.0
                
                if self.aug_random:
                    warped_img = self.transform['random'](img.astype("uint8"))
                    warped_img = warped_img.permute(1, 2, 0).numpy() 

                if self.aug_color: # aug_color only warped_img
                    warped_img = self.transform['color'](img.astype('uint8'))
                    warped_img = warped_img.permute(1, 2, 0).numpy()

                if self.aug_affine_color:
                    warped_img = self.transform['random_color'](img.astype('uint8'))
                    warped_img = warped_img.permute(1, 2, 0).numpy()

                img = img/255.0
                val_img = val_img/255.0

                out[name] = {"gt": img, "warped": warped_img, "gt_val": val_img} # 0~1 numpy

        if self.is_test:      
            return {'gt_eyel':out["eyel"]["gt"], 'gt_eyer':out["eyer"]["gt"], 'gt_lips':out["lips"]["gt"], 'idx':idx}

        else:
            return {'gt_eyel':out["eyel"]["gt"], 's_eyel':out["eyel"]["warped"], 'v_eyel':out["eyel"]["gt_val"], \
                    'gt_eyer':out["eyer"]["gt"], 's_eyer':out["eyer"]["warped"], 'v_eyer':out["eyer"]["gt_val"], \
                    'gt_lips':out["lips"]["gt"], 's_lips':out["lips"]["warped"], 'v_lips':out["lips"]["gt_val"], \
                    'idx':idx}        



import os
import torch
import torch.nn.functional as F
from torch.utils.data import Dataset, DataLoader
import torchvision.transforms as transforms
import glob
import numpy as np
import cv2
import sys
import numpy as np

from kornia.utils.helpers import _torch_inverse_cast, _torch_solve_cast
from kornia.geometry.conversions import (
    convert_affinematrix_to_homography,
    normalize_homography,
)

sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), './../../dataset/data_preparation/renderer/')))
from animation import AnimationBlendShape, AnimationPCA


class FaceImgDataset_local(Dataset):

    def __init__(self, root_dir, anim_dir, is_test=False, aug_color=False, device="cuda"):
        
        ### animation(that rendered image frames) directory ###
            ## Blendshape case
        # self.animation = AnimationBlendShape(anim_dir)
        # val_anim_dir = "../../dataset/animation/test_normal.json"
        # self.animation_val = AnimationBlendShape(val_anim_dir)
            ## PCA case
        self.animation = AnimationPCA(anim_dir)
        # val_anim_dir = "../../dataset/animation/val_normal.pth"
        
        # val_dir = anim_dir
        # self.animation_val = AnimationPCA(val_dir)
        
        ### get full animation ###
        self.full_anim = self.animation.get_full_animation()
        # self.anim_val_full = self.animation_val.get_full_animation()

        ### image ### 
        load_list = ["img", "img_eyel", "img_eyer", "img_lips"]
        loaded_data = {}
        # loaded_data_val = {}

        for load_element in load_list:
            # file_list = [glob.glob(root_dir+e, recursive=True) for e in ["/**/"+load_element+"/*.png", "/**/"+load_element+"/*.jpg"]]
            file_list = [glob.glob(root_dir+"/"+load_element+"/*.png", recursive=False)]
            file_list = sum(file_list, [])
            file_list.sort()
            loaded_data[load_element] = file_list # contains all the file list in each list element(img, img_eyel, img_eyer, img_lips)

            # file_list_val = [glob.glob(val_dir+e, recursive=True) for e in ["/**/"+load_element+"/*.png", "/**/"+load_element+"/*.jpg"]]
            # file_list_val = [glob.glob(val_dir+"/"+load_element+"/*.png", recursive=False)]

            # file_list_val = sum(file_list_val, [])
            # file_list_val.sort()
            # loaded_data_val[load_element] = file_list_val

        self.img_file_list = loaded_data[load_list[0]]
        self.img_eyel_file_list = loaded_data[load_list[1]]
        self.img_eyer_file_list = loaded_data[load_list[2]]
        self.img_lips_file_list = loaded_data[load_list[3]]
        assert len(self.img_eyel_file_list) == len(self.img_eyer_file_list) == len(self.img_lips_file_list)
        self.index = len(self.img_eyel_file_list)

        # self.img_file_list_val = loaded_data_val[load_list[0]]
        # self.img_eyel_file_list_val = loaded_data_val[load_list[1]]
        # self.img_eyer_file_list_val = loaded_data_val[load_list[2]]
        # self.img_lips_file_list_val = loaded_data_val[load_list[3]]
        # self.val_len = len(self.img_eyel_file_list_val)
        

        self.transform = {
            'base': transforms.Compose([
                                # INPUT : tensor (0~255)
                                transforms.Resize((128, 128)),
                                transforms.ToTensor(),
            ]),
            'color': transforms.Compose([
                                # INPUT : tensor (0~255)
                                transforms.ToPILImage(),
                                transforms.Resize((128, 128)),
                                transforms.ColorJitter(brightness=(0.2, 2), 
                                                        contrast=(0.3, 2), 
                                                        saturation=(0.2, 2), 
                                                        hue=(-0.3, 0.3)),
                                transforms.ToTensor(),
            ])
        }

        self.is_test = is_test
        self.aug_color = aug_color
        self.device = device

        print("root_dir: ", root_dir, ", Img length: ", self.index) 

    def __len__(self):
        return self.index

    def __getitem__(self, idx):
        
        img_file_path = self.img_file_list[idx] 
        eyel_img_file_path = self.img_eyel_file_list[idx]
        eyer_img_file_path = self.img_eyer_file_list[idx]
        lips_img_file_path = self.img_lips_file_list[idx]
        
        # idx_val = idx % self.val_len
        # img_file_path_val = self.img_file_list_val[idx_val]
        # eyel_img_file_path_val = self.img_eyel_file_list_val[idx_val]
        # eyer_img_file_path_val = self.img_eyer_file_list_val[idx_val]
        # lips_img_file_path_val = self.img_lips_file_list_val[idx_val]

        if self.is_test == False:
            parameter = self.full_anim[idx]
            # parameter_val = self.anim_val_full[idx_val]

        file_path_list = [img_file_path, eyel_img_file_path, eyer_img_file_path, lips_img_file_path]
        # file_path_list_val = [img_file_path_val, eyel_img_file_path_val, eyer_img_file_path_val, lips_img_file_path_val]
        out = {}

        # for file_path, file_path_val in zip(file_path_list, file_path_list_val):
        for file_path in file_path_list:
            name = file_path.split("/")[-2].replace("_file_path", "").replace("img_", "") # eyel, eyer, lips
            img = cv2.imread(file_path)
            img = cv2.cvtColor(img, cv2.COLOR_BGR2RGB)
            img = cv2.resize(img, dsize=(128,128), interpolation=cv2.INTER_LANCZOS4)
            # val_img = cv2.imread(file_path_val)
            # val_img = cv2.cvtColor(val_img, cv2.COLOR_BGR2RGB)
            # val_img = cv2.resize(val_img, dsize=(128,128), interpolation=cv2.INTER_LANCZOS4)

            if self.is_test:
                ''' OUPUT : tensor (C,H,W), max value is 1 '''
                img = img.transpose(2,0,1).astype("float32")/255.0
                img = torch.from_numpy(img).to(self.device, dtype=torch.float)

                out[name] = {"gt": img}
            
            else: # train
                if self.aug_color:
                    img = self.transform['color'](img)
                    # val_img = self.transform['color'](val_img)
                warped_img, target_img, mat = random_warp(img)
                # warped_val_img, _, _ = random_warp(val_img)

                img = img / 255.0
                # val_img = val_img / 255.0

                # out[name] = {"gt": img, "warped": warped_img, "target": target_img, "warped_val": warped_val_img, "mat": mat}
                out[name] = {"gt": img, "warped": warped_img, "target": target_img, "mat": mat}


        if self.is_test:      
            return {'gt':out["img"]["gt"], 'gt_eyel':out["eyel"]["gt"], 'gt_eyer':out["eyer"]["gt"], 'gt_lips':out["lips"]["gt"], 'idx':idx}

        else:
            # return {'gt_eyel':out["eyel"]["gt"], 's_eyel':out["eyel"]["warped"], 't_eyel':out["eyel"]["target"], 'v_s_eyel':out["eyel"]["warped_val"], 'm_eyel':out["eyel"]["mat"], \
            #         'gt_eyer':out["eyer"]["gt"], 's_eyer':out["eyer"]["warped"], 't_eyer':out["eyer"]["target"], 'v_s_eyer':out["eyer"]["warped_val"], 'm_eyer':out["eyer"]["mat"], \
            #         'gt_lips':out["lips"]["gt"], 's_lips':out["lips"]["warped"], 't_lips':out["lips"]["target"], 'v_s_lips':out["lips"]["warped_val"], 'm_lips':out["lips"]["mat"], \
            #         'idx':idx, 'p':parameter, 'v_p':parameter_val}
            
            return {'gt_eyel':out["eyel"]["gt"], 's_eyel':out["eyel"]["warped"], 't_eyel':out["eyel"]["target"],  'm_eyel':out["eyel"]["mat"], \
                    'gt_eyer':out["eyer"]["gt"], 's_eyer':out["eyer"]["warped"], 't_eyer':out["eyer"]["target"],  'm_eyer':out["eyer"]["mat"], \
                    'gt_lips':out["lips"]["gt"], 's_lips':out["lips"]["warped"], 't_lips':out["lips"]["target"],  'm_lips':out["lips"]["mat"], \
                    'idx':idx, 'p':parameter, }
            

def random_warp(image):
    # assert image.shape == (256, 256, 3)
    # range_ = np.zeros(7)
    w = 128
    cell_size = [ w // (2**i) for i in range(1,4) ] [ np.random.randint(3) ]
    # cell_size = 32
    cell_count = w // cell_size + 1
    # cell_count = 5
    range_ = np.linspace( 0, w, cell_count)
    # range_ = np.linspace(0, 256, 5)

    mapx = np.broadcast_to(range_, (cell_count, cell_count)).copy()
    mapy = mapx.T

    mapx = mapx + np.random.normal(size=(cell_count, cell_count)) * (cell_size*0.12)
    mapy = mapy + np.random.normal(size=(cell_count, cell_count)) * (cell_size*0.12)

    half_cell_size = cell_size // 2
    interp_mapx = cv2.resize(mapx, (w+cell_size,)*2 )[half_cell_size:-half_cell_size,half_cell_size:-half_cell_size].astype(np.float32)
    interp_mapy = cv2.resize(mapy, (w+cell_size,)*2 )[half_cell_size:-half_cell_size,half_cell_size:-half_cell_size].astype(np.float32)

    warped_image = cv2.remap(image, interp_mapx, interp_mapy, cv2.INTER_LINEAR)
    
    img_torch = torch.from_numpy(cv2.cvtColor(warped_image, cv2.COLOR_BGR2RGB)/255)
    img_torch = torch.permute(img_torch, (2, 0, 1)).unsqueeze(0).float()

    src_points = np.stack([mapx.ravel(), mapy.ravel()], axis=-1)
    dst_points = np.mgrid[0:w+1:cell_size, 0:w+1:cell_size].T.reshape(-1, 2)
    
    mat = umeyama(src_points, dst_points, True)[0:2]

    mat_torch = torch.from_numpy(mat).type(torch.FloatTensor).unsqueeze(0)   # [1,2,3]
    
    M_3x3: torch.Tensor = convert_affinematrix_to_homography(mat_torch)
    dst_norm_trans_src_norm: torch.Tensor = normalize_homography(M_3x3, (256, 256), (256, 256))
    mat_torch = _torch_inverse_cast(dst_norm_trans_src_norm)  # torch.Size([1, 3, 3])
     
    grid = F.affine_grid(mat_torch[:, :2, :].float(), img_torch.size(), align_corners=True)    # [1, 128, 128, 2]
    image = torch.from_numpy(cv2.cvtColor(image, cv2.COLOR_BGR2RGB)/255).type(torch.FloatTensor).permute(2,0,1).unsqueeze(0)
    target_image = F.grid_sample(image, grid, align_corners=True)    # [1, 3, 128, 128]
    target_image =  np.array(target_image.squeeze(0).permute(1, 2, 0))
    target_image = (cv2.cvtColor(target_image, cv2.COLOR_BGR2RGB))
    mat_torch_2X3 = mat_torch[:, :2, :].squeeze(0)
    
    # target_image = cv2.warpAffine(image, mat, (w, w))
    # mask = cv2.warpAffine(mask, mat, (w, w))
    
    return warped_image, target_image, mat_torch_2X3



def umeyama(src, dst, estimate_scale):
    """Estimate N-D similarity transformation with or without scaling.
    Parameters
    ----------
    src : (M, N) array
        Source coordinates.
    dst : (M, N) array
        Destination coordinates.
    estimate_scale : bool
        Whether to estimate scaling factor.
    Returns
    -------
    T : (N + 1, N + 1)
        The homogeneous similarity transformation matrix. The matrix contains
        NaN values only if the problem is not well-conditioned.
    References
    ----------
    .. [1] "Least-squares estimation of transformation parameters between two
            point patterns", Shinji Umeyama, PAMI 1991, :DOI:`10.1109/34.88573`
    """

    num = src.shape[0]
    dim = src.shape[1]

    # Compute mean of src and dst.
    src_mean = src.mean(axis=0)
    dst_mean = dst.mean(axis=0)

    # Subtract mean from src and dst.
    src_demean = src - src_mean
    dst_demean = dst - dst_mean

    # Eq. (38).
    A = dst_demean.T @ src_demean / num

    # Eq. (39).
    d = np.ones((dim,), dtype=np.double)
    if np.linalg.det(A) < 0:
        d[dim - 1] = -1

    T = np.eye(dim + 1, dtype=np.double)

    U, S, V = np.linalg.svd(A)

    # Eq. (40) and (43).
    rank = np.linalg.matrix_rank(A)
    if rank == 0:
        return np.nan * T
    elif rank == dim - 1:
        if np.linalg.det(U) * np.linalg.det(V) > 0:
            T[:dim, :dim] = U @ V
        else:
            s = d[dim - 1]
            d[dim - 1] = -1
            T[:dim, :dim] = U @ np.diag(d) @ V
            d[dim - 1] = s
    else:
        T[:dim, :dim] = U @ np.diag(d) @ V

    if estimate_scale:
        # Eq. (41) and (42).
        scale = 1.0 / src_demean.var(axis=0).sum() * (S @ d)
    else:
        scale = 1.0

    T[:dim, dim] = dst_mean - scale * (T[:dim, :dim] @ src_mean.T)
    T[:dim, :dim] *= scale

    return T




```

## Source file: src/bpNet/train.py

{
  "path": "src/bpNet/train.py",
  "blob_sha": "37ccd0d97e99643517ce39a0d78990d8c043c11c",
  "sha256": "7961f2da070e29e4b68c6f72975c32611ffd24354b89504b837d1ccfaccaef67",
  "lines": 226,
  "bytes": 9515,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 15,
    "blendshape": 1,
    "gum": 11,
    "deform": 1,
    "face": 7,
    "lip": 9
  },
  "url": "https://github.com/LiyCg/DFL/blob/3bd74bdb45d25bb029be2eff7fb1816d46e52141/src/bpNet/train.py",
  "function_signature_candidates": [
    {
      "line": 45,
      "text": "def parse():"
    },
    {
      "line": 83,
      "text": "def sample_data(loader):"
    },
    {
      "line": 88,
      "text": "def data_sampler(dataset, shuffle, distributed=False):"
    }
  ]
}

```text


# utils 
import argparse
import time
import torch
from torch import nn
import torchvision
import os
import sys 
import torch.optim as optim
import numpy as np
import matplotlib.pyplot as plt
import glob
# import wandb

# from utils.loggers import get_logger, print_options


# model 
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), './../../dataset/data_preparation/renderer/')))
from renderer import Renderer
from model_render import ModelPCA
from animation import AnimationBlendShape, AnimationPCA

# data
from face_dataset import FaceImgDataset_local
from torch.utils.data import DataLoader
from torch.utils import data

# network
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), './../')))
from network.model import BPNet, MLP

ngpu= 1 
# num_epochs = 8
lr = 5e-5 # 원래 이거

# Beta1 hyperparam for Adam optimizers
beta1 = 0.9
beta2 = 0.999
device = torch.device("cuda:0" if torch.cuda.is_available() else "cpu")
workers = 0
batch_size = 16

def parse():
    now = time.localtime()
    times = map(str, 
        [
            now.tm_year, 
            str(now.tm_mon).zfill(2), 
            str(now.tm_mday).zfill(2), 
            str(now.tm_hour).zfill(2), 
            str(now.tm_min).zfill(2), 
            str(now.tm_sec).zfill(2)
        ]
    )

    parser = argparse.ArgumentParser(description='PyTorch BPNetwork Training')

    parser.add_argument('-d','--root_dir', type=str, help='path of dataset')
    parser.add_argument('-a','--anim_dir', type=str, default="/todo")
    parser.add_argument('-ch','--char_name', type=str)
    parser.add_argument('-p','--project_name', type=str, default="YAT")

    parser.add_argument('-k','--save_ckpt', action='store_true')
    parser.add_argument('-b','--ckpt_BPNet_dir', type=str)

    parser.add_argument('-c','--ckpt_dir', type=str, default="../checkpoint/BPNet")
    parser.add_argument('-ck','--load_ckpt_name', type=str, help="resume train from checkpoint")
    parser.add_argument('-r','--load_iter', type=str, help="resume train from the last iter")

    parser.add_argument('-v', '--version', type=str, choices=['train_BPNet','concat_aug_color', 'train_WEM'])

    args = parser.parse_args()
    if args.load_ckpt_name:
        args.ckpt_BPNet_dir = args.load_ckpt_name
    else:
        args.ckpt_BPNet_dir = "../checkpoint/BPNet/{0}-ch_{1}".format("_".join(times), args.char_name)
        os.makedirs(args.ckpt_BPNet_dir,exist_ok=True)

    return args

def sample_data(loader):
    while True:
        for batch in loader:
            yield batch

def data_sampler(dataset, shuffle, distributed=False):
    if distributed:
        return data.distributed.DistributedSampler(dataset, shuffle=shuffle)
    if shuffle:
        return data.RandomSampler(dataset)
    else:
        return data.SequentialSampler(dataset)

if __name__ == '__main__':
    args = parse()
    # wandb.init(project="Local-Retargeting-PG", entity="kaist-vml")
    # wandb.run.name = args.version
    
    # ## CGF revision pca param version (with normalization)
    # uncomment for use
    if args.anim_dir.split('.')[-1] == 'pth' or args.anim_dir.split('.')[-1] == 'pt': # takes .pt for YAT
        if args.version == 'concat_aug_color':
            face_dataset = FaceImgDataset_local(args.root_dir, args.anim_dir, is_pca=True, is_test=False, aug_color=True, device=device)
        else:
            face_dataset = FaceImgDataset_local(args.root_dir, args.anim_dir, is_pca=True, is_test=False, device=device)
        
        if args.project_name == "YAT": ## if YAT
            if args.char_name == 'Emily':
                num_basis = 121
            elif args.char_name == 'Malcolm':
                num_basis = 32
            elif args.char_name == 'Morphy':
                num_basis = 46
            elif args.char_name == 'Victor':
                num_basis = 45
            elif args.char_name == 'VMan':
                num_basis = 101      
            bpNet = BPNet(num_blendshpshapes_or_pcas=num_basis).to(device)
        else: # if everything else (till YAT)
            if args.char_name == "metahuman" or args.char_name == 'metasihun':
                bpNet = BPNet(num_blendshpshapes_or_pcas=40).to(device)
            elif args.char_name == "mery_deformed":
                bpNet = BPNet(num_blendshpshapes_or_pcas=40).to(device)
            else: 
                bpNet = BPNet(num_blendshpshapes_or_pcas=20).to(device)
    else:
        if args.version == 'concat_aug_color':
            face_dataset = FaceImgDataset_local(args.root_dir, args.anim_dir, is_pca=False, is_test=False, aug_color=True, device=device)
        else:
            face_dataset = FaceImgDataset_local(args.root_dir, args.anim_dir, is_pca=False, is_test=False, device=device)
        # ## CGF revision bshp param version
        ## uncomment for use
        bpNet = BPNet(num_blendshpshapes_or_pcas=51).to(device)
    
    # dataloader = sample_data(DataLoader(face_dataset, batch_size=batch_size, sampler=data_sampler(face_dataset, shuffle=True), num_workers=8, drop_last=True)) # 초기화 전에 worker 프로세스가 메모리를 잡아버려서 오류가 발생
    dataloader = sample_data(DataLoader(face_dataset, batch_size=batch_size, sampler=data_sampler(face_dataset, shuffle=True), num_workers=workers, drop_last=True))
    
    if args.load_ckpt_name:
        args.ckpt_BPNet_dir = os.path.join(args.ckpt_dir, args.load_ckpt_name)
        bpNet.load_state_dict(torch.load(os.path.join(args.ckpt_BPNet_dir, f"BPNet_{args.load_iter}.pth")))
        print(f"load BPNet_{args.load_iter}.pth")
    else:
        ckpt_dir_eyel = os.path.join(args.ckpt_BPNet_dir, "eyel")
        ckpt_dir_eyer = os.path.join(args.ckpt_BPNet_dir, "eyer")
        ckpt_dir_lips = os.path.join(args.ckpt_BPNet_dir, "lips")
        os.makedirs(ckpt_dir_eyel, exist_ok=True)
        os.makedirs(ckpt_dir_eyer, exist_ok=True)
        os.makedirs(ckpt_dir_lips, exist_ok=True)
    
    optimizer = optim.Adam(list(bpNet.parameters()), lr=lr, betas=(beta1, beta2))

    criterion = nn.L1Loss()

    print("-----------train begin-----------")

    from_iter = 0
    if args.load_iter:
        from_iter = int(args.load_iter)

    start_time = time.time()
    times = []
    for i in range(from_iter,500000 + 1):

        idx_to_str = str(i).zfill(6)
        optimizer.zero_grad()
        # ===================forward=====================
        data_ = next(dataloader)
        eyel_aug_img = data_['s_eyel'].to(device) # warped input batch of images
        eyer_aug_img = data_['s_eyer'].to(device) # warped input batch of images 
        lips_aug_img = data_['s_lips'].to(device) # warped input batch of images

        eyel_aug_img = eyel_aug_img.permute(0,3,1,2).to(device, dtype=torch.float)/255.0
        eyer_aug_img = eyer_aug_img.permute(0,3,1,2).to(device, dtype=torch.float)/255.0
        lips_aug_img = lips_aug_img.permute(0,3,1,2).to(device, dtype=torch.float)/255.0

        concated_img = torch.cat((eyel_aug_img, eyer_aug_img, lips_aug_img), dim=1)
        output_parameter = bpNet(concated_img, version="concat")

        gt_parameter = data_['p'] # train data image parameters
        gt_parameter = (gt_parameter).to(device, dtype=torch.float)
        loss_param = criterion(gt_parameter, output_parameter)

        loss = loss_param 

        loss.backward()
        optimizer.step()

        if args.save_ckpt:
            if i % 50 == 0: # 500 to 250 
                data_ = next(dataloader)
                with torch.no_grad(): # validation
                    eyel_aug_img_val = data_['v_s_eyel'].to(device)
                    eyer_aug_img_val = data_['v_s_eyer'].to(device)
                    lips_aug_img_val = data_['v_s_lips'].to(device)
                    eyel_aug_img_val = eyel_aug_img_val.permute(0,3,1,2).to(device, dtype=torch.float)/255.0
                    eyer_aug_img_val = eyer_aug_img_val.permute(0,3,1,2).to(device, dtype=torch.float)/255.0
                    lips_aug_img_val = lips_aug_img_val.permute(0,3,1,2).to(device, dtype=torch.float)/255.0
                    concated_img = torch.cat((eyel_aug_img_val, eyer_aug_img_val, lips_aug_img_val), dim=1)
                    bpNet.eval()
                    output_parameter_val = bpNet(concated_img, version="concat")
                    gt_parameter_val = data_['v_p']
                    gt_parameter_val = (gt_parameter_val).to(device, dtype=torch.float)
                    val_loss = criterion(gt_parameter_val, output_parameter_val).detach()

                print("{0} \ttotal loss: {1:.4f} \tval loss: {2:.4f}".format(i, loss.item(), val_loss.item()))
                
                if i == 0:
                    torchvision.utils.save_image(
                             torch.cat([eyel_aug_img, eyer_aug_img, lips_aug_img], dim=2), 
                            "{0}/aug_{1}.jpg".format(args.ckpt_BPNet_dir, idx_to_str), 
                            normalize=True, 
                            value_range=(0,1)
                        )
                
            if i % 2500 == 0 and i != 0:
                time_elapsed = time.time() - start_time
                times.append(i)
                times.append(time_elapsed)
                torch.save(bpNet.state_dict(), "{0}/BPNet_{1}.pth".format(args.ckpt_BPNet_dir, idx_to_str))
                time_check_txt = "./time_check.txt"
                with open(time_check_txt, "w") as file:
                    for t in times:
                        file.write(str(t) + "\n") 



```

## Source file: src/reenactNet/train.py

{
  "path": "src/reenactNet/train.py",
  "blob_sha": "304089975ae64cc640727e03f8b2b15dc0315f80",
  "sha256": "84d9e0cb63f4e98df0b0aa319bf31ecb872f440dae16b410ae5289a83cfc9e52",
  "lines": 331,
  "bytes": 17178,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 39,
    "sss": 1,
    "gum": 14,
    "deform": 2,
    "rig": 1,
    "face": 3,
    "lip": 23
  },
  "url": "https://github.com/LiyCg/DFL/blob/3bd74bdb45d25bb029be2eff7fb1816d46e52141/src/reenactNet/train.py",
  "function_signature_candidates": [
    {
      "line": 77,
      "text": "def parse():"
    },
    {
      "line": 126,
      "text": "def sample_data(loader):"
    },
    {
      "line": 132,
      "text": "def data_sampler(dataset, shuffle, distributed=False):"
    },
    {
      "line": 140,
      "text": "def train(args):"
    }
  ]
}

```text

## PG2023 - reviving
## child
# python train.py -s "../../dataset/image/real/sihun_brow_train_refined" "../../dataset/image/child/local_child_refined" -v "ch_child" -ch "child" --save_ckpt
# python train.py -s "../../dataset/image/real/sihun_brow_train" "../../dataset/image/child/local_child" -v "ch_child" -ch "child" --save_ckpt
# python train.py -s "../../dataset/image/real/sihun_brow_train" "../../dataset/image/child/local_child" -d "2023_07_19_09_01_27-ch_child" -r "120000" -v "ch_child" -ch "child" --save_ckpt
    ## child resume train
# python train.py -s "../../dataset/image/real/sihun_brow_train_refined" "../../dataset/image/child/local_child_refined" -v "ch_child" -ch "child" -d "2023_06_21_11_43_19-ch_child" -r "030000" --save_ckpt 
## mery
# python train.py -s "../../dataset/image/real/sihun_brow_train_refined" "../../dataset/image/mery/local_mery_refined" -v "ch_mery" -ch "mery" --save_ckpt
# python train.py -s "../../dataset/image/real/sihun_brow_train" "../../dataset/image/mery/local_mery" -v "ch_mery" -ch "mery" --save_ckpt
# python train.py -s "../../dataset/image/real/sihun_brow_train" "../../dataset/image/mery/local_mery" -d "2023_07_19_09_05_31-ch_mery" -r "120000" -v "ch_mery" -ch "mery" --save_ckpt

## malcolm
# python train.py -s "../../dataset/image/real/sihun_brow_train_refined" "../../dataset/image/malcolm/local_malcolm_refined" -v "ch_malcom" -ch "malcolm" --save_ckpt
# python train.py -s "../../dataset/image/real/sihun_brow_train" "../../dataset/image/malcolm/local_malcolm" -v "ch_malcom" -ch "malcolm" --save_ckpt
# python train.py -s "../../dataset/image/real/sihun_brow_train" "../../dataset/image/malcolm/local_malcolm" -d "2023_07_19_09_21_22-ch_malcom" -r "120000" -v "ch_malcom" -ch "malcolm" --save_ckpt
      
## girl(refined newly created dataset)
# python train.py -s "../../dataset/image/real/sihun_brow_train_refined" "../../dataset/image/girl/local_girl_refined_v2" -v "ch_girl" -ch "girl" --save_ckpt
# python train.py -s "../../dataset/image/real/sihun_brow_train_refined" "../../dataset/image/girl/local_girl_refined_v3" -v "ch_girl" -ch "girl" --save_ckpt (more neutral expressions)
    ## girl
# python train.py -s "../../dataset/image/real/sihun_brow_train" "../../dataset/image/girl/local_girl" -v "ch_girl" -ch "girl" --save_ckpt 
    ## girl (resume train with iter for .pth file specified) 
# python train.py -s "../../dataset/image/real/sihun_brow_train_refined" "../../dataset/image/girl/local_girl_refined_v3" -v "ch_girl" -ch "girl" -d "2023_07_20_10_45_00-ch_girl" -r "040000" --save_ckpt 

## piers
# python train.py -s "../../dataset/image/real/sihun_brow_train_refined" "../../dataset/image/piers/local_piers_refined" -v "ch_piers" -ch "piers" --save_ckpt
# python train.py -s "../../dataset/image/real/sihun_brow_train_refined" "../../dataset/image/piers/local_piers_refined" -v "ch_piers" -ch "piers" --save_ckpt

## metahuman
# python train.py -s "../../dataset/image/real/sihun_brow_train_refined" "../../dataset/image/metahuman/local_metahuman_refined" -v "ch_metahuman" -ch "metahuman" --save_ckpt

## CGF revision
    ## train with more closed eye and neutral face sihun images 
# python train.py -s "../../dataset/image/real/sihun_train_2408_brow_neutClosedAdded" "../../dataset/image/child/local_child_WEM_19215" -v "ch_child" -ch "child" --save_ckpt
# python train.py -s "../../dataset/image/real/local_real_original_woRot" "../../dataset/image/child/local_child_WEM_19215" -v "ch_child" -ch "child" --save_ckpt
    ## train with extremely deformed mery for showing extreme overlapping patch cases
# python train.py -s "../../dataset/image/real/unused/sihun_brow_train_old" "../../dataset/image/mery_deformed/local_mery_deformed" -v "ch_mery" -ch "mery" --save_ckpt

## CGF 2nd revision
# python train.py -s "../../dataset/image/real/sihun_brow_train_refined" "../../dataset/image/metahuman/local_metahuman_refined" -v "ch_metahuman" -ch "metahuman" --save_ckpt


# utils
import argparse
import random
import time
import torch
from torch import nn
import torchvision
import os
# import wandb
import sys
import glob
import numpy as np
from utils import *
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), './../utils')))
# from loggers import get_logger, print_options

# data 
from face_dataset import FaceImgDataset
from torch.utils.data import DataLoader
from torch.utils import data
from torchvision import transforms

# network
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), './../')))
from network.model import Encoder, Decoder, Autoencoder

# optimization
import torch.optim as optim
from pytorch_msssim import ssim #, ms_ssim, SSIM, MS_SSIM

# from torchsummary import summary as summary
import torch.nn.functional as F

def parse():
    now = time.localtime()
    times = map(str, 
        [
            now.tm_year, 
            str(now.tm_mon).zfill(2), 
            str(now.tm_mday).zfill(2), 
            str(now.tm_hour).zfill(2), 
            str(now.tm_min).zfill(2), 
            str(now.tm_sec).zfill(2)
        ]
    )
    times = "_".join(times)

    parser = argparse.ArgumentParser(description='PyTorch new Training')

    parser.add_argument('-s','--src_data_dirs', metavar='N', type=str, nargs='+', help='multiple path of src dataset')
    parser.add_argument('-t','--trg_data_dir', type=str, default="/todo")
    parser.add_argument('-ch','--character_name', type=str, help='model for render scale')
    parser.add_argument('-b','--save_ckpt', action='store_true')
    parser.add_argument('-p','--project_name', type=str, default="YAT")

    parser.add_argument('-d','--load_ckpt_name', type=str, help="resume train from checkpoint")
    parser.add_argument('-r','--load_iter', type=str, help="resume train from the last iter")

    parser.add_argument('-v', '--version', type=str, \
                        choices=['ch_child','ch_girl','ch_piers','ch_malcolm','ch_mery','ch_metahuman',\
                            'c2c_m2g', 'c2c_g2m', 'c2c_mal2me', 'c2c_me2mal', 'ch_Victor', 'ch_Emily', 'ch_Morphy', 'ch_VMan', 'ch_Malcolm'], 
                        help='name for reenactNet version')
    parser.add_argument('-c','--ckpt_dir', type=str, default="../checkpoint/ReenactNet")
    parser.add_argument('-lr','--lr', default=1e-5, type=float)

    parser.add_argument('--lambda_idt',      type=float, default=10.0,   help='weight for idt loss')
    parser.add_argument('--lambda_SSIM',      type=float, default=1.0,   help='weight for SSIM loss')
    parser.add_argument('--iters',      type=float, default=200000,   help='iterations')
    args = parser.parse_args()

    if args.load_ckpt_name:
        args.ckpt_dir = os.path.join("../checkpoint/ReenactNet", args.load_ckpt_name) # 2022_12_03_00_56_52-L1_ssim_aug_color_random

    else:
        args.ckpt_dir = os.path.join(args.ckpt_dir, f"{times}-{args.version}")
        os.makedirs(args.ckpt_dir, exist_ok=True)
    
    # log_path         = args.ckpt_dir+'/reenactNet_{}.log'
    # train_log        = get_logger("{}".format(times), path=log_path)
    # train_log.info("\n"+print_options(args, parser))
    return args

def sample_data(loader):
    while True:
        for batch in loader:
            yield batch


def data_sampler(dataset, shuffle, distributed=False):
    if distributed:
        return data.distributed.DistributedSampler(dataset, shuffle=shuffle)
    if shuffle:
        return data.RandomSampler(dataset)
    else:
        return data.SequentialSampler(dataset)

def train(args):

    device = torch.device("cuda:0" if torch.cuda.is_available() else "cpu")
    if args.project_name == "YAT":
        ids = ['source', 'target']
    else:
        ids = ['real', args.character_name]

    ############################
    # experiment configuration #
    ############################
    is_test = False
    
    aug_random = False
    aug_affine_color = True
    aug_color = False
    
    batch_size = 16
    ############################
    
    # default
    datasets = [FaceImgDataset(src_data_dir, is_test=is_test, aug_random=aug_random, aug_affine_color=aug_affine_color, aug_color=aug_color, id=id, device=device) for (src_data_dir, id) in zip(args.src_data_dirs, ids) ]
    dataloaders = [sample_data(DataLoader(dataset, batch_size=batch_size, sampler=data_sampler(dataset, shuffle=True), num_workers=4, drop_last=True)) for dataset in datasets]

    ckpt_dir_sub = os.path.join(args.ckpt_dir, "train_result")
    ckpt_dirs = {} 
    net = {}

    src_param=[]
    trg_param=[]

    parts = ["eyel", "eyer", "lips"]

    for part in parts:
        encoder  = Encoder().to(device)
        decoders = [Decoder(part).to(device) for i in range(len(dataloaders))]

        if args.load_ckpt_name:
            encoder.load_state_dict(torch.load(os.path.join(ckpt_dir_sub, f"{part}/encoder_{args.load_iter}.pth")))
            for i, decoder in enumerate(decoders):
                decoder.load_state_dict(torch.load(os.path.join(ckpt_dir_sub, f"{part}/decoder_{ids[i]}_{args.load_iter}.pth")))
                print(f"load {part} decoder_{ids[i]}_{args.load_iter}.pth")

        autoencoders = [Autoencoder(encoder, decoder) for decoder in decoders]

        net[part] = {f"autoencoders_{part}": autoencoders, f"encoder_{part}": encoder, f"decoders_{part}": decoders}
        src_param+=list(autoencoders[0].parameters())
        trg_param+=list(autoencoders[1].parameters())
        ckpt_dirs[part] = os.path.join(ckpt_dir_sub, part)

    # eyel / eyer / lips 
    for ckpt_dir in ckpt_dirs.values():
        if not os.path.exists(ckpt_dir):
            os.makedirs(ckpt_dir, exist_ok=True)

    autoencoders_eyel = net["eyel"]["autoencoders_eyel"]
    autoencoders_eyer = net["eyer"]["autoencoders_eyer"]
    autoencoders_lips = net["lips"]["autoencoders_lips"]
    optimizers = [optim.Adam(net_param, lr=args.lr, betas=(0.9, 0.999)) for net_param in [src_param, trg_param]]

    criterion_idt = nn.L1Loss()
    criterion_ssim = ssim

    from_iter = 0
    if args.load_iter:
        from_iter = int(args.load_iter)
    
    start_time = time.time()
    times = []
    for i in range(from_iter, args.iters + 1):

        for dataloader, autoencoder_eyel, autoencoder_eyer, autoencoder_lips, optimizer, id \
            in zip(dataloaders, autoencoders_eyel, autoencoders_eyer, autoencoders_lips, optimizers, ids):

            optimizer.zero_grad()
            
            data_ = next(dataloader)

            gt_eyel_img, src_eyel_img = data_['gt_eyel'], data_['s_eyel']
            gt_eyer_img, src_eyer_img = data_['gt_eyer'], data_['s_eyer']
            gt_lips_img, src_lips_img = data_['gt_lips'], data_['s_lips']

            gt_eyel_img = gt_eyel_img.permute(0,3,1,2).to(device, dtype=torch.float)
            src_eyel_img = src_eyel_img.permute(0,3,1,2).to(device, dtype=torch.float)

            gt_eyer_img = gt_eyer_img.permute(0,3,1,2).to(device, dtype=torch.float)
            src_eyer_img = src_eyer_img.permute(0,3,1,2).to(device, dtype=torch.float)
            
            gt_lips_img = gt_lips_img.permute(0,3,1,2).to(device, dtype=torch.float)
            src_lips_img = src_lips_img.permute(0,3,1,2).to(device, dtype=torch.float)
        
            src_eyel_img_fake, _ = autoencoder_eyel(src_eyel_img, type="eyel")
            src_eyer_img_fake, _ = autoencoder_eyer(src_eyer_img, type="eyer")
            src_lips_img_fake, _ = autoencoder_lips(src_lips_img, type="lips")

            loss_eyel = criterion_idt(src_eyel_img_fake, gt_eyel_img)
            loss_eyer = criterion_idt(src_eyer_img_fake, gt_eyer_img)
            loss_lips = criterion_idt(src_lips_img_fake, gt_lips_img)

            loss_idt = loss_eyel + loss_eyer + loss_lips 

            loss_ssim_eyel = (1 - criterion_ssim(src_eyel_img_fake, gt_eyel_img, data_range=1, size_average=True))
            loss_ssim_eyer = (1 - criterion_ssim(src_eyer_img_fake, gt_eyer_img, data_range=1, size_average=True))
            loss_ssim_lips = (1 - criterion_ssim(src_lips_img_fake, gt_lips_img, data_range=1, size_average=True))
            
            loss_ssim = loss_ssim_eyel + loss_ssim_eyer + loss_ssim_lips

            loss = loss_idt*args.lambda_idt + loss_ssim*args.lambda_SSIM

            loss.backward()
            optimizer.step()

            if i % 500 == 0:
                idx_to_str = str(i).zfill(6)
                
                if id != args.character_name:
                    with torch.no_grad():
                        # import pdb; pdb.set_trace()
                        result_eyel, _ = autoencoders_eyel[1](gt_eyel_img,type="eyel")
                        torchvision.utils.save_image(
                            torch.cat([gt_eyel_img, result_eyel], dim=2), 
                            f"{ckpt_dirs['eyel']}/src_2_trg_{idx_to_str}.jpg", 
                            normalize=True, 
                            value_range=(0,1)
                        )
                        result_eyer, _ = autoencoders_eyer[1](gt_eyer_img,type="eyer")
                        torchvision.utils.save_image(
                            torch.cat([gt_eyer_img, result_eyer], dim=2), 
                            f"{ckpt_dirs['eyer']}/src_2_trg_{idx_to_str}.jpg", 
                            normalize=True, 
                            value_range=(0,1)
                        )
                        result_lips, _ = autoencoders_lips[1](gt_lips_img,type="lips")
                        torchvision.utils.save_image(
                            torch.cat([gt_lips_img, result_lips], dim=2), 
                            f"{ckpt_dirs['lips']}/src_2_trg_{idx_to_str}.jpg", 
                            normalize=True, 
                            value_range=(0,1)
                        )
                
                torchvision.utils.save_image(
                        torch.cat([src_eyel_img, gt_eyel_img, src_eyel_img_fake], dim=2), 
                        "{0}/result_{1}_{2}.jpg".format(ckpt_dirs['eyel'], id, idx_to_str), 
                        normalize=True, 
                        value_range=(0,1)
                    )
                torchvision.utils.save_image(
                        torch.cat([src_eyer_img, gt_eyer_img, src_eyer_img_fake], dim=2), 
                        "{0}/result_{1}_{2}.jpg".format(ckpt_dirs['eyer'], id, idx_to_str), 
                        normalize=True, 
                        value_range=(0,1)
                    )
                torchvision.utils.save_image(
                        torch.cat([src_lips_img, gt_lips_img, src_lips_img_fake], dim=2), 
                        "{0}/result_{1}_{2}.jpg".format(ckpt_dirs['lips'], id, idx_to_str), 
                        normalize=True, 
                        value_range=(0,1)
                    )
        
        if args.save_ckpt:
            idx_to_str = str(i).zfill(6)
            
            if i % 500 == 0:
                print("{0} \ttotal loss: {1:.4f} \tloss_eyel: {2:.4f} \tloss_eyer: {3:.4f} \tloss_lips: {4:.4f} \tloss_ssim_eyel: {5:.4f} \tloss_ssim_eyer: {6:.4f} \tloss_ssim_lips: {7:.4f}".format(i, loss.item(), \
                    loss_eyel.item(), loss_eyer.item(), loss_lips.item(), loss_ssim_eyel.item(), loss_ssim_eyer.item(), loss_ssim_lips.item()))

            if i % 10000 == 0 and i != 0:
                time_elapsed = time.time() - start_time
                times.append(i)
                times.append(time_elapsed)
                torch.save(net['eyel']['encoder_eyel'].state_dict(),  "{0}/encoder_{1}.pth".format(ckpt_dirs['eyel'], idx_to_str))
                torch.save(net['eyer']['encoder_eyer'].state_dict(),  "{0}/encoder_{1}.pth".format(ckpt_dirs['eyer'], idx_to_str))
                torch.save(net['lips']['encoder_lips'].state_dict(),  "{0}/encoder_{1}.pth".format(ckpt_dirs['lips'], idx_to_str))

                [torch.save(decoder_eyel.state_dict(), "{0}/decoder_{1}_{2}.pth".format(ckpt_dirs['eyel'], src_data_dir.split("/")[-2], idx_to_str)) \
                        for src_data_dir, decoder_eyel in zip(args.src_data_dirs, net['eyel']['decoders_eyel'])]
                [torch.save(decoder_eyer.state_dict(), "{0}/decoder_{1}_{2}.pth".format(ckpt_dirs['eyer'], src_data_dir.split("/")[-2], idx_to_str)) \
                    for src_data_dir, decoder_eyer in zip(args.src_data_dirs, net['eyer']['decoders_eyer'])]
                [torch.save(decoder_lips.state_dict(), "{0}/decoder_{1}_{2}.pth".format(ckpt_dirs['lips'], src_data_dir.split("/")[-2], idx_to_str)) \
                    for src_data_dir, decoder_lips in zip(args.src_data_dirs, net['lips']['decoders_lips'])]
        time_check_txt = "./time_check.txt"
        with open(time_check_txt, "w") as file:
            for t in times:
                file.write(str(t) + "\n")   
    

if __name__ == '__main__':

    args = parse()
    train(args)

    


```
