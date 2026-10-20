#!/usr/bin/env python3
"""Compare source history admission to independently qualified nearest records."""
import argparse
import hashlib
import json
import struct
from fractions import Fraction
from pathlib import Path

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('reference', type=Path)
p.add_argument('filtered', type=Path)
p.add_argument('--output', type=Path, required=True)
a = p.parse_args()

def records(path):
    return list(struct.iter_unpack('<20I', path.read_bytes()))

images = []
for prefix in ['full-frame', 'exact-image-hit', 'optical-image-hit',
               'one-sided-image-hit', 'tail-image-hit']:
    for channel in ['gi', 'ssr', 'native']:
        name = f'{prefix}-{channel}.bin'
        original = (a.reference / name).read_bytes()
        assert original == (a.filtered / name).read_bytes(), name
        images.append({'name': name, 'bytes': len(original),
                       'sha256': hashlib.sha256(original).hexdigest()})

frame = json.loads((a.reference / 'world-frame.json').read_text())
states = {}
for kind in range(2):
    gold = {r['global_ray']: r for r in json.loads(
        (a.reference / f'full-frame-{kind}-gold.json').read_text())['rows']}
    seen = set()
    for tile in range(len(frame['tiles'])):
        stem = f'frame-{kind}-tile-{tile}'
        old_in, new_in = [records(root / f'{stem}-input.bin')
                          for root in [a.reference, a.filtered]]
        old, new = [records(root / f'{stem}-hits.bin')
                    for root in [a.reference, a.filtered]]
        assert len(old) == len(new) == len(old_in) == len(new_in)
        for before, after, ray, other in zip(old, new, old_in, new_in):
            assert all(ray[i] == other[i] for i in range(20) if i not in [7, 11, 15])
            key = ray[18] | ray[19] << 32
            assert key not in seen
            seen.add(key)
            assert after[4:6] == before[4:6] == ray[18:20]
            states[after[0]] = states.get(after[0], 0) + 1
            if after[0] == 2:
                # Only this directed scene's non-dyadic reason is admitted.
                assert before[0] == 1
                if after[1] == 8192:
                    candidate_frame = json.loads((a.filtered / 'world-frame.json').read_text())
                    manifest = json.loads((a.filtered / 'manifest.json').read_text())
                    assert candidate_frame['any_hit_admission_enabled']
                    assert manifest['materials'] and all(not m['double_sided'] for m in manifest['materials'])
                else:
                    assert after[1] == 4096 and before[15] & 1
                    distance = Fraction(gold[key]['exact_parameters'][0])
                    assert distance.denominator % 3 == 0 or distance.denominator % 5 == 0
            else:
                assert after[0] == before[0]
                if after[0] == 1:
                    assert after[6:18] == before[6:18]
    assert len(seen) == frame['size'][0] * frame['size'][1]

for prefix, reason in [('exact', None), ('optical', 2048), ('one-sided', 1024), ('tail', 4096)]:
    old, new = [records(root / f'{prefix}-image-hit-output.bin')
                for root in [a.reference, a.filtered]]
    changed = 0
    for before, after in zip(old, new):
        assert before[4:6] == after[4:6]
        if before[0] == 1:
            if reason is None:
                assert after[0] == 1 and before[6:18] == after[6:18]
            else:
                assert after[0] == 2
                fixture = json.loads((a.filtered / f'{prefix}-image-hit-fixture.json').read_text())
                expected_reason = 8192 if prefix == 'one-sided' and fixture.get('any_hit_admission_enabled') else reason
                assert after[1] == expected_reason
                changed += 1
        else:
            assert before[0] == after[0]
    assert reason is None or changed > 0

raw = (a.filtered / 'modular-input.bin').read_bytes()
gpu = list(struct.iter_unpack('<4I', (a.filtered / 'modular-output.bin').read_bytes()))
assert len(gpu) * 4 == len(raw)
for (word,), (echo, r3, r5, valid) in zip(struct.iter_unpack('<I', raw), gpu):
    assert echo == word
    finite = (word >> 23) & 255 != 255
    assert valid == int(finite)
    if finite:
        value = Fraction(struct.unpack('<f', struct.pack('<I', word))[0])
        residues = [value.numerator * pow(value.denominator, -1, prime) % prime
                    for prime in [3, 5]]
        assert [r3, r5] == residues
    else:
        assert [r3, r5] == [0, 0]

report = {'images_bit_identical': images, 'source_query_states': states,
          'gpu_modular_words_checked_independently': len(gpu),
          'precision_preserved_for_retained_parameters': True,
          'full_street_qualified': False, 'fps_qualified': False}
a.output.write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report))
