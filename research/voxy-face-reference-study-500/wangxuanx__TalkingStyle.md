# Facial rendering source scan: wangxuanx/TalkingStyle

Repository: https://github.com/wangxuanx/TalkingStyle

Pinned source revision: 8b5e0ee2ed1b6683e5fb362f92da297ed14ee940

Discovery topic: facial animation

Description: The official pytorch code for TalkingStyle: Personalized Speech-Driven Facial Animation with Style Preservation

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"flame": ["mead/templates/FLAME_sample.ply", "vocaset/templates/FLAME_sample.ply"], "lip": [".history/BIWI/regions/lip_20231212172642.txt", ".history/BIWI/regions/lip_20231212172648.txt", ".history/vocaset/regions/lip_20231212171939.txt", ".history/vocaset/regions/lip_20231212172731.txt", ".history/vocaset/regions/lip_20231212172737.txt", ".history/vocaset/regions/lip_20231212172743.txt", ".history/vocaset/regions/lip_20231212172855.txt", ".history/vocaset/regions/lip_20231212172856.txt", ".history/vocaset/regions/lip_20231212172938.txt", "BIWI/regions/lip.txt", "mead/regions/lip.txt", "vocaset/regions/lip.txt"]}

## Source file: main.py

{
  "path": "main.py",
  "blob_sha": "ed4d9620e56d00b9c2303d9be7abf8e0c02b5e42",
  "sha256": "9e4af6ca48c36511e58b10b0bdd0e8aee514254e1fbaf23e6e55c1715556ba08",
  "lines": 169,
  "bytes": 8156,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 21,
    "face": 9,
    "lip": 1
  },
  "url": "https://github.com/wangxuanx/TalkingStyle/blob/8b5e0ee2ed1b6683e5fb362f92da297ed14ee940/main.py",
  "function_signature_candidates": [
    {
      "line": 12,
      "text": "def trainer(args, train_loader, dev_loader, model, optimizer, criterion, epoch=100):"
    },
    {
      "line": 82,
      "text": "def test(args, model, test_loader,epoch):"
    },
    {
      "line": 116,
      "text": "def count_parameters(model):"
    },
    {
      "line": 119,
      "text": "def main():"
    }
  ]
}

