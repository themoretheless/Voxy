# Facial rendering source scan: JaewonSongJ1/DirectFacialRetargeting

Repository: https://github.com/JaewonSongJ1/DirectFacialRetargeting

Pinned source revision: f10456e723b62dd735d64efd9ff5218877fc1a35

Discovery topic: facial retargeting

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: CFR_training.py

{
  "path": "CFR_training.py",
  "blob_sha": "765b65fa9eb2aee2aee91e61096e5a8dc33055d7",
  "sha256": "54d2ad5b3cbee397ce244ad6eb41cbb7853e967be769205b8fb8405950a36c99",
  "lines": 162,
  "bytes": 5690,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/JaewonSongJ1/DirectFacialRetargeting/blob/f10456e723b62dd735d64efd9ff5218877fc1a35/CFR_training.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def __init__(self) :"
    },
    {
      "line": 13,
      "text": "def __del__(self) :"
    },
    {
      "line": 15,
      "text": "def reset(self) :"
    },
    {
      "line": 28,
      "text": "def buildDistMatrix(self, distMat, inputMat):"
    },
    {
      "line": 41,
      "text": "def basisFunc(self, i, x2) :"
    },
    {
      "line": 52,
      "text": "def dist(self, a, b) :"
    },
    {
      "line": 64,
      "text": "def buildBasisMatrix(self, inputMat) :"
    },
    {
      "line": 84,
      "text": "def setBasisFunction(self, basisType) :"
    },
    {
      "line": 92,
      "text": "def setLamda(self, value) :"
    },
    {
      "line": 96,
      "text": "def train(self, inputMat, outputMat):"
    },
    {
      "line": 109,
      "text": "def interpolate(self, sample) :"
    },
    {
      "line": 123,
      "text": "def __init__(self):"
    },
    {
      "line": 126,
      "text": "def RBFtraining(self, srcROE, tgtROE) :"
    },
    {
      "line": 135,
      "text": "def RBFrunning(self, srcAnimDataList):"
    }
  ]
}

```text

'''
Created on 2015. 6. 1.

@author: cimple
'''

import numpy as np
CFR_trainig_DEBUG = 0

class RBF :
    def __init__(self) :
        self.reset()        
    def __del__(self) :
        self.reset()
    def reset(self) :
        self.basisType = 'HARDY'        
        self.lamda = 0.0
        self.numInput = 0
        self.dimInput = 0
        self.dimOutput = 0
        self.inputMat = np.arange(0.0)
        self.basisMat = np.arange(0.0)
        self.inverseBasisMat = np.arange(0.0)
        self.weightMat = np.arange(0.0)
        self.minDist = np.arange(0.0)
        return;
    
    def buildDistMatrix(self, distMat, inputMat):        
        for i in range(self.numInput) :
            for j in range(self.numInput) :
                if(i == j) : distMat[i,j] = 0.0
                else : distMat[i,j] = self.dist(inputMat[i], inputMat[j])
        for i in range(self.numInput) :
            for j in range(self.numInput) :
                dmin = float("inf")
                if distMat[i,j] < dmin :
                    dmin = distMat[i, j]                    
                    self.minDist[i] = dmin
        return;
    
    def basisFunc(self, i, x2) :
        if self.basisType == 'HARDY' :
            return (x2 + self.minDist[i])**0.5
        elif self.basisType == 'GAUSSIAN' :
            return np.exp(-x2*0.1*0.1)
        elif self.basisType == 'LINEAR' :
            return abs((x2)**0.5)            
        else :
            print "Wrong basis type!!"
            return;
            
    def dist(self, a, b) :
        d1 = 0.0
        d2 = 0.0

        dim = a.shape[0]

        for i in range(dim) :
            d1 = a[i] - b[i]
            d2 += (d1*d1)

        return d2                
        
    def buildBasisMatrix(self, inputMat) :
        self.numInput = inputMat.shape[0]
        if(self.numInput <= 0) : return;
        self.dimInput = inputMat.shape[1]
        distMat = np.zeros((self.numInput,self.numInput))
        self.minDist = np.zeros(self.numInput)
               
        self.buildDistMatrix(distMat, inputMat)
        if CFR_trainig_DEBUG : print "Build dist matrix done"
        self.basisMat = np.zeros((self.numInput, self.numInput))
        self.inverseBasisMat = np.zeros((self.numInput, self.numInput))
        for i in range(self.numInput) :
            for j in range(self.numInput) :               
                self.basisMat[i, j] = self.basisFunc(j, distMat[i, j])
                if(i==j) : self.basisMat[i, j] += self.lamda
        if CFR_trainig_DEBUG : print "Calculate basis mat done"
        self.inverseBasisMat = np.linalg.inv(self.basisMat)
        if CFR_trainig_DEBUG : print "Calculate inverse mat done"
        return;
    
    def setBasisFunction(self, basisType) :
        if basisType not in ['GAUSSIAN', 'HARDY', 'LINEAR'] :
            print "Wrong basis function type!"
            return;
        else :
            self.basisType = basisType
            return;
        
    def setLamda(self, value) :
        self.lamda = value
        return;
    
    def train(self, inputMat, outputMat):
        if(outputMat.shape[0] <= 0) : return;
        if CFR_trainig_DEBUG : print "Build basis matrix start"
        self.dimOutput = outputMat.shape[1]
        self.inputMat = inputMat
        self.buildBasisMatrix(inputMat)
        resultMat = np.zeros((self.numInput, self.dimOutput))
        for i in  range(self.numInput) :
            for j in range(self.dimOutput) :
                resultMat[i,j] = outputMat[i,j]
        self.weightMat = np.dot(self.inverseBasisMat, resultMat)
        return;
    
    def interpolate(self, sample) :
        sampleMat = np.zeros(self.numInput)
        resultMat = np.zeros(self.dimOutput)        
        for i in range(self.numInput) :
            sampleMat[i] = self.basisFunc(i, self.dist(sample, self.inputMat[i]))        
        resultMat = np.dot(sampleMat, self.weightMat)
        result = np.zeros(self.dimOutput)
        
        for i in range(self.dimOutput) :            
            result[i] = resultMat[i]
        return result;  
    
    
class RBFtrain :    
    def __init__(self):
        self.rbfn = None
        
    def RBFtraining(self, srcROE, tgtROE) :
        if srcROE.shape[0] != tgtROE.shape[0] :
            print "ERROR: ROE size of source and target is different!"
            return        
        self.rbfn = RBF()
        self.rbfn.setBasisFunction('HARDY')
        self.rbfn.setLamda(0.1)
        self.rbfn.train(srcROE, tgtROE)
    
    def RBFrunning(self, srcAnimDataList):
        resultAnimDataList = []
        for srcAnimData in srcAnimDataList :
            result = self.rbfn.interpolate(srcAnimData)
            resultAnimDataList.append(result)
        return resultAnimDataList
        
        
#        
#    def RBF_interpolate(self, srcInput, trainData, numTargetCV):
#        rSrcInput = np.zeros(trainData.rbfTrain.rbfn.numInput)
#        for i in range(len(rSrcInput)) :
#            rSrcInput[i] = self.basisFunc(i, self.dist(srcInput, trainData.rbfTrain.rbfn.inputMat[i]))
#        resultMat = np.dot(rSrcInput, trainData.weightMat)
#        result = np.zeros(numTargetCV)        
#        for i in range(numTargetCV) :            
#            result[i] = resultMat[i]        
#        return result
#    
#    def CFR_running(self, srcAnimData, tgtCharData, trainData):
#        numTargetCV = tgtCharData.numCV
#        resultAnimDataList = []
#        for i in range(len(srcAnimData.animDataList)):
#            out = self.RBF_interpolate(srcAnimData.animDataList[i], trainData, numTargetCV)
#            resultAnimDataList.append(out)
#        return resultAnimDataList
          
        

```

