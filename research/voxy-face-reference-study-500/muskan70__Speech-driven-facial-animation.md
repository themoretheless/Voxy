# Facial rendering source scan: muskan70/Speech-driven-facial-animation

Repository: https://github.com/muskan70/Speech-driven-facial-animation

Pinned source revision: 3510c72477bb730dae7bb119411f6e439eaa1cc1

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"face": ["sample/Example/frontalfacedata/frame0.jpg", "sample/Example/frontalfacedata/frame1.jpg", "sample/Example/frontalfacedata/frame10.jpg", "sample/Example/frontalfacedata/frame100.jpg", "sample/Example/frontalfacedata/frame101.jpg", "sample/Example/frontalfacedata/frame102.jpg", "sample/Example/frontalfacedata/frame103.jpg", "sample/Example/frontalfacedata/frame104.jpg", "sample/Example/frontalfacedata/frame105.jpg", "sample/Example/frontalfacedata/frame106.jpg", "sample/Example/frontalfacedata/frame107.jpg", "sample/Example/frontalfacedata/frame108.jpg", "sample/Example/frontalfacedata/frame109.jpg", "sample/Example/frontalfacedata/frame11.jpg", "sample/Example/frontalfacedata/frame110.jpg", "sample/Example/frontalfacedata/frame111.jpg", "sample/Example/frontalfacedata/frame112.jpg", "sample/Example/frontalfacedata/frame113.jpg", "sample/Example/frontalfacedata/frame114.jpg", "sample/Example/frontalfacedata/frame115.jpg", "sample/Example/frontalfacedata/frame116.jpg", "sample/Example/frontalfacedata/frame117.jpg", "sample/Example/frontalfacedata/frame118.jpg", "sample/Example/frontalfacedata/frame119.jpg", "sample/Example/frontalfacedata/frame12.jpg", "sample/Example/frontalfacedata/frame120.jpg", "sample/Example/frontalfacedata/frame121.jpg", "sample/Example/frontalfacedata/frame122.jpg", "sample/Example/frontalfacedata/frame123.jpg", "sample/Example/frontalfacedata/frame124.jpg"]}

## Source file: src/SysUtils.py

{
  "path": "src/SysUtils.py",
  "blob_sha": "aeba3ae440029a805664d93f36376cea4edcec94",
  "sha256": "1544bb4fc54bef9fba33a33914dc1fed653ef43d188480c8737fec6b4e38e6cf",
  "lines": 93,
  "bytes": 3339,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 8
  },
  "url": "https://github.com/muskan70/Speech-driven-facial-animation/blob/3510c72477bb730dae7bb119411f6e439eaa1cc1/src/SysUtils.py",
  "function_signature_candidates": [
    {
      "line": 6,
      "text": "def is_Win32():"
    },
    {
      "line": 9,
      "text": "def make_dir(dir_path, exist_ok=True):"
    },
    {
      "line": 12,
      "text": "def get_current_time_string():"
    },
    {
      "line": 15,
      "text": "def get_items(folder, what_to_get=None):"
    },
    {
      "line": 28,
      "text": "def get_path(args):"
    },
    {
      "line": 32,
      "text": "def get_filename(path):"
    },
    {
      "line": 35,
      "text": "def get_extension(path):"
    },
    {
      "line": 38,
      "text": "def get_parent_dir(path):"
    },
    {
      "line": 41,
      "text": "def read_learning_rate(filename):"
    },
    {
      "line": 60,
      "text": "def __init__(self):"
    },
    {
      "line": 70,
      "text": "def prepare(self):"
    },
    {
      "line": 80,
      "text": "def parse(self):"
    }
  ]
}

