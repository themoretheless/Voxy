# Facial rendering source scan: Sonaly03/Joint-Face-Detection-and-Facial-Motion-Retargeting-for-Multiple-Faces

Repository: https://github.com/Sonaly03/Joint-Face-Detection-and-Facial-Motion-Retargeting-for-Multiple-Faces

Pinned source revision: 6fa4064b6f0305108ffedef7eba07f9f43972fc3

Discovery topic: facial retargeting

Description: A single end-to-end network to jointly predict the bounding box locations and 3DMM parameters for multiple faces

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"eye": ["PRN/Data/uv-data/uv_face_eyes.png"], "face": ["PRN/Data/net-data/mmod_human_face_detector.dat", "PRN/Data/uv-data/face_ind.txt", "PRN/Data/uv-data/facemargin", "PRN/Data/uv-data/uv_face.png", "PRN/Data/uv-data/uv_face_eyes.png", "PRN/Data/uv-data/uv_face_mask.png", "Results/FaceResult1.docx", "Results/FaceResult2.docx", "Results/FaceResult3.docx", "Results/FaceResult4.docx"]}

## Source file: PRN/api.py

{
  "path": "PRN/api.py",
  "blob_sha": "8e88530b34febc14be67f918970051ad0f2328a8",
  "sha256": "e5222c3044afbb29cdb5ce06f48c28e41a49dc38a572030dd4d97db751109d0c",
  "lines": 165,
  "bytes": 6535,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 7,
    "face": 18
  },
  "url": "https://github.com/Sonaly03/Joint-Face-Detection-and-Facial-Motion-Retargeting-for-Multiple-Faces/blob/6fa4064b6f0305108ffedef7eba07f9f43972fc3/PRN/api.py",
  "function_signature_candidates": [
    {
      "line": 16,
      "text": "def __init__(self, is_dlib_installed = False, prefix = '.'):"
    },
    {
      "line": 46,
      "text": "def dlib_detector(self, image):"
    },
    {
      "line": 49,
      "text": "def net_forward(self, image):"
    },
    {
      "line": 55,
      "text": "def main_process(self, input, image_info = None):"
    },
    {
      "line": 116,
      "text": "def generate_uvMap_coords(self):"
    },
    {
      "line": 126,
      "text": "def get_colors(self, image, vertices):"
    },
    {
      "line": 136,
      "text": "def get_landmark_points(self, pos):"
    },
    {
      "line": 142,
      "text": "def get_vertices_images(self, pos):"
    },
    {
      "line": 149,
      "text": "def get_colors_texture(self, texture):"
    }
  ]
}

