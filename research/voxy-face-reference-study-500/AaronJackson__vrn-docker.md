# Facial rendering source scan: AaronJackson/vrn-docker

Repository: https://github.com/AaronJackson/vrn-docker

Pinned source revision: b0cab65a50fad0d1a19d334598f8c27fbe47911c

Discovery topic: face reconstruction 3D

Description: :whale: Docker files for Large Pose 3D Face Reconstruction from a Single Image via Direct Volumetric CNN Regression 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"face": ["runner/face-alignment/LICENCE", "runner/face-alignment/README.md", "runner/face-alignment/facedetection.lua", "runner/face-alignment/facedetection_dlib.lua", "runner/face-alignment/main.lua", "runner/face-alignment/opts.lua", "runner/face-alignment/utils.lua"]}

## Source file: runner/raw2obj.py

{
  "path": "runner/raw2obj.py",
  "blob_sha": "a01c7e1eee53e70306378ca546489f3027b76524",
  "sha256": "8bf99f40d3a3af1a034cb3b87c85702cfc297997ddfba8d790c0da7013f51cfc",
  "lines": 56,
  "bytes": 1732,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 4,
    "face": 2
  },
  "url": "https://github.com/AaronJackson/vrn-docker/blob/b0cab65a50fad0d1a19d334598f8c27fbe47911c/runner/raw2obj.py",
  "function_signature_candidates": []
}

```text

import numpy as np
import scipy.misc
import mcubes
import argparse
from sklearn.neighbors import NearestNeighbors
import trimesh

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
vol = vol.astype(bool)

vertices, triangles = mcubes.marching_cubes(vol, 0.9)
vertices = vertices[:,(2,1,0)]
vertices[:,2] *= 0.5 # scale the Z component correctly

mesh = trimesh.Trimesh(vertices, triangles)
trimesh.constants.tol.merge = 1
mesh.merge_vertices()
vertices = mesh.vertices
triangles = mesh.faces

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
        f.write('v %0.0f %0.0f %0.0f %0.2f %0.2f %0.2f\n' % (vc[v,0],vc[v,1],vc[v,2],vc[v,3],vc[v,4],vc[v,5]))

    for t in range(0,triangles.shape[0]):
        f.write('f {} {} {}\n'.format(*triangles[t,:]+1))

print('Calculated the isosurface.')


```
