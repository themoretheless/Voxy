#!/usr/bin/env python3
"""Collect pinned implementation evidence. Collection is not manual review."""
import concurrent.futures, hashlib, json, pathlib, re, subprocess, urllib.parse, urllib.request
ROOT=pathlib.Path(__file__).resolve().parents[2]
OUT=ROOT/'docs/hardware-research'
REPOS='''godotengine/godot
bevyengine/bevy
FyroxEngine/Fyrox
PistonDevelopers/piston
ggez/ggez
not-fl3/macroquad
not-fl3/miniquad
hecrj/iced
bkaradzic/bgfx
bkaradzic/bx
floooh/sokol
raylib-extras/rlImGui
raysan5/raylib
urho3d/Urho3D
rbfx/rbfx
OGRECave/ogre
OGRECave/ogre-next
DiligentGraphics/DiligentCore
DiligentGraphics/DiligentEngine
ConfettiFX/The-Forge
google/filament
WickedEngine/WickedEngine
TheCherno/Hazel
nem0/LumixEngine
FlaxEngine/FlaxEngine
stride3d/stride
defold/defold
cocos/cocos-engine
cocos2d/cocos2d-x
BabylonJS/Babylon.js
mrdoob/three.js
playcanvas/engine
pixijs/pixijs
phaserjs/phaser
pmndrs/react-three-fiber
melonjs/melonJS
armory3d/armory
blender/blender
OpenMW/openmw
luanti-org/luanti
veloren/veloren
sp4cerat/Fast-Quadric-Mesh-Simplification
zeux/meshoptimizer
zeux/niagara
GameTechDev/Intel-Extensions-for-Vulkan
GPUOpen-LibrariesAndSDKs/VulkanMemoryAllocator
GPUOpen-LibrariesAndSDKs/D3D12MemoryAllocator
GPUOpen-Effects/FidelityFX-SDK
GPUOpen-LibrariesAndSDKs/Cauldron
GPUOpen-Effects/FidelityFX-FSR2
GPUOpen-Effects/FidelityFX-FSR
GPUOpen-Effects/FidelityFX-CAS
NVIDIAGameWorks/donut
NVIDIAGameWorks/nvrhi
NVIDIAGameWorks/Falcor
NVIDIA-RTX/RTXDI
NVIDIA-RTX/NRD
NVIDIA-RTX/Streamline
NVIDIA/cuda-samples
NVIDIA/warp
NVIDIA/PhysX
NVIDIAGameWorks/Flow
NVIDIAGameWorks/Blast
NVIDIA/cutlass
NVIDIA/thrust
NVIDIA/cub
coreylowman/cudarc
EmbarkStudios/rust-gpu
gfx-rs/wgpu
gfx-rs/naga
gfx-rs/gfx
ash-rs/ash
vulkano-rs/vulkano
KhronosGroup/Vulkan-Samples
KhronosGroup/Vulkan-Tools
KhronosGroup/Vulkan-ValidationLayers
KhronosGroup/SPIRV-Tools
KhronosGroup/glslang
KhronosGroup/SPIRV-Cross
KhronosGroup/OpenCL-SDK
KhronosGroup/OpenXR-SDK-Source
microsoft/DirectX-Graphics-Samples
microsoft/DirectXShaderCompiler
microsoft/DirectXTK12
microsoft/DirectXTex
microsoft/DirectXMesh
google/dawn
google/shaderc
google/angle
sdl-org/SDL
glfw/glfw
libsdl-org/SDL_image
SaschaWillems/Vulkan
nvpro-samples/nvpro_core
nvpro-samples/vk_raytracing_tutorial_KHR
nvpro-samples/vk_compute_mipmaps
nvpro-samples/gl_cuda_interop_pingpong_st
baldurk/renderdoc
wolfpld/tracy
jk-ander/earcut
jrouwe/JoltPhysics
bulletphysics/bullet3
dimforge/rapier
dimforge/parry
needle-mirror/com.unity.render-pipelines.core
needle-mirror/com.unity.render-pipelines.universal
Unity-Technologies/Graphics
Unity-Technologies/EntityComponentSystemSamples
Unity-Technologies/arfoundation-samples
v1mm/voxel-rs
DavidGoldberg/raytracer
karimnaaji/voxelizer
microsoft/DirectXTK
walterpie/ldtk_rust
SaschaWillems/Vulkan-glTF-PBR
nvpro-samples/vk_mini_samples
nvpro-samples/vk_raytrace
nvpro-samples/vk_raytracing_tutorial
NVIDIA-RTX/Path-Tracing-SDK
NVIDIA-RTX/RTXGI-DDGI
NVIDIA-RTX/DLSS
KhronosGroup/Vulkan-Hpp
KhronosGroup/MoltenVK
KhronosGroup/OpenCL-ICD-Loader
EmbarkStudios/puffin
Traverse-Research/gpu-allocator
Traverse-Research/rust-gpu
Traverse-Research/buffet
sebastianstarke/AI4Animation
DiligentGraphics/DiligentSamples
google/graphicsfuzz
google/clspv
shader-slang/slang
shader-slang/slang-rhi
IntelRealSense/librealsense
chinedufn/webgl-water
clayjohn/godot-volumetric-cloud-demo-v2
Scthe/WebGPU-Sponza
toji/webgpu-bundle
jagenjo/litescene.js
Twinklebear/webgpu-volume-raycaster
microsoft/WebGPU-D3D12'''.splitlines()
PAT=re.compile(r'(allocat|residen|barrier|semaphore|fence|indirect|dispatch|compute|cuda|shader|stream|device|render)',re.I)
EXT={'.rs','.cpp','.cc','.c','.h','.hpp','.cu','.wgsl','.glsl','.hlsl','.cs','.ts','.js','.metal'}
def api(endpoint):
 r=subprocess.run(['gh','api',endpoint],capture_output=True,text=True,timeout=90)
 if r.returncode: raise RuntimeError(r.stderr[-220:])
 return json.loads(r.stdout)
