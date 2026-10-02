#!/usr/bin/env python3
"""Validate source receipts and render the 100-repository index."""
import csv
import hashlib
import io
import json
import pathlib
import urllib.parse

ROOT = pathlib.Path(__file__).resolve().parents[1]
OUT = ROOT / 'docs/research/fluid-repositories-100'
PRIMARY = {
    'InteractiveComputerGraphics/PositionBasedDynamics': 'Demos/FluidDemo/TimeStepFluidModel.cpp',
    'matsuoka-601/Splash': 'render/narrowRangeFilter.wgsl',
    'rlguy/Blender-FLIP-Fluids': 'src/engine/diffuseparticlesimulation.cpp',
    'ttnghia/RealTimeFluidRendering': 'Shaders/filter-narrow-range.fs.glsl',
    'NVIDIAGameWorks/FleX': 'demo/d3d/shaders/compositePS.hlsl',
    'Popov72/FluidRendering': 'src/scenes/FluidSimulator2/fluidSimulator.ts',
    'pablode/flut': 'shaders/renderCurvature.frag',
    'SebLague/Fluid-Sim': 'Assets/Scripts/Simulation/Compute/FluidSim.compute',
    'dli/fluid': 'simulator.js',
    'leonardo-montes/Unity-ECS-Job-System-SPH': 'Assets/Job System/SPHSystem.cs',
    'jinleili/fluid-webgpu': 'shader/lbm/d2q9_collide.comp.glsl',
    'robkau/mlsmpm-particles-rs': 'src/step_p2g.rs',
    'lukedan/libfluid': 'src/pressure_solver.cpp',
    'bwiberg/position-based-fluids': 'kernels/timestep.cl',
    'larsbertram69/Lux': 'Lux Shader/LuxCore/Wetness/LuxWetness.cginc',
    'turanszkij/WickedEngine': 'WickedEngine/shaders/surfaceHF.hlsli',
    'ray-cast/ray-mmd': 'Materials/Programmable/Wetness/material_functions.fxsub',
    'rlguy/GridFluidSim3D': 'src/diffuseparticlesimulation.cpp',
}
FOCUSED = {
    'matsuoka-601/Splash', 'ttnghia/RealTimeFluidRendering', 'NVIDIAGameWorks/FleX',
    'rlguy/Blender-FLIP-Fluids', 'rlguy/GridFluidSim3D',
    'InteractiveComputerGraphics/SPlisHSPlasH', 'InteractiveComputerGraphics/PositionBasedDynamics',
    'dimforge/salva', 'Wumpf/blub', 'InteractiveComputerGraphics/splashsurf',
    'larsbertram69/Lux', 'turanszkij/WickedEngine', 'Shaderic/Realistic-Material-Wetness',
    'smkplus/WetShader', 'dli/fluid', 'pablode/flut',
}

def main():
    path = OUT / 'manifest.json'
    manifest = json.loads(path.read_text())
    repositories = manifest['repositories']
    assert len(repositories) == manifest['count'] == 100
    assert len({r['repository'].lower() for r in repositories}) == 100
    total_bytes = total_lines = total_files = 0
    lines = [
        '# Исходники 100 репозиториев: проверенный перечень', '',
        'Дата получения: 2 октября 2026. [Выводы для Voxy](findings.md). [Manifest с SHA-256](manifest.json).', '',
        'Для каждого проекта получен код на закреплённом commit. Обзор означает чтение выбранных участков; он не означает полный аудит, успешную сборку или запуск. «Подробнее» отмечает разбор в выводах. Названия в колонке участков — навигационные сигналы из кода, а не сертификат поддержки всех перечисленных возможностей. Часть проектов учебные, 2D, smoke или ocean: они нужны для сравнения отдельных методов, а не как готовая замена жидкости на теле.', '',
        '| № | Репозиторий | Закреплённый участок исходника | Участки для сравнения | Глубина |',
        '|---:|---|---|---|---|',
    ]
    csv_out = io.StringIO()
    writer = csv.writer(csv_out)
    writer.writerow(['number', 'repository', 'commit', 'source', 'source_sha256', 'source_url', 'files_retrieved', 'inspection_depth'])
    for index, repo in enumerate(repositories, 1):
        assert len(repo['commit']) == 40
        for source in repo['sources']:
            raw = pathlib.Path(source['cache_path']).read_bytes()
            assert hashlib.sha256(raw).hexdigest() == source['sha256']
            assert len(raw) == source['bytes']
            total_files += 1
            total_bytes += source['bytes']
            total_lines += source['lines']
            source['url'] = f"https://github.com/{repo['repository']}/blob/{repo['commit']}/{urllib.parse.quote(source['path'])}"
        preferred = PRIMARY.get(repo['repository'])
        source = next((s for s in repo['sources'] if s['path'] == preferred), None)
        if source is None:
            source = max(repo['sources'], key=lambda s: len(s['signals']) + min(s['lines'] / 300, 3))
        signals = source['signals']
        hint = ', '.join(signals) or 'шейдер / расчёт состояния; см. исходник'
        hit_lines = [n for hits in signals.values() for n in hits]
        anchor = f'#L{min(hit_lines)}' if hit_lines else ''
        source_url = source['url'] + anchor
        depth = 'Подробнее' if repo['repository'] in FOCUSED else 'Обзор участка'
        label = source['path'].replace('|', '\\|')
        lines.append(f"| {index} | [{repo['repository']}]({repo['url']}) | [{label}]({source_url}) | {hint} | {depth} |")
        repo['review_entrypoint'] = source['path']
        repo['review_depth'] = depth
        writer.writerow([index, repo['repository'], repo['commit'], source['path'], source['sha256'], source_url, len(repo['sources']), depth])
    lines += ['', f'Проверено соответствие локальных файлов manifest: {total_files} файлов, {total_bytes} байт, {total_lines} строк. Размер корпуса не равен объёму подробного ручного разбора.', '',
              'Для повторного получения: `python3 tools/research_fluid_repositories.py`, затем `--refine`, `python3 tools/enrich_fluid_survey.py` и `python3 tools/render_fluid_survey.py`. Новый запуск поиска может дать другой набор: источником истины для этого отчёта служит сохранённый manifest с закреплёнными ссылками, а не выдача поиска.', '']
    (OUT / 'repositories.md').write_text('\n'.join(lines))
    (OUT / 'repositories.csv').write_text(csv_out.getvalue())
    manifest['method'] = 'Initial screening of up to four selected source files per repository, followed by explicit algorithm-source enrichment. Selected code sections reviewed; 16 focused entries. No third-party builds, executions or benchmarks. Keyword signals are navigation aids, not correctness evidence.'
    manifest['verified_source_receipts'] = {'files': total_files, 'bytes': total_bytes, 'lines': total_lines, 'sha256_matches': True}
    manifest['reviewed_repositories'] = 100
    manifest['focused_entries'] = len(FOCUSED)
    path.write_text(json.dumps(manifest, indent=2, ensure_ascii=False))
    print(json.dumps(manifest['verified_source_receipts']))

if __name__ == '__main__':
    main()
