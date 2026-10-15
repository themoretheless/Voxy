# Facial rendering source scan: miqianmimi/Deformation-On-Human-Face

Repository: https://github.com/miqianmimi/Deformation-On-Human-Face

Pinned source revision: a811a84ff3dcb360cefd15097de8536352e45fc0

Discovery topic: face deformation

Description: mainly focus on the conversion from the human-being’s face to the baboon’s face.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"face": ["code/facepp.py"]}

## Source file: code/facepp.py

{
  "path": "code/facepp.py",
  "blob_sha": "3057b9abb606b911d06d8a5b0222d434e5aed047",
  "sha256": "757d6b4cfa55f5b0f6aaadf926aa3f9fbc6c73180f13c5f3d15be6926c4b3b82",
  "lines": 278,
  "bytes": 7789,
  "selection_score": 15,
  "keyword_line_counts": {
    "gum": 3,
    "face": 12
  },
  "url": "https://github.com/miqianmimi/Deformation-On-Human-Face/blob/a811a84ff3dcb360cefd15097de8536352e45fc0/code/facepp.py",
  "function_signature_candidates": [
    {
      "line": 32,
      "text": "def __init__(self, path):"
    },
    {
      "line": 36,
      "text": "def _get_content(self):"
    },
    {
      "line": 45,
      "text": "def get_filename(self):"
    },
    {
      "line": 59,
      "text": "def __init__(self, code, url, body):"
    },
    {
      "line": 64,
      "text": "def __str__(self):"
    },
    {
      "line": 80,
      "text": "def __init__(self, key, secret, srv=None,"
    },
    {
      "line": 102,
      "text": "def update_request(self, request):"
    },
    {
      "line": 108,
      "text": "def _setup_apiobj(self, api, path):"
    },
    {
      "line": 130,
      "text": "def __init__(self, api, path):"
    },
    {
      "line": 133,
      "text": "def __call__(self, *args, **kargs):"
    },
    {
      "line": 174,
      "text": "def _mkarg(self, kargs):"
    },
    {
      "line": 177,
      "text": "def enc(x):"
    },
    {
      "line": 201,
      "text": "def __init__(self):"
    },
    {
      "line": 207,
      "text": "def get_content_type(self):"
    },
    {
      "line": 210,
      "text": "def add_field(self, name, value):"
    },
    {
      "line": 215,
      "text": "def add_file(self, fieldname, filename, content, mimetype=None):"
    },
    {
      "line": 222,
      "text": "def __str__(self):"
    },
    {
      "line": 258,
      "text": "def _print_debug(msg):"
    }
  ]
}