```text

import sys
sys.path.append("/workspace/TalkingStyle")
import numpy as np
import argparse
from tqdm import tqdm
import os, shutil
import torch
import torch.nn as nn
from data_loader import get_dataloaders
from talkingstyle import TalkingStyle

def trainer(args, train_loader, dev_loader, model, optimizer, criterion, epoch=100):
    dev = args.device
    save_path = os.path.join(args.dataset,args.save_path)
    if os.path.exists(save_path):
        shutil.rmtree(save_path)
    os.makedirs(save_path)

    min_loss = 1000

    train_subjects_list = [i for i in args.train_subjects.split(" ")]
    iteration = 0
    for e in range(epoch+1):
        loss_log = []
        # train
        model.train()
        pbar = tqdm(enumerate(train_loader), total=len(train_loader))
        optimizer.zero_grad()

        for i, (audio, vertice, template, one_hot, file_name) in pbar:
            iteration += 1
            # to gpu
            audio, vertice, template, one_hot  = audio.to(dev), vertice.to(dev), template.to(dev), one_hot.to(dev)
            loss, loss_detail = model(audio, template, vertice, one_hot, criterion, teacher_forcing=False)
            loss.backward()
            loss_log.append(loss.item())
            if i % args.gradient_accumulation_steps==0:
                optimizer.step()
                optimizer.zero_grad()

            pbar.set_description("(Epoch {}, iteration {}) TRAIN LOSS:{:.8f}".format((e+1), iteration, np.mean(loss_log)))


        # validation
        valid_loss_log = []
        model.eval()
        for audio, vertice, template, one_hot_all,file_name in dev_loader:
            # to gpu
            audio, vertice, template, one_hot_all= audio.to(dev), vertice.to(dev), template.to(dev), one_hot_all.to(dev)
            train_subject = "_".join(file_name[0].split("_")[:-1])
            if train_subject in train_subjects_list:
                condition_subject = train_subject
                iter = train_subjects_list.index(condition_subject)
                one_hot = one_hot_all[:,iter,:]
                loss, loss_detail = model(audio, template, vertice, one_hot, criterion, teacher_forcing=False)
                valid_loss_log.append(loss.item())
            else:
                for iter in range(one_hot_all.shape[-1]):
                    condition_subject = train_subjects_list[iter]
                    one_hot = one_hot_all[:,iter,:]
                    loss = model(audio, template,  vertice, one_hot, criterion)
                    valid_loss_log.append(loss.item())
                        
        current_loss = np.mean(valid_loss_log)

        if save_path is None:
            os.makedirs(save_path)

        if current_loss < min_loss:
            print("saving best model...")
            print("current loss: ", current_loss)
            min_loss = current_loss
            torch.save(model.state_dict(), os.path.join(save_path,'best_model.pth'))

        if (e > 0 and e % 5 == 0) or e == args.max_epoch:
            torch.save(model.state_dict(), os.path.join(save_path,'{}_model.pth'.format(e)))

        print("epcoh: {}, current loss:{:.7f}".format(e+1,current_loss))
    return model

@torch.no_grad()
def test(args, model, test_loader,epoch):
    dev = args.device

    result_path = os.path.join(args.dataset,args.result_path)
    if os.path.exists(result_path):
        shutil.rmtree(result_path)
    os.makedirs(result_path)

    save_path = os.path.join(args.dataset,args.save_path)
    train_subjects_list = [i for i in args.train_subjects.split(" ")]

    model.load_state_dict(torch.load(os.path.join(save_path, '{}_model.pth'.format(epoch))))
    model = model.to(args.device)
    model.eval()
   
    for audio, vertice, template, one_hot_all, file_name in test_loader:
        # to gpu
        audio, vertice, template, one_hot_all= audio.to(dev), vertice.to(dev), template.to(dev), one_hot_all.to(dev)
        train_subject = "_".join(file_name[0].split("_")[:-1])
        if train_subject in train_subjects_list:
            condition_subject = train_subject
            iter = train_subjects_list.index(condition_subject)
            one_hot = one_hot_all[:,iter,:]
            prediction = model.predict(audio, template, one_hot)
            prediction = prediction.squeeze() # (seq_len, V*3)
            np.save(os.path.join(result_path, file_name[0].split(".")[0]+"_condition_"+condition_subject+".npy"), prediction.detach().cpu().numpy())
        else:
            for iter in range(one_hot_all.shape[-1]):
                condition_subject = train_subjects_list[iter]
                one_hot = one_hot_all[:,iter,:]
                prediction = model.predict(audio, template, one_hot)
                prediction = prediction.squeeze() # (seq_len, V*3)
                np.save(os.path.join(result_path, file_name[0].split(".")[0]+"_condition_"+condition_subject+".npy"), prediction.detach().cpu().numpy())
         
def count_parameters(model):
    return sum(p.numel() for p in model.parameters() if p.requires_grad)

def main():
    parser = argparse.ArgumentParser(description='TalkingStyle: A Novel Approach for the Generation of Personalized and Stylized Talking Face Avatars')
    parser.add_argument("--lr", type=float, default=0.0002, help='learning rate')
    parser.add_argument("--dataset", type=str, default="BIWI", help='vocaset or BIWI')
    parser.add_argument("--vertice_dim", type=int, default=23370*3, help='number of vertices - 5023*3 for vocaset; 23370*3 for BIWI')
    parser.add_argument("--feature_dim", type=int, default=128, help='64 for vocaset; 128 for BIWI')
    parser.add_argument("--period", type=int, default=25, help='period in PPE - 30 for vocaset; 25 for BIWI')
    parser.add_argument("--wav_path", type=str, default= "/workspace/CodeTalker_first_test/BIWI/wav_all", help='path of the audio signals')
    parser.add_argument("--vertices_path", type=str, default="/workspace/CodeTalker_first_test/BIWI/vertices_npy", help='path of the ground truth')
    parser.add_argument("--gradient_accumulation_steps", type=int, default=1, help='gradient accumulation')
    parser.add_argument("--max_epoch", type=int, default=100, help='number of epochs')
    parser.add_argument("--device", type=str, default="cuda:0")
    parser.add_argument("--template_file", type=str, default="templates.pkl", help='path of the personalized templates')
    parser.add_argument("--save_path", type=str, default="save/models", help='path of the trained models')
    parser.add_argument("--result_path", type=str, default="save/result", help='path to the predictions')
    parser.add_argument("--lip_region", type=str, default="BIWI/regions/lip.txt", help='path to the lip region')
    parser.add_argument("--train_subjects", type=str, default="F2 F3 F4 M3 M4 M5")
    parser.add_argument("--val_subjects", type=str, default="F2 F3 F4 M3 M4 M5")
    parser.add_argument("--test_subjects", type=str, default="F1 F5 F6 F7 F8 M1 M2 M6")
    # parser.add_argument("--train_subjects", type=str, default="FaceTalk_170728_03272_TA"
    #    " FaceTalk_170904_00128_TA FaceTalk_170725_00137_TA FaceTalk_170915_00223_TA"
    #    " FaceTalk_170811_03274_TA FaceTalk_170913_03279_TA"
    #    " FaceTalk_170904_03276_TA FaceTalk_170912_03278_TA")
    # parser.add_argument("--val_subjects", type=str, default="FaceTalk_170811_03275_TA"
    #    " FaceTalk_170908_03277_TA")
    # parser.add_argument("--test_subjects", type=str, default="FaceTalk_170809_00138_TA"
    #    " FaceTalk_170731_00024_TA")
    args = parser.parse_args()

    #build model
    model = TalkingStyle(args)
    print("model parameters: ", count_parameters(model))

    # to cuda
    assert torch.cuda.is_available()
    model = model.to(args.device)
    
    #load data
    dataset = get_dataloaders(args)

    # loss
    criterion = nn.MSELoss()

    # Train the model
    optimizer = torch.optim.Adam(filter(lambda p: p.requires_grad, model.parameters()), lr=args.lr)
    model = trainer(args, dataset["train"], dataset["test"], model, optimizer, criterion, epoch=args.max_epoch)
    
    test(args, model, dataset["test"], epoch=args.max_epoch)
    
if __name__=="__main__":
    main()

```

