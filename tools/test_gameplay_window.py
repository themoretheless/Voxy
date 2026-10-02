#!/usr/bin/env python3
"""Native acceptance; requires a real window/GPU, never substitutes headless output."""
import argparse
import hashlib
import pathlib
import re
import shutil
import subprocess
import tempfile


def main():
    root = pathlib.Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', default=str(root / 'target/debug/voxy_editor'))
    arguments = parser.parse_args()
    binary = pathlib.Path(arguments.binary).resolve()
    with tempfile.TemporaryDirectory(prefix='voxy-gameplay-') as temporary:
        fixture = pathlib.Path(temporary) / 'game'
        shutil.copytree(root / 'crates/voxy_editor/examples/game', fixture)
        scene = fixture / 'game.scene.json'
        before = hashlib.sha256(scene.read_bytes()).hexdigest()
        try:
            result = subprocess.run([str(binary), '--manifest', str(fixture / 'assets.json'),
                                     'player', '--scene', str(scene), '--game-smoke'],
                                    capture_output=True, text=True, timeout=120, check=False)
        except subprocess.TimeoutExpired as error:
            # Keep native progress visible on failure, including launch delays.
            for output in [error.stdout, error.stderr]:
                if output:
                    print(output.decode() if isinstance(output, bytes) else output, end='')
            raise SystemExit('native gameplay process exceeded the 120-second launch/run bound') from error
        print(result.stdout, end='')
        if result.stderr:
            print(result.stderr, end='')
        if result.returncode:
            raise SystemExit(result.returncode)
        for marker in ['GAME INPUT PASS:', 'GAME OWNERS PASS:', 'GAMEPLAY WINDOW PASS:']:
            if marker not in result.stdout:
                raise SystemExit(f'missing native acceptance: {marker}')
        match = re.search(r'GAMEPLAY WINDOW PASS: ticks=(\d+) native_frames=(\d+) authoring_restored=true', result.stdout)
        if not match or int(match[1]) < 29 or int(match[2]) < 12:
            raise SystemExit('insufficient native tick/frame proof')
        if hashlib.sha256(scene.read_bytes()).hexdigest() != before:
            raise SystemExit('Play smoke changed the authoring file')
        print('GAMEPLAY TEST PASS: real presented frames, actions, physics, commands, lifecycle, generation reuse, Stop')


if __name__ == '__main__':
    main()
