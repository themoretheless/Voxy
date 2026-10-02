import pathlib, sys, importlib, json, hashlib
ROOT=pathlib.Path(__file__).parent
sys.path.insert(0,str(ROOT.parents[1]/'archive-rag'/'2026-10-03'))
m=importlib.import_module('import')
def save(name,obj):
    (ROOT/name).write_text(json.dumps(obj,ensure_ascii=False,indent=2))
if __name__=='__main__':
    t=json.loads((ROOT/'manifest.json').read_text())
    m.call('get_schema',{})
    stage=pathlib.Path('/Users/themoretheless/Documents/Sources/.rag-imports/voxy/chat-history/2026-10-03');stage.mkdir(parents=True,exist_ok=True)
    target=stage/(t['thread_id']+'.md');target.write_text(pathlib.Path(t['path']).read_text())
    receipt=m.call('ingest_file',{'path':str(target),'uri':t['uri'],'title':'Voxy — Создать 2D/3D движок — снимок чата 2026-10-03','wing':'voxy','room':'chat-history','metadata_json':json.dumps(t,ensure_ascii=False)})
    save('receipt.json',receipt);print('RAW imported',m.decode(receipt),flush=True)
    stored=m.decode(m.call('get_source',{'uri':t['uri'],'include_chunks':False}))
    proof={'document_id':stored['id'],'uri':stored['uri'],'sha256':hashlib.sha256(stored['content'].encode()).hexdigest(),'wing':stored['wing'],'room':stored['room']}
    assert proof['sha256']==t['sha256'] and proof['wing']=='voxy' and proof['room']=='chat-history'
    save('verification.json',proof);print('RAW verified',flush=True)
    page=json.loads((ROOT/'wiki-page.json').read_text())
    save('wiki-receipt.json',m.call('write_wiki_page',page))
    stored_page=m.decode(m.call('get_wiki_page',{'id_or_slug':page['slug']}))
    assert stored_page['content']==page['content']
    save('wiki-verification.json',{'slug':page['slug'],'content_matches':True,'sha256':hashlib.sha256(stored_page['content'].encode()).hexdigest()})
    save('rebuild-receipt.json',m.call('rebuild_index',{}))
    found=m.decode(m.call('query_with_index',{'query':page['slug'],'top_k':3}))
    assert any(x['entry']['slug']==page['slug'] for x in found['matches'])
    save('search-proof.json',found)
    doctor=m.decode(m.call('doctor',{}));assert doctor['ok'] and doctor['ready_for_search']
    save('doctor.json',doctor)
    save('log-receipt.json',m.call('append_log',{'op':'voxy_chat_import','prefix':'INGEST','agent_name':'codex','entity_kind':'wiki','entity_id':'wiki://'+page['slug'],'message':'Ingested current Voxy engine chat snapshot; raw and wiki contents verified, catalog retrieval and doctor passed.','payload_json':json.dumps({'thread_id':t['thread_id'],'message_counts':t['message_counts'],'final_reports':t['final_reports'],'source':t['uri']})}))
    print('DONE: raw, wiki, index and doctor verified',flush=True)
