"""Compile the pinned upstream algorithm in a temporary research-only directory."""
from pathlib import Path
import argparse
import shutil
import subprocess
import tempfile


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--export-dir", type=Path, help="write base and generated LOD OBJ files without overwriting")
    args = parser.parse_args()
    if args.export_dir:
        args.export_dir = args.export_dir.resolve()
        if (args.export_dir / "base.obj").exists() or list(args.export_dir.glob("lod-*.obj")):
            raise RuntimeError("export directory already contains generated meshes")
        args.export_dir.mkdir(parents=True, exist_ok=True)
    root = Path(__file__).resolve().parent
    with tempfile.TemporaryDirectory(prefix="voxy-godot-lod-") as directory:
        work = Path(directory)
        for name in ("meshoptimizer.h", "simplifier.cpp", "allocator.cpp"):
            shutil.copyfile(root / ("thirdparty__meshoptimizer__" + name), work / name)
        shutil.copyfile(root / "importer_probe.cpp", work / "importer_probe.cpp")
        subprocess.run(
            ["clang++", "-std=c++17", "-O2", str(work / "importer_probe.cpp"),
             str(work / "simplifier.cpp"), str(work / "allocator.cpp"),
             "-o", str(work / "probe")],
            check=True, timeout=60,
        )
        command = [str(work / "probe")]
        if args.export_dir:
            command.append(str(args.export_dir))
        subprocess.run(command, check=True, timeout=30)


if __name__ == "__main__":
    main()
