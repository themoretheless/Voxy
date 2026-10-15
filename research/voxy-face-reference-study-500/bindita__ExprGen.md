# Facial rendering source scan: bindita/ExprGen

Repository: https://github.com/bindita/ExprGen

Pinned source revision: e18fcb46434a02e8f3102405e81f5cffbd7cff29

Discovery topic: facial rig

Description: Code to generate facial expressions on 3D MAYA rigs

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: savetomaya.py

{
  "path": "savetomaya.py",
  "blob_sha": "45cf4928b91bafdf6a93b2e448f8ed49b01470ff",
  "sha256": "a1f52dfce68670c85c877bdcb1f0e4f90e0ecb5e97ce176bc4f4150357829aaa",
  "lines": 81,
  "bytes": 2698,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 6,
    "rig": 11
  },
  "url": "https://github.com/bindita/ExprGen/blob/e18fcb46434a02e8f3102405e81f5cffbd7cff29/savetomaya.py",
  "function_signature_candidates": []
}

```text

'''
-------------------------
Code to read rig parameters from CSV file and set the parameters in 3D MAYA rig
Run with default inputs:
mayapy savetomaya.py

If you want to rename your input file, say maya.mb to maya_new.mb, run:
mayapy savetomaya.py -rename

To set input csv file name and input rig file name, run:
mayapy savetomaya.py -icsv /path/to/input_csv_file -irig /path/to/input_rig_file
-------------------------
'''

### import functions to load and initialize mayapy
import maya.standalone
maya.standalone.initialize()
import maya.cmds as cmds

### import other functions
import csv, argparse, os

### Add input arguments
parser = argparse.ArgumentParser()
parser.add_argument('-icsv', '--inputcsv', default='input.csv',
                    help='file name of input csv')
parser.add_argument('-irig', '--inputrig', default='Mery.mb',
                    help='file name of input MAYA rig')
parser.add_argument('-rename', action='store_true',
                    help='renames the modified file')

### Parse input arguments
args = parser.parse_args()

### Set the variables to appropriate values
MayaAttributesFilename = args.inputcsv

MayaRigFilename = args.inputrig

MayaRigFilename_new = MayaRigFilename.split('/')[-1]

if args.rename:
	MayaRigFilename_new = MayaRigFilename_new.replace('.m','_new.m')

### Read all parameters from CSV file
with open(MayaAttributesFilename, 'r') as csvfile:
	parameters = list(csv.reader(csvfile))

### Get number of parameters and number of examples
Nexamples, Nparameters = len(parameters)-1, len(parameters[0])

### Open the MAYA rig
cmds.file(MayaRigFilename, o=True, force=True)

### For each example
for e in range(Nexamples):
	### Update the current time frame to be edited
	cmds.currentTime(e+1, edit=True)
	### For each parameter (ignore 1st column containing name)
	for i in range(1,Nparameters):

		### Get the object name and attribute name of the parameter
		### For example, if parameter name is browLeft.translateX,
		### objname = browLeft, attrname = translateX
		objname, attrname = parameters[0][i].split('.')

		### If object name contains reference, get two separate variables
		if len(objname.split(':')) > 1:
			objname = objname.split(':')[-1]
			objnameNOreference = parameters[0][i].split(':')[-1]
		else:
			objnameNOreference = parameters[0][i]	

		### Update the parameter value
		cmds.setAttr(objnameNOreference, float(parameters[e+1][i]), keyable=True)
		### Key frame the updated parameter 
		cmds.setKeyframe(objname, at=attrname)

### Set the new file name and save the MAYA file in the same directory as the python file (current directory)
cmds.file(rename=os.getcwd()+'/'+MayaRigFilename_new)
cmds.file(save=True)


```

## Source file: renderfrommaya.py

{
  "path": "renderfrommaya.py",
  "blob_sha": "de55ff731168082cc82d109336a5be86fd5bdef9",
  "sha256": "1ac7bac8c3e0be975182f113b2d897d58a1a7befda998a50788206cc0382266c",
  "lines": 35,
  "bytes": 1247,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 5,
    "rig": 3
  },
  "url": "https://github.com/bindita/ExprGen/blob/e18fcb46434a02e8f3102405e81f5cffbd7cff29/renderfrommaya.py",
  "function_signature_candidates": []
}

```text

### import functions to load and initialize mayapy
import maya.standalone
maya.standalone.initialize()
import maya.cmds as cmds

### Add input arguments
import argparse
parser = argparse.ArgumentParser()
parser.add_argument('-irig', '--inputrig', default='Mery.mb',
                    help='file name of input MAYA rig')
parser.add_argument('-dirname', '--directoryname', default = '',
					help='renames the modified file')

### Parse input arguments
args = parser.parse_args()

### open the maya file to render
cmds.file(args.inputrig, o=True, iv=True, force=True)

### set up directory to store the rendered image
cmds.workspace( directory = args.directoryname )
cmds.workspace( fileRule = ['images', args.directoryname] )

### Set the time frame number you want to render
### Here we render the 10th frame, so start and end numbers are same
cmds.setAttr('defaultRenderGlobals.startFrame',10)
cmds.setAttr('defaultRenderGlobals.endFrame',10)

### render using the built-in camera 'fergcam_mery'
### saved image will have 256 x 256 dimensions
name = cmds.render('fergcam_mery',x=256,y=256)

### playblast option
#cmds.renderSettings(cam='fergcam_mery')
#cmds.playblast(f='mymovie',format='image',st=1,et=30,wh=[256,256],v=False,fo=True,os=True)

```

## Source file: extractfrommaya.py

