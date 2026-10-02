#!/usr/bin/env python3
"""Fetch reproducible, read-only source evidence for the fluid/VFX survey.

Does not execute third-party code. Raw sources remain in a temporary cache;
the checked-in manifest contains hashes, locations and inspection limits.
"""
import concurrent.futures
import hashlib
import json
import pathlib
import re
import subprocess
import sys
import time
import urllib.request

CACHE = pathlib.Path('/tmp/voxy-fluid-research')
OUT = pathlib.Path(__file__).resolve().parents[1] / 'docs/research/fluid-repositories-100'
QUERIES = [
    ('sph', 'fluid simulation SPH fork:false'),
    ('flip', 'fluid simulation FLIP fork:false'),
    ('pbf', 'position based fluids fork:false'),
    ('render', 'screen space fluid fork:false'),
    ('gpu', 'fluid simulation webgpu fork:false'),
    ('water', 'water shader fork:false'),
    ('splash', 'splash particles simulation fork:false'),
    ('mpm', 'fluid MPM fork:false'),
]
CURATED = ['InteractiveComputerGraphics/SPlisHSPlasH',
           'InteractiveComputerGraphics/PositionBasedDynamics',
           'matsuoka-601/Splash', 'matsuoka-601/WaterBall',
           'rlguy/GridFluidSim3D', 'rlguy/Blender-FLIP-Fluids',
           'thunil/mantaflow', 'NVIDIAGameWorks/FleX',
           'ttnghia/RealTimeFluidRendering', 'Popov72/FluidRendering',
           'pablode/flut', 'loganzartman/gl-pic-fluid',
           'SebLague/Fluid-Sim', 'jeantimex/fluid',
           'dli/fluid', 'PavelDoGreat/WebGL-Fluid-Simulation',
           'tunabrain/incremental-fluids', 'taichi-dev/taichi',
           'mflowcode/mfc', 'stevenlarg/FLUIDS3']
EXTENSIONS = {'.cpp', '.h', '.hpp', '.c', '.cu', '.cuh', '.cs', '.compute',
              '.shader', '.glsl', '.wgsl', '.frag', '.vert', '.ts', '.js',
              '.py', '.rs', '.gd', '.hlsl', '.metal', '.cginc', '.f90', '.html', '.fxsub', '.fsh'}
TERMS = {
    'pressure': r'pressure|\bdivergence\b|\bjacobi\b',
    'neighbors': r'neighbor|neighbour|spatial.?hash|cell.?index|counting.?sort',
    'particle-grid': r'\bflip\b|\bpic\b|\bp2g\b|\bg2p\b|affine.?velocity',
    'surface tension': r'surface.?tension|\bcapillar|cohesion',
    'depth/thickness': r'thickness|fluid.?depth|depth.?texture|depth.?buffer',
    'smoothing': r'bilateral|curvature|narrow.?range',
    'optics': r'fresnel|refract|beer.?lambert|absorption',
    'secondary particles': r'whitewater|\bspray\b|\bsplash\b|\bfoam\b|\bbubble',
    'wet surface': r'wetness|wetmap|wet.?surface|puddle',
    'collision': r'collision|collider|boundary|\bsdf\b',
    'viscosity': r'viscosity|viscous',
}

def gh(endpoint, fields=None):
    cmd = ['gh', 'api', endpoint]
    if fields:
        cmd += ['-X', 'GET']
        for key, value in fields.items():
            cmd += ['-f', f'{key}={value}']
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=90)
    if result.returncode:
        raise RuntimeError(result.stderr[:400])
    return json.loads(result.stdout)

