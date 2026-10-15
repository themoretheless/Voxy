# Facial rendering source scan: hpretorius/makehuman

Repository: https://github.com/hpretorius/makehuman

Pinned source revision: 8c767adbc4eec3dbccf837d5ce6e980e090c7a32

Discovery topic: MakeHuman

Description: avatar creator

Reported license: NOASSERTION

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE.md", "blendertools/makeclothes/license.txt", "blendertools/maketarget/license.txt", "blendertools/makewalk/license.txt", "makehuman/license.txt", "maketarget-standalone/license.txt"]

Topic paths: {"eye": ["blendertools/maketarget/data/eye.json", "blendertools/makewalk/source_rigs/eyes.src", "blendertools/makewalk/source_rigs/eyes_tpose.json", "makehuman/data/eyebrows/clear.thumb", "makehuman/data/eyes/clear.thumb", "makehuman/data/eyes/high-poly/high-poly.mhclo", "makehuman/data/eyes/high-poly/high-poly.obj", "makehuman/data/eyes/high-poly/high-poly.thumb", "makehuman/data/eyes/low-poly/low-poly.mhclo", "makehuman/data/eyes/low-poly/low-poly.obj", "makehuman/data/eyes/low-poly/low-poly.thumb", "makehuman/data/eyes/materials/brown.mhmat", "makehuman/data/eyes/materials/brown.thumb", "makehuman/data/eyes/materials/brown_eye.png", "makehuman/data/litspheres/skinmat_eye.png", "makehuman/data/targets/asym/asym-eye-1-l.target", "makehuman/data/targets/asym/asym-eye-1-r.target", "makehuman/data/targets/asym/asym-eye-2-l.target", "makehuman/data/targets/asym/asym-eye-2-r.target", "makehuman/data/targets/asym/asym-eye-3-l.target", "makehuman/data/targets/asym/asym-eye-3-r.target", "makehuman/data/targets/asym/asym-eye-4-l.target", "makehuman/data/targets/asym/asym-eye-4-r.target", "makehuman/data/targets/asym/asym-eye-5-l.target", "makehuman/data/targets/asym/asym-eye-5-r.target", "makehuman/data/targets/asym/asym-eye-6-l.target", "makehuman/data/targets/asym/asym-eye-6-r.target", "makehuman/data/targets/asym/asym-eye-7-l.target", "makehuman/data/targets/asym/asym-eye-7-r.target", "makehuman/data/targets/asym/asym-eye-8-l.target"], "skin": ["buildscripts/ppa/packages/makehuman/debian/makehuman-skins.install", "makehuman/apps/autoskinblender.py", "makehuman/data/litspheres/skinmat_african.png", "makehuman/data/litspheres/skinmat_asian.png", "makehuman/data/litspheres/skinmat_caucasian.png", "makehuman/data/litspheres/skinmat_eye.png", "makehuman/data/povray/skin.inc", "makehuman/data/shaders/glsl/skin_fragment_shader.txt", "makehuman/data/shaders/glsl/skin_vertex_shader.txt", "makehuman/data/skins/default.mhmat", "makehuman/data/skins/default.thumb", "makehuman/data/skins/notfound.thumb"], "sss": ["makehuman/data/povray/sss.inc"], "jaw": ["makehuman/data/targets/asym/asym-jaw-1-l.target", "makehuman/data/targets/asym/asym-jaw-1-r.target", "makehuman/data/targets/asym/asym-jaw-2-l.target", "makehuman/data/targets/asym/asym-jaw-2-r.target", "makehuman/data/targets/asym/asym-jaw-3-l.target", "makehuman/data/targets/asym/asym-jaw-3-r.target", "makehuman/data/targets/chin/chin-jaw-drop-decr.target", "makehuman/data/targets/chin/chin-jaw-drop-incr.target", "makehuman/data/targets/chin/images/chin-jaw-drop-decr-incr.png"], "mouth": ["makehuman/data/targets/asym/asym-mouth-1-l.target", "makehuman/data/targets/asym/asym-mouth-1-r.target", "makehuman/data/targets/asym/asym-mouth-2-l.target", "makehuman/data/targets/asym/asym-mouth-2-r.target", "makehuman/data/targets/expression/units/african/mouth-compression.target", "makehuman/data/targets/expression/units/african/mouth-corner-puller.target", "makehuman/data/targets/expression/units/african/mouth-depression-retraction.target", "makehuman/data/targets/expression/units/african/mouth-depression.target", "makehuman/data/targets/expression/units/african/mouth-elevation.target", "makehuman/data/targets/expression/units/african/mouth-eversion.target", "makehuman/data/targets/expression/units/african/mouth-open.target", "makehuman/data/targets/expression/units/african/mouth-parling.target", "makehuman/data/targets/expression/units/african/mouth-part-later.target", "makehuman/data/targets/expression/units/african/mouth-protusion.target", "makehuman/data/targets/expression/units/african/mouth-pursing.target", "makehuman/data/targets/expression/units/african/mouth-retraction.target", "makehuman/data/targets/expression/units/african/mouth-upward-retraction.target", "makehuman/data/targets/expression/units/asian/mouth-compression.target", "makehuman/data/targets/expression/units/asian/mouth-corner-puller.target", "makehuman/data/targets/expression/units/asian/mouth-depression-retraction.target", "makehuman/data/targets/expression/units/asian/mouth-depression.target", "makehuman/data/targets/expression/units/asian/mouth-elevation.target", "makehuman/data/targets/expression/units/asian/mouth-eversion.target", "makehuman/data/targets/expression/units/asian/mouth-open.target", "makehuman/data/targets/expression/units/asian/mouth-parling.target", "makehuman/data/targets/expression/units/asian/mouth-part-later.target", "makehuman/data/targets/expression/units/asian/mouth-protusion.target", "makehuman/data/targets/expression/units/asian/mouth-pursing.target", "makehuman/data/targets/expression/units/asian/mouth-retraction.target", "makehuman/data/targets/expression/units/asian/mouth-upward-retraction.target"], "teeth": ["makehuman/plugins/3_libraries_teeth.py"], "tongue": ["makehuman/plugins/3_libraries_tongue.py"], "deform": ["makehuman/plugins/9_export_fbx/fbx_deformer.py"], "shader": ["makehuman/data/shaders/glsl/litsphere_fragment_shader.txt", "makehuman/data/shaders/glsl/litsphere_vertex_shader.txt", "makehuman/data/shaders/glsl/normalmap_fragment_shader.txt", "makehuman/data/shaders/glsl/normalmap_vertex_shader.txt", "makehuman/data/shaders/glsl/phong_fragment_shader.txt", "makehuman/data/shaders/glsl/phong_vertex_shader.txt", "makehuman/data/shaders/glsl/skin_fragment_shader.txt", "makehuman/data/shaders/glsl/skin_vertex_shader.txt", "makehuman/data/shaders/glsl/toon_fragment_shader.txt", "makehuman/data/shaders/glsl/toon_vertex_shader.txt", "makehuman/data/shaders/glsl/xray_fragment_shader.txt", "makehuman/data/shaders/glsl/xray_vertex_shader.txt", "makehuman/lib/shader.py"], "rig": ["blendertools/makewalk/source_rigs/accad.src", "blendertools/makewalk/source_rigs/accad_tpose.json", "blendertools/makewalk/source_rigs/cmu_tpose.json", "blendertools/makewalk/source_rigs/eyes.src", "blendertools/makewalk/source_rigs/eyes_tpose.json", "blendertools/makewalk/source_rigs/max.src", "blendertools/makewalk/source_rigs/mb.src", "blendertools/makewalk/source_rigs/vicon.src", "blendertools/makewalk/target_rigs/default.trg", "blendertools/makewalk/target_rigs/default_tpose.json", "blendertools/makewalk/target_rigs/makehuman_tpose.json", "blendertools/makewalk/target_rigs/mb.trg", "blendertools/makewalk/target_rigs/mh-alpha7.trg", "blendertools/makewalk/target_rigs/mhx.trg", "blendertools/makewalk/target_rigs/rigify.trg", "buildscripts/deb/debian/copyright", "buildscripts/ppa/packages/makehuman-dev/debian/copyright", "buildscripts/ppa/packages/makehuman/debian/copyright", "makehuman/data/icons/right.png", "makehuman/data/icons/right.svg", "makehuman/data/rigs/default.mhskel", "makehuman/data/rigs/default.thumb", "makehuman/data/rigs/default_weights.mhw", "makehuman/data/targets/expression/units/african/eye-right-closure.target", "makehuman/data/targets/expression/units/african/eye-right-opened-up.target", "makehuman/data/targets/expression/units/african/eye-right-slit.target", "makehuman/data/targets/expression/units/african/eyebrows-right-down.target", "makehuman/data/targets/expression/units/african/eyebrows-right-extern-up.target", "makehuman/data/targets/expression/units/african/eyebrows-right-inner-up.target", "makehuman/data/targets/expression/units/african/eyebrows-right-up.target"], "face": ["makehuman/data/poseunits/face-poseunits.bvh", "makehuman/data/poseunits/face-poseunits.json", "makehuman/data/povray/makehuman_facegroup_documentation.pov"], "lip": ["makehuman/data/targets/mouth/images/mouth-lowerlip-ext-down-up.png", "makehuman/data/targets/mouth/images/mouth-lowerlip-height-decr-incr.png", "makehuman/data/targets/mouth/images/mouth-lowerlip-middle-down-up.png", "makehuman/data/targets/mouth/images/mouth-lowerlip-volume-decr-incr.png", "makehuman/data/targets/mouth/images/mouth-lowerlip-width-decr-incr.png", "makehuman/data/targets/mouth/images/mouth-upperlip-ext-down-up.png", "makehuman/data/targets/mouth/images/mouth-upperlip-height-decr-incr.png", "makehuman/data/targets/mouth/images/mouth-upperlip-middle-down-up.png", "makehuman/data/targets/mouth/images/mouth-upperlip-volume-decr-incr.png", "makehuman/data/targets/mouth/images/mouth-upperlip-width-decr-incr.png", "makehuman/data/targets/mouth/mouth-lowerlip-ext-down.target", "makehuman/data/targets/mouth/mouth-lowerlip-ext-up.target", "makehuman/data/targets/mouth/mouth-lowerlip-height-decr.target", "makehuman/data/targets/mouth/mouth-lowerlip-height-incr.target", "makehuman/data/targets/mouth/mouth-lowerlip-middle-down.target", "makehuman/data/targets/mouth/mouth-lowerlip-middle-up.target", "makehuman/data/targets/mouth/mouth-lowerlip-volume-decr.target", "makehuman/data/targets/mouth/mouth-lowerlip-volume-incr.target", "makehuman/data/targets/mouth/mouth-lowerlip-width-decr.target", "makehuman/data/targets/mouth/mouth-lowerlip-width-incr.target", "makehuman/data/targets/mouth/mouth-upperlip-ext-down.target", "makehuman/data/targets/mouth/mouth-upperlip-ext-up.target", "makehuman/data/targets/mouth/mouth-upperlip-height-decr.target", "makehuman/data/targets/mouth/mouth-upperlip-height-incr.target", "makehuman/data/targets/mouth/mouth-upperlip-middle-down.target", "makehuman/data/targets/mouth/mouth-upperlip-middle-up.target", "makehuman/data/targets/mouth/mouth-upperlip-volume-decr.target", "makehuman/data/targets/mouth/mouth-upperlip-volume-incr.target", "makehuman/data/targets/mouth/mouth-upperlip-width-decr.target", "makehuman/data/targets/mouth/mouth-upperlip-width-incr.target"]}

