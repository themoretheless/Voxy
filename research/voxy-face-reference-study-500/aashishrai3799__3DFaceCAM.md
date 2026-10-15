# Facial rendering source scan: aashishrai3799/3DFaceCAM

Repository: https://github.com/aashishrai3799/3DFaceCAM

Pinned source revision: 7c908737ffde34844535e3d74e39e4c9f59b4d43

Discovery topic: face model 3D

Description: Implementation of a 3D Face Generative Model

Reported license: NOASSERTION

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"face": ["3dfacecam.gif", "data/face_v.npy", "generate_faces.py"]}

## Source file: generate_faces.py

{
  "path": "generate_faces.py",
  "blob_sha": "a108ecb0b4604fdb17d33d2196364d871c7cf79d",
  "sha256": "76313cd855574bda2a5de48c5bc40a6aaf6597a2119bf87b4e9867156846d32f",
  "lines": 105,
  "bytes": 3119,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 1,
    "jaw": 1,
    "mouth": 2,
    "gum": 7,
    "rig": 1,
    "face": 1,
    "lip": 1
  },
  "url": "https://github.com/aashishrai3799/3DFaceCAM/blob/7c908737ffde34844535e3d74e39e4c9f59b4d43/generate_faces.py",
  "function_signature_candidates": [
    {
      "line": 15,
      "text": "def generate_shapes(num_imgs=1, exp_list=[1,2]):"
    },
    {
      "line": 36,
      "text": "def generate_tex(num_imgs=1, exp_list=[1,2]):"
    }
  ]
}

```text

import yaml
from train_gan3d import shape_GAN
import os
from test_gan3d import Test
import torch

from test_texture import test_texture
from texture_model.progan_modules import Generator, Discriminator
from texture_model.assets.texture_processing import add_tex
from data.edit_obj import get_mtl
import argparse



def generate_shapes(num_imgs=1, exp_list=[1,2]):

	with open("config.yml","r") as cfgfile:
		  cfg = yaml.safe_load(cfgfile)

	gan = shape_GAN(cfg, device, args)

	path_d = args.path_decoder
	path_gid = args.path_gid
	path_gexp = args.path_gexp

	with torch.no_grad():

		  test = Test(gan, args)
		  test.load_models(path_d=path_d, path_gid=path_gid, path_gexp=path_gexp)

		  for i in range(num_imgs):
		      test.set_z_id(torch.randn(1,20).to(device))
		      test.generate(str(i), intensities=True, save_obj=True, render=False, exp_list=exp_list)


def generate_tex(num_imgs=1, exp_list=[1,2]):

	input_code_size = 128
	channel = 256

	g_running = Generator(in_channel=channel, input_code_dim=input_code_size+20+20, pixel_norm=False, tanh=False)
	g_running = torch.nn.DataParallel(g_running)


	g_running = g_running.to(device)

	model_dir = 'checkpoints/texture_models/'
	number = '142000'

	g_running.load_state_dict(torch.load(model_dir + 'checkpoint/' + number + '_g.model'), strict=False)
	g_running = torch.nn.DataParallel(g_running)
	g_running.train(False)

	test_texture(g_running, num_imgs=num_imgs, exp_list=exp_list, input_code_size=input_code_size, device=device, alpha=1, out_path='results/', zid_dict_path='data/zid_dictionary.pkl')



if __name__ == '__main__':


	### Expressions list for reference

	# ['0_neutral.jpg', '1_smile.jpg', '2_mouth_stretch.jpg', '3_anger.jpg', '4_jaw_left.jpg', '5_jaw_right.jpg', '6_jaw_forward.jpg', '7_mouth_left.jpg', '8_mouth_right.jpg', '9_dimpler.jpg', '10_chin_raiser.jpg', '11_lip_puckerer.jpg', '12_lip_funneler.jpg', '13_sadness.jpg', '14_lip_roll.jpg', '15_grin.jpg', '16_cheek_blowing.jpg', '17_eye_closed.jpg', '18_brow_raiser.jpg', '19_brow_lower.jpg']



	parser = argparse.ArgumentParser()

	parser.add_argument('--results', type=str, default='results/')
	parser.add_argument('--path_decoder', type=str, default='checkpoints/ae/Decoder/2000')
	parser.add_argument('--path_gid', type=str, default='checkpoints/gan3d/Generator_Checkpoint_id/8.0')
	parser.add_argument('--path_gexp', type=str, default='checkpoints/gan3d/Generator_Checkpoint_exp/8.0')
	parser.add_argument('--checkpoints_path', type=str, default='checkpoints/gan3d/')
	parser.add_argument('--zid_dict', type=str, default='data/zid_dictionary.pkl')

	args = parser.parse_args()



	device = torch.device("cuda")
	
	num_imgs = 1
	exp_list = [1,2]    #[Smile, Mouth Stretch]

	print('Generating', num_imgs, 'Faces...')
	
	### GENERATE SHAPES ###
	
	generate_shapes(num_imgs, exp_list)
	
	### GENERATE TEXTURES ###
	
	generate_tex(num_imgs, exp_list)

	add_tex.add_texture_template(in_path='results/', base_path = 'texture_model/assets/texture_processing/base_tex.npy', out_resolution=1024)
		  
	### GENERATE MTL ###	
	
	get_mtl(in_path='results/')  
	
	
		  
    
    
    


```

