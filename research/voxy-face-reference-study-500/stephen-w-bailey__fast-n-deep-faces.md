# Facial rendering source scan: stephen-w-bailey/fast-n-deep-faces

Repository: https://github.com/stephen-w-bailey/fast-n-deep-faces

Pinned source revision: 53173c6367dfa3a20d3193ad7a0e77ac1e898f02

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE.txt"]

Topic paths: {"deform": ["cnnModel/rigidDeformer.py"], "lbs": ["compare/lbs.py"], "shader": ["render/shaders.py"], "rig": ["cnnModel/rigidDeformer.py", "computeRigidAttachment.py"], "face": ["fast-n-deep-faces.bib"]}

## Source file: cnnModel/rigidDeformer.py

{
  "path": "cnnModel/rigidDeformer.py",
  "blob_sha": "281938ece244066f17ec6208e31e5621af86ffab",
  "sha256": "9cbbe4052e18ccc87bab363ecb0679e70d43ccbef8420e4a9a61c1ab007c1b19",
  "lines": 117,
  "bytes": 3571,
  "selection_score": 30,
  "keyword_line_counts": {
    "deform": 4,
    "rig": 13
  },
  "url": "https://github.com/stephen-w-bailey/fast-n-deep-faces/blob/53173c6367dfa3a20d3193ad7a0e77ac1e898f02/cnnModel/rigidDeformer.py",
  "function_signature_candidates": [
    {
      "line": 5,
      "text": "def rigidRegister(source, target):"
    },
    {
      "line": 27,
      "text": "def rigidRegisterTF(source, target):"
    },
    {
      "line": 51,
      "text": "def __init__(self,neutral,rigidFiles,mask):"
    },
    {
      "line": 83,
      "text": "def deform(self,mesh):"
    },
    {
      "line": 95,
      "text": "def deformTF(self,mesh):"
    }
  ]
}

```text

import numpy as np
import pickle
import tensorflow as tf

def rigidRegister(source, target):

    # Normalize the data
    v = target
    p = source
    pMean = np.mean(p,0)
    vMean = np.mean(v,0)
    pp = p - pMean
    vp = v - vMean

    # Compute the rotation
    M = vp.T.dot(pp)
    u,sig,v = np.linalg.svd(M)
    sig = 1 / sig
    Qsqrt = u.dot(np.diag(sig).dot(u.T))
    R = Qsqrt.dot(M).T

    # Compute the translation
    t = (vMean - pMean.dot(R)).astype('float32')

    return R,t

def rigidRegisterTF(source, target):

    # Normalize the data
    v = target
    p = source
    pMean = tf.reduce_mean(p,-2)
    vMean = tf.reduce_mean(v,-2)
    pp = p - pMean[:,np.newaxis]
    vp = v - vMean[:,np.newaxis]

    # Compute the rotation
    M = tf.matmul(tf.transpose(vp,(0,2,1)),pp)
    sig,u,v = tf.linalg.svd(M)
    sig = 1 / sig
    Qsqrt = tf.matmul(u,sig[...,np.newaxis]*tf.transpose(u,(0,2,1)))
    R = tf.transpose(tf.matmul(Qsqrt,M),(0,2,1))

    # Compute the translation
    t = vMean[:,np.newaxis]-tf.matmul(pMean[:,np.newaxis],R)

    return R,t

class RigidDeformer:

    def __init__(self,neutral,rigidFiles,mask):

        # Load the data
        self.neutral = neutral
        self.mask = mask
        self.usedNeutral = neutral[mask]
        self.parts = []
        rank = []
        for f in rigidFiles:
            with open(f,'rb') as file:
                data = pickle.load(file)
            self.parts += data['parts']
            rank += data['rank']

        # Choose the vertices for each part
        n = 10
        self.rank = []
        for r in rank:
            if len(r) == len(self.neutral):
                r = r[mask]
            order = np.argsort(r)
            self.rank.append(order[:n])

        # Create a list of all vertices not in parts
        parts = []
        for p in self.parts:
            parts += [i for i in p]
        parts = np.asarray(parts)
        self.notParts = np.asarray([i for i in range(len(neutral)) if i not in parts])
        parts = np.concatenate([parts,self.notParts],0)
        self.partIndex = np.argsort(parts)

    def deform(self,mesh):

        mesh = mesh.copy()
        usedMesh = mesh[self.mask]
        for r,p in zip(self.rank,self.parts):
            target = usedMesh[r]
            neutral = self.usedNeutral[r]
            rigid = self.neutral[p]
            R,t = rigidRegister(neutral,target)
            mesh[p] = rigid.dot(R)+t
        return mesh

    def deformTF(self,mesh):

        with tf.variable_scope('rigid_deform'):
            usedMesh = tf.gather(mesh,self.mask,axis=0)
            parts = []
            partIndex = []
            rs = np.concatenate(self.rank,0)
            neutral = self.usedNeutral[rs].reshape((len(self.rank),-1,3))
            target = tf.reshape(tf.gather(usedMesh,rs,axis=0),(len(self.rank),-1,3))
            R,t = rigidRegisterTF(neutral,target)
            maxP = np.max([len(p) for p in self.parts])
            rigid = np.zeros((len(self.parts),maxP,3),dtype='float32')
            index = []
            for i,p in enumerate(self.parts):
                rigid[i,:len(p)] = self.neutral[p]
                index.append(np.stack((i*np.ones(len(p),dtype='int32'),np.arange(len(p)).astype('int32')),-1))
            index = np.concatenate(index,axis=0)
            parts = tf.matmul(rigid,R)+t
            parts = tf.gather_nd(parts,index)
            parts = (parts,tf.gather(mesh,self.notParts,axis=0))
            parts = tf.concat(parts,axis=0)
            mesh = tf.gather(parts,self.partIndex,axis=0)
        return mesh


```

