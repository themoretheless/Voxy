# Facial rendering source scan: on1262/facialanimation

Repository: https://github.com/on1262/facialanimation

Pinned source revision: 0d3ee3326e715fea363e715b999f31549e48bd75

Discovery topic: facial animation

Description: Source code for: Expressive Speech-driven Facial Animation with controllable emotions

Reported license: Apache-2.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE.md", "third_party/DAN/LICENSE"]

Topic paths: {"flame": ["scripts/visualize_flame_params.py", "third_party/EMOCABasic/models/DecaFLAME.py"], "lbs": ["third_party/EMOCABasic/decautils/lbs.py"], "rig": ["models/fusion_origin/__init__.py", "models/fusion_origin/attention.py", "models/fusion_origin/config.yml", "models/fusion_origin/model.py"], "face": ["third_party/EMOCABasic/decautils/FaceDetector.py", "utils/interface.py"]}

## Source file: utils/interface.py

{
  "path": "utils/interface.py",
  "blob_sha": "b6c718b16716ca6aaeec4468e817ea93a9fd7ed8",
  "sha256": "cb5f0aaab4b93fa57bdbcf9fb27d39d785b4d414cfda9ffd3c8ae5d649033258",
  "lines": 347,
  "bytes": 15127,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 2,
    "flame": 8,
    "face": 12
  },
  "url": "https://github.com/on1262/facialanimation/blob/0d3ee3326e715fea363e715b999f31549e48bd75/utils/interface.py",
  "function_signature_candidates": [
    {
      "line": 20,
      "text": "def __init__(self, device):"
    },
    {
      "line": 27,
      "text": "def forward(self, codedict):"
    },
    {
      "line": 35,
      "text": "def get_codedict(cls):"
    },
    {
      "line": 40,
      "text": "def __init__(self, device, decoder_only=False):"
    },
    {
      "line": 54,
      "text": "def encode(self, imgs, return_img):"
    },
    {
      "line": 62,
      "text": "def move_result(self, outs, result, device, start, end):"
    },
    {
      "line": 78,
      "text": "def decode(self, codedict_list, return_set:set, target_device=None):"
    },
    {
      "line": 153,
      "text": "def _decode(self, codedict, return_set:set):"
    },
    {
      "line": 170,
      "text": "def __init__(self, device):"
    },
    {
      "line": 194,
      "text": "def set_norm(self, norm:torch.Tensor):"
    },
    {
      "line": 205,
      "text": "def inference(self, img:torch.Tensor, norm=True):"
    },
    {
      "line": 224,
      "text": "def _inference(self, img:torch.Tensor):"
    },
    {
      "line": 239,
      "text": "def __init__(self, device):"
    },
    {
      "line": 256,
      "text": "def __init__(self, device):"
    },
    {
      "line": 287,
      "text": "def forward(self, data):"
    },
    {
      "line": 306,
      "text": "def __init__(self, device):"
    },
    {
      "line": 310,
      "text": "def forward(self, data):"
    },
    {
      "line": 331,
      "text": "def __init__(self, device):"
    },
    {
      "line": 336,
      "text": "def forward(self, data):"
    }
  ]
}

