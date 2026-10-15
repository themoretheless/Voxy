# Facial rendering source scan: microsoft/Deep3DFaceReconstruction

Repository: https://github.com/microsoft/Deep3DFaceReconstruction

Pinned source revision: 1935ea92af72a72c4cf7b8a677a822afd8554b51

Discovery topic: face reconstruction 3D

Description: Accurate 3D Face Reconstruction with Weakly-Supervised Learning: From Single Image to Image Set (CVPRW 2019)

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"skin": ["skin.py"], "face": ["BFM/facemodel_info.mat", "face_decoder.py"]}

## Source file: skin.py

{
  "path": "skin.py",
  "blob_sha": "614a4541fd60d58cda5badc5054f102d9c02e606",
  "sha256": "e32ceb2a6f387fd24600bb0806a12afa5e251c33ecc6aff19ffb611a3f5518c6",
  "lines": 103,
  "bytes": 4693,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 24
  },
  "url": "https://github.com/microsoft/Deep3DFaceReconstruction/blob/1935ea92af72a72c4cf7b8a677a822afd8554b51/skin.py",
  "function_signature_candidates": [
    {
      "line": 5,
      "text": "def __init__(self, dim, num, w, mu, cov, cov_det, cov_inv):"
    },
    {
      "line": 18,
      "text": "def likelihood(self, data):"
    },
    {
      "line": 37,
      "text": "def _rgb2ycbcr(rgb):"
    },
    {
      "line": 49,
      "text": "def _bgr2ycbcr(bgr):"
    },
    {
      "line": 85,
      "text": "def skinmask(imbgr):"
    }
  ]
}

```text

import math
import numpy as np

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

```

## Source file: face_decoder.py

{
  "path": "face_decoder.py",
  "blob_sha": "77ce28b710b1cb371b98bbf4ba7d733d21fde983",
  "sha256": "9fe8b7f700135adb158c3fd0e97154d7b3da28a8ddda13e12d5bb60a7c1aac25",
  "lines": 345,
  "bytes": 13688,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 1,
    "rig": 4,
    "face": 97,
    "lip": 5
  },
  "url": "https://github.com/microsoft/Deep3DFaceReconstruction/blob/1935ea92af72a72c4cf7b8a677a822afd8554b51/face_decoder.py",
  "function_signature_candidates": [
    {
      "line": 17,
      "text": "def __init__(self,model_path = './BFM/BFM_model_front.mat'):"
    },
    {
      "line": 33,
      "text": "def __init__(self):"
    },
    {
      "line": 38,
      "text": "def Reconstruction_Block(self,coeff,opt):"
    },
    {
      "line": 80,
      "text": "def Split_coeff(self,coeff):"
    },
    {
      "line": 93,
      "text": "def Shape_formation_block(self,id_coeff,ex_coeff,facemodel):"
    },
    {
      "line": 104,
      "text": "def Compute_norm(self,face_shape,facemodel):"
    },
    {
      "line": 130,
      "text": "def Texture_formation_block(self,tex_coeff,facemodel):"
    },
    {
      "line": 138,
      "text": "def Compute_rotation_matrix(self,angles):"
    },
    {
      "line": 185,
      "text": "def Projection_block(self,face_shape,camera_scale,f_scale):"
    },
    {
      "line": 212,
      "text": "def Compute_landmark(self,face_shape,facemodel):"
    },
    {
      "line": 221,
      "text": "def Illumination_block(self,face_texture,norm_r,gamma):"
    },
    {
      "line": 256,
      "text": "def Rigid_transform_block(self,face_shape,rotation,translation):"
    },
    {
      "line": 263,
      "text": "def Render_block(self,face_shape,face_norm,face_color,camera_scale,f_scale,facemodel,batchsize,is_train=True):"
    }
  ]
}

