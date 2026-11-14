import {writeFileSync} from 'node:fs';
import {basename, resolve} from 'node:path';
import assert from 'node:assert/strict';

const [port, url, fixture, screenshot, mode] = process.argv.slice(2);
const touch = mode === "touch";
const gamepad = mode === "gamepad";
const follow = mode === "follow" || touch || gamepad;
const framesPerTick = follow ? 2 : 1;
assert.ok(port && url && fixture,
  'Usage: node project-character.mjs CDP_PORT GAME_URL WALK_VPAK [SCREENSHOT] [follow|touch|gamepad]');
const packageName = basename(fixture);
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
  const {identifier} = await send('Page.addScriptToEvaluateOnNewDocument', {source: `
    (() => {
      if (${gamepad}) {
        window.padSample = null;
        Object.defineProperty(navigator, 'getGamepads', {
          configurable: true,
          value: () => [window.padSample],
        });
      }
      const frames = new Map();
      let next = 0;
      let time = 0;
      window.requestAnimationFrame = callback => { frames.set(++next, callback); return next; };
      window.cancelAnimationFrame = id => frames.delete(id);
      window.driveFrames = count => {
        for (let i = 0; i < count; i++) {
          time += 1000 / (60 * ${framesPerTick}) + 0.00001;
          const ready = [...frames.values()];
          frames.clear();
          for (const frame of ready) frame(time);
        }
      };
    })();`});
  if (touch) {
    await send('Emulation.setDeviceMetricsOverride', {
      width: 420, height: 820, deviceScaleFactor: 2, mobile: true,
    });
    await send('Emulation.setTouchEmulationEnabled', {enabled: true, maxTouchPoints: 3});
  }
  await send('Page.navigate', {url});
  await send('Page.bringToFront');
  await waitFor("document.querySelector('#package')?.disabled === false");
  await send('Page.removeScriptToEvaluateOnNewDocument', {identifier});
  await evaluate(`(async () => {
    const {WebEngine} = await import(${JSON.stringify(moduleUrl)});
    const step = WebEngine.prototype.advance_project_frame;
    window.testCharacter = {ticks: 0, camera: []};
    WebEngine.prototype.advance_project_frame = function(elapsed) {
      const ticks = step.call(this, elapsed);
      window.testCharacter.engine = this;
      window.testCharacter.ticks += ticks;
      if (${follow}) {
        window.testCharacter.camera.push({ticks, target: [...this.project_camera_target()]});
      }
      return ticks;
    };
    return true;
  })()`);
  const {root} = await send('DOM.getDocument');
  const {nodeId} = await send('DOM.querySelector', {nodeId: root.nodeId, selector: '#package'});
  await send('DOM.setFileInputFiles', {nodeId, files: [resolve(fixture)]});
  await waitFor(`document.querySelector('#status').textContent === ${JSON.stringify(packageName)}`);
  assert.equal(await evaluate("document.activeElement.id"), "game");
  await evaluate('driveFrames(1)');
  const rows = {};
  let total = 0;
  async function advance(label, count) {
    total += count;
    rows[label] = await evaluate(`(() => {
      driveFrames(${count * framesPerTick});
      const t = window.testCharacter;
      return {ticks: t.ticks, scene: JSON.parse(t.engine.project_game_state())};
    })()`);
    assert.equal(rows[label].ticks, total);
  }
  async function key(code, key, windowsVirtualKeyCode, pressed) {
    await send('Input.dispatchKeyEvent', {
      type: pressed ? 'rawKeyDown' : 'keyUp', code, key, windowsVirtualKeyCode,
    });
  }
  if (gamepad) {
    async function pad(x, z = 0, buttons = [], connected = true) {
      await evaluate(`window.padSample = ${JSON.stringify(connected ? {
        connected, index: 0, mapping: 'standard', axes: [x, z, 0, 0],
        buttons: Array.from({length: 17}, (_, index) => ({pressed: buttons.includes(index)})),
      } : null)}`);
    }
    const pose = label => rows[label].scene.objects.find(object => object.id === 'player').translation;
    await pad(0.08);
    await advance('deadZone', 30);
    assert.equal(pose('deadZone')[0], 0);
    await pad(0.575);
    await advance('halfStick', 12);
    assert.ok(Math.abs(pose('halfStick')[0] - 0.2) < 1e-6);
    await pad(0.575, 0, [0]);
    await evaluate('driveFrames(1)');
    await pad(0.575);
    await evaluate('driveFrames(1)');
    total++;
    await advance('jump', 10);
    assert.ok(pose('jump')[0] > pose('halfStick')[0] + 0.15);
    assert.ok(pose('jump')[1] > 0.45);
    await pad(0, 0, [], false);
    await advance('disconnected', 40);
    assert.deepEqual(pose('disconnected'), [pose('jump')[0], 0.2, 0]);
    await pad(1);
    await advance('unarmed', 10);
    assert.deepEqual(pose('unarmed'), pose('disconnected'));
    await pad(0);
    await advance('neutral', 1);
    await pad(0, 0, [14]);
    await advance('dpad', 4);
    assert.ok(pose('dpad')[0] < pose('neutral')[0] - 0.1);
    await evaluate("document.querySelector('#package').focus()");
    await advance('blurred', 10);
    await evaluate("document.querySelector('#game').focus()");
    await advance('refocusedHeld', 10);
    assert.deepEqual(pose('refocusedHeld'), pose('blurred'));
    assert.deepEqual(pose('blurred'), pose('dpad'));
    await pad(0);
    await advance('rearmed', 1);
    await pad(-0.575);
    await key('KeyD', 'd', 68, true);
    await advance('combined', 4);
    await key('KeyD', 'd', 68, false);
    await advance('stickOnly', 4);
    assert.ok(pose('combined')[0] > pose('rearmed')[0] + 0.06);
    assert.ok(pose('stickOnly')[0] < pose('combined')[0] - 0.06);
    assert.deepEqual(errors, []);
    console.log(JSON.stringify(rows, null, 2));
  } else {
    const touches = new Map();
    async function touchDown(id, code) {
      const point = await evaluate(`(() => {
        const button = document.querySelector('[data-code="${code}"]');
        const rect = button.getBoundingClientRect();
        return {x: rect.x + rect.width / 2, y: rect.y + rect.height / 2};
      })()`);
      assert.ok(point.x > 0 && point.y > 0);
      touches.set(id, {...point, id});
      await send('Input.dispatchTouchEvent', {type: 'touchStart', touchPoints: [...touches.values()]});
    }
    async function touchEnd(id) {
      touches.delete(id);
      await send('Input.dispatchTouchEvent', {type: 'touchEnd', touchPoints: [...touches.values()]});
    }
    await advance('settled', 30);
    if (touch) await touchDown(1, 'KeyD');
    else await key('KeyD', 'd', 68, true);
    await advance('wall', 45);
    if (touch) {
      await touchDown(2, 'Space');
      await touchEnd(2);
      assert.equal(await evaluate("document.querySelectorAll('[data-held]').length"), 1);
    } else {
      await key('KeyD', 'd', 68, false);
      await key('Space', ' ', 32, true);
      await key('Space', ' ', 32, false);
    }
    await advance('jump', 11);
    if (touch) await touchEnd(1);
    await advance('landed', 40);
    if (touch) await touchDown(3, 'KeyA');
    await key('KeyA', 'a', 65, true);
    await advance('left', touch ? 4 : 8);
    if (touch) {
      await key('KeyA', 'a', 65, false);
      await advance('touchOnly', 4);
      await send('Input.dispatchTouchEvent', {type: 'touchCancel', touchPoints: []});
      touches.clear();
      assert.equal(await evaluate("document.querySelectorAll('[data-held]').length"), 0);
      await advance('cancelled', 10);
      await touchDown(4, 'KeyD');
      await advance('focusHeld', 4);
    }
    await evaluate("document.querySelector('#package').focus()");
    await advance('stopped', 10);
    await key('KeyA', 'a', 65, false);
    const pose = label => rows[label].scene.objects.find(object => object.id === 'player').translation;
    assert.ok(Math.abs(pose('settled')[1] - 0.2) < 1e-6);
    assert.ok(Math.abs(pose('wall')[0] - 0.8) < 1e-6);
    assert.ok(pose('jump')[1] > pose('wall')[1] + 0.25);
    assert.ok(Math.abs(pose('landed')[1] - 0.2) < 1e-6);
    if (touch) {
      assert.ok(pose('touchOnly')[0] < pose('left')[0] - 0.1);
      assert.deepEqual(pose('touchOnly'), pose('cancelled'));
      assert.ok(pose('focusHeld')[0] > pose('cancelled')[0] + 0.1);
      assert.deepEqual(pose('focusHeld'), pose('stopped'));
      assert.equal(await evaluate("document.querySelectorAll('[data-held]').length"), 0);
    } else {
      assert.ok(pose('left')[0] < pose('landed')[0] - 0.2);
      assert.deepEqual(pose('left'), pose('stopped'));
    }
    if (follow) {
      const trace = await evaluate('window.testCharacter.camera');
      const start = 1 + 30 * framesPerTick;
      const moving = trace.slice(start + 3, start + 30);
      let subTickMoves = 0;
      for (let i = 1; i < moving.length; i++) {
        assert.ok(moving[i].target[0] > moving[i - 1].target[0]);
        if (moving[i].ticks === 0) subTickMoves++;
      }
      assert.ok(subTickMoves >= 10);
      rows.camera = {subTickMoves, trace};
    }
    assert.deepEqual(errors, []);
    if (screenshot) {
      const {data} = await send('Page.captureScreenshot');
      writeFileSync(screenshot, Buffer.from(data, 'base64'));
    }
    console.log(JSON.stringify(rows, null, 2));
  }
} finally {
  if (touch) {
    await send('Emulation.setTouchEmulationEnabled', {enabled: false});
    await send('Emulation.clearDeviceMetricsOverride');
  }
  ws.close();
}