## Source file: makehuman/lib/shader.py

{
  "path": "makehuman/lib/shader.py",
  "blob_sha": "7bbaa2970cbc3ef287b3d3d44d14475781a54a88",
  "sha256": "7bcd436cfcd749648f9a3d1ecc3c8f0f45b017f6a88b2a0d86d9b6d832e9a8df",
  "lines": 548,
  "bytes": 24591,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 81,
    "rig": 1
  },
  "url": "https://github.com/hpretorius/makehuman/blob/8c767adbc4eec3dbccf837d5ce6e980e090c7a32/makehuman/lib/shader.py",
  "function_signature_candidates": [
    {
      "line": 50,
      "text": "def __init__(self, pgm, index, name, pytype, dims):"
    },
    {
      "line": 62,
      "text": "def __call__(self, index, values):"
    },
    {
      "line": 65,
      "text": "def update(self):"
    },
    {
      "line": 115,
      "text": "def check(cls, type):"
    },
    {
      "line": 121,
      "text": "def __init__(self, pgm, index, name, type):"
    },
    {
      "line": 129,
      "text": "def set(self, data):"
    },
    {
      "line": 139,
      "text": "def update(self):"
    },
    {
      "line": 229,
      "text": "def check(cls, type):"
    },
    {
      "line": 235,
      "text": "def __init__(self, pgm, index, name, type):"
    },
    {
      "line": 240,
      "text": "def set(self, data):"
    },
    {
      "line": 259,
      "text": "def reset(cls):"
    },
    {
      "line": 271,
      "text": "def supported(cls):"
    },
    {
      "line": 277,
      "text": "def glslVersionStr(cls):"
    },
    {
      "line": 283,
      "text": "def glslVersion(cls):"
    },
    {
      "line": 303,
      "text": "def __init__(self, path, defines = []):"
    },
    {
      "line": 321,
      "text": "def __del__(self):"
    },
    {
      "line": 328,
      "text": "def createShader(file, type, defines = [], defineables = None):"
    },
    {
      "line": 354,
      "text": "def _getDefineables(sourceStr):"
    },
    {
      "line": 373,
      "text": "def _splitVersionDeclaration(sourceStr):"
    },
    {
      "line": 398,
      "text": "def delete(self):"
    },
    {
      "line": 409,
      "text": "def initShader(self):"
    },
    {
      "line": 448,
      "text": "def getUniforms(self):"
    },
    {
      "line": 468,
      "text": "def updateUniforms(self):"
    },
    {
      "line": 472,
      "text": "def setUniforms(self, params):"
    },
    {
      "line": 489,
      "text": "def requiresVertexTangent(self):"
    },
    {
      "line": 494,
      "text": "def getShader(path, defines=[], cache=None):"
    },
    {
      "line": 538,
      "text": "def reloadShaders():"
    }
  ]
}