```text


import numpy as np
import os
from skimage.io import imread, imsave
from skimage.transform import estimate_transform, warp
from time import time
import dlib

from predictor import FacePointPositionPredictor
#This method is the main class for transforming 2D images to 3D face as it uses the PRN network for generating the semantic features of the facial points
class FaceModelling:

# all the paths and  ariables that are used in the program ahead are intialized in this method
# the resolution of the input and output is set to 256 becasue the image  is further divided by 255    
# the landmark file which is used for generating  the points are also intialzed here 
    def __init__(self, is_dlib_installed = False, prefix = '.'):

        
        self.resolution_input = 256
        self.resolution_output = 256

        
        if is_dlib_installed:
           
            detector_path = os.path.join(prefix, 'Data/net-data/mmod_human_face_detector.dat')
            self.face_detector = dlib.cnn_face_detection_model_v1(
                    detector_path)

        
        self.pos_predictor = FacePointPositionPredictor(self.resolution_input, self.resolution_output)
        image_path = os.path.join(prefix, 'Data/net-data/256_256_resfcn256_weight')
        if not os.path.isfile(image_path + '.data-00000-of-00001'):
            
            exit()
        self.pos_predictor.restore(image_path)

        
        self.uv_kpt_ind = np.loadtxt(prefix + '/Data/uv-data/uv_kpt_ind.txt').astype(np.int32) 
        self.face_ind = np.loadtxt(prefix + '/Data/uv-data/face_ind.txt').astype(np.int32) 
        self.triangles = np.loadtxt(prefix + '/Data/uv-data/triangles.txt').astype(np.int32)
        
        self.uv_coords = self.generate_uvMap_coords()        

    

    def dlib_detector(self, image):
        return self.face_detector(image, 1)

    def net_forward(self, image):
        
        return self.pos_predictor.predict(image)
#this is the main method of the pRN network whihc read the image as input , then divides them
#a rectangular background is made around each image and then the face is detected
# here the image detected is also divided and made to a 255 block image which is used for further processing        
    def main_process(self, input, image_info = None):
        
        if isinstance(input, str):
            try:
                image = imread(input)
            except IOError:
                print("error opening file: ", input)
                return None
        else:
            image = input

        if image.ndim < 3:
            image = np.tile(image[:,:,np.newaxis], [1,1,3])

        if image_info is not None:
            if np.max(image_info.shape) > 4: 
                kpt = image_info
                if kpt.shape[0] > 3:
                    kpt = kpt.T
                left = np.min(kpt[0, :]); right = np.max(kpt[0, :]); 
                top = np.min(kpt[1,:]); bottom = np.max(kpt[1,:])
            else: 
                coverbox = image_info
                left = coverbox [0]; right = coverbox [1]; top = coverbox [2]; bottom = coverbox [3]
            old_size = (right - left + bottom - top)/2
            center = np.array([right - (right - left) / 2.0, bottom - (bottom - top) / 2.0])
            size = int(old_size*1.6)
        else:
            detected_faces = self.dlib_detector(image)
            if len(detected_faces) == 0:
                print('no detected face')
                return None

            d = detected_faces[0].rect 
            left = d.left(); right = d.right(); top = d.top(); bottom = d.bottom()
            old_size = (right - left + bottom - top)/2
            center = np.array([right - (right - left) / 2.0, bottom - (bottom - top) / 2.0 + old_size*0.14])
            size = int(old_size*1.58)

       
        source = np.array([[center[0]-size/2, center[1]-size/2], [center[0] - size/2, center[1]+size/2], [center[0]+size/2, center[1]-size/2]])
        destination = np.array([[0,0], [0,self.resolution_input - 1], [self.resolution_input - 1, 0]])
        tform = estimate_transform('similarity', source, destination)
        
        image = image/255
        cropped_image = warp(image, tform.inverse, output_shape=(self.resolution_input, self.resolution_input))


        cropped_pos = self.net_forward(cropped_image)


       
        cropped_vertices = np.reshape(cropped_pos, [-1, 3]).T
        z = cropped_vertices[2,:].copy()/tform.params[0,0]
        cropped_vertices[2,:] = 1
        vertices = np.dot(np.linalg.inv(tform.params), cropped_vertices)
        vertices = np.vstack((vertices[:2,:], z))
        pos = np.reshape(vertices.T, [self.resolution_output, self.resolution_output, 3])
        
        return pos
#this method deals with generated the coordinates for the UV map spaceon which the various tranformation is applied and the resultant is use ahead            
    def generate_uvMap_coords(self):
        resolution = self.resolution_output
        uvMap_coords = np.meshgrid(range(resolution),range(resolution))
        uvMap_coords = np.transpose(np.array(uvMap_coords), [1,2,0])
        uvMap_coords = np.reshape(uvMap_coords, [resolution**2, -1]);
        uvMap_coords = uvMap_coords[self.face_ind, :]
        uvMap_coords = np.hstack((uvMap_coords[:,:2], np.zeros([uvMap_coords.shape[0], 1])))
        return uvMap_coords
    
 #this method is used to get the colours for the box that is detecting the face and then using teh same image for 3D reconstruction   
    def get_colors(self, image, vertices):
        
        [h, w, _] = image.shape
        vertices[:,0] = np.minimum(np.maximum(vertices[:,0], 0), w - 1)  
        vertices[:,1] = np.minimum(np.maximum(vertices[:,1], 0), h - 1) 
        ind = np.round(vertices).astype(np.int32)
        colors = image[ind[:,1], ind[:,0], :] 

        return colors
# the landmark points are used for detetcing the points on the faces    
    def get_landmark_points(self, pos):
        
        kpt = pos[self.uv_kpt_ind[1,:], self.uv_kpt_ind[0,:], :]
        return kpt


    def get_vertices_images(self, pos):
        
        all_vertices = np.reshape(pos, [self.resolution_output**2, -1])
        vertices = all_vertices[self.face_ind, :]

        return vertices
# the colour and reshaping of the verticces is done in this method
    def get_colors_texture(self, texture):
        
        all_colors = np.reshape(texture, [self.resolution_output**2, -1])
        colors = all_colors[self.face_ind, :]

        return colors


   










```

