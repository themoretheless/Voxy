# Facial rendering source scan: sicxu/Deep3DFaceRecon_pytorch

Repository: https://github.com/sicxu/Deep3DFaceRecon_pytorch

Pinned source revision: 9167c7136aebef0c54d3ac74f4b0396222a491c1

Discovery topic: face reconstruction 3D

Description: Accurate 3D Face Reconstruction with Weakly-Supervised Learning: From Single Image to Image Set (CVPRW 2019). A PyTorch implementation.

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"skin": ["util/skin_mask.py"], "face": ["BFM/facemodel_info.mat", "models/facerecon_model.py", "util/test_mean_face.txt"]}

## Source file: util/skin_mask.py

{
  "path": "util/skin_mask.py",
  "blob_sha": "a8a74e4c3b40d13b0258b83a12f56321a85bb179",
  "sha256": "dd71ab2895a67fa3b562a616da0c4830867880f71e3434518f2138920cd5755b",
  "lines": 125,
  "bytes": 5333,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 29,
    "face": 1
  },
  "url": "https://github.com/sicxu/Deep3DFaceRecon_pytorch/blob/9167c7136aebef0c54d3ac74f4b0396222a491c1/util/skin_mask.py",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "def __init__(self, dim, num, w, mu, cov, cov_det, cov_inv):"
    },
    {
      "line": 23,
      "text": "def likelihood(self, data):"
    },
    {
      "line": 42,
      "text": "def _rgb2ycbcr(rgb):"
    },
    {
      "line": 54,
      "text": "def _bgr2ycbcr(bgr):"
    },
    {
      "line": 90,
      "text": "def skinmask(imbgr):"
    },
    {
      "line": 111,
      "text": "def get_skin_mask(img_path):"
    }
  ]
}