```text

#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""
**Project Name:**      MakeHuman

**Product Home Page:** http://www.makehuman.org/

**Code Home Page:**    https://bitbucket.org/MakeHuman/makehuman/

**Authors:**           Glynn Clements

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

TODO
"""

import os.path
import numpy as np
from OpenGL.GL import *
from OpenGL.GL.ARB.texture_multisample import *
from OpenGL import GL
import texture
import log
from core import G
import io

class Uniform(object):
    def __init__(self, pgm, index, name, pytype, dims):
        if type(name) is bytes:
            self.name = name.decode('utf-8')
        else:
            self.name = name
        self.index = index
        self.pytype = pytype
        self.dims = dims
        self.values = None
        self.pgm = pgm
        self.location = glGetUniformLocation(self.pgm, name)

    def __call__(self, index, values):
        raise NotImplementedError

    def update(self):
        pass

class VectorUniform(Uniform):
    uniformTypes = {
        GL_FLOAT:               ((1,),  np.float32,     float,  glUniform1fv,           glGetUniformfv),
        GL_FLOAT_VEC2:          ((2,),  np.float32,     float,  glUniform2fv,           glGetUniformfv),
        GL_FLOAT_VEC3:          ((3,),  np.float32,     float,  glUniform3fv,           glGetUniformfv),
        GL_FLOAT_VEC4:          ((4,),  np.float32,     float,  glUniform4fv,           glGetUniformfv),
        GL_INT:                 ((1,),  np.int32,       int,    glUniform1iv,           glGetUniformiv),
        GL_INT_VEC2:            ((2,),  np.int32,       int,    glUniform2iv,           glGetUniformiv),
        GL_INT_VEC3:            ((3,),  np.int32,       int,    glUniform3iv,           glGetUniformiv),
        GL_INT_VEC4:            ((4,),  np.int32,       int,    glUniform4iv,           glGetUniformiv),
        GL_UNSIGNED_INT:        ((1,),  np.uint32,      int,    glUniform1uiv,          glGetUniformuiv),
        GL_UNSIGNED_INT_VEC2:   ((2,),  np.uint32,      int,    glUniform2uiv,          glGetUniformuiv),
        GL_UNSIGNED_INT_VEC3:   ((3,),  np.uint32,      int,    glUniform3uiv,          glGetUniformuiv),
        GL_UNSIGNED_INT_VEC4:   ((4,),  np.uint32,      int,    glUniform4uiv,          glGetUniformuiv),
        GL_BOOL:                ((1,),  np.int32,       bool,   glUniform1iv,           glGetUniformiv),
        GL_BOOL_VEC2:           ((2,),  np.int32,       bool,   glUniform2iv,           glGetUniformiv),
        GL_BOOL_VEC3:           ((3,),  np.int32,       bool,   glUniform3iv,           glGetUniformiv),
        GL_BOOL_VEC4:           ((4,),  np.int32,       bool,   glUniform4iv,           glGetUniformiv),
        GL_FLOAT_MAT2:          ((2,2), np.float32,     float,  glUniformMatrix2fv,     glGetUniformfv),
        GL_FLOAT_MAT2x3:        ((2,3), np.float32,     float,  glUniformMatrix2x3fv,   glGetUniformfv),
        GL_FLOAT_MAT2x4:        ((2,4), np.float32,     float,  glUniformMatrix2x4fv,   glGetUniformfv),
        GL_FLOAT_MAT3x2:        ((3,2), np.float32,     float,  glUniformMatrix3x2fv,   glGetUniformfv),
        GL_FLOAT_MAT3:          ((3,3), np.float32,     float,  glUniformMatrix3fv,     glGetUniformfv),
        GL_FLOAT_MAT3x4:        ((3,4), np.float32,     float,  glUniformMatrix3x4fv,   glGetUniformfv),
        GL_FLOAT_MAT4x2:        ((4,2), np.float32,     float,  glUniformMatrix4x2fv,   glGetUniformfv),
        GL_FLOAT_MAT4x3:        ((4,3), np.float32,     float,  glUniformMatrix4x3fv,   glGetUniformfv),
        GL_FLOAT_MAT4:          ((4,4), np.float32,     float,  glUniformMatrix4fv,     glGetUniformfv),
        }

    if 'glUniform1dv' in globals():
        uniformTypes2 = {
            GL_DOUBLE:              ((1,),  np.float64,     float,  glUniform1dv,           glGetUniformdv),
            GL_DOUBLE_VEC2:         ((2,),  np.float64,     float,  glUniform2dv,           glGetUniformdv),
            GL_DOUBLE_VEC3:         ((3,),  np.float64,     float,  glUniform3dv,           glGetUniformdv),
            GL_DOUBLE_VEC4:         ((4,),  np.float64,     float,  glUniform4dv,           glGetUniformdv),
            GL_DOUBLE_MAT2:         ((2,2), np.float64,     float,  glUniformMatrix2dv,     glGetUniformdv),
            GL_DOUBLE_MAT2x3:       ((2,3), np.float64,     float,  glUniformMatrix2x3dv,   glGetUniformdv),
            GL_DOUBLE_MAT2x4:       ((2,4), np.float64,     float,  glUniformMatrix2x4dv,   glGetUniformdv),
            GL_DOUBLE_MAT3x2:       ((3,2), np.float64,     float,  glUniformMatrix3x2dv,   glGetUniformdv),
            GL_DOUBLE_MAT3:         ((3,3), np.float64,     float,  glUniformMatrix3dv,     glGetUniformdv),
            GL_DOUBLE_MAT3x4:       ((3,4), np.float64,     float,  glUniformMatrix3x4dv,   glGetUniformdv),
            GL_DOUBLE_MAT4x2:       ((4,2), np.float64,     float,  glUniformMatrix4x2dv,   glGetUniformdv),
            GL_DOUBLE_MAT4x3:       ((4,3), np.float64,     float,  glUniformMatrix4x3dv,   glGetUniformdv),
            GL_DOUBLE_MAT4:         ((4,4), np.float64,     float,  glUniformMatrix4dv,     glGetUniformdv),
            }

    @classmethod
    def check(cls, type):
        if hasattr(cls, 'uniformTypes2') and cls.uniformTypes2:
            cls.uniformTypes.update(cls.uniformTypes2)
            cls.uniformTypes2.clear()
        return type in cls.uniformTypes

    def __init__(self, pgm, index, name, type):
        dims, dtype, pytype, glfunc, glquery = self.uniformTypes[type]
        super(VectorUniform, self).__init__(pgm, index, name, pytype, dims)
        self.type = type
        self.dtype = dtype
        self.glfunc = glfunc
        self.glquery = glquery

    def set(self, data):
        if data is None:
            self.set(self.values)
            return
        values = np.asarray(data, dtype=self.dtype).reshape(self.dims)
        if len(self.dims) > 1:
            self.glfunc(self.location, 1, GL_TRUE, values)
        else:
            self.glfunc(self.location, len(values) // self.dims[0], values)

    def update(self):
        values = np.zeros(self.dims, dtype=self.dtype)
        self.glquery(self.pgm, self.location, values)
        if len(self.dims) > 1:
            values = values.T
        self.values = values
        log.debug('VectorUniform(%s) = %s', self.name, self.values)
        return self.values

class SamplerUniform(Uniform):

    currentSampler = 0

    textureTargets = {
        GL_SAMPLER_1D:                                  GL_TEXTURE_1D,
        GL_SAMPLER_2D:                                  GL_TEXTURE_2D,
        GL_SAMPLER_3D:                                  GL_TEXTURE_3D,
        GL_SAMPLER_CUBE:                                GL_TEXTURE_CUBE_MAP,
        GL_SAMPLER_1D_SHADOW:                           GL_TEXTURE_1D,
        GL_SAMPLER_2D_SHADOW:                           GL_TEXTURE_2D,
        GL_SAMPLER_1D_ARRAY:                            GL_TEXTURE_1D_ARRAY,
        GL_SAMPLER_2D_ARRAY:                            GL_TEXTURE_2D_ARRAY,
        GL_SAMPLER_1D_ARRAY_SHADOW:                     GL_TEXTURE_1D_ARRAY,
        GL_SAMPLER_2D_ARRAY_SHADOW:                     GL_TEXTURE_2D_ARRAY,
        GL_SAMPLER_2D_MULTISAMPLE:                      GL_TEXTURE_2D_MULTISAMPLE,
        GL_SAMPLER_2D_MULTISAMPLE_ARRAY:                GL_TEXTURE_2D_MULTISAMPLE_ARRAY,
        GL_SAMPLER_CUBE_SHADOW:                         GL_TEXTURE_CUBE_MAP,
        GL_SAMPLER_BUFFER:                              GL_TEXTURE_BUFFER,
        GL_SAMPLER_2D_RECT:                             GL_TEXTURE_RECTANGLE,
        GL_SAMPLER_2D_RECT_SHADOW:                      GL_TEXTURE_RECTANGLE,
        GL_INT_SAMPLER_1D:                              GL_TEXTURE_1D,
        GL_INT_SAMPLER_2D:                              GL_TEXTURE_2D,
        GL_INT_SAMPLER_3D:                              GL_TEXTURE_3D,
        GL_INT_SAMPLER_CUBE:                            GL_TEXTURE_CUBE_MAP,
        GL_INT_SAMPLER_1D_ARRAY:                        GL_TEXTURE_1D_ARRAY,
        GL_INT_SAMPLER_2D_ARRAY:                        GL_TEXTURE_2D_ARRAY,
        GL_INT_SAMPLER_2D_MULTISAMPLE:                  GL_TEXTURE_2D_MULTISAMPLE,
        GL_INT_SAMPLER_2D_MULTISAMPLE_ARRAY:            GL_TEXTURE_2D_MULTISAMPLE_ARRAY,
        GL_INT_SAMPLER_BUFFER:                          GL_TEXTURE_BUFFER,
        GL_INT_SAMPLER_2D_RECT:                         GL_TEXTURE_RECTANGLE,
        GL_UNSIGNED_INT_SAMPLER_1D:                     GL_TEXTURE_1D,
        GL_UNSIGNED_INT_SAMPLER_2D:                     GL_TEXTURE_2D,
        GL_UNSIGNED_INT_SAMPLER_3D:                     GL_TEXTURE_3D,
        GL_UNSIGNED_INT_SAMPLER_CUBE:                   GL_TEXTURE_CUBE_MAP,
        GL_UNSIGNED_INT_SAMPLER_1D_ARRAY:               GL_TEXTURE_1D_ARRAY,
        GL_UNSIGNED_INT_SAMPLER_2D_ARRAY:               GL_TEXTURE_2D_ARRAY,
        GL_UNSIGNED_INT_SAMPLER_2D_MULTISAMPLE:         GL_TEXTURE_2D_MULTISAMPLE,
        GL_UNSIGNED_INT_SAMPLER_2D_MULTISAMPLE_ARRAY:   GL_TEXTURE_2D_MULTISAMPLE_ARRAY,
        GL_UNSIGNED_INT_SAMPLER_BUFFER:                 GL_TEXTURE_BUFFER,
        GL_UNSIGNED_INT_SAMPLER_2D_RECT:                GL_TEXTURE_RECTANGLE,
        }

    if 'GL_IMAGE_1D' in globals():
        try:
            textureTargets2 = {
                GL_IMAGE_1D:                                    GL_TEXTURE_1D,
                GL_IMAGE_2D:                                    GL_TEXTURE_2D,
                GL_IMAGE_3D:                                    GL_TEXTURE_3D,
                GL_IMAGE_2D_RECT:                               GL_TEXTURE_RECTANGLE,
                GL_IMAGE_CUBE:                                  GL_TEXTURE_CUBE_MAP,
                GL_IMAGE_BUFFER:                                GL_TEXTURE_BUFFER,
                GL_IMAGE_1D_ARRAY:                              GL_TEXTURE_1D_ARRAY,
                GL_IMAGE_2D_ARRAY:                              GL_TEXTURE_2D_ARRAY,
                GL_IMAGE_2D_MULTISAMPLE:                        GL_TEXTURE_2D_MULTISAMPLE,
                GL_IMAGE_2D_MULTISAMPLE_ARRAY:                  GL_TEXTURE_2D_MULTISAMPLE_ARRAY,
                GL_INT_IMAGE_1D:                                GL_TEXTURE_1D,
                GL_INT_IMAGE_2D:                                GL_TEXTURE_2D,
                GL_INT_IMAGE_3D:                                GL_TEXTURE_3D,
                GL_INT_IMAGE_2D_RECT:                           GL_TEXTURE_RECTANGLE,
                GL_INT_IMAGE_CUBE:                              GL_TEXTURE_CUBE_MAP,
                GL_INT_IMAGE_BUFFER:                            GL_TEXTURE_BUFFER,
                GL_INT_IMAGE_1D_ARRAY:                          GL_TEXTURE_1D_ARRAY,
                GL_INT_IMAGE_2D_ARRAY:                          GL_TEXTURE_2D_ARRAY,
                GL_INT_IMAGE_2D_MULTISAMPLE:                    GL_TEXTURE_2D_MULTISAMPLE,
                GL_INT_IMAGE_2D_MULTISAMPLE_ARRAY:              GL_TEXTURE_2D_MULTISAMPLE_ARRAY,
                GL_UNSIGNED_INT_IMAGE_1D:                       GL_TEXTURE_1D,
                GL_UNSIGNED_INT_IMAGE_2D:                       GL_TEXTURE_2D,
                GL_UNSIGNED_INT_IMAGE_3D:                       GL_TEXTURE_3D,
                GL_UNSIGNED_INT_IMAGE_2D_RECT:                  GL_TEXTURE_RECTANGLE,
                GL_UNSIGNED_INT_IMAGE_CUBE:                     GL_TEXTURE_CUBE_MAP,
                GL_UNSIGNED_INT_IMAGE_BUFFER:                   GL_TEXTURE_BUFFER,
                GL_UNSIGNED_INT_IMAGE_1D_ARRAY:                 GL_TEXTURE_1D_ARRAY,
                GL_UNSIGNED_INT_IMAGE_2D_ARRAY:                 GL_TEXTURE_2D_ARRAY,
                GL_UNSIGNED_INT_IMAGE_2D_MULTISAMPLE:           GL_TEXTURE_2D_MULTISAMPLE,
                GL_UNSIGNED_INT_IMAGE_2D_MULTISAMPLE_ARRAY:     GL_TEXTURE_2D_MULTISAMPLE_ARRAY,
                }
        except:
            pass

    @classmethod
    def check(cls, type):
        if hasattr(cls, 'textureTargets2') and cls.textureTargets2:
            cls.textureTargets.update(cls.textureTargets2)
            cls.textureTargets2.clear()
        return type in cls.textureTargets

    def __init__(self, pgm, index, name, type):
        target = self.textureTargets[type]
        super(SamplerUniform, self).__init__(pgm, index, name, str, (1,))
        self.target = target

    def set(self, data):
        cls = type(self)
        glActiveTexture(GL_TEXTURE0 + cls.currentSampler)
        if data is not None:
            tex = texture.getTexture(data)
        else:
            tex = None
        if tex not in (False, None):
            glBindTexture(self.target, tex.textureId)
        else:
            tex = texture.getTexture(texture.NOTFOUND_TEXTURE)
            if tex in (None, False):
                glBindTexture(self.target, 0)
            else:
                glBindTexture(self.target, tex.textureId)
        glUniform1i(self.location, cls.currentSampler)
        cls.currentSampler += 1

    @classmethod
    def reset(cls):
        cls.currentSampler = 0

class Shader(object):
    _supported = None
    if G.args.get('noshaders', False):
        _supported = False

    _glsl_version_str = None
    _glsl_version = None

    @classmethod
    def supported(cls):
        if cls._supported is None:
            cls._supported = bool(glCreateProgram)
        return cls._supported

    @classmethod
    def glslVersionStr(cls):
        cls.glslVersion()
        cls._glsl_version_str = OpenGL.GL.glGetString(OpenGL.GL.GL_SHADING_LANGUAGE_VERSION)
        return cls._glsl_version_str

    @classmethod
    def glslVersion(cls):
        if cls._glsl_version is None:
            if not cls.supported():
                cls._glsl_version = (0,0)
                cls._glsl_version_str = "NOT SUPPORTED!"
                return cls._glsl_version

            cls._glsl_version_str = OpenGL.GL.glGetString(OpenGL.GL.GL_SHADING_LANGUAGE_VERSION)
            # true string rather than byte string
            cls._glsl_version_str = str(cls._glsl_version_str, encoding='utf-8')
            log.debug(cls._glsl_version_str)
            if cls._glsl_version_str:
                import re
                glsl_version = re.search('[0-9]+\.[0-9]+', cls._glsl_version_str).group(0)
                glsl_v_major, glsl_v_minor = glsl_version.split('.')
            else:
                glsl_v_major, glsl_v_minor = (0, 0)
            cls._glsl_version = (int(glsl_v_major), int(glsl_v_minor))
        return cls._glsl_version

    def __init__(self, path, defines = []):
        if not self.supported():
            raise RuntimeError("No shader support detected")

        super(Shader, self).__init__()

        self.path = path

        self.vertexId = None
        self.fragmentId = None
        self.shaderId = None
        self.modified = None
        self.uniforms = None
        self.defines = defines
        self.vertexTangentAttrId = None

        self.initShader()

    def __del__(self):
        try:
            self.delete()
        except Exception:
            pass

    @staticmethod
    def createShader(file, type, defines = [], defineables = None):
        with io.open(file, 'rU') as f:
            source = f.read()
        if "#version" not in source:
            log.warning("The shader source in %s does not contain an explicit GLSL version declaration. This could cause problems with some compilers.", file)

        if defineables != None:
            for d in Shader._getDefineables(source):
                if d not in defineables:
                    defineables.append(d)
        if defines:
            # Add #define instructions for shader preprocessor to enable extra
            # shader features at compile time
            firstComments, code = Shader._splitVersionDeclaration(source)
            defineLines = "\n".join([ "#define " + define for define in defines])
            source = "\n".join([firstComments, defineLines, code])
        shader = glCreateShader(type)
        glShaderSource(shader, source)
        glCompileShader(shader)
        if not glGetShaderiv(shader, GL_COMPILE_STATUS):
            logmsg = glGetShaderInfoLog(shader)
            log.error("Error compiling shader: %s", logmsg)
            return None
        return shader

    @staticmethod
    def _getDefineables(sourceStr):
        """
        Note: currently only supports #ifdef and #ifndef preprocessor statements
        The syntax #if defined DEFINEABLE is not supported
        """
        lines = sourceStr.split("\n")
        result = []
        for l in lines:
            l = l.strip()
            if not l:
                continue
            l = l.split()
            if len(l) < 2:
                continue
            if l[0] in ['#ifdef', '#ifndef']:
                result.append(l[1])
        return result

    @staticmethod
    def _splitVersionDeclaration(sourceStr):
        """
        Split source string in part that contains the #version declaration,
        that should occur before any instructions (only comments can preceed),
        and the rest of the shader code.
        Define statements can be inserted between the two split strings.
        This is to ensure that any #version statements remain the first
        instruction in the shader source, conform with the GLSL spec.
        If no #version statement is found, the full source will be in the second
        string, allowing #define statements to be inserted at the top of the
        source.
        NOTE: For this to work correctly, #version effectively needs to be the
        first instruction in the shader source, but this is usually enforced by
        the GLSL compiler anyway.
        Returns a tuple of two strings.
        """
        if "#version" in sourceStr:
            lines = sourceStr.split("\n")
            for lIdx,line in enumerate(lines):
                # Determine line where #version occurs
                if line.strip().startswith("#version"):
                    return "\n".join(lines[:lIdx+1]), "\n".join(lines[lIdx+1:])
        # Else don't split source
        return "", sourceStr

    def delete(self):
        if self.vertexId:
            glDeleteShader(self.vertexId)
            self.vertexId = None
        if self.fragmentId:
            glDeleteShader(self.fragmentId)
            self.fragmentId = None
        if self.shaderId:
            glDeleteProgram(self.shaderId)
            self.shaderId = None

    def initShader(self):
        vertexSource = self.path + '_vertex_shader.txt'
        geometrySource = self.path + '_geometry_shader.txt'
        fragmentSource = self.path + '_fragment_shader.txt'

        self.shaderId = glCreateProgram()

        self.defineables = []

        if os.path.isfile(vertexSource):
            self.vertexId = self.createShader(vertexSource, GL_VERTEX_SHADER, self.defines, self.defineables)
            if self.vertexId == None:
                raise RuntimeError("No vertex shader program compiled, cannot set vertex shader. (%s)" % vertexSource)
            glAttachShader(self.shaderId, self.vertexId)

        if os.path.isfile(geometrySource) and 'GL_GEOMETRY_SHADER' in globals():
            self.geometryId = self.createShader(geometrySource, GL_GEOMETRY_SHADER, self.defines, self.defineables)
            if self.geometryId == None:
                raise RuntimeError("No geometry shader program compiled, cannot set geometry shader. (%s)" % geometrySource)
            glAttachShader(self.shaderId, self.geometryId)

        if os.path.isfile(fragmentSource):
            self.fragmentId = self.createShader(fragmentSource, GL_FRAGMENT_SHADER, self.defines, self.defineables)
            if self.fragmentId == None:
                raise RuntimeError("No fragment shader program compiled, cannot set fragment shader. (%s)" % fragmentSource)
            glAttachShader(self.shaderId, self.fragmentId)

        glLinkProgram(self.shaderId)
        if not glGetProgramiv(self.shaderId, GL_LINK_STATUS):
            log.error("Error linking shader: %s", glGetProgramInfoLog(self.shaderId))
            self.delete()
            return

        self.vertexTangentAttrId = glGetAttribLocation(self.shaderId, b'tangent')

        self.uniforms = None
        self.glUniforms = []
        self.updateUniforms()

    def getUniforms(self):
        if self.uniforms is None:
            parameterCount = glGetProgramiv(self.shaderId, GL_ACTIVE_UNIFORMS)
            self.uniforms = []
            for index in range(parameterCount):
                name, size, type = glGetActiveUniform(self.shaderId, index)
                if name.startswith(b'gl_'):
                    log.debug("Shader: adding built-in uniform %s", name)
                    self.glUniforms.append(name)
                    continue
                if VectorUniform.check(type):
                    uniform = VectorUniform(self.shaderId, index, name, type)
                elif SamplerUniform.check(type):
                    uniform = SamplerUniform(self.shaderId, index, name, type)
                # probably superfluous, to be deleted:
                # uniform.update()
                self.uniforms.append(uniform)

        return self.uniforms

    def updateUniforms(self):
        for uniform in self.getUniforms():
            uniform.update()

    def setUniforms(self, params):
        import glmodule
        SamplerUniform.reset()

        for uniform in self.getUniforms():
            value = params.get(uniform.name)
            uniform.set(value)

        # Disable other texture units
        for gl_tex_idx in range(GL_TEXTURE0 + SamplerUniform.currentSampler,
                                GL_TEXTURE0 + glmodule.MAX_TEXTURE_UNITS):
            glActiveTexture(gl_tex_idx)
            glBindTexture(GL_TEXTURE_2D, 0)
            glDisable(GL_TEXTURE_2D)
            glBindTexture(GL_TEXTURE_1D, 0)
            glDisable(GL_TEXTURE_1D)

    def requiresVertexTangent(self):
        return self.vertexTangentAttrId != -1

_shaderCache = {}

def getShader(path, defines=[], cache=None):
    shader = None
    cache = cache or _shaderCache

    path1 = path + '_vertex_shader.txt'
    path2 = path + '_fragment_shader.txt'
    path3 = path + '_geometry_shader.txt'
    paths = [p for p in [path1, path2, path3] if os.path.isfile(p)]
    if not paths:
        log.error('No shader file found at path %s. Shader not loaded.', path)
        cache[path] = False
        return False

    mtime = max(os.path.getmtime(p) for p in paths)

    cacheName = path
    if defines:
        # It's important that the defines are sorted alfpabetically here
        cacheName = cacheName + "@" + "|".join(defines)

    if cacheName in cache:
        shader = cache[cacheName]
        if shader is False:
            return shader

        if mtime > shader.modified:
            log.message('reloading %s', cacheName)
            try:
                shader.initShader()
                shader.modified = mtime
            except RuntimeError as _:
                log.error("Error loading shader %s", cacheName, exc_info=True)
                shader = False
    else:
        try:
            shader = Shader(path, defines)
            shader.modified = mtime
        except RuntimeError as _:
            log.error("Error loading shader %s", path, exc_info=True)
            shader = False

    cache[cacheName] = shader
    return shader

def reloadShaders():
    log.message('Reloading shaders')
    for path in _shaderCache:
        if _shaderCache[path]:
            try:
                _shaderCache[path].initShader()
            except RuntimeError as _:
                log.error("Error loading shader %s", path, exc_info=True)
                _shaderCache[path] = False
        else:
            del _shaderCache[path]


```

