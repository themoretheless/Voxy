# Facial rendering source scan: DNA-Rendering/DNA-Rendering

Repository: https://github.com/DNA-Rendering/DNA-Rendering

Pinned source revision: a84cb31b934128fdfc1b324de3559909ffad39e2

Discovery topic: human rendering

Description: DNA-RENDERING: A Diverse Neural Actor Repository for High-Fidelity Human-centric Rendering

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: scripts/3DGS/cameras.py

{
  "path": "scripts/3DGS/cameras.py",
  "blob_sha": "ae01fac1f292d0fedcebbfd2ab09aa378434d5de",
  "sha256": "3ef44c0ea476228509b3576af86498d1bea93d035283675a28404b9c60ae0d8f",
  "lines": 78,
  "bytes": 2941,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 7
  },
  "url": "https://github.com/DNA-Rendering/DNA-Rendering/blob/a84cb31b934128fdfc1b324de3559909ffad39e2/scripts/3DGS/cameras.py",
  "function_signature_candidates": [
    {
      "line": 18,
      "text": "def __init__(self, colmap_id, R, T, FoVx, FoVy, image, gt_alpha_mask,"
    },
    {
      "line": 67,
      "text": "def __init__(self, width, height, fovy, fovx, znear, zfar, world_view_transform, full_proj_transform):"
    }
  ]
}

```text

#
# Copyright (C) 2023, Inria
# GRAPHDECO research group, https://team.inria.fr/graphdeco
# All rights reserved.
#
# This software is free for non-commercial, research and evaluation use 
# under the terms of the LICENSE.md file.
#
# For inquiries contact  george.drettakis@inria.fr
#

import torch
from torch import nn
import numpy as np
from utils.graphics_utils import getWorld2View2, getProjectionMatrix, getProjectionMatrix2

class Camera(nn.Module):
    def __init__(self, colmap_id, R, T, FoVx, FoVy, image, gt_alpha_mask,
                 image_name, uid,
                 K=None, use_K=False,
                 trans=np.array([0.0, 0.0, 0.0]), scale=1.0, data_device = "cuda"
                 ):
        super(Camera, self).__init__()

        self.uid = uid
        self.colmap_id = colmap_id
        self.R = R
        self.T = T
        self.FoVx = FoVx
        self.FoVy = FoVy
        self.K = K  
        self.use_K = use_K
        self.image_name = image_name

        try:
            self.data_device = torch.device(data_device)
        except Exception as e:
            print(e)
            print(f"[Warning] Custom device {data_device} failed, fallback to default cuda device" )
            self.data_device = torch.device("cuda")

        self.original_image = image.clamp(0.0, 1.0).to(self.data_device)
        self.image_width = self.original_image.shape[2]
        self.image_height = self.original_image.shape[1]

        if gt_alpha_mask is not None:
            self.original_image *= gt_alpha_mask.to(self.data_device)
        else:
            self.original_image *= torch.ones((1, self.image_height, self.image_width), device=self.data_device)

        self.zfar = 100.0
        self.znear = 0.01

        self.trans = trans
        self.scale = scale

        self.world_view_transform = torch.tensor(getWorld2View2(R, T, trans, scale)).transpose(0, 1).cuda()
        if not self.use_K:
            self.projection_matrix = getProjectionMatrix(znear=self.znear, zfar=self.zfar, fovX=self.FoVx, fovY=self.FoVy).transpose(0,1).cuda()
        else:
            self.projection_matrix = getProjectionMatrix2(self.znear, self.zfar, self.K, self.image_width, self.image_height).transpose(0,1).cuda()

        self.full_proj_transform = (self.world_view_transform.unsqueeze(0).bmm(self.projection_matrix.unsqueeze(0))).squeeze(0)
        self.camera_center = self.world_view_transform.inverse()[3, :3]

class MiniCam:
    def __init__(self, width, height, fovy, fovx, znear, zfar, world_view_transform, full_proj_transform):
        self.image_width = width
        self.image_height = height    
        self.FoVy = fovy
        self.FoVx = fovx
        self.znear = znear
        self.zfar = zfar
        self.world_view_transform = world_view_transform
        self.full_proj_transform = full_proj_transform
        view_inv = torch.inverse(self.world_view_transform)
        self.camera_center = view_inv[3][:3]



```

## Source file: scripts/3DGS/__init__.py

{
  "path": "scripts/3DGS/__init__.py",
  "blob_sha": "166936f77607c42d771b3f3ed26bc6cc68d8f1df",
  "sha256": "b8463eb8ffd5e71804429935b773253680864738eec18d994dbe03ca240c602f",
  "lines": 96,
  "bytes": 4284,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 1,
    "rig": 2
  },
  "url": "https://github.com/DNA-Rendering/DNA-Rendering/blob/a84cb31b934128fdfc1b324de3559909ffad39e2/scripts/3DGS/__init__.py",
  "function_signature_candidates": [
    {
      "line": 25,
      "text": "def __init__(self, args : ModelParams, gaussians : GaussianModel, load_iteration=None, shuffle=True, resolution_scales=[1.0]):"
    },
    {
      "line": 88,
      "text": "def save(self, iteration):"
    },
    {
      "line": 92,
      "text": "def getTrainCameras(self, scale=1.0):"
    },
    {
      "line": 95,
      "text": "def getTestCameras(self, scale=1.0):"
    }
  ]
}

