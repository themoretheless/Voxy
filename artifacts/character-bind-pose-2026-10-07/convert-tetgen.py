"""Offline geometry-only TetGen .node/.ele conversion to Voxy's Medit profile."""
import pathlib, sys
prefix=pathlib.Path(sys.argv[1])
def rows(path):
    return [l.split('#')[0].split() for l in path.read_text().splitlines() if l.split('#')[0].strip()]
n=rows(prefix.with_suffix(prefix.suffix+'.node')); e=rows(prefix.with_suffix(prefix.suffix+'.ele'))
np,dim,attrs,marks=map(int,n[0]); assert dim==3 and attrs==0 and marks in (0,1)
nc,order,cellattrs=map(int,e[0]); assert order==4 and cellattrs==0
assert len(n)==np+1 and len(e)==nc+1
nodes={}; lines=['MeshVersionFormatted 1','Dimension 3',f'Vertices {np}']
for row in n[1:]:
    assert len(row)==4+marks
    identity=int(row[0]);assert identity not in nodes
    nodes[identity]=len(nodes)+1;lines.append(' '.join(row[1:4])+' 0')
lines+=['Triangles 0',f'Tetrahedra {nc}'];seen=set()
for row in e[1:]:
    assert len(row)==5;identity=int(row[0]);assert identity not in seen;seen.add(identity)
    lines.append(' '.join(str(nodes[int(x)]) for x in row[1:])+' 0')
lines.append('End');pathlib.Path(sys.argv[2]).write_text('\n'.join(lines)+'\n')
print({'vertices':np,'tetrahedra':nc,'scope':'geometry only; node boundary markers are topology tags, not imported material references'})
