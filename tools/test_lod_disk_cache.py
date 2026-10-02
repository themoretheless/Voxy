#!/usr/bin/env python3
"""Real OBJ importer disk cache acceptance; requires built certify_obj_lod."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / 'target/debug/examples/certify_obj_lod'
FIXTURES = ROOT / 'docs/engine-research/mechanisms/godot-lod'


def main():
    with tempfile.TemporaryDirectory(prefix='voxy-lod-cache-') as temporary:
        root = Path(temporary)
        base = root / 'base.obj'
        variant = root / 'variant.obj'
        shutil.copyfile(FIXTURES / 'flat-grid-base.obj', base)
        shutil.copyfile(FIXTURES / 'flat-grid-coarse.obj', variant)
        cache = root / 'cache'

        def run(depth=1, success=True):
            result = subprocess.run([str(BINARY), str(base), str(variant), '1000000', str(depth), 'indexed', str(cache)], capture_output=True, text=True, timeout=60)
            if (result.returncode == 0) != success:
                raise RuntimeError(result.stdout + result.stderr)
            return result

        first = run()
        assert 'cache_hit=false' in first.stderr
        hit = run()
        assert 'cache_hit=true' in hit.stderr
        assert hit.stdout == first.stdout
        assert 'triangle_tests=' not in hit.stderr
        assert len(list(cache.glob('*.artifact'))) == 1
        options = run(2)
        assert 'cache_hit=false' in options.stderr
        assert len(list(cache.glob('*.artifact'))) == 2
        base.write_bytes(base.read_bytes() + b'\n# source identity change\n')
        changed = run()
        assert 'cache_hit=false' in changed.stderr
        assert changed.stdout == first.stdout
        assert len(list(cache.glob('*.artifact'))) == 3
        assert 'cache_hit=true' in run().stderr
        before = {p.name: p.read_bytes() for p in cache.glob('*.artifact')}
        for p in cache.glob('*.artifact'):
            data = bytearray(p.read_bytes())
            data[-1] ^= 1
            p.write_bytes(data)
        damaged = {p.name: p.read_bytes() for p in cache.glob('*.artifact')}
        rejected = run(success=False)
        assert 'Corrupt' in rejected.stderr
        assert damaged == {p.name: p.read_bytes() for p in cache.glob('*.artifact')}
        assert not list(cache.glob('*.part'))
        print(json.dumps({'binary_sha256': hashlib.sha256(BINARY.read_bytes()).hexdigest(), 'miss_hit_equal': True, 'hit_skips_search': True, 'option_invalidation': True, 'source_invalidation': True, 'corruption_rejected_without_replacement': True, 'entries_retained': len(before)}, indent=2))


if __name__ == '__main__':
    main()
