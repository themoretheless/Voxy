#!/usr/bin/env python3
"""Verify a frozen-history incremental potential on manufactured tensors.
Not an HGO implementation, experimental fit or thermodynamic dissipation proof.
"""
import json
import math
from pathlib import Path
from stress_memory_reference import advance


def transpose(a):
    return list(map(list,zip(*a)))


def multiply(a,b):
    return [[sum(a[i][k]*b[k][j] for k in range(3)) for j in range(3)] for i in range(3)]


def main():
    # Nonzero symmetric reference history and nonlinear objective elastic energy.
    f=[[1.2,.13,-.04],[.07,.9,.08],[.02,-.03,1.1]]
    previous=[[110.,13.,-8.],[13.,95.,3.],[-8.,3.,108.]]
    memory=[[21.,-2.,1.],[-2.,17.,4.],[1.,4.,19.]]
    tau,beta,dt,a=31.75,.24,.25,100.
    z=dt/tau; decay=math.exp(-z); gain=-math.expm1(-z)/z
    factor=1+beta*gain
    history=[[decay*memory[i][j]-beta*gain*previous[i][j] for j in range(3)] for i in range(3)]
    # Psi_el=a/4 C:C; S_el=2*dPsi_el/dC=a*C.
    def potential(f):
        c=multiply(transpose(f),f)
        return factor*a/4*sum(x*x for row in c for x in row)+.5*sum(history[i][j]*c[i][j] for i in range(3) for j in range(3))
    c=multiply(transpose(f),f)
    elastic=[[a*x for x in row] for row in c]
    q=advance(tuple(x for row in memory for x in row),tuple(x for row in previous for x in row),tuple(x for row in elastic for x in row),dt,tau,beta)
    total=[[elastic[i][j]+q[3*i+j] for j in range(3)] for i in range(3)]
    first_piola=multiply(f,total)
    errors=[]
    for h in (1e-4,1e-5,1e-6):
        worst=0.
        for i in range(3):
            for j in range(3):
                plus=[r[:] for r in f];minus=[r[:] for r in f]
                plus[i][j]+=h;minus[i][j]-=h
                fd=(potential(plus)-potential(minus))/(2*h)
                worst=max(worst,abs(fd-first_piola[i][j]))
        errors.append(dict(step=h,max_first_piola_error_pa=worst))
    assert errors[-1]['max_first_piola_error_pa']<1e-6
    report=dict(scope='Manufactured nonlinear elastic law with frozen reference-stress history; no tissue calibration or HGO equivalence',
                potential='(1+beta*g)*Psi_el(C) + 0.5*(decay*Q_prev-beta*g*S_prev):C',
                g='(1-exp(-dt/tau))/(dt/tau)',
                condition='Symmetric second Piola tensors in a fixed reference measure; same elastic potential drives memory',
                derivative='P = F*(S_el+Q_trial)',finite_difference_checks=errors,
                outstanding='HGO law, volumetric/isochoric separation, multi-branch coupling, dissipation and Body integration')
    Path('docs/stress-memory-potential-proof.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report,indent=2))


if __name__=='__main__':
    main()
