# Facial rendering source scan: Christoferis/RhubarbBarbara

Repository: https://github.com/Christoferis/RhubarbBarbara

Pinned source revision: 32f42a7a92cabb86c7bf39065bbd28b8f8d819b2

Discovery topic: mouth animation

Description: A Python GUI script designed to work with Rhubarb Lip Sync to create mouth animation fast and easy in just mere seconds (depending on video length)

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: installer_win.py

{
  "path": "installer_win.py",
  "blob_sha": "bb24fd39635a6ac46ca523e0b5e5ca0f24ac25f2",
  "sha256": "5f47599f31c0496c49843c9508a5407ce7e4827073b356d2da501631ceef8356",
  "lines": 85,
  "bytes": 2438,
  "selection_score": 0,
  "keyword_line_counts": {
    "lip": 1
  },
  "url": "https://github.com/Christoferis/RhubarbBarbara/blob/32f42a7a92cabb86c7bf39065bbd28b8f8d819b2/installer_win.py",
  "function_signature_candidates": [
    {
      "line": 18,
      "text": "def get_rhubarb():"
    },
    {
      "line": 32,
      "text": "def standard_path():"
    },
    {
      "line": 49,
      "text": "def finalize():"
    },
    {
      "line": 64,
      "text": "def main():"
    }
  ]
}

```text

#installer for Rhubarb Barbara by Christoferis GNU GPL v3
# https://github.com/christoferis/rhubarbbarbara

import json
from os import system, path as pt
import pip
from time import sleep



welcome = '''
Welcome to the Installer / Configurator of RhubarbBarbara written by Christoferis!
This Wizard will guide you through configurating and installing all the needs of RhubarbBarbara.
'''
config = None


def get_rhubarb():
    global config
    
    path = input("Please provide the path to Rhubarb Lip Sync \n (ex.: D:/Music/rhubarb-lip-sync-1.10.0-win32/rhubarb-lip-sync-1.10.0-win32 or D:/Music/rhubarb-lip-sync-1.10.0-win32/rhubarb-lip-sync-1.10.0-win32/rhubarb.exe): ")

    #check for .exe
    if path.find(".exe") <= -1:
        path += "rhubarb.exe"

    #test if file exists
    config["rhubarb"] = path


#get the standard path if desired
def standard_path():
    global config
    path = input("Do you want to set a standard output path? Type NONE to skip: ")

    if path in ("NONE", "none"):
        config["standard-savepath"] = ""
    else:
        if pt.isdir(path) == True:
            config["standard-savepath"] = path

        #return the function if check fails
        else:
            print("Invalid Path or Path doesn't exist")
            return standard_path()

    
#install the packages + write config
def finalize():
    global config

    #write everything
    with open("config.json", mode="w") as cfg:
        cfg.write(json.dumps(config))

    #confirm
    choice = input("Do you want to proceed installing the packages? [y / n]: ")

    if choice in ("Y", "y"):
        # two different versions: py launcher and no py launcher
        system("py -3 -m pip install moviepy")
        system("pip3 install moviepy")

def main():
    global welcome
    global config

    #open config
    with open("config.json", mode="r") as cfg:
        config = json.loads(cfg.read())

    print(welcome)
    get_rhubarb()
    standard_path()
    finalize()

    print("\n\nThanks for installing RhubarbBarbara! Usage Tutorial can be found here: https://sites.google.com/view/christoferis/code-projects/rhubarbbarbara")
    sleep(10)


try:
    main()
except Exception as e:
    print("An Error Occured while Installing, try running it again, if problem persists make an Github Issue \n(https://github.com/christoferis/RhubarbBarbara)\n\n" + str(e) + "\n\n This window closes in 10 Seconds automatically")
    sleep(10)

```

## Source file: RhubarbBarbara.py