```text

import json
import os
from threading import Thread

import numpy as np
import plyfile
import torch
from fitting.fit_utils import Mesh
from PIL import Image
from torchvision import transforms
from transformers import Wav2Vec2Processor

from third_party.DAN.networks.dan import DAN
from third_party.EMOCABasic import FLAME, DecaModule
from third_party.FaceFormer import faceformer
from utils.config_loader import GBL_CONF, PATH


class FLAMEModel():
    def __init__(self, device):
        print('loading FLAME model')
        self.device = device
        self.flame = FLAME().to(device)
        for p in self.flame.parameters():
            p.requires_grad = False
    
    def forward(self, codedict):
        codedict = {key:val.to(self.device) for key,val in codedict.items()}
        verts,_,_ = self.flame(shape_params=codedict['shapecode'], \
            expression_params=codedict['expcode'], \
            pose_params=codedict['posecode'], eye_pose_params=None)
        return verts
    
    @classmethod
    def get_codedict(cls):
        return {'shapecode': torch.zeros(1,100), 'expcode': torch.zeros(1,50), 'posecode': torch.zeros(1,6)}


class EMOCAModel():
    def __init__(self, device, decoder_only=False):
        self.device = device
        self.emoca = DecaModule(decoder_only)
        self.emoca.load_state_dict(torch.load(PATH['3rd']['emoca']['state_dict']), strict=(not decoder_only))
        for p in self.emoca.parameters():
            p.requires_grad = False
        self.emoca.to(device=device)
        self.emoca.eval()
        self.max_batch_size=GBL_CONF['inference']['emoca']['batchsize']

    """
    input: imgs(tensor), batch,3,224,224. format = convert_img( o_code='emoca')
    output: dict{'shapecode(100)', 'texcode','expcode(50)','posecode(6)','cam(9,3)','lightcode', 'images(optional)'}
    """
    def encode(self, imgs, return_img):
        if isinstance(imgs, torch.Tensor):
            return self.emoca.encode({'image':imgs.float().to(self.device)}, return_img) # emoca use 3hwRGB01 format
        elif isinstance(imgs, dict):
            assert('image' in imgs.keys())
            imgs['image'] = imgs['image'].float().to(self.device)
            return self.emoca.encode(imgs, return_img)

    def move_result(self, outs, result, device, start, end):
        for key in outs.keys():
            result[key][start:end,...] += outs[key].to(device)
        outs.clear()


    '''
    inputs:
        list(dict) or dict
        return_set:
            {'verts', 'geo', 'coarse', 'faces'}
    outputs: dict
        output_images_coarse: batch*3*224*224 image format=emoca
        verts: batch*5663
        geometry_coarse: grey image
    '''
    def decode(self, codedict_list, return_set:set, target_device=None):
        if target_device is None:
            target_device = self.device
        if 'geo' in return_set:
            return_set.add('verts')
        if isinstance(codedict_list, dict):
            codedict_list = [codedict_list]
        for codedict in codedict_list:
            try:
                for key in codedict.keys(): # assert seq length are the same
                    if codedict[key] is not None and key != 'decode_verts':
                        assert(codedict[key].size(0) == codedict['shapecode'].size(0))
                assert(codedict['shapecode'].size(1) == 100)
                #assert(codedict['lightcode'].size(-2) == 9 and codedict['lightcode'].size(-1) == 3)
                assert(codedict['expcode'].size(1) == 50)
                assert(codedict['posecode'].size(1) == 6)
                if 'coarse' in return_set:
                    assert(codedict['cam'].size(1) == 3)
            except AssertionError as e:
                print('EMOCA decode error, check tensor size in codedict')
                for key in codedict.keys(): # assert seq length are the same
                    if codedict[key] is not None:
                        print('key:', str(key), ' size:', codedict[key].size())

        # concat codedict
        len_list = [cd['shapecode'].size(0) for cd in codedict_list]
        for idx in range(len(len_list)):
            len_list[idx] += (len_list[idx-1] if idx > 0 else 0)
        # codedict is about 2 MB per batch(70imgs), imgs is 40 MB per batch
        codedict_keys = {'shapecode', 'expcode', 'posecode'}
        if 'coarse' in return_set:
            codedict_keys.update({'texcode', 'cam'})
        if 'geo' in return_set:
            codedict_keys.update({'cam'})
        if 'decode_verts' in return_set:
            codedict_keys.update({'decode_verts'})
        codedict = {key:torch.cat([cd[key] for cd in codedict_list], dim=0).to(self.device) for key in codedict_keys}
        
        # avoid re-split
        result = {}
        if 'coarse' in return_set:
            result['output_images_coarse'] = torch.zeros((len_list[-1],3,224,224), dtype=torch.float32, device=target_device)
        if 'verts' in return_set:
            result['verts'] = torch.zeros((len_list[-1], 5023, 3), dtype=torch.float32, device=target_device)
        if 'geo' in return_set:
            result['geometry_coarse'] = torch.zeros((len_list[-1],3,224,224), dtype=torch.float32, device=target_device)

        # check_num = result[-1,-1,-1,-1]
        batch_size = codedict['expcode'].size(0)
        if batch_size > self.max_batch_size:
            t_new = None
            now_batch = self.max_batch_size
            while now_batch < batch_size + self.max_batch_size:
                in_dict = {key:val[now_batch - self.max_batch_size:min(batch_size, now_batch),...] for key,val in codedict.items()}
                if t_new is not None:
                    t_new.join()
                    del t_new
                #print('before decode: ', torch.cuda.max_memory_allocated(self.device))
                out_dict = self._decode(in_dict, return_set)
                t_new = Thread(target=self.move_result, args= \
                    (out_dict, result, target_device, now_batch - self.max_batch_size, min(batch_size, now_batch)))
                t_new.start()
                now_batch += self.max_batch_size
            t_new.join()
        else:
            result = self._decode(codedict, return_set)
            for key in result:
                result[key] = result[key].to(target_device)
        
        if 'faces' in return_set:
            result['faces'] = self.emoca.deca.render.faces[0].detach().to(target_device) # topography is constant
        return result

            
    # 48 Bytes for 1 pixel, image(3*224*224) = 6.8 MB, 1 batch=1 GB
    def _decode(self, codedict, return_set:set):
        codedict = {key:val.to(self.device) for key,val in codedict.items()}
        values = self.emoca.decode(codedict, return_set=return_set)
        outdict = {}
        if 'coarse' in return_set:
            outdict['output_images_coarse'] = values['predicted_images'] # background rendered
        if 'verts' in return_set:
            outdict['verts'] = values['verts']
        if 'geo' in return_set:
            outdict['geometry_coarse'] = self.emoca.deca.render.render_shape(values['verts'], values['trans_verts'])
        for key in outdict.keys():
            if outdict[key] is not None:
                assert(outdict[key].size(0) == codedict['shapecode'].size(0))
        return outdict


class DANModel():
    def __init__(self, device):
        self.device = device
        self.labels = GBL_CONF['inference']['dan']['emo_label']
        if os.path.exists(PATH['3rd']['dan']['norm']):
            print('load from new_norm.pt')
            self.out_norm = torch.load(PATH['3rd']['dan']['norm'])['norm'].to(self.device)
            print('DAN label:', self.labels)
            print('DAN [mean, std]:', self.out_norm)
        else:
            self.out_norm = None
        self.data_transforms = transforms.Compose([
                                    transforms.Normalize(mean=GBL_CONF['inference']['dan']['transform_mean'],
                                    std=GBL_CONF['inference']['dan']['transform_std'])
                                ])
        self.max_batch_size = GBL_CONF['inference']['dan']['batchsize']
        self.tensor2img = transforms.ToPILImage()
        self.img2tensor = transforms.ToTensor()
        self.model = DAN(num_head=4, num_class=7, pretrained=False)
        self.model.load_state_dict(torch.load(PATH['3rd']['dan']['state_dict'], map_location=self.device)['model_state_dict'],strict=True)
        for p in self.model.parameters():
            p.requires_grad = False
        self.model.to(device=self.device)
        self.model.eval()
    
    def set_norm(self, norm:torch.Tensor):
        # :,0 = mean, :,1 = std
        assert(norm.size() == (2,7))
        self.out_norm = norm.to(self.device)

    """
    input: img: batch,3,224,224
    output:
        label: str
        out(tensor): batch,7 not norm
    """
    def inference(self, img:torch.Tensor, norm=True):
        # large batch will cause out of memory error
        batch_size = img.size(0)
        assert(img.size()[1:] == (3,224,224))

        if batch_size > self.max_batch_size:
            result = torch.empty((batch_size, 7), device=self.device)
            now_batch = self.max_batch_size
            while now_batch < batch_size + self.max_batch_size:
                in_img = img[now_batch - self.max_batch_size:min(batch_size, now_batch),...]
                result[now_batch - self.max_batch_size:min(batch_size, now_batch),...] = self._inference(in_img)
                now_batch += self.max_batch_size
        else:
            result = self._inference(img)
        if norm and self.out_norm is not None:
            self.out_norm = self.out_norm.to()
            result = (result - self.out_norm[0,:]) / self.out_norm[1,:]
        return result

    def _inference(self, img:torch.Tensor):
        # RGB
        img = self.data_transforms(img)
        img = img.to(self.device)

        out, features, out_heads = self.model(img)
        #print('input size: ', img.size()) 1,3,224,224
        _, pred = torch.max(out,1)
        ind_list = pred.long().tolist()
        label = torch.LongTensor(data=ind_list)# [batch_size]
        # label = self.labels[ind_list]
        # label is not tensor but str
        return out

class FaceFormerConfig:
    def __init__(self, device):
        conf = GBL_CONF['inference']['faceformer']
        self.model_name = conf['model_name']
        self.dataset = conf['dataset']
        self.fps = conf['fps']
        self.feature_dim = conf['feature_dim']
        self.period =  conf['period']
        self.vertice_dim = conf['vertice_dim']
        self.device = device
        self.template_path = PATH['3rd']['faceformer']['template']
        self.train_subjects = conf['train_subjects']
        self.val_subjects = conf['val_subjects']
        self.test_subjects = conf['test_subjects']
        self.subject = None # vertex template
        self.condition = conf['condition'] # speaking style

class FaceFormerModel:
    def __init__(self, device):
        # config
        config = FaceFormerConfig(device)
        self.config = config
        self.device = device
        # build model
        self.model = faceformer.Faceformer(config)
        self.model.load_state_dict(torch.load(PATH['3rd']['faceformer']['state_dict'], map_location=self.device))
        self.model = self.model.to(torch.device(device))
        self.model.eval()
        # init
        self.processor = Wav2Vec2Processor.from_pretrained(PATH['3rd']['faceformer']['wav2vec2'])
        
        
        self.template = {}
        # load ply and conver to dict
        template_dir = config.template_path
        plys = sorted(os.listdir(template_dir))
        for ply in plys:
            plydata = plyfile.PlyData.read(os.path.join(template_dir, ply))
            tmp_tensor = torch.as_tensor(
                np.asarray([plydata['vertex']['x'], plydata['vertex']['y'], plydata['vertex']['z']]).T.reshape((1, config.vertice_dim)), 
                device=self.device) # N, 3
            self.template[ply.split('.')[0]] = tmp_tensor
        self.train_subjects_list = [i for i in config.train_subjects.split(" ")]
        self.one_hot_labels = np.eye(len(self.train_subjects_list))
        iter = self.train_subjects_list.index(config.condition)
        one_hot = self.one_hot_labels[iter]
        one_hot = np.reshape(one_hot,(-1,one_hot.shape[0]))
        self.one_hot = torch.FloatTensor(one_hot).to(device=device)

    def forward(self, data):
        speech_array = data['wav'] # 1, seq_len*16000
        audio_feature = np.squeeze(self.processor(speech_array,sampling_rate=16000).input_values)
        audio_feature = np.reshape(audio_feature,(-1,audio_feature.shape[0]))
        audio_feature = torch.FloatTensor(audio_feature).to(device=self.device)
        # use ground truth when possible
        if 'name' in data.keys():
            # Warning: batchsize should be 1
            assert(len(data['name']) == 1)
            tmp = self.template[data['name'][0].split('=')[0]]
        else:
            tmp = self.template[self.config.subject]
        prediction = self.model.predict(audio_feature, tmp, self.one_hot)
        pred = prediction.permute(1,2,0).view(prediction.size(1), 5023,3)
        return pred



class VOCAModel:
    def __init__(self, device):
        self.device = device
        self.out_path = os.path.join(PATH['dataset']['vocaset']['fit_output'])

    def forward(self, data):
        # load obj
        person, idx_str = data['name'].split('=')
        m_path = os.path.join(self.out_path, person, idx_str, 'meshes')
        arrs = []
        sorted_list = sorted(os.listdir(m_path))
        # 60fps to 30 pfs
        sorted_list = sorted_list[::2]
        # print(sorted_list)
        for name in sorted_list:
            arrs.append(Mesh.read_obj(os.path.join(m_path, name)))
        arrs = np.stack(arrs, axis=0)
        return torch.as_tensor(arrs).to(self.device)

class BaselineConverter:
    '''
    @deprecated

    test mesh convertion precision. use converted ground truth as model output
    
    '''
    def __init__(self, device):
        self.device = device
        self.data_path = os.path.join(PATH['dataset']['biwi'], 'fit_output')
        self.flame = FLAMEModel(device)

    def forward(self, data):
        person, idx_str = data['name'].split('_')
        json_path = os.path.join(self.data_path, person, idx_str, 'params.json')
        with open(json_path, 'r') as f:
            params = torch.as_tensor(json.load(f)).to(self.device)
        seq_len = params.size(0)
        # print('params size: ', params.size())
        codedict = {'shapecode':params[:,:100], 'expcode':params[:,100:150], 'posecode':params[:,150:156]}
        return self.flame.forward(codedict)





```