```text

import pathlib as plb
import datetime
import sys
import argparse

def is_Win32():
    return sys.platform.startswith("win")

def make_dir(dir_path, exist_ok=True):
    plb.Path(dir_path).mkdir(parents=True, exist_ok=exist_ok)

def get_current_time_string():
    return datetime.datetime.now().strftime('%Y-%m-%d-%H-%M')

def get_items(folder, what_to_get=None):
    if what_to_get is None:
        what_to_get = "stem"
    if what_to_get not in ["stem", "name", "full"]:
        raise ValueError("Incorrect flag")
    path = plb.Path(folder)
    if what_to_get == "stem":
        return [item.stem for item in path.iterdir()]
    elif what_to_get == "name":
        return [item.name for item in path.iterdir()]
    elif what_to_get == "full":
        return [str(item) for item in path.iterdir()]

def get_path(args):
    return "/".join(args)
    

def get_filename(path):
    return plb.Path(path).stem

def get_extension(path):
    return plb.Path(path).suffix

def get_parent_dir(path):
    return plb.Path(path).parent

def read_learning_rate(filename):
    if not plb.Path(filename).exists():
        return None
    lr = []
    with open(filename, "r") as f:
        for line in f:
            tokens = line.split()
            if len(tokens) > 2:
                raise ValueError("there are more than 2 entries")
            if len(tokens) == 0:
                continue
            elif len(tokens) == 1:
                lr += [float(tokens[0])]
            elif len(tokens) == 2:
                lr += [float(tokens[0])]*int(tokens[1])
    return lr


class ArgParser(object):
    def __init__(self):
        self.config = {}
        self.config["num_epochs"] = 300
        self.config["minibatch_size"] = 128
        self.config["epoch_size"] = 50000
        self.config["constlr"] = True
        self.config["lr"] = 0.0001
        self.config["lr_list"] = None
        self.config["momentum"] = 0.9

    def prepare(self):
        self.parser = argparse.ArgumentParser(description="Process input arguments for training")
        self.parser.add_argument("--epoch", type=int, default=self.config["num_epochs"])
        self.parser.add_argument("--minibatch", type=int, default=self.config["minibatch_size"])
        self.parser.add_argument("--epoch_size", type=int, default=self.config["epoch_size"])
        self.parser.add_argument("--constlr", action="store_true")
        self.parser.add_argument("--lr", type=float, default=self.config["lr"])
        self.parser.add_argument("--momentum", type=float, default=self.config["momentum"])
        self.parser.add_argument("--lr_file", type=str)

    def parse(self):
        self.args = self.parser.parse_args()
        self.config["num_epochs"] = self.args.epoch
        self.config["minibatch_size"] = self.args.minibatch
        self.config["epoch_size"] = self.args.epoch_size
        self.config["constlr"] = self.args.constlr
        self.config["lr"] = self.args.lr
        self.config["momentum"] = self.args.momentum

        if self.config["num_epochs"] < 1 or self.config["minibatch_size"] < 1 or self.config["epoch_size"] < 1:
            raise ValueError("number of minibatches, epochs or minibatch size must be positive")

        if (not self.config["constlr"]) and self.args.lr_file:
            self.config["lr_list"] = read_learning_rate(self.args.lr_file)


```

## Source file: src/ShapeUtils.py

{
  "path": "src/ShapeUtils.py",
  "blob_sha": "1fb455a615a0006a462e34088145fb5486eac51a",
  "sha256": "f9899eceba9aab26bd354a02eaed3c11e2ac229222b091561355992f70eca357",
  "lines": 268,
  "bytes": 9061,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 4,
    "rig": 1,
    "lip": 3
  },
  "url": "https://github.com/muskan70/Speech-driven-facial-animation/blob/3510c72477bb730dae7bb119411f6e439eaa1cc1/src/ShapeUtils.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def load_processed_baseshapes(filename=\"\"):"
    },
    {
      "line": 18,
      "text": "def load_triangles(filename=\"\"):"
    },
    {
      "line": 26,
      "text": "def calc_shape(allshapes, e):"
    },
    {
      "line": 39,
      "text": "def calc_all_shapes(allshapes, Es):"
    },
    {
      "line": 47,
      "text": "def transform_shape(shape, R=None, T=None):"
    },
    {
      "line": 56,
      "text": "def calc_vertex_normals(vertices, triangles):"
    },
    {
      "line": 58,
      "text": "def normalize_v3(arr):"
    },
    {
      "line": 77,
      "text": "def __init__(self, width=640, height=480, caption=\"Renderer\"):"
    },
    {
      "line": 84,
      "text": "def initGL(self):"
    },
    {
      "line": 90,
      "text": "def render(self, shape, triangles, normals=None, text=\"\"):"
    },
    {
      "line": 171,
      "text": "def capture_screen(self):"
    },
    {
      "line": 183,
      "text": "def get_3D_render(self, shape, triangles):"
    },
    {
      "line": 187,
      "text": "def exit(self):"
    },
    {
      "line": 193,
      "text": "def __init__(self, draw_error=True):"
    },
    {
      "line": 199,
      "text": "def visualize(self, image, e_real, e_fake):"
    },
    {
      "line": 223,
      "text": "def exit(self):"
    },
    {
      "line": 226,
      "text": "def restart(self):"
    },
    {
      "line": 231,
      "text": "def draw_error_bar_plot(e_real, e_fake, final_size=(900,100)):"
    }
  ]
}