{
  "path": "RhubarbBarbara.py",
  "blob_sha": "0902d6fe9cf1b9e21f80b239746b0234ff6d4208",
  "sha256": "a01db67dc98ca61c7e23bb3c8bc6f966c18beb8d4efb70b9e4892c914ac69b8d",
  "lines": 369,
  "bytes": 10330,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 1,
    "mouth": 27,
    "rig": 4,
    "lip": 9
  },
  "url": "https://github.com/Christoferis/RhubarbBarbara/blob/32f42a7a92cabb86c7bf39065bbd28b8f8d819b2/RhubarbBarbara.py",
  "function_signature_candidates": [
    {
      "line": 35,
      "text": "def about():"
    },
    {
      "line": 54,
      "text": "def videoImage(data):"
    },
    {
      "line": 59,
      "text": "def rhubarb():"
    },
    {
      "line": 94,
      "text": "def __init__(self, window, mouthshape):"
    },
    {
      "line": 127,
      "text": "def create_wid(self):"
    },
    {
      "line": 157,
      "text": "def remove_from_dict(self):"
    },
    {
      "line": 169,
      "text": "def add_to_dict(self):"
    },
    {
      "line": 184,
      "text": "def process():"
    },
    {
      "line": 231,
      "text": "def start():"
    },
    {
      "line": 268,
      "text": "def get_path(widget, type):"
    },
    {
      "line": 295,
      "text": "def gui(window, stdpath):"
    },
    {
      "line": 339,
      "text": "def main():"
    }
  ]
}

