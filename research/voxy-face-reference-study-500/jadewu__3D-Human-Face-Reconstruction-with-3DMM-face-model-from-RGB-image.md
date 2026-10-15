# Facial rendering source scan: jadewu/3D-Human-Face-Reconstruction-with-3DMM-face-model-from-RGB-image

Repository: https://github.com/jadewu/3D-Human-Face-Reconstruction-with-3DMM-face-model-from-RGB-image

Pinned source revision: 9984579927b96c873d2b45b89819bd80b228b259

Discovery topic: face reconstruction 3D

Description: Reconstruct 3D model from 2D human face images and CNN based PCA generation.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"face": ["BFM/facemodel_info.mat", "FaceLandmarkDetection.py"]}

## Source file: FaceLandmarkDetection.py

{
  "path": "FaceLandmarkDetection.py",
  "blob_sha": "7abf29f31fd66280dedd56f60acdf6e9af2c3bc9",
  "sha256": "735622be6ec7088c41ef5f90a81d454d8c5ffb082293278e0714cdd9121fada6",
  "lines": 50,
  "bytes": 2592,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 2,
    "lip": 1
  },
  "url": "https://github.com/jadewu/3D-Human-Face-Reconstruction-with-3DMM-face-model-from-RGB-image/blob/9984579927b96c873d2b45b89819bd80b228b259/FaceLandmarkDetection.py",
  "function_signature_candidates": [
    {
      "line": 27,
      "text": "def prepare_dataset(name, size, _image_list):"
    }
  ]
}

```text

import face_alignment
import glob
from tqdm import tqdm
from skimage import io
import matplotlib.pyplot as plt
import random

import h5py

random.seed(3)
print ("Set random seed to 3!")

block_img_list = ["24_Alan_Ritchson_0006.jpg", "50_Suzie_Plakson_0012.jpg", "18_Ross_Bagley_0005.jpg", "38_Maggie_Siff_0011.jpg", "58_Tony_Todd_0011.jpg", "37_Sanaa_Lathan_0007.jpg", "34_Robin_Thicke_0008.jpg", "56_Tress_MacNeille_0010.jpg", "29_Devon_Sawa_0006.jpg", "52_Roberto_Benigni_0004.jpg", "42_Michelle_Yeoh_0011.jpg", "54_Hulk_Hogan_0005.jpg", "43_Michelle_Yeoh_0009.jpg", "55_Bill_Mumy_0012.jpg", "57_Mark_Boone_Junior_0010.jpg", "56_Jim_Cummings_0015.jpg", "43_Shane_Black_0006.jpg", "48_Amy_Sedaris_0014.jpg", "43_Aidan_Gillen_0011.jpg", "48_Brad_Bird_0008.jpg", "53_Catherine_Bach_0005.jpg", "47_Brad_Bird_0005.jpg", "39_Naveen_Andrews_0014.jpg", "34_Burn_Gorman_0006.jpg", "19_Mae_Whitman_0009.jpg", "26_Jaimie_Alexander_0006.jpg", "56_Sting_0004.jpg", "33_Linda_Cardellini_0004.jpg", "58_Robert_Picardo_0007.jpg"]

fa = face_alignment.FaceAlignment(face_alignment.LandmarksType._3D, flip_input=False)

image_list = glob.glob("./data/CACD2000/*.jpg")
# print(image_list)
random.shuffle(image_list)
num_image = int(len(image_list)/6)

num_train_image = int(num_image*0.70)
num_val_image = int(num_image*0.15)
num_test_image = num_image-num_train_image-num_val_image
# print(num_train_image, num_val_image, num_test_image)

def prepare_dataset(name, size, _image_list):

    with h5py.File("./data/CACD2000_{}.hdf5".format(name), 'w') as f:
        image_dset = f.create_dataset("img", shape=(size, 250, 250 , 3), dtype='uint8')
        landmark_dset = f.create_dataset("lmk_2D", shape=(size, 68, 2), dtype='uint8')
        for idx, image_path in tqdm(enumerate(_image_list), desc="Processing Landmark...", total=size):
            if image_path.split('/')[-1] in block_img_list:
                continue
            image = io.imread(image_path)
            image_dset[idx] = image
            lmk = fa.get_landmarks(image)
            if lmk is None:
                print ("Error in {}".format(image_path))
            else:
                landmark_dset[idx] = lmk[0][:,:2]

print ("Prepare validation set:")
prepare_dataset("val", num_val_image, image_list[:num_val_image])
print ("Prepare test set:")
prepare_dataset("test", num_test_image, image_list[num_val_image:num_val_image+num_test_image])
print ("Prepare train set:")
prepare_dataset("train", num_train_image, image_list[27241-num_train_image:27241])

print ("Dataset prearation stopped for protecting the current data folder!")

```

