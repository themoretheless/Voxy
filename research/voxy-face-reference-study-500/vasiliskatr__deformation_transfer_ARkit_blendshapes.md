# Facial rendering source scan: vasiliskatr/deformation_transfer_ARkit_blendshapes

Repository: https://github.com/vasiliskatr/deformation_transfer_ARkit_blendshapes

Pinned source revision: 5faaf6a48621cc5b186b2fbfac24d97c1101ef14

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"eye": ["data/ARKit_blendShapes/eyeBlinkLeft.obj", "data/ARKit_blendShapes/eyeBlinkRight.obj", "data/ARKit_blendShapes/eyeLookDownLeft.obj", "data/ARKit_blendShapes/eyeLookDownRight.obj", "data/ARKit_blendShapes/eyeLookInLeft.obj", "data/ARKit_blendShapes/eyeLookInRight.obj", "data/ARKit_blendShapes/eyeLookOutLeft.obj", "data/ARKit_blendShapes/eyeLookOutRight.obj", "data/ARKit_blendShapes/eyeLookUpLeft.obj", "data/ARKit_blendShapes/eyeLookUpRight.obj", "data/ARKit_blendShapes/eyeSquintLeft.obj", "data/ARKit_blendShapes/eyeSquintRight.obj", "data/ARKit_blendShapes/eyeWideLeft.obj", "data/ARKit_blendShapes/eyeWideRight.obj"], "blendshape": ["data/ARKit_blendShapes/Neutral.obj", "data/ARKit_blendShapes/browDownLeft.obj", "data/ARKit_blendShapes/browDownRight.obj", "data/ARKit_blendShapes/browInnerUp.obj", "data/ARKit_blendShapes/browOuterUpLeft.obj", "data/ARKit_blendShapes/browOuterUpRight.obj", "data/ARKit_blendShapes/cheekPuff.obj", "data/ARKit_blendShapes/cheekSquintLeft.obj", "data/ARKit_blendShapes/cheekSquintRight.obj", "data/ARKit_blendShapes/eyeBlinkLeft.obj", "data/ARKit_blendShapes/eyeBlinkRight.obj", "data/ARKit_blendShapes/eyeLookDownLeft.obj", "data/ARKit_blendShapes/eyeLookDownRight.obj", "data/ARKit_blendShapes/eyeLookInLeft.obj", "data/ARKit_blendShapes/eyeLookInRight.obj", "data/ARKit_blendShapes/eyeLookOutLeft.obj", "data/ARKit_blendShapes/eyeLookOutRight.obj", "data/ARKit_blendShapes/eyeLookUpLeft.obj", "data/ARKit_blendShapes/eyeLookUpRight.obj", "data/ARKit_blendShapes/eyeSquintLeft.obj", "data/ARKit_blendShapes/eyeSquintRight.obj", "data/ARKit_blendShapes/eyeWideLeft.obj", "data/ARKit_blendShapes/eyeWideRight.obj", "data/ARKit_blendShapes/jawForward.obj", "data/ARKit_blendShapes/jawLeft.obj", "data/ARKit_blendShapes/jawOpen.obj", "data/ARKit_blendShapes/jawRight.obj", "data/ARKit_blendShapes/mouthClose.obj", "data/ARKit_blendShapes/mouthDimpleLeft.obj", "data/ARKit_blendShapes/mouthDimpleRight.obj"], "jaw": ["data/ARKit_blendShapes/jawForward.obj", "data/ARKit_blendShapes/jawLeft.obj", "data/ARKit_blendShapes/jawOpen.obj", "data/ARKit_blendShapes/jawRight.obj"], "mouth": ["data/ARKit_blendShapes/mouthClose.obj", "data/ARKit_blendShapes/mouthDimpleLeft.obj", "data/ARKit_blendShapes/mouthDimpleRight.obj", "data/ARKit_blendShapes/mouthFrownLeft.obj", "data/ARKit_blendShapes/mouthFrownRight.obj", "data/ARKit_blendShapes/mouthFunnel.obj", "data/ARKit_blendShapes/mouthLeft.obj", "data/ARKit_blendShapes/mouthLowerDownLeft.obj", "data/ARKit_blendShapes/mouthLowerDownRight.obj", "data/ARKit_blendShapes/mouthPressLeft.obj", "data/ARKit_blendShapes/mouthPressRight.obj", "data/ARKit_blendShapes/mouthPucker.obj", "data/ARKit_blendShapes/mouthRight.obj", "data/ARKit_blendShapes/mouthRollLower.obj", "data/ARKit_blendShapes/mouthRollUpper.obj", "data/ARKit_blendShapes/mouthShrugLower.obj", "data/ARKit_blendShapes/mouthShrugUpper.obj", "data/ARKit_blendShapes/mouthSmileLeft.obj", "data/ARKit_blendShapes/mouthSmileRight.obj", "data/ARKit_blendShapes/mouthStretchLeft.obj", "data/ARKit_blendShapes/mouthStretchRight.obj", "data/ARKit_blendShapes/mouthUpperUpLeft.obj", "data/ARKit_blendShapes/mouthUpperUpRight.obj"], "deform": ["local_packages/__pycache__/deformationTransfer.cpython-38.pyc", "local_packages/deformationTransfer.py"], "rig": ["data/ARKit_blendShapes/browDownRight.obj", "data/ARKit_blendShapes/browOuterUpRight.obj", "data/ARKit_blendShapes/cheekSquintRight.obj", "data/ARKit_blendShapes/eyeBlinkRight.obj", "data/ARKit_blendShapes/eyeLookDownRight.obj", "data/ARKit_blendShapes/eyeLookInRight.obj", "data/ARKit_blendShapes/eyeLookOutRight.obj", "data/ARKit_blendShapes/eyeLookUpRight.obj", "data/ARKit_blendShapes/eyeSquintRight.obj", "data/ARKit_blendShapes/eyeWideRight.obj", "data/ARKit_blendShapes/jawRight.obj", "data/ARKit_blendShapes/mouthDimpleRight.obj", "data/ARKit_blendShapes/mouthFrownRight.obj", "data/ARKit_blendShapes/mouthLowerDownRight.obj", "data/ARKit_blendShapes/mouthPressRight.obj", "data/ARKit_blendShapes/mouthRight.obj", "data/ARKit_blendShapes/mouthSmileRight.obj", "data/ARKit_blendShapes/mouthStretchRight.obj", "data/ARKit_blendShapes/mouthUpperUpRight.obj", "data/ARKit_blendShapes/noseSneerRight.obj"]}

