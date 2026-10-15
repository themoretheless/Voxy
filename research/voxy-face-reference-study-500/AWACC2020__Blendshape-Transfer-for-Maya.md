# Facial rendering source scan: AWACC2020/Blendshape-Transfer-for-Maya

Repository: https://github.com/AWACC2020/Blendshape-Transfer-for-Maya

Pinned source revision: 616a6a90a41e81af6ff36f33f79de180c791ef55

Discovery topic: blendshape

Description: Transfer Blendshape between meshes with different topology

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"blendshape": ["BlendShape_Transfer/BlendShape_Transfer.py", "BlendShape_Transfer/BlendShape_Transfer_CHS.py", "BlendShape_Transfer_demonstration.mp4", "Uninstall BlendShape Transfer.py", "install BlendShape Transfer CHS.py", "install BlendShape Transfer.py"]}

## Source file: install BlendShape Transfer.py

{
  "path": "install BlendShape Transfer.py",
  "blob_sha": "1548035ad4e050528d518e55f99f92c0787f6eb7",
  "sha256": "0bea4ee99fa3f230975ddb89424644e32cd07661993134dce9f363d5023994cb",
  "lines": 99,
  "bytes": 3307,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 6
  },
  "url": "https://github.com/AWACC2020/Blendshape-Transfer-for-Maya/blob/616a6a90a41e81af6ff36f33f79de180c791ef55/install%20BlendShape%20Transfer.py",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "def onMayaDroppedPythonFile(*args, **kwargs):"
    },
    {
      "line": 17,
      "text": "def MayaDropinstall():"
    }
  ]
}

```text

# Author  : AWACS
# Time    : 2020/08/15

import os
import shutil

try:
    import maya.mel
    import maya.cmds as cmds
    isMaya = True
except ImportError:
    isMaya = False

def onMayaDroppedPythonFile(*args, **kwargs):
    pass

def MayaDropinstall():
    """Drag and drop this file into the scene executes the file."""

    dic_script = {
        "author_folder" : "AWACS",
        "Contain_Folder" : "BlendShape_Transfer",
        "Script_folder" : "BlendShape_Transfer",
        "Script_module_import" : "BlendShape_Transfer",

        "Script_name" : "BlendShape_Transfer",
        "Script_Shelf_icon" : None ,
        "imageOverlayLabel":"BsTransfer",
        "Script_annotation" : 
        "Transfer Blendshape between meshes with different topology",
    }


    ContainPath = os.path.join(os.path.dirname(__file__), dic_script["Contain_Folder"])

    if not os.path.exists(ContainPath):
        raise IOError('Cannot find ' + ContainPath)

    User_script_dir =  cmds.internalVar(userScriptDir=1)
    author_folder_dir = os.path.join( User_script_dir , dic_script['author_folder'])

    try:
        os.makedirs( author_folder_dir )
    except:
        # author_folder_dir Already exist
        pass

    # Script_folder_fullpath = os.path.join( author_folder_dir , dic_script["Script_folder"] )
    Script_folder_fullpath =  author_folder_dir +"/"+ dic_script["Script_folder"] 
    # print (Script_folder_fullpath)
    # Script_folder_fullpath = os.path.normpath(Script_folder_fullpath)
    # print (Script_folder_fullpath)    
    # print ("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA")
    # Script_folder_fullpath = os.path.normpath(Script_folder_fullpath)
    # print (Script_folder_fullpath)

    if os.path.isdir( Script_folder_fullpath ):
        print ( str( Script_folder_fullpath ) + " is Already Exist " + " -------------reinstalling")
        shutil.rmtree( Script_folder_fullpath )
    else :
        pass
    #Create Script folder and copy Contains from contain Folder 
    shutil.copytree( ContainPath , Script_folder_fullpath )

    print ('// install success')
    if dic_script["Script_Shelf_icon"]:
        iconPath = os.path.join(Script_folder_fullpath, dic_script["Script_Shelf_icon"])
        iconPath = os.path.normpath(iconPath)
        # module_name = "BlendShape_Transfer"
    else:
        iconPath = "commandButton.png"

    command = '''import sys
script_path = "{path}"
if script_path not in sys.path:
    sys.path.append(script_path)
import {module_name}
if sys.version_info.major > 2 :
    from imp import reload
reload ({module_name})
'''.format(path=Script_folder_fullpath , module_name = dic_script["Script_module_import"] )

    shelf = maya.mel.eval('$gShelfTopLevel=$gShelfTopLevel')
    parent = cmds.tabLayout(shelf, query=True, selectTab=True)
    cmds.shelfButton(
        command=command,
        annotation=dic_script["Script_annotation"] ,
        sourceType='Python',
        image=iconPath,
        image1=iconPath,
        imageOverlayLabel = dic_script["imageOverlayLabel"],
        parent=parent
    )

    print("// {Script_name} has been added to current shelf".format( Script_name = dic_script["Script_name"]))


if isMaya:
    MayaDropinstall()


```

## Source file: Uninstall BlendShape Transfer.py

