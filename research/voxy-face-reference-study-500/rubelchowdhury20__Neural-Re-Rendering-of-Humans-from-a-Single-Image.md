# Facial rendering source scan: rubelchowdhury20/Neural-Re-Rendering-of-Humans-from-a-Single-Image

Repository: https://github.com/rubelchowdhury20/Neural-Re-Rendering-of-Humans-from-a-Single-Image

Pinned source revision: d00701458c75ddcab2d3d3030c87e8dc9e04307c

Discovery topic: human rendering

Description: Pytorch implementation of the paper, Neural re-rendering of humans from a single image.

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"face": ["docs/normal_faces.pickle", "notebooks/normal_faces.pickle"], "lip": ["notebooks/lip dense pickle to npy.ipynb", "notebooks/lip images dense to atlas texture.ipynb", "notebooks/lip listing pairs.ipynb", "notebooks/lip_pairs.pkl"]}

## Source file: main.py

{
  "path": "main.py",
  "blob_sha": "5e4a05c1a36b2528b0d58db355777e6be8f798fc",
  "sha256": "79ecc3566fa211edeb6585c7a22b4f4ab0e78230694e77097c6e502d126c9abf",
  "lines": 112,
  "bytes": 6352,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 49,
    "lip": 1
  },
  "url": "https://github.com/rubelchowdhury20/Neural-Re-Rendering-of-Humans-from-a-Single-Image/blob/d00701458c75ddcab2d3d3030c87e8dc9e04307c/main.py",
  "function_signature_candidates": [
    {
      "line": 16,
      "text": "def main(args):"
    }
  ]
}

