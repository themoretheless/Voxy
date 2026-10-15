# Facial rendering source scan: smogelmose/MLMP

Repository: https://github.com/smogelmose/MLMP

Pinned source revision: 66c23126a0aa0d30aa3597f5fc7a32b11265e647

Discovery topic: ARKit blendshape

Description: ARKit Facial Expression Blendshape Dataset

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: arkit_to_edgeimpulse.py

{
  "path": "arkit_to_edgeimpulse.py",
  "blob_sha": "6fc87ebb69dfb4749b0c484f65d0340677116318",
  "sha256": "3f9a4a62cacdbbcfd73b9492417049978c25004aad85315484d01f81a6a65f15",
  "lines": 76,
  "bytes": 2070,
  "selection_score": 0,
  "keyword_line_counts": {
    "blendshape": 1,
    "gum": 2
  },
  "url": "https://github.com/smogelmose/MLMP/blob/66c23126a0aa0d30aa3597f5fc7a32b11265e647/arkit_to_edgeimpulse.py",
  "function_signature_candidates": [
    {
      "line": 6,
      "text": "def parse_timecode_60fps(timecode):"
    },
    {
      "line": 21,
      "text": "def convert_csv(input_csv):"
    },
    {
      "line": 44,
      "text": "def process_folder(folder_path):"
    },
    {
      "line": 54,
      "text": "def main():"
    }
  ]
}

```text

import os
import pandas as pd
import re
import argparse

def parse_timecode_60fps(timecode):
    match = re.match(r"(\d+):(\d+):(\d+):(\d+)\.(\d+)", timecode)
    if not match:
        raise ValueError(f"Invalid timecode: {timecode}")
    HH, MM, SS, FF, mmm = map(int, match.groups())
    total_ms = (
        HH * 3600000 +
        MM * 60000 +
        SS * 1000 +
        FF * (1000 / 60) +
        mmm
    )
    return total_ms


def convert_csv(input_csv):
    print(f"Checking: {input_csv}")

    df = pd.read_csv(input_csv)

    if "Timecode" not in df.columns:
        print(f"⚠️ Skipping (no Timecode column): {input_csv}\n")
        return

    print(f"Converting: {input_csv}")

    timestamps = df["Timecode"].apply(parse_timecode_60fps)
    timestamps = timestamps - timestamps.iloc[0]

    df_clean = df.drop(columns=["Timecode", "BlendshapeCount"], errors="ignore")
    df_clean.insert(0, "timestamp", timestamps)

    out_csv = input_csv.replace(".csv", "_edgeimpulse.csv")
    df_clean.to_csv(out_csv, index=False)

    print(f"✓ Saved: {out_csv}\n")


def process_folder(folder_path):
    print(f"\n📁 Scanning folder: {folder_path}\n")

    for root, dirs, files in os.walk(folder_path):
        for file in files:
            if file.lower().endswith(".csv") and "_edgeimpulse" not in file:
                full_path = os.path.join(root, file)
                convert_csv(full_path)


def main():
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("path", nargs="?", default=None)
    args, unknown = parser.parse_known_args()

    if args.path:
        if os.path.isdir(args.path):
            process_folder(args.path)
        elif os.path.isfile(args.path):
            convert_csv(args.path)
        else:
            print(f"❌ Path not found: {args.path}")
        return

    script_dir = os.path.dirname(os.path.abspath(__file__))
    print("No path given → auto-processing current script folder:")
    print(f"👉 {script_dir}\n")

    process_folder(script_dir)


if __name__ == "__main__":
    main()


```