def source_score(path):
    low = path.lower()
    if pathlib.PurePosixPath(low).suffix not in EXTENSIONS:
        return -100
    if re.search(r'(^|/)(vendor|third_party|thirdparty|external|extern|node_modules|dist|build|deps|imgui|glad|glm)(/|$)', low) or 'cuda5.5_include' in low or 'postprocessing/' in low:
        return -100
    score = sum(weight for term, weight in [
        ('fluid', 8), ('sph', 8), ('flip', 8), ('mpm', 8), ('pressure', 6),
        ('surface', 5), ('render', 4), ('depth', 6), ('thickness', 8),
        ('bilateral', 8), ('splash', 8), ('spray', 8), ('water', 5),
        ('simulation', 6), ('solver', 6), ('compute', 3), ('shader', 3),
        ('particle', 4), ('wet', 8), ('foam', 6), ('main', 1)] if term in low)
    base = pathlib.PurePosixPath(low).name
    score += sum(weight for term, weight in [('solver', 8), ('timestep', 10),
                 ('fluidsimulation', 12), ('fluid', 3), ('bilateral', 10),
                 ('narrowrange', 10), ('surface', 4), ('wet', 10), ('pressure', 8)] if term in base)
    if any(term in low for term in ('simulationdata', 'c_bindings', '.reg.cpp', '/benches/', 'test', '/editor/', '_cache.', 'aabb.', 'benchmark')):
        score -= 30
    if pathlib.PurePosixPath(low).suffix in {'.h', '.hpp'}:
        score -= 6
    return score

def inspect(item, force=False):
    name = item['full_name']
    folder = CACHE / name.replace('/', '__')
    folder.mkdir(parents=True, exist_ok=True)
    result_path = folder / 'inspection.json'
    if result_path.exists() and not force:
        return json.loads(result_path.read_text())
    try:
        if force and result_path.exists() and json.loads(result_path.read_text()).get('commit'):
            sha = json.loads(result_path.read_text())['commit']
        else:
            commit = gh(f"repos/{name}/commits/{item['default_branch']}")
            sha = commit['sha']
        tree = gh(f'repos/{name}/git/trees/{sha}', {'recursive': '1'})
        blobs = [b for b in tree['tree'] if b['type'] == 'blob' and
                 b.get('size', 0) <= 512000 and source_score(b['path']) > 0]
        blobs.sort(key=lambda b: (-source_score(b['path']), b['path']))
        selected = []
        # Include both simulation and rendering, rather than four adjacent headers.
        used_stems = set()
        for blob in blobs:
            stem = pathlib.PurePosixPath(blob['path']).stem.lower()
            if stem in used_stems:
                continue
            used_stems.add(stem)
            selected.append(blob)
            if len(selected) == 4:
                break
        sources = []
        for blob in selected:
            path = blob['path']
            url = f'https://raw.githubusercontent.com/{name}/{sha}/{urllib.parse.quote(path)}'
            req = urllib.request.Request(url, headers={'User-Agent': 'Voxy-source-survey'})
            with urllib.request.urlopen(req, timeout=45) as response:
                raw = response.read(512001)
            if len(raw) > 512000:
                continue
            body = raw.decode('utf-8', errors='replace')
            local = folder / path
            local.parent.mkdir(parents=True, exist_ok=True)
            local.write_bytes(raw)
            tags = {}
            for tag, pattern in TERMS.items():
                hits = [i for i, line in enumerate(body.splitlines(), 1)
                        if re.search(pattern, line, re.I)]
                if hits:
                    tags[tag] = hits[:8]
            sources.append({'path': path, 'bytes': len(raw),
                            'lines': len(body.splitlines()),
                            'sha256': hashlib.sha256(raw).hexdigest(),
                            'url': f'https://github.com/{name}/blob/{sha}/{path}',
                            'signals': tags, 'cache_path': str(local)})
        result = {'repository': name, 'url': item['html_url'], 'commit': sha,
                  'description': item.get('description'), 'language': item.get('language'),
                  'license': (item.get('license') or {}).get('spdx_id'),
                  'category': item.get('survey_category', 'curated'),
                  'fork': item.get('fork'), 'tree_truncated': tree.get('truncated', False),
                  'sources': sources, 'inspection': 'source screening; not built or executed',
                  'usable': bool(sources) and any(s['signals'] for s in sources)}
    except Exception as exc:
        result = {'repository': name, 'usable': False, 'error': str(exc)}
    result_path.write_text(json.dumps(result, indent=2, ensure_ascii=False))
    return result

