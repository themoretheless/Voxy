# Facial rendering source scan: songjaewon/CFR

Repository: https://github.com/songjaewon/CFR

Pinned source revision: 605d369780022d9dec028c52eb6a89f6331acef9

Discovery topic: facial retargeting

Description: Characteristic Facial Retargeting

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"face": ["CFR_mayaInterface.py", "CFR_mayaInterface.pyc"]}

## Source file: CFR_mayaInterface.py

{
  "path": "CFR_mayaInterface.py",
  "blob_sha": "623b2e2b735f88591f13ae7bd2c6ed167fafebf2",
  "sha256": "822bdff5c1b8d1f7557f96cdda412e8256f806ad5206b029aa192f591502d4cd",
  "lines": 158,
  "bytes": 6052,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 1
  },
  "url": "https://github.com/songjaewon/CFR/blob/605d369780022d9dec028c52eb6a89f6331acef9/CFR_mayaInterface.py",
  "function_signature_candidates": [
    {
      "line": 19,
      "text": "def __init__(self):"
    },
    {
      "line": 23,
      "text": "def addROEfile(self, charName, CVlist, ROE, fileName):"
    },
    {
      "line": 54,
      "text": "def createAnimFile(self, charName, CVlist, animDataList, frameList, fileName):"
    },
    {
      "line": 62,
      "text": "def createAnim(self, charName, CVlist, frameList, fileName):"
    },
    {
      "line": 71,
      "text": "def importAnimFromFile(self, charName, fileName):"
    },
    {
      "line": 92,
      "text": "def makeMatFromFile(self, fileName):"
    },
    {
      "line": 106,
      "text": "def DEBUG_dataCompare(self, srcMat, tgtMat):"
    },
    {
      "line": 121,
      "text": "def createTrainingFile(self, srcFileName, tgtFileName, trainingFileName):"
    },
    {
      "line": 139,
      "text": "def createFinalResult(self, srcAnimFileName, tgtCharDataFileName, trainDataFileName, resultAnimFileName):"
    }
  ]
}