```text

# Standard library imports
import os
import glob
import argparse

# Third party library imports
import torch
import torch.nn as nn
from torchvision import models

# Local imports
import config
import evaluate
import train

def main(args):
	# updating all the global variables based on the input arguments

	# updating batch size
	if(args.batch_size):
		config.PARAMS["batch_size"] = args.batch_size


	# taking care of gpu ids
	str_ids = args.gpu_ids.split(',')
	args.gpu_ids = []
	for str_id in str_ids:
		id = int(str_id)
		if id >= 0:
			args.gpu_ids.append(id)


	# updating command line arguments to the ARGS variable
	config.args = args

	# calling required functions based on the input arguments
	if args.is_train:
		train.train(config)
	else:
		evaluate.evaluate(config)


if __name__ == '__main__':
	parser = argparse.ArgumentParser()

	parser.add_argument('--is_train', action='store_false', help='continue training: load the latest model')

	# project parameters
	parser.add_argument('--name', type=str, default='render', help='name of the experiment. It decides where to store samples and models')        
	parser.add_argument('--checkpoints_dir', type=str, default='./checkpoints', help='models are saved here')


	# network parameters
	parser.add_argument("--batch_size", type=int, default=1, help="the batch_size for training as well as for inference")
	parser.add_argument('--lr', type=float, default=0.0002, help='initial learning rate for adam')
	parser.add_argument('--beta1', type=float, default=0.5, help='momentum term of adam')
	parser.add_argument('--norm', type=str, default='instance', help='instance normalization or batch normalization')
	parser.add_argument('--gpu_ids', type=str, default='0', help='gpu ids: e.g. 0  0,1,2, 0,2. use -1 for CPU')
	parser.add_argument('--niter', type=int, default=100, help='# of iter at starting learning rate')
	parser.add_argument('--niter_decay', type=int, default=100, help='# of iter to linearly decay learning rate to zero')

	# parser.add_argument("--data_directory", type=str, default="/media/rainier/rubel/projects/virtual-try-on/dataset/", help="path to the directory having images for training.")
	# parser.add_argument("--data_directory", type=str, default="/media/tensor/EXTDRIVE/projects/virtual-try-on/dataset/zalando_final/", help="path to the directory having images for training.")	
	parser.add_argument("--data_directory", type=str, default="/media/tensor/EXTDRIVE/projects/virtual-try-on/dataset/lip_mpv_dataset/", help="path to the directory having images for training.")
	parser.add_argument('--continue_train', action='store_true', help='continue training: load the latest model')
	parser.add_argument('--load_pretrain', type=str, default='', help='load the pretrained model from the specified location')
	parser.add_argument('--which_epoch', type=str, default='latest', help='which epoch to load? set to latest to use latest cached model')


	# for displays
	parser.add_argument('--display_freq', type=int, default=1, help='frequency of showing training results on screen')
	parser.add_argument('--print_freq', type=int, default=50, help='frequency of showing training results on console')
	parser.add_argument('--save_latest_freq', type=int, default=100, help='frequency of saving the latest results')
	parser.add_argument('--save_epoch_freq', type=int, default=2, help='frequency of saving checkpoints at the end of epochs')
	parser.add_argument('--no_html', action='store_true', help='do not save intermediate training results to [opt.checkpoints_dir]/[opt.name]/web/')
	parser.add_argument('--display_winsize', type=int, default=512,  help='display window size')
	parser.add_argument('--tf_log', action='store_true', help='if specified, use tensorboard logging. Requires tensorflow installed')


	# for feature net
	parser.add_argument('--feature_depth', type=int, default=2, help="# of downsampling blocks in the feature net architecture")
	parser.add_argument('--feature_output_nc', type=int, default=16, help="# of channels of feature net output")


	# for generator
	parser.add_argument('--netG_input_nc', type=int, default=22, help="# of input channels to the generator")
	parser.add_argument('--ngf', type=int, default=64, help='# of gen filters in first conv layer')
	parser.add_argument('--netG', type=str, default='global', help='selects model to use for netG')
	parser.add_argument('--n_downsample_global', type=int, default=2, help='number of downsampling layers in netG') 
	parser.add_argument('--n_blocks_global', type=int, default=3, help='number of residual blocks in the global generator network')
	parser.add_argument('--n_blocks_local', type=int, default=3, help='number of residual blocks in the local enhancer network')
	parser.add_argument('--n_local_enhancers', type=int, default=1, help='number of local enhancers to use')        
	parser.add_argument('--niter_fix_global', type=int, default=0, help='number of epochs that we only train the outmost local enhancer')


	# for discriminators        
	parser.add_argument('--num_D', type=int, default=1, help='number of discriminators to use')
	parser.add_argument('--n_layers_D', type=int, default=3, help='only used if which_model_netD==n_layers')
	parser.add_argument('--ndf', type=int, default=64, help='# of discrim filters in first conv layer')    
	# parser.add_argument('--lambda_feat', type=float, default=10.0, help='weight for feature matching loss')                
	parser.add_argument('--no_ganFeat_loss', action='store_true', help='if specified, do *not* use discriminator feature matching loss')
	parser.add_argument('--no_vgg_loss', action='store_true', help='if specified, do *not* use VGG feature matching loss')        
	parser.add_argument('--no_lsgan', action='store_true', help='do *not* use least square GAN, if false, use vanilla GAN')
	parser.add_argument('--pool_size', type=int, default=0, help='the size of image buffer that stores previously generated images')


	# loss arguments
	parser.add_argument('--lambda_tex', type=float, default=1.0, help='lambda value for feature/texture loss in total loss')
	parser.add_argument('--lambda_adv', type=float, default=1.0, help='lambda value for adversarial loss in total loss')
	parser.add_argument('--lambda_vgg', type=float, default=5.0, help='lambda value for vgg loss in total loss')    

	main(parser.parse_args())

```

## Source file: train.py

{
  "path": "train.py",
  "blob_sha": "7624feb91300835f8bcb88c8c9835285e3c5a64e",
  "sha256": "2e255c64084b7b7ddb44f6e04effd4082989339c8c2d6da8cc17ab7ded19396d",
  "lines": 188,
  "bytes": 6847,
  "selection_score": 0,
  "keyword_line_counts": {
    "lip": 1
  },
  "url": "https://github.com/rubelchowdhury20/Neural-Re-Rendering-of-Humans-from-a-Single-Image/blob/d00701458c75ddcab2d3d3030c87e8dc9e04307c/train.py",
  "function_signature_candidates": [
    {
      "line": 26,
      "text": "def train(config):"
    }
  ]
}