## Source file: Blender/CVA.py

{
  "path": "Blender/CVA.py",
  "blob_sha": "e748416b5566f632cd28c587a624a49162d240e7",
  "sha256": "65be8a1b39caa3011b317d0c82904f11255466010e6d8a114a9643ef19853d28",
  "lines": 174,
  "bytes": 7188,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 4,
    "mouth": 7,
    "rig": 7,
    "face": 5
  },
  "url": "https://github.com/Sonaly03/Joint-Face-Detection-and-Facial-Motion-Retargeting-for-Multiple-Faces/blob/6fa4064b6f0305108ffedef7eba07f9f43972fc3/Blender/CVA.py",
  "function_signature_candidates": [
    {
      "line": 43,
      "text": "def s_v(m, n, length, v):"
    },
    {
      "line": 57,
      "text": "def exit(m, c):"
    },
    {
      "line": 65,
      "text": "def get_range(m, n, v):"
    },
    {
      "line": 78,
      "text": "def execute(m, c):"
    },
    {
      "line": 87,
      "text": "def modal(m, c, e):"
    },
    {
      "line": 151,
      "text": "def endplay(m, o):"
    },
    {
      "line": 156,
      "text": "def camera(m):"
    },
    {
      "line": 164,
      "text": "def to_register():"
    },
    {
      "line": 167,
      "text": "def to_unregister():"
    }
  ]
}

