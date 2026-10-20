#!/usr/bin/env python3
"""Compare hierarchy variants without treating diagnostic counters as geometry."""
import argparse
import hashlib
import json
import struct
from pathlib import Path

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('reference', type=Path)
p.add_argument('candidate', type=Path)
p.add_argument('--output', type=Path, required=True)
a = p.parse_args()
manifest = json.loads((a.reference / 'manifest.json').read_text())
assert manifest == json.loads((a.candidate / 'manifest.json').read_text())
source_bytes = 0
for mesh in manifest['meshes']:
    for field in ['vertices', 'indices', 'instances']:
        name = mesh[field]['file']
        raw = (a.reference / name).read_bytes()
        assert raw == (a.candidate / name).read_bytes(), name
        source_bytes += len(raw)
images = []
for prefix in ['full-frame', 'exact-image-hit', 'optical-image-hit',
               'one-sided-image-hit', 'tail-image-hit']:
    for channel in ['gi', 'ssr', 'native']:
        name = f'{prefix}-{channel}.bin'
        raw = (a.reference / name).read_bytes()
        assert raw == (a.candidate / name).read_bytes(), name
        images.append({'name': name, 'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest()})
frames = [json.loads((root / 'world-frame.json').read_text())
          for root in [a.reference, a.candidate]]
assert frames[0]['size'] == frames[1]['size'] and frames[0]['tiles'] == frames[1]['tiles']
rows = exact_hits = 0
unresolved = {}
for kind in range(2):
    seen = set()
    for index, (_, _, width, height) in enumerate(frames[0]['tiles']):
        stem = f'frame-{kind}-tile-{index}'
        data = []
        for root in [a.reference, a.candidate]:
            inputs = list(struct.iter_unpack('<20I', (root / f'{stem}-input.bin').read_bytes()))
            raw = (root / f'{stem}-hits.bin').read_bytes()
            stride = len(raw) // (width * height * 4)
            assert stride in [20, 28]
            hits = list(struct.iter_unpack('<' + 'I' * stride, raw))
            assert len(inputs) == len(hits) == width * height
            data.append((inputs, hits))
        for br, bh, cr, ch in zip(data[0][0], data[0][1], data[1][0], data[1][1]):
            assert all(br[i] == cr[i] for i in range(20) if i not in [7, 11, 15])
            assert bh[4:6] == ch[4:6] == br[18:20]
            assert bh[4:6] not in seen
            seen.add(bh[4:6])
            assert bh[0] == ch[0]
            if bh[0] == 1:
                assert bh[6:16] == ch[6:16] and bh[17] == ch[17]
                assert bh[18] & 12 == ch[18] & 12
                assert bh[20:] == ch[20:]
                exact_hits += 1
            elif bh[0] == 2:
                assert frames[0]['history_admission'] and frames[1]['history_admission']
                assert bh[1] == ch[1]
                assert bh[6:18] == ch[6:18] and bh[19:] == ch[19:]
                assert (bh[18] ^ ch[18]) & ~65536 == 0
                unresolved[bh[1]] = unresolved.get(bh[1], 0) + 1
            rows += 1
    assert len(seen) == frames[0]['size'][0] * frames[0]['size'][1]
report = {'source_bytes_identical': source_bytes, 'original_queries_once': rows,
          'canonical_hits_and_uv_identical': exact_hits, 'matching_unresolved_reasons': unresolved, 'images_bit_identical': images,
          'diagnostic_work_counters_may_differ': True, 'fps_qualified': False}
a.output.write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report))