```text

# standard imports
import os
import math
import time
import glob
import pickle
import random

# third party imports
import torch
from torch.utils.data import DataLoader

import numpy as np
from collections import OrderedDict

# local imports
import config
from modules.data_loader import NeuralDataset
from modules.models import create_model
from modules.utils.util import AverageMeter
from modules.utils.util import tensor2im
from modules.utils.visualizer import Visualizer



def train(config):
	# important variables
	data_path = config.args.data_directory

	# steps for preparing and splitting the data for training
	with open(data_path + "lip_pairs.pkl", "rb") as fp:   #Pickling
		image_list = pickle.load(fp)
	# image_list = [i.split("/")[-1] for i in glob.glob(data_path + "model_images/*")]

	# splitting the images to train and validation set
	random.shuffle(image_list)
	train_image_list = image_list[:math.floor(len(image_list) * 0.8)]
	val_image_list = image_list[math.ceil(len(image_list) * 0.8):]

	# epoch options
	iter_path = os.path.join(config.args.checkpoints_dir, config.args.name, 'iter.txt')
	# epoch_iter => the number of iterations in the current epoch
	# start_epoch => this is the epoch from which the training will start
	if config.args.continue_train:
		try:
			start_epoch, epoch_iter = np.loadtxt(iter_path , delimiter=',', dtype=int)
		except:
			start_epoch, epoch_iter = 1, 0
		print('Resuming from epoch %d at iteration %d' % (start_epoch, epoch_iter))        
	else:    
		start_epoch, epoch_iter = 1, 0



	# loading dataloader
	train_dataset = NeuralDataset(train_image_list, data_path, config.data_transforms["train"])
	train_loader = DataLoader(train_dataset, **config.PARAMS)
	dataset_size = len(train_loader)

	# initializng the model
	model = create_model.CreateModel(config)
	if config.args.is_train and len(config.args.gpu_ids):
		model = torch.nn.DataParallel(model, device_ids=config.args.gpu_ids)

	optimizer_feature, optimizer_G, optimizer_D = model.module.optimizer_feature, model.module.optimizer_G, model.module.optimizer_D


	# initializing the visualization option
	visualizer = Visualizer(config.args)

	# saving options
	total_steps = (start_epoch-1) * dataset_size + epoch_iter	# total number of iterations/steps happened till now, from the start of the training
	save_delta = total_steps % config.args.save_latest_freq		
	display_delta = total_steps % config.args.display_freq
	print_delta = total_steps % config.args.print_freq

	feature_loss_meter = AverageMeter()
	loss_D_meter = AverageMeter()
	loss_G_GAN_meter = AverageMeter()
	loss_G_VGG_meter = AverageMeter()
	loss_G_meter = AverageMeter()


	for epoch in range(start_epoch, config.args.niter + config.args.niter_decay + 1):
		epoch_start_time = time.time()
		if epoch != start_epoch:
			epoch_iter = epoch_iter % dataset_size	# reinitializing the current epoch iterations after every epoch
		for idx, batch in enumerate(train_loader):

			total_steps += config.args.batch_size
			epoch_iter += config.args.batch_size
			
			feature_loss, loss_D, loss_G_GAN, loss_G_VGG, rendered_image, src_img, tgt_img, src_feat_rendered_on_tgt, tgt_feat_rendered_on_tgt = model(batch)

			feature_loss = torch.mean(feature_loss)
			loss_D = torch.mean(loss_D)
			loss_G_GAN = torch.mean(loss_G_GAN)
			loss_G_VGG = torch.mean(loss_G_VGG)

			 # calculate final loss scalar
			loss_G = config.args.lambda_tex * feature_loss + config.args.lambda_adv * loss_G_GAN + config.args.lambda_vgg * loss_G_VGG
			# loss_G = config.args.lambda_tex * feature_loss + config.args.lambda_adv * loss_G_GAN

			feature_loss_meter.update(feature_loss.item(), config.args.batch_size)
			loss_D_meter.update(loss_D.item(), config.args.batch_size)
			loss_G_GAN_meter.update(loss_G_GAN.item(), config.args.batch_size)
			loss_G_VGG_meter.update(loss_G_VGG.item(), config.args.batch_size)
			loss_G_meter.update(loss_G.item(), config.args.batch_size)

			############### Backward Pass ####################
			optimizer_G.zero_grad()
			optimizer_feature.zero_grad()
			optimizer_D.zero_grad()
			
			# update generator weights
			loss_G.backward(retain_graph=True)          
			optimizer_G.step()
			
			# update feature-net weights
			optimizer_feature.step()
			
			# update discriminator weights
			loss_D.backward        
			optimizer_D.step()

			### print out errors
			if total_steps % config.args.print_freq == print_delta:
				print("Train Progress--\t"
					"Train Epoch: {} [{}/{}]\t"
					"generator Loss:{:.4f} ({:.4f})\t"
					"Discriminator Loss:{:.4f} ({:.4f})\t"

					
					"feature Loss:{:.4f} ({:.4f})\t"
					"Adversarial Loss:{:.4f} ({:.4f})\t"
					"Vgg Loss:{:.4f} ({:.4f})".format(epoch, epoch_iter, dataset_size,
																loss_G_meter.val, loss_G_meter.avg,
																loss_D_meter.val, loss_D_meter.avg,
																feature_loss_meter.val, feature_loss_meter.avg,
																loss_G_GAN_meter.val, loss_G_GAN_meter.avg,
																loss_G_VGG_meter.val, loss_G_VGG_meter.avg))
				print("\n")

			if total_steps % config.args.display_freq == display_delta:
				fake_image = rendered_image[0].cpu().detach()
				src_on_tgt = src_feat_rendered_on_tgt[0,:3,:,:].cpu().detach()
				tgt_on_tgt = tgt_feat_rendered_on_tgt[0,:3,:,:].cpu().detach()

				src_on_tgt_masked = src_feat_rendered_on_tgt[0,-3:,:,:].cpu().detach()
				tgt_on_tgt_masked = tgt_feat_rendered_on_tgt[0,-3:,:,:].cpu().detach()

				visuals = OrderedDict([("source_image", tensor2im(src_img[0])),
										("target_image", tensor2im(tgt_img[0])),
										("rendered_image", tensor2im(fake_image)),
										("src on tgt", tensor2im(src_on_tgt)),
										("tgt on tgt", tensor2im(tgt_on_tgt)),
										("src on tgt masked", tensor2im(src_on_tgt_masked)),
										("tgt on tgt masked", tensor2im(tgt_on_tgt_masked)),
										("source texture", tensor2im(batch[2][0]))])

				visualizer.display_current_results(visuals, epoch, total_steps)

			### save latest model
			if total_steps % config.args.save_latest_freq == save_delta:
				print('saving the latest model (epoch %d, total_steps %d)' % (epoch, total_steps))
				model.module.render_net.save('latest')
				model.module.save_feature_net("latest")            
				np.savetxt(iter_path, (epoch, epoch_iter), delimiter=',', fmt='%d')


		### save model for this epoch
		if epoch % config.args.save_epoch_freq == 0:
			print('saving the model at the end of epoch %d, iters %d' % (epoch, total_steps))        
			
			model.module.render_net.save('latest')
			model.module.save_feature_net("latest")            

			model.module.render_net.save(epoch)
			model.module.save_feature_net(epoch)

			np.savetxt(iter_path, (epoch+1, 0), delimiter=',', fmt='%d')

		### linearly decay learning rate after certain iterations
		if epoch > config.args.niter:
			model.module.update_learning_rate()





```

