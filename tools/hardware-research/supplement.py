import hashlib,json,pathlib,urllib.request,urllib.parse
BASE=pathlib.Path(__file__).resolve().parents[2]/'target/hardware-research/sources'
PAIRS={
'NVIDIA/cuda-samples':['cpp/5_Domain_Specific/simpleVulkan/main.cpp','cpp/5_Domain_Specific/simpleVulkan/SineWaveSimulation.cu'],
'godotengine/godot':['servers/rendering/rendering_device_graph.cpp'],
'gfx-rs/wgpu':['wgpu-core/src/device/life.rs'],
'google/filament':['filament/backend/src/vulkan/VulkanCommands.cpp'],
'bevyengine/bevy':['crates/bevy_render/src/gpu_readback.rs'],
'bkaradzic/bgfx':['examples/37-gpudrivenrendering/gpudrivenrendering.cpp'],
}
for name,paths in PAIRS.items():
 d=BASE/name.replace('/','__'); p=d/'record.json'; r=json.loads(p.read_text())
 for path in paths:
  if any(f['path']==path for f in r['files']):continue
  url=f'https://raw.githubusercontent.com/{name}/{r["commit"]}/{urllib.parse.quote(path)}'
  with urllib.request.urlopen(url,timeout=45) as response: raw=response.read(2000000)
  local=f'supplement-{len(r["files"])}{pathlib.Path(path).suffix}';(d/local).write_bytes(raw)
  r['files'].insert(0,dict(path=path,local=local,sha256=hashlib.sha256(raw).hexdigest(),url=f'https://github.com/{name}/blob/{r["commit"]}/{path}',lines=len(raw.splitlines()),excerpts=[]))
 p.write_text(json.dumps(r,ensure_ascii=False,indent=2)+'\n')
 print(name,flush=True)