## Source file: BFM.py

{
  "path": "BFM.py",
  "blob_sha": "59f763445655346a5dc8a8945af9792843ae7825",
  "sha256": "26c02c211510d6ce1ba59d91fb1f47b343a3f5eaa61812d6b2e0374d28b21eb5",
  "lines": 108,
  "bytes": 4094,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 3,
    "face": 9
  },
  "url": "https://github.com/jadewu/3D-Human-Face-Reconstruction-with-3DMM-face-model-from-RGB-image/blob/9984579927b96c873d2b45b89819bd80b228b259/BFM.py",
  "function_signature_candidates": [
    {
      "line": 12,
      "text": "def __init__(self):"
    },
    {
      "line": 29,
      "text": "def __init__(self):"
    },
    {
      "line": 50,
      "text": "def get_shape(self, id_param, ex_param):"
    },
    {
      "line": 68,
      "text": "def get_texture(self, tex_param):"
    },
    {
      "line": 79,
      "text": "def compute_rotation_matrix(self, rotate_param):"
    }
  ]
}

```text

import torch
from torch import nn
import numpy as np
from scipy.io import loadmat, savemat
from array import array

class BFM():
	"""
	This is a numpy implementation of BFM model
		for visualization purpose, not used in the DNN model
	"""
	def __init__(self):
		model_path = './BFM/BFM_model_front.mat'
		model = loadmat(model_path)
		self.meanshape = model['meanshape'].T # mean face shape 
		self.idBase = model['idBase'] # identity basis
		self.exBase = model['exBase'] # expression basis
		self.meantex = model['meantex'].T # mean face texture
		self.texBase = model['texBase'] # texture basis
		self.point_buf = model['point_buf'] # adjacent face index for each vertex, starts from 1 (only used for calculating face normal)
		self.tri = model['tri'] # vertex index for each triangle face, starts from 1
		self.keypoints = np.squeeze(model['keypoints']).astype(np.int32) - 1 # 68 face landmark index, starts from 0

class BFM_torch(nn.Module):
	"""
	This is a torch implementation of the BFM model
	Used in the DNN model, comes with gradient support
	"""
	def __init__(self):
		super(BFM_torch, self).__init__()
		model_path = './BFM/BFM_model_front.mat'
		model = loadmat(model_path)
		# [107127, 1]
		self.register_buffer("meanshape", torch.tensor(model['meanshape'].T, dtype=torch.float32))
		# [107127, 80]
		self.register_buffer("idBase", torch.tensor(model['idBase'], dtype=torch.float32))
		# [107127, 64]
		self.register_buffer("exBase", torch.tensor(model['exBase'], dtype=torch.float32))
		# [107127, 1]
		self.register_buffer("meantex", torch.tensor(model['meantex'].T, dtype=torch.float32))
		# [107121, 80]
		self.register_buffer('texBase', torch.tensor(model['texBase'], dtype=torch.float32))
		# [70789, 3]
		self.register_buffer('tri', torch.tensor(model['tri'], dtype=torch.int32))
		# [35709, 8] Max is 70789;
		self.register_buffer('point_buf', torch.tensor(model['point_buf'], dtype=torch.int32))
		# [68]
		self.register_buffer('keypoints', torch.tensor(np.squeeze(model['keypoints']).astype(np.int32) - 1, dtype=torch.int32))

	def get_shape(self, id_param, ex_param):
		"""
		Perform shape assembly from index parameter and expression parameter
		id_param: [bs, 80]
		ex_param: [bs, 64]
		return: [bs, 107127, 1]
		"""
		assert id_param.shape[0] == ex_param.shape[0]
		bs = id_param.shape[0]

		id_base = self.idBase[None,:,:].expand(bs,-1,-1)
		ex_base = self.exBase[None,:,:].expand(bs,-1,-1)
		
		face_shape = self.meanshape+torch.bmm(id_base,id_param[:,:,None])+torch.bmm(ex_base,ex_param[:,:,None])
		face_shape = face_shape.reshape(bs,-1, 3)
		face_shape = face_shape - torch.mean(self.meanshape[None,:,:].reshape(1,-1,3), dim=1, keepdim=True)        
		return face_shape

	def get_texture(self, tex_param):
		"""
		Perform texture assembly from texture parameter
		tex_param: [bs, 80]
		return: [bs, 107127, 1]
		"""
		bs = tex_param.shape[0]
		tex_base = self.texBase[None,:,:].expand(bs,-1,-1)
		
		return self.meantex+torch.bmm(tex_base,tex_param[:,:,None])

	def compute_rotation_matrix(self, rotate_param):
		"""
		Perform rotation based on the batch rotation parameter
		rotate_param: [bs, 3]
		return: [bs, 3, 3]
		"""
		pitch, yaw, roll = rotate_param[:,0], rotate_param[:,1], rotate_param[:,2]
		bs = rotate_param.shape[0]
		device = rotate_param.device

		pitch_matrix = torch.eye(3, device=device)[None,:,:].expand(bs,-1,-1).clone()
		yaw_matrix = torch.eye(3, device=device)[None,:,:].expand(bs,-1,-1).clone()
		roll_matrix  = torch.eye(3, device=device)[None,:,:].expand(bs,-1,-1).clone()

		pitch_matrix[:,1,1] = torch.cos(pitch)
		pitch_matrix[:,2,2] = torch.cos(pitch)
		pitch_matrix[:,1,2] = -torch.sin(pitch)
		pitch_matrix[:,2,1] = torch.sin(pitch)

		yaw_matrix[:,0,0] = torch.cos(yaw)
		yaw_matrix[:,2,2] = torch.cos(yaw)
		yaw_matrix[:,0,2] = torch.sin(yaw)
		yaw_matrix[:,2,0] = -torch.sin(yaw)

		roll_matrix[:,0,0] = torch.cos(roll)
		roll_matrix[:,1,1] = torch.cos(roll)
		roll_matrix[:,0,1] = -torch.sin(roll)
		roll_matrix[:,1,0] = torch.sin(roll)

		return torch.bmm(torch.bmm(roll_matrix, yaw_matrix), pitch_matrix).permute(0,2,1)

```

