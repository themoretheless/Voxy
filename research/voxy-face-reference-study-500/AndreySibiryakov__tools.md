# Facial rendering source scan: AndreySibiryakov/tools

Repository: https://github.com/AndreySibiryakov/tools

Pinned source revision: 2a78f3ebfac78841eb69b2aa771a2faa10b8d827

Discovery topic: facial animation

Description: Facial animation tools

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"eye": ["eye_random_movement.py"], "skin": ["skin_percent_import_export.py"], "blendshape": ["split_blendshapes.py"], "shader": ["fix_scene_shaders_gui.py"], "rig": ["fs_rig_chunks.py"], "face": ["facefx_batch/CommitAnimation.py", "facefx_batch/ProcFaceGraph.py", "facefx_batch/SetAnimation.html", "facefx_batch/SetAnimation.py", "facefx_batch/addition_to_setAnimation.py", "facefx_batch/base_config.py", "facefx_batch/commit_facefx_chrs.py", "facefx_batch/facefx_config.py", "facefx_batch/mute_events.py", "save_animation_to_facefx.py"]}

## Source file: fs_rig_chunks.py

{
  "path": "fs_rig_chunks.py",
  "blob_sha": "5c93fb5372d8a9cd93bf644be85edeefbd84682c",
  "sha256": "4788442f6a0dbed8bd9a3f145f1ea559211df0a30b9b2fbb77017722565451df",
  "lines": 104,
  "bytes": 4266,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 4,
    "blendshape": 2,
    "jaw": 2,
    "mouth": 1,
    "lip": 2
  },
  "url": "https://github.com/AndreySibiryakov/tools/blob/2a78f3ebfac78841eb69b2aa771a2faa10b8d827/fs_rig_chunks.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def set_blink_for_emotions(bl_node, pairs, blink_ctrl, blink_bl):"
    },
    {
      "line": 62,
      "text": "def set_remap_function_points(node, data):"
    },
    {
      "line": 75,
      "text": "def set_sticky_lips(bl_node, jaw, st_1, st_2):"
    },
    {
      "line": 86,
      "text": "def set_double_blend(driver, bl, driven_1, driven_2):"
    }
  ]
}