## Source file: hubert.py

{
  "path": "hubert.py",
  "blob_sha": "9c0f49912006bdae723d668f53688184a8768ece",
  "sha256": "97dbc43d286243e01c3c81af4ca3add0181bea61f0a2676a46428005f20a9ae9",
  "lines": 146,
  "bytes": 5917,
  "selection_score": 0,
  "keyword_line_counts": {
    "lip": 1
  },
  "url": "https://github.com/wangxuanx/TalkingStyle/blob/8b5e0ee2ed1b6683e5fb362f92da297ed14ee940/hubert.py",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "def _compute_mask_indices("
    },
    {
      "line": 60,
      "text": "def linear_interpolation(features, input_fps, output_fps, output_len=None):"
    },
    {
      "line": 69,
      "text": "def __init__(self, config):"
    },
    {
      "line": 71,
      "text": "def forward("
    }
  ]
}

```text

import torch
import torch.nn.functional as F
import numpy as np
from transformers import HubertModel, HubertConfig
from transformers.modeling_outputs import BaseModelOutput
from typing import Optional, Tuple
_CONFIG_FOR_DOC = "HubertConfig"

# initialize our encoder with the pre-trained hubert weights.
def _compute_mask_indices(
    shape: Tuple[int, int],
    mask_prob: float,
    mask_length: int,
    attention_mask: Optional[torch.Tensor] = None,
    min_masks: int = 0,
) -> np.ndarray:
    bsz, all_sz = shape
    mask = np.full((bsz, all_sz), False)

    all_num_mask = int(
        mask_prob * all_sz / float(mask_length)
        + np.random.rand()
    )
    all_num_mask = max(min_masks, all_num_mask)
    mask_idcs = []
    padding_mask = attention_mask.ne(1) if attention_mask is not None else None
    for i in range(bsz):
        if padding_mask is not None:
            sz = all_sz - padding_mask[i].long().sum().item()
            num_mask = int(
                mask_prob * sz / float(mask_length)
                + np.random.rand()
            )
            num_mask = max(min_masks, num_mask)
        else:
            sz = all_sz
            num_mask = all_num_mask

        lengths = np.full(num_mask, mask_length)

        if sum(lengths) == 0:
            lengths[0] = min(mask_length, sz - 1)

        min_len = min(lengths)
        if sz - min_len <= num_mask:
            min_len = sz - num_mask - 1

        mask_idc = np.random.choice(sz - min_len, num_mask, replace=False)
        mask_idc = np.asarray([mask_idc[j] + offset for j in range(len(mask_idc)) for offset in range(lengths[j])])
        mask_idcs.append(np.unique(mask_idc[mask_idc < sz]))

    min_len = min([len(m) for m in mask_idcs])
    for i, mask_idc in enumerate(mask_idcs):
        if len(mask_idc) > min_len:
            mask_idc = np.random.choice(mask_idc, min_len, replace=False)
        mask[i, mask_idc] = True
    return mask

# linear interpolation layer
def linear_interpolation(features, input_fps, output_fps, output_len=None):
    features = features.transpose(1, 2)
    seq_len = features.shape[2] / float(input_fps)
    if output_len is None:
        output_len = int(seq_len * output_fps)
    output_features = F.interpolate(features,size=output_len, align_corners=True,mode='linear')
    return output_features.transpose(1, 2)

class HubertModel(HubertModel):
    def __init__(self, config):
        super().__init__(config)
    def forward(
        self,
        input_values,
        dataset,
        attention_mask=None,
        output_attentions=None,
        output_hidden_states=None,
        return_dict=None,
        frame_num=None
    ):
        self.config.output_attentions = True
        output_attentions = output_attentions if output_attentions is not None else self.config.output_attentions
        output_hidden_states = (
            output_hidden_states if output_hidden_states is not None else self.config.output_hidden_states
        )
        return_dict = return_dict if return_dict is not None else self.config.use_return_dict

        hidden_states = self.feature_extractor(input_values) #torch.Size([1, 512, x])
        hidden_states = hidden_states.transpose(1, 2) #torch.Size([1, x, 512])

        if dataset == "BIWI":
            # cut audio feature
            if hidden_states.shape[1]%2 != 0:
                hidden_states = hidden_states[:, :-1]
            if frame_num and hidden_states.shape[1]>frame_num*2:
                hidden_states = hidden_states[:, :frame_num*2]
        elif dataset == "vocaset":
            hidden_states = linear_interpolation(hidden_states, 50, 30, output_len=frame_num)
     
        if attention_mask is not None:
            output_lengths = self._get_feat_extract_output_lengths(attention_mask.sum(-1))
            attention_mask = torch.zeros(
                hidden_states.shape[:2], dtype=hidden_states.dtype, device=hidden_states.device
            )
            attention_mask[
                (torch.arange(attention_mask.shape[0], device=hidden_states.device), output_lengths - 1)
            ] = 1
            attention_mask = attention_mask.flip([-1]).cumsum(-1).flip([-1]).bool()

        hidden_states = self.feature_projection(hidden_states)

        if self.config.apply_spec_augment and self.training:
            batch_size, sequence_length, hidden_size = hidden_states.size()
            if self.config.mask_time_prob > 0:
                mask_time_indices = _compute_mask_indices(
                    (batch_size, sequence_length),
                    self.config.mask_time_prob,
                    self.config.mask_time_length,
                    attention_mask=attention_mask,
                    min_masks=2,
                )
                hidden_states[torch.from_numpy(mask_time_indices)] = self.masked_spec_embed.to(hidden_states.dtype)
            if self.config.mask_feature_prob > 0:
                mask_feature_indices = _compute_mask_indices(
                    (batch_size, hidden_size),
                    self.config.mask_feature_prob,
                    self.config.mask_feature_length,
                )
                mask_feature_indices = torch.from_numpy(mask_feature_indices).to(hidden_states.device)
                hidden_states[mask_feature_indices[:, None].expand(-1, sequence_length, -1)] = 0
        encoder_outputs = self.encoder(
            hidden_states,
            attention_mask=attention_mask,
            output_attentions=output_attentions,
            output_hidden_states=output_hidden_states,
            return_dict=return_dict,
        )
        hidden_states = encoder_outputs[0]
        if not return_dict:
            return (hidden_states,) + encoder_outputs[1:]

        return BaseModelOutput(
            last_hidden_state=hidden_states,
            hidden_states=encoder_outputs.hidden_states,
            attentions=encoder_outputs.attentions,
        )


```

