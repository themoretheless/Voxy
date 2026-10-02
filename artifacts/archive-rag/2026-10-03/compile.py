import importlib, json, pathlib
m=importlib.import_module('import')
ROOT=pathlib.Path(__file__).parent
if __name__=='__main__':
    manifest=json.loads((ROOT/'manifest.json').read_text())
    receipts=json.loads((ROOT/'receipts.json').read_text())
    assert {x['uri'] for x in manifest}=={x['uri'] for x in receipts}, 'Incomplete raw import'
    pages=json.loads((ROOT/'wiki-pages.json').read_text())
    path=ROOT/'wiki-receipts.json'
    written=json.loads(path.read_text()) if path.exists() else []
    done={p['slug'] for p in written}
    for page in pages:
        if page['slug'] in done:continue
        result=m.call('write_wiki_page',page)
        written.append({'slug':page['slug'],'result':result})
        path.write_text(json.dumps(written,ensure_ascii=False,indent=2))
        print('WIKI '+page['slug'],flush=True)
    index=json.loads((ROOT/'index-page.json').read_text())
    result=m.call('write_wiki_page',index)
    (ROOT/'index-receipt.json').write_text(json.dumps(result,ensure_ascii=False,indent=2))
    rebuild=m.call('rebuild_index',{})
    (ROOT/'rebuild-receipt.json').write_text(json.dumps(rebuild,ensure_ascii=False,indent=2))
    print('INDEX rebuilt',flush=True)
    log=m.call('append_log',{'op':'voxy_archive_import','prefix':'INGEST','agent_name':'codex','entity_kind':'wiki','entity_id':'wiki://voxy-archived-work-2026-10-03','message':'Imported all 21 selected archived Voxy-related chats, 6281 public messages, with per-chat historical summaries and provenance.','payload_json':json.dumps({'raw_sources':21,'wiki_summaries':21,'index':'voxy-archived-work-2026-10-03','manifest':str(ROOT/'manifest.json')})})
    (ROOT/'log-receipt.json').write_text(json.dumps(log,ensure_ascii=False,indent=2))