```text

import maya.cmds as cmds

blink_pairs = {'{bl}.EyeSquint_{side}': '{bl}.squint_{side}_blink_delta',
               '{bl}.EyeOpen_{side}': '{bl}.eye_open_{side}_blink_delta',
               '{bl}.EyeUp_{side}': '{bl}.eye_up_{side}_blink_delta',
               '{bl}.MouthSmile_{side}': '{bl}.smile_{side}_blink_delta',
               '{bl}.Sneer_{side}': '{bl}.sneer_{side}_blink_delta',
               '{bl}.CheekSquint_{side}': '{bl}.cheek_squint_{side}_blink_delta'}


def set_blink_for_emotions(bl_node, pairs, blink_ctrl, blink_bl):
    # Sets rules for neutral blink value
    sum_blinks = cmds.shadingNode('plusMinusAverage', asUtility=True)
    sub_from_neutral_blink = cmds.shadingNode(
        'plusMinusAverage', asUtility=True)
    remap_neutral_blink = cmds.shadingNode('remapValue', asUtility=True)
    # Set up node
    cmds.setAttr("%s.operation" % sub_from_neutral_blink, 2)
    # Connections for sum_blinks
    for index, bl in enumerate(pairs.keys()):
        cmds.connectAttr('%s' % bl,
                         '%s.input1D[%s]' % (sum_blinks, index), f=True)

    cmds.connectAttr('%s.output1D' % sum_blinks,
                     '%s.input1D[1]' % sub_from_neutral_blink, f=True)
    # Connections for sub_from_neutral_blink
    cmds.connectAttr(
        blink_ctrl, '%s.input1D[0]' % sub_from_neutral_blink, f=True)
    cmds.connectAttr('%s.output1D' % sub_from_neutral_blink,
                     '%s.inputValue' % remap_neutral_blink, f=True)
    # Connections for remap_neutral_blink
    cmds.connectAttr('%s.outValue' % remap_neutral_blink,
                     '%s' % blink_bl, f=True)

    for driver_bl, driven_bl in pairs.iteritems():
        condition = cmds.shadingNode('condition', asUtility=True)
        cmds.connectAttr(blink_ctrl, '%s.firstTerm' % condition, f=True)
        cmds.connectAttr(driver_bl, '%s.colorIfTrueR' % condition, f=True)
        cmds.connectAttr('%s.outColorR' % condition, driven_bl, f=True)
        cmds.setAttr("%s.colorIfFalseR" % condition, 0)
        # Greater than
        cmds.setAttr("%s.operation" % condition, 2)


sides = ['R', 'L']
bl = 'blendShape1'
ctrl = 'ExpressionBlendshapes.EyeBlink_{}'
blink_bl = '%s.Blink_{}' % bl

for side in sides:
    data = {dr.format(bl=bl, side=side): dn.format(bl=bl, side=side)
            for dr, dn in blink_pairs.iteritems()}
    set_blink_for_emotions(bl, data, ctrl.format(side), blink_bl.format(side))


# Sticky lips
# (position, value)
first_node = [(0, 0), (0.1, 0.3), (0.2, 0)]
second_node = [(0.1, 0), (0.2, 0.3), (0.3, 0)]


def set_remap_function_points(node, data):

    for index, d in enumerate(data):
        position, value = d[0], d[1]
        cmds.setAttr("{}.value[{}].value_Position".format(
            node, index), position)
        cmds.setAttr("{}.value[{}].value_FloatValue".format(
            node, index), value)
        # Linear interpolation
        cmds.setAttr("{}.value[{}].value_Interp".format(
            node, index), 1)


def set_sticky_lips(bl_node, jaw, st_1, st_2):

    for sticky, data in zip([st_1, st_2], [first_node, second_node]):
        sticky_remap = cmds.shadingNode('remapValue', asUtility=True)
        set_remap_function_points(sticky_remap, data)
        cmds.connectAttr('%s.%s' % (bl_node, jaw),
                         '%s.inputValue' % sticky_remap, f=True)
        cmds.connectAttr('%s.outValue' % sticky_remap,
                         '%s.%s' % (bl_node, sticky), f=True)


def set_double_blend(driver, bl, driven_1, driven_2):
    first_node = [(0, 0), (0.5, 1), (1, 0)]
    second_node = [(0.5, 0), (1, 1)]

    for driven, data in zip([driven_1, driven_2], [first_node, second_node]):
        driven_remap = cmds.shadingNode('remapValue', asUtility=True)
        set_remap_function_points(driven_remap, data)
        cmds.connectAttr(driver, '%s.inputValue' % driven_remap, f=True)
        cmds.connectAttr('%s.outValue' % driven_remap,
                         '%s.%s' % (bl, driven), f=True)


first_node = [(0, 0), (0.3, 1), (0.6, 0)]
second_node = [(0.3, 0), (0.6, 1), (1, 0)]
third_node = [(0.6, 0), (1, 1)]


first_node = [(0, 0), (0.5, 1), (1, 0)]
second_node = [(0.5, 0), (1, 1)]


```

## Source file: split_blendshapes.py

{
  "path": "split_blendshapes.py",
  "blob_sha": "ea12f7eebccc332179e51730cbb823c14898a16d",
  "sha256": "40ea582939c6a62b4e2fd732253433c17472ba32b3d84a242e208fd640027427",
  "lines": 133,
  "bytes": 5020,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 12,
    "gum": 1,
    "rig": 3,
    "face": 2,
    "lip": 1
  },
  "url": "https://github.com/AndreySibiryakov/tools/blob/2a78f3ebfac78841eb69b2aa771a2faa10b8d827/split_blendshapes.py",
  "function_signature_candidates": [
    {
      "line": 21,
      "text": "def __init__(self, map_path='', gender='male'):"
    },
    {
      "line": 58,
      "text": "def read_cmds(self):"
    },
    {
      "line": 63,
      "text": "def in_scene(self, data, select=False):"
    },
    {
      "line": 77,
      "text": "def blendshapes_in_scene(self, select=False):"
    },
    {
      "line": 80,
      "text": "def elements_in_scene(self, select=False):"
    },
    {
      "line": 83,
      "text": "def mesh_for_proc(self, a):"
    },
    {
      "line": 89,
      "text": "def prepare_blendshapes(self):"
    }
  ]
}