def collect(name):
 d=ROOT/'target/hardware-research/sources'/name.replace('/','__'); record=d/'record.json'
 if record.exists() and json.loads(record.read_text()).get('selection_version') == 2: return json.loads(record.read_text())
 try:
  previous=json.loads(record.read_text()) if record.exists() else None
  if previous: sha=previous['commit']
  else:
   meta=api(f'repos/{name}'); sha=api(f'repos/{name}/commits/{urllib.parse.quote(meta["default_branch"],safe="")}')['sha']
  tree=api(f'repos/{name}/git/trees/{sha}?recursive=1')
  files=[x for x in tree['tree'] if x['type']=='blob' and pathlib.PurePosixPath(x['path']).suffix in EXT and 300<=x.get('size',0)<=120000 and PAT.search(x['path']) and not re.search(r'(third_party|third-party|thirdparty|3rdparty|vendor/|external/|node_modules|test[s]?/|v8/|include/cppgc|bevy_platform|components/misc/)',x['path'],re.I)]
  def score(x):
   p=x['path'].lower(); return (sum(w in p for w in ['vulkan','dx12','d3d12','metal','opengl','render','graphics','gpu','cuda'])*15 + sum(w in p for w in ['allocat','residen','barrier','indirect','compute','device','render_graph','rendergraph'])*10 + ('src/' in p)*3 + (pathlib.PurePosixPath(p).suffix in {'.cpp','.cc','.c','.rs','.cu','.ts','.js','.cs'})*8 - ('mod.rs' in p or 'index.ts' in p or 'pure.ts' in p)*15, -len(p))
  files.sort(key=score,reverse=True); evidence=[]
  for x in files[:3]:
   url=f'https://raw.githubusercontent.com/{name}/{sha}/{urllib.parse.quote(x["path"])}'
   with urllib.request.urlopen(url,timeout=45) as response: raw=response.read(120001)
   if len(raw)>120000: continue
   text=raw.decode('utf-8',errors='replace'); lines=text.splitlines()
   hits=[i for i,line in enumerate(lines) if re.search(r'(dispatch|barrier|allocate|alloc_buffer|create_buffer|createBuffer|vkCmd|vkAllocate|cuda[A-Z]|cu[A-Z]|fence|submit|drawIndirect|draw_indirect|request_device|create_device|createDevice|begin_render_pass|compile|createShader)', line, re.I) and not line.lstrip().startswith(('//','*','#include','import','use ','#define','export *'))]
   if not hits: hits=[i for i,line in enumerate(lines) if PAT.search(line) and not line.lstrip().startswith(('//','*','#include','import','use ','export *'))]
   if not hits: continue
   d.mkdir(parents=True,exist_ok=True); filename=f'{len(evidence)}{pathlib.Path(x["path"]).suffix}'; (d/filename).write_bytes(raw)
   selected=[]
   for i in hits:
    if len(selected)>=3: break
    if any(abs(i-j)<10 for j in selected): continue
    selected.append(i)
   evidence.append(dict(path=x['path'],sha256=hashlib.sha256(raw).hexdigest(),local=filename,url=f'https://github.com/{name}/blob/{sha}/{x["path"]}',lines=len(lines),excerpts=[dict(start=max(1,i-2),text='\n'.join(lines[max(0,i-3):i+7])) for i in selected]))
  if not evidence: raise RuntimeError('no selected implementation files')
  result=dict(selection_version=2,repository=name,commit=sha,tree_truncated=tree.get('truncated',False),status='collected_not_reviewed',files=evidence)
  d.mkdir(parents=True,exist_ok=True); record.write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n'); return result
 except Exception as e: return dict(repository=name,status='failed',error=str(e))
def main():
 OUT.mkdir(parents=True,exist_ok=True)
 results=[]
 with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
  for r in pool.map(collect,REPOS):
   results.append(r); print(r['status'],r['repository'],flush=True)
 (ROOT/'target/hardware-research/collection-records.json').write_text(json.dumps(results,ensure_ascii=False,indent=2)+'\n')
 (OUT/'collection-attempts.json').write_text(json.dumps([{k:r[k] for k in ['repository','commit','status','error'] if k in r} for r in results],ensure_ascii=False,indent=2)+'\n')
 print('COLLECTED',sum(r['status']!='failed' for r in results),'of',len(results),flush=True)
if __name__=='__main__':main()