## Source file: local_packages/deformationTransfer.py

{
  "path": "local_packages/deformationTransfer.py",
  "blob_sha": "c84832f886414b306d03080561677a5f1d2e03ee",
  "sha256": "c0c5f189e201970e61f234e9d759f2a7bf1f6bb30c9b77b101f2aaf6093d836b",
  "lines": 441,
  "bytes": 19359,
  "selection_score": 15,
  "keyword_line_counts": {
    "deform": 8,
    "face": 119
  },
  "url": "https://github.com/vasiliskatr/deformation_transfer_ARkit_blendshapes/blob/5faaf6a48621cc5b186b2fbfac24d97c1101ef14/local_packages/deformationTransfer.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def find_adjsacent_faces (faces):"
    },
    {
      "line": 27,
      "text": "def compute_v4 (vertices, faces):"
    },
    {
      "line": 37,
      "text": "def compute_face_normals(vertices, faces):"
    },
    {
      "line": 48,
      "text": "def compute_V_inverse_local (vertices, face, v4):"
    },
    {
      "line": 59,
      "text": "def compute_V_inverse (vertices, faces, v4):"
    },
    {
      "line": 70,
      "text": "def makeEs_ATA_ATc(vertices, faces, Vinv):"
    },
    {
      "line": 191,
      "text": "def makeEi_ATA_ATc(vertices, faces, Vinv):"
    },
    {
      "line": 255,
      "text": "def compute_face_cenetroids (vertices,faces):"
    },
    {
      "line": 263,
      "text": "def get_compatible_faces (d_target_centroids, target_faces, d_target_face_normals, source_centroids, source_faces, source_face_normals, threshold = 10):"
    },
    {
      "line": 324,
      "text": "def get_correspondece_faces (source_vertices, source_faces, deformed_target_vertices, target_faces):"
    },
    {
      "line": 351,
      "text": "def make_source_rotation_matrix (vertices_1, faces_1, v4_1, vertices_2, faces_2):"
    },
    {
      "line": 383,
      "text": "def makeEd_A(corr, AN_Vertices, AN_Faces, AN_Vinv):"
    },
    {
      "line": 435,
      "text": "def makeEd_ATc(corr, source_rotation, Ed_A):"
    }
  ]
}