```text

import numpy as np
import math

import pyglet
from pyglet.gl import *
import ctypes
from PIL import Image
import cv2


def load_processed_baseshapes(filename=""):
    if filename:
        baseshapes = np.load(filename)
    else:
        baseshapes = np.load("../shapes/baseshapes.npy")
    return baseshapes

def load_triangles(filename=""):
    if filename:
        triangles = np.load(filename)
    else:
        triangles = np.load("../shapes/triangles.npy")
    return triangles


def calc_shape(allshapes, e):
    assert(e.size == 46)
    # e is a list or np.array vector
    e = np.array(e).astype(np.float32)
    full_e = np.ones(e.size+1, dtype=np.float32)
    full_e[1:] = e
    shape = full_e @ allshapes
    # reshape to numVer*3
    numVert = int(shape.size / 3)
    ret_shape = np.reshape(shape, (numVert,3))
    return ret_shape


def calc_all_shapes(allshapes, Es):
    shapes = []
    for e in Es:
        shape = cal_shape(allshapes, e)
        shapes.append(shape)
    return shapes


def transform_shape(shape, R=None, T=None):
    ret_shape = np.copy(shape)
    if R is not None:
        ret_shape = ret_shape @ R.transpose()
    if T is not None:
        ret_shape = np.add(ret_shape, T)
    return ret_shape


def calc_vertex_normals(vertices, triangles):

    def normalize_v3(arr):
        lens = np.sqrt( arr[:,0]**2 + arr[:,1]**2 + arr[:,2]**2 )
        arr[:,0] /= lens
        arr[:,1] /= lens
        arr[:,2] /= lens                
        return arr

    norm = np.zeros(vertices.shape, dtype=vertices.dtype)
    tris = vertices[triangles]
    n = np.cross(tris[: :,1] - tris[: :,0], tris[: :,2] - tris[: :,0])
    normalize_v3(n)
    norm[triangles[:,0]] += n
    norm[triangles[:,1]] += n
    norm[triangles[:,2]] += n
    normalize_v3(norm)
    return norm


class Renderer(object):
    def __init__(self, width=640, height=480, caption="Renderer"):
        self.win = pyglet.window.Window(width=width, height=height, caption=caption, visible=False)
        self.width = width
        self.height = height
        self.initGL()
        self.buffer = (GLubyte * (3*width*height))(0)

    def initGL(self):
        glClearColor(1.0, 1.0, 1.0, 1.0)
        glClearDepth(1.0)
        glShadeModel(GL_SMOOTH)
        glPolygonMode(GL_FRONT_AND_BACK, GL_FILL)

    def render(self, shape, triangles, normals=None, text=""):
        self.win.dispatch_events()

        if normals is None:
            normals = calc_vertex_normals(shape, triangles)

        glClear(GL_COLOR_BUFFER_BIT|GL_DEPTH_BUFFER_BIT)
        glClearColor(1.0, 1.0, 1.0, 1.0)
         
        # render shape
        glEnable(GL_DEPTH_TEST)
        glEnable(GL_LIGHTING)
        glEnable(GL_LIGHT0)
        ambient_light = (GLfloat*4)(*[0.25, 0.25, 0.25, 1.0])
        diffuse_light = (GLfloat*4)(*[0.7, 0.7, 0.7, 1.0])
        light_pos = (GLfloat*3)(*[-1000.0, 0.0, 200000.0])
        glLightfv(GL_LIGHT0, GL_AMBIENT, ambient_light)
        glLightfv(GL_LIGHT0, GL_DIFFUSE, diffuse_light)
        glLightfv(GL_LIGHT0, GL_POSITION, light_pos)        

        diffuse_material = (GLfloat*4)(*[0.5, 0.5, 0.5, 1.0])
        glEnable(GL_COLOR_MATERIAL)
        glColorMaterial(GL_FRONT, GL_AMBIENT_AND_DIFFUSE)
        glMaterialfv(GL_FRONT, GL_DIFFUSE, diffuse_material)

        # setup projection
        glMatrixMode(GL_PROJECTION)
        glLoadIdentity()
        gluPerspective(45.0, float(self.width)/float(self.height), 0.1, 100.0)
        glMatrixMode(GL_MODELVIEW)
        glLoadIdentity()
        glTranslatef(0.0, 0.0, -4.0)

        # vertex pointer & drawarray
        glEnableClientState(GL_VERTEX_ARRAY)
        glEnableClientState(GL_NORMAL_ARRAY)
        va = shape[triangles].flatten().astype(ctypes.c_float)
        no = normals[triangles].flatten().astype(ctypes.c_float)
        nTri = triangles.shape[0]
        glVertexPointer(3, GL_FLOAT, 0, va.ctypes.data)
        glNormalPointer(GL_FLOAT, 0, no.ctypes.data)
        glColor3f(0.81, 0.59, 0.49)
        glDrawArrays(GL_TRIANGLES, 0, nTri*3)

        # render the eyebrow
        #glColor3f(172.0/255, 86.0/255, 57.0/255)
        glColor3f(0.67, 0.34, 0.22)
        #left_indices = [7187, 9427, 9429, 2139, 7170, 9414]
        #right_indices = [713, 10870, 4451, 4293, 4275, 4271]
        eyebrow_tris = np.array([   [9427, 7187, 9414],
                                    [9427, 9414, 7170],
                                    [9427, 7170, 9429],
                                    [9429, 7170, 2139],
                                    [714, 4271, 10870],
                                    [10870, 4271, 4451],
                                    [4451, 4271, 4275],
                                    [4451, 4275, 4293] ], dtype=np.int32)
        ebv = shape[eyebrow_tris].flatten().astype(ctypes.c_float)
        ebn = normals[eyebrow_tris].flatten().astype(ctypes.c_float)
        glVertexPointer(3, GL_FLOAT, 0, ebv.ctypes.data)
        glNormalPointer(GL_FLOAT, 0, ebn.ctypes.data)

        glDisable(GL_DEPTH_TEST)
        glDrawArrays(GL_TRIANGLES, 0, 24)

        if text:
            #draw text
            glDisable(GL_DEPTH_TEST)
            glDisable(GL_LIGHT0)
            glDisable(GL_LIGHTING)
            glDisable(GL_COLOR_MATERIAL)
        
            glMatrixMode(GL_PROJECTION)
            glLoadIdentity()
            gluOrtho2D(0, self.width, 0, self.height)
            glMatrixMode(GL_MODELVIEW)
            glLoadIdentity()
            pyglet.text.Label(text, font_name='Times New Roman', font_size=24, x=5, y=5, anchor_x='left',anchor_y='baseline', color=(255, 0, 255, 128)).draw()
        # render to screen
        self.win.flip()

    def capture_screen(self):
        glReadBuffer(GL_FRONT)
        glReadPixels(0, 0, self.width, self.height, GL_RGB, GL_UNSIGNED_BYTE, self.buffer)
        image = Image.frombytes(mode="RGB", size=(self.width, self.height), data=self.buffer)
        image = image.transpose(Image.FLIP_TOP_BOTTOM)
        image = np.array(image)
        # convert RGB to BGR for OpenCV
        new_image = np.copy(image)
        new_image[:,:,0] = image[:,:,2]
        new_image[:,:,2] = image[:,:,0]
        return new_image

    def get_3D_render(self, shape, triangles):
        self.render(shape, triangles)
        return self.capture_screen()

    def exit(self):
        self.win.close()



class Visualizer(object):
    def __init__(self, draw_error=True):
        self.baseshapes = load_processed_baseshapes()
        self.triangles = load_triangles()
        self.renderer = Renderer()
        self.draw_error = draw_error

    def visualize(self, image, e_real, e_fake):
        shape_fake = calc_shape(self.baseshapes, e_fake)
        shape_real = calc_shape(self.baseshapes, e_real)

        self.renderer.render(shape_real, self.triangles)
        img_real = self.renderer.capture_screen()
        self.renderer.render(shape_fake, self.triangles)
        img_fake = self.renderer.capture_screen()

        # result
        new_img = np.zeros((300,900,3), dtype=np.uint8)
        if image is not None:
            new_img[:,0:300,:] = cv2.resize(image, (300,300), interpolation=cv2.INTER_CUBIC)
        new_img[:,300:600,:] = img_real[52:352,170:470,:]
        new_img[:,600:900,:] = img_fake[52:352,170:470,:]
    
        # error text
        if self.draw_error:
            error = np.sum(np.square(e_real-e_fake))
            txt = "error: {:.4f}".format(error)
            cv2.putText(new_img, txt, (10,280), cv2.FONT_HERSHEY_COMPLEX, 1.0, (255, 0, 255), 1)
        
        return new_img

    def exit(self):
        self.renderer.exit()

    def restart(self):
        self.renderer = Renderer()


#--------------------------------------------------
def draw_error_bar_plot(e_real, e_fake, final_size=(900,100)):
    error = np.round(np.abs(e_real - e_fake) * 100.0).astype(np.int32)
    eg = np.round(e_real * 100.0).astype(np.int32)
    ef = np.round(e_fake * 100.0).astype(np.int32)
    # draw 46 bars
    img = np.zeros((460, 220, 3), dtype=np.uint8) + 255
    for i in range(46):
        # draw the error bars
        y1 = 2 + i*10
        y2 = y1 + 6
        x1 = 120
        x2 = 120 + error[i]
        img = cv2.rectangle(img, (x1,y1), (x2,y2), (255, 0, 0), cv2.FILLED)
        img = cv2.putText(img, "{:d}".format(i+1), (105, y1+5), cv2.FONT_HERSHEY_PLAIN, 0.5, (0, 0, 255), 1)
        # draw e_fake bars
        x1 = 0
        x2 = ef[i]
        img = cv2.rectangle(img, (x1,y1), (x2,y1+3), (0, 255, 0), cv2.FILLED)
        # draw e_real bars
        x2 = eg[i]
        img = cv2.rectangle(img, (x1,y1+3), (x2,y2), (0, 0, 255), cv2.FILLED)
    img = cv2.transpose(img)
    img = cv2.flip(img, 0)
    ret = cv2.resize(img, final_size)
    return ret
 

if __name__ == "__main__":
    triangles = load_triangles()
    baseshapes = load_processed_baseshapes()
    renderer = Renderer()
    e = np.zeros(46, dtype=np.float32)
    shape = calc_shape(baseshapes, e)
    renderer.render(shape, triangles)
    img = renderer.capture_screen()
    renderer.exit()
    cv2.imshow("image", img)
    cv2.waitKey()

```

