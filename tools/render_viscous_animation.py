"""Render actual liquid_viscous_animation CSV states to a neutral fluid GIF."""
import argparse
import csv
import math
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont
from liquid_surface_renderer import render_surfaces

parser = argparse.ArgumentParser()
parser.add_argument("states", type=Path)
parser.add_argument("output", type=Path)
args = parser.parse_args()
states = {}
with args.states.open() as source:
    for row in csv.reader(source):
        frame = int(row[1])
        state = states.setdefault(frame, {"particles": [], "film": [], "surface": []})
        numbers = list(map(float, row[2:]))
        if row[0] == "S":
            state["summary"] = numbers
        elif row[0] == "P":
            state["particles"].append(numbers)
        elif row[0] == "F":
            state["film"].append(numbers)
        elif row[0] == "T":
            state["surface"].append([numbers[:3],numbers[3:6],numbers[6:9]])
        elif row[0] == "V":
            if abs(numbers[1]-numbers[0]) > .02001*numbers[0]:
                raise RuntimeError("surface mesh volume mismatch")
        elif row[0] == "R":
            if any(not math.isfinite(v) or v < 0 for v in numbers):
                raise RuntimeError("invalid polymer energy ledger")
            state["polymer"] = numbers
if len(states) != 91 or any("summary" not in s for s in states.values()):
    raise RuntimeError("incomplete simulation export; expected 91 complete frames")
if max(s["summary"][-1] for s in states.values()) > 1e-12:
    raise RuntimeError("component mass conservation failed")
if states[90]["summary"][2] == 0 or states[90]["summary"][3] == 0:
    raise RuntimeError("no deposited fluid or spray in simulation")

W, H, AA = 960, 600, 2
def font(size):
    return ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial.ttf", size * AA)
title, normal, small = font(27), font(18), font(15)
def project(x, y, z):
    return ((W * .5 + 6800 * (x + .48 * z)) * AA,
            (H * .76 - 6800 * (y + .30 * z)) * AA)
def text(draw, at, label, f=normal, color=(200, 210, 222)):
    draw.text((at[0] * AA, at[1] * AA), label, fill=color, font=f)
def color(gel, shade=1.0):
    return tuple(int(v * shade) for v in (225 + 25 * gel, 232 + 15 * gel, 238 - 5 * gel))

images = []
for frame in sorted(states):
    state = states[frame]
    time, mass, captured, fragments, error = state["summary"]
    image = Image.new("RGB", (W * AA, H * AA), (13, 20, 30))
    draw = ImageDraw.Draw(image)
    text(draw, (34, 26), "Вязкая жидкость · неоднородная смесь", title, (240, 244, 250))
    subtitle = "Вязкоупругая струя · капли · удары о мокрую плёнку" if "polymer" in state else "Связная поверхность струи, капель и плёнки"
    text(draw, (36, 64), subtitle, normal, (150, 171, 191))
    corners = [project(x, 0, z) for x, z in [(-.05,-.025),(.05,-.025),(.05,.025),(-.05,.025)]]
    draw.polygon(corners, fill=(29, 43, 57))
    draw.line(corners + [corners[0]], fill=(63, 86, 105), width=AA)
    for x in range(-4, 5):
        draw.line([project(x*.01,0,-.025),project(x*.01,0,.025)],fill=(36,51,66),width=AA)
    for z in [-.02,-.01,0,.01,.02]:
        draw.line([project(-.05,0,z),project(.05,0,z)],fill=(36,51,66),width=AA)
    image = render_surfaces(image,state["surface"],state["film"],project)
    draw = ImageDraw.Draw(image)
    # Neutral nozzle, matching the source position/direction, matching the source geometry.
    nozzle_start=project(-.011,.051,0);nozzle_end=project(-.009,.038,0)
    draw.line([nozzle_start,nozzle_end],fill=(91,110,126),width=14*AA)
    draw.line([nozzle_start,nozzle_end],fill=(148,167,180),width=6*AA)
    text(draw,(38,115),f"t = {time:.3f} с",normal,(228,236,246))
    text(draw,(38,142),f"В плёнке: {mass*1e6:.0f} мг",small)
    text(draw,(38,163),f"Фрагменты: {int(fragments)}",small)
    text(draw,(675,117),"Водная фаза + вязкая фаза",small)
    if time > .12:
        text(draw,(675,143),"Сдвиг поверхности включён",small,(166,207,224))
        draw.line([(735*AA,174*AA),(810*AA,174*AA)],fill=(166,207,224),width=2*AA)
        draw.polygon([(810*AA,174*AA),(800*AA,168*AA),(800*AA,180*AA)],fill=(166,207,224))
    draw.line([(52*AA,538*AA),(120*AA,538*AA)],fill=(160,181,199),width=2*AA)
    text(draw,(57,544),"10 мм",small)
    text(draw,(233,550),"Синтетические коэффициенты · замедленное воспроизведение",small,(144,166,185))
    images.append(image.resize((W,H),Image.Resampling.LANCZOS))
args.output.parent.mkdir(parents=True,exist_ok=True)
durations=[40]*len(images);durations[0]=300;durations[-1]=900
images[0].save(args.output,save_all=True,append_images=images[1:],duration=durations,loop=0,optimize=True,disposal=2)
images[43].save(args.output.with_suffix(".png"))
print(f"GIF PASS: {args.output}, frames={len(images)}, conserved component masses")
