# Facial rendering source scan: pansanity666/TransHuman

Repository: https://github.com/pansanity666/TransHuman

Pinned source revision: efe870349f39e8e5fce3379b76107b0c4e35e2df

Discovery topic: human rendering

Description: Official code for ICCV 2023 paper: "TransHuman: A Transformer-based Human Representation for Generalizable Neural Human Rendering".

Reported license: NOASSERTION

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE", "third_parties/lpips/LICENSE", "third_parties/yacs/LICENSE", "third_parties/yacs/NOTICE"]

Topic paths: {"lbs": ["third_parties/smpl/models/basicModel_neutral_lbs_10_207_0_v1.0.0.pkl"]}

## Source file: run.py

{
  "path": "run.py",
  "blob_sha": "76e930c3259dacc613eda1d8dbdcf90e81c620a2",
  "sha256": "8f36963bd6de271551f2718470baa8b3e05d06f25abfea6c0ff115937687f72c",
  "lines": 166,
  "bytes": 5579,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/pansanity666/TransHuman/blob/efe870349f39e8e5fce3379b76107b0c4e35e2df/run.py",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "def run_evaluate():"
    },
    {
      "line": 57,
      "text": "def run_visualize():"
    },
    {
      "line": 112,
      "text": "def run_reconstruction():"
    },
    {
      "line": 160,
      "text": "def run_light_stage():"
    }
  ]
}

```text

from lib.config import cfg, args
import numpy as np 

import sys 
sys.path.append('./pytorch3d')

def run_evaluate():
    # metric evaluation 
    from lib.datasets import make_data_loader
    from lib.evaluators import make_evaluator
    import tqdm
    import torch
    from lib.networks import make_network
    from lib.utils import net_utils
    from lib.networks.renderer import make_renderer
    import re 
    
    if cfg.test.full_eval: 
        cfg.test.exp_folder_name += 'full_eval'

    cfg.flag_train = False
    cfg.perturb = 0
    
    network = make_network(cfg).cuda()
    net_utils.load_network(network,
                           cfg.trained_model_dir,
                           resume=cfg.resume,
                           epoch=cfg.test.epoch)
    network.train()

    data_loader = make_data_loader(cfg, is_train=False)
    renderer = make_renderer(cfg, network)
    evaluator = make_evaluator(cfg)

    with torch.no_grad():
        for batch in tqdm.tqdm(data_loader):
            for k in batch:
                if k != 'meta':
                    if k == 'tar_img_path':
                        continue
                    if k == 'input_img_paths':
                        continue    
                    if k == 'human_name':
                        continue
                    if isinstance(batch[k], tuple) or isinstance(batch[k],
                                                                 list):
                        batch[k] = [b.cuda() for b in batch[k]]
                    else:
                        batch[k] = batch[k].cuda()
            
            # Fast rendering 
            output = renderer.render_fast(batch, is_train=False)

            evaluator.evaluate(output, batch)
    evaluator.summarize()

def run_visualize():
    # video visualization 
    from lib.networks import make_network
    from lib.datasets import make_data_loader
    from lib.utils.net_utils import load_network
    from lib.utils import net_utils
    import tqdm
    import torch
    from lib.visualizers import make_visualizer
    from lib.networks.renderer import make_renderer

    cfg.perturb = 0

    network = make_network(cfg).cuda()
    load_network(network,
                 cfg.trained_model_dir,
                 resume=cfg.resume,
                 epoch=cfg.test.epoch)
    network.train()

    data_loader = make_data_loader(cfg, is_train=False)
    renderer = make_renderer(cfg, network)
    visualizer = make_visualizer(cfg)
    
    for batch in tqdm.tqdm(data_loader):
        
        # batch to cuda 
        for k in batch:
            if k == 'img_path':
                continue
            if k == 'input_img_paths':
                continue    
            if k == 'tar_img_path':
                continue
            if k == 'human_name':
                continue
            if k != 'meta':
                if isinstance(batch[k], tuple) or isinstance(batch[k], list):
                    batch[k] = [b.cuda() for b in batch[k]]
                else:
                    batch[k] = batch[k].cuda()
        
        ### Visualize input images. 
        # import torchvision
        # import os 
        # # import torch.distributed as dist 
        # cached_pth = cfg.trained_model_dir.replace('trained_model', 'cached_imgs')
        # os.makedirs(cached_pth, exist_ok=True)
        # # torchvision.utils.save_image(batch['input_imgs'][0][0], cached_pth + '/E_{}_it_{}_rank_{}.jpg'.format(epoch, iteration, dist.get_rank()))
        # torchvision.utils.save_image(batch['input_imgs'][0][0], cached_pth + '/batch_vos.jpg')

        with torch.no_grad():
            output = renderer.render_fast(batch)
            visualizer.visualize(output, batch)

def run_reconstruction():
    # mesh reconstuction 
    from lib.networks import make_network
    from lib.datasets import make_data_loader
    from lib.utils.net_utils import load_network
    from lib.utils import net_utils
    import tqdm
    import torch
    from lib.visualizers import make_visualizer
    from lib.networks.renderer import make_renderer

    cfg.perturb = 0

    network = make_network(cfg).cuda()
    load_network(network,
                 cfg.trained_model_dir,
                 resume=cfg.resume,
                 epoch=cfg.test.epoch)
    network.train()

    data_loader = make_data_loader(cfg, is_train=False)
    renderer = make_renderer(cfg, network)
    visualizer = make_visualizer(cfg)
    

    for batch in tqdm.tqdm(data_loader):
        # batch to cuda 
        for k in batch:
            if k == 'img_path':
                continue
            if k == 'input_img_paths':
                continue    
            if k == 'tar_img_path':
                continue
            if k == 'human_name':
                continue
            if k != 'meta':
                if isinstance(batch[k], tuple) or isinstance(batch[k], list):
                    batch[k] = [b.cuda() for b in batch[k]]
                else:
                    batch[k] = batch[k].cuda()
     
        with torch.no_grad():
            # mesh reconstruction does not support fast rendering ATM.
            output = renderer.render(batch)
            visualizer.visualize(output, batch)
            
            
def run_light_stage():
    from lib.utils.light_stage import ply_to_occupancy
    ply_to_occupancy.ply_to_occupancy()


if __name__ == '__main__':
    globals()['run_' + args.type]()


```