```text

import numpy as np
from math import sqrt, acos, cos, sin
import scipy.sparse as sp
import numba
from numba import jit



def find_adjsacent_faces (faces):
    """ For every triangle/face in a mesh, it retuns the adjacent triangles/faces """

    print ("\nFinding adjacent faces/triangles...")
    adjacent_faces = [[] for row in range(len(faces))] 
    flist = faces[:,[0, 1, 2]].tolist()
    for i in range(len(flist)):
        set1 = set(flist[i])
        for j in range(i+1, len(flist)):
            set2 = set(flist[j])
            if(len(set1 & set2)==2): #i.e. if 2 same vertex indices are found between set1 and set2 ->  set1 and set2 are adjacent
                adjacent_faces[i] += [j]
                adjacent_faces[j] += [i]
            if(len(adjacent_faces[i])>=3): break
    return adjacent_faces


@jit(nopython=True, parallel=True)
def compute_v4 (vertices, faces):
    v4 = np.zeros((len(faces), 3), dtype = np.float32)
    for k in numba.prange(len(faces)):
        cross = np.cross(vertices[faces[k, 1],0:3] - vertices[faces[k, 0],0:3], vertices[faces[k, 2],0:3]-vertices[faces[k, 0],0:3])
        v4[k] = vertices[faces[k, 0],0:3] + cross / sqrt(np.linalg.norm(cross))
        
    return v4


@jit(nopython=True, parallel=True)
def compute_face_normals(vertices, faces):
    face_normals = np.zeros((len(faces), 3), dtype = np.float32)
    for k in numba.prange(len(faces)):
        cross = np.cross(vertices[faces[k, 1],0:3] - vertices[faces[k, 0],0:3], vertices[faces[k, 2],0:3] - vertices[faces[k, 0],0:3])  
        face_normals[k] = cross / np.linalg.norm(cross)
    
    return face_normals




def compute_V_inverse_local (vertices, face, v4):
    
    a = np.array([vertices[face[1],0:3] - vertices[face[0],0:3]])
    b = np.array([vertices[face[2],0:3] - vertices[face[0],0:3]])
    c = np.array([v4 - vertices[face[0],0:3]])
    V_inverse_local = np.linalg.solve(np.hstack([a.T, b.T, c.T]), np.identity(3))
    
    return V_inverse_local



def compute_V_inverse (vertices, faces, v4):
    V_inverse = np.zeros((len(faces), 3, 3), dtype = np.float32)  #  inverse  matrix of vertices, including v4 
    for i in range(len(faces)):
        V_inverse[i] = compute_V_inverse_local (vertices, faces[i], v4[i])
    
    return V_inverse




#make large matrices and vectors
def makeEs_ATA_ATc(vertices, faces, Vinv):
    
    Adjacent = find_adjsacent_faces(faces)
    
    adj_num = 0
    for i in range(len(Adjacent)):
        for j in range(len(Adjacent[i])):
            adj_num+=1

    Es_cVector = sp.csr_matrix((1, adj_num*9), dtype=np.float32)
    Es_cVector = Es_cVector.T
    Es_A = sp.lil_matrix((adj_num*9, len(vertices)*3 + len(faces)*3), dtype=np.float32)

    row = 0
    Vinv_list = Vinv.tolist()
    Faces = faces.tolist()
    len_vertices = len(vertices)

    for i in range(len(Adjacent)):
        for j in Adjacent[i]:

            e1 = np.sum(Vinv[i,0:3,0])
            e2 = np.sum(Vinv[i,0:3,1])
            e3 = np.sum(Vinv[i,0:3,2])

            idx_v1 = Faces[i][0]
            Es_A[row*9, idx_v1*3]     = -e1
            Es_A[row*9+1, idx_v1*3+1] = -e1
            Es_A[row*9+2, idx_v1*3+2] = -e1
            Es_A[row*9+3, idx_v1*3]   = -e2
            Es_A[row*9+4, idx_v1*3+1] = -e2
            Es_A[row*9+5, idx_v1*3+2] = -e2
            Es_A[row*9+6, idx_v1*3]   = -e3
            Es_A[row*9+7, idx_v1*3+1] = -e3
            Es_A[row*9+8, idx_v1*3+2] = -e3

            idx_v2 = Faces[i][1]
            Es_A[row*9, idx_v2*3]     = Vinv_list[i][0][0]
            Es_A[row*9+1, idx_v2*3+1] = Vinv_list[i][0][0]
            Es_A[row*9+2, idx_v2*3+2] = Vinv_list[i][0][0]
            Es_A[row*9+3, idx_v2*3]   = Vinv_list[i][0][1]
            Es_A[row*9+4, idx_v2*3+1] = Vinv_list[i][0][1]
            Es_A[row*9+5, idx_v2*3+2] = Vinv_list[i][0][1]
            Es_A[row*9+6, idx_v2*3]   = Vinv_list[i][0][2]
            Es_A[row*9+7, idx_v2*3+1] = Vinv_list[i][0][2]
            Es_A[row*9+8, idx_v2*3+2] = Vinv_list[i][0][2]

            idx_v3 = Faces[i][2]
            Es_A[row*9, idx_v3*3]     = Vinv_list[i][1][0]
            Es_A[row*9+1, idx_v3*3+1] = Vinv_list[i][1][0]
            Es_A[row*9+2, idx_v3*3+2] = Vinv_list[i][1][0]
            Es_A[row*9+3, idx_v3*3]   = Vinv_list[i][1][1]
            Es_A[row*9+4, idx_v3*3+1] = Vinv_list[i][1][1]
            Es_A[row*9+5, idx_v3*3+2] = Vinv_list[i][1][1]
            Es_A[row*9+6, idx_v3*3]   = Vinv_list[i][1][2]
            Es_A[row*9+7, idx_v3*3+1] = Vinv_list[i][1][2]
            Es_A[row*9+8, idx_v3*3+2] = Vinv_list[i][1][2]

            Es_A[row*9, i*3+len_vertices*3]     = Vinv_list[i][2][0]
            Es_A[row*9+1, i*3+len_vertices*3+1] = Vinv_list[i][2][0]
            Es_A[row*9+2, i*3+len_vertices*3+2] = Vinv_list[i][2][0]
            Es_A[row*9+3, i*3+len_vertices*3]   = Vinv_list[i][2][1]
            Es_A[row*9+4, i*3+len_vertices*3+1] = Vinv_list[i][2][1]
            Es_A[row*9+5, i*3+len_vertices*3+2] = Vinv_list[i][2][1]
            Es_A[row*9+6, i*3+len_vertices*3]   = Vinv_list[i][2][2]
            Es_A[row*9+7, i*3+len_vertices*3+1] = Vinv_list[i][2][2]
            Es_A[row*9+8, i*3+len_vertices*3+2] = Vinv_list[i][2][2]

            e1 = np.sum(Vinv[j,0:3,0])
            e2 = np.sum(Vinv[j,0:3,1])
            e3 = np.sum(Vinv[j,0:3,2])

            idx_v1 = Faces[j][0]
            Es_A[row*9, idx_v1*3]     += e1
            Es_A[row*9+1, idx_v1*3+1] += e1
            Es_A[row*9+2, idx_v1*3+2] += e1
            Es_A[row*9+3, idx_v1*3]   += e2
            Es_A[row*9+4, idx_v1*3+1] += e2
            Es_A[row*9+5, idx_v1*3+2] += e2
            Es_A[row*9+6, idx_v1*3]   += e3
            Es_A[row*9+7, idx_v1*3+1] += e3
            Es_A[row*9+8, idx_v1*3+2] += e3

            idx_v2 = Faces[j][1]
            Es_A[row*9, idx_v2*3]     += -Vinv_list[j][0][0]
            Es_A[row*9+1, idx_v2*3+1] += -Vinv_list[j][0][0]
            Es_A[row*9+2, idx_v2*3+2] += -Vinv_list[j][0][0]
            Es_A[row*9+3, idx_v2*3]   += -Vinv_list[j][0][1]
            Es_A[row*9+4, idx_v2*3+1] += -Vinv_list[j][0][1]
            Es_A[row*9+5, idx_v2*3+2] += -Vinv_list[j][0][1]
            Es_A[row*9+6, idx_v2*3]   += -Vinv_list[j][0][2]
            Es_A[row*9+7, idx_v2*3+1] += -Vinv_list[j][0][2]
            Es_A[row*9+8, idx_v2*3+2] += -Vinv_list[j][0][2]

            idx_v3 = Faces[j][2]
            Es_A[row*9, idx_v3*3]     += -Vinv_list[j][1][0]
            Es_A[row*9+1, idx_v3*3+1] += -Vinv_list[j][1][0]
            Es_A[row*9+2, idx_v3*3+2] += -Vinv_list[j][1][0]
            Es_A[row*9+3, idx_v3*3]   += -Vinv_list[j][1][1]
            Es_A[row*9+4, idx_v3*3+1] += -Vinv_list[j][1][1]
            Es_A[row*9+5, idx_v3*3+2] += -Vinv_list[j][1][1]
            Es_A[row*9+6, idx_v3*3]   += -Vinv_list[j][1][2]
            Es_A[row*9+7, idx_v3*3+1] += -Vinv_list[j][1][2]
            Es_A[row*9+8, idx_v3*3+2] += -Vinv_list[j][1][2]

            Es_A[row*9, j*3+len_vertices*3]     += -Vinv_list[j][2][0]
            Es_A[row*9+1, j*3+len_vertices*3+1] += -Vinv_list[j][2][0]
            Es_A[row*9+2, j*3+len_vertices*3+2] += -Vinv_list[j][2][0]
            Es_A[row*9+3, j*3+len_vertices*3]   += -Vinv_list[j][2][1]
            Es_A[row*9+4, j*3+len_vertices*3+1] += -Vinv_list[j][2][1]
            Es_A[row*9+5, j*3+len_vertices*3+2] += -Vinv_list[j][2][1]
            Es_A[row*9+6, j*3+len_vertices*3]   += -Vinv_list[j][2][2]
            Es_A[row*9+7, j*3+len_vertices*3+1] += -Vinv_list[j][2][2]
            Es_A[row*9+8, j*3+len_vertices*3+2] += -Vinv_list[j][2][2]

            row += 1

    return np.dot(Es_A.T, Es_A), np.dot(Es_A.T, Es_cVector)



def makeEi_ATA_ATc(vertices, faces, Vinv):
    Flat_identity = sp.csc_matrix((0, 1), dtype=np.float32)
    for i in range(len(faces)):
        Flat_identity = sp.vstack((Flat_identity, sp.identity(3, format="lil").reshape((9, 1))), format="csc", dtype=np.float32)

    Ei_cVector = Flat_identity
    Ei_A = sp.lil_matrix((len(faces)*9, len(vertices)*3 + len(faces)*3), dtype=np.float32)

    Vinv_list = Vinv.tolist()
    Faces = faces.tolist()
    len_vertices = len(vertices)
    for i in range(len(faces)):
        idx_f = i
        e1 = np.sum(Vinv[idx_f,0:3,0])
        e2 = np.sum(Vinv[idx_f,0:3,1])
        e3 = np.sum(Vinv[idx_f,0:3,2])
                
        idx_v1 = Faces[idx_f][0]
        Ei_A[i*9, idx_v1*3]     = -e1
        Ei_A[i*9+1, idx_v1*3+1] = -e1
        Ei_A[i*9+2, idx_v1*3+2] = -e1
        Ei_A[i*9+3, idx_v1*3]   = -e2
        Ei_A[i*9+4, idx_v1*3+1] = -e2
        Ei_A[i*9+5, idx_v1*3+2] = -e2
        Ei_A[i*9+6, idx_v1*3]   = -e3
        Ei_A[i*9+7, idx_v1*3+1] = -e3
        Ei_A[i*9+8, idx_v1*3+2] = -e3

        idx_v2 = Faces[idx_f][1]
        Ei_A[i*9, idx_v2*3]     = Vinv_list[idx_f][0][0]
        Ei_A[i*9+1, idx_v2*3+1] = Vinv_list[idx_f][0][0]
        Ei_A[i*9+2, idx_v2*3+2] = Vinv_list[idx_f][0][0]
        Ei_A[i*9+3, idx_v2*3]   = Vinv_list[idx_f][0][1]
        Ei_A[i*9+4, idx_v2*3+1] = Vinv_list[idx_f][0][1]
        Ei_A[i*9+5, idx_v2*3+2] = Vinv_list[idx_f][0][1]
        Ei_A[i*9+6, idx_v2*3]   = Vinv_list[idx_f][0][2]
        Ei_A[i*9+7, idx_v2*3+1] = Vinv_list[idx_f][0][2]
        Ei_A[i*9+8, idx_v2*3+2] = Vinv_list[idx_f][0][2]
                
        idx_v3 = Faces[idx_f][2]
        Ei_A[i*9, idx_v3*3]     = Vinv_list[idx_f][1][0]
        Ei_A[i*9+1, idx_v3*3+1] = Vinv_list[idx_f][1][0]
        Ei_A[i*9+2, idx_v3*3+2] = Vinv_list[idx_f][1][0]
        Ei_A[i*9+3, idx_v3*3]   = Vinv_list[idx_f][1][1]
        Ei_A[i*9+4, idx_v3*3+1] = Vinv_list[idx_f][1][1]
        Ei_A[i*9+5, idx_v3*3+2] = Vinv_list[idx_f][1][1]
        Ei_A[i*9+6, idx_v3*3]   = Vinv_list[idx_f][1][2]
        Ei_A[i*9+7, idx_v3*3+1] = Vinv_list[idx_f][1][2]
        Ei_A[i*9+8, idx_v3*3+2] = Vinv_list[idx_f][1][2]

        Ei_A[i*9, idx_f*3+len_vertices*3]    = Vinv_list[idx_f][2][0]
        Ei_A[i*9+1, idx_f*3+len_vertices*3+1] = Vinv_list[idx_f][2][0]
        Ei_A[i*9+2, idx_f*3+len_vertices*3+2] = Vinv_list[idx_f][2][0]
        Ei_A[i*9+3, idx_f*3+len_vertices*3]   = Vinv_list[idx_f][2][1]
        Ei_A[i*9+4, idx_f*3+len_vertices*3+1] = Vinv_list[idx_f][2][1]
        Ei_A[i*9+5, idx_f*3+len_vertices*3+2] = Vinv_list[idx_f][2][1]
        Ei_A[i*9+6, idx_f*3+len_vertices*3]   = Vinv_list[idx_f][2][2]
        Ei_A[i*9+7, idx_f*3+len_vertices*3+1] = Vinv_list[idx_f][2][2]
        Ei_A[i*9+8, idx_f*3+len_vertices*3+2] = Vinv_list[idx_f][2][2]

    return np.dot(Ei_A.T, Ei_A), np.dot(Ei_A.T, Ei_cVector)


@jit(nopython=True, parallel=True)
def compute_face_cenetroids (vertices,faces):
    centeroids = np.zeros((len(faces), 3), dtype=np.float32)
    for i in numba.prange(len(faces)):
        centeroids[i] = (vertices[faces[i, 0]] + vertices[faces[i, 1]] + vertices[faces[i, 2]])/3
    return centeroids



def get_compatible_faces (d_target_centroids, target_faces, d_target_face_normals, source_centroids, source_faces, source_face_normals, threshold = 10):

    """ 
    Gets the compatible faces(triangles) between two objects of different topology.
    The corresponding faces must be close to each other (centroid proximity)
    and the respective face(triangle) normals should be less than 90deg.
    
    For a pair of objects of different topology, this funtion is called twice. 
    target mesh -> source mesh
    and 
    source mesh -> target mesh
    ---------------------------------------
    d_target_centroids: ndarray with the centroids for each face(triangle) of the deforrmed target mesh [:,3]
    target_faces: ndarray with the faces(triangles) of target mesh [:,3]
    d_target_face_normals: ndarray with the normals for every face(triangle) of target mesh [:,3]
    source_centroids: ndarray with the centroids for each face(triangle) of source mesh [:,3]
    source_faces: ndarray with the faces(triangles) of source mesh [:,3]
    source_face_normals: ndarray with the normals for every face(triangle) of the source mesh [:,3]
    threshold: scalar, consider matches which are closer than this
    ---------------------------------------
     """

    threshold = 2
    corr = []
    ##for all target Faces
    for i in range(len(target_faces)):
        validindex = -1
        norm = np.linalg.norm(d_target_centroids[i] - source_centroids, axis=1)
        while(np.min(norm) < threshold):
            idx = np.argmin(norm)
            if(np.dot(d_target_face_normals[i], source_face_normals[idx])>0): # check that the normals of close faces are < 90 deg.
                validindex = idx
                break
            else:
                norm[idx] = threshold
        if(validindex >= 0):
            corr += [[validindex, i]]
    
    print (len(corr), " correspondeces found out of ", len(target_faces), " faces on the target mesh")
    corr_step1 = len(corr)
    
        ##for all Source Faces
    for i in range(len(source_faces)):
        validindex = -1
        norm = np.linalg.norm(source_centroids[i] - d_target_centroids, axis=1)
        while(np.min(norm) < threshold):
            idx = np.argmin(norm)
            if(np.dot(source_face_normals[i], d_target_face_normals[idx])>0):
                validindex = idx
                break
            else:
                norm[idx] = threshold
        if(validindex >= 0):
            corr += [[i, validindex]]
            
    print (len(corr) - corr_step1, " correspondeces found out of ", len(source_faces), "faces on the source mesh")  
    
    return corr



def get_correspondece_faces (source_vertices, source_faces, deformed_target_vertices, target_faces):

    """ 
    Uses [def get_compatible_faces] to compute all the  face correspondeces between the source object and target object
    ---------------------------------------
    source_vertices: np.ndarray source_vertices neutral face [:,3]
    source_faces: np.ndarray source_faces neutral face (object 1) [:,3]
    deformed_target_vertices: np.ndarray target_vertices neutral face which is deformed using NRICP to mactch the shape of the source neutral face [:,3]
    target_faces: np.ndarray target_faces neutral face (object 1) [:,3] (does not change with NRICP since the topology of the deformed_target is the same as the target)
    ---------------------------------------
    """
    
    print ("\nComputing triangle correspondences...")
    
    source_face_normals = compute_face_normals(source_vertices, source_faces)
    source_centroids = compute_face_cenetroids(source_vertices, source_faces)

    deformed_target_face_normals = compute_face_normals(deformed_target_vertices, target_faces)
    deformed_target_centroids = compute_face_cenetroids(deformed_target_vertices, target_faces)
    
    correspondences = get_compatible_faces(deformed_target_centroids, target_faces, deformed_target_face_normals, source_centroids, source_faces, source_face_normals)
    
    return correspondences




def make_source_rotation_matrix (vertices_1, faces_1, v4_1, vertices_2, faces_2):
    """ 
    Computes the matrix of affine transformations for each face(triangle) of a 3D object which goes from frame A(vertices-1) to frame B(vertices_2)
    We use this to extract the deformation for each pair of the source blend shapes (generic blend shapes).
    ---------------------------------------
    vertices_1: np.ndarray source_vertices neutral face (object 1) [:,3]
    vertices_2: np.ndarray source_vertices some expression  (object 2) [:,3]
    faces_1 = faces_2: np.ndarray with the faces(triangles) [:,3]. The rotation matrix is calculated for objects of the same topology 
    - this could be reduced to take only faces_1 as input [:,3]
    v4_1: the 4th vertic for every face(triangle) of the source_vertices neutral face (i.e. vertices_1), see [def compute_v4]  [:,3]
    ---------------------------------------
    """
    print ("Computing source rotation...")
    source_rotation = np.zeros((len(faces_1), 3, 3), dtype = np.float32)
        
    for i in range(len(faces_1)):
        
        Vinv = compute_V_inverse_local (vertices_1, faces_1[i], v4_1[i])#Source Neutral Vinv
    
        a = np.array([vertices_2[faces_2[i,1],0:3] - vertices_2[faces_2[i,0],0:3]]) #v2-v1
        b = np.array([vertices_2[faces_2[i,2],0:3] - vertices_2[faces_2[i,0],0:3]]) #v3-v1
        cross = np.cross(a, b)
        c = cross / sqrt(np.linalg.norm(cross)) #v4-v1 
      
        source_rotation[i] = np.dot(np.hstack([a.T, b.T, c.T]), Vinv) # S = np.dot(SE_Vtil, SN_Vinv)
    
    return source_rotation




#Make Deformation Transfer matrix
def makeEd_A(corr, AN_Vertices, AN_Faces, AN_Vinv):
    Ed_A = sp.lil_matrix((len(corr)*9, len(AN_Vertices)*3 + len(AN_Faces)*3), dtype=np.float32)

    for i in range(len(corr)):
        idx_f = corr[i][1]
        e1 = np.sum(AN_Vinv[idx_f,0:3,0])
        e2 = np.sum(AN_Vinv[idx_f,0:3,1])
        e3 = np.sum(AN_Vinv[idx_f,0:3,2])

        Ed_A[i*9, AN_Faces[idx_f,0]*3]     = -e1
        Ed_A[i*9+1, AN_Faces[idx_f,0]*3+1] = -e1
        Ed_A[i*9+2, AN_Faces[idx_f,0]*3+2] = -e1
        Ed_A[i*9+3, AN_Faces[idx_f,0]*3]   = -e2
        Ed_A[i*9+4, AN_Faces[idx_f,0]*3+1] = -e2
        Ed_A[i*9+5, AN_Faces[idx_f,0]*3+2] = -e2
        Ed_A[i*9+6, AN_Faces[idx_f,0]*3]   = -e3
        Ed_A[i*9+7, AN_Faces[idx_f,0]*3+1] = -e3
        Ed_A[i*9+8, AN_Faces[idx_f,0]*3+2] = -e3

        Ed_A[i*9, AN_Faces[idx_f,1]*3]     = AN_Vinv[idx_f,0,0]
        Ed_A[i*9+1, AN_Faces[idx_f,1]*3+1] = AN_Vinv[idx_f,0,0]
        Ed_A[i*9+2, AN_Faces[idx_f,1]*3+2] = AN_Vinv[idx_f,0,0]
        Ed_A[i*9+3, AN_Faces[idx_f,1]*3]   = AN_Vinv[idx_f,0,1]
        Ed_A[i*9+4, AN_Faces[idx_f,1]*3+1] = AN_Vinv[idx_f,0,1]
        Ed_A[i*9+5, AN_Faces[idx_f,1]*3+2] = AN_Vinv[idx_f,0,1]
        Ed_A[i*9+6, AN_Faces[idx_f,1]*3]   = AN_Vinv[idx_f,0,2]
        Ed_A[i*9+7, AN_Faces[idx_f,1]*3+1] = AN_Vinv[idx_f,0,2]
        Ed_A[i*9+8, AN_Faces[idx_f,1]*3+2] = AN_Vinv[idx_f,0,2]

        Ed_A[i*9, AN_Faces[idx_f,2]*3]     = AN_Vinv[idx_f,1,0]
        Ed_A[i*9+1, AN_Faces[idx_f,2]*3+1] = AN_Vinv[idx_f,1,0]
        Ed_A[i*9+2, AN_Faces[idx_f,2]*3+2] = AN_Vinv[idx_f,1,0]
        Ed_A[i*9+3, AN_Faces[idx_f,2]*3]   = AN_Vinv[idx_f,1,1]
        Ed_A[i*9+4, AN_Faces[idx_f,2]*3+1] = AN_Vinv[idx_f,1,1]
        Ed_A[i*9+5, AN_Faces[idx_f,2]*3+2] = AN_Vinv[idx_f,1,1]
        Ed_A[i*9+6, AN_Faces[idx_f,2]*3]   = AN_Vinv[idx_f,1,2]
        Ed_A[i*9+7, AN_Faces[idx_f,2]*3+1] = AN_Vinv[idx_f,1,2]
        Ed_A[i*9+8, AN_Faces[idx_f,2]*3+2] = AN_Vinv[idx_f,1,2]

        Ed_A[i*9, idx_f*3+len(AN_Vertices)*3]     = AN_Vinv[idx_f,2,0]
        Ed_A[i*9+1, idx_f*3+len(AN_Vertices)*3+1] = AN_Vinv[idx_f,2,0]
        Ed_A[i*9+2, idx_f*3+len(AN_Vertices)*3+2] = AN_Vinv[idx_f,2,0]
        Ed_A[i*9+3, idx_f*3+len(AN_Vertices)*3]   = AN_Vinv[idx_f,2,1]
        Ed_A[i*9+4, idx_f*3+len(AN_Vertices)*3+1] = AN_Vinv[idx_f,2,1]
        Ed_A[i*9+5, idx_f*3+len(AN_Vertices)*3+2] = AN_Vinv[idx_f,2,1]
        Ed_A[i*9+6, idx_f*3+len(AN_Vertices)*3]   = AN_Vinv[idx_f,2,2]
        Ed_A[i*9+7, idx_f*3+len(AN_Vertices)*3+1] = AN_Vinv[idx_f,2,2]
        Ed_A[i*9+8, idx_f*3+len(AN_Vertices)*3+2] = AN_Vinv[idx_f,2,2]
            
    return Ed_A


def makeEd_ATc(corr, source_rotation, Ed_A):
    Ed_cVector = sp.lil_matrix((1, len(corr)*9), dtype=np.float32)            
    for i in range(len(corr)):
        Ed_cVector[0, i*9:i*9+9] = (source_rotation[corr[i][0]].T).flatten()
    Ed_cVector = Ed_cVector.T

    return np.dot(Ed_A.T, Ed_cVector)

```

