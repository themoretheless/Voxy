# Facial rendering source scan: benject/mirror

Repository: https://github.com/benject/mirror

Pinned source revision: 434c2b16e4cfa6274060c6bd62bcaee4396f1d07

Discovery topic: facial animation

Description: Mirror : a maya facial capture animation toolkit based on mediapipe

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"face": ["scene/facemesh.mb", "src/face_capture.py", "ui/FaceCapture_ui.ui"], "lip": ["sample/video_clip.json", "sample/video_clip.mp4"]}

## Source file: src/face_capture.py

{
  "path": "src/face_capture.py",
  "blob_sha": "c3e6b2ae46871746e4b1b13f9d7fbad6309e5113",
  "sha256": "a40abb4f7a8594488ac15c62666c28470d53cead0a668874327e9616206cf3e8",
  "lines": 200,
  "bytes": 5814,
  "selection_score": 15,
  "keyword_line_counts": {
    "gum": 3,
    "iris": 2,
    "face": 33,
    "lip": 1
  },
  "url": "https://github.com/benject/mirror/blob/434c2b16e4cfa6274060c6bd62bcaee4396f1d07/src/face_capture.py",
  "function_signature_candidates": [
    {
      "line": 18,
      "text": "def __init__(self,file):"
    },
    {
      "line": 23,
      "text": "def image_capture(self):"
    },
    {
      "line": 75,
      "text": "def video_capture(self):"
    }
  ]
}