## Source file: train_net.py

{
  "path": "train_net.py",
  "blob_sha": "707fc43a8d3be091e1d8b22c08a0c99f5aee8a56",
  "sha256": "4c1d802cd81284eaabd64a06bc62052ff077521efff0d9ba05cf5e4576f2360e",
  "lines": 137,
  "bytes": 4237,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 1
  },
  "url": "https://github.com/pansanity666/TransHuman/blob/efe870349f39e8e5fce3379b76107b0c4e35e2df/train_net.py",
  "function_signature_candidates": [
    {
      "line": 20,
      "text": "def fix_random_seeds(seed=31):"
    },
    {
      "line": 34,
      "text": "def train(cfg, network):"
    },
    {
      "line": 90,
      "text": "def test(cfg, network):"
    },
    {
      "line": 101,
      "text": "def synchronize():"
    },
    {
      "line": 116,
      "text": "def main():"
    }
  ]
}

```text

from lib.config import cfg, args
from lib.networks import make_network
from lib.train import make_trainer, make_optimizer, make_lr_scheduler, \
    make_recorder, set_lr_scheduler
from lib.datasets import make_data_loader
from lib.utils.net_utils import load_model, save_model, load_network
# from lib.evaluators import make_evaluator
import torch.multiprocessing
import torch
import torch.distributed as dist
import os

import numpy as np
import random

import sys 
sys.path.append('./pytorch3d')
    

def fix_random_seeds(seed=31):
    """
    Fix random seeds.
    """
    torch.manual_seed(seed)
    torch.cuda.manual_seed_all(seed)
    np.random.seed(seed)
    random.seed(seed)

    ### original 
    torch.backends.cudnn.deterministic = True # 
    torch.backends.cudnn.benchmark = False #


def train(cfg, network):

    # network is wrapped with NeRFWrapper and sent to Trainer here. 
    trainer = make_trainer(cfg, network)
    optimizer = make_optimizer(cfg, network)
    scheduler = make_lr_scheduler(cfg, optimizer)
    recorder = make_recorder(cfg)
    begin_epoch = load_model(network,
                             optimizer,
                             scheduler,
                             recorder,
                             cfg.trained_model_dir,
                             resume=cfg.resume,
                             specified_resume=cfg.specified_resume)
    
    set_lr_scheduler(cfg, scheduler) # set config values to scheduler parameters 
    
    train_loader = make_data_loader(cfg,
                                    is_train=True,
                                    is_distributed=cfg.distributed,
                                    max_iter=cfg.ep_iter)


    cfg.global_iter = begin_epoch * len(train_loader)

    for epoch in range(begin_epoch, cfg.train.epoch):

        recorder.epoch = epoch

        # make sure the colorjitter is same across difference runnings. 
        train_loader.dataset.set_epoch(epoch)
   
        # set epoch to get different random seed for each epoch. 
        if cfg.distributed:
            train_loader.batch_sampler.sampler.set_epoch(epoch)

        # train one epoch 
        trainer.train(epoch, train_loader, optimizer, recorder)
        scheduler.step()

        if (epoch + 1) % cfg.save_freq == 0 and cfg.local_rank == 0 and int(os.getenv('RANK', '0')) == 0:
            save_model(network, optimizer, scheduler, recorder,
                       cfg.trained_model_dir, epoch)

        if (epoch + 1) % cfg.save_latest_ep == 0 and cfg.local_rank == 0 and int(os.getenv('RANK', '0')) == 0:
            save_model(network,
                       optimizer,
                       scheduler,
                       recorder,
                       cfg.trained_model_dir,
                       epoch,
                       last=True)
            
    return network


def test(cfg, network):
    trainer = make_trainer(cfg, network)
    val_loader = make_data_loader(cfg, is_train=False)
    evaluator = make_evaluator(cfg)
    epoch = load_network(network,
                         cfg.trained_model_dir,
                         resume=cfg.resume,
                         epoch=cfg.test.epoch)
    trainer.val(epoch, val_loader, evaluator)


def synchronize():
    """
    Helper function to synchronize (barrier) among all processes when
    using distributed training
    """
    if not dist.is_available():
        return
    if not dist.is_initialized():
        return
    world_size = dist.get_world_size()
    if world_size == 1:
        return
    dist.barrier()


def main():
    
    if cfg.distributed:
        cfg.local_rank = int(os.environ['RANK']) % torch.cuda.device_count()
        torch.cuda.set_device(cfg.local_rank)
        torch.distributed.init_process_group(backend="nccl",
                                             init_method="env://")
        synchronize()
        
    fix_random_seeds(cfg.seed)

    cfg.flag_train = True
    network = make_network(cfg)

    if args.test:
        test(cfg, network)
    else:
        train(cfg, network)


if __name__ == "__main__":
    main()


```

