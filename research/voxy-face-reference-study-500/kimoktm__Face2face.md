# Facial rendering source scan: kimoktm/Face2face

Repository: https://github.com/kimoktm/Face2face

Pinned source revision: a700555ede40890c1714879f6f50442342871750

Discovery topic: face model 3D

Description: A python library to to fit 3D morphable models to images of faces and capture facial performance overtime with no markers or a special mount

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"face": ["cli/openFaceLandmarks.py", "cli/processFacewarehouse.py", "cli/textureFace.py", "face2face/__init__.py", "face2face/models.py", "face2face/optimize/__init__.py", "face2face/optimize/camera.py", "face2face/optimize/derivative.py", "face2face/optimize/image.py", "face2face/utils/__init__.py", "face2face/utils/io.py", "face2face/utils/mesh.py", "face2face/utils/opengl.py", "face2face/utils/transform.py"]}

## Source file: cli/textureFace.py

{
  "path": "cli/textureFace.py",
  "blob_sha": "21439cfd7f99739fabe7e5e1999cb61b1042047e",
  "sha256": "2b0584d4d39e879a1d6175d37d670e822bbb2b3527ddf1c268ec2fef8b30447e",
  "lines": 221,
  "bytes": 7701,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 1,
    "jaw": 3,
    "gum": 4,
    "rig": 2,
    "face": 23
  },
  "url": "https://github.com/kimoktm/Face2face/blob/a700555ede40890c1714879f6f50442342871750/cli/textureFace.py",
  "function_signature_candidates": [
    {
      "line": 21,
      "text": "def getFaceKeypoints(img, detector, predictor, maxImgSizeForDetection=320):"
    },
    {
      "line": 44,
      "text": "def saveImage(path, img):"
    },
    {
      "line": 51,
      "text": "def main():"
    }
  ]
}