## Source file: src/framevideo.py

{
  "path": "src/framevideo.py",
  "blob_sha": "24ba7d70e8ec3737d39417d17b4846aa226c1c73",
  "sha256": "6fd3ea27023ef7529078752327820e2d896d4911b320137b83487b2b47c9f188",
  "lines": 32,
  "bytes": 1026,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/muskan70/Speech-driven-facial-animation/blob/3510c72477bb730dae7bb119411f6e439eaa1cc1/src/framevideo.py",
  "function_signature_candidates": [
    {
      "line": 6,
      "text": "def make_video(pathIn,actor,file):"
    }
  ]
}

```text

import cv2
import numpy as np
import os
from os.path import isfile, join

def make_video(pathIn,actor,file):
    p=actor + "__"+ file+ "__" +"video.mp4"
    p=str(p)
    fps = 15
    frame_array = []
    files = [f for f in os.listdir(pathIn) if isfile(join(pathIn, f))]
    #for sorting the file names properly
    #files.sort(key = lambda x: x[5:-4])
    #files.sort()
    frame_array = []
    files = [f for f in os.listdir(pathIn) if isfile(join(pathIn, f))]
    #for sorting the file names properly
    #files.sort(key = lambda x: x[5:-4])
    for i in range(len(files)):
        filename=pathIn + files[i]
        #reading each files
        img = cv2.imread(filename)
        height, width, layers = img.shape
        size = (width,height)

        #inserting the frames into an image array
        frame_array.append(img)
    out = cv2.VideoWriter(p,cv2.VideoWriter_fourcc(*'DIVX'), fps, size)
    for i in range(len(frame_array)):
        # writing to a image array
        out.write(frame_array[i])
    out.release()

```