## Source file: metric.py

{
  "path": "metric.py",
  "blob_sha": "199421cacacf3822215199af978296bdbf022237",
  "sha256": "f3f6b82951917b12bc2fb7eb364d43ddd8ed23c57c6de23526012acb9d6cb595",
  "lines": 81,
  "bytes": 3341,
  "selection_score": 0,
  "keyword_line_counts": {
    "mouth": 6,
    "gum": 6,
    "lip": 1
  },
  "url": "https://github.com/wangxuanx/TalkingStyle/blob/8b5e0ee2ed1b6683e5fb362f92da297ed14ee940/metric.py",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "def main():"
    }
  ]
}

```text

import numpy as np
import argparse
import os
import pickle


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--train_subjects", type=str, default="F2 F3 F4 M3 M4 M5")
    parser.add_argument("--pred_path", type=str, default="/workspace/TalkingStyle/BIWI/save_concat/result")
    parser.add_argument("--gt_path", type=str, default="/workspace/CodeTalker_first_test/BIWI/vertices_npy")
    parser.add_argument("--region_path", type=str, default="BIWI/regions/")
    parser.add_argument("--templates_path", type=str, default="BIWI/templates.pkl")
    args = parser.parse_args()

    train_subject_list = args.train_subjects.split(" ")
    sentence_list = ["e"+str(i).zfill(2) for i in range(37,41)]

    with open(args.templates_path, 'rb') as fin:
        templates = pickle.load(fin,encoding='latin1')

    with open(os.path.join(args.region_path, "lve.txt")) as f:
        maps = f.read().split(", ")
        mouth_map = [int(i) for i in maps]

    with open(os.path.join(args.region_path, "fdd.txt")) as f:
        maps = f.read().split(", ")
        upper_map = [int(i) for i in maps]
    

    cnt = 0
    vertices_gt_all = []
    vertices_pred_all = []
    motion_std_difference = []

    for subject in train_subject_list:
        for sentence in sentence_list:
            vertices_gt = np.load(os.path.join(args.gt_path,subject+"_"+sentence+".npy")).reshape(-1,23370,3)
            vertices_pred = np.load(os.path.join(args.pred_path,subject+"_"+sentence+"_condition_"+subject+".npy")).reshape(-1,23370,3)
            vertices_pred = vertices_pred[:vertices_gt.shape[0],:,:]
            vertices_gt = vertices_gt[:vertices_pred.shape[0],:,:]

            motion_pred = vertices_pred - templates[subject].reshape(1,23370,3)
            motion_gt = vertices_gt - templates[subject].reshape(1,23370,3)

            cnt += vertices_gt.shape[0]

            vertices_gt_all.extend(list(vertices_gt))
            vertices_pred_all.extend(list(vertices_pred))

            L2_dis_upper = np.array([np.square(motion_gt[:,v, :]) for v in upper_map])
            L2_dis_upper = np.transpose(L2_dis_upper, (1,0,2))
            L2_dis_upper = np.sum(L2_dis_upper,axis=2)
            L2_dis_upper = np.std(L2_dis_upper, axis=0)
            gt_motion_std = np.mean(L2_dis_upper)
            
            L2_dis_upper = np.array([np.square(motion_pred[:,v, :]) for v in upper_map])
            L2_dis_upper = np.transpose(L2_dis_upper, (1,0,2))
            L2_dis_upper = np.sum(L2_dis_upper,axis=2)
            L2_dis_upper = np.std(L2_dis_upper, axis=0)
            pred_motion_std = np.mean(L2_dis_upper)

            motion_std_difference.append(gt_motion_std - pred_motion_std)

    print('Frame Number: {}'.format(cnt))

    vertices_gt_all = np.array(vertices_gt_all)
    vertices_pred_all = np.array(vertices_pred_all)
    

    L2_dis_mouth_max = np.array([np.square(vertices_gt_all[:,v, :]-vertices_pred_all[:,v,:]) for v in mouth_map])
    L2_dis_mouth_max = np.transpose(L2_dis_mouth_max, (1,0,2))
    L2_dis_mouth_max = np.sum(L2_dis_mouth_max,axis=2)
    L2_dis_mouth_max = np.max(L2_dis_mouth_max,axis=1)

    print('Lip Vertex Error: {:.4e}'.format(np.mean(L2_dis_mouth_max)))
    print('FDD: {:.4e}'.format(sum(motion_std_difference)/len(motion_std_difference)))

    
if __name__=="__main__":
    main()

```

