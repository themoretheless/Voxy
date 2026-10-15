# Facial rendering source scan: haixpham/end2end_AU_speech

Repository: https://github.com/haixpham/end2end_AU_speech

Pinned source revision: 04cbbb4e4d25e41e8a97df3f673dc7fbd4351357

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {}

## Source file: src/SysUtils.py

{
  "path": "src/SysUtils.py",
  "blob_sha": "55d6ff3127dfa92e81f946ddc4fd2e87d52b73e9",
  "sha256": "3057387453e9551ae2a821d17a6712dc7be1f17a73d2e0489c9f824c7a263797",
  "lines": 101,
  "bytes": 3487,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 8,
    "rig": 1
  },
  "url": "https://github.com/haixpham/end2end_AU_speech/blob/04cbbb4e4d25e41e8a97df3f673dc7fbd4351357/src/SysUtils.py",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "def is_Win32():"
    },
    {
      "line": 17,
      "text": "def make_dir(dir_path, exist_ok=True):"
    },
    {
      "line": 20,
      "text": "def get_current_time_string():"
    },
    {
      "line": 23,
      "text": "def get_items(folder, what_to_get=None):"
    },
    {
      "line": 36,
      "text": "def get_path(args):"
    },
    {
      "line": 40,
      "text": "def get_filename(path):"
    },
    {
      "line": 43,
      "text": "def get_extension(path):"
    },
    {
      "line": 46,
      "text": "def get_parent_dir(path):"
    },
    {
      "line": 49,
      "text": "def read_learning_rate(filename):"
    },
    {
      "line": 68,
      "text": "def __init__(self):"
    },
    {
      "line": 78,
      "text": "def prepare(self):"
    },
    {
      "line": 88,
      "text": "def parse(self):"
    }
  ]
}

```text

'''
Copyright (c) 2018 Hai Pham, Rutgers University
http://www.cs.rutgers.edu/~hxp1/

This code is free to use for academic/research purpose.

'''

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

## Source file: src/LayerUtils.py

{
  "path": "src/LayerUtils.py",
  "blob_sha": "073d6c82e337d6dfd391343fc1ce3ce9b5f59c93",
  "sha256": "c2b23522e1f45f39037489beca2f578d77ebae215c01f3ccbb5298d33545b432",
  "lines": 97,
  "bytes": 3678,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/haixpham/end2end_AU_speech/blob/04cbbb4e4d25e41e8a97df3f673dc7fbd4351357/src/LayerUtils.py",
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
      "line": 93,
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


# bi-directional recurrence function op
# fwd, bwd: a recurrent op, LSTM or GRU
def bi_recurrence(input, fwd, bwd, name=""):
    F = C.layers.Recurrence(fwd, go_backwards=False, name='fwd_rnn')(input)
    B = C.layers.Recurrence(bwd, go_backwards=True, name='bwd_rnn')(input)
    h = C.splice(F, B, name=name)
    return h

```

## Source file: src/ShapeUtils2.py

{
  "path": "src/ShapeUtils2.py",
  "blob_sha": "ee82f57d0eb62467d43f2ec7a5441227f1a83bfb",
  "sha256": "2a468447c5b7834bbfe911bd17987968de2f63f5b812edc9ceccb5837890bc34",
  "lines": 325,
  "bytes": 11200,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 4,
    "rig": 2,
    "face": 1,
    "lip": 3
  },
  "url": "https://github.com/haixpham/end2end_AU_speech/blob/04cbbb4e4d25e41e8a97df3f673dc7fbd4351357/src/ShapeUtils2.py",
  "function_signature_candidates": [
    {
      "line": 23,
      "text": "def load_shape(filename, load_triangles = False):"
    },
    {
      "line": 34,
      "text": "def load_shapes_raw(path=filename_template):"
    },
    {
      "line": 48,
      "text": "def load_shapes(path=filename_template):"
    },
    {
      "line": 64,
      "text": "def load_processed_baseshapes(filename=\"\"):"
    },
    {
      "line": 71,
      "text": "def load_triangles(filename=\"\"):"
    },
    {
      "line": 79,
      "text": "def calc_shape(allshapes, e):"
    },
    {
      "line": 92,
      "text": "def calc_all_shapes(allshapes, Es):"
    },
    {
      "line": 100,
      "text": "def transform_shape(shape, R=None, T=None):"
    },
    {
      "line": 109,
      "text": "def calc_vertex_normals(vertices, triangles):"
    },
    {
      "line": 111,
      "text": "def normalize_v3(arr):"
    },
    {
      "line": 130,
      "text": "def __init__(self, width=640, height=480, caption=\"Renderer\"):"
    },
    {
      "line": 138,
      "text": "def initGL(self):"
    },
    {
      "line": 144,
      "text": "def render(self, shape, triangles, normals=None, text=\"\"):"
    },
    {
      "line": 225,
      "text": "def capture_screen(self):"
    },
    {
      "line": 240,
      "text": "def get_3D_render(self, shape, triangles):"
    },
    {
      "line": 244,
      "text": "def exit(self):"
    },
    {
      "line": 250,
      "text": "def __init__(self, draw_error=True):"
    },
    {
      "line": 256,
      "text": "def visualize(self, image, e_real, e_fake):"
    },
    {
      "line": 280,
      "text": "def exit(self):"
    },
    {
      "line": 283,
      "text": "def restart(self):"
    },
    {
      "line": 288,
      "text": "def draw_error_bar_plot(e_real, e_fake, final_size=(900,100)):"
    }
  ]
}