```text


import bpy
from imutils import face_utils
import dlib
import cv2
import time
import numpy
from bpy.props import FloatProperty
    
class CV(bpy.types.Operator):

    bl_idname = "mw.opencv_operator"
    bl_label = "CV Project"
    #Path for Landmark points location 
    l = "/Users/Rohit/Desktop/Python/CV/shape_predictor_68_face_landmarks.dat"
    detector = dlib.get_frontal_face_detector()
    predictor = dlib.shape_predictor(l)

    _timer = None
    _cap  = None
    
    width = 850
    height = 650

    stop :bpy.props.BoolProperty()

    #3D Modal Points  
    mpoints = numpy.array([
                                (0.0, 0.0, 0.0),  #nose           
                                (0.0, -330.5, -65.0),  #chin      
                                (-225.0, 170.0, -135.0),   #left eye left corner	  
                                (225.5, 170.0, -135.0),   #right eye right corner   
                                (-150.5, -150.0, -125.0),    #left mouth corner
                                (150.0, -150.5, -125.0)    #right mouth corner  
                            ], dtype = numpy.float32)
    #Camara Matrix	
    cmatrix = numpy.array(
                            [[height, 0.0, width/2],
                            [0.0, height, height/2],
                            [0.0, 0.0, 1.0]], dtype = numpy.float32
                            )
    #Maintains a moving average of the length provided
    def s_v(m, n, length, v):
        if not hasattr(m, 'smooth'):
            m.smooth = {}
        if not n in m.smooth:
            m.smooth[n] = numpy.array([v])
        else:
            m.smooth[n] = numpy.insert(arr=m.smooth[n], obj=0, values=v)
            if m.smooth[n].size > length:
                m.smooth[n] = numpy.delete(m.smooth[n], m.smooth[n].size-1, 0)
        sum = 0
        for val in m.smooth[n]:
            sum += val
        return sum / m.smooth[n].size
    
    def exit(m, c):
        mw = c.window_manager #open window manager and UI
        mw.event_timer_remove(m._timer) #add timer to the window
        cv2.destroyAllWindows()
        m._cap.release()
        m._cap = None


    def get_range(m, n, v):
        if not hasattr(m, 'range'):
            m.range = {}
        if not n in m.range:
            m.range[n] = numpy.array([v, v])
        else:
            m.range[n] = numpy.array([min(v, m.range[n][0]), max(v, m.range[n][1])] )
        v_r = m.range[n][1] - m.range[n][0]
        if v_r != 0:
            return (v - m.range[n][0]) / v_r
        else:
            return 0
        
    def execute(m, c):
        bpy.app.handlers.frame_change_pre.append(m.endplay)

        mw = c.window_manager
        m._timer = mw.event_timer_add(0.03, window=c.window)
        mw.modal_handler_add(m)
        return {'RUNNING_MODAL'}
    

    def modal(m, c, e):

        if (e.type in {'RIGHTMOUSE', 'ESC'}) or m.stop == True:
            m.exit(c) #exits
            return {'CANCELLED'}

        if e.type == 'TIMER':
            m.camera()
            _, image = m._cap.read()
            gray = cv2.cvtColor(image, cv2.COLOR_BGR2GRAY)
            rects = m.detector(gray, 0)
	#Find landmark for each detected face
            for (i, rect) in enumerate(rects):
                s = m.predictor(gray, rect)
                s = face_utils.shape_to_np(s)
          	#2D Points
                image_points = numpy.array([s[30],     #nose tip
                                            s[8],      #chin
                                            s[36],     #left eye left corner
                                            s[45],     #right eye right corner
                                            s[48],     #left mouth corner
                                            s[54]      #right mouth cormer
                                        ], dtype = numpy.float32)
             
                coeffs = numpy.zeros((4,1)) #return new array of given shape and type.
             	#Modal co-ordinate system to 3D conversion
                if hasattr(m, 'rotation_vector'):
                    (success, m.rotation_vector, m.translation_vector) = cv2.solvePnP(m.mpoints, 
                        image_points, m.cmatrix, coeffs, flags=cv2.SOLVEPNP_ITERATIVE, 
                        rvec=m.rotation_vector, tvec=m.translation_vector, 
                        useExtrinsicGuess=True)
                else:
                    (success, m.rotation_vector, m.translation_vector) = cv2.solvePnP(m.mpoints, 
                        image_points, m.cmatrix, coeffs, flags=cv2.SOLVEPNP_ITERATIVE, 
                        useExtrinsicGuess=False)
             
                if not hasattr(m, 'first_angle'):
                    m.first_angle = numpy.copy(m.rotation_vector)
             	#Using blender charecter pre trained model named "Vincent"
                b = bpy.data.objects["RIG-Vincent"].pose.bones
                #to move the head position 
                b["head_fk"].rotation_euler[0] = m.s_v("h_x", 3, (m.rotation_vector[0] - m.first_angle[0])) / 1   
                b["head_fk"].rotation_euler[2] = m.s_v("h_y", 3, -(m.rotation_vector[1] - m.first_angle[1])) / 1.5  
                b["head_fk"].rotation_euler[1] = m.s_v("h_z", 3, (m.rotation_vector[2] - m.first_angle[2])) / 1.3   
                
                b["mouth_ctrl"].location[2] = m.s_v("m_h", 2, -m.get_range("mouth_height", numpy.linalg.norm(s[62] - s[66])) * 0.06 )
                b["mouth_ctrl"].location[0] = m.s_v("m_w", 2, (m.get_range("mouth_width", numpy.linalg.norm(s[54] - s[48])) - 0.5) * -0.04)
                b["brow_ctrl_L"].location[2] = m.s_v("b_l", 3, (m.get_range("brow_left", numpy.linalg.norm(s[19] - s[27])) -0.5) * 0.04)
                b["brow_ctrl_R"].location[2] = m.s_v("b_r", 3, (m.get_range("brow_right", numpy.linalg.norm(s[24] - s[27])) -0.5) * 0.04)
                #code for smooth transition
                b["head_fk"].keyframe_insert(data_path="rotation_euler", index=-1)
                b["mouth_ctrl"].keyframe_insert(data_path="location", index=-1)
                b["brow_ctrl_L"].keyframe_insert(data_path="location", index=2)
                b["brow_ctrl_R"].keyframe_insert(data_path="location", index=2)
         	#draw circle using cv2 function
                for (x, y) in s:
                    cv2.circle(image, (x, y), 2, (0, 255, 255), -1)
                 
            cv2.imshow("Output", image)
            cv2.waitKey(1)

        return {'PASS_THROUGH'}

    
    def endplay(m, o):
        print(format(o.frame_current) + " / " + format(o.frame_end))
        if o.frame_current == o.frame_end:
            bpy.ops.screen.animation_cancel(restore_frame=False)
    
    def camera(m):
        if m._cap == None:
            m._cap = cv2.VideoCapture(0)
            m._cap.set(cv2.CAP_PROP_FRAME_WIDTH, m.width)
            m._cap.set(cv2.CAP_PROP_FRAME_HEIGHT, m.height)
            m._cap.set(cv2.CAP_PROP_BUFFERSIZE, 1) #no of frames stored in buffer
            time.sleep(0.5)

def to_register():
    bpy.utils.register_class(CV)

def to_unregister():
    bpy.utils.unregister_class(CV)

if __name__ == "__main__":
    to_register()

     #test call
bpy.ops.mw.opencv_operator()


```

