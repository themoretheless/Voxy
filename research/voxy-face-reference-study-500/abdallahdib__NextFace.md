# Facial rendering source scan: abdallahdib/NextFace

Repository: https://github.com/abdallahdib/NextFace

Pinned source revision: 07b2b1c7da2021e939e469f82fd9823e3d0ec67c

Discovery topic: face reconstruction 3D

Description: A high-fidelity 3D face reconstruction library from monocular RGB image(s)

Reported license: GPL-3.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {}

## Source file: image.py

{
  "path": "image.py",
  "blob_sha": "83f5ea79ad120bda84366f1774c945acbc411734",
  "sha256": "65cad6c3d3a98b0308d3beccf8b0e72b5e680253acbff258f5ed16b59b9e763e",
  "lines": 132,
  "bytes": 5198,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/abdallahdib/NextFace/blob/07b2b1c7da2021e939e469f82fd9823e3d0ec67c/image.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def saveImage(image, fileName, gamma = 2.2):"
    },
    {
      "line": 21,
      "text": "def overlayImage(background, image):"
    },
    {
      "line": 36,
      "text": "def resizeImage(image, targetResolution):"
    },
    {
      "line": 59,
      "text": "def __init__(self, path, device, maxRes = 512):"
    },
    {
      "line": 82,
      "text": "def __init__(self, path, device, maxRes = 512):"
    },
    {
      "line": 128,
      "text": "def asNumpyArray(self):"
    }
  ]
}

```text

import sys
from os import walk
import numpy as np
import torch
import cv2
import os


def saveImage(image, fileName, gamma = 2.2):
    '''
    save image to drive
    :param image: float tensor [w, h, 3/4]
    :param fileName: path to where to save the image
    :param gamma: gamma correction
    :return:
    '''

    import pyredner
    pyredner.imwrite(image.cpu().detach(), fileName, gamma = gamma)

def overlayImage(background, image):
    '''
    overlay image on top of background image an image on a background image.
    :param background: float tensor [width,height,3]
    :param image: float tensor [width, height, 4]
    :return: float tensor [width, height, 3]
    '''
    assert(torch.is_tensor(background) and torch.is_tensor(image) and background.dim() == 3 and image.dim() == 3 and background.shape[0] == image.shape[0] and background.shape[1] == image.shape[1])
    assert(background.shape[-1] == 3 and image.shape[-1] == 4)
    from torchvision import transforms
    background = transforms.ToPILImage()(background.permute(2, 1, 0).clone().detach().cpu()).convert("RGB")
    image = transforms.ToPILImage()(torch.clamp(image.permute(2, 1, 0), 0, 1).clone().detach().cpu()).convert("RGBA")
    background.paste(image, (0, 0), image)
    return transforms.ToTensor()(background).permute(2, 1, 0)

def resizeImage(image, targetResolution):
    '''
    resize an image (as numpy array) to the target resolution
    :param image: numpy array [h, w, 4/3/1]
    :param targetResolution: int > 0
    :return: numpy array [h, w, 4/3/1]
    '''
    assert(image is not None and isinstance(image, np.ndarray) and len(image.shape) == 3 and image.shape[-1] == 3 or image.shape[-1] == 4 or image.shape[-1] == 1)
    dmax = max(image.shape[0], image.shape[1])

    if (dmax > targetResolution):
        print("[INFO] resizing input image to fit:", targetResolution,"px resolution...")
        if (image.shape[0] > image.shape[1]):
            scale = float(targetResolution) / float(image.shape[0])
        else:
            scale = float(targetResolution) / float(image.shape[1])
        img = cv2.resize(image, (int(image.shape[1] * scale), int(image.shape[0] * scale)), interpolation=cv2.INTER_CUBIC )
    else:
        return image
    return img

class Image:

    def __init__(self, path, device, maxRes = 512):
        '''
        class that represent a single image  as a pytorch tensor [1, h, w, channels]
        :param path: the path to the image
        :param device: where to store the image ('cpu' or 'cuda')
        :param maxRes: maximum allowed resolution (depending on the gpu/cpu memory and speed, this limit can be increased or removed)
        '''
        assert(maxRes > 0)
        print('loading image from path: ', path)
        self.device = device
        numpyImage = cv2.imread(path)[..., 0:3]
        assert (numpyImage is not None)
        numpyImage = resizeImage(cv2.cvtColor(numpyImage, cv2.COLOR_BGR2RGB), int(maxRes))
        self.tensor = (torch.from_numpy(numpyImage).to(self.device).to(dtype=torch.float32) / 255.0).unsqueeze(0)
        self.height = numpyImage.shape[0]
        self.width = numpyImage.shape[1]
        self.channels = numpyImage.shape[2]
        self.gamma = 2.2
        self.center = torch.tensor([ self.width / 2, self.height / 2], dtype = torch.float32, device = self.device).reshape(1, -1)
        self.imageName = os.path.basename(path)

class ImageFolder:

    def __init__(self, path, device, maxRes = 512):
        '''
        class that represent images in a given path
        :param path: the path to the image
        :param device: where to store the image ('cpu' or 'cuda')
        '''
        print('loading images from path: ', path)
        self.device = device
        self.tensor = None
        self.imageNames = []
        supportedFormats = ['.jpg', '.jpeg', '.png']

        filenames = next(walk(path), (None, None, []))[2]
        width = None
        height = None
        ct = 0

        assert (len(filenames) > 0)  # no images found in the given directory
        for filename in filenames:
            if os.path.splitext(filename)[1].lower() in supportedFormats:
                image = Image(path + '/' + filename, device, maxRes)

                if width is None:
                    width = image.width
                    height = image.height
                    self.tensor = torch.zeros([len(filenames), height, width, image.channels], device = self.device)
                    self.center = torch.zeros([len(filenames), 2], device = self.device)

                assert image.width == width and image.height == height

                self.width = image.width
                self.height = image.height
                self.channels = image.channels
                self.tensor[ct] = image.tensor[0].clone().detach()
                self.center[ct] = image.center[0].clone().detach()
                self.imageNames.append(image.imageName)
                image = None

                ct += 1


        import gc
        gc.collect()
        self.gamma = 2.2

    @property
    def asNumpyArray(self):
        return self.tensor.detach().cpu().numpy() * 255.0

if __name__ == "__main__":
    pass

```