## Source file: makehuman/apps/autoskinblender.py

{
  "path": "makehuman/apps/autoskinblender.py",
  "blob_sha": "8e078085bc6ab5a88d8971a5653be96462f8974b",
  "sha256": "a67741700dd3ad37a82b083091bb6fd5e9725a7280b6c5739bd64d70b0e22636",
  "lines": 120,
  "bytes": 4163,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 11,
    "rig": 1
  },
  "url": "https://github.com/hpretorius/makehuman/blob/8c767adbc4eec3dbccf837d5ce6e980e090c7a32/makehuman/apps/autoskinblender.py",
  "function_signature_candidates": [
    {
      "line": 56,
      "text": "def __init__(self, human):"
    },
    {
      "line": 68,
      "text": "def checkUpdate(self):"
    },
    {
      "line": 76,
      "text": "def getEthnicState(self):"
    },
    {
      "line": 81,
      "text": "def getLitsphereTexture(self):"
    },
    {
      "line": 85,
      "text": "def getDiffuseColor(self):"
    },
    {
      "line": 89,
      "text": "def update(self):"
    }
  ]
}

```text

#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""
**Project Name:**      MakeHuman

**Product Home Page:** http://www.makehuman.org/

**Code Home Page:**    https://bitbucket.org/MakeHuman/makehuman/

**Authors:**           Jonas Hauquier

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

Blend skin and color properties based on human ethnic values
"""

import material
import image
import image_operations
import numpy as np
from getpath import getSysDataPath


asianColor     = np.asarray([0.721, 0.568, 0.431], dtype=np.float32)
africanColor   = np.asarray([0.207, 0.113, 0.066], dtype=np.float32)
caucasianColor = np.asarray([0.843, 0.639, 0.517], dtype=np.float32)

class EthnicSkinBlender(object):
    """
    Skin blender for the adaptive_skin_tone litsphere texture. Makes sure that
    the texture is set to a blend of the three ethnic skin tones based on the
    human macro settings.
    """
    def __init__(self, human):
        self.human = human
        self.skinCache = { 'caucasian' : image.Image(getSysDataPath('litspheres/skinmat_caucasian.png')),
                           'african'   : image.Image(getSysDataPath('litspheres/skinmat_african.png')),
                           'asian'     : image.Image(getSysDataPath('litspheres/skinmat_asian.png')) }
        self._previousEthnicState = [0, 0, 0]

        self._litsphereTexture = None
        self._diffuseColor = material.Color()

        self.checkUpdate()

    def checkUpdate(self):
        newEthnicState = self.getEthnicState()
        if self._previousEthnicState != newEthnicState:
            self.update()
            self._previousEthnicState = newEthnicState
            return True
        return False

    def getEthnicState(self):
        return [ self.human.getCaucasian(),
                 self.human.getAfrican(),
                 self.human.getAsian()     ]

    def getLitsphereTexture(self):
        self.checkUpdate()
        return self._litsphereTexture

    def getDiffuseColor(self):
        self.checkUpdate()
        return self._diffuseColor

    def update(self):
        caucasianWeight = self.human.getCaucasian()
        africanWeight   = self.human.getAfrican()
        asianWeight     = self.human.getAsian()
        blends = []

        # Set litsphere texture
        if caucasianWeight > 0:
            blends.append( ('caucasian', caucasianWeight) )
        if africanWeight > 0:
            blends.append( ('african', africanWeight) )
        if asianWeight > 0:
            blends.append( ('asian', asianWeight) )

        if len(blends) == 1:
            img = self.skinCache[blends[0][0]]
            img.markModified()
        else:
            img = image_operations.mix(self.skinCache[blends[0][0]], self.skinCache[blends[1][0]], blends[0][1], blends[1][1])
            if len(blends) > 2:
                img = image_operations.mix(img, self.skinCache[blends[2][0]], 1.0, blends[2][1])

        # Set parameter so the image can be referenced when material is written to file (and texture can be cached)
        img.sourcePath = getSysDataPath("litspheres/adaptive_skin_tone.png")
        self._litsphereTexture = img

        # Set diffuse color
        diffuse = asianWeight     * asianColor   + \
                  africanWeight   * africanColor + \
                  caucasianWeight * caucasianColor
        self._diffuseColor = material.Color(diffuse)



```

