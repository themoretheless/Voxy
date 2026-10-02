#!/usr/bin/env python3
"""Local body panel; validation and persistence are delegated to model_mcp."""
import argparse,json,subprocess,threading
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer
from pathlib import Path

class Bridge:
    def __init__(self,binary,path):
        self.process=subprocess.Popen([str(binary),str(path)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True)
        self.lock=threading.Lock();self.counter=0
        self.rpc('initialize',{'protocolVersion':'2025-06-18','capabilities':{},'clientInfo':{'name':'body-panel','version':'1'}})
    def rpc(self,method,params):
        with self.lock:
            self.counter+=1
            self.process.stdin.write(json.dumps(dict(jsonrpc='2.0',id=self.counter,method=method,params=params))+'\n');self.process.stdin.flush()
            line=self.process.stdout.readline()
            if not line:raise RuntimeError('MCP server terminated')
            response=json.loads(line)
            if 'error' in response:raise ValueError(response['error']['message'])
            return response['result']
    def tool(self,name,args=None):
        result=self.rpc('tools/call',{'name':name,'arguments':args or {}})
        if result.get('isError'):raise ValueError(result['content'][0]['text'])
        return result['structuredContent']
    def close(self):
        self.process.stdin.close()
        try:self.process.wait(timeout=3)
        except subprocess.TimeoutExpired:self.process.terminate();self.process.wait(timeout=3)

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--binary',type=Path,required=True);parser.add_argument('--parameters',type=Path,required=True);parser.add_argument('--port',type=int,default=8774)
    args=parser.parse_args();bridge=Bridge(args.binary.resolve(),args.parameters.resolve())
    origin=f'http://127.0.0.1:{args.port}'
    class Handler(BaseHTTPRequestHandler):
        def reply(self,status,value,kind='application/json'):
            data=value if isinstance(value,bytes) else json.dumps(value).encode()
            self.send_response(status);self.send_header('Content-Type',kind);self.send_header('Content-Length',str(len(data)));self.send_header('Cache-Control','no-store');self.end_headers();self.wfile.write(data)
        def allowed(self):
            return self.headers.get('Host')==f'127.0.0.1:{args.port}' and self.headers.get('Origin',origin)==origin
        def do_GET(self):
            if not self.allowed():self.reply(403,{'error':'local origin required'});return
            try:
                if self.path=='/':self.reply(200,Path(__file__).with_name('index.html').read_bytes(),'text/html; charset=utf-8')
                elif self.path=='/schema':self.reply(200,bridge.tool('model_schema'))
                elif self.path=='/body':self.reply(200,bridge.tool('model_get'))
                elif self.path=='/film':self.reply(200,bridge.tool('film_get'))
                elif self.path=='/view':self.reply(200,bridge.tool('view_get'))
                elif self.path=='/measurements':self.reply(200,bridge.tool('model_measurements'))
                else:self.reply(404,{'error':'not found'})
            except (ValueError,RuntimeError) as e:self.reply(400,{'error':str(e)})
        def do_POST(self):
            if not self.allowed():self.reply(403,{'error':'local origin required'});return
            try:
                size=int(self.headers.get('Content-Length','0'))
                if size<=0 or size>65536:raise ValueError('invalid body size')
                value=json.loads(self.rfile.read(size))
                if self.path=='/body':result=bridge.tool('model_update',{'parameters':value})
                elif self.path=='/reset':result=bridge.tool('model_reset')
                elif self.path=='/view':
                    result=bridge.tool('view_update',{'settings':value})
                    self.reply(200,{'settings':result,'saved':True});return
                elif self.path=='/film':
                    result=bridge.tool('film_update',{'settings':value})
                    self.reply(200,{'settings':result,'saved':True});return
                else:self.reply(404,{'error':'not found'});return
                self.reply(200,{'parameters':result,'saved':True,'liveApplied':None})
            except (ValueError,RuntimeError) as e:self.reply(400,{'error':str(e)})
    server=ThreadingHTTPServer(('127.0.0.1',args.port),Handler)
    print(origin,flush=True)
    try:server.serve_forever()
    except KeyboardInterrupt:pass
    finally:server.server_close();bridge.close()
if __name__=='__main__':main()
