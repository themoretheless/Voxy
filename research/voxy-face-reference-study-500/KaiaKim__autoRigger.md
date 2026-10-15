# Facial rendering source scan: KaiaKim/autoRigger

Repository: https://github.com/KaiaKim/autoRigger

Pinned source revision: 7998f1c2e54803d36f37cfc75e7653b5c1c19c52

Discovery topic: facial rig

Description: Maya Facial auto rigging tool! The facial rig can be added in to any kind of body rig. (Beta)

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"eye": ["Kaia_AutoRigger/modules/__pycache__/EyeFunc.cpython-37.pyc", "Kaia_AutoRigger/modules/__pycache__/eye.cpython-37.pyc", "Kaia_AutoRigger/modules/__pycache__/eye.cpython-39.pyc", "Kaia_AutoRigger/modules/eye.py"], "blendshape": ["Kaia_AutoRigger/json/April_rigData_v01/blendshapeCrv.json", "Kaia_AutoRigger/json/Daisy_rigData_v05/blendshapeCrv.json", "Kaia_AutoRigger/json/Gabi_rigData_v03/blendshapeCrv.json", "Kaia_AutoRigger/json/Guy_rigData_v01/blendshapeCrv.json"], "mouth": ["Kaia_AutoRigger/modules/__pycache__/MouthFunc.cpython-37.pyc", "Kaia_AutoRigger/modules/__pycache__/mouth.cpython-37.pyc", "Kaia_AutoRigger/modules/__pycache__/mouth.cpython-39.pyc", "Kaia_AutoRigger/modules/mouth.py"], "rig": ["Kaia_AutoRigger/__init__.py", "Kaia_AutoRigger/__pycache__/UI.cpython-37.pyc", "Kaia_AutoRigger/__pycache__/__init__.cpython-37.pyc", "Kaia_AutoRigger/__pycache__/__init__.cpython-39.pyc", "Kaia_AutoRigger/__pycache__/binder.cpython-37.pyc", "Kaia_AutoRigger/__pycache__/binder.cpython-39.pyc", "Kaia_AutoRigger/__pycache__/builder.cpython-37.pyc", "Kaia_AutoRigger/__pycache__/builder.cpython-39.pyc", "Kaia_AutoRigger/__pycache__/data_.cpython-37.pyc", "Kaia_AutoRigger/__pycache__/data_.cpython-39.pyc", "Kaia_AutoRigger/__pycache__/helper.cpython-37.pyc", "Kaia_AutoRigger/__pycache__/helper.cpython-39.pyc", "Kaia_AutoRigger/__pycache__/namer.cpython-37.pyc", "Kaia_AutoRigger/__pycache__/namer.cpython-39.pyc", "Kaia_AutoRigger/binder.py", "Kaia_AutoRigger/builder.py", "Kaia_AutoRigger/data_.py", "Kaia_AutoRigger/helper.py", "Kaia_AutoRigger/json/April_rigData_v01/blendshapeCrv.json", "Kaia_AutoRigger/json/April_rigData_v01/ctlOrient.json", "Kaia_AutoRigger/json/April_rigData_v01/geoNames.json", "Kaia_AutoRigger/json/April_rigData_v01/vertIndices.json", "Kaia_AutoRigger/json/Daisy_rigData_v05/blendshapeCrv.json", "Kaia_AutoRigger/json/Daisy_rigData_v05/ctlOrient.json", "Kaia_AutoRigger/json/Daisy_rigData_v05/geoNames.json", "Kaia_AutoRigger/json/Daisy_rigData_v05/vertIndices.json", "Kaia_AutoRigger/json/Gabi_rigData_v03/blendshapeCrv.json", "Kaia_AutoRigger/json/Gabi_rigData_v03/ctlOrient.json", "Kaia_AutoRigger/json/Gabi_rigData_v03/geoNames.json", "Kaia_AutoRigger/json/Gabi_rigData_v03/vertIndices.json"], "face": ["Kaia_AutoRigger/prepare/faceBinds.ma", "Kaia_AutoRigger/prepare/faceGuides.ma"]}

## Source file: Kaia_AutoRigger/modules/eye.py

{
  "path": "Kaia_AutoRigger/modules/eye.py",
  "blob_sha": "f8f2fc3d1cc15d04e8c957924a653110ebbd50f4",
  "sha256": "90539667469216e746ddfa7eb807e372094c4c8b49eaa0ff308dc8d2a8dd2bca",
  "lines": 40,
  "bytes": 1477,
  "selection_score": 18,
  "keyword_line_counts": {
    "eye": 1
  },
  "url": "https://github.com/KaiaKim/autoRigger/blob/7998f1c2e54803d36f37cfc75e7653b5c1c19c52/Kaia_AutoRigger/modules/eye.py",
  "function_signature_candidates": [
    {
      "line": 8,
      "text": "def connectEyeCtls(ctls, rotCtl, aimCtl, aimMicroCtls, binds):"
    }
  ]
}

