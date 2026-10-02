#!/usr/bin/env python3
"""Read-only corpus audit; validation remains enabled under python -O."""
import argparse
import collections
import hashlib
import json
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[2] / 'docs/engine-research'
ACCEPTED = {'engine', 'specialized_engine', 'derived_engine'}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def audit_mechanisms(root):
    manifests = 0
    sources = 0
    for path in sorted((root / 'mechanisms').glob('*/sources.json')):
        manifest = json.loads(path.read_text())
        repository, commit = manifest['repository'], manifest['commit']
        require(re.fullmatch(r'[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+', repository),
                f'invalid mechanism repository: {path}')
        require(re.fullmatch('[0-9a-f]{40}', commit), f'unpinned mechanism: {path}')
        seen = set()
        require(manifest['sources'], f'empty source manifest: {path}')
        for source in manifest['sources']:
            name = source['file']
            require(pathlib.PurePath(name).name == name and name not in {'.', '..'},
                    f'unsafe mechanism filename: {name}')
            require(name not in seen, f'duplicate mechanism source: {name}')
            seen.add(name)
            remote = pathlib.PurePosixPath(source['path'])
            require(not remote.is_absolute() and '..' not in remote.parts,
                    f'unsafe remote source path: {remote}')
            require(source['url'] == f"https://raw.githubusercontent.com/{repository}/{commit}/{source['path']}",
                    f'unpinned mechanism URL: {name}')
            require(hashlib.sha256((path.parent / name).read_bytes()).hexdigest() == source['sha256'],
                    f'mechanism digest mismatch: {name}')
            sources += 1
        manifests += 1
    return dict(manifests=manifests, verified_sources=sources)


def audit(root):
    candidates = json.loads((root / 'candidates.json').read_text())['repositories']
    repositories = {repo['id']: repo for repo in candidates}
    require(len(repositories) == len(candidates), 'duplicate candidate IDs')
    require(len({repo['full_name'].casefold() for repo in candidates}) == len(candidates),
            'duplicate candidate names')
    records = {}
    for path in sorted((root / 'evidence').glob('*/record.json')):
        record = json.loads(path.read_text())
        identity = record['repository_id']
        require(identity in repositories, f'evidence outside corpus: {identity}')
        require(identity not in records, f'duplicate evidence: {identity}')
        require(path.parent.name == str(identity), f'wrong evidence directory: {path}')
        repo = repositories[identity]
        require(record['repository'] == repo['full_name'], f'identity mismatch: {identity}')
        commit = record['commit']
        require(re.fullmatch('[0-9a-f]{40}', commit), f'invalid commit: {identity}')
        require(record['source_url'] == f"{repo['html_url']}/tree/{commit}",
                f'unpinned or wrong source URL: {identity}')
        digest = hashlib.sha256((path.parent / 'README.txt').read_bytes()).hexdigest()
        require(digest == record['readme_sha256'], f'README digest mismatch: {identity}')
        if record['classification'] in ACCEPTED:
            require(record.get('classification_reason'), f'missing acceptance reason: {identity}')
            require(record.get('entries'), f'missing root evidence: {identity}')
        records[identity] = record
    mirror = json.loads((root / 'classifications.json').read_text())
    mirrored = set()
    for record in mirror:
        identity = record['repository_id']
        require(identity not in mirrored, f'duplicate classification: {identity}')
        require(records.get(identity) == record, f'classification drift: {identity}')
        mirrored.add(identity)
    # Unreviewed records need not yet occur in the classification mirror.
    require(all(record['classification'] == 'pending_manual_review' or identity in mirrored
                for identity, record in records.items()), 'classified evidence missing from mirror')
    counts = collections.Counter(record['classification'] for record in records.values())
    accepted = sum(counts[category] for category in ACCEPTED)
    status = json.loads((root / 'status.json').read_text())
    require(status['candidates'] == len(candidates), 'stale candidate count')
    require(status['pinned_readme_root_evidence'] == len(records), 'stale evidence count')
    require(status['classification_counts'] == dict(counts), 'stale classification counts')
    require(status['accepted_game_engines'] == accepted, 'stale accepted count')
    require(status['non_derived_game_engine_records'] == counts['engine'] + counts['specialized_engine'],
            'stale non-derived count')
    return dict(candidates=len(candidates), verified_pinned_records=len(records),
                mechanism_integrity=audit_mechanisms(root),
                accepted_engine_identities=accepted, derived_engine_identities=counts['derived_engine'],
                review_depth_counts=dict(collections.Counter(record.get('source_review', 'unspecified')
                                                           for record in records.values())),
                scope='Saved identity/evidence integrity; not build, runtime or architectural parity proof.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=pathlib.Path, default=ROOT)
    print(json.dumps(audit(parser.parse_args().root), indent=2))


if __name__ == '__main__':
    main()