```text

#RhubarbBarbara is a Python tool for using Rhubarb Lip Sync in Production, without the need of Software like AE, OpenToonz etc.
#by creating videos out of the data provided
#by Christoferis GNU GPL v3 (https://www.sites.google.com/view/christoferis)
#trying to use a minimal amount of non stock dependencies

import json
import tkinter as tk
from os import mkdir
from random import random
from subprocess import PIPE, Popen
from tkinter import filedialog as fd
from tkinter import messagebox as mb

from moviepy.editor import AudioFileClip, ImageClip, concatenate_videoclips
from PIL import Image, ImageTk

#Todo: 
#V.1:
# - Loading bar 
# - About Page, icon + info
# - FPS Option

# optional
# - Path Information display at image thing
# - Better UI
# - freeze

#vars
output = str()
audiopath = str()
savepath = str()
config = None
window = None

def about():
    abt = tk.Toplevel()

    #Creator, and Thanks
    tk.Label(abt, text='''
    RhubarbBarbara, an application by Christoferis\n
    written in Python 3.7.8, Version 1.0\n
    This App uses parts from Rhubarb Lip Sync by Daniel S. Wolf(https://github.com/DanielSWolf/rhubarb-lip-sync) and MoviePy by Zulko(https://github.com/Zulko/moviepy) \n 
    This App is maintained by Christoferis but feel free to help out!\n\n

    Repo: https://github.com/Christoferis/RhubarbBarbara

    Submit an Issue on Github if a problem persists or view the website (https://sites.google.com/view/christoferis/code-projects/rhubarbbarbara) for Troubleshooting measures.

    '''
    ).pack()



def videoImage(data):
    #function version of prior videoImage Class (Problems with class instances)
    return ImageClip(img=mouthSelector.mouthshapes[data["value"]], transparent=True, duration=float(data["end"]) - float(data["start"]))

#Rhubarb integration
def rhubarb():
    global audiopath
    global config

    #get rhubarb path
    rhu = config["rhubarb"]

    #check for extended shapes
    extshapes = ""
    for shape in mouthSelector.mouthshapes:
        #g
        if shape == "G":
            extshapes += "G"
        #h
        elif shape == "H":
            extshapes += "H"
        #x
        elif shape == "X":
            extshapes += "X"
        else:
            pass
    
    print(extshapes)

    #open a cmd instance of Rhubarb
    cmd = Popen([rhu, "-f", "json", "--extendedShapes", extshapes, audiopath], stdout=PIPE)
    
    result = cmd.communicate()

    return json.loads(result[0])["mouthCues"]

#class for mouthselector thingy
class mouthSelector:
    mouthshapes = dict()

    def __init__(self, window, mouthshape):
        self.window = window
        self.logo = mouthshape
        self.button = None
        

        #mouthshape selector:
        if mouthshape in ("A","B","C","D","E","F"):
            self.ext = False
        else:
            self.ext = True

        if mouthshape == "A":
            self.image = "icons\lisa-A.png"
        elif mouthshape == "B":
            self.image = "icons\lisa-B.png"
        elif mouthshape == "C":
            self.image = "icons\lisa-C.png"
        elif mouthshape == "D":
            self.image = "icons\lisa-D.png"
        elif mouthshape == "E":
            self.image = "icons\lisa-E.png"
        elif mouthshape == "F":
            self.image = "icons\lisa-F.png"
        elif mouthshape == "G":
            self.image = "icons\lisa-G.png"
        elif mouthshape == "H":
            self.image = "icons\lisa-H.png"
        elif mouthshape == "X":
            self.image = "icons\lisa-X.png"
        
        pass

    def create_wid(self):

        shape = tk.Frame(self.window)
        
        #mouthshape îdentifier
        if self.ext == True:
            prevstr = self.logo + "\n " + "(Optional)"
        else:
            prevstr = self.logo

        tk.Label(shape, text=prevstr).pack(side="left")

        #Mouthshape preview (https://github.com/DanielSWolf/rhubarb-lip-sync#readme)
        img = ImageTk.PhotoImage(Image.open(self.image).resize((75, 62)))
        imgframe = tk.Label(shape, image=img)
        imgframe.image = img
        imgframe.pack(side="left")


        #button to remove image
        rembutton = tk.Button(shape, text="X", command=self.remove_from_dict)
        rembutton.pack(side="right", fill="both")

        #button to open filedialog
        self.button = tk.Button(shape, text="Add an Image", command=self.add_to_dict)
        self.button.pack(side="right", fill="both")

        
        return shape

    def remove_from_dict(self):
        #reset button color
        self.button.config(background="#F0F0F0", foreground="black")

        #remove from dict
        try:
            del mouthSelector.mouthshapes[self.logo]
        except KeyError:
            pass

        pass

    def add_to_dict(self):
        #add in only png images / jpg 
        shape = fd.askopenfilename(defaultextension=".png")
        
        
        if shape != "":
            #open and append to dict
            mouthSelector.mouthshapes[self.logo] = shape

            #set button color
            self.button.config(background="blue", foreground="white")
        else:
            print(shape)


def process():
    global output
    global audiopath

    #find the codec
    mouthData = rhubarb()

    if output.find(".mp4") >= 0:
        codec = "libx264"
        video = True

    elif output.find(".avi") >= 0:
        codec = "png"
        video = True

    #add in image sequence format and convert to path + img identifier using the foldername
    elif output.find(".folder"):
        #make directory
        output = output.replace(".folder", "")
        mkdir(output)
        #make identifier + complete output path
        ident = output.split("/")
        #complete output
        output = output + "\\" + ident[len(ident) - 1] + "%06d.png"

        video = False


    imageclips = list()

    for data in mouthData:
        imageclips.append(videoImage(data))
    
    #concatenate all 
    final = concatenate_videoclips(imageclips, method="compose")
    

    #Render out, if video True = Render video and add audio, else render imagesequence
    if video == True:
        final = final.set_audio(AudioFileClip(audiopath))
        final.write_videofile(output, codec=codec, fps=60)
        
    elif video == False:
        final.write_images_sequence(fps=60, withmask=True, nameformat=output)
    

#startmethod + checker
def start():
    #flag int for keeping missing things at minimum
    global audiopath
    global output

    flag = int(0)

    #check images (for the important ones)
    for shape in ("A","B","C","D","E","F"):
        if shape not in mouthSelector.mouthshapes:
            flag += 1
            break

    #check audiopath + output
    if output == "":
        flag += 1
    
    if audiopath == "":
        flag += 1 

    #endcheck  
    print(flag)
    if flag > 0:
        #show error screen
        tk.messagebox.showinfo(title="You forgot something!",
        message=
        '''
        You can't start yet!
        Parts are incomplete, please correct them and try again
        if this Problem persists please open an issue on GitHub"
        ''' + " " + str(flag))
    else:
        process()
        pass
        

#Filegrabber for arbitrary files
def get_path(widget, type):
    global audiopath
    global output

    #reset display widget and add in path info
    #audio for audio path, output for output path
    if type == "audio":
        path = fd.askopenfilename(defaultextension=".wav", filetypes=[('WAV-Audio(*.wav)', '*.wav')])

        if path != '':
            widget.set(path)
            audiopath = path
        else:
            pass

    elif type == "output":
        path = fd.asksaveasfilename(defaultextension=".avi", filetypes=
        [('MP4-Video(*.mp4)', '*.mp4'), ("AVI-Video(*.avi)", "*.avi"), ('Image Sequence(*.*)', '*.folder')]
        )

        if path != '':
            widget.set(path)
            output = path
        else:
            pass

#GUI
def gui(window, stdpath):

    #about page
    tk.Button(window, text="Credits + Help", command=about).pack(side="top")

    #make main Frame for the images
    top = tk.Frame(window)

    for shape in ("X","H","G","F","E","D","C","B","A"):
        frame = mouthSelector(window=top, mouthshape=shape)
        frame.create_wid().pack(side="bottom")

    top.pack(expand=2, pady=10, fill="both")

    #Frame for the start Button and Audio Input
    bottom = tk.Frame(window)
    
    #audio Entry box + Stringvar
    audiopath = tk.StringVar()
    audiopath.set("Path to Audio")

    audio = tk.Entry(bottom, state="readonly", textvariable=audiopath)
    audio.pack(side="left", expand=1, fill="x")

    #outputpath
    outputpath = tk.StringVar()
    outputpath.set(stdpath)

    output = tk.Entry(bottom, state="readonly", textvariable=outputpath)
    output.pack(side="left", expand=1, fill="x")

    #start button
    tk.Button(bottom, text="start", background="green", foreground="white", command=start).pack(side="right", fill="both")
    
    #search file buttons
    #Audio File Button
    tk.Button(bottom, text="Open Audio File", command=lambda: get_path(audiopath, type="audio")).pack(side="right", fill="both")

    #Output Button
    tk.Button(bottom, text="Set Output", command=lambda: get_path(outputpath, type="output")).pack(side="bottom", fill="both")

    bottom.pack(side="bottom", fill="both", expand=2, pady= 50, padx=50)

#Main Function 
def main():
    #open config file and fetch and make random name
    global config
    global output
    global window
    config = json.loads(open("config.json", mode="r").read())
    
    #standard config
    if config["standard-savepath"] != "":
        standard = config["standard-savepath"] + "/" + str(round(random()*100000 + 1)) + ".avi"
        output = standard
    else:
        standard = "Outputpath"
    
    #construct main window and call GUI
    
    window = tk.Tk()
    window.title("RhubarbBarbara by Christoferis (v1.0)")
    
    gui(window=window, stdpath=standard)


    window.mainloop()
    
    pass

try:
    main()
except Exception as e:
    mb.showwarning(message="A problem occured, check Error message for details, and if issue persists, make an Github Issue \n(https://github.com/christoferis/RhubarbBarbara) \n\n" + str(e))
    exit()


```

