# Facial rendering source scan: Dogter521/LSF-Animation

Repository: https://github.com/Dogter521/LSF-Animation

Pinned source revision: e5a6ab856d94dc3f761146e7e6275f0b9d462e8c

Discovery topic: facial animation

Description: Official repository of Siggraph Asia 2025 paper "LSF-Animation: Label-Free Speech-Driven Facial Animation via Implicit Feature Representation"

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"flame": ["deps/flame/.pre-commit-config.yaml", "deps/flame/__init__.py", "deps/flame/__pycache__/__init__.cpython-39.pyc", "deps/flame/flame_pytorch/__init__.py", "deps/flame/flame_pytorch/__pycache__/__init__.cpython-39.pyc", "deps/flame/flame_pytorch/__pycache__/config.cpython-39.pyc", "deps/flame/flame_pytorch/__pycache__/flame.cpython-39.pyc", "deps/flame/flame_pytorch/config.py", "deps/flame/flame_pytorch/flame.py", "deps/flame/main.py", "deps/flame/model/__init__.py", "deps/flame/model/put_flame_here", "deps/flame/setup.py"], "face": ["configs/model/losses/loss_FaceDiff.yaml", "diffusion/evaluation_facediff.py", "diffusion/train_facediff.sh", "framework/launch/face.code-workspace", "framework/model/losses/compute_FaceDiff.py", "framework/model/utils/face.code-workspace", "post_process/anim_transfer/newface1.npy", "post_process/anim_transfer/newface2.npy", "post_process/anim_transfer/newface3.npy"]}

## Source file: diffusion/evaluation_facediff.py

{
  "path": "diffusion/evaluation_facediff.py",
  "blob_sha": "a08e155e70fc44e059e613f13387d63da42058cf",
  "sha256": "8a0c72b109a0b2dc4f1ba5ceabf0f6e82fb1121029a0c56af58646419168b583",
  "lines": 329,
  "bytes": 15145,
  "selection_score": 15,
  "keyword_line_counts": {
    "mouth": 13,
    "gum": 25,
    "face": 3,
    "lip": 1
  },
  "url": "https://github.com/Dogter521/LSF-Animation/blob/e5a6ab856d94dc3f761146e7e6275f0b9d462e8c/diffusion/evaluation_facediff.py",
  "function_signature_candidates": [
    {
      "line": 39,
      "text": "def lve_compute(vertices_gt, vertices_pred, mouth_map):"
    },
    {
      "line": 50,
      "text": "def seq_std_compute(motion, map):"
    },
    {
      "line": 62,
      "text": "def test_diff(args, model, test_loader, epoch, diffusion, device=\"cuda:0\"):"
    },
    {
      "line": 262,
      "text": "def count_parameters(model):"
    }
  ]
}