## Source file: scripts/visualize_flame_params.py

{
  "path": "scripts/visualize_flame_params.py",
  "blob_sha": "a561bce4c8229579b4f506a78193bfd15c814954",
  "sha256": "9949e5bd201cee44adfba3f4b4caeafc0ef457519000708cce200a9092c1a388",
  "lines": 30,
  "bytes": 1287,
  "selection_score": 15,
  "keyword_line_counts": {
    "mouth": 1,
    "face": 1
  },
  "url": "https://github.com/on1262/facialanimation/blob/0d3ee3326e715fea363e715b999f31549e48bd75/scripts/visualize_flame_params.py",
  "function_signature_candidates": []
}

```text

import torch
from utils.interface import EMOCAModel
from utils.converter import save_img, convert_img
import os

if __name__ == '__main__':
    counts = 200
    out_folder = os.path.join('.', 'visualize_params')
    os.makedirs(out_folder, exist_ok=True)
    emoca = EMOCAModel(device='cuda:7', decoder_only=True)
    code_dict = {'expcode':torch.zeros(counts,50),'shapecode': torch.zeros(counts,100), 'posecode':torch.zeros(counts,6), 'cam': torch.zeros(counts,3)}
    for idx in range(counts):
        if idx >= 100:
            code_dict['shapecode'][idx,idx-100] = -3
        else:
            code_dict['shapecode'][idx, idx] = 3
        code_dict['cam'][idx,0] = 10
    imgs = emoca.decode(code_dict, {'geo'}, target_device='cpu')['geometry_coarse']
    for idx in range(counts):
        name = -(idx-counts // 2) if idx > counts // 2 else idx
        if idx == counts // 2:
            name = '-0'
        save_img(convert_img(imgs[idx,...], 'emoca', 'tvsave'), os.path.join(out_folder,f'{name}.png'))
    print('Done')
    # expression params idx related to mouth open:
    # [1, 3, 5]
    # expression params idx related to mourh horizontal movement:
    # [0, 1, 3, 6, 7]
    # only first 20 expression params is useful for expression generation



```