```text

# -*- coding: utf-8 -*-

"""a simple facepp sdk
example:
api = API(key, secret)
api.detect(img = File('/tmp/test.jpg'))"""

__all__ = ['File', 'APIError', 'API']


DEBUG_LEVEL = 1

import sys
import socket
import urllib2
import json
import os.path
import itertools
import mimetools
import mimetypes
import time
import tempfile
from collections import Iterable


class File(object):

    """an object representing a local file"""
    path = None
    content = None

    def __init__(self, path):
        self.path = path
        self._get_content()

    def _get_content(self):
        """read image content"""

        if os.path.getsize(self.path) > 2 * 1024 * 1024:
            raise APIError(-1, None, 'image file size too large')
        else:
            with open(self.path, 'rb') as f:
                self.content = f.read()

    def get_filename(self):
        return os.path.basename(self.path)


class APIError(Exception):
    code = None
    """HTTP status code"""

    url = None
    """request URL"""

    body = None
    """server response body; or detailed error information"""

    def __init__(self, code, url, body):
        self.code = code
        self.url = url
        self.body = body

    def __str__(self):
        return 'code={s.code}\nurl={s.url}\n{s.body}'.format(s=self)

    __repr__ = __str__


class API(object):
    key = None
    secret = None
    server = 'https://api-cn.faceplusplus.com/facepp/v3/'

    decode_result = True
    timeout = None
    max_retries = None
    retry_delay = None

    def __init__(self, key, secret, srv=None,
                 decode_result=True, timeout=30, max_retries=10,
                 retry_delay=5):
        """:param srv: The API server address
        :param decode_result: whether to json_decode the result
        :param timeout: HTTP request timeout in seconds
        :param max_retries: maximal number of retries after catching URL error
            or socket error
        :param retry_delay: time to sleep before retrying"""
        self.key = key
        self.secret = secret
        if srv:
            self.server = srv
        self.decode_result = decode_result
        assert timeout >= 0 or timeout is None
        assert max_retries >= 0
        self.timeout = timeout
        self.max_retries = max_retries
        self.retry_delay = retry_delay

        _setup_apiobj(self, self, [])

    def update_request(self, request):
        """overwrite this function to update the request before sending it to
        server"""
        pass


def _setup_apiobj(self, api, path):
    if self is not api:
        self._api = api
        self._urlbase = api.server + '/'.join(path)

    lvl = len(path)
    done = set()
    for i in _APIS:
        if len(i) <= lvl:
            continue
        cur = i[lvl]
        if i[:lvl] == path and cur not in done:
            done.add(cur)
            setattr(self, cur, _APIProxy(api, i[:lvl + 1]))


class _APIProxy(object):
    _api = None
    """underlying :class:`API` object"""

    _urlbase = None

    def __init__(self, api, path):
        _setup_apiobj(self, api, path)

    def __call__(self, *args, **kargs):
        if len(args):
            raise TypeError('Only keyword arguments are allowed')
        form = _MultiPartForm()
        for (k, v) in kargs.iteritems():
            if isinstance(v, File):
                form.add_file(k, v.get_filename(), v.content)

        url = self._urlbase
        for k, v in self._mkarg(kargs).iteritems():
            form.add_field(k, v)

        request = urllib2.Request(url)
        body = str(form)
        request.add_header('Content-type', form.get_content_type())
        request.add_header('Content-length', str(len(body)))
        request.add_data(body)

        self._api.update_request(request)

        retry = self._api.max_retries
        while True:
            retry -= 1
            try:
                ret = urllib2.urlopen(request, timeout=self._api.timeout).read()
                break
            except urllib2.HTTPError as e:
                raise APIError(e.code, url, e.read())
            except (socket.error, urllib2.URLError) as e:
                if retry < 0:
                    raise e
                _print_debug('caught error: {}; retrying'.format(e))
                time.sleep(self._api.retry_delay)

        if self._api.decode_result:
            try:
                ret = json.loads(ret)
            except:
                raise APIError(-1, url, 'json decode error, value={0!r}'.format(ret))
        return ret

    def _mkarg(self, kargs):
        """change the argument list (encode value, add api key/secret)
        :return: the new argument list"""
        def enc(x):
            if isinstance(x, unicode):
                return x.encode('utf-8')
            return str(x)

        kargs = kargs.copy()
        kargs['api_key'] = self._api.key
        kargs['api_secret'] = self._api.secret
        for (k, v) in kargs.items():
            if isinstance(v, Iterable) and not isinstance(v, basestring):
                kargs[k] = ','.join([enc(i) for i in v])
            elif isinstance(v, File) or v is None:
                del kargs[k]
            else:
                kargs[k] = enc(v)

        return kargs


# ref: http://www.doughellmann.com/PyMOTW/urllib2/
class _MultiPartForm(object):

    """Accumulate the data to be used when posting a form."""

    def __init__(self):
        self.form_fields = []
        self.files = []
        self.boundary = mimetools.choose_boundary()
        return

    def get_content_type(self):
        return 'multipart/form-data; boundary=%s' % self.boundary

    def add_field(self, name, value):
        """Add a simple field to the form data."""
        self.form_fields.append((name, value))
        return

    def add_file(self, fieldname, filename, content, mimetype=None):
        """Add a file to be uploaded."""
        if mimetype is None:
            mimetype = mimetypes.guess_type(filename)[0] or 'application/octet-stream'
        self.files.append((fieldname, filename, mimetype, content))
        return

    def __str__(self):
        """Return a string representing the form data, including attached files."""
        # Build a list of lists, each containing "lines" of the
        # request.  Each part is separated by a boundary string.
        # Once the list is built, return a string where each
        # line is separated by '\r\n'.
        parts = []
        part_boundary = '--' + self.boundary

        # Add the form fields
        parts.extend(
            [part_boundary,
             'Content-Disposition: form-data; name="%s"' % name,'',value, ]
            for name, value in self.form_fields
        )

        # Add the files to upload
        parts.extend(
            [part_boundary,
             'Content-Disposition: file; name="%s"; filename="%s"' %
             (field_name, filename),
             'Content-Type: %s' % content_type,
             '',
             body,
             ]
            for field_name, filename, content_type, body in self.files
        )

        # Flatten the list and add closing boundary marker,
        # then return CR+LF separated data
        flattened = list(itertools.chain(*parts))
        flattened.append('--' + self.boundary + '--')
        flattened.append('')
        return '\r\n'.join(flattened)


def _print_debug(msg):
    if DEBUG_LEVEL:
        sys.stderr.write(str(msg) + '\n')

_APIS = [
    '/detect',
    '/compare',
    '/search',
    '/faceset/create',
    '/faceset/addface',
    '/faceset/removeface',
    '/faceset/update',
    '/faceset/getdetail',
    '/faceset/delete',
    '/faceset/getfacesets',
    '/face/analyze',
    '/face/getdetail',
    '/face/setuserid'
]

_APIS = [i.split('/')[1:] for i in _APIS]


```

## Source file: code/run.py

{
  "path": "code/run.py",
  "blob_sha": "d3038715b497f47f5060dc4992fb113ff01af9fc",
  "sha256": "774f2add1f1939ee6e37a6644177db251d257ac5e9ddc561bc6e516a2b6fc612",
  "lines": 312,
  "bytes": 13840,
  "selection_score": 0,
  "keyword_line_counts": {
    "deform": 1,
    "rig": 3,
    "face": 11
  },
  "url": "https://github.com/miqianmimi/Deformation-On-Human-Face/blob/a811a84ff3dcb360cefd15097de8536352e45fc0/code/run.py",
  "function_signature_candidates": [
    {
      "line": 21,
      "text": "def rgb2bgr(img):"
    },
    {
      "line": 28,
      "text": "def __init__(self,oriPath,protoPath,oriPointsPath,protoPointsPath,regionsPath):"
    },
    {
      "line": 48,
      "text": "def initUI(self):"
    },
    {
      "line": 178,
      "text": "def detectKeyPoints(self):"
    },
    {
      "line": 183,
      "text": "def saveImg(self):"
    },
    {
      "line": 193,
      "text": "def transformAction(self):"
    },
    {
      "line": 240,
      "text": "def showProtoDialog(self):"
    },
    {
      "line": 247,
      "text": "def showLoadOriDialog(self):"
    },
    {
      "line": 254,
      "text": "def showLoadRegionDialog(self):"
    },
    {
      "line": 261,
      "text": "def showLoadProtoDialog(self):"
    },
    {
      "line": 269,
      "text": "def showOriDialog(self):"
    },
    {
      "line": 280,
      "text": "def changeEValue(self,x):"
    },
    {
      "line": 285,
      "text": "def changeAlphaValue(self,x):"
    },
    {
      "line": 292,
      "text": "def affineChoiceChange(self,item):"
    },
    {
      "line": 299,
      "text": "def distanceChoiceChange(self,item):"
    }
  ]
}

