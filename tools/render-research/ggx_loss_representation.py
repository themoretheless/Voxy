"""Diagnostic only: interpolate actual GPU nodes after extracting roughness^4.

No production admission or change of accuracy thresholds. Positive roughness
nodes are used to extrapolate the coefficient towards zero, so the coefficient
limit itself remains unqualified. This experiment identifies whether the raw
loss representation is the dominant source of interpolation error.
"""
import argparse
import json
import math
from pathlib import Path


def weights(t):
    return [-t*(t-1)*(t-2)/6, (t+1)*(t-1)*(t-2)/2,
            -(t+1)*t*(t-2)/2, (t+1)*t*(t-1)/6]


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('table', type=Path)
    p.add_argument('reference', type=Path)
    p.add_argument('--square-view-grid', action='store_true')
    args = p.parse_args()
    rows = [json.loads(l) for l in args.table.read_text().splitlines()]
    cells = [r for r in rows if 'nv' in r and not r.get('interpolated')]
    n = math.isqrt(len(cells))
    assert n*n == len(cells) and n >= 5
    table = {}
    for r in cells:
        x = round((math.sqrt(r['nv']) if args.square_view_grid else r['nv'])*(n-1))
        y = round(r['roughness']*(n-1))
        assert (x,y) not in table
        table[x,y] = r
    assert len(table) == n*n
    results = []
    for ref in [json.loads(l) for l in args.reference.read_text().splitlines()]:
        nv, roughness = ref['nv'], ref['roughness']
        px = (math.sqrt(nv) if args.square_view_grid else nv)*(n-1)
        py = roughness*(n-1)
        ox = min(max(math.floor(px)-1, 0), n-4)
        # Never divide the exact zero row by roughness^4. This is explicit
        # extrapolation from positive nodes, not an asserted boundary limit.
        oy = min(max(math.floor(py)-1, 1), n-4)
        wx, wy = weights(px-ox-1), weights(py-oy-1)
        coefficient = math.fsum(
            wx[x]*wy[y]*table[ox+x,oy+y]['unit_loss'] /
            (((oy+y)/(n-1))**4)
            for y in range(4) for x in range(4))
        loss = roughness**4*coefficient
        expected = ref['reference']['loss']
        error = abs(loss-expected)/expected if expected else abs(loss)
        results.append(dict(nv=nv, roughness=roughness, loss=loss,
                            reference_loss=expected, relative_loss_error=error,
                            passed=0 <= loss <= 1 and error <= 1e-4))
    print(json.dumps(dict(scope='CPU diagnostic on actual GPU nodes; no production admission',
                          node_count=n*n, relative_threshold=1e-4,
                          failures=sum(not r['passed'] for r in results),
                          results=results), indent=2))


if __name__ == '__main__':
    main()