```text

"""This script is to generate skin attention mask for Deep3DFaceRecon_pytorch
"""

import math
import numpy as np
import os
import cv2

class GMM:
    def __init__(self, dim, num, w, mu, cov, cov_det, cov_inv):
        self.dim = dim # feature dimension
        self.num = num # number of Gaussian components
        self.w = w # weights of Gaussian components (a list of scalars)
        self.mu= mu # mean of Gaussian components (a list of 1xdim vectors)
        self.cov = cov # covariance matrix of Gaussian components (a list of dimxdim matrices)
        self.cov_det = cov_det # pre-computed determinet of covariance matrices (a list of scalars)
        self.cov_inv = cov_inv # pre-computed inverse covariance matrices (a list of dimxdim matrices)

        self.factor = [0]*num
        for i in range(self.num):
            self.factor[i] = (2*math.pi)**(self.dim/2) * self.cov_det[i]**0.5
        
    def likelihood(self, data):
        assert(data.shape[1] == self.dim)
        N = data.shape[0]
        lh = np.zeros(N)

        for i in range(self.num):
            data_ = data - self.mu[i]

            tmp = np.matmul(data_,self.cov_inv[i]) * data_
            tmp = np.sum(tmp,axis=1)
            power = -0.5 * tmp

            p = np.array([math.exp(power[j]) for j in range(N)])
            p = p/self.factor[i]
            lh += p*self.w[i]
        
        return lh


def _rgb2ycbcr(rgb):
    m = np.array([[65.481, 128.553, 24.966],
                  [-37.797, -74.203, 112],
                  [112, -93.786, -18.214]])
    shape = rgb.shape
    rgb = rgb.reshape((shape[0] * shape[1], 3))
    ycbcr = np.dot(rgb, m.transpose() / 255.)
    ycbcr[:, 0] += 16.
    ycbcr[:, 1:] += 128.
    return ycbcr.reshape(shape)


def _bgr2ycbcr(bgr):
    rgb = bgr[..., ::-1]
    return _rgb2ycbcr(rgb)


gmm_skin_w = [0.24063933, 0.16365987, 0.26034665, 0.33535415]
gmm_skin_mu = [np.array([113.71862, 103.39613, 164.08226]),
                np.array([150.19858, 105.18467, 155.51428]),
                np.array([183.92976, 107.62468, 152.71820]),
                np.array([114.90524, 113.59782, 151.38217])]
gmm_skin_cov_det = [5692842.5, 5851930.5, 2329131., 1585971.]
gmm_skin_cov_inv = [np.array([[0.0019472069, 0.0020450759, -0.00060243998],[0.0020450759, 0.017700525, 0.0051420014],[-0.00060243998, 0.0051420014, 0.0081308950]]),
                    np.array([[0.0027110141, 0.0011036990, 0.0023122299],[0.0011036990, 0.010707724, 0.010742856],[0.0023122299, 0.010742856, 0.017481629]]),
                    np.array([[0.0048026871, 0.00022935172, 0.0077668377],[0.00022935172, 0.011729696, 0.0081661865],[0.0077668377, 0.0081661865, 0.025374353]]),
                    np.array([[0.0011989699, 0.0022453172, -0.0010748957],[0.0022453172, 0.047758564, 0.020332102],[-0.0010748957, 0.020332102, 0.024502251]])]

gmm_skin = GMM(3, 4, gmm_skin_w, gmm_skin_mu, [], gmm_skin_cov_det, gmm_skin_cov_inv)

gmm_nonskin_w = [0.12791070, 0.31130761, 0.34245777, 0.21832393]
gmm_nonskin_mu = [np.array([99.200851, 112.07533, 140.20602]),
                    np.array([110.91392, 125.52969, 130.19237]),
                    np.array([129.75864, 129.96107, 126.96808]),
                    np.array([112.29587, 128.85121, 129.05431])]
gmm_nonskin_cov_det = [458703648., 6466488., 90611376., 133097.63]
gmm_nonskin_cov_inv = [np.array([[0.00085371657, 0.00071197288, 0.00023958916],[0.00071197288, 0.0025935620, 0.00076557708],[0.00023958916, 0.00076557708, 0.0015042332]]),
                    np.array([[0.00024650150, 0.00045542428, 0.00015019422],[0.00045542428, 0.026412144, 0.018419769],[0.00015019422, 0.018419769, 0.037497383]]),
                    np.array([[0.00037054974, 0.00038146760, 0.00040408765],[0.00038146760, 0.0085505722, 0.0079136286],[0.00040408765, 0.0079136286, 0.010982352]]),
                    np.array([[0.00013709733, 0.00051228428, 0.00012777430],[0.00051228428, 0.28237113, 0.10528370],[0.00012777430, 0.10528370, 0.23468947]])]

gmm_nonskin = GMM(3, 4, gmm_nonskin_w, gmm_nonskin_mu, [], gmm_nonskin_cov_det, gmm_nonskin_cov_inv)

prior_skin = 0.8
prior_nonskin = 1 - prior_skin


# calculate skin attention mask
def skinmask(imbgr):
    im = _bgr2ycbcr(imbgr)

    data = im.reshape((-1,3))

    lh_skin = gmm_skin.likelihood(data)
    lh_nonskin = gmm_nonskin.likelihood(data)

    tmp1 = prior_skin * lh_skin
    tmp2 = prior_nonskin * lh_nonskin
    post_skin = tmp1 / (tmp1+tmp2) # posterior probability

    post_skin = post_skin.reshape((im.shape[0],im.shape[1]))

    post_skin = np.round(post_skin*255)
    post_skin = post_skin.astype(np.uint8)
    post_skin = np.tile(np.expand_dims(post_skin,2),[1,1,3]) # reshape to H*W*3

    return post_skin


def get_skin_mask(img_path):
    print('generating skin masks......')
    names = [i for i in sorted(os.listdir(
        img_path)) if 'jpg' in i or 'png' in i or 'jpeg' in i or 'PNG' in i]
    save_path = os.path.join(img_path, 'mask')
    if not os.path.isdir(save_path):
        os.makedirs(save_path)
    
    for i in range(0, len(names)):
        name = names[i]
        print('%05d' % (i), ' ', name)
        full_image_name = os.path.join(img_path, name)
        img = cv2.imread(full_image_name).astype(np.float32)
        skin_img = skinmask(img)
        cv2.imwrite(os.path.join(save_path, name), skin_img.astype(np.uint8))


```