## Source file: compare/lbs.py

{
  "path": "compare/lbs.py",
  "blob_sha": "9d239e89b8a7ee5f9c02a9adcb13a681abb9ad53",
  "sha256": "c1cafa7759d236f1f6851ca53a77dd91c7183b796eabf330174035ae4dd0639c",
  "lines": 143,
  "bytes": 4772,
  "selection_score": 15,
  "keyword_line_counts": {
    "gum": 11,
    "lbs": 5,
    "face": 4
  },
  "url": "https://github.com/stephen-w-bailey/fast-n-deep-faces/blob/53173c6367dfa3a20d3193ad7a0e77ac1e898f02/compare/lbs.py",
  "function_signature_candidates": [
    {
      "line": 17,
      "text": "def main():"
    }
  ]
}

```text

import argparse
import numpy as np
import os
import pickle
import scipy.io as sio
import sys
import tqdm
from pathlib import Path

basePath = (Path(__file__).parent / '..').resolve()
sys.path.append(os.path.join(basePath, '.'))

import errors
import PoseGenerator
import SSD

def main():
    parser = argparse.ArgumentParser(description='Use SSDRB to generate LBS weights')
    parser.add_argument('--cacheFile', type=str, required=True)
    parser.add_argument('--controlFile', type=str, required=True)
    parser.add_argument('--geoFile', type=str, required=True)
    parser.add_argument('--animFile', type=str)
    parser.add_argument('--sampleFile', type=str)
    parser.add_argument('--outputMesh', type=str)
    parser.add_argument('--sampleCount', type=int, default=1000)
    parser.add_argument('--bones', type=int, default=12)
    parser.add_argument('--k', type=int, default=4)
    parser.add_argument('--ssdFile', type=str)
    args = parser.parse_args()

    with open(args.cacheFile,'rb') as file:
        data = pickle.load(file)

    if args.sampleFile is not None and os.path.isfile(args.sampleFile):
        with open(args.sampleFile,'rb') as file:
            samples = pickle.load(file)
    else:
        samples = None

    active = data['active']
    default = data['neutral'][active]
    faces = data['faces']
    pg=PoseGenerator.PoseGeneratorRemote(args.controlFile,args.geoFile,'localhost',9001)
    pg.connect()
    mesh = []
    sampleMesh = []

    sampleCount = args.sampleCount * 2
    if args.animFile is not None and os.path.isfile(args.animFile):
        with open(args.animFile,'rb') as file:
            anim = pickle.load(file)
    else:
        np.random.seed(9001)
        anim = [pg.createRandomPose() for _ in range(sampleCount)]

    try:
        pg.setActiveVertices(active)
        print('Generating meshes')
        order = range(len(anim))
        for index in tqdm.tqdm(order):
            pose = anim[index]
            pg.setPose(pose)
            v = pg.getVertices()[active]
            mesh.append(v)
        if samples is not None:
            print('Generating samples')
            for pose in tqdm.tqdm(samples):
                pg.setPose(pose)
                v = pg.getVertices()[active]
                sampleMesh.append(v)
        else:
            sampleAnim = anim[sampleCount//2:]
            sampleMesh = mesh[sampleCount//2:]
            anim = anim[:sampleCount//2]
            mesh = mesh[:sampleCount//2]

    finally:
        pg.close()

    if args.sampleFile is not None and not os.path.isfile(args.sampleFile):
        with open(args.sampleFile,'wb') as file:
            pickle.dump(sampleAnim,file)
    if args.animFile is not None and not os.path.isfile(args.animFile):
        with open(args.animFile,'wb') as file:
            pickle.dump(anim,file)

    mesh = np.asarray(mesh)
    sampleMesh = np.asarray(sampleMesh)

    # Train model
    ssd = SSD.SSD()
    if args.ssdFile is None or not os.path.isfile(args.ssdFile):
        ssd.runSSD(default,sampleMesh,args.bones,args.k,faces=faces)
        if args.ssdFile is not None:
            with open(args.ssdFile,'wb') as file:
                out = dict(ssdWeights=ssd.weights,
                           ssdRestBones=ssd.restBones,
                           ssdRest=ssd.rest)
                pickle.dump(out,file)
    else:
        with open(args.ssdFile,'rb') as file:
            params = pickle.load(file)
        ssd.weights = params['ssdWeights']
        ssd.restBones = params['ssdRestBones']
        ssd.rest = params['ssdRest']

    # Evaluate model
    bones = np.repeat(ssd.restBones[np.newaxis],len(mesh),axis=0)
    error = ssd.getFitError(mesh,bones)
    prevError = error
    print('Initial error: '+str(error))
    for _ in range(20):
        bones = ssd.computeBones(mesh,bones)
        error = ssd.getFitError(mesh,bones)
        print('Error: '+str(error))
        if prevError-error < 1e-4:
            break
        else:
            prevError = error
    print('Final error: '+str(error))

    lbsMesh = []
    es = []
    normalErrors = []
    adjList = errors.buildFaceAdjList(faces)
    for i in tqdm.trange(len(bones)):
        v = ssd.computeMesh(bones[i])
        source = mesh[i]
        lbsMesh.append(ssd.computeMesh(bones[i]))
        ne = errors.normalError(source,v,faces)
        normalErrors.append(ne)
    diff = np.mean(np.sqrt(np.sum(np.square(lbsMesh-mesh),-1)),-1)
    print('Computed error: '+str(np.mean(diff)))
    print('Normal error: '+str(np.mean(normalErrors)))

    if args.outputMesh is not None:
        res = dict(neutral=data['neutral'],
                   active = data['active'])
        res['lbs'] = lbsMesh
        sio.savemat(args.outputMesh,res)

if __name__=='__main__':
    main()


```

