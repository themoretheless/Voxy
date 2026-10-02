import importlib, pathlib, json, hashlib
m=importlib.import_module('import')
r=pathlib.Path(__file__).parent
manifest=json.loads((r/'manifest.json').read_text())
results=[]
for t in manifest:
    obj=m.decode(m.call('get_source',{'uri':t['uri'],'include_chunks':False}))
    actual=hashlib.sha256(obj['content'].encode()).hexdigest()
    metadata=json.loads(obj.get('metadata_json','{}'))
    ok=actual==t['sha256'] and metadata.get('sha256')==t['sha256'] and obj.get('wing')=='voxy' and obj.get('room')=='archived-chats'
    results.append({'thread_id':t['id'],'uri':t['uri'],'document_id':obj.get('id'),'sha256':actual,'matches':ok,'wing':obj.get('wing'),'room':obj.get('room'),'layer':obj.get('layer')})
    print('VERIFIED',len(results),t['title'],ok,flush=True)
(r/'verification.json').write_text(json.dumps(results,ensure_ascii=False,indent=2))
assert all(t['matches'] for t in results)
page=m.decode(m.call('get_wiki_page',{'id_or_slug':'voxy-archived-work-2026-10-03'}))
(r/'index-readback.json').write_text(json.dumps(page,ensure_ascii=False,indent=2))
assert all(t['uri'] in page['content'] for t in manifest)
checks=[]
for name,args in [('query_with_index',{'query':'Voxy все наработки архивных чатов 2026-10-03','top_k':5}),('query_with_index',{'query':'voxy-archive-01a0f863-5900-7602-9822-89725fb8a242','include_content':True,'top_k':2}),('query_with_index',{'query':'voxy-archive-01a0f478-2ce6-7143-b83b-1a8b07048f71','include_content':True,'top_k':2}),('doctor',{})]:
    result=m.call(name,args);checks.append({'tool':name,'arguments':args,'result':result});(r/'search-proof.json').write_text(json.dumps(checks,ensure_ascii=False,indent=2));print('CHECK',name,flush=True)
(r/'search-proof.json').write_text(json.dumps(checks,ensure_ascii=False,indent=2))
print('DONE: all sources match, index read back, retrieval and doctor recorded',flush=True)
