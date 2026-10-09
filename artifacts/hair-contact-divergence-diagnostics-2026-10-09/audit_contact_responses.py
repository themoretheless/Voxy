"""Independent high-precision solve of captured physical contact operators."""
import argparse,json
from decimal import Decimal,localcontext
from pathlib import Path
D=Decimal.from_float

def solve_case(case):
    first,end=case['first'],case['end'];n=len(case['rhs']);band=9
    matrix=list(map(D,case['matrix']));factor=matrix.copy()
    pivots=[]
    for i in range(first,end):
        start=max(first,i-band+1)
        for j in range(start,i+1):
            value=matrix[i*band+i-j]-sum((factor[i*band+i-k]*factor[j*band+j-k] for k in range(max(start,j-band+1),j)),Decimal(0))
            if i==j:
                if value<=0:raise ValueError('captured operator is not positive definite')
                pivots.append(value);factor[i*band]=value.sqrt()
            else:factor[i*band+i-j]=value/factor[j*band]
    outputs=[]
    for load in case['loads']:
        value=list(map(D,load))
        for i in range(first,end):
            value[i]=(value[i]-sum((factor[i*band+i-j]*value[j] for j in range(max(first,i-band+1),i)),Decimal(0)))/factor[i*band]
        for i in range(end-1,first-1,-1):
            value[i]=(value[i]-sum((factor[j*band+j-i]*value[j] for j in range(i+1,min(end,i+band))),Decimal(0)))/factor[i*band]
        outputs.append(value)
    return outputs,min(pivots)

def projection(case,responses):
    loads=[list(map(D,load)) for load in case['loads']];bounds=list(map(D,case['bounds']))
    gram=[[sum((a*b for a,b in zip(load,response)),Decimal(0)) for response in responses] for load in loads]
    k=gram;b=bounds;det=k[0][0]*k[1][1]-k[0][1]*k[1][0]
    candidates=[]
    if max(b)<=0:candidates.append([Decimal(0),Decimal(0)])
    for index in range(2):
        reaction=b[index]/k[index][index]
        other=1-index
        if reaction>=0 and k[other][index]*reaction>=b[other]:
            pair=[Decimal(0),Decimal(0)];pair[index]=reaction;candidates.append(pair)
    if det>0:
        reaction=[(b[0]*k[1][1]-b[1]*k[0][1])/det,(b[1]*k[0][0]-b[0]*k[1][0])/det]
        if min(reaction)>=0:candidates.append(reaction)
    if len(candidates)!=1:raise ValueError('ambiguous/unresolved independent two-plane active set')
    reaction=candidates[0]
    increment=[sum((responses[column][row]*reaction[column] for column in range(2)),Decimal(0)) for row in range(len(case['rhs']))]
    return {'gram':[[str(x) for x in row] for row in gram],'relative_determinant':str(det/(k[0][0]*k[1][1])),'reaction':list(map(str,reaction))},increment

def audit(matrix_path,response_path):
    cases=json.loads(Path(matrix_path).read_text());response=json.loads(Path(response_path).read_text());rows=[];references=[]
    with localcontext() as context:
        context.prec=80
        for index,case in enumerate(cases):
            exact,pivot=solve_case(case);references.append([[str(value) for value in vector] for vector in exact])
            description,ideal=projection(case,exact)
            row={'case':index,'minimum_cholesky_pivot':str(pivot),'exact_projection':description}
            increments={}
            for side in ['native','gpu']:
                actual=[list(map(D,vector)) for vector in response[side][index]]
                scale=max(abs(x) for vector in exact for x in vector)
                relative=max(abs(a-b) for x,y in zip(exact,actual) for a,b in zip(x,y))/scale
                desc,increment=projection(case,actual);increments[side]=increment
                position_error=max(abs(a-b) for coordinate,(a,b) in enumerate(zip(ideal,increment)) if coordinate%6<3)
                row[side]={'response_relative_error':str(relative),'projected_position_error_m':str(position_error),'projection':desc}
            row['native_gpu_projected_position_delta_m']=str(max(abs(a-b) for coordinate,(a,b) in enumerate(zip(increments['native'],increments['gpu'])) if coordinate%6<3))
            rows.append(row)
    return {'decimal_digits':80,'cases':rows,'reference_responses':references,'limits':'One fixed two-plane tangent problem, not a nonlinear/full-model admission result.'}

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('matrices');parser.add_argument('responses');parser.add_argument('--output',required=True);args=parser.parse_args()
    result=audit(args.matrices,args.responses);Path(args.output).write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({key:value for key,value in result.items() if key!='reference_responses'},indent=2))
