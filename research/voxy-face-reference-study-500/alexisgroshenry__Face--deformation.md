# Facial rendering source scan: alexisgroshenry/Face--deformation

Repository: https://github.com/alexisgroshenry/Face--deformation

Pinned source revision: 2febef051de237fab9ea16d47d581067d1065187

Discovery topic: face deformation

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"deform": ["deform.py"], "face": ["code/__pycache__/face_belong.cpython-37.pyc", "code/face_belong.py"]}

## Source file: deform.py

{
  "path": "deform.py",
  "blob_sha": "fd0ffeb214d873731277ac5c694d7bde7fe9eef6",
  "sha256": "90a3bb50fe3975905f7d48d92926f4463bafff41041404e66157ace430c06319",
  "lines": 169,
  "bytes": 5581,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 4,
    "jaw": 4,
    "mouth": 1,
    "gum": 3,
    "deform": 9,
    "rig": 8,
    "face": 2
  },
  "url": "https://github.com/alexisgroshenry/Face--deformation/blob/2febef051de237fab9ea16d47d581067d1065187/deform.py",
  "function_signature_candidates": [
    {
      "line": 28,
      "text": "def deformation() :"
    },
    {
      "line": 122,
      "text": "def add_points(points,part=\"jaw\") :"
    },
    {
      "line": 155,
      "text": "def good_points(origine,goal) :"
    }
  ]
}

```text

# import the necessary packages
import argparse
from imutils import face_utils
import numpy as np
import imutils
import dlib
import cv2
from math import *
import matplotlib.pyplot as plt
from numpy.linalg import inv
from scipy.spatial import Delaunay
from scipy import ndimage
from scipy.interpolate import interp1d
import sys
sys.path.insert(1, './code')

from drawing_extract import *
from image_extract import *
from face_belong import *

parser = argparse.ArgumentParser()
parser.add_argument("-i","--image", help="image we want to deform",
                    type=str)
parser.add_argument("-d","--drawing", help="drawing we want to deform",
                    type=str)
args = parser.parse_args()

def deformation() :
        # init the drawing and picture
    print("Loading image and drawing")
    dessin = cv2.imread('drawings/{}'.format(args.drawing)) 
    image = cv2.imread("images/{}".format(args.image))
    # resize
    image = cv2.resize(image,dessin.shape[:2][::-1])

    print("Get the extremities of drawing")
    res = get_extremities(dessin)
    ext = []
    for k in res : 
        current_ext = []
        for key,values in res[k].items() :
            if values == 1 :
                current_ext.append(key)
        # how to choose? closest to (0,0)
        norm0 = np.linalg.norm(np.array(current_ext[0]))
        norm1 = np.linalg.norm(np.array(current_ext[1]))
        if norm0 > norm1 :
            ext.append(current_ext[1])
        else : 
            ext.append(current_ext[0]) 

    print("Labelling the drawing")
    res_d = compute_init_drawing(dessin,ext)

    print("Extracting image")
    res_im = compute_init_pict(image)
    res_im = add_hair(res_im)

    print("Interpolation") 
    points_tot = {}
    for k in res_d :
        points_tot[k] = {}
        points_tot[k]["drawing"] = res_d[k]
        if k == 'mouth' :
            points_tot[k]["image"] = add_points(res_im[k]["points"][:12],part=k)
        else :
            points_tot[k]["image"] = add_points(res_im[k]["points"],part=k)

    print("Same amount of points")
    for k in points_tot : 
        points_tot[k]["drawing"] = good_points(points_tot[k]["drawing"],points_tot[k]["image"])

    print("Stack points")
    image_points = []
    drawing_points = []
    for k in points_tot :
        for i in points_tot[k]["image"] :
            image_points.append(i)
        for j in points_tot[k]["drawing"] :
            drawing_points.append(j)
    
    print("Delaunay triangulation")
    points =np.array([list(coords) for coords in image_points])
    drawing_points = np.array([list(coords) for coords in drawing_points])

    # compute Delaunay method
    tri = Delaunay(points)

    print("Compute final deformation") 
    #result matrix
    deformed = np.zeros(dessin.shape)

    for i in range(dessin.shape[1]):
        if i%50 == 0 :
            print("Column {} over {}".format(i,dessin.shape[1]))
        for j in range(dessin.shape[0]):
            for f in tri.simplices:
                l=list(drawing_points[v] for v in list(f))
                l2 = list(points[v] for v in list(f))
                if inside_convex_polygon([i,j],l):
                    coefs=get_coefs([i,j],l)
                    x = int(sum(coefs[i]*l2[i][0] for i in range(3) ))
                    y = int(sum(coefs[i]*l2[i][1] for i in range(3) ))
                    # a cause des arrondis, sometimes ca dépasse
                    if x > dessin.shape[1]-1 :
                        x = dessin.shape[1] -1
                    if y > dessin.shape[0]-1 :
                        y = dessin.shape[0] - 1
                    deformed[j,i]=image[y,x]
                    break
    deformed = deformed.astype("uint8")

    print("Saving image")
    name = args.image.split(".")[0] + "-" + args.drawing.split(".")[0]
    cv2.imwrite("result/{}.png".format(name),deformed)

    return None


# add points according to cubic interpolation
# not so good for now
def add_points(points,part="jaw") :
    if part == "hair" or part == "left_eybrow" or part == "right_eyebrow" :
        return points
    x = []
    y = []
    for a,b in points:
        x.append(a)
        y.append(b)
    if not (part in ["jaw",'left_eyebrow','right_eyebrow']) :
        x.append(points[0][0])
        y.append(points[0][1])
    t = [k for k in range(len(x))]
    new_points = []
    fx = interp1d(t, x, kind='cubic')
    fy = interp1d(t, y, kind='cubic')
    for i in t : 
        try : 
            # last point in the first point if we circle
            if i != len(t)-1 and not(part in ["jaw",'left_eyebrow','right_eyebrow']) :
                x_x = fx(i)
                y_y = fy(i)
                new_points.append([int(x_x),int(y_y)])
            elif (part in ["jaw",'left_eyebrow','right_eyebrow']) : 
                x_x = fx(i)
                y_y = fy(i)
                new_points.append([int(x_x),int(y_y)])
            x_x = fx(i+0.5)
            y_y = fy(i+0.5)
            new_points.append([int(x_x),int(y_y)])
        except :
            a = 0 # actually not a mistake, but I did not know what to say
    return new_points

def good_points(origine,goal) :
    # we have origine points (greater number) and want to obtain as much points as in goal
    n = len(goal)
    count = len(origine)
    obj = []
    interval = count//n
    for i in range(n) :
        obj.append(origine[i*interval])
    return obj


    


deformation()


```