## Source file: utils.py

{
  "path": "utils.py",
  "blob_sha": "1547b457d21f65cde1d802b08d121422abd988b8",
  "sha256": "90f116cae86948930244e52c8cfbd29905659dfe739d4e476c9198bd98feb757",
  "lines": 124,
  "bytes": 4188,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 10
  },
  "url": "https://github.com/abdallahdib/NextFace/blob/07b2b1c7da2021e939e469f82fd9823e3d0ec67c/utils.py",
  "function_signature_candidates": [
    {
      "line": 6,
      "text": "def saveObj(filename, materialName, vertices, faces, normals = None, tcoords = None, textureFileName = 'texture.png'):"
    },
    {
      "line": 79,
      "text": "def saveLandmarksVerticesProjections(imageTensor, projPoints, landmarks):"
    },
    {
      "line": 102,
      "text": "def mkdir_p(path):"
    },
    {
      "line": 113,
      "text": "def loadDictionaryFromPickle(picklePath):"
    },
    {
      "line": 120,
      "text": "def writeDictionaryToPickle(dict, picklePath):"
    }
  ]
}

```text

import numpy as np
import torch
import cv2
import os

def saveObj(filename, materialName, vertices, faces, normals = None, tcoords = None, textureFileName = 'texture.png'):
    '''
    write mesh to an obj file
    :param filename: path to where to save the obj file
    :param materialFileName: material name
    :param vertices:  float tensor [n, 3]
    :param faces: tensor [#triangles, 3]
    :param normals: float tensor [n, 3]
    :param tcoords: float tensor [n, 2]
    :param textureFileName: name of the texture to use with material
    :return:
    '''
    assert(vertices.dim() == 2 and  vertices.shape[-1] == 3)
    assert (faces.dim() == 2 and faces.shape[-1] == 3)

    if normals is not None:
        assert (normals.dim() == 2 and normals.shape[-1] == 3)

    if tcoords is not None:
        assert (tcoords.dim() == 2 and tcoords.shape[-1] == 2)

    if torch.is_tensor(vertices):
        vertices = vertices.detach().cpu().numpy()
    if torch.is_tensor(faces):
        faces = faces.detach().cpu().numpy()
    if torch.is_tensor(normals):
        normals = normals.detach().cpu().numpy()
    if torch.is_tensor(tcoords):
        tcoords = tcoords.detach().cpu().numpy()

    assert(isinstance(vertices, np.ndarray))
    assert (isinstance(faces, np.ndarray))
    assert (isinstance(normals, np.ndarray))
    assert (isinstance(tcoords, np.ndarray))

    #write material
    f = open(os.path.dirname(filename) + '/' + materialName, 'w')
    f.write('newmtl material0\n')
    f.write('map_Kd ' + textureFileName + '\n')
    f.close()

    f = open(filename, 'w')
    f.write('###########################################################\n')
    f.write('# OBJ file generated by faceYard 2021\n')
    f.write('#\n')
    f.write('# Num Vertices: %d\n' % (vertices.shape[0]))
    f.write('# Num Triangles: %d\n' % (faces.shape[0]))
    f.write('#\n')
    f.write('###########################################################\n')
    f.write('\n')
    f.write('mtllib ' + materialName + '\n')

    #write vertices
    for v in vertices:
        f.write('v %f %f %f\n' % (v[0], v[1], v[2]))

    # write the tcoords
    if tcoords is not None and tcoords.shape[0] > 0:
        for uv in tcoords:
            f.write('vt %f %f\n' % (uv[0], uv[1]))

    #write the normals
    if normals is not None and normals.shape[0] > 0:
        for n in normals:
            f.write('vn %f %f %f\n' % (n[0], n[1], n[2]))

    f.write('usemtl material0\n')
    #write face indices list
    for t in faces:
        f.write('f %d/%d/%d %d/%d/%d %d/%d/%d\n' % (t[0] + 1, t[0] + 1,t[0] + 1,
                                              t[1] + 1, t[1] + 1,t[1] + 1,
                                              t[2] + 1, t[2] + 1, t[2] + 1))
    f.close()
def saveLandmarksVerticesProjections(imageTensor, projPoints, landmarks):
    '''
    for debug, render the projected vertices and landmakrs on image
    :param images: [w, h, 3]
    :param projPoints: [n, 3]
    :param landmarks: [n, 2]
    :return: tensor [w, h, 3
    '''
    assert(imageTensor.dim() == 3 and imageTensor.shape[-1] == 3 )
    assert(projPoints.dim() == 2 and projPoints.shape[-1] == 2)
    assert(projPoints.shape == landmarks.shape)
    image = imageTensor.clone().detach().cpu().numpy() * 255.
    landmarkCount = landmarks.shape[0]
    for i in range(landmarkCount):
        x = landmarks[i, 0]
        y = landmarks[i, 1]
        cv2.circle(image, (int(x), int(y)), 2, (0, 255, 0), -1)
        x = projPoints[i, 0]
        y = projPoints[i, 1]
        cv2.circle(image, (int(x), int(y)), 2, (0, 0, 255), -1)

    return image

def mkdir_p(path):
    import errno
    import os

    try:
        os.makedirs(path)
    except OSError as exc:
        if exc.errno == errno.EEXIST and os.path.isdir(path):
            pass
        else:
            raise
def loadDictionaryFromPickle(picklePath):
    import pickle
    handle = open(picklePath, 'rb')
    assert handle is not None
    dic = pickle.load(handle)
    handle.close()
    return dic
def writeDictionaryToPickle(dict, picklePath):
    import pickle
    handle = open(picklePath, 'wb')
    pickle.dump(dict, handle, pickle.HIGHEST_PROTOCOL)
    handle.close()

```