## Source file: lib/__init__.py

{
  "path": "lib/__init__.py",
  "blob_sha": "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
  "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "lines": 0,
  "bytes": 0,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/pansanity666/TransHuman/blob/efe870349f39e8e5fce3379b76107b0c4e35e2df/lib/__init__.py",
  "function_signature_candidates": []
}

```text



```

## Source file: lib/utils/SMPL.py

{
  "path": "lib/utils/SMPL.py",
  "blob_sha": "638b730283730cf25169655b2dc392f89ff9c0cb",
  "sha256": "aa5050c6e40ee55e4aecee5993c5e97ee2471137693c0dc3fceb34460368752a",
  "lines": 186,
  "bytes": 7186,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 1,
    "lbs": 2,
    "rig": 2,
    "face": 1
  },
  "url": "https://github.com/pansanity666/TransHuman/blob/efe870349f39e8e5fce3379b76107b0c4e35e2df/lib/utils/SMPL.py",
  "function_signature_candidates": [
    {
      "line": 45,
      "text": "def write_point_cloud(ply_filename, points):"
    },
    {
      "line": 73,
      "text": "def __init__(self, sex, model_dir):"
    },
    {
      "line": 107,
      "text": "def __call__(self, pose, beta, ratio=1.0):"
    },
    {
      "line": 114,
      "text": "def _call(self, pose, beta, v_template):"
    }
  ]
}

