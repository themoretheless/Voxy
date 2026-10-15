# Facial rendering source scan: RtjShreyD/Playing-with-Makehuman-scripts

Repository: https://github.com/RtjShreyD/Playing-with-Makehuman-scripts

Pinned source revision: bd2bf3d8c4db0f719962b467712d2fe84bfbee74

Discovery topic: MakeHuman

Description: In this repository I will be adding Python scripts with Makehuman to edit the modifiers of a humanoid object using the plugins

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: fns.py

{
  "path": "fns.py",
  "blob_sha": "e44a8e1ff112121279c1956cc5fbe6a7b210d71c",
  "sha256": "aef4aa39f81f44f621adb45d365ede3b20b9786fde633a408612be4c776dde09",
  "lines": 50,
  "bytes": 2930,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 1
  },
  "url": "https://github.com/RtjShreyD/Playing-with-Makehuman-scripts/blob/bd2bf3d8c4db0f719962b467712d2fe84bfbee74/fns.py",
  "function_signature_candidates": [
    {
      "line": 5,
      "text": "def measures():"
    },
    {
      "line": 38,
      "text": "def change(old_m): #This function compares the new and old array after every small change in any parameter and returns a comparative array"
    }
  ]
}

```text

from core import G
import humanmodifier


def measures():
	
	Measurerer = (__import__("0_modeling_a_measurement"))  #Class form the 0_modelling_measurement file, without this we cant get metric measurements from Makehuman
	human = G.app.mhapi.modifiers.human  #Selected current humanoid object
	ruler = Measurerer.Ruler() 
	H = ruler.getMeasure    #ruler is an object to calculate metric measures
	#print 'waist [cm]: ',ruler.getMeasure(human,'measure/measure-waist-circ-decr|incr','metric')
	#data = ruler.getMeasure(human,'measure/measure-waist-circ-decr|incr','metric')
	height = human.getHeightCm()	
	neck_cir = H(human,'measure/measure-neck-circ-decr|incr','metric') #This is how measurement of a particular modifier(parameter) is fetched, here all of them are defined
	neck_ht = H(human,'measure/measure-neck-height-decr|incr','metric')
	ua_cir = H(human,'measure/measure-upperarm-circ-decr|incr','metric')
	ua_len = H(human,'measure/measure-upperarm-length-decr|incr','metric')
	la_len = H(human,'measure/measure-lowerarm-length-decr|incr','metric')
	wrist = H(human,'measure/measure-wrist-circ-decr|incr','metric')
	fc_dist = H(human,'measure/measure-frontchest-dist-decr|incr','metric')
	bust = H(human,'measure/measure-bust-circ-decr|incr','metric')
	ubust = H(human,'measure/measure-underbust-circ-decr|incr','metric')
	waist = H(human,'measure/measure-waist-circ-decr|incr','metric')
	np2waist = H(human,'measure/measure-napetowaist-dist-decr|incr','metric')
	w2hip = H(human,'measure/measure-waisttohip-dist-decr|incr','metric')
	sh_dist = H(human,'measure/measure-shoulder-dist-decr|incr','metric')
	hip_cir = H(human,'measure/measure-hips-circ-decr|incr','metric')
	ul_ht = H(human,'measure/measure-upperleg-height-decr|incr','metric')
	th_cir = H(human,'measure/measure-thigh-circ-decr|incr','metric')
	ll_ht = H(human,'measure/measure-lowerleg-height-decr|incr','metric')
	calf_cir = H(human,'measure/measure-calf-circ-decr|incr','metric')
	knee = H(human,'measure/measure-knee-circ-decr|incr','metric')
	ankle = H(human,'measure/measure-ankle-circ-decr|incr','metric')
	#storing the above values in a list below and return the list	
	data = [height, neck_cir, neck_ht, ua_cir, ua_len, la_len, wrist, fc_dist, bust, ubust, waist, np2waist, w2hip, sh_dist, hip_cir, ul_ht, th_cir, ll_ht, calf_cir, knee, ankle]
	return data
		
def change(old_m): #This function compares the new and old array after every small change in any parameter and returns a comparative array
	old = old_m
	new = measures()
	mat = []  #Empty list or array
	
	for  i in range(21): #range is 21 because in all there are 21 metric measurements that are being returned by the functions measures, change
		delta = new[i] - old[i]
		if  delta == 0:
			mat.append(old[i]) #adding element to the array from left to right(0th location to nth location)
		else: 
			mat.append(new[i])
			
	return 	mat


```

## Source file: works.py

