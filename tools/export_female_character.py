"""Run with Blender --background --factory-startup --disable-autoexec --python ... -- SOURCE_BLEND OUTPUT_DIR.
Exports the Blender Studio CC0 realistic female body without executing asset scripts.
"""
import bpy,sys,os,json,math
from mathutils import Vector
from mathutils.bvhtree import BVHTree
args=sys.argv[sys.argv.index('--')+1:]
source,output=args[:2]
sex=args[2] if len(args)>2 else "female"
if sex not in ("female","male"):raise ValueError("sex must be female or male")
os.makedirs(output,exist_ok=True)
bpy.ops.wm.open_mainfile(filepath=source)
body=bpy.data.objects['GEO-body_'+sex+'_realistic']
for m in body.modifiers:
    if m.type=='MULTIRES':m.levels=0
bpy.context.view_layer.update()
deps=bpy.context.evaluated_depsgraph_get()
def mesh_for(obj):return bpy.data.meshes.new_from_object(obj.evaluated_get(deps),depsgraph=deps)
base=mesh_for(body)
base.calc_loop_triangles()
world=[body.matrix_world@v.co for v in base.vertices]
cx=0.5*(min(p.x for p in world)+max(p.x for p in world))
height=min(p.z for p in world)
scale=1.64/(max(p.z for p in world)-height) if sex=="male" else 1.
def convert(p):return Vector(((p.x-cx)*scale,(p.z-height)*scale-0.82,-p.y*scale))
def positions(obj,mesh):return [convert(obj.matrix_world@v.co) for v in mesh.vertices]
basepoints=positions(body,base)
# An anterior abdominal region of the actual anatomical surface, not a planar proxy.
selected=[]
for tri in base.loop_triangles:
    ids=tuple(tri.vertices);p=sum((basepoints[i] for i in ids),Vector())/3
    if abs(p.x)<0.13 and 0.08<p.y<0.32 and p.z>0.045:selected.append(ids)
used=sorted({i for t in selected for i in t});remap={i:j for j,i in enumerate(used)}
patchpoints=[basepoints[i] for i in used];patchtriangles=[tuple(remap[i] for i in t) for t in selected]
edges={}
for t in patchtriangles:
    for a,b in [(t[0],t[1]),(t[1],t[2]),(t[2],t[0])]:
        key=tuple(sorted((a,b)));edges[key]=edges.get(key,0)+1
pins=sorted({i for e,n in edges.items() if n==1 for i in e})
if len(patchpoints)<12 or len(patchtriangles)<12:raise RuntimeError('skin patch selection empty')
for m in body.modifiers:
    if m.type=='MULTIRES':m.levels=1
bpy.context.view_layer.update();deps=bpy.context.evaluated_depsgraph_get()
render=mesh_for(body);render.calc_loop_triangles()
renderpoints=positions(body,render)
def write_obj(name,obj,mesh,points):
    mesh.calc_loop_triangles()
    normal_matrix=obj.matrix_world.to_3x3().inverted().transposed()
    with open(os.path.join(output,name),'w') as f:
        f.write('# Blender Studio Human Base Meshes v1.4.1; CC0; Y-up metres\n')
        for p in points:f.write('v %.9g %.9g %.9g\n'%tuple(p))
        for v in mesh.vertices:
            n=(normal_matrix@v.normal).normalized();f.write('vn %.9g %.9g %.9g\n'%(n.x,n.z,-n.y))
        for t in mesh.loop_triangles:f.write('f '+' '.join('%d//%d'%(i+1,i+1) for i in t.vertices)+'\n')
write_obj('body.obj',body,render,renderpoints)
for label in ['L','R']:
    eye=bpy.data.objects['GEO-body_'+sex+'_realistic.eye.'+label];mesh=mesh_for(eye)
    write_obj('eye-'+label.lower()+'.obj',eye,mesh,positions(eye,mesh))
bvh=BVHTree.FromPolygons(patchpoints,patchtriangles,all_triangles=True)
def bary(point,tri):
    a,b,c=[patchpoints[i] for i in tri];v0=b-a;v1=c-a;v2=point-a
    d00=v0.dot(v0);d01=v0.dot(v1);d11=v1.dot(v1);d20=v2.dot(v0);d21=v2.dot(v1);den=d00*d11-d01*d01
    b=(d11*d20-d01*d21)/den;c=(d00*d21-d01*d20)/den
    weights=[1-b-c,b,c]
    if min(weights)<-1e-5 or max(weights)>1+1e-5:raise RuntimeError("invalid BVH barycentric binding")
    weights=[max(0.0,min(1.0,w)) for w in weights]
    total=sum(weights)
    return [w/total for w in weights]
bindings=[]
for vertex,p in enumerate(renderpoints):
    nearest,normal,face,distance=bvh.find_nearest(p)
    if distance is not None and distance<0.02:
        weights=bary(nearest,patchtriangles[face]);fade=max(0,1-distance/0.02)
        bindings.append({'vertex':vertex,'triangle':face,'weights':weights,'fade':fade,'position':list(p)})
data={'positions':[list(p) for p in patchpoints],'triangles':patchtriangles,'pins':pins,'bindings':bindings,'render_vertices':len(renderpoints)}
with open(os.path.join(output,'abdomen-skin.json'),'w') as f:json.dump(data,f,separators=(',',':'))
print(sex.upper()+' EXPORT',json.dumps({'body_vertices':len(renderpoints),'body_triangles':len(render.loop_triangles),'skin_vertices':len(patchpoints),'skin_triangles':len(patchtriangles),'pins':len(pins),'bindings':len(bindings)}))
