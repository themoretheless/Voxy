#!/usr/bin/env python3
"""Verify saved evidence integrity and produce an honest, reproducible review queue."""
import collections
import hashlib
import json
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[2]
OUT = ROOT / 'docs/engine-research'

def main():
    corpus = json.loads((OUT / 'candidates.json').read_text())
    repos = corpus['repositories']
    assert len({r['id'] for r in repos}) == len(repos), 'duplicate repository IDs'
    assert len({r['full_name'].lower() for r in repos}) == len(repos), 'duplicate repository names'
    records = {}
    for path in (OUT / 'evidence').glob('*/record.json'):
        record = json.loads(path.read_text())
        assert re.fullmatch('[0-9a-f]{40}', record['commit']), 'invalid pinned commit'
        assert hashlib.sha256((path.parent / 'README.txt').read_bytes()).hexdigest() == record['readme_sha256'], 'README digest mismatch'
        assert record['repository_id'] not in records, 'duplicate evidence ID'
        records[record['repository_id']] = record
    assert set(records).issubset({r['id'] for r in repos}), 'evidence outside corpus'
    counts = collections.Counter(r['classification'] for r in records.values())
    summary = dict(candidates=len(repos), pinned_readme_root_evidence=len(records),
                   classification_counts=dict(sorted(counts.items())),
                   accepted_game_engines=counts['engine'] + counts['specialized_engine'] + counts['derived_engine'],
                   non_derived_game_engine_records=counts['engine'] + counts['specialized_engine'],
                   target_game_engines=500,
                   note='Identity classification is not architectural review or build/runtime proof.')
    (OUT / 'status.json').write_text(json.dumps(summary, indent=2)+'\n')
    lines = ['# Engine repository research queue', '',
             f'Discovered candidates: {len(repos)}. Pinned README/root records: {len(records)}.',
             f'Classified game engines: {summary["accepted_game_engines"]}/500.',
             'Architectural review and implementation acceptance remain separate.', '',
             '| Repository | Classification | Pinned source / evidence |', '|---|---|---|']
    for repo in repos:
        record = records.get(repo['id'])
        category = record['classification'] if record else 'not_collected'
        source = f"[source]({record['source_url']}) / [record](evidence/{repo['id']}/record.json)" if record else 'pending'
        lines.append(f"| [{repo['full_name']}]({repo['html_url']}) | {category} | {source} |")
    (OUT / 'review-queue.md').write_text('\n'.join(lines)+'\n')
    catalog = ['# Repository discovery catalog', '', f'{len(repos)} candidates; classification and source-review status are in review-queue.md.', '', '| Repository | Language | Description |', '|---|---|---|']
    for repo in repos:
        description = (repo['description'] or '').replace('|', ' / ').replace('\n', ' ')
        catalog.append(f"| [{repo['full_name']}]({repo['html_url']}) | {repo['language'] or 'unknown'} | {description} |")
    (OUT / 'catalog.md').write_text('\n'.join(catalog)+'\n')
    print(json.dumps(summary))

if __name__ == '__main__':
    main()