```text

import tensorflow as tf 
import math as m
import numpy as np
from scipy.io import loadmat
import platform

is_windows = platform.system() == "Windows"

if not is_windows:
	from renderer import mesh_renderer
###############################################################################################
# Reconstruct 3D face based on output coefficients and facemodel
###############################################################################################

# BFM 3D face model
class BFM():
	def __init__(self,model_path = './BFM/BFM_model_front.mat'):
		model = loadmat(model_path)
		self.meanshape = tf.constant(model['meanshape']) # mean face shape. [3*N,1]
		self.idBase = tf.constant(model['idBase']) # identity basis. [3*N,80]
		self.exBase = tf.constant(model['exBase'].astype(np.float32)) # expression basis. [3*N,64]
		self.meantex = tf.constant(model['meantex']) # mean face texture. [3*N,1] (0-255)
		self.texBase = tf.constant(model['texBase']) # texture basis. [3*N,80]
		self.point_buf = tf.constant(model['point_buf']) # face indices for each vertex that lies in. starts from 1. [N,8]
		self.face_buf = tf.constant(model['tri']) # vertex indices for each face. starts from 1. [F,3]
		self.front_mask_render = tf.squeeze(tf.constant(model['frontmask2_idx'])) # vertex indices for small face region to compute photometric error. starts from 1.
		self.mask_face_buf = tf.constant(model['tri_mask2']) # vertex indices for each face from small face region. starts from 1. [f,3]
		self.skin_mask = tf.squeeze(tf.constant(model['skinmask'])) # vertex indices for pre-defined skin region to compute reflectance loss
		self.keypoints = tf.squeeze(tf.constant(model['keypoints'])) # vertex indices for 68 landmarks. starts from 1. [68,1]

# Analytic 3D face
class Face3D():
	def __init__(self):
		facemodel = BFM()
		self.facemodel = facemodel

	# analytic 3D face reconstructions with coefficients from R-Net
	def Reconstruction_Block(self,coeff,opt):
		#coeff: [batchsize,257] reconstruction coefficients

		id_coeff,ex_coeff,tex_coeff,angles,translation,gamma,camera_scale,f_scale = self.Split_coeff(coeff)
		# [batchsize,N,3] canonical face shape in BFM space
		face_shape = self.Shape_formation_block(id_coeff,ex_coeff,self.facemodel)
		# [batchsize,N,3] vertex texture (in RGB order)
		face_texture = self.Texture_formation_block(tex_coeff,self.facemodel)
		# [batchsize,3,3] rotation matrix for face shape
		rotation = self.Compute_rotation_matrix(angles)
		# [batchsize,N,3] vertex normal
		face_norm = self.Compute_norm(face_shape,self.facemodel)
		norm_r = tf.matmul(face_norm,rotation)

		# do rigid transformation for face shape using predicted rotation and translation
		face_shape_t = self.Rigid_transform_block(face_shape,rotation,translation)
		# compute 2d landmark projections 
		# landmark_p: [batchsize,68,2]	
		face_landmark_t = self.Compute_landmark(face_shape_t,self.facemodel)
		landmark_p = self.Projection_block(face_landmark_t,camera_scale,f_scale)

		# [batchsize,N,3] vertex color (in RGB order)
		face_color = self.Illumination_block(face_texture, norm_r, gamma)

		# reconstruction images and region masks for computing photometric loss		
		render_imgs,img_mask,img_mask_crop = self.Render_block(face_shape_t,norm_r,face_color,camera_scale,f_scale,self.facemodel,opt.batch_size,opt.is_train)

		self.id_coeff = id_coeff
		self.ex_coeff = ex_coeff
		self.tex_coeff = tex_coeff
		self.f_scale = f_scale
		self.gamma = gamma
		self.face_shape = face_shape
		self.face_shape_t = face_shape_t
		self.face_texture = face_texture
		self.face_color = face_color
		self.landmark_p = landmark_p
		self.render_imgs = render_imgs
		self.img_mask = img_mask
		self.img_mask_crop = img_mask_crop

	#----------------------------------------------------------------------------------------------
	def Split_coeff(self,coeff):

		id_coeff = coeff[:,:80]
		ex_coeff = coeff[:,80:144]
		tex_coeff = coeff[:,144:224]
		angles = coeff[:,224:227]
		gamma = coeff[:,227:254]
		translation = coeff[:,254:257]
		camera_scale = tf.ones([tf.shape(coeff)[0],1])
		f_scale = tf.ones([tf.shape(coeff)[0],1])

		return id_coeff,ex_coeff,tex_coeff,angles,translation,gamma,camera_scale,f_scale

	def Shape_formation_block(self,id_coeff,ex_coeff,facemodel):
		face_shape = tf.einsum('ij,aj->ai',facemodel.idBase,id_coeff) + \
					tf.einsum('ij,aj->ai',facemodel.exBase,ex_coeff) + facemodel.meanshape

		# reshape face shape to [batchsize,N,3]
		face_shape = tf.reshape(face_shape,[tf.shape(face_shape)[0],-1,3])
		# re-centering the face shape with mean shape
		face_shape = face_shape - tf.reshape(tf.reduce_mean(tf.reshape(facemodel.meanshape,[-1,3]),0),[1,1,3])

		return face_shape

	def Compute_norm(self,face_shape,facemodel):
		shape = face_shape
		face_id = facemodel.face_buf
		point_id = facemodel.point_buf

		# face_id and point_id index starts from 1
		face_id = tf.cast(face_id - 1,tf.int32)
		point_id = tf.cast(point_id - 1,tf.int32)

		#compute normal for each face
		v1 = tf.gather(shape,face_id[:,0], axis = 1)
		v2 = tf.gather(shape,face_id[:,1], axis = 1)
		v3 = tf.gather(shape,face_id[:,2], axis = 1)
		e1 = v1 - v2
		e2 = v2 - v3
		face_norm = tf.cross(e1,e2)

		face_norm = tf.nn.l2_normalize(face_norm, dim = 2) # normalized face_norm first
		face_norm = tf.concat([face_norm,tf.zeros([tf.shape(face_shape)[0],1,3])], axis = 1)

		#compute normal for each vertex using one-ring neighborhood
		v_norm = tf.reduce_sum(tf.gather(face_norm, point_id, axis = 1), axis = 2)
		v_norm = tf.nn.l2_normalize(v_norm, dim = 2)
		
		return v_norm

	def Texture_formation_block(self,tex_coeff,facemodel):
		face_texture = tf.einsum('ij,aj->ai',facemodel.texBase,tex_coeff) + facemodel.meantex

		# reshape face texture to [batchsize,N,3], note that texture is in RGB order
		face_texture = tf.reshape(face_texture,[tf.shape(face_texture)[0],-1,3])

		return face_texture

	def Compute_rotation_matrix(self,angles):
		n_data = tf.shape(angles)[0]

		# compute rotation matrix for X-axis, Y-axis, Z-axis respectively
		rotation_X = tf.concat([tf.ones([n_data,1]),
			tf.zeros([n_data,3]),
			tf.reshape(tf.cos(angles[:,0]),[n_data,1]),
			-tf.reshape(tf.sin(angles[:,0]),[n_data,1]),
			tf.zeros([n_data,1]),
			tf.reshape(tf.sin(angles[:,0]),[n_data,1]),
			tf.reshape(tf.cos(angles[:,0]),[n_data,1])],
			axis = 1
			)

		rotation_Y = tf.concat([tf.reshape(tf.cos(angles[:,1]),[n_data,1]),
			tf.zeros([n_data,1]),
			tf.reshape(tf.sin(angles[:,1]),[n_data,1]),
			tf.zeros([n_data,1]),
			tf.ones([n_data,1]),
			tf.zeros([n_data,1]),
			-tf.reshape(tf.sin(angles[:,1]),[n_data,1]),
			tf.zeros([n_data,1]),
			tf.reshape(tf.cos(angles[:,1]),[n_data,1])],
			axis = 1
			)

		rotation_Z = tf.concat([tf.reshape(tf.cos(angles[:,2]),[n_data,1]),
			-tf.reshape(tf.sin(angles[:,2]),[n_data,1]),
			tf.zeros([n_data,1]),
			tf.reshape(tf.sin(angles[:,2]),[n_data,1]),
			tf.reshape(tf.cos(angles[:,2]),[n_data,1]),
			tf.zeros([n_data,3]),
			tf.ones([n_data,1])],
			axis = 1
			)

		rotation_X = tf.reshape(rotation_X,[n_data,3,3])
		rotation_Y = tf.reshape(rotation_Y,[n_data,3,3])
		rotation_Z = tf.reshape(rotation_Z,[n_data,3,3])

		# R = RzRyRx
		rotation = tf.matmul(tf.matmul(rotation_Z,rotation_Y),rotation_X)

		rotation = tf.transpose(rotation, perm = [0,2,1])

		return rotation

	def Projection_block(self,face_shape,camera_scale,f_scale):

		# pre-defined camera focal for pespective projection
		focal = tf.constant(1015.0)
		focal = focal*f_scale
		focal = tf.reshape(focal,[-1,1])
		batchsize = tf.shape(focal)[0]

		# define camera position
		camera_pos = tf.reshape(tf.constant([0.0,0.0,10.0]),[1,1,3])*tf.reshape(camera_scale,[-1,1,1])
		reverse_z = tf.tile(tf.reshape(tf.constant([1.0,0,0,0,1,0,0,0,-1.0]),[1,3,3]),[tf.shape(face_shape)[0],1,1])

		# compute projection matrix
		p_matrix = tf.concat([focal,tf.zeros([batchsize,1]),112.*tf.ones([batchsize,1]),tf.zeros([batchsize,1]),focal,112.*tf.ones([batchsize,1]),tf.zeros([batchsize,2]),tf.ones([batchsize,1])],axis = 1)
		p_matrix = tf.reshape(p_matrix,[-1,3,3])

		# convert z in world space to the distance to camera
		face_shape = tf.matmul(face_shape,reverse_z) + camera_pos
		aug_projection = tf.matmul(face_shape,tf.transpose(p_matrix,[0,2,1]))

		# [batchsize, N,2] 2d face projection
		face_projection = aug_projection[:,:,0:2]/tf.reshape(aug_projection[:,:,2],[tf.shape(face_shape)[0],tf.shape(aug_projection)[1],1])


		return face_projection


	def Compute_landmark(self,face_shape,facemodel):

		# compute 3D landmark postitions with pre-computed 3D face shape
		keypoints_idx = facemodel.keypoints
		keypoints_idx = tf.cast(keypoints_idx - 1,tf.int32)
		face_landmark = tf.gather(face_shape,keypoints_idx,axis = 1)

		return face_landmark

	def Illumination_block(self,face_texture,norm_r,gamma):
		n_data = tf.shape(gamma)[0]
		n_point = tf.shape(norm_r)[1]
		gamma = tf.reshape(gamma,[n_data,3,9])
		# set initial lighting with an ambient lighting
		init_lit = tf.constant([0.8,0,0,0,0,0,0,0,0])
		gamma = gamma + tf.reshape(init_lit,[1,1,9])

		# compute vertex color using SH function approximation
		a0 = m.pi 
		a1 = 2*m.pi/tf.sqrt(3.0)
		a2 = 2*m.pi/tf.sqrt(8.0)
		c0 = 1/tf.sqrt(4*m.pi)
		c1 = tf.sqrt(3.0)/tf.sqrt(4*m.pi)
		c2 = 3*tf.sqrt(5.0)/tf.sqrt(12*m.pi)

		Y = tf.concat([tf.tile(tf.reshape(a0*c0,[1,1,1]),[n_data,n_point,1]),
			tf.expand_dims(-a1*c1*norm_r[:,:,1],2),
			tf.expand_dims(a1*c1*norm_r[:,:,2],2),
			tf.expand_dims(-a1*c1*norm_r[:,:,0],2),
			tf.expand_dims(a2*c2*norm_r[:,:,0]*norm_r[:,:,1],2),
			tf.expand_dims(-a2*c2*norm_r[:,:,1]*norm_r[:,:,2],2),
			tf.expand_dims(a2*c2*0.5/tf.sqrt(3.0)*(3*tf.square(norm_r[:,:,2])-1),2),
			tf.expand_dims(-a2*c2*norm_r[:,:,0]*norm_r[:,:,2],2),
			tf.expand_dims(a2*c2*0.5*(tf.square(norm_r[:,:,0])-tf.square(norm_r[:,:,1])),2)],axis = 2)

		color_r = tf.squeeze(tf.matmul(Y,tf.expand_dims(gamma[:,0,:],2)),axis = 2)
		color_g = tf.squeeze(tf.matmul(Y,tf.expand_dims(gamma[:,1,:],2)),axis = 2)
		color_b = tf.squeeze(tf.matmul(Y,tf.expand_dims(gamma[:,2,:],2)),axis = 2)

		#[batchsize,N,3] vertex color in RGB order
		face_color = tf.stack([color_r*face_texture[:,:,0],color_g*face_texture[:,:,1],color_b*face_texture[:,:,2]],axis = 2)

		return face_color

	def Rigid_transform_block(self,face_shape,rotation,translation):
		# do rigid transformation for 3D face shape
		face_shape_r = tf.matmul(face_shape,rotation)
		face_shape_t = face_shape_r + tf.reshape(translation,[tf.shape(face_shape)[0],1,3])

		return face_shape_t

	def Render_block(self,face_shape,face_norm,face_color,camera_scale,f_scale,facemodel,batchsize,is_train=True):
		if is_train and is_windows:
			raise ValueError('Not support training with Windows environment.')

		if is_windows:
			return [],[],[]

		# render reconstruction images 
		n_vex = int(facemodel.idBase.shape[0].value/3)
		fov_y = 2*tf.atan(112./(1015.*f_scale))*180./m.pi
		fov_y = tf.reshape(fov_y,[batchsize])
		# full face region
		face_shape = tf.reshape(face_shape,[batchsize,n_vex,3])
		face_norm = tf.reshape(face_norm,[batchsize,n_vex,3])
		face_color = tf.reshape(face_color,[batchsize,n_vex,3])

		# pre-defined cropped face region
		mask_face_shape = tf.gather(face_shape,tf.cast(facemodel.front_mask_render-1,tf.int32),axis = 1)
		mask_face_norm = tf.gather(face_norm,tf.cast(facemodel.front_mask_render-1,tf.int32),axis = 1)
		mask_face_color = tf.gather(face_color,tf.cast(facemodel.front_mask_render-1,tf.int32),axis = 1)

		# setting cammera settings
		camera_position = tf.constant([[0,0,10.0]])*tf.reshape(camera_scale,[-1,1])
		camera_lookat = tf.constant([0,0,0.0])
		camera_up = tf.constant([0,1.0,0])

		# setting light source position(intensities are set to 0 because we have computed the vertex color)
		light_positions = tf.tile(tf.reshape(tf.constant([0,0,1e5]),[1,1,3]),[batchsize,1,1])
		light_intensities = tf.tile(tf.reshape(tf.constant([0.0,0.0,0.0]),[1,1,3]),[batchsize,1,1])
		ambient_color = tf.tile(tf.reshape(tf.constant([1.0,1,1]),[1,3]),[batchsize,1])

		#using tf_mesh_renderer for rasterization (https://github.com/google/tf_mesh_renderer)
		# img: [batchsize,224,224,3] images in RGB order (0-255)
		# mask:[batchsize,224,224,1] transparency for img ({0,1} value)
		with tf.device('/cpu:0'):
			img_rgba = mesh_renderer.mesh_renderer(face_shape,
				tf.cast(facemodel.face_buf-1,tf.int32),
				face_norm,
				face_color,
				camera_position = camera_position,
				camera_lookat = camera_lookat,
				camera_up = camera_up,
				light_positions = light_positions,
				light_intensities = light_intensities,
				image_width = 224,
				image_height = 224,
				fov_y = fov_y,
				near_clip = 0.01,
				far_clip = 50.0,
				ambient_color = ambient_color)

		img = img_rgba[:,:,:,:3]
		mask = img_rgba[:,:,:,3:]

		img = tf.cast(img[:,:,:,::-1],tf.float32) #transfer RGB to BGR
		mask = tf.cast(mask,tf.float32) # full face region

		if is_train:
			# compute mask for small face region
			with tf.device('/cpu:0'):
				img_crop_rgba = mesh_renderer.mesh_renderer(mask_face_shape,
					tf.cast(facemodel.mask_face_buf-1,tf.int32),
					mask_face_norm,
					mask_face_color,
					camera_position = camera_position,
					camera_lookat = camera_lookat,
					camera_up = camera_up,
					light_positions = light_positions,
					light_intensities = light_intensities,
					image_width = 224,
					image_height = 224,
					fov_y = fov_y,
					near_clip = 0.01,
					far_clip = 50.0,
					ambient_color = ambient_color)

			mask_f = img_crop_rgba[:,:,:,3:]
			mask_f = tf.cast(mask_f,tf.float32) # small face region
			return img,mask,mask_f

		img_rgba = tf.cast(tf.clip_by_value(img_rgba,0,255),tf.float32)

		return img_rgba,mask,mask


```