```text

#!/usr/bin/env python3
# -*- coding: utf-8 -*-
from face2face.models import MeshModel
from face2face.utils.opengl import Render
from face2face.optimize.camera import estimateCamMat, splitCamMat
from face2face.utils.mesh import generateFace, generateTexture, getImgsColors, writePly

import dlib
import cv2
import numpy as np
from skimage import img_as_float, img_as_ubyte
from skimage.transform import resize
import matplotlib.pyplot as plt

import os
import glob
import argparse



def getFaceKeypoints(img, detector, predictor, maxImgSizeForDetection=320):
    imgScale = 1
    scaledImg = img
    if max(img.shape) > maxImgSizeForDetection:
        imgScale = maxImgSizeForDetection / float(max(img.shape))
        scaledImg = cv2.resize(img, (int(img.shape[1] * imgScale), int(img.shape[0] * imgScale)))

    dets = detector(scaledImg, 1)

    if len(dets) == 0:
        return None

    shapes2D = []
    for det in dets:
        faceRectangle = dlib.rectangle(int(det.left() / imgScale), int(det.top() / imgScale), int(det.right() / imgScale), int(det.bottom() / imgScale))
        dlibShape = predictor(img, faceRectangle)
        shape2D = np.array([[p.x, p.y] for p in dlibShape.parts()])
        shape2D = shape2D.T
        shapes2D.append(shape2D)

    return shapes2D


def saveImage(path, img):
    b,g,r = cv2.split(img)
    img = cv2.merge([r,g,b])
    img = img_as_ubyte(img)
    cv2.imwrite(path, img)


def main():
    # Set weights for the 3DMM RGB color shape, landmark shape, and regularization terms
    scale_factor = 1.0
    max_iterations = 40
    wCol = 1
    # wLan = 1.25e-4
    wLan = 2.9e-5
    wRegC = 0.025e-5
    wRegS = 0.25e-5


    # Change directory to the folder that holds the VRN data, OpenPose landmarks, and original images (frames) from the source video
    os.chdir('./data')

    # Load 3DMM
    m = MeshModel('../models/bfm2017.npz')
    
    # Set an orthographic projection for the camera matrix
    cam = 'orthographic'

    # Landmark detector
    predictor_path = "../models/shape_predictor_68_face_landmarks.dat"
    detector = dlib.get_frontal_face_detector()
    predictor = dlib.shape_predictor(predictor_path)
    

    # apply mask on faces if supplied
    if FLAGS.face_mask is not None:
        mask_id = np.load(FLAGS.face_mask)
        m.face = np.delete(m.face, mask_id, axis = 0)
        m.vertex2face = np.array([np.where(np.isin(m.face.T, vertexInd).any(axis = 0))[0] for vertexInd in range(m.numVertices)])

    # load images
    idCoef = np.zeros(m.numId)
    expCoef = np.zeros(m.numExp)
    texCoef = np.zeros(m.numTex)
    shCoef = np.zeros((9, 3))
    shCoef[0, 0] = 0.5
    shCoef[0, 1] = 0.5
    shCoef[0, 2] = 0.5

    imgs = []
    landmarks = []
    img_params = []
    renderObj = None

    # load images & estimate camera params
    data_path = os.path.join(FLAGS.input_dir, '*g')
    keyframes = glob.glob(data_path)
    keyframes.sort()

    for i, frame in enumerate(keyframes):
        # Load the source video frame and convert to 64-bit float
        b,g,r = cv2.split(cv2.imread(frame))
        img_org = cv2.merge([r,g,b])
        # img_org = cv2.GaussianBlur(img_org, (3, 3), 0)
        img = img_as_float(img_org)

        shape2D = getFaceKeypoints(img_org, detector, predictor)
        shape2D = np.asarray(shape2D)[0].T 
        lm = shape2D[m.targetLMInd, :2]

        # Resize image for speed
        img = resize(img, (int(img.shape[0] / scale_factor), int(img.shape[1] / scale_factor)))
        lm = lm / scale_factor

        """
        Initial registration of similarity transform and shape coefficients
        """        
        param = np.r_[np.zeros(m.numId + m.numExp + 6), 1]
        
        # Get the vertex values of the 3DMM landmarks
        lm3D = generateFace(param, m, ind = m.sourceLMInd).T

        vertexCoords = generateFace(param, m)

        # Estimate the camera projection matrix from the landmark correspondences
        camMat = estimateCamMat(lm, lm3D, cam)
        
        # Factor the camera projection matrix into the intrinsic camera parameters and the rotation/translation similarity transform parameters
        s, angles, t = splitCamMat(camMat, cam)

        # Concatenate parameters for input into optimization routine. Note that the translation vector here is only (2,) for x and y (no z)
        unique_img_param = np.r_[shCoef.flatten(), expCoef, angles, t, s]

        if i == 0:
            # Initialize render Object
            texture = m.texMean
            meshData = np.r_[vertexCoords.T, texture.T]
            renderObj = Render(img.shape[1], img.shape[0], meshData, m.face)

            # Adjust Landmarks to be consistent across segments
            p1_id = 27 # nose
            p2_id = 8  # jaw
            x2 = lm[p1_id, 0]
            x1 = lm[p2_id, 0]
            y2 = lm[p1_id, 1]
            y1 = lm[p2_id, 1]
            nosejaw_dist = ((x2 - x1)**2 + (y2 - y1)**2)**(1/2)
            wLan = wLan * (225.0 / nosejaw_dist)


        # append parameters
        imgs.append(img)
        landmarks.append(lm)
        img_params.append(unique_img_param)

    imgs = np.asarray(imgs)
    landmarks = np.asarray(landmarks)
    img_params = np.asarray(img_params)

    if len(imgs.shape) == 3:
        imgs = imgs[np.newaxis, :]
        landmarks = landmarks[np.newaxis, :]
        img_params = img_params[np.newaxis, :]

    #
    # Jointly optimize all params over N images
    #
    allParams = np.load(os.path.join(FLAGS.output_dir, "all_params.npy"))
    texCoef = allParams[: texCoef.size]
    idCoef = allParams[texCoef.size : texCoef.size + idCoef.size]
    img_params = allParams[texCoef.size + idCoef.size :].reshape(img_params.shape)

    # Visualize results
    vertexCoordsList = []
    shCoefList = []

    for i in range(img_params.shape[0]):
        shCoef = img_params[i, : 27]
        expCoef = img_params[i, 27 : ]
        texParam = np.r_[texCoef, shCoef]
        shapeParam = np.r_[idCoef, expCoef]

        # Generate 3DMM vertices from shape and similarity transform parameters
        vertexCoords = generateFace(np.r_[shapeParam[:-1], 0, shapeParam[-1]], m)

        vertexCoordsList.append(vertexCoords)
        shCoefList.append(shCoef.reshape((9, 3)))

        # if i == 2:
        #     first_frame_param = np.r_[texCoef, shCoef, idCoef, expCoef]
        #     np.save(os.path.join(FLAGS.output_dir, "params"), first_frame_param)

    # TEST: Capture Texture
    vertexImgColor = getImgsColors(vertexCoordsList, shCoefList, imgs, m, renderObj)
    np.save(os.path.join(FLAGS.output_dir, "texture"), vertexImgColor)

    for i in range(img_params.shape[0]):
        lightedImgColor = generateTexture(vertexCoordsList[i], np.r_[texCoef, shCoefList[i].flatten()], m, vertexImgColor)
        renderObj = Render(imgs[i].shape[1], imgs[i].shape[0], np.r_[vertexCoordsList[i].T, lightedImgColor.T], m.face, False, imgs[i])
        renderObj.render()
        rendering, pixelCoord, pixelFaces, pixelBarycentricCoords = renderObj.grabRendering(return_info = True)
        saveImage(os.path.join(FLAGS.output_dir, "textured_" + str(i) + ".png"), rendering)

        if i == 0:
            writePly(os.path.join(FLAGS.output_dir, "textured_mesh.ply"), vertexCoordsList[0], m.face, lightedImgColor)

    plt.show()


if __name__ == "__main__":

    parser = argparse.ArgumentParser(description = 'Initialize Identity & Texture from multiple frames')
    parser.add_argument('--input_dir', help = 'Path to frames')
    parser.add_argument('--output_dir', help = 'Output directory')
    parser.add_argument('--face_mask', help = 'Path to face ids to mask as eyes (optional)')

    FLAGS, unparsed = parser.parse_known_args()

    main()


```