```text

'''
Copyright (c) 2018 Hai Pham, Rutgers University
http://www.cs.rutgers.edu/~hxp1/

This code is free to use for academic/research purpose.

'''

import plyfile
import numpy as np
import math

import pyglet
from pyglet.gl import *
import ctypes
from PIL import Image
import cv2

from SysUtils import is_Win32

filename_template = "../shapes/shape_{:d}.ply"

def load_shape(filename, load_triangles = False):
    mesh = plyfile.PlyData.read(filename)
    # convert vertices to numpy array
    vertices = np.transpose(np.vstack((mesh['vertex']['x'],mesh['vertex']['y'],mesh['vertex']['z'])))
    # get triangles
    if load_triangles:
        tridata = mesh['face'].data['vertex_indices']
        triangles = plyfile.make2d(tridata)
        return vertices, triangles
    return vertices

def load_shapes_raw(path=filename_template):
    n = 47
    shapes = []
    triangles = None
    for i in range(n):
        filename = path.format(i)
        if triangles is None:
            shape, triangles = load_shape(filename, load_triangles=True)
        else:
            shape = load_shape(filename)
        shapes.append(shape)
    return shapes, triangles


def load_shapes(path=filename_template):
    shapes, triangles = load_shapes_raw(path)
    # compact shapes into
    shape_list = []
    numvert = shapes[0].shape[0]
    for i, shape in enumerate(shapes):
        flat_shape = shape.flatten()
        if i == 0:
            shape_list.append(flat_shape)
        else:
            flat_shape = flat_shape - shape_list[0]
            shape_list.append(flat_shape)
    allshapes = np.stack(shape_list)
    return allshapes, triangles


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

###########################################################################
class Renderer(object):
    def __init__(self, width=640, height=480, caption="Renderer"):
        self.win = pyglet.window.Window(width=width, height=height, caption=caption, visible=False)
        self.width = width
        self.height = height
        self.initGL()
        self.buffer = (GLubyte * (3*width*height))(0)
        self.is_win32 = is_Win32()

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
        if(self.is_win32):
            glReadBuffer(GL_FRONT)
        else:
            glReadBuffer(GL_BACK)
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


#----------------------------------------------------------------------------------------------
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
 

#if __name__ == "__main__":
#    triangles = load_triangles()
#    baseshapes = load_processed_baseshapes()
#    renderer = Renderer()
#    e = np.zeros(46, dtype=np.float32)
#    shape = calc_shape(baseshapes, e)
#    renderer.render(shape, triangles)
#    img = renderer.capture_screen()
#    renderer.exit()
#    cv2.imshow("image", img)
#    cv2.waitKey()

```

## Source file: src/eval_speech.py

{
  "path": "src/eval_speech.py",
  "blob_sha": "5f48bf48d18431d50b7019ac0d809a544658e982",
  "sha256": "95ddc16a2bceb17b1070537ae57998a5a8c9b21dbbbb56ecefa7281493a25264",
  "lines": 139,
  "bytes": 5014,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 3,
    "rig": 1,
    "face": 1
  },
  "url": "https://github.com/haixpham/end2end_AU_speech/blob/04cbbb4e4d25e41e8a97df3f673dc7fbd4351357/src/eval_speech.py",
  "function_signature_candidates": [
    {
      "line": 18,
      "text": "def load_image(path):"
    },
    {
      "line": 24,
      "text": "def load_image_stack(paths):"
    },
    {
      "line": 29,
      "text": "def load_exp_sequence(path, use_medfilt=False, ksize=3):"
    },
    {
      "line": 37,
      "text": "def is_recurrent(model):"
    },
    {
      "line": 46,
      "text": "def estimate_one_audio_seq(model, audio_seq, small_mem=False):"
    },
    {
      "line": 72,
      "text": "def visualize_one_audio_seq(model, video_frame_list, audio_csv_file, exp_npy_file, visualizer, save_dir):"
    },
    {
      "line": 119,
      "text": "def test_one_seq(visualizer):"
    }
  ]
}

