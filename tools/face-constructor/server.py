#!/usr/bin/env python3
"""Local detailed face editor; atomic JSON updates consumed by the native preview."""
import argparse, json, math, os, tempfile
from pathlib import Path
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
ROOT = Path(__file__).resolve().parents[2]
SCHEMA = json.loads((ROOT/'assets/characters/face-controls.json').read_text())
CONTROLS = {c['key']: c for c in SCHEMA}
parser = argparse.ArgumentParser()
parser.add_argument('--preset', type=Path, default=ROOT/'assets/characters/face-current.json')
parser.add_argument('--port', type=int, default=8791)
args = parser.parse_args()
args.preset = args.preset.resolve()
def validate(data):
    if not isinstance(data, dict): raise ValueError('Ожидается объект параметров')
    result = {c['key']:c['default'] for c in SCHEMA}
    for key, value in data.items():
        c = CONTROLS.get(key)
        if c is None: raise ValueError('Неизвестный параметр: '+key)
        if isinstance(value,bool) or not isinstance(value,(int,float)) or not math.isfinite(value) or not c['min'] <= value <= c['max']:
            raise ValueError('Недопустимое значение: '+key)
        result[key] = value
    return result
def save(data):
    data = validate(data)
    args.preset.parent.mkdir(parents=True, exist_ok=True)
    fd, name = tempfile.mkstemp(dir=args.preset.parent, prefix='.face-')
    try:
        with os.fdopen(fd,'w') as f: json.dump(data,f,indent=2); f.write('\n')
        os.replace(name,args.preset)
    finally:
        if os.path.exists(name): os.unlink(name)
    return data
if not args.preset.exists(): save({})
class Handler(BaseHTTPRequestHandler):
    def reply(self,code,data,content='application/json'):
        payload=data.encode() if isinstance(data,str) else json.dumps(data,ensure_ascii=False).encode()
        self.send_response(code);self.send_header('Content-Type',content+'; charset=utf-8');self.send_header('Content-Length',str(len(payload)));self.end_headers();self.wfile.write(payload)
    def do_GET(self):
        if self.path=='/': return self.reply(200,(Path(__file__).with_name('index.html')).read_text(),'text/html')
        if self.path=='/schema': return self.reply(200,SCHEMA)
        if self.path=='/preset': return self.reply(200,validate(json.loads(args.preset.read_text())))
        self.reply(404,{'error':'Not found'})
    def do_POST(self):
        if self.path!='/preset': return self.reply(404,{'error':'Not found'})
        try:
            length=int(self.headers.get('Content-Length','0'))
            if length>65536: raise ValueError('Preset too large')
            self.reply(200,save(json.loads(self.rfile.read(length))))
        except (ValueError,OSError) as e: self.reply(400,{'error':str(e)})
    def log_message(self,*args): pass
print(f'Constructor: http://127.0.0.1:{args.port}; preset: {args.preset}',flush=True)
ThreadingHTTPServer(('127.0.0.1',args.port),Handler).serve_forever()
