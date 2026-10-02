#!/usr/bin/env python3
"""Add explicitly selected algorithm files to the read-only fluid survey."""
import concurrent.futures
import hashlib
import json
import pathlib
import re
import urllib.parse
import urllib.request
import research_fluid_repositories as survey

FILES = {
    'leonardo-montes/Unity-ECS-Job-System-SPH': ['Assets/Job System/SPHSystem.cs'],
    'lukedan/libfluid': ['src/pressure_solver.cpp', 'src/simulation.cpp'],
    'robkau/mlsmpm-particles-rs': ['src/step_p2g.rs', 'src/step_g2p.rs'],
    'jinleili/fluid-webgpu': ['shader/lbm/d2q9_collide.comp.glsl'],
    'bwiberg/position-based-fluids': ['kernels/timestep.cl'],
    'matsuoka-601/Splash': ['render/narrowRangeFilter.wgsl', 'mls-mpm/p2g_2.wgsl', 'mls-mpm/g2p.wgsl'],
    'ttnghia/RealTimeFluidRendering': ['Shaders/filter-narrow-range.fs.glsl', 'Shaders/composition-pass.fs.glsl', 'Shaders/normal-pass.fs.glsl'],
    'rlguy/Blender-FLIP-Fluids': ['src/engine/diffuseparticlesimulation.cpp'],
    'rlguy/GridFluidSim3D': ['src/diffuseparticlesimulation.cpp', 'src/fluidsimulation.cpp'],
    'NVIDIAGameWorks/FleX': ['demo/d3d/shaders/ellipsoidDepthPS.hlsl', 'demo/d3d/shaders/blurDepthPS.hlsl', 'demo/d3d/shaders/compositePS.hlsl', 'demo/d3d/shaders/diffusePS.hlsl'],
    'Popov72/FluidRendering': ['src/scenes/FluidSimulator2/fluidSimulator.ts'],
    'dli/fluid': ['simulator.js', 'renderer.js'],
    'hadashiA/UnityURP-ScreenSpaceFluid': ['Assets/Scripts/SsfPass.cs'],
    'turanszkij/WickedEngine': ['WickedEngine/shaders/surfaceHF.hlsli', 'WickedEngine/shaders/emittedparticle_sphdensityCS.hlsl', 'WickedEngine/shaders/emittedparticle_sphforceCS.hlsl'],
    'ray-cast/ray-mmd': ['Materials/Programmable/Wetness/material_common.fxsub', 'Materials/Programmable/Wetness/material_functions.fxsub'],
    'smkplus/WetShader': ['Assets/Shaders/WetShader 1.shader', 'Assets/Shaders/WetShader 2.shader'],
}

def fetch_file(repo, path):
    url = f"https://raw.githubusercontent.com/{repo['repository']}/{repo['commit']}/{urllib.parse.quote(path)}"
    request = urllib.request.Request(url, headers={'User-Agent': 'Voxy-source-survey'})
    with urllib.request.urlopen(request, timeout=60) as response:
        raw = response.read(1500001)
    if len(raw) > 1500000:
        raise ValueError('selected source exceeds limit')
    local = survey.CACHE / repo['repository'].replace('/', '__') / path
    local.parent.mkdir(parents=True, exist_ok=True)
    local.write_bytes(raw)
    lines = raw.decode('utf-8', errors='replace').splitlines()
    signals = {}
    for tag, pattern in {**survey.TERMS, 'material roughness': r'roughness|smoothness|glossiness|wetness'}.items():
        hits = [i for i, line in enumerate(lines, 1) if re.search(pattern, line, re.I)]
        if hits:
            signals[tag] = hits[:8]
    return {'path': path, 'bytes': len(raw), 'lines': len(lines),
            'sha256': hashlib.sha256(raw).hexdigest(),
            'url': f"https://github.com/{repo['repository']}/blob/{repo['commit']}/{urllib.parse.quote(path)}",
            'signals': signals, 'cache_path': str(local), 'selection': 'explicit algorithm selection'}

def main():
    manifest_path = survey.OUT / 'manifest.json'
    manifest = json.loads(manifest_path.read_text())
    by_name = {r['repository']: r for r in manifest['repositories']}
    for name in ['smkplus/WetShader', 'ray-cast/ray-mmd']:
        by_name[name] = json.loads((survey.CACHE / name.replace('/', '__') / 'inspection.json').read_text())
    item = survey.gh('repos/rinafumoto/FluidSimulation')
    by_name[item['full_name']] = survey.inspect(item, True)
    with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
        tasks = {pool.submit(fetch_file, by_name[name], path): (name, path)
                 for name, paths in FILES.items() for path in paths}
        for future in concurrent.futures.as_completed(tasks):
            name, path = tasks[future]
            source = future.result()
            repo = by_name[name]
            repo['sources'] = [s for s in repo['sources'] if s['path'] != path] + [source]
            repo['usable'] = True
            print('read', name, path, source['lines'], flush=True)
    manifest['repositories'] = list(by_name.values())
    manifest['count'] = len(by_name)
    manifest_path.write_text(json.dumps(manifest, indent=2, ensure_ascii=False))
    print('ENRICHED SURVEY:', len(by_name), flush=True)

if __name__ == '__main__':
    main()