```text

"""
import split_blendshapes
reload(split_blendshapes)

spl = split_blendshapes.SplitBlendshapes()
spl.blendshapes_in_scene()
spl.elements_in_scene()
spl.prepare_blendshapes()
"""


import sys
import maya.cmds as cmds
sys.path.append('u:/face/scripts/')
import functions as f
reload(f)


class SplitBlendshapes():

    def __init__(self, map_path='', gender='male'):
        if not map_path:
            self.map_path = 'u:/face/mocap/cm_mapping.txt'
        else:
            self.map_path = map_path

        if gender == 'male':
            # Difference between simple and complex in lip zones.
            # Simple one is divided on left, right.
            # Complex is divided on upper left, upper right, lower left, lower right.
            self.div_mesh_simple = 'male_head_simple'
            self.div_mesh_complex = 'male_head_complex'
        elif gender == 'female':
            self.div_mesh_simple = 'female_head_simple'
            self.div_mesh_complex = 'female_head_complex'
        else:
            sys.exit('Gender must be "male" or "female".')
        self.cm_group = 'cm_rig'
        # Operation names to get elements from combined blendshape
        self.rename = 'rename'
        self.split = 'split'
        self.symm = 'symm'
        self.move = 'move'
        # Conditions for "split" operation
        self.simple = 'simple'
        self.complex = 'complex'
        self.data = []

        if not cmds.objExists(self.div_mesh_simple) or not cmds.objExists(self.div_mesh_complex):
            print '# Need', self.div_mesh_simple, self.div_mesh_complex
            sys.exit('# No or not enough template meshes to split elements from.')
        self.read_cmds()
        # Meshes in scene, also called combined. Happiness.
        self.blendshapes = list(set([cmd[2] for cmd in self.data if cmd[2]]))
        # Meshes obtained from combined ones. Low left lid of happiness.
        self.elements = list(set([cmd[0] for cmd in self.data if cmd[0]]))

    def read_cmds(self):
        with open(self.map_path) as g:
            for l in g:
                self.data.append(l.strip().split("\t"))

    def in_scene(self, data, select=False):
        existing_data = [d for d in data if cmds.objExists(d)]
        missing_data = [d for d in data if d not in existing_data]
        cmds.select(clear=True)
        if select:
            cmds.select(existing_data)

        if missing_data:

            for bl in sorted(missing_data):
                print '#', bl, 'not found'
        else:
            print '# All meshes are in scene.'

    def blendshapes_in_scene(self, select=False):
        self.in_scene(self.blendshapes, select=select)

    def elements_in_scene(self, select=False):
        self.in_scene(self.elements, select=select)

    def mesh_for_proc(self, a):
        if a == self.simple:
            return self.div_mesh_simple
        elif a == self.complex:
            return self.div_mesh_complex

    def prepare_blendshapes(self):
        '''
        Collects all meshes to split and argument, simple or complex
        Split every mesh in collected list
        Moves or/and renames splitted meshes
        Does the same with the others
        Deletes tmp meshes and groups
        '''
        if not cmds.objExists(self.cm_group):
            cmds.group(n=self.cm_group, empty=True)

        # Collects split meshes
        split_meshes = [(bl, opt) for res, suf, bl, proc,
                        opt in self.data if proc == self.split]
        split_meshes = list(set(split_meshes))
        split_grps = []
        for bl, opt in split_meshes:
            if not cmds.objExists(bl):
                continue
            split_grps.append(f.split_blendshape(self.mesh_for_proc(opt), bl))

        # Collects meshes for renaming including split
        split_meshes_for_rename = [[bl + '_' + suf, res] for res, suf, bl, proc,
                                   opt in self.data if proc == self.split]
        meshes_for_rename = [[bl, res] for res, suf, bl, proc,
                             opt in self.data if proc == self.rename]

        for a, b in split_meshes_for_rename + meshes_for_rename:
            if cmds.objExists(b):
                print b, 'exists, cannot rename', a, 'to', b
                continue
            if not cmds.objExists(a):
                continue
            a_dup = cmds.duplicate(a)[0]
            cmds.rename(a_dup, b)
        # Moves meshes to a single group
        meshes_for_move = [res for res, suf, bl, proc,
                           opt in self.data if proc == self.move]
        renamed_meshes = [
            b for a, b in split_meshes_for_rename + meshes_for_rename]

        meshes_to_parent = [m for m in meshes_for_move + renamed_meshes if cmds.objExists(m)]
        cmds.parent(meshes_to_parent, self.cm_group)
        # Deletes tmp meshes and groups
        cmds.delete(split_grps)


```