## Source file: render/shaders.py

{
  "path": "render/shaders.py",
  "blob_sha": "80ea9290aa69526af8fb5b8f322b114988a9e3e3",
  "sha256": "31b2193fe925693cbfab34ea06b2aeddb9ca05e942bb61697b64b95ae45e48ce",
  "lines": 91,
  "bytes": 2267,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 15
  },
  "url": "https://github.com/stephen-w-bailey/fast-n-deep-faces/blob/53173c6367dfa3a20d3193ad7a0e77ac1e898f02/render/shaders.py",
  "function_signature_candidates": [
    {
      "line": 3,
      "text": "def createAndCompileShader(stype,source):"
    },
    {
      "line": 18,
      "text": "def initialize():"
    },
    {
      "line": 41,
      "text": "void main(void)"
    },
    {
      "line": 68,
      "text": "void main(void)"
    }
  ]
}

```text

from OpenGL import GL

def createAndCompileShader(stype,source):
    shader=GL.glCreateShader(stype)
    GL.glShaderSource(shader,source)
    GL.glCompileShader(shader)

    result=GL.glGetShaderiv(shader,GL.GL_COMPILE_STATUS)
    
    if (result!=1):
        raise Exception("Couldn't compile shader\nShader compilation Log:\n"+str(GL.glGetShaderInfoLog(shader)))
    return shader

vertex_shader = None
fragment_shader = None
program = None

def initialize():

    global vertex_shader
    global fragment_shader
    global program

    vertex_shader=createAndCompileShader(GL.GL_VERTEX_SHADER,"""
    #version 330 core

    layout(location = 0) in vec3 vertexPosition;
    layout(location = 1) in vec3 vertexNormal;
    layout(location = 2) in vec3 vertexColor;
    layout(location = 3) in float vertexVisible;

    out vec3 normal;
    out vec3 c;
    out vec3 position;
    out float visible;

    uniform mat4 MVP;
    uniform vec2 cop;
    uniform mat3 r;

    void main(void)
    {
        position = vertexPosition;
        gl_Position = MVP * vec4(vertexPosition,1);
        normal = vertexNormal;
        c = vertexColor;
        visible = vertexVisible;
    }
    """)

    fragment_shader=createAndCompileShader(GL.GL_FRAGMENT_SHADER,"""
    #version 330 core

    in vec3 position;
    in vec3 normal;
    in vec3 c;
    in float visible;

    out vec4 color;

    uniform vec3 amb;
    uniform vec3 direct;
    uniform vec3 directDir;
    uniform vec3 camT;
    uniform float specAng;
    uniform float specMag;

    void main(void)
    {
        vec3 n = normalize(normal);
        vec3 l = -normalize(directDir);
        float ref = max(0, dot(n,l));
        vec3 d = direct * ref;
        vec3 tex = d * c;

        // Compute shininess
        vec3 refDir = (2*dot(n,l)*n-l);
        vec3 camDiff = camT - position;
        vec3 H = normalize((l+normalize(camDiff)));
        camDiff = normalize(camDiff);
        float refDiff = max(0,dot(refDir,H));
        vec3 shine = direct * specMag * pow(refDiff,specAng);

        color = vec4(amb + visible * (tex + shine), 1.0);
    }
    """)

    program = GL.glCreateProgram()
    GL.glAttachShader(program, vertex_shader)
    GL.glAttachShader(program, fragment_shader)
    GL.glLinkProgram(program)


```

