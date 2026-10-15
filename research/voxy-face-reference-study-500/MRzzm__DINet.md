# Facial rendering source scan: MRzzm/DINet

Repository: https://github.com/MRzzm/DINet

Pinned source revision: 3b57fb0a2482213327890fbb76baeafdaa412597

Discovery topic: face deformation

Description: The source code of "DINet: deformation inpainting network for realistic face visually dubbing on high resolution video."

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"lip": ["dataset/dataset_DINet_clip.py", "train_DINet_clip.py"]}

## Source file: train_DINet_clip.py

{
  "path": "train_DINet_clip.py",
  "blob_sha": "d929b21078d2215fb71c975560276636a353525c",
  "sha256": "0782a980d6f500f0a3d09de4895621559a67fb1d38a7766c7fdac628e4706381",
  "lines": 151,
  "bytes": 7860,
  "selection_score": 15,
  "keyword_line_counts": {
    "mouth": 4,
    "lip": 18
  },
  "url": "https://github.com/MRzzm/DINet/blob/3b57fb0a2482213327890fbb76baeafdaa412597/train_DINet_clip.py",
  "function_signature_candidates": []
}

```text

from models.Discriminator import Discriminator
from models.VGG19 import Vgg19
from models.DINet import DINet
from models.Syncnet import SyncNetPerception
from utils.training_utils import get_scheduler, update_learning_rate,GANLoss
from config.config import DINetTrainingOptions
from sync_batchnorm import convert_model
from torch.utils.data import DataLoader
from dataset.dataset_DINet_clip import DINetDataset


import random
import numpy as np
import torch
import torch.nn as nn
import torch.optim as optim
import os
import torch.nn.functional as F

if __name__ == "__main__":
    '''
            clip training code of DINet
            in the resolution you want, using clip training code after frame training
            
        '''
    # load config
    opt = DINetTrainingOptions().parse_args()
    random.seed(opt.seed)
    np.random.seed(opt.seed)
    torch.cuda.manual_seed(opt.seed)
    # load training data
    train_data = DINetDataset(opt.train_data,opt.augment_num,opt.mouth_region_size)
    training_data_loader = DataLoader(dataset=train_data,  batch_size=opt.batch_size, shuffle=True,drop_last=True)
    train_data_length = len(training_data_loader)
    # init network
    net_g = DINet(opt.source_channel,opt.ref_channel,opt.audio_channel).cuda()
    net_dI = Discriminator(opt.source_channel ,opt.D_block_expansion, opt.D_num_blocks, opt.D_max_features).cuda()
    net_dV = Discriminator(opt.source_channel * 5, opt.D_block_expansion, opt.D_num_blocks, opt.D_max_features).cuda()
    net_vgg = Vgg19().cuda()
    net_lipsync = SyncNetPerception(opt.pretrained_syncnet_path).cuda()
    # parallel
    net_g = nn.DataParallel(net_g)
    net_g = convert_model(net_g)
    net_dI = nn.DataParallel(net_dI)
    net_dV = nn.DataParallel(net_dV)
    net_vgg = nn.DataParallel(net_vgg)
    # setup optimizer
    optimizer_g = optim.Adam(net_g.parameters(), lr=opt.lr_g)
    optimizer_dI = optim.Adam(net_dI.parameters(), lr=opt.lr_dI)
    optimizer_dV = optim.Adam(net_dV.parameters(), lr=opt.lr_dI)
    ## load frame trained DInet weight
    print('loading frame trained DINet weight from: {}'.format(opt.pretrained_frame_DINet_path))
    checkpoint = torch.load(opt.pretrained_frame_DINet_path)
    net_g.load_state_dict(checkpoint['state_dict']['net_g'])
    # set criterion
    criterionGAN = GANLoss().cuda()
    criterionL1 = nn.L1Loss().cuda()
    criterionMSE = nn.MSELoss().cuda()
    # set scheduler
    net_g_scheduler = get_scheduler(optimizer_g, opt.non_decay, opt.decay)
    net_dI_scheduler = get_scheduler(optimizer_dI, opt.non_decay, opt.decay)
    net_dV_scheduler = get_scheduler(optimizer_dV, opt.non_decay, opt.decay)
    # set label of syncnet perception loss
    real_tensor = torch.tensor(1.0).cuda()
    # start train
    for epoch in range(opt.start_epoch, opt.non_decay+opt.decay+1):
        net_g.train()
        for iteration, data in enumerate(training_data_loader):
            # forward
            source_clip,source_clip_mask, reference_clip,deep_speech_clip,deep_speech_full = data
            source_clip = torch.cat(torch.split(source_clip, 1, dim=1), 0).squeeze(1).float().cuda()
            source_clip_mask = torch.cat(torch.split(source_clip_mask, 1, dim=1), 0).squeeze(1).float().cuda()
            reference_clip = torch.cat(torch.split(reference_clip, 1, dim=1), 0).squeeze(1).float().cuda()
            deep_speech_clip = torch.cat(torch.split(deep_speech_clip, 1, dim=1), 0).squeeze(1).float().cuda()
            deep_speech_full = deep_speech_full.float().cuda()
            fake_out = net_g(source_clip_mask,reference_clip,deep_speech_clip)
            fake_out_half = F.avg_pool2d(fake_out, 3, 2, 1, count_include_pad=False)
            source_clip_half = F.interpolate(source_clip, scale_factor=0.5, mode='bilinear')
            # (1) Update DI network
            optimizer_dI.zero_grad()
            _,pred_fake_dI = net_dI(fake_out)
            loss_dI_fake = criterionGAN(pred_fake_dI, False)
            _,pred_real_dI = net_dI(source_clip)
            loss_dI_real = criterionGAN(pred_real_dI, True)
            # Combined DI loss
            loss_dI = (loss_dI_fake + loss_dI_real) * 0.5
            loss_dI.backward(retain_graph=True)
            optimizer_dI.step()

            # (2) Update DV network
            optimizer_dV.zero_grad()
            condition_fake_dV = torch.cat(torch.split(fake_out, opt.batch_size, dim=0), 1)
            _, pred_fake_dV = net_dV(condition_fake_dV)
            loss_dV_fake = criterionGAN(pred_fake_dV, False)
            condition_real_dV = torch.cat(torch.split(source_clip, opt.batch_size, dim=0), 1)
            _, pred_real_dV = net_dV(condition_real_dV)
            loss_dV_real = criterionGAN(pred_real_dV, True)
            # Combined DV loss
            loss_dV = (loss_dV_fake + loss_dV_real) * 0.5
            loss_dV.backward(retain_graph=True)
            optimizer_dV.step()

            # (2) Update DINet
            _, pred_fake_dI = net_dI(fake_out)
            _, pred_fake_dV = net_dV(condition_fake_dV)
            optimizer_g.zero_grad()
            # compute perception loss
            perception_real = net_vgg(source_clip)
            perception_fake = net_vgg(fake_out)
            perception_real_half = net_vgg(source_clip_half)
            perception_fake_half = net_vgg(fake_out_half)
            loss_g_perception = 0
            for i in range(len(perception_real)):
                loss_g_perception += criterionL1(perception_fake[i], perception_real[i])
                loss_g_perception += criterionL1(perception_fake_half[i], perception_real_half[i])
            loss_g_perception = (loss_g_perception / (len(perception_real) * 2)) * opt.lamb_perception
            # # gan dI loss
            loss_g_dI = criterionGAN(pred_fake_dI, True)
            # # gan dV loss
            loss_g_dV = criterionGAN(pred_fake_dV, True)
            ## sync perception loss
            fake_out_clip = torch.cat(torch.split(fake_out, opt.batch_size, dim=0), 1)
            fake_out_clip_mouth = fake_out_clip[:, :, train_data.radius:train_data.radius + train_data.mouth_region_size,
            train_data.radius_1_4:train_data.radius_1_4 + train_data.mouth_region_size]
            sync_score = net_lipsync(fake_out_clip_mouth, deep_speech_full)
            loss_sync = criterionMSE(sync_score, real_tensor.expand_as(sync_score)) * opt.lamb_syncnet_perception
            # combine all losses
            loss_g =   loss_g_perception + loss_g_dI +loss_g_dV + loss_sync
            loss_g.backward()
            optimizer_g.step()

            print(
                "===> Epoch[{}]({}/{}):  Loss_DI: {:.4f} Loss_GI: {:.4f} Loss_DV: {:.4f} Loss_GV: {:.4f} Loss_perception: {:.4f} Loss_sync: {:.4f} lr_g = {:.7f} ".format(
                    epoch, iteration, len(training_data_loader), float(loss_dI), float(loss_g_dI),float(loss_dV), float(loss_g_dV), float(loss_g_perception),float(loss_sync),
                    optimizer_g.param_groups[0]['lr']))

        update_learning_rate(net_g_scheduler, optimizer_g)
        update_learning_rate(net_dI_scheduler, optimizer_dI)
        update_learning_rate(net_dV_scheduler, optimizer_dV)
        # checkpoint
        if epoch %  opt.checkpoint == 0:
            if not os.path.exists(opt.result_path):
                os.mkdir(opt.result_path)
            model_out_path = os.path.join(opt.result_path, 'netG_model_epoch_{}.pth'.format(epoch))
            states = {
                'epoch': epoch + 1,
                'state_dict': {'net_g': net_g.state_dict(),'net_dI': net_dI.state_dict(),'net_dV': net_dV.state_dict()},
                'optimizer': {'net_g': optimizer_g.state_dict(), 'net_dI': optimizer_dI.state_dict(), 'net_dV': optimizer_dV.state_dict()}
            }
            torch.save(states, model_out_path)
            print("Checkpoint saved to {}".format(epoch))


```

