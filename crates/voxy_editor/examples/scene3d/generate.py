"""Regenerate the original, embedded-texture static GLB acceptance fixture."""
import json, struct, zlib, math
from pathlib import Path
root = Path(__file__).parent
# Six independent faces preserve UV seams; lighting uses world-space face derivatives.
positions, uvs, indices = [], [], []
for face in [
    [(-1,-1,1),(1,-1,1),(1,1,1),(-1,1,1)],
    [(1,-1,-1),(-1,-1,-1),(-1,1,-1),(1,1,-1)],
    [(-1,-1,-1),(-1,-1,1),(-1,1,1),(-1,1,-1)],
    [(1,-1,1),(1,-1,-1),(1,1,-1),(1,1,1)],
    [(-1,1,1),(1,1,1),(1,1,-1),(-1,1,-1)],
    [(-1,-1,-1),(1,-1,-1),(1,-1,1),(-1,-1,1)],
]:
    base = len(positions)
    positions.extend(face); uvs.extend([(0,0),(1,0),(1,1),(0,1)])
    indices.extend(base+i for i in [0,1,2,0,2,3])
def chunk(kind, data):
    return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind+data))
rows = b''.join(b'\0' + b''.join(bytes((230,130,40,255) if (x+y)%2 else (30,120,200,255)) for x in range(4)) for y in range(4))
png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB',4,4,8,6,0,0,0)) + chunk(b'IDAT',zlib.compress(rows)) + chunk(b'IEND',b'')
parts = [b''.join(struct.pack('<3f',*v) for v in positions), b''.join(struct.pack('<2f',*uv) for uv in uvs), struct.pack('<36H',*indices),png]
binary = b''; views=[]
for part in parts:
    binary += b'\0' * (-len(binary)%4)
    views.append({'buffer':0,'byteOffset':len(binary),'byteLength':len(part)})
    binary += part
spec = {'asset':{'version':'2.0','generator':'Voxy original fixture'},'buffers':[{'byteLength':len(binary)}], 'bufferViews':views,
    'accessors':[{'bufferView':0,'componentType':5126,'count':24,'type':'VEC3','min':[-1,-1,-1],'max':[1,1,1]}, {'bufferView':1,'componentType':5126,'count':24,'type':'VEC2'}, {'bufferView':2,'componentType':5123,'count':36,'type':'SCALAR'}],
    'images':[{'bufferView':3,'mimeType':'image/png'}], 'textures':[{'source':0}], 'materials':[{'pbrMetallicRoughness':{'baseColorTexture':{'index':0},'metallicFactor':0,'roughnessFactor':1}}],
    'meshes':[{'primitives':[{'attributes':{'POSITION':0,'TEXCOORD_0':1},'indices':2,'material':0}]}],
    'nodes':[{'name':'Assembly','children':[1,2]}, {'name':'Textured cube','mesh':0,'translation':[0,-0.35,0.5],'scale':[0.1,0.1,0.1],'rotation':[0,math.sin(0.3),0,math.cos(0.3)]}, {'name':'Second cube','mesh':0,'translation':[0.35,-0.38,0.55],'scale':[0.07,0.07,0.07]}], 'scenes':[{'nodes':[0]}], 'scene':0}
encoded = json.dumps(spec,separators=(',',':')).encode(); encoded += b' ' * (-len(encoded)%4)
binary += b'\0' * (-len(binary)%4)
glb = struct.pack('<4sII',b'glTF',2,12+8+len(encoded)+8+len(binary)) + struct.pack('<I4s',len(encoded),b'JSON') + encoded + struct.pack('<I4s',len(binary),b'BIN\0') + binary
(root/'assembly.glb').write_bytes(glb)
