# Facial rendering source scan: stevorackovic/FacialAnimation

Repository: https://github.com/stevorackovic/FacialAnimation

Pinned source revision: 8ad897581f5efb6b8010374318b804dbaed2ebe8

Discovery topic: facial animation

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"blendshape": ["Scripts/DataPreparation/ExtractBlendshapes.py", "Scripts/DataPreparation/ExtractCorrectiveBlendshapes.py"], "rig": ["Figures/InverseRigProblem.PNG"]}

## Source file: Scripts/DataPreparation/ExtractBlendshapes.py

{
  "path": "Scripts/DataPreparation/ExtractBlendshapes.py",
  "blob_sha": "9eee28c81f8d7003784889327920d35133d5076e",
  "sha256": "fe34c7983b1f45a5b275135e83df319ed4db92d1d876937e439178b26013f801",
  "lines": 77,
  "bytes": 3129,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 3,
    "teeth": 1,
    "face": 2
  },
  "url": "https://github.com/stevorackovic/FacialAnimation/blob/8ad897581f5efb6b8010374318b804dbaed2ebe8/Scripts/DataPreparation/ExtractBlendshapes.py",
  "function_signature_candidates": []
}

```text

# -*- coding: utf-8 -*-
"""
Created on Thu Aug 31 11:59:06 2023

@author: Stevo Rackovic

This script is to be executed in Autodesk Maya console, with a metahuman (or some other) avatar.
"""

N = 100   # Set here the number of frames of your animaiton
n = 72147 # Set here the number of vertices (times 3) of your avatar. The default value for metahumans is given here
data_dir = r'..\FacialAnimation\Data' # Put a path to your data directory

# -----------------------------------------------------------------------------
print('Extracting data with the following properties:')
print('Number of vertices in the mesh: ', int(n/3))
print('Number of the frames in the animation: ', N)

import os
import numpy as np
import pymel.core as pycore

bs_node = pycore.PyNode("head_lod0_mesh_blendShapes") 
neutral_mesh = pycore.PyNode("head_lod0_mesh") # mesh of the face - this might not be in a neutral position at the momment, so later we will have to set all the weights to 0 and obtain a real neutral face.
# Before running the next line, we need to select this node:
bs_node2 = pycore.PyNode("CTRL_expressions") 
names0 = [str(atr) for atr in pycore.listAttr(keyable=True)] # This should give over 200 names, so we need to check which ones produce zero offset (puppils) or are connected to the shoulder area.
m = len(names0)

## First we extract weights for each frame:
W = np.zeros((N,m)) # number of frames we have in this model
for i in range(N):
    pycore.currentTime(i)
    W[i]=[bs_node2.getAttr(nm) for nm in names0]
    
# Now we can also remove the controllers with zero offset:
W_offset = np.mean(W,axis=0)
W = W[:,W_offset>0]
names = []
for i in range(len(W_offset)):
    if W_offset[i] > 0:
        names.append(names0[i]) # this yields somewhere under 150 controllers for metahumans
m = len(names)
print('Number of blendshapes: ', m)
        
# Now go back to the first frame and set all weights to zero, to extract the neutral face
pycore.currentTime(0)
for nm in names0:
    bs_node2.setAttr(nm,0)
neutral_verts = np.array([[x, y, z] for x, y, z in neutral_mesh.getPoints()], dtype=np.float64).flatten()

# Extract each (delta) blendshape:
# The idea is to activate a single controller to 1 (or whatever is the max value) and all the rest to zero, and just take vertices
deltas = []
for nm in names:
    bs_node2.setAttr(nm,1)
    shape_points = np.array([[x, y, z] for x, y, z in neutral_mesh.getPoints()], dtype=np.float64).flatten()
    shape_delta = shape_points - neutral_verts
    deltas.append(shape_delta)
    bs_node2.setAttr(nm,0)
deltas = np.array(deltas)

# Now remove all those that do not affest the mesh (they are ususally connected only to pupils or teeth)
D_offset = np.sum(np.abs(deltas),axis=1)
deltas = deltas[D_offset>0]
names1 = []
for i in range(len(D_offset)):
    if D_offset[i] > 0:
        names1.append(names[i]) # this yields 130 controllers
W = W[:,D_offset>0]

np.save(os.path.join(data_dir,'weights.npy'),W)
np.save(os.path.join(data_dir,'deltas.npy'),deltas.T)
np.save(os.path.join(data_dir,'neutral.npy'),neutral_verts)

print('Data extracted successfully, stored at ', data_dir)



```

