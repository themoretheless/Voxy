#!/usr/bin/env python3
"""Verify cached fallback checks, raw counters and unchanged original street output."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('report',type=Path)
p.add_argument('--raw',type=Path,required=True)
p.add_argument('--reference-stem',type=Path,required=True)
p.add_argument('--checked-run',type=Path)
p.add_argument('--output',type=Path,required=True)
a=p.parse_args()
report=json.loads(a.report.read_text());data=report['consumer_cache_audit']
assert data['consumer_cache']
records=data['region_counters'];raw=a.raw.read_bytes()
assert len(raw)==len(records)*48
assert [list(v) for v in struct.iter_unpack('<12I',raw)]==records
stem=a.report.with_suffix('');prov=json.loads(Path(str(stem)+'-provenance.json').read_text())
checking=prov['settings'].get('VOXY_STREET_WORLD_RAY_CONSUMER_CACHE_CHECK')=='1'
assert prov['settings']['VOXY_STREET_WORLD_RAY_CONSUMER_CACHE']=='1'
staged=prov['settings'].get('VOXY_STREET_WORLD_RAY_CONSUMER_STAGES')=='1'
if staged: assert data['consumer_stages_enabled']
# At most one stage-record check and one colour check per pixel/encode.
assert 2*report['frames']*report['internal_resolution'][0]*report['internal_resolution'][1]*2 < 2**32
per=data['tiles_per_kind'];assert len(records)%per==0
sums=[[sum(row[k] for row in records[start:start+per]) for k in range(8,12)] for start in range(0,len(records),per)]
assert sums==data['by_kind']
for pruned,checked,mismatched,nonfinite in sums:
    assert mismatched==0
    if checking: assert checked>=pruned>0
    assert pruned<=report['internal_resolution'][0]*report['internal_resolution'][1]
reference=json.loads(Path(str(a.reference_stem)+'.json').read_text())
for key in ['frames','simulation_steps','rendered_state_blake3','full_collision_triangles','internal_resolution','output_resolution','world_regions']:
    assert report[key]==reference[key],key
images=[]
for ext in ['.png','.pfm']:
    x=Path(str(stem)+ext).read_bytes();y=Path(str(a.reference_stem)+ext).read_bytes()
    assert x==y,ext
    images.append(dict(extension=ext,bit_identical=True,sha256=hashlib.sha256(x).hexdigest()))
if a.checked_run:
    other=json.loads(a.checked_run.read_text());assert other['consumer_cache_audit']['consumer_cache']
    assert all(v[1]>=v[0]>0 and v[2]==0 for v in other['consumer_cache_audit']['by_kind'])
    other_prov=json.loads(Path(str(a.checked_run.with_suffix(''))+'-provenance.json').read_text())
    assert other_prov['binary_sha256']==prov['binary_sha256']
result=dict(cache_checker_enabled=checking,consumer_stages_enabled=data.get("consumer_stages_enabled",False),
            staged_record_scope="44 projection words and all 8 hit words independently checked" if staged and checking else None,mismatch_counter_scope=data.get("mismatch_counter_scope","last completed source frame only"),by_kind=sums,images=images,replay_and_workload_identical=True,
            cached_values_scope='Float32 prepass/resolver bit equality for checked/pruned or fallback-used rays only',
            source_replay_scope='renderer may mark consumer-irrelevant rays inactive; never a geometric miss proof',
            complete_motion_cycle_qualified=False,presented_120_fps_qualified=False)
a.output.write_text(json.dumps(result,indent=2));print(json.dumps(result))
