"""Audit original VQI1 physical inequality residuals using exact rational products."""
import struct
import json
import sys
from fractions import Fraction as F
from pathlib import Path
raw=Path(sys.argv[1]).read_bytes()
assert raw[:4]==b'VQI1'
pos=4

def read(fmt):
    global pos
    size=struct.calcsize(fmt)
    result=struct.unpack_from(fmt,raw,pos)
    pos+=size
    return result

def values(count):
    return read('<'+'d'*count)

rows,width,systems,failed=read('<4I')
tolerance=values(1)[0]
bounds=values(rows)
reactions=values(rows)
for _ in range(rows): values(width)
values(width)  # Whitened coordinates; physical response is audited below.
loads=[]
responses=[]
for _ in range(systems):
    count,band,start,end=read('<4I')
    values(count*band)
    values(count)
    loads.append([values(count) for _ in range(rows)])
    responses.append(values(count))
assert pos==len(raw)
results=[]
for row in range(rows):
    gap=sum((F(a)*F(b) for load,response in zip(loads,responses) for a,b in zip(load[row],response)),F())-F(bounds[row])
    residual=abs(gap) if reactions[row]>0 else max(-gap,F())
    if residual>F(tolerance): results.append({'row':row,'gap':float(gap),'residual':float(residual),'reaction':reactions[row]})
print(json.dumps({'scope':'exact original physical inequality residuals; not force balance or energy validation','rows':rows,'width':width,'systems':systems,'captured_failed_row':failed,'tolerance':tolerance,'violations':results},indent=2))