## Source file: makehuman/plugins/3_libraries_teeth.py

{
  "path": "makehuman/plugins/3_libraries_teeth.py",
  "blob_sha": "0b63e18576dd157f341782732e894b170c0c5581",
  "sha256": "4211b3879b017d8ae409546d2fa5f0aebfdb6b9f693b33444236940ecb76b67a",
  "lines": 86,
  "bytes": 2362,
  "selection_score": 15,
  "keyword_line_counts": {
    "sss": 1,
    "teeth": 7,
    "rig": 1,
    "face": 1
  },
  "url": "https://github.com/hpretorius/makehuman/blob/8c767adbc4eec3dbccf837d5ce6e980e090c7a32/makehuman/plugins/3_libraries_teeth.py",
  "function_signature_candidates": [
    {
      "line": 45,
      "text": "def __init__(self, category):"
    },
    {
      "line": 48,
      "text": "def getObjectLayer(self):"
    },
    {
      "line": 51,
      "text": "def proxySelected(self, proxy):"
    },
    {
      "line": 54,
      "text": "def proxyDeselected(self, proxy, suppressSignal = False):"
    },
    {
      "line": 57,
      "text": "def onShow(self, event):"
    },
    {
      "line": 69,
      "text": "def load(app):"
    },
    {
      "line": 84,
      "text": "def unload(app):"
    }
  ]
}

