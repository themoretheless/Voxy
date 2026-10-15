# Facial rendering source scan: xubaixinxbx/3dheads

Repository: https://github.com/xubaixinxbx/3dheads

Pinned source revision: cbb9ab686149fda16a67e5756bc0f360b7e7222a

Discovery topic: human rendering

Description: [ICCV 2023] Code for "Deformable Model-Driven Neural Rendering for High-Fidelity 3D Reconstruction of Human Heads Under Low-View Settings"

Reported license: NOASSERTION

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"face": ["code/confs/face_st1.conf", "code/confs/face_st2.conf", "code/confs/facescape_config/bash_st1.sh", "code/confs/facescape_config/bash_st2.sh", "code/confs/facescape_config/st1.conf", "code/confs/facescape_config/st2.conf", "code/datasets/face_dataset.py", "code/datasets/facescape_dataset.py"]}

## Source file: code/datasets/face_dataset.py

{
  "path": "code/datasets/face_dataset.py",
  "blob_sha": "30a7832f074f4a9b64a00da67ffcc0e43935c0f0",
  "sha256": "a4363c7126bd5cdc5e271aca07c8ffd87e52623ede58e06231f3e57da05041b7",
  "lines": 336,
  "bytes": 13457,
  "selection_score": 15,
  "keyword_line_counts": {
    "wrinkle": 1,
    "face": 5,
    "lip": 1
  },
  "url": "https://github.com/xubaixinxbx/3dheads/blob/cbb9ab686149fda16a67e5756bc0f360b7e7222a/code/datasets/face_dataset.py",
  "function_signature_candidates": [
    {
      "line": 20,
      "text": "def load_xml(file_name):"
    },
    {
      "line": 93,
      "text": "def load_obj(obj_name):"
    },
    {
      "line": 120,
      "text": "def __init__(self, **kwargs):"
    },
    {
      "line": 234,
      "text": "def __len__(self):"
    },
    {
      "line": 237,
      "text": "def __getitem__(self, idx):"
    },
    {
      "line": 257,
      "text": "def collate_fn(self, batch_list):"
    },
    {
      "line": 274,
      "text": "def change_sampling_idx(self, sampling_size):"
    },
    {
      "line": 281,
      "text": "def get_specific_item(self, people_order=-1, cam_id=-1):"
    },
    {
      "line": 311,
      "text": "def get_scale_mat(self):"
    },
    {
      "line": 315,
      "text": "def get_id_list(self):"
    },
    {
      "line": 318,
      "text": "def get_novel_pose_between(self, idx_0, idx_1, i, n_frames=60):"
    }
  ]
}

