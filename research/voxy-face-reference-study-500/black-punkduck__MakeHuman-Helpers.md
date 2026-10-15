# Facial rendering source scan: black-punkduck/MakeHuman-Helpers

Repository: https://github.com/black-punkduck/MakeHuman-Helpers

Pinned source revision: 6b645e0e221079c1cf55cf33a83ca2ecc78e592c

Discovery topic: MakeHuman

Description: Helpers used for manipulating objects in MakeHuman Environment

Reported license: AGPL-3.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["license.txt"]

Topic paths: {}

## Source file: wprint.py

{
  "path": "wprint.py",
  "blob_sha": "6d4a159a53289d83fb43dd52307a624375fb4e11",
  "sha256": "ea86da9bdfe9ea8e30f5d39baa9ee3eb6cde50682b788b9c1bafeab9c938c6d0",
  "lines": 82,
  "bytes": 2191,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 1
  },
  "url": "https://github.com/black-punkduck/MakeHuman-Helpers/blob/6b645e0e221079c1cf55cf33a83ca2ecc78e592c/wprint.py",
  "function_signature_candidates": [
    {
      "line": 37,
      "text": "def __init__(self, prec=4, mcol=4):"
    },
    {
      "line": 42,
      "text": "def new(self):"
    },
    {
      "line": 45,
      "text": "def appval(self, num, val):"
    },
    {
      "line": 64,
      "text": "def appgroup (self, verts, key):"
    },
    {
      "line": 72,
      "text": "def appweights (self, verts):"
    }
  ]
}

```text

#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""

**Project Name:**      MakeHuman-Helpers

**Authors:**           Black Punkduck

**Code Home Page:**    https://github.com/black-punkduck/MakeHuman-Helpers

**Copyright(c):**      Black Punkduck

**Licensing:**         AGPL3

    This file is part of the MakeHuman Support Tools

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as
    published by the Free Software Foundation, either version 3 of the
    License, or (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.

Abstract
--------
Output class for support tools
"""

class v_array:
    def __init__(self, prec=4, mcol=4):
        self.prec = prec
        self.mcol = mcol
        self.col = -1

    def new(self):
        self.col = -1
    
    def appval(self, num, val):
        x = round(val, self.prec)
        if x ==  0.0:
            return ("")
        t = ""
        if self.col == 0:
            t += ",\n\t\t"
        elif self.col < 0:
            self.col = 0
            t += "\t\t"
        else:
            t += ", "

        t += "[" + str(num) + ", " + str(x) + "]"
        self.col +=1
        if self.col == self.mcol:
            self.col = 0
        return t

    def appgroup (self, verts, key):
        self.new()
        t = "\t\"" + key + "\": [\n"
        for vnum in sorted(verts[key]):
            t += self.appval (vnum, verts[key][vnum])
        t += "\n\t]"
        return t

    def appweights (self, verts):
        t = ""
        for key in sorted(verts):
            elems = len (verts[key])
            if elems > 0:
                if t != "":
                    t += ",\n"
                t += self.appgroup (verts, key)
        t += "\n"
        return t



```

## Source file: normalize.py

{
  "path": "normalize.py",
  "blob_sha": "2cd83919b30420ae6882a589a4a5cc8a951205db",
  "sha256": "86f40fad18cb011db08b297d5569a172da60df051f0e93029e19236ed37e32df",
  "lines": 114,
  "bytes": 3488,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 7,
    "rig": 1
  },
  "url": "https://github.com/black-punkduck/MakeHuman-Helpers/blob/6b645e0e221079c1cf55cf33a83ca2ecc78e592c/normalize.py",
  "function_signature_candidates": []
}