## Source file: cli/openFaceLandmarks.py

{
  "path": "cli/openFaceLandmarks.py",
  "blob_sha": "da6e33512e41ceb9482b98be1718648c877a2a87",
  "sha256": "397c9fd0bcc1ad3e46964aa929de1e103ded29a31210d7cedff4d1dc4301d0db",
  "lines": 105,
  "bytes": 3219,
  "selection_score": 15,
  "keyword_line_counts": {
    "gum": 4,
    "rig": 1,
    "face": 9
  },
  "url": "https://github.com/kimoktm/Face2face/blob/a700555ede40890c1714879f6f50442342871750/cli/openFaceLandmarks.py",
  "function_signature_candidates": [
    {
      "line": 18,
      "text": "def getFaceKeypoints(frame_cnt, openFace_landmarks):"
    },
    {
      "line": 29,
      "text": "def getdlibKeypoints(img, detector, predictor, maxImgSizeForDetection=320):"
    },
    {
      "line": 52,
      "text": "def drawPoints(img, points, color=(0, 255, 0)):"
    },
    {
      "line": 56,
      "text": "def main():"
    }
  ]
}

```text

import os
import glob
import argparse
import pandas as pd

import cv2
import numpy as np
from skimage import img_as_float

#delete
import dlib


# Basic model parameters as external flags.
FLAGS = None


def getFaceKeypoints(frame_cnt, openFace_landmarks):
    shapes2D = []
    frame = openFace_landmarks[openFace_landmarks['frame'] == frame_cnt]

    for i in range(0, 68):
        x = frame[' x_' + str(i)].values[0]
        y = frame[' y_' + str(i)].values[0]
        shapes2D.append([x, y])

    return shapes2D

def getdlibKeypoints(img, detector, predictor, maxImgSizeForDetection=320):
    imgScale = 1
    scaledImg = img
    if max(img.shape) > maxImgSizeForDetection:
        imgScale = maxImgSizeForDetection / float(max(img.shape))
        scaledImg = cv2.resize(img, (int(img.shape[1] * imgScale), int(img.shape[0] * imgScale)))

    dets = detector(scaledImg, 1)

    if len(dets) == 0:
        return None

    shapes2D = []
    for det in dets:
        faceRectangle = dlib.rectangle(int(det.left() / imgScale), int(det.top() / imgScale), int(det.right() / imgScale), int(det.bottom() / imgScale))
        dlibShape = predictor(img, faceRectangle)
        shape2D = np.array([[p.x, p.y] for p in dlibShape.parts()])
        shape2D = shape2D.T
        shapes2D.append(shape2D)

    return shapes2D


def drawPoints(img, points, color=(0, 255, 0)):
    for point in points:
        cv2.circle(img, (int(point[0]), int(point[1])), 2, color)

def main():
    # Landmark detector
    landmarks = pd.read_csv(FLAGS.openFace_landmarks)
    # cap = cv2.VideoCapture(FLAGS.video)
    # Landmark detector
    os.chdir('./data')
    predictor_path = "../models/shape_predictor_68_face_landmarks.dat"
    detector = dlib.get_frontal_face_detector()
    predictor = dlib.shape_predictor(predictor_path)
    cap = cv2.VideoCapture(FLAGS.video)

    frame_cnt = 1
    while(cap.isOpened()):
        ret, frame = cap.read()
        if ret == True:
            # Load the source video frame and convert to 64-bit float
            # b,g,r = cv2.split(frame)
            # img_org = cv2.merge([r,g,b])
            # img = img_as_float(img_org)

            # dlib2D = getdlibKeypoints(frame, detector, predictor)
            # dlib2D = np.asarray(dlib2D)[0].T 
            # drawPoints(frame, dlib2D, (255, 0, 0))

            shape2D = getFaceKeypoints(frame_cnt, landmarks)
            shape2D = np.asarray(shape2D)

            #frame = cv2.resize(frame,(1080, 720), interpolation = cv2.INTER_CUBIC)
            drawPoints(frame, shape2D)
            cv2.imshow('frame', frame)

            cv2.imwrite(os.path.join(FLAGS.output_dir, str(frame_cnt) + ".png"), frame)
            frame_cnt = frame_cnt + 1
            if cv2.waitKey(1) & 0xFF == ord('q'):
                break
        else:
            break
        


if __name__ == '__main__':

    parser = argparse.ArgumentParser(description = 'Extract Keyframes for initalization')
    parser.add_argument('--video', help = 'Path to input video')
    parser.add_argument('--openFace_landmarks', help = 'Path to openface landmarks')
    parser.add_argument('--output_dir', help = 'Output directory')

    FLAGS, unparsed = parser.parse_known_args()

    main()


```