```text

'''
Created on 2015. 6. 1.

@author: cimple
'''
import maya.cmds as mc
import sys
sys.path.append('C:/Users/cimple/Documents/GitHub/CFR')

import CFR_dataclass
reload(CFR_dataclass)
from CFR_dataclass import *

import CFR_training
reload(CFR_training)
from CFR_training import *

class CFR_mayaInterface:
    def __init__(self):
        self.DEBUG = 1
        None
    
    def addROEfile(self, charName, CVlist, ROE, fileName):
        import os.path        
        if (os.path.isfile(fileName)==True):
            f = open(fileName)
            import pickle
            charData = pickle.load(f)
            if charData.charName != charName :
                print 'Error : Adding to different character ROE file! charData.charName : ', charData.charName, ' Input character name: ', charName 
                return
            if charData.CVlist != CVlist :
                print 'Error : Adding to different CV list! charData.CV list and input CVlist is different.'
                return
            charData.addROE(ROE)
            f.close()
            f = open(fileName, 'w')
            pickle.dump(charData, f)
            f.close()
        else:
            charData = CFR_charData(charName, CVlist)
            charData.addROE(ROE)
            f = open(fileName, 'w')
            import pickle
            pickle.dump(charData, f)
            f.close()     
        if self.DEBUG==1:
            print '--- Adding ROE file Done --- '
            print 'fileName : ', fileName
            print 'charName : ', charData.charName
            print 'numCV : ', charData.numCV
            print 'numROE : ', charData.numROE   
    
    def createAnimFile(self, charName, CVlist, animDataList, frameList, fileName):
        srcAnimData = CFR_animData(charName, CVlist, animDataList, frameList)        
        #writing file
        f = open(fileName, 'w')
        import pickle
        pickle.dump(srcAnimData, f)
        f.close()
    
    def createAnim(self, charName, CVlist, frameList, fileName):        
        animDataList = []
        for frame in frameList:
            tmpValList = []
            for cv in CVlist :
                tmpValList.append(mc.getAttr(cv, t=frame))
            animDataList.append(tmpValList)
        self.createAnimFile(charName, CVlist, animDataList, frameList, fileName)
    
    def importAnimFromFile(self, charName, fileName):
        f = open(fileName)
        import pickle
        animData = pickle.load(f)
        
        if animData.charName != charName :
            print 'Error : Character name is different! Data character name : ', animData.charName, ' Input character name : ', charName
            return
        
        for dataIdx, CVdata in enumerate(animData.animDataList):
            mc.currentTime(animData.frameList[dataIdx])
            for cvIdx, CV in enumerate(animData.CVlist) :
                mc.setAttr(CV, CVdata[cvIdx])
                mc.setKeyframe(CV)
        f.close()
        print 'Create Animation File Done'
        print 'File Name : ', fileName
        print 'Char Name : ', charName
        print 'Frame Length : ' ,animData.numFrames
        print 'Char CV number : ', animData.numCV 
    
    def makeMatFromFile(self, fileName):
        f = open(fileName, 'r')
        mat = np.arange(0.0)
        for line in f :
            dataList = line.split()        
            tempMat = np.arange(float(len(dataList)))
            for i in range(len(dataList)) :            
                tempMat[i] = float(dataList[i])            
            if mat.size == 0 :
                mat = tempMat
            else :
                mat = np.vstack((mat, tempMat))
        return mat
    
    def DEBUG_dataCompare(self, srcMat, tgtMat):
        oldSrcMat = self.makeMatFromFile('C:/Users/cimple/Documents/maya/projects/default/data/old_humanROE.txt')
        oldMalcomMat = self.makeMatFromFile('C:/Users/cimple/Documents/maya/projects/default/data/old_MalcomROE.txt')
        
        print 'srcError'
        for j in range(len(oldSrcMat)):
            for i in range(len(oldSrcMat[j])):
                if (abs(oldSrcMat[j][i]-srcMat[j][i])>1e-5) : 
                    print oldSrcMat[j][i], srcMat[j][i], oldSrcMat[j][i]-srcMat[j][i]
        print 'tgtError'
        for j in range(len(oldMalcomMat)):
            for i in range(len(oldMalcomMat[j])):
                if (abs(oldMalcomMat[j][i]-tgtMat[j][i])>1e-5) : 
                    print oldMalcomMat[j][i], tgtMat[j][i], oldMalcomMat[j][i]-tgtMat[j][i]
    
    def createTrainingFile(self, srcFileName, tgtFileName, trainingFileName):
        import pickle
        srcFile = open(srcFileName)
        tgtFile = open(tgtFileName)
        srcCharData = pickle.load(srcFile)
        tgtCharData = pickle.load(tgtFile)
        
        rbfTrain = RBFtrain()
        rbfTrain.RBFtraining(srcCharData.ROE, tgtCharData.ROE)
        trainData = CFR_trainData(srcCharData.charName, tgtCharData.charName, rbfTrain)
        
        trainFile = open(trainingFileName, 'w')
        pickle.dump(trainData, trainFile)
        
        srcFile.close()
        tgtFile.close()
        trainFile.close()
    
    def createFinalResult(self, srcAnimFileName, tgtCharDataFileName, trainDataFileName, resultAnimFileName):
        import pickle
        srcAnimFile = open(srcAnimFileName)
        srcAnimData = pickle.load(srcAnimFile)
        tgtCharFile = open(tgtCharDataFileName)
        tgtCharData = pickle.load(tgtCharFile)                
        trainDataFile = open(trainDataFileName)
        trainData = pickle.load(trainDataFile)
        
        
        resultAnimDataList = trainData.rbfTrain.RBFrunning(srcAnimData.animDataList)        
        resultAnimData = CFR_animData(tgtCharData.charName, tgtCharData.CVlist, resultAnimDataList, srcAnimData.frameList)
        
        resultAnimFile = open(resultAnimFileName, 'w')
        pickle.dump(resultAnimData, resultAnimFile)
        
        srcAnimFile.close()
        tgtCharFile.close()
        trainDataFile.close()
        resultAnimFile.close()    

```

## Source file: CFR_training.py

{
  "path": "CFR_training.py",
  "blob_sha": "5f5b262fde30247c81a43f301599a2b49a6bc0cb",
  "sha256": "f36ce9ae35769e0358730cbde2a4e7d3fadd75b8810a306d12291e2d12582bfa",
  "lines": 160,
  "bytes": 5720,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/songjaewon/CFR/blob/605d369780022d9dec028c52eb6a89f6331acef9/CFR_training.py",
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
      "line": 61,
      "text": "def buildBasisMatrix(self, inputMat) :"
    },
    {
      "line": 81,
      "text": "def setBasisFunction(self, basisType) :"
    },
    {
      "line": 89,
      "text": "def setLamda(self, value) :"
    },
    {
      "line": 93,
      "text": "def train(self, inputMat, outputMat):"
    },
    {
      "line": 107,
      "text": "def interpolate(self, sample) :"
    },
    {
      "line": 121,
      "text": "def __init__(self):"
    },
    {
      "line": 124,
      "text": "def RBFtraining(self, srcROE, tgtROE) :"
    },
    {
      "line": 133,
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

## Source file: CFR_dataclass.py

{
  "path": "CFR_dataclass.py",
  "blob_sha": "91f1be0c7b603d8cfeaa8d5d06f32188de02da40",
  "sha256": "8dd64a8119a5d722868cc42380ed0a9181f97c08122386499468cbbe3860683c",
  "lines": 41,
  "bytes": 1259,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/songjaewon/CFR/blob/605d369780022d9dec028c52eb6a89f6331acef9/CFR_dataclass.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def __init__(self, charName, CVlist):"
    },
    {
      "line": 16,
      "text": "def addROE(self, cvValList):"
    },
    {
      "line": 28,
      "text": "def __init__(self, charName, CVlist, animDataList, frameList):"
    },
    {
      "line": 37,
      "text": "def __init__(self, srcCharName, tgtCharName, rbfTrain):"
    }
  ]
}