## Source file: code/face_belong.py

{
  "path": "code/face_belong.py",
  "blob_sha": "662511ac0d9397e9ccba6e8e08093cd61101212c",
  "sha256": "e40ba06bdc406e2edfcb1074e7e8ced0deddcd612ac5bd9d88d79295af7f6792",
  "lines": 97,
  "bytes": 3450,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 2,
    "face": 4
  },
  "url": "https://github.com/alexisgroshenry/Face--deformation/blob/2febef051de237fab9ea16d47d581067d1065187/code/face_belong.py",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "def get_min_max(x,y) :"
    },
    {
      "line": 13,
      "text": "def inside_convex_polygon(point, vertices):"
    },
    {
      "line": 36,
      "text": "def on_convexe_polygone(point,vertices) :"
    },
    {
      "line": 45,
      "text": "def get_side(a, b):"
    },
    {
      "line": 54,
      "text": "def v_sub(a, b):"
    },
    {
      "line": 57,
      "text": "def cosine_sign(a, b):"
    },
    {
      "line": 60,
      "text": "def get_face(point,F,V) :"
    },
    {
      "line": 67,
      "text": "def get_coefs(point,vertices) :"
    }
  ]
}

```text

import numpy as np
from numpy.linalg import inv

RIGHT = "RIGHT"
LEFT = "LEFT"

def get_min_max(x,y) :
    if x>y : 
        return y,x
    return x,y

# whether the point in inside the polygone, here triangle
def inside_convex_polygon(point, vertices):
    # inside the triangle
    previous_side = None
    n_vertices = len(vertices)
    for n in range(n_vertices):
        a, b = vertices[n], vertices[(n+1)%n_vertices]
        affine_segment = v_sub(b, a)
        affine_point = v_sub(point, a)
        current_side = get_side(affine_segment, affine_point)
        if current_side is None:
            #outside or over an edge
            # check if over an edge
            min_x,max_x = get_min_max(a[0],b[0])
            min_y,max_y = get_min_max(a[1],b[1])
            if point[0] >= min_x and point[0] <= max_x and point[1]>=min_y and point[1] <= max_y :
                return True
            return False #outside
        elif previous_side is None: #first segment
            previous_side = current_side
        elif previous_side != current_side:
            return False
    return True

def on_convexe_polygone(point,vertices) : 
    # on the vertices or edges
    for i in range(len(vertices)) : # will be 3 cause it is a triangle
        if (vertices[i][0] - vertices[(i+1)%len(vertices)][0]) == 0 and vertices[i][1]== point[1] : 
            return False
        if point[1] == (vertices[i][1] - vertices[(i+1)%len(vertices)][1]) / (vertices[i][0] - vertices[(i+1)%len(vertices)][0]) * (point[0] -vertices[i][0]) + vertices[i][1] : 
            return True # aka the point is on the edge betwen vertice[i] and vertice[i+1]
    return False

def get_side(a, b):
    x = cosine_sign(a, b)
    if x < 0:
        return LEFT
    elif x > 0: 
        return RIGHT
    else:
        return None

def v_sub(a, b):
    return (a[0]-b[0], a[1]-b[1])

def cosine_sign(a, b):
    return a[0]*b[1]-a[1]*b[0]

def get_face(point,F,V) :
    # given a point, return the face in which he is contained
    for ind,k in enumerate(F) :
        if inside_convex_polygon(point,[V[k[0]],V[k[1]],V[k[2]]]) :
            return ind 

# point = alpha * point[0] + beta * point[1] + gamma * point[2]
def get_coefs(point,vertices) :
    # one point and the three vertices
    M = np.ones((3,3))
    M[1,0] = vertices[0][0]
    M[1,1] = vertices[1][0]
    M[1,2] = vertices[2][0]
    
    M[2,0] = vertices[0][1]
    M[2,1] = vertices[1][1]
    M[2,2] = vertices[2][1]
    
    Y = np.zeros((3,1))
    Y[0,0] = 1
    Y[1,0] = point[0]
    Y[2,0] = point[1]
    try :
        return inv(M) @ Y
    except :
        # the face is flat
        # If the face is flat, which point are more important?
        default = np.zeros((3,1))
        default[0,0] = np.linalg.norm(np.array(vertices[0])-np.array(point))
        default[1,0] = np.linalg.norm(np.array(vertices[1])-np.array(point))
        default[2,0] = np.linalg.norm(np.array(vertices[2])-np.array(point))
        norm = np.linalg.norm(np.array(vertices[0])-np.array(point)) + np.linalg.norm(np.array(vertices[1])-np.array(point)) +np.linalg.norm(np.array(vertices[2])-np.array(point))
        default = default / norm
        default[0,0] = 1-default[0,0]
        default[1,0] = 1-default[1,0]
        default[2,0] = 1-default[2,0]
        default = default / np.linalg.norm(default)
        return default

```