## Source file: landmarks/LARkit.py

{
  "path": "landmarks/LARkit.py",
  "blob_sha": "bb59dea4195154a5a0dbadaeff8fd6a378dd02da",
  "sha256": "f083ad784ca9fc2ef5b8d2a57d655ddaf20fe9ab446a104c7b5d9f523af89066",
  "lines": 26,
  "bytes": 609,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 4,
    "mouth": 6,
    "rig": 4
  },
  "url": "https://github.com/vasiliskatr/deformation_transfer_ARkit_blendshapes/blob/5faaf6a48621cc5b186b2fbfac24d97c1101ef14/landmarks/LARkit.py",
  "function_signature_candidates": []
}

```text

import numpy as np
LM = np.zeros(15).astype(int)
### Indices for key landmarks
LM[0] = 360 #lm_nose_tip = 30 

LM[1] = 1191 #lm_right_eye_R = 45 
LM[2] =  1203#lm_right_eye_L = 42

LM[3] =  606#lm_left_eye_L = 36 
LM[4] =  436#lm_left_eye_R = 39

LM[5] =  140#lm_mouth_L = 48
LM[6] =  824#lm_mouth_R = 54

LM[7] =  223#lm_mouth_middle_Top_T = 51
LM[8] = 776#lm_mouth_middle_Top_B = 62

LM[9] =  767#lm_mouth_middle_Bottom_T = 66
LM[10] =  242#lm_mouth_middle_Bottom_B = 57


LM [11] =  546# boundary_left
LM [12] =  1162 # boundary_right
LM [13] =  509# boundary_left_top
LM [14] =  1080# boundary_right_top



```