## Source file: train_ae.py

{
  "path": "train_ae.py",
  "blob_sha": "7d454dad99b8eb2762000931d973d621c398828f",
  "sha256": "c5406acb6b45bc4c3e820c9596a98bb2e428587e83d35f43e93947e2dc557eaa",
  "lines": 60,
  "bytes": 2444,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 1,
    "gum": 9
  },
  "url": "https://github.com/aashishrai3799/3DFaceCAM/blob/7c908737ffde34844535e3d74e39e4c9f59b4d43/train_ae.py",
  "function_signature_candidates": []
}

```text

import argparse, os, sys
import numpy as np
import torch
from shape_model.ae.models.AE import AE

parser = argparse.ArgumentParser(
        description='Main function to call training for different AutoEncoders')
parser.add_argument('--batch-size', type=int, default=128, metavar='N',
                    help='input batch size for training (default: 128)')
parser.add_argument('--epochs', type=int, default=2500, metavar='N',
                    help='number of epochs to train (default: 10)')
parser.add_argument('--model', type=str, default='AE', metavar='N',
                    help='Which architecture to use')
parser.add_argument('--log-interval', type=int, default=10, metavar='N',
                    help='how many batches to wait before logging training status')
parser.add_argument('--dataset', type=str, default='./data/displace_data.npy', metavar='N',
                    help='path to *.npy dataset file')
parser.add_argument('--path_enc_id', type=str, default='./checkpoints/ae/Encoder_id', metavar='N',
                    help='path to id encoder checkpoints')
parser.add_argument('--path_enc_exp', type=str, default='./checkpoints/ae/Encoder_exp', metavar='N',
                    help='path to exp encoder checkpoints')
parser.add_argument('--path_dec', type=str, default='./checkpoints/ae/Decoder', metavar='N',
                    help='path to decoder checkpoints')

args = parser.parse_args()
print(args)
args.cuda = True

ae = AE(args)
architectures = {'AE':  ae}

print(args.model)
if __name__ == "__main__":

    try:
        autoenc = architectures[args.model]
    except KeyError:
        print('---------------------------------------------------------')
        print('Model architecture not supported. ', end='')
        print('Maybe you can implement it?')
        print('---------------------------------------------------------')
        sys.exit()

    try:
        path_enc_id = args.path_enc_id
        path_enc_exp = args.path_enc_exp
        path_dec = args.path_dec

        if not os.path.exists(path_enc_id):
            os.makedirs(path_enc_id)
        if not os.path.exists(path_enc_exp):
            os.makedirs(path_enc_exp)
        if not os.path.exists(path_dec):
            os.makedirs(path_dec)

        for epoch in range(1, args.epochs + 1):
            autoenc.train(epoch)
            # autoenc.test(epoch)
    except (KeyboardInterrupt, SystemExit):
        print("Manual Interruption")


```

## Source file: train_gan3d.py

{
  "path": "train_gan3d.py",
  "blob_sha": "792d31fb1fc3c56d1fce68b171466482f320e0ba",
  "sha256": "d5c7c7c4507bba6343ed93f619ff94216b0e82e882325d52e3d7abf634af187b",
  "lines": 160,
  "bytes": 6517,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 4
  },
  "url": "https://github.com/aashishrai3799/3DFaceCAM/blob/7c908737ffde34844535e3d74e39e4c9f59b4d43/train_gan3d.py",
  "function_signature_candidates": [
    {
      "line": 18,
      "text": "def __init__(self, cfg, device, args):"
    },
    {
      "line": 42,
      "text": "def load_train_dataloader(self,path):"
    },
    {
      "line": 45,
      "text": "def gen_id_dict(self):"
    },
    {
      "line": 57,
      "text": "def load_id_dict(self, file_path):"
    },
    {
      "line": 67,
      "text": "def train(self):"
    }
  ]
}

