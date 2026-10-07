"""Snapshot immutable observations and invoke native Rust audits, no physics in Python."""
from pathlib import Path
import json, os, subprocess, hashlib
root = Path(__file__).resolve().parent
binary = Path('/tmp/voxy-contact-derivatives-20261006/release/examples/body_motion_snapshot-46341a4b5e34a250')
end = 60
energy = (root / 'capture.energy.jsonl').read_bytes().splitlines(keepends=True)
assert len(energy) >= end + 1
energy_path = root / 'observed-through-step60-energy.jsonl'
energy_path.write_bytes(b''.join(energy[:end + 1]))
nodes = (root / 'capture.nodes.jsonl').read_bytes().splitlines(keepends=True)
nodes = [line for line in nodes if json.loads(line)['time_s'] <= end / 240.]
node_path = root / 'observed-through-step60-nodes.jsonl'
node_path.write_bytes(b''.join(nodes))
assert len(nodes) == 6
base = os.environ.copy()
base.update(VOXY_RIG_SUPPORTED_CAPTURE=str(node_path), VOXY_RIG_SUPPORTED_ENERGY_CAPTURE=str(energy_path), VOXY_RIG_SUPPORTED_CAPTURE_STEPS='480', VOXY_RIG_PREFIX_COMMITTED_STEPS=str(end), VOXY_RIG_SUPPORTED_ENERGY_RATE_J_S='5e-10')
results = []
def run(name, env, test, expected_success):
    result = subprocess.run([str(binary), 'collision_tests::' + test, '--exact', '--ignored', '--nocapture'], env=env, capture_output=True, text=True)
    (root / (name + '.log')).write_text(result.stdout + result.stderr)
    assert (result.returncode == 0) == expected_success, (name, result.returncode, result.stdout[-1000:], result.stderr[-1000:])
    results.append({'name':name,'exit_code':result.returncode,'expected_success':expected_success,'matched_expectation':True})
prefix = 'audits_committed_full_rig_nodes_and_energy_prefix'
complete = 'audits_completed_full_rig_supported_capture_against_native_clip'
run('observed-through-step60-node-audit',base,prefix,True)
missing_node = root / 'negative-missing-node.jsonl'
missing_node.write_bytes(b''.join(nodes[:-1]))
env = base.copy(); env['VOXY_RIG_SUPPORTED_CAPTURE'] = str(missing_node)
run('negative-missing-node',env,prefix,False)
missing_energy = root / 'negative-missing-energy.jsonl'
missing_energy.write_bytes(b''.join(energy[:end]))
env = base.copy(); env['VOXY_RIG_SUPPORTED_ENERGY_CAPTURE'] = str(missing_energy)
run('negative-missing-energy',env,prefix,False)
displaced = [json.loads(line) for line in nodes]
displaced[-1]['nodes'][1282][0][0] += 0.001
pin_path = root / 'negative-displaced-pin.jsonl'
pin_path.write_text(''.join(json.dumps(row) + '\n' for row in displaced))
env = base.copy(); env['VOXY_RIG_SUPPORTED_CAPTURE'] = str(pin_path)
run('negative-displaced-pin',env,prefix,False)
run('negative-incomplete-as-complete',base,complete,False)
env = base.copy(); env['VOXY_RIG_PREFIX_COMMITTED_STEPS'] = '480'
run('negative-full-endpoint-as-prefix',env,prefix,False)
short_root = root.parent / 'factored-kinetic-energy-2026-10-07'
env = base.copy(); env.update(VOXY_RIG_SUPPORTED_CAPTURE=str(short_root / 'short.nodes.jsonl'), VOXY_RIG_SUPPORTED_ENERGY_CAPTURE=str(short_root / 'short.energy.jsonl'), VOXY_RIG_SUPPORTED_CAPTURE_STEPS='3')
run('completed-short-audit-after-prefix-refactor',env,complete,True)
report = {'status':'native audit expectations passed','observed_committed_steps':end,'requested_steps':480,'node_checkpoints':len(nodes),'full_clip_qualified':False,'results':results,'fixture_sha256':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in [energy_path,node_path]},'native_audit_binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest()}
(root / 'observed-through-step60-audit-result.json').write_text(json.dumps(report,indent=2) + '\n')
print(json.dumps({'checks':len(results),'observed_steps':end,'full_clip_qualified':False}))
