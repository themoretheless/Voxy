#!/usr/bin/env python3
"""Build a signed arm64 NativeActivity APK using explicit Android SDK/NDK paths."""
import argparse
import os
from pathlib import Path
import platform
import shutil
import struct
import subprocess
import sys
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[2]


def run(*args, env=None):
    subprocess.run([str(arg) for arg in args], check=True, cwd=ROOT, env=env)


def validate_library(path):
    """Reject a wrong architecture or load segment layout before APK packaging."""
    data = path.read_bytes()
    if len(data) < 64 or data[:6] != b'\x7fELF\x02\x01':
        raise ValueError('Expected a little-endian ELF64 Android library')
    kind, machine = struct.unpack_from('<HH', data, 16)
    if kind != 3 or machine != 183:
        raise ValueError('Expected an AArch64 shared library')
    offset = struct.unpack_from('<Q', data, 32)[0]
    size, count = struct.unpack_from('<HH', data, 54)
    if size != 56 or not count or offset < 64 or offset + size * count > len(data):
        raise ValueError('Invalid ELF program header table')
    loads = 0
    for index in range(count):
        header = struct.unpack_from('<IIQQQQQQ', data, offset + index * size)
        tag, _, file_offset, address, _, file_size, memory_size, alignment = header
        if tag != 1:
            continue
        loads += 1
        if (alignment < 16384 or alignment & (alignment - 1)
                or file_offset % 16384 != address % 16384
                or file_size > memory_size or file_offset + file_size > len(data)):
            raise ValueError('ELF load segment is invalid or incompatible with 16 KiB pages')
    if not loads:
        raise ValueError('ELF library contains no load segments')


def select_device(adb, requested=None):
    """Select one online device before a build/install; never guess among devices."""
    output = subprocess.check_output([str(adb), 'devices'], text=True)
    devices = []
    for line in output.splitlines():
        fields = line.split()
        if len(fields) == 2 and fields[1] == 'device':
            devices.append(fields[0])
    if requested is not None:
        if requested not in devices:
            raise ValueError('Selected adb device is missing, offline or unauthorized')
        return requested
    if len(devices) != 1:
        raise ValueError('Install requires one online adb device; use --device SERIAL to select explicitly')
    return devices[0]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true', help='Check prerequisites without building')
    parser.add_argument('--install', action='store_true', help='Install on the selected adb device')
    parser.add_argument('--device', help='Explicit adb serial for --install')
    args = parser.parse_args()
    if args.device is not None and not args.install:
        parser.error('--device requires --install')
    sdk = os.environ.get('ANDROID_HOME') or os.environ.get('ANDROID_SDK_ROOT')
    ndk = os.environ.get('ANDROID_NDK_HOME')
    if not sdk or not ndk:
        raise ValueError('Set ANDROID_HOME and ANDROID_NDK_HOME to installed SDK and NDK directories')
    sdk, ndk = Path(sdk).resolve(), Path(ndk).resolve()
    host = {'Darwin': 'darwin-x86_64', 'Linux': 'linux-x86_64'}.get(platform.system())
    if host is None:
        raise ValueError('This packaging script currently supports macOS/Linux hosts')
    tools = sdk / 'build-tools' / os.environ.get('VOXY_ANDROID_BUILD_TOOLS', '35.0.0')
    toolchain = ndk / 'toolchains/llvm/prebuilt' / host / 'bin'
    linker = toolchain / 'aarch64-linux-android26-clang'
    android_jar = sdk / 'platforms/android-35/android.jar'
    for path in [linker, toolchain / 'llvm-ar', android_jar,
                 tools / 'aapt2', tools / 'zipalign', tools / 'apksigner']:
        if not path.is_file():
            raise ValueError(f'Missing Android prerequisite: {path}')
    for name in ['cargo', 'rustup', 'java', 'keytool']:
        if not shutil.which(name):
            raise ValueError(f'Missing executable: {name}')
    installed = subprocess.check_output(['rustup', 'target', 'list', '--installed'], text=True)
    if 'aarch64-linux-android' not in installed.splitlines():
        raise ValueError('Install Rust target: rustup target add aarch64-linux-android')
    adb = sdk / 'platform-tools/adb'
    if args.install and not adb.is_file():
        raise ValueError(f'Missing adb: {adb}')
    selected_device = select_device(adb, args.device) if args.install else None
    if args.check:
        print('Android packaging prerequisites available; no compilation or device proof performed')
        return
    env = os.environ.copy()
    env['CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER'] = str(linker)
    env['CC_aarch64_linux_android'] = str(linker)
    env['AR_aarch64_linux_android'] = str(toolchain / 'llvm-ar')
    # Required for Android devices using 16 KiB memory pages.
    env['CARGO_ENCODED_RUSTFLAGS'] = env.get('CARGO_ENCODED_RUSTFLAGS', '') + (
        '\x1f' if env.get('CARGO_ENCODED_RUSTFLAGS') else '') + (
        '-Clink-arg=-Wl,-z,max-page-size=16384')
    run('cargo', 'build', '--locked', '-p', 'voxy_mobile', '--release',
        '--target', 'aarch64-linux-android', env=env)
    target = Path(env.get('CARGO_TARGET_DIR', ROOT / 'target'))
    if not target.is_absolute():
        target = ROOT / target
    library = target / 'aarch64-linux-android/release/libvoxy_mobile.so'
    validate_library(library)
    output = ROOT / 'target/android'
    output.mkdir(parents=True, exist_ok=True)
    key = output / 'debug.keystore'
    if not key.exists():
        run('keytool', '-genkeypair', '-keystore', key, '-storepass', 'android',
            '-keypass', 'android', '-alias', 'androiddebugkey', '-keyalg', 'RSA',
            '-keysize', '2048', '-validity', '10000', '-dname', 'CN=Voxy Development')
    # Development signing only. Never reuse this key for production distribution.
    with tempfile.TemporaryDirectory(prefix='voxy-apk-', dir=output) as staging:
        staging = Path(staging)
        raw, aligned = staging / 'raw.apk', staging / 'aligned.apk'
        run(tools / 'aapt2', 'link', '-o', raw, '--manifest',
            ROOT / 'mobile/android/AndroidManifest.xml', '-I', android_jar,
            '--version-code', '1', '--version-name', '0.1.0')
        with zipfile.ZipFile(raw, 'a') as archive:
            archive.write(library, 'lib/arm64-v8a/libvoxy_mobile.so',
                          compress_type=zipfile.ZIP_STORED)
        run(tools / 'zipalign', '-P', '16', '-f', '4', raw, aligned)
        apk = output / 'voxy-debug.apk'
        run(tools / 'apksigner', 'sign', '--ks', key, '--ks-key-alias',
            'androiddebugkey', '--ks-pass', 'pass:android', '--key-pass',
            'pass:android', '--out', apk, aligned)
        run(tools / 'apksigner', 'verify', '--verbose', apk)
        run(tools / 'zipalign', '-c', '-P', '16', '4', apk)
    print(f'Built and verified development APK: {apk}')
    if args.install:
        run(adb, '-s', selected_device, 'install', '-r', apk)
        run(adb, '-s', selected_device, 'shell', 'am', 'start', '-W', '-n', 'dev.voxy.engine/android.app.NativeActivity')


if __name__ == '__main__':
    try:
        main()
    except (ValueError, subprocess.CalledProcessError) as error:
        print(f'Android build failed: {error}', file=sys.stderr)
        sys.exit(1)