```text

#!/usr/bin/python
# -*- coding: utf-8 -*-

# Author: Bing Chen

import os
import json
import mediapipe as mp
import cv2

mpDrawUtils = mp.solutions.mediapipe.python.solutions.drawing_utils
mpFacemesh = mp.solutions.mediapipe.python.solutions.face_mesh

mpDrawSpec = mpDrawUtils.DrawingSpec(color=(0,255,0),thickness=1,circle_radius=1)

class face_capture():

  def __init__(self,file):

    self.file = file

      
  def image_capture(self):
    """
    using mediapipe to get 468 face feature points and dump them to a json file
    利用mediapipe库,获取468个特征点并写入json 文件
    """


    basename,ext = os.path.splitext(self.file) #get the extension of image file  
    json_file = self.file.replace(ext,".json") #replace extension with .json

    IMAGE = cv2.imread(self.file)
    
    with mpFacemesh.FaceMesh(static_image_mode=True, max_num_faces=1,
        refine_landmarks=True,
        min_detection_confidence=0.5,
        min_tracking_confidence=0.5) as face_mesh:

      image = cv2.cvtColor(IMAGE,cv2.COLOR_BGR2RGB)
      results = face_mesh.process(image)
      image = cv2.cvtColor(image,cv2.COLOR_RGB2BGR)

      height  = image.shape[0]
      width = image.shape[1]

      sx = width / height #根骨骼x轴缩放
      frame = 1 #当前帧

      if results.multi_face_landmarks:

        for facelms in results.multi_face_landmarks:

          mpDrawUtils.draw_landmarks(image,facelms,mpFacemesh.FACEMESH_TESSELATION,mpDrawSpec,mpDrawSpec)
          
          value = []
          for lms in facelms.landmark:      
            
            pos = (lms.x,lms.y,lms.z)
            value.append(pos) #特征点位置

          with open(json_file,'w') as jsonfile:

            json.dump([{'frame':frame,'sx':sx,'jnt_pos':value}],jsonfile,indent=4)
      

      cv2.namedWindow("img",cv2.WINDOW_NORMAL)
      cv2.resizeWindow("img",width,height) #adjust window's width and height
      cv2.imshow("img",image)   

      if cv2.waitKey(50000) & 0xFF == 27:
        exit() 


  def video_capture(self):

    # For video input:
    
    cap = cv2.VideoCapture(self.file)

    basename,ext = os.path.splitext(self.file)
    json_file = self.file.replace(ext,".json") #replace extension with .json

    #准备写入文件的数据
    data = []
    
    sx = 1.0
    frame = 0
    value = []

    dict = {'frame':frame,'sx':sx,'jnt_pos':value}          
    data.append(dict)

    
    with mpFacemesh.FaceMesh(
        max_num_faces=1,
        refine_landmarks=True,
        min_detection_confidence=0.5,
        min_tracking_confidence=0.5) as face_mesh:
      
      while cap.isOpened():
        success, image = cap.read()        

        if not success:
          print("Ignoring empty camera frame.")
          # If loading a video, use 'break' instead of 'continue'.

          with open(json_file,'w') as jsonfile:
            json.dump(data,jsonfile,indent=4)
          break
        
        height = image.shape[0]
        width  = image.shape[1]

        sx=(width / height) #根骨骼x轴缩放

        frame += 1

        value = []

        # To improve performance, optionally mark the image as not writeable to
        # pass by reference.
        image.flags.writeable = False
        image = cv2.cvtColor(image, cv2.COLOR_BGR2RGB)
        results = face_mesh.process(image)

        # Draw the face mesh annotations on the image.
        image.flags.writeable = True
        image = cv2.cvtColor(image, cv2.COLOR_RGB2BGR)
        if results.multi_face_landmarks:
          for facelms in results.multi_face_landmarks:
            mpDrawUtils.draw_landmarks(
                image=image,
                landmark_list=facelms,
                connections=mpFacemesh.FACEMESH_TESSELATION,
                landmark_drawing_spec=None,
                connection_drawing_spec=mp.solutions.mediapipe.python.solutions.drawing_styles
                .get_default_face_mesh_tesselation_style())
            mpDrawUtils.draw_landmarks(
                image=image,
                landmark_list=facelms,
                connections=mpFacemesh.FACEMESH_CONTOURS,
                landmark_drawing_spec=None,
                connection_drawing_spec=mp.solutions.mediapipe.python.solutions.drawing_styles
                .get_default_face_mesh_contours_style())
            mpDrawUtils.draw_landmarks(
                image=image,
                landmark_list=facelms,
                connections=mpFacemesh.FACEMESH_IRISES,
                landmark_drawing_spec=None,
                connection_drawing_spec=mp.solutions.mediapipe.python.solutions.drawing_styles
                .get_default_face_mesh_iris_connections_style())

            for lms in facelms.landmark:
              pos = (lms.x,lms.y,lms.z)
              value.append(pos) #特征点位置
        dict = {'frame':frame,'sx':sx,'jnt_pos':value}

        data.append(dict)

        # Flip the image horizontally for a selfie-view display.
        cv2.imshow('MediaPipe Face Mesh', image)
        if cv2.waitKey(5) & 0xFF == 27:          
          break


if(__name__ == '__main__'):

  '''
    usage: 
    py -3 face_capture.py -v video.mp4
  '''

  import argparse
  parser = argparse.ArgumentParser(
      description='input the filename')
  group = parser.add_mutually_exclusive_group() #添加互斥参数
  group.add_argument(
    '-i','--image',
    help = 'the input image file')
  group.add_argument(
    '-v', '--video', 
    help='the input video file')
  
  
  args = parser.parse_args()

  #print(args)

  if ((args.image==None) and (args.video==None)):

    print("you must input a image or a video file")
  
  elif not (args.image == None):

    face_cap = face_capture(args.image)
    face_cap.image_capture()
  else:
    face_cap = face_capture(args.video)
    face_cap.video_capture()

```

## Source file: src/capture_to_mesh.py

