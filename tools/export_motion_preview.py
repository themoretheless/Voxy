#!/usr/bin/env python3
"""Create a local frame-by-frame preview from body_motion_snapshot receipts."""
import argparse
import csv
import json
import math
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("frames", type=Path)
parser.add_argument("output", type=Path)
args = parser.parse_args()
frames = sorted(args.frames.glob("frame-*.png"))
if not frames:
    parser.error("no rendered frames")
with (args.frames / "secondary-motion.csv").open() as source:
    rows = list(csv.DictReader(source))
samples = {}
for row in rows:
    time = float(row["time_s"])
    offset = [float(row[f"offset_{axis}_m"]) for axis in "xyz"]
    if not all(math.isfinite(value) for value in [time, *offset]):
        parser.error("nonfinite motion receipt")
    samples.setdefault(time, []).append({"sample": int(row["sample"]), "mm": 1000 * math.sqrt(sum(value * value for value in offset))})
times = sorted(samples)
if len(times) != len(frames) or any(b <= a for a, b in zip(times, times[1:])):
    parser.error("frame and receipt timeline mismatch")
args.output.parent.mkdir(parents=True, exist_ok=True)
import os
data = [{"time": time, "image": os.path.relpath(frame.resolve(), args.output.parent.resolve()), "samples": samples[time]} for time, frame in zip(times, frames)]
maximum = max(sample["mm"] for frame in data for sample in frame["samples"]) or 1
duration = times[-1] - times[0] or 1
colors = ["#7dd3fc", "#c4b5fd", "#fcd34d", "#86efac"]
paths = []
for index, sample_id in enumerate(sorted({sample["sample"] for frame in data for sample in frame["samples"]})):
    points = [(40 + 720 * (frame["time"] - times[0]) / duration, 160 - 130 * sample["mm"] / maximum) for frame in data for sample in frame["samples"] if sample["sample"] == sample_id]
    path = " ".join(f"{'M' if i == 0 else 'L'}{x:.2f},{y:.2f}" for i, (x, y) in enumerate(points))
    paths.append(f'<path d="{path}" fill="none" stroke="{colors[index % len(colors)]}" stroke-width="2"><title>Область {sample_id + 1}</title></path>')
chart = '<svg viewBox="0 0 800 200" role="img" aria-label="Отклонение свободных центров тканей от жёсткого движения костей во времени" style="width:100%">' + f'<text x="40" y="20" fill="white" font-size="14">Отклонение, мм · максимум {maximum:.2f}</text>' + ''.join(paths) + '<line id="cursor" x1="40" x2="40" y1="28" y2="160" stroke="white"/>' + f'<text x="40" y="185" fill="white" font-size="14">{times[0]:.2f} с</text><text x="720" y="185" fill="white" font-size="14">{times[-1]:.2f} с</text></svg>'
html = r'''<!doctype html><html lang="ru"><meta charset="utf-8">
<title>Скелет и мягкие ткани</title>
<style>body{background:#101923;color:#eee;font:18px system-ui;max-width:900px;margin:24px auto;padding:16px}img{width:min(100%,640px);display:block}button,select,input{font:inherit;margin:8px}input{width:65%}pre{font:16px monospace}</style>
<h1>Скелет и мягкие ткани</h1><p>Нейтральная техническая демонстрация. Отдельные объёмы тканей; модель кожи ещё не связана с их деформацией.</p>
<img id="frame" alt="Кадр скелетной анимации"><button id="play">Пауза</button>
<label>Скорость <select id="speed"><option value="0.25">¼×</option><option value="0.5">½×</option><option selected value="1">1×</option></select></label>
<input id="seek" type="range" min="0" step="1" aria-label="Кадр"><pre id="receipt"></pre>
CHART
<script>
const frames=DATA;
const picture=document.getElementById('frame'),seek=document.getElementById('seek'),play=document.getElementById('play'),speed=document.getElementById('speed'),receipt=document.getElementById('receipt');
seek.max=frames.length-1;
let index=0,playing=true,previous=null,elapsed=0;
function show(){const f=frames[index];picture.src=f.image;seek.value=index;receipt.textContent=`Время ${f.time.toFixed(2)} с · кадр ${index+1}/${frames.length}\nОтклонение тканей от костей:\n`+f.samples.map(s=>`Область ${s.sample+1}: ${s.mm.toFixed(3)} мм`).join('\n');const x=40+720*(f.time-frames[0].time)/(frames.at(-1).time-frames[0].time||1);document.getElementById('cursor').setAttribute('x1',x);document.getElementById('cursor').setAttribute('x2',x);}
play.onclick=()=>{playing=!playing;play.textContent=playing?'Пауза':'Играть';elapsed=0;};
seek.oninput=()=>{index=Number(seek.value);elapsed=0;show();};
function tick(now){if(previous!==null&&playing){elapsed+=(now-previous)/1000*Number(speed.value);let interval=index<frames.length-1?frames[index+1].time-frames[index].time:0.5;while(elapsed>=interval){elapsed-=interval;index=(index+1)%frames.length;show();interval=index<frames.length-1?frames[index+1].time-frames[index].time:0.5;}}previous=now;requestAnimationFrame(tick);}
show();requestAnimationFrame(tick);
</script></html>'''.replace('CHART', chart).replace('DATA', json.dumps(data, ensure_ascii=False).replace('</', '<\\/'))
args.output.write_text(html)
print(f"{args.output}: {len(frames)} frames, {len(rows)} finite motion receipts")
