"""Offline sampled-cell refinement qualification, never production admission.

Uses fixed physical probes across levels. GPU coefficients and interpolation
are compared against independent adaptive f64 integration. Failing cases cause
refinement, never relaxed tolerances or silent omission.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('output',type=Path)
    p.add_argument('--moments',type=Path,required=True)
    p.add_argument('--lookup',type=Path,required=True)
    p.add_argument('--reference',type=Path,required=True)
    p.add_argument('--refinement-target',type=float,default=1e-5)
    a=p.parse_args();assert 0<a.refinement_target<=1e-4
    if (a.output/'progress.json').exists():raise FileExistsError('preserve existing qualification; choose a fresh output directory')
    a.output.mkdir(parents=True,exist_ok=True)
    here=Path(__file__).resolve().parent
    state={'status':'running','scope':'sampled physical cells; not whole-domain or production admission','pid':os.getpid(),'sample_relative_threshold':1e-4,'refinement_target':a.refinement_target,'levels':[]}
    def save():
        tmp=a.output/'progress.tmp';tmp.write_text(json.dumps(state,indent=2)+'\n');tmp.replace(a.output/'progress.json')
    def run(name,argv,allowed=(0,),environment=None):
        state['stage']=name;save()
        with (a.output/(name+'.stdout')).open('w') as out,(a.output/(name+'.stderr')).open('w') as err:
            result=subprocess.run([str(x) for x in argv],stdout=out,stderr=err,env=environment)
        if result.returncode not in allowed:raise RuntimeError(f'{name} failed with exit {result.returncode}')
    try:
        first=a.output/'level-1'
        run('generate-1',[sys.executable,here/'ggx_local_stencil.py','generate',first,'--cell-probes'])
        run('direct-probes',[a.moments,524288,'--query-file='+str(first.with_suffix('.probes.txt')),'--batch-size=32'])
        environment=dict(os.environ,VOXY_GGX_REFERENCE_WARPED='1')
        reference=a.output/'independent-reference.jsonl'
        run('reference',[sys.executable,here/'dfg_endpoint_reference.py',a.output/'direct-probes.stdout',a.reference,reference],environment=environment)
        for refinement in [1,2,4,8]:
            prefix=a.output/f'level-{refinement}'
            if refinement!=1:
                run(f'generate-{refinement}',[sys.executable,here/'ggx_local_stencil.py','generate',prefix,'--cell-probes','--refinement',refinement])
                assert prefix.with_suffix('.probes.txt').read_bytes()==first.with_suffix('.probes.txt').read_bytes(),'physical probes changed between levels'
            run(f'nodes-{refinement}',[a.moments,524288,'--query-file='+str(prefix.with_suffix('.txt')),'--batch-size=32'])
            run(f'lookup-{refinement}',[a.lookup,prefix.with_suffix('.json'),a.output/f'nodes-{refinement}.stdout'])
            run(f'comparison-{refinement}',[sys.executable,here/'ggx_compare_reference.py',reference,a.output/f'lookup-{refinement}.stdout','--samples',524288,'--fresnel'],allowed=(0,1))
            report=json.loads((a.output/f'comparison-{refinement}.stdout').read_text())
            assert report['cases']==72 and report['relative_loss_threshold']==1e-4
            maximum={c:max(r['relative_'+c+'_error'] for r in report['results']) for c in ['loss','A','B']}
            decision='accept_sampled_cells' if report['failures']==0 and max(maximum.values())<a.refinement_target else ('refine' if refinement<8 else 'reject_budget_exhausted')
            state['levels'].append({'refinement':refinement,'cases':report['cases'],'failures':report['failures'],'max_relative_errors':maximum,'decision':decision})
            save()
            if decision=='accept_sampled_cells':
                state.update(status='sampled_cells_passed',stage='complete',production_admitted=False,whole_cell_error_bound_proven=False);save();print(json.dumps(state));return
        state.update(status='refinement_budget_exhausted',production_admitted=False);save();raise SystemExit(1)
    except Exception as error:
        state.update(status='failed',error=str(error));save();raise


if __name__=='__main__':main()