```text

#!/usr/bin/python3
# -*- coding: utf-8 -*-

"""

**Project Name:**      MakeHuman-Helpers

**Authors:**           Black Punkduck

**Code Home Page:**    https://github.com/black-punkduck/MakeHuman-Helpers

**Copyright(c):**      Black Punkduck

**Licensing:**         AGPL3

    This file is part of the MakeHuman Support Tools

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as
    published by the Free Software Foundation, either version 3 of the
    License, or (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.

Abstract
--------
Skript to normalize weights of a weights file so sum up to 1
"""

import re
import sys
import json
import argparse
import wprint as wp

configfilename = "data/mh_helper.json"

parser = argparse.ArgumentParser(description='Normalize weights of a weights file so they do sum up to 1.')
parser.add_argument('-f', default="", metavar='default_weights', help='use different default weight file (default: see configuration)')

parser.add_argument('-s', default=0, metavar='MINWEIGHTNUM', type=int,  help='lowest vertex number to check (default: 0)')
parser.add_argument('-m', default=0, metavar='MAXWEIGHTNUM', type=int,  help='highest vertex to check (default: see configuration ranges/_end_)')
parser.add_argument('-p', default=4, metavar='PRECISION', type=int,  help='precision of weights (default: 4)')
parser.add_argument('-c', default=4, metavar='COLUMNS', type=int,  help='number of output columns for weights (default: 4)')
parser.add_argument('-D', default=0.002, metavar='DIFFERENCE', type=float,  help='smallest discrepancy to recalculate default: 0.002)')
args = parser.parse_args()

#
# read configuration
#
cfile = open (configfilename, "r")
config = json.load (cfile)
cfile.close()

minweights = args.s
if args.m == 0:
    maxweights = config['ranges']['_end_']
else:
    maxweights = args.m

#
# read weight-file
#
if len(args.f):
    weightfilename = args.f
else:
    weightfilename = config["default_weights"]

cfile = open (weightfilename, "r")
weights = json.load (cfile)
cfile.close()

verts = {}
for key in weights["weights"].keys():
    group = weights["weights"][key]

    for comb in range (0, len(group)):
        vnum = int(group[comb][0])
        val  = group[comb][1]
        if vnum not in verts:
            verts[vnum] = { '_sum' : val, key: val}
        else:
            verts[vnum][key] = val
            verts[vnum]['_sum'] += val


#
# create weights to merge (replace mode)
#
new_g = {}

for vnum in range (minweights, maxweights):
    if vnum not in verts:
        continue
    else:
        r = 1.0 - verts[vnum]['_sum']
        if (r > args.D or r < -args.D):
            f = 1.0 / (1.0 - r)
            for group in verts[vnum].keys():
                if group != '_sum':
                    if group not in new_g:
                        new_g[group] = {}
                    new_g[group][vnum] = round(verts[vnum][group] * f,args.p)

va = wp.v_array(prec=args.p, mcol=args.c)
dtext = va.appweights (new_g)
print (dtext, end="")



```

## Source file: shorten_bvh.py

{
  "path": "shorten_bvh.py",
  "blob_sha": "7a076ed257dcf191c20330db20518854d1d66fb8",
  "sha256": "1fcb2c8d9c8c261307b2de257810c46b59e2c0749ff488f93b48342a0fd38fe2",
  "lines": 50,
  "bytes": 1234,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 4
  },
  "url": "https://github.com/black-punkduck/MakeHuman-Helpers/blob/6b645e0e221079c1cf55cf33a83ca2ecc78e592c/shorten_bvh.py",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "def shortestString(x):"
    }
  ]
}

