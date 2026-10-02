"""Xcode slice routing tests; mocked tools do not prove native iOS linking."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).with_name('build-rust.sh')
MOCK = '''#!/usr/bin/env python3
import json, os, sys
from pathlib import Path
args = sys.argv[1:]
with open(os.environ['BUILD_LOG'], 'a') as log:
    log.write(json.dumps([Path(sys.argv[0]).name, *args]) + '\\n')
if Path(sys.argv[0]).name == 'cargo':
    target = args[args.index('--target') + 1]
    profile = args[args.index('--profile') + 1]
    output = Path(os.environ['CARGO_TARGET_DIR']) / target / ('debug' if profile == 'dev' else 'release') / 'libvoxy_mobile.a'
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(target.encode())
elif '--show-sdk-path' in args:
    if os.environ.get('MOCK_SDK_MISSING') == '1':
        print('SDK unavailable', file=sys.stderr)
        sys.exit(1)
    print('/mock/sdk')
elif args[:2] == ['lipo', '-create']:
    inputs = args[2:args.index('-output')]
    Path(args[-1]).write_bytes(b'|'.join(Path(path).read_bytes() for path in inputs))
'''


class BuildTests(unittest.TestCase):
    def run_build(self, platform, archs, configuration='Debug', sdk_missing=False):
        with tempfile.TemporaryDirectory(prefix='voxy ios test ') as directory:
            root = Path(directory)
            script = root / 'mobile/ios/build-rust.sh'
            script.parent.mkdir(parents=True)
            shutil.copyfile(SCRIPT, script)
            binaries = root / 'mock bin'
            binaries.mkdir()
            for name in ('cargo', 'xcrun'):
                command = binaries / name
                command.write_text(MOCK)
                command.chmod(0o755)
            log = root / 'commands.jsonl'
            output = root / 'build products'
            environment = dict(os.environ, PLATFORM_NAME=platform, ARCHS=archs,
                               CONFIGURATION=configuration, BUILT_PRODUCTS_DIR=str(output),
                               VOXY_CARGO_BIN=str(binaries), BUILD_LOG=str(log))
            environment['MOCK_SDK_MISSING'] = '1' if sdk_missing else '0'
            result = subprocess.run(['sh', str(script)], env=environment,
                                    capture_output=True, text=True)
            commands = [json.loads(line) for line in log.read_text().splitlines()] if log.exists() else []
            artifact = output / 'libvoxy_mobile.a'
            return result, commands, artifact.read_bytes() if artifact.exists() else None

    def test_device_debug_single_slice(self):
        result, commands, artifact = self.run_build('iphoneos', 'arm64')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(artifact, b'aarch64-apple-ios')
        self.assertEqual([c for c in commands if c[0] == 'cargo'],
                         [['cargo', 'build', '--locked', '-p', 'voxy_mobile', '--target',
                           'aarch64-apple-ios', '--profile', 'dev']])
        self.assertFalse(any('lipo' in command for command in commands))

    def test_universal_simulator_release_and_spaces(self):
        result, commands, artifact = self.run_build('iphonesimulator', 'arm64 x86_64', 'Release')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(artifact, b'aarch64-apple-ios-sim|x86_64-apple-ios')
        builds = [c for c in commands if c[0] == 'cargo']
        self.assertEqual([c[c.index('--target') + 1] for c in builds],
                         ['aarch64-apple-ios-sim', 'x86_64-apple-ios'])
        self.assertTrue(all(c[-1] == 'release' for c in builds))
        lipo = [c for c in commands if c[1:3] == ['lipo', '-create']][0]
        self.assertEqual(len(lipo), 7)
        self.assertIn('voxy ios test ', lipo[3])

    def test_invalid_second_architecture_builds_nothing(self):
        result, commands, artifact = self.run_build('iphonesimulator', 'arm64 i386')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Unsupported iOS architecture', result.stderr)
        self.assertEqual(commands, [])
        self.assertIsNone(artifact)

    def test_missing_sdk_builds_nothing(self):
        result, commands, artifact = self.run_build('iphoneos', 'arm64', sdk_missing=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('SDK unavailable', result.stderr)
        self.assertFalse(any(c[0] == 'cargo' for c in commands))
        self.assertIsNone(artifact)

    def test_wrong_platform_builds_nothing(self):
        result, commands, artifact = self.run_build('macosx', 'arm64')
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(commands, [])
        self.assertIsNone(artifact)


if __name__ == '__main__':
    unittest.main()