## Source file: config.py

{
  "path": "config.py",
  "blob_sha": "43fbd61b8965378a2a84830767549b97d6708185",
  "sha256": "daaa01b372ee60c4a9192b5cd7199dc6fce35bae278b86e96dbb238f660909b2",
  "lines": 47,
  "bytes": 932,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 1
  },
  "url": "https://github.com/rubelchowdhury20/Neural-Re-Rendering-of-Humans-from-a-Single-Image/blob/d00701458c75ddcab2d3d3030c87e8dc9e04307c/config.py",
  "function_signature_candidates": []
}

```text

 
# Third party imports
import torch
from torchvision import transforms

# Device configuration
DEVICE = torch.device('cuda' if torch.cuda.is_available() else 'cpu')


# Data preprocessing details
data_transforms = {
	'train': transforms.Compose([
		transforms.Resize((256, 256)),
		transforms.ToTensor(),
		transforms.Normalize([0.485, 0.456, 0.406], [0.229, 0.224, 0.225])
	]),
	'val': transforms.Compose([
		transforms.Resize((256, 256)),
		transforms.ToTensor(),
		transforms.Normalize([0.485, 0.456, 0.406], [0.229, 0.224, 0.225])
	]),
}

texture_transform = transforms.Compose([
		# transforms.Resize((300, 450)),
		transforms.ToTensor(),
		transforms.Normalize([0.485, 0.456, 0.406], [0.229, 0.224, 0.225])
	])


dense_transform = transforms.Compose([
		transforms.ToTensor()
	])

# params info
PARAMS = {'batch_size': 1,
			'shuffle': True,
			'num_workers': 16}


# network parameters




# command line arguments
args = {}

```

## Source file: evaluate.py

{
  "path": "evaluate.py",
  "blob_sha": "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
  "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "lines": 0,
  "bytes": 0,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/rubelchowdhury20/Neural-Re-Rendering-of-Humans-from-a-Single-Image/blob/d00701458c75ddcab2d3d3030c87e8dc9e04307c/evaluate.py",
  "function_signature_candidates": []
}

```text



```