```text

import os
import torch
import numpy as np

import utils.general as utils
from utils import rend_util
from glob import glob

import re
import sys
from tqdm import tqdm
import xml.etree.ElementTree as ET
import json 
from collections import defaultdict
from scipy.spatial.transform import Rotation as Rot
from scipy.spatial.transform import Slerp

base_dir = os.path.dirname(os.path.abspath(__file__))
sys.path.append(base_dir)
def load_xml(file_name):
	if not os.path.exists(file_name):
		return {}
	tree = ET.parse(file_name)
	root = tree.getroot()
	sens = {}
	for sensor in root.iter('sensor'):
		ID = sensor.attrib['id']
		resol = sensor.find('resolution')
		resol = (int(resol.attrib['height']), int(resol.attrib['width']))
		calib = sensor.find('calibration')
		K = np.identity(3)
		D = []
		for c in list(calib):
			if c.tag == 'f':
				K[0,0] = K[1,1] = float(c.text)
			elif c.tag == 'fx':
				K[0,0] = float(c.text)
			elif c.tag == 'fy':
				K[1,1] = float(c.text)
			elif c.tag == 'cx':
				K[0,2] = float(c.text)
				if True:# K[0,2] <= resol[1]/3.:
					K[0,2] += resol[1]/2.
			elif c.tag == 'cy':
				K[1,2] = float(c.text)
				if True:# K[1,2] <= resol[0]/3.:
					K[1,2] += resol[0]/2.
			elif c.tag == 'k1':
				D = D + [0]*max(1-len(D),0)
				D[0] = float(c.text)
			elif c.tag == 'k2':
				D = D + [0]*max(2-len(D),0)
				D[1] = float(c.text)
			elif c.tag == 'p1':
				D = D + [0]*max(3-len(D),0)
				D[2] = float(c.text)
			elif c.tag == 'p2':
				D = D + [0]*max(4-len(D),0)
				D[3] = float(c.text)
			elif c.tag == 'k3':
				D = D + [0]*max(5-len(D),0)
				D[4] = float(c.text)
			elif c.tag == 'k4':
				D = D + [0]*max(6-len(D),0)
				D[5] = float(c.text)
			elif c.tag == 'k5':
				D = D + [0]*max(7-len(D),0)
				D[6] = float(c.text)
			elif c.tag == 'k6':
				D = D + [0]*max(8-len(D),0)
				D[7] = float(c.text)
		if len(D) < 5 and len(D) > 0:
			D = D + [0]*(5-len(D))
		D = np.array(D, K.dtype) if len(D) > 0 else None
		sens[ID] = (K, D, resol)
	cams = {}
	for view in root.iter('camera'):
		ID = view.attrib['sensor_id']
		name = view.attrib['label']
		if not name[-4:].lower() in ['.jpg','.png']:
			name += '.jpg'
		if len(name[:-4]) == 1:
			name = '0' + name
		if view.find('transform') is None:
			continue
		cam2world = np.array([float(f) \
			for f in re.split(' |\n|\r|\t', view.find('transform').text) \
			if len(f) > 0], K.dtype).reshape(4,4)
		cams[name] = (sens[ID][0], sens[ID][1], \
			cam2world.reshape(4,4), sens[ID][2])
		# Intrinsics, distCoeff, cam2world_cv, height_width
	return	cams
def load_obj(obj_name):
	if obj_name[-4:].lower() != '.obj' or not os.path.exists(obj_name):
		return
	with open(obj_name, 'r') as fp:
		text = fp.readlines()
	v = []; fv = []
	pbar = tqdm(text)
	if hasattr(pbar, 'set_description'):
		pbar.set_description('Loading %s' % os.path.basename(obj_name))
	for line in pbar:
		line = [s for s in line.strip().split(' ') if len(s) > 0]
		if len(line) >= 3 and line[0] == 'v':
			v += [[float(f) for f in line[1:]]]
		elif len(line) >= 4 and line[0] == 'f':
			f = [[],[],[]]
			for i in range(1, len(line)):
				l = line[i].split('/')
				for j in range(3):
					if j < len(l) and len(l[j]) > 0:
						f[j] += [int(l[j]) - 1]
			fv += [[f[0][0], f[0][i-1], f[0][i]] for i in range(2,len(f[0]))]
	v  = torch.from_numpy(np.array(v, np.float32))
	fv = torch.from_numpy(np.array(fv, np.int64))
	return	v, fv

class FaceDataset(torch.utils.data.Dataset):

    def __init__(self, **kwargs):
        '''
        kwargs:
        conf: dataset config
        '''
        super(FaceDataset, self).__init__()
        
        self.conf = kwargs['conf']
        self.img_res = self.conf['img_res']
        self.total_pixels = self.img_res[0] * self.img_res[1]
        self.num_id = 0
        self.white_flag = self.conf['white_flag']

        self.sampling_idx = None
        self.scan_id = self.conf.get_int('scan_id')
        self.cam_scale = self.conf.get_float('cam_scale')
        self.data_dir = self.conf['data_dir']
        assert os.path.exists(self.data_dir), "Data directory is empty"
        image_dir = self.data_dir
        # there are 15 wrinkle id and 15 smooth id in base_id.txt
        pr_30id_path = self.conf.get_string('id_path', default='./confs/id.txt')
        id_list = np.loadtxt(pr_30id_path, dtype=np.str_).tolist()
        id_list.sort()

        self.intrinsics_all = []
        self.pose_all = []
        self.rgb_images_path = []
        self.rgb_images_all = []
        self.ids = []
        self.names = []
        self.id_list = id_list
        print('PR-facedataset load {} identities: {}'.format(len(id_list), id_list))
        init_view_path = self.conf.get_string('init_view_path', default=None)
        sparse_view_num = self.conf.get_int('sparse_view_num', default=-1)
    
        # fine-tune config
        self.fine_tune_flag = self.conf.get_bool('fine_tune_flag',default=False)
        if self.fine_tune_flag:
            print('for fine-tune, id will be added in previous id_list as last item')
            id_list.append(str(self.scan_id))
            # select random view for fine-tune
            view_array = np.arange(1, 31)
            sparse_view_num = self.conf.get_int('fine_tune_view')
            select_view = np.random.choice(view_array, sparse_view_num, replace=False).tolist()
            if 15 not in select_view:
                select_view[-1]=15
            print('{} fine-tune view: {}'.format(self.scan_id,select_view))
        
            
        if sparse_view_num != -1:
            assert init_view_path != None, print('there is no init_view_path specified, when assign a sparse_view_num')
            with open(init_view_path,'r') as f:
                sparse_info = json.load(f)
                assert sparse_info['sparse_view_num'] == sparse_view_num, print('load sparse view wrong')
                sparse_info = sparse_info['info']
    
        for id in id_list:
            instance_image_path = os.path.join(image_dir, id)
            instance_list = os.listdir(instance_image_path)
            if self.scan_id != -1 and not str(self.scan_id) == id:
                # if given a scan_id to specify, we filter other id
               continue
            if sparse_view_num != -1:
                if init_view_path is None:
                    view_array = np.array([int(i[:-4]) for i in instance_list if 'png' in i[-4:]])
                    sparse_view_list = np.random.choice(view_array, sparse_view_num, replace=False).tolist()
                    if 15 not in sparse_view_list:
                        # make sure the frontal face image in datset
                        sparse_view_list[-1] = 15
                else:
                    sparse_view_list = sparse_info[id]
                if self.fine_tune_flag and init_view_path is None:
                    sparse_view_list = select_view
            else:
                # if no view specified, we use all views~[1, 30] in PR-dataset
                sparse_view_list = [i for i in range(1, 31)]
            print('{} training views: {}'.format(id,sparse_view_list))
            for item in instance_list:
                file_name = os.path.join(instance_image_path, item)
                if '.xml' in item:
                    cameras = load_xml(file_name)
                    pbar = tqdm(cameras.items())
                    if hasattr(pbar, 'set_description'):
                        pbar.set_description('Loading Dataset: %s' % id)    
                    for name, (K, dist, c2w, hw) in pbar:
                        if self.fine_tune_flag and init_view_path is None:
                            if int(name.split('.')[0]) not in select_view:
                                continue
                        fname = os.path.join(instance_image_path, name) 
                        if not os.path.exists(fname):
                            # print(fname,"dont exist")
                            continue
                        if sparse_view_num != -1 and int(name[:-4]) not in sparse_view_list:
                            continue
            
                        self.rgb_images_path.append(fname)
                        rgb = rend_util.load_rgb(fname, white_flag=self.white_flag)
                        rgb = rgb[:3,:,:].reshape(3, -1).transpose(1, 0)
                        rgb = torch.from_numpy(rgb).float()
                        self.rgb_images_all.append(rgb)
                        self.names.append(name)
                        self.ids.append(id_list.index(id))
                        c2w = c2w[...,:3,:4].reshape(-1,3,4)
                        c2w = np.concatenate([c2w, [[[0,0,0,1]]]*len(c2w)], 1)
                        # import pdb; pdb.set_trace()
                        c2w[:,:3,3] = c2w[:,:3,3]*self.cam_scale
                        self.pose_all.append(c2w.astype(np.float32))
                        K = K[...,:2,:3].reshape(-1,2,3)
                        K = np.concatenate([K, [[[0,0,1]]]*len(K)], 1)
                        self.intrinsics_all.append(K.astype(np.float32))         
        self.pose_all = torch.from_numpy(np.concatenate(self.pose_all, 0))
        self.intrinsics_all = torch.from_numpy(np.concatenate(self.intrinsics_all, 0))
        self.ids = torch.from_numpy(np.array(self.ids)) # [1,1,0,0,0,1,2]
        
    def __len__(self):
        return len(self.rgb_images_path)

    def __getitem__(self, idx):
        idx = idx % len(self.rgb_images_path)    
        uv = np.mgrid[0:self.img_res[0], 0:self.img_res[1]].astype(np.int32)
        uv = torch.from_numpy(np.flip(uv, axis=0).copy()).float()
        uv = uv.reshape(2, -1).transpose(1, 0)
        rgb = self.rgb_images_all[idx]
        sample = {
            "uv": uv,
            "intrinsics": self.intrinsics_all[idx],
            "pose": self.pose_all[idx],
            "id": self.ids[idx]
        }
        ground_truth = {
            "rgb": rgb
        }
        if self.sampling_idx is not None:
            ground_truth["rgb"] = rgb[self.sampling_idx, :]
            sample["uv"] = uv[self.sampling_idx, :]
        return idx, sample, ground_truth

    def collate_fn(self, batch_list):
        # get list of dictionaries and returns input, ground_true as dictionary for all batch instances
        batch_list = zip(*batch_list)

        all_parsed = []
        for entry in batch_list:
            if type(entry[0]) is dict:
                # make them all into a new dict
                ret = {}
                for k in entry[0].keys():
                    ret[k] = torch.stack([obj[k] for obj in entry])
                all_parsed.append(ret)
            else:
                all_parsed.append(torch.LongTensor(entry))

        return tuple(all_parsed)
    
    def change_sampling_idx(self, sampling_size):
        self.sampling_size = sampling_size
        if sampling_size == -1:
            self.sampling_idx = None
        else:
            self.sampling_idx = torch.randperm(self.total_pixels)[:sampling_size]
    
    def get_specific_item(self, people_order=-1, cam_id=-1):
        # people_order: [0, 29]
        # cam_id \in [1, 30]
        if people_order == -1:
            person_id = str(self.scan_id) # person_id means: 340 / 393 etc...
            idx = self.id_list.index(person_id)
        else:
            person_id = self.id_list[people_order]
        cam_file = os.path.join(self.data_dir, person_id, 'cameraParameters.xml')
        cameras = load_xml(cam_file)
        c2w_all = []
        intrinsics_all = []
        for name, (K, dist, c2w, hw) in cameras.items():
            if int(name[:2]) == cam_id:
                select_img_path = os.path.join(self.data_dir, person_id, name)
                c2w = c2w[...,:3,:4].reshape(-1,3,4)
                c2w[:,:3,3] = c2w[:,:3,3]*self.cam_scale
                c2w = np.concatenate([c2w, [[[0,0,0,1]]]*len(c2w)], 1)
                c2w_all.append(c2w.astype(np.float32))
                K = K[...,:2,:3].reshape(-1,2,3)
                K = np.concatenate([K, [[[0,0,1]]]*len(K)], 1)
                intrinsics_all.append(K.astype(np.float32))
        c2w_all = torch.from_numpy(np.concatenate(c2w_all, 0))
        intrinsics_all = torch.from_numpy(np.concatenate(intrinsics_all, 0))
        idx = torch.from_numpy(np.array([people_order]))
        rgb = rend_util.load_rgb(select_img_path, white_flag=self.white_flag)
        rgb = rgb[:3,:,:].reshape(3, -1).transpose(1, 0)
        rgb = torch.from_numpy(rgb).float()
        return c2w_all, intrinsics_all, idx, rgb.unsqueeze(0)

    def get_scale_mat(self):
        print("eval call get_scale_mat in FaceDataset")
        return None 
    
    def get_id_list(self):
        return self.id_list
    
    def get_novel_pose_between(self, idx_0, idx_1, i, n_frames=60):
        ratio = np.sin(((i / n_frames) - 0.5) * np.pi) * 0.5 + 0.5
        pose_0 = self.pose_all[idx_0].numpy()
        pose_1 = self.pose_all[idx_1].numpy()
        pose_0 = np.linalg.inv(pose_0)
        pose_1 = np.linalg.inv(pose_1)
        rot_0 = pose_0[:3, :3]
        rot_1 = pose_1[:3, :3]
        rots = Rot.from_matrix(np.stack([rot_0, rot_1]))
        key_times = [0, 1]
        slerp = Slerp(key_times, rots)
        rot = slerp(ratio)
        pose = np.diag([1.0, 1.0, 1.0, 1.0])
        pose = pose.astype(np.float32)
        pose[:3, :3] = rot.as_matrix()
        pose[:3, 3] = ((1.0 - ratio) * pose_0 + ratio * pose_1)[:3, 3]
        pose = np.linalg.inv(pose)

        return torch.from_numpy(pose)

```