```text

'''
Created on 2015. 6. 1.

@author: cimple
'''
import numpy as np

class CFR_charData:
    def __init__(self, charName, CVlist):
        self.charName = charName        
        self.CVlist = CVlist
        self.numCV = len(self.CVlist)
        self.ROE = None
        self.numROE = 0      
    
    def addROE(self, cvValList):
        if len(self.CVlist) != len(cvValList): 
            print 'ERROR : length of the input CV values is different from the lentg of CV list'
            return
        if self.ROE == None:
            self.ROE = np.array(cvValList)
        else:
            self.ROE = np.vstack((self.ROE, cvValList))
        self.numROE = int(self.ROE.shape[0])
          
    
class CFR_animData:
    def __init__(self, charName, CVlist, animDataList, frameList):
        self.charName = charName        
        self.CVlist = CVlist
        self.numCV = len(self.CVlist)
        self.animDataList = np.array(animDataList)
        self.numFrames = len(frameList)
        self.frameList = frameList

class CFR_trainData:
    def __init__(self, srcCharName, tgtCharName, rbfTrain):
        self.srcCharName = srcCharName
        self.tgtCharName = tgtCharName
        self.rbfTrain = rbfTrain
        self.weightMat = rbfTrain.rbfn.weightMat

```

## Source file: TestExample.py

{
  "path": "TestExample.py",
  "blob_sha": "2490d594d0a2e8d47860d11faf66e6c4fc6462d1",
  "sha256": "a1fb585ad146961fce06d41d66a023f334c38bbf17e234d5a59895c1a193e13b",
  "lines": 68,
  "bytes": 16969,
  "selection_score": -2,
  "keyword_line_counts": {
    "eye": 1,
    "skin": 1,
    "jaw": 3,
    "mouth": 2,
    "teeth": 1,
    "tongue": 1,
    "face": 23,
    "lip": 3
  },
  "url": "https://github.com/songjaewon/CFR/blob/605d369780022d9dec028c52eb6a89f6331acef9/TestExample.py",
  "function_signature_candidates": [
    {
      "line": 65,
      "text": "def importFinalResult():"
    }
  ]
}

