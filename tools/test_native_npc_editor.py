#!/usr/bin/env python3
"""Verify NPC goal authoring, visible traversal and a source-free native launch."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import tempfile
import wave


def scene_object(id_, position, components, scale=(1, 1, 1)):
    return dict(
        id=id_, parent=None, name=id_, active=True, translation=position,
        rotation=[0, 0, 0, 1], scale=scale, components=components,
    )


def write_fixture(project, checkpoint):
    vertices = [
        (-1, -1, -1), (1, -1, -1), (1, 1, -1), (-1, 1, -1),
        (-1, -1, 1), (1, -1, 1), (1, 1, 1), (-1, 1, 1),
    ]
    triangles = []
    for a, b, c, d in [(4, 3, 2, 1), (5, 6, 7, 8), (1, 5, 8, 4),
                       (2, 3, 7, 6), (4, 8, 7, 3), (1, 2, 6, 5)]:
        triangles.extend([(a, b, c), (a, c, d)])
    vertices = [f'v {x / 10} {y / 10} {z / 10}\n' for x, y, z in vertices]
    faces = [f'f {a} {b} {c}\n' for a, b, c in triangles]
    (project / 'body.obj').write_text(''.join(vertices + faces))
    assets = [{'asset': 'body', 'source': 'body.obj'}]
    objects = [
        scene_object('npc', [0, 0.1, 0], {
            'editor.model.v1': 'body',
            'game.character.v1': {
                'half_extents': [0.1] * 3, 'speed': 3, 'gravity': -2.4, 'jump_speed': 0.9,
            },
        }),
        scene_object('floor', [0.5, -0.2, 0.5], {'game.box.v1': {'half_extents': [2, 0.2, 2]}}),
        scene_object('wall', [0.5, 0.4, 0], {
            'editor.model.v1': 'body', 'game.box.v1': {'half_extents': [0.1] * 3},
        }, [1, 4, 5.5]),
    ]
    if checkpoint:
        with wave.open(str(project / 'tone.wav'), 'wb') as output:
            output.setparams((1, 2, 48_000, 0, 'NONE', 'not compressed'))
            output.writeframes(b''.join(
                struct.pack('<h', round(8000 * math.sin(2 * math.pi * i / 120)))
                for i in range(2400)
            ))
        assets.append({'asset': 'tone', 'source': 'tone.wav'})
        objects[0]['components']['game.audio-source.v1'] = {
            'asset': 'tone', 'bus': 0, 'gain': 0.8, 'looping': True, 'spatial': True,
            'near': 0.1, 'far': 5,
        }
        objects[1]['components']['game.audio-listener.v1'] = {}
        objects.append(scene_object('hud', [0, 0, 0], {
            'game.ui-element.v1': {
                'origin': [0.1, 0.1], 'size': [0.2, 0.2], 'color': [0.2, 0.8, 0.5, 1],
                'layer': 0, 'enabled': True, 'action': 'voxy.ui.hide', 'text': None,
            },
        }))
    (project / 'assets.json').write_text(json.dumps({'version': 1, 'assets': assets}))
    (project / 'main.scene.json').write_text(json.dumps({
        'version': 1, 'objects': objects, 'instances': [],
    }))


def qualify(editor, root, checkpoint, standalone_save):
    project = root / 'project'
    descriptor = project / 'project.voxy'
    env = dict(os.environ, VOXY_GAME_SMOKE_NPC_GOAL='npc', VOXY_PREFAB_SMOKE_NPC_GOAL='1')
    if checkpoint:
        env['VOXY_PREFAB_SMOKE_NPC_CHECKPOINT'] = '1'
    if checkpoint or standalone_save:
        env['VOXY_GAME_SMOKE_KEEP_VISIBLE'] = '1'

    def run(*args, extra_env=None):
        result = subprocess.run(
            [str(editor), *map(str, args)], cwd=root, env=env | (extra_env or {}),
            capture_output=True, text=True, timeout=75,
        )
        log = result.stdout + result.stderr
        if result.returncode:
            raise RuntimeError(log)
        return log

    run('--create-project', project, 'Native NPC navigation')
    write_fixture(project, checkpoint or standalone_save)
    editor_log = run('--project', descriptor, '--prefab-smoke')
    editor_match = re.search(r'NPC EDITOR PASS frames=(\d+) ticks=(\d+)', editor_log)
    if not editor_match or 'NPC NATIVE PASS' not in editor_log:
        raise RuntimeError(editor_log)
    saved = json.loads((project / 'main.scene.json').read_text())
    npc = next(obj for obj in saved['objects'] if obj['id'] == 'npc')
    goal = npc['components']['game.npc-destination.v1']['goal']
    if goal != [1, 0] or 'game.npc-route.v1' in npc['components']:
        raise RuntimeError('Editor did not save only the authored destination')

    checkpoint_report = None
    if checkpoint:
        match = re.search(r'NPC CHECKPOINT PASS ticks=(\d+) audio_frames=(\d+) peak=([\d.]+)', editor_log)
        checkpoint_path = project / 'npc-route.voxyedr'
        if not match or not checkpoint_path.is_file():
            raise RuntimeError(editor_log)
        tick, frames, peak = match.groups()
        checkpoint_report = dict(
            tick=int(tick), audio_frames=int(frames), peak=float(peak),
            sha256=hashlib.sha256(checkpoint_path.read_bytes()).hexdigest(),
        )
    package = root / 'navigation.vpak'
    run('--build-project', descriptor, package)
    project_log = run('--project', descriptor, '--game-native-smoke')
    shutil.rmtree(project)
    package_log = run('--package', package, '--game-native-smoke')
    standalone_checkpoint = None
    if standalone_save:
        save_path = root / 'standalone.voxyedr'
        save_env = {'VOXY_GAME_SAVE_FILE': str(save_path), 'VOXY_GAME_SMOKE_SNAPSHOT': 'save'}
        load_env = save_env | {'VOXY_GAME_SMOKE_SNAPSHOT': 'load'}
        saved_log = run('--package', package, '--game-native-smoke', extra_env=save_env)
        if 'STANDALONE SNAPSHOT SAVE PASS' not in saved_log or not save_path.is_file():
            raise RuntimeError(saved_log)
        loaded_log = run('--package', package, '--game-native-smoke', extra_env=load_env)
        restored = re.search(r'STANDALONE SNAPSHOT LOAD PASS ticks=(\d+) audio_frames=(\d+) peak=([\d.]+)', loaded_log)
        if not restored or 'NPC NATIVE PASS' not in loaded_log or 'GAME NATIVE PASS' not in loaded_log:
            raise RuntimeError(loaded_log)
        tick, frames, peak = restored.groups()
        standalone_checkpoint = dict(
            restarted_from_package=True, tick=int(tick), audio_frames=int(frames), peak=float(peak),
            sha256=hashlib.sha256(save_path.read_bytes()).hexdigest(),
        )
    state_log = run('--package', package, '--game-check')
    state = json.loads(next(line.removeprefix('GAME STATE ') for line in state_log.splitlines()
                            if line.startswith('GAME STATE ')))
    npc = next(obj for obj in state['objects'] if obj['id'] == 'npc')
    path = npc['components']['game.npc-route.v1']['waypoints']
    if path[0] != [0, 0, 0] or path[-1] != [1, 0, 0] or max(abs(p[2]) for p in path) < 0.75:
        raise RuntimeError(f'NPC route did not go around the wall: {path}')

    runs = {}
    for label, log in [('project', project_log), ('package', package_log)]:
        match = re.search(r'NPC NATIVE PASS owner=npc frames=(\d+) ticks=(\d+) '
                          r'x=([-\d.]+) z=([-\d.]+)', log)
        if not match or 'GAME NATIVE PASS' not in log:
            raise RuntimeError(log)
        frames, ticks, x, z = match.groups()
        if int(frames) < 3 or int(ticks) < 50 or abs(float(x) - 1) > 1e-4 or abs(float(z)) > 1e-4:
            raise RuntimeError(log)
        runs[label] = dict(frames=int(frames), fixed_ticks=int(ticks), position_xz=[float(x), float(z)])
    return dict(
        editor_sha256=hashlib.sha256(editor.read_bytes()).hexdigest(),
        package_sha256=hashlib.sha256(package.read_bytes()).hexdigest(),
        source_deleted_before_package_launch=True, waypoints=path,
        editor_native=dict(frames=int(editor_match[1]), fixed_ticks=int(editor_match[2]), saved_goal=goal),
        native_runs=runs, checkpoint=checkpoint_report, standalone_checkpoint=standalone_checkpoint,
    )


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--checkpoint', action='store_true')
    parser.add_argument('--standalone-checkpoint', action='store_true')
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='voxy-native-npc-') as directory:
        root = Path(directory)
        editor = root / 'voxy_editor'
        shutil.copy2(args.binary.resolve(strict=True), editor)
        print(json.dumps(qualify(editor, root, args.checkpoint, args.standalone_checkpoint), indent=2))