```text

#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""
**Project Name:**      MakeHuman

**Product Home Page:** http://www.makehumancommunity.org/

**Code Home Page:**    https://bitbucket.org/MakeHuman/makehuman/

**Authors:**           Jonas Hauquier

**Copyright(c):**      MakeHuman Team 2001-2017

**Licensing:**         AGPL3

    This file is part of MakeHuman (www.makehumancommunity.org).

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

Teeth proxy library.
"""

import gui3d
import mh
import proxychooser

class TeethTaskView(proxychooser.ProxyChooserTaskView):

    def __init__(self, category):
        super(TeethTaskView, self).__init__(category, 'teeth', tagFilter = True, descriptionWidget = False)

    def getObjectLayer(self):
        return 5

    def proxySelected(self, proxy):
        self.human.teethProxy = proxy

    def proxyDeselected(self, proxy, suppressSignal = False):
        self.human.teethProxy = None

    def onShow(self, event):
        super(TeethTaskView, self).onShow(event)
        if gui3d.app.getSetting('cameraAutoZoom'):
            gui3d.app.setFaceCamera()


# This method is called when the plugin is loaded into makehuman
# The app reference is passed so that a plugin can attach a new category, task, or other GUI elements


taskview = None

def load(app):
    global taskview

    category = app.getCategory('Geometries')
    taskview = TeethTaskView(category)
    taskview.sortOrder = 1.3
    category.addTask(taskview)

    taskview.registerLoadSaveHandlers()


# This method is called when the plugin is unloaded from makehuman
# At the moment this is not used, but in the future it will remove the added GUI elements


def unload(app):
    taskview.onUnload()



```