## Source file: Scripts/DataPreparation/ExtractCorrectiveBlendshapes.py

{
  "path": "Scripts/DataPreparation/ExtractCorrectiveBlendshapes.py",
  "blob_sha": "7e83a6353d103dea4c68099fb0096fc8f16cdbf1",
  "sha256": "947edd3e3f4c57b1e4b1ebba88786f09e62f8e77c36eaba55c860ced1b5bc17a",
  "lines": 207,
  "bytes": 7665,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 2
  },
  "url": "https://github.com/stevorackovic/FacialAnimation/blob/8ad897581f5efb6b8010374318b804dbaed2ebe8/Scripts/DataPreparation/ExtractCorrectiveBlendshapes.py",
  "function_signature_candidates": []
}

```text

# -*- coding: utf-8 -*-
"""
Created on Thu Aug 31 13:16:54 2023

@author: Stevo Rackovic

This script is to be executed in Autodesk Maya console, with a metahuman (or some other) avatar.
"""

data_dir = r'..\FacialAnimation\Data' # Put a path to your data directory
m = 130  # Put the number of your character blendhsapes
m2 = 642 # Put a number of all the controllers under the blendshape node
names = [] # copy the names from a text file here 

# -----------------------------------------------------------------------------

import numpy as np
import pymel.core as pycore
import os

bs_node = pycore.PyNode("head_lod0_mesh_blendShapes") 
neutral_mesh = pycore.PyNode("head_lod0_mesh")
# Before runing the next line, I need to select controllers node:
bs_node2 = pycore.PyNode("CTRL_expressions") 

deltas = np.load(os.path.join(data_dir,'deltas.npy')).T
neutral = np.load(os.path.join(data_dir,'neutral.npy'))

for nm in names:
    bs_node2.setAttr(nm,0)
    
### First level corrections:

first_level = {}
for i in range(m-1):
    bs_node2.setAttr(names[i],.5)
    for j in range(i+1,m):
        bs_node2.setAttr(names[j],.5)
        candidates = []
        c_values = []
        for c_shape in range(m2):
            if bs_node.w[c_shape].get() == .25:
                candidates.append(c_shape)
                c_values.append(bs_node.w[c_shape].get())
        if len(candidates) > 0:
            first_level[(i,j)] = [candidates, c_values]
        bs_node2.setAttr(names[j],0)
    bs_node2.setAttr(names[i],0)
    
fli = list(first_level.items())
keys = np.array([fli[i][0] for i in range(len(fli))])
corr_keys = np.array([fli[i][1][0] for i in range(len(fli))])

corr_shapes = []
for i in range(len(fli)):
    k1,k2 = keys[i][0],keys[i][1]
    bs_node2.setAttr(names[k1],1)    
    bs_node2.setAttr(names[k2],1)
    pred = neutral + deltas[k1] + deltas[k2]
    true = np.array([[x, y, z] for x, y, z in neutral_mesh.getPoints()], dtype=np.float64).flatten()
    corr_shapes.append(true-pred)
    bs_node2.setAttr(names[k1],0)    
    bs_node2.setAttr(names[k2],0)
    
### Second level corrections:

second_level = {}
for i in range(len(fli)):
    k1 = fli[i][0][0]
    k2 = fli[i][0][1]
    bs_node2.setAttr(names[k1],.5)
    bs_node2.setAttr(names[k2],.5)
    for j in range(m):
        if j != k1 and j!= k2:
            bs_node2.setAttr(names[j],.5)
            candidates = []
            c_values = []
            for c_shape in range(m2):
                if bs_node.w[c_shape].get() == .125:
                    candidates.append(c_shape)
                    c_values.append(bs_node.w[c_shape].get())
            if len(candidates) > 0:
                second_level[tuple(sorted((k1,k2,j)))] = [candidates, c_values]
            bs_node2.setAttr(names[j],0)
    bs_node2.setAttr(names[k1],0)    
    bs_node2.setAttr(names[k2],0)
    
sli = list(second_level.items())
keys2 = np.array([sli[i][0] for i in range(len(sli))])
corr_keys2 = np.array([sli[i][1][0] for i in range(len(sli))])
 
corr_shapes2 = []
for i in range(len(sli)):
    k1,k2,k3 = keys2[i][0],keys2[i][1],keys2[i][2]
    bs_node2.setAttr(names[k1],1)    
    bs_node2.setAttr(names[k2],1)
    bs_node2.setAttr(names[k3],1)
    pred = neutral + deltas[k1] + deltas[k2] + deltas[k3]
    if (k1,k2) in first_level:
        idx = list(first_level.keys()).index((k1,k2))
        pred += corr_shapes[idx]
    if (k1,k3) in first_level:
        idx = list(first_level.keys()).index((k1,k3))
        pred += corr_shapes[idx]
    if (k2,k3) in first_level:
        idx = list(first_level.keys()).index((k2,k3))
        pred += corr_shapes[idx]
    true = np.array([[x, y, z] for x, y, z in neutral_mesh.getPoints()], dtype=np.float64).flatten()
    corr_shapes2.append(true-pred)
    bs_node2.setAttr(names[k1],0)    
    bs_node2.setAttr(names[k2],0)
    bs_node2.setAttr(names[k3],0)
    
### Third level corrections:

# Likewise, I asume that only that the only candidates are those triplets that already appeared in the second level corrections.
third_level = {}
for i in range(len(sli)):
    k1 = sli[i][0][0]
    k2 = sli[i][0][1]
    k3 = sli[i][0][2]
    bs_node2.setAttr(names[k1],.5)
    bs_node2.setAttr(names[k2],.5)
    bs_node2.setAttr(names[k3],.5)
    for j in range(m):
        if j != k1 and j!= k2 and j!=k3:
            bs_node2.setAttr(names[j],.5)
            candidates = []
            c_values = []
            for c_shape in range(m2):
                if bs_node.w[c_shape].get() == .0625:
                    candidates.append(c_shape)
                    c_values.append(bs_node.w[c_shape].get())
            if len(candidates) > 0:
                third_level[tuple(sorted((k1,k2,k3,j)))] = [candidates, c_values]
            bs_node2.setAttr(names[j],0)
    bs_node2.setAttr(names[k1],0)    
    bs_node2.setAttr(names[k2],0)
    bs_node2.setAttr(names[k3],0)
    
tli = list(third_level.items())
keys3 = np.array([tli[i][0] for i in range(len(tli))])
corr_keys3 = np.array([tli[i][1][0] for i in range(len(tli))])

corr_shapes3 = []
for i in range(len(tli)):
    k1,k2,k3,k4 = keys3[i][0],keys3[i][1],keys3[i][2],keys3[i][3]
    bs_node2.setAttr(names[k1],1)    
    bs_node2.setAttr(names[k2],1)
    bs_node2.setAttr(names[k3],1)
    bs_node2.setAttr(names[k4],1)
    pred = neutral + deltas[k1] + deltas[k2] + deltas[k3] + deltas[k4]
    if (k1,k2) in first_level:
        idx = list(first_level.keys()).index((k1,k2))
        pred += corr_shapes[idx]
    if (k1,k3) in first_level:
        idx = list(first_level.keys()).index((k1,k3))
        pred += corr_shapes[idx]
    if (k1,k4) in first_level:
        idx = list(first_level.keys()).index((k1,k4))
        pred += corr_shapes[idx]
    if (k2,k3) in first_level:
        idx = list(first_level.keys()).index((k2,k3))
        pred += corr_shapes[idx]
    if (k2,k4) in first_level:
        idx = list(first_level.keys()).index((k2,k4))
        pred += corr_shapes[idx]
    if (k3,k4) in first_level:
        idx = list(first_level.keys()).index((k3,k4))
        pred += corr_shapes[idx]
    if (k1,k2,k3) in second_level:
        idx = list(second_level.keys()).index((k1,k2,k3))
        pred += corr_shapes2[idx]
    if (k1,k2,k4) in second_level:
        idx = list(second_level.keys()).index((k1,k2,k4))
        pred += corr_shapes2[idx]
    if (k1,k3,k4) in second_level:
        idx = list(second_level.keys()).index((k1,k3,k4))
        pred += corr_shapes2[idx]
    if (k2,k3,k4) in second_level:
        idx = list(second_level.keys()).index((k2,k3,k4))
        pred += corr_shapes2[idx]
    true = np.array([[x, y, z] for x, y, z in neutral_mesh.getPoints()], dtype=np.float64).flatten()
    corr_shapes3.append(true-pred)
    bs_node2.setAttr(names[k1],0)    
    bs_node2.setAttr(names[k2],0)
    bs_node2.setAttr(names[k3],0)
    bs_node2.setAttr(names[k4],0)
    
### We assume there are no higher levels of corrections

### Save the extracted corrective terms and their indices
corr_shapes = np.array(corr_shapes)
corr_shapes2 = np.array(corr_shapes2)
corr_shapes3 = np.array(corr_shapes3)
np.save(os.path.join(data_dir,'bs1.npy'),corr_shapes)
np.save(os.path.join(data_dir,'bs2.npy'),corr_shapes2)
np.save(os.path.join(data_dir,'bs3.npy'),corr_shapes3)
np.save(os.path.join(data_dir,'keys1.npy'),keys)
np.save(os.path.join(data_dir,'keys2.npy'),keys2)
np.save(os.path.join(data_dir,'keys3.npy'),keys3)

print('Number of corrective terms of the first level: ', len(corr_shapes))
print('Number of corrective terms of the second level: ', len(corr_shapes2))
print('Number of corrective terms of the third level: ', len(corr_shapes3))
print('Data extracted successfully, stored at ', data_dir)



```

