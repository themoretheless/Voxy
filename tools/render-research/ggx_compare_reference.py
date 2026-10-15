"""Compare every prescribed reference case, using exact source-f32 identities."""
import argparse
import json
import math
import struct
from pathlib import Path


def key(row):
    return tuple(struct.pack('<f',float(row[name])) for name in ('nv','roughness'))


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('reference',type=Path)
    parser.add_argument('gpu',type=Path)
    parser.add_argument('--samples',type=int,required=True)
    parser.add_argument('--fresnel',action='store_true',help='also qualify independent Schlick A/B references')
    args=parser.parse_args()
    refs=[json.loads(l) for l in args.reference.read_text().splitlines() if l.strip()]
    refs=[r for r in refs if 'nv' in r]
    gpu=[json.loads(l) for l in args.gpu.read_text().splitlines() if l.strip()]
    gpu=[r for r in gpu if r.get('samples')==args.samples]
    reference={key(r):r for r in refs};candidates={key(r):r for r in gpu}
    assert len(reference)==len(refs) and len(candidates)==len(gpu),'duplicate case identity'
    assert reference.keys()==candidates.keys(),'missing or extra reference cases'
    failures=[];results=[]
    for identity,ref in reference.items():
        q=candidates[identity]
        if q.get('interpolated'):
            assert isinstance(q.get('valid'),bool),'interpolation must expose explicit GPU validity'
        gpu_valid=q.get('valid',True) is True
        assert all(math.isfinite(q[k]) and 0<=q[k]<=1 for k in ('A','B','unit_loss'))
        expected=ref['reference']['loss'];observed=q['unit_loss']
        relative=abs(observed-expected)/expected if expected else abs(observed)
        nv=ref['nv'];roughness=ref['roughness'];alpha=roughness*roughness
        rmin=alpha*math.exp(-20);sv=math.sqrt((1-nv)*(1+nv))
        minimum_light_z=(nv*(1-rmin*rmin)-2*sv*rmin)/(1+rmin*rmin)
        loss_bound=1.
        if minimum_light_z>0:
            maximum_tangent=sv+2*rmin*(nv+sv*rmin)
            x=alpha*alpha*(maximum_tangent/minimum_light_z)**2
            lambda_light=x/(2*(math.sqrt(1+x)+1))
            y=alpha*alpha*(sv/nv)**2
            lambda_view=y/(2*(math.sqrt(1+y)+1))
            loss_bound=lambda_light/(1+lambda_view+lambda_light)
        # Bound the omitted masking contribution, not the entire probability
        # of the tiny slope disk: its kernel tends to zero for smooth normal view.
        omitted_loss_bound=ref['reference'].get('omitted_radius_probability_bound',0)*loss_bound
        uncertainty=ref['reference'].get('omitted_horizon_loss_bound',0)+ref['azimuth_refinement_difference']+ref['reference'].get('radial_estimated_error',0)+omitted_loss_bound
        # Numerical uncertainty diagnostics must be below the comparison budget;
        # these estimates do not constitute a rigorous quadrature certificate.
        assert uncertainty<=expected*1e-6+1e-34,'reference uncertainty too large'
        result=dict(nv=q['nv'],roughness=q['roughness'],samples=args.samples,relative_loss_error=relative,gpu_loss=observed,reference_loss=expected,reference_estimated_uncertainty=uncertainty)
        result['gpu_valid']=gpu_valid
        failed=not gpu_valid or relative>=1e-4
        if args.fresnel:
            for channel in ('A','B'):
                expected_channel=ref['reference'][channel]
                actual=q[channel]
                channel_relative=abs(actual-expected_channel)/expected_channel if expected_channel else abs(actual)
                result['relative_'+channel+'_error']=channel_relative
                if channel_relative>=1e-4:failed=True
            nv=ref['nv'];roughness=ref['roughness'];rmin=roughness*roughness*math.exp(-20)
            # In the omitted tiny slope disk, 1-V.H is bounded by this value.
            fresnel_bound=min(1.,(1-nv)+math.sqrt((1-nv)*(1+nv))*rmin+rmin*rmin/2)**5
            bias_uncertainty=ref.get('bias_azimuth_refinement_difference',0)+ref['reference'].get('bias_radial_estimated_error',0)+ref['reference'].get('omitted_radius_probability_bound',0)*fresnel_bound
            assert bias_uncertainty<=ref['reference']['B']*1e-6+1e-34,'B reference uncertainty too large'
            result['B_reference_estimated_uncertainty']=bias_uncertainty
        results.append(result)
        if failed:failures.append(result)
    print(json.dumps(dict(status='unit_loss_grid_passed' if not failures else 'unit_loss_grid_failed',cases=len(results),failures=len(failures),relative_loss_threshold=1e-4,max_relative_loss_error=max(r['relative_loss_error'] for r in results),scope='Selected view/roughness grid; full layered BRDF and table interpolation not qualified.',fresnel_checked=args.fresnel,results=results),indent=2))
    raise SystemExit(bool(failures))


if __name__=='__main__':main()
