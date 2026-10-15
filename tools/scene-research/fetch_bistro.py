#!/usr/bin/env python3
"""Fetch pinned scene dependencies, preserving every source texel and mesh byte."""
from concurrent.futures import ThreadPoolExecutor, as_completed
import hashlib
import json
from pathlib import Path, PurePosixPath
import time
import urllib.parse
import urllib.request

ROOT=Path(__file__).resolve().parents[2]
DEST=ROOT/'assets/environments/bistro'
CACHE=DEST/'cache'
CACHE.mkdir(parents=True,exist_ok=True)
STATUS=DEST/'download-status.json'
REPO='zeux/niagara_bistro'

def request_json(url):
    with urllib.request.urlopen(urllib.request.Request(url,headers={'User-Agent':'Voxy-scene-import'}),timeout=60) as stream:
        return json.load(stream)

def checked_path(name):
    path=PurePosixPath(name)
    if path.is_absolute() or '..' in path.parts:raise ValueError('unsafe asset path')
    return CACHE.joinpath(*path.parts)

def digest(path,size):
    h=hashlib.sha1(f'blob {size}\0'.encode())
    with path.open('rb') as stream:
        while chunk:=stream.read(1024*1024):h.update(chunk)
    return h.hexdigest()

def main():
    commit=request_json(f'https://api.github.com/repos/{REPO}/commits/master')['sha']
    tree=request_json(f'https://api.github.com/repos/{REPO}/git/trees/{commit}?recursive=1')
    if tree.get('truncated'):raise ValueError('truncated repository tree')
    files={v['path']:v for v in tree['tree'] if v['type']=='blob'}
    def fetch(name):
        meta=files[name];path=checked_path(name);size=meta['size'];expected=meta['sha']
        if path.exists() and path.stat().st_size==size and digest(path,size)==expected:return name,size
        path.parent.mkdir(parents=True,exist_ok=True)
        temporary=path.with_name(path.name+'.download')
        url=f'https://raw.githubusercontent.com/{REPO}/{commit}/'+urllib.parse.quote(name,safe='/')
        for attempt in range(3):
            try:
                h=hashlib.sha1(f'blob {size}\0'.encode());count=0
                with urllib.request.urlopen(url,timeout=90) as source,temporary.open('wb') as target:
                    while chunk:=source.read(1024*1024):target.write(chunk);h.update(chunk);count+=len(chunk)
                if count!=size or h.hexdigest()!=expected:raise ValueError(f'asset integrity mismatch: {name}')
                temporary.replace(path);return name,size
            except Exception:
                if attempt==2:raise
                time.sleep(attempt+1)
    fetch('bistro.gltf')
    gltf=json.loads((CACHE/'bistro.gltf').read_text())
    # DDS extension sources are the actual repository textures; PNG entries are
    # absent alternate representations. Never substitute missing textures.
    images={texture['extensions']['MSFT_texture_dds']['source'] if 'MSFT_texture_dds' in texture.get('extensions',{}) else texture.get('source') for texture in gltf.get('textures',[])}
    refs={'bistro.gltf','LICENSE','README.md'}|{b['uri'] for b in gltf.get('buffers',[])}|{gltf['images'][i]['uri'] for i in images if i is not None}
    missing=sorted(refs-files.keys())
    if missing:raise ValueError(f'missing source assets: {missing}')
    manifest={'repository':f'https://github.com/{REPO}','commit':commit,'original_asset':'https://developer.nvidia.com/orca/amazon-lumberyard-bistro','geometry_and_texture_quality':'original unchanged source bytes','files':[{k:files[name][k] for k in ['path','size','sha']} for name in sorted(refs)]}
    (DEST/'source-manifest.json').write_text(json.dumps(manifest,indent=2))
    status={'status':'running','commit':commit,'assets_total':len(refs),'bytes_total':sum(files[n]['size'] for n in refs),'assets_completed':0,'bytes_completed':0,'errors':[],'started_unix':time.time()}
    def publish():
        temporary=STATUS.with_suffix('.tmp');temporary.write_text(json.dumps(status,indent=2));temporary.replace(STATUS)
    publish()
    with ThreadPoolExecutor(max_workers=6) as pool:
        tasks={pool.submit(fetch,name):name for name in sorted(refs)}
        for task in as_completed(tasks):
            try:
                name,size=task.result();status['assets_completed']+=1;status['bytes_completed']+=size
            except Exception as error:status['errors'].append({'asset':tasks[task],'error':str(error)})
            status['updated_unix']=time.time();publish()
    status['status']='failed' if status['errors'] else 'downloaded_not_rendered';publish()
    if status['errors']:raise SystemExit(1)

if __name__=='__main__':main()
