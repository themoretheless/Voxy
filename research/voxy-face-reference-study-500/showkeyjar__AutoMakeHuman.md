# Facial rendering source scan: showkeyjar/AutoMakeHuman

Repository: https://github.com/showkeyjar/AutoMakeHuman

Pinned source revision: 1d0df749e52f322dafdf40509330dec4fc95af21

Discovery topic: MakeHuman

Description: MakeHuman's plugin for auto create human models

Reported license: NOASSERTION

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE", "test/async-rl/LICENSE"]

Topic paths: {"rig": ["src/7_auto_human_pose/data/MyAutoData/ORIGINAL_MODEL.bvh"]}

## Source file: bin/install.py

{
  "path": "bin/install.py",
  "blob_sha": "fd5bb13506247dcdb6b17ee20766d154d781b7e4",
  "sha256": "ee0fe18827b5c3ffaa79b6c09202c9fd7c206b73319ebc54b9d6575e9ccb4afd",
  "lines": 34,
  "bytes": 1061,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 2
  },
  "url": "https://github.com/showkeyjar/AutoMakeHuman/blob/1d0df749e52f322dafdf40509330dec4fc95af21/bin/install.py",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "def copy_plugin(source_root: Path, target_root: Path, plugin_name: str) -> None:"
    },
    {
      "line": 16,
      "text": "def main() -> int:"
    }
  ]
}

```text

#!/usr/bin/env python3
import argparse
import shutil
from pathlib import Path


def copy_plugin(source_root: Path, target_root: Path, plugin_name: str) -> None:
    src = source_root / plugin_name
    dst = target_root / plugin_name
    if dst.exists():
        shutil.rmtree(dst)
    shutil.copytree(src, dst)
    print(f"Installed {plugin_name} -> {dst}")


def main() -> int:
    parser = argparse.ArgumentParser(description="Install AutoMakeHuman plugins into a MakeHuman plugins directory.")
    parser.add_argument("--target", required=True, help="Path to the MakeHuman plugins directory")
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parents[1]
    source_root = repo_root / "src"
    target_root = Path(args.target).expanduser().resolve()
    target_root.mkdir(parents=True, exist_ok=True)

    for plugin_name in ("7_auto_human_body", "7_auto_human_pose"):
        copy_plugin(source_root, target_root, plugin_name)

    print("Installation complete.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())


```

## Source file: src/__init__.py

{
  "path": "src/__init__.py",
  "blob_sha": "32dd9478f3f9fa8938d5f774f460cb4b89fdd21b",
  "sha256": "c424fc322e3da1feeaebc5da7ca7f51939f7e442d1dec1f237c0faf2c411c280",
  "lines": 2,
  "bytes": 30,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/showkeyjar/AutoMakeHuman/blob/1d0df749e52f322dafdf40509330dec4fc95af21/src/__init__.py",
  "function_signature_candidates": []
}

```text

# main function
# git is error

```

## Source file: src/7_auto_human_body/__init__.py

{
  "path": "src/7_auto_human_body/__init__.py",
  "blob_sha": "79ab261cddc0f20811faea7c685b8409e5c123fc",
  "sha256": "ea41f5bfdc95fc2f443238c652c1e945bdd5e4b41fde64627cf97045e0bff171",
  "lines": 17,
  "bytes": 406,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/showkeyjar/AutoMakeHuman/blob/1d0df749e52f322dafdf40509330dec4fc95af21/src/7_auto_human_body/__init__.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def load(app):"
    },
    {
      "line": 16,
      "text": "def unload(app):"
    }
  ]
}

```text

#!/usr/bin/env python3
# -*- coding: utf-8 -*-
import os

from .auto_human_body import loadModifierTaskViews

PLUGIN_DIR = os.path.dirname(os.path.abspath(__file__))
SLIDER_CONFIG = os.path.join(PLUGIN_DIR, 'data', 'MyAutoData', 'Mysliders.json')


def load(app):
    category = app.getCategory('Utilities')
    loadModifierTaskViews(SLIDER_CONFIG, app.selectedHuman, category)


def unload(app):
    pass


```

## Source file: src/7_auto_human_pose/__init__.py

{
  "path": "src/7_auto_human_pose/__init__.py",
  "blob_sha": "90fcabef6718faffbc588cc1b29a5e88bcdbda21",
  "sha256": "5bdf0c51cad88e1f11c13431867017e901f905045481c0c3afe19b5b6b3dc4ce",
  "lines": 14,
  "bytes": 368,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/showkeyjar/AutoMakeHuman/blob/1d0df749e52f322dafdf40509330dec4fc95af21/src/7_auto_human_pose/__init__.py",
  "function_signature_candidates": [
    {
      "line": 6,
      "text": "def load(app):"
    },
    {
      "line": 13,
      "text": "def unload(app):"
    }
  ]
}

```text

#!/usr/bin/env python3
# -*- coding: utf-8 -*-
from .auto_human_pose import PoseLibraryAotu1TaskView


def load(app):
    category = app.getCategory('Utilities')
    taskview = category.addTask(PoseLibraryAotu1TaskView(category))
    app.addLoadHandler('pose', taskview.loadHandler)
    app.addSaveHandler(taskview.saveHandler, priority=6)


def unload(app):
    pass


```