```text

#!/usr/bin/python3
import sys
import numpy as np
from PIL import Image
#import cv2

#from morphing import morphingAction
from matplotlib.backends.backend_agg import FigureCanvasAgg as FigureCanvas
from matplotlib.figure import Figure
import matplotlib.image as mpimg
import matplotlib.pyplot as plt

from PySide.QtGui import (QWidget, QToolTip, 
    QPushButton, QApplication,QLabel,QHBoxLayout,QVBoxLayout,QComboBox,QSlider)
from PySide import QtGui
from PySide.QtGui import QIcon,QFont,QPixmap,QImage
from PySide.QtCore import QCoreApplication,Qt

from transformation import local_affine_transformation,affine_points_MLS,preprocess
from util import *
def rgb2bgr(img):
        b,g,r,a = cv2.split(img)
        return cv2.merge([r,g,b,a])

#The GUI class we construct.
class TransformationFaceUI(QWidget):

    def __init__(self,oriPath,protoPath,oriPointsPath,protoPointsPath,regionsPath):
        super(TransformationFaceUI,self).__init__()
        #Initialize the data or object we need
        
        localParas,points = preprocess(oriPath,protoPath,oriPointsPath,protoPointsPath,regionsPath,'L2')
        self.oriImg,self.protoImg,self.regionsPoints,self.is_in_regions_fun,self.distance_funs,self.affine_funs = localParas
        self.oriPlotDict,self.protoPlotDict,self.oriPoints ,self.protoPoints = points
        self.oriPoints = np.array(self.oriPoints)
        self.protoPoints = np.array(self.protoPoints)
        self.e = 2
        self.alpha = 1
        self.oriPath = oriPath
        self.protoPath = protoPath
        self.oriPointsPath = oriPointsPath
        self.protoPointsPath=protoPointsPath
        self.regionsPath = regionsPath
        self.transform = 'Local Affine Transformation'
        self.newImg = None
        self.initUI()

    def initUI(self):
        QToolTip.setFont(QFont('SansSerif', 10))
        self.setGeometry(300, 200, 810, 430)
        self.setWindowTitle('Transformation on Human Face')

       
        #Method choose combobox
        self.comboAffineLabel = QLabel(self)
        self.comboAffineLabel.setText('Transformation Method:')
        self.comboAffineLabel.setGeometry(60,270,230,30)

        self.comboAffine = QComboBox(self)
        self.comboAffine.addItem("Local Affine Transformation")
        self.comboAffine.addItem("Moving Least Squares")
        self.comboAffine.addItem("Morphing")
        self.comboAffine.setGeometry(22,290,225,30)
        self.comboAffine.activated[str].connect(self.affineChoiceChange)   

        #The button to choose the original figure
        self.oriBtn = QPushButton('Choose Original Picture', self)
        self.oriBtn.setToolTip('Choose Original Picture')
        self.oriBtn.setGeometry(20,330,230,30)
        self.oriBtn.clicked.connect(self.showOriDialog)

        #The button to choose the proto figure
        self.protoBtn = QPushButton('Choose Proto Picture', self)
        self.protoBtn.setToolTip('Choose Proto Picture')
        self.protoBtn.setGeometry(20,365,230,30)
        self.protoBtn.clicked.connect(self.showProtoDialog)

        #The distance function choose combobox
        self.comboLabel = QLabel(self)
        self.comboLabel.setText('Distance Fun:')
        self.comboLabel.setGeometry(310,280,200,30)

        self.comboDis = QComboBox(self)
        self.comboDis.addItem("L2")
        self.comboDis.addItem("L1")
        self.comboDis.setGeometry(410,280,80,30)

        self.comboDis.activated[str].connect(self.distanceChoiceChange)   

        #E choose slider
        
        self.eLabel = QLabel(self)
        self.eLabel.setText('E Value:0.00')
        self.eLabel.setGeometry(310,320,200,30)
        self.eSld = QSlider(Qt.Horizontal, self )
        self.eSld.setRange(0,10**5)
        self.eSld.setFocusPolicy(Qt.NoFocus)
        self.eSld.setGeometry(390,320,120,30)
        self.eSld.valueChanged[int].connect(self.changeEValue)

        #alpha choose slider
        self.aLabel = QLabel(self)
        self.aLabel.setText('Alpha Value:0.00')
        self.aLabel.setGeometry(310,360,200,30)
        self.aSld = QSlider(Qt.Horizontal, self)
        self.aSld.setRange(0,10**5)
        self.aSld.setFocusPolicy(Qt.NoFocus)
        self.aSld.setGeometry(410,360, 100, 30)
        self.aSld.valueChanged[int].connect(self.changeAlphaValue)
        
        # Picture show 
        self.oriTextLabel = QLabel(self)
        self.protoTextLabel = QLabel(self)
        self.transTextLabel = QLabel(self)
        self.oriTextLabel.setText('The Orginal Picture')
        self.protoTextLabel.setText('The Proto Picture')
        self.transTextLabel.setText('The Picture after Transformation')

        self.oriTextLabel.move(70,5)
        self.protoTextLabel.move(350,5)
        self.transTextLabel.move(580,5)

        self.oriLabel = QLabel(self)
        self.protoLabel = QLabel(self)
        self.transLabel = QLabel(self)

        pixmap = QPixmap(self.oriPath)
        pixmap2 = QPixmap(self.protoPath)
        self.oriLabel.setPixmap(pixmap)
        self.protoLabel.setPixmap(pixmap2)
        self.transLabel.setPixmap(pixmap)

        #Position setting
        self.oriLabel.setGeometry(20,20,230,230)
        self.protoLabel.setGeometry(290,20,230,230)
        self.transLabel.setGeometry(560,20,230,230)
        self.oriLabel.setScaledContents(True)
        self.protoLabel.setScaledContents(True)
        self.transLabel.setScaledContents(True)
        #Load button
        self.loadOriBtn = QPushButton('Load Ori Points', self)
        self.loadOriBtn.setToolTip('Load Control Points From Txt File')
        self.loadOriBtn.setGeometry(550,280,130,30)
        self.loadOriBtn.clicked.connect(self.showLoadOriDialog)

        self.loadProtoBtn = QPushButton('Load Proto Points', self)
        self.loadProtoBtn.setToolTip('Load Control Points From Txt File')
        self.loadProtoBtn.setGeometry(680,280,130,30)
        self.loadProtoBtn.clicked.connect(self.showLoadProtoDialog)
        #Face ++ button
        self.faceBtn = QPushButton('Face++ Keypoint', self)
        self.faceBtn.setToolTip('Save the Face++ Keypoints')
        self.faceBtn.setGeometry(550,315,130,30)
        self.faceBtn.clicked.connect(self.detectKeyPoints)
        #Load region Button
        self.loadRegionBtn = QPushButton('Load Regions', self)
        self.loadRegionBtn.setToolTip('Load Regions From Txt File')
        self.loadRegionBtn.setGeometry(680,315,130,30)
        self.loadRegionBtn.clicked.connect(self.showLoadRegionDialog)


        #Save Button setting
        self.saveBtn = QPushButton('Save', self)
        self.saveBtn.setToolTip('Transform this picture to the shape of Baboon')
        self.saveBtn.setGeometry(560, 350, 110, 40)
        self.saveBtn.clicked.connect(self.saveImg)

        #Transform action button
        self.confirmBtn = QPushButton('Generate', self)
        self.confirmBtn.setToolTip('Generate')
        self.confirmBtn.setGeometry(680, 350, 110, 40)
        self.confirmBtn.clicked.connect(self.transformAction)


        self.show()

    #Invoke face++ and save which is connected to the facebtn
    def detectKeyPoints(self):
        print(self.oriPath)
        save_points('face_keypoints_ori.txt', detect((self.oriPath).encode('utf-8')))

    #Save img connected to the save button
    def saveImg(self):
        if self.newImg == None:
            QtGui.QMessageBox.information(self, "Error", "There is not transformed figure")
        result = Image.fromarray(self.newImg)
        filenames = self.oriPath.split('/')
        filenames[len(filenames)-1] = 'trans_'+filenames[len(filenames)-1]
        newPath = '/'.join(filenames)
        result.save(newPath)

    #Connected to the transform button did the deformation and show it 
    def transformAction(self):
        
        try:
            #For different methods have different solution
            if self.transform == 'Morphing':
                self.oriImg = np.array(Image.open(self.oriPath))
                self.protoImg = np.array(Image.open(self.protoPath))
                if self.oriImg.shape != self.protoImg.shape:
                    QtGui.QMessageBox.information(self, "Error", "It is image morphing and required the same size of two images,please choose other images")
                    return
                newImg = morphingAction((self.oriPath).encode('utf-8'),self.protoPath.encode('utf-8'),self.protoPointsPath.encode('utf-8'),self.alpha)
            else:
                localParas,points = preprocess(self.oriPath,self.protoPath,self.oriPointsPath,self.protoPointsPath,self.regionsPath,'L2')
                if points == None:
                    QtGui.QMessageBox.information(self, "Error", localParas)
                    return 
                self.oriImg,self.protoImg,self.regionsPoints,self.is_in_regions_fun,self.distance_funs,self.affine_funs = localParas
                self.oriPlotDict,self.protoPlotDict,self.oriPoints ,self.protoPoints = points
                if self.oriImg.shape[len(self.oriImg.shape)-1] != self.protoImg.shape[len(self.protoImg.shape)-1]:
                    QtGui.QMessageBox.information(self, "Error", "The type of the figures is not the same, please choose another figure")
                    return
                if self.transform == 'Local Affine Transformation':
                    newImg = local_affine_transformation(self.oriImg,self.protoImg,self.e,self.regionsPoints,self.is_in_regions_fun,self.distance_funs,self.affine_funs)
                elif self.transform == "Moving Least Squares":
                    newImg = affine_points_MLS(self.oriImg,self.protoImg,self.oriPoints,self.protoPoints,self.alpha)
        except BaseException :
            
            QtGui.QMessageBox.information(self, "Error", "There are error in the point choice or other things.")
            newImg = morphingAction((self.oriPath).encode('utf-8'),self.protoPath.encode('utf-8'),self.protoPointsPath.encode('utf-8'),self.alpha)

        
        self.newImg = np.uint8(newImg)
        newImg=rgb2bgr(np.uint8(newImg))

        qimage = QImage(newImg,newImg.shape[1],newImg.shape[0],QImage.Format_ARGB32)
        pixmap_array = QPixmap.fromImage(qimage)
        self.transLabel.setPixmap(pixmap_array)
################################################################################################
################################################################################################
################################################################################################
################################################################################################
################################################################################################
################################################################################################
################################################################################################
################################################################################################

    #The file chooser , all of the function begin with show is the same but for different parameter.
    def showProtoDialog(self):

        fname, _ = QtGui.QFileDialog.getOpenFileName(self, 'Open file',
                    '/home',"Image Files (*.png *.jpg *.bmp)")
        if  fname != None and fname != '':
            self.protoPath = fname
            self.protoLabel.setPixmap(QPixmap(self.protoPath))
    def showLoadOriDialog(self):

        fname, _ = QtGui.QFileDialog.getOpenFileName(self, 'Open file',
                    '/home',"Text files (*.txt)")
        if  fname != None and fname != '':
            self.oriPointsPath = fname

    def showLoadRegionDialog(self):

        fname, _ = QtGui.QFileDialog.getOpenFileName(self, 'Open file',
                    '/home',"Text files (*.txt)")
        if  fname != None and fname != '':
            self.regionsPath = fname
    
    def showLoadProtoDialog(self):

        fname, _ = QtGui.QFileDialog.getOpenFileName(self, 'Open file',
                    '/home',"Text files (*.txt)")
        if  fname != None and fname != '':
            self.protoPointsPath = fname
        
    
    def showOriDialog(self):

        fname, _ = QtGui.QFileDialog.getOpenFileName(self, 'Open file',
                    '/home',"Image Files (*.png *.jpg *.bmp)")
        if  fname != None and fname != '':
            self.oriPath = fname
            print(self.oriPath)
            self.oriLabel.setPixmap(QPixmap(self.oriPath))

        
    #Connected with eSld change the e value in LAT method    
    def changeEValue(self,x):
        self.e = 4.0*x/10**5
        self.eLabel.setText('E Value:'+str( '%.2f' % self.e))

    #Connected with aSld change the alpha value in MLS and Morphing method(but the alpha range is not the same)   
    def changeAlphaValue(self,x):
        if self.transform == 'Moving Least Squares':
            self.alpha = 2.0*x/10**5
        elif self.transform == 'Morphing':
            self.alpha = 1.0*x/10**5
        self.aLabel.setText('Alpha Value:'+str('%.2f' % self.alpha))
    #Connected to the method combobox
    def affineChoiceChange(self,item):
        
        if self.transform in ['Moving Least Squares','Morphing'] and item in ['Moving Least Squares','Morphing'] and item != self.transform:
            self.alpha = 0.0
            self.aSld.setValue(self.alpha)
        self.transform = item
     #Connected to the distance combobox    
    def distanceChoiceChange(self,item):
        self.oriImg,self.protoImg,self.protoPoints,self.affine_funs = preprocess(oriPath,protoPath,oriPointsPath,protoPointsPath,item)



#run 
if __name__ == '__main__':
    #app = QApplication(sys.argv)
    oriPath = 'data/zxh151.png'
    protoPath = 'data/ape.png'
    ex = TransformationFaceUI(oriPath,protoPath,'data/zxh.txt','data/ape.txt','data/regions.txt')
    
    sys.exit(app.exec_())
    


```

