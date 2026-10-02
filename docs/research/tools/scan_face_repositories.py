"""Retrieve pinned source samples. Mechanical scanning is not a deep code review."""
import concurrent.futures, hashlib, json, re, subprocess, time, urllib.parse, urllib.request
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]/'.research/face-source-study-500'
ROOT.mkdir(parents=True,exist_ok=True)
QUERIES=['blendshape','facial animation','skin shader','eye shader','subsurface scattering','facial rig','FLAME 3D','mouth animation','lip sync 3D','human rendering','face reconstruction 3D','facial retargeting','ARKit blendshape','FACS face','wrinkle shader','face model 3D','cornea shader','character creator 3D','MakeHuman','DLSS','teeth 3D','tongue animation','face deformation','facial mocap']
EXTS={'.py','.cpp','.cc','.c','.h','.hpp','.hlsl','.hlsli','.glsl','.shader','.cginc','.gdshader','.gd','.ts','.js','.cs','.wgsl','.rs','.cu','.slang','.m','.mel','.osl','.sl','.ush','.usf','.swift','.lua','.mm','.jsx','.tsx','.vert','.frag','.comp'}
WORDS=['eye','skin','subsurface','sss','blendshape','blend_shape','jaw','mouth','teeth','tooth','gum','tongue','pharynx','throat','wrinkle','flame','deform','lbs','facs','iris','cornea','shader','dlss','rig','face','lip']
def api(endpoint):
 for attempt in range(4):
  p=subprocess.run(['gh','api',endpoint],capture_output=True,text=True,timeout=100)
  if p.returncode==0:return json.loads(p.stdout)
  if 'rate limit' in p.stderr.lower():time.sleep(20*(attempt+1));continue
  raise RuntimeError(p.stderr[-500:])
 raise RuntimeError('rate limit retries exhausted')
def inventory():
 rows={}
 for r in json.loads(Path('/tmp/voxy-face-source-study/selected-100.json').read_text()):
  rows[r['name']]={'name':r['name'],'branch':r.get('branch','HEAD'),'url':r['url'],'category':r.get('category','reference'),'discovery':'previous-100'}
 for query in QUERIES:
  q=urllib.parse.quote(query+' in:name,description fork:false')
  data=api(f'search/repositories?q={q}&per_page=100')
  for item in data['items']:
   description=item.get('description') or ''
   if re.search(r'face recognition|facial recognition|emotion recognition|face detection',description,re.I) and not re.search(r'3d|blendshape|render|rig|avatar',description,re.I):continue
   name=item['full_name']
   rows.setdefault(name,{'name':name,'branch':item['default_branch'],'url':item['html_url'],'description':description,'stars':item['stargazers_count'],'fork':item['fork'],'license':(item.get('license') or {}).get('spdx_id'),'discovery':query})
  (ROOT/'inventory.json').write_text(json.dumps(list(rows.values()),ensure_ascii=False,indent=2))
  print('DISCOVERY',query,'unique_candidates',len(rows),flush=True)
 return list(rows.values())
def retrieve(row):
 name=row['name'];dest=ROOT/name.replace('/','__');dest.mkdir(exist_ok=True)
 if (dest/'manifest.json').exists():
  try:prior=json.loads((dest/'manifest.json').read_text())
  except json.JSONDecodeError:prior={}
  if prior.get('files') and not any('qtquick/' in f['path'].lower() for f in prior['files']):return prior
 result=dict(row,status='pending',files=[],review_status='mechanical_source_scan',deep_review=False)
 try:
  if not re.fullmatch(r'[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+',name):raise ValueError('invalid repository name')
  tree=api(f'repos/{name}/git/trees/{row["branch"]}?recursive=1')
  result.update(sha=tree['sha'],tree_truncated=tree.get('truncated',False))
  paths=[x['path'] for x in tree.get('tree',[]) if x['type']=='blob']
  result['topic_paths']={word:[p for p in paths if word in p.lower()][:30] for word in WORDS if any(word in p.lower() for p in paths)}
  result['license_paths']=[p for p in paths if re.search(r'(^|/)(license|copying|notice)(\.|$)',p,re.I)][:20]
  code=[]
  for item in tree.get('tree',[]):
   path=item['path'];low=path.lower()
   if item['type']!='blob' or Path(path).suffix.lower() not in EXTS or item.get('size',0)>250000:continue
   if any(w in low for w in ['node_modules/','thirdparty/','third_party/','external/','vendor/','jquery','qtquick/']):continue
   score=sum(15 if w in Path(low).stem else 3 for w in WORDS if w in low)
   if Path(path).suffix.lower() in {'.hlsl','.hlsli','.glsl','.shader','.cginc','.gdshader','.wgsl'}:score+=12
   if any(w in low for w in ['test','example','demo']):score-=2
   code.append((score,path,item))
  code.sort(key=lambda r:(-r[0],len(r[1]),r[1]))
  for score,path,item in code[:6]:
   url=f'https://raw.githubusercontent.com/{name}/{tree["sha"]}/'+urllib.parse.quote(path,safe='/')
   try:
    with urllib.request.urlopen(url,timeout=30) as response:body=response.read(250001)
    if len(body)>250000:continue
    content=body.decode('utf-8');local=dest/f'source-{len(result["files"])+1}{Path(path).suffix}'
    local.write_bytes(body)
    matches={word:sum(word in line.lower() for line in content.splitlines()) for word in WORDS}
    result['files'].append({'path':path,'local':str(local),'blob_sha':item['sha'],'sha256':hashlib.sha256(body).hexdigest(),'lines':len(content.splitlines()),'bytes':len(body),'selection_score':score,'keyword_line_counts':{k:v for k,v in matches.items() if v},'url':f'https://github.com/{name}/blob/{tree["sha"]}/{urllib.parse.quote(path,safe="/")}'})
   except Exception as error:result.setdefault('file_errors',[]).append(str(error)[:180])
   if len(result['files'])>=4:break
  result['status']='source_scanned' if result['files'] else 'no_source_found'
 except Exception as error:result['status']='failed';result['error']=str(error)
 temporary=dest/'manifest.json.tmp'
 temporary.write_text(json.dumps(result,indent=2,ensure_ascii=False))
 temporary.replace(dest/'manifest.json')
 return result