## Source file: code/datasets/facescape_dataset.py

{
  "path": "code/datasets/facescape_dataset.py",
  "blob_sha": "5e065d6eebb3f7f96c3cd539419b083677339d3a",
  "sha256": "2a2172c64e7d354bb609a6dcb905546d054d1f77fe3fc98ac58bfbc6a4d72d47",
  "lines": 246,
  "bytes": 9487,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 3,
    "jaw": 1,
    "mouth": 2,
    "rig": 2,
    "face": 1,
    "lip": 2
  },
  "url": "https://github.com/xubaixinxbx/3dheads/blob/cbb9ab686149fda16a67e5756bc0f360b7e7222a/code/datasets/facescape_dataset.py",
  "function_signature_candidates": [
    {
      "line": 13,
      "text": "def load_rgb_resize(path, factor):"
    },
    {
      "line": 28,
      "text": "def load_mask_resize(path, factor):"
    },
    {
      "line": 40,
      "text": "def __init__(self,**kwargs):"
    },
    {
      "line": 95,
      "text": "def __len__(self):"
    },
    {
      "line": 98,
      "text": "def __getitem__(self, idx):"
    },
    {
      "line": 124,
      "text": "def collate_fn(self, batch_list):"
    },
    {
      "line": 141,
      "text": "def change_sampling_idx(self, sampling_size):"
    },
    {
      "line": 156,
      "text": "def get_scale_mat(self):"
    },
    {
      "line": 159,
      "text": "def read_single_dataset(self, image_dir, params_dir, ply_path, mask_dir, i, j, factor, expression, split):"
    }
  ]
}