## Source file: eye_random_movement.py

{
  "path": "eye_random_movement.py",
  "blob_sha": "fd9fd4f549ace5e7067cc8ca805343a75caf7b61",
  "sha256": "db7e271264122cba843b1d905ae52a3f1c469299c77fe326858a55c568a79c88",
  "lines": 63,
  "bytes": 1797,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 2
  },
  "url": "https://github.com/AndreySibiryakov/tools/blob/2a78f3ebfac78841eb69b2aa771a2faa10b8d827/eye_random_movement.py",
  "function_signature_candidates": []
}

```text

import random

'''
# Far blend
# In frames
blend = (0, 3)
# In frames
duration = (7, 15)
# In angles
intencity_vertical = (-3.5, 3.5)
# More horizontal movement over vertical
intencity_horizontal = (-4.5, 4.5)

# Close blend
# In frames
blend = (0, 3)
# In frames
duration = (5, 12)
# In angles
intencity_vertical = (-2, 2)
# More horizontal movement over vertical
intencity_horizontal = (-3, 3)
'''

# In frames
blend = (0, 3)
# In frames
duration = (5, 10)
# In angles
intencity_vertical = (-2, 2)
# More horizontal movement over vertical
intencity_horizontal = (-5, 5)
start, end = f.get_timeline()
# start, end = 0, 100
attr_x = ['bn_eye_{}.rotateX'.format(side) for side in ['r', 'l']]
attr_z = ['bn_eye_{}.rotateZ'.format(side) for side in ['r', 'l']]
frame_value_data = {}
blend_val = 0
duration_val = 0
intencity_val_x = 0
intencity_val_z = 0
# Generates key values over time
for frame in xrange(start, end + 1):
    # If duration with with static key value ended, but blend not added
    if duration_val == 0 and blend_val != 0:
        frame_value_data[frame] = None
        blend_val -= 1
        continue
    # Both duration and blend added to data
    elif duration_val == 0 and blend_val == 0:
        intencity_val_x = random.uniform(*intencity_vertical)
        intencity_val_z = random.uniform(*intencity_horizontal)
        blend_val = round(random.uniform(*blend), 0)
        duration_val = round(random.uniform(*duration), 0)

    duration_val -= 1
    frame_value_data[frame] = [intencity_val_x, intencity_val_z]

for frame, value in frame_value_data.iteritems():
    if not value:
        continue
    cmds.setKeyframe(attr_x, time=frame, value=value[0])
    cmds.setKeyframe(attr_z, time=frame, value=value[1])


```

## Source file: fix_scene_shaders_gui.py