## Source file: models/facerecon_model.py

{
  "path": "models/facerecon_model.py",
  "blob_sha": "dfaaea92ac42affc8ec09caf179b9f6b52a7e360",
  "sha256": "4e1a2b6a8591a8f58e6bcb2edb9938bc2876f8e2ae68f16e7be5672fbb459da9",
  "lines": 228,
  "bytes": 11084,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 1,
    "gum": 27,
    "face": 24,
    "lip": 1
  },
  "url": "https://github.com/sicxu/Deep3DFaceRecon_pytorch/blob/9167c7136aebef0c54d3ac74f4b0396222a491c1/models/facerecon_model.py",
  "function_signature_candidates": [
    {
      "line": 20,
      "text": "def modify_commandline_options(parser, is_train=True):"
    },
    {
      "line": 74,
      "text": "def __init__(self, opt):"
    },
    {
      "line": 122,
      "text": "def set_input(self, input):"
    },
    {
      "line": 134,
      "text": "def forward(self):"
    },
    {
      "line": 145,
      "text": "def compute_losses(self):"
    },
    {
      "line": 177,
      "text": "def optimize_parameters(self, isTrain=True):"
    },
    {
      "line": 186,
      "text": "def compute_visuals(self):"
    },
    {
      "line": 208,
      "text": "def save_mesh(self, name):"
    },
    {
      "line": 219,
      "text": "def save_coeff(self,name):"
    }
  ]
}

