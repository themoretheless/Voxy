"""Orthographic cutaway of computed OBJ surfaces, with CSV measurements.
Requires Pillow. No shape synthesis or displacement exaggeration is applied.
"""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont
import argparse, csv, math, json
parser = argparse.ArgumentParser()
parser.add_argument('--contact', action='store_true')
parser.add_argument('--surface', action='store_true')
parser.add_argument('--unprotected', action='store_true')
parser.add_argument('--reference-load-pa', type=float)
parser.add_argument('--follower-load-pa', type=float)
parser.add_argument('--specimen', choices=['urethra','vagina'])
parser.add_argument('--release', action='store_true')
parser.add_argument('--refinement', action='store_true')
parser.add_argument('--axial-refinement', action='store_true')
parser.add_argument('--radial-refinement', action='store_true')
parser.add_argument('--axial-resolutions', nargs=3, type=int, default=[3,6,12])
parser.add_argument('--audit')
parser.add_argument('--surface-compare', action='store_true')
parser.add_argument('--sectors', type=int, default=8)
parser.add_argument('--segments', type=int, default=3)
parser.add_argument('--radial', type=int, default=1)
parser.add_argument('--meshes', default='/tmp/voxy-urogenital-axial')
parser.add_argument('--csv', default='docs/urogenital-axial.csv')
parser.add_argument('--output', default='docs/urogenital-axial-render.png')
args = parser.parse_args()
audit_rows = json.loads(Path(args.audit).read_text())['states'] if args.audit else []
audit = {(r['specimen'],r.get('radial',1) if args.radial_refinement else r.get('segments',3) if args.axial_refinement else r['sectors'],r['stage']):r for r in audit_rows}
data=list(csv.DictReader(open(args.csv)))
rows = ({(r['specimen'],int(r.get('radial',1) if args.radial_refinement else r['segments'] if args.axial_refinement else r['sectors']),r['stage']):r for r in data} if args.refinement or args.axial_refinement or args.radial_refinement else {(r['specimen'], r['stage']): r for r in data})
font_path = '/System/Library/Fonts/Supplemental/Arial.ttf'
font = ImageFont.truetype(font_path, 22)
small = ImageFont.truetype(font_path, 17)
large = ImageFont.truetype(font_path, 30)
image = Image.new('RGB', (1000 if args.unprotected else 1500, 610 if args.specimen else 1040), '#f4f7fa')
draw = ImageDraw.Draw(image)
draw.text((35, 24), f'Computed walls — follower pressure {args.follower_load_pa:g} Pa' if args.follower_load_pa is not None else f'Computed walls — reference-area load {args.reference_load_pa:g} Pa' if args.reference_load_pa is not None else 'Computed wall — unprotected compression' if args.unprotected else 'Computed walls — nonadjacent surface contact' if args.surface_compare or args.surface else 'Computed contact — angular mesh refinement' if args.refinement else 'Computed walls — selected pair contact' if args.contact else 'Computed layered walls — axial muscle zones', font=large, fill='#132b43')
draw.text((35, 70), 'Actual OBJ geometry • orthographic half cutaway • synthetic materials • no displacement magnification', font=small, fill='#435c70')
stages = ([('rest','Rest'), ('compressed','Compression / no barrier'), ('barrier','Compression / pair barrier')] if args.contact else [('rest', 'Rest'), ('pressure', 'Pressure 20 Pa'), ('local_outer', 'Middle outer muscle: activation 0.03')])
if args.release:
    stages = [('rest','Rest'), ('barrier','Loaded / pair barrier'), ('released','Released / same barrier state')]
if args.refinement:
    stages=[('8','8 angular sectors'),('16','16 angular sectors'),('32','32 angular sectors')]
if args.axial_refinement:
    stages=[(str(n),f'{n} axial segments') for n in args.axial_resolutions]
if args.radial_refinement:
    stages=[(str(n),f'{n} elements per layer') for n in [1,2,3]]
if args.surface_compare:
    stages=[('rest','Rest'),('pair_only','Selected point pairs'),('surface','Nonadjacent surface contact')]
