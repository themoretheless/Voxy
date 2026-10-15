# Facial rendering source scan: mmanoharahere/GeneralizedSpeechAnimation

Repository: https://github.com/mmanoharahere/GeneralizedSpeechAnimation

Pinned source revision: 81a2434734ad0b7fd6f510b2132b7bb775da16ce

Discovery topic: mouth animation

Description: Generated mouth texture animations that synchronizes with input speech from a single reference speaker using LSTM.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: config.py

{
  "path": "config.py",
  "blob_sha": "36107d0c3cf20e6a7ea66378e060bd08158b2838",
  "sha256": "d45069ed3fa32d997271359a234f24ff2b4fd9d00808bbd2f034f4f1169e5af1",
  "lines": 67,
  "bytes": 1726,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 1
  },
  "url": "https://github.com/mmanoharahere/GeneralizedSpeechAnimation/blob/81a2434734ad0b7fd6f510b2132b7bb775da16ce/config.py",
  "function_signature_candidates": []
}

```text

#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Created on Wed Nov 14 13:19:09 2018

@author: shruti
"""

import os


#generate trimmed videos required paths :
dataset_path = 'speech_animation/'
video_path = os.path.join(dataset_path, 'data/videos')
trimmed_videos_path = os.path.join(dataset_path, 'data/trimmed_videos')
captions_path = os.path.join(dataset_path, 'data/captions')

number_of_videos = 303 #4


#audio features extraction required var
audio_path = os.path.join(dataset_path, 'data/audios')
audio_kp_path = os.path.join(dataset_path, 'data/audio_kp')


#video features extract required variables
extract_images_path = os.path.join(dataset_path, 'data/images')
images_kp_path = os.path.join(dataset_path, 'data/images_kp_raw')


#pre-download this -- add this to the requirements file
pre_download_shape_predictor = 'shape_predictor_68_face_landmarks.dat' 


#pca and upsampling file
pca_path = os.path.join(dataset_path, 'pca')
numOfFiles = 19087 #139


#run.py required variables
time_delay = 20
look_back = 50


#test file names
filename = '00302-014' 
#Good videos:    '00300-050', '00221-043', 
#videos:     '00295-001', '00301-097', '00296-014'
#Bad videos:     '00300-039', '00302-014'


inputFile = 'data/audios/' + filename + '.wav'
outputFolder = 'testing_output_images_' + filename + '/' 
model_file = 'my_model.h5'


#train.py required variables
train_epoch = 30
train_n_videos = numOfFiles #2


#test file variables
inputTestFolder = 'data/images/' + filename +'/'
inputToA2KeyModel = 'data/test_data_' + filename + '/images/'
outputFolderGroundTruth ='data/GroundTruth_input_' + filename + '/'
outputFolderKp = 'data/test_data_' + filename + '/'
saveFilename = outputFolderKp + 'kp_test.pickle'


```

## Source file: runModel.py

{
  "path": "runModel.py",
  "blob_sha": "79fe14c88778330ca5611501313ca61a0035b4fc",
  "sha256": "405da0fadd86bb9262d91e117b3f1fbb8236216f08315c72806735f6e327a0ef",
  "lines": 204,
  "bytes": 6984,
  "selection_score": 0,
  "keyword_line_counts": {
    "mouth": 7,
    "rig": 2,
    "lip": 2
  },
  "url": "https://github.com/mmanoharahere/GeneralizedSpeechAnimation/blob/81a2434734ad0b7fd6f510b2132b7bb775da16ce/runModel.py",
  "function_signature_candidates": [
    {
      "line": 30,
      "text": "def __init__(self):"
    },
    {
      "line": 46,
      "text": "def get_audio_features(self,X):"
    },
    {
      "line": 62,
      "text": "def scale_data(self,X):"
    },
    {
      "line": 67,
      "text": "def predict_y(self,X):"
    },
    {
      "line": 81,
      "text": "def subsample(self, y, fps_from = 100.0, fps_to = 29.97):"
    },
    {
      "line": 95,
      "text": "def get_kp_data(self):"
    },
    {
      "line": 100,
      "text": "def set_kp_length(self, kp, y_pred):"
    },
    {
      "line": 110,
      "text": "def getOriginalKeypoints(self,kp_features_mouth, N, tilt, mean):"
    },
    {
      "line": 122,
      "text": "def drawLips(self,keypoints, new_img, c = (255, 255, 255), th = 1, show = False):"
    },
    {
      "line": 136,
      "text": "def get_mouth_boundary_images(self,y_pred,kp):"
    },
    {
      "line": 159,
      "text": "def run(self):"
    }
  ]
}