## Source file: src/7.LayerUtils.py

{
  "path": "src/7.LayerUtils.py",
  "blob_sha": "0e0bd96b5a9c727b8ed388894a06d40b32981d3e",
  "sha256": "c4acbc9264ab42e83b69df620560db9c0b4cf2d5030a7e44e1f0d89b59229956",
  "lines": 95,
  "bytes": 3598,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/muskan70/Speech-driven-facial-animation/blob/3510c72477bb730dae7bb119411f6e439eaa1cc1/src/7.LayerUtils.py",
  "function_signature_candidates": [
    {
      "line": 5,
      "text": "def lrelu(input, leak=0.2, name=\"\"):"
    },
    {
      "line": 9,
      "text": "def bn(input, activation=None, name=\"\"):"
    },
    {
      "line": 18,
      "text": "def bn_relu(input, name=\"\"):"
    },
    {
      "line": 22,
      "text": "def bn_lrelu(input, name=\"\"):"
    },
    {
      "line": 25,
      "text": "def conv(input, filter_shape, num_filters, strides=(1,1), init=C.he_normal(), activation=None, pad=True, name=\"\"):"
    },
    {
      "line": 29,
      "text": "def conv_bn(input, filter_shape, num_filters, strides=(1,1), init=C.he_normal(), activation=None, name=\"\"):"
    },
    {
      "line": 35,
      "text": "def conv_bn_nopad(input, filter_shape, num_filters, strides=(1,1), init=C.he_normal(), activation=None, name=\"\"):"
    },
    {
      "line": 41,
      "text": "def conv_bn_relu(input, filter_shape, num_filters, strides=(1,1), init=C.he_normal(), name=\"\"):"
    },
    {
      "line": 45,
      "text": "def conv_bn_lrelu(input, filter_shape, num_filters, strides=(1,1), init=C.he_normal(), name=\"\"):"
    },
    {
      "line": 49,
      "text": "def conv_bn_relu_nopad(input, filter_shape, num_filters, strides=(1,1), init=C.he_normal(), name=\"\"):"
    },
    {
      "line": 53,
      "text": "def conv_bn_lrelu_nopad(input, filter_shape, num_filters, strides=(1,1), init=C.he_normal(), name=\"\"):"
    },
    {
      "line": 56,
      "text": "def flatten(input, name=\"\"):"
    },
    {
      "line": 61,
      "text": "def flatten_2D(input, name):"
    },
    {
      "line": 66,
      "text": "def broadcast_xy(input_vec, h, w):"
    },
    {
      "line": 80,
      "text": "def conv_from_weights(x, weights, bias=None, padding=True, name=\"\"):"
    },
    {
      "line": 91,
      "text": "def bi_recurrence(input, fwd, bwd, name=\"\"):"
    }
  ]
}