```text

#
# Copyright (C) 2023, Inria
# GRAPHDECO research group, https://team.inria.fr/graphdeco
# All rights reserved.
#
# This software is free for non-commercial, research and evaluation use 
# under the terms of the LICENSE.md file.
#
# For inquiries contact  george.drettakis@inria.fr
#

import os
import random
import json
from utils.system_utils import searchForMaxIteration
from scene.dataset_readers import sceneLoadTypeCallbacks
from scene.gaussian_model import GaussianModel
from arguments import ModelParams
from utils.camera_utils import cameraList_from_camInfos, camera_to_JSON

class Scene:

    gaussians : GaussianModel

    def __init__(self, args : ModelParams, gaussians : GaussianModel, load_iteration=None, shuffle=True, resolution_scales=[1.0]):
        """b
        :param path: Path to colmap scene main folder.
        """
        self.model_path = args.model_path
        self.loaded_iter = None
        self.gaussians = gaussians

        if load_iteration:
            if load_iteration == -1:
                self.loaded_iter = searchForMaxIteration(os.path.join(self.model_path, "point_cloud"))
            else:
                self.loaded_iter = load_iteration
            print("Loading trained model at iteration {}".format(self.loaded_iter))

        self.train_cameras = {}
        self.test_cameras = {}

        if os.path.exists(os.path.join(args.source_path, "sparse")):
            scene_info = sceneLoadTypeCallbacks["Colmap"](args.source_path, args.images, args.eval)
        elif os.path.exists(os.path.join(args.source_path, "transforms_train.json")):
            print("Found transforms_train.json file, assuming Blender data set!")
            scene_info = sceneLoadTypeCallbacks["Blender"](args.source_path, args.white_background, args.eval)
        elif args.source_path.endswith(".smc"):
            print("Found smc file, assuming DNA-Rendering data set!")
            scene_info = sceneLoadTypeCallbacks["DNA-Rendeing"](args.source_path, args.white_background, args.eval)
        else:
            assert False, "Could not recognize scene type!"

        if not self.loaded_iter:
            with open(scene_info.ply_path, 'rb') as src_file, open(os.path.join(self.model_path, "input.ply") , 'wb') as dest_file:
                dest_file.write(src_file.read())
            json_cams = []
            camlist = []
            if scene_info.test_cameras:
                camlist.extend(scene_info.test_cameras)
            if scene_info.train_cameras:
                camlist.extend(scene_info.train_cameras)
            for id, cam in enumerate(camlist):
                json_cams.append(camera_to_JSON(id, cam))
            with open(os.path.join(self.model_path, "cameras.json"), 'w') as file:
                json.dump(json_cams, file)

        if shuffle:
            random.shuffle(scene_info.train_cameras)  # Multi-res consistent random shuffling
            random.shuffle(scene_info.test_cameras)  # Multi-res consistent random shuffling

        self.cameras_extent = scene_info.nerf_normalization["radius"]

        for resolution_scale in resolution_scales:
            print("Loading Training Cameras")
            self.train_cameras[resolution_scale] = cameraList_from_camInfos(scene_info.train_cameras, resolution_scale, args)
            print("Loading Test Cameras")
            self.test_cameras[resolution_scale] = cameraList_from_camInfos(scene_info.test_cameras, resolution_scale, args)

        if self.loaded_iter:
            self.gaussians.load_ply(os.path.join(self.model_path,
                                                           "point_cloud",
                                                           "iteration_" + str(self.loaded_iter),
                                                           "point_cloud.ply"))
        else:
            self.gaussians.create_from_pcd(scene_info.point_cloud, self.cameras_extent)

    def save(self, iteration):
        point_cloud_path = os.path.join(self.model_path, "point_cloud/iteration_{}".format(iteration))
        self.gaussians.save_ply(os.path.join(point_cloud_path, "point_cloud.ply"))

    def getTrainCameras(self, scale=1.0):
        return self.train_cameras[scale]

    def getTestCameras(self, scale=1.0):
        return self.test_cameras[scale]

```

## Source file: scripts/3DGS/SMCReader.py

{
  "path": "scripts/3DGS/SMCReader.py",
  "blob_sha": "fea0572470927875757a5f6c11ff548456c19d39",
  "sha256": "9c9b1629dbfc66407fe05c2458d5481eac75b7d6b81f853a38242c1a1f0dd2f6",
  "lines": 405,
  "bytes": 15866,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/DNA-Rendering/DNA-Rendering/blob/a84cb31b934128fdfc1b324de3559909ffad39e2/scripts/3DGS/SMCReader.py",
  "function_signature_candidates": [
    {
      "line": 8,
      "text": "def __init__(self, file_path):"
    },
    {
      "line": 58,
      "text": "def get_available_keys(self):"
    },
    {
      "line": 61,
      "text": "def get_actor_info(self):"
    },
    {
      "line": 64,
      "text": "def get_Camera_12mp_info(self):"
    },
    {
      "line": 67,
      "text": "def get_Camera_5mp_info(self):"
    },
    {
      "line": 70,
      "text": "def get_Kinect_info(self):"
    },
    {
      "line": 74,
      "text": "def get_Calibration_all(self):"
    },
    {
      "line": 104,
      "text": "def get_Calibration(self, Camera_id):"
    },
    {
      "line": 125,
      "text": "def get_Kinect_Calibration_all(self):"
    },
    {
      "line": 158,
      "text": "def get_kinect_Calibration(self, Camera_id):"
    },
    {
      "line": 182,
      "text": "def __read_color_from_bytes__(self, color_array):"
    },
    {
      "line": 186,
      "text": "def get_mask(self, Camera_id, Frame_id=None,disable_tqdm=True):"
    },
    {
      "line": 228,
      "text": "def get_img(self, Camera_group, Camera_id, Image_type,Frame_id=None,disable_tqdm=True):"
    },
    {
      "line": 285,
      "text": "def get_Keypoints2d(self, Camera_id, Frame_id=None):"
    },
    {
      "line": 322,
      "text": "def get_Keypoints3d(self, Frame_id=None):"
    },
    {
      "line": 350,
      "text": "def get_SMPLx(self, Frame_id=None):"
    },
    {
      "line": 397,
      "text": "def release(self):"
    }
  ]
}