## Source file: Scripts/LMMM/Execute.py

{
  "path": "Scripts/LMMM/Execute.py",
  "blob_sha": "cbe404244be3965e4c0ec0f9e6aa5e1ffc1fb043",
  "sha256": "38f642109adf3ee0a4e9117616aaffb89692d7281c8c9e8eedb6b7dab4bfa732",
  "lines": 83,
  "bytes": 4046,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 2,
    "blendshape": 1,
    "rig": 2
  },
  "url": "https://github.com/stevorackovic/FacialAnimation/blob/8ad897581f5efb6b8010374318b804dbaed2ebe8/Scripts/LMMM/Execute.py",
  "function_signature_candidates": []
}

```text

# -*- coding: utf-8 -*-
"""
Created on Wed Oct  4 17:15:32 2023

@author: Stevo Rackovic

"""

train_frames = 10           # this will take the first 'train_frames' from 'weights.npy' matrix as a training set
num_iter = 200              # the number of iterations of the CD solver
lmbd =  5                   # the regularization parameter of the objective funciton
tolerance = 0.0005          # stopping criteria (if the norm of the difference of the two consecutive vectors i s less than tolerance, break)
n_batches = 40              # for parallel computing within the function for computing the terms and coefficients of the surogate
initialization = 'CO'       # Initialization method. If 'CO', use the speudoinverse warm start of Cetinaslan aand Orvalho. If '0', use zero-initialization

# -----------------------------------------------------------------------------
import numpy as np
import os
from LMMMFunctions import minimization
work_dir = os.path.dirname(os.path.dirname(os.getcwd()))
data_dir = os.path.join(work_dir,'Data')
predictions_dir = os.path.join(work_dir,'Predictions\LMMM')
from HelperFunctions import quartic_rig

neutral = np.load(os.path.join(data_dir,'neutral.npy'))
deltas = np.load(os.path.join(data_dir,'deltas.npy'))
n,m = deltas.shape
bs1  = np.load(os.path.join(data_dir,'bs1.npy'))
bs2  = np.load(os.path.join(data_dir,'bs2.npy'))
bs3  = np.load(os.path.join(data_dir,'bs3.npy'))
keys1  = np.load(os.path.join(data_dir,'keys1.npy'))
keys2  = np.load(os.path.join(data_dir,'keys2.npy'))
keys3  = np.load(os.path.join(data_dir,'keys3.npy'))
eig_max_D = np.load(os.path.join(data_dir,'eigen_max.npy'))   # the largest eigenvalues
eig_min_D = np.load(os.path.join(data_dir,'eigen_min.npy'))   # the smallest eigenvalues
sigma_D   = np.load(os.path.join(data_dir,'singular.npy'))    # the largest singular values

print('Dimensions of the data:')
print('Number of vertices in the mesh: ', int(n/3))
print('Number of blendshapes: ', m)
print('Number of corrective terms of the first level: ', len(keys1))
print('Number of corrective terms of the second level: ', len(keys2))
print('Number of corrective terms of the third level: ', len(keys3))

weights = np.load(os.path.join(data_dir,'weights.npy'))
weights_train = weights[:train_frames]
N = len(weights_train)
target_meshes = np.array([quartic_rig(C, deltas, bs1, bs2, bs3, keys1, keys2, keys3) for C in weights_train])
print('Total animation frames: ', len(weights))
print('Training animation frames: ', len(weights_train))
print('')

if initialization == 'CO':
    # Solution with warm-start (initializaed at the pseudoinverse solution of Cetinaslan and Orvalho)
    print('Solving for CO initialization')
    Bpsd = (np.linalg.inv(deltas.T.dot(deltas) + lmbd*np.eye(m))).dot(deltas.T)
    Predictions = []
    for frame in range(N):
        target_mesh = target_meshes[frame]
        x0 = Bpsd.dot(target_mesh)
        x0[x0<0] = 0
        x0[x0>1] = 1
        res = minimization(num_iter,x0,deltas,target_mesh,eig_max_D,eig_min_D,sigma_D,lmbd,bs1,keys1,n_batches)
        Predictions.append(res)
    filename = f'Pred_initialization_{initialization}_lmbd_{lmbd}.npy'
    np.save(os.path.join(predictions_dir, filename), np.array(Predictions)) 
    print('')
    print('Predictions made and stored at ', predictions_dir)
    
if initialization == '0':
    # Solution with warm-start (initializaed at the pseudoinverse solution of Cetinaslan and Orvalho)
    print('Solving for 0-initialization')
    Bpsd = (np.linalg.inv(deltas.T.dot(deltas) + lmbd*np.eye(m))).dot(deltas.T)
    Predictions = []
    for frame in range(N):
        target_mesh = target_meshes[frame]
        x0 = np.zeros(m)
        res = minimization(num_iter,x0,deltas,target_mesh,eig_max_D,eig_min_D,sigma_D,lmbd,bs1,keys1,n_batches)
        Predictions.append(res)
    filename = f'Pred_initialization_{initialization}_lmbd_{lmbd}.npy'
    np.save(os.path.join(predictions_dir, filename), np.array(Predictions)) 
    print('')
    print('Predictions made and stored at ', predictions_dir)

```

