#!/usr/bin/env python3
"""Independent rational oracle for optional GPU world-position tails.

The two non-dyadic cases derive t from the fixture's original z plane, never
from rounded GPU parameter words. Existing dyadic cases require zero t tail.
"""
import argparse
import json
import struct
from pathlib import Path
from audit_material_surface_matrix import CONFIGS, configurations, CORE, rows, stem, audit, signature
from audit_world_triangle_case import word
from make_world_exact_parameter_gold import pair


def check(root, legacy=None):
    comparison = audit(root)
    legacy_checked = 0
    if legacy is not None:
        audit(legacy)
        for name in CONFIGS:
            old_dir, new_dir = legacy/name/'data', root/name/'data'
            old_proof = json.loads((old_dir/'proof.json').read_text())
            for run in old_proof['runs']:
                key = stem(run)
                old_rays = rows(old_dir/(key+'-input-before.bin'))
                new_rays = rows(new_dir/(key+'-input-before.bin'))
                old_hits = rows(old_dir/(key+'.bin'))
                new_hits = rows(new_dir/(key+'.bin'), json.loads((new_dir/'proof.json').read_text()).get('output_record_words',20))
                assert len(new_rays) >= len(old_rays)
                for a,b,x,y in zip(old_rays,new_rays,old_hits,new_hits):
                    assert signature(a) == signature(b)
                    assert tuple(x[i] for i in CORE)+(x[18]&0x1e,) == tuple(y[i] for i in CORE)+(y[18]&0x1e,), (name,key,'position option changed query semantics')
                    legacy_checked += 1
    checked = hits = non_dyadic_exact = non_dyadic_residual = partial_checks = 0
    for name in configurations(root):
        directory = root / name / 'data'
        proof = json.loads((directory / 'proof.json').read_text())
        assert proof['hit_position']
        for run in proof['runs']:
            key = stem(run)
            rays = rows(directory / (key + '-input-before.bin'))
            results = rows(directory / (key + '.bin'), proof.get('output_record_words',20))
            positions = list(struct.iter_unpack('<8I', (directory / (key + '-position.bin')).read_bytes()))
            assert len(positions) == len(rays)
            if proof.get('partial_position_check'):
                raw = (directory / (key + '-partial.bin')).read_bytes()
                primary = (directory / (key + '.bin')).read_bytes()
                tail = (directory / (key + '-position.bin')).read_bytes()
                size = len(primary)
                assert len(raw) == size + len(tail)
                assert struct.unpack_from('<I', raw)[0] == 3
                record_bytes = proof.get('output_record_words',20)*4
                assert raw[record_bytes:size] == primary[record_bytes:]
                assert raw[size:size+32] == bytes(32)
                assert raw[size+32:] == tail[32:]
                partial_checks += 1
            for index, (ray, hit, position) in enumerate(zip(rays, results, positions)):
                case = proof['cases'][index]
                if proof.get('hit_uv'):
                    assert hit[20:28] == (0,0,0,0,3 if hit[0]==1 else 0,0,0,0)
                checked += 1
                if hit[0] != 1:
                    assert position == (0,) * 8, (name, key, case, 'stale position')
                    continue
                origin = [word(ray[k]) + word(ray[k+4]) for k in range(3)]
                direction = [word(ray[k+8]) + word(ray[k+12]) for k in range(3)]
                if case.startswith('rational '):
                    assert hit[6] in (0, 1)
                    z_plane = word(0x3f000000) if hit[6] == 1 else 0
                    t = (z_plane-origin[2])/direction[2]
                    expected_t, tail = pair(t)
                    assert tail and list(hit[8:10]) == expected_t and hit[15] & 1
                else:
                    assert not hit[15] & 1, (case, 'fixture needs source rational t')
                    t = word(hit[8]) + word(hit[9])
                expected = []
                residual = 0
                for axis in range(3):
                    components, tail = pair(origin[axis] + direction[axis]*t)
                    expected.extend(components)
                    residual |= int(tail) << axis
                expected.extend([7, residual])
                assert list(position) == expected, (name, key, case, position, expected)
                if case.startswith('rational '):
                    non_dyadic_exact += residual == 0
                    non_dyadic_residual += residual != 0
                hits += 1
    assert non_dyadic_exact > 0 and non_dyadic_residual > 0
    return dict(status='world_position_rational_oracle_passed', checked_records=checked,
                checked_hit_positions=hits, legacy_query_results_unchanged=legacy_checked, partial_hit_to_inactive_checks=partial_checks, non_dyadic_t_exact_positions=non_dyadic_exact,
                non_dyadic_t_residual_positions=non_dyadic_residual,
                source_query_comparison=comparison, render_consumer_connected=False,
                full_scene_qualified=False, fps_qualified=False)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--legacy-reference', type=Path)
    args = parser.parse_args()
    report = check(args.root, args.legacy_reference)
    args.output.write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps({k:v for k,v in report.items() if k!='source_query_comparison'}))