{
  "path": "fix_scene_shaders_gui.py",
  "blob_sha": "d195a142cbd458fceba4935b23306704e60cdac9",
  "sha256": "2b36d6a6112d51d5b7c71eb1ca760e8eadacd29e408c9b42844f2a2ed2c0ef0e",
  "lines": 490,
  "bytes": 16225,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 15,
    "rig": 1,
    "face": 9
  },
  "url": "https://github.com/AndreySibiryakov/tools/blob/2a78f3ebfac78841eb69b2aa771a2faa10b8d827/fix_scene_shaders_gui.py",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "def get_meshes():"
    },
    {
      "line": 20,
      "text": "def find_texture(texture):"
    },
    {
      "line": 40,
      "text": "def fix_texture_files_path():"
    },
    {
      "line": 55,
      "text": "def get_texture_name(texture):"
    },
    {
      "line": 72,
      "text": "def get_texture(f):"
    },
    {
      "line": 81,
      "text": "def rename_2d_texture(texture, name):"
    },
    {
      "line": 96,
      "text": "def rename_sg_nodes(template, sg='', texture='', count=''):"
    },
    {
      "line": 184,
      "text": "def find_identical_meshes(regex, vtxs_check=True, vtxs=[]):"
    },
    {
      "line": 223,
      "text": "def get_mesh_diffuse_name(mesh):"
    },
    {
      "line": 252,
      "text": "def get_most_used_texture_and_heads():"
    },
    {
      "line": 303,
      "text": "def get_shading_group(mesh):"
    },
    {
      "line": 316,
      "text": "def split_mesh_on_parts(mesh):"
    },
    {
      "line": 342,
      "text": "def add_head_sg(sg, template, count=''):"
    },
    {
      "line": 367,
      "text": "def fix_sg_names():"
    },
    {
      "line": 407,
      "text": "def fix_face_names():"
    },
    {
      "line": 438,
      "text": "def fix_materials(*args):"
    },
    {
      "line": 464,
      "text": "def enable_checkboxes(*args):"
    }
  ]
}

