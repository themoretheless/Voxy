# Facial rendering source scan: fatihyaylacimk/Realistic-Serpent-Mouse-Follow-Simulation

Repository: https://github.com/fatihyaylacimk/Realistic-Serpent-Mouse-Follow-Simulation

Pinned source revision: c6cc54b3923d9e3ba80092a2d2074e0d4759bc83

Discovery topic: tongue animation

Description: Realistic serpent mouse-follow simulation in JavaScript with organic snake movement, shaped head, eyes, forked tongue, patterned body, scale details, ground texture and Canvas animation.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: Realistic_Serpent_Mouse_Follow/script.js

{
  "path": "Realistic_Serpent_Mouse_Follow/script.js",
  "blob_sha": "be1885eff8b69b9ce8e007c3502fe92db846f47a",
  "sha256": "2c1aeca8681b58b86743878867e531f8249148ef8b261696cda8ca930625c2f0",
  "lines": 590,
  "bytes": 13962,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 8,
    "jaw": 5,
    "tongue": 3,
    "rig": 4,
    "lip": 5
  },
  "url": "https://github.com/fatihyaylacimk/Realistic-Serpent-Mouse-Follow-Simulation/blob/c6cc54b3923d9e3ba80092a2d2074e0d4759bc83/Realistic_Serpent_Mouse_Follow/script.js",
  "function_signature_candidates": []
}

