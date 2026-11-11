import {writeFileSync} from 'node:fs';
import {basename, resolve} from 'node:path';
import assert from 'node:assert/strict';

const [port, url, fixture, mode, screenshot] = process.argv.slice(2);
assert.ok(port && url && fixture && ['auto', 'retry'].includes(mode),
  'Usage: node project-recovery.mjs CDP_PORT GAME_URL GLTF_VPAK auto|retry [SCREENSHOT]');
const packageName = basename(fixture);
const packageUrl = new URL(encodeURIComponent(packageName), url).href;
const moduleUrl = new URL('pkg/voxy_web.js', url).href;
const targets = await (await fetch('http://127.0.0.1:' + port + '/json/list')).json();
const page = targets.find(target => target.type === 'page');
assert.ok(page?.webSocketDebuggerUrl, 'Launch Chrome with a blank tab and CDP enabled');
const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise(resolve => ws.addEventListener('open', resolve, {once:true}));
let id = 0;
const waiting = new Map();
const errors = [];
ws.addEventListener('message', event => {
  const m = JSON.parse(event.data);
  if (m.id) { waiting.get(m.id)?.(m); waiting.delete(m.id); }
  if (m.method === 'Runtime.exceptionThrown') errors.push(m.params.exceptionDetails);
});
async function send(method, params = {}) {
  const seq = ++id;
  const p = new Promise(resolve => waiting.set(seq, resolve));
  ws.send(JSON.stringify({id:seq, method, params}));
  const reply = await p;
  assert.equal(reply.error, undefined, JSON.stringify(reply.error));
  return reply.result;
}
async function evaluate(expression) {
  const result = await send('Runtime.evaluate', {expression,awaitPromise:true,returnByValue:true});
  assert.equal(result.exceptionDetails, undefined, JSON.stringify(result.exceptionDetails));
  return result.result.value;
}
async function waitFor(expression) {
  for (let attempt = 0; attempt < 120; attempt++) {
    if (await evaluate(expression)) return;
    await new Promise(resolve => setTimeout(resolve, 250));
  }
  throw Error('Timed out: ' + await evaluate("document.querySelector('#status')?.textContent"));
}
try {
  await send('Runtime.enable');
  await send('Page.enable');
  await send('Page.navigate', {url});
  await waitFor("document.querySelector('#package')?.disabled === false");
  await evaluate(`(async () => {
    const {WebEngine} = await import(${JSON.stringify(moduleUrl)});
    const create = WebEngine.create;
    const step = WebEngine.prototype.advance_project_frame;
    const render = WebEngine.prototype.render;
    window.testRecovery = {ticks:0, frames:0, failedAttempts:0, failNext:false};
    const t = window.testRecovery;
    WebEngine.create = function(...args) {
      if (t.failNext) {
        t.failNext = false;
        ++t.failedAttempts;
        return Promise.reject(new Error('injected adapter request failure'));
      }
      return create.apply(this,args);
    };
    WebEngine.prototype.advance_project_frame = function(elapsed) {
      if (t.last !== this && t.last) t.restored = this.project_game_state();
      t.last = this;
      const ticks = step.call(this, elapsed);
      t.ticks += ticks;
      return ticks;
    };
    WebEngine.prototype.render = function(...args) {
      const presented = render.apply(this,args);
      if (presented) ++t.frames;
      return presented;
    };
    return true;
  })()`);
  const {root} = await send('DOM.getDocument');
  const {nodeId} = await send('DOM.querySelector', {nodeId:root.nodeId,selector:'#package'});
  await send('DOM.setFileInputFiles', {nodeId,files:[resolve(fixture)]});
  await waitFor("window.testRecovery.ticks >= 30");
  const before = await evaluate(`(() => {
    const t = window.testRecovery;
    t.canvas = document.querySelector('canvas');
    t.failNext = ${mode === 'retry'};
    const state = t.last.project_game_state();
    t.last.destroy_device_for_validation();
    return {state,ticks:t.ticks,frames:t.frames,backend:t.last.backend()};
  })()`);
  assert.equal(before.backend, 'BrowserWebGpu');
  let stopped = null;
  if (mode === 'retry') {
    await waitFor("document.querySelector('#recover').hidden === false");
    stopped = await evaluate(`(() => {
      const t = window.testRecovery;
      return {ticks:t.ticks,frames:t.frames,failedAttempts:t.failedAttempts,state:t.last.project_game_state()};
    })()`);
    assert.equal(stopped.failedAttempts,1);
    assert.equal(stopped.ticks,before.ticks);
    assert.equal(stopped.frames,before.frames);
    assert.equal(stopped.state,before.state);
    await evaluate("document.querySelector('#recover').click()");
  }
  await waitFor(`document.querySelector('#status').textContent === ${JSON.stringify(packageName)} && window.testRecovery.restored`);
  await waitFor(`window.testRecovery.ticks >= ${before.ticks + 30}`);
  const after = await evaluate(`(() => {
    const t = window.testRecovery;
    return {state:t.last.project_game_state(),restored:t.restored,ticks:t.ticks,frames:t.frames,
            newCanvas:t.canvas !== document.querySelector('canvas'),backend:t.last.backend()};
  })()`);
  assert.equal(after.restored,before.state);
  assert.notEqual(after.state,before.state);
  assert.ok(after.frames > before.frames);
  assert.equal(after.newCanvas,true);
  assert.equal(after.backend,'BrowserWebGpu');
  const control = await evaluate(`(async () => {
    const {WebEngine} = await import(${JSON.stringify(moduleUrl)});
    const engine = await WebEngine.create(document.createElement('canvas'), 'webgpu');
    const bytes = new Uint8Array(await (await fetch(${JSON.stringify(packageUrl)})).arrayBuffer());
    engine.load_project_package(bytes);
    let remaining = ${after.ticks};
    while (remaining > 0) {
      const batch = Math.min(remaining, 120);
      engine.step_project_game(batch);
      remaining -= batch;
    }
    return engine.project_game_state();
  })()`);
  assert.equal(after.state,control);
  assert.deepEqual(errors,[]);
  if (screenshot) {
    const {data} = await send('Page.captureScreenshot');
    writeFileSync(screenshot,Buffer.from(data,'base64'));
  }
  for (const row of [before, stopped, after].filter(Boolean)) {
    row.state = JSON.parse(row.state);
    if (row.restored) row.restored = JSON.parse(row.restored);
  }
  console.log(JSON.stringify({mode,before,stopped,after,matchesUninterrupted:true},null,2));
} finally { ws.close(); }
