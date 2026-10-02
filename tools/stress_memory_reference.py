#!/usr/bin/env python3
"""Exact convolution oracle for a prescribed piecewise-linear elastic stress.

This is not a tissue fit or a complete finite-strain material. All tensor
components must be expressed in one consistent reference stress measure.
"""
import json
import math
from pathlib import Path


def advance(memory, previous_stress, next_stress, dt, tau, beta):
    """Q' + Q/tau = beta*S', integrated exactly for linear S on this interval."""
    if not all(math.isfinite(x) for x in (dt, tau, beta)) or dt <= 0 or tau <= 0 or beta < 0:
        raise ValueError('invalid convolution parameters')
    if len(memory) != len(previous_stress) or len(memory) != len(next_stress) or not memory:
        raise ValueError('incompatible stress components')
    if any(not math.isfinite(x) for values in (memory, previous_stress, next_stress) for x in values):
        raise ValueError('nonfinite stress history')
    z = dt/tau
    weight = math.exp(-z)
    gain = -math.expm1(-z)/z if z else 1.
    result = tuple(weight*q + beta*gain*(b-a) for q,a,b in zip(memory,previous_stress,next_stress))
    if any(not math.isfinite(x) for x in result):
        raise ValueError('stress-memory overflow')
    return result


def verify():
    tau, beta, rate = 31.75, 0.24, 100.
    # Manufactured inputs, not experimental measurements.
    q = advance((123.,), (10.,), (10.,), 900., tau, beta)[0]
    assert math.isclose(q, 123*math.exp(-900/tau), rel_tol=1e-14)
    duration = 120.
    exact = beta*rate*tau*(-math.expm1(-duration/tau))
    errors = []
    for n in (1, 12, 120, 1200):
        memory = (0.,)
        for k in range(n):
            memory = advance(memory, (rate*duration*k/n,), (rate*duration*(k+1)/n,), duration/n, tau, beta)
        error = abs(memory[0]-exact)
        assert error <= 1e-10*max(1.,abs(exact))
        errors.append(dict(substeps=n, absolute_error_pa=error))
    tiny = advance((0.,), (0.,), (1.,), 1e-12, tau, beta)[0]
    assert math.isclose(tiny,beta,rel_tol=1e-12)
    # Linear operator: tensor components evolve independently in a fixed measure.
    tensor = advance((0.,)*9, (0.,)*9, tuple(range(9)), 1., tau, beta)
    for k in range(9):
        assert tensor[k] == advance((0.,),(0.,),(float(k),),1.,tau,beta)[0]
    for dt in (0., -1., math.nan):
        try:
            advance((0.,),(0.,),(1.,),dt,tau,beta)
        except ValueError:
            pass
        else:
            raise AssertionError('invalid duration accepted')
    return dict(scope='Manufactured analytical stress-history oracle; not experimental data or a calibrated tissue law',
                equation='dQ/dt + Q/tau = beta*dS/dt',
                interpolation='Elastic stress linear on each interval, all components in a fixed reference measure',
                tau_s=tau,beta=beta,elastic_stress_rate_pa_s=rate,duration_s=duration,
                analytical_memory_pa=exact,refinement_checks=errors,
                constant_stress_decay_verified=True,tiny_interval_verified=True,
                tensor_components_verified=True,invalid_duration_rejected=True)


if __name__ == '__main__':
    report = verify()
    path = Path('docs/stress-memory-reference-proof.json')
    path.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report,indent=2))