## Source file: code/util.py

{
  "path": "code/util.py",
  "blob_sha": "aad869ad8eabffe5c57f0eb5654ab3c56a86d831",
  "sha256": "8f5d41cbe02a03336c23bd47d437625a05223d856ae44f641716155ca164fb49",
  "lines": 210,
  "bytes": 6151,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 1
  },
  "url": "https://github.com/miqianmimi/Deformation-On-Human-Face/blob/a811a84ff3dcb360cefd15097de8536352e45fc0/code/util.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def is_in_regions_fun(p1,regions):"
    },
    {
      "line": 25,
      "text": "def sign(p1,p2,p3):"
    },
    {
      "line": 29,
      "text": "def is_in_triangle(p1,region):"
    },
    {
      "line": 35,
      "text": "def find_index(v,proto_points):"
    },
    {
      "line": 41,
      "text": "def weight_mls(points,v,alpha):"
    },
    {
      "line": 45,
      "text": "def distance_l2(x,y):"
    },
    {
      "line": 49,
      "text": "def distance_l1(x,y):"
    },
    {
      "line": 53,
      "text": "def line_distance(p1,p2,p3):"
    },
    {
      "line": 58,
      "text": "def linear_affine_fun(bias):"
    },
    {
      "line": 59,
      "text": "def linear_affine_tmp(x):"
    },
    {
      "line": 64,
      "text": "def similarity_fun(src,dst):"
    },
    {
      "line": 66,
      "text": "def affine_fun(x):"
    },
    {
      "line": 71,
      "text": "def affine_fun(src,dst):"
    },
    {
      "line": 73,
      "text": "def affine_fun(x):"
    },
    {
      "line": 78,
      "text": "def distance_fun(region,item):"
    },
    {
      "line": 83,
      "text": "def distance(p1):"
    },
    {
      "line": 95,
      "text": "def is_in_parallel(p1,p2,p3):"
    },
    {
      "line": 104,
      "text": "def distance_line_fun(line):"
    },
    {
      "line": 105,
      "text": "def distance(p1):"
    },
    {
      "line": 116,
      "text": "def weight(x,distance_funs,e):"
    },
    {
      "line": 123,
      "text": "def transform(x,weights,affine_funs):"
    },
    {
      "line": 127,
      "text": "def line_points(start,end,n):"
    },
    {
      "line": 135,
      "text": "def save_points(name,new_dict):"
    },
    {
      "line": 141,
      "text": "def load_data(path):"
    },
    {
      "line": 153,
      "text": "def plot(img,points_dict):"
    },
    {
      "line": 175,
      "text": "def linear_interpolation(location,img):"
    },
    {
      "line": 192,
      "text": "def load_region(path):"
    },
    {
      "line": 200,
      "text": "def dictToList(oriDict,protoDict):"
    }
  ]
}