## Source file: dataset/dataset_DINet_clip.py

{
  "path": "dataset/dataset_DINet_clip.py",
  "blob_sha": "7d5e8c6a0c980b899bf8f19accdab6ac2b1a4329",
  "sha256": "b2082b4ff72a47a2f6ac29f0f9e5cc73d87ca5716a2fbb296c0dc027d997b90f",
  "lines": 111,
  "bytes": 6213,
  "selection_score": 15,
  "keyword_line_counts": {
    "mouth": 5,
    "lip": 39
  },
  "url": "https://github.com/MRzzm/DINet/blob/3b57fb0a2482213327890fbb76baeafdaa412597/dataset/dataset_DINet_clip.py",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "def get_data(json_name,augment_num):"
    },
    {
      "line": 24,
      "text": "def __init__(self,path_json,augment_num,mouth_region_size):"
    },
    {
      "line": 34,
      "text": "def __getitem__(self, index):"
    },
    {
      "line": 110,
      "text": "def __len__(self):"
    }
  ]
}

```text

import torch
import numpy as np
import json
import random
import cv2

from torch.utils.data import Dataset


def get_data(json_name,augment_num):
    print('start loading data')
    with open(json_name,'r') as f:
        data_dic = json.load(f)
    data_dic_name_list = []
    for augment_index in range(augment_num):
        for video_name in data_dic.keys():
            data_dic_name_list.append(video_name)
    random.shuffle(data_dic_name_list)
    print('finish loading')
    return data_dic_name_list,data_dic


class DINetDataset(Dataset):
    def __init__(self,path_json,augment_num,mouth_region_size):
        super(DINetDataset, self).__init__()
        self.data_dic_name_list,self.data_dic = get_data(path_json,augment_num)
        self.mouth_region_size = mouth_region_size
        self.radius = mouth_region_size//2
        self.radius_1_4 = self.radius//4
        self.img_h = self.radius * 3 + self.radius_1_4
        self.img_w = self.radius * 2 + self.radius_1_4 * 2
        self.length = len(self.data_dic_name_list)

    def __getitem__(self, index):
        video_name = self.data_dic_name_list[index]
        video_clip_num = len(self.data_dic[video_name]['clip_data_list'])
        source_anchor = random.sample(range(video_clip_num), 1)[0]
        source_image_path_list = self.data_dic[video_name]['clip_data_list'][source_anchor]['frame_path_list']
        source_clip_list = []
        source_clip_mask_list = []
        deep_speech_list = []
        reference_clip_list = []
        for source_frame_index in range(2, 2 + 5):
            ## load source clip
            source_image_data = cv2.imread(source_image_path_list[source_frame_index])[:, :, ::-1]
            source_image_data = cv2.resize(source_image_data, (self.img_w, self.img_h)) / 255.0
            source_clip_list.append(source_image_data)
            source_image_mask = source_image_data.copy()
            source_image_mask[self.radius:self.radius + self.mouth_region_size,
            self.radius_1_4:self.radius_1_4 + self.mouth_region_size, :] = 0
            source_clip_mask_list.append(source_image_mask)

            ## load deep speech feature
            deepspeech_array = np.array(self.data_dic[video_name]['clip_data_list'][source_anchor]['deep_speech_list'][
                                       source_frame_index - 2:source_frame_index + 3])
            deep_speech_list.append(deepspeech_array)

            ## ## load reference images
            reference_frame_list = []
            reference_anchor_list = random.sample(range(video_clip_num), 5)
            for reference_anchor in reference_anchor_list:
                reference_frame_path_list = self.data_dic[video_name]['clip_data_list'][reference_anchor][
                    'frame_path_list']
                reference_random_index = random.sample(range(9), 1)[0]
                reference_frame_path = reference_frame_path_list[reference_random_index]
                reference_frame_data = cv2.imread(reference_frame_path)[:, :, ::-1]
                reference_frame_data = cv2.resize(reference_frame_data, (self.img_w, self.img_h)) / 255.0
                reference_frame_list.append(reference_frame_data)
            reference_clip_list.append(np.concatenate(reference_frame_list, 2))

        source_clip = np.stack(source_clip_list, 0)
        source_clip_mask = np.stack(source_clip_mask_list, 0)
        deep_speech_clip = np.stack(deep_speech_list, 0)
        reference_clip = np.stack(reference_clip_list, 0)
        deep_speech_full = np.array(self.data_dic[video_name]['clip_data_list'][source_anchor]['deep_speech_list'])

        # # display data
        # display_source = np.concatenate(source_clip_list,1)
        # display_source_mask = np.concatenate(source_clip_mask_list,1)
        # display_reference0 = np.concatenate([reference_clip_list[0][:,:,:3],reference_clip_list[0][:,:,3:6],reference_clip_list[0][:,:,6:9],
        #                                 reference_clip_list[0][:,:,9:12],reference_clip_list[0][:,:,12:15]],1)
        # display_reference1 = np.concatenate([reference_clip_list[1][:, :, :3], reference_clip_list[1][:, :, 3:6],
        #                                 reference_clip_list[1][:, :, 6:9],
        #                                 reference_clip_list[1][:, :, 9:12], reference_clip_list[1][:, :, 12:15]],1)
        # display_reference2 = np.concatenate([reference_clip_list[2][:, :, :3], reference_clip_list[2][:, :, 3:6],
        #                                 reference_clip_list[2][:, :, 6:9],
        #                                 reference_clip_list[2][:, :, 9:12], reference_clip_list[2][:, :, 12:15]],1)
        # display_reference3 = np.concatenate([reference_clip_list[3][:, :, :3], reference_clip_list[3][:, :, 3:6],
        #                                 reference_clip_list[3][:, :, 6:9],
        #                                 reference_clip_list[3][:, :, 9:12], reference_clip_list[3][:, :, 12:15]],1)
        # display_reference4 = np.concatenate([reference_clip_list[4][:, :, :3], reference_clip_list[4][:, :, 3:6],
        #                                 reference_clip_list[4][:, :, 6:9],
        #                                 reference_clip_list[4][:, :, 9:12], reference_clip_list[4][:, :, 12:15]],1)
        # merge_img = np.concatenate([display_source,display_source_mask,
        #                             display_reference0,display_reference1,display_reference2,display_reference3,
        #                             display_reference4],0)
        # cv2.imshow('test',(merge_img[:,:,::-1] * 255).astype(np.uint8))
        # cv2.waitKey(-1)



        # # 2 tensor
        source_clip = torch.from_numpy(source_clip).float().permute(0, 3, 1, 2)
        source_clip_mask = torch.from_numpy(source_clip_mask).float().permute(0, 3, 1, 2)
        reference_clip = torch.from_numpy(reference_clip).float().permute(0, 3, 1, 2)
        deep_speech_clip = torch.from_numpy(deep_speech_clip).float().permute(0, 2, 1)
        deep_speech_full = torch.from_numpy(deep_speech_full).permute(1, 0)
        return source_clip,source_clip_mask, reference_clip,deep_speech_clip,deep_speech_full

    def __len__(self):
        return self.length


```

