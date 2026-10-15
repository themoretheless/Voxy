# Facial rendering source scan: ziyeshanwai/BlendShapeMaker

Repository: https://github.com/ziyeshanwai/BlendShapeMaker

Pinned source revision: 5a32bbf6651d9cea1c8441db446ac3c9f5845b3e

Discovery topic: blendshape

Description: BlendShapeMaker python3.6

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"eye": ["Inputs/Mario_eye.png", "Results/Mario_eye.png"], "blendshape": ["BlendshapeMaker.py", "data/blendshapes/00m.obj", "data/blendshapes/01m.obj", "data/blendshapes/02m.obj", "data/blendshapes/03m.obj", "data/blendshapes/04m.obj", "data/blendshapes/05m.obj", "data/blendshapes/06m.obj", "data/blendshapes/07m.obj", "data/blendshapes/08m.obj", "data/blendshapes/09m.obj", "data/blendshapes/10m.obj", "data/blendshapes/11m.obj", "data/blendshapes/12m.obj", "data/blendshapes/13m.obj", "data/blendshapes/14m.obj", "data/blendshapes/15m.obj", "data/blendshapes/16m.obj", "data/blendshapes/17m.obj", "data/blendshapes/18m.obj", "data/blendshapes/19m.obj", "data/blendshapes/20m.obj", "data/blendshapes/21m.obj", "data/blendshapes/22m.obj", "data/blendshapes/23m.obj", "data/blendshapes/24m.obj", "data/blendshapes/25m.obj", "data/blendshapes/26m.obj", "data/blendshapes/27m.obj", "data/blendshapes/28m.obj"], "deform": ["paper/Onizuka_Landmark-Guided_Deformation_Transfer_of_Template_Facial_Expressions_for_Automatic_Generation_ICCVW_2019_paper.pdf"]}

## Source file: BlendshapeMaker.py

{
  "path": "BlendshapeMaker.py",
  "blob_sha": "929ace54ea2fc38be9b0ec64dc2d72aac54f679c",
  "sha256": "86fef2fb4e097e93771d79aeb50a763a097194343a4a74ae4b5300a60e7814b2",
  "lines": 34,
  "bytes": 844,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 1
  },
  "url": "https://github.com/ziyeshanwai/BlendShapeMaker/blob/5a32bbf6651d9cea1c8441db446ac3c9f5845b3e/BlendshapeMaker.py",
  "function_signature_candidates": [
    {
      "line": 13,
      "text": "def main():"
    }
  ]
}

```text

#!/usr/bin/env python2
# -*- coding: utf-8 -*-

import _tkinter
import tkinter as tk
# import imp
import os
import sys
APP_ROOT = os.path.dirname( os.path.abspath( __file__ ) )
sys.path.append(os.path.join(APP_ROOT,'lib'))
import Menu
import BMApp
def main():
    
    ''' Create Menu to load data '''
    # M = imp.load_source('Menu', APP_ROOT+'/lib/Menu.py')
    root = tk.Tk()
    root.title(u"BlendshapeMaker")
    menu_app = Menu.Menu(root)
    menu_app.mainloop()
    
    '''　If a data path has been selected:
        Create the main application and start the loop'''
    if (len(menu_app.filename) > 0):
        # A = imp.load_source('BMApp', APP_ROOT+'/lib/BMApp.py')
        root = tk.Tk()
        app = BMApp.BMApp(menu_app.filename, root)
        app.mainloop()
    
    return 0

if __name__ == '__main__':
    main()
    exit(0)

```

## Source file: lib/Menu.py

{
  "path": "lib/Menu.py",
  "blob_sha": "985c4f446030c42f1664211da09b544b90f84e10",
  "sha256": "114e0e865039fd0d32f5a8791defc4e4bf4fb58147c055d4742d2ba172ba0d9b",
  "lines": 47,
  "bytes": 1754,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/ziyeshanwai/BlendShapeMaker/blob/5a32bbf6651d9cea1c8441db446ac3c9f5845b3e/lib/Menu.py",
  "function_signature_candidates": [
    {
      "line": 16,
      "text": "def key(self, event):"
    },
    {
      "line": 20,
      "text": "def callback(self, event):"
    },
    {
      "line": 31,
      "text": "def __init__(self, master=None):"
    }
  ]
}

```text

# File created by Diego Thomas the 21-11-2016
# File overwrited by hayato Onizuka the 13-09-2017

# File to handle the menu main loop
import cv2
import _tkinter
import tkinter as tk
from PIL import ImageTk
from tkinter.filedialog import askopenfilename

from os import path
APP_ROOT = path.dirname( path.abspath( __file__ ) )

## Class to handle menu GUI
class Menu(tk.Frame):
    def key(self, event):
        if (event.keysym == 'Escape'):
            self.root.destroy()

    def callback(self, event):
        x = event.x
        y = event.y
        if (self.menu_label[y,x,2] == 255 and self.menu_label[y,x,1] == 0 and self.menu_label[y,x,0] == 0):
            self.filename = askopenfilename(filetypes=[('.ply','*.ply')]) # show an "Open" dialog box and return the path to the selected file
            if(len(self.filename) > 0):
                print(self.filename)
                self.root.destroy()
        elif (self.menu_label[y,x,2] == 0 and self.menu_label[y,x,1] == 255 and self.menu_label[y,x,0] == 0):
            self.root.destroy() # destroy the window
    
    def __init__(self, master=None):
        self.root = master
        self.filename = ""
        
        tk.Frame.__init__(self, master)
        self.pack()
        
        self.canvas = tk.Canvas(self, bg="white", height=768, width=1024)
        self.canvas.pack()
    
        #self.menu = ImageTk.PhotoImage(file = "./images/menu_image.jpeg")
        self.menu = ImageTk.PhotoImage(file = APP_ROOT+"/../images/menu_image.jpeg")
        self.canvas.create_image(0, 0, anchor=tk.NW, image=self.menu)
        self.menu_label = cv2.imread(APP_ROOT+"/../images/menu_label.tiff")

        self.root.bind("<Key>", self.key)
        self.root.bind("<Button-1>", self.callback)


```

## Source file: lib/BMApp.py

{
  "path": "lib/BMApp.py",
  "blob_sha": "9349d4ae661fdda137b5bab037229cc5a0cf03fe",
  "sha256": "6b3e9cf8cb91a9a8ae27cbdde280cd1252b27bd35628c2e6edd8ffeaf384eaba",
  "lines": 77,
  "bytes": 2102,
  "selection_score": 0,
  "keyword_line_counts": {
    "blendshape": 6,
    "deform": 3
  },
  "url": "https://github.com/ziyeshanwai/BlendShapeMaker/blob/5a32bbf6651d9cea1c8441db446ac3c9f5845b3e/lib/BMApp.py",
  "function_signature_candidates": [
    {
      "line": 19,
      "text": "def __init__(self, path, master=None):"
    },
    {
      "line": 39,
      "text": "def startapp(self):"
    },
    {
      "line": 49,
      "text": "def routine(self):"
    }
  ]
}

```text

#!/usr/bin/env python2
# -*- coding: utf-8 -*-

from math import cos, sin, pi
from os import path
import imp
import time
from PIL import Image
from PIL import ImageTk
import cv2
import numpy as np
import _tkinter
import tkinter as tk
APP_ROOT = path.dirname( path.abspath( __file__ ) )
BMManager = imp.load_source('BMManager', APP_ROOT + '/BMManager.py')


class BMApp(tk.Frame):
    def __init__(self, path, master=None):
        self.path = path        #path for 3D Avatar
        self.root = master

        tk.Frame.__init__(self, master)

        self.pack()

        self.root.title(u"BlendshapeMaker")
        self.root.geometry("500x150+450+50")
        self.root.resizable(0, 0)

        self.Label = tk.Label(self.root, text=u"Make Blendshape models of\n\n%s\n" % self.path)
        self.Label.pack()

        self.Button = tk.Button(self.root, text=u'Start', padx=50, command=self.startapp)
        self.Button.pack()

        self.root.mainloop()

    def startapp(self):

        #disable Start Button
        self.Button.configure(state='disabled')

        self.Label2 = tk.Label(self.root, text=u"\nProcessing...")
        self.Label2.pack()
        self.root.after(50, self.routine)


    def routine(self):
        start_routine = time.time()
        self.root.destroy()
        #Load blendshapes
        print("******Load given Blendshape meshes******")
        self.BM = BMManager.BMMng('data/blendshapes/', self.path)
        self.BM.LoadBS()
        print(len(self.BM.BlendShapes), "Blendshapes are loaded")

        #Load Avatar
        print("******Load Avatar******")
        self.BM.LoadAvatar()

        #Rescale
        print("******Rescale Avatar******")
        self.BM.RescaleAvatar()

        #Triangle Correspondence
        print("******Triangle Correspondence******")
        self.BM.TriangleCorrespondence(0)

        #Deformation Transfer
        print("******Deformation transfer******")
        self.BM.DeformationTransfer()


        elapsed_time = time.time() - start_routine
        #End
        print("Complete. (Total : {:.2f} min)".format(elapsed_time/60))


```

## Source file: lib/show3d.py

