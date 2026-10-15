"""Independent exact-view-grazing Smith GGX reference (N.V=0 limit).

The unit-Fresnel loss tends to zero. At grazing VNDF slope azimuth has
cos(phi)/2 density on [-pi/2,pi/2]; after r=alpha*tan(theta), radial density
is 4/pi*sin(theta)^2. Integrate Schlick's fifth-degree polynomial over the
azimuth analytically, then adapt theta. These reference limits are not yet
implemented by the production moment producer or its center-sampled LUT.
"""
import json
import math
from ggx_normal_reference import simpson


def grazing_moments(roughness):
    assert 0 <= roughness <= 1
    if roughness == 0:
        return dict(A=0.,B=1.,unit_loss=0.)
    alpha=roughness*roughness
    def channel(distance,want_a):
        # Distance from pi/2 avoids subtracting nearly equal angles.
        sine=math.cos(distance);cosine=math.sin(distance)
        q=alpha*sine/math.sqrt(cosine*cosine+alpha*alpha*sine*sine)
        # Azimuth-averaged 1-(1-V.H)^5, evaluated directly for small q.
        a=q*(5*math.pi/4-q*(20/3-q*(15*math.pi/8-q*(8/3-q*5*math.pi/32))))
        b=1-5*math.pi/4*q+20/3*q*q-15*math.pi/8*q**3+8/3*q**4-5*math.pi/32*q**5
        return 4/math.pi*sine*sine*(a if want_a else b)
    # Partition the narrow endpoint layer before adaptive refinement.
    points=sorted(set([0.,math.pi/2]+[d for d in [alpha/4,alpha,alpha*4,.001,.01,.1,.5,1.] if 0<d<math.pi/2]))
    result=[];errors=[]
    for want_a in [True,False]:
        values=[];estimates=[]
        for left,right in zip(points,points[1:]):
            value,error=simpson(lambda distance:channel(distance,want_a),left,right,1e-15/(len(points)-1),depth=40)
            values.append(value);estimates.append(error)
        result.append(math.fsum(values));errors.append(math.fsum(estimates))
    assert abs(sum(result)-1.)<1e-11
    assert all(0<=v<=1 for v in result)
    return dict(A=result[0],B=result[1],unit_loss=0.,estimated_errors=errors)


def main():
    maximum=grazing_moments(1.)
    assert abs(maximum['B']-1/21)<1e-13
    assert abs(maximum['A']-20/21)<1e-13
    for roughness in [0.,.0001,.001,.01,.1,.4,.8,1.]:
        print(json.dumps(dict(nv=0.,roughness=roughness,reference=grazing_moments(roughness))))
    print(json.dumps(dict(status='grazing_reference_self_checks_passed',scope='Independent f64 reference and analytic alpha=1 check; estimates are not formal certificates. GPU zero-view limit and table boundary integration not implemented.')))


if __name__=='__main__':main()