## Source file: models/fusion_origin/model.py

{
  "path": "models/fusion_origin/model.py",
  "blob_sha": "0342106e7581dece236ec51665e57154d066241a",
  "sha256": "9a3f77ee89945ca55b45af503a95fe60a99211b849d141501f7e417ca4d48c22",
  "lines": 392,
  "bytes": 16912,
  "selection_score": 3,
  "keyword_line_counts": {
    "rig": 1,
    "face": 1,
    "lip": 3
  },
  "url": "https://github.com/on1262/facialanimation/blob/0d3ee3326e715fea363e715b999f31549e48bd75/models/fusion_origin/model.py",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "def apply_V_mask(lstm_out: torch.Tensor):"
    },
    {
      "line": 21,
      "text": "def zero_padding(t:torch.tensor, pad_size, dim=-1, t_first=True):"
    },
    {
      "line": 42,
      "text": "def __init__(self, in_channels, out_channels,dp=0.1):"
    },
    {
      "line": 51,
      "text": "def forward(self, x, seq_len):"
    },
    {
      "line": 64,
      "text": "def __init__(self, in_channels, out_channels,dp=0.1):"
    },
    {
      "line": 72,
      "text": "def forward(self, x):"
    },
    {
      "line": 84,
      "text": "def __init__(self, emo_channels, out_hidden=768, in_hidden=768, dp=0.1):"
    },
    {
      "line": 93,
      "text": "def forward(self, emo_logits, seqs_len):"
    },
    {
      "line": 112,
      "text": "def __init__(self, dim_hidden, dim_output, num_heads, n_blocks, dp):"
    },
    {
      "line": 123,
      "text": "def forward(self, x):"
    },
    {
      "line": 131,
      "text": "def __init__(self, dim_style=128, dim_aud=128, dim_out=56):"
    },
    {
      "line": 141,
      "text": "def forward(self, wav_out, style):"
    },
    {
      "line": 149,
      "text": "def __init__(self, dim_emo_ebd=128, dim_params=56):"
    },
    {
      "line": 160,
      "text": "def forward(self, params, emo_logits):"
    },
    {
      "line": 168,
      "text": "def __init__(self, debug=0):"
    },
    {
      "line": 214,
      "text": "def batch_forward(self, in_dict): # used in training phase"
    },
    {
      "line": 219,
      "text": "def smooth(self, params):"
    },
    {
      "line": 223,
      "text": "def out_norm_apply(self, params):"
    },
    {
      "line": 226,
      "text": "def test_forward(self, in_dict):"
    },
    {
      "line": 266,
      "text": "def forward(self, in_dict):"
    },
    {
      "line": 365,
      "text": "def from_configs(cls, config):"
    },
    {
      "line": 368,
      "text": "def get_configs(self):"
    },
    {
      "line": 373,
      "text": "def get_opt_list(self):"
    },
    {
      "line": 376,
      "text": "def get_output_mask(self, seqs_len):"
    },
    {
      "line": 386,
      "text": "def set_norm(self, norm_dict, device):"
    },
    {
      "line": 391,
      "text": "def set_emoca(self, emoca):"
    }
  ]
}

