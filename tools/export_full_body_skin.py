"""Blender background export of a reduced full-body physics shell and render bindings.
Usage: Blender --background --factory-startup --disable-autoexec --python SCRIPT -- SOURCE_BLEND RENDER_OBJ OUTPUT_JSON
"""
import bpy
import json
import math
import sys
from mathutils import Vector
from mathutils.bvhtree import BVHTree
args=sys.argv[sys.argv.index('--') + 1:]
source, render_obj, output = args[:3]
sex=args[3] if len(args)>3 else 'female'
if sex not in ('female','male'):raise ValueError('sex must be female or male')
bpy.ops.wm.open_mainfile(filepath=source)
body = bpy.data.objects['GEO-body_'+sex+'_realistic']
for modifier in body.modifiers:
    if modifier.type == 'MULTIRES':
        modifier.levels = 0
bpy.context.view_layer.update()
deps = bpy.context.evaluated_depsgraph_get()
base = bpy.data.meshes.new_from_object(body.evaluated_get(deps), depsgraph=deps)
world = [body.matrix_world @ vertex.co for vertex in base.vertices]
cx = 0.5 * (min(p.x for p in world) + max(p.x for p in world))
floor = min(p.z for p in world)
scale=1.64/(max(p.z for p in world)-floor) if sex=="male" else 1.
# Copy the evaluated base; do not alter the source asset or high-resolution OBJ.
proxy = bpy.data.objects.new('FullBodySkinPhysics', base)
bpy.context.collection.objects.link(proxy)
proxy.matrix_world = body.matrix_world.copy()
modifier = proxy.modifiers.new('PhysicsResolution', 'DECIMATE')
modifier.ratio = 0.06
modifier.use_collapse_triangulate = True
bpy.context.view_layer.update()
deps = bpy.context.evaluated_depsgraph_get()
mesh = bpy.data.meshes.new_from_object(proxy.evaluated_get(deps), depsgraph=deps)
mesh.calc_loop_triangles()
def convert(point):
    return Vector(((point.x-cx)*scale,(point.z-floor)*scale-0.82,-point.y*scale))
points = [convert(proxy.matrix_world @ vertex.co) for vertex in mesh.vertices]
triangles = [tuple(triangle.vertices) for triangle in mesh.loop_triangles]
used = sorted({i for triangle in triangles for i in triangle})
remap = {old: new for new, old in enumerate(used)}
points = [points[i] for i in used]
triangles = [tuple(remap[i] for i in triangle) for triangle in triangles]
bvh = BVHTree.FromPolygons(points, triangles, all_triangles=True)
renderpoints = []
with open(render_obj) as file:
    for line in file:
        if line.startswith('v '):
            renderpoints.append(Vector(tuple(map(float, line.split()[1:]))))
def barycentric(point, triangle):
    a, b, c = [points[i] for i in triangle]
    u, v, w = b-a, c-a, point-a
    uu, uv, vv, wu, wv = u.dot(u), u.dot(v), v.dot(v), w.dot(u), w.dot(v)
    denominator = uu*vv-uv*uv
    if denominator <= 0:
        raise RuntimeError('degenerate shell binding')
    y = (vv*wu-uv*wv)/denominator
    z = (uu*wv-uv*wu)/denominator
    weights = [max(0., min(1., value)) for value in [1-y-z, y, z]]
    total = sum(weights)
    return [value/total for value in weights]
bindings = []
distances = []
for vertex, point in enumerate(renderpoints):
    nearest, normal, face, distance = bvh.find_nearest(point)
    if face is None or not math.isfinite(distance):
        raise RuntimeError('unbound render vertex')
    distances.append(distance)
    bindings.append({'vertex': vertex, 'triangle': face,
                     'weights': barycentric(nearest, triangles[face]),
                     'fade': 1., 'position': list(point)})
# Every triangle needs a tangential collagen reference axis. Body-up is a
# reproducible convention, not measured Langer directions; switch at horizontal faces.
directions = []
for triangle in triangles:
    a, b, c = [points[i] for i in triangle]
    normal = (b-a).cross(c-a).normalized()
    axis = Vector((0., 1., 0.))
    if abs(normal.dot(axis)) > 0.95:
        axis = Vector((1., 0., 0.))
    directions.append(list(axis))
data = {'positions': [list(p) for p in points], 'triangles': triangles,
        'pins': [], 'directions': directions, 'bindings': bindings,
        'render_vertices': len(renderpoints), 'coverage': 'full_body',
        'max_binding_distance_m': max(distances)}
with open(output, 'w') as file:
    json.dump(data, file, separators=(',', ':'))
print('FULL BODY SKIN', json.dumps({'vertices': len(points), 'triangles': len(triangles),
      'bindings': len(bindings), 'max_distance_m': max(distances)}))
