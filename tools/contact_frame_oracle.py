#!/usr/bin/env python3
"""Selected-feature oracle for logged world poses and rounded origin shifts.

Recomputed translated world endpoints are not the solver's latent local endpoints.
New frame records separately compare actual latent local endpoints for the same selected pair.
This report is a frame-sensitivity diagnostic, not a nonlinear solver checkpoint.
"""
import argparse
from decimal import Decimal, localcontext
import json
from pathlib import Path
import re


def sub(a, b):
    return [x-y for x, y in zip(a, b)]


def dot(a, b):
    return sum((x*y for x, y in zip(a, b)), Decimal(0))


def cross(a, b):
    return [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]]


def selected_gap(body, obstacle, active_body, active_obstacle):
    a = [[Decimal.from_float(x) for x in p] for p in body]
    b = [[Decimal.from_float(x) for x in p] for p in obstacle]
    if len(active_body)==1 and len(active_obstacle)==1:
        delta=sub(a[active_body[0]],b[active_obstacle[0]])
        distance=dot(delta,delta).sqrt()
    elif sorted((len(active_body),len(active_obstacle)))==[1,2]:
        vertex,edge=(a[active_body[0]],[b[i] for i in active_obstacle]) if len(active_body)==1 else (b[active_obstacle[0]],[a[i] for i in active_body])
        direction=sub(edge[1],edge[0]);r=sub(vertex,edge[0])
        parameter=dot(r,direction)/dot(direction,direction)
        if not (0<parameter<1): raise ValueError('projection leaves selected interior edge')
        delta=[r[i]-parameter*direction[i] for i in range(3)]
        distance=dot(delta,delta).sqrt()
    elif len(active_body) == 2 and len(active_obstacle) == 2:
        p, q = [a[i] for i in active_body]
        r, t = [b[i] for i in active_obstacle]
        u, v, w = sub(q, p), sub(t, r), sub(p, r)
        aa, bb, cc, dd, ee = dot(u,u), dot(u,v), dot(v,v), dot(u,w), dot(v,w)
        denominator = aa*cc-bb*bb
        s = (bb*ee-cc*dd)/denominator
        t = (aa*ee-bb*dd)/denominator
        if not (0 < s < 1 and 0 < t < 1):
            raise ValueError('edge parameters leave selected interior feature')
        delta = [w[i]+s*u[i]-t*v[i] for i in range(3)]
        distance = dot(delta,delta).sqrt()
    elif sorted((len(active_body), len(active_obstacle))) == [1,3]:
        triangle, vertex = (a,b[active_obstacle[0]]) if len(active_body)==3 else (b,a[active_body[0]])
        u, v, r = sub(triangle[1],triangle[0]), sub(triangle[2],triangle[0]), sub(vertex,triangle[0])
        aa, bb, cc, dd, ee = dot(u,u), dot(u,v), dot(v,v), dot(u,r), dot(v,r)
        denominator = aa*cc-bb*bb
        s, t = (cc*dd-bb*ee)/denominator, (aa*ee-bb*dd)/denominator
        if not (s>0 and t>0 and s+t<1):
            raise ValueError('projection leaves selected interior face')
        normal = cross(u,v)
        distance = abs(dot(r,normal))/dot(normal,normal).sqrt()
    else:
        raise ValueError('unsupported logged feature topology')
    return distance-Decimal.from_float(0.0001)


def report(path):
    rows = []
    with localcontext() as ctx:
        ctx.prec=90
        for line in path.read_text().splitlines():
            if line.startswith('IMPLICIT_FRAME_GEOMETRY '):
                m=re.search(r'dt=(\S+) world_body=(\[.*?\]) local_body=(\[.*?\]) world_obstacle=Some\((\[.*?\])\) local_obstacle=Some\((\[.*?\])\) nearest_world=(.*?) nearest_local=(.*)$',line)
                if not m: continue
                world_body,local_body=json.loads(m[2]),json.loads(m[3])
                world_obstacle,local_obstacle=json.loads(m[4]),json.loads(m[5])
                info=m[6]
                face=json.loads(re.search(r'body_face: (\[.*?\])',info)[1])
                wa=json.loads(re.search(r'body_weights: (\[.*?\])',info)[1])
                wb=json.loads(re.search(r'obstacle_weights: (\[.*?\])',info)[1])
                active_a=[i for i,w in enumerate(wa) if w>0]
                active_b=[i for i,w in enumerate(wb) if w>0]
                try:
                    world=selected_gap([world_body[i] for i in face],world_obstacle,active_a,active_b)
                    local=selected_gap([local_body[i] for i in face],local_obstacle,active_a,active_b)
                    energy=lambda g: -Decimal.from_float(100.)*(g-Decimal.from_float(.003))**2*(g/Decimal.from_float(.003)).ln() if g>0 else None
                    ew,el=energy(world),energy(local)
                    rows.append(dict(mode='actual-latent-local-endpoint',dt_s=float(m[1]),body_face=face,
                                     world_gap_m=str(world),actual_local_gap_m=str(local),gap_shift_m=str(local-world),
                                     selected_pair_energy_shift_j=str(el-ew) if ew is not None and el is not None else None,
                                     nearest_local_debug=m[7]))
                except ValueError as error: rows.append(dict(mode='actual-latent-local-endpoint',error=str(error),body_face=face))
                continue
            if not line.startswith('IMPLICIT_TERMINAL_GEOMETRY '):
                continue
            m = re.search(r'dt=(\S+) body_start=(\[.*?\]) body_end=(\[.*?\]) nearest_end=(.*?) obstacle_triangle_end=Some\((\[.*\])\)$', line)
            if not m:
                continue
            start, end, obstacle = json.loads(m[2]), json.loads(m[3]), json.loads(m[5])
            info = m[4]
            face = json.loads(re.search(r'body_face: (\[.*?\])', info)[1])
            wa = json.loads(re.search(r'body_weights: (\[.*?\])', info)[1])
            wb = json.loads(re.search(r'obstacle_weights: (\[.*?\])', info)[1])
            body = [end[i] for i in face]
            origin = start[0]
            shifted = lambda points: [[p[i]-origin[i] for i in range(3)] for p in points]
            active_a = [i for i,w in enumerate(wa) if w>0]
            active_b = [i for i,w in enumerate(wb) if w>0]
            try:
                world = selected_gap(body,obstacle,active_a,active_b)
                local = selected_gap(shifted(body),shifted(obstacle),active_a,active_b)
                rows.append(dict(mode='translated-published-world-endpoint',dt_s=float(m[1]),body_face=face,world_gap_m=str(world),
                                 translated_world_gap_m=str(local),gap_shift_m=str(local-world)))
            except ValueError as error:
                rows.append(dict(error=str(error),body_face=face))
    return dict(scope=__doc__,decimal_precision=90,poses=rows)


if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('log',type=Path)
    args=parser.parse_args()
    print(json.dumps(report(args.log),indent=2))