```text

#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Created on Thu Nov 22 17:03:29 2018

@author: shruti
"""

from keras.models import load_model

from sklearn.preprocessing import MinMaxScaler
import numpy as np

import pickle as pkl

import cv2
import scipy.io.wavfile as wav
from python_speech_features import logfbank
import subprocess
import imageio
import os
import glob

from config import *
#########################################################################################
#define a class to get trimmed videos in the entire folder

class runModel:
    
    def __init__(self):
         self.time_delay = time_delay
         #initiliaze audio extraction
         self.look_back = look_back
         self.outputFolder = outputFolder
         self.model = load_model(model_file)
         self.inputFile = inputFile
         #check if the path exists
         #if it exists -- delete the previously saved one and save a new one.
         
         if(os.path.exists(self.outputFolder)):
             cmd = 'rm -rf '+self.outputFolder + '&& mkdir ' + self.outputFolder
             subprocess.call(cmd ,shell=True)
         if not(os.path.exists(self.outputFolder)):
             subprocess.call('mkdir -p ' + self.outputFolder, shell=True)  
    
    def get_audio_features(self,X):
        (rate, sig) = wav.read(self.inputFile)
        audio = logfbank(sig,rate)
        
        start = ( self.time_delay- self.look_back) if ( self.time_delay- self.look_back > 0) else 0
        for i in range(start, len(audio)- self.look_back):
             a = np.array(audio[i:i+ self.look_back])
             X.append(a)
        
        X = np.array(X)
        shapeX = X.shape
        X = X.reshape(-1, X.shape[2])
        print('Shapes:', X.shape)
        
        return X,shapeX
    
    def scale_data(self,X):
        scalerX = MinMaxScaler(feature_range=(0, 1))
        X = scalerX.fit_transform(X)
        return X
    
    def predict_y(self,X):
        with open('data/pca/scalery22847.pickle', 'rb') as pkl_file: #####change here
            scalery = pkl.load(pkl_file) 
            
        with open('data/pca/pca22847.pickle', 'rb') as pkl_file:
            pca = pkl.load(pkl_file)
            
        y_pred = self.model.predict(X)
        y_pred = scalery.inverse_transform(y_pred)
        y_pred = pca.inverse_transform(y_pred)
        
        return y_pred
   
     
    def subsample(self, y, fps_from = 100.0, fps_to = 29.97):
        factor = int(np.ceil(fps_from/fps_to))
        #subsample the points
        new_y = np.zeros((int(y.shape[0]/factor), 20, 2)) #(timesteps, 20) = (500, 20x2)
        for idx in range(new_y.shape[0]):
            if not (idx*factor > y.shape[0]-1):
			# Get into (x, y) format
                new_y[idx, :, 0] = y[idx*factor, 0:20]
                new_y[idx, :, 1] = y[idx*factor, 20:]
            else:
                break
        new_y = [np.array(each) for each in new_y.tolist()]
        return new_y

    def get_kp_data(self):
        with open('data/test_data_' + filename + '/kp_test.pickle', 'rb') as pkl_file: ######change hereeee
            kp= pkl.load(pkl_file)
        return kp  
    
    def set_kp_length(self, kp, y_pred):
        if (len(kp) < len(y_pred)):
            n = len(kp)
            y_pred = y_pred[:n]
        else:
            n = len(y_pred)
            kp = kp[:n]
            
        return n,kp,y_pred
    
    def getOriginalKeypoints(self,kp_features_mouth, N, tilt, mean):
        # Denormalize the points
        kp_dn = N * kp_features_mouth
        # Add the tilt
        x, y = kp_dn[:, 0], kp_dn[:, 1]
        c, s = np.cos(tilt), np.sin(tilt)
        x_dash, y_dash = x*c + y*s, -x*s + y*c
        kp_tilt = np.hstack((x_dash.reshape((-1,1)), y_dash.reshape((-1,1))))
        # Shift to the mean
        kp = kp_tilt + mean
        return kp
    
    def drawLips(self,keypoints, new_img, c = (255, 255, 255), th = 1, show = False):
        keypoints = np.float32(keypoints)
        for i in range(48, 59):
            cv2.line(new_img, tuple(keypoints[i]), tuple(keypoints[i+1]), color=c, thickness=th)
        cv2.line(new_img, tuple(keypoints[48]), tuple(keypoints[59]), color=c, thickness=th)
        cv2.line(new_img, tuple(keypoints[48]), tuple(keypoints[60]), color=c, thickness=th)
        cv2.line(new_img, tuple(keypoints[54]), tuple(keypoints[64]), color=c, thickness=th)
        cv2.line(new_img, tuple(keypoints[67]), tuple(keypoints[60]), color=c, thickness=th)
        
        for i in range(60, 67):
            cv2.line(new_img, tuple(keypoints[i]), tuple(keypoints[i+1]), color=c, thickness=th)
            
        return
        
    def get_mouth_boundary_images(self,y_pred,kp):
        for idx, (x, k) in enumerate(zip(y_pred, kp)):
            
            unit_mouth_kp, N, tilt, mean, unit_kp, keypoints = k[0], k[1], k[2], k[3], k[4], k[5]
            kps = self.getOriginalKeypoints(x, N, tilt, mean)
            keypoints[48:68] = kps
            
            imgfile = 'data/test_data_' + filename + '/images/' + str(idx+1).rjust(5, '0') + '.png'  ####change is required
            im = cv2.imread(imgfile)
            self.drawLips(keypoints, im, c = (255, 255, 255), th = 1, show = False)
            
            
            im_out = np.zeros_like(im)
            im1 = np.hstack((im, im_out))
            im1 = im1[:, :256]
            
            cv2.imwrite(self.outputFolder + str(idx) + '.png', im1)
            
            
        print('Done writing all images')
        
        return
                
    def run(self):
        X = []
        X ,shapeX= self.get_audio_features(X)
        X = self.scale_data(X)
        X = X.reshape(shapeX)
        
        #predict y - pred
        y_pred = self.predict_y(X)
        print('Upsampled number:', len(y_pred))
        y_pred = self.subsample(y_pred, 100, 34)
        print('Subsampled :', len(y_pred))
        
        ##visualization---------------------------------
        kp = self.get_kp_data()
        n,kp,y_pred = self.set_kp_length(kp, y_pred)
        
        ### get the mouth boundaries on images
        self.get_mouth_boundary_images(y_pred, kp)
        return        



#read path from the user to the data folder
if __name__== "__main__":
    runModel_obj = runModel()
    runModel_obj.run()
    print("Images with mouth boundaries on Obama saved!!!!") 
    
    
###Generate video
os.chdir(outputFolder)
with imageio.get_writer('Results.mp4', mode='I', fps=30) as writer:
        for file in sorted(glob.glob("*.png"), key=os.path.getmtime):
            image = imageio.imread(file)
            writer.append_data(image)
print("Video without audio generated!!!!")

os.chdir('..')

input_path = outputFolder + "Results.mp4"
result_path = outputFolder + "Output.mp4"

cmd = "ffmpeg -i '"+input_path+ "' -i '"+ inputFile + "' \
-c:v copy -c:a aac -strict experimental '"+result_path+"'"
subprocess.call(cmd ,shell=True)
print("Video with audio generated!!!")


```