```text

import os
import torch
import numpy as np

import utils.general as utils
from utils import rend_util
from glob import glob
import cv2
import json
import trimesh
import skimage

def load_rgb_resize(path, factor):
    img = cv2.imread(path)
    img = cv2.cvtColor(img, cv2.COLOR_BGR2RGB)
    H = img.shape[0]
    W = img.shape[1]

    img = cv2.resize(img, (W // factor, H // factor), interpolation=cv2.INTER_AREA)
    img = skimage.img_as_float32(img)
    # pixel values between [-1,1]
    # img -= 0.5
    # img *= 2.
    img = img.transpose(2, 0, 1)
    return img


def load_mask_resize(path, factor):
    alpha = cv2.imread(path, 0)
    H = alpha.shape[0]
    W = alpha.shape[1]
    alpha = cv2.resize(alpha, (W // factor, H // factor), interpolation=cv2.INTER_AREA)
    alpha = skimage.img_as_float32(alpha)
    object_mask = alpha > 0.5
    #object_mask = alpha < 0.5
    return object_mask

class SceneDataset(torch.utils.data.Dataset):

    def __init__(self,**kwargs):
        
        expression = ['neutral', 'smile', 'mouth_stretch', 'anger', 'jaw_left', 'jaw_right', 'jaw_forward',
                'mouth_left', 'mouth_right', 'dimpler', 'chin_raiser',
                'lip_puckerer', 'lip_funneler', 'sadness', 'lip_roll', 'grin', 'cheek_blowing', 'eye_closed',
                'brow_raiser', 'brow_lower']
        id_list = [2, 3, 4, 5, 6, 7, 8, 9, 10, 11]
        self.id_list = id_list
        self.conf = kwargs['conf']
        self.img_res = self.conf['img_res']
        
        data_dir = self.conf['data_dir']
        scan_id = self.conf['scan_id']
        views = self.conf['views']
        split = self.conf['split']
        factor = self.conf['factor']
        self.st1_flag = self.conf.get_bool('st1_flag', default=True)

        i = scan_id
        j = facescape_exp = 16
        self.views = views
        self.instance_dir = data_dir
        image_dir = '{0}/image'.format(self.instance_dir)
        mask_dir = '{0}/mask'.format(self.instance_dir)
        params_dir = '{0}/image'.format(self.instance_dir)
        ply_path = '{0}/model'.format(self.instance_dir)
        
        self.intrinsics_all = []
        self.pose_all = []
        self.rgb_images = []
        self.h = []
        self.w = []
        self.object_masks = []
        self.id = []
        self.exp = []
        self.exp_num = []
        self.sample_list = []
        self.have_sample = []
        self.ids = []
        self.names = []
        if self.st1_flag:
            for idx in id_list:
                if scan_id!=-1 and idx != scan_id:
                    continue
                self.read_single_dataset(image_dir, params_dir, ply_path, mask_dir, idx, j, factor, expression, split)        
        else:
            self.id_list = [scan_id]
            self.read_single_dataset(image_dir, params_dir, ply_path, mask_dir, scan_id, j, factor, expression, split)        
        self.n_images = len(self.pose_all)
        print('image num: {0}'.format(self.n_images))
        self.ids = torch.from_numpy(np.array(self.ids))
        self.total_pixels = None
        self.img_res = None
        self.sampling_idx = None

    def __len__(self):
        return self.n_images

    def __getitem__(self, idx):
        uv = np.mgrid[0:self.h[idx], 0:self.w[idx]].astype(np.int32)
        uv = torch.from_numpy(np.flip(uv, axis=0).copy()).float()
        uv = uv.reshape(2, -1).transpose(1, 0)

        sample = {
            "uv": uv,
            "intrinsics": self.intrinsics_all[idx],
            "pose": self.pose_all[idx],
            "id": self.ids[idx]
        }

        ground_truth = {
            "rgb": self.rgb_images[idx],
            'hw': torch.from_numpy(np.array([self.h[idx], self.w[idx]])),
            'mask': self.object_masks[idx]
        }

        if self.sampling_idx is not None:
            s_idx = self.sampling_idx[idx]
            ground_truth["rgb"] = self.rgb_images[idx][s_idx, :]
            sample["uv"] = uv[s_idx, :]
            ground_truth['mask'] = ground_truth['mask'][s_idx]

        return idx, sample, ground_truth

    def collate_fn(self, batch_list):
        # get list of dictionaries and returns input, ground_true as dictionary for all batch instances
        batch_list = zip(*batch_list)

        all_parsed = []
        for entry in batch_list:
            if type(entry[0]) is dict:
                # make them all into a new dict
                ret = {}
                for k in entry[0].keys():
                    ret[k] = torch.stack([obj[k] for obj in entry])
                all_parsed.append(ret)
            else:
                all_parsed.append(torch.LongTensor(entry))

        return tuple(all_parsed)

    def change_sampling_idx(self, sampling_size):
        # if sampling_size == -1:
        #     self.sampling_idx = None
        # else:
        #     self.sampling_idx = torch.randperm(self.total_pixels)[:sampling_size]
        if sampling_size == -1:
            self.sampling_idx = None
        else:
            self.sampling_idx = []
            for i in range(self.n_images):
                total = self.sample_list[i].shape[0]
                s_idx = torch.randperm(total)[:sampling_size]
                self.have_sample[i][self.sample_list[i][s_idx]] = 0
                self.sampling_idx.append(self.sample_list[i][s_idx])

    def get_scale_mat(self):
        return np.load(self.cam_file)['scale_mat_0']

    def read_single_dataset(self, image_dir, params_dir, ply_path, mask_dir, i, j, factor, expression, split):
        land_dir = os.path.join(self.instance_dir, 'landmark_indices.npz')
        landmark = np.load(land_dir)['v10'][30]
        with open(os.path.join(self.instance_dir, f"Rt_scale_dict.json"), 'r') as f:
            Rt_scale_dict = json.load(f)
        if split == 'train':
            if self.st1_flag:
                i_dir = os.path.join(image_dir, str(i), '{0}_{1}'.format(j, expression[j-1]), split) # for template 
            else:
                i_dir = os.path.join(image_dir, str(i), '{0}_{1}'.format(j, expression[j-1]), split + f'_{self.views}') # for stage2
        elif split == 'test':
            i_dir = os.path.join(image_dir, str(i), '{0}_{1}'.format(j, expression[j-1]), split) # for test
            
        par_dir = os.path.join(params_dir, str(i), '{0}_{1}'.format(j, expression[j-1]))
        with open(os.path.join(par_dir, f"params.json"), 'r') as f:
            meta = json.load(f)
        image_path = os.listdir(i_dir)
        image_path.sort()
        scale = Rt_scale_dict['%d' % i]['%d' % j][0]
        Rt = np.array(Rt_scale_dict['%d' % i]['%d' % j][1])
        
        ply_name = '{0}_{1}.obj'.format(j, expression[j - 1])
        ply = os.path.join(ply_path, str(i), ply_name)
        mview_mesh = trimesh.load(ply, process=False, maintain_order=True)
        # align multi-view model to TU model
        point = mview_mesh.vertices[landmark]
        for p in image_path:
            if not p.endswith('.jpg'): continue
            self.id.append(torch.tensor(0))
            self.exp.append(torch.tensor(0))
            num = p[:-4]
            self.exp_num.append(int(num))
            r_path = os.path.join(i_dir, p)
            rgb = load_rgb_resize(r_path, factor)

            self.h.append(rgb.shape[1])
            self.w.append(rgb.shape[2])
            rgb = rgb.reshape(3, -1).transpose(1, 0)
            self.rgb_images.append(torch.from_numpy(rgb).float())

            m_path = os.path.join(mask_dir, str(i), '{0}_{1}'.format(j, expression[j-1]), split, num + '.jpg')
            object_mask = load_mask_resize(m_path, factor)
            h, w = object_mask.shape
            x_axis, y_axis = np.where(object_mask == True)
            x_min = max(x_axis.min() - 30, 0)
            x_max = min(x_axis.max() + 30, h - 1)
            y_min = max(y_axis.min() - 30, 0)
            y_max = min(y_axis.max() + 30, w - 1)
            sam_list = []
            for x in range(x_min, x_max + 1):
                for y in range(y_min, y_max + 1):
                    sam_list.append(x * w + y)
            # train with the pixels in the mask area
            self.sample_list.append(torch.tensor(sam_list))
            self.have_sample.append(torch.zeros(h * w) + 255)
            object_mask = object_mask.reshape(-1)
            self.object_masks.append(torch.from_numpy(object_mask).bool())

            # obtain camera's intrinsics and extrinsics
            K = np.array(meta[num + '_K'])
            K[:2, :3] /= factor
            pose = np.array(meta[num + '_Rt'])[:3, :4]
            R_cv2gl = np.array([[1, 0, 0], [0, -1, 0], [0, 0, -1]])
            pose = R_cv2gl.dot(pose)
            cam_pose = np.eye(4)
            cam_pose[:3, :3] = pose[:3, :3].T
            cam_pose[:3, 3] = -pose[:3, :3].T.dot(pose[:, 3])

            rays_o = cam_pose[:3, 3]
            rays_o *= scale
            rays_o = np.dot(Rt[:3, :3], rays_o.T).T + Rt[:3, 3]
            rays_o -= point
            rays_o[2] += 40
            rays_o /= 100

            r = cam_pose[:3, :3]
            r = np.dot(Rt[:3, :3], r)

            cam_pose[:3, 3] = rays_o
            cam_pose[:3, :3] = r
            # cam_pose[:3,:3] = 9
            intrinsics = np.eye(4)
            intrinsics[:3, :3] = K

            self.pose_all.append(torch.from_numpy(cam_pose).float())
            self.intrinsics_all.append(torch.from_numpy(intrinsics).float())
            self.ids.append(self.id_list.index(i))
            self.names.append(str(p.split('/')[-1]))


```

## Source file: code/model/loss.py

{
  "path": "code/model/loss.py",
  "blob_sha": "22415589b6f4945012c92ef28616135ca1fc6a5e",
  "sha256": "098181df83154df75806dd96cde0f399a3bed04690975265e7f72b17b2a9ead2",
  "lines": 116,
  "bytes": 4949,
  "selection_score": 0,
  "keyword_line_counts": {
    "deform": 12
  },
  "url": "https://github.com/xubaixinxbx/3dheads/blob/cbb9ab686149fda16a67e5756bc0f360b7e7222a/code/model/loss.py",
  "function_signature_candidates": [
    {
      "line": 6,
      "text": "def __init__(self, **kwargs):"
    },
    {
      "line": 24,
      "text": "def get_rgb_loss(self,rgb_values, rgb_gt):"
    },
    {
      "line": 29,
      "text": "def get_eikonal_loss(self, grad_theta):"
    },
    {
      "line": 33,
      "text": "def get_disp_loss(self, delta_s):"
    },
    {
      "line": 37,
      "text": "def get_disp_grad_loss(self, grad_delta_s):"
    },
    {
      "line": 41,
      "text": "def get_latent_code_loss(self, code):"
    },
    {
      "line": 46,
      "text": "def forward(self, model_outputs, ground_truth):"
    }
  ]
}