## Source file: Scripts/HelperFunctions.py

{
  "path": "Scripts/HelperFunctions.py",
  "blob_sha": "8ac3dd3185510361bf37d72f0555deee36439e2d",
  "sha256": "8db657628936ae2518e85cddb5ecc5c82c5cd16b5a3ceddd0ec2ac2ec8db8616",
  "lines": 281,
  "bytes": 10265,
  "selection_score": 0,
  "keyword_line_counts": {
    "blendshape": 26,
    "rig": 6
  },
  "url": "https://github.com/stevorackovic/FacialAnimation/blob/8ad897581f5efb6b8010374318b804dbaed2ebe8/Scripts/HelperFunctions.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def quartic_rig(C, deltas, bs1, bs2, bs3, keys1, keys2, keys3):"
    },
    {
      "line": 41,
      "text": "def quadratic_rig(C,deltas,bs1,keys1):"
    },
    {
      "line": 61,
      "text": "def objective_function(C,deltas,bs1,keys1,target_mesh,lmbd):"
    },
    {
      "line": 89,
      "text": "def rmse(mesh1,mesh2,percentile=95):"
    },
    {
      "line": 109,
      "text": "def error_cardinality(path, file_name, target_meshes, deltas, bs1, bs2, bs3, keys1, keys2, keys3):"
    },
    {
      "line": 172,
      "text": "def optimal_w(s,b):"
    },
    {
      "line": 183,
      "text": "def optimal_C(deltas,msh,order,m):"
    },
    {
      "line": 191,
      "text": "def ctr_order(deltas):"
    },
    {
      "line": 200,
      "text": "def compute_error_density(vrtcs_list,ctr_list,coord_list,deltas):"
    },
    {
      "line": 255,
      "text": "def compute_density(vrtcs_list,AssignmentMatrix):"
    }
  ]
}

