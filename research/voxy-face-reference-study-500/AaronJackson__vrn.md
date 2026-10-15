# Facial rendering source scan: AaronJackson/vrn

Repository: https://github.com/AaronJackson/vrn

Pinned source revision: ea5184770216e7c6f7ec2a1e1ccb0a17b5ecc4e6

Discovery topic: face reconstruction 3D

Description: :man:  Code for "Large Pose 3D Face Reconstruction from a Single Image via Direct Volumetric CNN Regression"

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {}

## Source file: vis.py

{
  "path": "vis.py",
  "blob_sha": "04b81acbca11a6758309f9a08213717939253183",
  "sha256": "8618f6337a5008e55b1f50434cc769177e9b6b8963f6b2a5cd6f7800401db734",
  "lines": 42,
  "bytes": 1231,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 4,
    "face": 1
  },
  "url": "https://github.com/AaronJackson/vrn/blob/ea5184770216e7c6f7ec2a1e1ccb0a17b5ecc4e6/vis.py",
  "function_signature_candidates": []
}

```text

import numpy as np
import visvis as vv
import argparse

parser = argparse.ArgumentParser(description='Visualise the 3D volume')
parser.add_argument('--image', dest='image',
                    help="The background image to display")
parser.add_argument('--volume', dest='volume',
                    help="The volume to render")
parser.add_argument('--texture', dest='texture',
                    help="Show textured mesh NOT YET IMPLEMENTED")

args = parser.parse_args()

vol = np.fromfile(args.volume, dtype=np.int8)
vol = vol.reshape((200,192,192))

im = vv.imread(args.image)

t = vv.imshow(im)
t.interpolate = True # interpolate pixels

# volshow will use volshow3 and rendering the isosurface if OpenGL
# version is >= 2.0. Otherwise, it will show slices with bars that you
# can move (much less useful).
volRGB = np.stack(((vol > 1) * im[:,:,0],
                   (vol > 1) * im[:,:,1],
                   (vol > 1) * im[:,:,2]), axis=3)

v = vv.volshow(volRGB, renderStyle='iso')
v.transformations[1].sz = 0.5 # Z was twice as deep during training

l0 = vv.gca()
l0.light0.ambient = 0.9 # 0.2 is default for light 0
l0.light0.diffuse = 1.0 # 1.0 is default

a = vv.gca()
a.camera.fov = 0 # orthographic

vv.use().Run()




```

## Source file: raw2obj.py

{
  "path": "raw2obj.py",
  "blob_sha": "c6cf4fe77cad6c1a80dc3dc868d6dbdb217963ce",
  "sha256": "19c54955b40b254799c10337517bdacdf27f84c93f30e91edc062a510cb1ee01",
  "lines": 50,
  "bytes": 1571,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 4,
    "face": 1
  },
  "url": "https://github.com/AaronJackson/vrn/blob/ea5184770216e7c6f7ec2a1e1ccb0a17b5ecc4e6/raw2obj.py",
  "function_signature_candidates": []
}

```text

import numpy as np
import scipy.misc
import mcubes
import argparse
from sklearn.neighbors import NearestNeighbors


parser = argparse.ArgumentParser(description='Visualise the 3D volume')
parser.add_argument('--image', dest='image',
                    help="The background image to display")
parser.add_argument('--volume', dest='volume',
                    help="The volume to render")
parser.add_argument('--obj', dest='obj',
                    help="The file path of the object")

args = parser.parse_args()

im = scipy.misc.imread(args.image, False, 'RGB')

vol = np.fromfile(args.volume, dtype=np.int8)
vol = vol.reshape((200,192,192))
vol = vol.astype(float)

vertices, triangles = mcubes.marching_cubes(vol, 10)
vertices = vertices[:,(2,1,0)]
vertices[:,2] *= 0.5 # scale the Z component correctly

r = im[:,:,0].flatten()
g = im[:,:,1].flatten()
b = im[:,:,2].flatten()

vcx,vcy = np.meshgrid(np.arange(0,192),np.arange(0,192))
vcx = vcx.flatten()
vcy = vcy.flatten()
vc = np.vstack((vcx, vcy, r, g, b)).transpose()
neigh = NearestNeighbors(n_neighbors=1)
neigh.fit(vc[:,:2])
n = neigh.kneighbors(vertices[:,(0,1)], return_distance=False)
colour = vc[n,2:].reshape((vertices.shape[0],3)).astype(float) / 255

vc = np.hstack((vertices, colour))

with open(args.obj, 'w') as f:
    for v in range(0,vc.shape[0]):
        f.write('v %0.2f %0.2f %0.2f %0.2f %0.2f %0.2f\n' % (vc[v,0],vc[v,1],vc[v,2],vc[v,3],vc[v,4],vc[v,5]))

    for t in range(0,triangles.shape[0]):
        f.write('f {} {} {}\n'.format(*triangles[t,:]+1))

print('Calculated the isosurface.')


```