## Source file: directMarkerUI.py

{
  "path": "directMarkerUI.py",
  "blob_sha": "cca21bead3dca2d09782c96c0d0d210fee55cc7e",
  "sha256": "a6c4d3661216bf5e5c816fba205eaf6d55353df872e75f1b275525481d1684c6",
  "lines": 612,
  "bytes": 24818,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 29,
    "face": 13
  },
  "url": "https://github.com/JaewonSongJ1/DirectFacialRetargeting/blob/f10456e723b62dd735d64efd9ff5218877fc1a35/directMarkerUI.py",
  "function_signature_candidates": [
    {
      "line": 24,
      "text": "def __init__( self, A, fraction=0.90 ):"
    },
    {
      "line": 36,
      "text": "def pc( self ):"
    },
    {
      "line": 44,
      "text": "def vars_pc( self, x ):"
    },
    {
      "line": 48,
      "text": "def pc_vars( self, p ):"
    },
    {
      "line": 52,
      "text": "def pc_obs( self, p ):"
    },
    {
      "line": 56,
      "text": "def obs_pc( self, obs ):"
    },
    {
      "line": 60,
      "text": "def obs( self, x ):"
    },
    {
      "line": 63,
      "text": "def vars( self, obs ):"
    },
    {
      "line": 72,
      "text": "def __init__( self, A, axis=0, scale=True, verbose=1 ):"
    },
    {
      "line": 87,
      "text": "def uncenter( self, x ):"
    },
    {
      "line": 91,
      "text": "def __init__(self, markerList, frameRange):"
    },
    {
      "line": 98,
      "text": "def addMarkerTrainData(self, frame):"
    },
    {
      "line": 101,
      "text": "def removeMarkerTrainData(self, idx):"
    },
    {
      "line": 104,
      "text": "def getMarkerData(self):"
    },
    {
      "line": 121,
      "text": "def __init__(self, ctrlList):"
    },
    {
      "line": 127,
      "text": "def getCtrlAttrList(self):"
    },
    {
      "line": 133,
      "text": "def setCtrlAttrList(self, ctrlAttrValList):"
    },
    {
      "line": 138,
      "text": "def addRigTrainData(self):"
    },
    {
      "line": 145,
      "text": "def removeRigTrainData(self, idx):"
    },
    {
      "line": 151,
      "text": "def editRigTrainData(self, idx):"
    },
    {
      "line": 161,
      "text": "def __init__(self, markerList, faceRigCtrlList, frameRange):"
    },
    {
      "line": 166,
      "text": "def editTrainData(self, frame):"
    },
    {
      "line": 170,
      "text": "def addTrainData(self):"
    },
    {
      "line": 175,
      "text": "def removeTrainData(self, idx):"
    },
    {
      "line": 181,
      "text": "def running(self):"
    },
    {
      "line": 197,
      "text": "def __init__(self):"
    },
    {
      "line": 213,
      "text": "def loadUiWidget(self, uifilename):"
    },
    {
      "line": 221,
      "text": "def connectSignals(self):"
    },
    {
      "line": 235,
      "text": "def ui_ctrlListWidgetItemClicked(self, item):"
    },
    {
      "line": 238,
      "text": "def ui_markerListWidgetItemClicked(self, item):"
    },
    {
      "line": 241,
      "text": "def ui_RetargetingBtnClicked(self):"
    },
    {
      "line": 244,
      "text": "def ui_TrainingSetNeutralBtnClicked(self):"
    },
    {
      "line": 250,
      "text": "def ui_TrainingSliderValueChanged(self, val):"
    },
    {
      "line": 258,
      "text": "def ui_MarkerSetBtnClicked(self):"
    },
    {
      "line": 263,
      "text": "def ui_MarkerResetBtnClicked(self):"
    },
    {
      "line": 267,
      "text": "def ui_CtrlSetBtnClicked(self):"
    },
    {
      "line": 272,
      "text": "def ui_CtrlResetBtnClicked(self):"
    },
    {
      "line": 276,
      "text": "def ui_ReadyForTrainingBtnClicked(self):"
    },
    {
      "line": 292,
      "text": "def ui_TrainingAddBtnClicked(self):"
    },
    {
      "line": 308,
      "text": "def ui_TrainingRemoveBtnClicked(self):"
    },
    {
      "line": 329,
      "text": "def ui_trainingPartEnable(self):"
    },
    {
      "line": 339,
      "text": "def ui_trainingPartDisable(self):"
    },
    {
      "line": 351,
      "text": "def __init__(self, tabWidget, UIInatance):"
    },
    {
      "line": 373,
      "text": "def okBtnClicked(self):"
    },
    {
      "line": 379,
      "text": "def connections(self):"
    },
    {
      "line": 386,
      "text": "def __init__(self, tabWidget):"
    },
    {
      "line": 407,
      "text": "def renameBtnClicked(self):"
    },
    {
      "line": 411,
      "text": "def connections(self):"
    },
    {
      "line": 418,
      "text": "def __init__(self):"
    },
    {
      "line": 433,
      "text": "def getMayaWindow():"
    }
  ]
}