## Source file: cli/processFacewarehouse.py

{
  "path": "cli/processFacewarehouse.py",
  "blob_sha": "bc4089c663213eedec5927ae9ec0ed9dc1af59da",
  "sha256": "e65e789687fdaa1b3ce71b202b0b90134c06a61c0eb47f9ed9b2ae57ad160d73",
  "lines": 188,
  "bytes": 8374,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 5,
    "rig": 1,
    "face": 11
  },
  "url": "https://github.com/kimoktm/Face2face/blob/a700555ede40890c1714879f6f50442342871750/cli/processFacewarehouse.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def importObjFW(dirName, shape = 0, dataToImport = ['v', 'vt', 'f'], pose = 20):"
    },
    {
      "line": 75,
      "text": "def generateModels(dirName, saveDirName = './'):"
    },
    {
      "line": 111,
      "text": "def saveLandmarks(dirName, saveDirName = './'):"
    },
    {
      "line": 130,
      "text": "def saveDepthMaps(dirName, saveDirName = './'):"
    },
    {
      "line": 155,
      "text": "def saveMasks(dirName, saveDirName = './masks/', mask = 'faceMask.obj', poses = 20):"
    }
  ]
}

```text

#!/usr/bin/env python3
# -*- coding: utf-8 -*-
from face2face.utils.transform import PCA
from face2face.utils.io import exportObj

import numpy as np
import re, os

def importObjFW(dirName, shape = 0, dataToImport = ['v', 'vt', 'f'], pose = 20):
    """
    Return the geometric and texture vertices along with the quadrilaterials containing the geometric and texture indices for all 150 testers of FaceWarehouse for a certain shape/pose/expression. Input (1) a string for the directory name that contains the folders 'Tester_1' through 'Tester_150', (2) an int for the shape number, which is in the range [0, 46] (1 neutral + 46 expressions), and (3) a list containing strings to indicate what part of the .obj file to read ('v' = geometric vertices, 'vt' = texture vertices, 'f' = face quadrilaterals).
    """
    # Number of observations (people/testers) in the dataset
    numTesters = 150
    
    # If input is just a single .obj file
    if dirName.endswith('.obj'):
        singleFile = True
        fName = dirName
    else:
        singleFile = False
        
        # Make sure directory name has final forward slash
        if not dirName.endswith('/'):
            dirName += '/'
    
    for i in range(numTesters):
        if (not singleFile) and pose == 47:
            fName = dirName + 'Tester_' + str(i+1) + '/Blendshape/shape_' + str(shape) + '.obj'
        elif (not singleFile) and pose == 20:
            fName = dirName + 'Tester_' + str(i+1) + '/TrainingPose/pose_' + str(shape) + '.obj'
        
        with open(fName) as fd:
            # Initialize lists to store the data from the .obj files
            v = []      # Geometric vertices (x, y, z)
            vt = []     # Texture vertices (U, V)
            f = []      # Face quadrilaterals
            for line in fd:
                if line.startswith('v ') and 'v' in dataToImport:
                    v.append([float(num) for num in line[2:].split(' ')])
                elif line.startswith('vt') and 'vt' in dataToImport and i == 0:
                    vt.append([float(num) for num in line[3:].split(' ')])
                elif line.startswith('f') and 'f' in dataToImport and i == 0:
                    f.append([int(ind) for ind in re.split('/| ', line[2:])])
                else:
                    continue
        
        if i == 0:
            geoV = np.empty((numTesters, len(v), 3))
            textV = np.empty((len(vt), 2))
            quad = np.empty((2, len(f), 4), dtype = 'int')
            
        # Store the data for each shape
        if 'vt' in dataToImport and i == 0:
            textV[:, :] = np.array(vt)
        if 'f' in dataToImport and i == 0:
            if len(f[0]) == 12:
                quad[0, :, :] = np.array(f)[:, [0, 3, 6, 9]]
                quad[1, :, :] = np.array(f)[:, [1, 4, 7, 10]]
            elif len(f[0]) == 3:
                quad = np.array(f)
            
        if 'v' in dataToImport and not singleFile:
            geoV[i, :, :] = np.array(v)
        elif 'v' in dataToImport and singleFile:
            geoV = np.array(v)
            break
        else:
            break
    
    # Select which data to return based on the dataToImport input
    objToData = {'v': geoV, 'vt': textV, 'f': quad}
    return [objToData.get(key) for key in dataToImport]

def generateModels(dirName, saveDirName = './'):
    """
    Generate eigenmodels of face meshes for (1) the neutral face and (2) the expressions. Save eigenvectors, eigenvalues, and means into .npy arrays.
    """
    if not dirName.endswith('/'):
        dirName += '/'
    if not saveDirName.endswith('/'):
        saveDirName += '/'
        
    # Neutral face
    print('Loading neutral faces')
    vNeu = importObjFW(dirName, shape = 0, dataToImport = ['v'])[0]
    vNeu = np.reshape(vNeu, (150, vNeu.shape[1]*3))
    
    evalNeu, evecNeu, meanNeu = PCA(vNeu)
    
    
    np.save(saveDirName + 'idEval', evalNeu)
    np.save(saveDirName + 'idEvec', evecNeu)
    np.save(saveDirName + 'idMean', meanNeu)
    
    # Expressions (from the 46 expression blendshapes)
    vExp = np.empty((150*46, vNeu.shape[1]))
    for s in range(46):
        print('Loading expression %d' % (s+1))
        temp = importObjFW(dirName, shape = s+1, dataToImport = ['v'], pose = 47)[0]
        # Subtract the neutral shape from the expression shape for each test subject
        vExp[s*150: (s+1)*150, :] = np.reshape(temp, (150, vNeu.shape[1])) - vNeu
    
    
    evalExp, evecExp = PCA(vExp, numPC = 76)[:2]
    
    
    np.save(saveDirName + 'expEval', evalExp)
    np.save(saveDirName + 'expEvec', evecExp)

def saveLandmarks(dirName, saveDirName = './'):
    """
    Read the landmarks from the TrainingPoses and save them in a .npy file.
    """
    if not dirName.endswith('/'):
        dirName += '/'
    if not saveDirName.endswith('/'):
        saveDirName += '/'
        
    landmarks = np.empty((20, 150, 74, 2))
    for pose in range(20):
        for tester in range(150):
            fName = 'Tester_' + str(tester+1) + '/TrainingPose/pose_' + str(pose) + '.land'
            with open(dirName + fName, 'r') as fd:
                next(fd)
                landmarks[pose, tester, :, :] = np.array([[float(num) for num in line.split(' ')] for line in fd])
    
    np.save(saveDirName + 'landmarksTrainingPoses', landmarks)

def saveDepthMaps(dirName, saveDirName = './'):
    """
    Read the depth information from the Kinect .poses files and save them in a .npy file.
    """
    if not dirName.endswith('/'):
        dirName += '/'
    if not saveDirName.endswith('/'):
        saveDirName += '/'
        
    depth = np.empty((20, 150, 480, 640), dtype = 'uint16')
    for pose in range(20):
        for tester in range(150):
            fName = 'Tester_' + str(tester+1) + '/TrainingPose/pose_' + str(pose) + '.poses'
            with open(dirName + fName, 'rb') as fd:
                # First 4 bytes contain the frame number, and the next 640*480*3 contain the RGB data, so skip these
                fd.seek(4 + 1*640*480*3)
                
                # Each depth map value is 2 bytes (short)
                d = fd.read(2*640*480)
            
            # Convert bytes to int
            depth[pose, tester, :, :] = np.array([int.from_bytes(bytes([x, y]), byteorder = 'little') for x, y in zip(d[0::2], d[1::2])]).reshape((480, 640))
            
    np.save(saveDirName + 'depthMaps', depth)

def saveMasks(dirName, saveDirName = './masks/', mask = 'faceMask.obj', poses = 20):
    """
    Loop through the original 3D head models in the directory defined by dirName and extract the facial area defined by mask .obj file, saving the facial 3D models of the original 3D heads into new .obj files in the directory defined by saveDirName.
    """
    if not saveDirName.endswith('/'):
        saveDirName += '/'
        
    # Loop through the poses/shapes
    for shape in range(poses):
        # The reference mask defining the facial region is based off of the first tester in pose/shape 0
        if shape == 0:
            v = importObjFW(dirName, shape, dataToImport = ['v'], pose = poses)[0]
            faceMask = importObjFW(mask, shape = 0)[0]
            idx = np.zeros(faceMask.shape[0], dtype = int)
            for i, vertex in enumerate(faceMask):
                idx[i] = np.where(np.equal(vertex, v[0, :, :]).all(axis = 1))[0]
        else:
            v = importObjFW(dirName, shape, dataToImport = ['v'], pose = poses)[0]
        
        v = v[:, idx, :]
        
        for tester in range(150):
            print('Processing shape %d for tester %d' % (shape+1, tester+1))
            if poses == 47:
                if not os.path.exists(saveDirName + 'Tester_' + str(tester+1) + '/Blendshape/'):
                    os.makedirs(saveDirName + 'Tester_' + str(tester+1) + '/Blendshape/')
                fName = saveDirName + 'Tester_' + str(tester+1) + '/Blendshape/shape_' + str(shape) + '.obj'
            
            if poses == 20:
                if not os.path.exists(saveDirName + 'Tester_' + str(tester+1) + '/TrainingPose/'):
                    os.makedirs(saveDirName + 'Tester_' + str(tester+1) + '/TrainingPose/')
                fName = saveDirName + 'Tester_' + str(tester+1) + '/TrainingPose/pose_' + str(shape) + '.obj'
            
            exportObj(v[tester, :, :], fNameIn = mask, fNameOut = fName)


```