{
  "path": "lib/show3d.py",
  "blob_sha": "cfe7cbdd985f6b9a3a3fb6b91447b42876ced003",
  "sha256": "8d4f7c6fa2f93e90dad80018970032a9a1beb1c1625ac9448881bd0031c0b74c",
  "lines": 1515,
  "bytes": 68932,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 60,
    "face": 89
  },
  "url": "https://github.com/ziyeshanwai/BlendShapeMaker/blob/5a32bbf6651d9cea1c8441db446ac3c9f5845b3e/lib/show3d.py",
  "function_signature_candidates": [
    {
      "line": 18,
      "text": "def __init__(self, filename):"
    },
    {
      "line": 21,
      "text": "def LoadMesh(self):"
    },
    {
      "line": 88,
      "text": "def DrawMesh(self):"
    },
    {
      "line": 211,
      "text": "def right_press(self, event):"
    },
    {
      "line": 215,
      "text": "def left_press(self, idx):"
    },
    {
      "line": 219,
      "text": "def center_press(self, event):"
    },
    {
      "line": 223,
      "text": "def right_motion(self, event):"
    },
    {
      "line": 235,
      "text": "def center_motion(self, event):"
    },
    {
      "line": 247,
      "text": "def wheel_scroll(self, event):"
    },
    {
      "line": 253,
      "text": "def Update_Mesh(self):"
    },
    {
      "line": 259,
      "text": "def Update_idx_f(self, i):"
    },
    {
      "line": 297,
      "text": "def __init__(self, master, Avatar, Source, Corr):"
    },
    {
      "line": 361,
      "text": "def DrawMesh(self, meshnum):"
    },
    {
      "line": 489,
      "text": "def Update_Mesh(self, meshnum):"
    },
    {
      "line": 495,
      "text": "def left_press(self, event):"
    },
    {
      "line": 522,
      "text": "def right_press(self, event, meshnum):"
    },
    {
      "line": 526,
      "text": "def center_press(self, event, meshnum):"
    },
    {
      "line": 530,
      "text": "def right_motion(self, event, meshnum):"
    },
    {
      "line": 555,
      "text": "def center_motion(self, event, meshnum):"
    },
    {
      "line": 580,
      "text": "def wheel_scroll(self, event, meshnum):"
    },
    {
      "line": 589,
      "text": "def press_check(self, event):"
    },
    {
      "line": 594,
      "text": "def change_landmark(self, i):"
    },
    {
      "line": 605,
      "text": "def change_candidate(self, i):"
    },
    {
      "line": 616,
      "text": "def register_landmark(self):"
    },
    {
      "line": 624,
      "text": "def destroy_window(self):"
    },
    {
      "line": 627,
      "text": "def __init__(self, master, Avatar, Source, source_landmarks, previous_landmarks):"
    },
    {
      "line": 723,
      "text": "def main():"
    },
    {
      "line": 752,
      "text": "def DrawMesh(self, meshnum):"
    },
    {
      "line": 878,
      "text": "def Update_Mesh(self, meshnum):"
    },
    {
      "line": 884,
      "text": "def left_press(self, event, meshnum):"
    },
    {
      "line": 937,
      "text": "def right_press(self, event, meshnum):"
    },
    {
      "line": 941,
      "text": "def center_press(self, event, meshnum):"
    },
    {
      "line": 945,
      "text": "def right_motion(self, event, meshnum):"
    },
    {
      "line": 970,
      "text": "def center_motion(self, event, meshnum):"
    },
    {
      "line": 995,
      "text": "def wheel_scroll(self, event, meshnum):"
    },
    {
      "line": 1004,
      "text": "def press_check(self, event):"
    },
    {
      "line": 1009,
      "text": "def change_landmark(self, i):"
    },
    {
      "line": 1020,
      "text": "def change_candidate(self, i):"
    },
    {
      "line": 1031,
      "text": "def register_landmark(self):"
    },
    {
      "line": 1039,
      "text": "def destroy_window(self):"
    },
    {
      "line": 1042,
      "text": "def __init__(self, master, Avatar, Source, StoA, AtoS, AvatarAdjacent_v):"
    },
    {
      "line": 1144,
      "text": "def DrawMesh(self, meshnum):"
    },
    {
      "line": 1279,
      "text": "def Update_Mesh(self, meshnum):"
    },
    {
      "line": 1285,
      "text": "def left_press(self, event, meshnum):"
    },
    {
      "line": 1314,
      "text": "def right_press(self, event, meshnum):"
    },
    {
      "line": 1318,
      "text": "def center_press(self, event, meshnum):"
    },
    {
      "line": 1322,
      "text": "def right_motion(self, event, meshnum):"
    },
    {
      "line": 1347,
      "text": "def center_motion(self, event, meshnum):"
    },
    {
      "line": 1372,
      "text": "def wheel_scroll(self, event, meshnum):"
    },
    {
      "line": 1381,
      "text": "def press_check(self, event):"
    }
  ]
}