```text

from __future__ import division
#from PySide import QtCore, QtGui, QtUiTools
#import shiboken
import shiboken2
from shiboken2 import wrapInstance
from PySide2 import QtCore, QtGui, QtUiTools, QtWidgets
import maya.OpenMayaUI as apiUI
import os
import pickle

import maya.cmds as mc
import numpy as np
import sys
currentPath = '/home/d10441/PycharmProjects/DirectFacialRetargeting'
sys.path.append(currentPath)
import CFR_training
reload(CFR_training)
from CFR_training import *
dot = np.dot
DEBUG=0

#...............................................................................
class PCA:
    def __init__( self, A, fraction=0.90 ):
        assert 0 <= fraction <= 1
            # A = U . diag(d) . Vt, O( m n^2 ), lapack_lite --
        self.U, self.d, self.Vt = np.linalg.svd( A, full_matrices=False )
        assert np.all( self.d[:-1] >= self.d[1:] )  # sorted
        self.eigen = self.d**2
        self.sumvariance = np.cumsum(self.eigen)
        self.sumvariance /= self.sumvariance[-1]
        self.npc = np.searchsorted( self.sumvariance, fraction ) + 1
        self.dinv = np.array([ 1/d if d > self.d[0] * 1e-6  else 0
                                for d in self.d ])

    def pc( self ):
        """ e.g. 1000 x 2 U[:, :npc] * d[:npc], to plot etc. """
        n = self.npc
        return self.U[:, :n] * self.d[:n]

    # These 1-line methods may not be worth the bother;
    # then use U d Vt directly --

    def vars_pc( self, x ):
        n = self.npc
        return self.d[:n] * dot( self.Vt[:n], x.T ).T  # 20 vars -> 2 principal

    def pc_vars( self, p ):
        n = self.npc
        return dot( self.Vt[:n].T, (self.dinv[:n] * p).T ) .T  # 2 PC -> 20 vars

    def pc_obs( self, p ):
        n = self.npc
        return dot( self.U[:, :n], p.T )  # 2 principal -> 1000 obs

    def obs_pc( self, obs ):
        n = self.npc
        return dot( self.U[:, :n].T, obs ) .T  # 1000 obs -> 2 principal

    def obs( self, x ):
        return self.pc_obs( self.vars_pc(x) )  # 20 vars -> 2 principal -> 1000 obs

    def vars( self, obs ):
        return self.pc_vars( self.obs_pc(obs) )  # 1000 obs -> 2 principal -> 20 vars


class Center:
    """ A -= A.mean() /= A.std(), inplace -- use A.copy() if need be
        uncenter(x) == original A . x
    """
        # mttiw
    def __init__( self, A, axis=0, scale=True, verbose=1 ):
        self.mean = A.mean(axis=axis)
        if verbose:
            if DEBUG==1 : print "Center -= A.mean:", self.mean
        A -= self.mean
        if scale:
            std = A.std(axis=axis)
            self.std = np.where( std, std, 1. )
            if verbose:
                if DEBUG==1 : print "Center /= A.std:", self.std
            A /= self.std
        else:
            self.std = np.ones( A.shape[-1] )
        self.A = A

    def uncenter( self, x ):
        return np.dot( self.A, x * self.std ) + np.dot( x, self.mean )

class MarkerData():
    def __init__(self, markerList, frameRange):
        self.markerList = markerList
        self.frameRange = frameRange
        self.markerDataList = []
        #self.getMarkerData()
        self.markerTrainDataList = []

    def addMarkerTrainData(self, frame):
        self.markerTrainDataList.append(self.markerDataList[frame-1])

    def removeMarkerTrainData(self, idx):
        self.markerTrainDataList.pop(idx)

    def getMarkerData(self):
        rawMarkerData = []
        for t in range(self.frameRange[0], self.frameRange[1]+1) :
            tmpList = []
            for marker in self.markerList :
                tmpList = tmpList + list(mc.getAttr(marker+'.translate', t=t)[0])
                #mc.currentTime(t)
                #tmpList = tmpList + mc.xform(marker, q=True, t=True, ws=True)
            rawMarkerData.append(tmpList)
        rawMarkerData = np.array(rawMarkerData)
        Center(rawMarkerData)
        fraction = .95
        p = PCA(rawMarkerData, fraction=fraction)
        self.markerDataList = p.pc()


class FaceRig():
    def __init__(self, ctrlList):
        self.ctrlList = ctrlList
        self.ctrlAttrList = []
        self.rigTrainDataList = []
        self.getCtrlAttrList()

    def getCtrlAttrList(self):
        for ctrl in self.ctrlList :
            attrList = mc.listAttr(ctrl, k=True)
            for attr in attrList :
                self.ctrlAttrList.append(ctrl+'.'+attr)

    def setCtrlAttrList(self, ctrlAttrValList):
        for i, ctrlAttr in enumerate(self.ctrlAttrList):
            mc.setAttr(ctrlAttr, ctrlAttrValList[i])
            mc.setKeyframe(ctrlAttr)

    def addRigTrainData(self):
        tmpList = []
        for ctrlAttr in self.ctrlAttrList :
            tmpList.append(mc.getAttr(ctrlAttr))
            mc.setKeyframe(ctrlAttr)
        self.rigTrainDataList.append(tmpList)

    def removeRigTrainData(self, idx):
        for ctr in self.ctrlAttrList :
            f = mc.currentTime(q=True)
            mc.cutKey(ctr, t=(f,f))
        self.rigTrainDataList.pop(idx)

    def editRigTrainData(self, idx):
        tmpList = []
        for ctrlAttr in self.ctrlAttrList :
            tmpList.append(mc.getAttr(ctrlAttr))
            mc.setKeyframe(ctrlAttr)
        self.rigTrainDataList[idx] = tmpList



class CFR_MarkerDirect():
    def __init__(self, markerList, faceRigCtrlList, frameRange):
        self.marker = MarkerData(markerList, frameRange)
        self.faceRig = FaceRig(faceRigCtrlList)
        self.trainingFrameList = []

    def editTrainData(self, frame):
        idx = self.trainingFrameList.index(frame)
        self.faceRig.editRigTrainData(idx)

    def addTrainData(self):
        self.marker.addMarkerTrainData(mc.currentTime(q=True))
        self.faceRig.addRigTrainData()
        self.trainingFrameList.append(mc.currentTime(q=True))

    def removeTrainData(self, idx):
        self.marker.removeMarkerTrainData(idx)
        self.faceRig.removeRigTrainData(idx)
        self.trainingFrameList.pop(idx)


    def running(self):
        """
        RBF Training
        """
        rbfTrain = RBFtrain()
        markerDataArr = np.array(self.marker.markerTrainDataList)
        ctrlDataArr = np.array(self.faceRig.rigTrainDataList)
        rbfTrain.RBFtraining(markerDataArr, ctrlDataArr)
        result = rbfTrain.RBFrunning(self.marker.markerDataList)
        for f, data in enumerate(result):
            #mc.currentTime(f)
            for i, ctrlAttr in enumerate(self.faceRig.ctrlAttrList) :
                mc.setAttr(ctrlAttr, data[i])
                mc.setKeyframe(ctrlAttr, t=f+1)

class DirectRetargetingWidget(QtWidgets.QFrame):
    def __init__(self):
        QtWidgets.QFrame.__init__(self)
        self.setFrameStyle(QtWidgets.QFrame.Panel | QtWidgets.QFrame.Raised)
        self.setLayout(QtWidgets.QVBoxLayout())
        self.widgetuiFilePath = currentPath+'/ui/directFacialRetargetingWidget.ui'
        self.widgetUI = self.loadUiWidget(self.widgetuiFilePath)
        self.styleFilePath = currentPath+'/stylesheet/widgetScheme.qss'
        with open(self.styleFilePath, 'r') as file:
            style_sheet = file.read()
        self.setStyleSheet(style_sheet)
        self.Instance = None
        self.MarkerList = []
        self.CtrlList = []
        self.connectSignals()


    def loadUiWidget(self, uifilename):
        loader = QtUiTools.QUiLoader()
        uifile = QtCore.QFile(uifilename)
        uifile.open(QtCore.QFile.ReadOnly)
        ui = loader.load(uifile, self)
        uifile.close()
        return ui

    def connectSignals(self):
        self.widgetUI.ui_MarkerSetBtn.clicked.connect(self.ui_MarkerSetBtnClicked)
        self.widgetUI.ui_MarkerResetBtn.clicked.connect(self.ui_MarkerResetBtnClicked)
        self.widgetUI.ui_CtrlSetBtn.clicked.connect(self.ui_CtrlSetBtnClicked)
        self.widgetUI.ui_CtrlResetBtn.clicked.connect(self.ui_CtrlResetBtnClicked)
        self.widgetUI.ui_ReadyForTrainingBtn.clicked.connect(self.ui_ReadyForTrainingBtnClicked)
        self.widgetUI.ui_TrainingAddBtn.clicked.connect(self.ui_TrainingAddBtnClicked)
        self.widgetUI.ui_TrainingSlider.valueChanged.connect(self.ui_TrainingSliderValueChanged)
        self.widgetUI.ui_TrainingRemoveBtn.clicked.connect(self.ui_TrainingRemoveBtnClicked)
        self.widgetUI.ui_TrainingSetNeutralBtn.clicked.connect(self.ui_TrainingSetNeutralBtnClicked)
        self.widgetUI.ui_RetargetingBtn.clicked.connect(self.ui_RetargetingBtnClicked)
        self.widgetUI.ui_markerListWidget.itemClicked.connect(self.ui_markerListWidgetItemClicked)
        self.widgetUI.ui_ctrlListWidget.itemClicked.connect(self.ui_ctrlListWidgetItemClicked)

    def ui_ctrlListWidgetItemClicked(self, item):
        mc.select(item.text(), r=True)

    def ui_markerListWidgetItemClicked(self, item):
        mc.select(item.text(), r=True)

    def ui_RetargetingBtnClicked(self):
        self.Instance.running()

    def ui_TrainingSetNeutralBtnClicked(self):
        attrList = self.Instance.faceRig.ctrlAttrList
        neutralValList = self.Instance.faceRig.rigTrainDataList[0]
        for i, attr in enumerate(attrList):
            mc.setAttr(attr, neutralValList[i])

    def ui_TrainingSliderValueChanged(self, val):
        if len(self.Instance.trainingFrameList)>0 :
            mc.currentTime(self.Instance.trainingFrameList[val-1])
        else:
            return
        #print self.Instance.faceRig.rigTrainDataList


    def ui_MarkerSetBtnClicked(self):
        self.MarkerList = mc.ls(sl=True)
        for marker in self.MarkerList :
            self.widgetUI.ui_markerListWidget.addItem(marker)

    def ui_MarkerResetBtnClicked(self):
        self.MarkerList = []
        self.widgetUI.ui_markerListWidget.clear()

    def ui_CtrlSetBtnClicked(self):
        self.CtrlList = mc.ls(sl=True)
        for ctrl in self.CtrlList :
            self.widgetUI.ui_ctrlListWidget.addItem(ctrl)

    def ui_CtrlResetBtnClicked(self):
        self.CtrlList = []
        self.widgetUI.ui_ctrlListWidget.clear()

    def ui_ReadyForTrainingBtnClicked(self):
        if len(self.MarkerList)==0 :
            print 'ERROR: No  Marker List'
            return
        if len(self.CtrlList)==0:
            print 'ERROR: No  Controller List'
            return
        startFrame = self.widgetUI.ui_StartFrameSpinBox.value()
        endFrame = self.widgetUI.ui_EndFrameSpinBox.value()
        if endFrame-startFrame <= 0 :
            print 'ERROR: Invalid Frame Range'
            return
        self.Instance = CFR_MarkerDirect(self.MarkerList, self.CtrlList, [startFrame, endFrame])
        self.Instance.marker.getMarkerData()
        self.ui_trainingPartEnable()

    def ui_TrainingAddBtnClicked(self):
        if mc.currentTime(q=True) in self.Instance.trainingFrameList :
            self.Instance.editTrainData(mc.currentTime(q=True))
        else:
            self.Instance.addTrainData()
            self.widgetUI.ui_TrainingSlider.setMinimum(1)
            self.widgetUI.ui_TrainingSlider.setMaximum(len(self.Instance.trainingFrameList))
            self.widgetUI.ui_TrainingSlider.setValue(len(self.Instance.trainingFrameList))
            self.widgetUI.ui_TrainingSpinBox.setMinimum(1)
            self.widgetUI.ui_TrainingSpinBox.setMaximum(len(self.Instance.trainingFrameList))
            self.widgetUI.ui_TrainingSpinBox.setValue(len(self.Instance.trainingFrameList))
            self.widgetUI.ui_TrainingTotalExampleNumberLabel.setText('/'+str(len(self.Instance.trainingFrameList)))

        if len(self.Instance.trainingFrameList) >= 2:
            self.widgetUI.ui_RetargetingBtn.setEnabled(True)

    def ui_TrainingRemoveBtnClicked(self):
        idx = self.widgetUI.ui_TrainingSlider.value()
        if idx < 1:
            print 'ERORR: Cannot Remove Anything'
        elif idx == 1:
            self.Instance.removeTrainData(idx-1)
            self.widgetUI.ui_TrainingSlider.setMaximum(len(self.Instance.trainingFrameList))
            self.widgetUI.ui_TrainingSpinBox.setMaximum(len(self.Instance.trainingFrameList))
            self.widgetUI.ui_TrainingSlider.setMinimum(0)
            self.widgetUI.ui_TrainingSpinBox.setMinimum(0)
            self.widgetUI.ui_TrainingTotalExampleNumberLabel.setText('/0')
        else:
            self.Instance.removeTrainData(idx-1)
            self.widgetUI.ui_TrainingSlider.setMaximum(len(self.Instance.trainingFrameList))
            self.widgetUI.ui_TrainingSpinBox.setMaximum(len(self.Instance.trainingFrameList))
            self.widgetUI.ui_TrainingTotalExampleNumberLabel.setText('/'+str(len(self.Instance.trainingFrameList)))

        if len(self.Instance.trainingFrameList) <= 1:
            self.widgetUI.ui_RetargetingBtn.setEnabled(False)


    def ui_trainingPartEnable(self):
        self.widgetUI.ui_TrainingTitleLabel.setEnabled(True)
        self.widgetUI.ui_TrainingSlider.setEnabled(True)
        self.widgetUI.ui_TrainingSpinBox.setEnabled(True)
        self.widgetUI.ui_TrainingTotalExampleNumberLabel.setEnabled(True)
        self.widgetUI.ui_TrainingAddBtn.setEnabled(True)
        self.widgetUI.ui_TrainingRemoveBtn.setEnabled(True)
        self.widgetUI.ui_TrainingSetNeutralBtn.setEnabled(True)
        self.widgetUI.ui_TrainingNoticeLabel.setEnabled(True)

    def ui_trainingPartDisable(self):
        self.widgetUI.ui_TrainingTitleLabel.setEnabled(False)
        self.widgetUI.ui_TrainingSlider.setEnabled(False)
        self.widgetUI.ui_TrainingSpinBox.setEnabled(False)
        self.widgetUI.ui_TrainingTotalExampleNumberLabel.setEnabled(False)
        self.widgetUI.ui_TrainingAddBtn.setEnabled(False)
        self.widgetUI.ui_TrainingRemoveBtn.setEnabled(False)
        self.widgetUI.ui_TrainingSetNeutralBtn.setEnabled(False)
        self.widgetUI.ui_TrainingNoticeLabel.setEnabled(False)


class TabDeleteDialog(QtWidgets.QDialog):
    def __init__(self, tabWidget, UIInatance):
        super(TabDeleteDialog, self).__init__(parent=tabWidget)
        global tabDeleteDialog
        try:
            tabDeleteDialog.close()
            tabDeleteDialog.deleteLater()
        except: pass

        self.setWindowTitle('Delete Tab')
        self.tabWidget = tabWidget
        self.UIInatance = UIInatance

        self.warningLabel = QtWidgets.QLabel('Are you sure to delete current tab?')
        self.okBtn = QtWidgets.QPushButton('OK', parent=self)
        self.closeBtn = QtWidgets.QPushButton('Close', parent=self)

        self.layout = QtWidgets.QBoxLayout(QtWidgets.QBoxLayout.LeftToRight, self)
        self.layout.addWidget(self.warningLabel)
        self.layout.addWidget(self.okBtn)
        self.layout.addWidget(self.closeBtn)
        self.connections()

    def okBtnClicked(self):
        self.UIInatance.DirectRetargetingWidgetList.pop(self.tabWidget.currentIndex())
        self.tabWidget.removeTab(self.tabWidget.currentIndex())
        self.close()


    def connections(self):
        self.okBtn.clicked.connect(self.okBtnClicked)
        self.closeBtn.clicked.connect(self.close)



class TabRenameDialog(QtWidgets.QDialog):
    def __init__(self, tabWidget):
        super(TabRenameDialog, self).__init__(parent=tabWidget)
        global tabRenameDialog
        try:
            tabRenameDialog.close()
            tabRenameDialog.deleteLater()
        except: pass

        self.setWindowTitle('Rename Tab')
        self.tabWidget = tabWidget

        self.nameLE = QtWidgets.QLineEdit('NewName', parent=self)
        self.renameBtn = QtWidgets.QPushButton('Rename', parent=self)
        self.closeBtn = QtWidgets.QPushButton('Close', parent=self)

        self.layout = QtWidgets.QBoxLayout(QtWidgets.QBoxLayout.LeftToRight, self)
        self.layout.addWidget(self.nameLE)
        self.layout.addWidget(self.renameBtn)
        self.layout.addWidget(self.closeBtn)
        self.connections()

    def renameBtnClicked(self):
        self.tabWidget.setTabText(self.tabWidget.currentIndex(), self.nameLE.text())
        self.close()

    def connections(self):
        self.renameBtn.clicked.connect(self.renameBtnClicked)
        self.closeBtn.clicked.connect(self.close)



class DirectRetargetingWindow(QtWidgets.QDialog):
    def __init__(self):
        QtWidgets.QDialog.__init__(self)
        self.uiFilePath = currentPath+'/ui/tabWidget.ui'
        self.MainWindow = None
        self.setObjectName('DirectFacialRetargetingUI')
        self.DirectRetargetingWidgetList = []
        self._dock_widget = self._dock_name = None
        self.setLayout(QtWidgets.QVBoxLayout())
        self.styleFilePath = currentPath+'/stylesheet/dockUIScheme.qss'
        #self.styleFilePath.open(QtCore.QFile.ReadOnly)
        with open(self.styleFilePath, 'r') as file:
            style_sheet = file.read()
        self.setStyleSheet(style_sheet)


    def getMayaWindow():
        ptr = apiUI.MQtUtil.mainWindow()
        if ptr is not None:
            return shiboken2.wrapInstance(long(ptr), QtWidgets.QWidget)


    def loadUiWidget(self, uifilename, parent=getMayaWindow()):
        loader = QtUiTools.QUiLoader()
        uifile = QtCore.QFile(uifilename)
        uifile.open(QtCore.QFile.ReadOnly)
        ui = loader.load(uifile, parent)
        uifile.close()
        return ui

    def connectSignals(self):
        self.MainWindow.ui_LoadTrainingDataBtn.clicked.connect(self.ui_LoadTrainingDataBtnClicked)
        self.MainWindow.ui_SaveTrainingDataBtn.clicked.connect(self.ui_SaveTrainingDataBtnClicked)
        self.MainWindow.ui_CreateNewTabBtn.clicked.connect(self.ui_CreateNewTabBtnClicked)
        self.MainWindow.tabWidget.customContextMenuRequested.connect(self.tabWidgetContextMenu)


    def tabWidgetContextMenu(self, pos):
        if self.MainWindow.tabWidget.currentWidget() == None :
            return
        contextMenu = QtWidgets.QMenu(self)
        action1 = contextMenu.addAction('Rename Tab')
        action2 = contextMenu.addAction('Delete Tab')
        action1.triggered.connect(self.popupAction_renameTab)
        action2.triggered.connect(self.popupAction_deleteTab)
        contextMenu.exec_(self.MainWindow.tabWidget.currentWidget().mapToGlobal(pos))


    def popupAction_renameTab(self):
        renameDialog = TabRenameDialog(self.MainWindow.tabWidget)
        renameDialog.show()

    def popupAction_deleteTab(self):
        deleteTabDialog = TabDeleteDialog(self.MainWindow.tabWidget, self)
        deleteTabDialog.show()

    def ui_CreateNewTabBtnClicked(self):
        new_widget = DirectRetargetingWidget()
        self.DirectRetargetingWidgetList.append(new_widget)
        tab = QtWidgets.QWidget()
        vLayout = QtWidgets.QVBoxLayout(tab)
        vLayout.setContentsMargins(5,5,5,5)
        vLayout.addWidget(new_widget)
        self.MainWindow.tabWidget.addTab(tab, self.MainWindow.ui_CharacterNameLE.text())
        self.MainWindow.tabWidget.setCurrentIndex(self.MainWindow.tabWidget.count()-1)

    def ui_SaveTrainingDataBtnClicked(self):
        saveInstance = SaveLoadFileData()
        for i in range(self.MainWindow.tabWidget.count()):
            saveInstance.tabList.append(self.MainWindow.tabWidget.tabText(i))

        #print 'MarkeRList : ', self.DirectRetargetingWidgetList[0].MarkerList

        for i, DirectRetargetingWidget in enumerate(self.DirectRetargetingWidgetList) :
            saveInstance.markerListDic[saveInstance.tabList[i]] = DirectRetargetingWidget.MarkerList
            saveInstance.ctrlListDic[saveInstance.tabList[i]] = DirectRetargetingWidget.CtrlList
            saveInstance.trainingFrameListDic[saveInstance.tabList[i]] = DirectRetargetingWidget.Instance.trainingFrameList
            saveInstance.frameRangeDic[saveInstance.tabList[i]] = [DirectRetargetingWidget.widgetUI.ui_StartFrameSpinBox.value(), DirectRetargetingWidget.widgetUI.ui_EndFrameSpinBox.value()]
            saveInstance.markerDataListDic[saveInstance.tabList[i]] = DirectRetargetingWidget.Instance.marker.markerTrainDataList
            saveInstance.rigTrainingDataListDic[saveInstance.tabList[i]] = DirectRetargetingWidget.Instance.faceRig.rigTrainDataList

        filePath = mc.fileDialog2(okCaption='Data Save', ff='*.txt', fm=0)
        f = open(filePath[0], 'w')
        pickle.dump(saveInstance, f)
        f.close()

    def ui_LoadTrainingDataBtnClicked(self):
        filePath = mc.fileDialog2(okCaption='Data Load', ff='*.txt', fm=1)[0]
        self.loadTrainingFile(filePath)

    def loadTrainingFile(self, filePath):
        f = open(filePath)
        loadInstance = pickle.load(f)
        f.close()

        tabList = loadInstance.tabList

        for tabName in tabList :
            new_widget = DirectRetargetingWidget()
            markerList = loadInstance.markerListDic[tabName]
            ctrlList = loadInstance.ctrlListDic[tabName]
            trainingFrameList = loadInstance.trainingFrameListDic[tabName]
            markerDataList = loadInstance.markerDataListDic[tabName]
            rigTrainDataList = loadInstance.rigTrainingDataListDic[tabName]
            frameRange = loadInstance.frameRangeDic[tabName]

            new_widget.Instance = CFR_MarkerDirect(markerList, ctrlList, [frameRange[0], frameRange[1]])
            new_widget.Instance.marker.frameRange = frameRange
            new_widget.Instance.marker.markerList = markerList
            new_widget.Instance.marker.getMarkerData()
            #new_widget.Instance.marker.markerDataList = markerDataList
            new_widget.Instance.marker.markerTrainDataList = markerDataList
            new_widget.Instance.faceRig.rigTrainDataList = rigTrainDataList
            new_widget.Instance.trainingFrameList = trainingFrameList

            for marker in markerList :
                new_widget.widgetUI.ui_markerListWidget.addItem(marker)
            for ctrl in ctrlList :
                new_widget.widgetUI.ui_ctrlListWidget.addItem(ctrl)

            new_widget.widgetUI.ui_StartFrameSpinBox.setValue(frameRange[0])
            new_widget.widgetUI.ui_EndFrameSpinBox.setValue(frameRange[1])
            new_widget.ui_trainingPartEnable()
            new_widget.widgetUI.ui_TrainingSlider.setMinimum(1)
            new_widget.widgetUI.ui_TrainingSlider.setMaximum(len(new_widget.Instance.trainingFrameList))
            new_widget.widgetUI.ui_TrainingSlider.setValue(len(new_widget.Instance.trainingFrameList))
            new_widget.widgetUI.ui_TrainingSpinBox.setMinimum(1)
            new_widget.widgetUI.ui_TrainingSpinBox.setMaximum(len(new_widget.Instance.trainingFrameList))
            new_widget.widgetUI.ui_TrainingSpinBox.setValue(len(new_widget.Instance.trainingFrameList))
            new_widget.widgetUI.ui_TrainingTotalExampleNumberLabel.setText('/'+str(len(new_widget.Instance.trainingFrameList)))

            if len(new_widget.Instance.trainingFrameList) >= 2:
                new_widget.widgetUI.ui_RetargetingBtn.setEnabled(True)


            self.DirectRetargetingWidgetList.append(new_widget)

            tab = QtWidgets.QWidget()
            vLayout = QtWidgets.QVBoxLayout(tab)
            vLayout.setContentsMargins(5,5,5,5)
            vLayout.addWidget(new_widget)
            self.MainWindow.tabWidget.addTab(tab, tabName)
            self.MainWindow.tabWidget.setCurrentIndex(self.MainWindow.tabWidget.count()-1)


    def connectDockWidget(self, dock_name, dock_widget):
        self._dock_widget = dock_widget
        self._dock_name = dock_name

    def close(self):
        if self._dock_widget:
            mc.deleteUI(self._dock_name)
        else:
            QtWidgets.QDialog.close(self)
        self._dock_widget = self._dock_name = None

class SaveLoadFileData():
    def __init__(self):
        self.tabList = []
        self.markerListDic = {}
        self.ctrlListDic = {}
        self.trainingFrameListDic = {}
        self.markerDataListDic = {}
        self.rigTrainingDataListDic = {}
        self.frameRangeDic = {}


def main(docked=True):
    global PyForm
    PyForm=DirectRetargetingWindow()

    if docked is True:
        PyForm.close()
        ptr = apiUI.MQtUtil.mainWindow()
        if ptr is not None:
            main_window = shiboken2.wrapInstance(long(ptr), QtWidgets.QWidget)
        PyForm.MainWindow = PyForm.loadUiWidget(PyForm.uiFilePath)
        PyForm.layout().addWidget(PyForm.MainWindow)
        PyForm.connectSignals()
        PyForm.setParent(main_window)
        size = PyForm.size()

        name = apiUI.MQtUtil.fullName(long(shiboken2.getCppPointer(PyForm)[0]))
        dock = mc.dockControl(
            allowedArea = ['right', 'left'],
            area = 'right',
            floating = False,
            content = name,
            width = size.width(),
            height = size.height(),
            label = 'Direct Facial Retargeting UI')

        widget = apiUI.MQtUtil.findControl(dock)
        dock_widget = shiboken2.wrapInstance(long(widget), QtCore.QObject)
        PyForm.connectDockWidget(dock, dock_widget)
    PyForm.show()

```

