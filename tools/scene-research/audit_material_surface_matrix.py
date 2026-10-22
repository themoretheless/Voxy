#!/usr/bin/env python3
"""Compare actual exact/deferred GPU results against complete exact query records."""
import argparse, hashlib, json, struct
from pathlib import Path

CONFIGS = ['full-unknown', 'full-known', 'deferred-unknown', 'deferred-known',
           'native-inline-known', 'native-separate-known', 'previous-known',
           'native-inline-unknown']
PROTECTED = [i for i in range(20) if i not in (7, 11, 15)]
SIGNATURE = [i for i in PROTECTED if i not in (18, 19)]
CORE = [0, 1, 2, 6, 7, *range(8, 16), 17]

def rows(path, words=20):
    raw = path.read_bytes()
    assert len(raw) % (words*4) == 0, path
    return list(struct.iter_unpack('<'+str(words)+'I', raw))

def signature(row):
    return tuple(row[i] for i in SIGNATURE)

def stem(run):
    return f"{run['policy']}-order{str(run['certify_order']).lower()}-replay{run['replay']}"

def configurations(root):
    path=root/'matrix-extra-configurations.json'
    extra=json.loads(path.read_text()) if path.exists() else []
    assert isinstance(extra,list) and all(isinstance(name,str) and name and name not in ('.','..') and '/' not in name and '\\' not in name for name in extra)
    assert len(set(CONFIGS+extra))==len(CONFIGS)+len(extra),'duplicate matrix configuration'
    return CONFIGS+extra


def audit(root):
    digest = json.loads((root / 'binary.json').read_text())['sha256']
    assert hashlib.sha256((root / 'voxy_ray_probe').read_bytes()).hexdigest() == digest
    proofs = {}
    for name in configurations(root):
        directory = root / name
        process = json.loads((directory / 'process.json').read_text())
        assert process['state'] == 'terminal' and process['returncode'] == 0, name
        p = json.loads((directory / 'data/proof.json').read_text())
        assert p['schema'] == 'voxy-material-surface-nearest-v2'
        assert not p['render_consumer_connected'] and not p['fps_qualified']
        assert p['deferred'] == (not name.startswith('full-'))
        assert p['known_material_table'] == (name.endswith('-known'))
        proofs[name] = p
    references = {}
    for known in (False, True):
        name = 'full-known' if known else 'full-unknown'
        directory = root / name / 'data'
        for policy in ('geometric', 'material'):
            run = next(r for r in proofs[name]['runs'] if r['policy'] == policy and not r['certify_order'])
            key = stem(run)
            inputs = rows(directory / (key + '-input-before.bin'))
            outputs = rows(directory / (key + '.bin'), proofs[name].get('output_record_words',20))
            assert len(inputs) == len(outputs) == len(proofs[name]['cases'])
            reference = {}
            for ray, hit in zip(inputs, outputs):
                sig = signature(ray)
                value = tuple(hit[i] for i in CORE) + (hit[18] & 0x1e,)
                if sig in reference:
                    assert reference[sig] == value, 'inconsistent complete exact reference'
                reference[sig] = value
            references[known, policy] = reference
    checked = 0
    results = []
    changed_ray_recertified = False
    for name, p in proofs.items():
        directory = root / name / 'data'
        native_used = previous_used = 0
        for run in p['runs']:
            key = stem(run)
            before = rows(directory / (key + '-input-before.bin'))
            after = rows(directory / (key + '-input-after.bin'))
            output_path = directory / (key + '.bin')
            outputs = rows(output_path, p.get('output_record_words',20))
            assert len(before) == len(after) == len(outputs) == run['rays']
            routes = [0, 0, 0, 0]
            ref = references[p['known_material_table'], run['policy']]
            for index, (old, new, hit) in enumerate(zip(before, after, outputs)):
                indices = PROTECTED if p['deferred'] else range(20)
                assert all(old[i] == new[i] for i in indices), (name, key, index, 'protected ray changed')
                assert (hit[4], hit[5]) == (old[18], old[19]), (name, key, index, 'ray identity')
                expected = ref[signature(old)]
                actual = tuple(hit[i] for i in CORE) + (hit[18] & 0x1e,)
                assert actual == expected, (name, key, index, 'state/reason/source/tuv/validity/material differs', actual, expected)
                for i, bit in enumerate((2048, 4096, 8192, 16384)):
                    routes[i] += bool(hit[18] & bit)
                native_used += bool(hit[18] & 16384)
                previous_used += bool(hit[18] & 32768)
                if p.get('previous_ray_changed') and run['replay'] == 1 and index == 0 and run['policy'] == 'material':
                    baseline = rows(directory / f"material-order{str(run['certify_order']).lower()}-replay0-input-before.bin")[0]
                    assert old[18:20] == baseline[18:20] and signature(old) != signature(baseline)
                    assert hit[0] == 1 and hit[6] == 1 and hit[8] == 0x3f400000
                    assert not hit[18] & 32768 and hit[18] & 4096
                    changed_ray_recertified = True
            assert routes == run['routes_miss_winner_candidates_native'], (name, key, 'route report differs from raw records')
            checked += len(outputs)
            results.append(dict(configuration=name, run=key, rays=len(outputs), routes=routes,
                                output_sha256=hashlib.sha256(output_path.read_bytes()).hexdigest()))
        assert native_used == p['native_hints_consumed'] and previous_used == p['previous_hits_consumed']
        if p['native']:
            assert native_used > 0
        if p['previous']:
            assert previous_used > 0 and p.get('previous_ray_changed')
    assert changed_ray_recertified
    return dict(status='exact_material_surface_matrix_passed', checked_query_results=checked,
                core_fields_compared=CORE, material_event_mask=0x1e,
                diagnostics_excluded=['arena_peak', 'candidate_count', 'route_flags', 'tie_work_count'],
                complete_t_u_v_pairs_and_residuals_identical=True,
                stable_ray_ids_preserved=True, changed_ray_old_back_face_rejected=True,
                protected_input_words=PROTECTED, raw_route_counts_match=True, runs=results,
                render_consumer_connected=False, full_scene_qualified=False, fps_qualified=False)

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('root', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = audit(args.root)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({k: v for k, v in result.items() if k != 'runs'}))
