"""Run preserved native Rust executables in ABBA order; no physics in this script."""
from pathlib import Path
import hashlib, json, os, re, statistics, subprocess, time
root = Path(__file__).resolve().parents[2]
out = Path(__file__).resolve().parent
bins = {mode: Path('/tmp/voxy-implicit-evaluation-' + mode + '-20261007') for mode in ['before','after']}
manifest = root / 'artifacts/character-bind-pose-2026-10-07/startup-three-source-pins-strict-energy-regions.json'
env = dict(os.environ, VOXY_CAPTURE_NODE_STATE='1')
rows = []
reference = {suffix: (out / ('before-warmup' + suffix)).read_bytes() for suffix in ['.nodes.json', '.nodes.jsonl', '.energy.jsonl', '.csv']}
for index, mode in enumerate(['after', 'before', 'after', 'after', 'before', 'before', 'after', 'after', 'before']):
    warmup = index == 0
    stem = out / (('after-warmup' if warmup else f'{index:02d}-{mode}'))
    begin = time.perf_counter()
    with stem.with_suffix('.log').open('wb') as log:
        exit_code = subprocess.run([str(bins[mode]), str(stem.with_suffix('.png')), '--cesium', '--contact', '--capture-steps=3', '--tissue-regions='+str(manifest)], cwd=root, env=env, stdout=log, stderr=subprocess.STDOUT).returncode
    wall = time.perf_counter() - begin
    text = stem.with_suffix('.log').read_text()
    match = re.search(r'snapshot simulation \+ render seconds=([0-9.]+); step receipts=(.*)', text)
    same = {suffix: Path(str(stem)+suffix).exists() and Path(str(stem)+suffix).read_bytes() == data for suffix,data in reference.items()}
    row = {'index':index,'mode':mode,'warmup':warmup,'exit_code':exit_code,'wall_s':wall,'simulation_and_render_s':float(match[1]) if match else None,'step_receipts':match[2] if match else None,'exact_bytes_equal_baseline':same}
    rows.append(row)
    (out/'paired-observations.json').write_text(json.dumps(rows,indent=2)+'\n')
    print(json.dumps(row),flush=True)
    if exit_code != 0 or not all(same.values()) or match is None:
        raise SystemExit('Native capture did not pass exact baseline comparison; preserve outputs.')
measured = [row for row in rows if not row['warmup']]
medians = {mode:statistics.median(row['simulation_and_render_s'] for row in measured if row['mode']==mode) for mode in bins}
result = {'scope':'Eight alternating ABBA short full-volume captures; three 1/240s frames plus initial state. Not full 480-step clip or universal engine speedup.','measured_captures':8,'median_simulation_and_render_s':medians,'relative_elapsed_reduction':1-medians['after']/medians['before'],'all_states_energy_and_csv_byte_identical':True,'binary_sha256':{mode:hashlib.sha256(path.read_bytes()).hexdigest() for mode,path in bins.items()},'manifest_sha256':hashlib.sha256(manifest.read_bytes()).hexdigest(),'observations':rows}
(out/'paired-result.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({'medians':medians,'relative_elapsed_reduction':result['relative_elapsed_reduction']}),flush=True)