```text

import logging
import os
from pathlib import Path
import pickle
import torch
import numpy as np
import argparse
import datetime
import sys
current_dir = os.path.dirname(os.path.abspath(__file__))
parent_dir = os.path.abspath(os.path.join(current_dir, '..'))
sys.path.append(parent_dir)

from models import FaceDiff
from data_loader import get_dataloaders
from utils import *

logger = logging.getLogger(__name__)


emo_dict = {
       "0": "neutral",   # only have one intensity level
       "1": "happy",
       "2": "sad",
       "3": "surprised",
       "4": "fear",
       "5": "disgusted",
       "6": "angry",
       "7": "contempt"
   }

int_dict = {
       "0": "low",
       "1": "medium",
       "2": "high",
   }


def lve_compute(vertices_gt, vertices_pred, mouth_map):
    # L2_dis_mouth_max: (428, T, 3), 428 vertex indices
    vertices_gt = np.array(vertices_gt)
    vertices_pred = np.array(vertices_pred)
    L2_dis_mouth_max = np.array([np.square(vertices_gt[:, v, :] - vertices_pred[:, v, :]) for v in mouth_map])
    L2_dis_mouth_max = np.transpose(L2_dis_mouth_max, (1, 0, 2))    # (T, 428, 3)
    L2_dis_mouth_max = np.sum(L2_dis_mouth_max, axis=2)
    L2_dis_mouth_max = np.max(L2_dis_mouth_max, axis=1)
    return L2_dis_mouth_max


def seq_std_compute(motion, map):
    # map: 1501 vertex indices
    # motion[:, v, :]: (T, 3)
    L2_dis = np.array([np.square(motion[:, v, :]) for v in map])    # (1501, T, 3)
    L2_dis = np.transpose(L2_dis, (1, 0, 2))                        # (T, 1501, 3)
    L2_dis = np.sum(L2_dis, axis=2)
    L2_dis = np.std(L2_dis, axis=0)
    std = np.mean(L2_dis)
    return std


@torch.no_grad()
def test_diff(args, model, test_loader, epoch, diffusion, device="cuda:0"):
    result_path = os.path.join(args.result_path)
    os.makedirs(result_path, exist_ok=True)

    train_subjects_list = [i for i in args.train_subjects.split(" ")]
    model_path = f'{args.save_path}/{args.model}_{args.dataset}_{epoch}.pth'
    print("load model from...", model_path)
    sys.stdout.flush()
    model.load_state_dict(torch.load(model_path, map_location=device))
    model = model.to(device)
    model.eval()
    print("Device checking:", device)

    # load templates for evaluation
    with open(Path(f"{parent_dir}/datasets/regions/lve.txt"))as f:
        maps = f.read().split(",")
        mouth_map = [int(i) for i in maps]
    with open(Path(f"{parent_dir}/datasets/regions/fdd.txt")) as f:
        maps = f.read().split(",")
        upper_map = [int(i) for i in maps]
    with open(Path(f"{parent_dir}/datasets/templates_mead_vert.pkl"), 'rb') as f:
        templates = pickle.load(f, encoding='latin1')   # [5023, 3]

    sr = 16000
    # count frame numbers
    seq_count = 0
    frame_count = 0
    vertices_all_gt = []        # mve, lve: gt
    vertices_all_pred = []      # mve, lve: first prediction
    mee_all = []                # mean value
    ce_all = []                 # closest value
    motion_std_difference = []  # fdd: first prediction
    diversity = 0               # 2 subsets
    for idx, (audio, vertice, template, one_hot_all, file_name) in enumerate(test_loader):
        vertice = vertice_path = str(vertice[0])
        vertice = np.load(vertice, allow_pickle=True)
        vertices_npy_gt = vertice.copy()                    # (T, 5023, 3)

        vertice_path = os.path.split(vertice_path)[-1][:-4]
        emo = emo_dict[str(vertice_path.split("_")[2])]     # retrieve emotion
        ints = int_dict[str(vertice_path.split("_")[3])]    # retrieve intensity
        gt_path = os.path.join(args.result_path, f"{vertice_path}_{emo}_{ints}_gt.npy")

        if (idx + 1) % 100 == 0:
            print(f"Saving gt: {gt_path}")
            sys.stdout.flush()
            np.save(gt_path, vertices_npy_gt)

        vertice = vertice.astype(np.float32)
        vertice = torch.from_numpy(vertice)
        vertice = vertice.reshape(1, vertice.shape[0], vertice.shape[1] * vertice.shape[2])

        audio, vertice = audio.to(device=device), vertice.to(device=device)
        template, one_hot_all = template.to(device=device), one_hot_all.to(device=device)

        num_frames = int(audio.shape[-1] / sr * args.output_fps)
        shape = (1, num_frames - 1, args.vertice_dim) if num_frames < vertice.shape[1] else vertice.shape

        train_subject = file_name[0].split("_")[0]
        sys.stdout.flush()
        assert train_subject in train_subjects_list, "train_subject not in train_subjects_list"

        one_hot = one_hot_all.to(device=device)
        ce_lve_set = []  # save 10 lve values
        motion_set = []  # save 10 samples
        for sample_idx in range(1, args.num_samples + 1):
            # use ddim
            sample = diffusion.ddim_sample_loop(
                model,
                shape,
                clip_denoised=False,
                model_kwargs={
                    "cond_embed": audio,
                    "one_hot": one_hot,
                    "template": template,
                },
                skip_timesteps=args.skip_steps,     # skip 900 timesteps
                init_image=None,
                progress=None,
                dump_steps=None,
                noise=None,
                const_noise=False,
                device=args.device,
            )
            sample = sample.squeeze()
            sample = sample.detach().cpu().numpy()  # (T, 5023*3)

            vertices_npy_pred = sample.reshape(-1, 5023, 3)
            vertices_npy_pred = vertices_npy_pred[:vertices_npy_gt.shape[0], :, :]  # (T, 5023, 3)
            vertices_npy_gt = vertices_npy_gt[:vertices_npy_pred.shape[0], :, :]    # (T, 5023, 3)

            """CE: compute lve for each samples"""
            ce_lve = lve_compute(vertices_gt=list(vertices_npy_gt),
                                 vertices_pred=list(vertices_npy_pred),
                                 mouth_map=mouth_map)
            ce_lve_set.append(ce_lve)               # (T,)

            # save 10 samples
            motion_set.append(vertices_npy_pred)    # (T, 5023, 3)
            torch.cuda.empty_cache()

            if args.num_samples != 1:
                out_path = f"{vertice_path}_{emo}_{ints}_{sample_idx}.npy"
            else:
                out_path = f"{vertice_path}_{emo}_{ints}_one.npy"

            # save vertices (a small part)
            if (idx + 1) % 100 == 0:                # save every 100 keyids for checking
                if (sample_idx + 1) % 3 == 1:       # save 3 samples in 10
                    print(f"Saving pred: {out_path}")
                    np.save(os.path.join(args.result_path, out_path), sample)

        """MVE, LVE: save prediction of all audio samples"""
        vertices_all_gt.extend(list(vertices_npy_gt))               # length T of items (5023, 3)
        vertices_all_pred.extend(list(motion_set[0]))               # use the first sample

        """MEE: mean over 10 samples"""
        motion_set_stack = np.stack(motion_set, axis=0)
        vertices_npy_pred_mean = np.mean(motion_set_stack, axis=0)  # (T, 5023, 3)
        mee_lve = lve_compute(vertices_gt=list(vertices_npy_gt),
                              vertices_pred=list(vertices_npy_pred_mean),
                              mouth_map=mouth_map)
        mee_all.extend(list(mee_lve))

        """CE: closest lve in 10 samples"""
        smallest_lve = None
        smallest_lve_value = float('inf')                           # start with an infinitely large value
        for lve_of_one_seq in ce_lve_set:
            lve_value = np.sum(lve_of_one_seq)
            if lve_value < smallest_lve_value:
                smallest_lve_value = lve_value
                smallest_lve = lve_of_one_seq                       # (T,)
        assert smallest_lve is not None, "No smallest distance found"
        ce_all.extend(list(smallest_lve))

        # count sequence, frame numbers
        frame_count += vertices_npy_gt.shape[0]
        seq_count += 1

        """FDD computation: use the first sample"""
        subject_template = templates[train_subject].reshape(1, 5023, 3)
        subject_template = subject_template.detach().cpu().numpy()  # (1, 5023, 3)
        upper_std_gt = seq_std_compute(motion=vertices_npy_gt - subject_template,
                                       map=upper_map)
        upper_std_pred = seq_std_compute(motion=motion_set[0] - subject_template,
                                         map=upper_map)
        motion_std_difference.append(upper_std_gt - upper_std_pred)

        """Diversity computation"""
        np.random.shuffle(motion_set)                               # list of (T, 5023, 3) number=10
        subset1 = motion_set[:5]
        subset2 = motion_set[5:]
        motion_diversity = 0
        for sample1, sample2 in zip(subset1, subset2):
            motion_diversity += np.linalg.norm(sample1 - sample2, axis=2).mean(axis=1).mean()
        if len(subset1) == 5 and len(subset2) == 5:
            motion_diversity /= len(subset1)
            diversity += motion_diversity
        else:
            raise ValueError("Subset length mismatch 5")

        part_time = datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        print(f"{part_time} Done sampling: {vertice_path} ")
        sys.stdout.flush()
        torch.cuda.empty_cache()

    print('Total sequence number: {}'.format(seq_count))
    print('Total frame number: {}'.format(frame_count))
    sys.stdout.flush()
    if seq_count == 0:
        print("No sequences were processed. Unable to compute metrics.")
    else:
        """MVE computation"""
        vertices_all_gt = np.array(vertices_all_gt)  # (frame_cunt, 5023, 3)
        vertices_all_pred = np.array(vertices_all_pred)
        vertices_dis = np.linalg.norm(vertices_all_gt - vertices_all_pred, axis=2)
        print('MVE: {:.4e}'.format(np.mean(vertices_dis)))

        """LVE computation"""
        L2_dis_mouth_max = lve_compute(vertices_gt=vertices_all_gt,
                                       vertices_pred=vertices_all_pred,
                                       mouth_map=mouth_map)
        print('LVE: {:.4e}'.format(np.mean(L2_dis_mouth_max)))

        """MEE computation"""
        print('MEE: {:.4e}'.format(np.mean(mee_all)))

        """CE computation"""
        print('CE: {:.4e}'.format(np.mean(ce_all)))

        """FDD computation"""
        print('FDD: {:.4e}'.format(sum(motion_std_difference) / len(motion_std_difference)))

        """Divertiy computation"""
        print('Diversity: {:.4e}'.format(diversity / seq_count))

        print(f"All the sampling are done. ")
        sys.stdout.flush()


def count_parameters(model):
    return sum(p.numel() for p in model.parameters() if p.requires_grad)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument("--lr", type=float, default=0.0001, help='learning rate')
    parser.add_argument("--dataset", type=str, default="mead", help='Name of the dataset folder. eg: BIWI')
    parser.add_argument("--data_path", type=str, default=f"{parent_dir}/datasets/")
    parser.add_argument("--vertice_dim", type=int, default=15069, help='number of vertices - 23370*3 for BIWI dataset')
    parser.add_argument("--feature_dim", type=int, default=256, help='Latent Dimension to encode the inputs to')
    parser.add_argument("--gru_dim", type=int, default=256, help='GRU Vertex decoder hidden size')
    parser.add_argument("--gru_layers", type=int, default=2, help='GRU Vertex decoder hidden size')
    parser.add_argument("--wav_path", type=str, default="wav", help='path of the audio signals')
    parser.add_argument("--vertices_path", type=str, default="vertex", help='path of the ground truth')
    parser.add_argument("--gradient_accumulation_steps", type=int, default=1, help='gradient accumulation')
    parser.add_argument("--max_epoch", type=int, default=50, help='number of epochs')
    parser.add_argument("--device", type=str, default="cuda:0")
    parser.add_argument("--model", type=str, default="face_diffuser", help='name of the trained model')
    parser.add_argument("--template_file", type=str, default="templates_mead_vert.pkl",
                        help='path of the train subject templates')
    parser.add_argument("--save_path", type=str, default="outputs/model", help='path of the trained models')
    parser.add_argument("--result_path", type=str, default="results/evaluation", help='path to the predictions')
    parser.add_argument("--train_subjects", type=str, default="M003 M005 M007 M009 M011 M012 M013 M019 "
                                                              "M022 M023 M024 M025 M026 M027 M028 M029 "
                                                              "M030 M031 W009 W011 W014 W015 W016 W018 "
                                                              "W019 W021 W023 W024 W025 W026 W028 W029")
    parser.add_argument("--val_subjects", type=str, default="M003 M005 M007 M009 M011 M012 M013 M019 "
                                                            "M022 M023 M024 M025 M026 M027 M028 M029 "
                                                            "M030 M031 W009 W011 W014 W015 W016 W018 "
                                                            "W019 W021 W023 W024 W025 W026 W028 W029")
    parser.add_argument("--test_subjects", type=str, default="M003 M005 M007 M009 M011 M012 M013 M019 "
                                                             "M022 M023 M024 M025 M026 M027 M028 M029 "
                                                             "M030 M031 W009 W011 W014 W015 W016 W018 "
                                                             "W019 W021 W023 W024 W025 W026 W028 W029")
    parser.add_argument("--input_fps", type=int, default=50,
                        help='HuBERT last hidden state produces 50 fps audio representation')
    parser.add_argument("--output_fps", type=int, default=25,
                        help='fps of the visual data, BIWI was captured in 25 fps')
    parser.add_argument("--diff_steps", type=int, default=1000, help='number of diffusion steps')
    parser.add_argument("--skip_steps", type=int, default=900, help='number of diffusion steps to skip during inference')
    parser.add_argument("--num_samples", type=int, default=10, help='number of samples to generate per audio')
    args = parser.parse_args()

    start_time = datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")  # Get the start time
    print(f"Start time: {start_time}")
    sys.stdout.flush()
    assert torch.cuda.is_available()
    diffusion = create_gaussian_diffusion(args)

    model = FaceDiff(
        args,
        vertice_dim=args.vertice_dim,
        latent_dim=args.feature_dim,
        diffusion_steps=args.diff_steps,
        gru_latent_dim=args.gru_dim,
        num_layers=args.gru_layers,
    )
    print("model parameters: ", count_parameters(model))
    sys.stdout.flush()
    cuda = torch.device(args.device)
    dataset = get_dataloaders(args)

    test_diff(args, model, dataset["test"], args.max_epoch, diffusion, device=args.device)

    end_time = datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    print(f"End time: {end_time}")



```