## Source file: code/image_extract.py

{
  "path": "code/image_extract.py",
  "blob_sha": "58915521b80ab01d22cfd57ed4b600aad06d2251",
  "sha256": "6c835e19d99ef77128bf2c656efa4e5e51d60924d4227e31c7a4d44e19f14c62",
  "lines": 87,
  "bytes": 3041,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 2,
    "jaw": 4,
    "mouth": 3,
    "rig": 8,
    "face": 12
  },
  "url": "https://github.com/alexisgroshenry/Face--deformation/blob/2febef051de237fab9ea16d47d581067d1065187/code/image_extract.py",
  "function_signature_candidates": [
    {
      "line": 6,
      "text": "def rect_to_bb(rect):"
    },
    {
      "line": 17,
      "text": "def shape_to_np(shape, dtype=\"int\"):"
    },
    {
      "line": 27,
      "text": "def compute_init_pict(image) :"
    },
    {
      "line": 57,
      "text": "def get_bottom(L) :"
    },
    {
      "line": 65,
      "text": "def add_hair(res_im) :"
    }
  ]
}

```text

import dlib
import cv2
from imutils import face_utils
import numpy as np

def rect_to_bb(rect):
    # take a bounding predicted by dlib and convert it
    # to the format (x, y, w, h) as we would normally do
    # with OpenCV
    x = rect.left()
    y = rect.top()
    w = rect.right() - x
    h = rect.bottom() - y
    # return a tuple of (x, y, w, h)
    return (x, y, w, h)

def shape_to_np(shape, dtype="int"):
    # initialize the list of (x, y)-coordinates
    coords = np.zeros((68, 2), dtype=dtype)
    # loop over the 68 facial landmarks and convert them
    # to a 2-tuple of (x, y)-coordinates
    for i in range(0, 68):
        coords[i] = (shape.part(i).x, shape.part(i).y)
    # return the list of (x, y)-coordinates
    return coords

def compute_init_pict(image) :
    # initialize dlib's face detector (HOG-based) and then create
    # the facial landmark predictor
    detector = dlib.get_frontal_face_detector()
    predictor = dlib.shape_predictor("model/shape_predictor_68_face_landmarks.dat")
    # load the input image, resize it, and convert it to grayscale
    gray = cv2.cvtColor(image, cv2.COLOR_BGR2GRAY)
    # detect faces in the grayscale image
    rects = detector(gray, 1)

    # loop over the face detections
    res = {}
    for rect in rects:
        # determine the facial landmarks for the face region, then
        # convert the landmark (x, y)-coordinates to a NumPy array
        shape = predictor(gray, rect)
        shape = face_utils.shape_to_np(shape)
        # loop over the face parts individually
        for (name, (i, j)) in face_utils.FACIAL_LANDMARKS_IDXS.items():
            res[name] = {}
            res[name]["points"] = []
            # clone the original image so we can draw on it, then
            # display the name of the face part on the image
            for (indice,(x, y)) in enumerate(shape[i:j]):
                res[name]["points"].append(shape[i:j][indice])

    mat_res = np.zeros(image.shape[:2])

    return res

def get_bottom(L) :
    # return the bottomest point
    bot = L[0][1]
    for k in L :
        if k[1]>bot :
            bot = k[1]
    return bot

def add_hair(res_im) :
    # coordinates of the face
    mid = res_im["jaw"]["points"][0][1]
    left = res_im["jaw"]["points"][0][0]
    right = res_im["jaw"]["points"][-1][0]
    left1 = res_im["right_eye"]["points"][0][0]
    right1 = res_im["left_eye"]["points"][3][0]
    bottom = get_bottom(res_im["jaw"]["points"])
    middle_mouth = res_im["mouth"]["points"][3][0]
    dist = bottom - mid
    
    # points
    top_left = [left,mid-dist]
    top_right = [right,mid-dist]
    hair_left = [left1,int(mid-0.8*dist)]
    hair_right = [right1,int(mid-0.8*dist)]
    top_hair = [middle_mouth,int(mid-1.3*dist)]
    bottom_hair = [middle_mouth,int(mid-0.8*dist)]
    hair = [top_left,top_hair,top_right,hair_right,bottom_hair,hair_left]
    
    res_im["hair"] = {}
    res_im["hair"]["points"] = hair
    return res_im

```