{
  "path": "src/capture_to_mesh.py",
  "blob_sha": "f98625370d313cf11c284aee288dd57964442f3b",
  "sha256": "557d401b5bda65e83a23e3e9c4487ae710aa181754a7d4391a3142f16c17c565",
  "lines": 213,
  "bytes": 6613,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 12
  },
  "url": "https://github.com/benject/mirror/blob/434c2b16e4cfa6274060c6bd62bcaee4396f1d07/src/capture_to_mesh.py",
  "function_signature_candidates": [
    {
      "line": 17,
      "text": "def onMayaDroppedPythonFile(obj):"
    },
    {
      "line": 31,
      "text": "def __init__(self,root_path):"
    },
    {
      "line": 71,
      "text": "def ref_scene_file(self):"
    },
    {
      "line": 82,
      "text": "def display_maya_fps(self,val):"
    },
    {
      "line": 91,
      "text": "def get_maya_fps(self):"
    },
    {
      "line": 103,
      "text": "def select_video_file(self):"
    },
    {
      "line": 120,
      "text": "def select_anim_file(self):"
    },
    {
      "line": 142,
      "text": "def process(self):"
    },
    {
      "line": 150,
      "text": "def __init__(self):"
    },
    {
      "line": 156,
      "text": "def place_joints(self,filename):"
    }
  ]
}