## Source file: utils_ref.py

{
  "path": "utils_ref.py",
  "blob_sha": "b28b932cd4a10be8bcbf098ed3ff6b332ea68395",
  "sha256": "87344f70c270ce1f7ea5263505678f0794c1cf0df6ea45b1dbf81ee5af32d94b",
  "lines": 80,
  "bytes": 2338,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 3,
    "face": 3,
    "lip": 4
  },
  "url": "https://github.com/mmanoharahere/GeneralizedSpeechAnimation/blob/81a2434734ad0b7fd6f510b2132b7bb775da16ce/utils_ref.py",
  "function_signature_candidates": [
    {
      "line": 20,
      "text": "def get_sec(time):"
    },
    {
      "line": 25,
      "text": "def all_landmarks(filename):"
    },
    {
      "line": 36,
      "text": "def Tilt(landmarks):"
    },
    {
      "line": 46,
      "text": "def getKP(keypoints):"
    },
    {
      "line": 63,
      "text": "def drawLips(keypoints, new_img, c = (255, 255, 255), th = 1, show = False):"
    }
  ]
}

```text

#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Created on Wed Nov 14 13:17:10 2018

@author: mansi
"""

import numpy as np
import dlib
from skimage import io
from imutils import face_utils
import cv2
from config import *

predictor_path = pre_download_shape_predictor
detector = dlib.get_frontal_face_detector()
predictor = dlib.shape_predictor(predictor_path)

def get_sec(time):
    hours, mins , seconds = time.split(':')
    return int(hours) * 3600 + int(mins) * 60 + float(seconds)


def all_landmarks(filename):
    
    image = io.imread(filename)
    locate = detector(image, 1);
    landmarks = np.empty([1,1])
    for i, j in enumerate(locate):
        landmarks = predictor(image, j);
        landmarks = face_utils.shape_to_np(landmarks);
    
    return landmarks

def Tilt(landmarks):
    
    eyes_keypoints = np.array(landmarks[36:48])
    x = eyes_keypoints[:, 0]
    y = -1*eyes_keypoints[:, 1]
    m = np.polyfit(x, y, 1)
    tilt = np.degrees(np.arctan(m[0]))
    
    return tilt

def getKP(keypoints):

    mean_lip_keypoints = np.average(keypoints[48:68], 0)
    keypoints_new = keypoints - mean_lip_keypoints
    x_new = keypoints_new[:, 0]
    y_new = keypoints_new[:, 1]
    theta = np.deg2rad(Tilt(keypoints_new))
    c = np.cos(theta)
    s = np.sin(theta)
    x = x_new*c - y_new*s
    y = x_new*s + y_new*c 
    keypoints_tilt = np.hstack((x.reshape((-1,1)), y.reshape((-1,1))))
    N = np.linalg.norm(keypoints_tilt, 2)
    
    return [keypoints_tilt/N, N, theta, mean_lip_keypoints]


def drawLips(keypoints, new_img, c = (255, 255, 255), th = 1, show = False):
    
    keypoints = np.float32(keypoints)
    
    for i in range(48, 59):
        cv2.line(new_img, tuple(keypoints[i]), tuple(keypoints[i+1]), color=c, thickness=th)
    cv2.line(new_img, tuple(keypoints[48]), tuple(keypoints[59]), color=c, thickness=th)
    cv2.line(new_img, tuple(keypoints[48]), tuple(keypoints[60]), color=c, thickness=th)
    cv2.line(new_img, tuple(keypoints[54]), tuple(keypoints[64]), color=c, thickness=th)
    cv2.line(new_img, tuple(keypoints[67]), tuple(keypoints[60]), color=c, thickness=th)
    for i in range(60, 67):
        cv2.line(new_img, tuple(keypoints[i]), tuple(keypoints[i+1]), color=c, thickness=th)
    
    if (show == True):
        cv2.imshow('lol', new_img)
        cv2.waitKey(10000)




```

