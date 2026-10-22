#!/usr/bin/env python3
"""Run a frozen test executable into a NEW evidence directory; never signal rivals."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import threading
import time

parser = argparse.ArgumentParser()
parser.add_argument('--binary', required=True, type=Path)
parser.add_argument('--output', required=True, type=Path)
parser.add_argument('--input-only', action='store_true')
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=False)
binary = args.binary.resolve()
(args.output / 'binary.sha256').write_text(hashlib.sha256(binary.read_bytes()).hexdigest()+'\n')

def snapshot():
    rows = subprocess.check_output(['ps', '-axo', 'pid,ppid,state,comm'], text=True).splitlines()
    tokens = ['rustc','cargo','street_walk','cpu-replay','sdf_core','voxy_app','llama','ollama','allpaka','liquid_snapshot']
    clients = [r for r in rows if any(s in r for s in tokens)]
    return dict(time=time.time(), clients=clients)

for run in range(1, 3):
    deadline, quiet = time.monotonic()+120, 0
    while quiet < 10:
        sample = snapshot()
        busy = any('rustc' in r or 'cpu-replay' in r for r in sample['clients'])
        if not args.input_only:
            busy |= any(any(s in r for s in ['street_walk','sdf_core','voxy_app','llama','allpaka','liquid_snapshot']) for r in sample['clients'])
            agx = subprocess.check_output(['ioreg','-r','-c','AGXAccelerator','-d','1'], text=True)
            utilization = re.search(r'"Device Utilization %"=(\d+)', agx)
            busy |= utilization is None or int(utilization[1]) > 10
        quiet = 0 if busy else quiet+1
        if time.monotonic() >= deadline:
            (args.output / 'blocked.json').write_text(json.dumps(sample, indent=2)+'\n')
            raise SystemExit('No qualifying quiet window; competitors were left running.')
        time.sleep(.2)
    stop, samples = threading.Event(), []
    def monitor():
        while not stop.is_set():
            samples.append(snapshot())
            stop.wait(.2)
    worker = threading.Thread(target=monitor)
    worker.start()
    pattern = 'paired_generated_input' if args.input_only else 'paired_generated_'
    command = [str(binary),pattern,'--ignored','--nocapture','--test-threads=1']
    if not args.input_only:
        command += ['--skip','paired_generated_input']
    with (args.output / f'pairs-{run}.log').open('w') as output:
        result = subprocess.run(command, stdout=output, stderr=subprocess.STDOUT)
    stop.set()
    worker.join()
    (args.output / f'clients-{run}.json').write_text(json.dumps(samples, indent=2)+'\n')
    if result.returncode:
        raise SystemExit(result.returncode)