```text

#!/usr/bin/python3

import argparse
import re


def shortestString(x):
    f = float(x)
    if abs(f) <  0.00001:
        return("0")
    else:
        return(str(f))

parser = argparse.ArgumentParser(
        description='shorten bvh file and change z coordinate of root')
parser.add_argument('-z', default=0.0, type=float, help='change of z coordinate (negative => down)')
parser.add_argument('inputfile', type=str, help='unchanged bvh-file')
parser.add_argument('outputfile', type=str, help='changed bvh-file')

args = parser.parse_args()

with open(args.inputfile) as f:
    try:
        out= open(args.outputfile, "w")
    except:
        print ("Cannot write:" + args.outputfile)
        exit (20)
    #
    # handle HIERARCHY
    #
    for line in f:
        out.write(line)
        m=re.search("^\s*MOTION", line)
        if m:
            break

    for line in f:
        m=re.search("^\s*Frame", line)
        if m:
            out.write(line)
            continue

        columns = [shortestString(x) for x in line.split()]
        if args.z != 0.0:
            columns[2] = "{z:.5f}".format(z=float(columns[2]) + args.z)
        newline = ' '.join(columns) + "\n"
        out.write(newline)

    out.close()
    exit(0)


```

## Source file: weights_sum.py

{
  "path": "weights_sum.py",
  "blob_sha": "4065aa87b0a56d58e983b57490bef4470cd2d96e",
  "sha256": "d90a4a83650d2457cc9338ac457fb1ea0d29d0c18bcad8e27030bee0f54e3d8b",
  "lines": 100,
  "bytes": 3161,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 6,
    "rig": 1
  },
  "url": "https://github.com/black-punkduck/MakeHuman-Helpers/blob/6b645e0e221079c1cf55cf33a83ca2ecc78e592c/weights_sum.py",
  "function_signature_candidates": []
}

```text

#!/usr/bin/python3
# -*- coding: utf-8 -*-

"""

**Project Name:**      MakeHuman-Helpers

**Authors:**           Black Punkduck

**Code Home Page:**    https://github.com/black-punkduck/MakeHuman-Helpers

**Copyright(c):**      Black Punkduck

**Licensing:**         AGPL3

    This file is part of the MakeHuman Support Tools

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as
    published by the Free Software Foundation, either version 3 of the
    License, or (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.

Abstract
--------
Skript to check if weights of a weight-file will sum up to 1.
"""

import re
import sys
import json
import argparse

configfilename = "data/mh_helper.json"

parser = argparse.ArgumentParser(description='Check if weights of a weight-file will sum up to 1.')
parser.add_argument('-f', default="", metavar='default_weights', help='use different default weight file (default: see configuration)')
parser.add_argument('-L', default=0.999, metavar='LOWEST', type=float,  help='lowest weight to be accepted (default: 0.999)')
parser.add_argument('-H', default=1.001, metavar='HIGHEST', type=float,  help='highest weight to be accepted (default: 1.001)')
parser.add_argument('-s', default=0, metavar='MINWEIGHTNUM', type=int,  help='lowest vertex number to check (default: 0)')
parser.add_argument('-m', default=0, metavar='MAXWEIGHTNUM', type=int,  help='highest vertex to check (default: see configuration ranges/_end_)')
args = parser.parse_args()

#
# read configuration
#
cfile = open (configfilename, "r")
config = json.load (cfile)
cfile.close()

minweights = args.s
if args.m == 0:
    maxweights = config['ranges']['_end_']
else:
    maxweights = args.m

#
# read weight-file
#
if len(args.f):
    weightfilename = args.f
else:
    weightfilename = config["default_weights"]

cfile = open (weightfilename, "r")
weights = json.load (cfile)
cfile.close()

verts = {}
for key in weights["weights"].keys():
    group = weights["weights"][key]

    for comb in range (0, len(group)):
        vnum = int(group[comb][0])
        val  = group[comb][1]
        if vnum not in verts:
            verts[vnum] = { '_sum' : val, key: val}
        else:
            verts[vnum][key] = val
            verts[vnum]['_sum'] += val


for vnum in range (minweights, maxweights):
    if vnum not in verts:
        print( str(vnum) + ": undefined")
    else:
        s = verts[vnum]['_sum']
        if s < args.L or s > args.H:
            t = str(vnum) + ": " + str(round(s, 4)) + ":="
            for group in verts[vnum].keys():
                if group != '_sum':
                    t += "  " + group + ":" + str(verts[vnum][group])
            print (t)


```