## Source file: landmarks/__init__.py

{
  "path": "landmarks/__init__.py",
  "blob_sha": "8b137891791fe96927ad78e64b0aad7bded08bdc",
  "sha256": "01ba4719c80b6fe911b091a7c05124b64eeece964e09c058ef8f9805daca546b",
  "lines": 1,
  "bytes": 1,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/vasiliskatr/deformation_transfer_ARkit_blendshapes/blob/5faaf6a48621cc5b186b2fbfac24d97c1101ef14/landmarks/__init__.py",
  "function_signature_candidates": []
}

```text




```

## Source file: landmarks/LICT_narrow_r.py

{
  "path": "landmarks/LICT_narrow_r.py",
  "blob_sha": "d057f24c380ef285a042e20ae64db3e062a23a16",
  "sha256": "3121d297623addd7960b3df15beb986d7164c8dc963e361c391e4ece57b8b682",
  "lines": 32,
  "bytes": 913,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 8,
    "mouth": 6,
    "rig": 4
  },
  "url": "https://github.com/vasiliskatr/deformation_transfer_ARkit_blendshapes/blob/5faaf6a48621cc5b186b2fbfac24d97c1101ef14/landmarks/LICT_narrow_r.py",
  "function_signature_candidates": []
}

```text

import numpy as np
LM = np.zeros(18).astype(int)
### Indices for key landmarks
LM[0] = 5069 #lm_nose_tip 

LM[1] = 3888#lm_right_eye_outer 3888
LM[2] = 3621 #lm_right_eye_inner (tears) 3621

LM[3] = 1244  #lm_left_eye_inner (tears) 1244
LM[4] = 2023 #lm_left_eye_outer 2023

LM[5] = 6128#lm_mouth_R 
LM[6] = 5567  #lm_mouth_L 5567

LM[7] = 6323#lm_mouth_middle_Top_T 
LM[8] = 6248 #lm_mouth_middle_Top_B

LM[9] = 6399 #lm_mouth_middle_Bottom_T 
LM[10] = 6414 #lm_mouth_middle_Bottom_B 

LM[11] = 3138#center_chin

# Use edge vertices for reigid alignment (the less the better)
LM[12] =  2141 # top_of_mask_mid (edge vertice)
LM[13] =  1844 # mask_edge_Right_eyeheight (edge vertice)
LM[14] = 4063 # mask_edge_left_eyeheight (edge vertice)
LM[15] = 829# mask_edge_right_beloweyeheight (edge vertice)
LM[16] = 3083# mask_edge_left_beloweyeheight (edge vertice)
LM[17] = 2003# bottom_mask_neck)mid (edge vertice)





```