## Source file: code/drawing_extract.py

{
  "path": "code/drawing_extract.py",
  "blob_sha": "41eff610f74ceadfa4f009f0999682616d0a6926",
  "sha256": "8618d674c115620e0a93305cf7ae259a4c2d6e512d09500fb1308b3b1d684b5e",
  "lines": 112,
  "bytes": 4030,
  "selection_score": 0,
  "keyword_line_counts": {
    "skin": 1,
    "rig": 1
  },
  "url": "https://github.com/alexisgroshenry/Face--deformation/blob/2febef051de237fab9ea16d47d581067d1065187/code/drawing_extract.py",
  "function_signature_candidates": [
    {
      "line": 6,
      "text": "def find_feature_ext(father,i,j,nb_features,union,res_dr,points_dr,dessin):"
    },
    {
      "line": 21,
      "text": "def get_extremities(dessin) :"
    },
    {
      "line": 51,
      "text": "def get_neighbors(i,j,dessin) :"
    },
    {
      "line": 64,
      "text": "def find_feature(father,i,j,nb_features,union,res_draw,res_dr,points_dr,dessin):"
    },
    {
      "line": 80,
      "text": "def compute_init_drawing(dessin,ext) :"
    }
  ]
}

```text

import matplotlib.pyplot as plt
import numpy as np

# get the extremities so as to start at the correct place
# careful for the drawing : extremities must contain only one neighbor
def find_feature_ext(father,i,j,nb_features,union,res_dr,points_dr,dessin):
    # according to a point of coordinate (i,j) we will look whether its 8 neighbors are in the same feature
    grad_x = [-1,0,1]
    grad_y = [-1,0,1]
    n,m = union.shape
    for e1 in grad_x:
        for e2 in grad_y:
            x=i+e1
            y=j+e2
            if m*x+y!=father and dessin[x,y,0]!=255. and union[x,y]==m*x+y:    
                union[x,y]=father
                points_dr[father][x,y] = get_neighbors(x,y,dessin)
                find_feature_ext(father,x,y,nb_features,union,res_dr,points_dr,dessin)
                

def get_extremities(dessin) :
    # given the drawing, will return the number of neighbors for each point that is not white
    
    # the points of the drawing
    points_dr = {}    
    
    # union find for all the features
    union=np.zeros(dessin.shape[:2])
    (n,m)=union.shape
    for i in range(n):
        for j in range(m):
            union[i,j]=m*i+j
            

    res_dr = {} # temporary contains the points, that still need labeling before going into res
    nb_features=0
    for j in range(m):
        for i in range(n):
            if dessin[i,j,0]==255.:
                union[i,j]=0
            #premier pixel d'un feature
            elif union[i,j]==m*i+j:
                points_dr[m*i+j] = {}
                points_dr[m*i+j][i,j] = get_neighbors(i,j,dessin)
                
                find_feature_ext(m*i+j,i,j,nb_features,union,res_dr,points_dr,dessin)
                res_dr[str(nb_features)] = points_dr[m*i+j]
                nb_features+=1
    return res_dr

def get_neighbors(i,j,dessin) :
    # numbers of neighbors of the (i,j) point
    S = -1
    neighbors = []
    for e1 in [-1,0,1] :
        for e2 in [-1,0,1] : 
            if dessin[i+e1,j+e2,0] != 255 :
                S +=1
                neighbors.append([i+e1,j+e2])
    
    return S


def find_feature(father,i,j,nb_features,union,res_draw,res_dr,points_dr,dessin):
    # according to a point we know is on the feature identified as *father*, we check if its neighbors are also.
    grad_x = [-1,0,1]
    grad_y = [-1,0,1]
    n,m = union.shape
    for e1 in grad_x:
        for e2 in grad_y:
            x=i+e1
            y=j+e2
            if m*x+y!=father and dessin[x,y,0]!=255. and union[x,y]==m*x+y:    
                union[x,y]=father
                points_dr[father].append([y,x])
                res_draw[str(nb_features)][x,y] = 255
                find_feature(father,x,y,nb_features,union,res_draw,res_dr,points_dr,dessin)
                

def compute_init_drawing(dessin,ext) :
    # the points of the drawing
    points_dr = {}    
    
    # union find for all the features
    union=np.zeros(dessin.shape[:2])
    (n,m)=union.shape
    for i in range(n):
        for j in range(m):
            union[i,j]=m*i+j
            
    res = {} # final result, after getting the right label
    res_draw = {} # contains one image per feature, to help labeling
    res_dr = {} # temporary contains the points, that still need labeling before going into res
    nb_features = 0
    
    # we will look for each feature that we found previously
    for x,y in ext :
        points_dr[m*x+y] = [[y,x]]
        res_draw[str(nb_features)] = np.zeros(union.shape)
        res_draw[str(nb_features)][x,y] = 255
        find_feature(m*x+y,x,y,nb_features,union,res_draw,res_dr,points_dr,dessin)
        res_dr[str(nb_features)] = points_dr[m*x+y]
        nb_features+=1
        
    # asking which part it is            
    for k in res_draw :
        plt.imshow(res_draw[k])
        plt.show()
        feature = input("Which feature is it ?")
        res[feature] = res_dr[k]
        
    return res

```