```text

import torch
import torch.nn as nn
import torch.optim as optim

import os
import sys
import yaml
import argparse
import numpy as np
import pickle as pkl

sys.path.append(".")

from shape_model.architectures import generator_network, Discriminator
from shape_model.gan3d.utils.utils import make_ones, make_zeros, load_data, create_train_folders

class shape_GAN():
    def __init__(self, cfg, device, args):
        self.device = device
        self.cfg = cfg
        self.n_feat_id = self.cfg["n_feat_id"]
        self.n_feat_ex = self.cfg["n_feat_ex"]
        self.z_dim_noise = self.cfg["z_noise_dim"]
        self.z_dim_g_id = self.cfg["z_noise_dim"]+self.cfg["z_id_dim"]
        self.z_dim_g_ex = self.cfg["z_noise_dim"]+self.cfg["z_id_dim"]+self.cfg["z_ex_dim"]
        self.n_class_id = self.cfg["n_class_id"]
        self.n_class_ex = self.cfg["n_class_ex"]

        self.generator = generator_network(self.z_dim_g_id, self.z_dim_g_ex, self.n_feat_id, self.n_feat_ex).to(self.device)
        self.discriminator = Discriminator(self.n_feat_id, self.n_feat_ex, self.n_class_id, self.n_class_ex).to(self.device)

        self.g_optim = optim.Adam(self.generator.parameters(), lr=self.cfg["lr_g"])
        self.d_optim = optim.Adam(self.discriminator.parameters(), lr=self.cfg["lr_d"])

        self.criterion_gan = nn.BCEWithLogitsLoss()
        self.criterion_class= nn.CrossEntropyLoss()

        self.num_epochs = self.cfg["num_epochs"]
        self.folder = args.checkpoints_path
        self.zid_dict = args.zid_dict

    def load_train_dataloader(self,path):
        self.trainloader = load_data(path)

    def gen_id_dict(self):
        id_noise_dict = torch.empty((self.n_class_id, self.z_dim_g_id - self.z_dim_noise))
        dict_obj = dict()
        for i in range(1,self.n_class_id+1):
            noise_vec = torch.randn(self.z_dim_g_id - self.z_dim_noise)
            dict_obj[i] = noise_vec
            id_noise_dict[i-1] = noise_vec

        with open(os.path.join(self.folder,"zid_dictionary.pkl"),"wb") as dic:
            pkl.dump(dict_obj,dic)
        return id_noise_dict

    def load_id_dict(self, file_path):
        dic = pkl.load(open(file_path, 'rb'))
        print("loaded")
        id_noise_dict = torch.empty((self.n_class_id, self.z_dim_g_id - self.z_dim_noise))
        for i in range(1,self.n_class_id+1):
            id_noise_dict[i-1] = dic[i]
        id_noise_dict = id_noise_dict

        return id_noise_dict

    def train(self):
        self.generator.train()
        self.discriminator.train()
        self.load_train_dataloader(args.encoded_data)
        path_gen_id, path_gen_exp, path_disc = create_train_folders(self.folder)

        if self.zid_dict is not None:
            id_noise_dict = self.load_id_dict(self.zid_dict).to(device)
        else:
            id_noise_dict = self.gen_id_dict()

        for epoch in range(self.num_epochs):
            g_error_id = 0.0
            g_error_exp = 0.0
            g_error = 0.0
            d_error = 0.0

            for i, (data,label_id,label_ex) in enumerate(self.trainloader):
                real_data = data.to(device)
                label_id = label_id.long().to(device)
                label_ex = label_ex.long().to(device)

                batch_size = data.shape[0]

                ##Discriminator##
                z_noise = torch.randn(batch_size, 5).to(device)
                z_id = torch.index_select(id_noise_dict, 0, label_id)
                z_ex = torch.nn.functional.one_hot(label_ex, num_classes=self.n_class_ex)

                noise_id = torch.cat((z_noise, z_id), 1).to(device)
                noise_ex = torch.cat((z_noise, z_id, z_ex), 1).to(device)
                fake_data_id, fake_data_exp= self.generator(noise_id, noise_ex)

                fake_data = torch.cat((fake_data_id,fake_data_exp), 1)
                fake_data = fake_data.detach()

                self.d_optim.zero_grad()
                predict_real, predict_id, predict_ex= self.discriminator(real_data)
                error_real = self.criterion_gan(predict_real, make_ones(batch_size).to(device)) + self.criterion_class(predict_ex, label_ex) + self.criterion_class(predict_id, label_ex)
                error_real.backward()

                predict_fake, fake_id, fake_ex = self.discriminator(fake_data)
                error_fake = self.criterion_gan(predict_fake, make_zeros(batch_size).to(device))
                error_fake.backward()

                self.d_optim.step()

                d_error += error_real + error_fake

                ##Generator##
                z_noise = torch.randn(batch_size, 5).to(device)
                z_id = torch.index_select(id_noise_dict, 0, label_id)
                z_ex = torch.nn.functional.one_hot(label_ex.long(), num_classes=self.n_class_ex)

                noise_id = torch.cat((z_noise, z_id), 1).to(device)
                noise_ex = torch.cat((z_noise, z_id, z_ex), 1).to(device)
                fake_data_id, fake_data_exp = self.generator(noise_id, noise_ex)

                fake_data = torch.cat((fake_data_id,fake_data_exp), 1)
                self.g_optim.zero_grad()
                predict, fake_id, fake_ex = self.discriminator(fake_data)
                error = self.criterion_gan(predict, make_ones(batch_size).to(self.device)) + self.criterion_class(fake_ex, label_ex) + self.criterion_class(fake_id, label_id)
                error.backward()

                self.g_optim.step()

                g_error += error


            print('Epoch {}: g_loss: {:.8f}  d_loss: {:.8f}\r'.format(epoch, g_error/i,  d_error/i))
            if ((epoch+1) % 100)==0:

                torch.save(self.generator.generator_id.state_dict(), path_gen_id + str(int(epoch+1)/100))
                torch.save(self.generator.generator_exp.state_dict(), path_gen_exp + str(int(epoch + 1) / 100))
                torch.save(self.discriminator.state_dict(), path_disc + str(int(epoch + 1) / 100))

        print('Training Finished')

if __name__ == '__main__':
    device = torch.device("cuda:0")

    parser = argparse.ArgumentParser()

    parser.add_argument('--checkpoints_path', type=str, default='checkpoints/gan3d/')
    parser.add_argument('--encoded_data', type=str, default='data/reduced_train_data_130.npy')
    parser.add_argument('--zid_dict', type=str, default='data/zid_dictionary.pkl')

    args = parser.parse_args()

    with open("config.yml","r") as cfgfile:
        cfg = yaml.safe_load(cfgfile)

    gan = shape_GAN(cfg, device, args)
    gan.train()


```