{
  "path": "works.py",
  "blob_sha": "59028804b3def54e45fc2474853e5fe6a8b53843",
  "sha256": "5a6abd7be4ce77c1d9a1746f816f98d2613309b0c7fc0d0f1151307f9fc0fdc3",
  "lines": 36,
  "bytes": 817,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/RtjShreyD/Playing-with-Makehuman-scripts/blob/bd2bf3d8c4db0f719962b467712d2fe84bfbee74/works.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def metric_write(mat,r,c,sheet,wb):"
    }
  ]
}

```text

import xlwt
from xlwt import Workbook
import fns
from fns import measures, change
from core import G
import humanmodifier

wb  = Workbook()
sheet1 = wb.add_sheet('Sheet 1', cell_overwrite_ok=True)

def metric_write(mat,r,c,sheet,wb):
	for row, value in enumerate(mat, start=r):
		sheet.write(row, c, value) 	
	
m1 = measures() 	
human = G.app.mhapi.modifiers.human
for  i in range(11):
	a = 0.1*(i)
	human.setMuscle(a)
	n1 = change(m1)
	chf	= str(a)
	label = 'muscle-'+chf
	sheet1.write(0,(3+i),label)
	metric_write(n1,1,(3+i),sheet1,wb)
	
	for i2 in range (11):
		a2 = 0.1*(i2)
		human.setWeight(a2)
		n2 = change(n1)
		chf2 = str(a2)
		label2 = 'weight-'+chf2
		sheet1.write((22+(22*i2)),(3+i),label2)
		metric_write(n2,(23+(22*i2)),(3+i),sheet1,wb)

wb.save('xlwt lookup.xls')



```

## Source file: trial1.py

{
  "path": "trial1.py",
  "blob_sha": "1332f07dd7ea3e9351ee4f3e765fdc94f11b2f6d",
  "sha256": "e928cffcc3d45dfba1c9f9eb1ec81b00c3074f54e4a15b4a5e847b26b16555c9",
  "lines": 15,
  "bytes": 482,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/RtjShreyD/Playing-with-Makehuman-scripts/blob/bd2bf3d8c4db0f719962b467712d2fe84bfbee74/trial1.py",
  "function_signature_candidates": []
}

```text

from core import G
import getpath
import humanmodifier

Measurerer = (__import__("0_modeling_a_measurement"))

human = G.app.mhapi.modifiers.human

ruler = Measurerer.Ruler()

#print 'waist [cm]: ',ruler.getMeasure(human,'measure/measure-waist-circ-decr|incr','metric')

human.getModifier('measure/measure-waist-circ-decr|incr').setValue(1.0)
human.applyAllTargets()   
#print 'waist [cm], 1.0: ',ruler.getMeasure(human,'measure/measure-waist-circ-decr|incr','metric')

```

## Source file: works2.py

{
  "path": "works2.py",
  "blob_sha": "c574becdf1c4e64b650d0276dfa4968f3bd6f0ab",
  "sha256": "8089d19ef2eeada934510c070a34e43ca5831ad5351e230144a5720e18de2c4b",
  "lines": 56,
  "bytes": 897,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/RtjShreyD/Playing-with-Makehuman-scripts/blob/bd2bf3d8c4db0f719962b467712d2fe84bfbee74/works2.py",
  "function_signature_candidates": []
}

```text

import sqlite3
import fns
from fns import measures, change
from core import G
import humanmodifier

conn = sqlite3.connect("lookup.db")
print("Created/Connected \n")
c = conn.cursor()
m1 = measures() 
##create table for change in muscles only
c.execute('''CREATE TABLE `muscle` (`delta_fc`	INTEGER,`height`	REAL,`neck_cir`	REAL,`neck_ht`	REAL,`ua_cir`	REAL,`ua_len`	REAL,`la_len`	REAL,`wrist`	REAL,`fc_dist`	REAL,`bust`	REAL,`ubust`	REAL,`waist`	REAL,`np2waist`	REAL,`w2hip`	REAL,`sh_dist`	REAL,`hip_cir`	REAL,`ul_ht`	REAL,`th_cir`	REAL,`ll_ht`	REAL,`calf_cir`	REAL,`knee`	REAL,`ankle`	REAL)''')

human = G.app.mhapi.modifiers.human
for  i in range(11):
	a = 0.1*(i)
	human.setMuscle(a)
	n1 = change(m1)
	n1.insert(0,a)
	c.execute('''INSERT INTO `muscle` VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)''',n1)
	
conn.commit()
conn.close()



































```
