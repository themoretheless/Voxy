# Facial rendering source scan: joeedh/FacialAutoRigger

Repository: https://github.com/joeedh/FacialAutoRigger

Pinned source revision: 94ddf2a8967012724ae0c155fd6390a7ad81093c

Discovery topic: facial rig

Description: Facial auto rigger for blender

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"rig": ["FacialAutoRigger/__init__.py", "FacialAutoRigger/cage.py", "FacialAutoRigger/data.json", "FacialAutoRigger/data.py", "FacialAutoRigger/ops.py", "FacialAutoRigger/props.py", "FacialAutoRigger/rig_utils.py", "FacialAutoRigger/rigger.py", "FacialAutoRigger/serializer.py", "FacialAutoRigger/shapedrivers.py", "FacialAutoRigger/shapekeys.py", "FacialAutoRigger/spline.py", "FacialAutoRigger/ui.py", "FacialAutoRigger/utils.py", "FacialRigGen.blend", "TestRigUtils.blend"]}

## Source file: FacialAutoRigger/rigger.py

{
  "path": "FacialAutoRigger/rigger.py",
  "blob_sha": "6206529329f6dfef5ddbce53bc16548aa1cb1c35",
  "sha256": "3af9d2b3deff622e91bcfc362067ec4589e8c37eb0a00f2abee9a7a8b3c2f5cd",
  "lines": 887,
  "bytes": 25413,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 20,
    "jaw": 1,
    "mouth": 4,
    "deform": 18,
    "rig": 129,
    "face": 21
  },
  "url": "https://github.com/joeedh/FacialAutoRigger/blob/94ddf2a8967012724ae0c155fd6390a7ad81093c/FacialAutoRigger/rigger.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def genBasePositions(meta):"
    },
    {
      "line": 14,
      "text": "def bloc(name):"
    },
    {
      "line": 33,
      "text": "def deformRig(scene, meta, newrig, oldrig, do_action=True, margin=0.0, use_surface_def=False):"
    },
    {
      "line": 103,
      "text": "def bloc(name):"
    },
    {
      "line": 291,
      "text": "def getpath(bone, property):"
    },
    {
      "line": 294,
      "text": "def getcurve(action, bone, property, array_index=0):"
    },
    {
      "line": 365,
      "text": "def updateConstraints(newrig, rest_frame=0):"
    },
    {
      "line": 380,
      "text": "def getkey(bone, con):"
    },
    {
      "line": 386,
      "text": "def do_update():"
    },
    {
      "line": 508,
      "text": "def ensureRig(name):"
    },
    {
      "line": 522,
      "text": "def copyRig(rig, name):"
    },
    {
      "line": 580,
      "text": "def genShapeKeyRig(scene, meta, rest_frame=1, rigname=\"FaceRig\"):"
    },
    {
      "line": 616,
      "text": "def generate(scene, meta, meshob, rigname=\"FaceRig\"):"
    },
    {
      "line": 638,
      "text": "def interp(a, b, t):"
    },
    {
      "line": 668,
      "text": "def head():"
    },
    {
      "line": 693,
      "text": "def makeChain(name, a, b, c, shape=1.0, isLid=True):"
    },
    {
      "line": 864,
      "text": "def generateShapeKeyRig(scene, meta, meshob, rigname=\"FaceRig\"):"
    }
  ]
}

