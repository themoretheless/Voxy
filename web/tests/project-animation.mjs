import {writeFileSync} from 'node:fs';
import {basename, resolve} from 'node:path';
import assert from 'node:assert/strict';

const [port, url, fixture, screenshot, mode] = process.argv.slice(2);
const events = mode === "events";
const transition = mode === "transition";
const retargetRoot = mode === "retarget-root";
const rotatedBasis = mode === "root-basis" || retargetRoot;
const rigidRoot = mode === "rigid-root";
const freeFade = mode === "rigid-fade-free";
const rigidFade = mode === "rigid-fade" || freeFade;
const rootMotion = mode === "root-motion" || rotatedBasis || rigidRoot;
assert.ok(port && url && fixture && screenshot,
  'Usage: node project-animation.mjs CDP_PORT GAME_URL ANIMATED_VPAK SCREENSHOT_PREFIX [events|budget|transition|root-motion|root-basis|retarget-root|rigid-root|rigid-fade|rigid-fade-free]');
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
      const frames = new Map();
      let next = 0;
      let time = 0;
      window.requestAnimationFrame = callback => { frames.set(++next, callback); return next; };
      window.cancelAnimationFrame = id => frames.delete(id);
      window.driveFrames = count => {
        for (let i = 0; i < count; i++) {
          time += 1000 / 60 + 0.00001;
          const ready = [...frames.values()];
          frames.clear();
          for (const frame of ready) frame(time);
        }
      };
    })();`});
  await send('Page.navigate', {url});
  await send('Page.bringToFront');
  await waitFor("document.querySelector('#package')?.disabled === false");
  await send('Page.removeScriptToEvaluateOnNewDocument', {identifier});
  await evaluate(`(async () => {
    const {WebEngine} = await import(${JSON.stringify(moduleUrl)});
    const step = WebEngine.prototype.advance_project_frame;
    window.testCharacter = {ticks: 0};
    if (${events}) {
      const take = WebEngine.prototype.take_project_animation_events;
      window.deferAnimationEvents = true;
      WebEngine.prototype.take_project_animation_events = function() {
        if (!window.deferAnimationEvents) return take.call(this);
      };
    }
    if (${events || transition || rootMotion || rigidFade}) {
      window.receivedAnimationEvents = [];
      document.addEventListener('voxy-animation-event', event => {
        window.receivedAnimationEvents.push(event.detail);
      });
    }
    WebEngine.prototype.advance_project_frame = function(elapsed) {
      const ticks = step.call(this, elapsed);
      window.testCharacter.engine = this;
      window.testCharacter.ticks += ticks;
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

  async function capture(label) {
    const rect = await evaluate(`(() => {
      const r = document.querySelector('canvas').getBoundingClientRect();
      return {x:r.x,y:r.y,width:r.width,height:r.height,scale:1};
    })()`);
    const {data} = await send('Page.captureScreenshot', {clip:rect});
    writeFileSync(screenshot + '-' + label + '.png', Buffer.from(data, 'base64'));
    return data;
  }

  if (mode === "budget") {
    const result = await evaluate(`(() => {
      const engine = window.testCharacter.engine;
      engine.step_project_game(29);
      let rejected = null;
      try { engine.step_project_game(1); } catch (error) { rejected = String(error); }
      const queue = JSON.parse(engine.take_project_animation_events());
      engine.step_project_game(1);
      const retry = JSON.parse(engine.take_project_animation_events());
      return {rejected,queued:queue.length,first:queue[0],last:queue.at(-1),retry};
    })()`);
    assert.match(result.rejected, /animation event budget/);
    assert.equal(result.queued, 4096);
    assert.equal(result.first.tick, 15);
    assert.deepEqual(result.last, result.first);
    assert.deepEqual(result.retry.map(({name,tick}) => [name,tick]), [['retry',30]]);
    assert.deepEqual(errors, []);
    console.log(JSON.stringify(result, null, 2));
  } else if (events) {
    await evaluate('driveFrames(15)');
    assert.equal(await evaluate('window.testCharacter.ticks'), 15);
    assert.deepEqual(await evaluate('window.receivedAnimationEvents'), []);
    await evaluate(`(() => {
      window.originalAnimationCanvas = document.querySelector('canvas');
      window.testCharacter.engine.destroy_device_for_validation();
    })()`);
    await waitFor('Boolean(window.testCharacter.engine.device_failure())');
    await evaluate('driveFrames(1)');
    await waitFor(`document.querySelector('canvas') !== window.originalAnimationCanvas
      && document.querySelector('#status').textContent === ${JSON.stringify(packageName)}`);
    await evaluate('window.deferAnimationEvents = false; driveFrames(1)');
    assert.equal(await evaluate('window.testCharacter.ticks'), 15);
    const resumed = await evaluate('window.receivedAnimationEvents');
    assert.deepEqual(resumed.map(({name,tick}) => [name,tick]), [
      ['fast-before',15], ['quarter',15], ['fast-mid',15],
    ]);
    await evaluate('driveFrames(105)');
    assert.equal(await evaluate('window.testCharacter.ticks'), 120);
    const received = await evaluate('window.receivedAnimationEvents');
    assert.deepEqual(received.map(({name,tick}) => [name,tick]), [
      ['fast-before',15], ['quarter',15], ['fast-mid',15],
      ['mid',30],
      ['fast-before',45], ['fast-mid',45],
      ['end',60], ['start',60],
      ['fast-before',75], ['quarter',75], ['fast-mid',75],
      ['mid',90],
      ['fast-before',105], ['fast-mid',105],
      ['end',120], ['start',120],
    ]);
    assert.ok(received.every(event => event.clip === 'move' && event.owner !== 'owner-1'));
    assert.equal(await evaluate('window.testCharacter.engine.take_project_animation_events() === undefined'), true);
    assert.deepEqual(errors, []);
    console.log(JSON.stringify({fixture:packageName,ticks:120,afterRecovery:resumed,received},null,2));
  } else if (rigidFade) {
    const snapshot = () => evaluate('JSON.parse(window.testCharacter.engine.project_game_state())');
    const player = scene => scene.objects.find(object => object.id === 'player');
    const request = `window.testCharacter.engine.transition_project_animation('player', JSON.stringify({
      clip: null, clip_name: 'alternate', speed: 1, transition_seconds: .5,
      root_motion_bone: 'root', root_motion_axes: [false,false,true], root_motion_rotation: true,
      events: [{name:'before-wall',phase:0.07},{name:'beyond-wall',phase:0.7}],
    }))`;
    assert.match(await evaluate(`(() => { try { ${request}; } catch (error) { return String(error); } })()`),
      /physically accepted root reference/);
    await evaluate('driveFrames(12)');
    const before = await snapshot();
    await evaluate(request);
    await evaluate('driveFrames(4)');
    const middle = await snapshot();
    assert.ok(player(middle).rotation[1] > player(before).rotation[1]);
    const middleFrame = await capture('middle');
    await evaluate(`(() => {
      window.originalAnimationCanvas = document.querySelector('canvas');
      window.testCharacter.engine.destroy_device_for_validation();
    })()`);
    await waitFor('Boolean(window.testCharacter.engine.device_failure())');
    await evaluate('driveFrames(1)');
    await waitFor(`document.querySelector('canvas') !== window.originalAnimationCanvas
      && document.querySelector('#status').textContent === ${JSON.stringify(packageName)}`);
    await evaluate('driveFrames(1)');
    assert.deepEqual(await snapshot(), middle);
    assert.ok(middleFrame === await capture('recovered'));
    await evaluate('driveFrames(80)');
    assert.equal(await evaluate("document.querySelector('#status').textContent"), packageName);
    assert.equal(await evaluate('window.testCharacter.ticks'), 96);
    const late = await snapshot();
    await evaluate('driveFrames(24)');
    const last = await snapshot();
    const received = await evaluate('window.receivedAnimationEvents');
    if (freeFade) {
      assert.ok(player(late).translation[2] > 0.06);
      assert.notDeepEqual(last, late);
      assert.deepEqual(received.slice(0, 2).map(event => event.name), ['before-wall', 'beyond-wall']);
    } else {
      assert.ok(player(late).rotation[1] > 0.30 && player(late).rotation[1] < 0.31);
      assert.ok(player(late).translation[2] > 0.03 && player(late).translation[2] < 0.05);
      assert.deepEqual(last, late);
      assert.deepEqual(received.map(event => event.name), ['before-wall']);
    }
    assert.deepEqual(errors, []);
    await capture(freeFade ? 'complete' : 'wall');
    console.log(JSON.stringify({fixture:packageName, ticks:120, before, middle, late, last, recoveredExact:true, received}, null, 2));
  } else if (rootMotion) {
    async function scene() {
      return await evaluate('JSON.parse(window.testCharacter.engine.project_game_state())');
    }
    const player = snapshot => snapshot.objects.find(object => object.id === 'player');
    const position = snapshot => player(snapshot).translation[rotatedBasis || rigidRoot ? 2 : 0];
    const snapshots = {};
    await evaluate('driveFrames(12)');
    snapshots[12] = await scene();
    assert.ok(Math.abs(position(snapshots[12]) - (rigidRoot ? 0.02 : rotatedBasis ? 0.5 : 0.4)) < 1e-6);
    const before = await capture('moving');
    await evaluate(`(() => {
      window.originalAnimationCanvas = document.querySelector('canvas');
      window.testCharacter.engine.destroy_device_for_validation();
    })()`);
    await waitFor('Boolean(window.testCharacter.engine.device_failure())');
    await evaluate('driveFrames(1)');
    await waitFor(`document.querySelector('canvas') !== window.originalAnimationCanvas
      && document.querySelector('#status').textContent === ${JSON.stringify(packageName)}`);
    await evaluate('driveFrames(1)');
    assert.deepEqual(await scene(), snapshots[12]);
    assert.ok(before === await capture('moving-recovered'), 'Root motion recovery changed the frame');
    await evaluate('driveFrames(18)');
    snapshots[30] = await scene();
    assert.ok(Math.abs(position(snapshots[30]) - (rigidRoot ? 0.039505783 : 0.8)) < 1e-6);
    if (rigidRoot) {
      assert.ok(player(snapshots[12]).rotation[1] > 0.15);
      assert.ok(player(snapshots[30]).rotation[1] > 0.30);
      assert.ok(player(snapshots[30]).rotation[1] < 0.31);
    }
    await evaluate('driveFrames(90)');
    snapshots[120] = await scene();
    if (rotatedBasis) {
      assert.ok(Math.abs(position(snapshots[120]) - position(snapshots[30])) < 1e-6);
    } else {
      assert.deepEqual(snapshots[120], snapshots[30]);
    }
    if (rigidRoot) {
      const stopped = await capture('wall');
      const result = await evaluate(`(() => {
        const engine = window.testCharacter.engine;
        const descriptor = {
          clip: null, clip_name: 'move', speed: 1, transition_seconds: .25,
          root_motion_bone: 'root', root_motion_axes: [false,false,true], root_motion_rotation: true,
        };
        let rejected = null;
        try { engine.transition_project_animation('player', JSON.stringify({...descriptor, root_motion_axes: [true,false,false]})); }
        catch (error) { rejected = String(error); }
        engine.transition_project_animation('player', JSON.stringify({...descriptor, speed:0}));
        const canvas = document.querySelector('canvas');
        engine.render(0, canvas.width, canvas.height);
        return rejected;
      })()`);
      assert.match(result, /same extraction/);
      assert.deepEqual(await scene(), snapshots[120]);
      assert.ok(stopped === await capture('paused-fade'), 'Paused fade changed the accepted character pose');
      await evaluate('driveFrames(16)');
      assert.deepEqual(await scene(), snapshots[120]);
      await evaluate(`window.testCharacter.engine.transition_project_animation('player', JSON.stringify({
        clip: null, clip_name: 'move', speed: 1, transition_seconds: 0,
        root_motion_bone: 'root', root_motion_axes: [false,false,true], root_motion_rotation: true,
      }))`);
      await evaluate('driveFrames(12)');
      assert.deepEqual(await scene(), snapshots[120]);
      assert.deepEqual(errors, []);
      console.log(JSON.stringify({fixture: packageName, snapshots, recoveredExact:true, pausedFadeRetainsContact:true, switchRetainsContact:true}, null, 2));
    } else {
      const stopped = await capture('wall');
      const response = await evaluate(`(() => {
        const engine = window.testCharacter.engine;
        const descriptor = {
          clip: null, clip_name: 'return', speed: 1, transition_seconds: .25,
          root_motion_bone: ${JSON.stringify(retargetRoot ? 'sourceHip' : 'root')},
          root_motion_axes: ${JSON.stringify(retargetRoot ? [false,false,true] : [true,false,false])},
        };
        let rejected = null;
        try {
          engine.transition_project_animation('player', JSON.stringify({
            ...descriptor, root_motion_axes: [false,true,false],
          }));
        } catch (error) { rejected = String(error); }
        engine.transition_project_animation('player', JSON.stringify(descriptor));
        const canvas = document.querySelector('canvas');
        engine.render(0, canvas.width, canvas.height);
        return rejected;
      })()`);
      assert.match(response, /same extraction/);
      assert.ok(stopped === await capture('turnaround-start'), 'Fade changed the accepted pose');
      await evaluate('driveFrames(30)');
      const afterTransition = await scene();
      assert.ok(position(afterTransition) < 0.4);
      const received = await evaluate('window.receivedAnimationEvents');
      assert.equal(received.length, 2);
      assert.ok(received.every(event => event.owner === 'player' && event.name === 'plant'));
      assert.deepEqual(errors, []);
      console.log(JSON.stringify({
        fixture: packageName, snapshots, afterTransition, recoveredExact: true, received,
      }, null, 2));
    }
  } else if (transition) {
    async function switchClip(clipName, seconds, markers = []) {
      return await evaluate(`(() => {
        const engine = window.testCharacter.engine;
        try {
          engine.transition_project_animation('owner-0', ${JSON.stringify(JSON.stringify({
            clip: null, clip_name: clipName, speed: 1, transition_seconds: seconds, events: markers,
          }))});
        } catch (error) {
          return String(error);
        }
        const canvas = document.querySelector('canvas');
        engine.render(0, canvas.width, canvas.height);
        return null;
      })()`);
    }
    await evaluate('driveFrames(12)');
    const before = await capture('walk');
    assert.equal(await switchClip('Run', 0.5, [{name:'run-mark',phase:0.05}]), null);
    assert.ok(before === await capture('fade-start'), 'Fade must start at the displayed pose');
    await evaluate('driveFrames(12)');
    const middle = await capture('fade-middle');
    assert.ok(middle !== before, 'Fade must advance the rendered pose');
    const eventsBeforeRecovery = await evaluate('window.receivedAnimationEvents');
    assert.deepEqual(eventsBeforeRecovery.map(event => [event.clip,event.name]), [['Run','run-mark']]);
    await evaluate(`(() => {
      window.originalAnimationCanvas = document.querySelector('canvas');
      window.testCharacter.engine.destroy_device_for_validation();
    })()`);
    await waitFor('Boolean(window.testCharacter.engine.device_failure())');
    await evaluate('driveFrames(1)');
    await waitFor(`document.querySelector('canvas') !== window.originalAnimationCanvas
      && document.querySelector('#status').textContent === ${JSON.stringify(packageName)}`);
    await evaluate('driveFrames(1)');
    assert.ok(middle === await capture('fade-recovered'), 'GPU recovery must retain the blended pose');
    assert.match(await switchClip('missing', 0.25), /missing/);
    assert.ok(middle === await capture('fade-rejected'), 'Invalid transition must retain the blended pose');
    assert.equal(await switchClip('Survey', 0.25, [{name:'survey-mark',phase:0.01}]), null);
    assert.ok(middle === await capture('interrupted-start'), 'Interruption must start at the displayed blend');
    await evaluate('driveFrames(16)');
    const finished = await capture('survey');
    assert.ok(finished !== middle, 'Replacement fade must advance');
    const received = await evaluate('window.receivedAnimationEvents');
    assert.deepEqual(received.map(event => [event.clip,event.name]), [['Run','run-mark'], ['Survey','survey-mark']]);
    assert.equal(await evaluate('window.testCharacter.ticks'), 40);
    assert.deepEqual(errors, []);
    console.log(JSON.stringify({
      fixture: packageName, ticks: 40, fadeStartExact: true, recoveredExact: true,
      rejectedExact: true, interruptionStartExact: true, received,
    }, null, 2));
  } else {
    const before = await capture('initial');
    await evaluate('driveFrames(24)');
    assert.equal(await evaluate('window.testCharacter.ticks'), 24);
    assert.equal(await evaluate("document.querySelector('#status').textContent"), packageName);
    const after = await capture('posed');
    assert.ok(before !== after, "Animated frame must differ from its initial frame");
    const delta = await evaluate(`(async () => {
      async function pixels(image) {
        const data = await (await fetch('data:image/png;base64,' + image)).blob();
        const bitmap = await createImageBitmap(data);
        const canvas = new OffscreenCanvas(bitmap.width,bitmap.height);
        const ctx = canvas.getContext('2d');
        ctx.drawImage(bitmap,0,0);
        return [bitmap.width,bitmap.height,ctx.getImageData(0,0,bitmap.width,bitmap.height).data];
      }
      const [width,height,a] = await pixels(${JSON.stringify(before)});
      const [,,b] = await pixels(${JSON.stringify(after)});
      let left = 0, right = 0;
      for (let i=0;i<width*height;i++) {
        if ([0,1,2].some(k=>a[i*4+k]!==b[i*4+k])) {
          if (i%width < width/2) left++; else right++;
        }
      }
      return {width,height,left,right};
    })()`);
    assert.ok(delta.left > 20, JSON.stringify(delta));
    assert.equal(delta.right, 0, JSON.stringify(delta));
    await evaluate(`(() => {
      window.originalAnimationCanvas = document.querySelector('canvas');
      window.testCharacter.engine.destroy_device_for_validation();
    })()`);
    await waitFor('Boolean(window.testCharacter.engine.device_failure())');
    await evaluate('driveFrames(1)');
    await waitFor(`document.querySelector('canvas') !== window.originalAnimationCanvas
      && document.querySelector('#status').textContent === ${JSON.stringify(packageName)}`);
    await evaluate('driveFrames(1)');
    assert.equal(await evaluate('window.testCharacter.ticks'), 24);
    const recovered = await capture('recovered');
    assert.ok(after === recovered, "Recovered canvas must match the animated frame byte for byte");
    assert.deepEqual(errors, []);
    console.log(JSON.stringify({fixture:packageName,ticks:24,delta,recoveredExact:true},null,2));
  }
} finally { ws.close(); }