```text

import maya.cmds as mc
import importlib

from . import util
importlib.reload(util)

###-----------------------------------------------------FUNCTION---------------------------------------------------
def connectEyeCtls(ctls, rotCtl, aimCtl, aimMicroCtls, binds):
    for i,ctl in enumerate(ctls):
        convGrp = mc.group(em=True,p=ctl+'_nul',n=ctl+'_converge')
        rotGrp = mc.group(convGrp, n=ctl+'_rot')
        aimGrp = mc.group(rotGrp, n=ctl+'_aim')
        mc.parent(ctl+'_orient',convGrp)
        '''
        #connectBind
        mc.parentConstraint(ctl, binds[i], mo=True)
        mc.scaleConstraint(ctl, binds[i], mo=False)
        '''
        #connectRotCtl
        mc.connectAttr(rotCtl+'.r',rotGrp+'.r')
        #connectMicroAim
        mc.aimConstraint(aimMicroCtls[i], aimGrp, mo=True, wut='none')
        
    #connectConverge
    mc.addAttr(aimCtl, shortName='converge', keyable=True, defaultValue=0, minValue=-2, maxValue=2)
    
    mul = mc.createNode('multDoubleLinear')
    mc.connectAttr(aimCtl+'.converge',mul+'.input1')
    mc.setAttr(mul+'.input2',20)
    
    mudi = mc.createNode('multiplyDivide')
    mc.connectAttr(mul+'.output',mudi+'.input1X')
    mc.connectAttr(mul+'.output',mudi+'.input1Y')
    mc.setAttr(mudi+'.input2X',1)
    mc.setAttr(mudi+'.input2Y',-1)
    
    mc.connectAttr(mudi+'.outputX',ctls[0]+'_converge'+'.ry')
    mc.connectAttr(mudi+'.outputY',ctls[1]+'_converge'+'.ry')
    
    

```

## Source file: Kaia_AutoRigger/modules/mouth.py

{
  "path": "Kaia_AutoRigger/modules/mouth.py",
  "blob_sha": "26070c0e4c3949051155be67443c1e58dd741f00",
  "sha256": "2be44df17fcb8ddaf86d0ed4419f8971a8da083236b078f6438bf1d88b49bdea",
  "lines": 222,
  "bytes": 8404,
  "selection_score": 18,
  "keyword_line_counts": {
    "skin": 2,
    "blendshape": 2,
    "mouth": 10,
    "deform": 1,
    "rig": 7,
    "face": 1,
    "lip": 1
  },
  "url": "https://github.com/KaiaKim/autoRigger/blob/7998f1c2e54803d36f37cfc75e7653b5c1c19c52/Kaia_AutoRigger/modules/mouth.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def createMouthCrv(name, upperCrv, lowerCrv):"
    },
    {
      "line": 30,
      "text": "def createBindmeshes(names, targs, grpName):"
    },
    {
      "line": 41,
      "text": "def skinBindmeshes(bms, jnts):"
    },
    {
      "line": 45,
      "text": "def createUvPin(names, bindmeshes):"
    },
    {
      "line": 57,
      "text": "def createMouthDrivers(names, curv, grpName):"
    },
    {
      "line": 72,
      "text": "def lipDrivers(names, pins, ctls, grpName):"
    },
    {
      "line": 86,
      "text": "def createClsGrp(name, targ, bindmeshes, grpName, weights=[]):"
    },
    {
      "line": 108,
      "text": "def createBsCrv(guides, names, grpName):"
    },
    {
      "line": 116,
      "text": "def createBsNode(name, orig, targList):"
    },
    {
      "line": 121,
      "text": "def setBsCvWeight(bs):"
    },
    {
      "line": 136,
      "text": "def symmetricMouthCrv(crvList):"
    },
    {
      "line": 151,
      "text": "def connectBs(ctl, blendCrvs, bs):"
    },
    {
      "line": 179,
      "text": "def matchCrvRtoL(posList):"
    },
    {
      "line": 189,
      "text": "def connectBigClus(C1, P1):"
    },
    {
      "line": 195,
      "text": "def cornerCtls(ctl, clus, P1, P2):"
    }
  ]
}