```text

'''
Created on 2015. 6. 2.

@author: cimple
'''

import sys
sys.path.append('C:/Users/cimple/Documents/GitHub/CFR')
import CFR_mayaInterface
reload(CFR_mayaInterface)
from CFR_mayaInterface import *

### Src Face ROE file generation
#def createSrcFaceROE():
#    for i in range(1, 37):
#        mc.currentTime(i)
#        CFRmi = CFR_mayaInterface()
#        srcFaceCVlist = [u'targetBlend.lBrowOutDn', u'targetBlend.rBrowOutDn', u'targetBlend.browsMidDn', u'targetBlend.rBrowOutUp', u'targetBlend.lBrowOutUp', u'targetBlend.browsMidUp', u'targetBlend.browsSqueeze', u'targetBlend.rBlink', u'targetBlend.lBlink', u'targetBlend.lSquint', u'targetBlend.rSquint', u'targetBlend.rSneer', u'targetBlend.lSneer', u'targetBlend.rFrown', u'targetBlend.lFrown', u'targetBlend.lSmile', u'targetBlend.rSmile', u'targetBlend.rMouth', u'targetBlend.lMouth', u'targetBlend.down', u'targetBlend.up', u'targetBlend.rJaw', u'targetBlend.lJaw', u'targetBlend.jawFwd', u'targetBlend.open', u'targetBlend.rLowerLibUp', u'targetBlend.lLowerLibUp', u'targetBlend.lLowerLibDown', u'targetBlend.rLowerLibDown', u'targetBlend.rUpperLipDown', u'targetBlend.lUpperLipDown', u'targetBlend.lUpperLipUp', u'targetBlend.rUpperLipUp', u'targetBlend.rNarrow', u'targetBlend.lNarrow']
#        currentROE = []
#        for cv in srcFaceCVlist :
#            currentROE.append(mc.getAttr(cv))
#        CFRmi.addROEfile('srcFace', srcFaceCVlist, currentROE, 'C:/Users/cimple/Documents/maya/projects/default/data/srcFaceROE.txt')
#createSrcFaceROE()


### Tgt Face ROE file generation
#def createTargetFaceROE():
#    for i in range(1, 37):
#        mc.currentTime(i)
#        CFRmi = CFR_mayaInterface()
#        MalcomCVlist = [u'ctlEyeShaperBtRt.rotateZ', u'ctlEyeShaperBtRt.rotateY', u'ctlEyeShaperBtRt.rotateX', u'ctlEyeShaperBtRt.translateZ', u'ctlEyeShaperBtRt.translateY', u'ctlEyeShaperBtRt.translateX', u'ctlEyeShaperTpRt.rotateZ', u'ctlEyeShaperTpRt.rotateY', u'ctlEyeShaperTpRt.rotateX', u'ctlEyeShaperTpRt.translateZ', u'ctlEyeShaperTpRt.translateY', u'ctlEyeShaperTpRt.translateX', u'ctlEyeShaperBtLf.rotateZ', u'ctlEyeShaperBtLf.rotateY', u'ctlEyeShaperBtLf.rotateX', u'ctlEyeShaperBtLf.translateZ', u'ctlEyeShaperBtLf.translateY', u'ctlEyeShaperBtLf.translateX', u'ctlEyeShaperTpLf.rotateZ', u'ctlEyeShaperTpLf.rotateY', u'ctlEyeShaperTpLf.rotateX', u'ctlEyeShaperTpLf.translateZ', u'ctlEyeShaperTpLf.translateY', u'ctlEyeShaperTpLf.translateX', u'ctlLidTpRt.squint', u'ctlLidTpRt.rotateZ', u'ctlLidTpRt.rotateY', u'ctlLidTpRt.rotateX', u'ctlLidTpRt.translateZ', u'ctlLidTpRt.translateY', u'ctlLidTpRt.translateX', u'ctlLidCrnInRt.rotateZ', u'ctlLidCrnInRt.rotateY', u'ctlLidCrnInRt.rotateX', u'ctlLidCrnInRt.translateZ', u'ctlLidCrnInRt.translateY', u'ctlLidCrnInRt.translateX', u'ctlLidBtRt.squint', u'ctlLidBtRt.lidCheek', u'ctlLidBtRt.rotateZ', u'ctlLidBtRt.rotateY', u'ctlLidBtRt.rotateX', u'ctlLidBtRt.translateZ', u'ctlLidBtRt.translateY', u'ctlLidBtRt.translateX', u'ctlEyeRt.rotateZ', u'ctlEyeRt.rotateY', u'ctlEyeRt.rotateX', u'ctlEyeRt.translateZ', u'ctlEyeRt.translateY', u'ctlEyeRt.translateX', u'ctlEyeLidRt.rotateZ', u'ctlEyeLidRt.rotateY', u'ctlEyeLidRt.rotateX', u'ctlEyeLidRt.translateZ', u'ctlEyeLidRt.translateY', u'ctlEyeLidRt.translateX', u'ctlLidTpLf.squint', u'ctlLidTpLf.rotateZ', u'ctlLidTpLf.rotateY', u'ctlLidTpLf.rotateX', u'ctlLidTpLf.translateZ', u'ctlLidTpLf.translateY', u'ctlLidTpLf.translateX', u'ctlLidCrnOtLf.rotateZ', u'ctlLidCrnOtLf.rotateY', u'ctlLidCrnOtLf.rotateX', u'ctlLidCrnOtLf.translateZ', u'ctlLidCrnOtLf.translateY', u'ctlLidCrnOtLf.translateX', u'ctlLidCrnInLf.rotateZ', u'ctlLidCrnInLf.rotateY', u'ctlLidCrnInLf.rotateX', u'ctlLidCrnInLf.translateZ', u'ctlLidCrnInLf.translateY', u'ctlLidCrnInLf.translateX', u'ctlLidBtLf.squint', u'ctlLidBtLf.lidCheek', u'ctlLidBtLf.rotateZ', u'ctlLidBtLf.rotateY', u'ctlLidBtLf.rotateX', u'ctlLidBtLf.translateZ', u'ctlLidBtLf.translateY', u'ctlLidBtLf.translateX', u'ctlEyeLidLf.rotateZ', u'ctlEyeLidLf.rotateY', u'ctlEyeLidLf.rotateX', u'ctlEyeLidLf.translateZ', u'ctlEyeLidLf.translateY', u'ctlEyeLidLf.translateX', u'ctlBrowOtLf.rotateZ', u'ctlBrowOtLf.rotateY', u'ctlBrowOtLf.rotateX', u'ctlBrowOtLf.translateZ', u'ctlBrowOtLf.translateY', u'ctlBrowOtLf.translateX', u'ctlBrowOtRt.rotateZ', u'ctlBrowOtRt.rotateY', u'ctlBrowOtRt.rotateX', u'ctlBrowOtRt.translateZ', u'ctlBrowOtRt.translateY', u'ctlBrowOtRt.translateX', u'ctlBrowMdRt.rotateZ', u'ctlBrowMdRt.rotateY', u'ctlBrowMdRt.rotateX', u'ctlBrowMdRt.translateZ', u'ctlBrowMdRt.translateY', u'ctlBrowMdRt.translateX', u'ctlBrowInRt.rotateZ', u'ctlBrowInRt.rotateY', u'ctlBrowInRt.rotateX', u'ctlBrowInRt.translateZ', u'ctlBrowInRt.translateY', u'ctlBrowInRt.translateX', u'ctlBrowMdLf.rotateZ', u'ctlBrowMdLf.rotateY', u'ctlBrowMdLf.rotateX', u'ctlBrowMdLf.translateZ', u'ctlBrowMdLf.translateY', u'ctlBrowMdLf.translateX', u'ctlBrowInLf.rotateZ', u'ctlBrowInLf.rotateY', u'ctlBrowInLf.rotateX', u'ctlBrowInLf.translateZ', u'ctlBrowInLf.translateY', u'ctlBrowInLf.translateX', u'ctlCornerLf.pucker', u'ctlCornerLf.nasoCrease', u'ctlCornerLf.disableAutoPucker', u'ctlCornerLf.cornerPinchOt', u'ctlCornerLf.cornerPinchIn', u'ctlCornerLf.cornerHeight', u'ctlCornerLf.rotateZ', u'ctlCornerLf.rotateY', u'ctlCornerLf.rotateX', u'ctlCornerLf.translateZ', u'ctlCornerLf.translateY', u'ctlCornerLf.translateX', u'ctlCornerRt.pucker', u'ctlCornerRt.nasoCrease', u'ctlCornerRt.disableAutoPucker', u'ctlCornerRt.cornerPinchOt', u'ctlCornerRt.cornerPinchIn', u'ctlCornerRt.cornerHeight', u'ctlCornerRt.rotateZ', u'ctlCornerRt.rotateY', u'ctlCornerRt.rotateX', u'ctlCornerRt.translateZ', u'ctlCornerRt.translateY', u'ctlCornerRt.translateX', u'ctlNoseRt.rotateZ', u'ctlNoseRt.rotateY', u'ctlNoseRt.rotateX', u'ctlNoseRt.translateZ', u'ctlNoseRt.translateY', u'ctlNoseRt.translateX', u'ctlNoseLf.rotateZ', u'ctlNoseLf.rotateY', u'ctlNoseLf.rotateX', u'ctlNoseLf.translateZ', u'ctlNoseLf.translateY', u'ctlNoseLf.translateX', u'ctlNose1.noseBrowCrunch', u'ctlNose1.rotateZ', u'ctlNose1.rotateY', u'ctlNose1.rotateX', u'ctlNose1.translateZ', u'ctlNose1.translateY', u'ctlNose1.translateX', u'ctlLipCrnRt.rotateZ', u'ctlLipCrnRt.rotateY', u'ctlLipCrnRt.rotateX', u'ctlLipCrnRt.translateZ', u'ctlLipCrnRt.translateY', u'ctlLipCrnRt.translateX', u'ctlLipCrnBtRt.rotateZ', u'ctlLipCrnBtRt.rotateY', u'ctlLipCrnBtRt.rotateX', u'ctlLipCrnBtRt.translateZ', u'ctlLipCrnBtRt.translateY', u'ctlLipCrnBtRt.translateX', u'ctlLipCrnTpRt.rotateZ', u'ctlLipCrnTpRt.rotateY', u'ctlLipCrnTpRt.rotateX', u'ctlLipCrnTpRt.translateZ', u'ctlLipCrnTpRt.translateY', u'ctlLipCrnTpRt.translateX', u'ctlLipCrnLf.rotateZ', u'ctlLipCrnLf.rotateY', u'ctlLipCrnLf.rotateX', u'ctlLipCrnLf.translateZ', u'ctlLipCrnLf.translateY', u'ctlLipCrnLf.translateX', u'ctlLipCrnTpLf.rotateZ', u'ctlLipCrnTpLf.rotateY', u'ctlLipCrnTpLf.rotateX', u'ctlLipCrnTpLf.translateZ', u'ctlLipCrnTpLf.translateY', u'ctlLipCrnTpLf.translateX', u'ctlLipCrnBtLf.rotateZ', u'ctlLipCrnBtLf.rotateY', u'ctlLipCrnBtLf.rotateX', u'ctlLipCrnBtLf.translateZ', u'ctlLipCrnBtLf.translateY', u'ctlLipCrnBtLf.translateX', u'ctlLipTp2Lf.rotateZ', u'ctlLipTp2Lf.rotateY', u'ctlLipTp2Lf.rotateX', u'ctlLipTp2Lf.translateZ', u'ctlLipTp2Lf.translateY', u'ctlLipTp2Lf.translateX', u'ctlLipMdTp.thicknessZ', u'ctlLipMdTp.thicknessY', u'ctlLipMdTp.puff', u'ctlLipMdTp.press', u'ctlLipMdTp.peak', u'ctlLipMdTp.md', u'ctlLipMdTp.funnel', u'ctlLipMdTp.rotateZ', u'ctlLipMdTp.rotateY', u'ctlLipMdTp.rotateX', u'ctlLipMdTp.translateZ', u'ctlLipMdTp.translateY', u'ctlLipMdTp.translateX', u'ctlLipTp1Rt.rotateZ', u'ctlLipTp1Rt.rotateY', u'ctlLipTp1Rt.rotateX', u'ctlLipTp1Rt.translateZ', u'ctlLipTp1Rt.translateY', u'ctlLipTp1Rt.translateX', u'ctlLipTp1Lf.rotateZ', u'ctlLipTp1Lf.rotateY', u'ctlLipTp1Lf.rotateX', u'ctlLipTp1Lf.translateZ', u'ctlLipTp1Lf.translateY', u'ctlLipTp1Lf.translateX', u'ctlLipTpCt.rotateZ', u'ctlLipTpCt.rotateY', u'ctlLipTpCt.rotateX', u'ctlLipTpCt.translateZ', u'ctlLipTpCt.translateY', u'ctlLipTpCt.translateX', u'ctlLipTp2Rt.rotateZ', u'ctlLipTp2Rt.rotateY', u'ctlLipTp2Rt.rotateX', u'ctlLipTp2Rt.translateZ', u'ctlLipTp2Rt.translateY', u'ctlLipTp2Rt.translateX', u'ctlCheek1Lf.puff', u'ctlCheek1Lf.rotateZ', u'ctlCheek1Lf.rotateY', u'ctlCheek1Lf.rotateX', u'ctlCheek1Lf.translateZ', u'ctlCheek1Lf.translateY', u'ctlCheek1Lf.translateX', u'ctlCheek1Rt.puff', u'ctlCheek1Rt.rotateZ', u'ctlCheek1Rt.rotateY', u'ctlCheek1Rt.rotateX', u'ctlCheek1Rt.translateZ', u'ctlCheek1Rt.translateY', u'ctlCheek1Rt.translateX', u'ctlJawRot.rotateZ', u'ctlJawRot.rotateY', u'ctlJawRot.rotateX', u'ctlJawRot.translateZ', u'ctlJawRot.translateY', u'ctlJawRot.translateX', u'ctlChin1.rotateZ', u'ctlChin1.rotateY', u'ctlChin1.rotateX', u'ctlChin1.translateZ', u'ctlChin1.translateY', u'ctlChin1.translateX', u'ctlMentalis.rotateZ', u'ctlMentalis.rotateY', u'ctlMentalis.rotateX', u'ctlMentalis.translateZ', u'ctlMentalis.translateY', u'ctlMentalis.translateX', u'ctlLipMdBt.thicknessZ', u'ctlLipMdBt.thicknessY', u'ctlLipMdBt.puff', u'ctlLipMdBt.press', u'ctlLipMdBt.peak', u'ctlLipMdBt.md', u'ctlLipMdBt.funnel', u'ctlLipMdBt.rotateZ', u'ctlLipMdBt.rotateY', u'ctlLipMdBt.rotateX', u'ctlLipMdBt.translateZ', u'ctlLipMdBt.translateY', u'ctlLipMdBt.translateX', u'ctlLipBtCt.rotateZ', u'ctlLipBtCt.rotateY', u'ctlLipBtCt.rotateX', u'ctlLipBtCt.translateZ', u'ctlLipBtCt.translateY', u'ctlLipBtCt.translateX', u'ctlLipBt1Rt.rotateZ', u'ctlLipBt1Rt.rotateY', u'ctlLipBt1Rt.rotateX', u'ctlLipBt1Rt.translateZ', u'ctlLipBt1Rt.translateY', u'ctlLipBt1Rt.translateX', u'ctlLipBt1Lf.rotateZ', u'ctlLipBt1Lf.rotateY', u'ctlLipBt1Lf.rotateX', u'ctlLipBt1Lf.translateZ', u'ctlLipBt1Lf.translateY', u'ctlLipBt1Lf.translateX', u'ctlLipBt2Rt.rotateZ', u'ctlLipBt2Rt.rotateY', u'ctlLipBt2Rt.rotateX', u'ctlLipBt2Rt.translateZ', u'ctlLipBt2Rt.translateY', u'ctlLipBt2Rt.translateX', u'ctlLipBt2Lf.rotateZ', u'ctlLipBt2Lf.rotateY', u'ctlLipBt2Lf.rotateX', u'ctlLipBt2Lf.translateZ', u'ctlLipBt2Lf.translateY', u'ctlLipBt2Lf.translateX', u'ctlTeethBtLower2Lf.rotateZ', u'ctlTeethBtLower2Lf.rotateY', u'ctlTeethBtLower2Lf.rotateX', u'ctlTeethBtLower2Lf.translateZ', u'ctlTeethBtLower2Lf.translateY', u'ctlTeethBtLower2Lf.translateX', u'ctlTeethBtLower2Rt.rotateZ', u'ctlTeethBtLower2Rt.rotateY', u'ctlTeethBtLower2Rt.rotateX', u'ctlTeethBtLower2Rt.translateZ', u'ctlTeethBtLower2Rt.translateY', u'ctlTeethBtLower2Rt.translateX', u'ctlTeethBtCt.rotateZ', u'ctlTeethBtCt.rotateY', u'ctlTeethBtCt.rotateX', u'ctlTeethBtCt.translateZ', u'ctlTeethBtCt.translateY', u'ctlTeethBtCt.translateX', u'ctlTeethBt2Rt.rotateZ', u'ctlTeethBt2Rt.rotateY', u'ctlTeethBt2Rt.rotateX', u'ctlTeethBt2Rt.translateZ', u'ctlTeethBt2Rt.translateY', u'ctlTeethBt2Rt.translateX', u'ctlTeethBt2Lf.rotateZ', u'ctlTeethBt2Lf.rotateY', u'ctlTeethBt2Lf.rotateX', u'ctlTeethBt2Lf.translateZ', u'ctlTeethBt2Lf.translateY', u'ctlTeethBt2Lf.translateX', u'ctlTeethBt1Rt.rotateZ', u'ctlTeethBt1Rt.rotateY', u'ctlTeethBt1Rt.rotateX', u'ctlTeethBt1Rt.translateZ', u'ctlTeethBt1Rt.translateY', u'ctlTeethBt1Rt.translateX', u'ctlTeethBt1Lf.rotateZ', u'ctlTeethBt1Lf.rotateY', u'ctlTeethBt1Lf.rotateX', u'ctlTeethBt1Lf.translateZ', u'ctlTeethBt1Lf.translateY', u'ctlTeethBt1Lf.translateX', u'ctlTeethBt.rotateZ', u'ctlTeethBt.rotateY', u'ctlTeethBt.rotateX', u'ctlTeethBt.translateZ', u'ctlTeethBt.translateY', u'ctlTeethBt.translateX', u'ctlTongue1.rotateZ', u'ctlTongue1.rotateY', u'ctlTongue1.rotateX', u'ctlTongue1.translateZ', u'ctlTongue1.translateY', u'ctlTongue1.translateX', u'ctlTongue2.rotateZ', u'ctlTongue2.rotateY', u'ctlTongue2.rotateX', u'ctlTongue2.translateZ', u'ctlTongue2.translateY', u'ctlTongue2.translateX', u'ctlTongue3.rotateZ', u'ctlTongue3.rotateY', u'ctlTongue3.rotateX', u'ctlTongue3.translateZ', u'ctlTongue3.translateY', u'ctlTongue3.translateX', u'ctlTongue4.rotateZ', u'ctlTongue4.rotateY', u'ctlTongue4.rotateX', u'ctlTongue4.translateZ', u'ctlTongue4.translateY', u'ctlTongue4.translateX', u'ctlTongue5.rotateZ', u'ctlTongue5.rotateY', u'ctlTongue5.rotateX', u'ctlTongue5.translateZ', u'ctlTongue5.translateY', u'ctlTongue5.translateX', u'ctlLipBt.rotateZ', u'ctlLipBt.rotateY', u'ctlLipBt.rotateX', u'ctlLipBt.translateZ', u'ctlLipBt.translateY', u'ctlLipBt.translateX', u'ctlJaw.lipSealHeight', u'ctlJaw.lipsSeal', u'ctlJaw.jawStretch', u'ctlJaw.rotateZ', u'ctlJaw.rotateY', u'ctlJaw.rotateX', u'ctlJaw.translateZ', u'ctlJaw.translateY', u'ctlJaw.translateX', u'ctlLipCornerSkinLf.rotateZ', u'ctlLipCornerSkinLf.rotateY', u'ctlLipCornerSkinLf.rotateX', u'ctlLipCornerSkinLf.translateZ', u'ctlLipCornerSkinLf.translateY', u'ctlLipCornerSkinLf.translateX', u'ctlLipCornerSkinRt.rotateZ', u'ctlLipCornerSkinRt.rotateY', u'ctlLipCornerSkinRt.rotateX', u'ctlLipCornerSkinRt.translateZ', u'ctlLipCornerSkinRt.translateY', u'ctlLipCornerSkinRt.translateX']
#        tmpValList = []
#        for CV in MalcomCVlist :
#            tmpValList.append(mc.getAttr(CV))
#        CFRmi.addROEfile('Malcom', MalcomCVlist, tmpValList, 'C:/Users/cimple/Documents/maya/projects/default/data/MalcomROE.txt')
#createTargetFaceROE()

## Src motion file generation
#def sourceAnimFileGeneration():
#    CFRmi = CFR_mayaInterface()
#    srcFaceCVlist = [u'targetBlend.lBrowOutDn', u'targetBlend.rBrowOutDn', u'targetBlend.browsMidDn', u'targetBlend.rBrowOutUp', u'targetBlend.lBrowOutUp', u'targetBlend.browsMidUp', u'targetBlend.browsSqueeze', u'targetBlend.rBlink', u'targetBlend.lBlink', u'targetBlend.lSquint', u'targetBlend.rSquint', u'targetBlend.rSneer', u'targetBlend.lSneer', u'targetBlend.rFrown', u'targetBlend.lFrown', u'targetBlend.lSmile', u'targetBlend.rSmile', u'targetBlend.rMouth', u'targetBlend.lMouth', u'targetBlend.down', u'targetBlend.up', u'targetBlend.rJaw', u'targetBlend.lJaw', u'targetBlend.jawFwd', u'targetBlend.open', u'targetBlend.rLowerLibUp', u'targetBlend.lLowerLibUp', u'targetBlend.lLowerLibDown', u'targetBlend.rLowerLibDown', u'targetBlend.rUpperLipDown', u'targetBlend.lUpperLipDown', u'targetBlend.lUpperLipUp', u'targetBlend.rUpperLipUp', u'targetBlend.rNarrow', u'targetBlend.lNarrow']
#    frameList=[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 190, 191, 192, 193, 194, 195, 196, 197, 198, 199, 200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219, 220, 221, 222, 223, 224, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255, 256, 257, 258, 259, 260, 261, 262, 263, 264, 265, 266, 267, 268, 269, 270, 271, 272, 273, 274, 275, 276, 277, 278, 279, 280, 281, 282, 283, 284, 285, 286, 287, 288, 289, 290, 291, 292, 293, 294, 295, 296, 297, 298, 299, 300, 301, 302, 303, 304, 305, 306, 307, 308, 309, 310, 311, 312, 313, 314, 315, 316, 317, 318, 319, 320, 321, 322, 323, 324, 325, 326, 327, 328, 329, 330, 331, 332, 333, 334, 335, 336, 337, 338, 339, 340, 341, 342, 343, 344, 345, 346, 347, 348, 349, 350, 351, 352, 353, 354, 355, 356, 357, 358, 359, 360]
#    CFRmi.createAnim('srcFace', srcFaceCVlist, frameList, 'C:/Users/cimple/Documents/maya/projects/default/data/srcFace_sourceAnim_360f.txt')
#sourceAnimFileGeneration()




#### Training file generation
#def trainingFileGeneration():
#    CFRmi = CFR_mayaInterface()
#    CFRmi.createTrainingFile('C:/Users/cimple/Documents/maya/projects/default/data/srcFaceROE.txt', 'C:/Users/cimple/Documents/maya/projects/default/data/MalcomROE.txt', 'C:/Users/cimple/Documents/maya/projects/default/data/srcFace-MalcomTraining.txt')
#trainingFileGeneration()



#### Running target motion
#def runningTargetMotion():
#    CFRmi = CFR_mayaInterface()
#    CFRmi.createFinalResult('C:/Users/cimple/Documents/maya/projects/default/data/srcFace_sourceAnim_360f.txt', 'C:/Users/cimple/Documents/maya/projects/default/data/MalcomROE.txt', 'C:/Users/cimple/Documents/maya/projects/default/data/srcFace-MalcomTraining.txt', 'C:/Users/cimple/Documents/maya/projects/default/data/srcFace-MalcomResult.txt')
#runningTargetMotion()    


### Importing final result
def importFinalResult():
    CFRmi = CFR_mayaInterface()
    CFRmi.importAnimFromFile('Malcom', 'C:/Users/cimple/Documents/maya/projects/default/data/srcFace-MalcomResult.txt')
importFinalResult()

```