```text

import bpy, bmesh
from mathutils import *
from math import *

from .data import basePositions;
from .utils import getMeshObject, DAGSortBones
from .cage import makeDeformMesh, simpleMeshDeformBind, simpleMeshDeformDeform
from .rig_utils import RigMaker
from .spline import *

def genBasePositions(meta):
    arm = meta.data
    pose = meta.pose
    def bloc(name):
        if name not in arm.bones: 
            print("Error: missing bone in armature data: ", name)            
            return Vector()
        
        loc = arm.bones[name].head
        loc = pose.bones[name].matrix_channel @ loc
        return loc
    
    buf = "basePositions = [\n"
    
    for p in meta.pose.bones:
        loc = bloc(p.name)
        buf += "  ('%s',  %s),\n" % (p.name, repr(loc))
    buf += "]\n"
    
    print("\n", buf, "\n")
    return buf
        
def deformRig(scene, meta, newrig, oldrig, do_action=True, margin=0.0, use_surface_def=False):
    basePositions2 = []
    
    for name, loc0 in basePositions:
        bone = meta.data.bones[name]
        pbone = meta.pose.bones[name]
        
        loc = Vector(bone.head)
        loc = pbone.matrix_channel @ loc
        
        basePositions2.append((name, loc))
    
    ob, vimap, inflate = makeDeformMesh(scene, meta, basePositions, "base", oldrig)
    targetob, targetvimap, targetinflate = makeDeformMesh(scene, meta, basePositions2, "target", oldrig, None)

    ob.hide_viewport = False
    targetob.hide_viewport = False
    
    for i in range(2):
      #i = 1
      ob2 = ob if i else targetob
      eps = inflate if i else targetinflate
      
      bm = bmesh.new()
      bm.from_mesh(ob2.data)
      
      for v in bm.verts:
        v.co += v.normal*eps*margin
      
      bm.to_mesh(ob2.data)
      ob2.data.update()
      #break
    
    #return
    #the rig insn't technically symmetric
    #should fix, for now make sure auto x-mirror
    #is off
    
    arm = meta.data
    pose = meta.pose
    
    defob = getMeshObject("_" + meta.name + "_rigdef", scene)
    defob.location = meta.location
    defob.select_set(True)

    defob.hide_viewport = False
    
    bm2 = bmesh.new()
    
    pose2 = newrig.pose
    arm2 = newrig.data
    bones = list(arm2.bones.keys())
    blocs = []
    
    #mat = Matrix(meta.matrix_world)
    #mat.invert()
    
    #mat = mat2 @ mat
    #mat = mat @ meta.matrix_world
    #imat = Matrix(mat)
    #imat.invert()
    
    boneheads = {}
    bpy.context.view_layer.objects.active = meta
    bpy.ops.object.mode_set(mode="EDIT")
    ebones = meta.data.edit_bones
    
    for bone in ebones:
        boneheads[bone.name] = Vector(bone.head)

    def bloc(name):
        #print(boneheads)
        loc = boneheads[name]
        loc = pose.bones[name].matrix_channel @ loc
        return loc
    
    bpy.ops.object.mode_set(mode="OBJECT")

    bpy.context.view_layer.objects.active = newrig
    bpy.ops.object.mode_set(mode="EDIT")
    ebones = newrig.data.edit_bones
    matlens = {}
    
    for name in bones:
        bone = ebones[name]
        #make three edges so we can derive a linear transformation
        
        a = bm2.verts.new(bone.head)
        b = bm2.verts.new(bone.tail)
        bm2.edges.new([a, b])
        
        t1 = bone.tail - bone.head
        t1.normalize()
        
        if abs(t1[2]) > abs(t1[1]) and abs(t1[2]) > abs(t1[0]):
          t2 = Vector([1, 0, 0])
        else:
          t2 = Vector([0, 0, 1])
        
        t2 = t1.cross(t2)
        t2.normalize()
        
        t3 = t2.cross(t1)
        t3.normalize()
        
        t1 = Vector([1, 0, 0])
        t2 = Vector([0, 1, 0])
        t3 = Vector([0, 0, 1])
        
        len1 = (bone.tail - bone.head).length*0.001
        
        matlens[name] = len1
        
        t1 = t1*len1 + a.co
        t2 = t2*len1 + a.co
        t3 = t3*len1 + a.co
        
        bm2.edges.new([a, bm2.verts.new(t1)])
        bm2.edges.new([a, bm2.verts.new(t2)])
        bm2.edges.new([a, bm2.verts.new(t3)])
        
        
        #print(bone.name)
        pass
    ebones = None
    bpy.ops.object.mode_set(mode="OBJECT")
            
    #for name, loc in basePositions:
        #bm2.verts.new(loc)
        
    bm2.to_mesh(defob.data)
    defob.data.update()
    
    defob.modifiers.clear()
    if not use_surface_def:
      mod = defob.modifiers.new("_MeshDeform", "MESH_DEFORM")
      mod.object = ob
      bpy.ops.object.meshdeform_bind({
          "object" : defob,
          "active_object" : defob,
          "modifier" : "_MeshDeform"
      }, modifier="_MeshDeform")
      #"""
    
    bm = bmesh.new()
    bm.from_mesh(ob.data)
    
    defbm = bmesh.new()
    defbm.from_mesh(defob.data)
    defbm.verts.index_update()
    
    if use_surface_def:
      bm.normal_update()
      #bmesh.ops.subdivide_edges(defbm, edges=defbm.edges, cuts=32, use_grid_fill=True)
      bindcos = simpleMeshDeformBind(bm, defbm)
    
    bm.verts.index_update()
    bm.verts.ensure_lookup_table()
    
    for v in bm.verts:
        co2 = targetob.data.vertices[v.index].co
        #v.co += (co2 - v.co)*0.5
        v.co = co2
        
        pass
            
    bm.normal_update()
        
    bm.to_mesh(ob.data)
    ob.data.update()
    
    dgraph = bpy.context.evaluated_depsgraph_get()
    
    if not use_surface_def:
      bm2 = bmesh.new()
      bm2.from_object(defob, dgraph, deform=True, cage=True)
    else:
      bm2 = defbm.copy()
      bm2.verts.index_update()
      
      simpleMeshDeformDeform(bm, bm2, bindcos)
      
      bm2.to_mesh(defob.data)
      defob.data.update()
    
    #bm2 = bmesh.new()
    #bm2.from_object(defob, dgraph, deform=True, cage=True)
    
    #for i, v in bm.verts:
    
    bm2.verts.ensure_lookup_table()
    bpy.context.view_layer.objects.active = newrig
    
    bpy.ops.object.mode_set(mode="EDIT")
    ebones = newrig.data.edit_bones
    pbones = newrig.pose.bones
    
    #linear cage transformation matrices
    bonemats = {}
    
    for i, name in enumerate(bones):
        bone = ebones[name]
        blen2 = matlens[name]
        
        offset = bm2.verts[i*5].co - bone.head;
        
        old = Vector(bone.head)
        oldtail = Vector(bone.tail)
        bone.head = bm2.verts[i*5].co
        bone.tail = bm2.verts[i*5+1].co
        
        dx = (bm2.verts[i*5+2].co - bone.head) / blen2
        dy = (bm2.verts[i*5+3].co - bone.head) / blen2
        dz = (bm2.verts[i*5+4].co - bone.head) / blen2
        
        mat = Matrix([
        [dx[0], dy[0], dz[0]],
        [dx[1], dy[1], dz[1]],
        [dx[2], dy[2], dz[2]]
        ])
        #mat.transpose();
        try:
          pass
          #mat.invert()
        except ValueError:
          print("matrix inversion failure", mat)
          
        mat.resize_4x4()
        
        #mat = Matrix()
        #print(mat)
        bonemats[name] = mat
        
        pbone = pbones[bone.name]
        loc, quat, scale = mat.decompose()
        #scale = (scale[0] + scale[1] + scale[2])/3.0
        scale = min(min(scale[0], scale[1]), scale[2])
        
        #pbone.custom_shape_scale *= scale
        
        
        #mat = Matrix.Translation(offset)
        #print(Matrix.Translation(offset))
        #print("=====", "\n", mat)
        
        #mat.transpose()
        #mat.invert()
        #print(dx, dy, dz, mat)
        #print("\n", mat @ Vector(), "\n", bone.head)
    
    if do_action:
      src_action = bpy.data.actions["FacialPoses"]
      
      action = newrig.animation_data.action
      if action is None or action == src_action:
        action = src_action.copy()
      newrig.animation_data.action = action
      
      def getpath(bone, property):
        return 'pose.bones["%s"].%s' % (bone.name, property)
      
      def getcurve(action, bone, property, array_index=0):
        path = getpath(bone, property)
        
        srcfc = None
        for fc in src_action.fcurves:
          if fc.data_path == path and fc.array_index == array_index:
            srcfc = fc
            break
          
        if srcfc is None:
          raise RuntimeError("failed to find source fcurce")
          
        for fc in action.fcurves:
          if fc.data_path == path and fc.array_index == array_index:
            for i, key in enumerate(fc.keyframe_points):
              fc.keyframe_points[i].co = srcfc.keyframe_points[i].co
              
            return fc


      for pbone in newrig.pose.bones:
        fcx = getcurve(action, pbone, "location", 0)
        fcy = getcurve(action, pbone, "location", 1)
        fcz = getcurve(action, pbone, "location", 2)
        
        if fcx is None:
          print("no animation key for bone", pbone.name)
          continue
        
        mat = bonemats[pbone.name]
        
        l = len(fcx.keyframe_points)
        if len(fcy.keyframe_points) != l or len(fcz.keyframe_points) != l:
          axis = ""
          if len(fcy.keyframe_points) != l:
            axis += " 1"
          if len(fcz.keyframe_points) != l:
            axis += " 2"
            
          print("ERROR missing axis keyframe data for axis" + axis)
          print(len(fcx.keyframe_points), len(fcy.keyframe_points), len(fcz.keyframe_points))
        
        #continue
        kx = fcx.keyframe_points
        ky = fcy.keyframe_points
        kz = fcz.keyframe_points
        
        for i in range(len(kx)):
          co = Vector([kx[i].co[1], ky[i].co[1], kz[i].co[1]])
          #print(co, co - (mat @ co))
          co = mat @ co
          
          kx[i].co[1] = co[0]
          ky[i].co[1] = co[1]
          kz[i].co[1] = co[2]
        
        fcx.update()
        fcy.update()
        fcz.update()
      
    for i in range(len(newrig.data.layers)):
        newrig.data.layers[i] = True
        
    bpy.ops.armature.calculate_roll(type="GLOBAL_NEG_Y")
    
    ebones = None
    bpy.ops.object.mode_set(mode="OBJECT")
    
    #bpy.ops.object.mode_set(mode="POSE")
    #bpy.context.view_layer.objects.active = defob
    
def updateConstraints(newrig, rest_frame=0):
  dgraph = bpy.context.evaluated_depsgraph_get()
  
  #set to rest pose frame
  if rest_frame is not None:
    bpy.context.scene.frame_set(rest_frame)

  #newrig.data.pose_position = "REST"
  newrig.data.update_tag()
  
  bpy.context.view_layer.objects.active = newrig
  bpy.ops.object.mode_set(mode="POSE")
  bpy.ops.pose.select_all(action="DESELECT")
  
  influences = {}
  def getkey(bone, con):
      return bone.name + "_" + con.name

  pbones = newrig.pose.bones
  abones = newrig.data.bones

  def do_update():
      newrig.update_tag(refresh=set(["OBJECT", "DATA", "TIME"]))
      newrig.data.update_tag()
      bpy.context.view_layer.update()
      
      evalrig = newrig.evaluated_get(dgraph)
      evalpose = evalrig.pose
      return evalrig, evalpose
  
  sortnames = DAGSortBones(newrig)
  #print("--->", len(sortnames), len(newrig.pose.bones), len(newrig.data.bones))
  for stepi in range(4):
      do_update()
      
      for name in sortnames:
          pbone = newrig.pose.bones[name]
          abone = newrig.data.bones[pbone.name]
          newrig.data.bones.active = abone
          
          for con in pbone.constraints:
              if con.type == "LIMIT_DISTANCE":
                  inf = con.influence
                  con.influence = 0
                  do_update()
                  
                  #print("Doing limit distance constraint")
                  #print("Reseted stretch to", con.rest_length, "for", pbone.name)
                  
                  target = con.subtarget
                  if target not in pbones:
                    print("bad constraint target", target, "for limit-distance for", pbone.name)
                    continue
                   
                  abone2 = abones[target] 
                  pbone2 = pbones[target]
                  
                  a = pbone2.matrix @ Vector()
                  b = pbone.matrix @ Vector()
                  
                  
                  #l = (abone2.head - abone.head).length
                  l = (a - b).length
                  con.distance = l
                  
                  con.influence = inf
                  do_update()
                  
                  #print("  distance", l)
                  
              if con.type == "STRETCH_TO": # and stepi == 0:
                  #print("Reseted stretch to", con.rest_length, "for", pbone.name)
                  
                  target = con.subtarget
                  if target not in pbones:
                    print("bad constraint target", target, "for stretchto of", pbone.name)
                    continue
                   
                  abone2 = abones[target] 
                  pbone2 = pbones[target]
                  
                  a = pbone2.matrix @ Vector()
                  b = pbone.matrix @ Vector()
                  
                  
                  #l = (abone2.head - abone.head).length
                  l = (a - b).length
                  con.rest_length = l
                  do_update()
              
              #do child of constraints after first pass
              if stepi == 0:
                continue
                
              if con.type == "CHILD_OF":
                  #print("  -> doing child of for bone", pbone.name)
                  
                  key1 = getkey(pbone, con)
                  if key1 not in influences:
                      influences[key1] = con.influence
                      con.influence = 1.0
                  
                  evalrig, evalpose = do_update()

                  if con.subtarget not in pbones:
                    print("missing child off target bone '%s'" % (con.subtarget))
                    continue
                    
                  pmat = evalpose.bones[con.subtarget].matrix
                  #mymat = abone.convert_local_to_pose(pmat, abone.matrix_local)
                  
                  parent = Matrix(pmat)
                  #parent = Matrix(mymat)
                  
                  try:
                    parent.invert()
                  except ValueError:
                    print(parent, "lacks an inverse")
                    parent = Matrix(abone.matrix_local)
                    parent.invert()
                    
                    continue
                  #parent = Matrix(abone.matrix_local)
                  #parent.invert()
                  
                  #print(parent)
                  con.inverse_matrix = parent
                  
 
  for pbone in newrig.pose.bones:
      for con in pbone.constraints:
          key1 = getkey(pbone, con)
          if key1 in influences:
              con.influence = influences[key1]
              
  newrig.data.pose_position = "POSE"
  bpy.ops.object.mode_set(mode="OBJECT")
  do_update()
  
    #bpy.data.objects.remove(defob)
    #bpy.data.objects.remove(ob)
    

def ensureRig(name):
  if name in bpy.data.objects:
    ob = bpy.data.objects[name]
    print(ob.name, ob.name not in bpy.context.scene.objects)

    if ob.name not in bpy.context.scene.objects:
      bpy.context.scene.collection.objects.link(ob)
    return ob
  else:
    ob = bpy.data.objects.new(name, bpy.data.armatures.new(name))
    bpy.context.scene.collection.objects.link(ob)

    return ob

def copyRig(rig, name):
    #copy rig
    rigob = rig
    newname = name
            
    bpy.ops.object.select_all(action="DESELECT")

    ctx = {
        "selected_objects" : [rigob],
        "active_object" : rigob,
    }
    
    rigob.select_set(True)
    ret = bpy.ops.object.duplicate(ctx, mode="DUMMY")
    #print(ret)
    #print(ctx)
    #print(bpy.context.active_object)
    
    rig2 = bpy.context.selected_objects[0]
    #print(rig2)
    
    if newname in bpy.data.objects:
        rig3 = bpy.data.objects[newname]
        rig3.data = rig2.data
        
        bad = False
        
        for p in rig2.pose.bones:
            #XXX FOR TESTING PURPOSES always regen; change back later
            if 1: #p.name not in rig3.pose.bones:
                print("missing bone; full regen")
                bad = True
                break
            #TODO
            #hrm, what to sync?
        if bad:
            #relink modifiers
            for ob in bpy.data.objects:
              if ob.type != "MESH": continue
              for mod in ob.modifiers:
                if not hasattr(mod, "object"): continue
                if mod.object != rig3: continue
                mod.object = rig2
                
            bpy.data.objects.remove(rig3)
            rig2.name = newname
            #relink armature modifiers
        else:
            bpy.data.objects.remove(rig2)
            rig2 = rig3
            
        pass
    else:
        rig2.name = newname
    
    return rig2    


def genShapeKeyRig(scene, meta, rest_frame=1, rigname="FaceRig"):
  oldframe = bpy.context.scene.frame_current
  
  internal_rig = bpy.data.objects["_autorig_InternalFaceRig"]
  
  name = rigname + "_skey_gen"

  newrig = copyRig(internal_rig, name)
  newrig.location = meta.location

  use_mirror_x = newrig.data.use_mirror_x
  newrig.data.use_mirror_x = False

  deformRig(scene, meta, newrig, internal_rig)
  updateConstraints(newrig, rest_frame=rest_frame)
  
  meta.select_set(True)
  bpy.context.view_layer.objects.active = newrig
  
  newrig.data.use_mirror_x = use_mirror_x
  
  bpy.context.scene.frame_set(oldframe)
  bpy.context.view_layer.update()
  
  return newrig

"""
widgets
EyeWidget : 8
NoseWidgetShape : 50
MouthWidget : 15
TweakShape2 : 200
NoseWidgetShape2 : 25
HeadShape2 : 2

"""
def generate(scene, meta, meshob, rigname="FaceRig"):
  if not meta.data.facerig.ismeta:
    meta = meta.data.facerig.metaob
    rigname = meta.data.facerig.rigname 

  name = rigname
  
  newrig = ensureRig(name)
  print("newrig", newrig, name)

  newrig.data.facerig.metaob = meta
  newrig.data.facerig.rigname = meta.data.facerig.rigname
  newrig.data.facerig.ismeta = False
  newrig.data.facerig.meshob = meta.data.facerig.meshob

  arm = newrig.data

  rigger = RigMaker(newrig, newrig.data, bpy.data.objects["DefWidget"], 1000.0)
  
  locs = {}
  loclist = []

  def interp(a, b, t):
    return a + (b - a) * t

  for b in meta.pose.bones:
    b2 = meta.data.bones[b.name]
    loc = b.matrix_channel @ b2.head
    locs[b.name] = loc 
    loclist.append(loc)

    print(b.name, loc)

  print("meta", meta)

  vmin = Vector([1e17, 1e17, 1e17])
  vmax = Vector([-1e17, -1e17, -1e17])
  for loc in loclist:
    for i in range(3):
      vmin[i] = min(vmin[i], loc[i])
      vmax[i] = max(vmax[i], loc[i])

  scale = (vmax - vmin).length

  rigger.widgetScale = scale * 0.1
  rigger.start()

  headhead = Vector([vmax[0]*0.5 + vmin[0]*0.5, vmax[1], vmin[2]])
  headtail = Vector(headhead)
  headtail[2] += (vmax[2] - vmin[2])*0.75
  rigger.getBone("Head", headhead, headtail, layers=0)

  def head():
    return arm.edit_bones["Head"]

  for side in [".L", ".R"]:
    metaeye = locs["Eye" + side]

    eye = rigger.getBone("Eye" + side, metaeye, metaeye + Vector([0,0,scale*0.2]))
    print("eye", eye) 
    eye.use_deform = False
    eye.parent = head()

    a = locs["BrowCenter" + side]
    b = locs["BrowMid" + side]
    c = locs["BrowCorner" + side]

    ta = locs["LidLeft.T" + side]
    tb = locs["LidTop" + side]
    tc = locs["LidRight.T" + side]

    ba = locs["LidLeft.B" + side]
    bb = locs["LidBottom" + side]
    bc = locs["LidRight.B" + side]

    eye_r = (ta - metaeye).length
    
    def makeChain(name, a, b, c, shape=1.0, isLid=True):
      dv = shape * (c - a) / 4.0

      spline = ArchLengthSpline3d([
        [a, a+(b-a)/3.0, b-dv, b],
        [b, b+dv, c-(c-b)/3.0, c]
      ])
      
      chain = rigger.makeBendyChain(name, 4, spline, side, deflayer=1)
      
      ctrl = rigger.getBone(name + "Main" + side, b, b + Vector([0,0,scale*0.05]), widget="default", layers=0).name
      
      if isLid:
        parent = rigger.getBone("MCH-" + name + side, a, c, widget=None, layers=11).name
        parent = arm.edit_bones[parent]
        parent.parent = head()
        parent.use_deform = False

      ctrl = arm.edit_bones[ctrl]

      ctrl.parent = head()
      ctrl.use_deform = False

      #move a little bit in front of chain to avoid numerical instability
      ctrl.head += Vector([0, -scale*0.035, 0.0])
      ctrl.tail += Vector([0, -scale*0.035, 0.0])

      rollaxis = (c - a).cross(ctrl.head - metaeye)
      rollaxis.normalize()

      if isLid:
        parent.align_roll(rollaxis)
        con = rigger.cLockedTrack(parent, ctrl)
        con.lock_axis = "LOCK_Y"
        con.track_axis = "TRACK_NEGATIVE_X"

      eye = arm.edit_bones["Eye" + side]
      
      for i, bname in enumerate(chain):
        bone = arm.edit_bones[bname]
        if isLid:
          bone.parent = parent
        else:
          bone.parent = None

          t = i / (len(chain) - 1)
          t = t*0.9 + 0.1
          t = 1.0 - abs(t-0.5)*2.0

          con = rigger.cChildOf(bone, ctrl, inf=t, name="childof_ctrl")
          con = rigger.cChildOf(bone, head(), inf=1.0-t, name="childof_head")

        if isLid:
          con = rigger.cLimitDistance(bone, eye, mode="LIMITDIST_ONSURFACE")
        else:
          con = rigger.cLimitDistance(bone, eye, mode="LIMITDIST_OUTSIDE")
          
        con.distance = 0.0

        rollaxis2 = (bone.head + bone.tail)*0.5 - ctrl.head
        rollaxis2.normalize()

        bone.align_roll(rollaxis2)

        if not isLid:
          continue

        #continue
        bname2 = "DEF-" + bname
        if bname2 not in arm.edit_bones:
          continue #last ctrl at end of chain
        bone2 = arm.edit_bones[bname2]

        rollaxis2 = bone2.head - metaeye
        bone2.align_roll(rollaxis2)

        con = rigger.cLockedTrack(bone2, "Eye" + side, name="locktomain")
        con.lock_axis = "LOCK_Y"
        con.track_axis = "TRACK_NEGATIVE_Z"
        #not working -> con.use_transform_limit = True
      
    makeChain("Brow", a, b, c, isLid=False)
    makeChain("Lid.T", ta, tb, tc)
    makeChain("Lid.B", ba, bb, bc)
    
    #create blockers
    vec = locs["Forehead" + side] - locs["BrowCenter" + side]
    co1 = a + vec*0.25
    co2 = c + vec*0.25

    blocker = rigger.getBone("DEF-Forehead" + side, co1, co2, layers=12) 
    blocker.parent = head()

    vec = locs["Cheek" + side] - locs["LidBottom" + side]
    co1 = ba + vec*0.75
    co2 = bc + vec*0.75

    blocker = rigger.getBone("DEF-Cheek" + side, co1, co2, layers=12) 
    blocker.parent = head()

    co1 = interp(locs["JawHinge" + side], locs["LidRight.B" + side], 0.5)
    co2 = interp(locs["Temple" + side], locs["BrowCorner" + side], 0.5)

    blocker = rigger.getBone("DEF-Temple" + side, co1, co2, layers=12) 
    blocker.parent = head()

  co1 = locs["BrowCenter.R"]*0.5 + locs["BrowCenter.L"]*0.5
  co2 = locs["Nose"]
  blocker = rigger.getBone("DEF-Nose", co1, co2, layers=12)
  blocker.head[1] += scale*0.04
  blocker.tail[1] += scale*0.04
  blocker.parent = head()

  main = (locs["MouthUpper"] + locs["MouthLower"])*0.5
  main[1] -= scale*0.1
  main = rigger.getBone("Mouth", main, main+Vector([0,0,scale*0.1]), widget="default", layers=0)
  main.use_deform = False
  main.parent = head()

  zip = (locs["MouthUpper"] + locs["MouthLower"])*0.5
  zip[1] -= scale*0.1
  zip = rigger.getBone("Zip", zip, zip+Vector([0,0,scale*0.025]), widget="default", layers=0)
  zip.parent = head()

  bpy.ops.object.mode_set(mode="POSE")
  rigger.resetDistances()

  updateConstraints(newrig, None)
  rigger.stop()

  bpy.ops.object.mode_set(mode="POSE")
  
  return
  
  #base_rig = bpy.data.objects["_autorig_FaceRigBase"]

  #newrig = copyRig(base_rig, name)
  newrig.location = meta.location

  newrig.data.facerig.meshob = meta.data.facerig.meshob
  newrig.data.facerig.rigname = meta.data.facerig.rigname
  
  use_mirror_x = newrig.data.use_mirror_x
  newrig.data.use_mirror_x = False

  deformRig(scene, meta, newrig, base_rig, do_action=False, margin=0, use_surface_def=True)
  updateConstraints(newrig, rest_frame=1)
  
  #meta.select_set(True)
  bpy.context.view_layer.objects.active = newrig
  bpy.ops.object.mode_set(mode="EDIT")
  
  #update EyeTarget, which lies outside the cage
  
  ebones = newrig.data.edit_bones
  #print(list(ebones.keys()))
  ft = ebones["EyeTarget"]
  
  eye1 = ebones["Eye.L"]
  eye2 = ebones["Eye.R"]
  size = (eye1.head - eye2.head).length
  
  ft.head[0] = 0
  ft.head[1] = eye1.head[1] - size*2
  ft.head[2] = eye1.head[2]
  
  ft.tail = ft.head + Vector([0, 0, size*0.5])
  
  bpy.ops.object.mode_set(mode="OBJECT")
  newrig.data.use_mirror_x = use_mirror_x
  
def generateShapeKeyRig(scene, meta, meshob, rigname="FaceRig"):
  #genBasePositions(bpy.data.objects["_autorig_MetaFaceRig"])
  newrig = genShapeKeyRig(scene, meta, rigname=rigname)
  
  mod = None
  
  for md in meshob.modifiers:
    if md.type == "ARMATURE" and md.object == newrig:
      md.show_viewport = True
      mod = md
    elif md.type == "ARMATURE" and md.object != newrig:
      md.show_viewport = False
    
  if mod is None:
    mod = meshob.modifiers.new("autorig1", "ARMATURE")
    mod.object = newrig

  bpy.ops.object.select_all(action="DESELECT")
  meshob.select_set(True)
  newrig.select_set(True)
  bpy.context.view_layer.objects.active = newrig
  
  bpy.ops.object.parent_set(type="ARMATURE_AUTO", xmirror=False)
  

```