```text

"""This script defines the face reconstruction model for Deep3DFaceRecon_pytorch
"""

import numpy as np
import torch
from .base_model import BaseModel
from . import networks
from .bfm import ParametricFaceModel
from .losses import perceptual_loss, photo_loss, reg_loss, reflectance_loss, landmark_loss
from util import util 
from util.nvdiffrast import MeshRenderer
from util.preprocess import estimate_norm_torch

import trimesh
from scipy.io import savemat

class FaceReconModel(BaseModel):

    @staticmethod
    def modify_commandline_options(parser, is_train=True):
        """  Configures options specific for CUT model
        """
        # net structure and parameters
        parser.add_argument('--net_recon', type=str, default='resnet50', choices=['resnet18', 'resnet34', 'resnet50'], help='network structure')
        parser.add_argument('--init_path', type=str, default='checkpoints/init_model/resnet50-0676ba61.pth')
        parser.add_argument('--use_last_fc', type=util.str2bool, nargs='?', const=True, default=False, help='zero initialize the last fc')
        parser.add_argument('--bfm_folder', type=str, default='BFM')
        parser.add_argument('--bfm_model', type=str, default='BFM_model_front.mat', help='bfm model')

        # renderer parameters
        parser.add_argument('--focal', type=float, default=1015.)
        parser.add_argument('--center', type=float, default=112.)
        parser.add_argument('--camera_d', type=float, default=10.)
        parser.add_argument('--z_near', type=float, default=5.)
        parser.add_argument('--z_far', type=float, default=15.)
        parser.add_argument('--use_opengl', type=util.str2bool, nargs='?', const=True, default=True, help='use opengl context or not')

        if is_train:
            # training parameters
            parser.add_argument('--net_recog', type=str, default='r50', choices=['r18', 'r43', 'r50'], help='face recog network structure')
            parser.add_argument('--net_recog_path', type=str, default='checkpoints/recog_model/ms1mv3_arcface_r50_fp16/backbone.pth')
            parser.add_argument('--use_crop_face', type=util.str2bool, nargs='?', const=True, default=False, help='use crop mask for photo loss')
            parser.add_argument('--use_predef_M', type=util.str2bool, nargs='?', const=True, default=False, help='use predefined M for predicted face')

            
            # augmentation parameters
            parser.add_argument('--shift_pixs', type=float, default=10., help='shift pixels')
            parser.add_argument('--scale_delta', type=float, default=0.1, help='delta scale factor')
            parser.add_argument('--rot_angle', type=float, default=10., help='rot angles, degree')

            # loss weights
            parser.add_argument('--w_feat', type=float, default=0.2, help='weight for feat loss')
            parser.add_argument('--w_color', type=float, default=1.92, help='weight for loss loss')
            parser.add_argument('--w_reg', type=float, default=3.0e-4, help='weight for reg loss')
            parser.add_argument('--w_id', type=float, default=1.0, help='weight for id_reg loss')
            parser.add_argument('--w_exp', type=float, default=0.8, help='weight for exp_reg loss')
            parser.add_argument('--w_tex', type=float, default=1.7e-2, help='weight for tex_reg loss')
            parser.add_argument('--w_gamma', type=float, default=10.0, help='weight for gamma loss')
            parser.add_argument('--w_lm', type=float, default=1.6e-3, help='weight for lm loss')
            parser.add_argument('--w_reflc', type=float, default=5.0, help='weight for reflc loss')



        opt, _ = parser.parse_known_args()
        parser.set_defaults(
                focal=1015., center=112., camera_d=10., use_last_fc=False, z_near=5., z_far=15.
            )
        if is_train:
            parser.set_defaults(
                use_crop_face=True, use_predef_M=False
            )
        return parser

    def __init__(self, opt):
        """Initialize this model class.

        Parameters:
            opt -- training/test options

        A few things can be done here.
        - (required) call the initialization function of BaseModel
        - define loss function, visualization images, model names, and optimizers
        """
        BaseModel.__init__(self, opt)  # call the initialization method of BaseModel
        
        self.visual_names = ['output_vis']
        self.model_names = ['net_recon']
        self.parallel_names = self.model_names + ['renderer']

        self.net_recon = networks.define_net_recon(
            net_recon=opt.net_recon, use_last_fc=opt.use_last_fc, init_path=opt.init_path
        )

        self.facemodel = ParametricFaceModel(
            bfm_folder=opt.bfm_folder, camera_distance=opt.camera_d, focal=opt.focal, center=opt.center,
            is_train=self.isTrain, default_name=opt.bfm_model
        )
        
        fov = 2 * np.arctan(opt.center / opt.focal) * 180 / np.pi
        self.renderer = MeshRenderer(
            rasterize_fov=fov, znear=opt.z_near, zfar=opt.z_far, rasterize_size=int(2 * opt.center), use_opengl=opt.use_opengl
        )

        if self.isTrain:
            self.loss_names = ['all', 'feat', 'color', 'lm', 'reg', 'gamma', 'reflc']

            self.net_recog = networks.define_net_recog(
                net_recog=opt.net_recog, pretrained_path=opt.net_recog_path
                )
            # loss func name: (compute_%s_loss) % loss_name
            self.compute_feat_loss = perceptual_loss
            self.comupte_color_loss = photo_loss
            self.compute_lm_loss = landmark_loss
            self.compute_reg_loss = reg_loss
            self.compute_reflc_loss = reflectance_loss

            self.optimizer = torch.optim.Adam(self.net_recon.parameters(), lr=opt.lr)
            self.optimizers = [self.optimizer]
            self.parallel_names += ['net_recog']
        # Our program will automatically call <model.setup> to define schedulers, load networks, and print networks

    def set_input(self, input):
        """Unpack input data from the dataloader and perform necessary pre-processing steps.

        Parameters:
            input: a dictionary that contains the data itself and its metadata information.
        """
        self.input_img = input['imgs'].to(self.device) 
        self.atten_mask = input['msks'].to(self.device) if 'msks' in input else None
        self.gt_lm = input['lms'].to(self.device)  if 'lms' in input else None
        self.trans_m = input['M'].to(self.device) if 'M' in input else None
        self.image_paths = input['im_paths'] if 'im_paths' in input else None

    def forward(self):
        output_coeff = self.net_recon(self.input_img)
        self.facemodel.to(self.device)
        self.pred_vertex, self.pred_tex, self.pred_color, self.pred_lm = \
            self.facemodel.compute_for_render(output_coeff)
        self.pred_mask, _, self.pred_face = self.renderer(
            self.pred_vertex, self.facemodel.face_buf, feat=self.pred_color)
        
        self.pred_coeffs_dict = self.facemodel.split_coeff(output_coeff)


    def compute_losses(self):
        """Calculate losses, gradients, and update network weights; called in every training iteration"""

        assert self.net_recog.training == False
        trans_m = self.trans_m
        if not self.opt.use_predef_M:
            trans_m = estimate_norm_torch(self.pred_lm, self.input_img.shape[-2])

        pred_feat = self.net_recog(self.pred_face, trans_m)
        gt_feat = self.net_recog(self.input_img, self.trans_m)
        self.loss_feat = self.opt.w_feat * self.compute_feat_loss(pred_feat, gt_feat)

        face_mask = self.pred_mask
        if self.opt.use_crop_face:
            face_mask, _, _ = self.renderer(self.pred_vertex, self.facemodel.front_face_buf)
        
        face_mask = face_mask.detach()
        self.loss_color = self.opt.w_color * self.comupte_color_loss(
            self.pred_face, self.input_img, self.atten_mask * face_mask)
        
        loss_reg, loss_gamma = self.compute_reg_loss(self.pred_coeffs_dict, self.opt)
        self.loss_reg = self.opt.w_reg * loss_reg
        self.loss_gamma = self.opt.w_gamma * loss_gamma

        self.loss_lm = self.opt.w_lm * self.compute_lm_loss(self.pred_lm, self.gt_lm)

        self.loss_reflc = self.opt.w_reflc * self.compute_reflc_loss(self.pred_tex, self.facemodel.skin_mask)

        self.loss_all = self.loss_feat + self.loss_color + self.loss_reg + self.loss_gamma \
                        + self.loss_lm + self.loss_reflc
            

    def optimize_parameters(self, isTrain=True):
        self.forward()               
        self.compute_losses()
        """Update network weights; it will be called in every training iteration."""
        if isTrain:
            self.optimizer.zero_grad()  
            self.loss_all.backward()         
            self.optimizer.step()        

    def compute_visuals(self):
        with torch.no_grad():
            input_img_numpy = 255. * self.input_img.detach().cpu().permute(0, 2, 3, 1).numpy()
            output_vis = self.pred_face * self.pred_mask + (1 - self.pred_mask) * self.input_img
            output_vis_numpy_raw = 255. * output_vis.detach().cpu().permute(0, 2, 3, 1).numpy()
            
            if self.gt_lm is not None:
                gt_lm_numpy = self.gt_lm.cpu().numpy()
                pred_lm_numpy = self.pred_lm.detach().cpu().numpy()
                output_vis_numpy = util.draw_landmarks(output_vis_numpy_raw, gt_lm_numpy, 'b')
                output_vis_numpy = util.draw_landmarks(output_vis_numpy, pred_lm_numpy, 'r')
            
                output_vis_numpy = np.concatenate((input_img_numpy, 
                                    output_vis_numpy_raw, output_vis_numpy), axis=-2)
            else:
                output_vis_numpy = np.concatenate((input_img_numpy, 
                                    output_vis_numpy_raw), axis=-2)

            self.output_vis = torch.tensor(
                    output_vis_numpy / 255., dtype=torch.float32
                ).permute(0, 3, 1, 2).to(self.device)

    def save_mesh(self, name):

        recon_shape = self.pred_vertex  # get reconstructed shape
        recon_shape[..., -1] = 10 - recon_shape[..., -1] # from camera space to world space
        recon_shape = recon_shape.cpu().numpy()[0]
        recon_color = self.pred_color
        recon_color = recon_color.cpu().numpy()[0]
        tri = self.facemodel.face_buf.cpu().numpy()
        mesh = trimesh.Trimesh(vertices=recon_shape, faces=tri, vertex_colors=np.clip(255. * recon_color, 0, 255).astype(np.uint8), process=False)
        mesh.export(name)

    def save_coeff(self,name):

        pred_coeffs = {key:self.pred_coeffs_dict[key].cpu().numpy() for key in self.pred_coeffs_dict}
        pred_lm = self.pred_lm.cpu().numpy()
        pred_lm = np.stack([pred_lm[:,:,0],self.input_img.shape[2]-1-pred_lm[:,:,1]],axis=2) # transfer to image coordinate
        pred_coeffs['lm68'] = pred_lm
        savemat(name,pred_coeffs)





```