## Source file: seqImgViewerUI.py

{
  "path": "seqImgViewerUI.py",
  "blob_sha": "b041119fa1c564851ddbc3abcfdde6fff88e9262",
  "sha256": "9abf0ec87de7e2261fa6762716800c7bf7ea7fb7d2f6973b6aa0176904d165e8",
  "lines": 99,
  "bytes": 3626,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 4
  },
  "url": "https://github.com/JaewonSongJ1/DirectFacialRetargeting/blob/f10456e723b62dd735d64efd9ff5218877fc1a35/seqImgViewerUI.py",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "def getMayaWindow():"
    },
    {
      "line": 16,
      "text": "def __init__(self):"
    },
    {
      "line": 62,
      "text": "def mayaTimeSliderChanged(self, msg):"
    },
    {
      "line": 66,
      "text": "def startLEchanged(self):"
    },
    {
      "line": 72,
      "text": "def endLEchanged(self):"
    },
    {
      "line": 79,
      "text": "def frameSliderChanged(self):"
    },
    {
      "line": 88,
      "text": "def show(self):"
    },
    {
      "line": 91,
      "text": "def close(self):"
    }
  ]
}

```text

__author__ = 'cimple'
from PySide2 import QtGui
from PySide2 import QtCore
import maya.OpenMaya as om
import maya.OpenMayaUI as mui
import maya.cmds as mc
import shiboken


def getMayaWindow():
    pointer = mui.MQtUtil.mainWindow()
    return shiboken.wrapInstance(long(pointer), QtGui.QWidget)


class ImgUI():
    def __init__(self):

        ##ui
        self.parent = getMayaWindow()
        self.window = QtGui.QMainWindow(self.parent)
        self.widget = QtGui.QWidget()
        self.window.setCentralWidget(self.widget)
        self.layout = QtGui.QVBoxLayout(self.widget)

        self.imgLabel = QtGui.QLabel("<img src = '/home/d10441/PycharmProjects/DirectFacialRetargeting/example/imgseq/faceSeq000.png'>")
        self.frameSlider = QtGui.QSlider()
        self.frameSlider.setOrientation(QtCore.Qt.Horizontal)
        self.frameSpinBox = QtGui.QSpinBox()
        self.startLE = QtGui.QLineEdit()
        self.endLE = QtGui.QLineEdit()

        self.startLE.setMaximumWidth(40)
        self.endLE.setMaximumWidth(40)
        self.startLE.setText('1')
        self.endLE.setText('730')
        self.frameSlider.setMinimum(int(self.startLE.text()))
        self.frameSlider.setMaximum(int(self.endLE.text()))
        self.frameSpinBox.setMinimum(int(self.startLE.text()))
        self.frameSpinBox.setMaximum(int(self.endLE.text()))
        self.startFrame = 1
        self.endFrame = 730

        ##layout
        self.layout.addWidget(self.imgLabel)
        self.sliderLayout = QtGui.QHBoxLayout()
        self.layout.addLayout(self.sliderLayout)
        self.sliderLayout.addWidget(self.startLE)
        self.sliderLayout.addWidget(self.frameSlider)
        self.sliderLayout.addWidget(self.frameSpinBox)
        self.sliderLayout.addWidget(self.endLE)

        ##connect
        self.frameSlider.valueChanged.connect(self.frameSpinBox.setValue)
        self.frameSpinBox.valueChanged.connect(self.frameSlider.setValue)
        self.frameSlider.valueChanged.connect(self.frameSliderChanged)
        self.startLE.textChanged.connect(self.startLEchanged)
        self.endLE.textChanged.connect(self.endLEchanged)

        self.frameCallback = om.MEventMessage.addEventCallback('timeChanged', self.mayaTimeSliderChanged)


    def mayaTimeSliderChanged(self, msg):
        currentTime = int(mc.currentTime(q=True))
        self.frameSlider.setValue(self.startFrame+currentTime-1)

    def startLEchanged(self):
        startVal = int(self.startLE.text())
        self.frameSlider.setMinimum(startVal)
        self.frameSpinBox.setMinimum(int(self.startLE.text()))
        self.startFrame = startVal

    def endLEchanged(self):
        endVal = int(self.endLE.text())
        self.frameSlider.setMaximum(endVal)
        self.frameSpinBox.setMaximum(int(self.endLE.text()))
        self.endFrame = endVal


    def frameSliderChanged(self):
        value = self.frameSlider.value()
        if value < 10 :
            self.imgLabel.setText("<img src = '/home/d10441/PycharmProjects/DirectFacialRetargeting/example/imgseq/faceSeq00"+str(value)+".png'>")
        elif value >= 10 and value < 100 :
            self.imgLabel.setText("<img src = '/home/d10441/PycharmProjects/DirectFacialRetargeting/example/imgseq/faceSeq0"+str(value)+".png'>")
        elif value >= 100 :
            self.imgLabel.setText("<img src = '/home/d10441/PycharmProjects/DirectFacialRetargeting/example/imgseq/faceSeq"+str(value)+".png'>")

    def show(self):
        self.window.show()

    def close(self):
        self.window.close()
        om.MEventMessage.removeCallback(self.frameCallback)



img = ImgUI()
img.show()
#img.close()


```