```text

'''
Copyright (c) 2018 Hai Pham, Rutgers University
http://www.cs.rutgers.edu/~hxp1/

This code is free to use for academic/research purpose.

'''

import numpy as np
import cntk as C
import cv2
from scipy.signal import medfilt

import ShapeUtils2 as SU
from SysUtils import make_dir, get_items


def load_image(path):
    img = cv2.imread(path, 1)
    if img.shape[0] != 100 or img.shape[1] != 100:
        img = cv2.resize(img, (100, 100), interpolation=cv2.INTER_CUBIC)
    return img

def load_image_stack(paths):
    imgs = [load_image(path) for path in paths]
    return np.stack(imgs)


def load_exp_sequence(path, use_medfilt=False, ksize=3):
    exp = np.load(path).astype(np.float32)
    if use_medfilt:
        exp = medfilt(exp, kernel_size=(ksize,1)).astype(np.float32)
    return exp


# estimate audio sequence -> exp
def is_recurrent(model):
    names = ["RNN", "rnn", "LSTM", "LSTM1", "GRU", "gru", "fwd_rnn", "bwd_rnn"]
    isrnn = False
    for name in names:
        if model.find_by_name(name) is not None:
            isrnn = True
    return isrnn


def estimate_one_audio_seq(model, audio_seq, small_mem=False):
    if isinstance(model, str):
        model = C.load_model(model)
    # set up 2 cases: if the model is recurrent or static
    if is_recurrent(model):
        n = audio_seq.shape[0]
        NNN = 125
        if n > NNN and small_mem:
            nseqs = n//NNN + 1
            indices = []
            for i in range(nseqs-1):
                indices.append(NNN*i + NNN)
            input_seqs = np.vsplit(audio_seq, indices)
            outputs = []
            for seq in input_seqs:
                output = model.eval({model.arguments[0]:[seq]})[0]
                outputs.append(output)
            output = np.concatenate(outputs)
        else:
            output = model.eval({model.arguments[0]:[audio_seq]})[0]
    else:
        output = model.eval({model.arguments[0]: audio_seq})
    return output


#----------------------- feed sequence -------------------------
def visualize_one_audio_seq(model, video_frame_list, audio_csv_file, exp_npy_file, visualizer, save_dir):
    if isinstance(model, str):
        model = C.load_model(model)
    # evaluate model with given audio data
    audio = np.loadtxt(audio_csv_file, dtype=np.float32, delimiter=",")
    audio_seq = np.reshape(audio, (audio.shape[0], 1, 128, 32))
    e_fake = estimate_one_audio_seq(model, audio_seq)
    if e_fake.shape[1] != 46:
        if e_fake.shape[1] == 49:
            e_fake = e_fake[:,3:]
        else:
            raise ValueError("unsupported output of audio model")
    # load true labels with optional median filter to smooth it (not used in training)
    e_real = load_exp_sequence(exp_npy_file, use_medfilt=True)

    if e_real.shape[0] != e_fake.shape[0]:
        raise ValueError("number of true labels and number of outputs do not match")
    
    # create directory to store output frames
    if video_frame_list:
        video = load_image_stack(video_frame_list)
        if video.shape[0] != e_real.shape[0]:
            print("number of frames and number of labels do not match. Not using video")
            video = None
    else:
        video = None
    # make folder to store generated frames
    make_dir(save_dir)

    n = e_real.shape[0]
    for i in range(n):
        if video is not None:
            img = video[i,:,:,:]
        else:
            img = None # not include input video in the output
        ef = e_fake[i,:]
        er = e_real[i,:]
        ret = visualizer.visualize(img, er, ef)
        # draw plot
        plot = SU.draw_error_bar_plot(er, ef, (ret.shape[1],200))
        ret = np.concatenate([ret, plot], axis=0)
        save_path = save_dir + "/result{:06d}.jpg".format(i)
        cv2.imwrite(save_path, ret)
        # can call cv2.imshow() here


#----------------------------------------------------------------------------------------
def test_one_seq(visualizer):
    # directory to store output video. It will be created if it doesn't exist
    save_dir = "H:/Speech_data/test_output_single"
    model_file = "H:/Speech_data/model_audio2exp_2018-08-01-05-14/model_audio2exp_2018-08-01-05-14.dnn"
    # video directory holding separate frames of the video. Each image should be square.
    video_dir = "H:/FrontalFaceData/RAVDESS/Actor_21/01-01-07-02-01-01-21"
    # spectrogram sequence is stored in a .csv file
    audio_file = "H:/Speech_data/RAVDESS_feat/Actor_21/01-01-07-02-01-01-21/dbspectrogram.csv"
    # AU labels are stored in an .npy file
    exp_file = "H:/Training_data_image/ExpLabels/RAVDESS/Actor_21/01-01-07-02-01-01-21.npy"

    video_list = get_items(video_dir, "full") # set to None if video_dir does not exist
    model = C.load_model(model_file)

    visualize_one_audio_seq(model, video_list, audio_file, exp_file, visualizer, save_dir)

#----------------------------------------------------------------------------------
if __name__ == "__main__":
    visualizer = SU.Visualizer()
    test_one_seq(visualizer)
    

```