```text

import numpy as np
from PIL import Image
import matplotlib.pyplot as plt
from math import floor
from skimage import transform as tf
from numpy.linalg import norm
from numpy.linalg import inv


#Test whether a point is in a region
def is_in_regions_fun(p1,regions):
    for i in range(len(regions)):
        
        if len(regions[i]) == 1:
            if (p1== regions[i][0]).all():
                return i
        elif len(regions[i]) == 2:
            if (p1== regions[i][0]).all() or (p1==regions[i][1]).all()  :
                return i
        elif len(regions[i]) == 3:
            if is_in_triangle(p1,regions[i]):
                return i
    return -1    

def sign(p1,p2,p3):
    return (p1[0]-p3[0])*(p2[1]-p3[1])-(p2[0]-p3[0])*(p1[1]-p1[1])

#Test whether a point is in a triangle
def is_in_triangle(p1,region):
    b1 = sign(p1,region[0],region[1]) < 0.0
    b2 = sign(p1,region[1],region[2]) < 0.0
    b3 = sign(p1,region[2],region[0]) < 0.0
    return ((b1==b2) and (b2==b3))

def find_index(v,proto_points):
    for i,point in enumerate(proto_points):
        if (np.array(point)==v).all():
            return i
    return -1
# weight function for mls method
def weight_mls(points,v,alpha):
    return np.array([1.0/np.sum(np.abs(v-point)**(2*alpha)) for point in points])

#distance function l2 norm
def distance_l2(x,y):
    return np.sqrt(np.sum((np.array(x)-np.array(y))**2))

#distance function l1 norm
def distance_l1(x,y):
    return np.sum(np.abs(np.array(x)-np.array(y)))

#line distance from a point to a line
def line_distance(p1,p2,p3):
    return norm(np.cross(p2-p1, p1-p3))/norm(p2-p1)


#Generate Affine function
def linear_affine_fun(bias):
    def linear_affine_tmp(x):
        return np.array(x)+np.array(bias)
    return linear_affine_tmp

#Similarity Transformation function
def similarity_fun(src,dst):
    tform = tf.estimate_transform('similarity', src, dst)
    def affine_fun(x):
        return tform(x)[0]
    return affine_fun

#Affine Transformation function
def affine_fun(src,dst):
    tform = tf.estimate_transform('affine', src, dst)
    def affine_fun(x):
        return tform(x)[0]
    return affine_fun

#Weights given the points and the distance function
def distance_fun(region,item):
    if item == 'L2':
        distance_ = distance_l2
    elif item == 'L1':
        distance_ = distance_l1
    def distance(p1):
        result = distance_(p1,np.mean(region,0))
        if len(region)==2:
            a = distance_(p1,region[0])
            b = distance_(p1,region[1])
            #result = min(result,a,b)  
        if result == 0:
            return 0.0000001
        return result
    return distance

#Judge where a poiint is on the parallel line
def is_in_parallel(p1,p2,p3):
    vec1 = p1-p2
    vec2 = p1-p3
    vec_line = p2-p3
    if np.sum(vec1*vec_line) < 0 or np.sum(vec2*vec_line) < 0:
        return False
    return True

#line distance function
def distance_line_fun(line):
    def distance(p1):
        dis1 = distance_l2(p1,line[0])
        dis2 = distance_l2(p1,line[1])
        #line_dis = distance_l2(line[0],line[1])
        if not is_in_parallel(p1,line[0],line[1]):
            return min(dis1,dis2)
        else:
            return line_distance(p1,line[0],line[1])
    return distance

#Weight for LAT
def weight(x,distance_funs,e):

    weights = np.array([1.0/(distance(x)**e) for distance in distance_funs])
    #print(weights)
    return weights/np.sum(weights)

#Get the transformed data
def transform(x,weights,affine_funs):
    return np.sum(np.array([affine_fun(x) for affine_fun in affine_funs])*weights.reshape(weights.shape[0],1),0)

# Produce the line points given the start and end point
def line_points(start,end,n):
    x_bin_width = (end[0] - start[0])/(n+1)
    y_bin_width = (end[1] - start[1])/(n+1)
    points = [start]
    for i in range(1,n+2):
        points.append(np.array([start[0]+x_bin_width*i,start[1]+y_bin_width*i]))
    return points
#Save the keypoints
def save_points(name,new_dict):
    outputfile = open(name,'w')
    for k,v in new_dict.iteritems():
        outputfile.write(k+'='+str(v[1])+','+str(v[0])+'\n')

#Load data from the format
def load_data(path):
    points = {}
    with open(path,'r') as reader:
        for line in reader:
            key,value = line.strip().split('=')

            x,y = value.split(',')
            x,y = int(x),int(y)
            points[key] = np.array([y,x])
    return points

#Plot data on faces
def plot(img,points_dict):
    print(points_dict)
    x = []
    y = []
    plt.figure()
    for k,v in points_dict.iteritems():
        if len(v) >= 2:
            for i in range(len(v)-1):
                for j in range(i+1,len(v)):
                    plt.plot([v[i][1],v[j][1]],[v[i][0],v[j][0]],'k')
                    plt.plot([v[i][1],v[j][1]],[v[i][0],v[j][0]],'ro')
        else:
            x.append(v[0][0])
            y.append(v[0][1])

    plt.imshow(img)
    plt.plot(y,x,'ro')
    plt.axis('off')
    #plt.show()
    #print(plt.gcf())
    return plt.gcf()
#Linear interpolation function
def linear_interpolation(location,img):
    x_size,y_size = img.shape[0:2]
    ori_i,ori_j = location
    new_i, u = int(floor(ori_i)),ori_i - floor(ori_i)
    new_j, v = int(floor(ori_j)),ori_j - floor(ori_j)
    #adjust when it is the border 
    new_ii = new_i+1
    new_jj = new_j+1
    if new_i == x_size-1:
        new_ii = new_i
        u = 0
    if new_j == y_size-1:
        new_jj = new_j
        v = 0
    #interpolation assignment
    return (1-u)*(1-v)*img[new_i][new_j]+(1-u)*v*img[new_i][new_jj]+u*(1-v)*img[new_ii][new_j] +u*v*img[new_ii][new_jj]

def load_region(path):
    lines = []
    with open(path,'r') as reader:
        for line in reader:
            terms = line.strip().split(',')
            lines.append(terms)
    return lines 
#change the dictionary to list
def dictToList(oriDict,protoDict):
    oriPoints = []
    protoPoints = []
    
    for key in protoDict:
        protoPoints.append(protoDict[key])
        oriPoints.append(oriDict[key])
    protoPoints = np.array(protoPoints)
    oriPoints = np.array(oriPoints)

    return oriPoints,protoPoints  


```

