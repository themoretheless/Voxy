# Facial rendering source scan: tobeatraceur/Face-Deformation

Repository: https://github.com/tobeatraceur/Face-Deformation

Pinned source revision: 2dddefdf1729706de1125096fbc11053249a2427

Discovery topic: face deformation

Description: a software with GUI to change one face to another by TPS(Thin plate spline) and B-spline algorithms

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"face": ["Code/HumanFace.pro", "Code/HumanFace.pro.user", "Code/debug/HumanFace.exe", "Code/face_detect.py", "Code/release/HumanFace.exe", "Code/shape_predictor_68_face_landmarks.dat", "EXE/HumanFace.exe", "EXE/face_detect.py", "EXE/shape_predictor_68_face_landmarks.dat"]}

## Source file: EXE/face_detect.py

{
  "path": "EXE/face_detect.py",
  "blob_sha": "e18004fe8946df24450952cd5c813c1251f51829",
  "sha256": "17d65210204ffde799bb54625faac82ad4c8763e70b1ee6c2c497b0286820191",
  "lines": 46,
  "bytes": 1158,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 2
  },
  "url": "https://github.com/tobeatraceur/Face-Deformation/blob/2dddefdf1729706de1125096fbc11053249a2427/EXE/face_detect.py",
  "function_signature_candidates": [
    {
      "line": 12,
      "text": "def detect(jpgname,txtname):"
    }
  ]
}

```text

# -*- coding: utf-8 -*-
"""
Created on Sat Nov 10 12:08:14 2018

@author: traceur
"""

import numpy as np
import cv2
import dlib

def detect(jpgname,txtname):
    detector = dlib.get_frontal_face_detector()
    predictor = dlib.shape_predictor('shape_predictor_68_face_landmarks.dat')
    
    # cv2读取图像
    img = cv2.imread(jpgname)
    #os.mknod("test.txt")       
    fp = open(txtname,'w')
    # 取灰度
    img_gray = cv2.cvtColor(img, cv2.COLOR_RGB2GRAY)
    
    # 人脸数rects
    rects = detector(img_gray, 0)
    for i in range(len(rects)):
        landmarks = np.matrix([[p.x, p.y] for p in predictor(img,rects[i]).parts()])
        for idx, point in enumerate(landmarks):
            # 68点的坐标
            pos = (point[0, 0], point[0, 1])
            print(idx,pos)
            new_context = str(float(point[0, 0])) + " " + str(float(point[0, 1])) + '\n'
            fp.write(new_context)     
    fp.close()

while 1:
	jpgname = input("输入人脸图片名称: ")
	txtname = input("输入关键点文档名称: ")
	detect(jpgname,txtname)
	print("识别成功")
	choice = input("exit? y/n:")
	if choice == 'y':
		break

	

	


```

## Source file: Code/face_detect.py

{
  "path": "Code/face_detect.py",
  "blob_sha": "831e5d5df15ee2b79bcd9ebb4662d77aae4511ce",
  "sha256": "363609a5f1715ba9a96d96b8a870980e62475971684b9507a41cedc5e4075ef9",
  "lines": 44,
  "bytes": 1339,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 2
  },
  "url": "https://github.com/tobeatraceur/Face-Deformation/blob/2dddefdf1729706de1125096fbc11053249a2427/Code/face_detect.py",
  "function_signature_candidates": [
    {
      "line": 12,
      "text": "def detect(jpgname,txtname):"
    }
  ]
}

```text

# -*- coding: utf-8 -*-
"""
Created on Sat Nov 10 12:08:14 2018

@author: traceur
"""

import numpy as np
import cv2
import dlib

def detect(jpgname,txtname):
    detector = dlib.get_frontal_face_detector()
    predictor = dlib.shape_predictor('shape_predictor_68_face_landmarks.dat')
    
    # cv2读取图像
    img = cv2.imread(jpgname)
    #os.mknod("test.txt")       
    fp = open(txtname,'w')
    # 取灰度
    img_gray = cv2.cvtColor(img, cv2.COLOR_RGB2GRAY)
    
    # 人脸数rects
    rects = detector(img_gray, 0)
    for i in range(len(rects)):
        landmarks = np.matrix([[p.x, p.y] for p in predictor(img,rects[i]).parts()])
        for idx, point in enumerate(landmarks):
            # 68点的坐标
            pos = (point[0, 0], point[0, 1])
            #print(idx,pos)
            new_context = str(float(point[0, 0])) + " " + str(float(point[0, 1])) + '\n'
            fp.write(new_context)
            # 利用cv2.circle给每个特征点画一个圈，共68个
            #cv2.circle(img, pos, 5, color=(0, 255, 0))
            # 利用cv2.putText输出1-68
            #font = cv2.FONT_HERSHEY_SIMPLEX
            #cv2.putText(img, str(idx+1), pos, font, 0.8, (0, 0, 255), 1,cv2.LINE_AA)
    
    #cv2.namedWindow("img", 2)
    #cv2.imshow("img", img)
    #cv2.waitKey(0)
    
            
    fp.close()


```

## Source file: Code/b.h

{
  "path": "Code/b.h",
  "blob_sha": "ecb3b9b07604c412e1a910b0819a53c9f286058d",
  "sha256": "c1f74e40adcdb8748af79ddcdab0bf5cde935ec3ccff2489b9f3ea2aef447516",
  "lines": 11,
  "bytes": 70,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/tobeatraceur/Face-Deformation/blob/2dddefdf1729706de1125096fbc11053249a2427/Code/b.h",
  "function_signature_candidates": []
}

```text

#ifndef B_H
#define B_H


class B
{
public:
    B();
};

#endif // B_H

```

## Source file: Code/b.cpp

{
  "path": "Code/b.cpp",
  "blob_sha": "a697669a69331af748f65b175ed4dba58042a69c",
  "sha256": "6d9f02d7fd681f7bacb477351e9386611c24a4ea7fed3a8ef6fbcb9691817f54",
  "lines": 6,
  "bytes": 28,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/tobeatraceur/Face-Deformation/blob/2dddefdf1729706de1125096fbc11053249a2427/Code/b.cpp",
  "function_signature_candidates": []
}

```text

#include "b.h"

B::B()
{

}


```
