# Facial rendering source scan: makehumancommunity/community-plugins-socket

Repository: https://github.com/makehumancommunity/community-plugins-socket

Pinned source revision: d5cb7eb956624a8052bb9696b172ed0d3eda56b3

Discovery topic: MakeHuman

Description: Socket server for makehuman

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"rig": ["buildscripts/debian/copyright"]}

## Source file: buildscripts/buildPPA.py

{
  "path": "buildscripts/buildPPA.py",
  "blob_sha": "fb695f6b512bea78259dd4184a773bfb269dad8c",
  "sha256": "0b112082c88a1fef42f18dc13cfd7807a831113d7795a8fef6b7a4b472512399",
  "lines": 215,
  "bytes": 7850,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 4
  },
  "url": "https://github.com/makehumancommunity/community-plugins-socket/blob/d5cb7eb956624a8052bb9696b172ed0d3eda56b3/buildscripts/buildPPA.py",
  "function_signature_candidates": [
    {
      "line": 75,
      "text": "fn = settings[\"package_base_name\"] + \"_\" + settings[\"timestamp\"] + \".orig.tar.gz\""
    },
    {
      "line": 91,
      "text": "def _sed_replace(filepath, templateToken, replaceStr):"
    },
    {
      "line": 94,
      "text": "def buildSourceTree():"
    },
    {
      "line": 141,
      "text": "def createSourceTarballs():"
    },
    {
      "line": 153,
      "text": "def createSourceDebs():"
    },
    {
      "line": 192,
      "text": "def createBinaryDebs():"
    }
  ]
}