```text

import maya.cmds as mc
import importlib

from . import util
from . import getset
importlib.reload(util)
importlib.reload(getset)

def createMouthCrv(name, upperCrv, lowerCrv):
    upperCVs = getset.getCVs(upperCrv)
    lowerCVs = getset.getCVs(lowerCrv)
    lowerCVs.reverse() #reverse list order so that it comes back to the start, as a circle
    cvList = upperCVs + lowerCVs

    sectionCount = 11
    mouthCrv = mc.circle(n=name, s=sectionCount, d=1)[0] #create a new curve #cv 0~11
    mc.delete(mouthCrv,constructionHistory=True)

    counter = 0
    for i in range(12): #We're going to iterate through mouth curv(CVs)
        pos = mc.xform(cvList[counter], q=True, t=True, ws=True) #get world position from the CV
        if counter == 6: #there's two cvs overlapping on the corner. 6 is the right corner CV.
            counter += 1 #We skip the overlapping cv by adding 1 to the counter
               
        mouthCV = mouthCrv + '.cv[%d]'%i #get mouth CV
        mc.move(pos[0], pos[1], pos[2], mouthCV) #snap mouth CV to the position

        counter += 1

def createBindmeshes(names, targs, grpName):
    grp = mc.group(em=True, n=grpName)
    for targ,name in zip(targs, names):
        pos=mc.xform(targ, q=True, ws=True, t=True)
        bindmesh = mc.polyPlane(w=.3, h=.3, sw=1, sh=1, n=name)[0]
        mc.move(pos[0],pos[1],pos[2], bindmesh)
        mc.makeIdentity(bindmesh, apply=True) #freeze transformation
        mc.delete(bindmesh, constructionHistory=True)
        
        mc.parent(bindmesh,grp)

def skinBindmeshes(bms, jnts):
    for jnt, bm in zip(jnts, bms):
        mc.skinCluster(jnt, bm, n='bindMesh_skinClst', tsb=True)

def createUvPin(names, bindmeshes):
    for bm,name in zip(bindmeshes, names):
        pin1=mc.createNode('uvPin')
        pin1=mc.rename(pin1, name)
        mc.setAttr(pin1+'.coordinate[0].coordinateU', 0.5)
        mc.setAttr(pin1+'.coordinate[0].coordinateV', 0.5)
        mc.setAttr(pin1+'.normalAxis', 1) #Y
        mc.setAttr(pin1+'.tangentAxis', 0) #X
        mc.connectAttr(bm+'Shape.worldMesh[0]', pin1+'.deformedGeometry')
        mc.connectAttr(bm+'ShapeOrig.outMesh', pin1+'.originalGeometry')    


def createMouthDrivers(names, curv, grpName):
    mc.group(em=True, n=grpName)
    curvShape = getset.getCrvShape(curv)
    cvs = getset.getCVs(curv)

    for num,cv in enumerate(cvs):
        mc.select(cl=True)
        poc1 = mc.createNode('pointOnCurveInfo')
        mc.connectAttr(curvShape+'.worldSpace[0]', poc1+'.inputCurve', f=True)
        mc.setAttr(poc1+'.turnOnPercentage', 1)
        mc.setAttr(poc1+'.parameter', num/11)
        jnt= mc.joint(rad=.5, n=names[num])
        mc.connectAttr(poc1+'.position', jnt+'.t')
        mc.parent(jnt, grpName)

def lipDrivers(names, pins, ctls, grpName):
    mc.group(em=True, n=grpName)

    for name,pin,ctl in zip(names,pins,ctls):  
        mc.select(cl=True)
        jnt = mc.joint(n=name, rad=.3)
        nul = mc.group(jnt, n=name+'_nul')
        mc.connectAttr(ctl+'_orient.r', nul+'.r')
        mc.connectAttr(pin+'.outputMatrix[0]', nul+'.offsetParentMatrix')
        mc.connectAttr(pin+'.outputMatrix[0]', ctl+'_nul'+'.offsetParentMatrix', force=True)
        util.connectTransform(ctl, jnt)
        mc.parent(nul, grpName)

###   
def createClsGrp(name, targ, bindmeshes, grpName, weights=[]):
    try:mc.select(grpName)
    except: mc.group(em=True, n=grpName)
    
    clus = mc.cluster(bindmeshes)[1]
    clus = mc.rename(clus, name)
    mc.connectAttr(targ+'.worldInverseMatrix', clus+'.offsetParentMatrix')
    mc.disconnectAttr(targ+'.worldInverseMatrix', clus+'.offsetParentMatrix')
    
    nul = mc.group(em=True, n=name+'_nul')
    mc.connectAttr(targ+'.worldMatrix', nul+'.offsetParentMatrix')
    mc.disconnectAttr(targ+'.worldMatrix', nul+'.offsetParentMatrix')
    
    mc.parent(clus, nul, r=True)
    mc.parent(nul ,grpName)
    
    if weights != []:
        for bm,val in zip(bindmeshes,weights):
            mc.select(bm+'.vtx[*]')
            mc.percent( name+'Cluster', v=val ) #set percents on the selected items to each value

        
def createBsCrv(guides, names, grpName):
    if not mc.ls(grpName): mc.group(em=True, n=grpName)

    for guide, name in zip(guides, names):
        upperCrv, lowerCrv = guide
        createMouthCrv(name, upperCrv, lowerCrv)
        mc.parent(name, grpName)

def createBsNode(name, orig, targList):
    bs = mc.blendShape(orig, n=name, o='local')[0] #o is origin
    for num,targ in enumerate(targList):
        mc.blendShape(bs, e=True, t=(orig, num, targ, 1.0))

def setBsCvWeight(bs):
    for i in range(8):
        for j in range(12):
            val=1
            if j in [3,9]: #middle vertex
                val=.5
            elif j in [0,1,2,10,11]: #left side vertex
                if 4<=i: #left side target
                    val=0
            elif j in [4,5,6,7,8]: #right side vertex
                if i<4: #right side target
                    val=0
            mc.setAttr(bs+'.inputTarget[0].inputTargetGroup[%d].targetWeights[%d]'%(i,j), val)
            
            
def symmetricMouthCrv(crvList):
    outList = []
    for crv in crvList:
        CVs = getset.getCVs(crv)
        posList = getset.getTransform(CVs, t=True, r=False, os=True)
        for num,i in enumerate(posList):
            if num<len(CVs)/4:
                x,y,z = i['pos']
                posList[len(CVs)//2-num]['pos'] = (-x,y,z)
            elif len(CVs)*3/4 < num:
                x,y,z = i['pos']
                posList[len(CVs)*3//2-num]['pos'] = (-x,y,z)
        outList += posList
    return outList

def connectBs(ctl, blendCrvs, bs):
    #rBlendCrvs = 
    #[ wideR smallR smileR frownR ]
    #[ 0     1     2      3      4 ]

    clp = mc.createNode('clamp')
    mc.setAttr(clp+'.maxR', 10)
    mc.setAttr(clp+'.minG', -10)
    mc.connectAttr(ctl+'.tx', clp+'.inputR')
    mc.connectAttr(ctl+'.tx', clp+'.inputG')
    mc.connectAttr(clp+'.outputR', bs+'.'+blendCrvs[1])
    
    mul = mc.createNode('multDoubleLinear')
    mc.setAttr(mul+'.input2', -1)
    mc.connectAttr(clp+'.outputG', mul+'.input1')
    mc.connectAttr(mul+'.output', bs+'.'+blendCrvs[0])
    ###
    rng = mc.createNode('setRange')
    mc.setAttr(rng+'.minX', 10)
    mc.setAttr(rng+'.oldMinX', -10)
    mc.setAttr(rng+'.maxY', 10)
    mc.setAttr(rng+'.oldMaxY', 10)
    mc.connectAttr(ctl+'.ty', rng+'.valueX')
    mc.connectAttr(ctl+'.ty', rng+'.valueY')
    mc.connectAttr(rng+'.outValueX', bs+'.'+blendCrvs[3])
    mc.connectAttr(rng+'.outValueY', bs+'.'+blendCrvs[2])
        
    
def matchCrvRtoL(posList):
    #actually, mouth_curve_r_wide & mouth_curve_l_wide are identical.
    #It doesn't mirror, it snaps CV positions
    rPosList = [d for d in posList if '_r_' in d['name']]
    lPosList = [d for d in posList if '_l_' in d['name']]
    for r,l in zip(rPosList, lPosList):
        l['pos'] = r['pos']
    outList = rPosList + lPosList
    return outList

def connectBigClus(C1, P1):
    dm1 = mc.createNode('decomposeMatrix')
    mc.connectAttr(P1+'.matrix', dm1+'.inputMatrix')
    mc.connectAttr(dm1+'.outputTranslate', C1+'.t')
    mc.connectAttr(dm1+'.outputRotate', C1+'.r')

def cornerCtls(ctl, clus, P1, P2):
    auto = ctl+'_auto'
    bm1 = mc.createNode('blendMatrix')
    mc.connectAttr(P2+'.matrix', bm1+'.target[0].targetMatrix')
    mc.connectAttr(P1+'.matrix', bm1+'.inputMatrix')
    mc.setAttr(bm1+'.target[0].weight', .5)
    dm1 = mc.createNode('decomposeMatrix')
    mc.connectAttr(bm1+'.outputMatrix', dm1+'.inputMatrix')
    mc.connectAttr(dm1+'.outputRotate', auto+'.r') #connect ctl auto
    mc.connectAttr(dm1+'.outputRotate', clus+'_nul'+'.r') #connect clus nul
    
    #add corner pin attr
    mc.addAttr(ctl, sn='cornerPin', k=True, dv=0, min=-1, max=1)
    #set range node
    ran1 = mc.createNode('setRange')
    #cornerPin >> valueX
    mc.connectAttr(ctl+'.cornerPin', ran1+'.valueX')
    # minX 0 maxX 1, oldMinX -1 oldMaxX 1
    mc.setAttr(ran1+'.minX', 0)
    mc.setAttr(ran1+'.maxX', 1)
    mc.setAttr(ran1+'.oldMinX', -1)
    mc.setAttr(ran1+'.oldMaxX', 1)
    #outValueX >> face_Lower_bindW0
    mc.connectAttr(ran1+'.outValueX', bm1+'.target[0].weight')






```