def main():
    CACHE.mkdir(parents=True, exist_ok=True)
    OUT.mkdir(parents=True, exist_ok=True)
    if '--refine' in sys.argv:
        manifest = json.loads((OUT / 'manifest.json').read_text())
        excluded = {'MFlowCode/MFC', 'danieljprice/splash', 'myguixx/SPLASH',
                    'marcozakaria/URP-LWRP-Shaders', 'Cyanilux/URP_WatercolourShaders',
                    'Bercon/feenikslintu', 'vishrutjetly/ML-Fluid-Simulation'}
        items = [{'full_name': r['repository'], 'default_branch': '',
                  'html_url': r['url'], 'description': r.get('description'),
                  'language': r.get('language'), 'license': {'spdx_id': r.get('license')},
                  'fork': r.get('fork'), 'survey_category': r.get('category')}
                 for r in manifest['repositories'] if r['repository'] not in excluded]
        for name in ['larsbertram69/Lux', 'Shaderic/Realistic-Material-Wetness',
                     'smkplus/WetShader', 'ray-cast/ray-mmd',
                     'turanszkij/WickedEngine', 'doyubkim/fluid-engine-dev',
                     'rlguy/GridFluidSim3D']:
            item = gh(f'repos/{name}')
            item['survey_category'] = 'wetness/engine' if len(items) < 98 else 'fluid engine'
            items.append(item)
        results = []
        with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
            futures = {pool.submit(inspect, item, True): item for item in items}
            for future in concurrent.futures.as_completed(futures):
                result = future.result()
                results.append(result)
                print(f"refined {len(results)} {result['repository']} usable={result['usable']}", flush=True)
        by_name = {r['repository']: r for r in results}
        manifest['repositories'] = [by_name[i['full_name']] for i in items if by_name[i['full_name']]['usable']]
        manifest['count'] = len(manifest['repositories'])
        manifest['excluded_or_failed'] += [r for r in results if not r['usable']]
        manifest['editorial_exclusions'] = sorted(excluded)
        (OUT / 'manifest.json').write_text(json.dumps(manifest, indent=2, ensure_ascii=False))
        print('REFINED SURVEY SAVED:', manifest['count'], flush=True)
        return
    candidates = {}
    for name in CURATED:
        try:
            item = gh(f'repos/{name}')
            item['survey_category'] = 'curated'
            candidates[item['full_name']] = item
        except Exception as exc:
            print('curated unavailable', name, str(exc), flush=True)
    groups = []
    for category, query in QUERIES:
        cache = CACHE / f'search-{category}.json'
        if cache.exists():
            data = json.loads(cache.read_text())
        else:
            data = gh('search/repositories', {'q': query, 'per_page': '50', 'sort': 'stars'})
            cache.write_text(json.dumps(data))
        group = []
        for item in data['items']:
            item['survey_category'] = category
            if not item.get('fork'):
                group.append(item)
        groups.append(group)
    # Round-robin topics prevents one popular tutorial family dominating selection.
    for rank in range(50):
        for group in groups:
            if rank < len(group):
                item = group[rank]
                candidates.setdefault(item['full_name'], item)
    ordered = list(candidates.values())
    results = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
        futures = [pool.submit(inspect, item) for item in ordered[:145]]
        for future in concurrent.futures.as_completed(futures):
            result = future.result()
            results.append(result)
            print(f"{len(results)} {result['repository']} usable={result['usable']}", flush=True)
    by_name = {r['repository']: r for r in results}
    usable = [by_name[item['full_name']] for item in ordered
              if item['full_name'] in by_name and by_name[item['full_name']]['usable']][:100]
    manifest = {'date': '2026-10-02', 'scope': 'fluids, splash physics, rendering and wet surfaces',
                'method': 'Pinned commits; up to four selected source files per repository; keyword signals are navigation aids, not proof of algorithm correctness. No third-party builds or benchmarks.',
                'count': len(usable), 'repositories': usable,
                'excluded_or_failed': [r for r in results if not r['usable']]}
    (OUT / 'manifest.json').write_text(json.dumps(manifest, indent=2, ensure_ascii=False))
    print(f'SURVEY SAVED: {len(usable)} repositories', flush=True)

if __name__ == '__main__':
    main()