```text

import cntk as C
import numpy as np


def lrelu(input, leak=0.2, name=""):
    return C.param_relu(C.constant((np.ones(input.shape)*leak).astype(np.float32)), input, name=name)


def bn(input, activation=None, name=""):
    if activation is not None:
        x = C.layers.BatchNormalization(map_rank=1, name=name+"_bn" if name else "")(input)
        x = activation(x, name=name)
    else:
        x = C.layers.BatchNormalization(map_rank=1, name=name)(input)
    return x


def bn_relu(input, name=""):
    return bn(input, activation=C.relu, name=name)


def bn_lrelu(input, name=""):
    return bn(input, activation=C.leaky_relu, name=name)

def conv(input, filter_shape, num_filters, strides=(1,1), init=C.he_normal(), activation=None, pad=True, name=""):
    return C.layers.Convolution(filter_shape, num_filters, strides=strides, pad=pad, activation=activation, init=init, bias=False, name=name)(input)


def conv_bn(input, filter_shape, num_filters, strides=(1,1), init=C.he_normal(), activation=None, name=""):
    x = conv(input, filter_shape, num_filters, strides, init, name=name+"_conv" if name else "")
    x = bn(x, activation, name=name)
    return x


def conv_bn_nopad(input, filter_shape, num_filters, strides=(1,1), init=C.he_normal(), activation=None, name=""):
    x = conv(input, filter_shape, num_filters, strides, init, pad=False, name=name+"_conv" if name else "")
    x = bn(x, activation, name=name)
    return x


def conv_bn_relu(input, filter_shape, num_filters, strides=(1,1), init=C.he_normal(), name=""):
    return conv_bn(input, filter_shape, num_filters, strides, init, activation=C.relu, name=name)


def conv_bn_lrelu(input, filter_shape, num_filters, strides=(1,1), init=C.he_normal(), name=""):
    return conv_bn(input, filter_shape, num_filters, strides, init, activation=C.leaky_relu, name=name)


def conv_bn_relu_nopad(input, filter_shape, num_filters, strides=(1,1), init=C.he_normal(), name=""):
    return conv_bn_nopad(input, filter_shape, num_filters, strides, init, activation=C.relu, name=name)


def conv_bn_lrelu_nopad(input, filter_shape, num_filters, strides=(1,1), init=C.he_normal(), name=""):
    return conv_bn_nopad(input, filter_shape, num_filters, strides, init, activation=C.leaky_relu, name=name)

def flatten(input, name=""):
    assert (len(input.shape) == 3)
    return C.reshape(input, input.shape[0]*input.shape[1]* input.shape[2], name=name)


def flatten_2D(input, name):
    assert (len(input.shape) >= 3)
    return C.reshape(input, (input.shape[-3], input.shape[-2]* input.shape[-1]), name=name)


def broadcast_xy(input_vec, h, w):
    """ broadcast input vector of length d to tensor (d x h x w) """
    assert(h > 0 and w > 0)
    d = input_vec.shape[0]
    # reshape vector to d x 1 x 1
    x = C.reshape(input_vec, (d, 1, 1))
    # create a zeros-like tensor of size (d x h x w)
    t = np.zeros((d, h, w), dtype=np.float32)
    y = C.constant(t)
    z = C.reconcile_dynamic_axes(y, x)
    z = z + x
    return z


def conv_from_weights(x, weights, bias=None, padding=True, name=""):
    """ weights is a numpy array """
    k = C.parameter(shape=weights.shape, init=weights)
    y = C.convolution(k, x, auto_padding=[False, padding, padding])
    if bias:
        b = C.parameter(shape=bias.shape, init=bias)
        y = y + bias
    y = C.alias(y, name=name)
    return y


def bi_recurrence(input, fwd, bwd, name=""):
    F = C.layers.Recurrence(fwd, go_backwards=False, name='fwd_rnn')(input)
    B = C.layers.Recurrence(bwd, go_backwards=True, name='bwd_rnn')(input)
    h = C.splice(F, B, name=name)
    return h

```
