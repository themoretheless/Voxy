#!/usr/bin/env python3
"""Keep unknown/transparent source materials behind a known nearest surface."""
import json,runpy,sys
from pathlib import Path
root=Path(sys.argv[1]);runpy.run_path(str(Path(__file__).with_name('make_world_material_fixture.py')),run_name='__main__')
m=json.loads((root/'manifest.json').read_text())
for i in range(0,len(m['meshes']),2):
    m['meshes'][i]['material'],m['meshes'][i+1]['material']=m['meshes'][i+1]['material'],m['meshes'][i]['material']
(root/'manifest.json').write_text(json.dumps(m,indent=2)+'\n')