## Source file: train.py

{
  "path": "train.py",
  "blob_sha": "26e856f46691b55884120c66876156442d302bf0",
  "sha256": "8dd4a8e20e598d300a7e7552f1c78beef81e3b65356dc9d7dae7297b502c1ec4",
  "lines": 166,
  "bytes": 7602,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 1
  },
  "url": "https://github.com/sicxu/Deep3DFaceRecon_pytorch/blob/9167c7136aebef0c54d3ac74f4b0396222a491c1/train.py",
  "function_signature_candidates": [
    {
      "line": 17,
      "text": "def setup(rank, world_size, port):"
    },
    {
      "line": 24,
      "text": "def cleanup():"
    },
    {
      "line": 27,
      "text": "def main(rank, world_size, train_opt):"
    }
  ]
}

```text

"""This script is the training script for Deep3DFaceRecon_pytorch
"""

import os
import time
import numpy as np
import torch
from options.train_options import TrainOptions
from data import create_dataset
from models import create_model
from util.visualizer import MyVisualizer
from util.util import genvalconf
import torch.multiprocessing as mp
import torch.distributed as dist


def setup(rank, world_size, port):
    os.environ['MASTER_ADDR'] = 'localhost'
    os.environ['MASTER_PORT'] = port

    # initialize the process group
    dist.init_process_group("gloo", rank=rank, world_size=world_size)

def cleanup():
    dist.destroy_process_group()

def main(rank, world_size, train_opt):
    val_opt = genvalconf(train_opt, isTrain=False)
    
    device = torch.device(rank)
    torch.cuda.set_device(device)
    use_ddp = train_opt.use_ddp
    
    if use_ddp:
        setup(rank, world_size, train_opt.ddp_port)

    train_dataset, val_dataset = create_dataset(train_opt, rank=rank), create_dataset(val_opt, rank=rank)
    train_dataset_batches, val_dataset_batches = \
        len(train_dataset) // train_opt.batch_size, len(val_dataset) // val_opt.batch_size
    
    model = create_model(train_opt)   # create a model given train_opt.model and other options
    model.setup(train_opt)
    model.device = device
    model.parallelize()

    if rank == 0:
        print('The batch number of training images = %d\n, \
            the batch number of validation images = %d'% (train_dataset_batches, val_dataset_batches))
        model.print_networks(train_opt.verbose)
        visualizer = MyVisualizer(train_opt)   # create a visualizer that display/save images and plots

    total_iters = train_dataset_batches * (train_opt.epoch_count - 1)   # the total number of training iterations
    t_data = 0
    t_val = 0
    optimize_time = 0.1
    batch_size = 1 if train_opt.display_per_batch else train_opt.batch_size

    if use_ddp:
        dist.barrier()

    times = []
    for epoch in range(train_opt.epoch_count, train_opt.n_epochs + 1):    # outer loop for different epochs; we save the model by <epoch_count>, <epoch_count>+<save_latest_freq>
        epoch_start_time = time.time()  # timer for entire epoch
        iter_data_time = time.time()    # timer for train_data loading per iteration
        epoch_iter = 0                  # the number of training iterations in current epoch, reset to 0 every epoch

        train_dataset.set_epoch(epoch)
        for i, train_data in enumerate(train_dataset):  # inner loop within one epoch
            iter_start_time = time.time()  # timer for computation per iteration
            if total_iters % train_opt.print_freq == 0:
                t_data = iter_start_time - iter_data_time
            total_iters += batch_size
            epoch_iter += batch_size

            torch.cuda.synchronize()
            optimize_start_time = time.time()

            model.set_input(train_data)  # unpack train_data from dataset and apply preprocessing
            model.optimize_parameters()   # calculate loss functions, get gradients, update network weights

            torch.cuda.synchronize()
            optimize_time = (time.time() - optimize_start_time) / batch_size * 0.005 + 0.995 * optimize_time

            if use_ddp:
                dist.barrier()

            if rank == 0 and (total_iters == batch_size or total_iters % train_opt.display_freq == 0):   # display images on visdom and save images to a HTML file
                model.compute_visuals()
                visualizer.display_current_results(model.get_current_visuals(), total_iters, epoch,
                    save_results=True,
                    add_image=train_opt.add_image)
                    # (total_iters == batch_size or total_iters % train_opt.evaluation_freq == 0)
            
            if rank == 0 and (total_iters == batch_size or total_iters % train_opt.print_freq == 0):    # print training losses and save logging information to the disk
                losses = model.get_current_losses()
                visualizer.print_current_losses(epoch, epoch_iter, losses, optimize_time, t_data)
                visualizer.plot_current_losses(total_iters, losses)

            if total_iters == batch_size or total_iters % train_opt.evaluation_freq == 0:
                with torch.no_grad():
                    torch.cuda.synchronize()
                    val_start_time = time.time()
                    losses_avg = {}
                    model.eval()
                    for j, val_data in enumerate(val_dataset):
                        model.set_input(val_data)
                        model.optimize_parameters(isTrain=False)
                        if rank == 0 and j < train_opt.vis_batch_nums:
                            model.compute_visuals()
                            visualizer.display_current_results(model.get_current_visuals(), total_iters, epoch,
                                    dataset='val', save_results=True, count=j * val_opt.batch_size,
                                    add_image=train_opt.add_image)

                        if j < train_opt.eval_batch_nums:
                            losses = model.get_current_losses()
                            for key, value in losses.items():
                                losses_avg[key] = losses_avg.get(key, 0) + value

                    for key, value in losses_avg.items():
                        losses_avg[key] = value / min(train_opt.eval_batch_nums, val_dataset_batches)

                    torch.cuda.synchronize()
                    eval_time = time.time() - val_start_time
                    
                    if rank == 0:
                        visualizer.print_current_losses(epoch, epoch_iter, losses_avg, eval_time, t_data, dataset='val') # visualize training results
                        visualizer.plot_current_losses(total_iters, losses_avg, dataset='val')
                model.train()      

            if use_ddp:
                dist.barrier()

            if rank == 0 and (total_iters == batch_size or total_iters % train_opt.save_latest_freq == 0):   # cache our latest model every <save_latest_freq> iterations
                print('saving the latest model (epoch %d, total_iters %d)' % (epoch, total_iters))
                print(train_opt.name)  # it's useful to occasionally show the experiment name on console
                save_suffix = 'iter_%d' % total_iters if train_opt.save_by_iter else 'latest'
                model.save_networks(save_suffix)
            
            if use_ddp:
                dist.barrier()
            
            iter_data_time = time.time()

        print('End of epoch %d / %d \t Time Taken: %d sec' % (epoch, train_opt.n_epochs, time.time() - epoch_start_time))
        model.update_learning_rate()                     # update learning rates at the end of every epoch.
        
        if rank == 0 and epoch % train_opt.save_epoch_freq == 0:              # cache our model every <save_epoch_freq> epochs
            print('saving the model at the end of epoch %d, iters %d' % (epoch, total_iters))
            model.save_networks('latest')
            model.save_networks(epoch)

        if use_ddp:
            dist.barrier()

if __name__ == '__main__':

    import warnings
    warnings.filterwarnings("ignore")
    
    train_opt = TrainOptions().parse()   # get training options
    world_size = train_opt.world_size               

    if train_opt.use_ddp:
        mp.spawn(main, args=(world_size, train_opt), nprocs=world_size, join=True)
    else:
        main(0, world_size, train_opt)


```