```text

import h5py
import cv2
import numpy as np 
from tqdm import tqdm 

class SMCReader:

    def __init__(self, file_path):
        """Read SenseMocapFile endswith ".smc".

        Args:
            file_path (str):
                Path to an SMC file.
            body_model (nn.Module or dict):
                Only needed for SMPL transformation to device frame
                if nn.Module: a body_model instance
                if dict: a body_model config
        """
        self.smc = h5py.File(file_path, 'r')
        self.__calibration_dict__ = None
        self.__kinect_calib_dict__ = None 
        self.__available_keys__ = list(self.smc.keys())
        
        self.actor_info = None 
        if hasattr(self.smc, 'attrs') and len(self.smc.attrs.keys()) > 0:
            self.actor_info = dict(
                id=self.smc.attrs['actor_id'],
                perf_id=self.smc.attrs['performance_id'],
                age=self.smc.attrs['age'],
                gender=self.smc.attrs['gender'],
                height=self.smc.attrs['height'],
                weight=self.smc.attrs['weight'],
                # ethnicity=self.smc.attrs['ethnicity'],
            )

        self.Camera_5mp_info = None 
        if 'Camera_5mp' in self.smc:
            self.Camera_5mp_info = dict(
                num_device=self.smc['Camera_5mp'].attrs['num_device'],
                num_frame=self.smc['Camera_5mp'].attrs['num_frame'],
                resolution=self.smc['Camera_5mp'].attrs['resolution'],
            )
        self.Camera_12mp_info = None 
        if 'Camera_12mp' in self.smc:
            self.Camera_12mp_info = dict(
                num_device=self.smc['Camera_12mp'].attrs['num_device'],
                num_frame=self.smc['Camera_12mp'].attrs['num_frame'],
                resolution=self.smc['Camera_12mp'].attrs['resolution'],
            )
        self.Kinect_info = None
        if 'Kinect' in self.smc:
            self.Kinect_info=dict(
                num_device=self.smc['Kinect'].attrs['num_device'],
                num_frame=self.smc['Kinect'].attrs['num_frame'],
                resolution=self.smc['Kinect'].attrs['resolution'],
            )

    def get_available_keys(self):
        return self.__available_keys__ 

    def get_actor_info(self):
        return self.actor_info
    
    def get_Camera_12mp_info(self):
        return self.Camera_12mp_info

    def get_Camera_5mp_info(self):
        return self.Camera_5mp_info
    
    def get_Kinect_info(self):
        return self.Kinect_info
    
    ### RGB Camera Calibration
    def get_Calibration_all(self):
        """Get calibration matrix of all cameras and save it in self
        
        Args:
            None

        Returns:
            Dictionary of calibration matrixs of all matrixs.
              dict( 
                Camera_Parameter: Camera_id : Matrix_type : value
              )
            Notice:
                Camera_id(str) in {'Camera_5mp': '0'~'47',  'Camera_12mp':'48'~'60'}
                Matrix_type in ['D', 'K', 'RT', 'Color_Calibration'] 
        """  
        if not 'Camera_Parameter' in self.smc:
            print("=== no key: Camera_Parameter.\nplease check available keys!")
            return None  

        if self.__calibration_dict__ is not None:
            return self.__calibration_dict__

        self.__calibration_dict__ = dict()
        for ci in self.smc['Camera_Parameter'].keys():
            self.__calibration_dict__.setdefault(ci,dict())
            for mt in ['D', 'K', 'RT', 'Color_Calibration'] :
                self.__calibration_dict__[ci][mt] = \
                    self.smc['Camera_Parameter'][ci][mt][()]
        return self.__calibration_dict__

    def get_Calibration(self, Camera_id):
        """Get calibration matrixs of a certain camera by its type and id 

        Args:
            Camera_id (int/str of a number):
                Camera_id(str) in {'Camera_5mp': '0'~'47',  
                    'Camera_12mp':'48'~'60'}
        Returns:
            Dictionary of calibration matrixs.
                ['D', 'K', 'RT', 'Color_Calibration'] 
        """
        if not 'Camera_Parameter' in self.smc:
            print("=== no key: Camera_Parameter.\nplease check available keys!")
            return None  

        rs = dict()
        for k in ['D', 'K', 'RT', 'Color_Calibration'] :
            rs[k] = self.smc['Camera_Parameter'][f'{int(Camera_id):02d}'][k][()]
        return rs

    ### Kinect Camera Calibration
    def get_Kinect_Calibration_all(self):
        """Get calibration matrix of all kinect cameras and save it in self
        
        Args:
            None

        Returns:
            Dictionary of calibration matrixs of all matrixs.
              dict( 
                Camera_group: Camera_id : Matrix_type : value
              )
            Notice:
                Camera_group(str) in ['Kinect']
                Camera_id(str) in {'Kinect': '0'~'7'}
                Matrix_type in ['D', 'K', 'RT'] 
        """  
        if not 'Calibration' in self.smc:
            print("=== no key: Calibration.\nplease check available keys!")
            return None  

        if self.__kinect_calib_dict__ is not None:
            return self.__kinect_calib_dict__

        self.__kinect_calib_dict__ = dict()
        for cg in ['Kinect']:
            self.__kinect_calib_dict__.setdefault(cg,dict())
            for ci in self.smc['Calibration'][cg].keys():
                self.__kinect_calib_dict__[cg].setdefault(ci,dict())
                for mt in ['D', 'K', 'RT'] :
                    self.__kinect_calib_dict__[cg][ci][mt] = \
                        self.smc['Calibration'][cg][ci][mt][()]
        return self.__kinect_calib_dict__

    def get_kinect_Calibration(self, Camera_id):
        """Get calibration matrixs of a certain kinect camera by its type and id 

        Args:
            Camera_group (str):
                Camera_group in ['Kinect'].
            Camera_id (int/str of a number):
                CameraID(str) in {'Kinect': '0'~'7'}
        Returns:
            Dictionary of calibration matrixs.
                ['D', 'K', 'RT'] 
        """  
        if not 'Calibration' in self.smc:
            print("=== no key: Calibration.\nplease check available keys!")
            return None 

        Camera_id = f'{int(Camera_id):02d}'
        assert(Camera_id in self.smc['Calibration']["Kinect"].keys())
        rs = dict()
        for k in ['D', 'K', 'RT']:
            rs[k] = self.smc['Calibration']["Kinect"][Camera_id][k][()]
        return rs

    ### RGB image
    def __read_color_from_bytes__(self, color_array):
        """Decode an RGB image from an encoded byte array."""
        return cv2.imdecode(color_array, cv2.IMREAD_COLOR)

    def get_mask(self, Camera_id, Frame_id=None,disable_tqdm=True):
        """Get mask from Camera_id, Frame_id

        Args:
            Camera_id (int/str of a number):
                Camera_id (str) in 
                    {'Camera_5mp': '0'~'47',  
                    'Camera_12mp':'48'~'60',
                    'Kinect': '0'~'7'}
            Frame_id a.(int/str of a number): '0' ~ 'num_frame'
                     b.list of numbers (int/str)
                     c.None: get batch of all imgs in order of time sequence 
        Returns:
            a single img :
              'color': HWC in bgr (uint8)
              'mask' : HW (uint8)
              'depth': HW (uint16)
        """ 
        if not 'Mask' in self.smc:
            print("=== no key: Mask.\nplease check available keys!")
            return None  

        Camera_id = str(Camera_id)

        assert(isinstance(Frame_id,(list,int, str, type(None))))
        if isinstance(Frame_id, (str,int)):
            Frame_id = str(Frame_id)
            assert(Frame_id in self.smc['Mask'][Camera_id]['mask'].keys())
            img_byte = self.smc['Mask'][Camera_id]['mask'][Frame_id][()]
            img_color = self.__read_color_from_bytes__(img_byte)
            img_color = np.max(img_color,2)
            return img_color           
        else:
            if Frame_id is None:
                Frame_id_list =sorted([int(l) for l in self.smc['Mask'][Camera_id]['mask'].keys()])
            elif isinstance(Frame_id, list):
                Frame_id_list = Frame_id
            rs = []
            for fi in  tqdm.tqdm(Frame_id_list, disable=disable_tqdm):
                rs.append(self.get_mask(Camera_id,fi))
            return np.stack(rs,axis=0)

    def get_img(self, Camera_group, Camera_id, Image_type,Frame_id=None,disable_tqdm=True):
        """Get image its Camera_group, Camera_id, Image_type and Frame_id

        Args:
            Camera_group (str):
                Camera_group in ['Camera_12mp', 'Camera_5mp','Kinect'].
            Camera_id (int/str of a number):
                CameraID (str) in 
                    {'Camera_5mp': '0'~'47',  
                    'Camera_12mp':'48'~'60',
                    'Kinect': '0'~'7'}
            Image_type(str) in 
                    {'Camera_5mp': ['color'],  
                    'Camera_12mp': ['color'],
                    'Kinect': ['depth', 'mask']}
            Frame_id a.(int/str of a number): '0' ~ 'num_frame'('149') 
                     b.list of numbers (int/str)
                     c.None: get batch of all imgs in order of time sequence 
        Returns:
            a single img :
              'color': HWC in bgr (uint8)
              'mask' : HW (uint8)
              'depth': HW (uint16)
        """ 
        if not Camera_group in self.smc:
            print("=== no key: %s.\nplease check available keys!" % Camera_group)
            return None

        assert(Camera_group in ['Camera_12mp', 'Camera_5mp','Kinect'])
        Camera_id = str(Camera_id)
        assert(Camera_id in self.smc[Camera_group].keys())
        assert(Image_type in self.smc[Camera_group][Camera_id].keys())
        assert(isinstance(Frame_id,(list,int, str, type(None))))
        if isinstance(Frame_id, (str,int)):
            Frame_id = str(Frame_id)
            assert(Frame_id in self.smc[Camera_group][Camera_id][Image_type].keys())
            if Image_type in ['color']:
                img_byte = self.smc[Camera_group][Camera_id][Image_type][Frame_id][()]
                img_color = self.__read_color_from_bytes__(img_byte)
            if Image_type == 'mask':
                img_byte = self.smc[Camera_group][Camera_id][Image_type][Frame_id][()]
                img_color = self.__read_color_from_bytes__(img_byte)
                img_color = np.max(img_color,2)
            if Image_type == 'depth':
                img_color = self.smc[Camera_group][Camera_id][Image_type][Frame_id][()]
            return img_color           
        else:
            if Frame_id is None:
                Frame_id_list =sorted([int(l) for l in self.smc[Camera_group][Camera_id][Image_type].keys()])
            elif isinstance(Frame_id, list):
                Frame_id_list = Frame_id
            rs = []
            for fi in tqdm(Frame_id_list, disable=disable_tqdm):
                rs.append(self.get_img(Camera_group, Camera_id, Image_type,fi))
            return np.stack(rs,axis=0)
    
    ###Keypoints2d
    def get_Keypoints2d(self, Camera_id, Frame_id=None):
        """Get keypoint2D by its Camera_group, Camera_id and Frame_id

        Args:
            Camera_id (int/str of a number):
                CameraID (str) in 
                    {'Camera_5mp': '0'~'47',  
                    'Camera_12mp':'48'~'60',}
            Frame_id a.(int/str of a number): '0' ~ 'num_frame-1'('149') 
                     b.list of numbers (int/str)
                     c.None: get batch of all imgs in order of time sequence 
        Returns:
            a single img :
              'color': HWC in bgr (uint8)
              'mask' : HW (uint8)
              'depth': HW (uint16)
        """ 
        if not 'Keypoints_2D' in self.smc:
            print("=== no key: Keypoints_2D.\nplease check available keys!")
            return None 

        Camera_id = f'{int(Camera_id):02d}'
        assert(isinstance(Frame_id,(list,int, str, type(None))))
        if isinstance(Frame_id, (str,int)):
            Frame_id = int(Frame_id)
            return self.smc['Keypoints_2D'][Camera_id][()][Frame_id,:]
        else:
            if Frame_id is None:
                return self.smc['Keypoints_2D'][Camera_id][()]
            elif isinstance(Frame_id, list):
                Frame_id_list = Frame_id
            rs = []
            for fi in  tqdm.tqdm(Frame_id_list):
                rs.append(self.get_Keypoints2d(Camera_id,fi))
            return np.stack(rs,axis=0)

    ###Keypoints3d
    def get_Keypoints3d(self, Frame_id=None):
        """Get keypoint3D Frame_id, TODO coordinate

        Args:
            Frame_id a.(int/str of a number): '0' ~ 'num_frame-1'('149') 
                     b.list of numbers (int/str)
                     c.None: get batch of all imgs in order of time sequence 
        Returns:
            Keypoints3d tensor: np.ndarray of shape ([N], ,3)
        """ 
        if not 'Keypoints_3D' in self.smc:
            print("=== no key: Keypoints_3D.\nplease check available keys!")
            return None 

        if isinstance(Frame_id, (str,int)):
            Frame_id = int(Frame_id)
            return self.smc['Keypoints_3D']["keypoints3d"][Frame_id,:]
        else:
            if Frame_id is None:
                return self.smc['Keypoints_3D']["keypoints3d"]
            elif isinstance(Frame_id, list):
                Frame_id_list = Frame_id
            rs = []
            for fi in tqdm.tqdm(Frame_id_list):
                rs.append(self.get_Keypoints3d(fi))
            return np.stack(rs,axis=0)

    ###SMPLx
    def get_SMPLx(self, Frame_id=None):
        """Get SMPL (world coordinate) computed by mocap processing pipeline.

        Args:
            Frame_id (int, list or None, optional):
                int: frame id of one selected frame
                list: a list of frame id
                None: all frames will be returned
                Defaults to None.

        Returns:
            dict:
                'global_orient': np.ndarray of shape (N, 3)
                'body_pose': np.ndarray of shape (N, 21, 3)
                'transl': np.ndarray of shape (N, 3)
                'betas': np.ndarray of shape (1, 10)
        """
        if not 'SMPLx' in self.smc:
            print("=== no key: SMPLx.\nplease check available keys!")
            return None 

        t_frame = self.smc['SMPLx']['betas'][()].shape[0]
        print("=== t_frame", self.smc['SMPLx']['betas'][()].shape, flush=True)

        key_arr = ['betas', 'expression', 'fullpose', 'transl']
        for k in key_arr:
            print("=== smplx %s" % k, self.smc['SMPLx'][k][()].shape[0], flush=True)
        
        if Frame_id is None:
            frame_list = range(t_frame)
        elif isinstance(Frame_id, list):
            frame_list = [int(fi) for fi in Frame_id]
        elif isinstance(Frame_id, (int,str)):
            Frame_id = int(Frame_id)
            assert Frame_id < t_frame,\
                f'Invalid frame_index {Frame_id}'
            frame_list = Frame_id
        else:
            raise TypeError('frame_id should be int, list or None.')

        smpl_dict = {}
        for key in ['betas', 'expression', 'fullpose', 'transl']:
            smpl_dict[key] = self.smc['SMPLx'][key][()][frame_list, ...]
        smpl_dict['scale'] = self.smc['SMPLx']['scale'][()]

        return smpl_dict

    def release(self):
        self.smc = None 
        self.__calibration_dict__ = None
        self.__kinect_calib_dict__ = None
        self.__available_keys__ = None
        self.actor_info = None 
        self.Camera_5mp_info = None
        self.Camera_12mp_info = None 
        self.Kinect_info = None


```