```text

import re
import os
import maya.cmds as cmds
import maya.mel as mel


def get_meshes():
    '''Gets all the transform meshes node names from a scene

    Returns:
            list: all meshes in scene

    '''
    objects = cmds.ls('*', type='mesh')
    meshes = cmds.listRelatives(objects, parent=True)
    meshes = list(set(meshes))
    return meshes


def find_texture(texture):
    '''Searches for the texture file by its name in a given drive.
        Uses to substitute old broken paths by up to date ones.

    Args:
        texture (str): full path of the texture

    Returns:
        str: Up to date full texture path
    '''
    path = 't:/'
    name = os.path.basename(texture)
    for path, dirs, file_names in os.walk(path):
        for file_name in file_names:
            if file_name == name:
                new_texture = os.path.join(path, file_name)
                return new_texture
    return texture


def fix_texture_files_path():
    '''Substitutes the right image path for all texture file nodes in scene.

    Returns:
        None:
    '''
    texutre_files = cmds.ls(type='file')

    for t_file in texutre_files:
        texture = cmds.getAttr("%s.fileTextureName" % t_file)
        found_texture_path = find_texture(texture)
        if found_texture_path:
            cmds.setAttr("%s.fileTextureName" % t_file, found_texture_path, type="string")


def get_texture_name(texture):
    '''Gets image file name of a texture node.

    Args:
        texture (str): file node

    Returns:
        str: image file name
    '''
    try:
        texture_path = cmds.getAttr("%s.fileTextureName" % texture)
        return os.path.basename(texture_path).split('.')[0]
    except Exception as e:
        print '# %s\nError while executing function get_texture_name().' % e
        return


def get_texture(f):
    '''Gets 2D texture node from a given texture node.

     Returns:
        str: 2D texture node name
    '''
    return cmds.ls(cmds.listHistory(f), type='place2dTexture')


def rename_2d_texture(texture, name):
    '''Renames 2D texture node from a given texture node.

    Args:
        texture (str): texture name
        name (str): new name for 2D texture

     Returns:
        None:
    '''
    texture_2d = cmds.ls(cmds.listHistory(str(texture)), type='place2dTexture')
    if texture_2d:
        cmds.rename(texture_2d[0], name)


def rename_sg_nodes(template, sg='', texture='', count=''):
    '''Renames nodes connected to a shading group or a texture according to template naming.
    Texture nodes types (normal, diffuse, specular) are guessed according to prefixes.
    They are standart: '_nm', '_refl' or '_bump'. Diffuse has no prefix.


    Args:
        template (str): name template to be given to nodes
        sg (str, optional): shading group node name. sg or texture should be filled.
        texture (str, optional): texture node name. sg or texture should be filled.
        count (str, optional): prefix. Should be written with underscore '_2'

    Returns:
        None:
    '''
    if not sg and not texture:
        return

    if not sg and texture:
        shaders = list(set(cmds.ls(cmds.listConnections(texture), materials=True)))
        sgs = list(set(cmds.ls(cmds.listConnections(shaders), type='shadingEngine')))
    else:
        sgs = [sg]
        shaders = list(set(cmds.ls(cmds.listConnections(sg), materials=True)))
    # Protects from bad named normals
    if shaders and sgs:
        nodes = cmds.listHistory(sgs)
        files = list(set(cmds.ls(nodes, type='file')))
        tangent = cmds.ls(nodes, type='bump2d')
    else:
        return
    # Pattern to specify type of materials in image file name.
    refl_pr = ur'_refl\b'
    nm_pr = ur'_nm\b'
    bump_pr = ur'_bump\b'
    # Prefixes, that are defined while renaming sg nodes.
    sg_pr = '_SG'
    sh_pr = '_M'
    diffuse_pr = '_D'
    specular_pr = '_S'
    normal_pr = '_N'
    tangent_pr = '_N_Tangent'
    texture_pr = '_2D'
    # Another protection from bad image files naming.
    # Assuming, that diffuse, specular and normal can be used only once in material.
    diffuse_used = False
    specular_used = False
    normal_used = False

    for f in list(set(files)):
        texture_name = get_texture_name(f)
        if not texture_name:
            continue

        if re.search(refl_pr, texture_name) or re.search(bump_pr, texture_name) and not specular_used:
            cmds.rename(f, template + specular_pr + count)
            rename_2d_texture(template + specular_pr + count, template + specular_pr + texture_pr + count)
            specular_used = True
        elif re.search(nm_pr, texture_name) and not normal_used:
            cmds.rename(f, template + normal_pr + count)
            rename_2d_texture(template + normal_pr + count, template + normal_pr + texture_pr + count)
            normal_used = True
        elif not diffuse_used:
            cmds.rename(f, template + diffuse_pr + count)
            rename_2d_texture(template + diffuse_pr + count, template + diffuse_pr + texture_pr + count)
            diffuse_used = True
        else:
            print '# Skipping file %s with texture name %s' % (f, texture_name)

    if tangent:
        try:
            cmds.rename(tangent, template + tangent_pr + count)
        except:
            print '# Did not rename tangent node %s' % tangent

    for shader in shaders:
        try:
            cmds.rename(shader, template + sh_pr + count)
        except:
            print '# Did not rename shader node %s' % shader

    for sg in sgs:
        try:
            cmds.rename(sg, template + sg_pr + count)
        except:
            print '# Did not rename shading group node %s' % sg


def find_identical_meshes(regex, vtxs_check=True, vtxs=[]):
    '''Searches for all polygon meshes in scene that matches regex expression
        and optional - vertex count.
        Preset vertex count fits two main types of head meshes, male, female,
        and male neck cut.


    Args:
        regex (str): regular expression
        vtxs_check (boolean, optional): condition, if to check the vertex count on
            top of the regular expression
        vtxs (list, optional): vertices count (int)

    Returns:
        list: polygon meshes that match search parameters

    Examples:
        >>> find_identical_meshes('(_head|head_)')
    '''
    # 2770 - is for cut Krest cut neck head
    if not vtxs:
        vtxs = [2782, 3335, 2770]
    meshes = get_meshes()
    found = []
    [found.append(m) for m in meshes if re.search(regex, m)]

    if not found:
        return

    # Meshes I'm searching for can by messy named,
    # so the only way to find them is to compare by vertices quantity.
    if not vtxs:
        return found

    meshes_filtered = [m for m in found for vtx in vtxs if cmds.polyEvaluate(m, v=True) == vtx]
    if meshes_filtered:
        return meshes_filtered


def get_mesh_diffuse_name(mesh):
    '''Returns diffuse image name, that is connected to mesh material.
        Diffuse is guessed with a help of prefixes of non-diffuse textures.
        [ur'_refl\b', ur'_nm\b', ur'_bump\b'].

    Args:
        mesh (str):

    Returns:
        list: diffuse image names
    '''
    shape = cmds.listRelatives(mesh, c=True)[0]
    sgs = list(set(cmds.ls(cmds.listConnections(shape), type='shadingEngine')))
    nodes = cmds.listHistory(sgs)
    textures = list(set(cmds.ls(nodes, type='file')))
    diffuse_names = []
    not_diffuse = [ur'_refl\b', ur'_nm\b', ur'_bump\b']

    for texture in textures:
        texture_file = cmds.getAttr("%s.fileTextureName" % texture)
        texture_file_name = os.path.basename(texture_file).split('.')[0]
        if [True for d in not_diffuse if re.search(d, texture_file_name)]:
            continue
        else:
            diffuse_names.append(texture_file_name)

    return list(set(diffuse_names))


def get_most_used_texture_and_heads():
    '''Searches for all polygon meshes in scene that matches regex expression
        and optional - vertex count.
        Preset vertex count fits two main types of head meshes, male, female,
        and male neck cut.


    Args:
        regex (str): regular expression
        vtxs_check (boolean, optional): condition, if to check the vertex count on
            top of the regular expression
        vtxs (list, optional): vertices count (int)

    Returns:
        list: polygon meshes that match search parameters

    Examples:
        >>> find_identical_meshes('(_head|head_)')
    '''
    # texture:number of used times
    head_match = {}
    # texture:head using it
    head_t_m_data = {}
    # texture: sll the heads using it
    head_used_data = {}

    for head in find_identical_meshes('(_head|head_)'):
        if not get_mesh_diffuse_name(head):
            continue
        head_t = get_mesh_diffuse_name(head)[0]
        if head_t not in head_t_m_data.keys():
            head_used_data[head_t] = [head]
        else:
            heads = head_used_data[head_t]
            heads.append(head)
            head_used_data[head_t] = heads

        if head_t not in head_match.keys():
            head_match[head_t] = 1
        else:
            head_match[head_t] += 1

        head_t_m_data[head_t] = head

    max_t = max(head_match.values())
    most_texture = [h for h in head_match.keys() if head_match[h] == max_t][0]
    head_of_most_texture = head_t_m_data[most_texture]
    another_heads = head_used_data[most_texture]
    return head_of_most_texture, another_heads


def get_shading_group(mesh):
    '''Gets shading group connected to a given mesh

    Args:
        mesh (str):

    Returns:
        list:
    '''
    shape = cmds.listRelatives(mesh, c=True)[0]
    return list(set(cmds.ls(cmds.listConnections(shape), type='shadingEngine')))


def split_mesh_on_parts(mesh):
    '''Divides polygonal mesh on the shader assigned zones.

    Args:
        mesh (str):

    Returns:
        dict: shading group name as key, list of mesh faces of mesh shape as value
    '''
    sgs = get_shading_group(mesh)
    if not sgs:
        return
    # Not sure about this. better leave for n
    sgs = list(set(sgs))
    mesh_data = {}

    for sg in sgs:
        cmds.hyperShade(objects=sg)
        selection = cmds.ls(sl=True)
        # filter selection to a given mesh only
        selection = [s for s in selection if mesh in s]
        mesh_data[sg] = selection

    return mesh_data


def add_head_sg(sg, template, count=''):
    '''Creates duplicate of a given shading group
        with a template name

    Args:
        sg (str): shading group name
        template (str): new name of a duplicated shading group
        count (str, optional): vertices count (int)

    Returns:
        list: updated shader name
    '''
    # Check base on the only one shader name.
    if not count or count <= 1:
        shader = template + '_M'
    else:
        shader = template + '_M' + count

    if not cmds.objExists(shader):
        tmp_head_name = 'tmp_head_M'
        cmds.duplicate(sg, rr=True, un=True, name=tmp_head_name)
        rename_sg_nodes(template, sg=tmp_head_name, count=count)
    return shader


def fix_sg_names():
    '''Iterates through all texture file nodes in scene.
        Renames all nodes connected to texture: shaders, shading
        groups, 2d textures, files

    '''
    mel.eval('hyperShadePanelMenuCommand("hyperShadePanel1", "deleteUnusedNodes");')
    # Disables because T drive is not set up everywhere
    # fix_texture_files_path()
    not_diffuse_prs = [ur'_refl\b', ur'_bump\b', ur'_nm\b']
    texture_data = {}
    counted_used = {}

    # Gets {texture name:file texture name} to prevent from
    # using the same texture twise, because function rename_sg_nodes()
    # goes up to sg and then lists all textures connected to sg.
    for texture in sorted(cmds.ls(type='file')):
        texture_file = cmds.getAttr("%s.fileTextureName" % texture)
        texture_file_name = os.path.basename(texture_file).split('.')[0]
        texture_file_name = re.sub(r"[^a-zA-Z0-9_.:]", '_', texture_file_name)
        texture_data[texture] = texture_file_name

    for texture, texture_file_name in texture_data.iteritems():
        if not cmds.objExists(texture):
            continue

        if [True for regex in not_diffuse_prs if re.search(regex, texture_file_name)]:
            continue

        # Decided to add counter like this: material, material_2, material_3.
        if texture_file_name in counted_used.keys():
            counter = counted_used[texture_file_name] + 1
            counted_used[texture_file_name] = counter
            counter_pr = '_' + str(counter)
        else:
            counted_used[texture_file_name] = 1
            counter_pr = ''
        rename_sg_nodes(texture_file_name, texture=texture, count=counter_pr)


def fix_face_names():
    '''Renames shading group and connected nodes of head meshes that uses
        the same image texture (diffuse). Template 'face' is used as name mask.
        If there are other head meshes with different textures most common
        texture on head is used. Updated materials applied to head meshes.
        Old named shading group is deleted
    '''
    heads_identical = []
    head, heads_identical = get_most_used_texture_and_heads()
    # head_sg = get_shading_group(head)[0]
    head_template_name = 'face'

    if not heads_identical:
        return

    for head_identical in heads_identical:
        head_parts = split_mesh_on_parts(head_identical)
        counter = 1

        for sg, part in head_parts.iteritems():
            if counter == 1:
                cmds.select(part)
                cmds.hyperShade(assign=add_head_sg(sg, head_template_name, count=''))
            else:
                cmds.select(part)
                cmds.hyperShade(assign=add_head_sg(sg, head_template_name, count='_' + str(counter)))
            counter += 1

    mel.eval('hyperShadePanelMenuCommand("hyperShadePanel1", "deleteUnusedNodes");')


def fix_materials(*args):
    '''General function that organizes scene shading groups
        in two passes. This one is the second one,
        that renames shading groups - fix_sg_names().
    '''
    sg_fixed = False
    try:
        fix_sg_names()
        print '# Fixed shading nodes names.'
        sg_fixed = True
    except Exception as e:
        print '# %s. Error while fixing scene shading group names.' % e

    '''This one is the second one, that renames shading groups
        connected to head meshes - fix_face_names().
    '''
    if sg_fixed:
        try:
            fix_face_names()
            print '# Fixed face material names.'
        except Exception as e:
            print '# %s. Error while fixing face names.' % e
    else:
        print '# Skipping face names fix.'


def enable_checkboxes(*args):
    '''General function that organizes scene shading groups.
        Enables all checkboxes in the menu - Show
    '''
    for model_p in ['modelPanel1', 'modelPanel2', 'modelPanel3', 'modelPanel4']:
        try:
            cmds.modelEditor(model_p,
                             allObjects=True,
                             grid=True,
                             hud=True,
                             sel=True,
                             manipulators=True,
                             edit=True)
        except:
            pass


# gui
cmds.window('Fix Materials', width=250)
cmds.columnLayout(adjustableColumn=True)
cmds.button(label='Fix',
            command=fix_materials,
            ann='Fix all materials in scene including heads')
cmds.button(label='Enable Show Checkboxes',
            command=enable_checkboxes,
            ann='Turns on all checkboxes in show menu')
cmds.showWindow()


```
