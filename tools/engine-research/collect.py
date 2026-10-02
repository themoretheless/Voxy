#!/usr/bin/env python3
"""Reproducible public GitHub candidate discovery; metadata is not source review."""
import argparse
import json
import pathlib
import subprocess
import time
import urllib.parse
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[2]
OUT = ROOT / 'docs/engine-research'
QUERIES = ['topic:game-engine fork:false', '"game engine" in:description fork:false', 'topic:3d-engine fork:false', 'topic:2d-game-engine fork:false']

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--target', type=int, default=1400)
    args = parser.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    snapshot = OUT / "candidates.json"
    previous = json.loads(snapshot.read_text()) if snapshot.exists() else {}
    candidates = {item["id"]: item for item in previous.get("repositories", [])}
    evidence = previous.get("provenance", [])
    completed = {(item["query"], item["page"]) for item in evidence}
    for query in QUERIES:
        for page in range(1, 11):
            if (query, page) in completed:
                continue
            params = urllib.parse.urlencode(dict(q=query, sort='stars', order='desc', per_page=100, page=page))
            url = 'https://api.github.com/search/repositories?' + params
            request = urllib.request.Request(url, headers={'User-Agent': 'Voxy-engine-research', 'Accept': 'application/vnd.github+json'})
            try:
                with urllib.request.urlopen(request, timeout=40) as response:
                    data = json.load(response)
            except Exception:
                try:
                    result = subprocess.run(["gh", "api", "search/repositories?" + params], capture_output=True, text=True, check=True, timeout=40)
                    data = json.loads(result.stdout)
                except Exception as error:
                    print(f'Query failed: {query} page {page}: {error}', flush=True)
                    break
            evidence.append(dict(query=query, page=page, url=url, fetched_at=time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()), incomplete_results=data.get('incomplete_results')))
            for item in data['items']:
                if item['fork']:
                    continue
                record = {key: item.get(key) for key in ['id', 'full_name', 'html_url', 'description', 'language', 'stargazers_count', 'archived', 'pushed_at', 'default_branch', 'license', 'topics', 'size']}
                record['review_status'] = 'candidate_metadata_only'
                record['discovery_query'] = query
                candidates.setdefault(item['id'], record)
            (OUT / 'candidates.json').write_text(json.dumps(dict(provenance=evidence, repositories=list(candidates.values())), ensure_ascii=False, indent=2) + '\n')
            print(f'{query} page={page}: unique={len(candidates)}', flush=True)
            if len(candidates) >= args.target:
                return
            if len(data['items']) < 100:
                break
            time.sleep(7)

if __name__ == '__main__':
    main()