## Source file: Kaia_AutoRigger/data_.py

{
  "path": "Kaia_AutoRigger/data_.py",
  "blob_sha": "438e1f906cc57722c89ca99ade5c2ccfedb3a668",
  "sha256": "3117acdc99fd4e8ab210f4299105f22677cb17393c83c1c8d8efdb681e8dfd73",
  "lines": 96,
  "bytes": 3741,
  "selection_score": 3,
  "keyword_line_counts": {
    "blendshape": 6,
    "rig": 2
  },
  "url": "https://github.com/KaiaKim/autoRigger/blob/7998f1c2e54803d36f37cfc75e7653b5c1c19c52/Kaia_AutoRigger/data_.py",
  "function_signature_candidates": [
    {
      "line": 13,
      "text": "def setDir(self):"
    },
    {
      "line": 19,
      "text": "def getOrientsData(self):"
    },
    {
      "line": 24,
      "text": "def getBsCrvData(self):"
    },
    {
      "line": 33,
      "text": "def createDefaultData(self):"
    },
    {
      "line": 61,
      "text": "def importData(self):"
    },
    {
      "line": 72,
      "text": "def exportData(self):"
    },
    {
      "line": 85,
      "text": "def assignGeo(self,x,flag):"
    },
    {
      "line": 91,
      "text": "def assignVerts(self,x,flag):"
    }
  ]
}