## Source file: trainModel.py

{
  "path": "trainModel.py",
  "blob_sha": "0088f303191aff8f4f8da3860d6d5b0c3b7b62ff",
  "sha256": "252abc5041f6c2db94e4ae9642725a6344b798a78acd41cc4d788368267ce68d",
  "lines": 148,
  "bytes": 5091,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/mmanoharahere/GeneralizedSpeechAnimation/blob/81a2434734ad0b7fd6f510b2132b7bb775da16ce/trainModel.py",
  "function_signature_candidates": [
    {
      "line": 20,
      "text": "def __init__(self):"
    },
    {
      "line": 29,
      "text": "def load_pickle_files(self):"
    },
    {
      "line": 39,
      "text": "def get_common_keys(self,audio_kp,video_kp):"
    },
    {
      "line": 46,
      "text": "def scale_data(self,X):"
    },
    {
      "line": 52,
      "text": "def get_kp_range(self,X,y,keys,audio_kp,video_kp):"
    },
    {
      "line": 69,
      "text": "def create_model(self):"
    },
    {
      "line": 78,
      "text": "def run(self):"
    }
  ]
}

```text

#!/usr/bin/env python3
# -*- coding: utf-8 -*-

from keras.models import Sequential
from keras.callbacks import ModelCheckpoint
from keras.layers import Dense, LSTM, Dropout
from sklearn.preprocessing import MinMaxScaler
import numpy as np
from tqdm import tqdm
import pickle as pkl
from keras.callbacks import TensorBoard
from time import time
from operator import itemgetter as ig
from config import *
#########################################################################################
#define a class to get trimmed videos in the entire folder

class trainModel:

    def __init__(self):
         self.time_delay = time_delay
         #initiliaze audio extraction
         self.look_back = look_back
         self.n_epoch = train_epoch
         self.n_videos = train_n_videos
         
         
    
    def load_pickle_files(self):
        with open('data/audio_kp_full/audio_kp22847.pickle', 'rb') as pkl_file:
            audio_kp = pkl.load(pkl_file)
        with open('data/pca_full/pkp22847.pickle', 'rb') as pkl_file:
            video_kp = pkl.load(pkl_file)
        with open('data/pca_full/pca22847.pickle', 'rb') as pkl_file:
            pca = pkl.load(pkl_file)
        
        return audio_kp,video_kp,pca
    
    def get_common_keys(self,audio_kp,video_kp):
        keys_audio = audio_kp.keys()
        keys_video = video_kp.keys()
        keys = sorted(list(set(keys_audio).intersection(set(keys_video))))
        
        return keys
    
    def scale_data(self,X):
        scalerX = MinMaxScaler(feature_range=(0, 1))
        X = scalerX.fit_transform(X)
        return X
    
    
    def get_kp_range(self,X,y,keys,audio_kp,video_kp):
        for key in tqdm(keys[0:self.n_videos]):
            audio = audio_kp[key]
            video = video_kp[key]
            if (len(audio) > len(video)):
                audio = audio[0:len(video)]
            else:
                video = video[0:len(audio)]
            start = (self.time_delay-self.look_back) if (self.time_delay-self.look_back > 0) else 0
            for i in range(start, len(audio)-self.look_back):
                a = np.array(audio[i:i+self.look_back])
                v = np.array(video[i+self.look_back-self.time_delay]).reshape((1, -1))
                X.append(a)
                y.append(v)
        return X,y
                
     
    def create_model(self):
        model = Sequential()
        model.add(LSTM(60, input_shape=(self.look_back, 26)))
        model.add(Dropout(0.25))
        model.add(Dense(20))
        model.compile(loss='mean_squared_error', optimizer='adam')
        print(model.summary())
        return model
    
    def run(self):
        audio_kp,video_kp,pca = self.load_pickle_files()
        c = list(audio_kp.keys())
        d = c[:numOfFiles] #19087 -- change in config
    
        audio_kp = dict(zip(d, ig(*d)(audio_kp)))
            
        keys = self.get_common_keys(audio_kp,video_kp)
        X,y = [],[]
        
        X,y = self.get_kp_range(X,y,keys,audio_kp,video_kp)
        X = np.array(X)
        y = np.array(y)
        shapeX = X.shape
        shapey = y.shape
        #print('Shapes:', X.shape, y.shape)
        X = X.reshape(-1, X.shape[2])
        y = y.reshape(-1, y.shape[2])
        #print('Shapes:', X.shape, y.shape)
        
        scalerX = MinMaxScaler(feature_range=(0, 1))
        scalery = MinMaxScaler(feature_range=(0, 1))
            
        scaler_transform = scalery.fit(y)
        
        with open('data/pca_full/scalery22847.pickle', 'wb') as pkl_file:
        	pkl.dump(scaler_transform, pkl_file)
            
        
        X = scalerX.fit_transform(X)
        y = scaler_transform.transform(y)
        
        
        X = X.reshape(shapeX)
        y = y.reshape(shapey[0], shapey[2])
        
        print('Shapes:', X.shape, y.shape)
        print('X mean:', np.mean(X), 'X var:', np.var(X))
        print('y mean:', np.mean(y), 'y var:', np.var(y))
        
        split1 = int(0.8*X.shape[0])
        split2 = int(0.9*X.shape[0])
        
        train_X = X[0:split1]
        train_y = y[0:split1]
        val_X = X[split1:split2]
        val_y = y[split1:split2]
        test_X = X[split2:]
        test_y = y[split2:]
        
        tbCallback = TensorBoard(log_dir="logs/{}".format(time()))
        checkpoint = ModelCheckpoint('my_model_best.h5', monitor='val_loss', verbose=0, save_best_only=True, save_weights_only=False, mode='auto', period=1)

        model = self.create_model()
        model.fit(train_X, train_y, epochs=self.n_epoch, batch_size=4, verbose=1, shuffle=True, callbacks=[tbCallback, checkpoint], validation_data=(val_X, val_y))
        test_error = np.mean(np.square(test_y - model.predict(test_X)))
        print('Test Error: ', test_error)
        
        model.save('my_model.h5')
        model.save_weights('my_model_weights.h5')
        print('Saved Model.')
        
        
        return        

#read path from the user to the data folder
if __name__== "__main__":
    trainModel_obj = trainModel()
    trainModel_obj.run()
    print("Model is training!") 
    


```
