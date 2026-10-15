"""Independent normal-incidence GGX loss audit; no GPU/source modification.

For unit Fresnel, VNDF=NDF at normal incidence. Half-normal slope squared
q has density a^2/(a^2+q)^2. q>=1 reflects below the horizon, with exact
probability a^2/(1+a^2). Above-horizon Smith-correlated weight is
1/(1+Lambda(L)). Integrate the remaining masking loss in log(q).
Adaptive Simpson differences estimate quadrature error, not a certificate.
"""
import argparse
import json
import math
import struct
from pathlib import Path


def simpson(f, left, right, tolerance, depth=28):
    middle = (left + right) / 2
    fl, fm, fr = f(left), f(middle), f(right)
    whole = (right-left) * (fl + 4*fm + fr) / 6
    def refine(l, r, a, b, c, previous, tol, remaining):
        m = (l+r)/2
        d, e = f((l+m)/2), f((m+r)/2)
        first = (m-l)*(a+4*d+b)/6
        second = (r-m)*(b+4*e+c)/6
        delta = first+second-previous
        # Stop at the f64 roundoff floor as well as the requested tolerance.
        # Refining below that floor creates cancellation noise, not accuracy.
        roundoff = 64*math.ulp(1.)*(abs(first)+abs(second)+abs(previous))
        if abs(delta) <= max(15*tol, roundoff):
            return first+second+delta/15, abs(delta)/15+roundoff
        if remaining == 0:
            raise ArithmeticError(f'quadrature did not converge: interval=({l},{r}), values=({a},{b},{c}), delta={delta}, tolerance={tol}, roundoff={roundoff}')
        x, ex = refine(l, m, a, d, b, first, tol/2, remaining-1)
        y, ey = refine(m, r, b, e, c, second, tol/2, remaining-1)
        return x+y, ex+ey
    return refine(left, right, fl, fm, fr, whole, tolerance, depth)


def normal_loss(roughness):
    if roughness == 0:
        return dict(loss=0., null_mass=0., masking_loss=0., estimated_error=0.)
    a2 = roughness**4
    null_mass = a2/(1+a2)
    log_min = math.log(a2)-40
    def integrand(t):
        q = math.exp(t)
        c = (1-q)/(1+q)
        if c <= 0:
            masking = 1.
        else:
            x = a2 * (4*q/(1+q)**2)/(c*c)
            lam = x/(2*(math.sqrt(1+x)+1))
            masking = lam/(1+lam)
        return a2*q/(a2+q)**2 * masking
    masking, error = simpson(integrand, log_min, 0., max(1e-20, a2*1e-11))
    # Omitted q<q_min masking is bounded by its probability mass.
    omitted_mass_bound = math.exp(-40)/(1+math.exp(-40))
    return dict(loss=null_mass+masking, null_mass=null_mass,
                masking_loss=masking, estimated_error=error,
                omitted_interval_absolute_bound=omitted_mass_bound)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('gpu_results', type=Path)
    parser.add_argument('--expect-baseline-failure', action='store_true')
    args = parser.parse_args()
    rows = [json.loads(line) for line in args.gpu_results.read_text().splitlines()]
    assert abs(normal_loss(1.)['loss']-math.log(2)) < 1e-10
    failures = []
    relative_errors = []
    for row in rows:
        if row.get('nv') != 1:
            continue
        # Rust prints the shortest f32 representation; recover the actual input.
        roughness = struct.unpack('<f', struct.pack('<f', row['roughness']))[0]
        reference = normal_loss(roughness)
        sampled = row['unit_loss']
        # Exact lower bound alone proves failure, independent of quadrature.
        below_bound = sampled < reference['null_mass']*(1-1e-6)
        relative_error = abs(sampled-reference['loss'])/reference['loss'] if reference['loss'] else abs(sampled)
        relative_errors.append(relative_error)
        result = dict(relative_error=relative_error, roughness=row['roughness'], samples=row['samples'],
                      sampled_loss=sampled, reference=reference,
                      violates_exact_null_lower_bound=below_bound)
        print(json.dumps(result))
        if below_bound:
            failures.append(result)
    if args.expect_baseline_failure:
        assert failures, 'expected rare-event regression no longer reproduced; reassess baseline'
        status = 'baseline_accuracy_failure_reproduced'
    else:
        assert not failures, 'GPU loss violates exact null probability lower bound'
        assert max(relative_errors) < 1e-4, 'normal-incidence relative loss accuracy failed'
        status = 'normal_incidence_relative_loss_gate_passed'
    print(json.dumps(dict(status=status, lower_bound_violations=len(failures),
                          max_relative_loss_error=max(relative_errors),
                          relative_loss_gate=1e-4,
                          scope='normal incidence only; not production or full directional qualification')))


if __name__ == '__main__':
    main()