```text

import maya.cmds as mc
import importlib
import json
from os.path import exists

from Kaia_AutoRigger.modules import getset
from Kaia_AutoRigger.modules import util

importlib.reload(getset)
importlib.reload(util)
###-----------------------------------------------------CLASS---------------------------------------------------
class ImportExport():
    def setDir(self):
        #get file path
        self.dir = mc.fileDialog2(
            fileFilter="*.json", dialogStyle=1, fm=3, caption='Set character data directory'
            )[0]

    def getOrientsData(self):
        #get ctrl orient data
        orients = [d+'_orient' for d in self.allCtls]
        self.data['orients'] = getset.getTransform(orients, t=False, r=True)
    
    def getBsCrvData(self):
        #get blendshape crv data
        allCVs = []
        for i in self.allCrv:
            CVs = getset.getCVs(i)
            allCVs += CVs
        self.data['bsCrv'] = getset.getTransform(allCVs, t=True, r=False, os=True)


    def createDefaultData(self):
        #if json files doesn't exist
        #notify users
        #write default json files
        missingFiles = []
        if exists(self.dir+'/geoNames.json') == False:
            missingFiles.append('geoNames.json')
            with open(self.dir+'/geoNames.json', "w") as wfile:
                json.dump(self.data['geo'], wfile)

        if exists(self.dir+'/vertIndices.json') == False:
            missingFiles.append('vertIndices.json')
            with open(self.dir+'/vertIndices.json',"w") as wfile:
                json.dump(self.data['verts'], wfile)

        if exists(self.dir+'/ctlOrient.json') == False:
            missingFiles.append('ctlOrient.json')
            with open(self.dir+'/ctlOrient.json',"w") as wfile:
                json.dump(self.data['orients'], wfile)

        if exists(self.dir+'/blendshapeCrv.json') == False:
            missingFiles.append('blendshapeCrv.json')
            with open(self.dir+'/blendshapeCrv.json',"w") as wfile:
                json.dump(self.data['bsCrv'], wfile)
        
        if missingFiles != []:
            mc.warning(', '.join([x for x in missingFiles])+' doesn\'t exist. Writing default files...')
        
    def importData(self):
        #read json files
        with open(self.dir + '/geoNames.json', "r") as rfile:
            self.data['geo'] = json.load(rfile)
        with open(self.dir + '/vertIndices.json', "r") as rfile:
            self.data['verts'] = json.load(rfile)
        with open(self.dir + '/ctlOrient.json', "r") as rfile:
            self.data['orients'] = json.load(rfile)
        with open(self.dir + '/blendshapeCrv.json', "r") as rfile:
            self.data['bsCrv'] = json.load(rfile)
    
    def exportData(self):
        #write json files
        with open(self.dir+'/geoNames.json', "w") as wfile:
            json.dump(self.data['geo'], wfile)
        with open(self.dir+'/vertIndices.json',"w") as wfile:
            json.dump(self.data['verts'], wfile)
        with open(self.dir+'/ctlOrient.json',"w") as wfile:
            json.dump(self.data['orients'], wfile)
        with open(self.dir+'/blendshapeCrv.json',"w") as wfile:
            json.dump(self.data['bsCrv'], wfile)

        mc.warning('Export Success! Directory: '+self.dir)

    def assignGeo(self,x,flag):
        if flag=='ass':
            self.data['geo'][x]=mc.ls(sl=True)
        if flag=='sel':
            mc.select(self.data['geo'][x])
        
    def assignVerts(self,x,flag):
        if flag=='ass':
            self.data['verts'][x]=mc.ls(sl=True,fl=True)
        elif flag=='sel':
            mc.select(self.data['verts'][x])
    

```