## Source file: data/edit_obj.py

{
  "path": "data/edit_obj.py",
  "blob_sha": "4298d158800f470f84c1c84e36cc140450689b74",
  "sha256": "c51e42ed765159d408204bc43e4f827e658d9717234430ca2ae9b8c5211ef815",
  "lines": 48,
  "bytes": 1037,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/aashishrai3799/3DFaceCAM/blob/7c908737ffde34844535e3d74e39e4c9f59b4d43/data/edit_obj.py",
  "function_signature_candidates": [
    {
      "line": 4,
      "text": "def get_mtl(in_path='results/'):"
    }
  ]
}

```text

import os


def get_mtl(in_path='results/'):

	base_obj = open('data/template_mesh.obj', "r")
	base_lines = base_obj.readlines()

	base_mtl = open('data/base_mtl.mtl', "r")
	base_mtl_lines = base_mtl.readlines()

	ids = os.listdir(in_path)
	
	for idd in ids:
	
		exps = os.listdir(os.path.join(in_path, idd))
		
		for exp in exps:
			
			files = os.listdir(os.path.join(in_path,idd,exp))

			files.sort()

			for f in files:
				
				if 'obj' in f:
					
					obj_filename = os.path.join(in_path,idd,exp,f)

					a_file = open(obj_filename, "r")
					lines = a_file.readlines()
					#print(lines[56864])

					base_lines[1:26318] = lines[:26317]
					base_lines[0] = 'mtllib ./' + f[:-4] + '.mtl\n'
					#print(lines[56864])

					with open(obj_filename, 'w') as new_obj:
						new_obj.writelines(base_lines)
						new_obj.close()
					
					#print(f)	
					with open(obj_filename[:-4] + '.mtl', 'w') as new_mtl:
						base_mtl_lines[7] = 'map_Kd ' + str(f[:-4]) + '.jpg'
						new_mtl.writelines(base_mtl_lines)
						new_mtl.close()




```
