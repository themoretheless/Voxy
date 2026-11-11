#!/usr/bin/env python3
"""Pack a Voxy runner and its resource package into a portable desktop ZIP."""

import argparse
import os
from pathlib import Path
import stat
import struct
import tempfile
import zipfile


def executable_name(binary, platform):
    with binary.open("rb") as file:
        header = file.read(64)
        if platform == "windows" and len(header) == 64 and header[:2] == b"MZ":
            file.seek(struct.unpack_from("<I", header, 60)[0])
            if file.read(4) == b"PE\0\0":
                return "voxy_game.exe"
        if platform == "linux" and header.startswith(b"\x7fELF"):
            return "voxy_game"
    raise ValueError(f"{binary} is not a {platform} executable")


def package_game(binary, package, output, platform):
    runner_name = executable_name(binary, platform)
    if not package.is_file():
        raise ValueError(f"missing resource package {package}")
    if output.suffix.lower() != ".zip":
        raise ValueError("output must end in .zip")
    if os.path.lexists(output):
        raise FileExistsError(f"output already exists: {output}")
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".voxy-package-", dir=output.parent) as temporary:
        staged = Path(temporary) / "game.zip"
        with zipfile.ZipFile(staged, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            runner = zipfile.ZipInfo.from_file(binary, runner_name)
            runner.create_system = 3
            runner.external_attr = (stat.S_IFREG | 0o755) << 16
            runner.compress_type = zipfile.ZIP_DEFLATED
            with binary.open("rb") as source, archive.open(runner, "w") as target:
                while chunk := source.read(1024 * 1024):
                    target.write(chunk)
            archive.write(package, "game.vpak")
        # Publish only a completed archive and fail atomically on existing output.
        os.link(staged, output)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--platform", required=True, choices=("windows", "linux"))
    parser.add_argument("binary", type=Path)
    parser.add_argument("package", type=Path)
    parser.add_argument("output", type=Path)
    options = parser.parse_args()
    try:
        package_game(options.binary, options.package, options.output, options.platform)
    except (OSError, ValueError) as error:
        parser.exit(1, f"{error}\n")
    print(options.output)