## Source file: headRotationRemoval.py

{
  "path": "headRotationRemoval.py",
  "blob_sha": "bf1735490f4ecf25f0b5cbace3aa32d55c1ef58b",
  "sha256": "a789a0c9daa5b0853f90f5a12914f17861cf86a2e30cb42da1579efe1cc2056e",
  "lines": 78,
  "bytes": 6629,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 2,
    "mouth": 2,
    "rig": 2
  },
  "url": "https://github.com/JaewonSongJ1/DirectFacialRetargeting/blob/f10456e723b62dd735d64efd9ff5218877fc1a35/headRotationRemoval.py",
  "function_signature_candidates": [
    {
      "line": 18,
      "text": "def __init__(self, totalMarkerList, staticMarkerList, referenceFrame):"
    },
    {
      "line": 23,
      "text": "def get_standardized_matrix(self, mat):"
    },
    {
      "line": 27,
      "text": "def get_fixed_rotation_matrix(self, initStaticMarkerPosMat, staticMarkerPosMat):"
    },
    {
      "line": 38,
      "text": "def get_fixed_marker_pos_mat(self, initStaticMarkerPosMat, staticMarkerPosMat, totalMarkerPosMat):"
    },
    {
      "line": 45,
      "text": "def get_locator_pos_list(self, locList):"
    },
    {
      "line": 49,
      "text": "def get_marker_pos_matrix(self, markerList, frame):"
    },
    {
      "line": 54,
      "text": "def rotationRemoval(self, frameRange):"
    }
  ]
}

