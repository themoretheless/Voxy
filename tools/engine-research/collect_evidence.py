#!/usr/bin/env python3
"""Pin candidates and collect README/root evidence, without executing foreign code."""
import argparse
import concurrent.futures
import hashlib
import json
import pathlib
import subprocess
import urllib.parse
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[2]
OUT = ROOT / 'docs/engine-research'
CACHE = OUT / 'evidence'

def api(endpoint):
    response = subprocess.run(['gh', 'api', endpoint], capture_output=True, text=True, timeout=60)
    if response.returncode:
        raise RuntimeError(f'GitHub API failed for {endpoint}: exit {response.returncode}')
    return json.loads(response.stdout)

def inspect(repo):
    destination = CACHE / str(repo['id'])
    if (destination / 'record.json').exists():
        return 'cached', repo['full_name']
    try:
        name = repo['full_name']
        commit = api(f'repos/{name}/commits/{urllib.parse.quote(repo["default_branch"], safe="")}')['sha']
        entries = api(f'repos/{name}/contents?ref={commit}')
        readmes = [entry for entry in entries if entry['type'] == 'file' and entry['name'].lower().startswith('readme')]
        readmes.sort(key=lambda entry: (entry['name'].lower() != 'readme.md', len(entry['name'])))
        content = b''
        readme_path = None
        if readmes:
            readme_path = readmes[0]['path']
            url = f'https://raw.githubusercontent.com/{name}/{commit}/{urllib.parse.quote(readme_path)}'
            with urllib.request.urlopen(url, timeout=45) as response:
                content = response.read(1024 * 1024 + 1)
            if len(content) > 1024 * 1024:
                raise RuntimeError('README exceeds 1 MiB evidence cap')
        record = dict(repository=name, repository_id=repo['id'], commit=commit,
                      source_url=f'https://github.com/{name}/tree/{commit}',
                      readme_path=readme_path, readme_sha256=hashlib.sha256(content).hexdigest(),
                      entries=[dict(path=e['path'], type=e['type'], sha=e['sha']) for e in entries],
                      classification='pending_manual_review', source_review='root_and_readme_only')
        destination.mkdir(parents=True, exist_ok=True)
        (destination / 'README.txt').write_bytes(content)
        temp = destination / 'record.tmp'
        temp.write_text(json.dumps(record, ensure_ascii=False, indent=2)+'\n')
        temp.replace(destination / 'record.json')
        return 'collected', name
    except Exception as error:
        return 'failed', f'{repo["full_name"]}: {error}'

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--limit', type=int, default=100)
    parser.add_argument('--workers', type=int, default=6)
    args = parser.parse_args()
    repos = json.loads((OUT / 'candidates.json').read_text())['repositories']
    CACHE.mkdir(parents=True, exist_ok=True)
    pending = [r for r in repos if not (CACHE / str(r['id']) / 'record.json').exists()][:args.limit]
    counts = {}
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as pool:
        for status, name in pool.map(inspect, pending):
            counts[status] = counts.get(status, 0) + 1
            print(status, name, flush=True)
    print(json.dumps(counts), flush=True)

if __name__ == '__main__':
    main()