## Source file: loss.py

{
  "path": "loss.py",
  "blob_sha": "6cb08f0a9fde60595cd1cea3753f0bf15a302ec9",
  "sha256": "b5386fc34f9dcc26338fc60680f79a063e255fcc094ae1c6ee96d44cd2bdaac0",
  "lines": 210,
  "bytes": 8662,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 1,
    "face": 51,
    "lip": 3
  },
  "url": "https://github.com/jadewu/3D-Human-Face-Reconstruction-with-3DMM-face-model-from-RGB-image/blob/9984579927b96c873d2b45b89819bd80b228b259/loss.py",
  "function_signature_candidates": [
    {
      "line": 15,
      "text": "def __init__(self, renderer, lmk_loss_w, recog_loss_w, device):"
    },
    {
      "line": 39,
      "text": "def split(self, params):"
    },
    {
      "line": 49,
      "text": "def compute_norm(self, vertices):"
    },
    {
      "line": 83,
      "text": "def lighting(self, norm, albedo, gamma):"
    },
    {
      "line": 115,
      "text": "def reconst_img(self, params, return_type=None):"
    },
    {
      "line": 148,
      "text": "def forward(self, params, gt_img, gt_lmk):"
    }
  ]
}

```text

import soft_renderer as sr
from torch import nn
import torch
import torch.nn.functional as f
import math
import numpy as np
from tqdm import tqdm
import os

from BFM import BFM_torch
from facenet_pytorch import InceptionResnetV1

class BFMFaceLoss(nn.Module):
    """Decode from the learned parameters to the 3D face model"""
    def __init__(self, renderer, lmk_loss_w, recog_loss_w, device):
        super(BFMFaceLoss, self).__init__()
        self.BFM_model = BFM_torch().to(device)
        self.renderer = renderer

        self.mse_criterion = nn.MSELoss()
        self.sl1_criterion = nn.SmoothL1Loss()
        self.lmk_loss_w = lmk_loss_w
        self.device = device

        self.a0 = torch.tensor(math.pi).to(self.device)
        self.a1 = torch.tensor(2*math.pi/math.sqrt(3.0)).to(self.device)
        self.a2 = torch.tensor(2*math.pi/math.sqrt(8.0)).to(self.device)
        self.c0 = torch.tensor(1/math.sqrt(4*math.pi)).to(self.device)
        self.c1 = torch.tensor(math.sqrt(3.0)/math.sqrt(4*math.pi)).to(self.device)
        self.c2 = torch.tensor(3*math.sqrt(5.0)/math.sqrt(12*math.pi)).to(self.device)

        # self.reverse_z = torch.eye(3).to(self.device)[None,:,:]
        # self.face_net = InceptionResnetV1(pretrained='vggface2').eval()
        # for param in self.face_net.parameters():
        #     param.requires_grad=False
        # self.face_net.to(device)
        # self.recog_loss_w = recog_loss_w    

    def split(self, params):
        id_coef = params[:,:80]
        ex_coef = params[:,80:144]
        tex_coef = params[:,144:224]
        angles = params[:,224:227]
        gamma  = params[:,227:254]
        translation = params[:,254:257]
        scale = params[:,257:]
        return id_coef, ex_coef, tex_coef, angles, gamma, translation, scale

    def compute_norm(self, vertices):
        """
        Compute the norm of the vertices
        Input:
            vertices[bs, 35709, 3]
        """
        bs = vertices.shape[0]
        face_id = torch.flip(self.BFM_model.tri.reshape(-1,3)-1, dims=[1])
        point_id = self.BFM_model.point_buf-1
        # [bs, 70789, 3]
        face_id = face_id[None,:,:].expand(bs,-1,-1)
        # [bs, 35709, 8]
        point_id = point_id[None,:,:].expand(bs,-1,-1)
        # [bs, 70789, 3] Gather the vertex location
        v1 = torch.gather(vertices, dim=1,index=face_id[:,:,:1].expand(-1,-1,3).long())
        v2 = torch.gather(vertices, dim=1,index=face_id[:,:,1:2].expand(-1,-1,3).long())
        v3 = torch.gather(vertices, dim=1,index=face_id[:,:,2:].expand(-1,-1,3).long())
        # Compute the edge
        e1 = v1-v2
        e2 = v2-v3
        # Normal [bs, 70789, 3]
        norm = torch.cross(e1, e2)
        # Normal appended with zero vector [bs, 70790, 3]
        norm = torch.cat([norm, torch.zeros(bs, 1, 3).to(self.device)], dim=1)
        # [bs, 35709*8, 3]
        point_id = point_id.reshape(bs,-1)[:,:,None].expand(-1,-1,3)
        # [bs, 35709*8, 3]
        v_norm = torch.gather(norm, dim=1, index=point_id.long())
        v_norm = v_norm.reshape(bs, 35709, 8, 3)
        # [bs, 35709, 3]
        v_norm = f.normalize(torch.sum(v_norm, dim=2), dim=-1)
        return v_norm


    def lighting(self, norm, albedo, gamma):
        """
        Add lighting to the albedo surface
        gamma: [bs, 27]
        norm: [bs, num_vertex, 3]
        albedo: [bs, num_vertex, 3]
        """
        assert norm.shape[0] == albedo.shape[0]
        assert norm.shape[0] == gamma.shape[0]
        bs = gamma.shape[0]
        num_vertex = norm.shape[1]

        init_light = torch.zeros(9).to(self.device)
        init_light[0] = 0.8
        gamma = gamma.reshape(bs,3,9)+init_light

        Y0 = self.a0*self.c0*torch.ones(bs, num_vertex, 1, device=self.device)
        Y1 = -self.a1*self.c1*norm[:,:,1:2]
        Y2 = self.a1*self.c1*norm[:,:,2:3]
        Y3 = -self.a1*self.c1*norm[:,:,0:1]
        Y4 = self.a2*self.c2*norm[:,:,0:1]*norm[:,:,1:2]
        Y5 = -self.a2*self.c2*norm[:,:,1:2]*norm[:,:,2:3]
        Y6 = self.a2*self.c2*0.5/math.sqrt(3.0)*(3*norm[:,:,2:3]**2-1)
        Y7 = -self.a2*self.c2*norm[:,:,0:1]*norm[:,:,2:3]
        Y8 = self.a2*self.c2*0.5*(norm[:,:,0:1]**2-norm[:,:,1:2]**2)
        # [bs, num_vertice, 9]
        Y = torch.cat([Y0,Y1,Y2,Y3,Y4,Y5,Y6,Y7,Y8],dim=2)

        light_color = torch.bmm(Y, gamma.permute(0,2,1))
        vertex_color = light_color*albedo
        return vertex_color

    def reconst_img(self, params, return_type=None):
        bs = params.shape[0]
        id_coef, ex_coef, tex_coef, angles, gamma, tranlation, scale = self.split(params)

        face_shape = self.BFM_model.get_shape(id_coef, ex_coef)
        face_albedo = self.BFM_model.get_texture(tex_coef) 
        face_shape[:,:,-1] *= -1
        # Recenter the face mesh
        face_albedo = face_albedo.reshape(bs, -1, 3)/255.

        # face model rotation, scale, translation, lighting, triangles
        rotation_matrix = self.BFM_model.compute_rotation_matrix(angles)
        face_shape = torch.bmm(face_shape, rotation_matrix)
        # Compute the normal
        normal = self.compute_norm(face_shape)
        
        face_shape = (1+scale[:,:,None])*face_shape
        face_shape = face_shape+tranlation[:,None,:]

        face_albedo = self.lighting(normal, face_albedo, gamma)

        tri = torch.flip(self.BFM_model.tri.reshape(-1,3)-1, dims=[-1])
        face_triangles = tri[None,:,:].expand(bs,-1,-1)

        recon_img = self.renderer(face_shape,
                                              face_triangles,
                                              face_albedo,
                                              texture_type="vertex")
        if return_type == 'all':
            return recon_img, face_shape, face_triangles, face_albedo
        else:
            return recon_img

    def forward(self, params, gt_img, gt_lmk):
        bs = params.shape[0]
        id_coef, ex_coef, tex_coef, angles, gamma, tranlation, scale = self.split(params)
        # print("Scale: ", scale)

        face_shape = self.BFM_model.get_shape(id_coef, ex_coef)
        face_albedo = self.BFM_model.get_texture(tex_coef) 
        face_shape[:,:,-1] *= -1
        # Recenter the face mesh
        face_albedo = face_albedo.reshape(bs, -1, 3)/255.


        # face model rotation, scale, translation, lighting, triangles
        rotation_matrix = self.BFM_model.compute_rotation_matrix(angles)
        face_shape = torch.bmm(face_shape, rotation_matrix)
        # Compute the normal
        normal = self.compute_norm(face_shape)
        
        face_shape = (1+scale[:,:,None])*face_shape
        face_shape = face_shape+tranlation[:,None,:]

        face_albedo = self.lighting(normal, face_albedo, gamma)

        tri = torch.flip(self.BFM_model.tri.reshape(-1,3)-1, dims=[-1])
        face_triangles = tri[None,:,:].expand(bs,-1,-1)

        # print("here is the bug1")
        # recon_img, recon_mesh = self.renderer(...)
        recon_mesh = sr.Mesh(face_shape,face_triangles,face_albedo,texture_type="vertex")
        
        # print("recon_mesh vertices size: ", recon_mesh.vertices.shape)
        # print("recon_mesh vertices: ", recon_mesh.vertices)
        # print("face_shape size: ", face_shape.shape)
        # print("face_shape: ", face_shape)

        # print("here is the bug2")
        recon_img = self.renderer(face_shape,
                                              face_triangles,
                                              face_albedo,
                                              texture_type="vertex")
        # print("after: ", face_shape)
        
        # print("here is the bug3")
        recon_lmk = recon_mesh.vertices[:, self.BFM_model.keypoints.long(), :]

        # Compute loss
        # remove the alpha channel
        mask = (recon_img[:,-1:,:,:].detach() > 0).float()
        # Image loss
        img_loss = self.mse_criterion(recon_img[:,:3,:,:], gt_img*mask)
        # Landmark loss
        recon_lmk_2D_rev = (recon_lmk[:,:,:2]+1)*250./2.
        recon_lmk_2D = (recon_lmk[:,:,:2]+1)*250./2.
        recon_lmk_2D[:,:,1] = 250.-recon_lmk_2D_rev[:,:,1] 
        lmk_loss = self.sl1_criterion(recon_lmk_2D, gt_lmk.float())
        # # face recog loss
        # recon_feature = self.face_net(recon_img[:,:3,:,:])
        # gt_feature = self.face_net(gt_img*mask)
        # recog_loss = self.mse_criterion(recon_feature, gt_feature)
        # all_loss = img_loss + self.lmk_loss_w*lmk_loss + self.recog_loss_w*recog_loss
        all_loss = img_loss + self.lmk_loss_w*lmk_loss
        # return all_loss, img_loss, lmk_loss, recog_loss, recon_img
        return all_loss, img_loss, lmk_loss, recon_img


```