## Source file: inference.py

{
  "path": "inference.py",
  "blob_sha": "ea21b30553a48b7045eeca0d2590cf4068aae09c",
  "sha256": "b6a10c5c45f75fdd603749fad09538316b9391d17ce0cbf7dfda6bf5253432de",
  "lines": 173,
  "bytes": 10440,
  "selection_score": 0,
  "keyword_line_counts": {
    "mouth": 4,
    "face": 11,
    "lip": 11
  },
  "url": "https://github.com/MRzzm/DINet/blob/3b57fb0a2482213327890fbb76baeafdaa412597/inference.py",
  "function_signature_candidates": [
    {
      "line": 15,
      "text": "def extract_frames_from_video(video_path,save_dir):"
    }
  ]
}

```text

from utils.deep_speech import DeepSpeech
from utils.data_processing import load_landmark_openface,compute_crop_radius
from config.config import DINetInferenceOptions
from models.DINet import DINet

import numpy as np
import glob
import os
import cv2
import torch
import subprocess
import random
from collections import OrderedDict

def extract_frames_from_video(video_path,save_dir):
    videoCapture = cv2.VideoCapture(video_path)
    fps = videoCapture.get(cv2.CAP_PROP_FPS)
    if int(fps) != 25:
        print('warning: the input video is not 25 fps, it would be better to trans it to 25 fps!')
    frames = videoCapture.get(cv2.CAP_PROP_FRAME_COUNT)
    frame_height = videoCapture.get(cv2.CAP_PROP_FRAME_HEIGHT)
    frame_width = videoCapture.get(cv2.CAP_PROP_FRAME_WIDTH)
    for i in range(int(frames)):
        ret, frame = videoCapture.read()
        result_path = os.path.join(save_dir, str(i).zfill(6) + '.jpg')
        cv2.imwrite(result_path, frame)
    return (int(frame_width),int(frame_height))

if __name__ == '__main__':
    # load config
    opt = DINetInferenceOptions().parse_args()
    if not os.path.exists(opt.source_video_path):
        raise ('wrong video path : {}'.format(opt.source_video_path))
    ############################################## extract frames from source video ##############################################
    print('extracting frames from video: {}'.format(opt.source_video_path))
    video_frame_dir = opt.source_video_path.replace('.mp4', '')
    if not os.path.exists(video_frame_dir):
        os.mkdir(video_frame_dir)
    video_size = extract_frames_from_video(opt.source_video_path,video_frame_dir)
    ############################################## extract deep speech feature ##############################################
    print('extracting deepspeech feature from : {}'.format(opt.driving_audio_path))
    if not os.path.exists(opt.deepspeech_model_path):
        raise ('pls download pretrained model of deepspeech')
    DSModel = DeepSpeech(opt.deepspeech_model_path)
    if not os.path.exists(opt.driving_audio_path):
        raise ('wrong audio path :{}'.format(opt.driving_audio_path))
    ds_feature = DSModel.compute_audio_feature(opt.driving_audio_path)
    res_frame_length = ds_feature.shape[0]
    ds_feature_padding = np.pad(ds_feature, ((2, 2), (0, 0)), mode='edge')
    ############################################## load facial landmark ##############################################
    print('loading facial landmarks from : {}'.format(opt.source_openface_landmark_path))
    if not os.path.exists(opt.source_openface_landmark_path):
        raise ('wrong facial landmark path :{}'.format(opt.source_openface_landmark_path))
    video_landmark_data = load_landmark_openface(opt.source_openface_landmark_path).astype(np.int)
    ############################################## align frame with driving audio ##############################################
    print('aligning frames with driving audio')
    video_frame_path_list = glob.glob(os.path.join(video_frame_dir, '*.jpg'))
    if len(video_frame_path_list) != video_landmark_data.shape[0]:
        raise ('video frames are misaligned with detected landmarks')
    video_frame_path_list.sort()
    video_frame_path_list_cycle = video_frame_path_list + video_frame_path_list[::-1]
    video_landmark_data_cycle = np.concatenate([video_landmark_data, np.flip(video_landmark_data, 0)], 0)
    video_frame_path_list_cycle_length = len(video_frame_path_list_cycle)
    if video_frame_path_list_cycle_length >= res_frame_length:
        res_video_frame_path_list = video_frame_path_list_cycle[:res_frame_length]
        res_video_landmark_data = video_landmark_data_cycle[:res_frame_length, :, :]
    else:
        divisor = res_frame_length // video_frame_path_list_cycle_length
        remainder = res_frame_length % video_frame_path_list_cycle_length
        res_video_frame_path_list = video_frame_path_list_cycle * divisor + video_frame_path_list_cycle[:remainder]
        res_video_landmark_data = np.concatenate([video_landmark_data_cycle]* divisor + [video_landmark_data_cycle[:remainder, :, :]],0)
    res_video_frame_path_list_pad = [video_frame_path_list_cycle[0]] * 2 \
                                    + res_video_frame_path_list \
                                    + [video_frame_path_list_cycle[-1]] * 2
    res_video_landmark_data_pad = np.pad(res_video_landmark_data, ((2, 2), (0, 0), (0, 0)), mode='edge')
    assert ds_feature_padding.shape[0] == len(res_video_frame_path_list_pad) == res_video_landmark_data_pad.shape[0]
    pad_length = ds_feature_padding.shape[0]

    ############################################## randomly select 5 reference images ##############################################
    print('selecting five reference images')
    ref_img_list = []
    resize_w = int(opt.mouth_region_size + opt.mouth_region_size // 4)
    resize_h = int((opt.mouth_region_size // 2) * 3 + opt.mouth_region_size // 8)
    ref_index_list = random.sample(range(5, len(res_video_frame_path_list_pad) - 2), 5)
    for ref_index in ref_index_list:
        crop_flag,crop_radius = compute_crop_radius(video_size,res_video_landmark_data_pad[ref_index - 5:ref_index, :, :])
        if not crop_flag:
            raise ('our method can not handle videos with large change of facial size!!')
        crop_radius_1_4 = crop_radius // 4
        ref_img = cv2.imread(res_video_frame_path_list_pad[ref_index- 3])[:, :, ::-1]
        ref_landmark = res_video_landmark_data_pad[ref_index - 3, :, :]
        ref_img_crop = ref_img[
                  ref_landmark[29, 1] - crop_radius:ref_landmark[29, 1] + crop_radius * 2 + crop_radius_1_4,
                  ref_landmark[33, 0] - crop_radius - crop_radius_1_4:ref_landmark[33, 0] + crop_radius +crop_radius_1_4,
                  :]
        ref_img_crop = cv2.resize(ref_img_crop,(resize_w,resize_h))
        ref_img_crop = ref_img_crop / 255.0
        ref_img_list.append(ref_img_crop)
    ref_video_frame = np.concatenate(ref_img_list, 2)
    ref_img_tensor = torch.from_numpy(ref_video_frame).permute(2, 0, 1).unsqueeze(0).float().cuda()

    ############################################## load pretrained model weight ##############################################
    print('loading pretrained model from: {}'.format(opt.pretrained_clip_DINet_path))
    model = DINet(opt.source_channel, opt.ref_channel, opt.audio_channel).cuda()
    if not os.path.exists(opt.pretrained_clip_DINet_path):
        raise ('wrong path of pretrained model weight: {}'.format(opt.pretrained_clip_DINet_path))
    state_dict = torch.load(opt.pretrained_clip_DINet_path)['state_dict']['net_g']
    new_state_dict = OrderedDict()
    for k, v in state_dict.items():
        name = k[7:]  # remove module.
        new_state_dict[name] = v
    model.load_state_dict(new_state_dict)
    model.eval()
    ############################################## inference frame by frame ##############################################
    if not os.path.exists(opt.res_video_dir):
        os.mkdir(opt.res_video_dir)
    res_video_path = os.path.join(opt.res_video_dir,os.path.basename(opt.source_video_path)[:-4] + '_facial_dubbing.mp4')
    if os.path.exists(res_video_path):
        os.remove(res_video_path)
    res_face_path = res_video_path.replace('_facial_dubbing.mp4', '_synthetic_face.mp4')
    if os.path.exists(res_face_path):
        os.remove(res_face_path)
    videowriter = cv2.VideoWriter(res_video_path, cv2.VideoWriter_fourcc(*'XVID'), 25, video_size)
    videowriter_face = cv2.VideoWriter(res_face_path, cv2.VideoWriter_fourcc(*'XVID'), 25, (resize_w, resize_h))
    for clip_end_index in range(5, pad_length, 1):
        print('synthesizing {}/{} frame'.format(clip_end_index - 5, pad_length - 5))
        crop_flag, crop_radius = compute_crop_radius(video_size,res_video_landmark_data_pad[clip_end_index - 5:clip_end_index, :, :],random_scale = 1.05)
        if not crop_flag:
            raise ('our method can not handle videos with large change of facial size!!')
        crop_radius_1_4 = crop_radius // 4
        frame_data = cv2.imread(res_video_frame_path_list_pad[clip_end_index - 3])[:, :, ::-1]
        frame_landmark = res_video_landmark_data_pad[clip_end_index - 3, :, :]
        crop_frame_data = frame_data[
                            frame_landmark[29, 1] - crop_radius:frame_landmark[29, 1] + crop_radius * 2 + crop_radius_1_4,
                            frame_landmark[33, 0] - crop_radius - crop_radius_1_4:frame_landmark[33, 0] + crop_radius +crop_radius_1_4,
                            :]
        crop_frame_h,crop_frame_w = crop_frame_data.shape[0],crop_frame_data.shape[1]
        crop_frame_data = cv2.resize(crop_frame_data, (resize_w,resize_h))  # [32:224, 32:224, :]
        crop_frame_data = crop_frame_data / 255.0
        crop_frame_data[opt.mouth_region_size//2:opt.mouth_region_size//2 + opt.mouth_region_size,
                        opt.mouth_region_size//8:opt.mouth_region_size//8 + opt.mouth_region_size, :] = 0

        crop_frame_tensor = torch.from_numpy(crop_frame_data).float().cuda().permute(2, 0, 1).unsqueeze(0)
        deepspeech_tensor = torch.from_numpy(ds_feature_padding[clip_end_index - 5:clip_end_index, :]).permute(1, 0).unsqueeze(0).float().cuda()
        with torch.no_grad():
            pre_frame = model(crop_frame_tensor, ref_img_tensor, deepspeech_tensor)
            pre_frame = pre_frame.squeeze(0).permute(1, 2, 0).detach().cpu().numpy() * 255
        videowriter_face.write(pre_frame[:, :, ::-1].copy().astype(np.uint8))
        pre_frame_resize = cv2.resize(pre_frame, (crop_frame_w,crop_frame_h))
        frame_data[
        frame_landmark[29, 1] - crop_radius:
        frame_landmark[29, 1] + crop_radius * 2,
        frame_landmark[33, 0] - crop_radius - crop_radius_1_4:
        frame_landmark[33, 0] + crop_radius + crop_radius_1_4,
        :] = pre_frame_resize[:crop_radius * 3,:,:]
        videowriter.write(frame_data[:, :, ::-1])
    videowriter.release()
    videowriter_face.release()
    video_add_audio_path = res_video_path.replace('.mp4', '_add_audio.mp4')
    if os.path.exists(video_add_audio_path):
        os.remove(video_add_audio_path)
    cmd = 'ffmpeg -i {} -i {} -c:v copy -c:a aac -strict experimental -map 0:v:0 -map 1:a:0 {}'.format(
        res_video_path,
        opt.driving_audio_path,
        video_add_audio_path)
    subprocess.call(cmd, shell=True)









```