def dot(a,b): return sum(x*y for x,y in zip(a,b))
def subtract(a,b): return [x-y for x,y in zip(a,b)]
def cross(a,b): return [a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
def normalize(a):
    norm=math.sqrt(dot(a,a)); return [v/norm for v in a]
right=normalize([3.,2.,0.]); view=normalize([2.,-3.,2.]); up=normalize(cross(view,right))
light=normalize([2.,-3.,4.])
if args.audit:
    draw.text((35,96),'Red outlines: projected crossing faces, including the hidden half; shared-vertex pairs excluded.',font=small,fill='#bd2f42')
specimens = [('urethra','Urethral wall',.02,.003), ('vagina','Vaginal wall',.03,.012)]
if args.specimen:
    specimens = [item for item in specimens if item[0] == args.specimen]
if args.surface and args.release:
    stages = [('rest','Rest'), ('barrier','Loaded / surface barrier'), ('released','Released / surface barrier')]
if args.unprotected:
    stages = [('rest','Rest'), ('compressed','Compression / no barrier')]
for row_index,(name,label,length,radius) in enumerate(specimens):
    top=120+row_index*430
    for column,(stage,title) in enumerate(stages):
        sectors=int(stage) if args.refinement else args.sectors
        physical_stage='barrier' if args.refinement or args.axial_refinement or args.radial_refinement else stage
        mesh_folder=(Path(args.meshes)/stage if args.surface_compare else Path(args.meshes)/stage if args.axial_refinement or args.radial_refinement else Path(args.meshes)/str(sectors) if args.refinement else Path(args.meshes))
        left=25+column*495
        draw.rounded_rectangle((left,top,left+480,top+410),radius=12,fill='white',outline='#d5dfe7',width=2)
        draw.text((left+18,top+14),label+' / '+title,font=small,fill='#15354f')
        vertices=[]; faces=[]
        for line in (mesh_folder/f'{name}-{physical_stage}.obj').read_text().splitlines():
            fields=line.split()
            if fields[0]=='v': vertices.append(list(map(float,fields[1:])))
            elif fields[0]=='f': faces.append([int(i)-1 for i in fields[1:]])
        assert all(math.isfinite(v) for p in vertices for v in p)
        resolution = int(stage) if args.axial_refinement or args.radial_refinement else sectors
        finding=audit.get((name,resolution,physical_stage),{})
        red_faces={i for pair in finding.get('candidate_pairs',[]) for i in pair}
        face_indices={tuple(face):i for i,face in enumerate(faces)}
        centered=[subtract(v,[0.,0.,length/2]) for v in vertices]
        pixels_per_m=250/(length+radius)
        projected=[(left+245+dot(p,right)*pixels_per_m,top+177-dot(p,up)*pixels_per_m) for p in centered]
        visible=[f for f in faces if sum(vertices[i][0] for i in f)/3>=-1e-12]
        visible.sort(key=lambda f: sum(dot(centered[i],view) for i in f)/3)
        for f in visible:
            a,b,c=[vertices[i] for i in f]
            normal=cross(subtract(b,a),subtract(c,a))
            norm=math.sqrt(dot(normal,normal))
            assert norm>0
            intensity=.35+.65*abs(dot([v/norm for v in normal],light))
            base_color=[220,62,72] if face_indices[tuple(f)] in red_faces else [110,177,211]
            color=tuple(int(v*intensity) for v in base_color)
            draw.polygon([projected[i] for i in f],fill=color,outline='#244e68')
        for index in sorted(red_faces):
            polygon=[projected[i] for i in faces[index]]
            draw.line(polygon+[polygon[0]],fill='#dc3e48',width=2)
        # The demonstrator has five radial rings and three axial segments.
        # Show actual distal ring XY projections, rather than a synthesized ellipse.
        segments = int(stage) if args.axial_refinement else args.segments
        radial = int(stage) if args.radial_refinement else args.radial
        rings = 4*radial+1
        assert len(vertices) == (segments+1)*rings*sectors
        center_x,center_y=left+389,top+172
        section_origin=[sum(v[k] for v in vertices[-rings*sectors:])/(rings*sectors) for k in range(2)]
        section_scale=60/radius
        colors=['white','#dfecf5','#a3c9e2','#76aaca','#467b9b']
        for layer in range(4,-1,-1):
            ring=vertices[-rings*sectors+layer*radial*sectors: -rings*sectors+(layer*radial+1)*sectors] if layer<4 else vertices[-sectors:]
            assert len(ring)==sectors
            xy=[(center_x+(p[0]-section_origin[0])*section_scale,center_y-(p[1]-section_origin[1])*section_scale) for p in ring]
            draw.polygon(xy,fill=colors[layer],outline='#244e68')
        draw.text((left+315,top+248),'Distal rings (XY)',font=small,fill='#435c70')
        values=rows[(name,resolution,physical_stage)] if args.refinement or args.axial_refinement or args.radial_refinement else rows[(name,stage)]
        initial=rows[(name,resolution,'rest')] if args.refinement or args.axial_refinement or args.radial_refinement else rows[(name,'rest')]
        change=(float(values['lumen_volume_m3'])/float(initial['lumen_volume_m3'])-1)*100
        draw.text((left+18,top+296),f'Signed lumen measure: {float(values["lumen_volume_m3"])*1e6:.6f} mL',font=font,fill='#16364f')
        draw.text((left+18,top+326),f'Change from rest: {change:+.4f}%',font=font,fill='#16364f')
        draw.text((left+18,top+356),f'Min J: {float(values["min_j"]):.6f}   Residual: {float(values["residual_n"]):.2e} N',font=small,fill='#435c70')
        footer = (f'Min selected distance: {float(values["min_pair_distance_m"])*1000:.4f} mm' if args.contact
            else f'Reference length: {length*1000:.0f} mm; scale fixed across this row')
        draw.text((left+18,top+382),footer,font=small,fill='#435c70')
draw.text((35,image.height-40),'No contact barrier; surface intersections require a separate audit.' if args.unprotected else 'Nonadjacent faces protected; shared-vertex pairs excluded; anatomical calibration remains open.' if args.surface_compare or args.surface else 'Selected-pair contact only; full surface collision and physiological calibration remain open.' if args.contact else 'Open oval lumen; closed-wall contact and full-body registration are not implemented by this stage.',font=small,fill='#435c70')
Path(args.output).parent.mkdir(parents=True,exist_ok=True)
image.save(args.output)
print(Path(args.output).resolve())
