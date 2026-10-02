import bpy,math,re,json,pathlib
from mathutils import Vector
ROOT=pathlib.Path('/Users/themoretheless/Documents/ChatGPT/Voxy'); OUT=ROOT/'target/hand-rig-rest-audit';OUT.mkdir(exist_ok=True)
bpy.ops.object.select_all(action='SELECT');bpy.ops.object.delete(use_global=False)
src=(ROOT/'crates/voxy_app/src/female_rig.rs').read_text();block=src.split('const FINGER_CHAINS:')[1].split('fn finger_clearance')[0]
pts=[list(map(float,m)) for m in re.findall(r'\[\s*([-\d.]+),\s*([-\d.]+),\s*([-\d.]+)\s*\]',block)];assert len(pts)==20
chains=[pts[i:i+4] for i in range(0,20,4)]
v=[];faces=[]
for l in (ROOT/'assets/characters/blender-female/body.obj').read_text().splitlines():
 if l.startswith('v '):v.append(tuple(map(float,l.split()[1:4])))
 elif l.startswith('f '):
  ids=[int(s.split('/')[0])-1 for s in l.split()[1:]]
  if all(v[i][0]>.315 and -.165<v[i][1]<.07 for i in ids):
   for j in range(1,len(ids)-1):faces.append((ids[0],ids[j],ids[j+1]))
used=sorted(set(i for f in faces for i in f));mapping={i:j for j,i in enumerate(used)};mesh=bpy.data.meshes.new('Actual source hand');mesh.from_pydata([v[i] for i in used],[],[tuple(mapping[i] for i in f) for f in faces]);mesh.update();obj=bpy.data.objects.new('Source hand, unchanged',mesh);bpy.context.collection.objects.link(obj)
def mat(name,color):
 m=bpy.data.materials.new(name);m.diffuse_color=(*color,1);return m
skin=mat('Neutral diagnostic',(0.43,.43,.43));obj.data.materials.append(skin)
for p in mesh.polygons:p.use_smooth=True
colors=[(1,.25,.1),(.2,.7,1),(.1,1,.4),(.9,.4,1),(1,.85,.15)];audit=[];projected=[]
def line(a,b,r,material):
 a,b=Vector(a),Vector(b);d=b-a;bpy.ops.mesh.primitive_cylinder_add(vertices=12,radius=r,depth=d.length,location=(a+b)/2);o=bpy.context.object;o.rotation_euler=d.to_track_quat('Z','Y').to_euler();o.data.materials.append(material);projected.append(o)
def dot(p,r,material):
 bpy.ops.mesh.primitive_uv_sphere_add(segments=12,ring_count=8,radius=r,location=p);o=bpy.context.object;o.data.materials.append(material);projected.append(o)
bpy.ops.object.camera_add(location=(.85,-.045,.075));cam=bpy.context.object;cam.rotation_euler=(Vector((.36,-.045,.075))-cam.location).to_track_quat('-Z','Y').to_euler();cam.data.type='ORTHO';cam.data.ortho_scale=.26;bpy.context.scene.camera=cam
scene=bpy.context.scene;scene.render.engine='BLENDER_WORKBENCH';scene.display.shading.light='STUDIO';scene.display.shading.color_type='MATERIAL';scene.display.shading.show_shadows=False;scene.display.shading.show_cavity=True;scene.display.shading.background_type='WORLD';scene.world.color=(.035,.035,.035);scene.render.resolution_x=900;scene.render.resolution_y=1000;scene.render.resolution_percentage=100

OUT=ROOT/'target/hand-bone-heat-audit';OUT.mkdir(exist_ok=True)
# Heat solve uses centimeter-sized geometry; export returns to original coordinates.
for vert in mesh.vertices:vert.co*=100
arm=bpy.data.armatures.new('Diagnostic hand skeleton');rig=bpy.data.objects.new('Diagnostic hand skeleton',arm);bpy.context.collection.objects.link(rig)
bpy.context.view_layer.objects.active=rig;rig.select_set(True);obj.select_set(False);bpy.ops.object.mode_set(mode='EDIT')
palm=arm.edit_bones.new('palm');palm.head=(34,-1,7);palm.tail=(38,-4,7)
for f,chain in enumerate(chains):
 parent=palm
 for j in range(3):
  b=arm.edit_bones.new('finger_%d_%d'%(f,j));b.head=Vector(chain[j])*100;b.tail=Vector(chain[j+1])*100;b.parent=parent;parent=b
bpy.ops.object.mode_set(mode='OBJECT');obj.select_set(True);bpy.context.view_layer.objects.active=rig
bpy.ops.object.parent_set(type='ARMATURE_AUTO')
mod=next(m for m in obj.modifiers if m.type=='ARMATURE');mod.use_deform_preserve_volume=True
rows=[]
for i,vert in enumerate(mesh.vertices):
 weights={obj.vertex_groups[g.group].name:g.weight for g in vert.groups}
 rows.append({'position':list(Vector(v[used[i]])),'weights':weights})
(OUT/'weights.json').write_text(json.dumps(rows))
# Return mesh/skeleton to model units without altering computed weights.
obj.parent=None;obj.matrix_world.identity()
for vert in mesh.vertices:vert.co/=100
bpy.context.view_layer.objects.active=rig;bpy.ops.object.mode_set(mode='EDIT')
for b in arm.edit_bones:b.head/=100;b.tail/=100
bpy.ops.object.mode_set(mode='OBJECT');mesh.update()
for degrees in [-30,30]:
 for p in rig.pose.bones:p.rotation_mode='QUATERNION';p.rotation_quaternion=(1,0,0,0)
 p=rig.pose.bones['finger_0_0'];axis=(Vector(chains[0][1])-Vector(chains[0][0])).cross(Vector((-1,0,0))).normalized()
 from mathutils import Quaternion
 p.rotation_quaternion=Quaternion(p.bone.matrix_local.to_3x3().inverted()@axis,math.radians(degrees))
 bpy.context.view_layer.update()
 for side,x in [('plus',.85),('minus',-.12)]:
  cam.location=(x,-.045,.075);cam.rotation_euler=(Vector((.36,-.045,.075))-cam.location).to_track_quat('-Z','Y').to_euler()
  scene.render.resolution_x=600;scene.render.resolution_y=700;scene.render.filepath=str(OUT/('thumb-root-%d-%s.png'%(degrees,side)));bpy.ops.render.render(write_still=True)
bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'diagnostic-heat-weights.blend'))