## Source file: deps/flame/flame_pytorch/flame.py

{
  "path": "deps/flame/flame_pytorch/flame.py",
  "blob_sha": "4d6d8a9ca35aee7d92706e85ec76bc2f6a14c94c",
  "sha256": "983ffd0dfcb556a82c8888f925bbe5e529f5b46d65665f24265ad9be98e53b1c",
  "lines": 318,
  "bytes": 12339,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 9,
    "flame": 20,
    "lbs": 4,
    "rig": 9,
    "face": 24
  },
  "url": "https://github.com/Dogter521/LSF-Animation/blob/e5a6ab856d94dc3f761146e7e6275f0b9d462e8c/deps/flame/flame_pytorch/flame.py",
  "function_signature_candidates": [
    {
      "line": 43,
      "text": "def __init__(self, config):"
    },
    {
      "line": 179,
      "text": "def _find_dynamic_lmk_idx_and_bcoords("
    },
    {
      "line": 228,
      "text": "def forward("
    }
  ]
}

```text

"""
FLAME Layer: Implementation of the 3D Statistical Face model in PyTorch

It is designed in a way to directly plug in as a decoder layer in a
Deep learning framework for training and testing

It can also be used for 2D or 3D optimisation applications

Author: Soubhik Sanyal
Copyright (c) 2019, Soubhik Sanyal
All rights reserved.

Max-Planck-Gesellschaft zur Foerderung der Wissenschaften e.V. (MPG) is holder of all proprietary rights on this
computer program.
You can only use this computer program if you have closed a license agreement with MPG or you get the right to use
the computer program from someone who is authorized to grant you that right.
Any use of the computer program without a valid license is prohibited and liable to prosecution.
Copyright 2019 Max-Planck-Gesellschaft zur Foerderung der Wissenschaften e.V. (MPG). acting on behalf of its
Max Planck Institute for Intelligent Systems and the Max Planck Institute for Biological Cybernetics.
All rights reserved.

More information about FLAME is available at http://flame.is.tue.mpg.de.

For questions regarding the PyTorch implementation please contact soubhik.sanyal@tuebingen.mpg.de
"""
# Modified from smplx code [https://github.com/vchoutas/smplx] for FLAME

import pickle

import numpy as np
import torch
import torch.nn as nn
from smplx.lbs import batch_rodrigues, lbs, vertices2landmarks
from smplx.utils import Struct, rot_mat_to_euler, to_np, to_tensor


class FLAME(nn.Module):
    """
    Given flame parameters this class generates a differentiable FLAME function
    which outputs the a mesh and 3D facial landmarks
    """

    def __init__(self, config):
        super(FLAME, self).__init__()
        # print("creating the FLAME Decoder")
        with open(config.flame_model_path, "rb") as f:
            self.flame_model = Struct(**pickle.load(f, encoding="latin1"))
        self.NECK_IDX = 1
        self.batch_size = config.batch_size
        self.dtype = torch.float32
        self.use_face_contour = config.use_face_contour
        self.faces = self.flame_model.f
        self.register_buffer(
            "faces_tensor",
            to_tensor(to_np(self.faces, dtype=np.int64), dtype=torch.long),
        )

        # Fixing remaining Shape betas
        # There are total 300 shape parameters to control FLAME; But one can use the first few parameters to express
        # the shape. For example 100 shape parameters are used for RingNet project
        default_shape = torch.zeros(
            [self.batch_size, 300 - config.shape_params],
            dtype=self.dtype,
            requires_grad=False,
        )
        self.register_parameter(
            "shape_betas", nn.Parameter(default_shape, requires_grad=False)
        )

        # Fixing remaining expression betas
        # There are total 100 shape expression parameters to control FLAME; But one can use the first few parameters to express
        # the expression. For example 50 expression parameters are used for RingNet project
        default_exp = torch.zeros(
            [self.batch_size, 100 - config.expression_params],
            dtype=self.dtype,
            requires_grad=False,
        )
        self.register_parameter(
            "expression_betas", nn.Parameter(default_exp, requires_grad=False)
        )

        # Eyeball and neck rotation
        default_eyball_pose = torch.zeros(
            [self.batch_size, 6], dtype=self.dtype, requires_grad=False
        )
        self.register_parameter(
            "eye_pose", nn.Parameter(default_eyball_pose, requires_grad=False)
        )

        default_neck_pose = torch.zeros(
            [self.batch_size, 3], dtype=self.dtype, requires_grad=False
        )
        self.register_parameter(
            "neck_pose", nn.Parameter(default_neck_pose, requires_grad=False)
        )

        # Fixing 3D translation since we use translation in the image plane

        self.use_3D_translation = config.use_3D_translation

        default_transl = torch.zeros(
            [self.batch_size, 3], dtype=self.dtype, requires_grad=False
        )
        self.register_parameter(
            "transl", nn.Parameter(default_transl, requires_grad=False)
        )

        # The vertices of the template model
        self.register_buffer(
            "v_template",
            to_tensor(to_np(self.flame_model.v_template), dtype=self.dtype),
        )

        # The shape components
        shapedirs = self.flame_model.shapedirs
        # The shape components
        self.register_buffer("shapedirs", to_tensor(to_np(shapedirs), dtype=self.dtype))

        j_regressor = to_tensor(to_np(self.flame_model.J_regressor), dtype=self.dtype)
        self.register_buffer("J_regressor", j_regressor)

        # Pose blend shape basis
        num_pose_basis = self.flame_model.posedirs.shape[-1]
        posedirs = np.reshape(self.flame_model.posedirs, [-1, num_pose_basis]).T
        self.register_buffer("posedirs", to_tensor(to_np(posedirs), dtype=self.dtype))

        # indices of parents for each joints
        parents = to_tensor(to_np(self.flame_model.kintree_table[0])).long()
        parents[0] = -1
        self.register_buffer("parents", parents)

        self.register_buffer(
            "lbs_weights", to_tensor(to_np(self.flame_model.weights), dtype=self.dtype)
        )

        # Static and Dynamic Landmark embeddings for FLAME

        with open(config.static_landmark_embedding_path, "rb") as f:
            static_embeddings = Struct(**pickle.load(f, encoding="latin1"))

        lmk_faces_idx = (static_embeddings.lmk_face_idx).astype(np.int64)
        self.register_buffer(
            "lmk_faces_idx", torch.tensor(lmk_faces_idx, dtype=torch.long)
        )
        lmk_bary_coords = static_embeddings.lmk_b_coords
        self.register_buffer(
            "lmk_bary_coords", torch.tensor(lmk_bary_coords, dtype=self.dtype)
        )

        if self.use_face_contour:
            conture_embeddings = np.load(
                config.dynamic_landmark_embedding_path,
                allow_pickle=True,
                encoding="latin1",
            )
            conture_embeddings = conture_embeddings[()]
            dynamic_lmk_faces_idx = np.array(conture_embeddings["lmk_face_idx"]).astype(
                np.int64
            )
            dynamic_lmk_faces_idx = torch.tensor(
                dynamic_lmk_faces_idx, dtype=torch.long
            )
            self.register_buffer("dynamic_lmk_faces_idx", dynamic_lmk_faces_idx)

            dynamic_lmk_bary_coords = conture_embeddings["lmk_b_coords"]
            dynamic_lmk_bary_coords = np.array(dynamic_lmk_bary_coords)
            dynamic_lmk_bary_coords = torch.tensor(
                dynamic_lmk_bary_coords, dtype=self.dtype
            )
            self.register_buffer("dynamic_lmk_bary_coords", dynamic_lmk_bary_coords)

            neck_kin_chain = []
            curr_idx = torch.tensor(self.NECK_IDX, dtype=torch.long)
            while curr_idx != -1:
                neck_kin_chain.append(curr_idx)
                curr_idx = self.parents[curr_idx]
            self.register_buffer("neck_kin_chain", torch.stack(neck_kin_chain))

    def _find_dynamic_lmk_idx_and_bcoords(
        self,
        vertices,
        pose,
        dynamic_lmk_faces_idx,
        dynamic_lmk_b_coords,
        neck_kin_chain,
        dtype=torch.float32,
    ):
        """
        Selects the face contour depending on the reletive position of the head
        Input:
            vertices: N X num_of_vertices X 3
            pose: N X full pose
            dynamic_lmk_faces_idx: The list of contour face indexes
            dynamic_lmk_b_coords: The list of contour barycentric weights
            neck_kin_chain: The tree to consider for the relative rotation
            dtype: Data type
        return:
            The contour face indexes and the corresponding barycentric weights
        Source: Modified for batches from https://github.com/vchoutas/smplx
        """

        batch_size = vertices.shape[0]

        aa_pose = torch.index_select(pose.view(batch_size, -1, 3), 1, neck_kin_chain)
        rot_mats = batch_rodrigues(aa_pose.view(-1, 3)).view(batch_size, -1, 3, 3)

        rel_rot_mat = (
            torch.eye(3, device=vertices.device, dtype=dtype)
            .unsqueeze_(dim=0)
            .expand(batch_size, -1, -1)
        )
        for idx in range(len(neck_kin_chain)):
            rel_rot_mat = torch.bmm(rot_mats[:, idx], rel_rot_mat)

        y_rot_angle = torch.round(
            torch.clamp(-rot_mat_to_euler(rel_rot_mat) * 180.0 / np.pi, max=39)
        ).to(dtype=torch.long)
        neg_mask = y_rot_angle.lt(0).to(dtype=torch.long)
        mask = y_rot_angle.lt(-39).to(dtype=torch.long)
        neg_vals = mask * 78 + (1 - mask) * (39 - y_rot_angle)
        y_rot_angle = neg_mask * neg_vals + (1 - neg_mask) * y_rot_angle

        dyn_lmk_faces_idx = torch.index_select(dynamic_lmk_faces_idx, 0, y_rot_angle)
        dyn_lmk_b_coords = torch.index_select(dynamic_lmk_b_coords, 0, y_rot_angle)

        return dyn_lmk_faces_idx, dyn_lmk_b_coords

    def forward(
        self,
        shape_params=None,
        expression_params=None,
        pose_params=None,
        neck_pose=None,
        eye_pose=None,
        transl=None,
    ):
        """
        Input:
            shape_params: N X number of shape parameters
            expression_params: N X number of expression parameters
            pose_params: N X number of pose parameters
        return:
            vertices: N X V X 3
            landmarks: N X number of landmarks X 3
        """
        bs = shape_params.shape[0]
        if self.batch_size==1:
            
            shape_betas = self.shape_betas.repeat(bs, 1)
            expression_betas = self.expression_betas.repeat(bs, 1)

            betas = torch.cat(
                [shape_params, shape_betas, expression_params, expression_betas],
                dim=1,
            )
            neck_pose = neck_pose if neck_pose is not None else self.neck_pose
            eye_pose = eye_pose if eye_pose is not None else self.eye_pose
            transl = transl if transl is not None else self.transl
            neck_pose = neck_pose.repeat(bs, 1)
            eye_pose = eye_pose.repeat(bs, 1)
            transl = transl.repeat(bs, 1)
            full_pose = torch.cat(
                [pose_params[:, :3], neck_pose, pose_params[:, 3:], eye_pose], dim=1
            )
        else:
            betas = torch.cat(
                [shape_params, self.shape_betas, expression_params, self.expression_betas],
                dim=1,
            )
            neck_pose = neck_pose if neck_pose is not None else self.neck_pose
            eye_pose = eye_pose if eye_pose is not None else self.eye_pose
            transl = transl if transl is not None else self.transl
            full_pose = torch.cat(
                [pose_params[:, :3], neck_pose, pose_params[:, 3:], eye_pose], dim=1
            )
        template_vertices = self.v_template.unsqueeze(0).repeat(bs, 1, 1)

        vertices, _ = lbs(
            betas,
            full_pose,
            template_vertices,
            self.shapedirs,
            self.posedirs,
            self.J_regressor,
            self.parents,
            self.lbs_weights,
        )

        lmk_faces_idx = self.lmk_faces_idx.unsqueeze(dim=0).repeat(bs, 1)
        lmk_bary_coords = self.lmk_bary_coords.unsqueeze(dim=0).repeat(
            bs, 1, 1
        )
        if self.use_face_contour:

            (
                dyn_lmk_faces_idx,
                dyn_lmk_bary_coords,
            ) = self._find_dynamic_lmk_idx_and_bcoords(
                vertices,
                full_pose,
                self.dynamic_lmk_faces_idx,
                self.dynamic_lmk_bary_coords,
                self.neck_kin_chain,
                dtype=self.dtype,
            )

            lmk_faces_idx = torch.cat([dyn_lmk_faces_idx, lmk_faces_idx], 1)
            lmk_bary_coords = torch.cat([dyn_lmk_bary_coords, lmk_bary_coords], 1)

        landmarks = vertices2landmarks(
            vertices, self.faces_tensor, lmk_faces_idx, lmk_bary_coords
        )

        if self.use_3D_translation:
            landmarks += transl.unsqueeze(dim=1)
            vertices += transl.unsqueeze(dim=1)

        return vertices, landmarks


```