```text

# -*- coding: utf-8 -*-
"""
Created on Thu Aug 31 14:07:51 2023

@author: Stevo Rackovic
"""

import os
import numpy as np

def quartic_rig(C, deltas, bs1, bs2, bs3, keys1, keys2, keys3):
    '''
    Computes a quartic rig given a weight vector C.

    Parameters
    ----------
    C : np.array(m)
        Vector of the controller activation weights, where m is the number of 
        blendshapes.
    deltas : np.array(n,m)
        Deltas blendshape matrix.
    bs1 : np.array(m1,n)
        Matrix containing (m1) corrective terms of the first level.
    bs2 : np.array(m2,n)
        Matrix containing (m2) corrective terms of the second level.
    bs3 : np.array(m3,n)
        Matrix containing (m3) corrective terms of the third level.
    keys1 : np.array(m1,2) with int entries
        Each row corresponds to the indices of the blendshapes that invoke the 
        correspodning corrective term from bs1.
    keys2 : np.array(m2,2) with int entries
        Each row corresponds to the indices of the blendshapes that invoke the 
        correspodning corrective term from bs2.
    keys3 : np.array(m3,2) with int entries
        Each row corresponds to the indices of the blendshapes that invoke the 
        correspodning corrective term from bs3.

    '''
    return deltas.dot(C) + bs1.T.dot(C[keys1].prod(1)) + bs2.T.dot(C[keys2].prod(1)) + bs3.T.dot(C[keys3].prod(1))

def quadratic_rig(C,deltas,bs1,keys1):
    '''
    Computes a quadratic rig (approximation) given a weight vector C.

    Parameters
    ----------
    C : np.array(m)
        Vector of the controller activation weights, where m is the number of 
        blendshapes.
    deltas : np.array(n,m)
        Deltas blendshape matrix.
    bs1 : np.array(m1,n)
        Matrix containing (m1) corrective terms of the first level.
    keys1 : np.array(m1,2) with int entries
        Each row corresponds to the indices of the blendshapes that invoke the 
        correspodning corrective term from bs1.

    '''
    return deltas.dot(C) + bs1.T.dot(C[keys1[:,0]]*C[keys1[:,1]])

def objective_function(C,deltas,bs1,keys1,target_mesh,lmbd):
    '''
    Gives a value of the objective cost. 

    Parameters
    ----------
    C : np.array(m)
        Vector of the controller activation weights, where m is the number of 
        blendshapes.
    deltas : np.array(n,m)
        Deltas blendshape matrix.
    bs1 : np.array(m1,n)
        Matrix containing (m1) corrective terms of the first level.
    keys1 : np.array(m1,2) with int entries
        Each row corresponds to the indices of the blendshapes that invoke the 
        correspodning corrective term from bs1.
    target_mesh : np.array(n)
        Ground-truth mesh vector.
    lmbd : float
        Regularization aprameter. lambda>=0.

    '''
    pred_mesh = quadratic_rig(C,deltas,bs1,keys1)
    return np.linalg.norm(pred_mesh-target_mesh)**2 + lmbd*np.sum(C)

# -----------------------------------------------------------------------------
# Functions for evaluating the results

def rmse(mesh1,mesh2,percentile=95):
    '''
    Computes root squared error between predicted and ground-truth meshes, and 
    returns mean, max, and percentile value of the obtained vector.

    Parameters
    ----------
    mesh1 : np.array(n)
        Predicted mesh vector, with n elements.
    mesh2 : np.array(n)
        Ground-truth mesh vector.
    percentile : int, optional
        Percentile value of the error vector to return. 0<=percentile<=100. 
        The default is 95.

    '''
    error = (mesh1[::3]-mesh2[::3])**2 + (mesh1[1::3]-mesh2[1::3])**2 + (mesh1[2::3]-mesh2[2::3])**2
    error = np.sqrt(error)
    return np.mean(error), np.max(error), np.percentile(error,percentile)

def error_cardinality(path, file_name, target_meshes, deltas, bs1, bs2, bs3, keys1, keys2, keys3):
    '''
    Computes the evaluation metrics; inn specific mesh error (mean and max), 
    cardinality (i.e., L0 norm) of the weight vector, L1 norm of the weight 
    vector as well as roughness penalty per blendhspape (inversely proportional 
    to temporal smoothness).

    Parameters
    ----------
    path : str
        Path to the predictions diectory.
    file_name : str
        Name of the predictions file.
    target_meshes : np.array(N,m)
        matrix contatining predicted weights, where N is the numebr of frames 
        and m is the nuber of blendshapes.
    deltas : np.array(n,m)
        Deltas blendshape matrix.
    bs1 : np.array(m1,n)
        Matrix containing (m1) corrective terms of the first level.
    bs2 : np.array(m2,n)
        Matrix containing (m2) corrective terms of the second level.
    bs3 : np.array(m3,n)
        Matrix containing (m3) corrective terms of the third level.
    keys1 : np.array(m1,2) with int entries
        Each row corresponds to the indices of the blendshapes that invoke the 
        correspodning corrective term from bs1.
    keys2 : np.array(m2,2) with int entries
        Each row corresponds to the indices of the blendshapes that invoke the 
        correspodning corrective term from bs2.
    keys3 : np.array(m3,2) with int entries
        Each row corresponds to the indices of the blendshapes that invoke the 
        correspodning corrective term from bs3.

    Returns
    -------
    Error_mean : float
        Average mesh error.
    Error_max : float
        Maximal mesh error.
    Cardinality : float
        Cardinality of the predicted weight vector (L0 norm).
    L1norm : float
        L1 norm of the predicted weigth vector.

    '''
    Ypred = np.load(os.path.join(path, file_name))
    Ypred[Ypred<0]=0
    Ypred[Ypred>1]=1
    Xpred = np.array([quartic_rig(Ypred[frame], deltas, bs1, bs2, bs3, keys1, keys2, keys3) for frame in range(len(Ypred))])
    err0 = [rmse(Xpred[frame],target_meshes[frame])[0] for frame in range(len(target_meshes))]
    err1 = [rmse(Xpred[frame],target_meshes[frame])[1] for frame in range(len(target_meshes))]
    Error_mean = np.mean(err0)
    Error_max = np.mean(err1)
    crd = [np.sum(Ypred[frame]>0) for frame in range(len(Ypred))]
    Cardinality = np.mean(crd)
    nrm = [np.linalg.norm(Ypred[frame],1) for frame in range(len(Ypred))]
    L1norm = np.mean(nrm)
    return Error_mean, Error_max, Cardinality, L1norm

# -----------------------------------------------------------------------------
# Functions for SSKLN:

def optimal_w(s,b):
    w = s.dot(b)/(b.dot(b))
    if w<=1 and w>=0:
        return w
    else:
        obj_0, obj_1 = np.linalg.norm(s), np.linalg.norm(s-b)
        if obj_1 < obj_0:
            return 1
        else:
            return 0
        
def optimal_C(deltas,msh,order,m):
    C = np.zeros(m)
    for i in order:
        b = deltas[:,i]
        C[i] += optimal_w(msh,b)
        msh = msh - C[i]*b
    return C

def ctr_order(deltas):
    offset = deltas[::3]**2 + deltas[1::3]**2 + deltas[2::3]**2
    offset = np.sum(offset,0)
    order  = np.argsort(-offset)
    return order

# -----------------------------------------------------------------------------
# Functions for clustering

def compute_error_density(vrtcs_list,ctr_list,coord_list,deltas):
    '''
    Computes the reconstruction error and density of the clustered deltas 
    blendshape matrix.

    Parameters
    ----------
    vrtcs_list : list
        Each element of this list is a list itself, filled up with integers, 
        that represent indices of the vertices belonging to a specific cluster.
    ctr_list : list
        Each element of this list is a list itself, filled up with integers, 
        that represent indices of the blendshapes belonging to a specific cluster.
    coord_list : list
        Each element of this list is a list itself, filled up with integers,
        corresponding to x,y and z coordinates of vertices in vrtcs_list.
    deltas : np.array(n,m)
        Deltas blendshape matrix.
        
    Returns
    -------
    ReconstructionError : float
        Reconstruction error of the clustered blendshape matrix.    
    Density : float
        Density of the clustered blendshape matrix.
    AssignmentMatrix : np.array(no. of blendshapes, no. of clusters)
        A binary matrix that specifies which controller is assigned to what cluster.
    Error : float
        Reconstruction error of the clustered blendshape matrix, without taking 
        into consideration kept information.

    '''
    num_clusters = len(vrtcs_list)
    n,m = deltas.shape
    Error        = 0.
    Information  = 0.
    Density      = 0.
    AssignmentMatrix = np.zeros((num_clusters,m))
    for clstr in range(num_clusters):
        ctr_clst = ctr_list[clstr]
        ctr_clst_cmp = [i for i in range(m) if i not in ctr_clst]
        vtx_clstr = vrtcs_list[clstr]
        coord_clstr = coord_list[clstr]
        dlt_clstr = deltas[coord_clstr][:,ctr_clst] # - Submatrix correspodning to the cluster 'clstr'
        dlt_clstr_cmp = deltas[coord_clstr][:,ctr_clst_cmp] # - A discarded submatrix of the cluster 'clstr'
        info = np.sum(dlt_clstr**2)
        err  = np.sum(dlt_clstr_cmp**2)
        AssignmentMatrix[clstr][ctr_clst] += 1
        Error += err
        Information += info
        dns = len(vtx_clstr) * len(ctr_clst)
        Density += dns
    ReconstructionError = Error/Information
    return ReconstructionError, Density, AssignmentMatrix.T, Error
    
def compute_density(vrtcs_list,AssignmentMatrix):
    '''
    Computes the inter-density of the clustered deltas blendshape matrix.

    Parameters
    ----------
    vrtcs_list : list
        Each element of this list is a list itself, filled up with integers, 
        that represent indices of the vertices belonging to a specific cluster.
    AssignmentMatrix : np.array(no. of blendshapes, no. of clusters)
        A binary matrix that specifies which controller is assigned to what cluster.        
        
    Returns
    ------- 
    InterDensity : float
        Inter-Density of the clustered blendshape matrix.    

    '''
    m,num_clusters = AssignmentMatrix.shape
    InterDensity = 0
    for ctr in range(m):
        Mrow = AssignmentMatrix[ctr]
        if np.sum(Mrow) > 1:
            for clstr in range(num_clusters):
                InterDensity += len(vrtcs_list[clstr])*Mrow[clstr]
    return InterDensity
        


```