## Source file: releases/release1.0/installer_win1.0.py

{
  "path": "releases/release1.0/installer_win1.0.py",
  "blob_sha": "cb3a6f64f450f022b389b87e9d1d0a21d37ed035",
  "sha256": "89a66b4c5ac77dae66e6bb1721d341e4bfdf0d7ab7dd1375c2780e5f944807b0",
  "lines": 84,
  "bytes": 2333,
  "selection_score": 0,
  "keyword_line_counts": {
    "lip": 1
  },
  "url": "https://github.com/Christoferis/RhubarbBarbara/blob/32f42a7a92cabb86c7bf39065bbd28b8f8d819b2/releases/release1.0/installer_win1.0.py",
  "function_signature_candidates": [
    {
      "line": 18,
      "text": "def get_rhubarb():"
    },
    {
      "line": 32,
      "text": "def standard_path():"
    },
    {
      "line": 49,
      "text": "def finalize():"
    },
    {
      "line": 63,
      "text": "def main():"
    }
  ]
}

```text

#installer for Rhubarb Barbara by Christoferis CC BY 4.0
# https://github.com/christoferis/rhubarbbarbara

import json
from os import system, path as pt
import pip
from time import sleep



welcome = '''
Welcome to the Installer / Configurator of RhubarbBarbara written by Christoferis!
This Wizard will guide you through configurating and installing all the needs of RhubarbBarbara.
'''
config = None


def get_rhubarb():
    global config
    
    path = input("Please provide the path to Rhubarb Lip Sync \n (ex.: D:/Music/rhubarb-lip-sync-1.10.0-win32/rhubarb-lip-sync-1.10.0-win32 or D:/Music/rhubarb-lip-sync-1.10.0-win32/rhubarb-lip-sync-1.10.0-win32/rhubarb.exe): ")

    #check for .exe
    if path.find(".exe") <= -1:
        path += "rhubarb.exe"

    #test if file exists
    config["rhubarb"] = path


#get the standard path if desired
def standard_path():
    global config
    path = input("Do you want to set a standard output path? Type NONE to skip: ")

    if path in ("NONE", "none"):
        config["standard-savepath"] = ""
    else:
        if pt.isdir(path) == True:
            config["standard-savepath"] = path

        #return the function if check fails
        else:
            print("Invalid Path or Path doesn't exist")
            return standard_path()

    
#install the packages + write config
def finalize():
    global config

    #write everything
    with open("config.json", mode="w") as cfg:
        cfg.write(json.dumps(config))

    #confirm
    choice = input("Do you want to proceed installing the packages? [y / n]: ")

    if choice in ("Y", "y"):
        pip.main(args=["install", "moviepy"])


def main():
    global welcome
    global config

    #open config
    with open("config.json", mode="r") as cfg:
        config = json.loads(cfg.read())

    print(welcome)
    get_rhubarb()
    standard_path()
    finalize()

    print("\n\nThanks for installing RhubarbBarbara! Usage Tutorial can be found here: https://sites.google.com/view/christoferis/code-projects/rhubarbbarbara")
    sleep(10)


try:
    main()
except Exception as e:
    print("An Error Occured while Installing, try running it again, if problem persists make an Github Issue \n(https://github.com/christoferis/RhubarbBarbara)\n\n" + str(e) + "\n\n This window closes in 10 Seconds automatically")
    sleep(10)

```