```text

import os

import numpy as np
import pickle
import torch

from cv2 import Rodrigues as rodrigues

MALE_PATH    = "basicmodel_m_lbs_10_207_0_v1.0.0.pkl"
FEMALE_PATH  = "basicModel_f_lbs_10_207_0_v1.0.0.pkl"
NEUTRAL_PATH = "SMPL_NEUTRAL.pkl"



# def transform_can_smpl(xyz):
#     center = np.array([0, 0, 0]).astype(np.float32)
#     rot = np.array([[np.cos(0), -np.sin(0)], [np.sin(0), np.cos(0)]])
#     rot = rot.astype(np.float32)
#     trans = np.array([0, 0, 0]).astype(np.float32)
#     # if np.random.uniform() > cfg.rot_ratio:
#     #     return xyz, center, rot, trans

#     xyz = xyz.copy()

#     # rotate the smpl
#     rot_range = np.pi / 32
#     t = np.random.uniform(-rot_range, rot_range)
#     rot = np.array([[np.cos(t), -np.sin(t)], [np.sin(t), np.cos(t)]])
#     rot = rot.astype(np.float32)
#     center = np.mean(xyz, axis=0)
#     xyz = xyz - center
#     xyz[:, [0, 2]] = np.dot(xyz[:, [0, 2]], rot.T)
#     xyz = xyz + center

#     # translate the smpl
#     x_range = 0.05
#     z_range = 0.025
#     x_trans = np.random.uniform(-x_range, x_range)
#     z_trans = np.random.uniform(-z_range, z_range)
#     trans = np.array([x_trans, 0, z_trans]).astype(np.float32)
#     xyz = xyz + trans

#     return xyz, center, rot, trans

def write_point_cloud(ply_filename, points):

    if points.shape[1] == 3: 
        colors = np.zeros((points.shape[0],3))
        points = np.concatenate((points, colors), axis=1)

    formatted_points = []
    for point in points:
        formatted_points.append("%f %f %f %d %d %d 0\n" % (point[0], point[1], point[2], point[3], point[4], point[5]))

    out_file = open(ply_filename, "w")
    out_file.write('''ply
    format ascii 1.0
    element vertex %d
    property float x
    property float y
    property float z
    property uchar blue
    property uchar green
    property uchar red
    property uchar alpha
    end_header
    %s
    ''' % (len(points), "".join(formatted_points)))
    out_file.close()
    

class SMPL():
    def __init__(self, sex, model_dir):
        super(SMPL, self).__init__()

        model_paths = {
            'male': os.path.join(model_dir, MALE_PATH),
            'female': os.path.join(model_dir, FEMALE_PATH),
            'neutral': os.path.join(model_dir, NEUTRAL_PATH)
        }

        with open(model_paths[sex], 'rb') as f:
            smpl_model = pickle.load(f, encoding='latin1')
        self.J_regressor = np.array(smpl_model['J_regressor'].todense())
        self.weights = smpl_model['weights']
        self.posedirs = smpl_model['posedirs']
        self.v_template = smpl_model['v_template']
        self.shapedirs = np.array(smpl_model['shapedirs'])
        self.faces = smpl_model['f'].astype('int32')
        self.kintree_table = smpl_model['kintree_table'].astype('int64')
        # print(self.kintree_table)
        # print(self.posedirs.shape) # (6890, 3, 207)
        # print(self.weights.shape) # (6890, 24)

        id_to_col = {self.kintree_table[1, i].item(): i for i in range(self.kintree_table.shape[1])}
        self.parent = np.array([id_to_col[self.kintree_table[0, it]] for it in range(1, self.kintree_table.shape[1])])

        self.pose_shape = [24, 3]
        self.beta_shape = [10]
        self.pose = np.zeros(self.pose_shape)
        self.beta = np.zeros(self.beta_shape)

        self.verts = None
        self.J = None
        self.R = None

    def __call__(self, pose, beta, ratio=1.0):

        if ratio == 1.0:
            return self._call(pose, beta, self.v_template)
        else:
            return self._call(pose, beta, self.v_template * ratio)
    
    def _call(self, pose, beta, v_template):
        # print(pose.shape, beta.shape, v_template.shape) # (1, 72) (10,) (6890, 3)
            
        # v_template = self.v_template              # (6890, 3)
        shapedirs = self.shapedirs.reshape(-1,10) # (6890, 10)
        beta = beta[:, None]                      # (10, 1)

        v_shaped = shapedirs.dot(beta).reshape(6890, 3) + v_template # (6890, 3)
        # write_point_cloud('./v_shaped.ply', v_shaped)
        # print(v_shaped)
        
        # the joints of new shape
        J = self.J_regressor.dot(v_shaped)                           # (24, 3)

        # input is a rotation matrix: (24,3,3)
        if pose.shape == (24, 3, 3):
            R = pose
        # input is a rotation axis-angle vector: (1, 72), (72, 1) or (72, )
        elif pose.shape == (1, 72) or pose.shape == (72, 1) or pose.shape == (72,):
            pose_vectors = pose.reshape(-1, 3)                      # (24, 3)
            R = np.array([rodrigues(pose_vectors[p_idx])[0] 
                            for p_idx in range(pose_vectors.shape[0])
                          ], 
                          dtype='float32')                          # (24, 3, 3)
        else:
            raise ValueError("Unsupported Pose Inputs - the Pose Shape is {}".format(pose.shape))
        # print(R.shape) # 24, 3, 3
        # print(R[0])
        # print(R[1])
    
        Is = np.eye(3, dtype='float32')[None, :]                    # (1, 3, 3)
        lrotmin = (R[1:,:] - Is).reshape(-1, 1)                     # (23x3x3, 1)
        posedirs = self.posedirs.reshape(-1,207)                    # (6890, 207)
        v_posed = v_shaped + posedirs.dot(lrotmin).reshape(6890, 3) # (6890, 3)
        # write_point_cloud('./v_posed.ply', v_posed)
        
        J_ = J.copy()
        J_[1:, :] = J[1:, :] - J[self.parent, :]                     # (24, 3)
        G_ = np.concatenate([R, J_[:, :, None]],  axis=-1)           # (24, 3, 4)
        pad_rows = np.array([[0, 0, 0, 1]], dtype='float32')
        pad_rows = np.repeat(pad_rows, 24, axis=0).reshape(-1, 1, 4)
        G_ = np.concatenate([G_, pad_rows], axis=1)                  # (24, 4, 4)

        G = [G_[0].copy()]
        for i in range(1, 24):
            G.append(G[self.parent[i-1]].dot(G_[i, :, :]))
        G = np.stack(G, axis=0)  # (24, 4, 4)

        joints = G[:, :3, 3]

        rest_joints = np.concatenate([J, np.zeros((24, 1))], axis=-1)[:, :, None]  # (24, 4, 1)
        zeros = np.zeros((24, 4, 3), dtype='float32')                              # (24, 4, 3)
        rest_joints_mtx = np.concatenate([zeros, rest_joints], axis=-1)            # (24, 4, 4) 
        posed_joints_mtx = np.matmul(G, rest_joints_mtx)
        G = G - posed_joints_mtx
                                                                    
        # print(G.shape) # 24, 4, 4
        # print(v_posed)
        # is v_poses == canonical smpl ?
        rest_shape_h = np.concatenate([v_posed, np.ones(v_posed.shape[0])[:, None]], axis=-1) #(6890, 4)
        T = self.weights.dot(G.reshape(24, -1)).reshape(6890, 4, 4) # (6890, 24) x (24, 4, 4) -> (6890, 4, 4)        
        v = np.matmul(T, rest_shape_h[:, :, None]) # (6890, 4, 4) x (6890, 4, 1) -> 6890, 4, 1 
        # print(v.shape)
        # write_point_cloud('./v.ply', v[:, :3, 0])
        
        # inv_T = torch.inverse(torch.tensor(T)).cpu().numpy()
        # ori_v = np.matmul(inv_T, v)
        # write_point_cloud('./ori_v.ply', ori_v[:, :3, 0])
        # assert False
        
        v = v[:, :3, 0]
        
        return v, joints, T 

```