## Source file: Kaia_AutoRigger/namer.py

{
  "path": "Kaia_AutoRigger/namer.py",
  "blob_sha": "95c1a754808e60984bf7b6e2194818bf836a5cb3",
  "sha256": "4c7f42fa98b68971991b057394935cdc101a4b936d78490ee4409f014ce00955",
  "lines": 206,
  "bytes": 8780,
  "selection_score": 3,
  "keyword_line_counts": {
    "eye": 12,
    "jaw": 7,
    "mouth": 16,
    "teeth": 3,
    "tongue": 3,
    "lbs": 1,
    "rig": 1,
    "face": 5,
    "lip": 18
  },
  "url": "https://github.com/KaiaKim/autoRigger/blob/7998f1c2e54803d36f37cfc75e7653b5c1c19c52/Kaia_AutoRigger/namer.py",
  "function_signature_candidates": [
    {
      "line": 3,
      "text": "def __init__(self):"
    },
    {
      "line": 18,
      "text": "def __init__(self,verts):"
    },
    {
      "line": 59,
      "text": "def lipBinds(self,inList, prefix=''):"
    },
    {
      "line": 75,
      "text": "def __init__(self):"
    },
    {
      "line": 100,
      "text": "def __init__(self):"
    },
    {
      "line": 107,
      "text": "def __init__(self,verts):"
    },
    {
      "line": 152,
      "text": "def eyeBinds(self,inList, prefix=''):"
    },
    {
      "line": 164,
      "text": "def __init__(self):"
    },
    {
      "line": 176,
      "text": "def __init__(self):"
    },
    {
      "line": 192,
      "text": "def __init__(self):"
    }
  ]
}