```text

#!/usr/bin/env python
# -*- coding: utf-8 -*-

"""
MakeHuman debian package build script

**Project Name:**      MakeHuman

**Product Home Page:** http://www.makehuman.org/

**Code Home Page:**    https://bitbucket.org/MakeHuman/makehuman/

**Authors:**           Joel Palmius, Jonas Hauquier

**Copyright(c):**      MakeHuman Team 2001-2017

**Licensing:**         AGPL3

    This file is part of MakeHuman (www.makehuman.org).

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

Create a debian DEB package for a plugin
"""

import sys
import os
import re
import subprocess
import shutil
import glob
import time

# --- CONFIGURATION SETTINGS --- 

settings = dict()

settings["package_base_name"] = "makehuman-community-plugins-socket"
settings["package_sub"] = "1ppa1"
settings["tree_to_copy"] = "8_server_socket"
settings["signString"] = "Joel Palmius <joepal1976@hotmail.com>"
settings["performSign"] = True
settings["performUpload"] = False
settings["gitpath"] = "/usr/bin/git"
settings["build_root"] = os.path.join("/tmp",settings["package_base_name"] + "_ppa_work")

# --- SECONDARY CONFIGURATION

settings["location_of_script"] = os.path.dirname(os.path.abspath(__file__))
settings["source_root"] = os.path.join(settings["location_of_script"], '..')
settings["ppa_work"] = os.path.join(settings["source_root"], "..", "work")
settings["binary_final_dest"] = os.path.join(settings["ppa_work"],"dist_deb")
settings["source_final_dest"] = os.path.join(settings["ppa_work"],"dist_ppa")
settings["timestamp"] = time.strftime("%Y%m%d%H%M%S")
settings["deb_config_location"] = os.path.join(settings["location_of_script"])
settings["deb_staging_location"] = os.path.join(settings["build_root"],settings["package_base_name"]) 
settings["main_deb_def"] = os.path.join(settings["deb_staging_location"],"debian")
settings["main_changelog"] = os.path.join(settings["main_deb_def"],"changelog")

# .orig tarball to create
fn = settings["package_base_name"] + "_" + settings["timestamp"] + ".orig.tar.gz"
settings["main_tar_file"] = os.path.abspath(os.path.join(settings["build_root"], fn))

print("--- SETTINGS ---")
for key in settings:
  print(key.ljust(20) + ": " + str(settings[key]))
print(" --- ")

# --- EVERYTHING BELOW THIS POINT IS LOGIC, HANDS OFF ---

print("Source root: " + settings["source_root"])
if not os.path.isdir( os.path.join(settings["source_root"], '.git') ):
  print("Error, the git root folder %s does not contain .git folder!" % settings["source_root"])
  print("Giving up.\n\n");
  sys.exit(1)

def _sed_replace(filepath, templateToken, replaceStr):
  subprocess.check_call(['sed', '-i', '-e', 's/%s/%s/' % (templateToken, replaceStr), filepath])

def buildSourceTree():
  if os.geteuid() != 0:
    print("WARNING: You are not root. You should be running this script with root permissions!")

  if os.path.exists(settings["build_root"]):
    # Ensure dest dir is empty
    shutil.rmtree(settings["build_root"])
  os.mkdir(settings["build_root"])

  print("\nABOUT TO COPY CONTENTS\n")

  srcDir = os.path.join(settings["source_root"],settings["tree_to_copy"])
  destDir = os.path.join(settings["build_root"],settings["tree_to_copy"])

  if destDir and destDir != "" and destDir != "/" and os.path.exists(destDir):
    subprocess.check_call(["rm", "-rf", destDir])

  subprocess.check_call(["cp", "-rfv", srcDir, destDir])

  shutil.copytree(settings["deb_config_location"],settings["deb_staging_location"])

  subprocess.check_call(["rm", "-rf", os.path.join(settings["deb_staging_location"],"buildPPA.py")])

  try:
    subprocess.check_call(["chown", "-R", "0:0", settings["build_root"]])
  except:
    print("Failed to chown to root. Operation not permitted?")
  try:
    subprocess.check_call(["chmod", "-R", "644", settings["build_root"]])
    for path, dirs, files in os.walk(settings["build_root"]):
      for d in dirs:
        dpath = os.path.join(settings["build_root"], path, d)
        try:
          subprocess.check_call(["chmod", "755", dpath])
        except:
          print("Failed to chmod 755 folder %s" % dpath)
    subprocess.check_call(["chmod", "755", settings["build_root"]])
  except Exception as e:
    print("Failed to chmod " + settings["build_prepare_destination"])
    print(e)

  dummy = os.path.join(settings["build_root"],"dummy.txt")

  with open(dummy, "w") as text_file:
    text_file.write("This is only because moronic debuild cannot handle tarballs which doesn't have a non-dir entry in the root")


def createSourceTarballs():
  print("\nABOUT TO CREATE SOURCE TARBALL\n\n");

  os.chdir(settings["build_root"])

  print("Tarfile: " + settings["main_tar_file"])
  print("CWD: " + os.getcwd())

  args = ["tar","-C",settings["build_root"],"-czf", settings["main_tar_file"], settings["tree_to_copy"], "dummy.txt"]
  print(args)
  subprocess.check_call(args)

def createSourceDebs():

  print("\nWRITING CHANGELOGS\n")

  #ts = Mon, 01 Jun 2015 15:17:49 +0200

  ts = time.strftime("%a, %d %b %Y %H:%M:%S +0200")

  with open(settings["main_changelog"], "w") as text_file:
    text_file.write(settings["package_base_name"] + " (" + settings["timestamp"] + "-" + settings["package_sub"] + ") bionic; urgency=low\n\n")
    text_file.write("  * Version bump\n\n")
    text_file.write(" -- " + settings["signString"] + "  " + ts + "\n\n")

  print("\nSTARTING TO BUILD SOURCE DEB DEFINITIONS\n")
  
  print("Unpacking " + settings["main_tar_file"])
  subprocess.check_call(["tar","-C",settings["deb_staging_location"],"-xzf", settings["main_tar_file"]])

  os.chdir(settings["main_deb_def"])

  print("Debuilding in " + os.getcwd())

  if not settings["performSign"] is None and not settings["performSign"]:
    subprocess.check_call(["debuild","-S","-sa","-uc","-us"])
  else:
    subprocess.check_call(["debuild","-S","-sa"])

  print("Copying source deb output to " + settings["source_final_dest"])

  for f in glob.glob(settings["build_root"] + "/*ppa*"):
    print(f)                                                                                                                                        
    shutil.copy(f, settings["source_final_dest"])

  print("Copying source tarballs to " + settings["source_final_dest"])

  for f in glob.glob(settings["build_root"] + "/*.orig.*"):
    print(f)                                                                                                                                        
    shutil.copy(f, settings["source_final_dest"])

def createBinaryDebs():
  print("\nSTARTING TO BUILD DEB FILES\n")
  
  os.chdir(settings["main_deb_def"])

  print("Debuilding in " + os.getcwd())

  if not settings["performSign"] is None and not settings["performSign"]:
    subprocess.check_call(["debuild","-uc","-us"])
  else:
    subprocess.check_call(["debuild"])

  print("Copying deb files to " + settings["binary_final_dest"])

  for f in glob.glob(settings["build_root"] + "/*.deb"):
    print(f)                                                                                                                                        
    shutil.copy(f, settings["binary_final_dest"])

if __name__ == '__main__':
  buildSourceTree()
  createSourceTarballs()
  createSourceDebs()
  createBinaryDebs()



```

## Source file: 8_server_socket/dirops.py