```text

import torch
from torch import nn
import utils.general as utils

class VolSDFLoss(nn.Module):
    def __init__(self, **kwargs):
        super().__init__()
        self.eikonal_weight = kwargs['eikonal_weight']
        self.rgb_loss = utils.get_class(kwargs['rgb_loss'])(reduction='mean')
        
        self.deform_weight = kwargs.get('deform_weight', 0)
        self.deform_grad_weight = kwargs.get('deform_grad_weight', 0)
        
        self.disp_weight = kwargs.get('disp_weight', 0)
        self.disp_loss = utils.get_class(kwargs['disp_loss'])(reduction='mean')
        self.disp_grad_weight = kwargs.get('disp_grad_weight', 0)
        self.disp_grad_loss = utils.get_class(kwargs['disp_grad_loss'])(reduction='mean')
        
        self.shape_code_weight = kwargs.get('shape_code_weight',0)
        self.color_code_weight = kwargs.get('color_code_weight',0)
        self.latent_code_loss = utils.get_class(kwargs['latent_code_loss'])(reduction='mean')


    def get_rgb_loss(self,rgb_values, rgb_gt):
        rgb_gt = rgb_gt.reshape(-1, 3)
        rgb_loss = self.rgb_loss(rgb_values, rgb_gt)
        return rgb_loss

    def get_eikonal_loss(self, grad_theta):
        eikonal_loss = ((grad_theta.norm(2, dim=1) - 1) ** 2).mean()
        return eikonal_loss
    
    def get_disp_loss(self, delta_s):
        correct_loss = self.disp_loss(delta_s, torch.zeros_like(delta_s).cuda().float())
        return correct_loss
    
    def get_disp_grad_loss(self, grad_delta_s):
        disp_grad_loss = self.disp_grad_loss(grad_delta_s, torch.zeros_like(grad_delta_s).cuda().float())
        return disp_grad_loss
    
    def get_latent_code_loss(self, code):
        latent_code_loss = self.latent_code_loss(code, torch.zeros_like(code).cuda().float())
        return latent_code_loss


    def forward(self, model_outputs, ground_truth):
        if 'rgb' in ground_truth:
            rgb_gt = ground_truth['rgb'].cuda()
            rgb_loss = self.get_rgb_loss(model_outputs['rgb_values'], rgb_gt)
        else:
            rgb_loss = torch.tensor(0.0).cuda().float()

        # eik loss
        if 'grad_theta' in model_outputs:
            eikonal_loss = self.get_eikonal_loss(model_outputs['grad_theta'])
        else:
            eikonal_loss = torch.tensor(0.0).cuda().float()
        
        # deform loss
        if 'grad_deform' in model_outputs:
            deform_grad_loss = model_outputs['grad_deform'].norm(2, dim=-1).mean()
        else:
            deform_grad_loss = torch.tensor(0.0).cuda().float()
        if 'delta_x' in model_outputs:
            deform_loss = model_outputs['delta_x'].norm(2, dim=-1).mean()
        else:
            deform_loss = torch.tensor(0.0).cuda().float()
        
        # displacement loss
        if 'delta_sdf' in model_outputs and model_outputs['delta_sdf'] is not None:
            delta_sdf = model_outputs['delta_sdf']
            delta_sdf_loss = self.get_disp_loss(delta_sdf)
        else:
            delta_sdf_loss = torch.tensor(0.0).cuda().float()
        if 'delta_sdf_grad' in model_outputs and model_outputs['delta_sdf_grad'] is not None:
            delta_sdf_grad = model_outputs['delta_sdf_grad']
            delta_sdf_grad_loss = self.get_disp_grad_loss(delta_sdf_grad)
        else:
            delta_sdf_grad_loss = torch.tensor(0.0).cuda().float()
        
        # latent code loss
        if 'code' in model_outputs:
            if 'shape_code' in model_outputs['code']:
                shape_code_loss = self.get_latent_code_loss(model_outputs['code']['shape_code'])
            else:
                shape_code_loss = torch.tensor(0.0).cuda().float()
            if 'color_code' in model_outputs['code']:
                color_code_loss = self.get_latent_code_loss(model_outputs['code']['color_code'])
            else:
                color_code_loss = torch.tensor(0.0).cuda().float()
        else:
            shape_code_loss = torch.tensor(0.0).cuda().float()
            color_code_loss = torch.tensor(0.0).cuda().float()
        
        loss = rgb_loss + \
               self.eikonal_weight * eikonal_loss + \
                self.disp_weight * delta_sdf_loss + \
                    self.disp_grad_weight * delta_sdf_grad_loss + \
                        self.shape_code_weight * shape_code_loss + \
                            self.color_code_weight * color_code_loss + \
                                self.deform_grad_weight * deform_grad_loss + \
                                    self.deform_weight * deform_loss

        output = {
            'loss': loss,
            'rgb_loss': rgb_loss,
            'eikonal_loss': eikonal_loss,
            'disp_loss': delta_sdf_loss,
            'disp_grad_loss': delta_sdf_grad_loss,
            'shape_code_loss': shape_code_loss,
            'color_code_loss': color_code_loss,
            'deform_grad_loss': deform_grad_loss,
            'deform_loss': deform_loss
        }

        return output


```

## Source file: code/utils/plots.py

{
  "path": "code/utils/plots.py",
  "blob_sha": "6d46e7488866b630e05b204a49472aa0bdedf230",
  "sha256": "d2039d363b0507b9c89e1552832d6220ac67394730cbdecf3c149476b22cd6ab",
  "lines": 452,
  "bytes": 19406,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 2,
    "rig": 1,
    "face": 32
  },
  "url": "https://github.com/xubaixinxbx/3dheads/blob/cbb9ab686149fda16a67e5756bc0f360b7e7222a/code/utils/plots.py",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "def sdf_func(x, shape_code, net):"
    },
    {
      "line": 17,
      "text": "def plot_mesh(net, shape_idx, path, epoch, resolution, grid_boundary, level=0,cam_scale=1.0):"
    },
    {
      "line": 30,
      "text": "def plot(net, indices, plot_data, path, epoch, img_res, plot_nimgs, resolution, grid_boundary,\\"
    },
    {
      "line": 77,
      "text": "def get_3D_scatter_trace(points, name='', size=3, surfacecolor='#bf2694', caption=None, opacity=1.0):"
    },
    {
      "line": 99,
      "text": "def get_3D_quiver_trace(points, directions, color='#bd1540', name=''):"
    },
    {
      "line": 123,
      "text": "def get_surface_trace(path, epoch, sdf, resolution=100, grid_boundary=[-2.0, 2.0], return_mesh=False, level=0, cam_scale=1.0):"
    },
    {
      "line": 170,
      "text": "def get_surface_high_res_mesh(sdf, resolution=100, grid_boundary=[-2.0, 2.0], level=0, take_components=True):"
    },
    {
      "line": 262,
      "text": "def get_surface_by_grid(grid_params, sdf, resolution=100, level=0, higher_res=False):"
    },
    {
      "line": 363,
      "text": "def get_grid_uniform(resolution, grid_boundary=[-2.0, 2.0]):"
    },
    {
      "line": 376,
      "text": "def get_grid(points, resolution, input_min=None, input_max=None, eps=0.1):"
    },
    {
      "line": 410,
      "text": "def plot_normal_maps(normal_maps, path, epoch, plot_nrow, img_res, writer=None):"
    },
    {
      "line": 427,
      "text": "def plot_images(rgb_points, ground_true, path, epoch, plot_nrow, img_res, writer=None, no_gt=False):"
    },
    {
      "line": 450,
      "text": "def lin2img(tensor, img_res):"
    }
  ]
}

