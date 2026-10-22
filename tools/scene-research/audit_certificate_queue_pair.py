#!/usr/bin/env python3
"""Compare source schedule outputs and audit captured original-index queue partitions."""
import argparse
import hashlib
import json
import struct
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('control',type=Path)
p.add_argument('candidate',type=Path)
p.add_argument('--expected-group-size',type=int,required=True)
p.add_argument('--output',type=Path,required=True)
a=p.parse_args()
frame=json.loads((a.candidate/'world-frame.json').read_text())
assert frame['sparse_certificates_prepared'] and frame['sparse_certificate_group_size']==a.expected_group_size
images=[]
for prefix in ['full-frame','exact-image-hit','optical-image-hit','one-sided-image-hit','tail-image-hit']:
    for channel in ['gi','ssr','native']:
        name=f'{prefix}-{channel}.bin'
        data=(a.control/name).read_bytes()
        assert data==(a.candidate/name).read_bytes(),name
        images.append({'name':name,'sha256':hashlib.sha256(data).hexdigest()})
records=0
for path in a.control.glob('frame-*-tile-*-input.bin'):
    before=list(struct.iter_unpack('<20I',path.read_bytes()))
    after=list(struct.iter_unpack('<20I',(a.candidate/path.name).read_bytes()))
    assert len(before)==len(after)
    for x,y in zip(before,after):
        assert all(x[k]==y[k]for k in range(20)if k not in [7,11,15])
    name=path.name.replace('-input.bin','-hits.bin')
    assert (a.control/name).read_bytes()==(a.candidate/name).read_bytes(),name
    records+=len(before)
assert records==2048
for name in ['guard-protocol.bin','bounded-full.bin','bounded-partial.bin']:
    assert (a.control/name).read_bytes()==(a.candidate/name).read_bytes(),name
snapshots=[]
for path in a.candidate.glob('certificate-queue-*.json'):
    q=json.loads(path.read_text())
    assert q['partition_complete'] and not q['duplicate_or_dropped_indices']
    assert q['worker_group_size']==a.expected_group_size
    assert len(q['indices'])==len(set(q['indices']))==q['queued']
    assert all(0<=i<q['requested']for i in q['indices'])
    assert q['inactive']+q['native_completed']+q['queued']==q['requested']
    assert q['indirect']==[(q['queued']+a.expected_group_size-1)//a.expected_group_size,1,1]
    snapshots.append(path.name)
assert len(snapshots)==39
proof={'original_source_records':records,'all_80_byte_hit_records_identical':True,'protected_source_coefficients_and_ids_identical':True,'queue_snapshots':snapshots,'worker_group_size':a.expected_group_size,'images':images,'full_scene_qualified':False,'fps_qualified':False}
a.output.write_text(json.dumps(proof,indent=2)+'\n')
print(json.dumps({'source_records':records,'images':len(images),'queue_snapshots':len(snapshots),'worker_group_size':a.expected_group_size,'passed':True}))