## Source file: makehuman/plugins/3_libraries_tongue.py

{
  "path": "makehuman/plugins/3_libraries_tongue.py",
  "blob_sha": "531db7d6c5b56620afd0c7f5545da61043b1a3ba",
  "sha256": "d98e1180601f67ad9039c94c9a2790c275d2326e3338de56d1fecee26b5a983d",
  "lines": 86,
  "bytes": 2368,
  "selection_score": 15,
  "keyword_line_counts": {
    "sss": 1,
    "tongue": 7,
    "rig": 1,
    "face": 1
  },
  "url": "https://github.com/hpretorius/makehuman/blob/8c767adbc4eec3dbccf837d5ce6e980e090c7a32/makehuman/plugins/3_libraries_tongue.py",
  "function_signature_candidates": [
    {
      "line": 45,
      "text": "def __init__(self, category):"
    },
    {
      "line": 48,
      "text": "def getObjectLayer(self):"
    },
    {
      "line": 51,
      "text": "def proxySelected(self, proxy):"
    },
    {
      "line": 54,
      "text": "def proxyDeselected(self, proxy, suppressSignal = False):"
    },
    {
      "line": 57,
      "text": "def onShow(self, event):"
    },
    {
      "line": 69,
      "text": "def load(app):"
    },
    {
      "line": 84,
      "text": "def unload(app):"
    }
  ]
}