## Source file: Blender/CVB.py

{
  "path": "Blender/CVB.py",
  "blob_sha": "9441de876e5a0c7ac95e600fddb74a08056095d8",
  "sha256": "5e2f81fdbc101088c3ba9213a8205f2a5f7fde1816873dc3056592657cbeea28",
  "lines": 32,
  "bytes": 696,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/Sonaly03/Joint-Face-Detection-and-Facial-Motion-Retargeting-for-Multiple-Faces/blob/6fa4064b6f0305108ffedef7eba07f9f43972fc3/Blender/CVB.py",
  "function_signature_candidates": [
    {
      "line": 17,
      "text": "def draw(context, layout, tool):"
    },
    {
      "line": 23,
      "text": "def to_register():"
    },
    {
      "line": 27,
      "text": "def to_unregister():"
    }
  ]
}

```text

import bpy


class CV_Project(bpy.types.WorkSpaceTool):

    bl_label = "CV Project"
    bl_space_type = 'VIEW_3D'
    bl_context_mode='OBJECT'
        
    
    bl_idname = "ui_plusr0rf8.opencv"
    bl_options = {'REGISTER'}

    bl_icon = "ops.generic.select_circle"

        
    def draw(context, layout, tool):

        store_row = layout.row()
        store_op = store_row.operator("wm.opencv_operator", text="Capture", icon="OUTLINER_OB_CAMERA")
        

def to_register():
    
    bpy.utils.register_tool(CV_Project, separator=True, group=True)

def to_unregister():

    bpy.utils.unregister_tool(CV_Project)

if __name__ == "__main__":
    to_register()

```

## Source file: PRN/predictor.py

{
  "path": "PRN/predictor.py",
  "blob_sha": "d73100ffd69e88a4943ad7347904cd53e4b3b529",
  "sha256": "ea9668aea506ff67cb0928fd0257b5da00337af9f269aa185605bccc525ffc49",
  "lines": 105,
  "bytes": 5266,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 1
  },
  "url": "https://github.com/Sonaly03/Joint-Face-Detection-and-Facial-Motion-Retargeting-for-Multiple-Faces/blob/6fa4064b6f0305108ffedef7eba07f9f43972fc3/PRN/predictor.py",
  "function_signature_candidates": [
    {
      "line": 6,
      "text": "def rb(x, num_out, kernel_size = 4, stride=1, a_fn=t.nn.relu, n_fn=cl.batch_norm, scope=None):"
    },
    {
      "line": 24,
      "text": "def __init__(self, resolution_input = 256, resolution_output = 256, channel = 3, name = 'resfcn256'):"
    },
    {
      "line": 30,
      "text": "def __call__(self, x, is_training = True):"
    },
    {
      "line": 74,
      "text": "def vars(self):"
    },
    {
      "line": 79,
      "text": "def __init__(self, resolution_input = 256, resolution_output = 256):"
    },
    {
      "line": 92,
      "text": "def restore(self, model_path):"
    },
    {
      "line": 95,
      "text": "def predict(self, image):"
    },
    {
      "line": 101,
      "text": "def predict_batch(self, images):"
    }
  ]
}