## Source file: scripts/3DGS/dataset_readers.py

{
  "path": "scripts/3DGS/dataset_readers.py",
  "blob_sha": "a943662941542582b7a65cff07721c8602067134",
  "sha256": "c37dd7bc5697237e20b6d5680a14fc57b9da49ab555d9c5562c7ec386f3add16",
  "lines": 397,
  "bytes": 15731,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 2,
    "jaw": 1,
    "rig": 3,
    "face": 1
  },
  "url": "https://github.com/DNA-Rendering/DNA-Rendering/blob/a84cb31b934128fdfc1b324de3559909ffad39e2/scripts/3DGS/dataset_readers.py",
  "function_signature_candidates": [
    {
      "line": 55,
      "text": "def getNerfppNorm(cam_info):"
    },
    {
      "line": 56,
      "text": "def get_center_and_diag(cam_centers):"
    },
    {
      "line": 78,
      "text": "def readColmapCameras(cam_extrinsics, cam_intrinsics, images_folder):"
    },
    {
      "line": 117,
      "text": "def fetchPly(path):"
    },
    {
      "line": 125,
      "text": "def storePly(path, xyz, rgb):"
    },
    {
      "line": 142,
      "text": "def readColmapSceneInfo(path, images, eval, llffhold=8):"
    },
    {
      "line": 189,
      "text": "def readCamerasFromTransforms(path, transformsfile, white_background, extension=\".png\"):"
    },
    {
      "line": 231,
      "text": "def readNerfSyntheticInfo(path, white_background, eval, extension=\".png\"):"
    },
    {
      "line": 267,
      "text": "def readCamerasDNARendering(path, info_dict, white_background, image_scaling=0.5, return_smplx=False):"
    },
    {
      "line": 355,
      "text": "def readDNARenderingInfo(path, white_background, eval):"
    }
  ]
}

