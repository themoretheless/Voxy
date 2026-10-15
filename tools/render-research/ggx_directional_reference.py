"""Independent GGX unit-energy loss in slope space (f64 quadrature).

No visible-normal sampling or GPU shader code is used. Radial below-horizon
mass is integrated analytically; masking loss uses adaptive quadrature in log
radius. Azimuth midpoint-grid refinement estimates error, not a certificate.
"""
import argparse
import json
import math
import struct
from pathlib import Path
from ggx_normal_reference import simpson, normal_loss


def directional_loss(nv, roughness, angles):
    if roughness == 0:
        return dict(loss=0., null_mass=0., masking_loss=0.)
    alpha=roughness*roughness
    a2=alpha*alpha
    sv=math.sqrt((1-nv)*(1+nv))
    x=a2*(sv/nv)**2
    lambda_v=x/(2*(math.sqrt(1+x)+1))
    factor=a2/(math.pi*nv*(1+lambda_v))
    nulls=[]; masks=[]
    for i in range(angles):
        cosine=math.cos((i+.5)*2*math.pi/angles)
        sine=math.sin((i+.5)*2*math.pi/angles)
        k=sv*cosine
        root=math.sqrt(nv*nv+k*k)
        # Stable positive root of nv*r^2-2*k*r-nv=0.
        rc=(k+root)/nv if k>=0 else nv/(root-k)
        def tail1(r): return 1/(2*(a2+r*r))
        def tail2(r): return math.atan(alpha/r)/(2*alpha)+r/(2*(a2+r*r))
        first=tail1(rc); second=tail2(rc)
        if k<0:
            maximum=-nv/k
            first-=tail1(maximum); second-=tail2(maximum)
        nulls.append(factor*(nv*first+k*second))
        upper=math.log(rc/alpha)
        def mask_integrand(distance):
            r=rc*math.exp(-distance)
            vh_numerator=nv+k*r
            # Factor the horizon root; expm1 preserves distance to it.
            horizon_distance=-rc*math.expm1(-distance)
            light_z=nv*horizon_distance*(r+1/rc)/(1+r*r)
            if light_z<=0:
                loss=1.
            else:
                tangent_x=2*vh_numerator*r*cosine/(1+r*r)-sv
                tangent_y=2*vh_numerator*r*sine/(1+r*r)
                y=a2*(tangent_x*tangent_x+tangent_y*tangent_y)/(light_z*light_z)
                lam=y/(2*(math.sqrt(1+y)+1))
                loss=lam/(1+lambda_v+lam)
            return factor*vh_numerator*r*r/(a2+r*r)**2*loss
        value,_=simpson(mask_integrand,0.,upper+20,max(1e-21,a2*1e-12),depth=40)
        masks.append(value)
    null_mass=math.fsum(nulls)*2*math.pi/angles
    masking=math.fsum(masks)*2*math.pi/angles
    return dict(loss=null_mass+masking,null_mass=null_mass,masking_loss=masking)


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('gpu_results',type=Path)
    parser.add_argument('--angles',type=int,default=256)
    args=parser.parse_args()
    rows=[json.loads(l) for l in args.gpu_results.read_text().splitlines()]
    for nv in [.05,.25,.7,1.]:
        expected=nv*math.log1p(1/nv)
        assert abs(directional_loss(nv,1.,args.angles)['loss']-expected)<1e-8
    for r in [.005,.05,.25,.6,1.]:
        assert abs(directional_loss(1.,r,16)['loss']-normal_loss(r)['loss'])<1e-10
    maximum=0.; refinement=0.; failures=0
    for row in rows:
        if row.get('samples')!=65536:continue
        nv,r=[struct.unpack('<f',struct.pack('<f',row[k]))[0] for k in ['nv','roughness']]
        coarse=directional_loss(nv,r,args.angles)
        fine=directional_loss(nv,r,2*args.angles)
        delta=abs(fine['loss']-coarse['loss'])
        relative=abs(row['unit_loss']-fine['loss'])/fine['loss'] if fine['loss'] else abs(row['unit_loss'])
        maximum=max(maximum,relative);refinement=max(refinement,delta)
        failures+=relative>=1e-4
        print(json.dumps(dict(nv=nv,roughness=r,samples=row['samples'],gpu_loss=row['unit_loss'],reference=fine,azimuth_refinement_difference=delta,relative_loss_error=relative)),flush=True)
    print(json.dumps(dict(max_relative_loss_error=maximum,max_azimuth_refinement_difference=refinement,failures_at_relative_gate_1e_4=failures,status='accuracy_gate_failed' if failures else 'directional_loss_gate_passed',scope='unit-Fresnel loss only; Fresnel B and full BRDF reference still required')))
    raise SystemExit(1 if failures else 0)


if __name__=='__main__':main()