## Source file: FacialAutoRigger/rig_utils.py

{
  "path": "FacialAutoRigger/rig_utils.py",
  "blob_sha": "82d50a1ffb6fe441c889ea7fa5329ac1a00ac127",
  "sha256": "a9358801560b8ea50842650a389a6349c2c2b8681d734ff51a2726a4888a47da",
  "lines": 317,
  "bytes": 7809,
  "selection_score": 15,
  "keyword_line_counts": {
    "deform": 2,
    "rig": 3
  },
  "url": "https://github.com/joeedh/FacialAutoRigger/blob/94ddf2a8967012724ae0c155fd6390a7ad81093c/FacialAutoRigger/rig_utils.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def getObject(name, type):"
    },
    {
      "line": 28,
      "text": "def getArmature(name):"
    },
    {
      "line": 32,
      "text": "def __init__(self, name, ob, scale=1.0):"
    },
    {
      "line": 38,
      "text": "def __init__(self, ob, arm, defaultWidget, defaultWidgetScale=1.0):"
    },
    {
      "line": 47,
      "text": "def start(self, ctx=None):"
    },
    {
      "line": 67,
      "text": "def stop(self, ctx=None):"
    },
    {
      "line": 78,
      "text": "def push(self, ctx=None):"
    },
    {
      "line": 84,
      "text": "def pop(self, ctx=None):"
    },
    {
      "line": 92,
      "text": "def addWidget(self, name, ob, scale=1.0):"
    },
    {
      "line": 95,
      "text": "def parentBone(self, bone, parent, connected=False):"
    },
    {
      "line": 103,
      "text": "def solveConstraints(self, iterations=3):"
    },
    {
      "line": 106,
      "text": "def getBone(self, name, head, tail, roll=None, widget=None, layers=None):"
    },
    {
      "line": 155,
      "text": "def resetDistances(self):"
    },
    {
      "line": 173,
      "text": "def _getCon(self, bone, ctype, name, subtarget=None, inf=1.0, prefix=\"RIG-\"):"
    },
    {
      "line": 194,
      "text": "def cStretchTo(self, bone, target, inf=1.0, name=\"stretchto\"):"
    },
    {
      "line": 203,
      "text": "def poseBone(self, bone):"
    },
    {
      "line": 209,
      "text": "def cCopyLocation(self, bone, target, inf=1.0, name=\"copyloc\"):"
    },
    {
      "line": 212,
      "text": "def cTrackTo(self, bone, target, inf=1.0, name=\"trackto\"):"
    },
    {
      "line": 215,
      "text": "def cLockedTrack(self, bone, target, inf=1.0, name=\"lockedtrack\"):"
    },
    {
      "line": 220,
      "text": "def cChildOf(self, bone, target, inf=1.0, name=\"childof\"):"
    },
    {
      "line": 223,
      "text": "def cLimitVolume(self, bone, target, inf=1.0, name=\"limitvolume\"):"
    },
    {
      "line": 226,
      "text": "def cLimitDistance(self, bone, target, inf=1.0, mode=\"OUTSIDE\", dis=0.0, name=\"limitdistance\"):"
    },
    {
      "line": 234,
      "text": "def makeChain(self, name, segs, spline, suffix=\"\", bone_visit=None, layers=None, parentBones=True):"
    },
    {
      "line": 264,
      "text": "def doVisit(self, bones, visit):"
    },
    {
      "line": 271,
      "text": "def makeBendyChain(self, name, segs, spline, suffix=\"\", bone_visit=None, widget=\"default\", layers=None, deflayer=10):"
    },
    {
      "line": 277,
      "text": "def visit(bone, i, chain):"
    }
  ]
}