if __name__=='__main__':
 rows=json.loads((ROOT/'inventory.json').read_text()) if (ROOT/'inventory.json').exists() else inventory()
 # Preserve previous references, then balance discovery topics instead of filling
 # the target with the first broad category returned by GitHub.
 groups={}
 for row in rows:groups.setdefault(row['discovery'],[]).append(row)
 ordered=groups.pop('previous-100',[])
 while any(groups.values()):
  for group in groups.values():
   if group:ordered.append(group.pop(0))
 excluded=[]
 relevant=[]
 for row in ordered:
  topic=row['discovery']
  text=(row['name']+' '+(row.get('description') or '')).lower()
  text=re.sub(r':[a-z_]+:',' ',text)
  reason=None
  if row['name'] in ['micooz/terminal-js', 'Danilo22Mh/FaceDetections', 'unite-deals/facerecog', 'sandeepsainihisar/jayka-with-js-animation', 'nadjiel/eye-dropper', 'monster555/flutter_shady_weather_demo', 'cocos-creator/cocos-parkour-character-control', 'metehangucl/MobileCharacterController', 'metehanguclu/MobileCharacterController', 'minecraftbedrockpro/Marketplace-Free', 'bradleyq/stable_player_display', 'digantgarude/Skintone-Detector', 'feniota/tiny-skin-viewer', 'AStox/GLSL_MagicEye', 'mattjacobus/eyesync', 'puff-dayo/wxReader', 'AngelGameDev/SmolEyes', 'DomNomNomVR/FixShaderRightEye', 'neobenedict/sdk_screenspace_shaders_advanced', 'manishnandwani/face-api', 'allimist/opencv-face-replace']:reason='Manual scope check: unrelated term collision or identity detection without facial rendering/rigging'
  if topic=='FLAME 3D' and not re.search(r'face|facial|human|avatar|morph|smpl|head model',text):reason='FLAME name collision; no facial/human-model evidence'
  if topic=='human rendering' and re.search(r'react|html|markdown|linked data',text) and not re.search(r'3d|avatar|face|smpl|neural|mesh',text):reason='Text/UI rendering, not human geometry or shading'
  if topic=='tongue animation' and not re.search(r'tongue',text):reason='Emoji-only tongue match'
  if topic=='FACS face' and (not re.search(r'facs|facial|action.unit|faceexpression|faceforge|facecept|face.*expression',text) or 'access control' in text):reason='FACS/Fac name collision, not a facial action-unit or animation implementation'
  if topic=='human rendering' and re.search(r'terminology server|human.friendly.*render|fhir',text):reason='Human-readable terminology UI, not human-body rendering'
  if topic=='eye shader' and re.search(r'eye.candy|eye.catching|chromium extension|late night work',text):reason='Eye comfort/candy wording, not an ocular shader'
  if reason:excluded.append(dict(row,exclusion_reason=reason))
  else:relevant.append(row)
 ordered=relevant
 (ROOT/'excluded-candidates.json').write_text(json.dumps(excluded,ensure_ascii=False,indent=2))
 print('TOPICAL FILTER excluded',len(excluded),'remaining',len(ordered),flush=True)
 results=[]
 good=[]
 with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
  for start in range(0,len(ordered),30):
   batch=ordered[start:start+30]
   for result in pool.map(retrieve,batch):
    results.append(result)
    if result['files']:good.append(result)
   print('SOURCE SCAN',len(results),'successful',len(good),'candidate_total',len(rows),flush=True)
   temporary=ROOT/'retrieval.json.tmp'
   temporary.write_text(json.dumps(results,ensure_ascii=False,indent=2))
   temporary.replace(ROOT/'retrieval.json')
   if len(good)>=500:
    (ROOT/'selected-500.json').write_text(json.dumps(good[:500],ensure_ascii=False,indent=2))
    print('TARGET REACHED',500,flush=True)
    break
 print('DONE successful',len(good),'candidates',len(rows),flush=True)