{
  "path": "extractfrommaya.py",
  "blob_sha": "e287f552c5e778dbe4495709b8e2e3a5c86a19f3",
  "sha256": "1e046e619374a196b55f0a8e5ebc47a51493b287f30b1beaa92bdbc768b293e1",
  "lines": 130,
  "bytes": 4574,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 5,
    "iris": 2,
    "rig": 14,
    "face": 2
  },
  "url": "https://github.com/bindita/ExprGen/blob/e18fcb46434a02e8f3102405e81f5cffbd7cff29/extractfrommaya.py",
  "function_signature_candidates": [
    {
      "line": 53,
      "text": "def extractAttributes():"
    }
  ]
}

```text

'''
-------------------------
Code to extract rig parameters values from a MAYA rig and save them in a CSV file

Run with default inputs:
mayapy extractfrommaya.py

If you want to specify the character (one of Mery, Bonnie, Ray and Malcolm), say Bonnie, run:
mayapy extractfrommaya.py -char Bonnie

To set input rig file name, run:
mayapy extractfrommaya.py -irig /path/to/input/rig/file
-------------------------
'''

### import functions to load and initialize mayapy
import maya.standalone
maya.standalone.initialize()
import maya.cmds as cmds

### import other functions
import csv, argparse, os

### Add input arguments
parser = argparse.ArgumentParser()
parser.add_argument('-char', '--character', default='Mery',
                    help='name of input character: Mery, Bonnie, Ray or Malcolm')
parser.add_argument('-irig', '--inputrig', default='Mery.mb',
                    help='file name of input MAYA rig')

### Parse input arguments
args = parser.parse_args()

### Set the variables to appropriate values
MayaRigFilename = args.inputrig
CSVfilename = MayaRigFilename.split('/')[-1].split('.')[0] + '.csv'

if args.character is 'Mery':
	Facial_Group_Name, Group_Name_Start = 'Mery_grp_cn_facials_controls', 'Mery_ac_'
elif args.character is 'Bonnie':
	Facial_Group_Name, Group_Name_Start = 'BONNIE:BONNIE_ANIM_FACE', 'BONNIE:BONNIE_ANIM_FACE_'
elif args.character is 'Ray':
	Facial_Group_Name, Group_Name_Start = 'CGTarian_Ray_2015_09_08:ray_face_controls', 'CGTarian_Ray_2015_09_08:ray_'
	# also use the Facial_Group_Name = 'CGTarian_Ray_2015_09_08:ray_sk_cn_head_offset'
elif args.character is 'Malcolm':
	Facial_Group_Name, Group_Name_Start = 'malcolm_v200:headControls', 'malcolm_v200:ctl'
else:
	print('Character name must be one of Mery, Bonnie, Ray and Malcolm')
	quit()


### Helper function to extract attributes from the opened rig
def extractAttributes():
	attrAndValues = {}	

	### list all descendent nodes of type transform
	allNodes = cmds.listRelatives(Facial_Group_Name, allDescendents=True, type='transform')
	
	for node in allNodes: 
		attrList = None
		if '_parentConstraint' in node:
			continue 

		### UNCOMMENT for BONNIE and RAY
		# all relevant transform nodes have associated shapes; if not, they are group names
		#if not cmds.listRelatives(node,shapes=True):
		#	continue

		### for each valid node, extract the names of the keyable attributes
		if str(node).startswith(Group_Name_Start):
			attrList = cmds.listAttr(node, keyable=True)		
		
		### get the full attribute name and its corresponding value for each attribute
		if attrList:
			for attr in attrList:				
				name = node + "." + attr # attribute name in the form "NODE_NAME.ATTR_NAME"
				value = cmds.getAttr(name) # attribute value
				attrAndValues[name] = value
	
	return attrAndValues

### Main function
if __name__ == '__main__':	
	orderedAttr = None
	with open(CSVfilename, 'w') as output:
		writer = csv.writer(output)

		### uncomment if you want to read multiple files in folders and subfolders
		#for path, subdirs, rigFiles in os.walk(folder):
		#	for rig in rigFiles:
		
		### uncomment if your folder has some non-maya files
		#base, ext = os.path.splitext(MayaRigFilename)
		#if ext != '.mb' and ext != '.ma': # skip if file isn't a rig (there could be some hidden files)
		#	continue

		### open the maya file
		print 'Opening file: ', MayaRigFilename, '...'
		fullpath = MayaRigFilename #os.path.join(path, rig)
		cmds.file(fullpath, o=True, iv=True, force=True)		
		
		### set the time frames to be extracted; default: all
		startFrame = int(cmds.getAttr('defaultRenderGlobals.startFrame'))
		endFrame = int(cmds.getAttr('defaultRenderGlobals.endFrame'))

		### loop over each time frame
		for i in range(startFrame, endFrame + 1): 
			cmds.currentTime(i)
			print 'Extracting for frame', i

			### call helper function for extracting attributes
			attrAndValues = extractAttributes()

			### write the attribute names in the first row of the csv file 
			if not orderedAttr:
				orderedAttr = list(attrAndValues.keys())
				orderedAttr.sort()
				orderedAttr.insert(0, 'file_name')
				# some file has an extra attribute: "Mery_ac_rg_iris1.translateY"; ignore this attribute
				ignore = 'Mery_ac_rg_iris1.translateY'
				if ignore in orderedAttr:
					orderedAttr.remove(ignore)
				writer.writerow(orderedAttr)

			### write the attribute values in the remaining rows of the csv file 
			attrAndValues['file_name'] = fullpath + "_" + str(i).zfill(4)
			row = []
			for attr in orderedAttr:
				row.append(attrAndValues[attr])
			writer.writerow(row)

```