```text

from mathutils import *
from math import *
import time, random

from . import utils

import bpy, bmesh

def getObject(name, type):
  if name in bpy.data.objects:
    ob = bpy.data.objects[name]
  else:
    if type == "ARMATURE":
      data = bpy.data.armatures.new(name)
    elif type == "MESH":
      data = bpy.data.meshes.new(name)

    ob = bpy.data.objects.new(name, data)

  scene = bpy.context.scene
  coll = scene.collection

  if ob.name not in coll.all_objects:
    coll.objects.link(ob)

  return ob

def getArmature(name):
  return getObject(name, "ARMATURE")

class Widget:
  def __init__(self, name, ob, scale=1.0):
    self.name = name
    self.ob = ob
    self.scale = scale

class RigMaker:
  def __init__(self, ob, arm, defaultWidget, defaultWidgetScale=1.0):
    self.ob = ob
    self.arm = arm
    self.widgets = {}
    self.addWidget("default", defaultWidget, defaultWidgetScale)
    self._stack = []
    self.widgetScale = 1.0
    self.startLayers = None

  def start(self, ctx=None):
    if ctx is None:
      ctx = bpy.context
    
    self.push(ctx)
    utils.ensureObjectMode(ctx)

    utils.setActiveOb(ctx, self.ob)

    self.startLayers = list(self.ob.data.layers)
    
    for i in range(len(self.ob.data.layers)):
      self.ob.data.layers[i] = True

    #paranoia check to make sure pose bones are flushed properly
    bpy.ops.object.mode_set(mode="OBJECT")

    #now enter armature edit mode
    bpy.ops.object.mode_set(mode="EDIT")

  def stop(self, ctx=None):
    if ctx is None:
      ctx = bpy.context
    self.pop(ctx)

    if not self.startLayers:
      return

    for i in range(len(self.ob.data.layers)):
      self.ob.data.layers[i] = self.startLayers[i]

  def push(self, ctx=None):
    if ctx is None:
      ctx = bpy.context
    
    self._stack.append([ctx, utils.saveBpyContext(ctx)])
  
  def pop(self, ctx=None):
    oldctx, data = self._stack.pop()

    if ctx is None:
      ctx = oldctx
    
    utils.loadBpyContext(data, ctx)

  def addWidget(self, name, ob, scale=1.0):
    self.widgets[name] = Widget(name, ob, scale)

  def parentBone(self, bone, parent, connected=False):
    if type(parent) is str:
      parent = self.arm.edit_bones[parent]
    if type(bone) is str:
      bone = self.arm.edit_bones[bone]
    
    bone.parent = parent

  def solveConstraints(self, iterations=3):
    pass

  def getBone(self, name, head, tail, roll=None, widget=None, layers=None):
    if type(widget) is str:
      widget = self.widgets[widget]
    
    arm = self.arm
    ob = self.ob

    head = Vector(head)
    tail = Vector(tail)

    if (tail - head).length < 0.0001:
      tail += Vector([0, 0, 0.01])

    if name in arm.edit_bones:
      bone = arm.edit_bones[name]
    else:
      bone = arm.edit_bones.new(name)

      #blender culls zero-length bones.
      bone.head = Vector(head)
      bone.tail = Vector(tail)

      print(head, tail)
      
      #update pose channel
      bpy.ops.object.mode_set(mode="OBJECT")
      bpy.ops.object.mode_set(mode="EDIT")

      bone = arm.edit_bones[name]
    
    bone.head = Vector(head)
    bone.tail = Vector(tail)

    if widget is not None:
      pbone = self.poseBone(bone)
      pbone.custom_shape = widget.ob
      pbone.custom_shape_scale = widget.scale * self.widgetScale

    if layers is not None:
      bone.layers[0] = False
      
      if type(layers) == int:
        bone.layers[layers] = True
      else:
        for layer in layers:
          bone.layers[layer] = True

    return bone

  def resetDistances(self):
    cons = []
    types = set(["LIMIT_DISTANCE", "STRETCH_TO"])
        
    for bone in self.ob.pose.bones:
      print(bone.name)

      for con in bone.constraints:
        if not con.name.startswith("RIG-"):
          continue

        print("Found " + con.name, con.type)

        if con.type in ["LIMIT_DISTANCE"]:
          con.distance = 0.0
        elif con.type == "STRETCH_TO":
          con.rest_length = 0.0
          
  def _getCon(self, bone, ctype, name, subtarget=None, inf=1.0, prefix="RIG-"):
    name = prefix + name

    pbone = self.poseBone(bone)
    for c in pbone.constraints:
      if c.name == name:
        return c
    
    c = pbone.constraints.new(ctype)
    c.name = name
    c.influence = inf 

    if subtarget is not None:
      if type(subtarget) != str:
        subtarget = subtarget.name

      c.target = self.ob 
      c.subtarget = subtarget

    return c

  def cStretchTo(self, bone, target, inf=1.0, name="stretchto"):
    pbone = self.poseBone(bone)

    print("Making constraint to", bone.name, pbone.name)

    con = self._getCon(bone, "STRETCH_TO", name, subtarget=target, inf=inf)
  
    con.rest_length = 0 #flag distance reset
  
  def poseBone(self, bone):
    if type(bone) != str:
      bone = bone.name
    
    return self.ob.pose.bones[bone]
    
  def cCopyLocation(self, bone, target, inf=1.0, name="copyloc"):
    con = self._getCon(bone, "COPY_LOCATION", name, subtarget=target, inf=inf)
      
  def cTrackTo(self, bone, target, inf=1.0, name="trackto"):
    pass

  def cLockedTrack(self, bone, target, inf=1.0, name="lockedtrack"):
    con = self._getCon(bone, "LOCKED_TRACK", name, subtarget=target, inf=inf)

    return con

  def cChildOf(self, bone, target, inf=1.0, name="childof"):
    return self._getCon(bone, "CHILD_OF", name, subtarget=target, inf=inf)

  def cLimitVolume(self, bone, target, inf=1.0, name="limitvolume"):
    pass

  def cLimitDistance(self, bone, target, inf=1.0, mode="OUTSIDE", dis=0.0, name="limitdistance"):
    #if dis is None:
    #  dis = 0.0 #constraint will reset itself
    con = self._getCon(bone, "LIMIT_DISTANCE", name, subtarget=target)
    con.limit_mode = mode

    return con

  def makeChain(self, name, segs, spline, suffix="", bone_visit=None, layers=None, parentBones=True):
    spline.checkUpdate()

    slen = spline.length
    s = 0

    ds = slen / segs
    chain = []

    for i in range(segs):
      head = spline.eval(s)
      tail = spline.eval(s + ds)

      name2 = "%s%i%s" % (name, i+1, suffix)

      bone = self.getBone(name2, head, tail, layers=layers)

      if i > 0 and parentBones:
        self.parentBone(bone, chain[i-1], True)

      chain.append(name2)
      s += ds

    arm = self.arm
    chain2 = [arm.edit_bones[s] for s in chain]

    self.doVisit(chain2, bone_visit)

    return chain

  def doVisit(self, bones, visit):
    if not visit:
      return

    for i in range(len(bones)):
      visit(bones[i], i, bones)
  
  def makeBendyChain(self, name, segs, spline, suffix="", bone_visit=None, widget="default", layers=None, deflayer=10):
    hgt = 0.25 * spline.length / segs

    arm = self.arm 
    ctrlchain = []

    def visit(bone, i, chain):
      name2 = "%s%i%s" % (name, i+1, suffix)
      head = Vector(bone.head)
      tail = Vector(head)
      tail[2] += hgt

      bone2 = self.getBone(name2, head, tail, None, widget, layers=layers)
      bone2.use_deform = False
      ctrlchain.append(name2)

      #name2 = "%s%i

    chain = self.makeChain("DEF-"+name, segs, spline, suffix, visit, layers=deflayer, parentBones=False)

    deflast = arm.edit_bones[chain[-1]]

    name2 = "%s%i%s" % (name, len(chain)+1, suffix)
    head = Vector(deflast.tail)
    tail = Vector(head)
    tail[2] += hgt

    bone2 = self.getBone(name2, head, tail, None, widget, layers=layers)
    bone2.use_deform = False
    ctrlchain.append(name2)
    
    ctrlchain2 = [arm.edit_bones[s] for s in ctrlchain]
    chain2 = [arm.edit_bones[s] for s in chain]

    for i in range(len(chain)):
      a = ctrlchain2[i]
      b = ctrlchain2[i+1]
      defa = chain2[i]
      
      self.cCopyLocation(defa, a)
      self.cStretchTo(defa, b)

    
    self.doVisit(chain2, bone_visit)
    self.doVisit(ctrlchain2, bone_visit) 

    return ctrlchain


```

