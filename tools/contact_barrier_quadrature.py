#!/usr/bin/env python3
"""Scalar force-quadrature diagnostic; does not simulate native mesh dynamics."""
import json
import math

NODES = [(0.046910077030668, 0.11846344252809454),
         (0.23076534494715845, 0.23931433524968325), (0.5, 0.28444444444444444),
         (0.7692346550528415, 0.23931433524968325),
         (0.953089922969332, 0.11846344252809454)]
MINIMUM, ACTIVATION, STIFFNESS = 0.0001, 0.003, 100.0


def energy(gap):
    return -STIFFNESS * (gap - ACTIVATION)**2 * math.log(gap / ACTIVATION)


def derivative(gap):
    offset = gap - ACTIVATION
    return -STIFFNESS * (2 * offset * math.log(gap / ACTIVATION) + offset**2 / gap)


def evaluate(endpoint_gap, mode):
    start_gap = 0.00010005 - MINIMUM
    panels = [(0.0, 1.0, start_gap, endpoint_gap)]
    best = math.inf
    admitted = False
    for _ in range(128):
        defects, work = [], 0.0
        for left, right, ga, gb in panels:
            panel_work = 0.0
            for node, weight in NODES:
                if mode == 'global-time':
                    time = left + (right - left) * node
                    gap = (start_gap + (endpoint_gap - start_gap) * time if time <= 0.5
                           else endpoint_gap + (start_gap - endpoint_gap) * (1 - time))
                else:
                    gap = (ga + (gb - ga) * node if node <= 0.5
                           else gb + (ga - gb) * (1 - node))
                panel_work += (gb - ga) * weight * derivative(gap)
            defects.append(abs(energy(gb) - energy(ga) - panel_work))
            work += panel_work
        error = energy(endpoint_gap) - energy(start_gap) - work
        best = min(best, abs(error))
        if abs(error) < 1e-10:
            admitted = True
            break
        if len(panels) >= 128:
            break
        index = max(range(len(panels)), key=lambda i: defects[i])
        a, b, ga, gb = panels[index]
        mid, gm = (a + b) / 2, (ga + gb) / 2
        panels[index:index + 1] = [(a, mid, ga, gm), (mid, b, gm, gb)]
    return dict(gap_m=endpoint_gap, mode=mode, panels=len(panels), admitted=admitted,
                work_defect_j=error, best_defect_j=best)


if __name__ == '__main__':
    results = [evaluate((MINIMUM + gap) - MINIMUM, mode)
               for gap in (4e-12, 4e-15, 4e-18)
               for mode in ('global-time', 'panel-local-gap')]
    print(json.dumps(dict(scope=__doc__, results=results), indent=2))