## Source file: model.py

{
  "path": "model.py",
  "blob_sha": "d5a9c6bb21b7dd3ea8a56ef0c6c9a250605c36aa",
  "sha256": "45be34bb96123774cc3ffeeaed636e779a994ecaa166c5ffeeba90f4c1813ad4",
  "lines": 142,
  "bytes": 5418,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 22
  },
  "url": "https://github.com/jadewu/3D-Human-Face-Reconstruction-with-3DMM-face-model-from-RGB-image/blob/9984579927b96c873d2b45b89819bd80b228b259/model.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def __init__(self, IF_PRETRAINED=False):"
    },
    {
      "line": 14,
      "text": "def forward(self, images):"
    }
  ]
}

```text

# import torch
import torch.nn as nn
from torchvision import models
from BFM import BFM_torch
import math

class BaseModel(nn.Module):
    """Get the base network, which is modified from ResNet50"""
    def __init__(self, IF_PRETRAINED=False):
        super(BaseModel, self).__init__()
        self.resnet50 = models.resnet50(pretrained=IF_PRETRAINED)
        self.resnet50.fc = nn.Linear(2048, 258)

    def forward(self, images):
    	# print("Resnet50 input images size: ",images[0].shape)
    	return self.resnet50(images)

# class BaseDecoder(nn.Module):
#     """Decode from the learned parameters to the 3D face model"""
#     def __init__(self):
#         super(BaseDecoder, self).__init__()
#         self.BFM_model = BFM_torch()

#     def split(self, params):
#         id_coef = params[:,:80]
#         ex_coef = params[:,80:144]
#         tex_coef = params[:,144:224]
#         angles = params[:,224:227]
#         gamma  = params[:,227:254]
#         translation = params[:,254:257]
#         scale = params[:,257:]

#         return id_coef, ex_coef, tex_coef, angles, gamma, translation, scale

#     def compute_norm(vertices):
#         """
#         Compute the norm of the vertices
#         Input:
#             vertices[bs, 35709, 3]
#         """
#         bs = vertices.shape[0]
#         face_id = self.BFM_model.tri-1
#         point_id = self.BFM_model.point_buf-1
#         # [bs, 70789, 3]
#         face_id = face_id[None,:,:].expand(bs,-1,-1)
#         # [bs, 35709, 8]
#         point_id = point_id[None,:,:].expand(bs,-1,-1)
#         # [bs, 70789, 3] Gather the vertex location
#         v1 = torch.gather(vertices, dim=1,index=face_id[:,:,:1].expand(-1,-1,3).long())
#         v2 = torch.gather(vertices, dim=1,index=face_id[:,:,1:2].expand(-1,-1,3).long())
#         v3 = torch.gather(vertices, dim=1,index=face_id[:,:,2:].expand(-1,-1,3).long())
#         # Compute the edge
#         e1 = v1-v2
#         e2 = v2-v3
#         # Normal [bs, 70789, 3]
#         norm = torch.cross(e1, e2)
#         # Normal appended with zero vector [bs, 70790, 3]
#         norm = torch.cat([norm, torch.zeros(bs, 1, 3)], dim=1)
#         # [bs, 35709*8, 3]
#         point_id = point_id.reshape(bs,-1)[:,:,None].expand(-1,-1,3)
#         # [bs, 35709*8, 3]
#         v_norm = torch.gather(norm, dim=1, index=point_id.long())
#         v_norm = v_norm.reshape(bs, 35709, 8, 3)
#         # [bs, 35709, 3]
#         v_norm = f.normalize(torch.sum(v_norm, dim=2), dim=-1)
#         return v_norm


#     def lighting(self, norm, albedo, gamma):
#         """
#         Add lighting to the albedo surface
#         gamma: [bs, 27]
#         norm: [bs, num_vertex, 3]
#         albedo: [bs, num_vertex, 3]
#         """
#         assert norm.shape[0] == albedo.shape[0]
#         assert norm.shape[0] == gamma.shape[0]
#         bs = gamma.shape[0]
#         num_vertex = norm.shape[1]

#         init_light = torch.zeros(9)
#         init_light[0] = 0.8
#         gamma = gamma.reshape(bs,3,9)+init_light

#         a0 = torch.tensor(math.pi)
#         a1 = torch.tensor(2*math.pi/math.sqrt(3.0))
#         a2 = torch.tensor(2*math.pi/math.sqrt(8.0))
#         c0 = torch.tensor(1/math.sqrt(4*math.pi))
#         c1 = torch.tensor(math.sqrt(3.0)/math.sqrt(4*math.pi))
#         c2 = torch.tensor(3*math.sqrt(5.0)/math.sqrt(12*math.pi))

#         Y0 = a0[None, None, :].expand(bs, num_vertex, 1)
#         Y1 = -a1*c1*norm[:,:,1:2]
#         Y2 = a1*c1*norm[:,:,2:3]
#         Y3 = -a1*c1*norm[:,:,0:1]
#         Y4 = a2*c2*norm[:,:,0:1]*norm[:,:,1:2]
#         Y5 = -a2*c2*norm[:,:,1]*norm[:,:,2:3]
#         Y6 = a2*c2*0.5/torch.sqrt(3.0)*(3*torch.square(norm[:,:,2:3])-1)
#         Y7 = -a2*c2*norm[:,:,0:1]*norm[:,:,2:3]
#         Y8 = a2*c2*0.5*(torch.square(norm[:,:,0:1])-torch.square(norm[:,:,1:2]))
#         # [bs, num_vertice, 9]
#         Y = torch.cat([Y0,Y1,Y2,Y3,Y4,Y5,Y6,Y7,Y8],dim=2)

#         light_color = torch.bmm(Y, gamma.permute(0,2,1))
#         vertex_color = light_color*albedo
#         return vertex_color



#     def forward(self, params):
#         bs = params.shape[0]
#         id_coef, ex_coef, tex_coef, angles, gamma, tranlation, scale = self.split(params)
#         face_shape = self.BFM_model.get_shape(id_coef, ex_coef)
#         face_albedo = self.BFM_model.get_texture(tex_coef)

#         face_shape = face_shape.reshape(bs, -1, 3)
#         # Recenter the face mesh
#         face_shape = face_shape - torch.mean(face_shape, dim=1, keepdim=True)        
#         face_albedo = face_albedo.reshape(bs, -1, 3)/255.

#         # face model scaling, rotation and translation
#         rotation_matrix = self.BFM_model.compute_rotation_matrix(angles)
#         face_shape = torch.bmm(face_shape, rotation_matrix)
#         # Compute the normal
#         normal = compute_norm(face_shape)
#         face_shape = scale[:,:,None]*face_shape
#         face_shape = face_shape+tranlation[:,None,:]

#         face_albedo = self.lighting(normal, face_albedo, gamma)

#         return face_shape, face_albedo

# class BFMReconstruction(nn.Module):
#     def __init__(self):
#         super(BFMReconstruction, self).__init__()
#         self.encoder = BaseModel(False)
#         self.decoder = BaseDecoder()

#     def forward(self, x):
#         params = self.encoder(x)
#         face_shape, face_albedo = self.decoder(params)
#         return face_shape, face_albedo

```