## Source file: FacialAutoRigger/ui.py

{
  "path": "FacialAutoRigger/ui.py",
  "blob_sha": "c531bb43d80c0d50eb51f01f919c7f86474a8631",
  "sha256": "cee8c885e02669e45226330a90028c230918c7685e02c5607bcd5fa69ca6fc9c",
  "lines": 115,
  "bytes": 3244,
  "selection_score": 3,
  "keyword_line_counts": {
    "teeth": 5,
    "rig": 29,
    "face": 29
  },
  "url": "https://github.com/joeedh/FacialAutoRigger/blob/94ddf2a8967012724ae0c155fd6390a7ad81093c/FacialAutoRigger/ui.py",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "def poll(cls, context):"
    },
    {
      "line": 13,
      "text": "def drawSwappableList(layout, context, prop):"
    },
    {
      "line": 63,
      "text": "def draw(self, context):"
    },
    {
      "line": 100,
      "text": "def draw(self, context):"
    }
  ]
}

```text

import bpy
from bpy.props import *

class ArmaturePanel (bpy.types.Panel):
    bl_space_type = 'PROPERTIES'
    bl_region_type = 'WINDOW'
    bl_context = "data"
    
    @classmethod
    def poll(cls, context):
        return context.armature

def drawSwappableList(layout, context, prop):
  ob = context.object
  arm = context.armature
  space = context.space_data
  
  facerig = arm.facerig
  
  col = layout.column()
    
  row = col.row()
  row.alignment = "LEFT"

  op = row.operator("object.facerig_add_swappable_mesh_group", text="+")
  op.path = "facerig." + prop
  
  for i, item in enumerate(getattr(facerig, prop).groups):
    #print("ITEM", item.objects, prop)
    
    row = col.row(align=True)
    row.alignment = "LEFT"
    row.prop(item, "name", text="")

    box = col.box()

    op = row.operator("object.facerig_add_swappable_mesh", text="+")
    op.path = "facerig." + prop + ".groups[%i].objects[0]" % (i)
    
    op = row.operator("object.facerig_rem_swappable_mesh_group", text="x")
    op.path = "facerig." + prop + ".groups[%i]" % (i)
    
    col2 = box.column()
    
    for j, item2 in enumerate(item.objects):
      row2 = col2.row()
      row2.alignment = "LEFT"
      row2.prop(item2, "object", text="")
      #row2.label(text="-")
      #row2.label(text="^")
      #row2.label(text="v")
      
      op = row2.operator("object.facerig_rem_swappable_mesh", text="-")
      op.path = "facerig." + prop + ".groups[%i].objects[%i]" % (i, j)
      
    #layout.prop(item, "object"
  
class DATA_PT_FaceRigPanel(ArmaturePanel):
    """Creates a Panel in the scene context of the properties editor"""
    bl_label = "Face Auto Rigger"

    
    def draw(self, context):
      layout = self.layout
      
      ob = context.object
      arm = context.armature
      space = context.space_data

      ob = context.object
      arm = context.armature
      space = context.space_data
      
      facerig = arm.facerig
      
      col = layout.column()
      
      col.prop(arm.facerig, "meshob", text="Model");
      col.prop(arm.facerig, "rigname", text="Rig Name");
      col.prop(arm.facerig, "devmode", text="DevMode");
      
      op = col.operator("object.facerig_gen_shapekey_rig");
      op = col.operator("object.facerig_gen_shapekeys");
      op = col.operator("object.facerig_gen_vgroups");
      op = col.operator("object.facerig_update_final");  
      op = col.operator("object.facerig_make_shape_drivers")

      op = col.operator("object.facerig_make_swap_drivers", text="Make Swap Drivers")
      op.path = "facerig.teeth_models"
    
      box = layout.box()
      box.label(text="Teeth Swap Meshes")
      drawSwappableList(box, context, "teeth_models")
      
  
class DATA_PT_FaceRigCtrl(ArmaturePanel):
    """Creates a Panel in the scene context of the properties editor"""
    bl_label = "Face Auto Rigger"

    def draw(self, context):
      layout = self.layout
      
      ob = context.object
      arm = context.armature
      space = context.space_data
    
      layout.prop(arm.facerig.teeth_models, "active")
      layout.prop(arm.facerig.teeth_models, "show_all")
      
      
from .utils import Registrar
bpy_exports = Registrar([
  DATA_PT_FaceRigPanel,
  DATA_PT_FaceRigCtrl
])


```

