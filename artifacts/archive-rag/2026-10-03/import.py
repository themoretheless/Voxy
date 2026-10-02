import json, pathlib, urllib.request, time
ROOT = pathlib.Path(__file__).parent
ENDPOINT = 'http://127.0.0.1:7432/mcp'
def call(name, arguments):
    payload = {'jsonrpc':'2.0','id':1,'method':'tools/call','params':{'name':name,'arguments':arguments}}
    request = urllib.request.Request(ENDPOINT,data=json.dumps(payload,ensure_ascii=False).encode(),headers={'Content-Type':'application/json','Accept':'application/json, text/event-stream','MCP-Protocol-Version':'2025-06-18'})
    with urllib.request.urlopen(request,timeout=1800) as response:
        raw=response.read().decode()
    obj=json.loads(raw) if raw.lstrip().startswith('{') else next(json.loads(line[6:]) for line in raw.splitlines() if line.startswith('data: '))
    if 'error' in obj: raise RuntimeError(obj['error'])
    result=obj['result']
    if result.get('isError'): raise RuntimeError(result)
    return result
def decode(result):
    return json.loads(next(item['text'] for item in result['content'] if item['type']=='text'))
if __name__=='__main__':
    manifest=json.loads((ROOT/'manifest.json').read_text())
    receipt_path=ROOT/'receipts.json'
    receipts=json.loads(receipt_path.read_text()) if receipt_path.exists() else []
    done={r['uri'] for r in receipts}
    for t in manifest:
        if t['uri'] in done: continue
        start=time.time()
        metadata={'wing':'voxy','room':'archived-chats','project':'voxy','thread_id':t['id'],'workspace':t['cwd'],'rollout_path':t['rollout_path'],'exported_at':'2026-10-03','sha256':t['sha256'],'historical':True,'message_counts':t['counts']}
        stage=pathlib.Path('/Users/themoretheless/Documents/Sources/.rag-imports/voxy/2026-10-03'); stage.mkdir(parents=True,exist_ok=True)
        target=stage/(t['id']+'.md'); target.write_text(pathlib.Path(t['path']).read_text())
        result=call('ingest_file',{'path':str(target),'uri':t['uri'],'title':'Архив Voxy — '+t['title'],'wing':'voxy','room':'archived-chats','metadata_json':json.dumps(metadata,ensure_ascii=False)})
        receipts.append({'uri':t['uri'],'thread_id':t['id'],'result':result})
        receipt_path.write_text(json.dumps(receipts,ensure_ascii=False,indent=2))
        print(json.dumps({'imported':len(receipts),'title':t['title'],'seconds':round(time.time()-start,1),'result':result},ensure_ascii=False),flush=True)