{
  "path": "Uninstall BlendShape Transfer.py",
  "blob_sha": "a7c518a91aa7a04305c0ffe17eff4b366eeba53e",
  "sha256": "c7eac0911446e37c713ad518d26aed17439a44bd67180927e5b2729c897c9642",
  "lines": 62,
  "bytes": 2088,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 5
  },
  "url": "https://github.com/AWACC2020/Blendshape-Transfer-for-Maya/blob/616a6a90a41e81af6ff36f33f79de180c791ef55/Uninstall%20BlendShape%20Transfer.py",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "def onMayaDroppedPythonFile(*args, **kwargs):"
    },
    {
      "line": 17,
      "text": "def MayaDropRemove():"
    }
  ]
}

```text

# Author  : AWACS
# Time    : 2020/09/03

import os
import shutil

try:
    import maya.mel
    import maya.cmds as cmds
    isMaya = True
except ImportError:
    isMaya = False

def onMayaDroppedPythonFile(*args, **kwargs):
    pass

def MayaDropRemove():
    """Drag and drop this file into the scene executes the file."""
    dic_script = {
        "author_folder" : "AWACS",
        "Contain_Folder" : "BlendShape_Transfer",
        "Script_folder" : "BlendShape_Transfer",
        "Script_module_import" : "BlendShape_Transfer",

        "Script_name" : "BlendShape_Transfer",
        "Script_Shelf_icon" : None ,
        "imageOverlayLabel":"BsTransfer",
        "Script_annotation" : 
        "Transfer Blendshape between meshes with different topology",
    }
    Popup_Dialog = True

    User_script_dir =  cmds.internalVar(userScriptDir=1)
    author_folder_dir = os.path.join( User_script_dir , dic_script["author_folder"])

    Script_folder_fullpath = os.path.join( author_folder_dir , dic_script["Script_folder"])

    if os.path.isdir( Script_folder_fullpath ):
        shutil.rmtree( Script_folder_fullpath )
        confirmDialog_message = "{Script_name} has been Removed From your software, \
you can remove the command from shelf now".format( Script_name = dic_script["Script_name"])
        print (confirmDialog_message)
        if Popup_Dialog:
            cmds.confirmDialog( title='Uninstall Success', message=confirmDialog_message, button=['OK'] )
    else :
        confirmDialog_message = "No such path , Nothing good to remove or unistall or whatever"
        print (confirmDialog_message)
        if Popup_Dialog:
            
            cmds.confirmDialog( title='Uninstall Failed', message=confirmDialog_message, button=['OK']  )
        return

    # check if there is any thing ,if not remove entire author_folder
    # print (os.listdir ( author_folder_dir ) )
    if len( os.listdir ( author_folder_dir ) ) < 1:
        shutil.rmtree( author_folder_dir )




if isMaya:
    MayaDropRemove()


```

## Source file: install BlendShape Transfer CHS.py

{
  "path": "install BlendShape Transfer CHS.py",
  "blob_sha": "578ed0c920fb6db2fe0720378b45d4c42c74879e",
  "sha256": "4b6af6760ce4b7b42cf8395d9b75666dc6ad3f712444ad7cae91ed3f3a66acba",
  "lines": 100,
  "bytes": 3313,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 6
  },
  "url": "https://github.com/AWACC2020/Blendshape-Transfer-for-Maya/blob/616a6a90a41e81af6ff36f33f79de180c791ef55/install%20BlendShape%20Transfer%20CHS.py",
  "function_signature_candidates": [
    {
      "line": 15,
      "text": "def onMayaDroppedPythonFile(*args, **kwargs):"
    },
    {
      "line": 18,
      "text": "def MayaDropinstall():"
    }
  ]
}

```text


# Author  : AWACS
# Time    : 2020/08/15

import os
import shutil

try:
    import maya.mel
    import maya.cmds as cmds
    isMaya = True
except ImportError:
    isMaya = False

def onMayaDroppedPythonFile(*args, **kwargs):
    pass

def MayaDropinstall():
    """Drag and drop this file into the scene executes the file."""

    dic_script = {
        "author_folder" : "AWACS",
        "Contain_Folder" : "BlendShape_Transfer",
        "Script_folder" : "BlendShape_Transfer",
        "Script_module_import" : "BlendShape_Transfer_CHS",

        "Script_name" : "BlendShape_Transfer",
        "Script_Shelf_icon" : None ,
        "imageOverlayLabel":"BsTransfer",
        "Script_annotation" : 
        "Transfer Blendshape between meshes with different topology",
    }


    ContainPath = os.path.join(os.path.dirname(__file__), dic_script["Contain_Folder"])

    if not os.path.exists(ContainPath):
        raise IOError('Cannot find ' + ContainPath)

    User_script_dir =  cmds.internalVar(userScriptDir=1)
    author_folder_dir = os.path.join( User_script_dir , dic_script['author_folder'])

    try:
        os.makedirs( author_folder_dir )
    except:
        # author_folder_dir Already exist
        pass

    # Script_folder_fullpath = os.path.join( author_folder_dir , dic_script["Script_folder"] )
    Script_folder_fullpath =  author_folder_dir +"/"+ dic_script["Script_folder"] 
    # print (Script_folder_fullpath)
    # Script_folder_fullpath = os.path.normpath(Script_folder_fullpath)
    # print (Script_folder_fullpath)    
    # print ("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA")
    # Script_folder_fullpath = os.path.normpath(Script_folder_fullpath)
    # print (Script_folder_fullpath)

    if os.path.isdir( Script_folder_fullpath ):
        print ( str( Script_folder_fullpath ) + " is Already Exist " + " -------------reinstalling")
        shutil.rmtree( Script_folder_fullpath )
    else :
        pass
    #Create Script folder and copy Contains from contain Folder 
    shutil.copytree( ContainPath , Script_folder_fullpath )

    print ('// install success')
    if dic_script["Script_Shelf_icon"]:
        iconPath = os.path.join(Script_folder_fullpath, dic_script["Script_Shelf_icon"])
        iconPath = os.path.normpath(iconPath)
        # module_name = "BlendShape_Transfer"
    else:
        iconPath = "commandButton.png"

    command = '''import sys
script_path = "{path}"
if script_path not in sys.path:
    sys.path.append(script_path)
import {module_name}
if sys.version_info.major > 2 :
    from imp import reload
reload ({module_name})
'''.format(path=Script_folder_fullpath , module_name = dic_script["Script_module_import"] )

    shelf = maya.mel.eval('$gShelfTopLevel=$gShelfTopLevel')
    parent = cmds.tabLayout(shelf, query=True, selectTab=True)
    cmds.shelfButton(
        command=command,
        annotation=dic_script["Script_annotation"] ,
        sourceType='Python',
        image=iconPath,
        image1=iconPath,
        imageOverlayLabel = dic_script["imageOverlayLabel"],
        parent=parent
    )

    print("// {Script_name} has been added to current shelf".format( Script_name = dic_script["Script_name"]))


if isMaya:
    MayaDropinstall()


```