## Source file: FacialAutoRigger/ops.py

{
  "path": "FacialAutoRigger/ops.py",
  "blob_sha": "94347d0bfa6cd6ab0d7be9bda6f6e6b2dda12ec7",
  "sha256": "6f2043020baa209bd7e339cd31fe8a146b3296459e3ba6af7d49da0808984fdb",
  "lines": 340,
  "bytes": 8633,
  "selection_score": 3,
  "keyword_line_counts": {
    "rig": 39,
    "face": 26,
    "lip": 6
  },
  "url": "https://github.com/joeedh/FacialAutoRigger/blob/94ddf2a8967012724ae0c155fd6390a7ad81093c/FacialAutoRigger/ops.py",
  "function_signature_candidates": [
    {
      "line": 21,
      "text": "def poll(cls, context):"
    },
    {
      "line": 24,
      "text": "def execute(self, context):"
    },
    {
      "line": 55,
      "text": "def poll(cls, context):"
    },
    {
      "line": 58,
      "text": "def execute(self, context):"
    },
    {
      "line": 89,
      "text": "def poll(cls, context):"
    },
    {
      "line": 92,
      "text": "def execute(self, context):"
    },
    {
      "line": 115,
      "text": "def poll(cls, context):"
    },
    {
      "line": 118,
      "text": "def execute(self, context):"
    },
    {
      "line": 123,
      "text": "def makeSwappableDriver(basepath, mswap, arm, obj, ctrlpath, i):"
    },
    {
      "line": 126,
      "text": "def getFCurve(path):"
    },
    {
      "line": 172,
      "text": "def makeSwappableDrivers(basepath, mswap, arm):"
    },
    {
      "line": 193,
      "text": "def poll(cls, context):"
    },
    {
      "line": 196,
      "text": "def execute(self, context):"
    },
    {
      "line": 221,
      "text": "def poll(cls, context):"
    },
    {
      "line": 224,
      "text": "def execute(self, context):"
    },
    {
      "line": 251,
      "text": "def poll(cls, context):"
    },
    {
      "line": 254,
      "text": "def execute(self, context):"
    },
    {
      "line": 270,
      "text": "def poll(cls, context):"
    },
    {
      "line": 273,
      "text": "def execute(self, context):"
    },
    {
      "line": 295,
      "text": "def poll(cls, context):"
    },
    {
      "line": 298,
      "text": "def execute(self, context):"
    },
    {
      "line": 318,
      "text": "def poll(cls, context):"
    },
    {
      "line": 321,
      "text": "def execute(self, context):"
    }
  ]
}