## Source file: util/html.py

{
  "path": "util/html.py",
  "blob_sha": "cc3262a1eafda34842e4dbad47bb6ba72f0c5a68",
  "sha256": "67fc18779468246bfc955f0081c6f5e2f05b3d0be2c4079d28e7f191bce62851",
  "lines": 86,
  "bytes": 3223,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/sicxu/Deep3DFaceRecon_pytorch/blob/9167c7136aebef0c54d3ac74f4b0396222a491c1/util/html.py",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "def __init__(self, web_dir, title, refresh=0):"
    },
    {
      "line": 35,
      "text": "def get_image_dir(self):"
    },
    {
      "line": 39,
      "text": "def add_header(self, text):"
    },
    {
      "line": 48,
      "text": "def add_images(self, ims, txts, links, width=400):"
    },
    {
      "line": 68,
      "text": "def save(self):"
    }
  ]
}

```text

import dominate
from dominate.tags import meta, h3, table, tr, td, p, a, img, br
import os


class HTML:
    """This HTML class allows us to save images and write texts into a single HTML file.

     It consists of functions such as <add_header> (add a text header to the HTML file),
     <add_images> (add a row of images to the HTML file), and <save> (save the HTML to the disk).
     It is based on Python library 'dominate', a Python library for creating and manipulating HTML documents using a DOM API.
    """

    def __init__(self, web_dir, title, refresh=0):
        """Initialize the HTML classes

        Parameters:
            web_dir (str) -- a directory that stores the webpage. HTML file will be created at <web_dir>/index.html; images will be saved at <web_dir/images/
            title (str)   -- the webpage name
            refresh (int) -- how often the website refresh itself; if 0; no refreshing
        """
        self.title = title
        self.web_dir = web_dir
        self.img_dir = os.path.join(self.web_dir, 'images')
        if not os.path.exists(self.web_dir):
            os.makedirs(self.web_dir)
        if not os.path.exists(self.img_dir):
            os.makedirs(self.img_dir)

        self.doc = dominate.document(title=title)
        if refresh > 0:
            with self.doc.head:
                meta(http_equiv="refresh", content=str(refresh))

    def get_image_dir(self):
        """Return the directory that stores images"""
        return self.img_dir

    def add_header(self, text):
        """Insert a header to the HTML file

        Parameters:
            text (str) -- the header text
        """
        with self.doc:
            h3(text)

    def add_images(self, ims, txts, links, width=400):
        """add images to the HTML file

        Parameters:
            ims (str list)   -- a list of image paths
            txts (str list)  -- a list of image names shown on the website
            links (str list) --  a list of hyperref links; when you click an image, it will redirect you to a new page
        """
        self.t = table(border=1, style="table-layout: fixed;")  # Insert a table
        self.doc.add(self.t)
        with self.t:
            with tr():
                for im, txt, link in zip(ims, txts, links):
                    with td(style="word-wrap: break-word;", halign="center", valign="top"):
                        with p():
                            with a(href=os.path.join('images', link)):
                                img(style="width:%dpx" % width, src=os.path.join('images', im))
                            br()
                            p(txt)

    def save(self):
        """save the current content to the HMTL file"""
        html_file = '%s/index.html' % self.web_dir
        f = open(html_file, 'wt')
        f.write(self.doc.render())
        f.close()


if __name__ == '__main__':  # we show an example usage here.
    html = HTML('web/', 'test_html')
    html.add_header('hello world')

    ims, txts, links = [], [], []
    for n in range(4):
        ims.append('image_%d.png' % n)
        txts.append('text_%d' % n)
        links.append('image_%d.png' % n)
    html.add_images(ims, txts, links)
    html.save()


```