```text

const canvas = document.getElementById("canvas");
const ctx = canvas.getContext("2d");

let width;
let height;
let dpr;
let time = 0;

const mouse = {
  x: 0,
  y: 0,
  active: false
};

const head = {
  x: 0,
  y: 0,
  angle: 0,
  speed: 4.6
};

const body = [];
const grass = [];
const dust = [];

const settings = {
  segments: 96,
  spacing: 7.8,
  maxTurn: 0.065
};

function resize() {
  dpr = Math.max(1, Math.min(window.devicePixelRatio || 1, 2));
  width = window.innerWidth;
  height = window.innerHeight;

  canvas.width = Math.floor(width * dpr);
  canvas.height = Math.floor(height * dpr);
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

  initScene();
}

function initScene() {
  head.x = width * 0.5;
  head.y = height * 0.5;
  head.angle = -0.2;

  body.length = 0;

  for (let i = 0; i < settings.segments; i++) {
    const t = i / (settings.segments - 1);
    const radius = getBodyRadius(t);

    body.push({
      x: head.x - Math.cos(head.angle) * settings.spacing * i,
      y: head.y - Math.sin(head.angle) * settings.spacing * i,
      angle: head.angle,
      radius,
      phase: Math.random() * Math.PI * 2
    });
  }

  grass.length = 0;
  const count = Math.floor((width * height) / 18000);

  for (let i = 0; i < count; i++) {
    grass.push({
      x: Math.random() * width,
      y: Math.random() * height,
      len: Math.random() * 28 + 12,
      angle: -Math.PI / 2 + (Math.random() - 0.5) * 0.7,
      alpha: Math.random() * 0.15 + 0.04
    });
  }
}

function getBodyRadius(t) {
  // thick middle, tapered tail, slightly wide neck/head area
  const taperTail = Math.pow(1 - t, 0.52);
  const midBulge = Math.sin((1 - t) * Math.PI) * 5.5;
  return 4.2 + taperTail * 11.5 + midBulge;
}

function drawBackground() {
  ctx.fillStyle = "rgba(6, 9, 8, 0.34)";
  ctx.fillRect(0, 0, width, height);

  const glow = ctx.createRadialGradient(head.x, head.y, 0, head.x, head.y, Math.min(width, height) * 0.58);
  glow.addColorStop(0, "rgba(95,130,80,0.16)");
  glow.addColorStop(0.45, "rgba(42,85,54,0.08)");
  glow.addColorStop(1, "rgba(0,0,0,0)");
  ctx.fillStyle = glow;
  ctx.fillRect(0, 0, width, height);

  drawGroundTexture();
}

function drawGroundTexture() {
  ctx.save();

  for (const g of grass) {
    const sway = Math.sin(time * 1.4 + g.x * 0.01) * 0.12;
    ctx.beginPath();
    ctx.moveTo(g.x, g.y);
    ctx.lineTo(
      g.x + Math.cos(g.angle + sway) * g.len,
      g.y + Math.sin(g.angle + sway) * g.len
    );
    ctx.lineWidth = 1;
    ctx.strokeStyle = `rgba(122, 160, 112, ${g.alpha})`;
    ctx.stroke();
  }

  ctx.restore();
}

function updateHead() {
  let targetX;
  let targetY;

  if (mouse.active) {
    targetX = mouse.x;
    targetY = mouse.y;
  } else {
    targetX = width / 2 + Math.cos(time * 0.42) * width * 0.35 + Math.sin(time * 1.1) * 90;
    targetY = height / 2 + Math.sin(time * 0.52) * height * 0.28 + Math.cos(time * 0.9) * 70;
  }

  const dx = targetX - head.x;
  const dy = targetY - head.y;
  const targetAngle = Math.atan2(dy, dx);

  let diff = targetAngle - head.angle;
  while (diff > Math.PI) diff -= Math.PI * 2;
  while (diff < -Math.PI) diff += Math.PI * 2;

  const distance = Math.sqrt(dx * dx + dy * dy);
  const speedFactor = Math.max(0.28, Math.min(1.15, distance / 230));

  head.angle += Math.max(-settings.maxTurn, Math.min(settings.maxTurn, diff * 0.08));
  head.speed = 2.4 + speedFactor * 3.2;

  const slither = Math.sin(time * 7) * 0.075;
  head.x += Math.cos(head.angle + slither) * head.speed;
  head.y += Math.sin(head.angle + slither) * head.speed;

  wrapHead();

  body[0].x = head.x;
  body[0].y = head.y;
  body[0].angle = head.angle;
}

function wrapHead() {
  const pad = 40;

  if (head.x < -pad) head.x = width + pad;
  if (head.x > width + pad) head.x = -pad;
  if (head.y < -pad) head.y = height + pad;
  if (head.y > height + pad) head.y = -pad;
}

function updateBody() {
  for (let i = 1; i < body.length; i++) {
    const prev = body[i - 1];
    const cur = body[i];

    const dx = prev.x - cur.x;
    const dy = prev.y - cur.y;
    const dist = Math.sqrt(dx * dx + dy * dy) || 1;
    const angle = Math.atan2(dy, dx);

    const desiredX = prev.x - Math.cos(angle) * settings.spacing;
    const desiredY = prev.y - Math.sin(angle) * settings.spacing;

    cur.x += (desiredX - cur.x) * 0.55;
    cur.y += (desiredY - cur.y) * 0.55;
    cur.angle = angle;

    const t = i / body.length;
    const lateral = Math.sin(time * 8.0 - i * 0.34) * (1 - t) * 1.65;
    cur.x += Math.cos(angle + Math.PI / 2) * lateral;
    cur.y += Math.sin(angle + Math.PI / 2) * lateral;
  }

  if (Math.random() > 0.62) {
    const tail = body[Math.floor(body.length * 0.62)];
    dust.push({
      x: tail.x + (Math.random() - 0.5) * 18,
      y: tail.y + (Math.random() - 0.5) * 18,
      vx: (Math.random() - 0.5) * 0.5,
      vy: (Math.random() - 0.5) * 0.5,
      life: 45,
      r: Math.random() * 5 + 2
    });
  }
}

function updateDust() {
  for (const d of dust) {
    d.x += d.vx;
    d.y += d.vy;
    d.life--;
    d.r *= 1.015;
  }

  for (let i = dust.length - 1; i >= 0; i--) {
    if (dust[i].life <= 0) dust.splice(i, 1);
  }
}

function drawDust() {
  ctx.save();

  for (const d of dust) {
    const alpha = Math.max(0, d.life / 45) * 0.08;
    ctx.beginPath();
    ctx.arc(d.x, d.y, d.r, 0, Math.PI * 2);
    ctx.fillStyle = `rgba(170, 150, 100, ${alpha})`;
    ctx.fill();
  }

  ctx.restore();
}

function getSidePoint(segment, side, scale = 1) {
  return {
    x: segment.x + Math.cos(segment.angle + Math.PI / 2 * side) * segment.radius * scale,
    y: segment.y + Math.sin(segment.angle + Math.PI / 2 * side) * segment.radius * scale
  };
}

function drawBodyShadow() {
  ctx.save();

  ctx.beginPath();

  for (let i = 0; i < body.length; i++) {
    const p = getSidePoint(body[i], 1, 1.15);
    if (i === 0) ctx.moveTo(p.x + 10, p.y + 14);
    else ctx.lineTo(p.x + 10, p.y + 14);
  }

  for (let i = body.length - 1; i >= 0; i--) {
    const p = getSidePoint(body[i], -1, 1.15);
    ctx.lineTo(p.x + 10, p.y + 14);
  }

  ctx.closePath();
  ctx.fillStyle = "rgba(0,0,0,0.28)";
  ctx.filter = "blur(5px)";
  ctx.fill();
  ctx.filter = "none";

  ctx.restore();
}

function drawBodyShape() {
  ctx.save();

  // body outline
  ctx.beginPath();

  for (let i = 0; i < body.length; i++) {
    const t = i / (body.length - 1);
    const scale = 1 + Math.sin(time * 5.0 - i * 0.24) * 0.055 * (1 - t);
    const p = getSidePoint(body[i], 1, scale);
    if (i === 0) ctx.moveTo(p.x, p.y);
    else ctx.lineTo(p.x, p.y);
  }

  for (let i = body.length - 1; i >= 0; i--) {
    const t = i / (body.length - 1);
    const scale = 1 + Math.sin(time * 5.0 - i * 0.24) * 0.055 * (1 - t);
    const p = getSidePoint(body[i], -1, scale);
    ctx.lineTo(p.x, p.y);
  }

  ctx.closePath();

  const gradient = ctx.createLinearGradient(head.x - 120, head.y - 80, head.x + 180, head.y + 120);
  gradient.addColorStop(0, "#284123");
  gradient.addColorStop(0.38, "#5f7e3c");
  gradient.addColorStop(0.68, "#263a20");
  gradient.addColorStop(1, "#0d1710");

  ctx.fillStyle = gradient;
  ctx.shadowBlur = 18;
  ctx.shadowColor = "rgba(45,90,46,0.55)";
  ctx.fill();

  ctx.lineWidth = 1.2;
  ctx.strokeStyle = "rgba(210,230,170,0.18)";
  ctx.stroke();

  drawBellyLine();
  drawScales();
  drawPattern();
  ctx.restore();
}

function drawBellyLine() {
  ctx.save();

  ctx.beginPath();
  for (let i = 5; i < body.length - 8; i++) {
    const s = body[i];
    const offset = Math.sin(time * 6 - i * 0.32) * 0.6;
    const x = s.x + Math.cos(s.angle + Math.PI / 2) * offset;
    const y = s.y + Math.sin(s.angle + Math.PI / 2) * offset;

    if (i === 5) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  }

  ctx.lineWidth = 2.2;
  ctx.strokeStyle = "rgba(230, 235, 170, 0.20)";
  ctx.stroke();

  ctx.restore();
}

function drawScales() {
  ctx.save();

  for (let i = 8; i < body.length - 5; i += 3) {
    const s = body[i];
    const t = i / body.length;
    const scaleAlpha = (1 - t) * 0.28;

    for (const side of [-1, 1]) {
      const p = getSidePoint(s, side, 0.45);
      const p2 = getSidePoint(s, side, 0.92);

      ctx.beginPath();
      ctx.moveTo(p.x, p.y);
      ctx.lineTo(p2.x, p2.y);
      ctx.lineWidth = Math.max(0.4, 1.5 * (1 - t));
      ctx.strokeStyle = `rgba(235,245,190,${scaleAlpha})`;
      ctx.stroke();
    }
  }

  ctx.restore();
}

function drawPattern() {
  ctx.save();

  for (let i = 7; i < body.length - 8; i += 5) {
    const s = body[i];
    const t = i / body.length;
    const size = s.radius * (0.72 - t * 0.28);

    const left = getSidePoint(s, 1, 0.45);
    const right = getSidePoint(s, -1, 0.45);

    ctx.beginPath();
    ctx.ellipse(left.x, left.y, size * 0.55, size * 0.28, s.angle + 0.55, 0, Math.PI * 2);
    ctx.fillStyle = `rgba(20, 35, 16, ${0.42 * (1 - t)})`;
    ctx.fill();

    ctx.beginPath();
    ctx.ellipse(right.x, right.y, size * 0.55, size * 0.28, s.angle - 0.55, 0, Math.PI * 2);
    ctx.fillStyle = `rgba(20, 35, 16, ${0.42 * (1 - t)})`;
    ctx.fill();

    if (i % 10 === 0) {
      ctx.beginPath();
      ctx.ellipse(s.x, s.y, size * 0.5, size * 0.22, s.angle, 0, Math.PI * 2);
      ctx.fillStyle = `rgba(200, 190, 98, ${0.16 * (1 - t)})`;
      ctx.fill();
    }
  }

  ctx.restore();
}

function drawHead() {
  const h = body[0];
  const a = h.angle;

  const nose = {
    x: h.x + Math.cos(a) * 26,
    y: h.y + Math.sin(a) * 26
  };

  const leftJaw = {
    x: h.x + Math.cos(a + 2.36) * 24,
    y: h.y + Math.sin(a + 2.36) * 24
  };

  const rightJaw = {
    x: h.x + Math.cos(a - 2.36) * 24,
    y: h.y + Math.sin(a - 2.36) * 24
  };

  const neck = {
    x: h.x - Math.cos(a) * 13,
    y: h.y - Math.sin(a) * 13
  };

  ctx.save();

  ctx.beginPath();
  ctx.moveTo(nose.x, nose.y);
  ctx.quadraticCurveTo(
    h.x + Math.cos(a + Math.PI / 2) * 24,
    h.y + Math.sin(a + Math.PI / 2) * 24,
    leftJaw.x,
    leftJaw.y
  );
  ctx.quadraticCurveTo(neck.x, neck.y, rightJaw.x, rightJaw.y);
  ctx.quadraticCurveTo(
    h.x + Math.cos(a - Math.PI / 2) * 24,
    h.y + Math.sin(a - Math.PI / 2) * 24,
    nose.x,
    nose.y
  );
  ctx.closePath();

  const hg = ctx.createLinearGradient(h.x - 30, h.y - 30, h.x + 35, h.y + 35);
  hg.addColorStop(0, "#74964b");
  hg.addColorStop(0.55, "#2f4a27");
  hg.addColorStop(1, "#132018");

  ctx.fillStyle = hg;
  ctx.shadowBlur = 20;
  ctx.shadowColor = "rgba(65,120,60,0.52)";
  ctx.fill();

  ctx.lineWidth = 1.2;
  ctx.strokeStyle = "rgba(230,240,190,0.23)";
  ctx.stroke();

  drawHeadDetails(h, a);
  drawTongue(h, a);

  ctx.restore();
}

function drawHeadDetails(h, a) {
  const eyeForward = 9;
  const eyeSide = 12;

  const eyes = [
    {
      x: h.x + Math.cos(a) * eyeForward + Math.cos(a + Math.PI / 2) * eyeSide,
      y: h.y + Math.sin(a) * eyeForward + Math.sin(a + Math.PI / 2) * eyeSide
    },
    {
      x: h.x + Math.cos(a) * eyeForward + Math.cos(a - Math.PI / 2) * eyeSide,
      y: h.y + Math.sin(a) * eyeForward + Math.sin(a - Math.PI / 2) * eyeSide
    }
  ];

  for (const e of eyes) {
    ctx.beginPath();
    ctx.ellipse(e.x, e.y, 4.8, 3.2, a, 0, Math.PI * 2);
    ctx.fillStyle = "#e4d56a";
    ctx.shadowBlur = 10;
    ctx.shadowColor = "rgba(230,210,90,0.65)";
    ctx.fill();

    ctx.beginPath();
    ctx.ellipse(e.x + Math.cos(a) * 0.4, e.y + Math.sin(a) * 0.4, 1.2, 3.0, a, 0, Math.PI * 2);
    ctx.fillStyle = "#050505";
    ctx.shadowBlur = 0;
    ctx.fill();
  }

  // nose nostrils
  for (const side of [-1, 1]) {
    const nx = h.x + Math.cos(a) * 22 + Math.cos(a + Math.PI / 2 * side) * 4.2;
    const ny = h.y + Math.sin(a) * 22 + Math.sin(a + Math.PI / 2 * side) * 4.2;

    ctx.beginPath();
    ctx.arc(nx, ny, 1.7, 0, Math.PI * 2);
    ctx.fillStyle = "rgba(0,0,0,0.55)";
    ctx.fill();
  }
}

function drawTongue(h, a) {
  const flick = Math.sin(time * 12) * 4;

  const start = {
    x: h.x + Math.cos(a) * 27,
    y: h.y + Math.sin(a) * 27
  };

  const mid = {
    x: h.x + Math.cos(a) * (39 + flick),
    y: h.y + Math.sin(a) * (39 + flick)
  };

  const end = {
    x: h.x + Math.cos(a) * (52 + flick),
    y: h.y + Math.sin(a) * (52 + flick)
  };

  ctx.beginPath();
  ctx.moveTo(start.x, start.y);
  ctx.quadraticCurveTo(
    mid.x + Math.cos(a + Math.PI / 2) * Math.sin(time * 18) * 2.5,
    mid.y + Math.sin(a + Math.PI / 2) * Math.sin(time * 18) * 2.5,
    end.x,
    end.y
  );
  ctx.lineWidth = 2.2;
  ctx.strokeStyle = "rgba(180, 42, 64, 0.88)";
  ctx.shadowBlur = 8;
  ctx.shadowColor = "rgba(220, 45, 70, 0.6)";
  ctx.stroke();

  // forked tongue
  for (const side of [-1, 1]) {
    ctx.beginPath();
    ctx.moveTo(end.x, end.y);
    ctx.lineTo(
      end.x + Math.cos(a + side * 0.65) * 11,
      end.y + Math.sin(a + side * 0.65) * 11
    );
    ctx.lineWidth = 1.4;
    ctx.stroke();
  }
}

function drawMouseTarget() {
  if (!mouse.active) return;

  ctx.save();
  ctx.beginPath();
  ctx.arc(mouse.x, mouse.y, 8 + Math.sin(time * 5) * 2, 0, Math.PI * 2);
  ctx.strokeStyle = "rgba(190,230,170,0.32)";
  ctx.lineWidth = 1;
  ctx.stroke();

  ctx.beginPath();
  ctx.arc(mouse.x, mouse.y, 28, 0, Math.PI * 2);
  ctx.strokeStyle = "rgba(190,230,170,0.09)";
  ctx.stroke();
  ctx.restore();
}

function animate() {
  time += 0.016;

  updateHead();
  updateBody();
  updateDust();

  drawBackground();
  drawDust();
  drawBodyShadow();
  drawBodyShape();
  drawHead();
  drawMouseTarget();

  requestAnimationFrame(animate);
}

window.addEventListener("resize", resize);

window.addEventListener("mousemove", (e) => {
  mouse.x = e.clientX;
  mouse.y = e.clientY;
  mouse.active = true;
});

window.addEventListener("mouseleave", () => {
  mouse.active = false;
});

window.addEventListener("touchmove", (e) => {
  const t = e.touches[0];
  mouse.x = t.clientX;
  mouse.y = t.clientY;
  mouse.active = true;
}, { passive: true });

window.addEventListener("touchend", () => {
  mouse.active = false;
});

resize();
ctx.fillStyle = "#050707";
ctx.fillRect(0, 0, width, height);
animate();


```
