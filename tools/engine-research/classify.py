#!/usr/bin/env python3
"""Apply explicit reviewed decisions; no automatic engine acceptance heuristics."""
import argparse
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[2]
OUT = ROOT / 'docs/engine-research'

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('decisions', type=pathlib.Path)
    args = parser.parse_args()
    decisions = json.loads(args.decisions.read_text())
    records = {json.loads(p.read_text())['repository']: p for p in (OUT/'evidence').glob('*/record.json')}
    assert set(decisions).issubset(records), 'classification needs pinned evidence'
    staged = []
    for name, decision in decisions.items():
        assert len(decision) == 2 and all(isinstance(v, str) and v for v in decision)
        path = records[name]
        record = json.loads(path.read_text())
        record.update(classification=decision[0], classification_reason=decision[1],
                      classification_basis='README and root source/build evidence inspected; no runtime verification')
        staged.append((path, record))
    for path, record in staged:
        temporary = path.with_suffix('.tmp')
        temporary.write_text(json.dumps(record, ensure_ascii=False, indent=2)+'\n')
        temporary.replace(path)
    reviewed = [json.loads(p.read_text()) for p in records.values()]
    reviewed = sorted((r for r in reviewed if r['classification'] != 'pending_manual_review'), key=lambda r:r['repository'])
    (OUT/'classifications.json').write_text(json.dumps(reviewed, ensure_ascii=False, indent=2)+'\n')
    print(f'Applied {len(staged)} explicit decisions; {len(reviewed)} classified records')

if __name__ == '__main__':
    main()