## Source file: face2face/models.py

{
  "path": "face2face/models.py",
  "blob_sha": "87ef7a391e447cf62e01ba26894b9c9ca68ba666",
  "sha256": "8689ea83f5cffc0ee42e14e95c9e9cfe8a1eb1b81dc08bb4d92e1ebabf7688e2",
  "lines": 82,
  "bytes": 5876,
  "selection_score": 3,
  "keyword_line_counts": {
    "eye": 1,
    "face": 7
  },
  "url": "https://github.com/kimoktm/Face2face/blob/a700555ede40890c1714879f6f50442342871750/face2face/models.py",
  "function_signature_candidates": [
    {
      "line": 35,
      "text": "def __init__(self, modelFile, numIdEvecs = 80, numExpEvecs = 78, numTexEvecs = 80):"
    }
  ]
}

```text

#!/usr/bin/env python3
# -*- coding: utf-8 -*-
import numpy as np
import os

class MeshModel:
    """A 3D Morphable Model class object
    
    Args:
        modelFile (str): Filename of .npz file containing 3DMM
        numIdEvecs (int): Number of the shape identity eigenvectors with the highest eigenvalues from the 3DMM to keep
        numExpEvecs (int): Number of the shape facial expression eigenvectors with the highest eigenvalues from the 3DMM to keep
        numTexEvecs (int): Number of the texture eigenvectors with the highest eigenvalues from the 3DMM to keep
            
    Attributes:
        numId (int): number of shape identity eigenvectors
        numExp (int): number of shape facial expression eigenvectors
        numTex (int): number of texture eigenvectors
        numVertices (int): number of vertices in the 3DMM
        numFaces (int): number of triangular faces in the 3DMM
        face (ndarray): array containing the vertex indices for each face, (numFaces, 3)
        vertex2face (ndarray): array containing the face index of each vertex, (numVertices,)
        idMean (ndarray): shape identity mean, (3, numVertices)
        idEvec (ndarray): shape identity eigenvectors, (3, numVertices, numId)
        idEval (ndarray): shape identity eigenvalues, (numId,)
        expEvec (ndarray): shape facial expression eigenvectors, (3, numVertices, numExp)
        expEval (ndarray): shape facial expression eigenvalues, (numExp)
        texMean (ndarray): texture mean, (3, numVertices)
        texEvec (ndarray): texture eigenvectors, (3, numVertices, numTex)
        texEval (ndarray): texture eigenvalues, (numTex,)
        targetLMInd (ndarray): landmark indices for OpenPose
        sourceLMInd (ndarray): vertex indices of the 3DMM that correspond to ``targetLMInd``
    """
    # def __init__(self, modelFile, numIdEvecs = 199, numExpEvecs = 100, numTexEvecs = 199):
    def __init__(self, modelFile, numIdEvecs = 80, numExpEvecs = 78, numTexEvecs = 80):
        """Loads a 3DMM from a .npz file.
        """
        
        model = os.path.splitext(os.path.basename(modelFile))[0]
        
        modelDict = np.load(modelFile)
        self.__dict__.update(modelDict)
        
        self.numId = numIdEvecs
        self.numExp = numExpEvecs
        self.idEvec = self.idEvec[:, :, :self.numId]
        self.idEval = self.idEval[:self.numId]
        self.expEvec = self.expEvec[:, :, :self.numExp]
        self.expEval = self.expEval[:self.numExp]
        
        self.numFaces = self.face.shape[0]
        
        # The Basel Face Model 2017 has a texture component, and for this 3DMM we found some correspondences between the OpenPose landmarks and the 3DMM vertex indices
        if model == 'bfm2017':
            self.numTex = numTexEvecs
            self.texEvec = self.texEvec[:, :, :self.numTex]
            self.texEval = self.texEval[:self.numTex]

            # openPose face-landmarks
            # These are indices representing the OpenPose landmarks that we have a correspondence with for the BFM2017 model
            # self.targetLMInd = np.array([0, 1, 2, 3, 8, 13, 14, 15, 16, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 61, 62, 63, 65, 66, 67, 68, 69])

            # # These are vertex indices that correspond with the OpenPose landmark indices above
            # self.sourceLMInd = np.array([16203, 16235, 16260, 16290, 27061, 22481, 22451, 22426, 22394, 8134, 8143, 8151, 8156, 6986, 7695, 8167, 8639, 9346, 2345, 4146, 5180, 6214, 4932, 4158, 10009, 11032, 12061, 13872, 12073, 11299, 5264, 6280, 7472, 8180, 8888, 10075, 11115, 9260, 8553, 8199, 7845, 7136, 7600, 8190, 8780, 8545, 8191, 7837, 4538, 11679])

            # dlib face-landmarks
            # These are indices representing the OpenPose landmarks that we have a correspondence with for the BFM2017 model
            # self.targetLMInd = np.array(range(0, 68))
            # self.targetLMInd = np.array([30, 36, 39, 42, 45, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67])

            # # These are vertex indices that correspond with the OpenPose landmark indices above
            # self.sourceLMInd = np.array([8156, 2602, 5830, 10390, 13481, 5522, 6026, 7355, 8181, 9007, 10329, 10857, 9730, 8670, 8199, 7726, 6898, 6291, 7364, 8190, 9016, 10088, 8663, 8191, 7719])

            # my landmarks
            self.targetLMInd = np.array([0, 1, 2, 3, 7, 8, 9, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67])

            self.sourceLMInd = np.array([16203, 16235, 16260, 16290, 26869, 27061, 27253, 22481, 22451, 22426, 22394, 22586, 22991, 23303, 23519, 23736, 24312, 24527, 24743, 25055, 25466, 8134, 8143, 8151, 8157, 6986, 7695, 8167, 8639, 9346, 2602, 4146, 4920, 5830, 4674, 3900, 10390, 11287, 12061, 13481, 12331, 11557, 5522, 6026, 7355, 8181, 9007, 10329, 10857, 9730, 8670, 8199, 7726, 6898, 6291, 7364, 8190, 9016, 10088, 8663, 8191, 7719])

            # # my landmarks - no eyebrows
            # self.targetLMInd = np.array([0, 1, 2, 3, 7, 8, 9, 13, 14, 15, 16, 19, 24, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67])
            
            # self.sourceLMInd = np.array([16203, 16235, 16260, 16290, 26869, 27061, 27253, 22481, 22451, 22426, 22394, 23303, 24743, 8134, 8143, 8151, 8157, 6986, 7695, 8167, 8639, 9346, 2602, 4146, 4920, 5830, 4674, 3900, 10390, 11287, 12061, 13481, 12331, 11557, 5522, 6026, 7355, 8181, 9007, 10329, 10857, 9730, 8670, 8199, 7726, 6898, 6291, 7364, 8190, 9016, 10088, 8663, 8191, 7719])

```