## Source file: computeRigidAttachment.py

{
  "path": "computeRigidAttachment.py",
  "blob_sha": "4c0813b754dbc739da4bb0a35c6d46b5a53656f6",
  "sha256": "bdd989393e061d76f17d31166f813d71ed2e647f379f336789a91aacf955232a",
  "lines": 162,
  "bytes": 5685,
  "selection_score": 15,
  "keyword_line_counts": {
    "gum": 4,
    "deform": 5,
    "rig": 47,
    "face": 1
  },
  "url": "https://github.com/stephen-w-bailey/fast-n-deep-faces/blob/53173c6367dfa3a20d3193ad7a0e77ac1e898f02/computeRigidAttachment.py",
  "function_signature_candidates": [
    {
      "line": 18,
      "text": "def modifyCaches(cacheFile, nonRigidParts):"
    },
    {
      "line": 37,
      "text": "def rankVertices(usedIndex,rigidIndex,meshes,neutral):"
    },
    {
      "line": 53,
      "text": "def computeRigidComponents(charts,meshes,neutral):"
    },
    {
      "line": 73,
      "text": "def main():"
    }
  ]
}

```text

import argparse
import matplotlib.pyplot as plt
import numpy as np
import os
import pickle
from skimage.transform import resize
import sys
import trimesh
import tqdm
import yaml
from pathlib import Path

basePath = (Path(__file__).parent).resolve()

import PoseGenerator
import SSD

def modifyCaches(cacheFile, nonRigidParts):
    with open(cacheFile, 'rb') as f:
        cacheData = pickle.load(f)
    cacheData['rigidCharts'] = cacheData['vCharts'].copy()
    rigidParts = [x for x in np.unique(cacheData['vCharts']) if x not in nonRigidParts]
    print("Non-Rigid Parts: ", nonRigidParts)
    print("Rigid Parts: ", rigidParts)
    for part in rigidParts:
        cacheData['vCharts'][cacheData['vCharts'] == part] = -1
    for part in nonRigidParts:
        cacheData['rigidCharts'][cacheData['rigidCharts'] == part] = -1
    with open(cacheFile, 'wb') as f:
        pickle.dump(cacheData, f)

    print('vCharts: ', dict(zip(*np.unique(cacheData['vCharts'], return_counts=True))))
    print('rigidCharts: ', dict(zip(*np.unique(cacheData['rigidCharts'], return_counts=True))))

    return rigidParts

def rankVertices(usedIndex,rigidIndex,meshes,neutral):

    # Compute the transformation of the rigid component
    rigidNeutral = neutral[rigidIndex]
    rigidMesh = meshes[:,rigidIndex]
    transformations = [SSD.rigidRegister(rigidNeutral,m) for m in rigidMesh]

    # Compute the mesh positions given the transofrmations
    usedNeutral = neutral[usedIndex]
    usedMesh = meshes[:,usedIndex]
    deformed = np.asarray([usedNeutral.dot(R)+t for R,t in transformations]).astype('float32')
    diff = np.sqrt(np.sum(np.square(usedMesh-deformed),-1))
    error = np.mean(diff,0)

    return error

def computeRigidComponents(charts,meshes,neutral):

    rigid = []
    nonrigid = []

    for i in range(np.max(charts)+1):
        mask = charts==i
        neutralPart = neutral[mask]
        meshesPart = meshes[:,mask]
        transformations = [SSD.rigidRegister(neutralPart,m) for m in meshesPart]
        deformed = np.asarray([neutralPart.dot(R)+t for R,t in transformations])
        diff = np.sum(np.square(meshesPart-deformed),-1)
        error = np.mean(diff)
        if error < 1e-6:
            rigid.append(i)
        else:
            nonrigid.append(i)

    return rigid,nonrigid

def main():
    parser = argparse.ArgumentParser(description='Train deformation approximation with CNN')
    parser.add_argument('--configFile', type=str, required=True)
    parser.add_argument('--outputFile', type=str, required=True)
    parser.add_argument('--nonRigidParts', nargs='+', type=int)
    args = parser.parse_args()
    with open(args.configFile) as file:
        config = yaml.load(file)

    with open(os.path.join(basePath,config['data_params']['cache_file']),'rb') as file:
        data = pickle.load(file)
    neutral = data['neutral']
    active = data['active']
    charts = data['vCharts']
    neutral = neutral[active]

    if 'rigidCharts' in data:
        print('Using rigid data from cache file')
        nonrigid = list(set([i for i in charts if i != -1]))
        rigidPartsIdx = list(set([i for i in data['rigidCharts'] if i != -1]))
        hasRigid = True
    else:
        hasRigid = False

    # Create the data pipeline
    print('Loading control file '+config['data_params']['control_file'])
    pg = PoseGenerator.PoseGeneratorRemote(os.path.join(basePath,config['data_params']['control_file']),os.path.join(basePath,config['data_params']['geo_file']),'localhost',9001)
    pg.connect()
    pg.setActiveVertices(active)

    # Generate some training data
    n = 250
    meshes = np.zeros((n,len(neutral),3),dtype='float32')
    try:
        for i in tqdm.trange(n):
            pg.setRandomPose()
            m = pg.getVertices()[active]
            meshes[i] = m
    finally:
        pg.close()

    # modify cache with information on which parts are rigid
    if not hasRigid:
        if args.nonRigidParts is None:
            print('Computing rigid components')
            rigid,nonrigid = computeRigidComponents(charts,meshes,neutral)
        else:
            nonrigid = args.nonRigidParts
        rigidPartsIdx = modifyCaches(os.path.join(basePath,config['data_params']['cache_file']), nonrigid)

    with open(os.path.join(basePath,config['data_params']['cache_file']),'rb') as file:
        data = pickle.load(file)
    neutral = data['neutral']
    active = data['active']
    neutral = neutral[active].astype('float32')
    neutralMean = np.mean(neutral,0)
    faces = data['faces']
    charts = data['vCharts']
    if 'sample_file' in config['data_params']:
        sampleFile = os.path.join(basePath,config['data_params']['sample_file'])
    else:
        sampleFile = None

    # Identify the used vertices
    usedV = np.arange(len(neutral))[charts>-1]

    # Identify the rigid vertices
    rigidCharts = data['rigidCharts']
    rigidMax = np.max(rigidCharts)+1
    rigidParts = []
    for i in range(rigidMax):
        if np.sum(rigidCharts==i):
            rigidParts.append(np.arange(len(neutral))[rigidCharts==i])
    print('Computing attachment for ' + str(len(rigidParts)) + ' parts')

    # Rank the vertices
    errors = [rankVertices(usedV,rp,meshes,neutral) for rp in rigidParts]
    for i in range(len(errors)):
        print('part '+str(rigidPartsIdx[i]))
        print('\tmin(errors): '+str(np.min(errors[i])))
        print('\tmax(errors): '+str(np.max(errors[i])))
        print('\terrors<0.1: '+str(np.sum(errors[i]<0.1)))

    # Save the results
    output = dict(parts=rigidParts,rank=errors)
    with open(args.outputFile,'wb') as file:
        pickle.dump(output,file)
    
if __name__=='__main__':
    main()


```
