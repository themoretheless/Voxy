import json,pathlib,re,sys
B=pathlib.Path(__file__).resolve().parents[2]/'target/hardware-research/sources'
EXCLUDE={'armory3d/armory','dimforge/parry','libsdl-org/SDL_image','sebastianstarke/AI4Animation','Traverse-Research/rust-gpu','PistonDevelopers/piston','pmndrs/react-three-fiber','microsoft/DirectXTK','bkaradzic/bx','dimforge/rapier','sp4cerat/Fast-Quadric-Mesh-Simplification'}
rs=sorted((json.loads(p.read_text()) for p in B.glob('*/record.json') if json.loads(p.read_text()).get('selection_version')==2),key=lambda r:r['repository'].lower())
rs=[r for r in rs if r['repository'] not in EXCLUDE]
start=int(sys.argv[1]) if len(sys.argv)>1 else 0
for i,r in list(enumerate(rs))[start:start+20]:
 print('\nINDEX',i,'REPO',r['repository'])
 f=r['files'][0];print('FILE',f['path']);lines=(B/r['repository'].replace('/','__')/f['local']).read_text().splitlines()
 hits=[j for j,s in enumerate(lines) if re.search(r'\b(\w*(?:allocat|barrier|dispatch|fence|submit|create_buffer|createBuffer|cuda|vkCmd|vkAllocate|indirect|device|render|compute|shader|compile|retire|pending)\w*)\s*\(',s,re.I) and not s.lstrip().startswith(('//','*','#','use ','import ','export *'))]
 if hits:
  chosen=[hits[len(hits)//2],hits[-1]]
 else: chosen=[len(lines)//2]
 for j in chosen: print('LINE',max(1,j-3),'\n'.join(lines[max(0,j-4):j+9]))
print('TOTAL',len(rs))