## Source file: code/morphing.py

{
  "path": "code/morphing.py",
  "blob_sha": "f188d556adc80a10558d74897df4b6411e9573f1",
  "sha256": "7307727283bd07a2dbb1e489273af908884f5261bd930d7946ec42ee76688c80",
  "lines": 217,
  "bytes": 7425,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 1,
    "mouth": 1,
    "deform": 1,
    "rig": 2,
    "face": 4,
    "lip": 1
  },
  "url": "https://github.com/miqianmimi/Deformation-On-Human-Face/blob/a811a84ff3dcb360cefd15097de8536352e45fc0/code/morphing.py",
  "function_signature_candidates": [
    {
      "line": 6,
      "text": "def readPoints(path) :"
    },
    {
      "line": 17,
      "text": "def load_data(path):"
    },
    {
      "line": 28,
      "text": "def affineTransform(src, srcTri, dstTri, size) :"
    },
    {
      "line": 38,
      "text": "def delaunaryTriangles(rect,points):"
    },
    {
      "line": 62,
      "text": "def morphing(oriImg,protoImg,oriPoints,protoPoints,alpha):"
    },
    {
      "line": 87,
      "text": "def addPoints(points,img):"
    },
    {
      "line": 102,
      "text": "def morphingTriangle(imgs, ts, alpha):"
    },
    {
      "line": 133,
      "text": "def detect(path):"
    },
    {
      "line": 151,
      "text": "def morphingAction(ori_path,proto_path,proto_point_path,alpha):"
    }
  ]
}