```text

import bpy
from bpy.props import *

from .utils import Registrar, decompose_path
import re

from .shapedrivers import makeShapeDrivers
from . import rigger
from . import shapekeys

class AddSwappableMesh(bpy.types.Operator):
    """Tooltip"""
    bl_idname = "object.facerig_add_swappable_mesh"
    bl_label = "Internal Face Rigger Operator"
    bl_options = {'UNDO'}

    #path to entry to insert before
    path: StringProperty()
    
    @classmethod
    def poll(cls, context):
        return context.armature

    def execute(self, context):
        print("PATHS", self.path)
        
        arm = context.armature
        trace, values = decompose_path(arm, self.path)
        mset = values[-3]
        index = int(trace[-1])
        
        lst = [obj.object for obj in mset.objects]
        lst.insert(index, None)
        
        mset.objects.clear()
        for l in lst:
          mset.objects.add()
          mset.objects[-1].object = l
          
        
        #print(trace, values)
        
        return {'FINISHED'}

class RemSwappableMesh(bpy.types.Operator):
    """Tooltip"""
    bl_idname = "object.facerig_rem_swappable_mesh"
    bl_label = "Internal Face Rigger Operator"
    bl_options = {'UNDO'}

    #path to entry to insert before
    path: StringProperty()
    
    @classmethod
    def poll(cls, context):
        return context.armature

    def execute(self, context):
        print("PATHS", self.path)
        
        arm = context.armature
        trace, values = decompose_path(arm, self.path)
        
        mset = values[-3]
        index = int(trace[-1])
        
        lst = [obj.object for obj in mset.objects]
        lst.pop(index)
        
        mset.objects.clear()
        for l in lst:
          mset.objects.add()
          mset.objects[-1].object = l
          
        
        #print(trace, values)
        
        return {'FINISHED'}

class AddSwappableMeshGroup(bpy.types.Operator):
    """Tooltip"""
    bl_idname = "object.facerig_add_swappable_mesh_group"
    bl_label = "Internal Face Rigger Operator"
    bl_options = {'UNDO'}

    path: StringProperty()
    
    @classmethod
    def poll(cls, context):
        return context.active_object is not None

    def execute(self, context):
        print("PATH", self.path)
        
        arm = context.armature
        trace, values = decompose_path(arm, self.path)
          
        print(trace, values)
        
        lst = values[-1]
        group = lst.groups.add()
        group.name = "unnamed"
        
        return {'FINISHED'}

class RemSwappableMeshGroup(bpy.types.Operator):
    """Tooltip"""
    bl_idname = "object.facerig_rem_swappable_mesh_group"
    bl_label = "Internal Face Rigger Operator"
    bl_options = {'UNDO'}
    
    path: StringProperty()
    
    @classmethod
    def poll(cls, context):
        return context.armature is not None

    def execute(self, context):
        print("PATHS", self.path)
        
        return {'FINISHED'}

def makeSwappableDriver(basepath, mswap, arm, obj, ctrlpath, i):
  anim = obj.animation_data
  
  def getFCurve(path):
    for d in anim.drivers:
      if d.data_path == ctrlpath:
        return d
    
    return anim.drivers.new(ctrlpath)
    
  path = basepath + ".active"
  print("yay")
  print(basepath)
  print(path)
  print(ctrlpath)
  
  fc = getFCurve(path)
  #if fc.driver is None:
  #  fc.driver_add()
  
  driver = fc.driver
  
  vars = driver.variables
  for j in range(len(vars)):
    vars.remove(vars[0])
  
  showall = vars.new()
  showall.name = "show_all"
  showall.type = "SINGLE_PROP"
  showall = showall.targets[0]
  
  active = vars.new()
  active.name = "active"
  active.type = "SINGLE_PROP"
  active = active.targets[0]
  
  showall.id_type = "ARMATURE"
  showall.id = arm
  active.id_type = "ARMATURE"
  active.id = arm
  
  showall.data_path = basepath + ".show_all"
  active.data_path = basepath + ".active"
  
  driver.type = "SCRIPTED"
  driver.expression = "active != %i and not show_all" % (i)
  
  print(fc)
  
def makeSwappableDrivers(basepath, mswap, arm):
  for i, group in enumerate(mswap.groups):
    for ob in group.objects:
      ob = ob.object
      
      if ob.animation_data is None:
        ob.animation_data_create()
      
      makeSwappableDriver(basepath, mswap, arm, ob, "hide_viewport", i)
      makeSwappableDriver(basepath, mswap, arm, ob, "hide_render", i)
      

class MakeSwappableMeshDrivers(bpy.types.Operator):
    """Tooltip"""
    bl_idname = "object.facerig_make_swap_drivers"
    bl_label = "Internal Face Rigger Operator"
    bl_options = {'UNDO'}
    
    path: StringProperty()
    
    @classmethod
    def poll(cls, context):
        return context.armature is not None

    def execute(self, context):
        print("PATHS", self.path)
        
        arm = context.armature
        
        trace, values = decompose_path(arm, self.path)
        mswap = values[-1]
        
        print(trace, values)
        print("making drivers!", mswap)
        makeSwappableDrivers("facerig." + trace[-1], mswap, arm)
        
        return {'FINISHED'}
    

class MakeShapeDrivers(bpy.types.Operator):
    """Tooltip"""
    bl_idname = "object.facerig_make_shape_drivers"
    bl_label = "Make Shape Drivers"
    bl_options = {'UNDO'}
    
    path: StringProperty()
    flipLeftRight : BoolProperty()
    
    @classmethod
    def poll(cls, context):
        return context.armature is not None

    def execute(self, context):
        print("PATHS", self.path)
        
        arm = context.armature
        armob = bpy.context.active_object
        
        print("making drivers!", context.active_object, context.armature)
        makeShapeDrivers(armob, -1.0 if self.flipLeftRight else 1.0);
        
        #trace, values = decompose_path(arm, self.path)
        #mswap = values[-1]
        
        #print(trace, values)
        
        return {'FINISHED'}
    

class GenerateShapeKeyRig(bpy.types.Operator):
    """Tooltip"""
    bl_idname = "object.facerig_gen_shapekey_rig"
    bl_label = "Make ShapeKey Rig"
    bl_options = {'UNDO'}
    
    path: StringProperty()
    flipLeftRight : BoolProperty()
    
    @classmethod
    def poll(cls, context):
      return context.armature is not None and context.armature.facerig.meshob is not None
      
    def execute(self, context):
      facerig = context.armature.facerig
      rigger.generateShapeKeyRig(context.scene, context.active_object, facerig.meshob, facerig.rigname)
      
      return {'FINISHED'}
   
class GenerateShapeKeys(bpy.types.Operator):
    """Tooltip"""
    bl_idname = "object.facerig_gen_shapekeys"
    bl_label = "Generate ShapeKeys"
    bl_options = {'UNDO'}
    
    path: StringProperty()
    flipLeftRight : BoolProperty()
    
    @classmethod
    def poll(cls, context):
        return context.armature is not None

    def execute(self, context):
      facerig = context.armature.facerig
      scene = context.scene 

      dgraph = context.evaluated_depsgraph_get()
      shapekeys.generate_shapekey_masks(facerig.meshob, dgraph, scene)
      shapekeys.generate_facial_rig(facerig.meshob, dgraph, scene)
            
        
      return {'FINISHED'}
    
   
class GenerateVGroups(bpy.types.Operator):
    """Generate Vertex Groups For Shapekeys"""
    bl_idname = "object.facerig_gen_vgroups"
    bl_label = "Generate Vertex Groups"
    bl_options = {'UNDO'}
    
    path: StringProperty()
    flipLeftRight : BoolProperty()
    
    @classmethod
    def poll(cls, context):
        return context.armature is not None

    def execute(self, context):
      facerig = context.armature.facerig
      scene = context.scene 

      dgraph = context.evaluated_depsgraph_get()
      shapekeys.generate_shapekey_masks(facerig.meshob, dgraph, scene)
        
      return {'FINISHED'}
    
   
class UpdateFinalRig(bpy.types.Operator):
    """Tooltip"""
    bl_idname = "object.facerig_update_final"
    bl_label = "Make/Update Final Rig"
    bl_options = {'UNDO'}
    
    path: StringProperty()
    flipLeftRight : BoolProperty()
    
    @classmethod
    def poll(cls, context):
        return context.armature is not None

    def execute(self, context):
      facerig = context.armature.facerig
      rigger.generate(context.scene, context.active_object, facerig.meshob, facerig.rigname)
        
      return {'FINISHED'}
    
   

bpy_exports = Registrar([
  AddSwappableMesh,
  RemSwappableMesh,
  AddSwappableMeshGroup,
  RemSwappableMeshGroup,
  MakeSwappableMeshDrivers,
  MakeShapeDrivers,
  UpdateFinalRig,
  GenerateShapeKeys,
  GenerateShapeKeyRig,
  GenerateVGroups
])


```