## Source file: framework/model/losses/compute_FaceDiff.py

{
  "path": "framework/model/losses/compute_FaceDiff.py",
  "blob_sha": "8311548777f15d760de9235d7548e98e4ee6dc88",
  "sha256": "e7bae9a16f23e1326cf9ba03e7b0733bb2a1aef6e52234cc4f2ce19a35f94589",
  "lines": 62,
  "bytes": 2027,
  "selection_score": 15,
  "keyword_line_counts": {},
  "url": "https://github.com/Dogter521/LSF-Animation/blob/e5a6ab856d94dc3f761146e7e6275f0b9d462e8c/framework/model/losses/compute_FaceDiff.py",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "def __init__(self,  **kwargs):"
    },
    {
      "line": 29,
      "text": "def update(self, pred=None, ref=None):"
    },
    {
      "line": 43,
      "text": "def compute(self):"
    },
    {
      "line": 48,
      "text": "def _update_loss(self, loss: str, *, outputs=None, inputs=None):"
    },
    {
      "line": 55,
      "text": "def loss_to_logname(self, loss: str, split: str):"
    }
  ]
}

```text

import torch
import os
from hydra.utils import instantiate
from pathlib import Path
from torch.nn import Module
from tensorboardX import SummaryWriter


class ComputeLosses(Module):
    def __init__(self,  **kwargs):
        super().__init__()

        # losses = ["recons", "vel", "total"]
        losses = ["recons", "total"]

        self.losses_values = {}
        for loss in losses:
            self.register_buffer(loss, torch.tensor(0.0))

        self.register_buffer("count", torch.tensor(0.0))
        self.losses = losses

        # Instantiate loss functions
        self._losses_func = {loss: instantiate(kwargs[loss + "_func"])
                             for loss in losses if loss not in ["total"]}
        # Save the lambda parameters
        self._params = {loss: kwargs[loss] for loss in losses if loss != "total"}

    def update(self, pred=None, ref=None):
        total: float = 0.0
        total += self._update_loss("recons", outputs=pred, inputs=ref)

        # pred_diff = latent_pred[:, 1:, :] - latent_pred[:, :-1, :]
        # ref_diff = latent_ref[:, 1:, :] - latent_ref[:, :-1, :]
        # total += self._update_loss("vel", outputs=pred_diff, inputs=ref_diff)

        # Update the total loss
        self.total += total.detach()
        self.count += 1

        return total

    def compute(self):
        count = self.count
        loss_dict = {loss: getattr(self, loss) / count for loss in self.losses}
        return loss_dict

    def _update_loss(self, loss: str, *, outputs=None, inputs=None):
        # Update the loss
        val = self._losses_func[loss](outputs, inputs)
        getattr(self, loss).__iadd__(val.detach())
        weighted_loss = self._params[loss] * val
        return weighted_loss

    def loss_to_logname(self, loss: str, split: str):
        if loss in ["recons", "vel", "total"]:
            log_name = f"{loss}/{split}"
        else:
            loss_type, name = loss.split("_")
            log_name = f"{loss_type}/{name}/{split}"
        return log_name



```