```text

from utils.interface import EMOCAModel
from utils.config_loader import GBL_CONF, PATH
from transformers import Wav2Vec2Model
from os.path import join
import torch
import torch.nn as nn
import torch.optim as optim
import torch.nn.functional as F
import math
from .attention import TransformerBlock



def apply_V_mask(lstm_out: torch.Tensor):
    mask = torch.linspace(0, lstm_out.size(1), steps=lstm_out.size(1), device=lstm_out.device, requires_grad=False) / lstm_out.size(1)
    (b,_,_,p) = lstm_out.size()
    mask = mask.view(1,-1,1).expand(b,-1,p) # batch, seqs_len, 1, params
    return lstm_out[:,:,0,:] * mask + lstm_out[:,:,1,:] * (1.0 - mask)


def zero_padding(t:torch.tensor, pad_size, dim=-1, t_first=True):
    assert(t.size(dim) <= pad_size)
    dev = t.device
    if t.size(dim) == pad_size:
        return t
    if t.dim() == 1:
        if t_first:
            return torch.cat([t, torch.zeros((pad_size-t.size(0)), device=dev)])
        else:
            return torch.cat([torch.zeros((pad_size-t.size(0)), device=dev),t])
    else:
        p_size = list(t.size())
        p_size[dim] = pad_size - t.size(dim)
        p_size = torch.Size(p_size)
        if t_first:
            return torch.cat([t, torch.zeros(p_size, device=dev)], dim=dim)
        else:
            return torch.cat([torch.zeros(p_size, device=dev), t], dim=dim)


class EmoPredLayer(nn.Module):
    def __init__(self, in_channels, out_channels,dp=0.1):
        super(EmoPredLayer, self).__init__()
        self.c_emo = out_channels
        self.lstm = nn.LSTM((in_channels), (128), batch_first=True, bidirectional=True, proj_size=out_channels)
        self.dropout = nn.Dropout(dp)
        # manually norm
        self.emo_min = -2
        self.emo_max = 2

    def forward(self, x, seq_len):
        out,_ = self.lstm(self.dropout(x))
        out = out.view(out.size(0), out.size(1), 2, out.size(2) // 2)
        out = apply_V_mask(out)
        out = (out + 1) * 0.5 * (self.emo_max - self.emo_min) + self.emo_min
        # parse
        out_emo_logits = torch.zeros((seq_len.sum(), self.c_emo), device=x.device)
        for idx in range(seq_len.size(0)):
            start = 0 if idx == 0 else seq_len[:idx].sum()
            out_emo_logits[start:start+seq_len[idx],:] = out[idx, 0:seq_len[idx],:]
        return out_emo_logits # not the last output

class WavLayer(nn.Module):
    def __init__(self, in_channels, out_channels,dp=0.1):
        super(WavLayer, self).__init__()
        self.den1 = nn.Linear(in_channels*3, 256)
        self.relu = nn.ReLU()
        self.norm = nn.LayerNorm(256)
        self.den2 = nn.Linear(256, out_channels)
        self.dropout = nn.Dropout(dp)

    def forward(self, x):
        x = self.norm(self.relu(self.dropout(self.den1(torch.reshape(x, (x.size(0), -1))))))
        return self.den2(x)

class EmoEmbeddingLayer(nn.Module):
    """
    EmoEmbeddingLayer
    input:
        emo_logits: Batch, seq_len, emo_channel
    output:
        Batch, seq_len, out_hidden
    """
    def __init__(self, emo_channels, out_hidden=768, in_hidden=768, dp=0.1):
        super().__init__()
        self.c_emo = emo_channels
        self.hid = in_hidden
        self.style_embedding = nn.Parameter(torch.zeros((1,emo_channels, in_hidden)), requires_grad=True)
        torch.nn.init.xavier_normal_(self.style_embedding)
        self.dropout = nn.Dropout(dp)
        
    
    def forward(self, emo_logits, seqs_len):
        #assert(emo_logits.dim()==3)
        emo_logits = self.dropout(emo_logits)
        try:
            assert(emo_logits.size(0) == torch.sum(seqs_len))
        except Exception as e:
            print('emo_logits:', emo_logits.size(), ' seqs_len:', seqs_len)
            min_s = min(emo_logits.size(0), torch.sum(seqs_len))
            emo_logits = emo_logits[:min_s,:]
            if seqs_len.size(0) == 1:
                seqs_len[0] = min_s
        out_emo_logits = torch.zeros(seqs_len.size(0), seqs_len.max(), self.c_emo, device=emo_logits.device)
        for idx in range(seqs_len.size(0)):
            start = 0 if idx == 0 else seqs_len[:idx].sum()
            out_emo_logits[idx, 0:seqs_len[idx],:] = emo_logits[start:start+seqs_len[idx],:]
        ebd = self.style_embedding.expand(seqs_len.size(0), self.c_emo, self.hid).to(emo_logits.device)
        return torch.bmm(out_emo_logits, ebd)

class StyleExtractor(nn.Module):
    def __init__(self, dim_hidden, dim_output, num_heads, n_blocks, dp):
        super().__init__()
        self.blocks = torch.nn.ModuleList(
            [TransformerBlock(dim_hidden, num_heads, dim_hidden*4, dp, mode='self') for _ in range(n_blocks)])
        self.den = nn.Linear(dim_hidden, dim_output)
        self.mask_ebd = nn.Parameter(torch.zeros((1,1, dim_hidden)), requires_grad=False)
        self.n_blocks = n_blocks
        nn.init.xavier_uniform_(self.mask_ebd)
    
    # x: batch, seq_len, features
    # return: batch, out_features
    def forward(self, x):
        # append mask ebd
        x = torch.cat([self.mask_ebd.expand(x.size(0), -1,-1),x], dim=-2)
        for idx in range(self.n_blocks):
            x = self.blocks[idx].forward(x, None, None)
        return self.den(x[:, 0, :])

class ParamPredictor(nn.Module):
    def __init__(self, dim_style=128, dim_aud=128, dim_out=56):
        super().__init__()
        self.convs = nn.Sequential(
            nn.Conv1d(dim_style+dim_aud, 128, 1),
            nn.ReLU(),
            nn.Conv1d(128, 128, 3, padding=1, groups=8),
            nn.ReLU(),
            nn.Conv1d(128, 56, 1)
        )

    def forward(self, wav_out, style):
        wav_out = wav_out.permute(0,2,1) # batch, feature, seq_len
        wav_out = torch.cat([wav_out, style.unsqueeze(-1).expand(wav_out.size(0), -1, wav_out.size(-1))], dim=-2)
        # feature dim=style + audio

        return self.convs(wav_out).permute(0,2,1) # batch, seq_len, feature

class ParamFixer(nn.Module):
    def __init__(self, dim_emo_ebd=128, dim_params=56):
        super().__init__()
        self.norm = nn.LayerNorm(dim_params+dim_emo_ebd)
        self.convs = nn.Sequential(
            nn.Conv1d(dim_emo_ebd+dim_params, 128, 1),
            nn.ReLU(),
            nn.Conv1d(128, 128, 3, padding=1, groups=8),
            nn.ReLU(),
            nn.Conv1d(128, dim_params, 1)
        )

    def forward(self, params, emo_logits):
        # print('params:', params.size(), 'emo_logits:', emo_logits.size())
        params_in = torch.cat([params, emo_logits], dim=-1)
        params_in = self.norm(params_in).permute(0,2,1)
        return (self.convs(params_in) + params.permute(0,2,1)).permute(0,2,1)

    
class Model(nn.Module):
    def __init__(self, debug=0):
        super(Model, self).__init__()
        self.model_name = 'fusion_origin'
        self.config = GBL_CONF['model'][self.model_name]
        self.out_norm = None
        self.wav_fea_channels = self.config['wav_fea_channels']
        self.params_channels = self.config['params_channels']
        self.hidden = self.config['hidden']
        self.dim_style = self.config['dim_style']
        self.emo_channels = self.config['emo_channels']
        self.dp = self.config['dp']
        self.debug = debug
        self.wav_len_per_seq=16000/30.0
        self._norm_check = False

        try:
            wav2vec_path = PATH['3rd']['wav2vec2']['pretrained']
            model = Wav2Vec2Model.from_pretrained(pretrained_model_name_or_path=None, \
                config=join(wav2vec_path,'config.json'), state_dict=torch.load(join(wav2vec_path,'pytorch_model.bin')))
        except Exception as e:
            print(e)
            print('ignore wav2vec2 warning')
        self.fea_extractor = model.feature_extractor
        for p in self.fea_extractor.parameters(recurse=True): # freeze wav2vec2
            p.requires_grad = False
        self.wav_layer = WavLayer(self.wav_fea_channels, self.hidden, dp=self.dp)
        self.pred_layer = EmoPredLayer(self.hidden, self.emo_channels, dp=self.dp)
        self.style_layer = StyleExtractor(self.hidden, self.dim_style, 1, 4, dp=self.dp)
        self.param_predictor = ParamPredictor(dim_style=self.dim_style, dim_aud=self.hidden, dim_out=self.params_channels)
        self.param_fixer = ParamFixer(dim_emo_ebd=self.hidden, dim_params=56)

        #for p in self.pred_layer.parameters(recurse=True):
        #    p.requires_grad = False
        self.emo_ebd_layer = EmoEmbeddingLayer(self.emo_channels, out_hidden=self.hidden, in_hidden=self.hidden, dp=self.dp)
        # opts
        self.opt_gen = optim.Adam(list(self.wav_layer.parameters(recurse=True))
            + list(self.param_predictor.parameters(recurse=True))
            + list(self.emo_ebd_layer.parameters(recurse=True))
            + list(self.param_fixer.parameters(recurse=True))
            + list(self.pred_layer.parameters(recurse=True))
        )
        self.opt_style = optim.Adam(
            list(self.style_layer.parameters(recurse=True))
        )
        

    def batch_forward(self, in_dict): # used in training phase
        # generate emotion tensor for no-video dataset
        assert('emo_logits_conf' in in_dict.keys())
        return self.forward(in_dict)
    
    def smooth(self, params):
        return torch.nn.functional.conv1d(
            params.permute(0,2,1), torch.ones((56,1,3), device=params.device)/3, padding=1, groups=56).permute(0,2,1)

    def out_norm_apply(self, params):
        return self.out_norm['min'] + (params+1) *0.5 * (self.out_norm['max'] - self.out_norm['min'])

    def test_forward(self, in_dict):
        assert(in_dict['wav'].size(0) == 1) # batch_size should be 1
        assert('emo_logits_conf' in in_dict.keys())
        '''
        emo tensor conf:
        no_use: output without fixer
        use: use input emo-logits, use predicted emo-logits if input is None
        one-hot: use predicted emo-logits with one-hot enhancement
        '''

        if 'code_dict' in in_dict.keys() and in_dict['code_dict'] is not None:
            if not isinstance(in_dict['code_dict'], list):
                in_dict['code_dict'] = [in_dict['code_dict']]
        else:
            in_dict['code_dict'] = None

        if in_dict['emo_logits_conf'] == ['one_hot']:
            if in_dict.get('emo_logits') is None:
                in_dict['pred_only'] = True
                in_dict['emo_logits'] = self.forward(in_dict)
                in_dict['pred_only'] = False
            assert(in_dict.get('emo_label') is not None)
            # TODO
            dan_labels = GBL_CONF['inference']['dan']['emo_label']
            label_dict = {label:idx for idx, label in enumerate(dan_labels)}
            emo_index = label_dict[in_dict['emo_label']]
            print('emo label:', in_dict['emo_label'])
            in_dict['emo_logits'] -= in_dict['emo_logits'].mean(dim=0) # normalize
            if in_dict.get('intensity') is None:
                intensity = GBL_CONF['inference']['inference_mode']['default_intensity'] # 0 to 1
            else:
                intensity = in_dict['intensity']
            in_dict['emo_logits'][:, :] -= intensity
            in_dict['emo_logits'][:, emo_index] += (4*intensity)
            in_dict['emo_logits_conf'] = ['use']
        
        in_dict['smooth'] = False if 'smooth' not in in_dict.keys() or in_dict['smooth'] != True else True
        out =  self.forward(in_dict)
        return out
    
    def forward(self, in_dict):
        wavs = in_dict.get('wav')
        seqs_len = in_dict.get('seqs_len')
        emo_logits = in_dict.get('emo_logits')
        code_dict = in_dict.get('code_dict')
        pred_only = in_dict.get('pred_only')
        smooth = in_dict.get('smooth')
        emo_logits_conf = in_dict.get('emo_logits_conf')

        assert(wavs is not None and seqs_len is not None)
        dev = wavs.device # regard as input device
        output = {}
        # input check
        assert(wavs.dim() == 2)
        if emo_logits is not None:
            emo_logits = emo_logits.to(wavs.device)
        seq_length = torch.max(seqs_len).item()

        # generate mask
        output['mask'] = self.get_output_mask(seqs_len).to(dev)

        clip_len = round(self.wav_len_per_seq)
        if round(seq_length*self.wav_len_per_seq) >= wavs.size(1):
            wavs = zero_padding(wavs, round(seq_length*self.wav_len_per_seq), dim=1)
        wavs = zero_padding(wavs, round(self.wav_len_per_seq)+wavs.size(1), dim=1, t_first=False)

        wavs_input_list = []
        assert((seq_length+1)*clip_len < wavs.size(1))
        for tick in range(seq_length):
            wavs_input_list.append(wavs[:, tick*clip_len:(tick+2)*clip_len].clone())
        wavs_input = torch.stack(wavs_input_list, dim=1)
        
        wav_out = []
        for tick in range(seq_length):
            wav_fea = self.fea_extractor(wavs_input[:,tick,:]).transpose(-1,-2) # batch_size, fea_len, 512
            wav_out.append(self.wav_layer(wav_fea).unsqueeze(1)) # batch, 1, out_fea
        wav_out = torch.cat(wav_out, dim=1) # batch_size, video_seq_len, out_fea
        pred = self.pred_layer(wav_out.detach(), seqs_len)
        if pred_only:
            return pred
        else:
            output['pred_emo_logits'] = pred
        
        if emo_logits_conf == ['use'] and emo_logits is None:
            emo_logits = pred
            in_dict['emo_logits'] = pred
        
        style = self.style_layer(wav_out)
        param_out = self.param_predictor(wav_out, style)

        if emo_logits is not None: # not all sample use emo_logits
            emo_ebd = self.emo_ebd_layer(emo_logits, seqs_len).expand(wav_out.size(0),-1,-1)
            param_emo = self.param_fixer(param_out, emo_ebd)
            for idx in range(len(emo_logits_conf)):
                if emo_logits_conf[idx] == 'no_use': # TODO
                    param_emo[idx,...] = param_out[idx,...]
        else:
            param_emo = None

        # smooth, only enable in test phase
        if smooth:
            param_out = self.smooth(param_out)
            if param_emo is not None:
                param_emo = self.smooth(param_emo)
            

        if self.out_norm is not None:
            if self._norm_check == False or self.out_norm['min'].device != dev:
                self.out_norm['min'] = self.out_norm['min'].to(dev)
                self.out_norm['max'] = self.out_norm['max'].to(dev)
                self._norm_check = True
            param_out = self.out_norm_apply(param_out)
            if param_emo is not None:
                param_emo = self.out_norm_apply(param_emo)

        if param_emo is not None:
            output['params'] = param_emo * output['mask']
        else:
            output['params'] = param_out * output['mask']

        output['params_ori'] = param_out * output['mask']
        
        if code_dict is not None:
            # replace code_dict
            for idx in range(len(code_dict)):
                try: 
                    assert(code_dict[idx]['expcode'].size(0) == seqs_len[idx])
                except AssertionError as e:
                    print('codedict error', code_dict[idx]['expcode'].size(0), seqs_len[idx], seqs_len)
                # code_dict[idx]['expcode'] = lstm_out[idx, :seqs_len[idx], :50]
                code_dict[idx]['expcode'] = output['params'][idx, :seqs_len[idx], :50]
                code_dict[idx]['posecode'][:,3] = output['params'][idx, :seqs_len[idx], 53]
                # code_dict[idx]['posecode'][:,[4,5]] = 0 # fixed dim

            output['code_dict'] = code_dict
            
        return output

    @classmethod
    def from_configs(cls, config):
        return cls(*tuple(config))

    def get_configs(self):
        # update out norm
        self.config[-2] = self.out_norm
        return self.config

    def get_opt_list(self):
        return [self.opt_gen, self.opt_style]

    def get_output_mask(self, seqs_len):
        max_seqs_len = max(seqs_len)
        # mask for seqs_len = tensor([3,2,5])
        # [[T,T,T,F,F],
        #  [T,T,F,F,F],
        # [T,T,T,T,T]]
        mask = torch.arange(0, max_seqs_len, 1).long().unsqueeze(0).expand(seqs_len.size(0), max_seqs_len) < seqs_len.unsqueeze(0).transpose(0,1)
        mask = mask.unsqueeze(dim=-1).expand(mask.size(0), mask.size(1), 56)
        return mask # no device

    def set_norm(self, norm_dict, device):
        self.out_norm = norm_dict
        self.out_norm['min'] = self.out_norm['min'].to(device)
        self.out_norm['max'] = self.out_norm['max'].to(device)

    def set_emoca(self, emoca):
        pass


```

## Source file: models/fusion_origin/__init__.py

{
  "path": "models/fusion_origin/__init__.py",
  "blob_sha": "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
  "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "lines": 0,
  "bytes": 0,
  "selection_score": 3,
  "keyword_line_counts": {},
  "url": "https://github.com/on1262/facialanimation/blob/0d3ee3326e715fea363e715b999f31549e48bd75/models/fusion_origin/__init__.py",
  "function_signature_candidates": []
}

```text



```