## Source file: BlendShape_Transfer/BlendShape_Transfer.py

{
  "path": "BlendShape_Transfer/BlendShape_Transfer.py",
  "blob_sha": "6b8fb104c6bbbff607cd97a56e4c07814b6a9b01",
  "sha256": "5db089177f24b3d3eac3c8a6fd659f9dcef881695d44c97ad599d3955835077d",
  "lines": 617,
  "bytes": 24714,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 38,
    "deform": 3,
    "lbs": 9,
    "rig": 7
  },
  "url": "https://github.com/AWACC2020/Blendshape-Transfer-for-Maya/blob/616a6a90a41e81af6ff36f33f79de180c791ef55/BlendShape_Transfer/BlendShape_Transfer.py",
  "function_signature_candidates": [
    {
      "line": 30,
      "text": "def BlendShape_Transfer_GUI():"
    },
    {
      "line": 109,
      "text": "def Object_Type_Checker( InputObject, Object_Type ):"
    },
    {
      "line": 124,
      "text": "def Set_Source_Geo():"
    },
    {
      "line": 135,
      "text": "def Set_Target_Geo():"
    },
    {
      "line": 146,
      "text": "def Set_Source_BS():"
    },
    {
      "line": 167,
      "text": "def Set_Existing_BS(  ):"
    },
    {
      "line": 178,
      "text": "def SearchingForBlendshape( InputNode ):"
    },
    {
      "line": 208,
      "text": "def ExecuteTransferBS():"
    },
    {
      "line": 242,
      "text": "def Create_Wrap( DriveR_Mesh, DriveN_Mesh, name, threshold = 0, maxDistance = 1.0, exclusiveBind = True,"
    },
    {
      "line": 263,
      "text": "def Create_BS( SourceGeo, BS_Name ):"
    },
    {
      "line": 271,
      "text": "def Initialize_BS( BSNode ):"
    },
    {
      "line": 284,
      "text": "def polySmooth_Custom( InputObject, Level = 1 ):"
    },
    {
      "line": 310,
      "text": "def Point_distance_Checker ( inputgeoA , inputgeoB , threshold , faster = True ):"
    },
    {
      "line": 349,
      "text": "def BlendShape_Transfer_Main( Source_Geo, BS_Node, Target_Geo, Add_to_Existing_BS, Existing_BS,"
    },
    {
      "line": 504,
      "text": "def SnapToPointBatch_Fullgeo( inputgeoA, inputgeoB, WorldSpace, threshold, VP_Refresh ):"
    },
    {
      "line": 551,
      "text": "def SnapToPointBatch_Selected_Points_only( inputgeoA, inputgeoB, PointList, WorldSpace, threshold, VP_Refresh ):"
    },
    {
      "line": 597,
      "text": "def DistanceMethod_Simplify( movedistant, threshold ):"
    },
    {
      "line": 605,
      "text": "def DistanceMethod_Full( movedistant, threshold ):"
    }
  ]
}