```text

import numpy as np
import cv2
from PIL import Image
import sys
import matplotlib.pyplot as plt
def readPoints(path) :
    # Create an array of points.
    points = [];
    # Read points
    with open(path) as file :
        for line in file :
            x, y = line.split()
            points.append((int(x), int(y)))

    return points
#load the points  from the format we define in readme
def load_data(path):
	points = {}
	with open(path,'r') as reader:
		for line in reader:
			key,value = line.strip().split('=')
			x,y = value.split(',')
			x,y = int(x),int(y)
			points[key] = np.array([x,y])
	return points

#This python file is for morphing 
def affineTransform(src, srcTri, dstTri, size) :
    
    # Given a pair of triangles, find the affine transform.
    warpMat = cv2.getAffineTransform( np.float32(srcTri), np.float32(dstTri) )
    
    # Apply the Affine Transform just found to the src image
    dst = cv2.warpAffine( src, warpMat, (size[0], size[1]), None, flags=cv2.INTER_LINEAR, borderMode=cv2.BORDER_REFLECT_101 )

    return dst
# Generate the triangles by using the opencv subdiv from the control points
def delaunaryTriangles(rect,points):
    #print(points,rect)
    subdiv = cv2.Subdiv2D(rect)
    pointsDict = {}
    #Get the triangle list 
    for i,p in enumerate(points) :
    	
        subdiv.insert((int(p[0]),int(p[1])))
        pointsDict[(int(p[0]),int(p[1]))] = i 
    triangleList = subdiv.getTriangleList();
    
    triangleIndList = []
    #Get the index of the  triangle points
    for t in triangleList :

        pt1 = (int(t[0]), int(t[1]))
        pt2 = (int(t[2]), int(t[3]))
        pt3 = (int(t[4]), int(t[5]))
        
        if pt1 in pointsDict and pt2 in pointsDict and pt3 in pointsDict:
        	triangleIndList.append([pointsDict[pt1],pointsDict[pt2],pointsDict[pt3]])
    
    return triangleIndList
#Do deformation on oriImg
def morphing(oriImg,protoImg,oriPoints,protoPoints,alpha):
    oriPoints = addPoints(oriPoints,oriImg)
    protoPoints = addPoints(protoPoints,protoImg)
    newShape = protoImg.shape
    #Get the morph points on M image or newImg
    morphPoints = (1-alpha)*oriPoints + alpha*protoPoints

    triangleIndList = delaunaryTriangles((0,0,newShape[1],newShape[0]),morphPoints)
    newImg = np.zeros(newShape,dtype=protoImg.dtype)
    for row in newImg  :
        for term in row:
            term[3]=255
   
    imgs = [oriImg,protoImg,newImg]
    #For each triangles do the affine transformation
    for tInd in triangleIndList:
    	ts = [[],[],[]]
    	for i in range(3):
        	ts[0].append(np.array(oriPoints[tInd[i]]))
        	ts[1].append(np.array(protoPoints[tInd[i]]))
        	ts[2].append(np.array(morphPoints[tInd[i]]))
        morphingTriangle(imgs,ts,alpha)
    return imgs[2]

#Add points on the borders
def addPoints(points,img):
    points_ = []
    for point in points:
        points_.append((int(point[0]),int(point[1])))
    y_size,x_size = img.shape[0]-1,img.shape[1]-1
    points_.append((0,int(y_size)))
    points_.append((int(x_size),int(y_size)))
    points_.append((int(x_size),0))
    points_.append((0,0))
    points_.append((0,int(y_size/2)))
    points_.append((int(x_size/2),0))
    points_.append((int(x_size/2),int(y_size)))
    points_.append((int(x_size),int(y_size/2)))
    return np.array(points_)
#Morphing on the triangles 
def morphingTriangle(imgs, ts, alpha):

    #Finding the corresponding rectangles and the images
    recs = []
    recImgs = []
    tRects = [[],[],[]]
    for i,t in enumerate(ts):
        #compute the rectangle boundary of each img
        recs.append(cv2.boundingRect(np.float32([t])))
        # get the corresponding rectangles on the imgs
        recImgs.append(imgs[i][recs[i][1]:(recs[i][1]+recs[i][3]),recs[i][0]:(recs[i][0]+recs[i][2])])
        # get the offsets corrdinates on the recImages
        for j in range(3):
            tRects[i].append(((ts[i][j][0]-recs[i][0]),(ts[i][j][1]-recs[i][1])))

    
    size = (recs[2][2],recs[2][3])
    #Apply the affineTransform function
    affineRec1 = affineTransform(recImgs[0],tRects[0],tRects[2],size)
    affineRec2 = affineTransform(recImgs[1],tRects[1],tRects[2],size)

    #Create the mask
    mask = np.zeros((recs[2][3], recs[2][2], imgs[2].shape[2]), dtype = np.float32)
    cv2.fillConvexPoly(mask,np.int32(tRects[2]),(1.0,1.0,1.0),16,0)

    #compute the morphing value
    morphRec = (1-alpha)*affineRec1 + alpha*affineRec2

    imgs[2][recs[2][1]:recs[2][1]+recs[2][3],recs[2][0]:recs[2][0]+recs[2][2]] = recImgs[2]*(1-mask)+mask*morphRec

#Invoke face++ api to detect the facial keypoints
def detect(path):
    API_KEY = "3o6_lMDRxcpYalXhuXq9cymJeeN7cHCS"
    API_SECRET = "6776wZFWYVfYjwDgS8G_0rmWhtXVyUcW"
    from facepp import API, File
    api = API(API_KEY, API_SECRET)
    result = api.detect(image_file=File(path),return_landmark=1)
    landmarks = result['faces'][0]['landmark']
    #print(landmarks)
    keys = [ 'left_eye_right_corner','left_eye_left_corner','right_eye_right_corner','right_eye_left_corner','left_eyebrow_right_corner','right_eyebrow_left_corner',
              'mouth_right_corner','mouth_left_corner','mouth_lower_lip_left_contour2','mouth_lower_lip_left_contour3','mouth_lower_lip_bottom','mouth_lower_lip_right_contour2','mouth_lower_lip_right_contour3','nose_left','nose_right']
    new_dict = {}
    for k,v in landmarks.iteritems():
        
        new_dict[k] = np.array([v['x'],v['y']])

    return new_dict

#The function invoked by the GUI
def morphingAction(ori_path,proto_path,proto_point_path,alpha):
    oriImg = np.array(Image.open(ori_path))
    protoImg = np.array(Image.open(proto_path))
    #Load the protot points data as the dictinary format
    protoDict = load_data(proto_point_path)
    #Get the Facial Keypoints by using face++
    oriDict = detect(ori_path)
    delList = []
    #Delete the key which proto points do not contain
    for key in oriDict:
        if key not in protoDict:
            delList.append(key)
    for key in delList:
        del oriDict[key]
    #change the dictionary to List
    oriPoints = []
    protoPoints = []
    
    for key in protoDict:
        protoPoints.append(protoDict[key])
        oriPoints.append(oriDict[key])
    protoPoints = np.array(protoPoints)
    oriPoints = np.array(oriPoints)

    newImg = morphing(oriImg,protoImg,oriPoints,protoPoints,alpha)
    
    return newImg

#A test example
if __name__ == '__main__':
    ori_path = 'data/zxh_new.png'
    proto_path = 'data/ape.png'
    oriImg = np.array(Image.open(ori_path))
    protoImg = np.array(Image.open(proto_path))
    
    
    protoDict = load_data('data/biaodian.txt')
   
    oriDict = detect(ori_path)
    delList = []
    for key in oriDict:
        if key not in protoDict:
            delList.append(key)
    for key in delList:
        del oriDict[key]

    oriPoints = []
    protoPoints = []
    
    for key in protoDict:
        protoPoints.append(protoDict[key])
        oriPoints.append(oriDict[key])
    protoPoints = np.array(protoPoints)
    oriPoints = np.array(oriPoints)
	
    
    alphas = [0.5,0.6,0.7,0.8,0.9,1]
    #alphas = [0,0.1,0.2,0.3,0.4]
    for i,alpha in enumerate(alphas):    
        newImg = morphing(oriImg,protoImg,oriPoints,protoPoints,alpha)
        
        plt.subplot(1,len(alphas),i+1)
        plt.imshow(np.uint8(newImg),interpolation=None)
        plt.axis('off')
        
    plt.show()
    

```