```text

"""
Head rotation removal Maya python script

Usage:

totalMarkerList = ['marker1', 'marker2', ..., 'marker100']   # create total facial marker list
staticMarkerList = ['marker1', 'marker2', 'marker3']        # create static facial marker list
HR = HeadRotationRemoval(totalMarkerList, staticMarkerList, 1)   # create HR instance with (totalMarker, staticMarker, referenceFrame)
HR.rotationRemoval([1,3066])    # run rotation removal with time range

"""

import maya.cmds as mc
import numpy as np
DEBUG = 0

class HeadRotationRemoval():
    def __init__(self, totalMarkerList, staticMarkerList, referenceFrame):
        self.totalMarkerList = totalMarkerList
        self.staticMarkerList = staticMarkerList
        self.referenceFrame = referenceFrame

    def get_standardized_matrix(self, mat):
        mean = mat.mean(axis=0)
        return mat - mean

    def get_fixed_rotation_matrix(self, initStaticMarkerPosMat, staticMarkerPosMat):
        mean = initStaticMarkerPosMat.mean(axis=0)
        initStaticMarkerPosMat_s = self.get_standardized_matrix(initStaticMarkerPosMat).T
        staticMarkerPosMat_s = self.get_standardized_matrix(staticMarkerPosMat).T
        H = np.dot(staticMarkerPosMat_s, initStaticMarkerPosMat_s.T)
        U,s,V = np.linalg.svd(H)
        I = np.identity(V.shape[0])
        I[V.shape[0]-1, V.shape[0]-1] = np.linalg.det(np.dot(V, U.T))
        R = np.dot(np.dot(V.T, I),U.T)
        return R

    def get_fixed_marker_pos_mat(self, initStaticMarkerPosMat, staticMarkerPosMat, totalMarkerPosMat):
        R = self.get_fixed_rotation_matrix(initStaticMarkerPosMat, staticMarkerPosMat)
        fixedTotalMarkerPosMat = np.dot(R, totalMarkerPosMat.T).T
        fixedStaticMarkerPosMat = np.dot(R, staticMarkerPosMat.T).T
        translationVec = initStaticMarkerPosMat.mean(axis=0) - fixedStaticMarkerPosMat.mean(axis=0)
        return fixedTotalMarkerPosMat+translationVec

    def get_locator_pos_list(self, locList):
        locPosList = [mc.pointPosition(loc, w=True) for loc in locList]
        return locPosList

    def get_marker_pos_matrix(self, markerList, frame):
        mc.currentTime(frame)
        posMat = np.asarray(self.get_locator_pos_list(markerList))
        return posMat

    def rotationRemoval(self, frameRange):
        initStaticMarkerPosMat = self.get_marker_pos_matrix(self.staticMarkerList, self.referenceFrame)  # Reference poses of static markers

        for f in range(frameRange[0], frameRange[1]+1):
            mc.currentTime(f)
            staticMarkerPosMat = self.get_marker_pos_matrix(self.staticMarkerList, f)
            totalMarkerPosMat = self.get_marker_pos_matrix(self.totalMarkerList, f)
            fixedTotalMarkerPosMat = self.get_fixed_marker_pos_mat(initStaticMarkerPosMat, staticMarkerPosMat, totalMarkerPosMat)

            for i, marker in enumerate(self.totalMarkerList) :
                if not mc.objExists('new_'+marker):
                    mc.spaceLocator(n='new_'+marker)
                mc.xform('new_'+marker, t=[fixedTotalMarkerPosMat[i][0], fixedTotalMarkerPosMat[i][1], fixedTotalMarkerPosMat[i][2]], ws=True)
                mc.setKeyframe('new_'+marker)


# totalMarkerList = [u'R_ForeHead_T01', u'R_ForeHead_T02', u'C_ForeHead_T', u'L_ForeHead_T02', u'L_ForeHead_T01', u'R_ForeHead_M01', u'R_ForeHead_M02', u'C_ForeHead_M', u'L_ForeHead_M02', u'L_ForeHead_M01', u'R_ForeHead_B01', u'R_ForeHead_B02', u'C_ForeHead_B03', u'L_ForeHead_B02', u'L_ForeHead_B01', u'R_Brow_01', u'R_Brow_02', u'R_Brow_03', u'R_Brow_04', u'R_Brow_05', u'L_Brow_01', u'L_Brow_02', u'L_Brow_03', u'L_Brow_04', u'L_Brow_05', u'C_Brow', u'R_Eye_01', u'R_Eye_02', u'R_Eye_03', u'R_Eye_04', u'R_Eye_05', u'R_Eye_06', u'R_Eye_07', u'L_Eye_01', u'L_Eye_02', u'L_Eye_03', u'L_Eye_04', u'L_Eye_05', u'L_Eye_06', u'L_Eye_07', u'R_Nose_Top', u'C_Nose_Top', u'L_Nose_Top', u'R_Nose_Mid', u'C_Nose_Mid', u'L_Nose_Mid', u'R_Nose_Bot', u'C_Nose_Bot', u'L_Nose_Bot', u'R_Cheek_T01', u'R_Cheek_T02', u'R_Cheek_T03', u'R_Cheek_T04', u'R_Cheek_M01', u'R_Cheek_M02', u'R_Cheek_M03', u'R_Cheek_M04', u'R_Cheek_B01', u'R_Cheek_B02', u'R_Cheek_B03', u'R_Cheek_B04', u'L_Cheek_T01', u'L_Cheek_T02', u'L_Cheek_T03', u'L_Cheek_T04', u'L_Cheek_M01', u'L_Cheek_M02', u'L_Cheek_M03', u'L_Cheek_M04', u'L_Cheek_B01', u'L_Cheek_B02', u'L_Cheek_B03', u'L_Cheek_B04', u'R_Phil_01', u'L_Phil_01', u'R_Mouth_T01', u'R_Mouth_T02', u'R_Mouth_T03', u'C_Mouth_Top', u'L_Mouth_T03', u'L_Mouth_T02', u'L_Mouth_T01', u'R_Mouth_B01', u'R_Mouth_B02', u'C_Mouth_Bot', u'L_Mouth_B02', u'L_Mouth_B01', u'R_Chin_T01', u'R_Chin_T02', u'R_Chin_T03', u'L_Chin_T03', u'L_Chin_T02', u'L_Chin_T01', u'R_Chin_B01', u'R_Chin_B02', u'R_Chin_B03', u'L_Chin_B03', u'L_Chin_B02', u'L_Chin_B01', u'head0', u'head1', u'head2']
# staticMarkerList = [u'head0', u'head1', u'head2']
# HR = HeadRotationRemoval(totalMarkerList, staticMarkerList, 1)
# HR.rotationRemoval([3200,4145])

totalMarkerList = [u'R_ForeHead_T01', u'R_ForeHead_T02', u'C_ForeHead_T', u'L_ForeHead_T02', u'L_ForeHead_T01', u'R_ForeHead_M01', u'R_ForeHead_M02', u'C_ForeHead_M', u'L_ForeHead_M02', u'L_ForeHead_M01', u'R_ForeHead_B01', u'R_ForeHead_B02', u'C_ForeHead_B03', u'L_ForeHead_B02', u'L_ForeHead_B01', u'R_Brow_01', u'R_Brow_02', u'R_Brow_03', u'R_Brow_04', u'R_Brow_05', u'L_Brow_01', u'L_Brow_02', u'L_Brow_03', u'L_Brow_04', u'L_Brow_05', u'C_Brow', u'R_Eye_01', u'R_Eye_02', u'R_Eye_03', u'R_Eye_04', u'R_Eye_05', u'R_Eye_06', u'R_Eye_07', u'L_Eye_01', u'L_Eye_02', u'L_Eye_03', u'L_Eye_04', u'L_Eye_05', u'L_Eye_06', u'L_Eye_07', u'R_Nose_Top', u'C_Nose_Top', u'L_Nose_Top', u'R_Nose_Mid', u'C_Nose_Mid', u'L_Nose_Mid', u'R_Nose_Bot', u'C_Nose_Bot', u'L_Nose_Bot', u'R_Cheek_T01', u'R_Cheek_T02', u'R_Cheek_T03', u'R_Cheek_T04', u'R_Cheek_M01', u'R_Cheek_M02', u'R_Cheek_M03', u'R_Cheek_M04', u'R_Cheek_B01', u'R_Cheek_B02', u'R_Cheek_B03', u'R_Cheek_B04', u'L_Cheek_T01', u'L_Cheek_T02', u'L_Cheek_T03', u'L_Cheek_T04', u'L_Cheek_M01', u'L_Cheek_M02', u'L_Cheek_M03', u'L_Cheek_M04', u'L_Cheek_B01', u'L_Cheek_B02', u'L_Cheek_B03', u'L_Cheek_B04', u'R_Phil_01', u'L_Phil_01', u'R_Mouth_T01', u'R_Mouth_T02', u'R_Mouth_T03', u'C_Mouth_Top', u'L_Mouth_T03', u'L_Mouth_T02', u'L_Mouth_T01', u'R_Mouth_B01', u'R_Mouth_B02', u'C_Mouth_Bot', u'L_Mouth_B02', u'L_Mouth_B01', u'R_Chin_T01', u'R_Chin_T02', u'R_Chin_T03', u'L_Chin_T03', u'L_Chin_T02', u'L_Chin_T01', u'R_Chin_B01', u'R_Chin_B02', u'R_Chin_B03', u'L_Chin_B03', u'L_Chin_B02', u'L_Chin_B01', u'left', u'center', u'right']
staticMarkerList = ['left', 'center', 'right', 'C_Nose_Bot']
HR = HeadRotationRemoval(totalMarkerList, staticMarkerList, 1)
HR.rotationRemoval([1,725])

```