```text

#!/usr/bin/env python2
# -*- coding: utf-8 -*-

import _tkinter
import tkinter as tk
import numpy as np
import numpy.linalg as LA
import imp
import time
from math import pi
from tkinter.filedialog import askopenfilename
from operator import itemgetter



class MyMesh():
    
    def __init__(self, filename):
        self.filename = filename

    def LoadMesh(self):
        print("Loading ", self.filename)

        f = open(self.filename, 'r')
        idx_v = 0
        idx_f = 0
        num_v = 0
        num_f = 0

        for line in f:
            words = line.split()
            #read header
            if (words[0] == "ply"):
                print("continue")
                continue

            if(words[0]=="format"):
                if(words[1]=="ascii"):
                    continue
                else:
                    print("cannot read this format")
                    break

            if(words[0]=="element"):
                if(words[1]=="vertex"):
                    num_v = int(words[2])
                    self.Vertices = np.zeros((int(words[2]), 4), dtype = np.float32)

                elif(words[1]=="face"):
                    num_f = int(words[2])
                    self.Faces = np.zeros((int(words[2]), 6), dtype = np.int32)
                    self.Normals = np.zeros((int(words[2]), 3), dtype = np.float32)

                continue

            if(words[0]=="end_header"):
                continue

            if (idx_v < num_v):
                self.Vertices[idx_v] = [float(words[0]), float(words[1]), float(words[2]), idx_v]
                idx_v += 1
                continue


            if(len(words)<12):
                self.Faces[idx_f]=np.array([int(words[1]), 0, int(words[2]), 0,  int(words[3]), 0])
            else:
                self.Faces[idx_f]=np.array([int(words[1]), 0, int(words[2]), 0,  int(words[3]), 0])


            idx_f+=1

            if(idx_f >= num_f):
                break

        #compute triangle normals
        for i in range(0, len(self.Faces)):
            cross = np.cross(self.Vertices[self.Faces[i, 2],0:3] - self.Vertices[self.Faces[i, 0],0:3], self.Vertices[self.Faces[i, 4],0:3] - self.Vertices[self.Faces[i, 0],0:3])
            self.Normals[i] = cross / LA.norm(cross)

        f.close()


class Visualize(tk.Frame):

    

    def DrawMesh(self):

        RotX = np.array([[1., 0., 0., 0.], \
                         [0., np.cos(self.anglex), np.sin(self.anglex), 0.], \
                         [0., -np.sin(self.anglex), np.cos(self.anglex), 0.], \
                         [0., 0., 0., 1.]], dtype=np.float32)

        RotY = np.array([[np.cos(self.angley), 0., -np.sin(self.angley), 0.], \
                         [0., 1., 0., 0.], \
                         [np.sin(self.angley), 0., np.cos(self.angley), 0.], \
                         [0., 0., 0., 1.]], dtype=np.float32)

        self.Cam_Rot = np.dot(RotX, np.dot(RotY, np.identity(4, dtype=np.float32))) #world camera rotation
        self.Cam_Pos = np.dot(self.Cam_Rot.T, [self.translation_x, self.translation_y, 400*self.scale, 1]) #world camera position

        t = -np.dot(self.Cam_Rot, self.Cam_Pos)[0:3].reshape(3, 1)
        self.W2Cam = np.vstack((np.hstack((self.Cam_Rot[0:3,0:3], t)), [0, 0, 0, 1]))

        self.intrinsic = np.array([[500, 0, self.Size[1]/2], 
                                   [0, 500, self.Size[0]/2], 
                                   [0, 0, 1]])

        #World to Camera
        pt = np.array([0., 0., 0., 1.])
        Vertices_ac = np.empty((0, 4), dtype=np.float32)
        for i in range(len(self.Avatar.Vertices)):
            pt[0:3] = self.Avatar.Vertices[i,0:3]
            Vertices_ac = np.vstack((Vertices_ac, np.dot(self.W2Cam, pt)))

        Vertices_sc = np.empty((0, 4), dtype=np.float32)
        for i in range(len(self.Source.Vertices)):
            pt[0:3] = self.Source.Vertices[i,0:3]
            Vertices_sc = np.vstack((Vertices_sc, np.dot(self.W2Cam, pt)))


        #sort
        CentroidZa = []
        Face_idxa = []
        for i in range(len(self.Avatar.Faces)):
            CentroidZa += [[i, (Vertices_ac[self.Avatar.Faces[i,0],2] + Vertices_ac[self.Avatar.Faces[i,2],2] + Vertices_ac[self.Avatar.Faces[i,4],2])/3]]
        CentroidZa.sort(key=itemgetter(1))
        for i in range(len(self.Avatar.Faces)):
            Face_idxa += [CentroidZa[i][0]]

        CentroidZs = []
        Face_idxs = []
        for i in range(len(self.Source.Faces)):
            CentroidZs += [[i, (Vertices_sc[self.Source.Faces[i,0],2] + Vertices_sc[self.Source.Faces[i,2],2] + Vertices_sc[self.Source.Faces[i,4],2])/3]]
        CentroidZs.sort(key=itemgetter(1))
        for i in range(len(self.Source.Faces)):
            Face_idxs += [CentroidZs[i][0]]

        #draw polygons from back to front
        pix = np.array([0., 0., 1.])
        for i in Face_idxa:
            poly = []
            gaze = (self.Cam_Pos[0:3] - self.Avatar.Vertices[self.Avatar.Faces[i,0],0:3]) / LA.norm(self.Cam_Pos[0:3] - self.Avatar.Vertices[self.Avatar.Faces[i,0],0:3])
            if(np.dot(self.Avatar.Normals[i], gaze)>=0):
                for k in range(3):
                    tmp = Vertices_ac[self.Avatar.Faces[i,2*k]]
                    pix = np.dot(self.intrinsic, [-tmp[0]/tmp[2], tmp[1]/tmp[2], 1])
                    column_index = int(round(pix[0]))
                    line_index = int(round(pix[1]))
                    poly.append((column_index, line_index))

                darkness = str(hex(int(128*(np.dot(self.Avatar.Normals[i], gaze)+0.7))))[2::]
                darkness = "#" + darkness + darkness + darkness

                
                if(i == self.idx_f):
                    self.canvas.create_polygon(poly, fill="red", outline="black", tag="apoly"+str(i))
                    self.canvas.tag_bind("apoly"+str(i), "<Button-1>", lambda event, idx=i: self.left_press(idx))
                elif(self.idx_f>(len(self.Avatar.Faces)-1)):
                    drawn = False
                    for j in range(len(self.opponent)):
                        if(i == self.opponent[j]):
                            self.canvas.create_polygon(poly, fill="green", outline="black", tag="apoly"+str(i))
                            self.canvas.tag_bind("apoly"+str(i), "<Button-1>", lambda event, idx=i: self.left_press(idx))
                            drawn = True
                            break
                    if(drawn==False):
                        self.canvas.create_polygon(poly, fill=darkness, tag="apoly"+str(i))
                        self.canvas.tag_bind("apoly"+str(i), "<Button-1>", lambda event, idx=i: self.left_press(idx))
                else:
                    self.canvas.create_polygon(poly, fill=darkness, tag="apoly"+str(i))
                    self.canvas.tag_bind("apoly"+str(i), "<Button-1>", lambda event, idx=i: self.left_press(idx))
                


        for i in Face_idxs:
            poly = []
            gaze = (self.Cam_Pos[0:3] - self.Source.Vertices[self.Source.Faces[i,0],0:3]) / LA.norm(self.Cam_Pos[0:3] - self.Source.Vertices[self.Source.Faces[i,0],0:3])
            if(np.dot(self.Source.Normals[i], gaze)>=0):
                for k in range(3):
                    tmp = Vertices_sc[self.Source.Faces[i,2*k]]
                    pix = np.dot(self.intrinsic, [-tmp[0]/tmp[2], tmp[1]/tmp[2], 1])
                    column_index = int(round(pix[0]))
                    line_index = int(round(pix[1]))
                    poly.append((column_index, line_index))

                darkness = str(hex(int(128*(np.dot(self.Source.Normals[i], gaze)+0.7))))[2::]
                darkness = "#" + darkness + darkness + darkness

                
                if(i == (self.idx_f-len(self.Avatar.Faces))):
                    self.canvas2.create_polygon(poly, fill="green", outline="black", tag="spoly"+str(i))
                    self.canvas2.tag_bind("spoly"+str(i), "<Button-1>", lambda event, idx=i+len(self.Avatar.Faces): self.left_press(idx))
                elif(self.idx_f<(len(self.Avatar.Faces)-1)):
                    drawn = False
                    for j in range(len(self.opponent)):
                        if(i == self.opponent[j]):
                            self.canvas2.create_polygon(poly, fill="red", outline="black", tag="spoly"+str(i))
                            self.canvas2.tag_bind("spoly"+str(i), "<Button-1>", lambda event, idx=i+len(self.Avatar.Faces): self.left_press(idx))
                            drawn = True
                            break                            
                    if(drawn==False):
                        self.canvas2.create_polygon(poly, fill=darkness, tag="spoly"+str(i))
                        self.canvas2.tag_bind("spoly"+str(i), "<Button-1>", lambda event, idx=i+len(self.Avatar.Faces): self.left_press(idx))
                else:
                    self.canvas2.create_polygon(poly, fill=darkness, tag="spoly"+str(i))
                    self.canvas2.tag_bind("spoly"+str(i), "<Button-1>", lambda event, idx=i+len(self.Avatar.Faces): self.left_press(idx))
                

    def right_press(self, event):
        self.right_x = event.x
        self.right_y = event.y

    def left_press(self, idx):
        self.idx_f = idx
        self.Update_idx_f(4)

    def center_press(self, event):
        self.center_x = event.x
        self.center_y = event.y
    
    def right_motion(self, event):
        delta_x = event.x - self.right_x
        delta_y = event.y - self.right_y

        self.angley -= 0.01*delta_x
        self.anglex -= 0.01*delta_y

        self.Update_Mesh()

        self.right_x = event.x
        self.right_y = event.y

    def center_motion(self, event):
        delta_x = event.x - self.center_x
        delta_y = event.y - self.center_y

        self.translation_x -= 0.5*delta_x
        self.translation_y -= -0.5*delta_y

        self.Update_Mesh()

        self.center_x = event.x
        self.center_y = event.y

    def wheel_scroll(self, event):
        if(self.scale + event.delta*0.1/abs(event.delta)>0.1):
            self.scale += event.delta*0.1/abs(event.delta)

        self.Update_Mesh()

    def Update_Mesh(self):
        self.canvas.delete(tk.ALL)
        self.canvas2.delete(tk.ALL)

        self.DrawMesh()

    def Update_idx_f(self, i):
        #control idx_f
        if(i==0 and self.idx_f > 0):
            self.idx_f -= 1
        elif(i==1 and self.idx_f < (len(self.Avatar.Faces)+len(self.Source.Faces)-1)):
            self.idx_f += 1
        elif(i==2 and int(self.EditBox.get()) >= 0 and int(self.EditBox.get()) < (len(self.Avatar.Faces)+len(self.Source.Faces))):
            self.idx_f = int(self.EditBox.get())
            
        #compute corresponding triangles of face[idx_f]
        self.opponent = []
        if(self.idx_f < len(self.Avatar.Faces)-1):
            for i in range(len(self.Corr)):
                if(self.Corr[i][1] == self.idx_f):
                    self.opponent += [self.Corr[i][0]]

            self.canvas.itemconfig("apoly"+str(self.idx_f), fill="red", outline="black")
            print("apoly"+str(self.idx_f))
            for i in self.opponent:
                self.canvas2.itemconfig("spoly"+str(i), fill="red", outline="black")
            print("opponent: s", self.opponent)

        else:
            for i in range(len(self.Corr)):
                if(self.Corr[i][0] == (self.idx_f-len(self.Avatar.Faces))):
                    self.opponent += [self.Corr[i][1]]

            self.canvas2.itemconfig("spoly"+str(self.idx_f-len(self.Avatar.Faces)), fill="green", outline="black")
            print("spoly"+str(self.idx_f-len(self.Avatar.Faces)))
            for i in self.opponent:
                self.canvas.itemconfig("apoly"+str(i), fill="green", outline="black")
            print("opponent: a", self.opponent)
        
        

        self.EditBox.delete(0, tk.END)
        self.EditBox.insert(tk.END,str(self.idx_f))

    def __init__(self, master, Avatar, Source, Corr):
        self.root = master
        self.Avatar = Avatar
        self.Source = Source
        self.Corr = Corr

        self.Size = (300,500) #Size of the canvas
        self.scale = 1
        self.anglex = 0
        self.angley = 0
        self.translation_x = 0
        self.translation_y = 0
        self.idx_f = 0
        self.opponent = []


        tk.Frame.__init__(self, master)
        self.pack()

        self.Console = tk.Frame(self.root)
        self.Console.pack()
        self.EditBox = tk.Entry(self.Console)
        self.EditBox.insert(tk.END,str(self.idx_f))
        self.Down = tk.Button(self.Console, text="<", command=lambda i=0: self.Update_idx_f(i))
        self.Up = tk.Button(self.Console, text=">", command=lambda i=1: self.Update_idx_f(i))
        
        self.Down.pack(side="left")
        self.EditBox.pack(side="left")
        self.Up.pack(side="left")

        self.Window = tk.Frame(self.root)
        self.Window2 = tk.Frame(self.root)
        

        self.canvas = tk.Canvas(self.Window, bg="black", height=self.Size[0], width=self.Size[1])
        self.canvas2 = tk.Canvas(self.Window2, bg="black", height=self.Size[0], width=self.Size[1])
        self.DrawMesh()

        self.AvatarInfo = tk.Label(self.Window, text="Vertices: %d\nFaces: %d" %(len(Avatar.Vertices), len(Avatar.Faces)))
        self.SourceInfo = tk.Label(self.Window2, text="Vertices: %d\nFaces: %d" %(len(Source.Vertices), len(Source.Faces)))

        self.Window.pack(side="right")
        self.Window2.pack(side="right")
        self.canvas.pack()
        self.canvas2.pack()
        self.AvatarInfo.pack()
        self.SourceInfo.pack()

        
        
        self.root.bind("<Button-2>", self.center_press)
        self.root.bind("<Button-3>", self.right_press)

        self.root.bind("<B2-Motion>", self.center_motion)
        self.root.bind("<B3-Motion>", self.right_motion)
        self.root.bind("<MouseWheel>", self.wheel_scroll)
        self.root.bind("<Return>", lambda event, i=2: self.Update_idx_f(i))






class SelectLandmarks(tk.Frame):
    def DrawMesh(self, meshnum):
        if(meshnum==0): 
            anglex = self.anglex
            angley = self.angley
            translation_x = self.translation_x
            translation_y = self.translation_y
            scale = self.scale
        elif(meshnum==1): 
            anglex = self.anglex_s
            angley = self.angley_s
            translation_x = self.translation_x_s
            translation_y = self.translation_y_s
            scale = self.scale_s

        RotX = np.array([[1., 0., 0., 0.], 
                         [0., np.cos(anglex), np.sin(anglex), 0.], 
                         [0., -np.sin(anglex), np.cos(anglex), 0.], 
                         [0., 0., 0., 1.]], dtype=np.float32)

        RotY = np.array([[np.cos(angley), 0., -np.sin(angley), 0.], 
                         [0., 1., 0., 0.], 
                         [np.sin(angley), 0., np.cos(angley), 0.], 
                         [0., 0., 0., 1.]], dtype=np.float32)

        Cam_Rot = np.dot(RotX, np.dot(RotY, np.identity(4, dtype=np.float32))) #world camera rotation
        Cam_Pos = np.dot(Cam_Rot.T, [translation_x, translation_y, 400*scale, 1]) #world camera position

        t = -np.dot(Cam_Rot, Cam_Pos)[0:3].reshape(3, 1)
        W2Cam = np.vstack((np.hstack((Cam_Rot[0:3,0:3], t)), [0, 0, 0, 1])) #view convert matrix

        #World to Camera and Camera to Pixel
        pt = np.array([0., 0., 0., 1.])
        if(meshnum==0):
            Vertices_ac = np.empty((0, 3), dtype=np.float32)
            self.Vertices_apix = np.empty((0, 2), dtype=np.float32)
            for i in range(len(self.Avatar.Vertices)):
                pt[0:3] = self.Avatar.Vertices[i,0:3]
                tmp = np.dot(W2Cam, pt)
                Vertices_ac = np.vstack((Vertices_ac, tmp[0:3]))
                pix = np.dot(self.intrinsic, [-tmp[0]/tmp[2], tmp[1]/tmp[2], 1])
                self.Vertices_apix = np.vstack((self.Vertices_apix, [int(round(pix[0])), int(round(pix[1]))]))

            #sort
            CentroidZa = []
            Face_idxa = []
            for i in range(len(self.Avatar.Faces)):
                CentroidZa += [[i, (Vertices_ac[self.Avatar.Faces[i,0],2] + Vertices_ac[self.Avatar.Faces[i,2],2] + Vertices_ac[self.Avatar.Faces[i,4],2])/3]]
            CentroidZa.sort(key=itemgetter(1))
            for i in range(len(self.Avatar.Faces)):
                Face_idxa += [CentroidZa[i][0]]

        elif(meshnum==1):
            Vertices_sc = np.empty((0, 3), dtype=np.float32)
            self.Vertices_spix = np.empty((0, 2), dtype=np.float32)
            for i in range(len(self.Source.Vertices)):
                pt[0:3] = self.Source.Vertices[i,0:3]
                tmp = np.dot(W2Cam, pt)
                Vertices_sc = np.vstack((Vertices_sc, tmp[0:3]))
                pix = np.dot(self.intrinsic, [-tmp[0]/tmp[2], tmp[1]/tmp[2], 1])
                self.Vertices_spix = np.vstack((self.Vertices_spix, [int(round(pix[0])), int(round(pix[1]))]))

            #sort
            CentroidZs = []
            Face_idxs = []
            for i in range(len(self.Source.Faces)):
                CentroidZs += [[i, (Vertices_sc[self.Source.Faces[i,0],2] + Vertices_sc[self.Source.Faces[i,2],2] + Vertices_sc[self.Source.Faces[i,4],2])/3]]
            CentroidZs.sort(key=itemgetter(1))
            for i in range(len(self.Source.Faces)):
                Face_idxs += [CentroidZs[i][0]]


        #draw polygons from back to front
        pix = np.array([0., 0., 1.])
        if(meshnum==0):
            for num in range(len(Face_idxa)):
                #start = time.time()
                i = Face_idxa[num]
                poly = []
                gaze = (Cam_Pos[0:3] - self.Avatar.Vertices[self.Avatar.Faces[i,0],0:3]) / LA.norm(Cam_Pos[0:3] - self.Avatar.Vertices[self.Avatar.Faces[i,0],0:3])
                inner = np.dot(self.Avatar.Normals[i], gaze)

                if(inner >= 0 and CentroidZa[num][1] < -20):
                    for k in range(3):
                        poly += [self.Vertices_apix[self.Avatar.Faces[i,2*k],0], self.Vertices_apix[self.Avatar.Faces[i,2*k],1]]

                    darkness = str(hex(int(128*(inner+0.7))))[2::]
                    darkness = "#" + darkness + darkness + darkness

                    if(self.wired.get()): 
                        self.canvas.create_line(poly[0], poly[1], poly[2], poly[3], fill="white")
                        self.canvas.create_line(poly[2], poly[3], poly[4], poly[5], fill="white")
                        self.canvas.create_line(poly[4], poly[5], poly[0], poly[1], fill="white")
                    else: self.canvas.create_polygon(poly, fill=darkness, tag="apoly"+str(i))
                # print("{:.7f}".format(time.time() - start))

            #draw candidates and landmarks
            for i in range(len(self.candidate)):
                idx = self.candidate[i]
                if(i == self.candidate_num):
                    self.canvas.create_oval(self.Vertices_apix[idx,0]-3, self.Vertices_apix[idx,1]-3, self.Vertices_apix[idx,0]+3, self.Vertices_apix[idx,1]+3, fill="red", tag="candidate"+str(i))
                else:self.canvas.create_oval(self.Vertices_apix[idx,0]-3, self.Vertices_apix[idx,1]-3, self.Vertices_apix[idx,0]+3, self.Vertices_apix[idx,1]+3, fill="gray", tag="candidate"+str(i))

            for i in range(len(self.AVATAR_LANDMARKS)):
                idx = self.AVATAR_LANDMARKS[i]
                if(idx != None): self.canvas.create_oval(self.Vertices_apix[idx,0]-3, self.Vertices_apix[idx,1]-3, self.Vertices_apix[idx,0]+3, self.Vertices_apix[idx,1]+3, fill="green", tag="landmark"+str(i))
            self.canvas.itemconfig("landmark"+str(self.landmark_num), width=2)

        elif(meshnum==1):
            for i in Face_idxs:
                poly = []
                gaze = (Cam_Pos[0:3] - self.Source.Vertices[self.Source.Faces[i,0],0:3]) / LA.norm(Cam_Pos[0:3] - self.Source.Vertices[self.Source.Faces[i,0],0:3])
                inner = np.dot(self.Source.Normals[i], gaze)
                if(inner>=0):
                    for k in range(3):
                        poly += [self.Vertices_spix[self.Source.Faces[i,2*k],0], self.Vertices_spix[self.Source.Faces[i,2*k],1]]

                    darkness = str(hex(int(128*(inner+0.7))))[2::]
                    darkness = "#" + darkness + darkness + darkness

                    self.canvas2.create_polygon(poly, fill=darkness, tag="spoly"+str(i))

            for i in range(len(self.SOURCE_LANDMARKS)):
                tmp = Vertices_sc[self.SOURCE_LANDMARKS[i]]
                pix = np.dot(self.intrinsic, [-tmp[0]/tmp[2], tmp[1]/tmp[2], 1])
                self.canvas2.create_oval(int(round(pix[0]))-3, int(round(pix[1]))-3, int(round(pix[0]))+3, int(round(pix[1]))+3, fill="gray", tag="landmark"+str(i))
            self.canvas2.itemconfig("landmark"+str(self.landmark_num), fill="red")


    def Update_Mesh(self, meshnum):
        if(meshnum==0): self.canvas.delete("all")
        elif(meshnum==1): self.canvas2.delete("all")

        self.DrawMesh(meshnum)

    def left_press(self, event):
        
        for i in range(len(self.candidate)):
            self.canvas.delete("candidate"+str(i))
        self.candidate = []
        self.candidate_num = 0
        
        #find candidate landmarks
        a = np.square(self.Vertices_apix - [event.x, event.y])
        norm = np.sqrt(a[:,0] + a[:,1])
        for i in range(len(self.Vertices_apix)):
            if(norm[i]<5):
                self.candidate +=[i]

        self.Label2.config(text=u"Candidate : %s" %str(self.candidate))
        if(len(self.candidate)>0):
            self.Label3.config(text=u"Selected : %s" %str(self.candidate[self.candidate_num]))
        else: self.Label3.config(text=u"Selected : None")

        for i in range(len(self.candidate)):
            idx = self.candidate[i]
            if(i == self.candidate_num):
                self.canvas.create_oval(self.Vertices_apix[idx,0]-3, self.Vertices_apix[idx,1]-3, self.Vertices_apix[idx,0]+3, self.Vertices_apix[idx,1]+3, fill="red", tag="candidate"+str(i))
            else:
                self.canvas.create_oval(self.Vertices_apix[idx,0]-3, self.Vertices_apix[idx,1]-3, self.Vertices_apix[idx,0]+3, self.Vertices_apix[idx,1]+3, fill="gray", tag="candidate"+str(i))


    def right_press(self, event, meshnum):
        if(meshnum==0): self.right_x = event.x; self.right_y = event.y
        elif(meshnum==1): self.right_x_s = event.x; self.right_y_s = event.y

    def center_press(self, event, meshnum):
        if(meshnum==0): self.center_x = event.x; self.center_y = event.y
        elif(meshnum==1): self.center_x_s = event.x; self.center_y_s = event.y

    def right_motion(self, event, meshnum):
        if(meshnum==0):
            delta_x = event.x - self.right_x
            delta_y = event.y - self.right_y

            self.angley -= 0.01*delta_x
            self.anglex -= 0.01*delta_y

            self.Update_Mesh(meshnum)

            self.right_x = event.x
            self.right_y = event.y

        elif(meshnum==1):
            delta_x = event.x - self.right_x_s
            delta_y = event.y - self.right_y_s

            self.angley_s -= 0.01*delta_x
            self.anglex_s -= 0.01*delta_y

            self.Update_Mesh(meshnum)

            self.right_x_s = event.x
            self.right_y_s = event.y

    def center_motion(self, event, meshnum):
        if(meshnum==0):
            delta_x = event.x - self.center_x
            delta_y = event.y - self.center_y

            self.translation_x -= 0.5*delta_x
            self.translation_y -= -0.5*delta_y

            self.Update_Mesh(meshnum)

            self.center_x = event.x
            self.center_y = event.y

        elif(meshnum==1):
            delta_x = event.x - self.center_x_s
            delta_y = event.y - self.center_y_s

            self.translation_x_s -= 0.5*delta_x
            self.translation_y_s -= -0.5*delta_y

            self.Update_Mesh(meshnum)

            self.center_x_s = event.x
            self.center_y_s = event.y

    def wheel_scroll(self, event, meshnum):
        if(meshnum==0):
            if(self.scale + event.delta*0.05/abs(event.delta)>0.1):
                self.scale += event.delta*0.05/abs(event.delta)
        elif(meshnum==1):
            if(self.scale_s + event.delta*0.05/abs(event.delta)>0.1):
                self.scale_s += event.delta*0.05/abs(event.delta)
        self.Update_Mesh(meshnum)

    def press_check(self, event):
        if(self.wired.get()==True): self.wired.set(False)
        else: self.wired.set(True)
        self.Update_Mesh(0)

    def change_landmark(self, i):
        self.canvas2.itemconfig("landmark"+str(self.landmark_num), fill="gray")
        self.canvas.itemconfig("landmark"+str(self.landmark_num), width=1)
        if(i==0 and self.landmark_num > 0): self.landmark_num -= 1
        elif(i==0 and self.landmark_num <= 0): self.landmark_num = len(self.SOURCE_LANDMARKS)-1
        elif(i==1 and self.landmark_num < len(self.SOURCE_LANDMARKS)-1): self.landmark_num += 1
        elif(i==1 and self.landmark_num >= len(self.SOURCE_LANDMARKS)-1): self.landmark_num = 0
        self.Label.config(text=u"Landmark %s    " %str(self.landmark_num+1))
        self.canvas2.itemconfig("landmark"+str(self.landmark_num), fill="red")
        self.canvas.itemconfig("landmark"+str(self.landmark_num), width=2)

    def change_candidate(self, i):
        self.canvas.itemconfig("candidate"+str(self.candidate_num), fill="gray")
        if(i==0 and self.candidate_num > 0): self.candidate_num -= 1
        elif(i==0 and self.candidate_num <= 0): self.candidate_num = len(self.candidate)-1
        elif(i==1 and self.candidate_num < len(self.candidate)-1): self.candidate_num += 1
        elif(i==1 and self.candidate_num >= len(self.candidate)-1): self.candidate_num = 0
        self.canvas.itemconfig("candidate"+str(self.candidate_num), fill="red")
        if(len(self.candidate)>0):
            self.Label3.config(text=u"Selected : %s" %str(self.candidate[self.candidate_num]))
        else: self.Label3.config(text=u"Selected : None")

    def register_landmark(self):
        self.canvas.delete("landmark"+str(self.landmark_num))
        idx = self.candidate[self.candidate_num]
        self.AVATAR_LANDMARKS[self.landmark_num] = idx
        self.canvas.create_oval(self.Vertices_apix[idx,0]-3, self.Vertices_apix[idx,1]-3, self.Vertices_apix[idx,0]+3, self.Vertices_apix[idx,1]+3, fill="green", width=2, tag="landmark"+str(self.landmark_num))
        self.change_landmark(1)
        print(self.AVATAR_LANDMARKS)

    def destroy_window(self):
        self.root.destroy()

    def __init__(self, master, Avatar, Source, source_landmarks, previous_landmarks):
        self.root = master
        self.Avatar = Avatar
        self.Source = Source

        self.Size = (300,500) #Size of the canvas
        self.intrinsic = np.array([[500, 0, self.Size[1]/2], 
                                   [0, 500, self.Size[0]/2], 
                                   [0, 0, 1]])
        self.scale = 1; self.scale_s = 1
        self.anglex = 0; self.anglex_s = 0
        self.angley = 0; self.angley_s = 0
        self.translation_x = 0; self.translation_x_s = 0
        self.translation_y = 0; self.translation_y_s = 0
        self.landmark_num = 0
        self.candidate_num = 0
        self.candidate = []
        self.wired = tk.BooleanVar()
        self.wired.set(False)

        self.SOURCE_LANDMARKS = source_landmarks
        self.AVATAR_LANDMARKS = [None for _ in range(len(self.SOURCE_LANDMARKS))]
        if len(previous_landmarks) > 0:
            for i in range(len(self.SOURCE_LANDMARKS)):
                self.AVATAR_LANDMARKS[i] = previous_landmarks[i]



        tk.Frame.__init__(self, master)
        self.pack()

        self.Window = tk.Frame(self.root)
        self.Window.pack(side="left", anchor="s")
        self.Window2 = tk.Frame(self.root)
        self.Window2.pack(side="left", anchor="s")

        self.Console = tk.Frame(self.Window)
        self.Console.pack(pady=15)
        self.Console2 = tk.Frame(self.Window2)
        self.Console2.pack()
        self.Console2_2 = tk.Frame(self.Console2)

        self.Label = tk.Label(self.Console, text=u"Landmark %s    " %str(self.landmark_num+1))
        self.Down = tk.Button(self.Console, text="<", command=lambda i=0: self.change_landmark(i))
        self.Up = tk.Button(self.Console, text=">", command=lambda i=1: self.change_landmark(i))
        
        self.Label.pack(side="left")
        self.Down.pack(side="left")
        self.Up.pack(side="left")
        

        self.Label2 = tk.Label(self.Console2, text=u"Candidate(s) : %s" %str(self.candidate))
        self.Label3 = tk.Label(self.Console2, text=u"Selected : None")
        self.Down2 = tk.Button(self.Console2_2, text="<", command=lambda i=0: self.change_candidate(i))
        self.Up2 = tk.Button(self.Console2_2, text=">", command=lambda i=1: self.change_candidate(i))
        self.Register = tk.Button(self.Console2_2, text="Register", command=self.register_landmark)
        self.OK = tk.Button(self.Console2_2, text="Finish", command=self.destroy_window)
        self.CheckBox = tk.Checkbutton(self.Console2_2, text = 'wireframe')
        
        self.Label2.pack()
        self.Label3.pack()
        self.Console2_2.pack()
        self.CheckBox.pack(side="left", padx=20)
        self.CheckBox.bind("<Button-1>", self.press_check)
        self.Down2.pack(side="left")
        self.Up2.pack(side="left")
        self.Register.pack(side="left")
        self.OK.pack(side="left", padx="20")
        

        self.canvas = tk.Canvas(self.Window2, bg="black", height=self.Size[0], width=self.Size[1])
        self.canvas2 = tk.Canvas(self.Window, bg="black", height=self.Size[0], width=self.Size[1])
        self.DrawMesh(0)
        self.DrawMesh(1)
        self.canvas.pack()
        self.canvas2.pack()

        
        self.canvas.bind("<Button-1>", self.left_press)
        self.canvas.bind("<Button-2>", lambda event, meshnum=0: self.center_press(event, meshnum))
        self.canvas.bind("<Button-3>", lambda event, meshnum=0: self.right_press(event, meshnum))
        self.canvas.bind("<B3-Motion>", lambda event, meshnum=0: self.right_motion(event, meshnum))
        self.canvas.bind("<B2-Motion>", lambda event, meshnum=0: self.center_motion(event, meshnum))
        self.canvas.bind("<MouseWheel>", lambda event, meshnum=0: self.wheel_scroll(event, meshnum))
        self.canvas.bind("<Enter>", lambda event: self.canvas.focus_set())
        self.canvas.bind("<Leave>", lambda event: self.root.focus_set())

        self.canvas2.bind("<Button-2>", lambda event, meshnum=1: self.center_press(event, meshnum))
        self.canvas2.bind("<Button-3>", lambda event, meshnum=1: self.right_press(event, meshnum))
        self.canvas2.bind("<B3-Motion>", lambda event, meshnum=1: self.right_motion(event, meshnum))
        self.canvas2.bind("<B2-Motion>", lambda event, meshnum=1: self.center_motion(event, meshnum))
        self.canvas2.bind("<MouseWheel>", lambda event, meshnum=1: self.wheel_scroll(event, meshnum))
        self.canvas2.bind("<Enter>", lambda event: self.canvas2.focus_set())
        self.canvas2.bind("<Leave>", lambda event: self.root.focus_set())


def main():
    root = tk.Tk()
    filename = askopenfilename(filetypes=[('.ply','*.ply')], initialdir="../MarioHead/")
    root.destroy()
    root.mainloop()
    print(filename)

    Avatar = MyMesh(filename)
    Avatar.LoadMesh()

    Source = MyMesh("../Mario Head/SourceMesh_21.ply")
    Source.LoadMesh()

    root = tk.Tk()
    root.title(u"DrawMesh")
    root.resizable(0, 0)
    init_list = []

    #V = Visualize(root, Avatar, Avatar, Adjacent)
    V = SelectLandmarks(root, Avatar, Source, init_list)
    V.mainloop()


if __name__ == '__main__':
    main()



class CVP(tk.Frame):
    def DrawMesh(self, meshnum):
        if(meshnum==0): 
            anglex = self.anglex
            angley = self.angley
            translation_x = self.translation_x
            translation_y = self.translation_y
            scale = self.scale
        elif(meshnum==1): 
            anglex = self.anglex_s
            angley = self.angley_s
            translation_x = self.translation_x_s
            translation_y = self.translation_y_s
            scale = self.scale_s

        RotX = np.array([[1., 0., 0., 0.], 
                         [0., np.cos(anglex), np.sin(anglex), 0.], 
                         [0., -np.sin(anglex), np.cos(anglex), 0.], 
                         [0., 0., 0., 1.]], dtype=np.float32)

        RotY = np.array([[np.cos(angley), 0., -np.sin(angley), 0.], 
                         [0., 1., 0., 0.], 
                         [np.sin(angley), 0., np.cos(angley), 0.], 
                         [0., 0., 0., 1.]], dtype=np.float32)

        Cam_Rot = np.dot(RotX, np.dot(RotY, np.identity(4, dtype=np.float32))) #world camera rotation
        Cam_Pos = np.dot(Cam_Rot.T, [translation_x, translation_y, 400*scale, 1]) #world camera position

        t = -np.dot(Cam_Rot, Cam_Pos)[0:3].reshape(3, 1)
        W2Cam = np.vstack((np.hstack((Cam_Rot[0:3,0:3], t)), [0, 0, 0, 1])) #view convert matrix

        #World to Camera and Camera to Pixel
        pt = np.array([0., 0., 0., 1.])
        if(meshnum==0):
            Vertices_ac = np.empty((0, 3), dtype=np.float32)
            self.Vertices_apix = np.empty((0, 2), dtype=np.float32)
            for i in range(len(self.Avatar.Vertices)):
                pt[0:3] = self.Avatar.Vertices[i,0:3]
                tmp = np.dot(W2Cam, pt)
                Vertices_ac = np.vstack((Vertices_ac, tmp[0:3]))
                pix = np.dot(self.intrinsic, [-tmp[0]/tmp[2], tmp[1]/tmp[2], 1])
                self.Vertices_apix = np.vstack((self.Vertices_apix, [int(round(pix[0])), int(round(pix[1]))]))

            #sort
            CentroidZa = []
            Face_idxa = []
            for i in range(len(self.Avatar.Faces)):
                CentroidZa += [[i, (Vertices_ac[self.Avatar.Faces[i,0],2] + Vertices_ac[self.Avatar.Faces[i,2],2] + Vertices_ac[self.Avatar.Faces[i,4],2])/3]]
            CentroidZa.sort(key=itemgetter(1))
            for i in range(len(self.Avatar.Faces)):
                Face_idxa += [CentroidZa[i][0]]

        elif(meshnum==1):
            Vertices_sc = np.empty((0, 3), dtype=np.float32)
            self.Vertices_spix = np.empty((0, 2), dtype=np.float32)
            for i in range(len(self.Source.Vertices)):
                pt[0:3] = self.Source.Vertices[i,0:3]
                tmp = np.dot(W2Cam, pt)
                Vertices_sc = np.vstack((Vertices_sc, tmp[0:3]))
                pix = np.dot(self.intrinsic, [-tmp[0]/tmp[2], tmp[1]/tmp[2], 1])
                self.Vertices_spix = np.vstack((self.Vertices_spix, [int(round(pix[0])), int(round(pix[1]))]))

            #sort
            CentroidZs = []
            Face_idxs = []
            for i in range(len(self.Source.Faces)):
                CentroidZs += [[i, (Vertices_sc[self.Source.Faces[i,0],2] + Vertices_sc[self.Source.Faces[i,2],2] + Vertices_sc[self.Source.Faces[i,4],2])/3]]
            CentroidZs.sort(key=itemgetter(1))
            for i in range(len(self.Source.Faces)):
                Face_idxs += [CentroidZs[i][0]]


        #draw polygons from back to front
        pix = np.array([0., 0., 1.])
        if(meshnum==0):
            for num in range(len(Face_idxa)):
                #start = time.time()
                i = Face_idxa[num]
                poly = []
                gaze = (Cam_Pos[0:3] - self.Avatar.Vertices[self.Avatar.Faces[i,0],0:3]) / LA.norm(Cam_Pos[0:3] - self.Avatar.Vertices[self.Avatar.Faces[i,0],0:3])
                inner = np.dot(self.Avatar.Normals[i], gaze)

                if(inner >= 0 and CentroidZa[num][1] < -20):
                    for k in range(3):
                        poly += [self.Vertices_apix[self.Avatar.Faces[i,2*k],0], self.Vertices_apix[self.Avatar.Faces[i,2*k],1]]

                    darkness = str(hex(int(128*(inner+0.7))))[2::]
                    darkness = "#" + darkness + darkness + darkness

                    if(self.wired.get()): 
                        self.canvas.create_line(poly[0], poly[1], poly[2], poly[3], fill="white")
                        self.canvas.create_line(poly[2], poly[3], poly[4], poly[5], fill="white")
                        self.canvas.create_line(poly[4], poly[5], poly[0], poly[1], fill="white")
                    else: self.canvas.create_polygon(poly, fill=darkness, tag="apoly"+str(i))
                # print("{:.7f}".format(time.time() - start))

            #draw candidates and landmarks
            for i in range(len(self.candidate)):
                idx = self.candidate[i]
                if(i == self.candidate_num):
                    self.canvas.create_oval(self.Vertices_apix[idx,0]-3, self.Vertices_apix[idx,1]-3, self.Vertices_apix[idx,0]+3, self.Vertices_apix[idx,1]+3, fill="red", tag="candidate"+str(i))
                else:self.canvas.create_oval(self.Vertices_apix[idx,0]-3, self.Vertices_apix[idx,1]-3, self.Vertices_apix[idx,0]+3, self.Vertices_apix[idx,1]+3, fill="gray", tag="candidate"+str(i))

            for i in range(len(self.AVATAR_LANDMARKS)):
                idx = self.AVATAR_LANDMARKS[i]
                if(idx != None): self.canvas.create_oval(self.Vertices_apix[idx,0]-3, self.Vertices_apix[idx,1]-3, self.Vertices_apix[idx,0]+3, self.Vertices_apix[idx,1]+3, fill="green", tag="landmark"+str(i))
            self.canvas.itemconfig("landmark"+str(self.landmark_num), width=2)

        elif(meshnum==1):
            for i in Face_idxs:
                poly = []
                gaze = (Cam_Pos[0:3] - self.Source.Vertices[self.Source.Faces[i,0],0:3]) / LA.norm(Cam_Pos[0:3] - self.Source.Vertices[self.Source.Faces[i,0],0:3])
                inner = np.dot(self.Source.Normals[i], gaze)
                if(inner>=0):
                    for k in range(3):
                        poly += [self.Vertices_spix[self.Source.Faces[i,2*k],0], self.Vertices_spix[self.Source.Faces[i,2*k],1]]

                    darkness = str(hex(int(128*(inner+0.7))))[2::]
                    darkness = "#" + darkness + darkness + darkness

                    self.canvas2.create_polygon(poly, fill=darkness, tag="spoly"+str(i))

            for idx in self.cvp_s:
                self.canvas2.create_oval(self.Vertices_spix[idx,0]-3, self.Vertices_spix[idx,1]-3, self.Vertices_spix[idx,0]+3,    self.Vertices_spix[idx,1]+3, fill="green", tag="candidate")



    def Update_Mesh(self, meshnum):
        if(meshnum==0): self.canvas.delete("all")
        elif(meshnum==1): self.canvas2.delete("all")

        self.DrawMesh(meshnum)

    def left_press(self, event, meshnum):

        self.candidate_num = 0
        
        #find candidate landmarks
        if(meshnum==0):
            a = np.square(self.Vertices_apix - [event.x, event.y])
            norm = np.sqrt(a[:,0] + a[:,1])
            self.candidate = [np.argmin(norm)]
            self.cvp_a = [np.argmin(norm)]
            self.cvp_s = self.AtoS[np.argmin(norm)]
            

            for idx in self.cvp_s:
                self.canvas2.create_oval(self.Vertices_spix[idx,0]-3, self.Vertices_spix[idx,1]-3, self.Vertices_spix[idx,0]+3,    self.Vertices_spix[idx,1]+3, fill="green", tag="candidate")

            #for idx in self.cvp_s:
                #self.canvas.create_oval(self.Vertices_spix[idx,0]-3, self.Vertices_spix[idx,1]-3, self.Vertices_spix[idx,0]+3,    #self.Vertices_spix[idx,1]+3, fill="green", tag="candidate")
            
            for idx in self.AvatarAdjacent_v[np.argmin(norm)]:
                self.canvas.create_oval(self.Vertices_apix[idx,0]-3, self.Vertices_apix[idx,1]-3, self.Vertices_apix[idx,0]+3,    self.Vertices_apix[idx,1]+3, fill="green", tag="candidate")
                self.canvas.create_line(self.Vertices_apix[idx,0], self.Vertices_apix[idx,1], self.Vertices_apix[np.argmin(norm),0], self.Vertices_apix[np.argmin(norm),1], fill="green", tag="candidate")


        elif(meshnum==1):
            a = np.square(self.Vertices_spix - [event.x, event.y])
            norm = np.sqrt(a[:,0] + a[:,1])
            self.candidate = self.StoA[np.argmin(norm)]
            self.cvp_s = [np.argmin(norm)]
            self.cvp_a = self.StoA[np.argmin(norm)]
            

            for idx in self.cvp_s:
                self.canvas2.create_oval(self.Vertices_spix[idx,0]-3, self.Vertices_spix[idx,1]-3, self.Vertices_spix[idx,0]+3,    self.Vertices_spix[idx,1]+3, fill="green", tag="candidate")

            for idx in self.cvp_a:
                self.canvas2.create_oval(self.Vertices_apix[idx,0]-3, self.Vertices_apix[idx,1]-3, self.Vertices_apix[idx,0]+3,    self.Vertices_apix[idx,1]+3, fill="red", tag="candidate")

        print("idx_a: {}".format(self.cvp_a) )
        print("idx_s: {}".format(self.cvp_s))

        self.Label2.config(text=u"Candidate : %s" %str(self.candidate))
        if(len(self.candidate)>0):
            self.Label3.config(text=u"Selected : %s" %str(self.candidate[self.candidate_num]))
        else: self.Label3.config(text=u"Selected : None")

        for i in range(len(self.candidate)):
            idx = self.candidate[i]
            if(i == self.candidate_num):
                self.canvas.create_oval(self.Vertices_apix[idx,0]-3, self.Vertices_apix[idx,1]-3, self.Vertices_apix[idx,0]+3, self.Vertices_apix[idx,1]+3, fill="red", tag="candidate"+str(i))
            else:self.canvas.create_oval(self.Vertices_apix[idx,0]-3, self.Vertices_apix[idx,1]-3, self.Vertices_apix[idx,0]+3, self.Vertices_apix[idx,1]+3, fill="gray", tag="candidate"+str(i))


    def right_press(self, event, meshnum):
        if(meshnum==0): self.right_x = event.x; self.right_y = event.y
        elif(meshnum==1): self.right_x_s = event.x; self.right_y_s = event.y

    def center_press(self, event, meshnum):
        if(meshnum==0): self.center_x = event.x; self.center_y = event.y
        elif(meshnum==1): self.center_x_s = event.x; self.center_y_s = event.y

    def right_motion(self, event, meshnum):
        if(meshnum==0):
            delta_x = event.x - self.right_x
            delta_y = event.y - self.right_y

            self.angley -= 0.01*delta_x
            self.anglex -= 0.01*delta_y

            self.Update_Mesh(meshnum)

            self.right_x = event.x
            self.right_y = event.y

        elif(meshnum==1):
            delta_x = event.x - self.right_x_s
            delta_y = event.y - self.right_y_s

            self.angley_s -= 0.01*delta_x
            self.anglex_s -= 0.01*delta_y

            self.Update_Mesh(meshnum)

            self.right_x_s = event.x
            self.right_y_s = event.y

    def center_motion(self, event, meshnum):
        if(meshnum==0):
            delta_x = event.x - self.center_x
            delta_y = event.y - self.center_y

            self.translation_x -= 0.5*delta_x
            self.translation_y -= -0.5*delta_y

            self.Update_Mesh(meshnum)

            self.center_x = event.x
            self.center_y = event.y

        elif(meshnum==1):
            delta_x = event.x - self.center_x_s
            delta_y = event.y - self.center_y_s

            self.translation_x_s -= 0.5*delta_x
            self.translation_y_s -= -0.5*delta_y

            self.Update_Mesh(meshnum)

            self.center_x_s = event.x
            self.center_y_s = event.y

    def wheel_scroll(self, event, meshnum):
        if(meshnum==0):
            if(self.scale + event.delta*0.05/abs(event.delta)>0.1):
                self.scale += event.delta*0.05/abs(event.delta)
        elif(meshnum==1):
            if(self.scale_s + event.delta*0.05/abs(event.delta)>0.1):
                self.scale_s += event.delta*0.05/abs(event.delta)
        self.Update_Mesh(meshnum)

    def press_check(self, event):
        if(self.wired.get()==True): self.wired.set(False)
        else: self.wired.set(True)
        self.Update_Mesh(0)

    def change_landmark(self, i):
        self.canvas2.itemconfig("landmark"+str(self.landmark_num), fill="gray")
        self.canvas.itemconfig("landmark"+str(self.landmark_num), width=1)
        if(i==0 and self.landmark_num > 0): self.landmark_num -= 1
        elif(i==0 and self.landmark_num <= 0): self.landmark_num = 50
        elif(i==1 and self.landmark_num < 50): self.landmark_num += 1
        elif(i==1 and self.landmark_num >= 50): self.landmark_num = 0
        self.Label.config(text=u"Landmark %s    " %str(self.landmark_num+1))
        self.canvas2.itemconfig("landmark"+str(self.landmark_num), fill="red")
        self.canvas.itemconfig("landmark"+str(self.landmark_num), width=2)

    def change_candidate(self, i):
        self.canvas.itemconfig("candidate"+str(self.candidate_num), fill="gray")
        if(i==0 and self.candidate_num > 0): self.candidate_num -= 1
        elif(i==0 and self.candidate_num <= 0): self.candidate_num = len(self.candidate)-1
        elif(i==1 and self.candidate_num < len(self.candidate)-1): self.candidate_num += 1
        elif(i==1 and self.candidate_num >= len(self.candidate)-1): self.candidate_num = 0
        self.canvas.itemconfig("candidate"+str(self.candidate_num), fill="red")
        if(len(self.candidate)>0):
            self.Label3.config(text=u"Selected : %s" %str(self.candidate[self.candidate_num]))
        else: self.Label3.config(text=u"Selected : None")

    def register_landmark(self):
        self.canvas.delete("landmark"+str(self.landmark_num))
        idx = self.candidate[self.candidate_num]
        self.AVATAR_LANDMARKS[self.landmark_num] = idx
        self.canvas.create_oval(self.Vertices_apix[idx,0]-3, self.Vertices_apix[idx,1]-3, self.Vertices_apix[idx,0]+3, self.Vertices_apix[idx,1]+3, fill="green", width=2, tag="landmark"+str(self.landmark_num))
        self.change_landmark(1)
        print(self.AVATAR_LANDMARKS)

    def destroy_window(self):
        self.root.destroy()

    def __init__(self, master, Avatar, Source, StoA, AtoS, AvatarAdjacent_v):
        self.root = master
        self.Avatar = Avatar
        self.Source = Source
        self.StoA = StoA
        self.AtoS = AtoS
        self.AvatarAdjacent_v = AvatarAdjacent_v

        self.Size = (300,500) #Size of the canvas
        self.intrinsic = np.array([[500, 0, self.Size[1]/2], 
                                   [0, 500, self.Size[0]/2], 
                                   [0, 0, 1]])
        self.scale = 1; self.scale_s = 1
        self.anglex = 0; self.anglex_s = 0
        self.angley = 0; self.angley_s = 0
        self.translation_x = 0; self.translation_x_s = 0
        self.translation_y = 0; self.translation_y_s = 0
        self.landmark_num = 0
        self.candidate_num = 0
        self.candidate = []
        self.cvp_a = []
        self.cvp_s = []
        self.wired = tk.BooleanVar()
        self.wired.set(False)

        self.AVATAR_LANDMARKS = [None, None, None, None, None, None, None, None, None, None, 
                                 None, None, None, None, None, None, None, None, None, None, 
                                 None, None, None, None, None, None, None, None, None, None, 
                                 None, None, None, None, None, None, None, None, None, None, 
                                 None, None, None, None, None, None, None, None, None, None, None]


        tk.Frame.__init__(self, master)
        self.pack()

        self.Window = tk.Frame(self.root)
        self.Window.pack(side="left", anchor="s")
        self.Window2 = tk.Frame(self.root)
        self.Window2.pack(side="left", anchor="s")

        self.Console = tk.Frame(self.Window)
        self.Console.pack(pady=15)
        self.Console2 = tk.Frame(self.Window2)
        self.Console2.pack()
        self.Console2_2 = tk.Frame(self.Console2)

        self.Label = tk.Label(self.Console, text=u"Landmark %s    " %str(self.landmark_num+1))
        self.Down = tk.Button(self.Console, text="<", command=lambda i=0: self.change_landmark(i))
        self.Up = tk.Button(self.Console, text=">", command=lambda i=1: self.change_landmark(i))
        
        self.Label.pack(side="left")
        self.Down.pack(side="left")
        self.Up.pack(side="left")
        

        self.Label2 = tk.Label(self.Console2, text=u"Candidate(s) : %s" %str(self.candidate))
        self.Label3 = tk.Label(self.Console2, text=u"Selected : None")
        self.Down2 = tk.Button(self.Console2_2, text="<", command=lambda i=0: self.change_candidate(i))
        self.Up2 = tk.Button(self.Console2_2, text=">", command=lambda i=1: self.change_candidate(i))
        self.Register = tk.Button(self.Console2_2, text="Register", command=self.register_landmark)
        self.OK = tk.Button(self.Console2_2, text="Finish", command=self.destroy_window)
        self.CheckBox = tk.Checkbutton(self.Console2_2, text = 'wireframe')
        
        self.Label2.pack()
        self.Label3.pack()
        self.Console2_2.pack()
        self.CheckBox.pack(side="left", padx=20)
        self.CheckBox.bind("<Button-1>", self.press_check)
        self.Down2.pack(side="left")
        self.Up2.pack(side="left")
        self.Register.pack(side="left")
        self.OK.pack(side="left", padx="20")
        

        self.canvas = tk.Canvas(self.Window2, bg="black", height=self.Size[0], width=self.Size[1])
        self.canvas2 = tk.Canvas(self.Window, bg="black", height=self.Size[0], width=self.Size[1])
        self.DrawMesh(0)
        self.DrawMesh(1)
        self.canvas.pack()
        self.canvas2.pack()

        
        self.canvas.bind("<Button-1>", lambda event, meshnum=0: self.left_press(event, meshnum))
        self.canvas.bind("<Button-2>", lambda event, meshnum=0: self.center_press(event, meshnum))
        self.canvas.bind("<Button-3>", lambda event, meshnum=0: self.right_press(event, meshnum))
        self.canvas.bind("<B3-Motion>", lambda event, meshnum=0: self.right_motion(event, meshnum))
        self.canvas.bind("<B2-Motion>", lambda event, meshnum=0: self.center_motion(event, meshnum))
        self.canvas.bind("<MouseWheel>", lambda event, meshnum=0: self.wheel_scroll(event, meshnum))
        self.canvas.bind("<Enter>", lambda event: self.canvas.focus_set())
        self.canvas.bind("<Leave>", lambda event: self.root.focus_set())

        self.canvas2.bind("<Button-1>", lambda event, meshnum=1: self.left_press(event, meshnum))
        self.canvas2.bind("<Button-2>", lambda event, meshnum=1: self.center_press(event, meshnum))
        self.canvas2.bind("<Button-3>", lambda event, meshnum=1: self.right_press(event, meshnum))
        self.canvas2.bind("<B3-Motion>", lambda event, meshnum=1: self.right_motion(event, meshnum))
        self.canvas2.bind("<B2-Motion>", lambda event, meshnum=1: self.center_motion(event, meshnum))
        self.canvas2.bind("<MouseWheel>", lambda event, meshnum=1: self.wheel_scroll(event, meshnum))
        self.canvas2.bind("<Enter>", lambda event: self.canvas2.focus_set())
        self.canvas2.bind("<Leave>", lambda event: self.root.focus_set())


class Show_Vertices(tk.Frame):
    def DrawMesh(self, meshnum):
        if(meshnum==0): 
            anglex = self.anglex
            angley = self.angley
            translation_x = self.translation_x
            translation_y = self.translation_y
            scale = self.scale
        elif(meshnum==1): 
            anglex = self.anglex_s
            angley = self.angley_s
            translation_x = self.translation_x_s
            translation_y = self.translation_y_s
            scale = self.scale_s

        RotX = np.array([[1., 0., 0., 0.], 
                         [0., np.cos(anglex), np.sin(anglex), 0.], 
                         [0., -np.sin(anglex), np.cos(anglex), 0.], 
                         [0., 0., 0., 1.]], dtype=np.float32)

        RotY = np.array([[np.cos(angley), 0., -np.sin(angley), 0.], 
                         [0., 1., 0., 0.], 
                         [np.sin(angley), 0., np.cos(angley), 0.], 
                         [0., 0., 0., 1.]], dtype=np.float32)

        Cam_Rot = np.dot(RotX, np.dot(RotY, np.identity(4, dtype=np.float32))) #world camera rotation
        Cam_Pos = np.dot(Cam_Rot.T, [translation_x, translation_y, 400*scale, 1]) #world camera position

        t = -np.dot(Cam_Rot, Cam_Pos)[0:3].reshape(3, 1)
        W2Cam = np.vstack((np.hstack((Cam_Rot[0:3,0:3], t)), [0, 0, 0, 1])) #view convert matrix

        #World to Camera and Camera to Pixel
        pt = np.array([0., 0., 0., 1.])
        if(meshnum==0):
            Vertices_ac = np.empty((0, 3), dtype=np.float32)
            self.Vertices_apix = np.empty((0, 2), dtype=np.float32)
            for i in range(len(self.Avatar.Vertices)):
                pt[0:3] = self.Avatar.Vertices[i,0:3]
                tmp = np.dot(W2Cam, pt)
                Vertices_ac = np.vstack((Vertices_ac, tmp[0:3]))
                pix = np.dot(self.intrinsic, [-tmp[0]/tmp[2], tmp[1]/tmp[2], 1])
                self.Vertices_apix = np.vstack((self.Vertices_apix, [int(round(pix[0])), int(round(pix[1]))]))

            #sort
            CentroidZa = []
            Face_idxa = []
            for i in range(len(self.Avatar.Faces)):
                CentroidZa += [[i, (Vertices_ac[self.Avatar.Faces[i,0],2] + Vertices_ac[self.Avatar.Faces[i,2],2] + Vertices_ac[self.Avatar.Faces[i,4],2])/3]]
            CentroidZa.sort(key=itemgetter(1))
            for i in range(len(self.Avatar.Faces)):
                Face_idxa += [CentroidZa[i][0]]

        elif(meshnum==1):
            Vertices_sc = np.empty((0, 3), dtype=np.float32)
            self.Vertices_spix = np.empty((0, 2), dtype=np.float32)
            for i in range(len(self.Source.Vertices)):
                pt[0:3] = self.Source.Vertices[i,0:3]
                tmp = np.dot(W2Cam, pt)
                Vertices_sc = np.vstack((Vertices_sc, tmp[0:3]))
                pix = np.dot(self.intrinsic, [-tmp[0]/tmp[2], tmp[1]/tmp[2], 1])
                self.Vertices_spix = np.vstack((self.Vertices_spix, [int(round(pix[0])), int(round(pix[1]))]))

            #sort
            CentroidZs = []
            Face_idxs = []
            for i in range(len(self.Source.Faces)):
                CentroidZs += [[i, (Vertices_sc[self.Source.Faces[i,0],2] + Vertices_sc[self.Source.Faces[i,2],2] + Vertices_sc[self.Source.Faces[i,4],2])/3]]
            CentroidZs.sort(key=itemgetter(1))
            for i in range(len(self.Source.Faces)):
                Face_idxs += [CentroidZs[i][0]]


        #draw polygons from back to front
        pix = np.array([0., 0., 1.])
        if(meshnum==0):
            for num in range(len(Face_idxa)):
                #start = time.time()
                i = Face_idxa[num]
                poly = []
                gaze = (Cam_Pos[0:3] - self.Avatar.Vertices[self.Avatar.Faces[i,0],0:3]) / LA.norm(Cam_Pos[0:3] - self.Avatar.Vertices[self.Avatar.Faces[i,0],0:3])
                inner = np.dot(self.Avatar.Normals[i], gaze)

                if(inner >= 0 and CentroidZa[num][1] < -20):
                    for k in range(3):
                        poly += [self.Vertices_apix[self.Avatar.Faces[i,2*k],0], self.Vertices_apix[self.Avatar.Faces[i,2*k],1]]

                    darkness = str(hex(int(128*(inner+0.7))))[2::]
                    darkness = "#" + darkness + darkness + darkness

                    if(self.wired.get()): 
                        self.canvas.create_line(poly[0], poly[1], poly[2], poly[3], fill="white")
                        self.canvas.create_line(poly[2], poly[3], poly[4], poly[5], fill="white")
                        self.canvas.create_line(poly[4], poly[5], poly[0], poly[1], fill="white")
                    else: self.canvas.create_polygon(poly, fill=darkness, tag="apoly"+str(i))
                #print "%.7f" %(time.time() - start)

            #draw Vertices
            for idx in xrange(len(self.Avatar.Vertices)):
                gaze = (Cam_Pos[0:3] - self.Avatar.Vertices[idx,0:3]) / LA.norm(Cam_Pos[0:3] - self.Avatar.Vertices[idx,0:3])
                inner = np.dot(self.Avatar.V_Normals[idx], gaze)

                if(inner >= 0 and idx in self.List_idxa and CentroidZa[num][1] < -20):
                    self.canvas.create_oval(self.Vertices_apix[idx,0]-3, self.Vertices_apix[idx,1]-3, self.Vertices_apix[idx,0]+3, self.Vertices_apix[idx,1]+3, fill="green", tag="landmark")
                elif(inner >= 0 and idx not in self.List_idxa and CentroidZa[num][1] < -20):
                    self.canvas.create_oval(self.Vertices_apix[idx,0]-3, self.Vertices_apix[idx,1]-3, self.Vertices_apix[idx,0]+3, self.Vertices_apix[idx,1]+3, fill="gray", tag="landmark")
            self.canvas.itemconfig("landmark"+str(self.landmark_num), width=2)
            
            for i in self.candidate:
                self.canvas.create_oval(self.Vertices_apix[idx,0]-3, self.Vertices_apix[idx,1]-3, self.Vertices_apix[idx,0]+3, self.Vertices_apix[idx,1]+3, fill="red", tag="candidate")


        elif(meshnum==1):
            for i in Face_idxs:
                poly = []
                gaze = (Cam_Pos[0:3] - self.Source.Vertices[self.Source.Faces[i,0],0:3]) / LA.norm(Cam_Pos[0:3] - self.Source.Vertices[self.Source.Faces[i,0],0:3])
                inner = np.dot(self.Source.Normals[i], gaze)
                if(inner>=0):
                    for k in range(3):
                        poly += [self.Vertices_spix[self.Source.Faces[i,2*k],0], self.Vertices_spix[self.Source.Faces[i,2*k],1]]

                    darkness = str(hex(int(128*(inner+0.7))))[2::]
                    darkness = "#" + darkness + darkness + darkness

                    self.canvas2.create_polygon(poly, fill=darkness, tag="spoly"+str(i))

            #draw Vertices
            for i in range(len(self.List_idxs)):
                idx = self.List_idxs[i]
                if(idx != None): self.canvas2.create_oval(self.Vertices_spix[idx,0]-3, self.Vertices_spix[idx,1]-3, self.Vertices_spix[idx,0]+3, self.Vertices_spix[idx,1]+3, fill="green", tag="landmark"+str(i))
            self.canvas2.itemconfig("landmark"+str(self.landmark_num), width=2)

            for idx in self.candidate:
                self.canvas2.create_oval(self.Vertices_spix[idx,0]-3, self.Vertices_spix[idx,1]-3, self.Vertices_spix[idx,0]+3,    self.Vertices_spix[idx,1]+3, fill="red", tag="candidate")



    def Update_Mesh(self, meshnum):
        if(meshnum==0): self.canvas.delete("all")
        elif(meshnum==1): self.canvas2.delete("all")

        self.DrawMesh(meshnum)

    def left_press(self, event, meshnum):

        self.candidate_num = 0
        
        #find candidate landmarks
        if(meshnum==0):
            a = np.square(self.Vertices_apix - [event.x, event.y])
            norm = np.sqrt(a[:,0] + a[:,1])
            self.candidate = [np.argmin(norm)]
            
            for idx in self.candidate:
                self.canvas.create_oval(self.Vertices_apix[idx,0]-3, self.Vertices_apix[idx,1]-3, self.Vertices_apix[idx,0]+3,    self.Vertices_apix[idx,1]+3, fill="red", tag="candidate")

            

            print("idx_a:", self.candidate)


        elif(meshnum==1):
            a = np.square(self.Vertices_spix - [event.x, event.y])
            norm = np.sqrt(a[:,0] + a[:,1])
            self.candidate = [np.argmin(norm)]

            for idx in self.candidate:
                self.canvas2.create_oval(self.Vertices_spix[idx,0]-3, self.Vertices_spix[idx,1]-3, self.Vertices_spix[idx,0]+3,    self.Vertices_spix[idx,1]+3, fill="red", tag="candidate")

            print("idx_s:", self.candidate)


    def right_press(self, event, meshnum):
        if(meshnum==0): self.right_x = event.x; self.right_y = event.y
        elif(meshnum==1): self.right_x_s = event.x; self.right_y_s = event.y

    def center_press(self, event, meshnum):
        if(meshnum==0): self.center_x = event.x; self.center_y = event.y
        elif(meshnum==1): self.center_x_s = event.x; self.center_y_s = event.y

    def right_motion(self, event, meshnum):
        if(meshnum==0):
            delta_x = event.x - self.right_x
            delta_y = event.y - self.right_y

            self.angley -= 0.01*delta_x
            self.anglex -= 0.01*delta_y

            self.Update_Mesh(meshnum)

            self.right_x = event.x
            self.right_y = event.y

        elif(meshnum==1):
            delta_x = event.x - self.right_x_s
            delta_y = event.y - self.right_y_s

            self.angley_s -= 0.01*delta_x
            self.anglex_s -= 0.01*delta_y

            self.Update_Mesh(meshnum)

            self.right_x_s = event.x
            self.right_y_s = event.y

    def center_motion(self, event, meshnum):
        if(meshnum==0):
            delta_x = event.x - self.center_x
            delta_y = event.y - self.center_y

            self.translation_x -= 0.5*delta_x
            self.translation_y -= -0.5*delta_y

            self.Update_Mesh(meshnum)

            self.center_x = event.x
            self.center_y = event.y

        elif(meshnum==1):
            delta_x = event.x - self.center_x_s
            delta_y = event.y - self.center_y_s

            self.translation_x_s -= 0.5*delta_x
            self.translation_y_s -= -0.5*delta_y

            self.Update_Mesh(meshnum)

            self.center_x_s = event.x
            self.center_y_s = event.y

    def wheel_scroll(self, event, meshnum):
        if(meshnum==0):
            if(self.scale + event.delta*0.05/abs(event.delta)>0.1):
                self.scale += event.delta*0.05/abs(event.delta)
        elif(meshnum==1):
            if(self.scale_s + event.delta*0.05/abs(event.delta)>0.1):
                self.scale_s += event.delta*0.05/abs(event.delta)
        self.Update_Mesh(meshnum)

    def press_check(self, event):
        if(self.wired.get()==True): self.wired.set(False)
        else: self.wired.set(True)
        self.Update_Mesh(0)

    def change_landmark(self, i):
        self.canvas2.itemconfig("landmark"+str(self.landmark_num), fill="gray")
        self.canvas.itemconfig("landmark"+str(self.landmark_num), width=1)
        if(i==0 and self.landmark_num > 0): self.landmark_num -= 1
        elif(i==0 and self.landmark_num <= 0): self.landmark_num = 50
        elif(i==1 and self.landmark_num < 50): self.landmark_num += 1
        elif(i==1 and self.landmark_num >= 50): self.landmark_num = 0
        self.Label.config(text=u"Landmark %s    " %str(self.landmark_num+1))
        self.canvas2.itemconfig("landmark"+str(self.landmark_num), fill="red")
        self.canvas.itemconfig("landmark"+str(self.landmark_num), width=2)

    def change_candidate(self, i):
        self.canvas.itemconfig("candidate"+str(self.candidate_num), fill="gray")
        if(i==0 and self.candidate_num > 0): self.candidate_num -= 1
        elif(i==0 and self.candidate_num <= 0): self.candidate_num = len(self.candidate)-1
        elif(i==1 and self.candidate_num < len(self.candidate)-1): self.candidate_num += 1
        elif(i==1 and self.candidate_num >= len(self.candidate)-1): self.candidate_num = 0
        self.canvas.itemconfig("candidate"+str(self.candidate_num), fill="red")
        if(len(self.candidate)>0):
            self.Label3.config(text=u"Selected : %s" %str(self.candidate[self.candidate_num]))
        else: self.Label3.config(text=u"Selected : None")

    def register_landmark(self):
        self.canvas.delete("landmark"+str(self.landmark_num))
        idx = self.candidate[self.candidate_num]
        self.AVATAR_LANDMARKS[self.landmark_num] = idx
        self.canvas.create_oval(self.Vertices_apix[idx,0]-3, self.Vertices_apix[idx,1]-3, self.Vertices_apix[idx,0]+3, self.Vertices_apix[idx,1]+3, fill="green", width=2, tag="landmark"+str(self.landmark_num))
        self.change_landmark(1)
        print(self.AVATAR_LANDMARKS)

    def destroy_window(self):
        self.root.destroy()


    def __init__(self, master, Avatar, Source, List_idxa, List_idxs):
        self.root = master
        self.Avatar = Avatar
        self.Source = Source
        self.List_idxa = List_idxa
        self.List_idxs = List_idxs

        self.Size = (300,500) #Size of the canvas
        self.intrinsic = np.array([[500, 0, self.Size[1]/2], 
                                   [0, 500, self.Size[0]/2], 
                                   [0, 0, 1]])
        self.scale = 1; self.scale_s = 1
        self.anglex = 0; self.anglex_s = 0
        self.angley = 0; self.angley_s = 0
        self.translation_x = 0; self.translation_x_s = 0
        self.translation_y = 0; self.translation_y_s = 0
        self.landmark_num = 0
        self.candidate_num = 0
        self.candidate = []
        self.wired = tk.BooleanVar()
        self.wired.set(False)

        self.AVATAR_LANDMARKS = [None, None, None, None, None, None, None, None, None, None, 
                                 None, None, None, None, None, None, None, None, None, None, 
                                 None, None, None, None, None, None, None, None, None, None, 
                                 None, None, None, None, None, None, None, None, None, None, 
                                 None, None, None, None, None, None, None, None, None, None, None]


        tk.Frame.__init__(self, master)
        self.pack()

        self.Window = tk.Frame(self.root)
        self.Window.pack(side="left", anchor="s")
        self.Window2 = tk.Frame(self.root)
        self.Window2.pack(side="left", anchor="s")

        self.Console = tk.Frame(self.Window)
        self.Console.pack(pady=15)
        self.Console2 = tk.Frame(self.Window2)
        self.Console2.pack()
        self.Console2_2 = tk.Frame(self.Console2)

        self.Label = tk.Label(self.Console, text=u"Landmark %s    " %str(self.landmark_num+1))
        self.Down = tk.Button(self.Console, text="<", command=lambda i=0: self.change_landmark(i))
        self.Up = tk.Button(self.Console, text=">", command=lambda i=1: self.change_landmark(i))
        
        self.Label.pack(side="left")
        self.Down.pack(side="left")
        self.Up.pack(side="left")
        

        self.Label2 = tk.Label(self.Console2, text=u"Candidate(s) : %s" %str(self.candidate))
        self.Label3 = tk.Label(self.Console2, text=u"Selected : None")
        self.Down2 = tk.Button(self.Console2_2, text="<", command=lambda i=0: self.change_candidate(i))
        self.Up2 = tk.Button(self.Console2_2, text=">", command=lambda i=1: self.change_candidate(i))
        self.Register = tk.Button(self.Console2_2, text="Register", command=self.register_landmark)
        self.OK = tk.Button(self.Console2_2, text="Finish", command=self.destroy_window)
        self.CheckBox = tk.Checkbutton(self.Console2_2, text = 'wireframe')
        
        self.Label2.pack()
        self.Label3.pack()
        self.Console2_2.pack()
        self.CheckBox.pack(side="left", padx=20)
        self.CheckBox.bind("<Button-1>", self.press_check)
        self.Down2.pack(side="left")
        self.Up2.pack(side="left")
        self.Register.pack(side="left")
        self.OK.pack(side="left", padx="20")
        

        self.canvas = tk.Canvas(self.Window2, bg="black", height=self.Size[0], width=self.Size[1])
        self.canvas2 = tk.Canvas(self.Window, bg="black", height=self.Size[0], width=self.Size[1])
        self.DrawMesh(0)
        self.DrawMesh(1)
        self.canvas.pack()
        self.canvas2.pack()

        
        self.canvas.bind("<Button-1>", lambda event, meshnum=0: self.left_press(event, meshnum))
        self.canvas.bind("<Button-2>", lambda event, meshnum=0: self.center_press(event, meshnum))
        self.canvas.bind("<Button-3>", lambda event, meshnum=0: self.right_press(event, meshnum))
        self.canvas.bind("<B3-Motion>", lambda event, meshnum=0: self.right_motion(event, meshnum))
        self.canvas.bind("<B2-Motion>", lambda event, meshnum=0: self.center_motion(event, meshnum))
        self.canvas.bind("<MouseWheel>", lambda event, meshnum=0: self.wheel_scroll(event, meshnum))
        self.canvas.bind("<Enter>", lambda event: self.canvas.focus_set())
        self.canvas.bind("<Leave>", lambda event: self.root.focus_set())

        self.canvas2.bind("<Button-1>", lambda event, meshnum=1: self.left_press(event, meshnum))
        self.canvas2.bind("<Button-2>", lambda event, meshnum=1: self.center_press(event, meshnum))
        self.canvas2.bind("<Button-3>", lambda event, meshnum=1: self.right_press(event, meshnum))
        self.canvas2.bind("<B3-Motion>", lambda event, meshnum=1: self.right_motion(event, meshnum))
        self.canvas2.bind("<B2-Motion>", lambda event, meshnum=1: self.center_motion(event, meshnum))
        self.canvas2.bind("<MouseWheel>", lambda event, meshnum=1: self.wheel_scroll(event, meshnum))
        self.canvas2.bind("<Enter>", lambda event: self.canvas2.focus_set())
        self.canvas2.bind("<Leave>", lambda event: self.root.focus_set())

```