## Source file: camera.py

{
  "path": "camera.py",
  "blob_sha": "1ef3b3f99fcca9ab7ef0f65b02b7dcbc36ed8243",
  "sha256": "0f078cd536461e8da81f7b2d9363e7aef74744a30de15487f11f69380d65c3ec",
  "lines": 65,
  "bytes": 3369,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/abdallahdib/NextFace/blob/07b2b1c7da2021e939e469f82fd9823e3d0ec67c/camera.py",
  "function_signature_candidates": [
    {
      "line": 8,
      "text": "def __init__(self, device):"
    },
    {
      "line": 23,
      "text": "def computeTransformation(self, rotation, translation):"
    },
    {
      "line": 48,
      "text": "def transformVertices(self, vertices, translation, rotation):"
    }
  ]
}

```text

import torch
import numpy as np



class Camera:

    def __init__(self, device):
        self.device = device

        self.rotXm1 = torch.tensor(np.array([[0., 0., 0.], [0., 1., 0.], [0., 0., 1.]]), dtype=torch.float, device=device)
        self.rotXm2 = torch.tensor(np.array([[0., 0., 0.], [0., 0., -1.], [0., 1., 0.]]), dtype=torch.float, device=device)
        self.rotXm3 = torch.tensor(np.array([[1., 0., 0.], [0., 0., 0.], [0., 0., 0.]]), dtype=torch.float, device=device)

        self.rotYm1 = torch.tensor(np.array([[1., 0., 0.], [0., 0., 0.], [0., 0., 1.]]), dtype=torch.float, device=device)
        self.rotYm2 = torch.tensor(np.array([[0., 0., 1.], [0., 0., 0.], [-1., 0., 0.]]), dtype=torch.float, device=device)
        self.rotYm3 = torch.tensor(np.array([[0., 0., 0.], [0., 1., 0.], [0., 0., 0.]]), dtype=torch.float, device=device)

        self.rotZm1 = torch.tensor(np.array([[1., 0., 0.], [0., 1., 0.], [0., 0., 0.]]), dtype=torch.float, device=device)
        self.rotZm2 = torch.tensor(np.array([[0., -1., 0.], [1., 0., 0.], [0., 0., 0.]]), dtype=torch.float, device=device)
        self.rotZm3 = torch.tensor(np.array([[0., 0., 0.], [0., 0., 0.], [0., 0., 1.]]), dtype=torch.float, device=device)

    def computeTransformation(self, rotation, translation):
        '''
        create a transformation matrix from rotation and translation
        rotation: [n, 3]
        translation: [n, 3]
        return: transformation matrix [n, 4, 3]
        '''

        assert (rotation.dim() == 2 and rotation.shape[-1] == 3)
        assert(translation.dim() == 2 and translation.shape[-1] == 3)

        rotx = torch.cos(rotation[..., :1, None]).expand(-1, 3, 3) * self.rotXm1 \
               + torch.sin(rotation[..., :1, None]).expand(-1, 3, 3) * self.rotXm2 \
               + self.rotXm3
        roty = torch.cos(rotation[..., 1:2, None]).expand(-1, 3, 3) * self.rotYm1 \
               + torch.sin(rotation[..., 1:2, None]).expand( -1, 3, 3) * self.rotYm2 \
               + self.rotYm3
        rotz = torch.cos(rotation[..., 2:, None]).expand(-1, 3, 3) * self.rotZm1 \
               + torch.sin(rotation[..., 2:, None]).expand(-1, 3, 3) * self.rotZm2 \
               + self.rotZm3

        rotMatrix = torch.matmul(rotz, torch.matmul(roty, rotx))
        transformation = torch.cat((rotMatrix, translation[ :, :, None]), -1)
        return transformation

    def transformVertices(self, vertices, translation, rotation):
        '''
        transform vertices by the rotation and translation vector
        :param vertices: tensor [n, verticesNumber, 3]
        :param translation:  tensor [n, 3]
        :param rotation: tensor [n, 3]
        :return: transformed vertices [n, verticesNumber, 3]
        '''
        assert (vertices.dim() == 3 and vertices.shape[-1] == 3)

        transformationMatrix = self.computeTransformation(rotation, translation)
        ones = torch.ones([vertices.shape[0], vertices.shape[1], 1], dtype = torch.float, device = vertices.device)
        vertices = torch.cat((vertices, ones), -1)
        framesNumber = transformationMatrix.shape[0]
        verticesNumber = vertices.shape[1]
        out = torch.matmul(transformationMatrix.view(1, framesNumber, 1, 3, 4),
                           vertices.view(framesNumber, verticesNumber, 4, 1)).view(1, framesNumber, verticesNumber, 3)
        return out[0]

```

