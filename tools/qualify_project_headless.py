#!/usr/bin/env python3
"""Exercise only create/validate/rename commands of a supplied editor binary."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile


def qualify(binary, output):
    binary = binary.resolve(strict=True)
    output.mkdir(parents=True, exist_ok=True)
    report_path = output / 'cli.json'
    if report_path.exists():
        raise ValueError('refusing to replace existing CLI report')
    results = []
    failures = []
    digest = hashlib.sha256(binary.read_bytes()).hexdigest()

    def run(command, *arguments, ok=True):
        if command not in {'--help', '--create-project', '--validate-project', '--rename-project'}:
            raise ValueError('command not admitted for window-free qualification')
        result = subprocess.run([str(binary), command, *map(str, arguments)], cwd='/', capture_output=True, text=True, timeout=30)
        entry = dict(command=command, arguments=list(map(str, arguments)), returncode=result.returncode, expected_success=ok, stdout=result.stdout, stderr=result.stderr)
        results.append(entry)
        if (result.returncode == 0) != ok:
            failures.append(entry)

    with tempfile.TemporaryDirectory(prefix='voxy-eng01-cli-') as temporary:
        root = Path(temporary)
        directory = root / 'project with spaces'
        path = directory / 'project.voxy'
        run('--help')
        run('--create-project', directory, 'Первый проект')
        run('--validate-project', path)
        original = path.read_bytes()
        run('--create-project', directory, 'Overwrite', ok=False)
        assert path.read_bytes() == original, 'existing descriptor was overwritten'
        run('--create-project', root / 'blank', ' ', ok=False)
        assert not (root / 'blank').exists(), 'invalid creation left a directory'
        for name in ('Renamed project', 'Сохранён повторно'):
            run('--rename-project', path, name)
            assert json.loads(path.read_text())['name'] == name
            run('--validate-project', path)
        saved = path.read_bytes()
        run('--rename-project', path, ' ', ok=False)
        assert path.read_bytes() == saved
        (directory / 'scenes').mkdir()
        (directory / 'scenes/start.json').write_bytes((directory / 'main.scene.json').read_bytes())
        descriptor = json.loads(saved)
        descriptor['startup_scene'] = 'scenes/start.json'
        path.write_text(json.dumps(descriptor))
        run('--validate-project', path)
        run('--rename-project', path, 'Nested scene')
        nested = path.read_bytes()
        descriptor['version'] = 999
        path.write_text(json.dumps(descriptor))
        bad = path.read_bytes()
        run('--validate-project', path, ok=False)
        run('--rename-project', path, 'Must fail', ok=False)
        assert path.read_bytes() == bad
        path.write_bytes(nested)
        (directory / 'scenes/start.json').unlink()
        run('--validate-project', path, ok=False)
        run('--rename-project', path, 'Must fail', ok=False)
        assert path.read_bytes() == nested
        (directory / 'scenes/start.json').write_bytes((directory / 'main.scene.json').read_bytes())
        moved = root / 'relocated project'
        directory.rename(moved)
        run('--validate-project', moved / 'project.voxy')
        run('--rename-project', moved / 'project.voxy', 'Relocated')
        run('--validate-project', moved / 'project.voxy')
        assert json.loads((moved / 'project.voxy').read_text())['startup_scene'] == 'scenes/start.json'
    stable = hashlib.sha256(binary.read_bytes()).hexdigest() == digest
    report = dict(schema_version=1, binary=str(binary), binary_sha256=digest, binary_stable=stable, cwd='/', window_gpu_audio_initialization=False, calls=results, passed=not failures and stable)
    report_path.write_text(json.dumps(report, indent=2, ensure_ascii=False) + '\n')
    if failures or not stable:
        raise RuntimeError(f'CLI qualification failed: {report_path}')
    print(f'{len(results)} CLI cases passed: {report_path}')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    qualify(args.binary, args.output)
