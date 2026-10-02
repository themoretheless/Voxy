import json,pathlib,subprocess,urllib.request,urllib.parse,hashlib
B=pathlib.Path(__file__).resolve().parents[2]/'target/hardware-research/sources'
pairs={
'GPUOpen-LibrariesAndSDKs/VulkanMemoryAllocator':['include/vk_mem_alloc.h'],
'GPUOpen-LibrariesAndSDKs/D3D12MemoryAllocator':['src/D3D12MemAlloc.cpp'],
'Traverse-Research/gpu-allocator':['src/vulkan/mod.rs'],
'mrdoob/three.js':['src/renderers/webgpu/WebGPUBackend.js'],
'NVIDIA-RTX/NRD':['Source/InstanceImpl.cpp'],
'NVIDIA/warp':['warp/native/cuda_util.h'],
'KhronosGroup/SPIRV-Cross':['spirv_glsl.cpp'],
}
for name,paths in pairs.items():
 d=B/name.replace('/','__'); p=d/'record.json'
 if p.exists():r=json.loads(p.read_text())
 else:
  def api(e):return json.loads(subprocess.check_output(['gh','api',e]))
  meta=api(f'repos/{name}');sha=api(f'repos/{name}/commits/{meta["default_branch"]}')['sha']
  r=dict(selection_version=2,repository=name,commit=sha,status='collected_not_reviewed',tree_truncated=False,files=[])
 for path in paths:
  if any(f['path']==path for f in r['files']):continue
  try:
   with urllib.request.urlopen(f'https://raw.githubusercontent.com/{name}/{r["commit"]}/{path}',timeout=45) as response:raw=response.read(4000001)
   if len(raw)>4000000:raise RuntimeError('oversize')
   d.mkdir(parents=True,exist_ok=True);local=f'deep-{len(r["files"])}{pathlib.Path(path).suffix}';(d/local).write_bytes(raw)
   r['files'].insert(0,dict(path=path,local=local,sha256=hashlib.sha256(raw).hexdigest(),url=f'https://github.com/{name}/blob/{r["commit"]}/{path}',lines=len(raw.splitlines()),excerpts=[]));p.write_text(json.dumps(r,indent=2)+'\n');print('OK',name,path,flush=True)
  except Exception as e:print('FAIL',name,path,e,flush=True)