```text

#
# Copyright (C) 2023, Inria
# GRAPHDECO research group, https://team.inria.fr/graphdeco
# All rights reserved.
#
# This software is free for non-commercial, research and evaluation use 
# under the terms of the LICENSE.md file.
#
# For inquiries contact  george.drettakis@inria.fr
#

import os
import sys
from PIL import Image
from typing import NamedTuple, Optional
from scene.colmap_loader import read_extrinsics_text, read_intrinsics_text, qvec2rotmat, \
    read_extrinsics_binary, read_intrinsics_binary, read_points3D_binary, read_points3D_text
from utils.graphics_utils import getWorld2View2, focal2fov, fov2focal
import numpy as np
import json
from pathlib import Path
from plyfile import PlyData, PlyElement
from utils.sh_utils import SH2RGB
from scene.gaussian_model import BasicPointCloud

######### for DNA-Rendering
import torch 
import cv2 
from smplx.body_models import SMPLX
dir_path = os.path.dirname(os.path.realpath(__file__))
sys.path.append(dir_path)
from SMCReader import SMCReader

class CameraInfo(NamedTuple):
    uid: int
    R: np.array
    T: np.array
    FovY: np.array
    FovX: np.array
    image: np.array
    image_path: str
    image_name: str
    width: int
    height: int
    K: Optional[np.array] = None  ### for DNA-Rendering
    use_K: Optional[bool] = False  ### for DNA-Rendering

class SceneInfo(NamedTuple):
    point_cloud: BasicPointCloud
    train_cameras: list
    test_cameras: list
    nerf_normalization: dict
    ply_path: str

def getNerfppNorm(cam_info):
    def get_center_and_diag(cam_centers):
        cam_centers = np.hstack(cam_centers)
        avg_cam_center = np.mean(cam_centers, axis=1, keepdims=True)
        center = avg_cam_center
        dist = np.linalg.norm(cam_centers - center, axis=0, keepdims=True)
        diagonal = np.max(dist)
        return center.flatten(), diagonal

    cam_centers = []

    for cam in cam_info:
        W2C = getWorld2View2(cam.R, cam.T)
        C2W = np.linalg.inv(W2C)
        cam_centers.append(C2W[:3, 3:4])

    center, diagonal = get_center_and_diag(cam_centers)
    radius = diagonal * 1.1

    translate = -center

    return {"translate": translate, "radius": radius}

def readColmapCameras(cam_extrinsics, cam_intrinsics, images_folder):
    cam_infos = []
    for idx, key in enumerate(cam_extrinsics):
        sys.stdout.write('\r')
        # the exact output you're looking for:
        sys.stdout.write("Reading camera {}/{}".format(idx+1, len(cam_extrinsics)))
        sys.stdout.flush()

        extr = cam_extrinsics[key]
        intr = cam_intrinsics[extr.camera_id]
        height = intr.height
        width = intr.width

        uid = intr.id
        R = np.transpose(qvec2rotmat(extr.qvec))
        T = np.array(extr.tvec)

        if intr.model=="SIMPLE_PINHOLE":
            focal_length_x = intr.params[0]
            FovY = focal2fov(focal_length_x, height)
            FovX = focal2fov(focal_length_x, width)
        elif intr.model=="PINHOLE":
            focal_length_x = intr.params[0]
            focal_length_y = intr.params[1]
            FovY = focal2fov(focal_length_y, height)
            FovX = focal2fov(focal_length_x, width)
        else:
            assert False, "Colmap camera model not handled: only undistorted datasets (PINHOLE or SIMPLE_PINHOLE cameras) supported!"

        image_path = os.path.join(images_folder, os.path.basename(extr.name))
        image_name = os.path.basename(image_path).split(".")[0]
        image = Image.open(image_path)

        cam_info = CameraInfo(uid=uid, R=R, T=T, FovY=FovY, FovX=FovX, image=image,
                              image_path=image_path, image_name=image_name, width=width, height=height)
        cam_infos.append(cam_info)
    sys.stdout.write('\n')
    return cam_infos

def fetchPly(path):
    plydata = PlyData.read(path)
    vertices = plydata['vertex']
    positions = np.vstack([vertices['x'], vertices['y'], vertices['z']]).T
    colors = np.vstack([vertices['red'], vertices['green'], vertices['blue']]).T / 255.0
    normals = np.vstack([vertices['nx'], vertices['ny'], vertices['nz']]).T
    return BasicPointCloud(points=positions, colors=colors, normals=normals)

def storePly(path, xyz, rgb):
    # Define the dtype for the structured array
    dtype = [('x', 'f4'), ('y', 'f4'), ('z', 'f4'),
            ('nx', 'f4'), ('ny', 'f4'), ('nz', 'f4'),
            ('red', 'u1'), ('green', 'u1'), ('blue', 'u1')]
    
    normals = np.zeros_like(xyz)

    elements = np.empty(xyz.shape[0], dtype=dtype)
    attributes = np.concatenate((xyz, normals, rgb), axis=1)
    elements[:] = list(map(tuple, attributes))

    # Create the PlyData object and write to file
    vertex_element = PlyElement.describe(elements, 'vertex')
    ply_data = PlyData([vertex_element])
    ply_data.write(path)

def readColmapSceneInfo(path, images, eval, llffhold=8):
    try:
        cameras_extrinsic_file = os.path.join(path, "sparse/0", "images.bin")
        cameras_intrinsic_file = os.path.join(path, "sparse/0", "cameras.bin")
        cam_extrinsics = read_extrinsics_binary(cameras_extrinsic_file)
        cam_intrinsics = read_intrinsics_binary(cameras_intrinsic_file)
    except:
        cameras_extrinsic_file = os.path.join(path, "sparse/0", "images.txt")
        cameras_intrinsic_file = os.path.join(path, "sparse/0", "cameras.txt")
        cam_extrinsics = read_extrinsics_text(cameras_extrinsic_file)
        cam_intrinsics = read_intrinsics_text(cameras_intrinsic_file)

    reading_dir = "images" if images == None else images
    cam_infos_unsorted = readColmapCameras(cam_extrinsics=cam_extrinsics, cam_intrinsics=cam_intrinsics, images_folder=os.path.join(path, reading_dir))
    cam_infos = sorted(cam_infos_unsorted.copy(), key = lambda x : x.image_name)

    if eval:
        train_cam_infos = [c for idx, c in enumerate(cam_infos) if idx % llffhold != 0]
        test_cam_infos = [c for idx, c in enumerate(cam_infos) if idx % llffhold == 0]
    else:
        train_cam_infos = cam_infos
        test_cam_infos = []

    nerf_normalization = getNerfppNorm(train_cam_infos)

    ply_path = os.path.join(path, "sparse/0/points3D.ply")
    bin_path = os.path.join(path, "sparse/0/points3D.bin")
    txt_path = os.path.join(path, "sparse/0/points3D.txt")
    if not os.path.exists(ply_path):
        print("Converting point3d.bin to .ply, will happen only the first time you open the scene.")
        try:
            xyz, rgb, _ = read_points3D_binary(bin_path)
        except:
            xyz, rgb, _ = read_points3D_text(txt_path)
        storePly(ply_path, xyz, rgb)
    try:
        pcd = fetchPly(ply_path)
    except:
        pcd = None

    scene_info = SceneInfo(point_cloud=pcd,
                           train_cameras=train_cam_infos,
                           test_cameras=test_cam_infos,
                           nerf_normalization=nerf_normalization,
                           ply_path=ply_path)
    return scene_info

def readCamerasFromTransforms(path, transformsfile, white_background, extension=".png"):
    cam_infos = []

    with open(os.path.join(path, transformsfile)) as json_file:
        contents = json.load(json_file)
        fovx = contents["camera_angle_x"]

        frames = contents["frames"]
        for idx, frame in enumerate(frames):
            cam_name = os.path.join(path, frame["file_path"] + extension)

            # NeRF 'transform_matrix' is a camera-to-world transform
            c2w = np.array(frame["transform_matrix"])
            # change from OpenGL/Blender camera axes (Y up, Z back) to COLMAP (Y down, Z forward)
            c2w[:3, 1:3] *= -1

            # get the world-to-camera transform and set R, T
            w2c = np.linalg.inv(c2w)
            R = np.transpose(w2c[:3,:3])  # R is stored transposed due to 'glm' in CUDA code
            T = w2c[:3, 3]

            image_path = os.path.join(path, cam_name)
            image_name = Path(cam_name).stem
            image = Image.open(image_path)

            im_data = np.array(image.convert("RGBA"))

            bg = np.array([1,1,1]) if white_background else np.array([0, 0, 0])

            norm_data = im_data / 255.0
            arr = norm_data[:,:,:3] * norm_data[:, :, 3:4] + bg * (1 - norm_data[:, :, 3:4])
            image = Image.fromarray(np.array(arr*255.0, dtype=np.byte), "RGB")

            fovy = focal2fov(fov2focal(fovx, image.size[0]), image.size[1])
            FovY = fovy 
            FovX = fovx

            cam_infos.append(CameraInfo(uid=idx, R=R, T=T, FovY=FovY, FovX=FovX, image=image,
                            image_path=image_path, image_name=image_name, width=image.size[0], height=image.size[1]))
            
    return cam_infos

def readNerfSyntheticInfo(path, white_background, eval, extension=".png"):
    print("Reading Training Transforms")
    train_cam_infos = readCamerasFromTransforms(path, "transforms_train.json", white_background, extension)
    print("Reading Test Transforms")
    test_cam_infos = readCamerasFromTransforms(path, "transforms_test.json", white_background, extension)
    
    if not eval:
        train_cam_infos.extend(test_cam_infos)
        test_cam_infos = []

    nerf_normalization = getNerfppNorm(train_cam_infos)

    ply_path = os.path.join(path, "points3d.ply")
    if not os.path.exists(ply_path):
        # Since this data set has no colmap data, we start with random points
        num_pts = 100_000
        print(f"Generating random point cloud ({num_pts})...")
        
        # We create random points inside the bounds of the synthetic Blender scenes
        xyz = np.random.random((num_pts, 3)) * 2.6 - 1.3
        shs = np.random.random((num_pts, 3)) / 255.0
        pcd = BasicPointCloud(points=xyz, colors=SH2RGB(shs), normals=np.zeros((num_pts, 3)))

        storePly(ply_path, xyz, SH2RGB(shs) * 255)
    try:
        pcd = fetchPly(ply_path)
    except:
        pcd = None

    scene_info = SceneInfo(point_cloud=pcd,
                           train_cameras=train_cam_infos,
                           test_cameras=test_cam_infos,
                           nerf_normalization=nerf_normalization,
                           ply_path=ply_path)
    return scene_info

def readCamerasDNARendering(path, info_dict, white_background, image_scaling=0.5, return_smplx=False):
    output_view = info_dict["views"]
    frame_idx = info_dict["frame_idx"]
    ratio = image_scaling
    
    cam_infos = []
    main_file = path 
    annot_file = path.replace('main', 'annotations').split('.')[0] + '_annots.smc'
    main_reader = SMCReader(main_file)
    annot_reader = SMCReader(annot_file)

    smplx_vertices = None 
    if return_smplx:
        gender = main_reader.actor_info['gender']
        model = SMPLX(
            'assets/body_models/smplx/', smpl_type='smplx',
            gender=gender, use_face_contour=True, flat_hand_mean=False, use_pca=False, 
            num_betas=10, num_expression_coeffs=10, ext='npz'
        )
        smplx_dict = annot_reader.get_SMPLx(Frame_id=frame_idx)
        betas = torch.from_numpy(smplx_dict["betas"]).unsqueeze(0).float()
        expression = torch.from_numpy(smplx_dict["expression"]).unsqueeze(0).float()
        fullpose = torch.from_numpy(smplx_dict["fullpose"]).unsqueeze(0).float()
        translation = torch.from_numpy(smplx_dict['transl']).unsqueeze(0).float()
        output = model(
            betas=betas, 
            expression=expression,
            global_orient = fullpose[:, 0].clone(),
            body_pose = fullpose[:, 1:22].clone(),
            jaw_pose = fullpose[:, 22].clone(),
            leye_pose = fullpose[:, 23].clone(),
            reye_pose = fullpose[:, 24].clone(),
            left_hand_pose = fullpose[:, 25:40].clone(), 
            right_hand_pose = fullpose[:, 40:55].clone(),
            transl = translation,
            return_verts=True)
        smplx_vertices = output.vertices.detach().cpu().numpy().squeeze()
        
    parent_dir = os.path.dirname(os.path.dirname(path))
    out_img_dir = os.path.join(parent_dir, "images")
    # os.makedirs(out_img_dir, exist_ok=True) 
    bg = np.array([255, 255, 255]) if white_background else np.array([0, 0, 0])
    idx = 0   
    for view_index in output_view:
        # Load K, R, T
        cam_params = annot_reader.get_Calibration(view_index)
        K = cam_params['K']
        D = cam_params['D'] # k1, k2, p1, p2, k3
        RT = cam_params['RT']

        # Load image, mask
        image = main_reader.get_img('Camera_5mp', view_index, Image_type='color', Frame_id=frame_idx)
        image = cv2.undistort(image, K, D)
        image = cv2.cvtColor(image, cv2.COLOR_BGR2RGB)
            
        mask = annot_reader.get_mask(view_index, Frame_id=frame_idx)
        mask = cv2.undistort(mask, K, D)
        mask = mask[..., np.newaxis].astype(np.float32) / 255.0
        image = image * mask + bg * (1.0 - mask)
        
        c2w = np.array(RT, dtype=np.float32)
        w2c = np.linalg.inv(c2w)
        R = np.transpose(w2c[:3,:3])  # R is stored transposed due to 'glm' in CUDA code
        T = w2c[:3, 3].copy()
        if ratio != 1.0:
            H, W = int(image.shape[0] * ratio), int(image.shape[1] * ratio)
            image = cv2.resize(image, (W, H), interpolation=cv2.INTER_AREA)
            mask = cv2.resize(mask, (W, H), interpolation=cv2.INTER_NEAREST)
            K[:2] = K[:2] * ratio

        H, W, _ = image.shape
        focalX = K[0,0]
        focalY = K[1,1]
        FovX = focal2fov(focalX, W)
        FovY = focal2fov(focalY, H)

        image = Image.fromarray(np.array(image, dtype=np.byte), "RGB")
        image_name = "%04d" % view_index
        image_path = os.path.join(out_img_dir, "%s.png" % image_name)

        cam_infos.append(CameraInfo(uid=idx, R=R, T=T, FovY=FovY, FovX=FovX, image=image,
                            image_path=image_path, image_name=image_name, width=W, height=H,
                            K=K, use_K=True))
            
        idx += 1
    
    return cam_infos, smplx_vertices

def readDNARenderingInfo(path, white_background, eval):

    test_view_arr = [3, 5, 7, 8, 11, 15, 17, 19, 21, 23, 27, 29, 31, 33, 35, 39, 41, 45]
    train_view_arr = [x for x in range(48) if x not in test_view_arr]
    train_info_dict = {
        "views": train_view_arr,
        "frame_idx": 1,
    }
    test_info_dict = {
        "views": test_view_arr,
        "frame_idx": 1,
    }
    print("Reading Training Transforms", flush=True)
    train_cam_infos, smplx_vertices = readCamerasDNARendering(path, train_info_dict, white_background, return_smplx=True)
    print("Reading Test Transforms", flush=True)
    test_cam_infos, _ = readCamerasDNARendering(path, test_info_dict, white_background, return_smplx=False)
    
    if not eval:
        train_cam_infos.extend(test_cam_infos)
        test_cam_infos = []

    nerf_normalization = getNerfppNorm(train_cam_infos)

    parent_dir = os.path.dirname(os.path.dirname(path))
    ply_path = os.path.join(parent_dir, "points3d.ply")
    print("Using SMPLX vertices to initiate", flush=True)
    num_pts, _ = smplx_vertices.shape 
    shs = np.random.random((num_pts, 3)) / 255.0
    pcd = BasicPointCloud(points=smplx_vertices, colors=SH2RGB(shs), normals=np.zeros((num_pts, 3)))
    storePly(ply_path, smplx_vertices, SH2RGB(shs) * 255)
    
    scene_info = SceneInfo(point_cloud=pcd,
                           train_cameras=train_cam_infos,
                           test_cameras=test_cam_infos,
                           nerf_normalization=nerf_normalization,
                           ply_path=ply_path)
    return scene_info

sceneLoadTypeCallbacks = {
    "Colmap": readColmapSceneInfo,
    "Blender" : readNerfSyntheticInfo,
    "DNA-Rendeing": readDNARenderingInfo,
}

```