## Source file: models/DINet.py

{
  "path": "models/DINet.py",
  "blob_sha": "5124bf0560597f11da8368a83be9022f4dd15471",
  "sha256": "8502ad2d9d7394506469f029e2ad362f9f1b3fd40f8193e5329f591991cb8e0b",
  "lines": 299,
  "bytes": 11871,
  "selection_score": 0,
  "keyword_line_counts": {
    "deform": 1
  },
  "url": "https://github.com/MRzzm/DINet/blob/3b57fb0a2482213327890fbb76baeafdaa412597/models/DINet.py",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "def make_coordinate_grid_3d(spatial_size, type):"
    },
    {
      "line": 31,
      "text": "def __init__(self, in_features,out_features, kernel_size, padding):"
    },
    {
      "line": 44,
      "text": "def forward(self, x):"
    },
    {
      "line": 61,
      "text": "def __init__(self, in_features,out_features, kernel_size, padding):"
    },
    {
      "line": 74,
      "text": "def forward(self, x):"
    },
    {
      "line": 91,
      "text": "def __init__(self, in_features, out_features, kernel_size=3, padding=1):"
    },
    {
      "line": 97,
      "text": "def forward(self, x):"
    },
    {
      "line": 108,
      "text": "def __init__(self, in_features, out_features, kernel_size, padding):"
    },
    {
      "line": 114,
      "text": "def forward(self, x):"
    },
    {
      "line": 124,
      "text": "def __init__(self, in_features, out_features, kernel_size=3, padding=1, stride=2):"
    },
    {
      "line": 130,
      "text": "def forward(self, x):"
    },
    {
      "line": 140,
      "text": "def __init__(self, in_features, out_features,  kernel_size, padding):"
    },
    {
      "line": 146,
      "text": "def forward(self, x):"
    },
    {
      "line": 156,
      "text": "def __init__(self, in_features, out_features,  kernel_size=3, padding=1):"
    },
    {
      "line": 162,
      "text": "def forward(self, x):"
    },
    {
      "line": 172,
      "text": "def __init__(self,  para_ch,feature_ch):"
    },
    {
      "line": 195,
      "text": "def forward(self, feature_map,para_code):"
    },
    {
      "line": 215,
      "text": "def __init__(self, source_channel,ref_channel,audio_channel):"
    },
    {
      "line": 276,
      "text": "def forward(self, source_img,ref_img,audio_feature):"
    }
  ]
}