```text

#!/usr/bin/python
# -*- coding: utf-8 -*-

# Author: Bing Chen

import maya.cmds as cmds
import json
import os ,sys
import subprocess

import PySide2.QtCore as QtCore
from PySide2.QtWidgets import QFileDialog
from PySide2.QtUiTools import QUiLoader



def onMayaDroppedPythonFile(obj):

    sys_path = os.path.abspath(os.path.join(os.path.dirname( __file__ ), '..')) 

    sys.path.append(sys_path) #install this mod to maya
    



class GUI():
    '''
    界面
    '''

    def __init__(self,root_path):
        
        self.time_unit = {
        'game': 15.0 ,
        'film': 24.0 ,
        'pal':  25.0 ,
        'ntsc': 30.0 ,
        'show': 48.0 ,
        'palf': 50.0 ,
        'ntscf': 60.0 }

        self.json_file = ''
        self.root_path = root_path



        self.ui_file = QtCore.QFile( os.path.join(self.root_path,r"ui\FaceCapture_ui.ui")) # 实例化 QFILE 对象
        self.ui_file.open(QtCore.QFile.ReadOnly) #打开ui_file

        loader = QUiLoader() #QUIloader 对象
        self.window = loader.load(self.ui_file) # 用QuiLoader对象读取ui_file

        self.ref_file = os.path.abspath(os.path.join(self.root_path,r"scene\facemesh.mb"))

        self.window.lineEdit_2.setText(self.ref_file)

        self.display_maya_fps(self.get_maya_fps())

        self.window.pushButton_4.clicked.connect(self.select_video_file) #将clicked信号连接到 选择视频文件 槽函数。 函数名作为参数 不带括号

        self.window.pushButton_3.clicked.connect(self.ref_scene_file) #将clicked信号连接到 参考文件 槽函数。 函数名作为参数 不带括号

        self.window.pushButton.clicked.connect(self.select_anim_file) #将clicked信号连接到 选择文件 槽函数。 函数名作为参数 不带括号

        #self.window.pushButton_2.clicked.connect(lambda: self.display_maya_fps(self.get_maya_fps())) #pyside 如果需要传参数 用 lamda表达式
        self.window.pushButton_2.clicked.connect(self.process)


        self.window.show()

    def ref_scene_file(self):

        cmds.file(self.ref_file, r=True, mergeNamespacesOnClash=False, namespace='facemesh')
        cmds.select("facemesh:root",r=True,hi=True)
        self.jnt_arr = cmds.ls(sl=True)[1:]
        for joint in self.jnt_arr:
            cmds.setKeyframe( joint + ".tx")
            cmds.setKeyframe( joint + ".ty")
            cmds.setKeyframe( joint + ".tz")
    

    def display_maya_fps(self,val):

        '''
        在界面上显示maya的fps
        '''
        
        self.window.label_2.setText(str(val))

    
    def get_maya_fps(self):

        '''
        获取maya 的fps
        '''
        try:
            self.fps_maya = self.time_unit[cmds.currentUnit(q=True,time=True)]
        except:
            self.fps_maya = float(cmds.currentUnit(q=True,time=True).replace("fps",""))

        return self.fps_maya

    def select_video_file(self):
        '''
        选择video文件
        '''

        dlg = QFileDialog()

        self.video_file = dlg.getOpenFileName()[0] #返回的是元组 获得video

        self.window.lineEdit_3.setText(self.video_file)

        p = subprocess.Popen(['py', '-3', os.path.abspath(os.path.join(self.root_path,r'src\face_capture.py')), '-v',self.video_file])

        print("processing video capture" , self.video_file)



    def select_anim_file(self):
        
        '''
        选择json文件
        '''

        dlg = QFileDialog()
        

        self.json_file = dlg.getOpenFileName()[0] #返回的是元组 获得json

        self.window.lineEdit.setText(self.json_file)

        #print(self.json_file)        
        
        file = open(self.json_file,"r")
        datas = json.load(file) #datas的数据结构为 [ {frame:%d,sx:%f,jnt_pos:[%f,%f,%f]},{...} ]
        file.close()
        
        self.window.label_4.setText(str(len(datas)))        


    def process(self):

        c2m = Capture_to_Mesh()
        c2m.place_joints(filename=self.json_file)



class Capture_to_Mesh():
    def __init__(self):

        #获取骨骼列表
        cmds.select("facemesh:root",r=True,hi=True)
        self.jnt_arr = cmds.ls(sl=True)[1:]

    def place_joints(self,filename):
        #读取json文件
        file = open(filename,"r")
        datas = json.load(file) #datas的数据结构为 [ {frame:%d,sx:%f,jnt_pos:[%f,%f,%f]},{...} ]

        for data in datas:

            sx = data['sx']
            pos_arr = data['jnt_pos']

            cmds.currentTime(data['frame'])

            if(len(pos_arr)==0):
                pass

            else:
            #初始化根骨骼位置，由于landmark值均为绝对坐标 因此在每一帧要先还原根骨骼位置，最后再将根骨骼位置进行旋转和缩放。

                cmds.setAttr("facemesh:root.rx",0)
                cmds.setAttr("facemesh:root.sx",1)
                cmds.setAttr("facemesh:root.sz",1) 

            #根据文件内容驱动骨骼位置

            '''
            要将mediapipe的landmark位置还原到maya空间,需要三个步骤:
            
            首先, 将landmark位置传给骨骼。mediapipe的landmark位置也就是骨骼位置(x,y,z)被归一化到[0,1]区间,
            为了匹配到图像,x的位置要乘上图片的宽,y的位置要乘上图片的高。在这里简单的将根骨骼y轴缩放设定为1 则 x轴的缩放就是 图像宽/图像高   

            其次, 根据mediapipe的文档, 摄像机的坐标和maya的世界坐标关系, 要将根骨骼的x轴旋转180°

            再次, 根据mediapipe 的文档,mediapipe采用了weak perspective projection camera model 因此 z轴的缩放与x轴缩放相同。
            原文:“while the Z coordinate is relative and is scaled as the X coodinate under the weak perspective projection camera model”
    
            '''
            for id , pos in enumerate(pos_arr):
                x = pos[0]
                y = pos[1]
                z = pos[2]
                #468-478是10个瞳孔的坐标（左右各5个）
                if id>467:
                    break

                cmds.xform(self.jnt_arr[id],ws=True,t=(float(x),float(y),float(z)))
                    
            cmds.setAttr("facemesh:root.sx",sx)  #第一步
            cmds.setAttr("facemesh:root.rx",180) #第二步
            cmds.setAttr("facemesh:root.sz",sx)  #第三步


# ====================================

import capture_to_mesh 

root_path = os.path.abspath(os.path.join(os.path.dirname( __file__ ), '..')) #after install we can get the current file's path
gui = capture_to_mesh.GUI(root_path) 



```
