"""Fetch pinned NVIDIA compiler wheels into ignored target storage (no install)."""
import hashlib
import json
from pathlib import Path
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[2]
DESTINATION = ROOT / "target" / "cuda-tooling"
VERSION = "12.6.85"


def main():
    DESTINATION.mkdir(parents=True, exist_ok=True)
    for package in ("nvidia-cuda-nvrtc-cu12", "nvidia-cuda-nvcc-cu12"):
        with urllib.request.urlopen(f"https://pypi.org/pypi/{package}/{VERSION}/json", timeout=30) as response:
            metadata = json.load(response)
        files = [item for item in metadata["urls"]
                 if "manylinux" in item["filename"] and "aarch64" in item["filename"]]
        if len(files) != 1:
            raise RuntimeError(f"Expected one pinned Linux ARM64 wheel for {package}")
        item = files[0]
        destination = DESTINATION / item["filename"]
        if not destination.exists():
            with urllib.request.urlopen(item["url"], timeout=30) as response:
                data = response.read()
            if hashlib.sha256(data).hexdigest() != item["digests"]["sha256"]:
                raise RuntimeError("Wheel SHA256 mismatch")
            destination.write_bytes(data)
        if hashlib.sha256(destination.read_bytes()).hexdigest() != item["digests"]["sha256"]:
            raise RuntimeError("Cached wheel SHA256 mismatch")
        with zipfile.ZipFile(destination) as archive:
            for member in archive.infolist():
                target = (DESTINATION / member.filename).resolve()
                if not target.is_relative_to(DESTINATION.resolve()):
                    raise RuntimeError("Invalid wheel path")
            archive.extractall(DESTINATION)
        print(f"Verified {item['filename']} SHA256 {item['digests']['sha256']}")
    assembler = DESTINATION / "nvidia" / "cuda_nvcc" / "bin" / "ptxas"
    assembler.chmod(0o755)


if __name__ == "__main__":
    main()