```text

# -!- coding: utf-8 -!-
# Author  : AWACS
# Time  : 2022/9/10
# version : 0.56 beta

import maya.cmds as cmds
import os

# def Question_Button( COMMAND = None , blank_space = 1 ,  btn_label = " ? " ):
#     cmds.text( l= " " ,w = blank_space )
#     cmds.button( c=lambda *args: eval( COMMAND ) , l= btn_label , w=20 )

# def Question_Window( imageNum = 0 , Window_name = 'Question_Window' , w = 540 , h = 540):
#     if cmds.window( Window_name , q=1, ex=1 ):
#         cmds.deleteUI( Window_name )
#     cmds.window( Window_name )
#     #cmds.dockControl( area='left', content=myWindow, allowedArea=allowedAreas )
#     imagelist = ["Question_BlendShape_Transfer_1.jpg" ,
#                  "Question_BlendShape_Transfer_2.jpg" ,
#                  "Question_BlendShape_Transfer_3.jpg" ,
#                  "Question_BlendShape_Transfer_4.jpg" ,
#                  "Question_BlendShape_Transfer_5.jpg" ,
#                  ]
#     print ( Question_Window_image_path +  "/" + imagelist[ imageNum ] )
#     cmds.paneLayout()
#     cmds.image( image= Question_Window_image_path +  "/" + imagelist[ imageNum ] )

#     cmds.showWindow( Window_name )

def BlendShape_Transfer_GUI():
    if cmds.window('BlendShape_Transfer', q = 1, ex = 1):
        cmds.deleteUI('BlendShape_Transfer')

    cmds.window('BlendShape_Transfer')
    cmds.showWindow('BlendShape_Transfer')
    cmds.columnLayout()

    cmds.rowLayout(nc = 6)
    cmds.text(label = " 1: ")
    cmds.text(label = "Transfer Method : ", w = 240, align = "right")
    cmds.optionMenu( "Transfer_Method" , w = 180 )
    cmds.menuItem( label='Wrap deformer' )
    cmds.menuItem( label='Sorry,No others method finished yet' )
    # Question_Button("Question_Window( 0 )", 10)
    cmds.setParent('..')

    cmds.rowLayout(nc = 6)
    cmds.text(label = " 2: ")
    cmds.text(label = "Select Original Mesh ( with old BS Node ) :  ", w = 240, align = "right")
    cmds.textField('Input_Source_Mesh', w = 140, h = 24, text = "")
    cmds.button(c = lambda *args: Set_Source_Geo(), label = "Confirm", w = 50)
    # Question_Button("Question_Window( 0 )", 10)
    cmds.setParent('..')

    cmds.rowLayout(nc = 6)
    cmds.text(label = " 3: ")
    cmds.text(label = "Select Target Mesh  ( Transfer to Where ) :  ", w = 240, align = "right")
    cmds.textField('Input_Target_Mesh', w = 140, h = 24, text = "")
    cmds.button(c = lambda *args: Set_Target_Geo(), label = "Confirm", w = 50)
    cmds.setParent('..')

    cmds.rowLayout(nc = 6)
    cmds.text(label = " 4: ")
    cmds.text(label = "Set Source Blendshape (Optional) :  ", w = 240, align = "right")
    cmds.textField('InputBS', w = 140, h = 24, text = "")
    cmds.button(c = lambda *args: Set_Source_BS(), label = "Confirm", w = 50)
    cmds.setParent('..')

    cmds.rowLayout(nc = 6)
    cmds.text(label = " 5: ")
    cmds.text(label = " New Blendshape Name :  ", w = 240, align = "right")
    cmds.textField('New_BS_Name', w = 180, h = 24, text = "BS_Transfered")
    cmds.setParent('..')

    cmds.rowLayout(nc = 6)
    cmds.text(label = " 6: ")
    # cmds.text(label = "Add to Existing Blendshape Node : ", w = 140)
    
    cmds.checkBox( "Add_to_exist_BS" , l ="     Add to existing Blendshape :  ", value = False, w = 240, align = "right")
    # cmds.text(label = " Existing_BS :  " ,w = 80)
    cmds.textField('Existing_BS', w = 140, h = 24, text = "")
    cmds.button(c = lambda *args: Set_Existing_BS(), label = "Confirm", w = 50)
    cmds.setParent('..')

    # cmds.rowLayout(nc = 6)
    # cmds.text(label = " 6: ")
    # cmds.text(label = "Distant threshold : ")
    # cmds.floatField("Distant_threshold", max = 0.1, min = 0.0, value = 0.002)
    # cmds.setParent('..')
    cmds.rowLayout(nc = 6)
    cmds.text(label = " 7: ")
    # cmds.text(label = "Add to Existing Blendshape Node : ", w = 140)
    
    cmds.checkBox( "Append_Driven" , l ="     Append Driven  ", value = False, w = 240, align = "right")
    # cmds.text(label = " Existing_BS :  " ,w = 80)
    cmds.setParent('..')


    cmds.rowLayout(nc = 6)
    cmds.text(label = " 8: ")
    cmds.button(c = lambda *args: ExecuteTransferBS(), label = "Execute BlendShape Transfer ", w = 180)
    cmds.setParent('..')


    cmds.rowLayout(nc = 6)
    cmds.text(label = " Notice : Scale X,Y,Z of mesh is better freeze at 1.0 ")
    cmds.setParent('..')

def Object_Type_Checker( InputObject, Object_Type ):
    """
    Check node type , check for shape if it is transform
    """
    if cmds.nodeType(InputObject) == Object_Type:
        return True
    else:
        if cmds.nodeType(InputObject) == "transform":
            RelativeNodes = cmds.listRelatives(InputObject)
            if RelativeNodes:
                for indRelative in RelativeNodes:
                    if cmds.nodeType(indRelative) == Object_Type:
                        return True
        return False

def Set_Source_Geo():
    Selection = cmds.ls(selection = True)
    if Selection:
        if Object_Type_Checker(Selection[0], "mesh"):
            cmds.textField('Input_Source_Mesh', e = 1, tx = Selection[0])
            print ("//// Source Mesh Selected : " + Selection[0])
        else:
            print ("//// Error Object Select , Please Select A Polygon Mesh ")
    else:
        print("//// Nothing Selected")

def Set_Target_Geo():
    Selection = cmds.ls(selection = True)
    if Selection:
        if Object_Type_Checker(Selection[0], "mesh"):
            cmds.textField('Input_Target_Mesh', e = 1, tx = Selection[0])
            print ("//// Target Mesh Selected : " + Selection[0])
        else:
            print ("//// Error Object Select , Please Select A Polygon Mesh ")
    else:
        print("//// Nothing Selected")

def Set_Source_BS():
    Selection = cmds.ls(selection = True)
    if Selection:
        Source_BS_list = []
        for indSel in Selection:
            if Object_Type_Checker(indSel, "blendShape"):
                Source_BS_list.append(indSel)
        print (Source_BS_list)
        if len(Source_BS_list) < 1:
            print ("//// Error Object Select , Please Select A BlendShape Node at least ")
        else:
            BS_Display = ""
            for indBS in Source_BS_list:
                BS_Display += indBS
                BS_Display += " "
            cmds.textField('InputBS', e = 1, tx = BS_Display)
            print ("//// Source BlendShape Selected : " + BS_Display)
    else:
        cmds.textField('InputBS', e = 1, tx = None)
        print ("//// Source BlendShape is empty,I will try to find a existing Blendshape if execute transfering ")

def Set_Existing_BS(  ):
    Selection = cmds.ls(selection = True)
    if Selection:
        if Object_Type_Checker(Selection[0], "blendShape"):
            cmds.textField('Existing_BS', e = 1, tx = Selection[0])
            print ("//// Existing Blendshape Selected : " + Selection[0])
        else:
            print ("//// Error Object Select , Please Select A BlendShape Node ")
    else:
        print("//// Nothing Selected")

def SearchingForBlendshape( InputNode ):
    """
    try to find Blendshape node from a mesh
    """
    RelativeNodes = cmds.listRelatives(InputNode ,shapes = True)
    SearchResult = []
    # Result_counter = 0
    if RelativeNodes:
        for indRel in RelativeNodes:
            # print (indRel)
            indRel_Rel_Nodes = cmds.listConnections(indRel)
            if indRel_Rel_Nodes:
                print (indRel_Rel_Nodes)
                for indRel_Rel_Rel in indRel_Rel_Nodes:
                    if Object_Type_Checker(indRel_Rel_Rel, "blendShape"):
                        SearchResult.append(indRel_Rel_Rel)
                        # Result_counter += 1
        SearchResult = list(set(SearchResult))
        print( InputNode + " BS Search Result : " + str(SearchResult) )
        # if Result_counter > 0 :
        print (SearchResult)
        if SearchResult == []:
            return None
        return SearchResult
        # else:
        #     return None
    else:
        return None


def ExecuteTransferBS():
    Source_Geo = cmds.textField('Input_Source_Mesh', query = 1, text = 1)
    Target_Geo = cmds.textField('Input_Target_Mesh', query = 1, text = 1)
    Source_BS_Text = cmds.textField('InputBS', query = 1, text = 1)
    NewBSName = cmds.textField('New_BS_Name', query = 1, text = 1)

    Append_Driven = cmds.checkBox( "Append_Driven" , query = 1, value = 1)
    Add_to_Exist_BS = cmds.checkBox( "Add_to_exist_BS" , query = 1, value = 1)
    Existing_BS = cmds.textField( "Existing_BS" , query = 1, text = 1) 

    if Add_to_Exist_BS:
        if len(Existing_BS) < 1 :
            Existing_BS = SearchingForBlendshape(Target_Geo)

    if Existing_BS is None :
        Add_to_Exist_BS = False

    if len(Source_BS_Text) > 0:
        Source_BS = Source_BS_Text.split(" ")
    else:
        Source_BS = SearchingForBlendshape(Source_Geo)

    print(Source_BS)

    if Source_BS:
        for IndSrcBS in Source_BS:
            if IndSrcBS == "" :
                continue
            if len(IndSrcBS) > 0 :
                print((Source_Geo, IndSrcBS, Target_Geo, Add_to_Exist_BS , Existing_BS , False, Append_Driven ,NewBSName))
                BlendShape_Transfer_Main(Source_Geo, IndSrcBS, Target_Geo, Add_to_Exist_BS , Existing_BS , False, Append_Driven ,NewBSName)
            else:
                print( "No Blendshape Node Found , please select Blendshape Node And Confirm Manually" )

def Create_Wrap( DriveR_Mesh, DriveN_Mesh, name, threshold = 0, maxDistance = 1.0, exclusiveBind = True,
                 autoWeightThreshold = True,
                 falloffMode = 0 ):
    """
    Create Wrap and setting up parameter
    """
    cmds.select(cl = 1)
    cmds.select(DriveN_Mesh)
    cmds.select(DriveR_Mesh, add = 1)
    tempString = cmds.deformer(type = 'wrap', n = name)
    wrapNode = tempString[0]
    cmds.setAttr(wrapNode + ".weightThreshold", threshold)
    cmds.setAttr(wrapNode + ".maxDistance", maxDistance)
    cmds.setAttr(wrapNode + ".exclusiveBind", exclusiveBind)
    cmds.setAttr(wrapNode + ".autoWeightThreshold", autoWeightThreshold)
    cmds.setAttr(wrapNode + ".falloffMode", falloffMode)

    cmds.AddWrapInfluence()
    cmds.select(cl = 1)
    return wrapNode

def Create_BS( SourceGeo, BS_Name ):
    cmds.select(cl = 1)
    cmds.select(SourceGeo)
    BS = cmds.blendShape(automatic = 1)
    renameResult = cmds.rename(BS[0], BS_Name)
    return renameResult


def Initialize_BS( BSNode ):
    """
    Zero out Blendshape attributes
    """
    BS_Node_target_list = cmds.listAttr(BSNode + ".w", k = True, m = True)
    cmds.setAttr(BSNode + ".envelope", 1)
    for indBS_Node_target in BS_Node_target_list:
        try:
            cmds.setAttr(BSNode + '.' + indBS_Node_target, 0)
        except:
            print("Initialize BS Func Failed , Failed to set 0 at ------" + BSNode + '.' + indBS_Node_target)


def polySmooth_Custom( InputObject, Level = 1 ):
    cmds.polySmooth(InputObject,
                    constructionHistory = 1,
                    osdSmoothTriangles = 0,
                    keepHardEdge = 0,
                    pushStrength = 0.1,
                    keepMapBorders = 1,
                    boundaryRule = 1,
                    method = 0,
                    smoothUVs = 1,
                    propagateEdgeHardness = 0,
                    keepSelectionBorder = 1,
                    roundness = 1,
                    subdivisionType = 2,
                    osdFvarPropagateCorners = 0,
                    keepTessellation = 1,
                    osdVertBoundary = 1,
                    divisions = 1,
                    osdFvarBoundary = 3,
                    keepBorder = 1,
                    continuity = 1,
                    osdCreaseMethod = 0,
                    divisionsPerEdge = 1,
                    subdivisionLevels = Level)


def Point_distance_Checker ( inputgeoA , inputgeoB , threshold , faster = True ):
    """
    check if mismatch pointpos
    mismatch return True
    match return False
    """
    ptnum = cmds.polyEvaluate( inputgeoA )['vertex']
    cmds.cycleCheck(e=False)

    if faster:
        for i in range( ptnum ):
            VPosA = cmds.xform( inputgeoA + '.vtx[' + str(i) + ']', query = 1, worldSpace = 1, translation = 1)
            VPosB = cmds.xform( inputgeoB + '.vtx[' + str(i) + ']', query = 1, worldSpace = 1, translation = 1)

            if abs( VPosB[0] - VPosA[0] ) > threshold:
                cmds.cycleCheck(e=True)
                return True
            if abs( VPosB[1] - VPosA[1] ) > threshold:
                cmds.cycleCheck(e=True)
                return True
            if abs( VPosB[2] - VPosA[2]) > threshold:
                cmds.cycleCheck(e=True)
                return True
        cmds.cycleCheck(e=True)
        return False
    else:
        for i in range( ptnum ):
            VPosA = cmds.xform( inputgeoA + '.vtx[' + str(i) + ']', query = 1, worldSpace = 1, translation = 1)
            VPosB = cmds.xform( inputgeoB + '.vtx[' + str(i) + ']', query = 1, worldSpace = 1, translation = 1)

            dist3 = ( ( VPosB[0] - VPosA[0] ) ** 2 + ( VPosB[1] - VPosA[1] ) ** 2 + ( VPosB[2] - VPosA[2] ) ** 2 ) 

            if dist3 > threshold**2 :
                cmds.cycleCheck(e=True)
                return True

        cmds.cycleCheck(e=True)
        return False

def BlendShape_Transfer_Main( Source_Geo, BS_Node, Target_Geo, Add_to_Existing_BS, Existing_BS,
                                           Subdivided_Transfer, Append_Driven , NewBSName = "BS_Transfered" ):
    # LowPolySrc , BS_of_LowPolySrc ):, HighPolyGeo , HighPoly_is_Extended , Transfer_Vertex , Transfer_Method , New_name_list ):
    # transfer the low Poly Mesh Blendshape to a Extended High Poly Mesh
    #=====================initialize===================
    BS_Node_target_list = cmds.listAttr(BS_Node + ".w", k = True, m = True)
    print (BS_Node_target_list)
    #=====================initialize===================


    # ----- recording input Connection of Source BS
    Target_input_Connections = {}
    for indTarget in BS_Node_target_list:
        indTarget_plugs = BS_Node + '.' + indTarget
        try:
            indTarget_Driven_input = cmds.listConnections( indTarget_plugs , plugs = True )[0]
            Target_input_Connections[indTarget] = indTarget_Driven_input
        except:
            Target_input_Connections[indTarget] = False

    # ----- disconnectAttr Source BS
    for indTarget , indInput_connection in Target_input_Connections.items():
        indTarget_plugs = BS_Node + '.' + indTarget
        if indInput_connection :
            try:
                cmds.disconnectAttr( indInput_connection , indTarget_plugs)
            except:
                print( "Failed to disconnectAttr {} ---- {} ".format( indTarget_plugs , indInput_connection))

    Initialize_BS(BS_Node)

    #=====================initialize End===================

    #=====================Wraping Start===================
    #1first Duplicate a Target Geo
    Target_Geo_Transfer_Ready = cmds.duplicate(Target_Geo, n = Target_Geo + "_Temp_Transfer_Ready")[0]

    # ----------- duplicate entire Source geometry with upstreamNodes is safer ?
    # Duplicated_Wrap_Src = cmds.duplicate(Source_Geo, upstreamNodes = True , renameChildren = True )
    # Duplicated_Wrap_Src = [ Source_Geo ]


    Wrap_Src_Geo = Source_Geo
    Wrap_Src_BS = BS_Node

    WarpNode = Create_Wrap(Wrap_Src_Geo, Target_Geo_Transfer_Ready, "WarpNode_Temp_For_BS_Transfer", 0, 1.0, False, False, 0)


    # try update?????????????
    # there may be some issue about maya dont update deformer before first time executing duplicate command 
    cmds.setAttr(Wrap_Src_BS + '.' + indTarget, 1)
    Temp_duplicateFirst = cmds.duplicate( Target_Geo_Transfer_Ready, n = Target_Geo_Transfer_Ready + "_Temp_" + "duplicateFirst_" + indTarget)
    cmds.setAttr(Wrap_Src_BS + '.' + indTarget, 0)
    cmds.delete(Temp_duplicateFirst)
    # try update Finished ?????????????--------------


    # --------- get individual wrap result
    Duped_Target_Geo_list = []
    for indTarget in BS_Node_target_list:

        cmds.setAttr(Wrap_Src_BS + '.' + indTarget, 1)
        cmds.setAttr(Wrap_Src_BS + '.' + indTarget, 1)
        # cmds.setAttr(WarpNode + ".envelope", 0)
        # cmds.setAttr(WarpNode + ".envelope", 1)
        # cmds.refresh( f = 1 )
        # cmds.refresh( f = 1 )
        ind_tmp_BS_Target = cmds.duplicate( Target_Geo_Transfer_Ready, n = Target_Geo_Transfer_Ready + "_Temp_" + indTarget)


        Duped_Target_Geo_list.append(ind_tmp_BS_Target[0])
        cmds.setAttr(Wrap_Src_BS + '.' + indTarget, 0)

    # ----- reconnectAttr Source BS
    for indTarget , indInput_connection in Target_input_Connections.items():
        indTarget_plugs = Wrap_Src_BS + '.' + indTarget
        if indInput_connection :
            try:
                cmds.connectAttr( indInput_connection , indTarget_plugs , f=1)
            except:
                print( "Failed to disconnectAttr {} ---- {} ".format( indTarget_plugs , indInput_connection))
                pass

    cmds.delete(WarpNode)
    cmds.delete(Target_Geo_Transfer_Ready)
    #=====================Wraping End===================
    #=====================Target BS setting up Start===================
    Target_Index = 0

    if Add_to_Existing_BS:
        FinalBS = Existing_BS[0]
        Existing_BS_Node_target_list = cmds.listAttr( FinalBS + ".w", k = True, m = True)
        print( Existing_BS_Node_target_list )
        # ----- increaseing after 2
        Target_Index += len(Existing_BS_Node_target_list) + 2
    else:
        if len(NewBSName) < 1:
            FinalBS = Create_BS(Target_Geo, Target_Geo + "_BS")
        else:
            FinalBS = Create_BS(Target_Geo, NewBSName)

    print ("Transfer to Blendshape : Blendshape name : {}".format (FinalBS) )
    #=====================Target BS setting up End===================
    
    #=====================Add Target to New BS Start===================
    cmds.select(cl = 1)
    for indTargetindex in range(len(Duped_Target_Geo_list)):
        print("transfering : {} ".format( BS_Node_target_list[indTargetindex] ))
        ind_tmp_BS_Target = Duped_Target_Geo_list[indTargetindex]
        BypassUnuseful_target = False
        
        if BypassUnuseful_target:
            mismatch = Point_distance_Checker( Target_Geo , ind_tmp_BS_Target , 0.02 )
            if mismatch is False :
                print("below point distance check threshold , by passed ")
                continue

        Target_Geo_Shape = cmds.listRelatives( Target_Geo , shapes = True )[0]

        cmds.select( ind_tmp_BS_Target )
        cmds.blendShape( FinalBS , e = 1, tc = True, w = (Target_Index, 0),
                        t = ( Target_Geo , Target_Index, ind_tmp_BS_Target + 'Shape', 1))

        indTarget = BS_Node_target_list[ indTargetindex ]
        indTarget_name = indTarget
        try:
            cmds.aliasAttr(indTarget_name, FinalBS + '.w[' + str(Target_Index) + ']')
        except:
            print( "name conflict might appear : " + indTarget_name )
            indTarget_name = indTarget_name + '_Name_Conflict'
            cmds.aliasAttr(indTarget_name , FinalBS + '.w[' + str(Target_Index) + ']')
            
        #=====================Addend driven attr Start===================
        if Append_Driven:
            indTarget_plugs = FinalBS + '.' + indTarget
            if Target_input_Connections[ indTarget ] :
                try:
                    cmds.connectAttr( Target_input_Connections[ indTarget ] , indTarget_plugs , f=1)
                except:
                    print( "Failed to disconnectAttr {} ---- {} ".format( Target_input_Connections[ indTarget ] , indTarget_plugs))
                    pass
        #=====================Addend driven attr End===================

        Target_Index += 1
        cmds.select(cl = 1)
    #=====================Add Target to New BS End ===================

    #===================== clean up ===================
    print ("cleanup ===================================================")
    
    cmds.delete(Duped_Target_Geo_list)


# From PSD_Point_Snap_GUI_2
# removed AxisLimit input
def SnapToPointBatch_Fullgeo( inputgeoA, inputgeoB, WorldSpace, threshold, VP_Refresh ):
    ptnum = cmds.polyEvaluate(inputgeoA)['vertex']
    cmds.cycleCheck(e = False)
    point_counter = 0
    if WorldSpace:
        for i in range(ptnum):
            Pointid = i

            VPosA = cmds.xform(inputgeoA + '.vtx[' + str(Pointid) + ']', query = 1, worldSpace = 1, translation = 1)
            VPosB = cmds.xform(inputgeoB + '.vtx[' + str(Pointid) + ']', query = 1, worldSpace = 1, translation = 1)
            movedistant = []
            movedistant.append(VPosB[0] - VPosA[0])
            movedistant.append(VPosB[1] - VPosA[1])
            movedistant.append(VPosB[2] - VPosA[2])

            if DistanceMethod_Full(movedistant, threshold):
                continue
            point_counter += 1

            cmds.move(VPosB[0], VPosB[1], VPosB[2], (inputgeoA + '.vtx[' + str(Pointid) + ']'), worldSpace = 1, a = 1)
            if VP_Refresh:
                cmds.refresh()
    else:

        for i in range(ptnum):
            Pointid = i

            VPosA = cmds.xform(inputgeoA + '.vtx[' + str(Pointid) + ']', query = 1, objectSpace = 1, translation = 1)
            VPosB = cmds.xform(inputgeoB + '.vtx[' + str(Pointid) + ']', query = 1, objectSpace = 1, translation = 1)
            movedistant = []
            movedistant.append(VPosB[0] - VPosA[0])
            movedistant.append(VPosB[1] - VPosA[1])
            movedistant.append(VPosB[2] - VPosA[2])

            if DistanceMethod_Full(movedistant, threshold):
                continue
            point_counter += 1

            cmds.move(movedistant[0], movedistant[1], movedistant[2], (inputgeoA + '.vtx[' + str(Pointid) + ']'),
                      objectSpace = 1, r = 1)
            if VP_Refresh:
                cmds.refresh()

    cmds.cycleCheck(e = True)
    print(str(point_counter) + " point Snap to the Target position")


def SnapToPointBatch_Selected_Points_only( inputgeoA, inputgeoB, PointList, WorldSpace, threshold, VP_Refresh ):
    cmds.cycleCheck(e = False)
    point_counter = 0
    if WorldSpace:
        for i in PointList:
            Pointid = int(i[:-1].split("[")[1])

            VPosA = cmds.xform(inputgeoA + '.vtx[' + str(Pointid) + ']', query = 1, worldSpace = 1, translation = 1)
            VPosB = cmds.xform(inputgeoB + '.vtx[' + str(Pointid) + ']', query = 1, worldSpace = 1, translation = 1)
            movedistant = []
            movedistant.append(VPosB[0] - VPosA[0])
            movedistant.append(VPosB[1] - VPosA[1])
            movedistant.append(VPosB[2] - VPosA[2])

            if DistanceMethod_Full(movedistant, threshold):
                continue
            point_counter += 1

            cmds.move(VPosB[0], VPosB[1], VPosB[2], (inputgeoA + '.vtx[' + str(Pointid) + ']'), worldSpace = 1, a = 1)
            if VP_Refresh:
                cmds.refresh()
    else:

        for i in PointList:
            Pointid = int(i[:-1].split("[")[1])

            VPosA = cmds.xform(inputgeoA + '.vtx[' + str(Pointid) + ']', query = 1, objectSpace = 1, translation = 1)
            VPosB = cmds.xform(inputgeoB + '.vtx[' + str(Pointid) + ']', query = 1, objectSpace = 1, translation = 1)
            movedistant = []
            movedistant.append(VPosB[0] - VPosA[0])
            movedistant.append(VPosB[1] - VPosA[1])
            movedistant.append(VPosB[2] - VPosA[2])

            if DistanceMethod_Full(movedistant, threshold):
                continue
            point_counter += 1

            cmds.move(movedistant[0], movedistant[1], movedistant[2], (inputgeoA + '.vtx[' + str(Pointid) + ']'),
                      objectSpace = 1, r = 1)
            if VP_Refresh:
                cmds.refresh()

    cmds.cycleCheck(e = True)
    print(str(point_counter) + " point Snap to the Target position")


def DistanceMethod_Simplify( movedistant, threshold ):
    if abs(movedistant[0]) < threshold:
        if abs(movedistant[1]) < threshold:
            if abs(movedistant[2]) < threshold:
                return False
    return True


def DistanceMethod_Full( movedistant, threshold ):
    dx = movedistant[0]
    dy = movedistant[1]
    dz = movedistant[2]
    Distance = dx * dx + dy * dy + dz * dz
    if Distance > (threshold * threshold * threshold):
        return False
    else:
        return True

# global Question_Window_image_path
# Question_Window_image_path = os.path.join(os.path.dirname(__file__), 'Question_BlendShape_Transfer')
BlendShape_Transfer_GUI()


```