```text

###-----------------------------------------------------CLASS---------------------------------------------------
class Face():
    def __init__(self):
        self.bind = 'face_bind'
        self.upBind = 'face_upper_bind' 
        self.jawTransBind = 'jaw_trans_bind'
        self.jawBind = 'jaw_bind'
        self.jawTipBind = 'jaw_tip_bind'
        self.binds = [self.bind, self.upBind, self.jawTransBind, self.jawBind, self.jawTipBind]
        
        self.root = 'face_ctl_root'
        self.upCtl = 'face_upper_ctl'
        self.jawCtl = 'jaw_ctl'
        self.ctls = [self.upCtl, self.jawCtl]


class Mouth():
    def __init__(self,verts):
        self.upGuide = 'lip_upper_guide'
        self.loGuide = 'lip_lower_guide'
        self.upCrv = 'lip_upper_curve' #No change this!
        self.loCrv = 'lip_lower_curve' #No change this!
        self.crv = 'mouth_curve' #No change this!

        self.upBindGrp = 'lip_upper_bind_grp' #Grp means group name
        self.loBindGrp = 'lip_lower_bind_grp'
        self.upBinds = self.lipBinds(verts['lipUpper'], prefix='lip_upper')
        self.loBinds = self.lipBinds(verts['lipLower'], prefix='lip_lower')
        self.binds = self.upBinds + self.loBinds
        
        mList = ['_corner_r','_upper_00_r','_upper_01_r','_upper_m','_upper_01_l','_upper_00_l','_corner_l','_lower_00_l','_lower_01_l','_lower_m','_lower_01_r','_lower_00_r']
        self.drivGrp = 'mouth_driver_grp'
        self.drivs = ['mouth'+d+'_driver' for d in mList]
        self.lipDrivGrp = 'lip_driver_grp'
        self.lipDrivs = ['lip'+d+'_driver' for d in mList]
        self.bindmeshesGrp = 'mouth_bindmesh_grp'
        self.bindmeshes = ['mouth'+d+'_bindmesh' for d in mList]
        self.uvPins = ['mouth'+d+'_uvPin' for d in mList]
        
        self.clusGrp = 'mouth_cls_grp'
        self.clus = 'mouth_cls'
        self.jawClus = ['jaw_upper_cls','jaw_lower_cls']
        self.cornerClus = ['mouth_corner_r_cls','mouth_corner_l_cls']

        self.ctl = 'mouth_ctl'
        self.lipCtlGrp = 'lip_ctl_grp'
        self.lipCtls =  ['lip'+d+'_ctl' for d in mList]
        self.cornerCtls = ['mouth_corner_r_ctl','mouth_corner_l_ctl']

        self.blendCrvGrp = 'mouth_BS_crv_grp'
        mbList = ['_wide','_small','_smile','_frown']
        self.blendGuides = [('lip_upper'+d+'_guide','lip_lower'+d+'_guide') for d in mbList]
        self.rBlendCrvs = ['mouth_r'+d+'_BSCurve' for d in mbList]
        self.lBlendCrvs = ['mouth_l'+d+'_BSCurve' for d in mbList]
        self.blendCrvs = self.rBlendCrvs + self.lBlendCrvs
        self.bsNode = 'mouth_curve_BS'


    def lipBinds(self,inList, prefix=''):
        outList = []
        for i in range(len(inList)):
            name = prefix
            if i==((len(inList)-1)/2):
                name += '_m_'
            elif i<((len(inList)-1)/2):
                name += '_r_%02d'%i #result: 'lip_lower_l_00_loc'
            elif i>((len(inList)-1)/2):
                name += '_l_%02d'%( (len(inList)-1)-i ) #result: 'lip_lower_r_00_loc'
            name += '_bind'
            
            outList.append(name) #append the loc name to self.lipLocs list
        return outList

class Tongue():
    def __init__(self):
        prefix = 'tongue_'

        tList=['_01','_02','_03','_04','_05','_06','_07',]
        self.binds = ['tongue'+d+'_bind' for d in tList]
        ###
        self.crv = prefix + 'spineIk_Crv'
        self.ikHand = prefix + 'spineIk'
        self.ikEff = prefix + 'effector'
        self.upObj = prefix + 'upObj'
        
        section = 2
        nameRef = ['start']+['%02d'%d for d in range(section)]+['end']
        nameRef = [prefix + d for d in nameRef]
        
        self.clsGrp = prefix + 'cls_Grp'
        self.clsNodes = [d+'_cls' for d in nameRef]
        self.clsHandles = [d+'_clsHandle' for d in nameRef]
        
        self.fkCtls = [d+'_FK_ctl' for d in nameRef]
        self.ikCtls = [d+'_IK_ctl' for d in nameRef]
        self.ctls = self.fkCtls + self.ikCtls


class Teeth():
    def __init__(self):
        self.binds = ['teeth_upper_bind','teeth_lower_bind']
        self.ctls = ['teeth_upper_ctl','teeth_lower_ctl']
        


class Lid():
    def __init__(self,verts):
        self.guides = ['lid_upper_r_guide','lid_lower_r_guide','lid_mid_r_guide']
        lList = ['_upper_r', '_lower_r', '_upper_l', '_lower_l']
        self.crvs = ['lid'+d+'_curve' for d in lList] #No change this!
        self.drivCrvs = ['lid'+d+'_driver_curve' for d in lList] #lid driver curve

        self.ctlGrp = 'blink_ctl_grp'
        self.ctls = ['blink'+d+'_ctl' for d in lList]  
        
        self.drivGrp = 'blink_driver_grp'
        self.drivs = [['blink%s_%d_driver'%(i,d) for d in range(5)] for i in lList]
        self.lidDrivGrp = 'lid_driver_grp'
        self.lidDrivs = [['lid%s_%d_driver'%(i,d) for d in range(5)] for i in lList]
        self.microCtlGrp = 'lid_micro_ctl_grp'
        self.microCtls = [['lid%s_%d_ctl'%(i,d) for d in range(5)] for i in lList]
        
        self.rBindGrp = 'lid_r_bind_grp'
        self.lBindGrp = 'lid_l_bind_grp'
        self.upperRBinds = self.eyeBinds(verts['lidUpperR'], prefix='lid_upper_r')
        self.upperLBinds = self.eyeBinds(verts['lidUpperR'], prefix='lid_upper_l') #we're just getting len of the list. left&right doesn't matter
        self.lowerRBinds = self.eyeBinds(verts['lidLowerR'], prefix='lid_lower_r')
        self.lowerLBinds = self.eyeBinds(verts['lidLowerR'], prefix='lid_lower_l')
        self.rBinds = self.upperRBinds + self.lowerRBinds
        self.lBinds = self.upperLBinds + self.lowerLBinds
        
        self.blendCrvGrp = 'lid_BS_crv_grp'
        
        self.blendGuides = [
            ['lid_upper_r_open_guide','lid_upper_r_guide','lid_mid_r_guide','lid_lower_r_guide'],
            ['lid_lower_r_open_guide', 'lid_lower_r_guide', 'lid_mid_r_guide', 'lid_upper_r_guide']
        ]
        suffixList = ['_open','_neutral', '_mid', '_closed']
        self.blendCrvs = [
            [self.crvs[0]+d for d in suffixList],
            [self.crvs[1]+d for d in suffixList],
            [self.crvs[2]+d for d in suffixList],
            [self.crvs[3]+d for d in suffixList]
        ]
        self.rBlendCrvs = self.blendCrvs[0]+self.blendCrvs[1]
        self.lBlendCrvs = self.blendCrvs[2]+self.blendCrvs[3]
        
        nodeNames = ['upper_r_open', 'upper_r_closed', 'lower_r_open', 'lower_r_closed']
        self.rBsNodes = ['lid_'+d+'_BS' for d in nodeNames]
        self.lBsNodes = [d.replace('_r_', '_l_') for d in self.rBsNodes]
        
    def eyeBinds(self,inList, prefix=''):
        outList = []
        for i in range(len(inList)):
            name = prefix
            name += '_%02d' %i #'lower_lid_00_l_bind'
            name += '_bind'
            
            outList.append(name) #append the bind name to self.lipBinds list
        return outList


class Eye():
    def __init__(self):
        self.socBinds = ['eye_socket_r_bind','eye_socket_l_bind'] 
        self.binds = ['eye_r_bind','eye_l_bind']
            ###
        self.rotCtl = 'eye_rot_ctl'
        self.ctls = ['eye_r_ctl','eye_l_ctl']
        self.aimCtl = 'eye_aim_ctl'
        self.aimMicroCtls = ['eye_r_aim_ctl','eye_l_aim_ctl']
            ###


class Brow():
    def __init__(self):
        self.bigBinds = ['brow_r_bind','brow_l_bind']
        self.inBinds = ['brow_inner_r_bind','brow_inner_l_bind']
        self.peakBinds = ['brow_peak_r_bind','brow_peak_l_bind']
        self.corBinds = ['brow_corrugator_r_bind','brow_corrugator_l_bind']
        self.binds = self.bigBinds + self.inBinds + self.corBinds + self.peakBinds
        
        self.ctlGrp = 'brow_ctl_grp'
        self.bigCtls = ['brow_r_ctl','brow_l_ctl']
        self.inCtls = ['brow_inner_r_ctl','brow_inner_l_ctl']
        self.corCtls = ['brow_corrugator_r_ctl','brow_corrugator_l_ctl']
        self.peakCtls = ['brow_peak_r_ctl','brow_peak_l_ctl']
        self.ctls = self.bigCtls + self.inCtls + self.corCtls + self.peakCtls

        
class Nose():
    def __init__(self):
        self.tipBind = 'nose_bind'
        self.bridgeBind = 'nose_bridge_bind'
        self.sneerBinds = ['sneer_r_bind','sneer_l_bind']
        self.nostrilBinds = ['r','l'] ###whatever name
        self.binds = [self.tipBind,self.bridgeBind] + self.sneerBinds
        
        self.ctlGrp = 'nose_ctl_grp'
        self.bridgeCtl = 'noseBridge_ctl'
        self.ctl = 'nose_ctl'
        self.sneerCtls = ['nose_sneer_r_ctl', 'nose_sneer_l_ctl']

        self.ctls = [self.bridgeCtl, self.ctl] + self.sneerCtls

        


```