## Source file: deps/flame/main.py

{
  "path": "deps/flame/main.py",
  "blob_sha": "a75f61e236f0a6b65dd19ee776428fe190f89442",
  "sha256": "aaee8ccdf0a710f2638ab535a0bcea7bb31dd62151f3dda747425c501c55b73f",
  "lines": 89,
  "bytes": 3406,
  "selection_score": 3,
  "keyword_line_counts": {
    "eye": 4,
    "jaw": 1,
    "flame": 9,
    "rig": 7,
    "face": 3
  },
  "url": "https://github.com/Dogter521/LSF-Animation/blob/e5a6ab856d94dc3f761146e7e6275f0b9d462e8c/deps/flame/main.py",
  "function_signature_candidates": []
}

```text

"""
Demo code to load the FLAME Layer and visualise the 3D landmarks on the Face

Author: Soubhik Sanyal
Copyright (c) 2019, Soubhik Sanyal
All rights reserved.

Max-Planck-Gesellschaft zur Foerderung der Wissenschaften e.V. (MPG) is holder of all proprietary rights on this
computer program.
You can only use this computer program if you have closed a license agreement with MPG or you get the right to use
the computer program from someone who is authorized to grant you that right.
Any use of the computer program without a valid license is prohibited and liable to prosecution.
Copyright 2019 Max-Planck-Gesellschaft zur Foerderung der Wissenschaften e.V. (MPG). acting on behalf of its
Max Planck Institute for Intelligent Systems and the Max Planck Institute for Biological Cybernetics.
All rights reserved.

More information about FLAME is available at http://flame.is.tue.mpg.de.

For questions regarding the PyTorch implementation please contact soubhik.sanyal@tuebingen.mpg.de
"""

import numpy as np
import pyrender
import torch
import trimesh

from flame_pytorch import FLAME, get_config

config = get_config()
radian = np.pi / 180.0
flamelayer = FLAME(config)

# Creating a batch of mean shapes
shape_params = torch.zeros(8, 100).cuda()

# Creating a batch of different global poses
# pose_params_numpy[:, :3] : global rotaation
# pose_params_numpy[:, 3:] : jaw rotaation
pose_params_numpy = np.array(
    [
        [0.0, 30.0 * radian, 0.0, 0.0, 0.0, 0.0],
        [0.0, -30.0 * radian, 0.0, 0.0, 0.0, 0.0],
        [0.0, 85.0 * radian, 0.0, 0.0, 0.0, 0.0],
        [0.0, -48.0 * radian, 0.0, 0.0, 0.0, 0.0],
        [0.0, 10.0 * radian, 0.0, 0.0, 0.0, 0.0],
        [0.0, -15.0 * radian, 0.0, 0.0, 0.0, 0.0],
        [0.0, 0.0 * radian, 0.0, 0.0, 0.0, 0.0],
        [0.0, -0.0 * radian, 0.0, 0.0, 0.0, 0.0],
    ],
    dtype=np.float32,
)
pose_params = torch.tensor(pose_params_numpy, dtype=torch.float32).cuda()

# Cerating a batch of neutral expressions
expression_params = torch.zeros(8, 50, dtype=torch.float32).cuda()
flamelayer.cuda()

# Forward Pass of FLAME, one can easily use this as a layer in a Deep learning Framework
vertice, landmark = flamelayer(
    shape_params, expression_params, pose_params
)  # For RingNet project
print(vertice.size(), landmark.size())

if config.optimize_eyeballpose and config.optimize_neckpose:
    neck_pose = torch.zeros(8, 3).cuda()
    eye_pose = torch.zeros(8, 6).cuda()
    vertice, landmark = flamelayer(
        shape_params, expression_params, pose_params, neck_pose, eye_pose
    )

# Visualize Landmarks
# This visualises the static landmarks and the pose dependent dynamic landmarks used for RingNet project
faces = flamelayer.faces
for i in range(8):
    vertices = vertice[i].detach().cpu().numpy().squeeze()
    joints = landmark[i].detach().cpu().numpy().squeeze()
    vertex_colors = np.ones([vertices.shape[0], 4]) * [0.3, 0.3, 0.3, 0.8]

    tri_mesh = trimesh.Trimesh(vertices, faces, vertex_colors=vertex_colors)
    mesh = pyrender.Mesh.from_trimesh(tri_mesh)
    scene = pyrender.Scene()
    scene.add(mesh)
    sm = trimesh.creation.uv_sphere(radius=0.005)
    sm.visual.vertex_colors = [0.9, 0.1, 0.1, 1.0]
    tfs = np.tile(np.eye(4), (len(joints), 1, 1))
    tfs[:, :3, 3] = joints
    joints_pcl = pyrender.Mesh.from_trimesh(sm, poses=tfs)
    scene.add(joints_pcl)
    pyrender.Viewer(scene, use_raymond_lighting=True)


```