```text

import tensorflow as t
import tensorflow.contrib.layers as cl
from tensorflow.contrib.framework import arg_scope
import numpy as np

def rb(x, num_out, kernel_size = 4, stride=1, a_fn=t.nn.relu, n_fn=cl.batch_norm, scope=None):
    assert num_out%2==0
    with t.variable_scope(scope, 'resBlock'):
        shortcut = x
        if stride != 1 or x.get_shape()[3] != num_out:
            shortcut = cl.conv2d(shortcut, num_out, kernel_size=1, stride=stride, 
                        activation_fn=None, normalizer_fn=None, scope='shortcut')
        x = cl.conv2d(x, num_out/2, kernel_size=1, stride=1, padding='SAME')
        x = cl.conv2d(x, num_out/2, kernel_size=kernel_size, stride=stride, padding='SAME')
        x = cl.conv2d(x, num_out, kernel_size=1, stride=1, activation_fn=None, padding='SAME', normalizer_fn=None)

        x += shortcut       
        x = n_fn(x)
        x = a_fn(x)
    return x


class resfcn256(object):
    def __init__(self, resolution_input = 256, resolution_output = 256, channel = 3, name = 'resfcn256'):
        self.name = name
        self.channel = channel
        self.resolution_input = resolution_input
        self.resolution_output = resolution_output

    def __call__(self, x, is_training = True):
        with t.variable_scope(self.name) as scope:
            with arg_scope([cl.batch_norm], is_training=is_training, scale=True):
                with arg_scope([cl.conv2d, cl.conv2d_transpose], activation_fn=t.nn.relu, 
                                     normalizer_fn=cl.batch_norm, 
                                     biases_initializer=None, 
                                     padding='SAME',
                                     weights_regularizer=cl.l2_regularizer(0.0002)):
                    size = 16  
                    
                    se = cl.conv2d(x, num_outputs=size, kernel_size=4, stride=1) 
                    se = rb(se, num_out=size * 2, kernel_size=4, stride=2) 
                    se = rb(se, num_out=size * 2, kernel_size=4, stride=1)
                    se = rb(se, num_out=size * 4, kernel_size=4, stride=2) 
                    se = rb(se, num_out=size * 4, kernel_size=4, stride=1) 
                    se = rb(se, num_out=size * 8, kernel_size=4, stride=2) 
                    se = rb(se, num_out=size * 8, kernel_size=4, stride=1) 
                    se = rb(se, num_out=size * 16, kernel_size=4, stride=2)
                    se = rb(se, num_out=size * 16, kernel_size=4, stride=1) 
                    se = rb(se, num_out=size * 32, kernel_size=4, stride=2)
                    se = rb(se, num_out=size * 32, kernel_size=4, stride=1) 

                    pd = cl.conv2d_transpose(se, size * 32, 4, stride=1) 
                    pd = cl.conv2d_transpose(pd, size * 16, 4, stride=2)  
                    pd = cl.conv2d_transpose(pd, size * 16, 4, stride=1)  
                    pd = cl.conv2d_transpose(pd, size * 16, 4, stride=1)  
                    pd = cl.conv2d_transpose(pd, size * 8, 4, stride=2)  
                    pd = cl.conv2d_transpose(pd, size * 8, 4, stride=1)  
                    pd = cl.conv2d_transpose(pd, size * 8, 4, stride=1)  
                    pd = cl.conv2d_transpose(pd, size * 4, 4, stride=2)  
                    pd = cl.conv2d_transpose(pd, size * 4, 4, stride=1)
                    pd = cl.conv2d_transpose(pd, size * 4, 4, stride=1) 
                    
                    pd = cl.conv2d_transpose(pd, size * 2, 4, stride=2)
                    pd = cl.conv2d_transpose(pd, size * 2, 4, stride=1)
                    pd = cl.conv2d_transpose(pd, size, 4, stride=2) 
                    pd = cl.conv2d_transpose(pd, size, 4, stride=1) 

                    pd = cl.conv2d_transpose(pd, 3, 4, stride=1) 
                    pd = cl.conv2d_transpose(pd, 3, 4, stride=1)
                    pos = cl.conv2d_transpose(pd, 3, 4, stride=1, activation_fn = t.nn.sigmoid)
                                
                    return pos
    @property
    def vars(self):
        return [var for var in t.global_variables() if self.name in var.name]


class FacePointPositionPredictor():
    def __init__(self, resolution_input = 256, resolution_output = 256): 

        self.resolution_input = resolution_input
        self.resolution_output = resolution_output
        self.MaxPos = resolution_input*1.1

        self.network = resfcn256(self.resolution_input, self.resolution_output)


        self.x = t.placeholder(t.float32, shape=[None, self.resolution_input, self.resolution_input, 3])  
        self.x_op = self.network(self.x, is_training = False)
        self.sess = t.Session(config=t.ConfigProto(gpu_options=t.GPUOptions(allow_growth=True)))

    def restore(self, model_path):        
        t.train.Saver(self.network.vars).restore(self.sess, model_path)
 
    def predict(self, image):
        pos = self.sess.run(self.x_op, 
                    feed_dict = {self.x: image[np.newaxis, :,:,:]})
        pos = np.squeeze(pos)
        return pos*self.MaxPos

    def predict_batch(self, images):
        pos = self.sess.run(self.x_op, 
                    feed_dict = {self.x: images})
        return pos*self.MaxPos



```