{
  "path": "8_server_socket/dirops.py",
  "blob_sha": "de6fe1c8d49effe4ba28b95a85d7ee0a0dd134f6",
  "sha256": "16581492c5752c55c4028b629a3ae91638889339442cb7151eea4bca6bda85d5",
  "lines": 22,
  "bytes": 518,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/makehumancommunity/community-plugins-socket/blob/d5cb7eb956624a8052bb9696b172ed0d3eda56b3/8_server_socket/dirops.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def __init__(self, sockettaskview):"
    },
    {
      "line": 16,
      "text": "def getUserDir(self,conn,jsonCall):"
    },
    {
      "line": 19,
      "text": "def getSysDir(self,conn,jsonCall):"
    }
  ]
}

```text

#!/usr/bin/python3
# -*- coding: utf-8 -*-

import mh
import os

from .abstractop import AbstractOp

class SocketDirOps(AbstractOp):

    def __init__(self, sockettaskview):
        super().__init__(sockettaskview)
        self.functions["getUserDir"] = self.getUserDir
        self.functions["getSysDir"] = self.getSysDir

    def getUserDir(self,conn,jsonCall):
        jsonCall.data = os.path.abspath(mh.getPath())

    def getSysDir(self,conn,jsonCall):
        jsonCall.data = os.path.abspath(mh.getSysPath()) 




```

## Source file: 8_server_socket/modops.py

{
  "path": "8_server_socket/modops.py",
  "blob_sha": "200fb06473b65f89a998964819282a0e5712210e",
  "sha256": "d620c14c092020aa76d5688f830c460b08314cb7793e15570d8e1e11983bc95e",
  "lines": 33,
  "bytes": 1070,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/makehumancommunity/community-plugins-socket/blob/d5cb7eb956624a8052bb9696b172ed0d3eda56b3/8_server_socket/modops.py",
  "function_signature_candidates": [
    {
      "line": 8,
      "text": "def __init__(self, sockettaskview):"
    },
    {
      "line": 14,
      "text": "def getAvailableModifierNames(self,conn,jsonCall):"
    },
    {
      "line": 17,
      "text": "def getAppliedTargets(self,conn,jsonCall):"
    },
    {
      "line": 20,
      "text": "def applyModifier(self,conn,jsonCall):"
    }
  ]
}

```text

#!/usr/bin/python3
# -*- coding: utf-8 -*-

from .abstractop import AbstractOp

class SocketModifierOps(AbstractOp):

    def __init__(self, sockettaskview):
        super().__init__(sockettaskview)
        self.functions["applyModifier"] = self.applyModifier
        self.functions["getAppliedTargets"] = self.getAppliedTargets
        self.functions["getAvailableModifierNames"] = self.getAvailableModifierNames

    def getAvailableModifierNames(self,conn,jsonCall):
        jsonCall.data = self.api.modifiers.getAvailableModifierNames()

    def getAppliedTargets(self,conn,jsonCall):
        jsonCall.data = self.api.modifiers.getAppliedTargets()

    def applyModifier(self,conn,jsonCall):
        modifierName = jsonCall.getParam("modifier")
        power = float(jsonCall.getParam("power"))
        modifier = self.api.internals.getHuman().getModifier(modifierName)

        if not modifier:
            jsonCall.setError("No such modifier")
            return

        self.api.modifiers.applyModifier(modifierName,power,True)
        jsonCall.setData("OK")





```

## Source file: cli/mhrc/applyModifier.py

{
  "path": "cli/mhrc/applyModifier.py",
  "blob_sha": "f2d72b68447a874b492763b6a9f5f547bd67156f",
  "sha256": "61d9c54e754ec97202b7d82b13da3db9fc7fd614fe0ec4045b3433048be8aae7",
  "lines": 42,
  "bytes": 874,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/makehumancommunity/community-plugins-socket/blob/d5cb7eb956624a8052bb9696b172ed0d3eda56b3/cli/mhrc/applyModifier.py",
  "function_signature_candidates": [
    {
      "line": 6,
      "text": "def usage():"
    }
  ]
}

```text

#!/usr/bin/python

import sys
from mhrc.JsonCall import JsonCall

def usage():
    print "USAGE:\n"
    print "  applyModifier.py <modifier name> <power>\n";
    print "Modifier name is the name as listed by the listAvailableModifiers"
    print "script. Power is either (usually) between 0.0 and +1.0, but some"
    print "modifiers may accept a range between -1.0 and +1.0."
    sys.exit(1)

if len(sys.argv) < 3:
    usage()

if len(sys.argv) > 3:
    usage()

modname = sys.argv[1]
modval = sys.argv[2]

jsc = JsonCall()

jsc.setFunction("applyModifier")
jsc.setParam("modifier",modname);
jsc.setParam("power",modval);

response = jsc.send()

if not response:
    print "Command failed (returned null response)\n"
    sys.exit(1)

if hasattr(response,"error") and getattr(response,"error"):
    print "ERROR: " + getattr(response,"error")
    sys.exit(1)

print "OK"





```
