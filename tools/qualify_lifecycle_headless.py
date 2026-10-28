#!/usr/bin/env python3
"""ENG-02: project/package lifecycle through explicitly headless CLI modes."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import shutil
import subprocess
import tempfile


def qualify(binary, output):
    binary = binary.resolve(strict=True)
    output.mkdir(parents=True, exist_ok=True)
    report = output / 'cli.json'
    if report.exists():
        raise ValueError('choose a fresh report directory')
    digest = hashlib.sha256(binary.read_bytes()).hexdigest()
    calls = []

    def run(*arguments):
        command = arguments[0]
        if command in {'--project', '--package'}:
            if len(arguments) != 3 or arguments[2] != '--game-check':
                raise ValueError('only game-check is admitted')
        elif command not in {'--create-project', '--build-project'}:
            raise ValueError('command not admitted')
        result = subprocess.run([str(binary), *map(str, arguments)], cwd='/', capture_output=True, text=True, timeout=60)
        calls.append(dict(arguments=list(map(str, arguments)), exit_code=result.returncode, stdout=result.stdout, stderr=result.stderr))
        report.write_text(json.dumps(dict(binary_sha256=digest, passed=False, calls=calls), indent=2)+'\n')
        if result.returncode:
            raise RuntimeError(f'headless CLI failed; see {report}')
        return result.stdout

    def state(stdout):
        assert 'GAME CHECK PASS ticks=120' in stdout
        return json.loads(next(line.removeprefix('GAME STATE ') for line in stdout.splitlines() if line.startswith('GAME STATE ')))

    with tempfile.TemporaryDirectory(prefix='voxy-eng02-cli-') as directory:
        root = Path(directory)
        source = root / 'project'
        path = source / 'project.voxy'
        run('--create-project', source, 'Lifecycle')
        scene = source / 'main.scene.json'
        scene.write_text(json.dumps(dict(version=1, objects=[dict(id='actor', parent=None, name='Actor', active=True, translation=[0,0,0], rotation=[0,0,0,1], scale=[1,1,1], components={'game.angular-motion.v1':dict(axis=[0,0,1], radians_per_second=2)})])))
        authored = scene.read_bytes()
        first = state(run('--project', path, '--game-check'))
        second = state(run('--project', path, '--game-check'))
        assert first == second
        assert scene.read_bytes() == authored
        rotation = first['objects'][0]['rotation']
        dot = rotation[2] * math.sin(2) + rotation[3] * math.cos(2)
        assert abs(abs(dot)-1) < 1e-5
        package = root / 'game.vpak'
        run('--build-project', path, package)
        shutil.rmtree(source)
        assert not path.exists()
        packaged = state(run('--package', package, '--game-check'))
        repeated = state(run('--package', package, '--game-check'))
        assert first == packaged == repeated
    stable = hashlib.sha256(binary.read_bytes()).hexdigest() == digest
    report.write_text(json.dumps(dict(binary=str(binary), binary_sha256=digest, binary_stable=stable, cwd='/', calls=calls, project_package_state_equal=True, authoring_file_unchanged=True, passed=stable, scope='Headless game-check only; no GPU/native presentation qualification'), indent=2)+'\n')
    if not stable:
        raise RuntimeError('binary changed during qualification')
    print(f'{len(calls)} headless CLI calls passed: {report}')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    qualify(args.binary, args.output)