## Source file: render.py

{
  "path": "render.py",
  "blob_sha": "79bda3e4ddeb8e7106e832799b2805c5f10cba3c",
  "sha256": "ccc4aef1bb09df14e2b253f6472953ee601850bf46ba02a51a0ce311ff5f0be4",
  "lines": 185,
  "bytes": 8739,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 3,
    "gum": 8,
    "flame": 2,
    "rig": 10,
    "face": 4
  },
  "url": "https://github.com/wangxuanx/TalkingStyle/blob/8b5e0ee2ed1b6683e5fb362f92da297ed14ee940/render.py",
  "function_signature_candidates": [
    {
      "line": 28,
      "text": "def render_mesh_helper(args, mesh, t_center, rot=np.zeros(3), tex_img=None,  z_offset=0):"
    },
    {
      "line": 108,
      "text": "def render_sequence_meshes(args, sequence_vertices, template, out_path, predicted_vertices_path, vt, ft ,tex_img):"
    },
    {
      "line": 143,
      "text": "def main():"
    }
  ]
}

```text

'''
Max-Planck-Gesellschaft zur Foerderung der Wissenschaften e.V. (MPG) is holder of all proprietary rights on this
computer program.
You can only use this computer program if you have closed a license agreement with MPG or you get the right to use
the computer program from someone who is authorized to grant you that right.
Any use of the computer program without a valid license is prohibited and liable to prosecution.
Copyright 2019 Max-Planck-Gesellschaft zur Foerderung der Wissenschaften e.V. (MPG). acting on behalf of its
Max Planck Institute for Intelligent Systems and the Max Planck Institute for Biological Cybernetics.
All rights reserved.
More information about VOCA is available at http://voca.is.tue.mpg.de.
For comments or questions, please email us at voca@tue.mpg.de
'''

import os, shutil
import pickle
import cv2
import scipy
import tempfile
import numpy as np
from subprocess import call
import argparse
os.environ['PYOPENGL_PLATFORM'] = 'osmesa' #egl
import pyrender
import trimesh
from psbody.mesh import Mesh

# The implementation of rendering is borrowed from VOCA: https://github.com/TimoBolkart/voca/blob/master/utils/rendering.py
def render_mesh_helper(args, mesh, t_center, rot=np.zeros(3), tex_img=None,  z_offset=0):
    if args.dataset == "BIWI":
        camera_params = {'c': np.array([800, 800]),
                         'k': np.array([-0.19816071, 0.92822711, 0, 0, 0]),
                         'f': np.array([4754.97941935 / 4, 4754.97941935 / 4])}
    elif args.dataset == "vocaset":
        camera_params = {'c': np.array([800, 800]),
                         'k': np.array([-0.19816071, 0.92822711, 0, 0, 0]),
                         'f': np.array([4754.97941935, 4754.97941935])}

    frustum = {'near': 0.01, 'far': 3.0, 'height': 1600, 'width': 1600}  # {'near': 0.01, 'far': 3.0, 'height': 800, 'width': 800}

    mesh_copy = Mesh(mesh.v, mesh.f)
    mesh_copy.v[:] = cv2.Rodrigues(rot)[0].dot((mesh_copy.v-t_center).T).T+t_center

    intensity = 2.0
    rgb_per_v = None

    primitive_material = pyrender.material.MetallicRoughnessMaterial(
                alphaMode='BLEND',
                baseColorFactor=[0.3, 0.3, 0.3, 1.0],
                metallicFactor=0.8,
                roughnessFactor=0.8
            )

    tri_mesh = trimesh.Trimesh(vertices=mesh_copy.v, faces=mesh_copy.f, vertex_colors=rgb_per_v)
    render_mesh = pyrender.Mesh.from_trimesh(tri_mesh, material=primitive_material,smooth=True)

    if args.background_black:
        scene = pyrender.Scene(ambient_light=[.2, .2, .2], bg_color=[0, 0, 0])#[0, 0, 0] black,[255, 255, 255] white
    else:
        scene = pyrender.Scene(ambient_light=[.2, .2, .2], bg_color=[255, 255, 255])#[0, 0, 0] black,[255, 255, 255] white

    camera = pyrender.IntrinsicsCamera(fx=camera_params['f'][0],
                                      fy=camera_params['f'][1],
                                      cx=camera_params['c'][0],
                                      cy=camera_params['c'][1],
                                      znear=frustum['near'],
                                      zfar=frustum['far'])

    scene.add(render_mesh, pose=np.eye(4))

    camera_pose = np.eye(4)
    camera_pose[:3,3] = np.array([0, 0, 1.0-z_offset])
    scene.add(camera, pose=[[1, 0, 0, 0],
                            [0, 1, 0, 0],
                            [0, 0, 1, 1],
                            [0, 0, 0, 1]])

    angle = np.pi / 6.0
    pos = camera_pose[:3,3]
    light_color = np.array([1., 1., 1.])
    light = pyrender.DirectionalLight(color=light_color, intensity=intensity)

    light_pose = np.eye(4)
    light_pose[:3,3] = pos
    scene.add(light, pose=light_pose.copy())

    light_pose[:3,3] = cv2.Rodrigues(np.array([angle, 0, 0]))[0].dot(pos)
    scene.add(light, pose=light_pose.copy())

    light_pose[:3,3] =  cv2.Rodrigues(np.array([-angle, 0, 0]))[0].dot(pos)
    scene.add(light, pose=light_pose.copy())

    light_pose[:3,3] = cv2.Rodrigues(np.array([0, -angle, 0]))[0].dot(pos)
    scene.add(light, pose=light_pose.copy())

    light_pose[:3,3] = cv2.Rodrigues(np.array([0, angle, 0]))[0].dot(pos)
    scene.add(light, pose=light_pose.copy())

    flags = pyrender.RenderFlags.SKIP_CULL_FACES
    try:
        r = pyrender.OffscreenRenderer(viewport_width=frustum['width'], viewport_height=frustum['height'])
        color, _ = r.render(scene, flags=flags)
    except:
        print('pyrender: Failed rendering frame')
        color = np.zeros((frustum['height'], frustum['width'], 3), dtype='uint8')

    return color[..., ::-1]

def render_sequence_meshes(args, sequence_vertices, template, out_path, predicted_vertices_path, vt, ft ,tex_img):
    num_frames = sequence_vertices.shape[0]
    print("num_frames",num_frames)
    file_name_pred = predicted_vertices_path.split('/')[-1].split('.')[0]
    # file_name_wav = os.path.join('/workspace/CodeTalker_first_test/BIWI/wav_all', file_name_pred.split('_condition_')[0]+'.wav')
    file_name_wav = os.path.join('/workspace/CodeTalker_first_test/BIWI/wav_all', file_name_pred.split('_condition_')[0]+'.wav')
    tmp_video_file_pred = tempfile.NamedTemporaryFile('w', suffix='.mp4', dir=out_path)
    writer_pred = cv2.VideoWriter(tmp_video_file_pred.name, cv2.VideoWriter_fourcc(*'mp4v'), args.fps, (1600, 1600), True)

    center = np.mean(sequence_vertices[0], axis=0)
    video_fname_pred = os.path.join(out_path, file_name_pred+'.mp4')
    for i_frame in range(num_frames):
        render_mesh = Mesh(sequence_vertices[i_frame], template.f)
        if vt is not None and ft is not None:
            render_mesh.vt, render_mesh.ft = vt, ft
        pred_img = render_mesh_helper(args, render_mesh, center, tex_img=tex_img)
        pred_img = pred_img.astype(np.uint8)
        img = pred_img
        writer_pred.write(img)

    writer_pred.release()

    # render without audio
    cmd = ('ffmpeg' + ' -i {0} -pix_fmt yuv420p -qscale 0 {1}'.format(
       tmp_video_file_pred.name, video_fname_pred)).split()
    call(cmd)

    # render with audio
    cmd = ('ffmpeg' + ' -i {0} -i {1} -vcodec h264 -ac 2 -channel_layout stereo -qscale 0 {2}'.format(
       file_name_wav, video_fname_pred, video_fname_pred.replace('.mp4', '_audio.mp4'))).split()
    call(cmd)
    if os.path.exists(video_fname_pred):
        os.remove(video_fname_pred)


def main():
    parser = argparse.ArgumentParser(description='TalkingStyle: A Novel Approach for the Generation of Personalized and Stylized Talking Face Avatars')
    parser.add_argument("--dataset", type=str, default="BIWI", help='vocaset or BIWI')
    parser.add_argument("--render_template_path", type=str, default="templates", help='path of the mesh in FLAME/BIWI topology')
    parser.add_argument('--background_black', type=bool, default=False, help='whether to use black background')
    parser.add_argument('--fps', type=int,default=25, help='frame rate - 30 for vocaset; 25 for BIWI')
    parser.add_argument("--vertice_dim", type=int, default=23370*3, help='number of vertices - 5023*3 for vocaset; 23370*3 for BIWI')
    parser.add_argument("--pred_path", type=str, default="/workspace/TalkingStyle/BIWI/save_film/result/100", help='path of the predictions')
    parser.add_argument("--output", type=str, default="/workspace/TalkingStyle/BIWI/save_film/render", help='path of the rendered video sequences')
    args = parser.parse_args()

    pred_path = os.path.join(args.dataset,args.pred_path)
    output_path = os.path.join(args.dataset,args.output)
   
    if not os.path.exists(output_path):
        os.makedirs(output_path)

    for file in os.listdir(pred_path):
        
        if file.endswith("npy"):
            predicted_vertices_path = os.path.join(pred_path,file)
            if args.dataset == "BIWI":
                template_file = os.path.join(args.dataset, args.render_template_path, "BIWI.ply")
            elif args.dataset == "vocaset":
                template_file = os.path.join(args.dataset, args.render_template_path, "FLAME_sample.ply")
            print("rendering: ", file)

            template = Mesh(filename=template_file)
            vt, ft = None, None
            tex_img = None

            predicted_vertices = np.load(predicted_vertices_path)
            predicted_vertices = np.reshape(predicted_vertices,(-1, args.vertice_dim//3, 3))

            # with open('/workspace/TalkingStyle/vocaset/templates.pkl', 'rb') as fin:
            #     templates = pickle.load(fin, encoding='latin1')

            # predicted_vertices = np.zeros_like(predicted_vertices) + templates['FaceTalk_170809_00138_TA']

            render_sequence_meshes(args,predicted_vertices, template, output_path,predicted_vertices_path,vt, ft ,tex_img)

if __name__=="__main__":
    main()


```