## Source file: train.py

{
  "path": "train.py",
  "blob_sha": "b84d7866388a551783323db94c4fc262a8a00646",
  "sha256": "0072797dab34dcff890765a5b3927b06fdfe5f10bb949312bdaee21c8a8a952a",
  "lines": 141,
  "bytes": 4986,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 4,
    "rig": 1,
    "face": 6
  },
  "url": "https://github.com/microsoft/Deep3DFaceReconstruction/blob/1935ea92af72a72c4cf7b8a677a822afd8554b51/train.py",
  "function_signature_candidates": [
    {
      "line": 17,
      "text": "def parse_args():"
    },
    {
      "line": 29,
      "text": "def restore_weights_and_initialize(opt):"
    },
    {
      "line": 64,
      "text": "def train():"
    }
  ]
}

```text

import tensorflow as tf 
import numpy as np 
import os
from options import Option
from reconstruction_model import *
from data_loader import *
from utils import *
import argparse
###############################################################################################
# training stage
###############################################################################################


os.environ['CUDA_VISIBLE_DEVICES'] = '0'

# training data and validation data
def parse_args():
    desc = "Deep3DFaceReconstruction"
    parser = argparse.ArgumentParser(description=desc)

    parser.add_argument('--data_path', type=str, default='./processed_data', help='training data folder')
    parser.add_argument('--val_data_path', type=str, default='./processed_data', help='validation data folder')
    parser.add_argument('--model_name', type=str, default='./model_test', help='model name')


    return parser.parse_args()

# initialize weights for resnet and facenet
def restore_weights_and_initialize(opt):
	var_list = tf.trainable_variables()
	g_list = tf.global_variables()

	# add batch normalization params into trainable variables 
	bn_moving_vars = [g for g in g_list if 'moving_mean' in g.name]
	bn_moving_vars += [g for g in g_list if 'moving_variance' in g.name]
	var_list +=bn_moving_vars

	# create saver to save and restore weights
	resnet_vars = [v for v in var_list if 'resnet_v1_50' in v.name]
	facenet_vars = [v for v in var_list if 'InceptionResnetV1' in v.name]
	saver_resnet = tf.train.Saver(var_list = resnet_vars)
	saver_facenet = tf.train.Saver(var_list = facenet_vars)

	saver = tf.train.Saver(var_list = resnet_vars + [v for v in var_list if 'fc-' in v.name],max_to_keep = 50)

	# create session
	sess = tf.InteractiveSession(config = opt.config)

	# create summary op
	train_writer = tf.summary.FileWriter(opt.train_summary_path, sess.graph)
	val_writer = tf.summary.FileWriter(opt.val_summary_path, sess.graph)

	# initialization
	tf.global_variables_initializer().run()
	tf.local_variables_initializer().run()

	saver_resnet.restore(sess,opt.R_net_weights)
	saver_facenet.restore(sess,opt.Perceptual_net_weights)

	return saver, train_writer,val_writer, sess


# main function for training
def train():

	# read BFM face model
	# transfer original BFM model to our model
	if not os.path.isfile('./BFM/BFM_model_front.mat'):
		transferBFM09()

	with tf.Graph().as_default() as graph:

		# training options
		args = parse_args()
		opt = Option(model_name=args.model_name)
		opt.data_path = [args.data_path]
		opt.val_data_path = [args.val_data_path]

		# load training data into queue
		train_iterator = load_dataset(opt)
		# create reconstruction model
		model = Reconstruction_model(opt)
		# send training data to the model
		model.set_input(train_iterator)
		# update model variables with training data
		model.step(is_train = True)
		# summarize training statistics
		model.summarize()

		# several training stattistics to be saved
		train_stat = model.summary_stat
		train_img_stat = model.summary_img
		train_op = model.train_op
		photo_error = model.photo_loss
		lm_error = model.landmark_loss
		id_error = model.perceptual_loss

		# load validation data into queue
		val_iterator = load_dataset(opt,train=False)
		# send validation data to the model
		model.set_input(val_iterator)
		# only do foward pass without updating model variables
		model.step(is_train = False)
		# summarize validation statistics
		model.summarize()
		val_stat = model.summary_stat
		val_img_stat = model.summary_img

		# initialization
		saver, train_writer,val_writer, sess = restore_weights_and_initialize(opt)

		# freeze the graph to ensure no new op will be added during training
		sess.graph.finalize()

		# training loop
		for i in range(opt.train_maxiter):
			_,ph_loss,lm_loss,id_loss = sess.run([train_op,photo_error,lm_error,id_error])
			print('Iter: %d; lm_loss: %f ; photo_loss: %f; id_loss: %f\n'%(i,np.sqrt(lm_loss),ph_loss,id_loss))
			# summarize training stats every <train_summary_iter> iterations
			if np.mod(i,opt.train_summary_iter) == 0:
				train_summary = sess.run(train_stat)
				train_writer.add_summary(train_summary,i)

			# summarize image stats every <image_summary_iter> iterations
			if np.mod(i,opt.image_summary_iter) == 0:
				train_img_summary = sess.run(train_img_stat)
				train_writer.add_summary(train_img_summary,i)

			# summarize validation stats every <val_summary_iter> iterations	
			if np.mod(i,opt.val_summary_iter) == 0:
				val_summary,val_img_summary = sess.run([val_stat,val_img_stat])
				val_writer.add_summary(val_summary,i)
				val_writer.add_summary(val_img_summary,i)

			# # save model variables every <save_iter> iterations	
			if np.mod(i,opt.save_iter) == 0:
				saver.save(sess,os.path.join(opt.model_save_path,'iter_%d.ckpt'%i))


if __name__ == '__main__':
	train()

```

