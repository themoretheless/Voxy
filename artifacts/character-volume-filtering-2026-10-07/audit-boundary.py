"""Independent connectivity audit of offline Medit candidates; no geometry repair."""
import collections, json, pathlib, sys
p=pathlib.Path(sys.argv[1]); tokens=iter(p.read_text().split())
def expect(s):
    assert next(tokens)==s
for s in ('MeshVersionFormatted','1','Dimension','3','Vertices'): expect(s)
n=int(next(tokens)); points=[]
for _ in range(n):
    points.append([float(next(tokens)) for _ in range(3)]); expect('0')
for s in ('Triangles','0','Tetrahedra'): expect(s)
cells=[]
for _ in range(int(next(tokens))):
    cells.append([int(next(tokens))-1 for _ in range(4)]); expect('0')
expect('End'); assert next(tokens,None) is None
faces=collections.defaultdict(list)
for i,(a,b,c,d) in enumerate(cells):
    for f in ((b,c,d),(a,d,c),(a,b,d),(a,c,b)): faces[tuple(sorted(f))].append(i)
boundary=[f for f,owners in faces.items() if len(owners)==1]
links=collections.defaultdict(list)
for a,b,c in boundary:
    for v,u,w in ((a,b,c),(b,c,a),(c,a,b)): links[v].append((u,w))
bad=[]
for v,edges in links.items():
    adjacency=collections.defaultdict(list)
    for a,b in edges: adjacency[a].append(b); adjacency[b].append(a)
    remaining=set(adjacency); sizes=[]
    while remaining:
        todo=[remaining.pop()]; count=0
        while todo:
            a=todo.pop(); count+=1
            for b in adjacency[a]:
                if b in remaining: remaining.remove(b); todo.append(b)
        sizes.append(count)
    if len(sizes)!=1 or any(len(x)!=2 for x in adjacency.values()):
        bad.append({'vertex':v,'position_m':points[v],'link_component_sizes':sizes,'link_degrees':dict(collections.Counter(map(len,adjacency.values())))})
cell_adjacency=[[] for _ in cells]
for owners in faces.values():
    if len(owners)==2:
        a,b=owners; cell_adjacency[a].append(b); cell_adjacency[b].append(a)
remaining=set(range(len(cells))); components=[]
while remaining:
    todo=[remaining.pop()]; members=[]
    while todo:
        a=todo.pop(); members.append(a)
        for b in cell_adjacency[a]:
            if b in remaining: remaining.remove(b); todo.append(b)
    components.append(len(members))
print(json.dumps({'input':str(p),'vertices':n,'tetrahedra':len(cells),'boundary_faces':len(boundary),'face_connected_volume_component_sizes':sorted(components,reverse=True),'nonmanifold_faces':sum(len(x)>2 for x in faces.values()),'bad_boundary_vertices':bad,'scope':'independent topology audit only; not geometric validity or full source skin admission'},indent=2))