## Source file: releases/release1.1/installer_win1.1.py

{
  "path": "releases/release1.1/installer_win1.1.py",
  "blob_sha": "bb24fd39635a6ac46ca523e0b5e5ca0f24ac25f2",
  "sha256": "5f47599f31c0496c49843c9508a5407ce7e4827073b356d2da501631ceef8356",
  "lines": 85,
  "bytes": 2438,
  "selection_score": 0,
  "keyword_line_counts": {
    "lip": 1
  },
  "url": "https://github.com/Christoferis/RhubarbBarbara/blob/32f42a7a92cabb86c7bf39065bbd28b8f8d819b2/releases/release1.1/installer_win1.1.py",
  "function_signature_candidates": [
    {
      "line": 18,
      "text": "def get_rhubarb():"
    },
    {
      "line": 32,
      "text": "def standard_path():"
    },
    {
      "line": 49,
      "text": "def finalize():"
    },
    {
      "line": 64,
      "text": "def main():"
    }
  ]
}

```text

#installer for Rhubarb Barbara by Christoferis GNU GPL v3
# https://github.com/christoferis/rhubarbbarbara

import json
from os import system, path as pt
import pip
from time import sleep



welcome = '''
Welcome to the Installer / Configurator of RhubarbBarbara written by Christoferis!
This Wizard will guide you through configurating and installing all the needs of RhubarbBarbara.
'''
config = None


def get_rhubarb():
    global config
    
    path = input("Please provide the path to Rhubarb Lip Sync \n (ex.: D:/Music/rhubarb-lip-sync-1.10.0-win32/rhubarb-lip-sync-1.10.0-win32 or D:/Music/rhubarb-lip-sync-1.10.0-win32/rhubarb-lip-sync-1.10.0-win32/rhubarb.exe): ")

    #check for .exe
    if path.find(".exe") <= -1:
        path += "rhubarb.exe"

    #test if file exists
    config["rhubarb"] = path


#get the standard path if desired
def standard_path():
    global config
    path = input("Do you want to set a standard output path? Type NONE to skip: ")

    if path in ("NONE", "none"):
        config["standard-savepath"] = ""
    else:
        if pt.isdir(path) == True:
            config["standard-savepath"] = path

        #return the function if check fails
        else:
            print("Invalid Path or Path doesn't exist")
            return standard_path()

    
#install the packages + write config
def finalize():
    global config

    #write everything
    with open("config.json", mode="w") as cfg:
        cfg.write(json.dumps(config))

    #confirm
    choice = input("Do you want to proceed installing the packages? [y / n]: ")

    if choice in ("Y", "y"):
        # two different versions: py launcher and no py launcher
        system("py -3 -m pip install moviepy")
        system("pip3 install moviepy")

def main():
    global welcome
    global config

    #open config
    with open("config.json", mode="r") as cfg:
        config = json.loads(cfg.read())

    print(welcome)
    get_rhubarb()
    standard_path()
    finalize()

    print("\n\nThanks for installing RhubarbBarbara! Usage Tutorial can be found here: https://sites.google.com/view/christoferis/code-projects/rhubarbbarbara")
    sleep(10)


try:
    main()
except Exception as e:
    print("An Error Occured while Installing, try running it again, if problem persists make an Github Issue \n(https://github.com/christoferis/RhubarbBarbara)\n\n" + str(e) + "\n\n This window closes in 10 Seconds automatically")
    sleep(10)

```