## Source file: utils.py

{
  "path": "utils.py",
  "blob_sha": "4083363b8df03507919bcbfc71cfd1d50989a6f7",
  "sha256": "2dc228a0c1f7f72480aed91e5030001bd6300b0a22d34ed713bad6b9687d51dd",
  "lines": 137,
  "bytes": 4380,
  "selection_score": 0,
  "keyword_line_counts": {
    "skin": 3,
    "rig": 9,
    "face": 9
  },
  "url": "https://github.com/microsoft/Deep3DFaceReconstruction/blob/1935ea92af72a72c4cf7b8a677a822afd8554b51/utils.py",
  "function_signature_candidates": [
    {
      "line": 8,
      "text": "def LoadExpBasis():"
    },
    {
      "line": 27,
      "text": "def transferBFM09():"
    },
    {
      "line": 98,
      "text": "def load_lm3d():"
    },
    {
      "line": 111,
      "text": "def load_img(img_path,lm_path):"
    },
    {
      "line": 119,
      "text": "def save_obj(path,v,f,c):"
    },
    {
      "line": 132,
      "text": "def load_graph(graph_filename):"
    }
  ]
}

```text

import numpy as np 
import tensorflow as tf
from PIL import Image
from scipy.io import loadmat,savemat
from array import array

# load expression basis
def LoadExpBasis():
	n_vertex = 53215
	Expbin = open('BFM/Exp_Pca.bin','rb')
	exp_dim = array('i')
	exp_dim.fromfile(Expbin,1)
	expMU = array('f')
	expPC = array('f')
	expMU.fromfile(Expbin,3*n_vertex)
	expPC.fromfile(Expbin,3*exp_dim[0]*n_vertex)

	expPC = np.array(expPC)
	expPC = np.reshape(expPC,[exp_dim[0],-1])
	expPC = np.transpose(expPC)

	expEV = np.loadtxt('BFM/std_exp.txt')

	return expPC,expEV

# transfer original BFM09 to our face model
def transferBFM09():
	original_BFM = loadmat('BFM/01_MorphableModel.mat')
	shapePC = original_BFM['shapePC'] # shape basis
	shapeEV = original_BFM['shapeEV'] # corresponding eigen value
	shapeMU = original_BFM['shapeMU'] # mean face
	texPC = original_BFM['texPC'] # texture basis
	texEV = original_BFM['texEV'] # eigen value
	texMU = original_BFM['texMU'] # mean texture

	expPC,expEV = LoadExpBasis()

	# transfer BFM09 to our face model

	idBase = shapePC*np.reshape(shapeEV,[-1,199])
	idBase = idBase/1e5 # unify the scale to decimeter
	idBase = idBase[:,:80] # use only first 80 basis

	exBase = expPC*np.reshape(expEV,[-1,79])
	exBase = exBase/1e5 # unify the scale to decimeter
	exBase = exBase[:,:64] # use only first 64 basis

	texBase = texPC*np.reshape(texEV,[-1,199])
	texBase = texBase[:,:80] # use only first 80 basis

	# our face model is cropped align face landmarks which contains only 35709 vertex.
	# original BFM09 contains 53490 vertex, and expression basis provided by JuYong contains 53215 vertex.
	# thus we select corresponding vertex to get our face model.

	index_exp = loadmat('BFM/BFM_front_idx.mat')
	index_exp = index_exp['idx'].astype(np.int32) - 1 #starts from 0 (to 53215)

	index_shape = loadmat('BFM/BFM_exp_idx.mat')
	index_shape = index_shape['trimIndex'].astype(np.int32) - 1 #starts from 0 (to 53490)
	index_shape = index_shape[index_exp]


	idBase = np.reshape(idBase,[-1,3,80])
	idBase = idBase[index_shape,:,:]
	idBase = np.reshape(idBase,[-1,80])

	texBase = np.reshape(texBase,[-1,3,80])
	texBase = texBase[index_shape,:,:]
	texBase = np.reshape(texBase,[-1,80])

	exBase = np.reshape(exBase,[-1,3,64])
	exBase = exBase[index_exp,:,:]
	exBase = np.reshape(exBase,[-1,64])

	meanshape = np.reshape(shapeMU,[-1,3])/1e5
	meanshape = meanshape[index_shape,:]
	meanshape = np.reshape(meanshape,[1,-1])

	meantex = np.reshape(texMU,[-1,3])
	meantex = meantex[index_shape,:]
	meantex = np.reshape(meantex,[1,-1])

	# other info contains triangles, region used for computing photometric loss,
	# region used for skin texture regularization, and 68 landmarks index etc.
	other_info = loadmat('BFM/facemodel_info.mat')
	frontmask2_idx = other_info['frontmask2_idx']
	skinmask = other_info['skinmask']
	keypoints = other_info['keypoints']
	point_buf = other_info['point_buf']
	tri = other_info['tri']
	tri_mask2 = other_info['tri_mask2']

	# save our face model
	savemat('BFM/BFM_model_front.mat',{'meanshape':meanshape,'meantex':meantex,'idBase':idBase,'exBase':exBase,'texBase':texBase,'tri':tri,'point_buf':point_buf,'tri_mask2':tri_mask2\
		,'keypoints':keypoints,'frontmask2_idx':frontmask2_idx,'skinmask':skinmask})

# load landmarks for standard face, which is used for image preprocessing
def load_lm3d():

	Lm3D = loadmat('./BFM/similarity_Lm3D_all.mat')
	Lm3D = Lm3D['lm']

	# calculate 5 facial landmarks using 68 landmarks
	lm_idx = np.array([31,37,40,43,46,49,55]) - 1
	Lm3D = np.stack([Lm3D[lm_idx[0],:],np.mean(Lm3D[lm_idx[[1,2]],:],0),np.mean(Lm3D[lm_idx[[3,4]],:],0),Lm3D[lm_idx[5],:],Lm3D[lm_idx[6],:]], axis = 0)
	Lm3D = Lm3D[[1,2,0,3,4],:]

	return Lm3D

# load input images and corresponding 5 landmarks
def load_img(img_path,lm_path):

	image = Image.open(img_path)
	lm = np.loadtxt(lm_path)

	return image,lm

# save 3D face to obj file
def save_obj(path,v,f,c):
	with open(path,'w') as file:
		for i in range(len(v)):
			file.write('v %f %f %f %f %f %f\n'%(v[i,0],v[i,1],v[i,2],c[i,0],c[i,1],c[i,2]))

		file.write('\n')

		for i in range(len(f)):
			file.write('f %d %d %d\n'%(f[i,0],f[i,1],f[i,2]))

	file.close()

# load .pb file into tensorflow graph
def load_graph(graph_filename):
	with tf.gfile.GFile(graph_filename,'rb') as f:
		graph_def = tf.GraphDef()
		graph_def.ParseFromString(f.read())

	return graph_def

```