```text

import plotly.graph_objs as go
import plotly.offline as offline
from plotly.subplots import make_subplots
import numpy as np
import torch
from skimage import measure
import torchvision
import trimesh
from PIL import Image

from utils import rend_util


def sdf_func(x, shape_code, net):
    return net.get_sdf_vals(x,shape_code)

def plot_mesh(net, shape_idx, path, epoch, resolution, grid_boundary, level=0,cam_scale=1.0):
    print('level=',level)
    # plot surface 
    shape_code = net.get_embedding(shape_idx=shape_idx,color_idx=shape_idx)['shape_code']
    surface_traces = get_surface_trace(path=path,
                                       epoch=epoch,
                                       sdf=lambda x: sdf_func(x, shape_code, net),
                                       resolution=resolution,
                                       grid_boundary=grid_boundary,
                                       level=level,
                                       cam_scale=cam_scale
                                       )

def plot(net, indices, plot_data, path, epoch, img_res, plot_nimgs, resolution, grid_boundary,\
        cam_scale=1.0, writer=None, plot_template=False, plot_delta_sdf=True,\
        level=0, only_image=False, no_points_on_plotly=True):
    
    rgb, normal = None, None
    if plot_data is not None:
        if 'pose' in plot_data:
            cam_loc, cam_dir = rend_util.get_camera_for_plot(plot_data['pose'])
            rgb=plot_images(plot_data['rgb_eval'], plot_data['rgb_gt'], path, str(epoch)+'_'+str(plot_data['id'].item()), plot_nimgs, img_res, writer,no_gt=False)
            normal=plot_normal_maps(plot_data['normal_map'], path, str(epoch)+'_'+str(plot_data['id'].item()), plot_nimgs, img_res, writer)
        
        if only_image:
            return rgb,normal

    data = []
    print('level=',level)
    print('plot temaplte={}, delta_sdf={}'.format(plot_template, plot_delta_sdf))
    # plot surface 
    shape_code = net.get_embedding(shape_idx=plot_data['id'],color_idx=plot_data['id'])['shape_code']
    surface_traces = get_surface_trace(path=path,
                                       epoch=epoch,
                                       sdf=lambda x: sdf_func(x, shape_code, net),
                                       resolution=resolution,
                                       grid_boundary=grid_boundary,
                                       level=level,
                                       cam_scale=cam_scale
                                       )

    if surface_traces is not None:
        data.append(surface_traces[0])
    if no_points_on_plotly:
        return rgb, normal
    # plot cameras locations
    if plot_data is not None:
        for i, loc, dir in zip(indices, cam_loc, cam_dir):
            data.append(get_3D_quiver_trace(loc.unsqueeze(0), dir.unsqueeze(0), name='camera_{0}'.format(i)))

    fig = go.Figure(data=data)
    scene_dict = dict(xaxis=dict(range=[-6, 6], autorange=False),
                      yaxis=dict(range=[-6, 6], autorange=False),
                      zaxis=dict(range=[-6, 6], autorange=False),
                      aspectratio=dict(x=1, y=1, z=1))
    fig.update_layout(scene=scene_dict, width=1200, height=1200, showlegend=True)
    filename = '{0}/surface_{1}.html'.format(path, epoch)
    offline.plot(fig, filename=filename, auto_open=False)


def get_3D_scatter_trace(points, name='', size=3, surfacecolor='#bf2694', caption=None, opacity=1.0):
    assert points.shape[1] == 3, "3d scatter plot input points are not correctely shaped "
    assert len(points.shape) == 2, "3d scatter plot input points are not correctely shaped "

    trace = go.Scatter3d(
        x=points[:, 0].cpu(),
        y=points[:, 1].cpu(),
        z=points[:, 2].cpu(),
        surfacecolor=surfacecolor,
        mode='markers',
        name=name,
        marker=dict(
            size=size,
            line=dict(
                width=2,
            ),
            opacity=opacity,
        ), text=caption)

    return trace


def get_3D_quiver_trace(points, directions, color='#bd1540', name=''):
    assert points.shape[1] == 3, "3d cone plot input points are not correctely shaped "
    assert len(points.shape) == 2, "3d cone plot input points are not correctely shaped "
    assert directions.shape[1] == 3, "3d cone plot input directions are not correctely shaped "
    assert len(directions.shape) == 2, "3d cone plot input directions are not correctely shaped "

    trace = go.Cone(
        name=name,
        x=points[:, 0].cpu(),
        y=points[:, 1].cpu(),
        z=points[:, 2].cpu(),
        u=directions[:, 0].cpu(),
        v=directions[:, 1].cpu(),
        w=directions[:, 2].cpu(),
        sizemode='absolute',
        sizeref=0.125,
        showscale=False,
        colorscale=[[0, color], [1, color]],
        anchor="tail"
    )
    # print('get cam loc in plotly', points, directions)
    return trace


def get_surface_trace(path, epoch, sdf, resolution=100, grid_boundary=[-2.0, 2.0], return_mesh=False, level=0, cam_scale=1.0):
    grid = get_grid_uniform(resolution, grid_boundary)
    points = grid['grid_points']

    z = []
    for i, pnts in enumerate(torch.split(points, 1000, dim=0)):
        z.append(sdf(pnts).detach().cpu().numpy())
    z = np.concatenate(z, axis=0)

    if (not (np.min(z) > level or np.max(z) < level)):
        print('implict surface cross level')
        z = z.astype(np.float32)

        verts, faces, normals, values = measure.marching_cubes(
            volume=z.reshape(grid['xyz'][1].shape[0], grid['xyz'][0].shape[0],
                             grid['xyz'][2].shape[0]).transpose([1, 0, 2]),
            level=level,
            spacing=(grid['xyz'][0][2] - grid['xyz'][0][1],
                     grid['xyz'][0][2] - grid['xyz'][0][1],
                     grid['xyz'][0][2] - grid['xyz'][0][1]))

        verts = verts + np.array([grid['xyz'][0][0], grid['xyz'][1][0], grid['xyz'][2][0]])

        I, J, K = faces.transpose()
        
        traces = [go.Mesh3d(x=verts[:, 0], y=verts[:, 1], z=verts[:, 2],
                            i=I, j=J, k=K, name='implicit_surface',
                            color='#ffffff', opacity=1.0, flatshading=False,
                            lighting=dict(diffuse=1, ambient=0, specular=0),
                            lightposition=dict(x=0, y=0, z=-1), showlegend=True)]


        meshexport = trimesh.Trimesh(verts, faces, vertex_normals=-normals)
        scale_mat = np.eye(4)
        scale_mat = scale_mat / cam_scale
        scale_mat[3,3] = 1
        meshexport.apply_transform(scale_mat)
        print('mesh has been transformed by a scale_mat cam_scale={}'.format(cam_scale))
        meshexport.export('{0}/surface_{1}.ply'.format(path, epoch), 'ply')

        if return_mesh:
            return meshexport
        return traces
    else:
        print('no surface trace found', np.min(z), np.max(z))
    return None

def get_surface_high_res_mesh(sdf, resolution=100, grid_boundary=[-2.0, 2.0], level=0, take_components=True):
    print('eval resolution:', resolution)
    # get low res mesh to sample point cloud
    grid = get_grid_uniform(100, grid_boundary)
    z = []
    points = grid['grid_points']

    for i, pnts in enumerate(torch.split(points, 100000, dim=0)):
        z.append(sdf(pnts).detach().cpu().numpy())
    z = np.concatenate(z, axis=0)

    z = z.astype(np.float32)

    verts, faces, normals, values = measure.marching_cubes(
        volume=z.reshape(grid['xyz'][1].shape[0], grid['xyz'][0].shape[0],
                         grid['xyz'][2].shape[0]).transpose([1, 0, 2]),
        level=level,
        spacing=(grid['xyz'][0][2] - grid['xyz'][0][1],
                 grid['xyz'][0][2] - grid['xyz'][0][1],
                 grid['xyz'][0][2] - grid['xyz'][0][1]))

    verts = verts + np.array([grid['xyz'][0][0], grid['xyz'][1][0], grid['xyz'][2][0]])

    mesh_low_res = trimesh.Trimesh(verts, faces, vertex_normals=-normals)
    if take_components:
        components = mesh_low_res.split(only_watertight=False)
        areas = np.array([c.area for c in components], dtype=np.float)
        mesh_low_res = components[areas.argmax()]

    recon_pc = trimesh.sample.sample_surface(mesh_low_res, 10000)[0]
    # extents = np.array([0.1,0.1,0.1]).astype(np.float32)
    # recon_pc = trimesh.sample.volume_rectangular(extents, 10000, transform=None)
    # recon_pc = trimesh.sample.sample_surface_even(mesh_low_res, 10000)[0]
    # (n-sample,3) on the surface of mesh
    recon_pc = torch.from_numpy(recon_pc).float().cuda()

    # Center and align the recon pc
    s_mean = recon_pc.mean(dim=0)
    s_cov = recon_pc - s_mean
    s_cov = torch.mm(s_cov.transpose(0, 1), s_cov)

    s_cov_eig = np.linalg.eig(s_cov.cpu().detach().numpy())
    s_cov = torch.from_numpy(s_cov_eig[1])
    s_cov = torch.tensor(s_cov,  dtype=torch.cfloat).cuda()
    vecs = torch.view_as_real(s_cov.transpose(0, 1))[:, :, 0]
    
    # vecs = torch.view_as_real(torch.linalg.eig(s_cov)[1].transpose(0, 1))[:, :, 0]
    if torch.det(vecs) < 0:
        vecs = torch.mm(torch.tensor([[1, 0, 0], [0, 0, 1], [0, 1, 0]]).cuda().float(), vecs)
    helper = torch.bmm(vecs.unsqueeze(0).repeat(recon_pc.shape[0], 1, 1),
                       (recon_pc - s_mean).unsqueeze(-1)).squeeze()

    grid_aligned = get_grid(helper.cpu(), resolution)

    grid_points = grid_aligned['grid_points']

    g = []
    for i, pnts in enumerate(torch.split(grid_points, 100000, dim=0)):
        g.append(torch.bmm(vecs.unsqueeze(0).repeat(pnts.shape[0], 1, 1).transpose(1, 2),
                           pnts.unsqueeze(-1)).squeeze() + s_mean)
    grid_points = torch.cat(g, dim=0)

    # MC to new grid
    points = grid_points
    z = []
    for i, pnts in enumerate(torch.split(points, 100000, dim=0)):
        z.append(sdf(pnts).detach().cpu().numpy())
    z = np.concatenate(z, axis=0)

    meshexport = None
    if (not (np.min(z) > level or np.max(z) < level)):

        z = z.astype(np.float32)

        verts, faces, normals, values = measure.marching_cubes(
            volume=z.reshape(grid_aligned['xyz'][1].shape[0], grid_aligned['xyz'][0].shape[0],
                             grid_aligned['xyz'][2].shape[0]).transpose([1, 0, 2]),
            level=level,
            spacing=(grid_aligned['xyz'][0][2] - grid_aligned['xyz'][0][1],
                     grid_aligned['xyz'][0][2] - grid_aligned['xyz'][0][1],
                     grid_aligned['xyz'][0][2] - grid_aligned['xyz'][0][1]))

        verts = torch.from_numpy(verts).cuda().float()
        verts = torch.bmm(vecs.unsqueeze(0).repeat(verts.shape[0], 1, 1).transpose(1, 2),
                   verts.unsqueeze(-1)).squeeze()
        verts = (verts + grid_points[0]).cpu().numpy()

        meshexport = trimesh.Trimesh(verts, faces, vertex_normals=-normals)

    return meshexport


def get_surface_by_grid(grid_params, sdf, resolution=100, level=0, higher_res=False):
    grid_params = grid_params * [[1.5], [1.0]]

    # params = PLOT_DICT[scan_id]
    input_min = torch.tensor(grid_params[0]).float()
    input_max = torch.tensor(grid_params[1]).float()

    if higher_res:
        # get low res mesh to sample point cloud
        grid = get_grid(None, 100, input_min=input_min, input_max=input_max, eps=0.0)
        z = []
        points = grid['grid_points']

        for i, pnts in enumerate(torch.split(points, 100000, dim=0)):
            z.append(sdf(pnts).detach().cpu().numpy())
        z = np.concatenate(z, axis=0)

        z = z.astype(np.float32)

        verts, faces, normals, values = measure.marching_cubes(
            volume=z.reshape(grid['xyz'][1].shape[0], grid['xyz'][0].shape[0],
                             grid['xyz'][2].shape[0]).transpose([1, 0, 2]),
            level=level,
            spacing=(grid['xyz'][0][2] - grid['xyz'][0][1],
                     grid['xyz'][0][2] - grid['xyz'][0][1],
                     grid['xyz'][0][2] - grid['xyz'][0][1]))

        verts = verts + np.array([grid['xyz'][0][0], grid['xyz'][1][0], grid['xyz'][2][0]])

        mesh_low_res = trimesh.Trimesh(verts, faces, normals)
        components = mesh_low_res.split(only_watertight=False)
        areas = np.array([c.area for c in components], dtype=np.float)
        mesh_low_res = components[areas.argmax()]

        recon_pc = trimesh.sample.sample_surface(mesh_low_res, 10000)[0]
        recon_pc = torch.from_numpy(recon_pc).float().cuda()

        # Center and align the recon pc
        s_mean = recon_pc.mean(dim=0)
        s_cov = recon_pc - s_mean
        s_cov = torch.mm(s_cov.transpose(0, 1), s_cov)
        vecs = torch.view_as_real(torch.linalg.eig(s_cov)[1].transpose(0, 1))[:, :, 0]
        if torch.det(vecs) < 0:
            vecs = torch.mm(torch.tensor([[1, 0, 0], [0, 0, 1], [0, 1, 0]]).cuda().float(), vecs)
        helper = torch.bmm(vecs.unsqueeze(0).repeat(recon_pc.shape[0], 1, 1),
                           (recon_pc - s_mean).unsqueeze(-1)).squeeze()

        grid_aligned = get_grid(helper.cpu(), resolution, eps=0.01)
    else:
        grid_aligned = get_grid(None, resolution, input_min=input_min, input_max=input_max, eps=0.0)

    grid_points = grid_aligned['grid_points']

    if higher_res:
        g = []
        for i, pnts in enumerate(torch.split(grid_points, 100000, dim=0)):
            g.append(torch.bmm(vecs.unsqueeze(0).repeat(pnts.shape[0], 1, 1).transpose(1, 2),
                               pnts.unsqueeze(-1)).squeeze() + s_mean)
        grid_points = torch.cat(g, dim=0)

    # MC to new grid
    points = grid_points
    z = []
    for i, pnts in enumerate(torch.split(points, 100000, dim=0)):
        z.append(sdf(pnts).detach().cpu().numpy())
    z = np.concatenate(z, axis=0)

    meshexport = None
    if (not (np.min(z) > level or np.max(z) < level)):

        z = z.astype(np.float32)

        verts, faces, normals, values = measure.marching_cubes(
            volume=z.reshape(grid_aligned['xyz'][1].shape[0], grid_aligned['xyz'][0].shape[0],
                             grid_aligned['xyz'][2].shape[0]).transpose([1, 0, 2]),
            level=level,
            spacing=(grid_aligned['xyz'][0][2] - grid_aligned['xyz'][0][1],
                     grid_aligned['xyz'][0][2] - grid_aligned['xyz'][0][1],
                     grid_aligned['xyz'][0][2] - grid_aligned['xyz'][0][1]))

        if higher_res:
            verts = torch.from_numpy(verts).cuda().float()
            verts = torch.bmm(vecs.unsqueeze(0).repeat(verts.shape[0], 1, 1).transpose(1, 2),
                       verts.unsqueeze(-1)).squeeze()
            verts = (verts + grid_points[0]).cpu().numpy()
        else:
            verts = verts + np.array([grid_aligned['xyz'][0][0], grid_aligned['xyz'][1][0], grid_aligned['xyz'][2][0]])

        meshexport = trimesh.Trimesh(verts, faces, normals)

        # CUTTING MESH ACCORDING TO THE BOUNDING BOX
        if higher_res:
            bb = grid_params
            transformation = np.eye(4)
            transformation[:3, 3] = (bb[1,:] + bb[0,:])/2.
            bounding_box = trimesh.creation.box(extents=bb[1,:] - bb[0,:], transform=transformation)

            meshexport = meshexport.slice_plane(bounding_box.facets_origin, -bounding_box.facets_normal)

    return meshexport

def get_grid_uniform(resolution, grid_boundary=[-2.0, 2.0]):
    x = np.linspace(grid_boundary[0], grid_boundary[1], resolution)
    y = x
    z = x

    xx, yy, zz = np.meshgrid(x, y, z)
    grid_points = torch.tensor(np.vstack([xx.ravel(), yy.ravel(), zz.ravel()]).T, dtype=torch.float)

    return {"grid_points": grid_points.cuda(),
            "shortest_axis_length": 2.0,
            "xyz": [x, y, z],
            "shortest_axis_index": 0}

def get_grid(points, resolution, input_min=None, input_max=None, eps=0.1):
    if input_min is None or input_max is None:
        input_min = torch.min(points, dim=0)[0].squeeze().numpy()
        input_max = torch.max(points, dim=0)[0].squeeze().numpy()

    bounding_box = input_max - input_min
    shortest_axis = np.argmin(bounding_box)
    if (shortest_axis == 0):
        x = np.linspace(input_min[shortest_axis] - eps,
                        input_max[shortest_axis] + eps, resolution)
        length = np.max(x) - np.min(x)
        y = np.arange(input_min[1] - eps, input_max[1] + length / (x.shape[0] - 1) + eps, length / (x.shape[0] - 1))
        z = np.arange(input_min[2] - eps, input_max[2] + length / (x.shape[0] - 1) + eps, length / (x.shape[0] - 1))
    elif (shortest_axis == 1):
        y = np.linspace(input_min[shortest_axis] - eps,
                        input_max[shortest_axis] + eps, resolution)
        length = np.max(y) - np.min(y)
        x = np.arange(input_min[0] - eps, input_max[0] + length / (y.shape[0] - 1) + eps, length / (y.shape[0] - 1))
        z = np.arange(input_min[2] - eps, input_max[2] + length / (y.shape[0] - 1) + eps, length / (y.shape[0] - 1))
    elif (shortest_axis == 2):
        z = np.linspace(input_min[shortest_axis] - eps,
                        input_max[shortest_axis] + eps, resolution)
        length = np.max(z) - np.min(z)
        x = np.arange(input_min[0] - eps, input_max[0] + length / (z.shape[0] - 1) + eps, length / (z.shape[0] - 1))
        y = np.arange(input_min[1] - eps, input_max[1] + length / (z.shape[0] - 1) + eps, length / (z.shape[0] - 1))

    xx, yy, zz = np.meshgrid(x, y, z)
    grid_points = torch.tensor(np.vstack([xx.ravel(), yy.ravel(), zz.ravel()]).T, dtype=torch.float).cuda()
    return {"grid_points": grid_points,
            "shortest_axis_length": length,
            "xyz": [x, y, z],
            "shortest_axis_index": shortest_axis}


def plot_normal_maps(normal_maps, path, epoch, plot_nrow, img_res, writer=None):
    normal_maps_plot = lin2img(normal_maps, img_res)

    tensor = torchvision.utils.make_grid(normal_maps_plot,
                                         scale_each=False,
                                         normalize=False,
                                         nrow=plot_nrow).cpu().detach().numpy()
    tensor = tensor.transpose(1, 2, 0)
    scale_factor = 255
    tensor = (tensor * scale_factor).astype(np.uint8)

    if writer is not None:
        writer.add_image("normal",tensor.transpose(2,0,1),int(epoch.split('_')[0]))
    img = Image.fromarray(tensor)
    img.save('{0}/normal_{1}.png'.format(path, epoch))
    return img

def plot_images(rgb_points, ground_true, path, epoch, plot_nrow, img_res, writer=None, no_gt=False):
    if not no_gt:
        ground_true = ground_true.cuda()
        output = torch.cat((rgb_points, ground_true), dim=0)
    else:
        output = rgb_points
    output_plot = lin2img(output, img_res)

    tensor = torchvision.utils.make_grid(output_plot,
                                         scale_each=False,
                                         normalize=False,
                                         nrow=plot_nrow).cpu().detach().numpy()

    tensor = tensor.transpose(1, 2, 0)
    scale_factor = 255
    tensor = (tensor * scale_factor).astype(np.uint8)
    if writer is not None:
        writer.add_image('image',tensor.transpose(2,0,1),int(epoch.split('-')[0]))
    img = Image.fromarray(tensor)
    img.save('{0}/rendering_{1}.png'.format(path, epoch))
    return img


def lin2img(tensor, img_res):
    batch_size, num_samples, channels = tensor.shape
    return tensor.permute(0, 2, 1).view(batch_size, channels, img_res[0], img_res[1])


```