## Source file: config.py

{
  "path": "config.py",
  "blob_sha": "5dd284d662581d7ccfc924cd1fab4595f22e1955",
  "sha256": "bf7bb9697a1ca9a1efc1700d702c0cda7efb5998a0773e862f5eb37466c7942c",
  "lines": 110,
  "bytes": 4042,
  "selection_score": 0,
  "keyword_line_counts": {
    "sss": 4
  },
  "url": "https://github.com/abdallahdib/NextFace/blob/07b2b1c7da2021e939e469f82fd9823e3d0ec67c/config.py",
  "function_signature_candidates": [
    {
      "line": 6,
      "text": "def __init__(self):"
    },
    {
      "line": 59,
      "text": "def fillFromDicFile(self, filePath):"
    },
    {
      "line": 107,
      "text": "def print(self):"
    }
  ]
}

```text

import copy
import sys


class Config:
	def __init__(self):
		#compute device
		self.device = 'cuda'

		#tracker
		self.lamdmarksDetectorType = 'mediapipe'  # Options ['mediapipe', 'fan']

		#morphable model
		self.path = 'baselMorphableModel'
		self.textureResolution = 256 #256 or 512
		self.trimPca = False  # if True keep only a subset of the pca basis (eigen vectors)

		#spherical harmonics
		self.bands = 9
		self.envMapRes = 64
		self.smoothSh = False
		self.saveExr = True
		#camera
		self.camFocalLength = 500.0 #focal length in pixels (f =  f_{mm} * imageWidth / sensorWidth)
		self.optimizeFocalLength = True #if True the initial focal length is estimated otherwise it remains constant

		#image
		self.maxResolution = 512

		#optimization
		self.iterStep1 = 2000 # number of iterations for the coarse optim
		self.iterStep2 = 400 #number of iteration for the first dense optim (based on statistical priors)
		self.iterStep3 = 100 #number of iterations for refining the statistical albedo priors
		self.weightLandmarksLossStep2 = 0.001 #landmarks weight during step2
		self.weightLandmarksLossStep3 = 0.001  # landmarks weight during step3

		self.weightShapeReg = 0.001 #weight for shape regularization
		self.weightExpressionReg = 0.001  # weight for expression regularization
		self.weightAlbedoReg = 0.001  # weight for albedo regularization

		self.weightDiffuseSymmetryReg = 50. #symmetry regularizer weight for diffuse texture (at step 3)
		self.weightDiffuseConsistencyReg = 100.  # consistency regularizer weight for diffuse texture (at step 3)
		self.weightDiffuseSmoothnessReg = 0.001  # smoothness regularizer weight for diffuse texture (at step 3)

		self.weightSpecularSymmetryReg = 30.  # symmetry regularizer weight for specular texture (at step 3)
		self.weightSpecularConsistencyReg = 2.  # consistency regularizer weight for specular texture (at step 3)
		self.weightSpecularSmoothnessReg = 0.001  # smoothness regularizer weight for specular texture (at step 3)

		self.weightRoughnessSymmetryReg = 10.  # symmetry regularizer weight for roughness texture (at step 3)
		self.weightRoughnessConsistencyReg = 0.  # consistency regularizer weight for roughness texture (at step 3)
		self.weightRoughnessSmoothnessReg = 0.002  # smoothness regularizer weight for roughness texture (at step 3)

		self.debugFrequency = 10 #display frequency during optimization
		self.saveIntermediateStage = False #if True the output of stage 1 and 2 are saved. stage 3 is always saved which is the output of the optim
		self.verbose = False #display loss on terminal if true

		self.rtSamples = 500 #the number of ray tracer samples to render the final output
		self.rtTrainingSamples = 8  # number of ray tracing to use during training
	def fillFromDicFile(self, filePath):
		'''
		overwrite default config
		:param filePath: path to the new config file
		:return:
		'''

		print('loading optim config from: ', filePath)
		fp = open(filePath, 'r')
		assert(fp is not None)
		Lines = fp.readlines()
		fp.close()

		dic = {}

		for line in Lines:
			oLine = copy.copy(line)

			if line[0] == '#' or line[0] == '\n':
				continue
			if '#' in line:
				line = line[0:line.find('#')].strip().replace('\t', '').replace('\n', '')

			if len(line) < 1:
				continue

			keyval = line.split('=')
			if len(keyval) == 2:
				#assert (len(keyval) == 2)
				key = keyval[0].strip()
				val = keyval[1].strip()
				val = val.replace('"', '').replace("'", "").strip()
				dic[key] = val
			else:
				print('[warning] unknown key/val: ', oLine, file=sys.stderr, flush=True)

		for k, v in dic.items():
			aType = type(getattr(self, k)).__name__
			if aType == 'str':
				setattr(self, k, v)
			elif aType == 'bool':
				setattr(self, k, v.lower() == 'true')
			elif aType == 'int':
				setattr(self, k, int(v))
			elif aType == 'float':
				setattr(self, k, float(v))
			else:
				raise RuntimeError("unknown dictionary type: "+ key + "=>" + val)
	def print(self):
		dic = self.__dict__
		for key, val in dic.items():
			print(key, '=>', val)


```
