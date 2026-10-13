"""Read-only snapshot of completed host profile records, never frame admission."""
import json,sys,collections,hashlib
from pathlib import Path
source=Path(sys.argv[1]);raw=source.read_bytes();records=[];incomplete=0
prefix='HAIR JOINT STAGE PROFILE '
for line in raw.decode('utf-8',errors='replace').splitlines():
 if not line.startswith(prefix):continue
 try:record=json.loads(line[len(prefix):])
 except json.JSONDecodeError:incomplete+=1;continue
 if record.get('backend')=='triple_active_set':records.append(record)
shapes=collections.Counter();totals=collections.Counter();stages=collections.Counter();hint_scopes=collections.defaultdict(collections.Counter)
for r in records:
 totals['recorded_active_set_iterations']+=sum(r.get('completed_active_set_iterations',[]))
 totals['operators_with_recorded_iterations']+=len(r.get('completed_active_set_iterations',[]))
 if r.get('completed_active_set_iterations') and not r['errors']:
  category='all_hinted' if r.get('seeded_upload_owners',0)==r['submitted_owners'] else ('all_cold' if not r.get('seeded_upload_owners',0) else 'mixed')
  hint_scopes[category].update(scopes=1,operators=len(r['completed_active_set_iterations']),iterations=sum(r['completed_active_set_iterations']))
 shapes.update(tuple(s) for s in r['submitted_operator_shapes_rows_coordinates'])
 for key in ['owners','cached_owners','submitted_owners','dispatches','submissions','errors','complete_workspace_payload_bytes','incoming_valid_seeded_owners','incoming_positive_seed_rows','seeded_upload_owners','cached_active_hint_owners']:totals[key]+=r.get(key,0)
 for k,v in r['stages'].items():stages[k]+=v
report={'scope':'snapshot of completed coordinate host scopes only; cache count includes shared aliases; errors are coordinate scope errors, not physical fallback; concurrent GPU workloads prevent isolated performance claims; no frame or FPS admission','log':str(source.resolve()),'snapshot_bytes':len(raw),'snapshot_sha256':hashlib.sha256(raw).hexdigest(),'completed_host_scopes':len(records),'incomplete_profile_lines':incomplete,'totals':dict(totals),'stage_sums':dict(stages),'hint_scope_iterations':{k:dict(v) for k,v in hint_scopes.items()},'submitted_shapes':[{'rows':s[0],'coordinates':s[1],'count':n} for s,n in shapes.most_common()],'slowest_scopes':[{'owners':r['owners'],'submitted':r['submitted_owners'],'cached_or_shared':r['cached_owners'],'errors':r['errors'],'active_set_iterations':r.get('completed_active_set_iterations',[]),'shapes':r['submitted_operator_shapes_rows_coordinates'],'stages':r['stages']} for r in sorted(records,key=lambda r:r['stages']['total_wall_ms'],reverse=True)[:8]]}
print(json.dumps(report,indent=2))