```text

#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""
**Project Name:**      MakeHuman

**Product Home Page:** http://www.makehumancommunity.org/

**Code Home Page:**    https://bitbucket.org/MakeHuman/makehuman/

**Authors:**           Jonas Hauquier

**Copyright(c):**      MakeHuman Team 2001-2017

**Licensing:**         AGPL3

    This file is part of MakeHuman (www.makehumancommunity.org).

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

Tongue proxy library.
"""

import gui3d
import mh
import proxychooser

class TongueTaskView(proxychooser.ProxyChooserTaskView):

    def __init__(self, category):
        super(TongueTaskView, self).__init__(category, 'tongue', tagFilter = True, descriptionWidget = False)

    def getObjectLayer(self):
        return 5

    def proxySelected(self, proxy):
        self.human.tongueProxy = proxy

    def proxyDeselected(self, proxy, suppressSignal = False):
        self.human.tongueProxy = None

    def onShow(self, event):
        super(TongueTaskView, self).onShow(event)
        if gui3d.app.getSetting('cameraAutoZoom'):
            gui3d.app.setFaceCamera()


# This method is called when the plugin is loaded into makehuman
# The app reference is passed so that a plugin can attach a new category, task, or other GUI elements


taskview = None

def load(app):
    global taskview

    category = app.getCategory('Geometries')
    taskview = TongueTaskView(category)
    taskview.sortOrder = 4
    category.addTask(taskview)

    taskview.registerLoadSaveHandlers()


# This method is called when the plugin is unloaded from makehuman
# At the moment this is not used, but in the future it will remove the added GUI elements


def unload(app):
    taskview.onUnload()



```