```text

import torch
from torch import nn
import torch.nn.functional as F
import math
import cv2
import numpy as np
from sync_batchnorm import SynchronizedBatchNorm2d as BatchNorm2d
from sync_batchnorm import SynchronizedBatchNorm1d as BatchNorm1d

def make_coordinate_grid_3d(spatial_size, type):
    '''
        generate 3D coordinate grid
    '''
    d, h, w = spatial_size
    x = torch.arange(w).type(type)
    y = torch.arange(h).type(type)
    z = torch.arange(d).type(type)
    x = (2 * (x / (w - 1)) - 1)
    y = (2 * (y / (h - 1)) - 1)
    z = (2 * (z / (d - 1)) - 1)
    yy = y.view(1,-1, 1).repeat(d,1, w)
    xx = x.view(1,1, -1).repeat(d,h, 1)
    zz = z.view(-1,1,1).repeat(1,h,w)
    meshed = torch.cat([xx.unsqueeze_(3), yy.unsqueeze_(3)], 3)
    return meshed,zz

class ResBlock1d(nn.Module):
    '''
        basic block
    '''
    def __init__(self, in_features,out_features, kernel_size, padding):
        super(ResBlock1d, self).__init__()
        self.in_features = in_features
        self.out_features = out_features
        self.conv1 = nn.Conv1d(in_channels=in_features, out_channels=in_features, kernel_size=kernel_size,
                               padding=padding)
        self.conv2 = nn.Conv1d(in_channels=in_features, out_channels=out_features, kernel_size=kernel_size,
                               padding=padding)
        if out_features != in_features:
            self.channel_conv = nn.Conv1d(in_features,out_features,1)
        self.norm1 = BatchNorm1d(in_features)
        self.norm2 = BatchNorm1d(in_features)
        self.relu = nn.ReLU()
    def forward(self, x):
        out = self.norm1(x)
        out = self.relu(out)
        out = self.conv1(out)
        out = self.norm2(out)
        out = self.relu(out)
        out = self.conv2(out)
        if self.in_features != self.out_features:
            out += self.channel_conv(x)
        else:
            out += x
        return out

class ResBlock2d(nn.Module):
    '''
            basic block
    '''
    def __init__(self, in_features,out_features, kernel_size, padding):
        super(ResBlock2d, self).__init__()
        self.in_features = in_features
        self.out_features = out_features
        self.conv1 = nn.Conv2d(in_channels=in_features, out_channels=in_features, kernel_size=kernel_size,
                               padding=padding)
        self.conv2 = nn.Conv2d(in_channels=in_features, out_channels=out_features, kernel_size=kernel_size,
                               padding=padding)
        if out_features != in_features:
            self.channel_conv = nn.Conv2d(in_features,out_features,1)
        self.norm1 = BatchNorm2d(in_features)
        self.norm2 = BatchNorm2d(in_features)
        self.relu = nn.ReLU()
    def forward(self, x):
        out = self.norm1(x)
        out = self.relu(out)
        out = self.conv1(out)
        out = self.norm2(out)
        out = self.relu(out)
        out = self.conv2(out)
        if self.in_features != self.out_features:
            out += self.channel_conv(x)
        else:
            out += x
        return out

class UpBlock2d(nn.Module):
    '''
            basic block
    '''
    def __init__(self, in_features, out_features, kernel_size=3, padding=1):
        super(UpBlock2d, self).__init__()
        self.conv = nn.Conv2d(in_channels=in_features, out_channels=out_features, kernel_size=kernel_size,
                              padding=padding)
        self.norm = BatchNorm2d(out_features)
        self.relu = nn.ReLU()
    def forward(self, x):
        out = F.interpolate(x, scale_factor=2)
        out = self.conv(out)
        out = self.norm(out)
        out = F.relu(out)
        return out

class DownBlock1d(nn.Module):
    '''
            basic block
    '''
    def __init__(self, in_features, out_features, kernel_size, padding):
        super(DownBlock1d, self).__init__()
        self.conv = nn.Conv1d(in_channels=in_features, out_channels=out_features, kernel_size=kernel_size,
                              padding=padding,stride=2)
        self.norm = BatchNorm1d(out_features)
        self.relu = nn.ReLU()
    def forward(self, x):
        out = self.conv(x)
        out = self.norm(out)
        out = self.relu(out)
        return out

class DownBlock2d(nn.Module):
    '''
            basic block
    '''
    def __init__(self, in_features, out_features, kernel_size=3, padding=1, stride=2):
        super(DownBlock2d, self).__init__()
        self.conv = nn.Conv2d(in_channels=in_features, out_channels=out_features, kernel_size=kernel_size,
                              padding=padding, stride=stride)
        self.norm = BatchNorm2d(out_features)
        self.relu = nn.ReLU()
    def forward(self, x):
        out = self.conv(x)
        out = self.norm(out)
        out = self.relu(out)
        return out

class SameBlock1d(nn.Module):
    '''
            basic block
    '''
    def __init__(self, in_features, out_features,  kernel_size, padding):
        super(SameBlock1d, self).__init__()
        self.conv = nn.Conv1d(in_channels=in_features, out_channels=out_features,
                              kernel_size=kernel_size, padding=padding)
        self.norm = BatchNorm1d(out_features)
        self.relu = nn.ReLU()
    def forward(self, x):
        out = self.conv(x)
        out = self.norm(out)
        out = self.relu(out)
        return out

class SameBlock2d(nn.Module):
    '''
            basic block
    '''
    def __init__(self, in_features, out_features,  kernel_size=3, padding=1):
        super(SameBlock2d, self).__init__()
        self.conv = nn.Conv2d(in_channels=in_features, out_channels=out_features,
                              kernel_size=kernel_size, padding=padding)
        self.norm = BatchNorm2d(out_features)
        self.relu = nn.ReLU()
    def forward(self, x):
        out = self.conv(x)
        out = self.norm(out)
        out = self.relu(out)
        return out

class AdaAT(nn.Module):
    '''
       AdaAT operator
    '''
    def __init__(self,  para_ch,feature_ch):
        super(AdaAT, self).__init__()
        self.para_ch = para_ch
        self.feature_ch = feature_ch
        self.commn_linear = nn.Sequential(
            nn.Linear(para_ch, para_ch),
            nn.ReLU()
        )
        self.scale = nn.Sequential(
                    nn.Linear(para_ch, feature_ch),
                    nn.Sigmoid()
                )
        self.rotation = nn.Sequential(
                nn.Linear(para_ch, feature_ch),
                nn.Tanh()
            )
        self.translation = nn.Sequential(
                nn.Linear(para_ch, 2 * feature_ch),
                nn.Tanh()
            )
        self.tanh = nn.Tanh()
        self.sigmoid = nn.Sigmoid()

    def forward(self, feature_map,para_code):
        batch,d, h, w = feature_map.size(0), feature_map.size(1), feature_map.size(2), feature_map.size(3)
        para_code = self.commn_linear(para_code)
        scale = self.scale(para_code).unsqueeze(-1) * 2
        angle = self.rotation(para_code).unsqueeze(-1) * 3.14159#
        rotation_matrix = torch.cat([torch.cos(angle), -torch.sin(angle), torch.sin(angle), torch.cos(angle)], -1)
        rotation_matrix = rotation_matrix.view(batch, self.feature_ch, 2, 2)
        translation = self.translation(para_code).view(batch, self.feature_ch, 2)
        grid_xy, grid_z = make_coordinate_grid_3d((d, h, w), feature_map.type())
        grid_xy = grid_xy.unsqueeze(0).repeat(batch, 1, 1, 1, 1)
        grid_z = grid_z.unsqueeze(0).repeat(batch, 1, 1, 1)
        scale = scale.unsqueeze(2).unsqueeze(3).repeat(1, 1, h, w, 1)
        rotation_matrix = rotation_matrix.unsqueeze(2).unsqueeze(3).repeat(1, 1, h, w, 1, 1)
        translation = translation.unsqueeze(2).unsqueeze(3).repeat(1, 1, h, w, 1)
        trans_grid = torch.matmul(rotation_matrix, grid_xy.unsqueeze(-1)).squeeze(-1) * scale + translation
        full_grid = torch.cat([trans_grid, grid_z.unsqueeze(-1)], -1)
        trans_feature = F.grid_sample(feature_map.unsqueeze(1), full_grid).squeeze(1)
        return trans_feature

class DINet(nn.Module):
    def __init__(self, source_channel,ref_channel,audio_channel):
        super(DINet, self).__init__()
        self.source_in_conv = nn.Sequential(
            SameBlock2d(source_channel,64,kernel_size=7, padding=3),
            DownBlock2d(64, 128, kernel_size=3, padding=1),
            DownBlock2d(128,256,kernel_size=3, padding=1)
        )
        self.ref_in_conv = nn.Sequential(
            SameBlock2d(ref_channel, 64, kernel_size=7, padding=3),
            DownBlock2d(64, 128, kernel_size=3, padding=1),
            DownBlock2d(128, 256, kernel_size=3, padding=1),
        )
        self.trans_conv = nn.Sequential(
            # 20 →10
            SameBlock2d(512, 128, kernel_size=3, padding=1),
            SameBlock2d(128, 128, kernel_size=11, padding=5),
            SameBlock2d(128, 128, kernel_size=11, padding=5),
            DownBlock2d(128, 128, kernel_size=3, padding=1),
            # 10 →5
            SameBlock2d(128, 128, kernel_size=7, padding=3),
            SameBlock2d(128, 128, kernel_size=7, padding=3),
            DownBlock2d(128, 128, kernel_size=3, padding=1),
            # 5 →3
            SameBlock2d(128, 128, kernel_size=3, padding=1),
            DownBlock2d(128, 128, kernel_size=3, padding=1),
            # 3 →2
            SameBlock2d(128, 128, kernel_size=3, padding=1),
            DownBlock2d(128, 128, kernel_size=3, padding=1),

        )
        self.audio_encoder = nn.Sequential(
            SameBlock1d(audio_channel, 128, kernel_size=5, padding=2),
            ResBlock1d(128, 128, 3, 1),
            DownBlock1d(128, 128, 3, 1),
            ResBlock1d(128, 128, 3, 1),
            DownBlock1d(128, 128, 3, 1),
            SameBlock1d(128, 128, kernel_size=3, padding=1)
        )

        appearance_conv_list = []
        for i in range(2):
            appearance_conv_list.append(
                nn.Sequential(
                    ResBlock2d(256, 256, 3, 1),
                    ResBlock2d(256, 256, 3, 1),
                    ResBlock2d(256, 256, 3, 1),
                    ResBlock2d(256, 256, 3, 1),
                )
            )
        self.appearance_conv_list = nn.ModuleList(appearance_conv_list)
        self.adaAT = AdaAT(256, 256)
        self.out_conv = nn.Sequential(
            SameBlock2d(512, 128, kernel_size=3, padding=1),
            UpBlock2d(128,128,kernel_size=3, padding=1),
            ResBlock2d(128, 128, 3, 1),
            UpBlock2d(128, 128, kernel_size=3, padding=1),
            nn.Conv2d(128, 3, kernel_size=(7, 7), padding=(3, 3)),
            nn.Sigmoid()
        )
        self.global_avg2d = nn.AdaptiveAvgPool2d(1)
        self.global_avg1d = nn.AdaptiveAvgPool1d(1)
    def forward(self, source_img,ref_img,audio_feature):
        ## source image encoder
        source_in_feature = self.source_in_conv(source_img)
        ## reference image encoder
        ref_in_feature = self.ref_in_conv(ref_img)
        ## alignment encoder
        img_para = self.trans_conv(torch.cat([source_in_feature,ref_in_feature],1))
        img_para = self.global_avg2d(img_para).squeeze(3).squeeze(2)
        ## audio encoder
        audio_para = self.audio_encoder(audio_feature)
        audio_para = self.global_avg1d(audio_para).squeeze(2)
        ## concat alignment feature and audio feature
        trans_para = torch.cat([img_para,audio_para],1)
        ## use AdaAT do spatial deformation on reference feature maps
        ref_trans_feature = self.appearance_conv_list[0](ref_in_feature)
        ref_trans_feature = self.adaAT(ref_trans_feature, trans_para)
        ref_trans_feature = self.appearance_conv_list[1](ref_trans_feature)
        ## feature decoder
        merge_feature = torch.cat([source_in_feature,ref_trans_feature],1)
        out = self.out_conv(merge_feature)
        return out





```
