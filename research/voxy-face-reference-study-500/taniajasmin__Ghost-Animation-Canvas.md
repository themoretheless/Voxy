# Facial rendering source scan: taniajasmin/Ghost-Animation-Canvas

Repository: https://github.com/taniajasmin/Ghost-Animation-Canvas

Pinned source revision: 59936577fc9786aa827be6d3d3433db5bd84f1c3

Discovery topic: tongue animation

Description: Adorable cat-ghost on HTML5 canvas — mouse-guided movement, glossy eyes, click-to-tongue, wink timing, keyboard zoom, DPR-aware and responsive.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: main.js

{
  "path": "main.js",
  "blob_sha": "2d15c0f6862a6173e4f680f5c143f968664eddd8",
  "sha256": "6db850467d4cd3bfc51b3bb0cbed558b59ce3374dceae6390126f8f73dc3d2cf",
  "lines": 756,
  "bytes": 27795,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 50,
    "mouth": 2,
    "tongue": 3,
    "iris": 1,
    "rig": 3,
    "face": 3,
    "lip": 16
  },
  "url": "https://github.com/taniajasmin/Ghost-Animation-Canvas/blob/59936577fc9786aa827be6d3d3433db5bd84f1c3/main.js",
  "function_signature_candidates": []
}

```text

// // ---------------- Canvas (HiDPI) ----------------
// const canvas = document.getElementById('scene');
// const ctx = canvas.getContext('2d', { alpha: false });

// let DPR = 1;
// function fit() {
//   DPR = Math.max(1, Math.min(2, window.devicePixelRatio || 1));
//   // Size canvas to CSS viewport
//   const w = document.documentElement.clientWidth;
//   const h = document.documentElement.clientHeight;
//   canvas.style.width  = w + 'px';
//   canvas.style.height = h + 'px';
//   canvas.width  = Math.round(w * DPR);
//   canvas.height = Math.round(h * DPR);
//   ctx.setTransform(1, 0, 0, 1, 0, 0);
//   ctx.scale(DPR, DPR); // draw in CSS pixels
// }
// addEventListener('resize', fit, { passive: true });
// fit();

// // ---------------- Input ----------------
// const keys = new Set();
// addEventListener('keydown', e => keys.add(e.key));
// addEventListener('keyup',   e => keys.delete(e.key));

// const mouse = { x: null, y: null, down: false };
// function mapPointer(clientX, clientY) {
//   // map to CSS coords (we draw in CSS px)
//   mouse.x = clientX;
//   mouse.y = clientY;
// }
// canvas.addEventListener('mousemove', e => mapPointer(e.clientX, e.clientY), { passive: true });
// canvas.addEventListener('mousedown', () => { mouse.down = true; });
// canvas.addEventListener('mouseup',   () => { mouse.down = false; });

// canvas.addEventListener('touchstart', e => {
//   e.preventDefault();
//   const t = e.touches[0];
//   mapPointer(t.clientX, t.clientY);
//   mouse.down = true;
// }, { passive: false });

// canvas.addEventListener('touchmove', e => {
//   e.preventDefault();
//   const t = e.touches[0];
//   mapPointer(t.clientX, t.clientY);
// }, { passive: false });

// canvas.addEventListener('touchend', () => { mouse.down = false; }, { passive: false });

// // ---------------- Utils ----------------
// const TAU = Math.PI * 2;
// const clamp = (v,a,b) => Math.max(a, Math.min(b, v));
// const lerp  = (a,b,t) => a + (b - a) * t;

// // // ---------------- Background ----------------
// // function drawFluidBackground(t) {
// //   const W = document.documentElement.clientWidth;
// //   const H = document.documentElement.clientHeight;

// //   // base dark gradient
// //   const base = ctx.createLinearGradient(0, 0, 0, H);
// //   base.addColorStop(0, '#0a1024');
// //   base.addColorStop(1, '#16203a');
// //   ctx.fillStyle = base;
// //   ctx.fillRect(0, 0, W, H);

// //   // moving color blobs
// //   const mx = mouse.x ?? W / 2;
// //   const my = mouse.y ?? H / 2;
// //   for (let i = 0; i < 3; i++) {
// //     const hue = (t * 0.02 + i * 120) % 360;
// //     const cx = mx + Math.sin(t * 0.001 + i) * (W * 0.20);
// //     const cy = my + Math.cos(t * 0.001 + i) * (H * 0.15);
// //     const grad = ctx.createRadialGradient(cx, cy, 0, W / 2, H / 2, Math.max(W, H));
// //     grad.addColorStop(0, `hsla(${hue}, 70%, 70%, 0.28)`);
// //     grad.addColorStop(1, `hsla(${hue}, 70%, 30%, 0.00)`);
// //     ctx.fillStyle = grad;
// //     ctx.fillRect(0, 0, W, H);
// //   }

// //   // floating sparkles
// //   ctx.save();
// //   ctx.globalAlpha = 0.6;
// //   ctx.fillStyle = '#ffffff';
// //   for (let i = 0; i < 18; i++) {
// //     const x = (Math.sin(t * 0.0005 + i * 0.7) * 0.5 + 0.5) * W;
// //     const y = (Math.cos(t * 0.0007 + i * 1.3) * 0.5 + 0.5) * H;
// //     const r = 1.5 + Math.sin(t * 0.002 + i) * 1.1;
// //     ctx.beginPath(); ctx.arc(x, y, r, 0, TAU); ctx.fill();
// //   }
// //   ctx.restore();

// //   // vignette
// //   const vg = ctx.createRadialGradient(W/2, H/2, 0, W/2, H/2, Math.max(W, H)/1.05);
// //   vg.addColorStop(0, 'rgba(0,0,0,0)');
// //   vg.addColorStop(1, 'rgba(0,0,0,0.45)');
// //   ctx.fillStyle = vg;
// //   ctx.fillRect(0, 0, W, H);
// // }

// // ---------------- Background ----------------
// function drawFluidBackground(t) {
//   const W = document.documentElement.clientWidth;
//   const H = document.documentElement.clientHeight;

//   // deep purple vertical gradient
//   const base = ctx.createLinearGradient(0, 0, 0, H);
//   base.addColorStop(0, '#2a1a49');  // top
//   base.addColorStop(1, '#3a225d');  // bottom
//   ctx.fillStyle = base;
//   ctx.fillRect(0, 0, W, H);

//   // very subtle moving bloom for richness
//   for (let i = 0; i < 2; i++) {
//     const hue = 285 + i * 20;
//     const cx = (Math.sin(t * 0.0006 + i) * 0.35 + 0.5) * W;
//     const cy = (Math.cos(t * 0.0005 + i * 1.7) * 0.35 + 0.55) * H;
//     const g = ctx.createRadialGradient(cx, cy, 0, cx, cy, Math.max(W, H) * 0.9);
//     g.addColorStop(0, `hsla(${hue}, 55%, 65%, 0.12)`);
//     g.addColorStop(1, `hsla(${hue}, 55%, 30%, 0.00)`);
//     ctx.fillStyle = g; ctx.fillRect(0, 0, W, H);
//   }

//   // vignette
//   const vg = ctx.createRadialGradient(W/2, H/2, 0, W/2, H/2, Math.max(W, H));
//   vg.addColorStop(0, 'rgba(0,0,0,0)');
//   vg.addColorStop(1, 'rgba(0,0,0,0.55)');
//   ctx.fillStyle = vg; ctx.fillRect(0, 0, W, H);
// }

// // ---------------- Wink ----------------
// function doubleWinkOpenness(ms, closeScale = 1.3, scale = 1){
//   if (scale < closeScale) return 1;
//   const t = ms % 4000;
//   const starts = [420, 960];
//   const half = 160, total = half * 2;
//   let open = 1;
//   for (const s of starts){
//     if (t >= s && t < s + total){
//       const u = (t - s) / half;
//       open = Math.min(open, (u <= 1) ? (1 - u) : (u - 1));
//     }
//   }
//   return Math.max(0, Math.min(1, open));
// }

// // ---------------- Ghost ----------------
// // function drawGhost(ctx, x, y, t, scale = 1) {
// //   const wobble = 1 + Math.sin(t * 0.003) * 0.02;
// //   const s = scale * wobble;

// //   // halo
// //   ctx.save();
// //   ctx.globalCompositeOperation = 'lighter';
// //   for (let i = 0; i < 18; i++){
// //     const r = 92 * s * (1 + i / 18);
// //     const a = 0.16 * (1 - i / 18) ** 2;
// //     ctx.fillStyle = `rgba(150,180,255,${a})`;
// //     ctx.beginPath(); ctx.arc(x, y - 8 * s, r, 0, TAU); ctx.fill();
// //   }
// //   ctx.restore();

// //   ctx.save();
// //   ctx.translate(x, y);

// //   // body silhouette
// //   const headW = 160*s, headH = 150*s;
// //   const hemY = 58*s, hemAmp = 14*s, hemW = 150*s;

// //   ctx.beginPath();
// //   ctx.moveTo(-headW*0.55, -headH*0.35);
// //   ctx.bezierCurveTo(-headW*0.55, -headH*1.05, headW*0.55, -headH*1.05, headW*0.55, -headH*0.35);
// //   ctx.bezierCurveTo(headW*0.75, -headH*0.05, headW*0.72, headH*0.15, headW*0.52, hemY);
// //   const bumps = 5, left = -hemW/2;
// //   for (let i=0;i<bumps;i++){
// //     const x0 = left + (i)*hemW/bumps;
// //     const x1 = left + (i+0.5)*hemW/bumps;
// //     const x2 = left + (i+1)*hemW/bumps;
// //     ctx.bezierCurveTo(x0 + hemW/(bumps*4), hemY + hemAmp, x1 - hemW/(bumps*4), hemY + hemAmp, x1, hemY);
// //     ctx.bezierCurveTo(x1 + hemW/(bumps*4), hemY - hemAmp, x2 - hemW/(bumps*4), hemY - hemAmp, x2, hemY);
// //   }
// //   ctx.bezierCurveTo(-headW*0.72, headH*0.15, -headW*0.75, -headH*0.05, -headW*0.55, -headH*0.35);
// //   ctx.closePath();

// //   // fill + rim
// //   const fill = ctx.createRadialGradient(-headW*0.25, -headH*0.9, 10*s, 0, 0, 240*s);
// //   fill.addColorStop(0.00, 'rgba(255,255,255,0.98)');
// //   fill.addColorStop(0.60, 'rgba(239,243,255,0.96)');
// //   fill.addColorStop(1.00, 'rgba(225,232,255,0.94)');
// //   ctx.fillStyle = fill; ctx.fill();

// //   const rim = ctx.createLinearGradient(-headW, -headH, headW, headH);
// //   rim.addColorStop(0, 'rgba(180,195,250,0.35)');
// //   rim.addColorStop(1, 'rgba(120,140,220,0.15)');
// //   ctx.strokeStyle = rim; ctx.lineWidth = Math.max(2, 2*s); ctx.stroke();

// //   // top gloss
// //   ctx.fillStyle = 'rgba(255,255,255,0.40)';
// //   ctx.beginPath(); ctx.ellipse(-headW*0.18, -headH*0.80, 46*s, 28*s, 0.18, 0, TAU); ctx.fill();

// //   // face
// //   const eyeOpen = doubleWinkOpenness(t, 1.3, scale);
// //   const eyeWhiteR = 19*s, pupilR = 9.5*s;
// //   const eyeL = { x: -36*s, y: -4*s }, eyeR = { x: 36*s, y: -4*s };

// //   // pupil follow
// //   const gx = x, gy = y;
// //   const mx = mouse.x ?? gx, my = mouse.y ?? gy;
// //   const pupilOffset = (ex, ey) => {
// //     const dx = mx - (gx + ex), dy = my - (gy + ey);
// //     const d = Math.hypot(dx, dy) || 1;
// //     const max = 4.2 * s;
// //     return { x: (dx/d)*max, y: (dy/d)*max };
// //   };
// //   const offL = pupilOffset(eyeL.x, eyeL.y);
// //   const offR = pupilOffset(eyeR.x, eyeR.y);

// //   function paintEye(c, off){
// //     ctx.fillStyle = '#fff';
// //     ctx.beginPath(); ctx.arc(c.x, c.y, eyeWhiteR, 0, TAU); ctx.fill();

// //     if (eyeOpen >= 0.6) {
// //       ctx.fillStyle = '#1f2647';
// //       ctx.beginPath(); ctx.arc(c.x + off.x, c.y + off.y, pupilR, 0, TAU); ctx.fill();
// //       ctx.fillStyle = '#fff';
// //       ctx.beginPath(); ctx.arc(c.x + off.x + 4*s, c.y + off.y - 4*s, Math.max(2,2*s), 0, TAU); ctx.fill();
// //     } else if (eyeOpen >= 0.3) {
// //       ctx.fillStyle = '#1f2647';
// //       ctx.beginPath(); ctx.ellipse(c.x, c.y, pupilR, 3*s, 0, 0, TAU); ctx.fill();
// //     } else {
// //       ctx.strokeStyle = '#1f2647'; ctx.lineWidth = Math.max(2,2*s);
// //       ctx.beginPath(); ctx.moveTo(c.x - eyeWhiteR + 2*s, c.y);
// //       ctx.lineTo(c.x + eyeWhiteR - 2*s, c.y); ctx.stroke();
// //     }
// //   }
// //   paintEye(eyeL, offL); paintEye(eyeR, offR);

// //   // blush + smile + dimples
// //   ctx.fillStyle = 'rgba(255,160,190,0.9)';
// //   ctx.beginPath(); ctx.arc(-60*s, 14*s, 8*s, 0, TAU); ctx.fill();
// //   ctx.beginPath(); ctx.arc( 60*s, 14*s, 8*s, 0, TAU); ctx.fill();

// //   ctx.strokeStyle = '#1f2647'; ctx.lineWidth = Math.max(3, 3*s);
// //   ctx.beginPath(); ctx.arc(0, 14*s, 22*s, Math.PI*0.12, Math.PI - Math.PI*0.12); ctx.stroke();
// //   ctx.fillStyle = '#1f2647';
// //   ctx.beginPath(); ctx.arc(-18*s, 15*s, 1.8*s, 0, TAU); ctx.fill();
// //   ctx.beginPath(); ctx.arc( 18*s, 15*s, 1.8*s, 0, TAU); ctx.fill();

// //   // tongue on press (optional)
// //   if (mouse.down && eyeOpen >= 0.6) {
// //     ctx.fillStyle = '#ff9eb6';
// //     ctx.beginPath(); ctx.arc(0, 26*s, 8*s, 0, Math.PI); ctx.fill();
// //   }

// //   ctx.restore();

// //   // ground shadow
// //   ctx.save();
// //   const shadowY = y + 84*s;
// //   ctx.globalAlpha = 0.22;
// //   ctx.fillStyle = '#000';
// //   ctx.beginPath(); ctx.ellipse(x, shadowY, 72*s, 18*s, 0, 0, TAU); ctx.fill();
// //   ctx.restore();
// // }

// // ---------------- Ghost ----------------
// function drawGhost(ctx, x, y, t, scale = 1) {
//   // palette
//   const bodyHi   = '#ffffff';
//   const bodyMid  = '#f6f2ff';
//   const bodyLow  = '#eae6ff';
//   const eyeDark  = '#1b1b25';
//   const pinkGlow = 'rgba(255,120,170,0.75)';      // rim-light color
//   const pinkSoft = 'rgba(255,120,170,0.25)';

//   const wobble = 1 + Math.sin(t * 0.003) * 0.02;
//   const s = scale * wobble;

//   ctx.save();
//   ctx.translate(x, y);

//   // ----- BODY (round teardrop) -----
//   const W = 220 * s, H = 260 * s; // rounder & fuller
//   ctx.beginPath();
//   ctx.ellipse(0, -20*s, W*0.46, H*0.42, 0.06, 0, TAU); // main blob
//   // soft hem (no big scallops to match ref)
//   ctx.quadraticCurveTo(W*0.28, 70*s, 0, 86*s);
//   ctx.quadraticCurveTo(-W*0.28, 70*s, -W*0.46, -20*s);
//   ctx.closePath();

//   // creamy fill
//   const body = ctx.createRadialGradient(-W*0.1, -H*0.7, 10*s, 0, 0, Math.max(W,H));
//   body.addColorStop(0.00, bodyHi);
//   body.addColorStop(0.55, bodyMid);
//   body.addColorStop(1.00, bodyLow);
//   ctx.fillStyle = body; ctx.fill();

//   // subtle outline
//   const rim = ctx.createLinearGradient(-W, -H, W, H);
//   rim.addColorStop(0, 'rgba(160,175,235,0.28)');
//   rim.addColorStop(1, 'rgba(120,140,220,0.10)');
//   ctx.strokeStyle = rim; ctx.lineWidth = Math.max(2, 2*s); ctx.stroke();

//   // bottom pink rim-light
//   const rl = ctx.createRadialGradient(0, 84*s, 4*s, 0, 84*s, 160*s);
//   rl.addColorStop(0, pinkGlow);
//   rl.addColorStop(1, 'rgba(255,120,170,0)');
//   ctx.globalCompositeOperation = 'lighter';
//   ctx.fillStyle = rl; ctx.fill();
//   ctx.globalCompositeOperation = 'source-over';

//   // ----- EARS (small rounded cat ears) -----
//   function ear(ax, ay, flip=1){
//     ctx.save(); ctx.translate(ax, ay); ctx.scale(flip, 1);
//     ctx.beginPath();
//     ctx.moveTo(0, 0);
//     ctx.quadraticCurveTo(26*s, -30*s, 18*s, -2*s);
//     ctx.quadraticCurveTo(14*s, 12*s, 0, 10*s);
//     ctx.closePath();
//     ctx.fillStyle = bodyMid; ctx.fill();
//     ctx.restore();
//   }
//   ear(-70*s, -120*s, -1);
//   ear( 70*s, -108*s,  1);

//   // ----- FACE -----
//   const eyeOpen = doubleWinkOpenness(t, 1.3, scale);
//   const eyeR = 30 * s;
//   const eyeLpos = { x: -52*s, y: -38*s };
//   const eyeRpos = { x:  52*s, y: -38*s };

//   // pupil follow
//   const gx = x, gy = y;
//   const mx = mouse.x ?? gx, my = mouse.y ?? gy;
//   const offset = (ex, ey) => {
//     const dx = mx - (gx + ex), dy = my - (gy + ey);
//     const d = Math.hypot(dx, dy) || 1;
//     const max = 4.2 * s;
//     return { x: (dx/d)*max, y: (dy/d)*max };
//   };
//   const offL = offset(eyeLpos.x, eyeLpos.y);
//   const offR = offset(eyeRpos.x, eyeRpos.y);

//   function glossyEye(c, off) {
//     // big circular sockets
//     ctx.fillStyle = '#fff';
//     ctx.beginPath(); ctx.arc(c.x, c.y, eyeR * 0.92, 0, TAU); ctx.fill();

//     // pupil/iris (solid dark)
//     if (eyeOpen >= 0.5) {
//       ctx.fillStyle = eyeDark;
//       ctx.beginPath(); ctx.arc(c.x + off.x, c.y + off.y, eyeR * 0.62, 0, TAU); ctx.fill();

//       // highlights
//       ctx.fillStyle = '#fff';
//       ctx.beginPath(); ctx.arc(c.x + off.x - 6*s, c.y + off.y - 10*s, 6*s, 0, TAU); ctx.fill();
//       ctx.beginPath(); ctx.arc(c.x + off.x + 10*s, c.y + off.y + 6*s, 3.2*s, 0, TAU); ctx.fill();
//     } else {
//       // blink
//       ctx.strokeStyle = eyeDark; ctx.lineWidth = Math.max(3, 3*s);
//       ctx.beginPath(); ctx.moveTo(c.x - eyeR*0.8, c.y); ctx.lineTo(c.x + eyeR*0.8, c.y); ctx.stroke();
//     }
//   }
//   glossyEye(eyeLpos, offL);
//   glossyEye(eyeRpos, offR);

//   // teeny nose
//   ctx.fillStyle = eyeDark;
//   ctx.beginPath(); ctx.arc(0, -16*s, 3.8*s, 0, TAU); ctx.fill();

//   // open mouth (tiny rounded triangle)
//   ctx.fillStyle = '#c8344e';
//   ctx.beginPath();
//   ctx.moveTo(-10*s, -2*s);
//   ctx.lineTo(10*s, -2*s);
//   ctx.quadraticCurveTo(0, 16*s, -10*s, -2*s);
//   ctx.closePath(); ctx.fill();

//   // tongue when pressed (peeks out a bit)
//   if (mouse.down && eyeOpen >= 0.6) {
//     ctx.fillStyle = '#ff8faa';
//     ctx.beginPath(); ctx.arc(0, 6*s, 7*s, 0, Math.PI); ctx.fill();
//   }

//   // soft cheek glow instead of blush dots
//   ctx.save(); ctx.globalCompositeOperation = 'lighter';
//   ctx.fillStyle = pinkSoft;
//   ctx.beginPath(); ctx.ellipse(-78*s, -2*s, 26*s, 18*s, 0.1, 0, TAU); ctx.fill();
//   ctx.beginPath(); ctx.ellipse( 78*s, -2*s, 26*s, 18*s, -0.1, 0, TAU); ctx.fill();
//   ctx.restore();

//   // ----- LITTLE PAWS -----
//   ctx.fillStyle = bodyMid;
//   function paw(px, py){ ctx.beginPath(); ctx.ellipse(px, py, 22*s, 16*s, 0.25, 0, TAU); ctx.fill(); }
//   paw(-48*s, 24*s);
//   paw( 48*s, 24*s);

//   ctx.restore();

//   // ground shadow
//   ctx.save();
//   const shadowY = y + 96*s;
//   ctx.globalAlpha = 0.22;
//   ctx.fillStyle = '#000';
//   ctx.beginPath(); ctx.ellipse(x, shadowY, 88*s, 22*s, 0, 0, TAU); ctx.fill();
//   ctx.restore();
// }

// // ---------------- Main loop ----------------
// let cx = document.documentElement.clientWidth  / 2;
// let cy = document.documentElement.clientHeight / 2;
// let scale = 1, targetScale = 1;
// const SCALE_MIN = 0.7, SCALE_MAX = 1.9;

// let last = performance.now();
// function frame(now) {
//   const dt = (now - last) / 1000; last = now;

//   // keyboard zoom / slide
//   if (keys.has('ArrowUp'))    targetScale = clamp(targetScale + 0.5 * dt, SCALE_MIN, SCALE_MAX);
//   if (keys.has('ArrowDown'))  targetScale = clamp(targetScale - 0.5 * dt, SCALE_MIN, SCALE_MAX);
//   if (keys.has('ArrowLeft'))  cx -= 200 * dt;
//   if (keys.has('ArrowRight')) cx += 200 * dt;
//   scale = lerp(scale, targetScale, 0.12);

//   // gentle follow to pointer (falls back to center)
//   const tx = mouse.x ?? document.documentElement.clientWidth  / 2;
//   const ty = mouse.y ?? document.documentElement.clientHeight / 2;
//   cx = lerp(cx, tx, 0.06);
//   cy = lerp(cy, ty, 0.04);

//   // draw
//   drawFluidBackground(now);
//   const bob = Math.sin(now * 0.006) * 8;
//   drawGhost(ctx, cx, cy + bob, now, scale);

//   requestAnimationFrame(frame);
// }
// requestAnimationFrame(frame);


// main.js
const canvas = document.getElementById('scene');
const ctx = canvas.getContext('2d', { alpha: false });

// --------- Pixel ratio + sizing ----------
let DPR = 1;
function fit() {
  DPR = Math.max(1, Math.min(2, window.devicePixelRatio || 1));
  const w = document.documentElement.clientWidth;
  const h = document.documentElement.clientHeight;
  canvas.style.width  = w + 'px';
  canvas.style.height = h + 'px';
  canvas.width  = Math.round(w * DPR);
  canvas.height = Math.round(h * DPR);
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.scale(DPR, DPR); // draw in CSS pixels
}
addEventListener('resize', fit, { passive: true });
fit();

// ---------------- Input ----------------
const keys = new Set();
addEventListener('keydown', e => keys.add(e.key));
addEventListener('keyup',   e => keys.delete(e.key));

const mouse = { x: null, y: null, down: false };
function mapPointer(clientX, clientY) { mouse.x = clientX; mouse.y = clientY; }
// init to center so eyes track immediately
mapPointer(document.documentElement.clientWidth / 2, document.documentElement.clientHeight / 2);

canvas.addEventListener('mousemove', e => mapPointer(e.clientX, e.clientY), { passive: true });
canvas.addEventListener('mouseleave', () => { mouse.x = null; mouse.y = null; }, { passive: true });
canvas.addEventListener('mousedown', () => { mouse.down = true; });
canvas.addEventListener('mouseup',   () => { mouse.down = false; });

canvas.addEventListener('touchstart', e => {
  e.preventDefault();
  const t = e.touches[0];
  mapPointer(t.clientX, t.clientY);
  mouse.down = true;
}, { passive: false });

canvas.addEventListener('touchmove', e => {
  e.preventDefault();
  const t = e.touches[0];
  mapPointer(t.clientX, t.clientY);
}, { passive: false });

canvas.addEventListener('touchend', () => { mouse.down = false; }, { passive: false });

// ---------------- Utils ----------------
const TAU = Math.PI * 2;
const clamp = (v,a,b) => Math.max(a, Math.min(b, v));
const lerp  = (a,b,t) => a + (b - a) * t;

// ---------------- Background ----------------
function drawFluidBackground(t) {
  const W = document.documentElement.clientWidth;
  const H = document.documentElement.clientHeight;

  // deep purple
  const base = ctx.createLinearGradient(0, 0, 0, H);
  base.addColorStop(0, '#2a1a49');
  base.addColorStop(1, '#3a225d');
  ctx.fillStyle = base;
  ctx.fillRect(0, 0, W, H);

  // subtle blooms
  for (let i = 0; i < 2; i++) {
    const hue = 285 + i * 20;
    const cx = (Math.sin(t * 0.0006 + i) * 0.35 + 0.5) * W;
    const cy = (Math.cos(t * 0.0005 + i * 1.7) * 0.35 + 0.55) * H;
    const g = ctx.createRadialGradient(cx, cy, 0, cx, cy, Math.max(W, H) * 0.9);
    g.addColorStop(0, `hsla(${hue}, 55%, 65%, 0.12)`);
    g.addColorStop(1, `hsla(${hue}, 55%, 30%, 0.00)`);
    ctx.fillStyle = g; ctx.fillRect(0, 0, W, H);
  }

  // vignette
  const vg = ctx.createRadialGradient(W/2, H/2, 0, W/2, H/2, Math.max(W, H));
  vg.addColorStop(0, 'rgba(0,0,0,0)');
  vg.addColorStop(1, 'rgba(0,0,0,0.55)');
  ctx.fillStyle = vg; ctx.fillRect(0, 0, W, H);
}

// ---------------- Wink timing ----------------
function doubleWinkOpenness(ms, closeScale = 1.3, scale = 1){
  if (scale < closeScale) return 1;
  const t = ms % 4000;
  const starts = [420, 960];
  const half = 160, total = half * 2;
  let open = 1;
  for (const s of starts){
    if (t >= s && t < s + total){
      const u = (t - s) / half;
      open = Math.min(open, (u <= 1) ? (1 - u) : (u - 1));
    }
  }
  return Math.max(0, Math.min(1, open));
}

// ---------------- Ground shadow (draw BEFORE ghost) ----------------
function drawGroundShadow(x, y, t, scale) {
  const wobble = 1 + Math.sin(t * 0.003) * 0.02;
  const s = scale * wobble;

  ctx.save();
  const shadowY = y + 96 * s;
  const rx = 92 * s;   // radius x
  const ry = 22 * s;   // radius y

  // slightly stronger center with soft falloff
  const grad = ctx.createRadialGradient(x, shadowY, 2, x, shadowY, rx);
  grad.addColorStop(0, 'rgba(0,0,0,0.35)');
  grad.addColorStop(1, 'rgba(0,0,0,0.00)');

  ctx.fillStyle = grad;
  ctx.beginPath();
  ctx.ellipse(x, shadowY, rx, ry, 0, 0, TAU);
  ctx.fill();
  ctx.restore();
}

// ---------------- Ghost ----------------
function drawGhost(ctx, x, y, t, scale = 1) {
  // palette
  const bodyHi   = '#ffffff';
  const bodyMid  = '#f6f2ff';
  const bodyLow  = '#eae6ff';
  const eyeDark  = '#1b1b25';
  const pinkGlow = 'rgba(255,120,170,0.75)';
  const pinkSoft = 'rgba(255,120,170,0.25)';

  const wobble = 1 + Math.sin(t * 0.003) * 0.02;
  const s = scale * wobble;

  ctx.save();
  ctx.translate(x, y);

  // ----- BODY with smoother tapered curl tail -----
  const W = 220 * s, H = 260 * s;

  ctx.beginPath();
  // round head top
  ctx.moveTo(-W*0.46, -H*0.05);
  ctx.bezierCurveTo(-W*0.46, -H*0.95,  W*0.46, -H*0.95,  W*0.46, -H*0.05);

  // right belly bulge
  ctx.bezierCurveTo(W*0.56,  H*0.06,   W*0.38,  H*0.34,   W*0.12,  H*0.52);

  // sweep to underside then into a left curl tip (thin, pointy)
  ctx.bezierCurveTo(-W*0.02, H*0.66,  -W*0.04, H*0.76,  -W*0.14, H*0.84);
  ctx.bezierCurveTo(-W*0.30, H*0.98,  -W*0.22, H*1.02,  -W*0.06, H*0.92); // tail tip pointing left

  // curve back toward body to close S-shape
  ctx.bezierCurveTo(W*0.02,  H*0.84,  -W*0.10, H*0.62,  -W*0.20, H*0.48);
  ctx.bezierCurveTo(-W*0.48, H*0.12,  -W*0.52,-H*0.08,  -W*0.46,-H*0.05);
  ctx.closePath();

  // creamy fill
  const body = ctx.createRadialGradient(-W*0.1, -H*0.7, 10*s, 0, 0, Math.max(W,H));
  body.addColorStop(0.00, bodyHi);
  body.addColorStop(0.55, bodyMid);
  body.addColorStop(1.00, bodyLow);
  ctx.fillStyle = body; ctx.fill();

  // subtle rim
  const rim = ctx.createLinearGradient(-W, -H, W, H);
  rim.addColorStop(0, 'rgba(160,175,235,0.28)');
  rim.addColorStop(1, 'rgba(120,140,220,0.10)');
  ctx.strokeStyle = rim; ctx.lineWidth = Math.max(2, 2*s); ctx.stroke();

  // bottom pink rim-light
  const rl = ctx.createRadialGradient(0, 84*s, 4*s, 0, 84*s, 160*s);
  rl.addColorStop(0, pinkGlow);
  rl.addColorStop(1, 'rgba(255,120,170,0)');
  ctx.globalCompositeOperation = 'lighter';
  ctx.fillStyle = rl; ctx.fill();
  ctx.globalCompositeOperation = 'source-over';

  // ----- EARS (small pointy triangles with soft corners) -----
  function ear(ax, ay, flip=1){
    ctx.save(); ctx.translate(ax, ay); ctx.scale(flip, 1);

    // outer ear
    ctx.beginPath();
    ctx.moveTo(0, 0);
    ctx.quadraticCurveTo(22*s, -32*s,  34*s, -6*s);
    ctx.quadraticCurveTo(18*s, 10*s,   0, 10*s);
    ctx.closePath();
    ctx.fillStyle = bodyMid; ctx.fill();

    // inner ear tint
    ctx.beginPath();
    ctx.moveTo(6*s, 0);
    ctx.quadraticCurveTo(18*s, -22*s,  26*s, -6*s);
    ctx.quadraticCurveTo(14*s, 6*s,    6*s, 6*s);
    ctx.closePath();
    ctx.fillStyle = 'rgba(255,140,170,0.25)';
    ctx.fill();

    ctx.restore();
  }
  ear(-72*s, -118*s, -1);
  ear( 72*s, -112*s,  1);

  // ----- FACE -----
  const eyeOpen = doubleWinkOpenness(t, 1.3, scale);
  const eyeR = 30 * s;
  const eyeLpos = { x: -52*s, y: -38*s };
  const eyeRpos = { x:  52*s, y: -38*s };

  // pupil follow
  const gx = x, gy = y;
  const mx = mouse.x ?? gx, my = mouse.y ?? gy;
  const offset = (ex, ey) => {
    const dx = mx - (gx + ex), dy = my - (gy + ey);
    const d = Math.hypot(dx, dy) || 1;
    const max = 4.2 * s;
    return { x: (dx/d)*max, y: (dy/d)*max };
  };
  const offL = offset(eyeLpos.x, eyeLpos.y);
  const offR = offset(eyeRpos.x, eyeRpos.y);

  function glossyEye(c, off) {
    ctx.fillStyle = '#fff';
    ctx.beginPath(); ctx.arc(c.x, c.y, eyeR * 0.92, 0, TAU); ctx.fill();

    if (eyeOpen >= 0.5) {
      ctx.fillStyle = eyeDark;
      ctx.beginPath(); ctx.arc(c.x + off.x, c.y + off.y, eyeR * 0.62, 0, TAU); ctx.fill();

      // highlights
      ctx.fillStyle = '#fff';
      ctx.beginPath(); ctx.arc(c.x + off.x - 6*s, c.y + off.y - 10*s, 6*s, 0, TAU); ctx.fill();
      ctx.beginPath(); ctx.arc(c.x + off.x + 10*s, c.y + off.y + 6*s, 3.2*s, 0, TAU); ctx.fill();
    } else {
      ctx.strokeStyle = eyeDark; ctx.lineWidth = Math.max(3, 3*s);
      ctx.beginPath(); ctx.moveTo(c.x - eyeR*0.8, c.y); ctx.lineTo(c.x + eyeR*0.8, c.y); ctx.stroke();
    }
  }
  glossyEye(eyeLpos, offL);
  glossyEye(eyeRpos, offR);

  // tiny nose
  ctx.fillStyle = eyeDark;
  ctx.beginPath(); ctx.arc(0, -16*s, 3.8*s, 0, TAU); ctx.fill();

  // small open mouth
  ctx.fillStyle = '#c8344e';
  ctx.beginPath();
  ctx.moveTo(-10*s, -2*s);
  ctx.lineTo(10*s, -2*s);
  ctx.quadraticCurveTo(0, 16*s, -10*s, -2*s);
  ctx.closePath(); ctx.fill();

  // tongue on press
  if (mouse.down && eyeOpen >= 0.6) {
    ctx.fillStyle = '#ff8faa';
    ctx.beginPath(); ctx.arc(0, 6*s, 7*s, 0, Math.PI); ctx.fill();
  }

  // soft cheek glow
  ctx.save(); ctx.globalCompositeOperation = 'lighter';
  ctx.fillStyle = pinkSoft;
  ctx.beginPath(); ctx.ellipse(-78*s, -2*s, 26*s, 18*s, 0.1, 0, TAU); ctx.fill();
  ctx.beginPath(); ctx.ellipse( 78*s, -2*s, 26*s, 18*s, -0.1, 0, TAU); ctx.fill();
  ctx.restore();

  // little paws
  ctx.fillStyle = bodyMid;
  function paw(px, py){ ctx.beginPath(); ctx.ellipse(px, py, 22*s, 16*s, 0.25, 0, TAU); ctx.fill(); }
  paw(-48*s, 24*s);
  paw( 48*s, 24*s);

  ctx.restore();
}

// ---------------- Main loop ----------------
let cx = document.documentElement.clientWidth  / 2;
let cy = document.documentElement.clientHeight / 2;
let scale = 1, targetScale = 1;
const SCALE_MIN = 0.7, SCALE_MAX = 1.9;

let last = performance.now();
function frame(now) {
  const dt = (now - last) / 1000; last = now;

  // keyboard zoom / slide
  if (keys.has('ArrowUp'))    targetScale = clamp(targetScale + 0.5 * dt, SCALE_MIN, SCALE_MAX);
  if (keys.has('ArrowDown'))  targetScale = clamp(targetScale - 0.5 * dt, SCALE_MIN, SCALE_MAX);
  if (keys.has('ArrowLeft'))  cx -= 200 * dt;
  if (keys.has('ArrowRight')) cx += 200 * dt;
  scale = lerp(scale, targetScale, 0.12);

  // gentle follow to pointer
  const tx = mouse.x ?? document.documentElement.clientWidth  / 2;
  const ty = mouse.y ?? document.documentElement.clientHeight / 2;
  cx = lerp(cx, tx, 0.06);
  cy = lerp(cy, ty, 0.04);

  // draw
  drawFluidBackground(now);
  const bob = Math.sin(now * 0.006) * 8;

  // shadow goes UNDER the body
  drawGroundShadow(cx, cy + bob, now, scale);

  // body
  drawGhost(ctx, cx, cy + bob, now, scale);

  requestAnimationFrame(frame);
}
requestAnimationFrame(frame);


```
